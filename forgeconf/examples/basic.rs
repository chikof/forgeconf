//! Basic configuration loading.
//!
//! Loads a TOML file declared on the struct, with defaults for missing keys and
//! an optional local override file.
//!
//! Run with: cargo run --example basic
#![cfg_attr(not(feature = "toml"), allow(dead_code))]

use forgeconf::Config;

#[derive(Debug, Config)]
#[config(
    file = "examples/fixtures/basic.toml",
    file(path = "examples/fixtures/basic.local.toml", optional)
)]
struct AppConfig {
    /// Server host address.
    host: String,
    /// Server port number.
    port: u16,
    /// Enable debug mode.
    debug: bool,
    /// Worker threads; absent from the file, so the default applies.
    #[config(default = 4)]
    workers: u8,
}

#[cfg(feature = "toml")]
fn main() -> Result<(), forgeconf::ConfigError> {
    let config = AppConfig::loader().load()?;

    println!("Configuration loaded:");
    println!("  host:    {}", config.host);
    println!("  port:    {}", config.port);
    println!("  debug:   {}", config.debug);
    println!("  workers: {}", config.workers);
    Ok(())
}

#[cfg(not(feature = "toml"))]
fn main() {
    eprintln!("this example needs the `toml` feature");
}
