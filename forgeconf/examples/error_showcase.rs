//! Comprehensive error showcase demonstrating miette integration.
//!
//! This example demonstrates all the different error types and how miette
//! presents them beautifully to users with helpful diagnostic information.
//!
//! Run with: cargo run --example error_showcase --features miette

use forgeconf::{forgeconf, ConfigError};

#[derive(Debug)]
#[forgeconf]
#[allow(dead_code)]
struct ValidConfig {
    host: String,
    port: u16,
    #[field(validate = forgeconf::validators::range(1024, 65535))]
    secure_port: u16,
}

fn main() -> Result<(), ConfigError> {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║          Forgeconf Error Handling Showcase                ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    println!("💡 This example demonstrates miette's beautiful error output!");
    println!("   Notice the error codes, help text, and clear formatting.\n");

    demonstrate_missing_field()?;
    demonstrate_type_mismatch()?;
    demonstrate_validation_error()?;
    demonstrate_parse_error()?;

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║                  Key Takeaways                             ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();
    println!("✓ Each error has a unique diagnostic code (forgeconf::*)");
    println!("✓ Clear error messages describing what went wrong");
    println!("✓ Type mismatches show expected vs actual values");
    println!("✓ Validation errors include the acceptable range");
    println!("✓ Help text provides actionable guidance");
    println!();
    println!("📖 All errors implement miette's Diagnostic trait, providing:");
    println!("   • Structured error codes for programmatic handling");
    println!("   • Helpful suggestions and context");
    println!("   • Support for source code highlighting (future feature)");
    println!();

    Ok(())
}

fn demonstrate_missing_field() -> Result<(), ConfigError> {
    println!("───────────────────────────────────────────────────────────");
    println!("Example 1: Missing Required Field");
    println!("───────────────────────────────────────────────────────────\n");

    let config_missing_field = r#"
        host = "localhost"
        port = 8080
        # secure_port is missing!
    "#;

    println!("Config content:");
    println!("{}", config_missing_field);

    match ValidConfig::parse_toml(config_missing_field) {
        Ok(_) => println!("❌ Unexpected success!"),
        Err(e) => {
            println!("✓ Error caught:\n");
            // With miette feature, Debug formatting is automatically fancy
            println!("{:?}", e);
            println!();
        },
    }

    Ok(())
}

fn demonstrate_type_mismatch() -> Result<(), ConfigError> {
    println!("───────────────────────────────────────────────────────────");
    println!("Example 2: Type Mismatch");
    println!("───────────────────────────────────────────────────────────\n");

    let config_wrong_type = r#"
        host = "localhost"
        port = "not-a-number"  # Should be a u16
        secure_port = 8443
    "#;

    println!("Config content:");
    println!("{}", config_wrong_type);

    match ValidConfig::parse_toml(config_wrong_type) {
        Ok(_) => println!("❌ Unexpected success!"),
        Err(e) => {
            println!("✓ Error caught:\n");
            println!("{:?}", e);
            println!();
        },
    }

    Ok(())
}

fn demonstrate_validation_error() -> Result<(), ConfigError> {
    println!("───────────────────────────────────────────────────────────");
    println!("Example 3: Validation Error");
    println!("───────────────────────────────────────────────────────────\n");

    let config_invalid_range = r#"
        host = "localhost"
        port = 8080
        secure_port = 80  # Below minimum of 1024
    "#;

    println!("Config content:");
    println!("{}", config_invalid_range);

    match ValidConfig::parse_toml(config_invalid_range) {
        Ok(_) => println!("❌ Unexpected success!"),
        Err(e) => {
            println!("✓ Error caught:\n");
            println!("{:?}", e);
            println!();
        },
    }

    Ok(())
}

fn demonstrate_parse_error() -> Result<(), ConfigError> {
    println!("───────────────────────────────────────────────────────────");
    println!("Example 4: Parse Error (Invalid TOML Syntax)");
    println!("───────────────────────────────────────────────────────────\n");

    let config_bad_syntax = r#"
        host = "localhost
        port = [broken
        secure_port = 8443
    "#;

    println!("Config content:");
    println!("{}", config_bad_syntax);

    match ValidConfig::parse_toml(config_bad_syntax) {
        Ok(_) => println!("❌ Unexpected success!"),
        Err(e) => {
            println!("✓ Error caught:\n");
            println!("{:?}", e);
            println!();
        },
    }

    Ok(())
}
