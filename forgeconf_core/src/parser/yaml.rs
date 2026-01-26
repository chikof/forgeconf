use std::collections::BTreeMap;

use yaml_rust2::{Yaml, YamlLoader};

use crate::{ConfigError, ConfigNode};

/// Parse YAML content into a ConfigNode tree.
/// If multiple YAML documents are present, only the first is used.
pub fn parse(input: &str) -> Result<ConfigNode, ConfigError> {
    let docs = YamlLoader::load_from_str(input).map_err(ConfigError::Yaml)?;
    let document = docs
        .into_iter()
        .next()
        .unwrap_or(Yaml::Null);
    Ok(convert(document))
}

/// Convert a YAML value into a ConfigNode.
fn convert(value: Yaml) -> ConfigNode {
    match value {
        Yaml::Null | Yaml::BadValue => ConfigNode::Null,
        Yaml::Real(text) | Yaml::String(text) => ConfigNode::Scalar(text),
        Yaml::Boolean(flag) => ConfigNode::Scalar(flag.to_string()),
        Yaml::Integer(num) => ConfigNode::Scalar(num.to_string()),
        Yaml::Array(values) => ConfigNode::Array(
            values
                .into_iter()
                .map(convert)
                .collect(),
        ),
        Yaml::Hash(map) => {
            let entries = map
                .into_iter()
                .map(|(key, value)| (scalar_key(key), convert(value)))
                .collect::<BTreeMap<_, _>>();
            ConfigNode::Table(entries)
        },
        Yaml::Alias(_) => ConfigNode::Null,
    }
}

/// Convert a YAML key to a string.
fn scalar_key(key: Yaml) -> String {
    match key {
        Yaml::String(text) => text,
        Yaml::Integer(num) => num.to_string(),
        Yaml::Real(text) => text,
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_yaml() {
        let input = r#"
            port: 8080
            host: localhost
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
    fn parses_nested_maps() {
        let input = r#"
            database:
              host: localhost
              port: 5432
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
    fn parses_sequences() {
        let input = r#"
            tags:
              - api
              - production
              - v2
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
        let input = "key: null";
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
    fn returns_error_on_invalid_yaml() {
        let input = "invalid:\n  - broken\n - indentation";
        let err = parse(input).unwrap_err();
        assert!(matches!(err, ConfigError::Yaml(_)));
    }
}
