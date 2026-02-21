use forgeconf::{ConfigError, forgeconf};

#[forgeconf]
struct HttpSettings {
    host: String,
    port: u16,
}

#[forgeconf]
struct DatabaseSettings {
    url: String,
    pool: u16,
}

#[forgeconf(config(path = "tests/fixtures/nested.toml"))]
struct ApplicationConfig {
    #[field(name = "http")]
    http: HttpSettings,
    #[field(name = "database")]
    database: DatabaseSettings,
}

#[test]
fn nested_structs_load_from_sections() -> Result<(), ConfigError> {
    let cfg = ApplicationConfig::loader()
        .with_config()
        .load()?;

    assert_eq!(
        cfg.http
            .host,
        "127.0.0.1"
    );
    assert_eq!(
        cfg.http
            .port,
        8080
    );
    assert_eq!(
        cfg.database
            .url,
        "postgres://service"
    );
    assert_eq!(
        cfg.database
            .pool,
        16
    );
    Ok(())
}
