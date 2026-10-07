use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Deserialize, Debug)]
struct NioToml {
    tasks: Option<HashMap<String, String>>,
}

pub fn run_task(name: &str) -> Result<()> {
    let toml_path = Path::new("nio.toml");
    if !toml_path.exists() {
        bail!("nio.toml not found in the current directory.");
    }

    let toml_content = fs::read_to_string(toml_path).context("Failed to read nio.toml")?;
    let config: NioToml = toml::from_str(&toml_content).context("Failed to parse nio.toml")?;

    let tasks = config.tasks.unwrap_or_default();
    let script = tasks.get(name).with_context(|| format!("Task '{}' not found in nio.toml", name))?;

    if Path::new("package.json").exists() && !Path::new("node_modules").exists() {
        bail!("node_modules not found. Please run 'npm install' before running tasks.");
    }

    println!("> nio-js task {}", name);
    
    let status = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", script])
            .status()
            .context("Failed to execute command")?
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(script)
            .status()
            .context("Failed to execute command")?
    };

    if !status.success() {
        bail!("Task '{}' failed with exit code: {}", name, status.code().unwrap_or(-1));
    }

    Ok(())
}
