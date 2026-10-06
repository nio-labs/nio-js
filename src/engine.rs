use crate::{
    network::{self, Policy},
    prepare::Capsule,
};
use anyhow::{Result, bail, ensure};
use axum::body::Bytes;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rquickjs::{
    Array, CatchResultExt, Context, Ctx, FromJs, Function, Module, Object, Promise, Runtime,
    TypedArray, Value,
    loader::{ImportAttributes, Loader, Resolver},
    module::Declared,
};
use serde_json::json;
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Limits {
    pub memory: usize,
    pub timeout: Duration,
    pub body: usize,
    pub policy: Policy,
    pub in_flight: Arc<AtomicUsize>,
}
#[derive(Clone, Debug)]
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
    pub fast_type: u8,
}
impl<'js> FromJs<'js> for Reply {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
        if let Some(array) = value.as_array() {
            let status: u16 = array.get(0)?;
            let fast_type: u8 = array.get(1)?;
            let raw: Value = array.get(2)?;
            let body = if raw.is_string() {
                Bytes::from(String::from_js(ctx, raw)?.into_bytes())
            } else {
                let array = TypedArray::<u8>::from_js(ctx, raw)?;
                let bytes = unsafe { array.as_bytes() }.ok_or(rquickjs::Error::Unknown)?;
                Bytes::copy_from_slice(bytes)
            };
            let headers = if fast_type != 0 {
                Vec::new()
            } else {
                let headers: Array = array.get(3)?;
                headers
                    .iter::<Array>()
                    .map(|pair| {
                        let pair = pair?;
                        Ok((pair.get(0)?, pair.get(1)?))
                    })
                    .collect::<rquickjs::Result<Vec<(String, String)>>>()?
            };
            return Ok(Self {
                status,
                headers,
                body,
                fast_type,
            });
        }
        let object = Object::from_js(ctx, value)?;
        let raw: Value = object.get("body")?;
        let body = if raw.is_string() {
            Bytes::from(String::from_js(ctx, raw)?.into_bytes())
        } else {
            let array = TypedArray::<u8>::from_js(ctx, raw)?;
            // Copy immediately: no JS runs while the borrowed buffer is alive.
            let bytes = unsafe { array.as_bytes() }.ok_or(rquickjs::Error::Unknown)?;
            Bytes::copy_from_slice(bytes)
        };
        let fast_type: u8 = object.get("fastType").unwrap_or(0);
        let headers = if fast_type != 0 {
            Vec::new()
        } else {
            let headers: Array = object.get("headers")?;
            headers
                .iter::<Array>()
                .map(|pair| {
                    let pair = pair?;
                    Ok((pair.get(0)?, pair.get(1)?))
                })
                .collect::<rquickjs::Result<Vec<(String, String)>>>()?
        };
        Ok(Self {
            status: object.get("status")?,
            headers,
            body,
            fast_type,
        })
    }
}
#[derive(Clone, Debug)]
pub struct Route {
    pub method: String,
    pub path: String,
    pub constant: Option<Reply>,
    pub arity: usize,
}
impl<'js> FromJs<'js> for Route {
    fn from_js(ctx: &Ctx<'js>, value: Value<'js>) -> rquickjs::Result<Self> {
        let object = Object::from_js(ctx, value)?;
        let arity: usize = object.get("arity").unwrap_or(0);
        Ok(Self {
            method: object.get("method")?,
            path: object.get("path")?,
            constant: object.get("constant")?,
            arity,
        })
    }
}
struct Graph(Arc<Capsule>);
impl Resolver for Graph {
    fn resolve<'js>(
        &mut self,
        _ctx: &Ctx<'js>,
        base: &str,
        name: &str,
        attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<String> {
        if attributes.is_some() {
            return Err(rquickjs::Error::new_resolving_message(
                base,
                name,
                "import attributes unsupported",
            ));
        }
        if name == "nio.js" {
            return Ok(name.into());
        }
        self.0
            .modules
            .get(base)
            .and_then(|m| m.imports.get(name))
            .cloned()
            .ok_or_else(|| {
                rquickjs::Error::new_resolving_message(
                    base,
                    name,
                    "module is not in the prepared graph",
                )
            })
    }
}
impl Loader for Graph {
    fn load<'js>(
        &mut self,
        ctx: &Ctx<'js>,
        name: &str,
        attributes: Option<ImportAttributes<'js>>,
    ) -> rquickjs::Result<Module<'js, Declared>> {
        if attributes.is_some() {
            return Err(rquickjs::Error::new_loading_message(
                name,
                "import attributes unsupported",
            ));
        }
        let code = if name == "nio.js" {
            include_str!("../runtime/nio.js")
        } else {
            &self
                .0
                .modules
                .get(name)
                .ok_or_else(|| rquickjs::Error::new_loading(name))?
                .code
        };
        Module::declare(ctx.clone(), name, code)
    }
}
struct Completion {
    id: u32,
    data: String,
}
pub struct Engine {
    pub routes: Vec<Route>,
    context: Context,
    runtime: Runtime,
    deadline: Arc<Mutex<Instant>>,
    completions: mpsc::Receiver<Completion>,
    timers: Arc<Mutex<Vec<(Instant, u32)>>>,
    limits: Limits,
    capsule: Arc<Capsule>,
}
impl Engine {
    pub fn new(capsule: Arc<Capsule>, limits: Limits) -> Result<Self> {
        for host in &capsule.network {
            ensure!(
                limits.policy.hosts.contains(host),
                "capsule requires ungranted network destination {host}"
            );
        }
        let runtime = Runtime::new()?;
        runtime.set_memory_limit(limits.memory);
        runtime.set_max_stack_size(512 * 1024);
        let deadline = Arc::new(Mutex::new(Instant::now() + limits.timeout));
        let deadline_check = deadline.clone();
        runtime.set_interrupt_handler(Some(Box::new(move || {
            Instant::now() >= *deadline_check.lock().unwrap()
        })));
        runtime.set_loader(Graph(capsule.clone()), Graph(capsule.clone()));
        let context = Context::full(&runtime)?;
        let assets = capsule.assets.clone();
        let (tx, completions) = mpsc::channel();
        let timers = Arc::new(Mutex::new(Vec::new()));
        let timers_cb = timers.clone();
        let policy = limits.policy.clone();
        let max = limits.body;
        let inflight = limits.in_flight.clone();
        let op_deadline = deadline.clone();
        context.with(|ctx| -> Result<()> {
            let globals = ctx.globals();
            globals.set("__nioMaxBody",max as u32)?;
            globals.set("__nioAsset",Function::new(ctx.clone(),move |name:String| -> rquickjs::Result<String> {
                let asset=assets.get(&name).ok_or(rquickjs::Error::Unknown)?;
                serde_json::to_string(asset).map_err(|_|rquickjs::Error::Unknown)
            })?)?;
            globals.set("__nioNative", Function::new(ctx.clone(), move |name: String| -> rquickjs::Result<String> {
                match name.as_str() {
                    "cpu" => {
                        let mut value: i32 = 42;
                        for _ in 0..100_000 {
                            value = value.wrapping_mul(1664525).wrapping_add(1013904223);
                        }
                        Ok((value as u32).to_string())
                    }
                    _ => Err(rquickjs::Error::Unknown),
                }
            })?)?;
            globals.set(
                "__nioPython",
                Function::new(
                    ctx.clone(),
                    move |module: String, func: String, payload: String| -> rquickjs::Result<String> {
                        crate::python::call_python_fn(&module, &func, &payload)
                            .map_err(|e| rquickjs::Error::new_loading_message(format!("{module}.{func}"), e.to_string()))
                    },
                )?,
            )?;
            globals.set(
                "__nioPythonEval",
                Function::new(
                    ctx.clone(),
                    move |code: String| -> rquickjs::Result<String> {
                        crate::python::eval_python(&code)
                            .map_err(|e| rquickjs::Error::new_loading_message("python.eval", e.to_string()))
                    },
                )?,
            )?;
            globals.set("__nioLog", Function::new(ctx.clone(),|s:String| { eprintln!("{s}"); })?)?;
            globals.set("__nioEncode", Function::new(ctx.clone(),|s:String|s.into_bytes())?)?;
            globals.set("__nioDecode", Function::new(ctx.clone(),|b:Vec<u8>|String::from_utf8_lossy(&b).into_owned())?)?;
            globals.set("__nioBase64",Function::new(ctx.clone(),|b:Vec<u8>|STANDARD.encode(b))?)?;
            globals.set("__nioUnbase64",Function::new(ctx.clone(),|s:String|STANDARD.decode(s).map_err(|_|rquickjs::Error::Unknown))?)?;
            globals.set("__nioQuery",Function::new(ctx.clone(),|s:String|url::form_urlencoded::parse(s.trim_start_matches('?').as_bytes()).map(|(a,b)|vec![a.into_owned(),b.into_owned()]).collect::<Vec<_>>())?)?;
            globals.set("__nioQueryEncode",Function::new(ctx.clone(),|s:String| -> rquickjs::Result<String> {
                let pairs:Vec<(String,String)> = serde_json::from_str(&s).map_err(|_|rquickjs::Error::Unknown)?;
                Ok(url::form_urlencoded::Serializer::new(String::new()).extend_pairs(pairs).finish())
            })?)?;
            globals.set("__nioUrl",Function::new(ctx.clone(),|s:String,base:String| -> rquickjs::Result<String> {
                let u = if base.is_empty() { url::Url::parse(&s) } else { url::Url::parse(&base).and_then(|b|b.join(&s)) }.map_err(|_|rquickjs::Error::Unknown)?;
                Ok(json!({"href":u.as_str(),"protocol":format!("{}:",u.scheme()),"hostname":u.host_str().unwrap_or(""),"host":u.host_str().map(|h|u.port().map(|p|format!("{h}:{p}")).unwrap_or(h.into())).unwrap_or_default(),"port":u.port().map(|p|p.to_string()).unwrap_or_default(),"pathname":u.path(),"search":u.query().map(|q|format!("?{q}")).unwrap_or_default(),"hash":u.fragment().map(|f|format!("#{f}")).unwrap_or_default(),"origin":u.origin().ascii_serialization()}).to_string())
            })?)?;
            globals.set("__nioOperation",Function::new(ctx.clone(),move |id:u32,kind:String,payload:String| -> rquickjs::Result<()> {
                let payload:serde_json::Value = serde_json::from_str(&payload).map_err(|_|rquickjs::Error::Unknown)?;
                if kind == "sleep" {
                    let ms = payload["ms"].as_u64().filter(|v|*v<=30000).ok_or(rquickjs::Error::Unknown)?;
                    let mut timers = timers_cb.lock().unwrap(); if timers.len() >= 8 { return Err(rquickjs::Error::Unknown); }
                    timers.push((Instant::now()+Duration::from_millis(ms),id)); return Ok(());
                }
                if kind != "fetch" { return Err(rquickjs::Error::Unknown); }
                let address = payload["url"].as_str().ok_or(rquickjs::Error::Unknown)?.to_owned();
                if inflight.fetch_update(Ordering::SeqCst,Ordering::SeqCst,|n| (n<8).then_some(n+1)).is_err() { return Err(rquickjs::Error::Unknown); }
                let tx=tx.clone(); let policy=policy.clone(); let inflight=inflight.clone();
                let timeout = op_deadline.lock().unwrap().saturating_duration_since(Instant::now());
                std::thread::spawn(move || {
                    let data = match network::download(&address,&policy,max,timeout,false) {
                        Ok(d) => json!({"value":{"url":d.url,"status":d.status,"headers":d.headers,"body":STANDARD.encode(d.bytes)}}),
                        Err(e) => json!({"error":e.to_string()}),
                    };
                    inflight.fetch_sub(1,Ordering::SeqCst);
                    let _ = tx.send(Completion { id,data:data.to_string() });
                }); Ok(())
            })?)?;
            ctx.eval::<(),_>(include_str!("../runtime/bootstrap.js")).catch(&ctx).map_err(|e|anyhow::anyhow!("bootstrap: {e}"))?;
            let promise = Module::evaluate(ctx.clone(),capsule.entry.as_str(),capsule.modules[&capsule.entry].code.as_str()).catch(&ctx).map_err(|e|anyhow::anyhow!(mapped_diagnostic(&capsule,&format!("entry module: {e}"))))?;
            ctx.globals().set("__nioEntry",promise)?;
            Ok(())
        })?;
        let mut engine = Self {
            routes: Vec::new(),
            context,
            runtime,
            deadline,
            completions,
            timers,
            limits,
            capsule,
        };
        engine
            .settle::<()>("__nioEntry")
            .map_err(|e| anyhow::anyhow!(mapped_diagnostic(&engine.capsule, &e.to_string())))?;
        engine.routes = engine.context.with(|ctx| -> Result<Vec<Route>> {
            let f: Function = ctx.globals().get("__nioRoutes")?;
            let value: Vec<Route> = f
                .call(())
                .catch(&ctx)
                .map_err(|e| anyhow::anyhow!("route registration: {e}"))?;
            for route in &value {
                if let Some(reply) = &route.constant {
                    ensure!(reply.body.len() <= engine.limits.body, "response too large");
                }
            }
            Ok(value)
        })?;
        Ok(engine)
    }
    pub fn dispatch(&mut self, route: usize, input: String) -> Result<Reply> {
        self.dispatch_inner(route, input)
            .map_err(|e| anyhow::anyhow!(mapped_diagnostic(&self.capsule, &e.to_string())))
    }
    fn dispatch_inner(&mut self, route: usize, input: String) -> Result<Reply> {
        *self.deadline.lock().unwrap() = Instant::now() + self.limits.timeout;
        // Keep the promise rooted in the context while host I/O runs outside it.
        let immediate = self.context.with(|ctx| -> Result<Option<Reply>> {
            let f: Function = ctx.globals().get("__nioDispatch")?;
            let value: Value = f
                .call((route as u32, input))
                .catch(&ctx)
                .map_err(|e| anyhow::anyhow!("handler: {e}"))?;
            if value.is_promise() {
                ctx.globals().set("__nioActive", value)?;
                Ok(None)
            } else {
                Ok(Some(Reply::from_js(&ctx, value)?))
            }
        })?;
        let reply = match immediate {
            Some(reply) => reply,
            None => self.settle("__nioActive")?,
        };
        ensure!(reply.body.len() <= self.limits.body, "response too large");
        // Finish a microtask checkpoint even when the handler returns synchronously.
        while match self.runtime.execute_pending_job() {
            Ok(true) => true,
            Ok(false) => false,
            Err(e) => bail!("JavaScript job failed: {e:?}"),
        } {
            ensure!(
                Instant::now() < *self.deadline.lock().unwrap(),
                "handler deadline exceeded"
            );
        }
        Ok(reply)
    }
    fn settle<T>(&mut self, global: &str) -> Result<T>
    where
        T: for<'js> rquickjs::FromJs<'js>,
    {
        loop {
            if Instant::now() >= *self.deadline.lock().unwrap() {
                bail!("handler deadline exceeded");
            }
            let result = self.context.with(|ctx| -> Result<Option<T>> {
                let p: Promise = ctx.globals().get(global)?;
                match p.result::<T>() {
                    Some(v) => Ok(Some(
                        v.catch(&ctx)
                            .map_err(|e| anyhow::anyhow!("handler rejected: {e}"))?,
                    )),
                    None => Ok(None),
                }
            })?;
            if let Some(value) = result {
                self.context.with(|ctx| ctx.globals().remove(global))?;
                return Ok(value);
            }
            match self.runtime.execute_pending_job() {
                Ok(true) => continue,
                Ok(false) => {}
                Err(e) => bail!("JavaScript job failed: {e:?}"),
            }
            let now = Instant::now();
            let mut ready = Vec::new();
            self.timers.lock().unwrap().retain(|(at, id)| {
                if *at <= now {
                    ready.push(Completion {
                        id: *id,
                        data: "{\"value\":null}".into(),
                    });
                    false
                } else {
                    true
                }
            });
            ready.extend(self.completions.try_iter());
            if ready.is_empty() {
                std::thread::sleep(Duration::from_millis(1));
            }
            for completion in ready {
                self.context.with(|ctx| -> Result<()> {
                    let f: Function = ctx.globals().get("__nioComplete")?;
                    f.call::<_, ()>((completion.id, completion.data))
                        .catch(&ctx)
                        .map_err(|e| anyhow::anyhow!("host completion: {e}"))?;
                    Ok(())
                })?;
            }
        }
    }
}

