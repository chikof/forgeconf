use forgeconf::{forgeconf, CliArguments, ConfigError};

#[forgeconf(config(path = "tests/fixtures/basic.toml"))]
struct OverrideConfig {
    port: u16,
    #[field(env = "FORGECONF_DATABASE_URL")]
    database_url: String,
}

#[test]
fn cli_and_env_sources_override_files() -> Result<(), ConfigError> {
    std::env::set_var("FORGECONF_DATABASE_URL", "postgres://override");

    let cfg = OverrideConfig::loader()
        .with_config()
        .add_source(
            CliArguments::new()
                .with_priority(255)
                .with_args(["--port=9000"]),
        )
        .load()?;

    assert_eq!(cfg.port, 9000);
    assert_eq!(cfg.database_url, "postgres://override");

    std::env::remove_var("FORGECONF_DATABASE_URL");
    Ok(())
}
