use std::collections::BTreeMap;

use toml::Value;

use crate::{ConfigError, ConfigNode};

pub fn parse(input: &str) -> Result<ConfigNode, ConfigError> {
    let value: Value = toml::from_str(input)?;
    Ok(convert(value))
}

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
