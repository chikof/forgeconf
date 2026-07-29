use std::collections::BTreeMap;

use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Deserialize)]
#[allow(dead_code, reason = "the schema mirrors the shared fixture")]
struct Server {
    host: String,
    port: u16,
    workers: u32,
}

#[derive(Deserialize)]
#[allow(dead_code, reason = "the schema mirrors the shared fixture")]
struct Database {
    url: String,
    pool_size: u32,
    timeout_secs: u64,
}

#[derive(Deserialize)]
#[allow(dead_code, reason = "the schema mirrors the shared fixture")]
struct AppConfig {
    name: String,
    debug: bool,
    features: Vec<String>,
    server: Server,
    database: Database,
    limits: BTreeMap<String, u64>,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: fp-config <file>")?;
    let cfg: AppConfig = Config::builder()
        .add_source(File::with_name(&path))
        .add_source(
            Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?
        .try_deserialize()?;
    println!(
        "port={} pool={} limits={}",
        cfg.server.port,
        cfg.database.pool_size,
        cfg.limits.len()
    );
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
