use std::collections::BTreeMap;

use yaml_rust2::{Yaml, YamlLoader};

use crate::{ConfigError, ConfigNode};

pub fn parse(input: &str) -> Result<ConfigNode, ConfigError> {
    let docs = YamlLoader::load_from_str(input)?;
    let document = docs
        .into_iter()
        .next()
        .unwrap_or(Yaml::Null);
    Ok(convert(document))
}

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

fn scalar_key(key: Yaml) -> String {
    match key {
        Yaml::String(text) => text,
        Yaml::Integer(num) => num.to_string(),
        Yaml::Real(text) => text,
        other => format!("{other:?}"),
    }
}
