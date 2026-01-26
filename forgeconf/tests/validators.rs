#![cfg(feature = "validators")]

use std::collections::BTreeMap;

use forgeconf::{ConfigError, ConfigNode, forgeconf};

#[derive(Debug)]
#[forgeconf]
struct ValidatorConfig {
    #[field(validate = forgeconf::validators::range(1000, 2000))]
    port: u16,
    #[field(
        validate = host_is_https,
        validate = forgeconf::validators::non_empty(),
        validate = forgeconf::validators::max_len(64),
    )]
    endpoint: String,
    #[field(validate = forgeconf::validators::one_of(vec![
        "debug".to_string(),
        "info".to_string(),
        "warn".to_string(),
    ]))]
    log_level: String,
    #[field(validate = forgeconf::validators::len_range(1, 3))]
    tags: Vec<String>,
}

fn host_is_https(value: &str, key: &str) -> Result<(), ConfigError> {
    if value.starts_with("https://") {
        Ok(())
    } else {
        Err(ConfigError::mismatch(key, "https url", value))
    }
}

#[test]
fn validators_accept_valid_values() -> Result<(), ConfigError> {
    let mut map = BTreeMap::new();
    map.insert("port".into(), ConfigNode::Scalar("1500".into()));
    map.insert("endpoint".into(), ConfigNode::Scalar("https://example.com".into()));
    map.insert("log_level".into(), ConfigNode::Scalar("info".into()));
    map.insert(
        "tags".into(),
        ConfigNode::Array(vec![
            ConfigNode::Scalar("api".into()),
            ConfigNode::Scalar("prod".into()),
        ]),
    );

    let node = ConfigNode::Table(map);
    let cfg = ValidatorConfig::load_from(&node)?;
    assert_eq!(cfg.port, 1500);
    assert_eq!(cfg.endpoint, "https://example.com");
    assert_eq!(cfg.log_level, "info");
    assert_eq!(cfg.tags, vec!["api".to_string(), "prod".to_string()]);
    Ok(())
}

#[test]
fn range_validator_fails() {
    let mut map = BTreeMap::new();
    map.insert("port".into(), ConfigNode::Scalar("9000".into()));
    map.insert("endpoint".into(), ConfigNode::Scalar("https://example.com".into()));
    map.insert("log_level".into(), ConfigNode::Scalar("info".into()));
    map.insert(
        "tags".into(),
        ConfigNode::Array(vec![
            ConfigNode::Scalar("api".into()),
            ConfigNode::Scalar("prod".into()),
        ]),
    );

    let node = ConfigNode::Table(map);
    let err = ValidatorConfig::load_from(&node).unwrap_err();
    assert!(matches!(
        err,
        ConfigError::TypeMismatch { field, expected, found, .. }
            if field == "port" && expected == "between 1000 and 2000" && found == "9000"
    ));
}

#[test]
fn one_of_validator_reports_error() {
    let mut map = BTreeMap::new();
    map.insert("port".into(), ConfigNode::Scalar("1500".into()));
    map.insert("endpoint".into(), ConfigNode::Scalar("https://example.com".into()));
    map.insert("log_level".into(), ConfigNode::Scalar("trace".into()));
    map.insert(
        "tags".into(),
        ConfigNode::Array(vec![
            ConfigNode::Scalar("api".into()),
            ConfigNode::Scalar("prod".into()),
        ]),
    );

    let node = ConfigNode::Table(map);
    let err = ValidatorConfig::load_from(&node).unwrap_err();
    assert!(matches!(
        err,
        ConfigError::TypeMismatch { field, expected, found, .. }
            if field == "log_level" && expected.contains("debug") && found == "trace"
    ));
}

#[test]
fn len_range_validator_rejects_long_collection() {
    let mut map = BTreeMap::new();
    map.insert("port".into(), ConfigNode::Scalar("1500".into()));
    map.insert("endpoint".into(), ConfigNode::Scalar("https://example.com".into()));
    map.insert("log_level".into(), ConfigNode::Scalar("debug".into()));
    map.insert(
        "tags".into(),
        ConfigNode::Array(vec![
            ConfigNode::Scalar("api".into()),
            ConfigNode::Scalar("prod".into()),
            ConfigNode::Scalar("edge".into()),
            ConfigNode::Scalar("overflow".into()),
        ]),
    );

    let node = ConfigNode::Table(map);
    let err = ValidatorConfig::load_from(&node).unwrap_err();
    assert!(matches!(
        err,
        ConfigError::TypeMismatch { field, expected, .. }
            if field == "tags" && expected.contains("length between")
    ));
}

#[cfg(feature = "regex")]
mod regex_validators {
    use regex::Regex;

    use super::*;

    #[derive(Debug)]
    #[forgeconf]
    struct RegexValidatorConfig {
        #[field(validate = forgeconf::validators::matches_regex(
            Regex::new(r"^[a-z0-9_-]+$").unwrap()
        ))]
        slug: String,
    }

    #[test]
    fn regex_validator_accepts_slug() -> Result<(), ConfigError> {
        let mut map = BTreeMap::new();
        map.insert("slug".into(), ConfigNode::Scalar("release_2024".into()));
        let node = ConfigNode::Table(map);
        RegexValidatorConfig::load_from(&node).map(|cfg| {
            assert_eq!(cfg.slug, "release_2024");
        })
    }

    #[test]
    fn regex_validator_rejects_invalid_slug() {
        let mut map = BTreeMap::new();
        map.insert("slug".into(), ConfigNode::Scalar("Release!".into()));
        let node = ConfigNode::Table(map);
        let err = RegexValidatorConfig::load_from(&node).unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch { field, expected, .. }
                if field == "slug" && expected.starts_with("match /")
        ));
    }
}
