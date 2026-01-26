//! Parse configuration from strings example.
//!
//! This example demonstrates parsing configuration directly from strings
//! rather than files, useful for testing or dynamic configuration.
//!
//! Run with: cargo run --example parse_strings --features parse

#[cfg(feature = "parse")]
use forgeconf::forgeconf;

#[cfg(feature = "parse")]
#[derive(Debug)]
#[forgeconf]
struct ApiConfig {
    endpoint: String,
    timeout: u32,
    retries: u8,
}

#[cfg(feature = "parse")]
fn main() {
    println!("=== Parse Configuration from Strings Example ===\n");

    // Parse from TOML string
    #[cfg(feature = "toml")]
    {
        let toml_config = r#"
            endpoint = "https://api.example.com"
            timeout = 30
            retries = 3
        "#;

        println!("Parsing TOML string...");
        match ApiConfig::parse_toml(toml_config) {
            Ok(config) => {
                println!("✓ TOML parsed successfully!");
                println!("  Endpoint: {}", config.endpoint);
                println!("  Timeout:  {}s", config.timeout);
                println!("  Retries:  {}", config.retries);
            },
            Err(e) => eprintln!("✗ TOML parse error: {:?}", e),
        }
    }

    // Parse from YAML string
    #[cfg(feature = "yaml")]
    {
        let yaml_config = r#"
endpoint: https://api.example.com
timeout: 30
retries: 3
        "#;

        println!("\nParsing YAML string...");
        match ApiConfig::parse_yaml(yaml_config) {
            Ok(config) => {
                println!("✓ YAML parsed successfully!");
                println!("  Endpoint: {}", config.endpoint);
                println!("  Timeout:  {}s", config.timeout);
                println!("  Retries:  {}", config.retries);
            },
            Err(e) => eprintln!("✗ YAML parse error: {:?}", e),
        }
    }

    // Parse from JSON string
    #[cfg(feature = "json")]
    {
        let json_config = r#"{
            "endpoint": "https://api.example.com",
            "timeout": 30,
            "retries": 3
        }"#;

        println!("\nParsing JSON string...");
        match ApiConfig::parse_json(json_config) {
            Ok(config) => {
                println!("✓ JSON parsed successfully!");
                println!("  Endpoint: {}", config.endpoint);
                println!("  Timeout:  {}s", config.timeout);
                println!("  Retries:  {}", config.retries);
            },
            Err(e) => eprintln!("✗ JSON parse error: {:?}", e),
        }
    }
}

#[cfg(not(feature = "parse"))]
fn main() {
    eprintln!("This example requires the 'parse' feature.");
    eprintln!("Run with: cargo run --example parse_strings --features parse");
    std::process::exit(1);
}
