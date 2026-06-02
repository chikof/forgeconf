use forgeconf::forgeconf;

// #[forgeconf] must be outermost so it can strip #[derive(Debug)] before the
// derive runs.
#[forgeconf(config(path = "tests/fixtures/basic.toml"))]
#[derive(Debug)]
struct SensitiveConfig {
    port: u16,
    #[field(sensitive)]
    database_url: String,
}

#[forgeconf(config(path = "tests/fixtures/basic.toml"))]
#[derive(Debug)]
struct SensitiveShorthandConfig {
    port: u16,
    #[sensitive]
    database_url: String,
}

#[forgeconf(config(path = "tests/fixtures/basic.toml"))]
struct SensitiveNoDerivConfig {
    port: u16,
    #[sensitive]
    database_url: String,
}

#[test]
fn sensitive_field_masked_in_debug() {
    let cfg = SensitiveConfig::loader().load().unwrap();
    let out = format!("{:?}", cfg);
    assert!(out.contains("***"), "sensitive field must show ***");
    assert!(!out.contains("postgres"), "sensitive field must not show actual value");
    assert!(out.contains("3000"), "non-sensitive field must be visible");
}

#[test]
fn sensitive_shorthand_attr_works() {
    let cfg = SensitiveShorthandConfig::loader().load().unwrap();
    let out = format!("{:?}", cfg);
    assert!(out.contains("***"));
    assert!(!out.contains("postgres"));
    assert!(out.contains("3000"));
}

#[test]
fn sensitive_gets_debug_without_derive() {
    let cfg = SensitiveNoDerivConfig::loader().load().unwrap();
    let out = format!("{:?}", cfg);
    assert!(out.contains("***"));
    assert!(!out.contains("postgres"));
}

#[test]
fn sensitive_field_value_accessible() {
    let cfg = SensitiveConfig::loader().load().unwrap();
    assert_eq!(cfg.port, 3000);
    assert!(cfg.database_url.contains("postgres://"));
}
