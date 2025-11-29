use forgeconf::{forgeconf, ConfigError};

#[forgeconf(config(path = "tests/fixtures/basic.toml"))]
struct BasicConfig {
    port: u16,
    database_url: String,
}

#[test]
fn loads_configuration_file() -> Result<(), ConfigError> {
    let cfg = BasicConfig::loader()
        .with_config()
        .load()?;

    assert_eq!(cfg.port, 3000);
    assert!(cfg
        .database_url
        .contains("postgres://"));
    Ok(())
}
