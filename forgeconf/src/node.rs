use std::collections::BTreeMap;

use crate::Origin;

/// Map of named child nodes.
pub type Table = BTreeMap<String, ConfigNode>;

/// A node of a configuration tree: a [`Value`] plus where it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigNode {
    pub(crate) value: Value,
    pub(crate) origin: Option<Origin>,
}

const _: () = assert!(size_of::<ConfigNode>() == 56);

/// The payload of a [`ConfigNode`].
///
/// Scalars stay text; the target type parses them.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Value {
    /// Absence of a value.
    Null,
    /// A leaf value stored as text.
    Scalar(String),
    /// An ordered list of child nodes.
    Array(Vec<ConfigNode>),
    /// A map of named child nodes.
    Table(Table),
}

const _: () = assert!(size_of::<Value>() == 32);

impl ConfigNode {
    /// Wrap `value` in a node without an origin.
    #[must_use]
    pub fn new(value: Value) -> Self {
        Self { value, origin: None }
    }

    /// A node holding [`Value::Null`].
    #[must_use]
    pub fn null() -> Self {
        Self::new(Value::Null)
    }

    /// A node holding [`Value::Scalar`].
    pub fn scalar(s: impl Into<String>) -> Self {
        Self::new(Value::Scalar(s.into()))
    }

    /// A node holding [`Value::Array`].
    #[must_use]
    pub fn array(items: Vec<ConfigNode>) -> Self {
        Self::new(Value::Array(items))
    }

    /// A node holding [`Value::Table`].
    #[must_use]
    pub fn table(t: Table) -> Self {
        Self::new(Value::Table(t))
    }

    /// Attach the place this node came from.
    #[must_use]
    pub fn with_origin(mut self, origin: Origin) -> Self {
        self.origin = Some(origin);
        self
    }

    /// The payload.
    #[must_use]
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Consume the node and return its payload.
    #[must_use]
    pub fn into_value(self) -> Value {
        self.value
    }

    /// Where this node came from, when known.
    #[must_use]
    pub fn origin(&self) -> Option<&Origin> {
        self.origin.as_ref()
    }

    /// The text of a scalar node.
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Value::Scalar(text) => Some(text),
            _ => None,
        }
    }

    /// The entries of a table node.
    #[must_use]
    pub fn as_table(&self) -> Option<&Table> {
        match &self.value {
            Value::Table(table) => Some(table),
            _ => None,
        }
    }

    /// The items of an array node.
    #[must_use]
    pub fn as_array(&self) -> Option<&[ConfigNode]> {
        match &self.value {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    /// Whether the node holds [`Value::Null`].
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self.value, Value::Null)
    }

    /// Table lookup; `None` for missing keys and non-table nodes.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&ConfigNode> {
        self.as_table()?.get(key)
    }

    /// Textual identifier of the payload kind: `"null"`, `"scalar"`, `"array"`
    /// or `"table"`.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self.value {
            Value::Null => "null",
            Value::Scalar(_) => "scalar",
            Value::Array(_) => "array",
            Value::Table(_) => "table",
        }
    }

    /// The text of a scalar node, or the node back when it is not a scalar.
    pub(crate) fn into_scalar(self) -> Result<String, Self> {
        match self.value {
            Value::Scalar(text) => Ok(text),
            value => Err(Self { value, origin: self.origin }),
        }
    }

    /// The items of an array node, or the node back when it is not an array.
    pub(crate) fn into_array(self) -> Result<Vec<ConfigNode>, Self> {
        match self.value {
            Value::Array(items) => Ok(items),
            value => Err(Self { value, origin: self.origin }),
        }
    }

    /// The entries of a table node, or the node back when it is not a table.
    pub(crate) fn into_table(self) -> Result<(Table, Option<Origin>), Self> {
        match self.value {
            Value::Table(table) => Ok((table, self.origin)),
            value => Err(Self { value, origin: self.origin }),
        }
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;

    #[test]
    fn constructors_set_value_and_no_origin() {
        let node = ConfigNode::scalar("8080");
        assert_eq!(node.value(), &Value::Scalar("8080".into()));
        assert_eq!(node.origin(), None);
        assert!(ConfigNode::null().is_null());
    }

    #[test]
    fn accessors_match_variant() {
        let mut table = Table::new();
        table.insert("port".into(), ConfigNode::scalar("1"));
        let node = ConfigNode::table(table);
        assert_eq!(node.get("port").and_then(ConfigNode::as_str), Some("1"));
        assert_eq!(node.get("other"), None);
        assert_eq!(node.as_str(), None);
        let array = ConfigNode::array(vec![ConfigNode::null()]);
        assert_eq!(array.as_array().map(<[_]>::len), Some(1));
        assert_eq!(array.get("x"), None);
    }

    #[test]
    fn kind_names_each_variant() {
        assert_eq!(ConfigNode::null().kind(), "null");
        assert_eq!(ConfigNode::scalar("a").kind(), "scalar");
        assert_eq!(ConfigNode::array(Vec::new()).kind(), "array");
        assert_eq!(ConfigNode::table(Table::new()).kind(), "table");
    }

    #[test]
    fn with_origin_attaches_origin() {
        let node = ConfigNode::null().with_origin(Origin::new("a.toml", 1, 2));
        assert_eq!(node.origin(), Some(&Origin::new("a.toml", 1, 2)));
        assert_eq!(node.into_value(), Value::Null);
    }
}
