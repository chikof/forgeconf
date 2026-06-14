use jzon::JsonValue;

use crate::{ConfigError, ConfigNode, ErrorKind, Table};

/// Parse JSON content into a ConfigNode tree.
pub(crate) fn parse(input: &str, _source: &str) -> Result<ConfigNode, ConfigError> {
    let value = jzon::parse(input).map_err(|source| {
        ConfigError::new(ErrorKind::Syntax {
            format: "json",
            message: source.to_string(),
        })
    })?;
    Ok(convert(value))
}

/// Convert a JSON value into a ConfigNode.
fn convert(value: JsonValue) -> ConfigNode {
    match value {
        JsonValue::Null => ConfigNode::null(),
        JsonValue::Boolean(flag) => ConfigNode::scalar(flag.to_string()),
        JsonValue::Number(num) => ConfigNode::scalar(num.to_string()),
        JsonValue::String(text) => ConfigNode::scalar(text),
        JsonValue::Short(short) => ConfigNode::scalar(short.to_string()),
        JsonValue::Array(items) => ConfigNode::array(items.into_iter().map(convert).collect()),
        JsonValue::Object(map) => ConfigNode::table(
            map.into_iter()
                .map(|(key, value)| (key, convert(value)))
                .collect::<Table>(),
        ),
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
        let node = parse(input, "<string>").unwrap();
        let table = node.as_table().unwrap();
        assert_eq!(table.get("port").unwrap().as_str(), Some("8080"));
        assert_eq!(table.get("host").unwrap().as_str(), Some("localhost"));
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
        let node = parse(input, "<string>").unwrap();
        let table = node.as_table().unwrap();
        let db = table.get("database").unwrap().as_table().unwrap();
        assert_eq!(db.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(db.get("port").unwrap().as_str(), Some("5432"));
    }

    #[test]
    fn parses_arrays() {
        let input = r#"
            {
                "tags": ["api", "production", "v2"]
            }
        "#;
        let node = parse(input, "<string>").unwrap();
        let table = node.as_table().unwrap();
        let items = table.get("tags").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_str(), Some("api"));
    }

    #[test]
    fn handles_null_values() {
        let input = r#"{"key": null}"#;
        let node = parse(input, "<string>").unwrap();
        let table = node.as_table().unwrap();
        assert!(table.get("key").unwrap().is_null());
    }

    #[test]
    fn handles_boolean_values() {
        let input = r#"{"enabled": true, "debug": false}"#;
        let node = parse(input, "<string>").unwrap();
        let table = node.as_table().unwrap();
        assert_eq!(table.get("enabled").unwrap().as_str(), Some("true"));
        assert_eq!(table.get("debug").unwrap().as_str(), Some("false"));
    }

    #[test]
    fn returns_error_on_invalid_json() {
        let input = r#"{"invalid": }"#;
        let err = parse(input, "<string>").unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Syntax { format: "json", .. }));
    }
}
