mod engine;
mod network;
mod prepare;
mod server;

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
    },
    Build {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[command(flatten)]
        preparation: Preparation,
        #[arg(long, value_delimiter = ',')]
        require_net: Vec<String>,
    },
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
        Command::Build {
            file,
            output,
            preparation,
            require_net,
        } => {
            ensure!(
                output.extension().is_some_and(|s| s == "njs"),
                "output must have .njs extension"
            );
            let c = prepare::prepare(&file, &preparation.options(require_net)?)?;
            prepare::atomic_write(&output, &serde_json::to_vec_pretty(&c)?)?;
            eprintln!("Built {} ({} modules)", output.display(), c.modules.len());
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
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()?;
            runtime
                .block_on(server::serve(
                    Arc::new(c),
                    limits,
                    std::net::SocketAddr::new(host, port),
                ))
                .context("service failed")?;
        }
    }
    Ok(())
}
