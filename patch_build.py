import sys

with open("src/main.rs", "r") as f:
    content = f.read()

target = """            prepare::atomic_write(&output, &serde_json::to_vec_pretty(&c)?)?;
            if format == "agent-json" {"""

replacement = """            prepare::atomic_write(&output, &serde_json::to_vec_pretty(&c)?)?;
            
            // Phase 3: The Standalone Compiler (Meta-Runtime Embedding)
            if std::path::Path::new("host").exists() {
                println!("Detected 'host/' directory. Embedding capsule into standalone native host...");
                let capsule_name = output.file_name().unwrap();
                let dest = std::path::Path::new("host").join(capsule_name);
                std::fs::copy(&output, &dest)?;
                
                if std::path::Path::new("host/Cargo.toml").exists() {
                    println!("Compiling Rust host...");
                    let status = std::process::Command::new("cargo")
                        .args(["build", "--release"])
                        .current_dir("host")
                        .status()?;
                    if status.success() {
                        let bin_name = if cfg!(windows) { "nio-custom-host.exe" } else { "nio-custom-host" };
                        let final_out = if cfg!(windows) { "my-app.exe" } else { "my-app" };
                        std::fs::copy(format!("host/target/release/{}", bin_name), final_out).unwrap_or(0);
                        println!("✨ Successfully compiled standalone Rust executable: ./{}", final_out);
                    }
                } else if std::path::Path::new("host/build.zig").exists() {
                    println!("Compiling Zig host...");
                    let status = std::process::Command::new("zig")
                        .args(["build", "-Doptimize=ReleaseFast"])
                        .current_dir("host")
                        .status()?;
                    if status.success() {
                        let bin_name = if cfg!(windows) { "host.exe" } else { "host" };
                        let final_out = if cfg!(windows) { "my-app.exe" } else { "my-app" };
                        std::fs::copy(format!("host/zig-out/bin/{}", bin_name), final_out).unwrap_or(0);
                        println!("✨ Successfully compiled standalone Zig executable: ./{}", final_out);
                    }
                }
            }

            if format == "agent-json" {"""

if target in content:
    content = content.replace(target, replacement)
    with open("src/main.rs", "w") as f:
        f.write(content)
    print("Patched src/main.rs successfully!")
else:
    print("Target not found in src/main.rs!")
