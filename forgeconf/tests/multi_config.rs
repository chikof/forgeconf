use forgeconf::{forgeconf, ConfigError};

#[forgeconf(
    config(path = "tests/fixtures/priority-base.toml", priority = 5),
    config(path = "tests/fixtures/priority-override.cfg", format = "toml", priority = 200)
)]
struct LayeredConfig {
    port: u16,
    database_url: String,
}

#[test]
fn explicit_format_and_priority_override() -> Result<(), ConfigError> {
    let cfg = LayeredConfig::loader()
        .with_config()
        .load()?;

    assert_eq!(cfg.port, 5555);
    assert_eq!(cfg.database_url, "postgres://override");

    Ok(())
}
