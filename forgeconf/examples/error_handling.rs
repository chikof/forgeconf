//! Error handling example.
//!
//! This example demonstrates the beautiful error messages provided by miette
//! when configuration loading fails.
//!
//! Run with: cargo run --example error_handling

use forgeconf::forgeconf;

#[derive(Debug)]
#[forgeconf]
#[allow(dead_code)]
struct BrokenConfig {
    // This field is required but will be missing
    required_field: String,
    // This field expects a number but will get a string
    port: u16,
}

fn main() {
    println!("=== Error Handling Example ===\n");
    println!("This example intentionally loads invalid configuration");
    println!("to demonstrate the error reporting capabilities.\n");

    // Try to parse invalid TOML
    let invalid_toml = r#"
        # Missing: required_field
        port = "not a number"
    "#;

    #[cfg(feature = "toml")]
    match BrokenConfig::parse_toml(invalid_toml) {
        Ok(_) => println!("Unexpected success!"),
        Err(e) => {
            println!("✓ Error caught as expected!\n");
            println!("Error details:");
            println!("{:?}", e);
            println!("\n{}", "=".repeat(60));
            println!("Notice the helpful error message with:");
            println!("  • Clear error code (forgeconf::missing_field or similar)");
            println!("  • Descriptive message");
            println!("  • Actionable help text");
            println!("{}", "=".repeat(60));
        },
    }

    #[cfg(not(feature = "toml"))]
    {
        eprintln!("This example requires the 'toml' feature.");
        eprintln!("Run with: cargo run --example error_handling");
    }
}
