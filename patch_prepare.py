with open("src/prepare.rs", "r") as f:
    content = f.read()

python_if = '    if path.ends_with(".py") {'

rust_if = """    if path.ends_with(".rs") {
        use std::io::Write;
        let out_dir = std::env::temp_dir();
        let so_path = out_dir.join(format!("plugin_{}.so", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let mut child = std::process::Command::new("rustc")
            .arg("--crate-type=cdylib")
            .arg("-O")
            .arg("-C").arg("opt-level=3")
            .arg("-")
            .arg("-o").arg(&so_path)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn rustc");
        
        child.stdin.as_mut().unwrap().write_all(source.as_bytes()).unwrap();
        child.wait().unwrap();
        
        let bytes = std::fs::read(&so_path).unwrap();
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let b64 = STANDARD.encode(&bytes);
        
        let mut js_wrapper = String::from("import { __loadRust } from 'nio.js';\n");
        js_wrapper.push_str(&format!("const __plugin = __loadRust('{}');\n", b64));
        
        for line in source.lines() {
            if line.contains("pub extern \\"C\\" fn") || line.contains("pub no_mangle extern \\"C\\" fn") || line.contains("#[no_mangle] pub extern \\"C\\" fn") {
                let parts: Vec<&str> = line.split("fn").nth(1).unwrap().split('(').collect();
                let fn_name = parts[0].trim();
                js_wrapper.push_str(&format!("export const {} = __plugin.get('{}');\n", fn_name, fn_name));
            }
        }
        return compile_inner(name, &js_wrapper, SourceType::mjs());
    }
"""

if rust_if not in content:
    content = content.replace(python_if, rust_if + python_if)
    with open("src/prepare.rs", "w") as f:
        f.write(content)
