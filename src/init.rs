use anyhow::Result;
use dialoguer::{Input, Select, theme::ColorfulTheme};
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
            let options = &["web", "app", "server"];
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
        "server" => init_server(name_opt, db_opt, ai_opt)?,
        _ => anyhow::bail!(
            "Unknown project type '{}'. Use 'web', 'app', or 'server'.",
            kind
        ),
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
            .default(if kind == "web" {
                "my-web-app".into()
            } else {
                "my-mobile-app".into()
            })
            .interact_text()?,
    };

    let frameworks = if kind == "web" {
        vec![
            "Blank",
            "Vanilla (Zero-build)",
            "Lit",
            "React",
            "Vue",
            "Eleventy",
        ]
    } else {
        vec!["Blank", "Vue", "React", "Svelte", "Vanilla"]
    };

    let selected_fw = match fw_opt {
        Some(fw) => fw,
        None => {
            let fw_selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt(if kind == "web" {
                    "Choose a Web Framework"
                } else {
                    "Choose a Framework for Capacitor"
                })
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
        fs::write(
            format!("{}/db/schema.sql", name),
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);\n",
        )?;
        fs::write(
            format!("{}/db/setup.sh", name),
            "#!/bin/bash\necho 'Setting up nio-db...'\n",
        )?;
    }

    // Generate AI Folder
    if selected_ai == "NioAI" {
        fs::create_dir_all(format!("{}/ai", name))?;
        fs::write(
            format!("{}/ai/agent.py", name),
            "def process_prompt(prompt):\n    return f'AI Processed: {prompt}'\n",
        )?;
        fs::write(
            format!("{}/ai/prompts.txt", name),
            "You are a helpful NioAI assistant.\n",
        )?;
    }

    // Generate root nio.toml early so it isn't skipped if the user cancels Vite
    let build_cmd = if kind == "app" {
        "npm run build && npx cap copy"
    } else {
        "npm run build"
    };
    let nio_toml = format!(
        r#"# NioJS Configuration File
# This file defines native task runner scripts (replaces npm run scripts).
# Run tasks using `nio-js task <name>` (e.g., `nio-js task dev`)

[tasks]
dev = "nio-js run server.ts"
build = "nio-js build server.ts -o dist/app.njs"
start = "nio-js run dist/app.njs"
dev_ui = "cd {} && npm run dev"
build_ui = "cd {} && {}"
serve = "nio-js task dev & nio-js task dev_ui & wait"
"#,
        kind, kind, build_cmd
    );
    fs::write(format!("{}/nio.toml", name), nio_toml)?;

    // Dockerfile at the root early
    let dockerfile = r#"FROM ubuntu:24.04
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://nio.dev/install.sh | bash
COPY . /app
WORKDIR /app
CMD ["nio-js", "task", "start"]
"#;
    fs::write(format!("{}/Dockerfile", name), dockerfile)?;

    // Generate Frontend Folder (web or app)
    let frontend_dir = format!("{}/{}", name, kind);

    if selected_fw == "Blank" {
        println!("Generating Blank template...");
        fs::create_dir_all(&frontend_dir)?;
        fs::create_dir_all(format!("{}/src", frontend_dir))?;

        let package_json = r#"{
  "name": "nio-js-blank",
  "version": "1.0.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview"
  },
  "devDependencies": {
    "vite": "^5.0.0"
  }
}"#;
        fs::write(format!("{}/package.json", frontend_dir), package_json)?;

        let index_html = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>NioJS App</title>
    <link rel="stylesheet" href="/src/style.css" />
  </head>
  <body>
    <div id="app">
      <img src="/src/logo.svg" alt="NioJS Logo" class="logo" />
      <h1>Welcome to NioJS</h1>
    </div>
    <script type="module" src="/src/main.js"></script>
  </body>
</html>"#;
        fs::write(format!("{}/index.html", frontend_dir), index_html)?;

        let main_js = "console.log('NioJS App Started!');\n";
        fs::write(format!("{}/src/main.js", frontend_dir), main_js)?;

        let style_css = r#"body {
  margin: 0;
  font-family: system-ui, -apple-system, sans-serif;
  padding: 2rem;
}
#app {
  /* No center in vertical */
  text-align: center;
}
.logo {
  height: 150px;
  width: auto;
  margin-bottom: 1rem;
}"#;
        fs::write(format!("{}/src/style.css", frontend_dir), style_css)?;

        // Use our actual logo
        let logo_content = r##"<svg width="2042" height="2042" viewBox="0 0 2042 2042" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="2042" height="2042" rx="460" fill="#008080"/>
