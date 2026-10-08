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
    launch_binary(
        std::path::Path::new(env!("CARGO_BIN_EXE_nio-js")),
        &["run", file.to_str().unwrap()],
        extra,
    )
}
fn launch_binary(binary: &std::path::Path, prefix: &[&str], extra: &[&str]) -> Service {
    let socket = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = socket.local_addr().unwrap().port();
    drop(socket);
    let child = Command::new(binary)
        .args(prefix)
        .args([
            "--port",
            &port.to_string(),
            "--timeout-ms",
            "1000",
            "--max-body",
            "1024",
        ])
        .args(extra)
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
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
    fs::remove_file(&source).unwrap();
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
    let mut artifact = fs::read(&capsule).unwrap();
    artifact[0] ^= 1;
    fs::write(&capsule, artifact).unwrap();
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
    fs::remove_file(&source).unwrap();
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
    fs::write(
        &source,
        "import {get} from 'nio.js'; get('/', 'capabilities');",
    )
    .unwrap();
    assert!(
        Command::new(env!("CARGO_BIN_EXE_nio-js"))
            .args([
                "build",
                source.to_str().unwrap(),
                "--frozen",
                "--require-net",
                "example.com",
                "-o",
                capsule.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
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

#[test]
fn standalone_runs_without_sources_or_capsule() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("main.ts");
    let capsule = dir.path().join("main.njs");
    let executable = dir.path().join(if cfg!(windows) {
        "service.exe"
    } else {
        "service"
    });
    fs::write(
        &source,
        "import {get} from 'nio.js'; get('/', () => 'embedded');",
    )
    .unwrap();
    assert!(
        Command::new(env!("CARGO_BIN_EXE_nio-js"))
            .args([
                "build",
                source.to_str().unwrap(),
                "-o",
                capsule.to_str().unwrap(),
                "--standalone",
                executable.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    fs::remove_file(&source).unwrap();
    fs::remove_file(capsule).unwrap();
    fs::remove_file(dir.path().join("nio.lock")).unwrap();
    let service = launch_binary(&executable, &[], &["--workers", "1"]);
    assert_eq!(
        Client::new()
            .get(format!("{}/", service.url))
            .send()
            .unwrap()
            .text()
            .unwrap(),
        "embedded"
    );
}

#[test]
#[ignore = "requires C, Rust, Go and Zig compilers on PATH"]
fn native_language_capsule_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("math.c"),
        "double add(double a, double b) { return a + b; }",
    )
    .unwrap();
    fs::write(
        dir.path().join("math.rs"),
        "#[unsafe(no_mangle)]\npub extern \"C\" fn sum(a: f64, b: f64, c: f64) -> f64 { a+b+c }",
    )
    .unwrap();
    fs::write(
        dir.path().join("math.zig"),
        "export fn multiply(a: f64, b: f64) f64 { return a*b; }",
    )
    .unwrap();
    fs::write(
        dir.path().join("text.go"),
        r#"package main
/*
#include <stdlib.h>
*/
import "C"
import "unsafe"
//export Text
func Text() *C.char { return C.CString("owned-go-string") }
//export FreeCString
func FreeCString(value *C.char) { C.free(unsafe.Pointer(value)) }
func main() {}
"#,
    )
    .unwrap();
    let source = dir.path().join("server.ts");
    let capsule = dir.path().join("server.njs");
    fs::write(
        &source,
        r#"
import {get} from 'nio.js';
import {add} from './math.c';
import {sum} from './math.rs';
import {multiply} from './math.zig';
import {Text} from './text.go';
get('/', () => [add(1.5, 2.5), sum(1,2,3), multiply(2,3.5), Text()]);
get('/invalid', () => add(1));
"#,
    )
    .unwrap();
    assert!(
        Command::new(env!("CARGO_BIN_EXE_nio-js"))
            .args([
                "build",
                source.to_str().unwrap(),
                "-o",
                capsule.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    for name in [
        "server.ts",
        "math.c",
        "math.rs",
        "math.zig",
        "text.go",
        "nio.lock",
    ] {
        fs::remove_file(dir.path().join(name)).unwrap();
    }
    let service = launch(&capsule, &["--offline", "--workers", "1"]);
    let client = Client::new();
    for _ in 0..10 {
        assert_eq!(
            client
                .get(format!("{}/", service.url))
                .send()
                .unwrap()
                .text()
                .map(|body| serde_json::from_str::<Value>(&body).unwrap())
                .unwrap(),
            json!([4, 6, 7, "owned-go-string"])
        );
    }
    assert_eq!(
        client
            .get(format!("{}/invalid", service.url))
            .send()
            .unwrap()
            .status(),
        500
    );
}

#[test]
fn signing_cli_requires_the_expected_publisher() {
    let dir = tempfile::tempdir().unwrap();
    let key = dir.path().join("publisher.key");
    let public = key.with_extension("pub");
    let source = dir.path().join("main.ts");
    let capsule = dir.path().join("main.njs");
    let binary = env!("CARGO_BIN_EXE_nio-js");
    assert!(
        Command::new(binary)
            .args([
                "keys",
                "gen",
                "--email",
                "test@example.com",
                "--output",
                key.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    let secret = fs::read(&key).unwrap();
    assert!(
        !Command::new(binary)
            .args([
                "keys",
                "gen",
                "--email",
                "test@example.com",
                "--output",
                key.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(fs::read(&key).unwrap(), secret);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&key).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    fs::write(&source, "export const value = 42;").unwrap();
    assert!(
        Command::new(binary)
            .args([
                "build",
                source.to_str().unwrap(),
                "-o",
                capsule.to_str().unwrap(),
                "--sign-key",
                key.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new(binary)
            .args([
                "verify",
                capsule.to_str().unwrap(),
                "--public-key",
                public.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        Command::new(binary)
            .args([
                "exec",
                capsule.to_str().unwrap(),
                "--public-key",
                public.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    fs::write(&public, [0; 32]).unwrap();
    assert!(
        !Command::new(binary)
            .args([
                "verify",
                capsule.to_str().unwrap(),
                "--public-key",
                public.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        !Command::new(binary)
            .args([
                "exec",
                capsule.to_str().unwrap(),
                "--public-key",
                public.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
}

#[test]
#[ignore = "requires Go and a C compiler on PATH"]
fn native_cloud_worker_example() {
    let dir = tempfile::tempdir().unwrap();
    for (name, contents) in [
        (
            "server.ts",
            include_str!("../examples/hybrid-cloud-worker/server.ts"),
        ),
        (
            "network.go",
            include_str!("../examples/hybrid-cloud-worker/network.go"),
        ),
        (
            "legacy_parser.c",
            include_str!("../examples/hybrid-cloud-worker/legacy_parser.c"),
        ),
    ] {
        fs::write(dir.path().join(name), contents).unwrap();
    }
    let capsule = dir.path().join("app.njs");
    assert!(
        Command::new(env!("CARGO_BIN_EXE_nio-js"))
            .args([
                "build",
                dir.path().join("server.ts").to_str().unwrap(),
                "-o",
                capsule.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    let service = launch(&capsule, &["--workers", "1"]);
    let body = Client::new()
        .get(format!("{}/api/iot/cluster-metrics", service.url))
        .send()
        .unwrap()
        .text()
        .unwrap();
    let data: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(data["infrastructure"]["active_nodes"], 5);
    assert_eq!(data["hardware_telemetry"]["normalized_value"], 14.2604);
}

#[cfg(feature = "python")]
#[test]
#[ignore = "requires Zig and Python on PATH"]
fn native_fraud_pipeline_example() {
    let dir = tempfile::tempdir().unwrap();
    for (name, contents) in [
        (
            "server.ts",
            include_str!("../examples/fraud-detection-pipeline/server.ts"),
        ),
        (
            "crypto.zig",
            include_str!("../examples/fraud-detection-pipeline/crypto.zig"),
        ),
        (
            "ml_model.py",
            include_str!("../examples/fraud-detection-pipeline/ml_model.py"),
        ),
    ] {
        fs::write(dir.path().join(name), contents).unwrap();
    }
    let capsule = dir.path().join("app.njs");
    assert!(
        Command::new(env!("CARGO_BIN_EXE_nio-js"))
            .args([
                "build",
                dir.path().join("server.ts").to_str().unwrap(),
                "-o",
                capsule.to_str().unwrap()
            ])
            .status()
            .unwrap()
            .success()
    );
    let service = launch(&capsule, &["--workers", "1"]);
    let client = Client::new();
    let url = format!("{}/api/fraud/analyze", service.url);
    for body in [
        "null",
        "{",
        r#"{"amount":-1,"user_id_hash":0,"account_age_days":0}"#,
        r#"{"amount":"100","user_id_hash":1,"account_age_days":3}"#,
    ] {
        assert_eq!(client.post(&url).body(body).send().unwrap().status(), 400);
    }
    for body in [
        r#"{"amount":15000.5,"user_id_hash":892341,"account_age_days":14}"#,
        r#"{"amount":0,"user_id_hash":0,"account_age_days":0}"#,
    ] {
        let response = client.post(&url).body(body).send().unwrap();
        assert_eq!(response.status(), 200);
        let value: Value = serde_json::from_str(&response.text().unwrap()).unwrap();
        assert!(value["signals"]["history_flags"].as_u64().unwrap() < 10);
        assert!(value["risk_score"].as_f64().unwrap().is_finite());
    }
}
