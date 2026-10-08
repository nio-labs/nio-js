import sys

with open("src/prepare.rs", "r") as f:
    lines = f.readlines()

for i, line in enumerate(lines):
    if 'if path.ends_with(".py") {' in line:
        insert_idx = i
        break

zig_block = """    if path.ends_with(".zig") {
        let out_dir = std::env::temp_dir();
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let src_path = out_dir.join(format!("plugin_{timestamp}.zig"));
        let so_path = out_dir.join(format!("plugin_{timestamp}.so"));
        
        std::fs::write(&src_path, source).expect("Failed to write zig file");
        
        let mut child = std::process::Command::new("zig")
            .arg("build-lib")
            .arg("-dynamic")
            .arg("-O").arg("ReleaseFast")
            .arg(format!("-femit-bin={}", so_path.display()))
            .arg(&src_path)
            .spawn()
            .expect("Failed to spawn zig compiler. Is 'zig' installed?");
            
        child.wait().unwrap();
        
        let bytes = std::fs::read(&so_path).expect("Failed to read compiled zig library");
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let b64 = STANDARD.encode(&bytes);
        
        let mut js_wrapper = String::from("");
        js_wrapper.push_str(&format!("const __plugin = globalThis.__loadRust('{}');\\n", b64));
        
        for line in source.lines() {
            if line.contains("export fn") {
                let parts: Vec<&str> = line.split("fn").nth(1).unwrap().split('(').collect();
                let fn_name = parts[0].trim();
                js_wrapper.push_str(&format!("export const {} = __plugin.get('{}');\\n", fn_name, fn_name));
            }
        }
        return compile_inner(name, &js_wrapper, SourceType::mjs());
    }
"""

lines.insert(insert_idx, zig_block)

with open("src/prepare.rs", "w") as f:
    f.writelines(lines)
print("Added zig support to prepare.rs")
