//! Configuration validation example.
//!
//! This example demonstrates how to use built-in validators to ensure
//! configuration values meet specific constraints.
//!
//! Run with: cargo run --example validators --features validators

#[cfg(feature = "validators")]
use forgeconf::forgeconf;

#[cfg(feature = "validators")]
#[derive(Debug)]
#[forgeconf(config(path = "examples/fixtures/validators.toml"))]
#[allow(dead_code)]
struct ServerConfig {
    /// Port must be between 1024 and 65535
    #[field(validate = forgeconf::validators::range(1024, 65535))]
    port: u16,

    /// Host cannot be empty
    #[field(validate = forgeconf::validators::non_empty())]
    host: String,

    /// Workers must be at least 1
    #[field(validate = forgeconf::validators::range(1, 100))]
    workers: u8,

    /// Log level must be one of: debug, info, warn, error
    #[field(validate = forgeconf::validators::one_of(vec![
        "debug".to_string(),
        "info".to_string(),
        "warn".to_string(),
        "error".to_string()
    ]))]
    log_level: String,

    /// API key must be at least 32 characters
    #[field(validate = forgeconf::validators::min_len(32))]
    api_key: String,

    /// Optional tags list (max 10 items)
    #[field(validate = forgeconf::validators::max_len(10))]
    tags: Vec<String>,
}

#[cfg(feature = "validators")]
fn main() {
    println!("=== Configuration Validation Example ===\n");

    match ServerConfig::loader()
        .with_config()
        .load()
    {
        Ok(config) => {
            println!("✓ Configuration loaded and validated successfully!");
            println!("\nServer Configuration:");
            println!("  Port:      {}", config.port);
            println!("  Host:      {}", config.host);
            println!("  Workers:   {}", config.workers);
            println!("  Log Level: {}", config.log_level);
            println!("  API Key:   {}...", &config.api_key[..8]);
            println!("  Tags:      {:?}", config.tags);
        },
        Err(e) => {
            eprintln!("✗ Configuration validation failed:\n{:?}", e);
            eprintln!("\nThis error shows that validators are working!");
            eprintln!("Check the configuration file and fix the invalid values.");
            std::process::exit(1);
        },
    }
}

#[cfg(not(feature = "validators"))]
fn main() {
    eprintln!("This example requires the 'validators' feature.");
    eprintln!("Run with: cargo run --example validators --features validators");
    std::process::exit(1);
}
