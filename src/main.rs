mod engine;
mod init;
mod mcp;
mod native;
mod network;
mod prepare;
pub mod python;
mod server;
mod task;

use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

#[derive(Parser)]
#[command(
    version,
    about = "QuickJS services with URL modules and portable .njs capsules"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Task {
        name: String,
    },
    Init {
        kind: Option<String>,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(short, long)]
        framework: Option<String>,
        #[arg(short, long)]
        db: Option<String>,
        #[arg(short, long)]
        ai: Option<String>,
    },
    Exec {
        file: PathBuf,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        #[command(flatten)]
        preparation: Preparation,
        #[arg(long, value_delimiter = ',')]
        allow_net: Vec<String>,
        #[arg(long)]
        allow_private_network: bool,
        #[arg(long, default_value = "64")]
        memory_mb: usize,
        #[arg(long, default_value = "30000")]
        timeout_ms: u64,
    },
    Run {
        file: PathBuf,
        #[arg(long, default_value = "3000")]
        port: u16,
        #[arg(long, default_value = "127.0.0.1")]
        host: std::net::IpAddr,
        #[command(flatten)]
        preparation: Preparation,
        #[arg(long, value_delimiter = ',')]
        allow_net: Vec<String>,
        #[arg(long)]
        allow_private_network: bool,
        #[arg(long, default_value = "64")]
        memory_mb: usize,
        #[arg(long, default_value = "1000")]
        timeout_ms: u64,
        #[arg(long, default_value = "1048576")]
        max_body: usize,
        #[arg(long, default_value = "0")]
        workers: usize,
    },
    Build {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[command(flatten)]
        preparation: Preparation,
        #[arg(long, value_delimiter = ',')]
        require_net: Vec<String>,
        #[arg(long, default_value = "text")]
        format: String,
    },
    Check {
        file: PathBuf,
        #[command(flatten)]
        preparation: Preparation,
        #[arg(long, default_value = "text")]
        format: String,
    },
    Mcp,
    Inspect {
        file: PathBuf,
    },
    Verify {
        file: PathBuf,
    },
}
#[derive(clap::Args)]
struct Preparation {
    #[arg(long)]
    frozen: bool,
    #[arg(long, conflicts_with = "frozen")]
    update: bool,
    #[arg(long)]
    offline: bool,
    #[arg(long, value_delimiter = ',')]
    allow_import: Vec<String>,
    #[arg(long)]
    import_map: Option<PathBuf>,
    #[arg(long)]
    cache: Option<PathBuf>,
    #[arg(long, value_name = "NAME=PATH")]
    asset: Vec<String>,
}
impl Preparation {
    fn options(&self, network: Vec<String>) -> Result<prepare::Options> {
        let mut o = prepare::Options {
            frozen: self.frozen,
            update: self.update,
            offline: self.offline,
            network,
            assets: self.asset.clone(),
            ..Default::default()
        };
        o.policy.hosts.extend(self.allow_import.clone());
        o.cache = self.cache.clone().unwrap_or_else(|| {
            std::env::var_os("NIO_JS_CACHE")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/nio-js"))
                })
                .unwrap_or(PathBuf::from(".nio-js-cache"))
        });
        if let Some(path) = &self.import_map {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Map {
                imports: BTreeMap<String, String>,
            }
            o.imports = serde_json::from_slice::<Map>(&std::fs::read(path)?)?.imports;
        }
        Ok(o)
    }
}
fn main() {
    if let Err(e) = main_result() {
        eprintln!("nio-js: {e:#}");
        std::process::exit(1);
    }
}
fn main_result() -> Result<()> {
    match Cli::parse().command {
        Command::Task { name } => {
            task::run_task(&name)?;
        }
        Command::Init {
            kind,
            name,
            framework,
            db,
            ai,
        } => {
            init::init_project(&kind, name, framework, db, ai)?;
        }
        Command::Build {
            file,
            output,
            preparation,
            require_net,
            format,
        } => {
            ensure!(
                output.extension().is_some_and(|s| s == "njs"),
                "output must have .njs extension"
            );
            let c = prepare::prepare(&file, &preparation.options(require_net)?)?;
            prepare::atomic_write(&output, &serde_json::to_vec_pretty(&c)?)?;
            if format == "agent-json" {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "ok",
                        "output": output.display().to_string(),
                        "modules_count": c.modules.len(),
                        "entry": c.entry
                    }))?
                );
            } else {
                eprintln!("Built {} ({} modules)", output.display(), c.modules.len());
            }
        }
        Command::Check {
            file,
            preparation,
            format,
        } => {
            let options = preparation.options(vec![])?;
            match prepare::prepare(&file, &options) {
                Ok(capsule) => {
                    if format == "agent-json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "status": "ok",
                                "file": file.display().to_string(),
                                "entry": capsule.entry,
                                "modules_count": capsule.modules.len(),
                                "network": capsule.network,
                                "diagnostics": []
                            }))?
                        );
                    } else {
                        println!(
                            "✓ Verified {} ({} modules, valid)",
                            file.display(),
                            capsule.modules.len()
                        );
                    }
                }
                Err(e) => {
                    if format == "agent-json" {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "status": "error",
                                "file": file.display().to_string(),
                                "message": e.to_string(),
                                "diagnostics": [{
                                    "file": file.display().to_string(),
                                    "message": e.to_string(),
                                    "severity": "error"
                                }]
                            }))?
                        );
                        std::process::exit(1);
                    } else {
                        return Err(e);
                    }
                }
            }
        }
        Command::Mcp => {
            mcp::run_mcp_server()?;
        }
        Command::Inspect { file } => {
            let c = prepare::read_capsule(&file)?;
            println!(
                "{}",
                serde_json::to_string_pretty(
                    &serde_json::json!({"format":c.format,"runtime":c.runtime,"entry":c.entry,"modules":c.modules.keys().collect::<Vec<_>>(),"required_network":c.network,"assets":c.assets.keys().collect::<Vec<_>>(),"digest":prepare::hash(&std::fs::read(file)?)})
                )?
            );
        }
        Command::Verify { file } => {
            prepare::read_capsule(&file)?;
            println!("Verified {}", file.display());
        }
        Command::Exec {
            file,
            args: _args,
            preparation,
            allow_net,
            allow_private_network,
            memory_mb,
            timeout_ms,
        } => {
            ensure!(
                (8..=1024).contains(&memory_mb),
                "memory must be between 8 and 1024 MiB"
            );
            ensure!(
                (10..=60000).contains(&timeout_ms),
                "timeout must be between 10 and 60000 ms"
            );
            let c = if file.extension().is_some_and(|s| s == "njs") {
                prepare::read_capsule(&file)?
            } else {
                prepare::prepare(&file, &preparation.options(vec![])?)?
            };
            let limits = engine::Limits {
                memory: memory_mb * 1024 * 1024,
                timeout: Duration::from_millis(timeout_ms),
                body: 1048576,
                policy: network::Policy {
                    hosts: allow_net,
                    private: allow_private_network,
                },
                in_flight: Arc::new(AtomicUsize::new(0)),
            };
            let _ = engine::Engine::new(Arc::new(c), limits)?;
        }
        Command::Run {
            file,
            port,
            host,
            preparation,
            allow_net,
            allow_private_network,
            memory_mb,
            timeout_ms,
            max_body,
            workers,
        } => {
            ensure!(
                (8..=1024).contains(&memory_mb),
                "memory must be between 8 and 1024 MiB"
            );
            ensure!(
                (10..=30000).contains(&timeout_ms),
                "timeout must be between 10 and 30000 ms"
            );
            ensure!(
                (1024..=16 * 1024 * 1024).contains(&max_body),
                "body limit must be between 1 KiB and 16 MiB"
            );
            ensure!(workers <= 64, "workers must not exceed 64");
            let c = if file.extension().is_some_and(|s| s == "njs") {
                prepare::read_capsule(&file)?
            } else {
                prepare::prepare(&file, &preparation.options(vec![])?)?
            };
            let limits = engine::Limits {
                memory: memory_mb * 1024 * 1024,
                timeout: Duration::from_millis(timeout_ms),
                body: max_body,
                policy: network::Policy {
                    hosts: allow_net,
                    private: allow_private_network,
                },
                in_flight: Arc::new(AtomicUsize::new(0)),
            };
            let worker_count = if workers > 0 {
                workers
            } else {
                detect_physical_cores()
            };
            let tokio_workers = worker_count.clamp(2, 4);
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(tokio_workers)
                .enable_all()
                .build()?;
            runtime
                .block_on(server::serve(
                    Arc::new(c),
                    limits,
                    std::net::SocketAddr::new(host, port),
                    worker_count,
                ))
                .context("service failed")?;
        }
    }
    Ok(())
}

fn detect_physical_cores() -> usize {
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/sys/devices/system/cpu") {
            let mut unique_cores = std::collections::HashSet::new();
            for entry in entries.flatten() {
                let name = entry.file_name();
                let s = name.to_string_lossy();
                if s.starts_with("cpu") && s[3..].chars().all(|c| c.is_ascii_digit()) {
                    let core_id_path = entry.path().join("topology/core_id");
                    let pkg_id_path = entry.path().join("topology/physical_package_id");
                    if let (Ok(core_id), Ok(pkg_id)) = (
                        std::fs::read_to_string(&core_id_path),
                        std::fs::read_to_string(&pkg_id_path),
                    ) {
                        unique_cores
                            .insert((pkg_id.trim().to_string(), core_id.trim().to_string()));
                    }
                }
            }
            if !unique_cores.is_empty() {
                return unique_cores.len();
            }
        }
    }
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
}
