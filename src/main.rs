mod engine;
mod ffi;
mod init;
mod mcp;
mod native;
mod network;
mod prepare;
pub mod python;
mod server;
mod standalone;
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
        #[arg(long)]
        host: Option<String>,
    },
    Exec {
        file: PathBuf,
        #[arg(long)]
        public_key: Option<PathBuf>,
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
        #[arg(long)]
        public_key: Option<PathBuf>,
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
        #[arg(long)]
        sign: Option<String>,
        #[arg(long, conflicts_with = "sign")]
        sign_key: Option<PathBuf>,
        #[arg(long)]
        standalone: Option<PathBuf>,
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
        #[arg(long)]
        public_key: Option<PathBuf>,
    },
    Keys {
        #[command(subcommand)]
        cmd: KeysCommand,
    },
}

#[derive(clap::Subcommand, Clone)]
enum KeysCommand {
    Gen {
        #[arg(long)]
        email: String,
        #[arg(short, long)]
        output: Option<PathBuf>,
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
    let embedded = standalone::embedded()?;
    let cli = if embedded.is_some() {
        let mut args = vec![
            std::env::args_os().next().unwrap_or_default(),
            "run".into(),
            "__embedded__.njs".into(),
        ];
        args.extend(std::env::args_os().skip(1));
        Cli::parse_from(args)
    } else {
        Cli::parse()
    };
    match cli.command {
        Command::Task { name } => {
            task::run_task(&name)?;
        }
        Command::Init {
            kind,
            name,
            framework,
            db,
            ai,
            host,
        } => {
            init::init_project(&kind, name, framework, db, ai, host)?;
        }
        Command::Build {
            file,
            output,
            preparation,
            require_net,
            sign,
            sign_key,
            standalone,
            format,
        } => {
            ensure!(
                output.extension().is_some_and(|s| s == "njs"),
                "output must have .njs extension"
            );
            let mut c = prepare::prepare(&file, &preparation.options(require_net)?)?;
            if let Some(identity) = sign {
                prepare::sign(&mut c, &std::fs::read(prepare::key_path(&identity)?)?)?;
            }
            if let Some(path) = sign_key {
                prepare::sign(&mut c, &std::fs::read(path)?)?;
            }
            prepare::atomic_write(&output, &prepare::capsule_bytes(&c)?)?;

            let standalone = standalone.or_else(|| {
                std::path::Path::new("host/nio.toml").exists().then(|| {
                    PathBuf::from(if cfg!(windows) {
                        "my-app.exe"
                    } else {
                        "my-app"
                    })
                })
            });
            if let Some(path) = standalone {
                ensure!(
                    path != output
                        && (!path.exists()
                            || std::fs::canonicalize(&path)? != std::fs::canonicalize(&output)?),
                    "standalone output must differ from capsule output"
                );
                standalone::build(&path, &c)?;
                if format != "agent-json" {
                    println!("Built standalone executable {}", path.display());
                }
            }

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
                    &serde_json::json!({"format":c.format,"runtime":c.runtime,"signed":c.signature.is_some(),"publisher_fingerprint":c.publisher.as_ref().map(|key| prepare::hash(key)),"entry":c.entry,"modules":c.modules.keys().collect::<Vec<_>>(),"required_network":c.network,"assets":c.assets.keys().collect::<Vec<_>>(),"digest":prepare::hash(&std::fs::read(file)?)})
                )?
            );
        }
        Command::Verify { file, public_key } => {
            let c = prepare::read_capsule(&file)?;
            if let Some(path) = public_key {
                prepare::verify_signature(&c, Some(&std::fs::read(path)?))?;
            }
            println!(
                "Verified {} ({})",
                file.display(),
                if c.signature.is_some() {
                    "signed"
                } else {
                    "unsigned"
                }
            );
        }
        Command::Keys { cmd } => match cmd {
            KeysCommand::Gen { email, output } => {
                use ed25519_dalek::SigningKey;
                use rand::RngCore;
                let mut bytes = [0u8; 32];
                rand::rngs::OsRng.fill_bytes(&mut bytes);
                let signing_key = SigningKey::from_bytes(&bytes);
                let path = output.unwrap_or(prepare::key_path(&email)?);
                ensure!(
                    path != path.with_extension("pub"),
                    "private key output must differ from .pub output"
                );
                if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent)?;
                }
                ensure!(
                    !path.with_extension("pub").exists(),
                    "public key output already exists"
                );
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                use std::io::Write;
                options.open(&path)?.write_all(&signing_key.to_bytes())?;
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path.with_extension("pub"))?
                    .write_all(&signing_key.verifying_key().to_bytes())?;
                println!("Generated key for {}", email);
            }
        },
        Command::Exec {
            file,
            public_key,
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
            let c = if file == std::path::Path::new("__embedded__.njs") && embedded.is_some() {
                embedded.clone().unwrap()
            } else if file.extension().is_some_and(|s| s == "njs") {
                prepare::read_capsule(&file)?
            } else {
                prepare::prepare(&file, &preparation.options(vec![])?)?
            };
            if let Some(path) = public_key {
                prepare::verify_signature(&c, Some(&std::fs::read(path)?))?;
            }
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
            public_key,
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
            let c = if file == std::path::Path::new("__embedded__.njs") && embedded.is_some() {
                embedded.clone().unwrap()
            } else if file.extension().is_some_and(|s| s == "njs") {
                prepare::read_capsule(&file)?
            } else {
                prepare::prepare(&file, &preparation.options(vec![])?)?
            };
            if let Some(path) = public_key {
                prepare::verify_signature(&c, Some(&std::fs::read(path)?))?;
            }
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
