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

    mod parse {
        use super::*;

        #[test]
        fn should_return_integer_field_from_basic_toml() {
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
        }

        #[test]
        fn should_return_string_field_from_basic_toml() {
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
                    .get("host")
                    .unwrap()
                    .to_string(),
                "localhost"
            );
        }

        #[test]
        fn should_return_nested_string_field() {
            let input = r#"
                [database]
                host = "localhost"
                port = 5432
            "#;
            let node = parse(input).unwrap();
            let db = node
                .as_table()
                .unwrap()
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
        }

        #[test]
        fn should_return_nested_integer_field() {
            let input = r#"
                [database]
                host = "localhost"
                port = 5432
            "#;
            let node = parse(input).unwrap();
            let db = node
                .as_table()
                .unwrap()
                .get("database")
                .unwrap()
                .as_table()
                .unwrap();
            assert_eq!(
                db.get("port")
                    .unwrap()
                    .to_string(),
                "5432"
            );
        }

        #[test]
        fn should_return_array_with_correct_length() {
            let input = r#"tags = ["api", "production", "v2"]"#;
            let node = parse(input).unwrap();
            let tags = node
                .as_table()
                .unwrap()
                .get("tags")
                .unwrap();
            assert!(matches!(tags, ConfigNode::Array(items) if items.len() == 3));
        }

        #[test]
        fn should_return_first_array_item() {
            let input = r#"tags = ["api", "production", "v2"]"#;
            let node = parse(input).unwrap();
            let tags = node
                .as_table()
                .unwrap()
                .get("tags")
                .unwrap();
            assert!(matches!(tags, ConfigNode::Array(items) if items[0].to_string() == "api"));
        }

        #[test]
        fn should_return_error_on_invalid_toml() {
            let input = "invalid = [toml";
            let err = parse(input).unwrap_err();
            assert!(matches!(err, ConfigError::Toml { .. }));
        }
    }
}
