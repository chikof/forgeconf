use forgeconf::{ConfigError, ConfigNode, Table, forgeconf};

#[derive(Debug)]
#[forgeconf]
struct ValidatorConfig {
    #[field(validate = forgeconf::validators::range(1000..=2000))]
    port: u16,
    #[field(
        validate = host_is_https,
        validate = forgeconf::validators::non_empty(),
        validate = forgeconf::validators::len(..=64),
    )]
    endpoint: String,
    #[field(validate = forgeconf::validators::one_of(vec![
        "debug".to_string(),
        "info".to_string(),
        "warn".to_string(),
    ]))]
    log_level: String,
    #[field(validate = forgeconf::validators::len(1..=3))]
    tags: Vec<String>,
}

fn host_is_https(value: &str) -> Result<(), ConfigError> {
    if value.starts_with("https://") {
        Ok(())
    } else {
        Err(ConfigError::invalid("must be an https url"))
    }
}

fn node(port: &str, endpoint: &str, log_level: &str, tags: &[&str]) -> ConfigNode {
    let mut map = Table::new();
    map.insert("port".into(), ConfigNode::scalar(port));
    map.insert("endpoint".into(), ConfigNode::scalar(endpoint));
    map.insert("log_level".into(), ConfigNode::scalar(log_level));
    map.insert(
        "tags".into(),
        ConfigNode::array(tags.iter().map(|tag| ConfigNode::scalar(*tag)).collect()),
    );
    ConfigNode::table(map)
}

#[test]
fn validators_accept_valid_values() -> Result<(), ConfigError> {
    let cfg =
        ValidatorConfig::load_from(&node("1500", "https://example.com", "info", &["api", "prod"]))?;
    assert_eq!(cfg.port, 1500);
    assert_eq!(cfg.endpoint, "https://example.com");
    assert_eq!(cfg.log_level, "info");
    assert_eq!(cfg.tags, vec!["api".to_string(), "prod".to_string()]);
    Ok(())
}

#[test]
fn range_validator_fails() {
    let node = node("9000", "https://example.com", "info", &["api", "prod"]);
    let err = ValidatorConfig::load_from(&node).unwrap_err();
    assert_eq!(err.to_string(), "port: must be between 1000 and 2000");
}

#[test]
fn custom_validator_fails() {
    let node = node("1500", "http://example.com", "info", &["api"]);
    let err = ValidatorConfig::load_from(&node).unwrap_err();
    assert_eq!(err.to_string(), "endpoint: must be an https url");
}

#[test]
fn one_of_validator_reports_error() {
    let node = node("1500", "https://example.com", "trace", &["api", "prod"]);
    let err = ValidatorConfig::load_from(&node).unwrap_err();
    assert_eq!(err.to_string(), "log_level: must be one of: debug, info, warn");
}

#[test]
fn len_range_validator_rejects_long_collection() {
    let node = node("1500", "https://example.com", "debug", &["api", "prod", "edge", "overflow"]);
    let err = ValidatorConfig::load_from(&node).unwrap_err();
    assert_eq!(err.to_string(), "tags: length must be between 1 and 3");
}
