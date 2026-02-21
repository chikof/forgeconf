use forgeconf::{ConfigError, forgeconf};

fn path() -> String {
    "tests/fixtures/basic.toml".to_string()
}

#[forgeconf(config(path = path()))]
struct BasicConfig {
    port: u16,
    database_url: String,
}

#[test]
fn dynamic_path_loading() -> Result<(), ConfigError> {
    let cfg = BasicConfig::loader()
        .with_config()
        .load()?;

    assert_eq!(cfg.port, 3000);
    assert!(
        cfg.database_url
            .contains("postgres://")
    );
    Ok(())
}
