use std::collections::BTreeMap;

use confique::Config;

#[derive(Config)]
#[allow(dead_code, reason = "the schema mirrors the shared fixture")]
struct Server {
    host: String,
    #[config(env = "APP_SERVER_PORT")]
    port: u16,
    workers: u32,
}

#[derive(Config)]
#[allow(dead_code, reason = "the schema mirrors the shared fixture")]
struct Database {
    url: String,
    #[config(env = "APP_DATABASE_POOL_SIZE")]
    pool_size: u32,
    timeout_secs: u64,
}

#[derive(Config)]
#[allow(dead_code, reason = "the schema mirrors the shared fixture")]
struct AppConfig {
    name: String,
    debug: bool,
    features: Vec<String>,
    #[config(nested)]
    server: Server,
    #[config(nested)]
    database: Database,
    limits: BTreeMap<String, u64>,
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).ok_or("usage: fp-confique <file>")?;
    let cfg = AppConfig::builder().env().file(path).load()?;
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
