use std::collections::BTreeMap;

use forgeconf::{Config, ConfigFile};

#[derive(Config)]
#[allow(dead_code, reason = "the schema mirrors the shared fixture")]
struct Server {
    host: String,
    port: u16,
    workers: u32,
}

#[derive(Config)]
#[allow(dead_code, reason = "the schema mirrors the shared fixture")]
struct Database {
    url: String,
    pool_size: u32,
    timeout_secs: u64,
}

#[derive(Config)]
#[config(env_prefix = "APP_")]
#[allow(dead_code, reason = "the schema mirrors the shared fixture")]
struct AppConfig {
    name: String,
    debug: bool,
    features: Vec<String>,
    server: Server,
    database: Database,
    limits: BTreeMap<String, u64>,
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_default();
    match AppConfig::loader().file(ConfigFile::new(path)).load() {
        Ok(cfg) => println!(
            "port={} pool={} limits={}",
            cfg.server.port,
            cfg.database.pool_size,
            cfg.limits.len()
        ),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        },
    }
}
