//! Nested configuration structures example.
//!
//! This example shows how to work with nested configuration sections.
//!
//! Run with: cargo run --example nested

use forgeconf::forgeconf;

#[derive(Debug)]
#[forgeconf]
struct DatabaseConfig {
    host: String,
    port: u16,
    username: String,
    database: String,
}

#[derive(Debug)]
#[forgeconf]
struct LoggingConfig {
    level: String,
    format: String,
}

#[derive(Debug)]
#[forgeconf(config(path = "examples/fixtures/nested.toml"))]
struct AppConfig {
    app_name: String,
    #[field(name = "database")]
    database: DatabaseConfig,
    #[field(name = "logging")]
    logging: LoggingConfig,
}

fn main() {
    println!("=== Nested Configuration Example ===\n");

    match AppConfig::loader()
        .with_config()
        .load()
    {
        Ok(config) => {
            println!("✓ Configuration loaded successfully!");
            println!("\nApplication: {}", config.app_name);
            println!("\nDatabase:");
            println!(
                "  Host:     {}",
                config
                    .database
                    .host
            );
            println!(
                "  Port:     {}",
                config
                    .database
                    .port
            );
            println!(
                "  User:     {}",
                config
                    .database
                    .username
            );
            println!(
                "  Database: {}",
                config
                    .database
                    .database
            );
            println!("\nLogging:");
            println!(
                "  Level:  {}",
                config
                    .logging
                    .level
            );
            println!(
                "  Format: {}",
                config
                    .logging
                    .format
            );
        },
        Err(e) => {
            eprintln!("✗ Failed to load configuration:\n{:?}", e);
            std::process::exit(1);
        },
    }
}
