use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::fs;

pub fn init_project(kind: &str) -> Result<()> {
    match kind {
        "web" => init_web()?,
        "app" => init_app()?,
        _ => anyhow::bail!("Unknown project type '{}'. Use 'web' or 'app'.", kind),
    }
    Ok(())
}

fn init_web() -> Result<()> {
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .default("my-web-app".into())
        .interact_text()?;

    let frameworks = &["Vanilla (Zero-build)", "Lit", "React", "Vue", "Eleventy"];
    let fw_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a Web Framework")
        .default(0)
        .items(&frameworks[..])
        .interact()?;
    let selected_fw = frameworks[fw_selection];
    
    let db_options = &["nio-db", "None"];
    let db_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you need a Database?")
        .default(0)
        .items(&db_options[..])
        .interact()?;
    let selected_db = db_options[db_selection];

    let ai_options = &["NioAI", "None"];
    let ai_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you need Intelligence (AI)?")
        .default(0)
        .items(&ai_options[..])
        .interact()?;
    let selected_ai = ai_options[ai_selection];

    println!("Initializing web project '{}' with {}, DB: {}, AI: {}...", name, selected_fw, selected_db, selected_ai);
    fs::create_dir_all(format!("{}/src", name))?;
    
    // Generate nio.toml
    let nio_toml = r#"# NioJS Configuration File
# This file defines native task runner scripts (replaces npm run scripts).
# Run tasks using `nio-js task <name>` (e.g., `nio-js task dev`)

[tasks]
dev = "nio-js run src/index.ts"
build = "nio-js build src/index.ts -o dist/app.njs"
start = "nio-js run dist/app.njs"
"#;
    fs::write(format!("{}/nio.toml", name), nio_toml)?;
    
    // Generate Dockerfile
    let dockerfile = r#"FROM ubuntu:24.04
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
RUN curl -fsSL https://nio.dev/install.sh | bash
COPY . /app
WORKDIR /app
CMD ["nio-js", "task", "start"]
"#;
    fs::write(format!("{}/Dockerfile", name), dockerfile)?;

    // Generate docker-compose.yml
    let docker_compose = r#"version: '3.8'
services:
  web:
    build: .
    ports:
      - "3000:3000"
"#;
    fs::write(format!("{}/docker-compose.yml", name), docker_compose)?;

    println!("Web project initialized successfully in ./{}", name);
    Ok(())
}

fn init_app() -> Result<()> {
    let name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Project name")
        .default("my-mobile-app".into())
        .interact_text()?;

    let frameworks = &["Vue", "React", "Svelte", "Vanilla"];
    let fw_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose a Framework for Capacitor")
        .default(0)
        .items(&frameworks[..])
        .interact()?;
    let selected_fw = frameworks[fw_selection];

    let db_options = &["nio-db", "None"];
    let db_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you need a Database?")
        .default(0)
        .items(&db_options[..])
        .interact()?;
    let selected_db = db_options[db_selection];

    let ai_options = &["NioAI", "None"];
    let ai_selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Do you need Intelligence (AI)?")
        .default(0)
        .items(&ai_options[..])
        .interact()?;
    let selected_ai = ai_options[ai_selection];

    println!("Initializing app project '{}' with {} (Capacitor), DB: {}, AI: {}...", name, selected_fw, selected_db, selected_ai);
    fs::create_dir_all(format!("{}/src", name))?;
    
    let (dev_cmd, build_cmd, deps, dev_deps) = match selected_fw {
        "Vue" => (
            "npx @vue/cli-service serve",
            "npx @vue/cli-service build",
            r#""vue": "^3.0.0""#,
            r#""@vue/cli-service": "~5.0.0", "@vue/compiler-sfc": "^3.0.0""#
        ),
        "React" => (
            "npx react-scripts start",
            "npx react-scripts build",
            r#""react": "^18.2.0", "react-dom": "^18.2.0""#,
            r#""react-scripts": "5.0.1""#
        ),
        "Svelte" => (
            "npx vite",
            "npx vite build",
            r#""svelte": "^4.0.0""#,
            r#""vite": "^4.0.0", "@sveltejs/vite-plugin-svelte": "^2.0.0""#
        ),
        _ => ( // Vanilla
            "npx vite",
            "npx vite build",
            r#""#,
            r#""vite": "^4.0.0""#
        ),
    };

    let nio_toml = format!(r#"# NioJS Configuration File
# This file defines native task runner scripts (replaces npm run scripts).
# Run tasks using `nio-js task <name>` (e.g., `nio-js task dev`)

[tasks]
dev = "{}"
build = "{} && npx cap copy"
sync = "npx cap sync"
"#, dev_cmd, build_cmd);
    fs::write(format!("{}/nio.toml", name), nio_toml)?;

    let package_json = format!(r#"{{
  "name": "{}",
  "version": "0.1.0",
  "private": true,
  "dependencies": {{
    "@capacitor/core": "latest"{}
  }},
  "devDependencies": {{
    "@capacitor/cli": "latest"{}
  }}
}}"#, name, 
    if deps.is_empty() { String::new() } else { format!(", {}", deps) },
    if dev_deps.is_empty() { String::new() } else { format!(", {}", dev_deps) }
    );
    fs::write(format!("{}/package.json", name), package_json)?;

    let dockerfile = r#"FROM node:20
WORKDIR /app
COPY . .
RUN npm install
CMD ["nio-js", "task", "dev"]
"#;
    fs::write(format!("{}/Dockerfile", name), dockerfile)?;

    println!("App project initialized successfully in ./{}", name);
    Ok(())
}
