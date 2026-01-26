use forgeconf::{forgeconf, ConfigError};

#[derive(Debug)]
#[forgeconf(config(path = "tests/fixtures/basic.toml"))]
#[allow(dead_code)]
struct BrokenConfig {
    missing: u32,
}

#[test]
fn missing_fields_raise_useful_errors() {
    let err = BrokenConfig::loader()
        .with_config()
        .load()
        .unwrap_err();

    match err {
        ConfigError::MissingValue { field, .. } => assert_eq!(field, "missing"),
        other => panic!("unexpected error: {other:?}"),
    }
}
