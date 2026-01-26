use std::collections::BTreeMap;

use jzon::JsonValue;

use crate::{ConfigError, ConfigNode};

/// Parse JSON content into a ConfigNode tree.
pub fn parse(input: &str) -> Result<ConfigNode, ConfigError> {
    let value = jzon::parse(input).map_err(|source| ConfigError::Json { source, span: None })?;
    Ok(convert(value))
}

/// Convert a JSON value into a ConfigNode.
fn convert(value: JsonValue) -> ConfigNode {
    match value {
        JsonValue::Null => ConfigNode::Null,
        JsonValue::Boolean(flag) => ConfigNode::Scalar(flag.to_string()),
        JsonValue::Number(num) => ConfigNode::Scalar(num.to_string()),
        JsonValue::String(text) => ConfigNode::Scalar(text),
        JsonValue::Short(short) => ConfigNode::Scalar(short.to_string()),
        JsonValue::Array(items) => ConfigNode::Array(
            items
                .into_iter()
                .map(convert)
                .collect(),
        ),
        JsonValue::Object(map) => {
            let entries = map
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
    fn parses_basic_json() {
        let input = r#"
            {
                "port": 8080,
                "host": "localhost"
            }
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
    fn parses_nested_objects() {
        let input = r#"
            {
                "database": {
                    "host": "localhost",
                    "port": 5432
                }
            }
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
            {
                "tags": ["api", "production", "v2"]
            }
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
    fn handles_null_values() {
        let input = r#"{"key": null}"#;
        let node = parse(input).unwrap();
        let table = node
            .as_table()
            .unwrap();
        assert!(matches!(
            table
                .get("key")
                .unwrap(),
            ConfigNode::Null
        ));
    }

    #[test]
    fn handles_boolean_values() {
        let input = r#"{"enabled": true, "debug": false}"#;
        let node = parse(input).unwrap();
        let table = node
            .as_table()
            .unwrap();
        assert_eq!(
            table
                .get("enabled")
                .unwrap()
                .to_string(),
            "true"
        );
        assert_eq!(
            table
                .get("debug")
                .unwrap()
                .to_string(),
            "false"
        );
    }

    #[test]
    fn returns_error_on_invalid_json() {
        let input = r#"{"invalid": }"#;
        let err = parse(input).unwrap_err();
        assert!(matches!(err, ConfigError::Json { .. }));
    }
}
