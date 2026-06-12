#![cfg(feature = "toml")]

use forgeconf::{ErrorKind, forgeconf};

#[derive(Debug)]
#[forgeconf(config(path = "tests/fixtures/basic.toml"))]
#[allow(dead_code)]
struct BrokenConfig {
    missing: u32,
}

#[test]
fn missing_fields_raise_useful_errors() {
    let err = BrokenConfig::loader().load().unwrap_err();

    assert!(matches!(err.kind(), ErrorKind::Missing));
    assert_eq!(err.path(), "missing");
    assert_eq!(err.to_string(), "missing: missing value");
}
