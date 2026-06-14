use yaml_rust2::{Yaml, YamlLoader};

use crate::{ConfigError, ConfigNode, ErrorKind, Table};

/// Parse YAML content into a ConfigNode tree.
/// If multiple YAML documents are present, only the first is used. An empty
/// document is an empty table.
pub(crate) fn parse(input: &str, _source: &str) -> Result<ConfigNode, ConfigError> {
    let docs = YamlLoader::load_from_str(input).map_err(|source| {
        ConfigError::new(ErrorKind::Syntax {
            format: "yaml",
            message: source.to_string(),
        })
    })?;
    Ok(docs
        .into_iter()
        .next()
        .map_or_else(|| ConfigNode::table(Table::new()), convert))
}

/// Convert a YAML value into a ConfigNode.
fn convert(value: Yaml) -> ConfigNode {
    match value {
        Yaml::Null | Yaml::BadValue | Yaml::Alias(_) => ConfigNode::null(),
        Yaml::Real(text) | Yaml::String(text) => ConfigNode::scalar(text),
        Yaml::Boolean(flag) => ConfigNode::scalar(flag.to_string()),
        Yaml::Integer(num) => ConfigNode::scalar(num.to_string()),
        Yaml::Array(values) => ConfigNode::array(values.into_iter().map(convert).collect()),
        Yaml::Hash(map) => ConfigNode::table(
            map.into_iter()
                .map(|(key, value)| (scalar_key(key), convert(value)))
                .collect::<Table>(),
        ),
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
        let node = parse(input, "<string>").unwrap();
        let table = node.as_table().unwrap();
        assert_eq!(table.get("port").unwrap().as_str(), Some("8080"));
        assert_eq!(table.get("host").unwrap().as_str(), Some("localhost"));
    }

    #[test]
    fn parses_nested_maps() {
        let input = r#"
            database:
              host: localhost
              port: 5432
        "#;
        let node = parse(input, "<string>").unwrap();
        let table = node.as_table().unwrap();
        let db = table.get("database").unwrap().as_table().unwrap();
        assert_eq!(db.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(db.get("port").unwrap().as_str(), Some("5432"));
    }

    #[test]
    fn parses_sequences() {
        let input = r#"
            tags:
              - api
              - production
              - v2
        "#;
        let node = parse(input, "<string>").unwrap();
        let table = node.as_table().unwrap();
        let items = table.get("tags").unwrap().as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].as_str(), Some("api"));
    }

    #[test]
    fn empty_document_is_empty_table() {
        assert_eq!(parse("", "<string>").unwrap(), ConfigNode::table(Table::new()));
        assert_eq!(
            parse("# only a comment\n", "<string>").unwrap(),
            ConfigNode::table(Table::new())
        );
    }

    #[test]
    fn handles_null_values() {
        let input = "key: null";
        let node = parse(input, "<string>").unwrap();
        let table = node.as_table().unwrap();
        assert!(table.get("key").unwrap().is_null());
    }

    #[test]
    fn returns_error_on_invalid_yaml() {
        let input = "invalid:\n  - broken\n - indentation";
        let err = parse(input, "<string>").unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Syntax { format: "yaml", .. }));
    }
}