fn mapped_diagnostic(capsule: &Capsule, message: &str) -> String {
    let mut output = message.to_owned();
    for (name, module) in &capsule.modules {
        let Ok(map) = oxc_sourcemap::SourceMap::from_json_string(&module.source_map) else {
            continue;
        };
        let lookup = map.generate_lookup_table();
        let prefix = format!("{name}:");
        for line in message.lines() {
            let Some(start) = line.find(&prefix) else {
                continue;
            };
            let rest = &line[start + prefix.len()..];
            let location: String = rest
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == ':')
                .collect();
            let parts: Vec<_> = location.trim_end_matches(':').split(':').collect();
            let Some(generated_line) = parts
                .first()
                .and_then(|v| v.parse::<u32>().ok())
                .filter(|v| *v > 0)
            else {
                continue;
            };
            let col = parts
                .get(1)
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(1)
                .saturating_sub(1);
            if let Some(token) = map.lookup_token_approx(&lookup, generated_line - 1, col) {
                output = output.replace(
                    &format!("{prefix}{location}"),
                    &format!(
                        "{name}:{}:{}",
                        token.get_src_line() + 1,
                        token.get_src_col() + 1
                    ),
                );
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prepare::{Options, prepare};
    fn fixture(source: &str, timeout: u64) -> Result<(tempfile::TempDir, Engine)> {
        let dir = tempfile::tempdir()?;
        std::fs::write(dir.path().join("main.ts"), source)?;
        let c = prepare(&dir.path().join("main.ts"), &Options::default())?;
        let engine = Engine::new(
            Arc::new(c),
            Limits {
                memory: 16 * 1024 * 1024,
                timeout: Duration::from_millis(timeout),
                body: 65536,
                policy: Policy::default(),
                in_flight: Arc::new(AtomicUsize::new(0)),
            },
        )?;
        Ok((dir, engine))
    }
    fn input() -> String {
        json!({"query":{},"params":{},"headers":[],"search":"","body":"","form":null,"requestId":"1"}).to_string()
    }
    fn text(reply: Reply) -> String {
        String::from_utf8(reply.body.to_vec()).unwrap()
    }
    #[test]
    fn native_reply_unicode_binary_thenables_and_microtasks() -> Result<()> {
        let (_dir, mut e) = fixture(
            "import {get,reply} from 'nio.js'; let n=0; get('/text',()=>{Promise.resolve().then(()=>n++);return 'héllo 🌏'}); get('/state',()=>String(n)); get('/binary',()=>reply(new Blob([new Uint8Array([0,255,128])]),{status:201,headers:{'x-binary':'yes'}})); get('/thenable',()=>({then(resolve){resolve('done')}}));",
            1000,
        )?;
        assert_eq!(text(e.dispatch(0, input())?), "héllo 🌏");
        assert_eq!(text(e.dispatch(1, input())?), "1");
        let binary = e.dispatch(2, input())?;
        assert_eq!(binary.status, 201);
        assert_eq!(binary.body.as_ref(), &[0, 255, 128]);
        assert!(binary.headers.contains(&("x-binary".into(), "yes".into())));
        assert_eq!(text(e.dispatch(3, input())?), "done");
        for source in [
            "import {get} from 'nio.js'; get('/',()=> 'é'.repeat(40000));",
            "import {get} from 'nio.js'; get('/',()=> new Blob([new Uint8Array(65537)]));",
        ] {
            let (_dir, mut e) = fixture(source, 1000)?;
            assert!(e.dispatch(0, input()).is_err());
        }
        assert!(
            fixture(
                "import {get} from 'nio.js'; get('/', 'é'.repeat(40000));",
                1000
            )
            .is_err()
        );
        Ok(())
    }
    #[test]
    fn async_callbacks_state_and_reply() -> Result<()> {
        let (_dir, mut e) = fixture(
            "import {get,reply} from 'nio.js'; let n:number=0; get('/',async()=>{await Promise.resolve(); return reply({n:++n},{status:201,headers:{'x-test':'yes'}})});",
            1000,
        )?;
        let r = e.dispatch(0, input())?;
        assert_eq!(r.status, 201);
        assert_eq!(text(r), "{\"n\":1}");
        assert_eq!(text(e.dispatch(0, input())?), "{\"n\":2}");
        Ok(())
    }
    #[test]
    fn timers_and_binary_files() -> Result<()> {
        let (_dir, mut e) = fixture(
            "import {get} from 'nio.js'; await new Promise(r=>setTimeout(r,5)); get('/',async()=>{await new Promise(r=>setTimeout(r,5)); const f=new File(['héllo'],'a.txt',{type:'text/plain'}); return {text:await f.text(),size:f.size,name:f.name};});",
            1000,
        )?;
        let value: serde_json::Value = serde_json::from_str(&text(e.dispatch(0, input())?))?;
        assert_eq!(value["text"], "héllo");
        assert_eq!(value["size"], 6);
        Ok(())
    }
    #[test]
    fn infinite_loop_and_unsettled_promise_are_bounded() -> Result<()> {
        for source in [
            "import {get} from 'nio.js'; get('/',()=>{for(;;) {}})",
            "import {get} from 'nio.js'; get('/',()=>new Promise(()=>{}))",
        ] {
            let (_dir, mut e) = fixture(source, 50)?;
            let start = Instant::now();
            assert!(e.dispatch(0, input()).is_err());
            assert!(start.elapsed() < Duration::from_secs(2));
        }
        Ok(())
    }
    #[test]
    fn diagnostics_map_back_to_typescript_and_heap_is_bounded() -> Result<()> {
        let source = "import {get} from 'nio.js';\ninterface Removed { n:number }\n\n\nget('/',()=> {\n  throw new Error('mapped failure');\n});";
        let (_dir, mut e) = fixture(source, 1000)?;
        let error = e.dispatch(0, input()).unwrap_err().to_string();
        assert!(error.contains("nio-src:///main.ts:6:"), "{error}");
        let (_dir, mut e) = fixture(
            "import {get} from 'nio.js'; get('/',()=>Array(10000000).fill('x'))",
            1000,
        )?;
        assert!(e.dispatch(0, input()).is_err());
        Ok(())
    }
    #[test]
    fn outbound_network_is_denied() -> Result<()> {
        let (_dir, mut e) = fixture(
            "import {get} from 'nio.js'; get('/',async()=>{try{await fetch('https://example.com')}catch(e){return e.message}})",
            1000,
        )?;
        assert!(text(e.dispatch(0, input())?).contains("permission denied"));
        Ok(())
    }
    #[test]
    fn closed_registration_and_invalid_return() -> Result<()> {
        let (_dir, mut e) = fixture(
            "import {get} from 'nio.js'; get('/',()=>{get('/late','late');return 'bad'})",
            1000,
        )?;
        assert!(e.dispatch(0, input()).is_err());
        let (_dir, mut e) = fixture("import {get} from 'nio.js'; get('/',()=>undefined)", 1000)?;
        assert!(e.dispatch(0, input()).is_err());
        Ok(())
    }
    #[test]
    fn native_cpu_execution() -> Result<()> {
        let (_dir, mut e) = fixture(
            "import {get,native} from 'nio.js'; const cpu = native('cpu'); get('/cpu', () => cpu());",
            1000,
        )?;
        let r = e.dispatch(0, input())?;
        assert_eq!(text(r), "289420874");
        Ok(())
    }
    #[test]
    #[cfg(feature = "python")]
    fn python_invocation_from_javascript() -> Result<()> {
        crate::python::init_python();
        let (_dir, mut e) = fixture(
            "import {get,pythonEval} from 'nio.js'; get('/py', () => pythonEval('100 + 23'));",
            1000,
        )?;
        let r = e.dispatch(0, input())?;
        assert_eq!(text(r), "123");
        Ok(())
    }
}
