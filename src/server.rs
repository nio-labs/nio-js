use crate::{
    engine::{Engine, Limits, Reply, Route},
    prepare::Capsule,
};
use anyhow::{Result, anyhow};
use axum::{
    Router,
    body::{Body, Bytes, to_bytes},
    extract::{Request, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::Response,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
};
use tokio::sync::oneshot;
struct Job {
    route: usize,
    input: String,
    id: u64,
    response: oneshot::Sender<Result<Reply, String>>,
}
#[derive(serde::Serialize)]
struct RequestPayload<'a> {
    method: &'a str,
    url: &'a str,
    headers: &'a [(&'a str, &'a str)],
    query: &'a BTreeMap<String, String>,
    search: &'a str,
    params: &'a BTreeMap<String, String>,
    body: &'a str,
    form: &'a Option<Vec<Value>>,
    #[serde(rename = "requestId")]
    request_id: &'a str,
}
#[derive(Clone)]
struct App {
    routes: Arc<Vec<Route>>,
    exact: Arc<HashMap<String, HashMap<String, usize>>>,
    constants: Arc<Vec<Option<CachedReply>>>,
    workers: Arc<Vec<mpsc::SyncSender<Job>>>,
    round_robin: Arc<std::sync::atomic::AtomicUsize>,
    limits: Limits,
    ids: Arc<AtomicU64>,
    stopping: Arc<AtomicBool>,
    requests: Arc<tokio::sync::Semaphore>,
}
#[derive(Clone)]
struct CachedReply {
    status: StatusCode,
    headers: HeaderMap,
    body: Bytes,
}
impl CachedReply {
    fn new(reply: Reply) -> Result<Self> {
        let status = StatusCode::from_u16(reply.status)?;
        let mut headers = HeaderMap::new();
        match reply.fast_type {
            1 => {
                headers.insert(
                    "content-type",
                    HeaderValue::from_static("text/plain; charset=utf-8"),
                );
            }
            2 => {
                headers.insert(
                    "content-type",
                    HeaderValue::from_static("application/json; charset=utf-8"),
                );
            }
            _ => {
                for (key, value) in reply.headers {
                    let name = HeaderName::try_from(key)?;
                    if matches!(
                        name.as_str(),
                        "connection"
                            | "transfer-encoding"
                            | "content-length"
                            | "upgrade"
                            | "keep-alive"
                            | "trailer"
                    ) {
                        continue;
                    }
                    headers.insert(name, HeaderValue::try_from(value)?);
                }
            }
        }
        Ok(Self {
            status,
            headers,
            body: reply.body,
        })
    }
    fn response(&self, id: u64) -> Response {
        let mut r = Response::new(
            if self.status == StatusCode::NO_CONTENT || self.status == StatusCode::NOT_MODIFIED {
                Body::empty()
            } else {
                Body::from(self.body.clone())
            },
        );
        *r.status_mut() = self.status;
        *r.headers_mut() = self.headers.clone();
        r.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(&id.to_string()).unwrap(),
        );
        r
    }
}
fn response(reply: Reply, id: u64) -> Result<Response> {
    Ok(CachedReply::new(reply)?.response(id))
}
fn error(status: StatusCode, message: &str, id: u64) -> Response {
    let mut r = Response::new(Body::from(
        json!({"error":message,"requestId":id.to_string()}).to_string(),
    ));
    *r.status_mut() = status;
    r.headers_mut().insert(
        "content-type",
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    r.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&id.to_string()).unwrap(),
    );
    r
}
fn match_path(pattern: &str, path: &str) -> Option<(BTreeMap<String, String>, usize)> {
    let a: Vec<_> = pattern.split('/').collect();
    let b: Vec<_> = path.split('/').collect();
    if a.len() != b.len() {
        return None;
    }
    let mut params = BTreeMap::new();
    let mut score = 0;
    for (a, b) in a.into_iter().zip(b) {
        if let Some(name) = a.strip_prefix(':') {
            if name.is_empty() || b.is_empty() {
                return None;
            }
            params.insert(
                name.into(),
                percent_encoding::percent_decode_str(b)
                    .decode_utf8()
                    .ok()?
                    .into_owned(),
            );
        } else if a != b {
            return None;
        } else {
            score += 1;
        }
    }
    Some((params, score))
}
async fn form(bytes: Bytes, content_type: &str, max: usize) -> Result<Option<Vec<Value>>> {
    if content_type.starts_with("application/x-www-form-urlencoded") {
        let mut fields = Vec::new();
        for (name, value) in url::form_urlencoded::parse(&bytes) {
            if fields.len() >= 32 {
                return Err(anyhow!("too many form fields"));
            }
            fields.push(json!({"name":name,"filename":null,"type":null,"body":STANDARD.encode(value.as_bytes())}));
        }
        return Ok(Some(fields));
    }
    if !content_type.starts_with("multipart/form-data") {
        return Ok(None);
    }
    let boundary = multer::parse_boundary(content_type)?;
    let constraints = multer::Constraints::new().size_limit(
        multer::SizeLimit::new()
            .whole_stream(max as u64)
            .per_field(max as u64),
    );
    let stream = futures_util::stream::once(async move { Ok::<Bytes, std::io::Error>(bytes) });
    let mut multipart = multer::Multipart::with_constraints(stream, boundary, constraints);
    let mut fields = Vec::new();
    while let Some(field) = multipart.next_field().await? {
        if fields.len() >= 32 {
            return Err(anyhow!("too many form fields"));
        }
        let name = field
            .name()
            .ok_or_else(|| anyhow!("form field missing name"))?
            .to_owned();
        let filename = field.file_name().map(str::to_owned);
        let ty = field.content_type().map(|t| t.to_string());
        let data = field.bytes().await?;
        fields
            .push(json!({"name":name,"filename":filename,"type":ty,"body":STANDARD.encode(data)}));
    }
    Ok(Some(fields))
}
async fn handle(State(app): State<App>, request: Request) -> Response {
    let id = app.ids.fetch_add(1, Ordering::Relaxed);
    let Ok(_permit) = app.requests.clone().try_acquire_owned() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Too many concurrent requests",
            id,
        );
    };
    if app.stopping.load(Ordering::Relaxed) {
        return error(StatusCode::SERVICE_UNAVAILABLE, "Service is stopping", id);
    }
    let (parts, body) = request.into_parts();
    let method = parts.method.as_str().to_owned();
    let path = parts.uri.path();
    let mut candidates = Vec::new();
    let mut allowed = Vec::new();
    if let Some(index) = app
        .exact
        .get(path)
        .and_then(|methods| methods.get(method.as_str()))
    {
        candidates.push((*index, BTreeMap::new(), usize::MAX));
    } else {
        for (index, route) in app.routes.iter().enumerate() {
            if let Some((params, score)) = match_path(&route.path, path) {
                allowed.push(route.method.as_str());
                if route.method == method {
                    candidates.push((index, params, score));
                }
            }
        }
    }
    let Some((index, params, _)) = candidates.into_iter().max_by_key(|(_, _, score)| *score) else {
        let mut r = error(
            if allowed.is_empty() {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::METHOD_NOT_ALLOWED
            },
            if allowed.is_empty() {
                "Not Found"
            } else {
                "Method Not Allowed"
            },
            id,
        );
        if !allowed.is_empty() {
            allowed.sort();
            allowed.dedup();
            r.headers_mut()
                .insert("allow", HeaderValue::from_str(&allowed.join(", ")).unwrap());
        }
        return r;
    };
    if let Some(reply) = &app.constants[index] {
        return reply.response(id);
    }
    let route_arity = app.routes[index].arity;
    let input = if route_arity == 0 {
        String::new()
    } else {
        let search = parts.uri.query().unwrap_or("");
        let authority = parts
            .headers
            .get("host")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("localhost");
        let url = format!("http://{authority}{}", parts.uri);
        let mut query = BTreeMap::new();
        if !search.is_empty() {
            for (k, v) in url::form_urlencoded::parse(search.as_bytes()) {
                query.entry(k.into_owned()).or_insert(v.into_owned());
            }
        }
        let headers: Vec<(&str, &str)> = parts
            .headers
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|val| (k.as_str(), val)))
            .collect();
        let content_type = parts
            .headers
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let has_body = parts
            .headers
            .get("content-length")
            .is_some_and(|v| v != "0")
            || parts.headers.contains_key("transfer-encoding");
        let bytes = if !has_body {
            Bytes::new()
        } else {
            match to_bytes(body, app.limits.body).await {
                Ok(b) => b,
                Err(_) => {
                    return error(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "Request body exceeds limit",
                        id,
                    );
                }
            }
        };
        let (encoded_body, fields) = if bytes.is_empty() {
            (String::new(), None)
        } else {
            let fields = match form(bytes.clone(), content_type, app.limits.body).await {
                Ok(f) => f,
                Err(_) => return error(StatusCode::BAD_REQUEST, "Invalid form data", id),
            };
            (STANDARD.encode(bytes), fields)
        };
        let id_str = id.to_string();
        match serde_json::to_string(&RequestPayload {
            method: &method,
            url: &url,
            headers: &headers,
            query: &query,
            search,
            params: &params,
            body: &encoded_body,
            form: &fields,
            request_id: &id_str,
        }) {
            Ok(s) => s,
            Err(_) => {
                return error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to serialize request",
                    id,
                );
            }
        }
    };
    let (response_tx, response_rx) = oneshot::channel();
    let num_workers = app.workers.len();
    let start_idx = app.round_robin.fetch_add(1, Ordering::Relaxed) % num_workers;
    let mut job = Some(Job {
        route: index,
        input,
        id,
        response: response_tx,
    });
    for i in 0..num_workers {
        let idx = (start_idx + i) % num_workers;
        match app.workers[idx].try_send(job.take().unwrap()) {
            Ok(()) => break,
            Err(mpsc::TrySendError::Full(j)) => {
                job = Some(j);
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                return error(StatusCode::SERVICE_UNAVAILABLE, "Execution unavailable", id);
            }
        }
    }
    if job.is_some() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Execution queue is full or unavailable",
            id,
        );
    }
    match tokio::time::timeout(app.limits.timeout * 9, response_rx).await {
        Ok(Ok(Ok(reply))) => response(reply, id)
            .unwrap_or_else(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Invalid response", id)),
        Ok(Ok(Err(message))) => error(
            if message.contains("deadline") || message.contains("interrupted") {
                StatusCode::GATEWAY_TIMEOUT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            },
            "Handler failed",
            id,
        ),
        _ => error(StatusCode::SERVICE_UNAVAILABLE, "Execution unavailable", id),
    }
}
async fn shutdown() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("signal handler");
        tokio::select! { _=tokio::signal::ctrl_c()=>{}, _=term.recv()=>{} }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
