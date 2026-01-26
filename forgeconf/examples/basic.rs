//! Basic configuration loading example.
//!
//! This example demonstrates the simplest way to load configuration from a TOML
//! file.
//!
//! Run with: cargo run --example basic

use forgeconf::forgeconf;

#[derive(Debug)]
#[forgeconf(config(path = "examples/fixtures/basic.toml"))]
struct AppConfig {
    /// Server host address
    host: String,
    /// Server port number
    port: u16,
    /// Enable debug mode
    debug: bool,
}

fn main() -> Result<(), forgeconf::ConfigError> {
    println!("=== Basic Configuration Loading Example ===\n");

    // Load the configuration
    let config = AppConfig::loader()
        .with_config()
        .load()?;

    println!("✓ Configuration loaded successfully!");
    println!("\nConfiguration values:");
    println!("  Host:  {}", config.host);
    println!("  Port:  {}", config.port);
    println!("  Debug: {}", config.debug);

    Ok(())
}
