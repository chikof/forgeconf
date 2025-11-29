use forgeconf::{forgeconf, ConfigError};

#[forgeconf(config(path = "tests/fixtures/defaults.toml"))]
struct DefaultsConfig {
    #[field(default = 8080)]
    port: u16,
    #[field(optional = true)]
    notes: Option<String>,
    #[field(name = "NAME", insensitive = true)]
    service_name: String,
}

#[test]
fn defaults_and_optional_fields_work() -> Result<(), ConfigError> {
    let cfg = DefaultsConfig::loader()
        .with_config()
        .load()?;

    assert_eq!(cfg.port, 8080);
    assert!(cfg
        .notes
        .is_none());
    assert_eq!(cfg.service_name, "example");
    Ok(())
}
