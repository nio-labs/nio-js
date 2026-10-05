use reqwest::blocking::Client;
use serde_json::{Value, json};
use std::{
    fs,
    net::TcpListener,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
struct Service {
    child: Child,
    url: String,
}
impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn launch(file: &std::path::Path, extra: &[&str]) -> Service {
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    drop(socket);
    let child = Command::new(env!("CARGO_BIN_EXE_nio-js"))
        .args([
            "run",
            file.to_str().unwrap(),
            "--port",
            &port.to_string(),
            "--timeout-ms",
            "100",
            "--max-body",
            "1024",
        ])
        .args(extra)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut s = Service {
        child,
        url: format!("http://127.0.0.1:{port}"),
    };
    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_millis(100))
        .build()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if client.get(format!("{}/", s.url)).send().is_ok() {
            return s;
        }
        assert!(
            s.child.try_wait().unwrap().is_none(),
            "service exited before readiness"
        );
        assert!(Instant::now() < deadline, "service never became ready");
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn source_capsule_http_upload_and_failure_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("server.ts");
    fs::write(&source,r#"
import {get,post,reply} from 'nio.js';
get('/', 'Hello World');
get('/greeting', ({query,searchParams})=>({name:query.name,all:searchParams.getAll('name')}));
get('/users/:id', ({params})=>params);
get('/spin', ()=>{for(;;){}});
get('/fail', ()=>{throw new Error('private details')});
get('/network', async()=>{try{await fetch('https://example.com')}catch(e){return e.message}});
post('/echo', async({json})=>reply(await json(),{status:201,headers:{'x-custom':'yes'}}));
post('/upload',async({formData})=>{const f=(await formData()).get('file');return {name:f.name,size:f.size,text:await f.text()}});
"#).unwrap();
    let c = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let service = launch(&source, &[]);
    let r = c.get(format!("{}/", service.url)).send().unwrap();
    assert_eq!(r.status(), 200);
    assert!(r.headers().contains_key("x-request-id"));
    assert_eq!(r.text().unwrap(), "Hello World");
    let r = c
        .get(format!("{}/greeting?name=A&name=B", service.url))
        .send()
        .unwrap();
    let v: Value = serde_json::from_str(&r.text().unwrap()).unwrap();
    assert_eq!(v, json!({"name":"A","all":["A","B"]}));
    let r = c.get(format!("{}/users/42", service.url)).send().unwrap();
    assert_eq!(r.text().unwrap(), "{\"id\":\"42\"}");
    let r = c
        .post(format!("{}/echo", service.url))
        .header("content-type", "application/json")
        .body("{\"hello\":42}")
        .send()
        .unwrap();
    assert_eq!(r.status(), 201);
    assert_eq!(r.headers()["x-custom"], "yes");
    let r = c
        .post(format!("{}/echo", service.url))
        .body("x".repeat(2048))
        .send()
        .unwrap();
    assert_eq!(r.status(), 413);
    let body = "--TEST\r\nContent-Disposition: form-data; name=\"file\"; filename=\"hello.txt\"\r\nContent-Type: text/plain\r\n\r\nhéllo\r\n--TEST--\r\n";
    let r = c
        .post(format!("{}/upload", service.url))
        .header("content-type", "multipart/form-data; boundary=TEST")
        .body(body)
        .send()
        .unwrap();
    assert_eq!(r.status(), 200);
    let v: Value = serde_json::from_str(&r.text().unwrap()).unwrap();
    assert_eq!(v, json!({"name":"hello.txt","size":6,"text":"héllo"}));
    assert_eq!(
        c.post(format!("{}/", service.url)).send().unwrap().status(),
        405
    );
    assert_eq!(
        c.get(format!("{}/missing", service.url))
            .send()
            .unwrap()
            .status(),
        404
    );
    let r = c.get(format!("{}/network", service.url)).send().unwrap();
    assert!(r.text().unwrap().contains("permission denied"));
    let r = c.get(format!("{}/fail", service.url)).send().unwrap();
    assert_eq!(r.status(), 500);
    assert!(!r.text().unwrap().contains("private details"));
    assert_eq!(
        c.get(format!("{}/spin", service.url))
            .send()
            .unwrap()
            .status(),
        504
    );
    assert_eq!(
        c.post(format!("{}/echo", service.url))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .unwrap()
            .status(),
        201
    );
    drop(service);
    let capsule = dir.path().join("server.njs");
    assert!(
        Command::new(env!("CARGO_BIN_EXE_nio-js"))
            .args([
                "build",
                source.to_str().unwrap(),
                "--frozen",
                "-o",
                capsule.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    // Delete the entire source graph: the artifact must be sufficient for execution.
    fs::remove_file(source).unwrap();
    fs::remove_file(dir.path().join("nio.lock")).unwrap();
    let service = launch(&capsule, &["--offline"]);
    assert_eq!(
        c.get(format!("{}/", service.url))
            .send()
            .unwrap()
            .text()
            .unwrap(),
        "Hello World"
    );
    drop(service);
    let mut artifact: Value = serde_json::from_slice(&fs::read(&capsule).unwrap()).unwrap();
    artifact["modules"]["nio-src:///server.ts"]["code"] = json!("console.log('tampered')");
    fs::write(&capsule, serde_json::to_vec(&artifact).unwrap()).unwrap();
    assert!(
        !Command::new(env!("CARGO_BIN_EXE_nio-js"))
            .args(["verify", capsule.to_str().unwrap()])
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn packaged_assets_and_required_capabilities() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("assets.ts");
    let data = dir.path().join("hello.txt");
    let capsule = dir.path().join("assets.njs");
    fs::write(&source,"import {get,asset,redirect} from 'nio.js'; get('/',asset('hello.txt')); get('/go',redirect('https://example.com'));").unwrap();
    fs::write(&data, "hello asset").unwrap();
    assert!(
        Command::new(env!("CARGO_BIN_EXE_nio-js"))
            .args([
                "build",
                source.to_str().unwrap(),
                "--asset",
                &format!("hello.txt={}", data.display()),
                "-o",
                capsule.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    fs::remove_file(source).unwrap();
    fs::remove_file(data).unwrap();
    let service = launch(&capsule, &[]);
    let c = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    assert_eq!(
        c.get(format!("{}/", service.url))
            .send()
            .unwrap()
            .text()
            .unwrap(),
        "hello asset"
    );
    let r = c.get(format!("{}/go", service.url)).send().unwrap();
    assert_eq!(r.status(), 302);
    assert_eq!(r.headers()["location"], "https://example.com");
    drop(service);
    let mut artifact: Value = serde_json::from_slice(&fs::read(&capsule).unwrap()).unwrap();
    artifact["network"] = json!(["example.com"]);
    fs::write(&capsule, serde_json::to_vec(&artifact).unwrap()).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_nio-js"))
        .args(["run", capsule.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("ungranted"));
}

#[test]
#[ignore = "downloads actual esm.sh and UNPKG modules; requires network access"]
fn cdn_imports_execute_then_rebuild_offline() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("cdn.ts");
    let capsule = dir.path().join("cdn.njs");
    let cache = dir.path().join("cache");
    fs::write(&source,"import {get} from 'nio.js'; import {z} from 'https://esm.sh/zod@3.23.8?target=es2022'; import {z as raw} from 'https://unpkg.com/zod@3.23.8/lib/index.mjs'; get('/',()=>z.string().parse(raw.string().parse('Hello World')));").unwrap();
    for offline in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_nio-js"));
        command.args([
            "build",
            source.to_str().unwrap(),
            "--cache",
            cache.to_str().unwrap(),
            "-o",
            capsule.to_str().unwrap(),
        ]);
        if offline {
            command.args(["--offline", "--frozen"]);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let service = launch(&capsule, &[]);
        let c = Client::builder().no_proxy().build().unwrap();
        assert_eq!(
            c.get(format!("{}/", service.url))
                .send()
                .unwrap()
                .text()
                .unwrap(),
            "Hello World"
        );
    }
    let lock_path = dir.path().join("nio.lock");
    let mut lock: Value = serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    let first = lock["remote"].as_object().unwrap().values().next().unwrap()["integrity"]
        .as_str()
        .unwrap();
    fs::write(cache.join(first), "corrupted").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nio-js"))
        .args([
            "build",
            source.to_str().unwrap(),
            "--cache",
            cache.to_str().unwrap(),
            "--offline",
            "--frozen",
            "-o",
            capsule.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    // Digests must never be allowed to become cache paths.
    let first_key = lock["remote"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    lock["remote"][&first_key]["integrity"] = json!("../../other");
    fs::write(lock_path, serde_json::to_vec(&lock).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_nio-js"))
        .args([
            "build",
            source.to_str().unwrap(),
            "--offline",
            "--frozen",
            "-o",
            capsule.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid lock digest"));
}
