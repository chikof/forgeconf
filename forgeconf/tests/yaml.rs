use forgeconf::{ConfigError, forgeconf};

#[forgeconf]
struct ServiceConfig {
    name: String,
    enabled: bool,
}

#[forgeconf(config(path = "tests/fixtures/yaml-config.yaml"))]
struct ClusterConfig {
    service: ServiceConfig,
    replicas: Vec<String>,
    port: u16,
}

#[test]
fn yaml_files_can_be_loaded() -> Result<(), ConfigError> {
    let cfg = ClusterConfig::loader()
        .with_config()
        .load()?;

    assert_eq!(
        cfg.service
            .name,
        "forgeconf"
    );
    assert!(
        cfg.service
            .enabled
    );
    assert_eq!(cfg.replicas, vec!["db-a", "db-b"]);
    assert_eq!(cfg.port, 8123);

    Ok(())
}
