use forgeconf::{ConfigError, forgeconf};

#[forgeconf(env_prefix = "PFXA_", config(path = "tests/fixtures/basic.toml"))]
struct PrefixOverrideConfig {
    port: u16,
    database_url: String,
}

#[forgeconf(env_prefix = "PFXB_", config(path = "tests/fixtures/basic.toml"))]
struct PrefixFallbackConfig {
    port: u16,
    database_url: String,
}

#[forgeconf(env_prefix = "PFXC_", config(path = "tests/fixtures/basic.toml"))]
struct PrefixWithExplicitEnvConfig {
    port: u16,
    #[field(env = "PFXC_CUSTOM_DB")]
    database_url: String,
}

#[test]
fn env_prefix_sets_env_var_for_each_field() -> Result<(), ConfigError> {
    unsafe {
        std::env::set_var("PFXA_PORT", "7777");
        std::env::set_var("PFXA_DATABASE_URL", "postgres://prefixed");
    }

    let cfg = PrefixOverrideConfig::loader().load()?;

    assert_eq!(cfg.port, 7777);
    assert_eq!(cfg.database_url, "postgres://prefixed");

    unsafe {
        std::env::remove_var("PFXA_PORT");
        std::env::remove_var("PFXA_DATABASE_URL");
    }
    Ok(())
}

#[test]
fn env_prefix_falls_through_to_config_when_var_unset() -> Result<(), ConfigError> {
    // PFXB_* vars are never set by any test — falls through to config file values.
    let cfg = PrefixFallbackConfig::loader().load()?;

    assert_eq!(cfg.port, 3000);
    assert_eq!(cfg.database_url, "postgres://localhost/app");
    Ok(())
}

#[test]
fn explicit_env_on_field_overrides_prefix() -> Result<(), ConfigError> {
    unsafe {
        std::env::set_var("PFXC_PORT", "8888");
        std::env::set_var("PFXC_CUSTOM_DB", "postgres://explicit");
    }

    let cfg = PrefixWithExplicitEnvConfig::loader().load()?;

    assert_eq!(cfg.port, 8888);
    assert_eq!(cfg.database_url, "postgres://explicit");

    unsafe {
        std::env::remove_var("PFXC_PORT");
        std::env::remove_var("PFXC_CUSTOM_DB");
    }
    Ok(())
}
