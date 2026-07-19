//! Configuration validation.
//!
//! Validators are plain callables taking `&T` and returning
//! `Result<(), ConfigError>`: closures, free functions or the helpers in
//! `forgeconf::validators`. A struct-level `validate` sees the whole value once
//! every field loaded.
//!
//! Run with: cargo run --example validators
#![cfg_attr(not(feature = "toml"), allow(dead_code))]

use forgeconf::{Config, ConfigError};

fn ensure_https(url: &str) -> Result<(), ConfigError> {
    if url.starts_with("https://") {
        Ok(())
    } else {
        Err(ConfigError::invalid("must start with https://"))
    }
}

fn no_debug_in_production(config: &ServerConfig) -> Result<(), ConfigError> {
    if config.log_level == "debug" && config.tags.iter().any(|tag| tag == "production") {
        Err(ConfigError::invalid("debug logging is not allowed in production"))
    } else {
        Ok(())
    }
}

#[derive(Debug, Config)]
#[config(file = "examples/fixtures/validators.toml", validate = no_debug_in_production)]
struct ServerConfig {
    /// Port must be between 1024 and 65535.
    #[config(validate = forgeconf::validators::range(1024..=65535))]
    port: u16,

    /// Host cannot be empty.
    #[config(validate = forgeconf::validators::non_empty())]
    host: String,

    /// Workers must be between 1 and 100.
    #[config(validate = forgeconf::validators::range(1..=100))]
    workers: u8,

    /// Log level must be one of the listed names.
    #[config(validate = forgeconf::validators::one_of(["debug", "info", "warn", "error"].map(String::from)))]
    log_level: String,

    /// API key must be at least 32 characters.
    #[config(validate = forgeconf::validators::len(32..))]
    api_key: String,

    /// At most 10 tags.
    #[config(validate = forgeconf::validators::len(..=10))]
    tags: Vec<String>,

    /// Defaults to a valid URL; a custom function checks whatever ends up here.
    #[config(default = "https://example.com/hook", validate = ensure_https)]
    webhook: String,
}

#[cfg(feature = "toml")]
#[allow(clippy::unwrap_used, reason = "the example forces failures to show their reports")]
fn main() -> Result<(), ConfigError> {
    use forgeconf::FileFormat;

    let config = ServerConfig::loader().load()?;
    println!("Configuration loaded and validated:");
    println!("  port:      {}", config.port);
    println!("  host:      {}", config.host);
    println!("  workers:   {}", config.workers);
    println!("  log level: {}", config.log_level);
    println!("  api key:   {}...", &config.api_key[..8]);
    println!("  tags:      {:?}", config.tags);
    println!("  webhook:   {}", config.webhook);

    let broken = r#"
        port = 80
        host = ""
        workers = 4
        log_level = "verbose"
        api_key = "short"
        tags = []
        webhook = "http://insecure.example.com"
    "#;
    let error = forgeconf::from_str::<ServerConfig>(broken, FileFormat::Toml).unwrap_err();
    println!("\nValidating a broken document reports every failure:\n{error}");
    Ok(())
}

#[cfg(not(feature = "toml"))]
fn main() {
    eprintln!("this example needs the `toml` feature");
}
