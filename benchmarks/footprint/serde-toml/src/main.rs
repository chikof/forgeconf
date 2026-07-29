use std::collections::BTreeMap;

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
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: fp-serde-toml <file>")?;
    let cfg: AppConfig = toml::from_str(&std::fs::read_to_string(path)?)?;
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
