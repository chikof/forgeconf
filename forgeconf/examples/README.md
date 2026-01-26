# Forgeconf Examples

This directory contains examples demonstrating various features of the forgeconf crate.

## Running Examples

All examples can be run using `cargo run --example <name>`. Some examples require specific features to be enabled.

## Available Examples

### 1. Basic Configuration Loading
**File:** `basic.rs`
**Run:** `cargo run --example basic`

Demonstrates the simplest way to load configuration from a TOML file.

```rust
#[derive(Debug)]
#[forgeconf(config(path = "examples/fixtures/basic.toml"))]
struct AppConfig {
    host: String,
    port: u16,
    debug: bool,
}
```

### 2. Nested Configuration Structures
**File:** `nested.rs`
**Run:** `cargo run --example nested`

Shows how to work with nested configuration sections using `#[field(name = "section")]`.

```rust
#[derive(Debug)]
#[forgeconf(config(path = "examples/fixtures/nested.toml"))]
struct AppConfig {
    app_name: String,
    #[field(name = "database")]
    database: DatabaseConfig,
    #[field(name = "logging")]
    logging: LoggingConfig,
}
```

### 3. Configuration Validation
**File:** `validators.rs`
**Run:** `cargo run --example validators --features validators`

Demonstrates using built-in validators to ensure configuration values meet specific constraints.

```rust
#[derive(Debug)]
#[forgeconf(config(path = "examples/fixtures/validators.toml"))]
struct ServerConfig {
    #[field(validate = forgeconf::validators::range(1024, 65535))]
    port: u16,

    #[field(validate = forgeconf::validators::non_empty())]
    host: String,

    #[field(validate = forgeconf::validators::one_of(vec![
        "debug".to_string(),
        "info".to_string(),
        "warn".to_string(),
        "error".to_string()
    ]))]
    log_level: String,
}
```

### 4. CLI Argument Overrides
**File:** `cli_override.rs`
**Run:** `cargo run --example cli_override --features cli -- --port=9000 --debug=true`

Shows how to override configuration values with command-line arguments.

```rust
match AppConfig::loader()
    .with_config()
    .add_source(
        CliArguments::new()
            .with_priority(255)
            .with_args(std::env::args().skip(1)),
    )
    .load()
{
    Ok(config) => println!("Loaded: {:?}", config),
    Err(e) => eprintln!("Error: {:?}", e),
}
```

### 5. Multiple Format Support
**File:** `multiple_formats.rs`
**Run:** `cargo run --example multiple_formats --all-features`

Demonstrates loading configuration from different file formats (TOML, YAML, JSON) using the same struct.

```rust
#[derive(Debug)]
#[forgeconf]
struct Config {
    name: String,
    version: String,
    enabled: bool,
}

// Then use: Config::parse_toml(), Config::parse_yaml(), Config::parse_json()
```

### 6. Parse Configuration from Strings
**File:** `parse_strings.rs`
**Run:** `cargo run --example parse_strings --features parse`

Shows how to parse configuration directly from strings rather than files, useful for testing or dynamic configuration.

```rust
let toml_config = r#"
    endpoint = "https://api.example.com"
    timeout = 30
    retries = 3
"#;

let config = ApiConfig::parse_toml(toml_config)?;
```

### 7. Error Handling
**File:** `error_handling.rs`
**Run:** `cargo run --example error_handling`

Demonstrates the beautiful error messages provided by miette when configuration loading fails.

```rust
// When configuration is invalid, you get helpful errors like:
// Error: missing required field 'port'
//   × forgeconf::missing_field
//   help: Add the missing field 'port' to your configuration file.
```

### 8. Error Showcase (Comprehensive)
**File:** `error_showcase.rs`
**Run:** `cargo run --example error_showcase --features miette`

**⭐ Recommended!** A comprehensive showcase of all error types with miette's beautiful formatting. Demonstrates:
- Missing required fields
- Type mismatches (e.g., string instead of number)
- Validation errors (e.g., out of range values)
- Parse errors (invalid syntax with line numbers!)

Each error shows miette's fancy output with error codes, help text, and clear formatting.

```rust
// Example output:
// forgeconf::type_mismatch
//   × type mismatch for field 'port'
//   help: Expected a value of type 'u16', but found 'not-a-number'. 
//         Check your configuration syntax.
```

**Important**: To get fancy error output in your applications, simply enable the `miette` feature and return `Result<(), ConfigError>` from `main()`:

```rust
use forgeconf::{forgeconf, ConfigError};

#[derive(Debug)]
#[forgeconf]
struct Config {
    port: u16,
}

fn main() -> Result<(), ConfigError> {
    let config = Config::parse_toml("port = 8080")?;
    println!("Port: {}", config.port);
    Ok(())
}
```

Then in your `Cargo.toml`:
```toml
[dependencies]
forgeconf = { version = "0.2", features = ["miette"] }
```

The fancy error formatting is completely transparent - no need to import miette types or wrap errors manually!

## Example Fixtures

The `fixtures/` directory contains sample configuration files used by the examples:

- `basic.toml` - Simple flat configuration
- `nested.toml` - Configuration with nested sections
- `validators.toml` - Configuration for validation examples
- `format.toml`, `format.yaml`, `format.json` - Same configuration in different formats

## Feature Flags

Some examples require specific feature flags:

- **Default features**: `toml`, `yaml`, `cli`, `validators`, `parse`
- **Optional features**: `json`, `regex`, `miette`

The `miette` feature enables beautiful error diagnostics with fancy formatting.

To run examples with all features:
```bash
cargo run --example <name> --all-features
```

To run examples with specific features:
```bash
cargo run --example validators --features validators
cargo run --example cli_override --features cli
cargo run --example parse_strings --features parse
```

## Learning Path

We recommend going through the examples in this order:

1. **basic** - Start here to understand the fundamentals
2. **parse_strings** - Learn about parsing from strings
3. **nested** - Work with structured configuration
4. **error_handling** - See how errors are reported
5. **error_showcase** - Comprehensive error type demonstration
6. **validators** - Add validation to your config
7. **cli_override** - Override values from command line
8. **multiple_formats** - Work with different file formats

## Need Help?

- Check the main [README.md](../README.md) for more documentation
- Look at the [integration tests](../tests/) for more usage patterns