<rect x="1262" y="1120" width="450" height="450" rx="225" fill="white"/>
</svg>"##;
        fs::write(format!("{}/src/logo.svg", frontend_dir), logo_content)?;
    } else {
        let template = match selected_fw.as_str() {
            "Vue" => "vue",
            "React" => "react",
            "Svelte" => "svelte",
            "Lit" => "lit",
            _ => "vanilla",
        };

        println!("Generating {} template using Vite...", selected_fw);
        let status = std::process::Command::new(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "npx"
        })
        .args(if cfg!(target_os = "windows") {
            vec![
                "/C",
                "npx",
                "-y",
                "create-vite@latest",
                &frontend_dir,
                "--template",
                template,
                "--no-interactive",
            ]
        } else {
            vec![
                "-y",
                "create-vite@latest",
                &frontend_dir,
                "--template",
                template,
                "--no-interactive",
            ]
        })
        .status()?;

        if !status.success() {
            println!(
                "Warning: Failed to generate template using create-vite. You may need to create the UI manually."
            );
        }
    }

    if kind == "app" {
        println!("Installing Capacitor...");
        let _ = std::process::Command::new(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "npm"
        })
        .args(if cfg!(target_os = "windows") {
            vec!["/C", "npm", "install", "@capacitor/core"]
        } else {
            vec!["install", "@capacitor/core"]
        })
        .current_dir(&frontend_dir)
        .status();

        let _ = std::process::Command::new(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "npm"
        })
        .args(if cfg!(target_os = "windows") {
            vec!["/C", "npm", "install", "-D", "@capacitor/cli"]
        } else {
            vec!["install", "-D", "@capacitor/cli"]
        })
        .current_dir(&frontend_dir)
        .status();

        let _ = std::process::Command::new(if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "npx"
        })
        .args(if cfg!(target_os = "windows") {
            vec![
                "/C",
                "npx",
                "-y",
                "cap",
                "init",
                &name,
                "com.example.app",
                "--web-dir",
                "dist",
            ]
        } else {
            vec![
                "-y",
                "cap",
                "init",
                &name,
                "com.example.app",
                "--web-dir",
                "dist",
            ]
        })
        .current_dir(&frontend_dir)
        .status();
    }

    println!(
        "\n✨ Monorepo project '{}' initialized successfully!\n",
        name
    );
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  cd {} && npm install && cd ..", kind);
    println!("  nio-js task serve\n");
    Ok(())
}

fn init_server(
    name_opt: Option<String>,
    db_opt: Option<String>,
    ai_opt: Option<String>,
) -> Result<()> {
    let name: String = match name_opt {
        Some(n) => n,
        None => Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Project name")
            .default("my-server".into())
            .interact_text()?,
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

    println!("Initializing server project '{}'...", name);
    fs::create_dir_all(&name)?;

    let server_ts = r#"// NioJS Backend Entrypoint
import { get, reply } from "nio.js";

get("/", () => reply("Hello from NioJS Backend!"));
"#;
    fs::write(format!("{}/server.ts", name), server_ts)?;

    if selected_db == "nio-db" {
        fs::create_dir_all(format!("{}/db", name))?;
        fs::write(
            format!("{}/db/schema.sql", name),
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);\n",
        )?;
        fs::write(
            format!("{}/db/setup.sh", name),
            "#!/bin/bash\necho 'Setting up nio-db...'\n",
        )?;
    }

    if selected_ai == "NioAI" {
        fs::create_dir_all(format!("{}/ai", name))?;
        fs::write(
            format!("{}/ai/agent.py", name),
            "def process_prompt(prompt):\n    return f'AI Processed: {prompt}'\n",
        )?;
        fs::write(
            format!("{}/ai/prompts.txt", name),
            "You are a helpful NioAI assistant.\n",
        )?;
    }

    let nio_toml = r#"# NioJS Configuration File
[tasks]
dev = "nio-js run server.ts"
build = "nio-js build server.ts -o dist/app.njs"
start = "nio-js run dist/app.njs"
"#;
    fs::write(format!("{}/nio.toml", name), nio_toml)?;

    let dockerfile = r#"FROM ubuntu:24.04
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://nio.dev/install.sh | bash
COPY . /app
WORKDIR /app
CMD ["nio-js", "task", "start"]
"#;
    fs::write(format!("{}/Dockerfile", name), dockerfile)?;

    println!("\n✨ Server project '{}' initialized successfully!\n", name);
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  nio-js task dev\n");
    Ok(())
}
