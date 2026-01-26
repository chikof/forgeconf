//! Test that miette fancy formatting works when returned from main.
//!
//! Run with: cargo run --example miette_test --features miette

use forgeconf::{forgeconf, ConfigError};

#[derive(Debug)]
#[forgeconf]
#[allow(dead_code)]
struct TestConfig {
    required_field: String,
    port: u16,
}

fn main() -> Result<(), ConfigError> {
    // This will cause a missing field error
    let toml = r#"
        port = 8080
        # required_field is missing!
    "#;

    TestConfig::parse_toml(toml)?;

    Ok(())
}
