use forgeconf::{ConfigError, forgeconf};

#[forgeconf(config(path = "tests/fixtures/basic.toml"))]
struct DeprecatedConfig {
    port: u16,
    #[field(deprecated = "use `connection_string` instead")]
    database_url: String,
}

#[forgeconf(config(path = "tests/fixtures/basic.toml"))]
struct DeprecatedNoMsgConfig {
    port: u16,
    #[field(deprecated)]
    database_url: String,
}

#[test]
fn deprecated_field_loads_with_warning() -> Result<(), ConfigError> {
    let cfg = DeprecatedConfig::loader().load()?;
    assert_eq!(cfg.port, 3000);
    assert!(cfg.database_url.contains("postgres://"));
    Ok(())
}

#[test]
fn deprecated_bare_flag_loads_with_warning() -> Result<(), ConfigError> {
    let cfg = DeprecatedNoMsgConfig::loader().load()?;
    assert_eq!(cfg.port, 3000);
    assert!(cfg.database_url.contains("postgres://"));
    Ok(())
}
