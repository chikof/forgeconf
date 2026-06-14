use toml::Value;

use crate::{ConfigError, ConfigNode, ErrorKind, Table};

/// Parse TOML content into a ConfigNode tree.
pub(crate) fn parse(input: &str, _source: &str) -> Result<ConfigNode, ConfigError> {
    let value: Value = toml::from_str(input).map_err(|source| {
        ConfigError::new(ErrorKind::Syntax {
            format: "toml",
            message: source.to_string(),
        })
    })?;
    Ok(convert(value))
}

/// Convert a TOML value into a ConfigNode.
fn convert(value: Value) -> ConfigNode {
    match value {
        Value::Boolean(flag) => ConfigNode::scalar(flag.to_string()),
        Value::Integer(num) => ConfigNode::scalar(num.to_string()),
        Value::Float(num) => ConfigNode::scalar(num.to_string()),
        Value::String(text) => ConfigNode::scalar(text),
        Value::Datetime(dt) => ConfigNode::scalar(dt.to_string()),
        Value::Array(values) => ConfigNode::array(values.into_iter().map(convert).collect()),
        Value::Table(values) => ConfigNode::table(
            values
                .into_iter()
                .map(|(key, value)| (key, convert(value)))
                .collect::<Table>(),
        ),
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
            let node = parse(input, "<string>").unwrap();
            let table = node.as_table().unwrap();
            assert_eq!(table.get("port").unwrap().as_str(), Some("8080"));
        }

        #[test]
        fn should_return_string_field_from_basic_toml() {
            let input = r#"
                port = 8080
                host = "localhost"
            "#;
            let node = parse(input, "<string>").unwrap();
            let table = node.as_table().unwrap();
            assert_eq!(table.get("host").unwrap().as_str(), Some("localhost"));
        }

        #[test]
        fn should_return_nested_string_field() {
            let input = r#"
                [database]
                host = "localhost"
                port = 5432
            "#;
            let node = parse(input, "<string>").unwrap();
            let db = node
                .as_table()
                .unwrap()
                .get("database")
                .unwrap()
                .as_table()
                .unwrap();
            assert_eq!(db.get("host").unwrap().as_str(), Some("localhost"));
        }

        #[test]
        fn should_return_nested_integer_field() {
            let input = r#"
                [database]
                host = "localhost"
                port = 5432
            "#;
            let node = parse(input, "<string>").unwrap();
            let db = node
                .as_table()
                .unwrap()
                .get("database")
                .unwrap()
                .as_table()
                .unwrap();
            assert_eq!(db.get("port").unwrap().as_str(), Some("5432"));
        }

        #[test]
        fn should_return_array_with_correct_length() {
            let input = r#"tags = ["api", "production", "v2"]"#;
            let node = parse(input, "<string>").unwrap();
            let tags = node.as_table().unwrap().get("tags").unwrap();
            assert_eq!(tags.as_array().map(<[_]>::len), Some(3));
        }

        #[test]
        fn should_return_first_array_item() {
            let input = r#"tags = ["api", "production", "v2"]"#;
            let node = parse(input, "<string>").unwrap();
            let tags = node.as_table().unwrap().get("tags").unwrap();
            assert_eq!(tags.as_array().unwrap()[0].as_str(), Some("api"));
        }

        #[test]
        fn should_return_error_on_invalid_toml() {
            let input = "invalid = [toml";
            let err = parse(input, "<string>").unwrap_err();
            assert!(matches!(err.kind(), ErrorKind::Syntax { format: "toml", .. }));
        }
    }
}
