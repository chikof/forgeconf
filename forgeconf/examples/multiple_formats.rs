//! Multiple format support example.
//!
//! This example demonstrates loading configuration from different file formats
//! (TOML, YAML, JSON) using the same struct.
//!
//! Run with: cargo run --example multiple_formats --all-features

use std::fs;

use forgeconf::forgeconf;

#[derive(Debug)]
#[forgeconf]
struct Config {
    name: String,
    version: String,
    enabled: bool,
}

fn main() {
    println!("=== Multiple Format Support Example ===\n");

    // Load from TOML
    #[cfg(feature = "toml")]
    {
        println!("Loading from TOML...");
        let toml_content =
            fs::read_to_string("examples/fixtures/format.toml").expect("Failed to read TOML file");

        match Config::parse_toml(&toml_content) {
            Ok(config) => {
                println!(
                    "✓ TOML: {} v{} (enabled: {})",
                    config.name, config.version, config.enabled
                );
            },
            Err(e) => println!("✗ TOML loading failed: {:?}", e),
        }
    }

    // Load from YAML
    #[cfg(feature = "yaml")]
    {
        println!("\nLoading from YAML...");
        let yaml_content =
            fs::read_to_string("examples/fixtures/format.yaml").expect("Failed to read YAML file");

        match Config::parse_yaml(&yaml_content) {
            Ok(config) => {
                println!(
                    "✓ YAML: {} v{} (enabled: {})",
                    config.name, config.version, config.enabled
                );
            },
            Err(e) => println!("✗ YAML loading failed: {:?}", e),
        }
    }

    // Load from JSON
    #[cfg(feature = "json")]
    {
        println!("\nLoading from JSON...");
        let json_content =
            fs::read_to_string("examples/fixtures/format.json").expect("Failed to read JSON file");

        match Config::parse_json(&json_content) {
            Ok(config) => {
                println!(
                    "✓ JSON: {} v{} (enabled: {})",
                    config.name, config.version, config.enabled
                );
            },
            Err(e) => println!("✗ JSON loading failed: {:?}", e),
        }
    }
}
