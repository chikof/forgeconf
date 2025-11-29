use std::collections::BTreeMap;

use jzon::JsonValue;

use crate::{ConfigError, ConfigNode};

pub fn parse(input: &str) -> Result<ConfigNode, ConfigError> {
    let value = jzon::parse(input)?;
    Ok(convert(value))
}

fn convert(value: JsonValue) -> ConfigNode {
    match value {
        JsonValue::Null => ConfigNode::Null,
        JsonValue::Boolean(flag) => ConfigNode::Scalar(flag.to_string()),
        JsonValue::Number(num) => ConfigNode::Scalar(num.to_string()),
        JsonValue::String(text) => ConfigNode::Scalar(text),
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
        JsonValue::Short(short) => ConfigNode::Scalar(short.to_string()),
    }
}
