import re

with open("src/init.rs", "r") as f:
    content = f.read()

# Add host selection prompt logic to both init_monorepo and init_server
# Actually, I can just patch init_server for now as a prototype, or both.
# Let's patch init_server first.

target_server = """    let selected_ai = match ai_opt {
        Some(ai) => ai,
        None => {
            let ai_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Do you need Intelligence (AI)?")
                .default(0)
                .items(&ai_options[..])
                .interact()?;
            ai_options[ai_selection].to_string()
        }
    };"""

replacement_server = target_server + """

    let host_options = &["Rust + smol (Default)", "Zig + libuv", "Rust + Tokio"];
    let selected_host = {
        let host_selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select your Native Host Architecture")
            .default(0)
            .items(&host_options[..])
            .interact()?;
        host_options[host_selection].to_string()
    };
"""

content = content.replace(target_server, replacement_server)

target_server_scaffold = """    if selected_ai == "NioAI" {
        fs::create_dir_all(format!("{}/ai", name))?;
        fs::write(
            format!("{}/ai/agent.py", name),
            "def process_prompt(prompt):\\n    return f'AI Processed: {prompt}'\\n",
        )?;
        fs::write(
            format!("{}/ai/prompts.txt", name),
            "You are a helpful NioAI assistant.\\n",
        )?;
    }"""

replacement_server_scaffold = target_server_scaffold + """

    // Scaffold the Host
    fs::create_dir_all(format!("{}/host", name))?;
    if selected_host == "Zig + libuv" {
        fs::write(
            format!("{}/host/build.zig", name),
            "// Zig build script to compile libuv and QuickJS\\nconst std = @import(\\"std\\");\\npub fn build(b: *std.Build) void {}\\n",
        )?;
        fs::write(
            format!("{}/host/main.zig", name),
            "// NioJS-Zig Ultra-Light Bare Metal Host\\nconst std = @import(\\"std\\");\\n\\npub fn main() !void {\\n    std.debug.print(\\"Booting Zig + libuv engine...\\\\n\\", .{});\\n}\\n",
        )?;
    } else if selected_host == "Rust + smol (Default)" {
        fs::write(
            format!("{}/host/Cargo.toml", name),
            "[package]\\nname = \\"nio-custom-host\\"\\nversion = \\"1.0.0\\"\\nedition = \\"2021\\"\\n\\n[dependencies]\\nsmol = \\"2.0\\"\\nrquickjs = \\"0.6\\"\\n",
        )?;
        fs::write(
            format!("{}/host/main.rs", name),
            "// NioJS Diet-Rust Host\\nfn main() {\\n    println!(\\"Booting Rust + smol engine...\\");\\n    smol::block_on(async {\\n        // HTTP listener\\n    });\\n}\\n",
        )?;
    } else {
        fs::write(
            format!("{}/host/Cargo.toml", name),
            "[package]\\nname = \\"nio-custom-host\\"\\nversion = \\"1.0.0\\"\\nedition = \\"2021\\"\\n\\n[dependencies]\\ntokio = { version = \\"1.0\\", features = [\\"full\\"] }\\nhyper = \\"1.0\\"\\nrquickjs = \\"0.6\\"\\n",
        )?;
        fs::write(
            format!("{}/host/main.rs", name),
            "// NioJS Standard Rust Host\\n#[tokio::main]\\nasync fn main() {\\n    println!(\\"Booting Rust + Tokio engine...\\");\\n}\\n",
        )?;
    }
"""

content = content.replace(target_server_scaffold, replacement_server_scaffold)

with open("src/init.rs", "w") as f:
    f.write(content)

print("Patched init.rs successfully!")
