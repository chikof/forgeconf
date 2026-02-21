//! CLI argument override example.
//!
//! This example shows how to override configuration values with command-line
//! arguments.
//!
//! Run with: cargo run --example cli_override --features cli -- --port=9000
//! --debug=true

#[cfg(feature = "cli")]
use forgeconf::{CliArguments, forgeconf};

#[cfg(feature = "cli")]
#[derive(Debug)]
#[forgeconf(config(path = "examples/fixtures/basic.toml"))]
struct AppConfig {
    host: String,
    port: u16,
    debug: bool,
}

#[cfg(feature = "cli")]
fn main() {
    println!("=== CLI Override Example ===\n");

    // Load config with CLI overrides
    // CLI arguments have highest priority (255) by default
    match AppConfig::loader()
        .with_config()
        .add_source(CliArguments::new().with_args(std::env::args().skip(1)))
        .load()
    {
        Ok(config) => {
            println!("✓ Configuration loaded with CLI overrides!");
            println!("\nFinal configuration:");
            println!("  Host:  {}", config.host);
            println!("  Port:  {}", config.port);
            println!("  Debug: {}", config.debug);
            println!("\nTry running with:");
            println!(
                "  cargo run --example cli_override --features cli -- --port=9000 --debug=true"
            );
        },
        Err(e) => {
            eprintln!("✗ Failed to load configuration:");
            eprintln!("{}", e);
            std::process::exit(1);
        },
    }
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("This example requires the 'cli' feature.");
    eprintln!("Run with: cargo run --example cli_override --features cli");
    std::process::exit(1);
}
