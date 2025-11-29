use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use crate::ConfigError;

/// Representation of a configuration tree.
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigNode {
    Table(BTreeMap<String, ConfigNode>),
    Array(Vec<ConfigNode>),
    Scalar(String),
    Null,
}

impl ConfigNode {
    /// Create an empty table node.
    pub fn empty_table() -> Self {
        ConfigNode::Table(BTreeMap::new())
    }

    /// Returns a reference to the underlying map when the node is a table.
    pub fn as_table(&self) -> Option<&BTreeMap<String, ConfigNode>> {
        match self {
            ConfigNode::Table(map) => Some(map),
            _ => None,
        }
    }

    /// Returns a mutable copy of the table content.
    pub fn to_owned_table(&self) -> Result<BTreeMap<String, ConfigNode>, ConfigError> {
        match self {
            ConfigNode::Table(map) => Ok(map.clone()),
            _ => Err(ConfigError::mismatch("root", "table", self.to_string())),
        }
    }

    /// Returns a textual identifier for the type of the node.
    pub fn kind(&self) -> &'static str {
        match self {
            ConfigNode::Table(_) => "table",
            ConfigNode::Array(_) => "array",
            ConfigNode::Scalar(_) => "scalar",
            ConfigNode::Null => "null",
        }
    }
}

impl Display for ConfigNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ConfigNode::Scalar(value) => write!(f, "{value}"),
            ConfigNode::Null => write!(f, "null"),
            ConfigNode::Array(items) => {
                let mut first = true;
                write!(f, "[")?;
                for item in items {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            },
            ConfigNode::Table(map) => {
                let mut first = true;
                write!(f, "{{")?;
                for (k, v) in map {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            },
        }
    }
}

/// Trait implemented by types that can be created from a [`ConfigNode`].
pub trait FromNode: Sized {
    fn from_node(node: &ConfigNode, key: &str) -> Result<Self, ConfigError>;
}

impl FromNode for ConfigNode {
    fn from_node(node: &ConfigNode, _key: &str) -> Result<Self, ConfigError> {
        Ok(node.clone())
    }
}

impl FromNode for String {
    fn from_node(node: &ConfigNode, key: &str) -> Result<Self, ConfigError> {
        match node {
            ConfigNode::Scalar(value) => Ok(value.clone()),
            ConfigNode::Null => Err(ConfigError::missing(key)),
            other => Err(ConfigError::mismatch(key, "string", other.kind())),
        }
    }
}

impl FromNode for bool {
    fn from_node(node: &ConfigNode, key: &str) -> Result<Self, ConfigError> {
        parse_scalar(node, key)
    }
}

macro_rules! impl_numeric_node {
    ($($ty:ty),* $(,)?) => {
        $(
            impl FromNode for $ty {
                fn from_node(node: &ConfigNode, key: &str) -> Result<Self, ConfigError> {
                    parse_scalar(node, key)
                }
            }
        )*
    };
}

impl_numeric_node!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

impl FromNode for char {
    fn from_node(node: &ConfigNode, key: &str) -> Result<Self, ConfigError> {
        let string = String::from_node(node, key)?;
        let mut chars = string.chars();
        match (chars.next(), chars.next()) {
            (Some(ch), None) => Ok(ch),
            _ => Err(ConfigError::mismatch(key, "character", string)),
        }
    }
}

impl<T> FromNode for Vec<T>
where
    T: FromNode,
{
    fn from_node(node: &ConfigNode, key: &str) -> Result<Self, ConfigError> {
        match node {
            ConfigNode::Array(items) => items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let nested_key = format!("{key}[{index}]");
                    T::from_node(item, &nested_key)
                })
                .collect(),
            ConfigNode::Null => Ok(Vec::new()),
            other => Err(ConfigError::mismatch(key, "array", other.kind())),
        }
    }
}

impl<T> FromNode for Option<T>
where
    T: FromNode,
{
    fn from_node(node: &ConfigNode, key: &str) -> Result<Self, ConfigError> {
        match node {
            ConfigNode::Null => Ok(None),
            other => T::from_node(other, key).map(Some),
        }
    }
}

fn parse_scalar<T>(node: &ConfigNode, key: &str) -> Result<T, ConfigError>
where
    T: FromStr,
    <T as FromStr>::Err: ToString,
{
    match node {
        ConfigNode::Scalar(value) => value
            .parse::<T>()
            .map_err(|_| ConfigError::mismatch(key, std::any::type_name::<T>(), value.clone())),
        ConfigNode::Null => Err(ConfigError::missing(key)),
        other => Err(ConfigError::mismatch(key, std::any::type_name::<T>(), other.kind())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vec_from_null_becomes_empty() {
        let node = ConfigNode::Null;
        let parsed = Vec::<u8>::from_node(&node, "numbers").unwrap();
        assert!(parsed.is_empty());
    }

    #[test]
    fn option_from_scalar_resolves() {
        let node = ConfigNode::Scalar("42".into());
        let parsed = Option::<u32>::from_node(&node, "answer").unwrap();
        assert_eq!(parsed, Some(42));
    }

    #[test]
    fn string_from_null_is_missing() {
        let node = ConfigNode::Null;
        let err = String::from_node(&node, "name").unwrap_err();
        assert!(matches!(err, ConfigError::MissingValue(key) if key == "name"));
    }

    #[test]
    fn to_owned_table_rejects_non_table() {
        let node = ConfigNode::Scalar("value".into());
        let err = node
            .to_owned_table()
            .unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch {
                field,
                expected,
                found
            } if field == "root" && expected == "table" && found == "value"
        ));
    }

    #[test]
    fn bool_from_scalar_parses() {
        let node = ConfigNode::Scalar("true".into());
        let parsed = bool::from_node(&node, "flag").unwrap();
        assert!(parsed);
    }

    #[test]
    fn numeric_parse_error_is_reported() {
        let node = ConfigNode::Scalar("not-a-number".into());
        let err = u16::from_node(&node, "size").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch {
                field,
                expected,
                ..
            } if field == "size" && expected.contains("u16")
        ));
    }

    #[test]
    fn char_from_long_string_is_rejected() {
        let node = ConfigNode::Scalar("abc".into());
        let err = char::from_node(&node, "initial").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch { field, expected, .. }
                if field == "initial" && expected == "character"
        ));
    }

    #[test]
    fn vec_from_non_array_errors() {
        let node = ConfigNode::Scalar("nope".into());
        let err = Vec::<u8>::from_node(&node, "values").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch { field, expected, .. }
                if field == "values" && expected == "array"
        ));
    }
}
