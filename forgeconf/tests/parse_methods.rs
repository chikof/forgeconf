#![cfg(feature = "parse")]

use forgeconf::{ConfigError, forgeconf};

#[forgeconf]
struct ParseConfig {
    port: u16,
    host: String,
    #[field(default = false)]
    debug: bool,
}

#[test]
#[cfg(feature = "toml")]
fn parse_toml_basic() -> Result<(), ConfigError> {
    let input = r#"
        port = 8080
        host = "localhost"
        debug = true
    "#;

    let config = ParseConfig::parse_toml(input)?;
    assert_eq!(config.port, 8080);
    assert_eq!(config.host, "localhost");
    assert!(config.debug);
    Ok(())
}

#[test]
#[cfg(feature = "toml")]
fn parse_toml_with_defaults() -> Result<(), ConfigError> {
    let input = r#"
        port = 3000
        host = "127.0.0.1"
    "#;

    let config = ParseConfig::parse_toml(input)?;
    assert_eq!(config.port, 3000);
    assert_eq!(config.host, "127.0.0.1");
    assert!(!config.debug);
    Ok(())
}

#[test]
#[cfg(feature = "yaml")]
fn parse_yaml_basic() -> Result<(), ConfigError> {
    let input = r#"
        port: 9090
        host: example.com
        debug: true
    "#;

    let config = ParseConfig::parse_yaml(input)?;
    assert_eq!(config.port, 9090);
    assert_eq!(config.host, "example.com");
    assert!(config.debug);
    Ok(())
}

#[test]
#[cfg(feature = "yaml")]
fn parse_yaml_with_defaults() -> Result<(), ConfigError> {
    let input = r#"
        port: 5000
        host: api.example.com
    "#;

    let config = ParseConfig::parse_yaml(input)?;
    assert_eq!(config.port, 5000);
    assert_eq!(config.host, "api.example.com");
    assert!(!config.debug);
    Ok(())
}

#[test]
#[cfg(feature = "json")]
fn parse_json_basic() -> Result<(), ConfigError> {
    let input = r#"
        {
            "port": 7070,
            "host": "json.example.com",
            "debug": true
        }
    "#;

    let config = ParseConfig::parse_json(input)?;
    assert_eq!(config.port, 7070);
    assert_eq!(config.host, "json.example.com");
    assert!(config.debug);
    Ok(())
}

#[test]
#[cfg(feature = "json")]
fn parse_json_with_defaults() -> Result<(), ConfigError> {
    let input = r#"
        {
            "port": 4000,
            "host": "api.json.com"
        }
    "#;

    let config = ParseConfig::parse_json(input)?;
    assert_eq!(config.port, 4000);
    assert_eq!(config.host, "api.json.com");
    assert!(!config.debug);
    Ok(())
}

#[forgeconf]
struct NestedParseConfig {
    port: u16,
    database: DatabaseConfig,
}

#[forgeconf]
struct DatabaseConfig {
    host: String,
    port: u16,
}

#[test]
#[cfg(feature = "toml")]
fn parse_toml_nested() -> Result<(), ConfigError> {
    let input = r#"
        port = 8080
        
        [database]
        host = "db.example.com"
        port = 5432
    "#;

    let config = NestedParseConfig::parse_toml(input)?;
    assert_eq!(config.port, 8080);
    assert_eq!(
        config
            .database
            .host,
        "db.example.com"
    );
    assert_eq!(
        config
            .database
            .port,
        5432
    );
    Ok(())
}

#[test]
#[cfg(feature = "yaml")]
fn parse_yaml_nested() -> Result<(), ConfigError> {
    let input = r#"
        port: 8080
        database:
          host: db.yaml.com
          port: 5432
    "#;

    let config = NestedParseConfig::parse_yaml(input)?;
    assert_eq!(config.port, 8080);
    assert_eq!(
        config
            .database
            .host,
        "db.yaml.com"
    );
    assert_eq!(
        config
            .database
            .port,
        5432
    );
    Ok(())
}

#[test]
#[cfg(feature = "json")]
fn parse_json_nested() -> Result<(), ConfigError> {
    let input = r#"
        {
            "port": 8080,
            "database": {
                "host": "db.json.com",
                "port": 5432
            }
        }
    "#;

    let config = NestedParseConfig::parse_json(input)?;
    assert_eq!(config.port, 8080);
    assert_eq!(
        config
            .database
            .host,
        "db.json.com"
    );
    assert_eq!(
        config
            .database
            .port,
        5432
    );
    Ok(())
}

#[forgeconf]
struct ConfigWithArrays {
    tags: Vec<String>,
    ports: Vec<u16>,
}

#[test]
#[cfg(feature = "toml")]
fn parse_toml_with_arrays() -> Result<(), ConfigError> {
    let input = r#"
        tags = ["api", "production", "v2"]
        ports = [8080, 8081, 8082]
    "#;

    let config = ConfigWithArrays::parse_toml(input)?;
    assert_eq!(config.tags, vec!["api", "production", "v2"]);
    assert_eq!(config.ports, vec![8080, 8081, 8082]);
    Ok(())
}

#[test]
#[cfg(feature = "yaml")]
fn parse_yaml_with_arrays() -> Result<(), ConfigError> {
    let input = r#"
        tags:
          - staging
          - v3
        ports:
          - 9090
          - 9091
    "#;

    let config = ConfigWithArrays::parse_yaml(input)?;
    assert_eq!(config.tags, vec!["staging", "v3"]);
    assert_eq!(config.ports, vec![9090, 9091]);
    Ok(())
}

#[test]
#[cfg(feature = "json")]
fn parse_json_with_arrays() -> Result<(), ConfigError> {
    let input = r#"
        {
            "tags": ["test", "experimental"],
            "ports": [7070, 7071, 7072]
        }
    "#;

    let config = ConfigWithArrays::parse_json(input)?;
    assert_eq!(config.tags, vec!["test", "experimental"]);
    assert_eq!(config.ports, vec![7070, 7071, 7072]);
    Ok(())
}
