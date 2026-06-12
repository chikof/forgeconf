//! Error handling example.
//!
//! This example shows how to inspect a [`ConfigError`] with `kind()` and
//! `path()` when configuration loading fails.
//!
//! Run with: cargo run --example error_handling

use forgeconf::forgeconf;
#[cfg(feature = "toml")]
use forgeconf::{ConfigError, ErrorKind};

#[derive(Debug)]
#[forgeconf]
#[allow(dead_code)]
struct BrokenConfig {
    // This field is required but will be missing
    required_field: String,
    // This field expects a number but will get a string
    port: u16,
}

#[cfg(feature = "toml")]
fn describe(err: &ConfigError) -> &'static str {
    match err.kind() {
        ErrorKind::Missing => "a required value is absent",
        ErrorKind::Mismatch { .. } => "a value has the wrong type",
        ErrorKind::Syntax { .. } => "the input is not valid TOML",
        _ => "some other error",
    }
}

fn main() {
    println!("=== Error Handling Example ===\n");
    println!("This example intentionally loads invalid configuration");
    println!("to demonstrate the error reporting capabilities.\n");

    #[cfg(feature = "toml")]
    {
        let invalid_toml = r#"
            # Missing: required_field
            port = "not a number"
        "#;
        let err = BrokenConfig::parse_toml(invalid_toml).unwrap_err();
        println!("kind:  {:?}", err.kind());
        println!("what:  {}", describe(&err));
        println!("path:  {}", err.path());
        println!("error: {err}");

        let invalid_port = "required_field = \"x\"\nport = \"not a number\"";
        let err = BrokenConfig::parse_toml(invalid_port).unwrap_err();
        println!("\nkind:  {:?}", err.kind());
        println!("what:  {}", describe(&err));
        println!("path:  {}", err.path());
        println!("error: {err}");
    }

    #[cfg(not(feature = "toml"))]
    {
        eprintln!("This example requires the 'toml' feature.");
        eprintln!("Run with: cargo run --example error_handling");
    }
}
