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
#[derive(Clone)]
struct App {
    routes: Arc<Vec<Route>>,
    exact: Arc<HashMap<String, HashMap<String, usize>>>,
    constants: Arc<Vec<Option<CachedReply>>>,
    jobs: mpsc::SyncSender<Job>,
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
    let method = request.method().as_str().to_owned();
    let path = request.uri().path();
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
    let search = request.uri().query().unwrap_or("").to_owned();
    let authority = request
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    let url = format!("http://{authority}{}", request.uri());
    let mut query = BTreeMap::new();
    for (k, v) in url::form_urlencoded::parse(search.as_bytes()) {
        query.entry(k.into_owned()).or_insert(v.into_owned());
    }
    let headers: Vec<_> = request
        .headers()
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|v| (k.as_str().to_owned(), v.to_owned()))
        })
        .collect();
    let content_type = request
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_owned();
    let bytes = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        to_bytes(request.into_body(), app.limits.body),
    )
    .await
    {
        Err(_) => {
            return error(
                StatusCode::REQUEST_TIMEOUT,
                "Request body deadline exceeded",
                id,
            );
        }
        Ok(Err(_)) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Request body exceeds limit",
                id,
            );
        }
        Ok(Ok(b)) => b,
    };
    let fields = match form(bytes.clone(), &content_type, app.limits.body).await {
        Ok(f) => f,
        Err(_) => return error(StatusCode::BAD_REQUEST, "Invalid form data", id),
    };
    let input=json!({"method":method,"url":url,"headers":headers,"query":query,"search":search,"params":params,"body":STANDARD.encode(bytes),"form":fields,"requestId":id.to_string()}).to_string();
    let (response_tx, response_rx) = oneshot::channel();
    if app
        .jobs
        .try_send(Job {
            route: index,
            input,
            id,
            response: response_tx,
        })
        .is_err()
    {
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
pub async fn serve(capsule: Arc<Capsule>, limits: Limits, address: SocketAddr) -> Result<()> {
    let (jobs, receiver) = mpsc::sync_channel::<Job>(8);
    let (ready, initialized) = mpsc::channel();
    let stopping = Arc::new(AtomicBool::new(false));
    let stop_worker = stopping.clone();
    let worker_limits = limits.clone();
    let worker_capsule = capsule.clone();
    let (done_tx, done_rx) = oneshot::channel();
    std::thread::Builder::new()
        .name("nio-js-engine".into())
        .spawn(move || {
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
            while let Ok(job) = receiver.recv() {
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
            let _ = done_tx.send(());
        })?;
    let routes = initialized
        .recv()
        .map_err(|_| anyhow!("engine startup failed"))?
        .map_err(|e| anyhow!(e))?;
    if routes.is_empty() {
        stopping.store(true, Ordering::Relaxed);
        drop(jobs);
        let _ = done_rx.await;
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
        jobs,
        limits: limits.clone(),
        ids: Arc::new(AtomicU64::new(1)),
        stopping: stopping.clone(),
        requests: Arc::new(tokio::sync::Semaphore::new(8)),
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
    let _ = tokio::time::timeout(limits.timeout + std::time::Duration::from_secs(1), done_rx).await;
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