pub async fn serve(
    capsule: Arc<Capsule>,
    limits: Limits,
    address: SocketAddr,
    workers: usize,
) -> Result<()> {
    let workers = workers.max(1);
    let per_worker_queue = 32;
    let total_queue_size = workers * per_worker_queue;
    let (ready_tx, ready_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel::<()>();
    let stopping = Arc::new(AtomicBool::new(false));
    let mut senders = Vec::with_capacity(workers);

    for worker_id in 0..workers {
        let (worker_tx, worker_rx) = mpsc::sync_channel::<Job>(per_worker_queue);
        senders.push(worker_tx);
        let stop_worker = stopping.clone();
        let worker_limits = limits.clone();
        let worker_capsule = capsule.clone();
        let ready = ready_tx.clone();
        let done = done_tx.clone();
        std::thread::Builder::new()
            .name(format!("nio-js-engine-{worker_id}"))
            .spawn(move || {
                let _done = done;
                let mut engine = match Engine::new(worker_capsule.clone(), worker_limits.clone()) {
                    Ok(e) => e,
                    Err(e) => {
                        let _ = ready.send(Err(e.to_string()));
                        return;
                    }
                };
                if ready.send(Ok(engine.routes.clone())).is_err() {
                    return;
                }
                while let Ok(job) = worker_rx.recv() {
                    if job.response.is_closed() || stop_worker.load(Ordering::Relaxed) {
                        continue;
                    }
                    let result = engine
                        .dispatch(job.route, job.input)
                        .map_err(|e| e.to_string());
                    let failed = result.is_err();
                    if let Err(e) = &result {
                        eprintln!("request {}: {e}", job.id);
                    }
                    let _ = job.response.send(result);
                    if failed {
                        // Drop outstanding jobs and bindings from a failed invocation before accepting another.
                        match Engine::new(worker_capsule.clone(), worker_limits.clone()) {
                            Ok(e) => engine = e,
                            Err(e) => {
                                eprintln!("runtime recycle failed: {e}");
                                break;
                            }
                        }
                    }
                }
            })?;
    }
    drop(ready_tx);

    let mut initial_routes = None;
    for _ in 0..workers {
        let routes = ready_rx
            .recv()
            .map_err(|_| anyhow!("engine startup failed"))?
            .map_err(|e| anyhow!(e))?;
        if initial_routes.is_none() {
            initial_routes = Some(routes);
        }
    }
    let routes = initial_routes.unwrap_or_default();
    if routes.is_empty() {
        stopping.store(true, Ordering::Relaxed);
        drop(senders);
        drop(done_tx);
        let _ = tokio::task::spawn_blocking(move || {
            let _ = done_rx.recv();
        })
        .await;
        return Ok(());
    }
    let mut exact: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let constants = routes
        .iter()
        .map(|route| route.constant.clone().map(CachedReply::new).transpose())
        .collect::<Result<Vec<_>>>()?;
    for (index, route) in routes.iter().enumerate() {
        if !route.path.split('/').any(|part| part.starts_with(':')) {
            exact
                .entry(route.path.clone())
                .or_default()
                .insert(route.method.clone(), index);
        }
    }
    let app = App {
        routes: Arc::new(routes),
        exact: Arc::new(exact),
        constants: Arc::new(constants),
        workers: Arc::new(senders),
        round_robin: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        limits: limits.clone(),
        ids: Arc::new(AtomicU64::new(1)),
        stopping: stopping.clone(),
        requests: Arc::new(tokio::sync::Semaphore::new(total_queue_size)),
    };
    let router = Router::new().fallback(handle).with_state(app);
    let listener = tokio::net::TcpListener::bind(address).await?;
    eprintln!("nio-js listening on http://{}", listener.local_addr()?);
    let (signal_tx, signal_rx) = oneshot::channel();
    use std::future::IntoFuture;
    let server = axum::serve(listener, router)
        .with_graceful_shutdown(async {
            let _ = signal_rx.await;
        })
        .into_future();
    tokio::pin!(server);
    tokio::select! {
        result=&mut server => { result?; },
        _=shutdown() => {
            stopping.store(true,Ordering::Relaxed);
            let _=signal_tx.send(());
            if tokio::time::timeout(limits.timeout+std::time::Duration::from_secs(5),&mut server).await.is_err() { eprintln!("shutdown drain deadline reached"); }
        }
    }
    drop(done_tx);
    let _ = tokio::time::timeout(
        limits.timeout + std::time::Duration::from_secs(1),
        tokio::task::spawn_blocking(move || {
            let _ = done_rx.recv();
        }),
    )
    .await;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn route_matching_is_segment_scoped() {
        assert!(match_path("/users/:id", "/users/42").is_some());
        assert!(match_path("/users/:id", "/users/42/extra").is_none());
        assert!(match_path("/users/:id", "/users/").is_none());
    }
}
