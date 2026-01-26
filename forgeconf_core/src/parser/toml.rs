use std::collections::BTreeMap;

use toml::Value;

use crate::{ConfigError, ConfigNode};

/// Parse TOML content into a ConfigNode tree.
pub fn parse(input: &str) -> Result<ConfigNode, ConfigError> {
    let value: Value =
        toml::from_str(input).map_err(|source| ConfigError::Toml { source, span: None })?;
    Ok(convert(value))
}

/// Convert a TOML value into a ConfigNode.
fn convert(value: Value) -> ConfigNode {
    match value {
        Value::Boolean(flag) => ConfigNode::Scalar(flag.to_string()),
        Value::Integer(num) => ConfigNode::Scalar(num.to_string()),
        Value::Float(num) => ConfigNode::Scalar(num.to_string()),
        Value::String(text) => ConfigNode::Scalar(text),
        Value::Datetime(dt) => ConfigNode::Scalar(dt.to_string()),
        Value::Array(values) => ConfigNode::Array(
            values
                .into_iter()
                .map(convert)
                .collect(),
        ),
        Value::Table(values) => {
            let entries = values
                .into_iter()
                .map(|(key, value)| (key, convert(value)))
                .collect::<BTreeMap<_, _>>();
            ConfigNode::Table(entries)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_toml() {
        let input = r#"
            port = 8080
            host = "localhost"
        "#;
        let node = parse(input).unwrap();
        let table = node
            .as_table()
            .unwrap();
        assert_eq!(
            table
                .get("port")
                .unwrap()
                .to_string(),
            "8080"
        );
        assert_eq!(
            table
                .get("host")
                .unwrap()
                .to_string(),
            "localhost"
        );
    }

    #[test]
    fn parses_nested_tables() {
        let input = r#"
            [database]
            host = "localhost"
            port = 5432
        "#;
        let node = parse(input).unwrap();
        let table = node
            .as_table()
            .unwrap();
        let db = table
            .get("database")
            .unwrap()
            .as_table()
            .unwrap();
        assert_eq!(
            db.get("host")
                .unwrap()
                .to_string(),
            "localhost"
        );
        assert_eq!(
            db.get("port")
                .unwrap()
                .to_string(),
            "5432"
        );
    }

    #[test]
    fn parses_arrays() {
        let input = r#"
            tags = ["api", "production", "v2"]
        "#;
        let node = parse(input).unwrap();
        let table = node
            .as_table()
            .unwrap();
        match table
            .get("tags")
            .unwrap()
        {
            ConfigNode::Array(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0].to_string(), "api");
            },
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn returns_error_on_invalid_toml() {
        let input = "invalid = [toml";
        let err = parse(input).unwrap_err();
        assert!(matches!(err, ConfigError::Toml { .. }));
    }
}
