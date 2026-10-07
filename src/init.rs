use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::fs;

pub fn init_project(
    kind_opt: &Option<String>,
    name_opt: Option<String>,
    fw_opt: Option<String>,
    db_opt: Option<String>,
    ai_opt: Option<String>,
) -> Result<()> {
    let kind = match kind_opt {
        Some(k) => k.clone(),
        None => {
            let options = &["web", "app"];
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("What kind of project do you want to create?")
                .default(0)
                .items(&options[..])
                .interact()?;
            options[selection].to_string()
        }
    };

    match kind.as_str() {
        "web" => init_monorepo("web", name_opt, fw_opt, db_opt, ai_opt)?,
        "app" => init_monorepo("app", name_opt, fw_opt, db_opt, ai_opt)?,
        _ => anyhow::bail!("Unknown project type '{}'. Use 'web' or 'app'.", kind),
    }
    Ok(())
}

fn init_monorepo(
    kind: &str,
    name_opt: Option<String>,
    fw_opt: Option<String>,
    db_opt: Option<String>,
    ai_opt: Option<String>,
) -> Result<()> {
    let name: String = match name_opt {
        Some(n) => n,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Project name")
            .default(if kind == "web" { "my-web-app".into() } else { "my-mobile-app".into() })
            .interact_text()?,
    };

    let frameworks = if kind == "web" {
        vec!["Vanilla (Zero-build)", "Lit", "React", "Vue", "Eleventy"]
    } else {
        vec!["Vue", "React", "Svelte", "Vanilla"]
    };

    let selected_fw = match fw_opt {
        Some(fw) => fw,
        None => {
            let fw_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(if kind == "web" { "Choose a Web Framework" } else { "Choose a Framework for Capacitor" })
                .default(0)
                .items(&frameworks[..])
                .interact()?;
            frameworks[fw_selection].to_string()
        }
    };
    
    let db_options = &["nio-db", "None"];
    let selected_db = match db_opt {
        Some(db) => db,
        None => {
            let db_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Do you need a Database?")
                .default(0)
                .items(&db_options[..])
                .interact()?;
            db_options[db_selection].to_string()
        }
    };

    let ai_options = &["NioAI", "None"];
    let selected_ai = match ai_opt {
        Some(ai) => ai,
        None => {
            let ai_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Do you need Intelligence (AI)?")
                .default(0)
                .items(&ai_options[..])
                .interact()?;
            ai_options[ai_selection].to_string()
        }
    };

    println!("Initializing monorepo project '{}'...", name);
    fs::create_dir_all(&name)?;

    // Generate root nio.toml (will be updated with UI tasks later)
    // Wait, we need to know the UI commands before generating it, so let's move this down.
    let server_ts = r#"// NioJS Backend Entrypoint
import { get, reply } from "nio.js";

get("/", () => reply("Hello from NioJS Backend!"));
"#;
    fs::write(format!("{}/server.ts", name), server_ts)?;

    // Generate DB Folder
    if selected_db == "nio-db" {
        fs::create_dir_all(format!("{}/db", name))?;
        fs::write(format!("{}/db/schema.sql", name), "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);\n")?;
        fs::write(format!("{}/db/setup.sh", name), "#!/bin/bash\necho 'Setting up nio-db...'\n")?;
    }

    // Generate AI Folder
    if selected_ai == "NioAI" {
        fs::create_dir_all(format!("{}/ai", name))?;
        fs::write(format!("{}/ai/agent.py", name), "def process_prompt(prompt):\n    return f'AI Processed: {prompt}'\n")?;
        fs::write(format!("{}/ai/prompts.txt", name), "You are a helpful NioAI assistant.\n")?;
    }

    // Generate Frontend Folder (web or app) using create-vite
    let frontend_dir = format!("{}/{}", name, kind);
    let template = match selected_fw.as_str() {
        "Vue" => "vue",
        "React" => "react",
        "Svelte" => "svelte",
        "Lit" => "lit",
        _ => "vanilla",
    };

    println!("Generating {} template using Vite...", selected_fw);
    let status = std::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "npx" })
        .args(if cfg!(target_os = "windows") {
            vec!["/C", "npx", "-y", "create-vite@latest", &frontend_dir, "--template", template]
        } else {
            vec!["-y", "create-vite@latest", &frontend_dir, "--template", template]
        })
        .status()?;

    if !status.success() {
        anyhow::bail!("Failed to generate template using create-vite");
    }

    if kind == "app" {
        println!("Installing Capacitor...");
        std::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "npm" })
            .args(if cfg!(target_os = "windows") { vec!["/C", "npm", "install", "@capacitor/core"] } else { vec!["install", "@capacitor/core"] })
            .current_dir(&frontend_dir)
            .status()?;
        
        std::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "npm" })
            .args(if cfg!(target_os = "windows") { vec!["/C", "npm", "install", "-D", "@capacitor/cli"] } else { vec!["install", "-D", "@capacitor/cli"] })
            .current_dir(&frontend_dir)
            .status()?;
            
        std::process::Command::new(if cfg!(target_os = "windows") { "cmd" } else { "npx" })
            .args(if cfg!(target_os = "windows") { vec!["/C", "npx", "-y", "cap", "init", &name, "com.example.app", "--web-dir", "dist"] } else { vec!["-y", "cap", "init", &name, "com.example.app", "--web-dir", "dist"] })
            .current_dir(&frontend_dir)
            .status()?;
    }

    // Generate root nio.toml
    let build_cmd = if kind == "app" { "npm run build && npx cap copy" } else { "npm run build" };
    let nio_toml = format!(r#"# NioJS Configuration File
# This file defines native task runner scripts (replaces npm run scripts).
# Run tasks using `nio-js task <name>` (e.g., `nio-js task dev`)

[tasks]
dev = "nio-js run server.ts"
build = "nio-js build server.ts -o dist/app.njs"
start = "nio-js run dist/app.njs"
dev_ui = "cd {} && npm run dev"
build_ui = "cd {} && {}"
serve = "nio-js task dev & nio-js task dev_ui & wait"
"#, kind, kind, build_cmd);
    fs::write(format!("{}/nio.toml", name), nio_toml)?;

    // Dockerfile at the root
    let dockerfile = r#"FROM ubuntu:24.04
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://nio.dev/install.sh | bash
COPY . /app
WORKDIR /app
CMD ["nio-js", "task", "start"]
"#;
    fs::write(format!("{}/Dockerfile", name), dockerfile)?;

    println!("\n✨ Monorepo project '{}' initialized successfully!\n", name);
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  cd {} && npm install && cd ..", kind);
    println!("  nio-js task serve\n");
    Ok(())
}
