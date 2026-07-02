//! Nested configuration structures.
//!
//! Inner structs derive `Config` too; the table with the field's key becomes
//! the inner struct. `Option<Struct>` sections may be absent.
//!
//! Run with: cargo run --example nested
#![cfg_attr(not(feature = "toml"), allow(dead_code))]

use forgeconf::Config;

#[derive(Debug, Config)]
struct DatabaseConfig {
    host: String,
    #[config(default = 5432)]
    port: u16,
    username: String,
    database: String,
}

#[derive(Debug, Config)]
#[config(deny_unknown_fields)]
struct LoggingConfig {
    level: String,
    format: String,
}

#[derive(Debug, Config)]
struct CacheConfig {
    ttl_secs: u32,
}

#[derive(Debug, Config)]
#[config(file = "examples/fixtures/nested.toml")]
struct AppConfig {
    app_name: String,
    database: DatabaseConfig,
    logging: LoggingConfig,
    /// No `[cache]` table in the file, so this stays `None`.
    cache: Option<CacheConfig>,
}

#[cfg(feature = "toml")]
fn main() -> Result<(), forgeconf::ConfigError> {
    let config = AppConfig::loader().load()?;

    println!("Application: {}", config.app_name);
    println!("Database:");
    println!("  host:     {}", config.database.host);
    println!("  port:     {}", config.database.port);
    println!("  user:     {}", config.database.username);
    println!("  database: {}", config.database.database);
    println!("Logging:");
    println!("  level:  {}", config.logging.level);
    println!("  format: {}", config.logging.format);
    match config.cache {
        Some(cache) => println!("Cache ttl: {}s", cache.ttl_secs),
        None => println!("Cache: not configured"),
    }
    Ok(())
}

#[cfg(not(feature = "toml"))]
fn main() {
    eprintln!("this example needs the `toml` feature");
}
