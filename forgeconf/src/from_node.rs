use std::str::FromStr;

use crate::{ConfigError, ConfigNode, Shape, Value};

/// Trait implemented by types that can be created from a [`ConfigNode`].
///
/// Errors carry the node's origin. Path segments are added by the caller with
/// [`ConfigError::at_key`] / [`ConfigError::at_index`], so nothing allocates on
/// success.
pub trait FromNode: Sized {
    /// Convert `node` into `Self`.
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError>;

    /// Value to use when the key is absent or null; `None` means the key is
    /// required.
    fn from_missing() -> Option<Self> {
        None
    }

    /// Static description of what this type accepts.
    fn shape() -> Shape;
}

impl FromNode for ConfigNode {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        Ok(node.clone())
    }

    fn shape() -> Shape {
        Shape::Any
    }
}

impl FromNode for String {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        scalar_text(node, "string").map(str::to_owned)
    }

    fn shape() -> Shape {
        Shape::Scalar("string")
    }
}

impl FromNode for bool {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        let text = scalar_text(node, "bool")?;
        let text = text.trim();
        let accepts = |words: [&str; 4]| words.iter().any(|word| text.eq_ignore_ascii_case(word));
        if accepts(["true", "1", "yes", "on"]) {
            Ok(true)
        } else if accepts(["false", "0", "no", "off"]) {
            Ok(false)
        } else {
            Err(ConfigError::mismatch("bool", text).with_origin(node.origin()))
        }
    }

    fn shape() -> Shape {
        Shape::Bool
    }
}

macro_rules! impl_parsed_scalar {
    ($($ty:ident),* $(,)?) => {
        $(
            impl FromNode for $ty {
                fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
                    parse_scalar(node, stringify!($ty))
                }

                fn shape() -> Shape {
                    Shape::Scalar(stringify!($ty))
                }
            }
        )*
    };
}

impl_parsed_scalar!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64);

impl FromNode for char {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        let text = scalar_text(node, "char")?;
        let mut chars = text.chars();
        match (chars.next(), chars.next()) {
            (Some(ch), None) => Ok(ch),
            _ => Err(ConfigError::mismatch("char", text).with_origin(node.origin())),
        }
    }

    fn shape() -> Shape {
        Shape::Scalar("char")
    }
}

impl<T: FromNode> FromNode for Vec<T> {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        match node.value() {
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(index, item)| T::from_node(item).map_err(|err| err.at_index(index)))
                .collect(),
            Value::Null => Ok(Vec::new()),
            _ => Err(ConfigError::mismatch("array", node.kind()).with_origin(node.origin())),
        }
    }

    fn from_missing() -> Option<Self> {
        Some(Vec::new())
    }

    fn shape() -> Shape {
        Shape::List(T::shape)
    }
}

impl<T: FromNode> FromNode for Option<T> {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        if node.is_null() { Ok(None) } else { T::from_node(node).map(Some) }
    }

    fn from_missing() -> Option<Self> {
        Some(None)
    }

    fn shape() -> Shape {
        Shape::Optional(T::shape)
    }
}

/// Text of a scalar node; null is a missing value, other kinds a mismatch
/// against `expected`.
fn scalar_text<'a>(node: &'a ConfigNode, expected: &'static str) -> Result<&'a str, ConfigError> {
    match node.value() {
        Value::Scalar(text) => Ok(text),
        Value::Null => Err(ConfigError::missing().with_origin(node.origin())),
        _ => Err(ConfigError::mismatch(expected, node.kind()).with_origin(node.origin())),
    }
}

fn parse_scalar<T: FromStr>(node: &ConfigNode, expected: &'static str) -> Result<T, ConfigError> {
    let text = scalar_text(node, expected)?;
    text.trim()
        .parse()
        .map_err(|_| ConfigError::mismatch(expected, text).with_origin(node.origin()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{ErrorKind, Origin};

    #[test]
    fn vec_from_null_is_empty() {
        assert_eq!(Vec::<u8>::from_node(&ConfigNode::null()).unwrap(), Vec::<u8>::new());
        assert_eq!(Vec::<u8>::from_missing(), Some(Vec::new()));
    }

    #[test]
    fn option_from_scalar_is_some() {
        let node = ConfigNode::scalar("42");
        assert_eq!(Option::<u32>::from_node(&node).unwrap(), Some(42));
    }

    #[test]
    fn option_from_null_is_none() {
        assert_eq!(Option::<u32>::from_node(&ConfigNode::null()).unwrap(), None);
        assert_eq!(Option::<u32>::from_missing(), Some(None));
        assert_eq!(u32::from_missing(), None);
    }

    #[test]
    fn string_from_null_is_missing() {
        let err = String::from_node(&ConfigNode::null()).unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Missing));
    }

    #[test]
    fn string_from_table_is_mismatch() {
        let err = String::from_node(&ConfigNode::table(Default::default())).unwrap_err();
        assert_eq!(err.to_string(), "expected string, found \"table\"");
    }

    #[test]
    fn numeric_trims_before_parsing() {
        assert_eq!(u16::from_node(&ConfigNode::scalar(" 8080 ")).unwrap(), 8080);
        assert_eq!(f64::from_node(&ConfigNode::scalar("1.5")).unwrap(), 1.5);
    }

    #[test]
    fn numeric_rejects_garbage_with_origin() {
        let node = ConfigNode::scalar("abc").with_origin(Origin::new("app.toml", 3, 8));
        let err = u16::from_node(&node)
            .unwrap_err()
            .at_key("port")
            .at_key("server");
        assert_eq!(err.to_string(), "server.port: expected u16, found \"abc\" (app.toml:3:8)");
    }

    #[test]
    fn bool_accepts_on() {
        assert!(bool::from_node(&ConfigNode::scalar("On")).unwrap());
        assert!(!bool::from_node(&ConfigNode::scalar(" NO ")).unwrap());
        assert!(bool::from_node(&ConfigNode::scalar("1")).unwrap());
        assert!(!bool::from_node(&ConfigNode::scalar("off")).unwrap());
    }

    #[test]
    fn bool_rejects_maybe() {
        let err = bool::from_node(&ConfigNode::scalar("maybe")).unwrap_err();
        assert_eq!(err.to_string(), "expected bool, found \"maybe\"");
    }

    #[test]
    fn char_rejects_multi_char_string() {
        let err = char::from_node(&ConfigNode::scalar("abc")).unwrap_err();
        assert_eq!(err.to_string(), "expected char, found \"abc\"");
        assert_eq!(char::from_node(&ConfigNode::scalar("x")).unwrap(), 'x');
    }

    #[test]
    fn vec_from_scalar_is_mismatch() {
        let err = Vec::<u8>::from_node(&ConfigNode::scalar("nope")).unwrap_err();
        assert_eq!(err.to_string(), "expected array, found \"scalar\"");
    }

    #[test]
    fn vec_error_path_has_index() {
        let node = ConfigNode::array(vec![
            ConfigNode::scalar("1"),
            ConfigNode::scalar("2"),
            ConfigNode::scalar("x"),
        ]);
        let err = Vec::<u8>::from_node(&node).unwrap_err().at_key("ports");
        assert_eq!(err.path(), "ports[2]");
        assert_eq!(err.to_string(), "ports[2]: expected u8, found \"x\"");
    }

    #[test]
    fn config_node_passes_through() {
        let node = ConfigNode::scalar("a");
        assert_eq!(ConfigNode::from_node(&node).unwrap(), node);
        assert!(matches!(ConfigNode::shape(), Shape::Any));
    }

    #[test]
    fn shapes_describe_types() {
        assert!(matches!(String::shape(), Shape::Scalar("string")));
        assert!(matches!(bool::shape(), Shape::Bool));
        assert!(matches!(u16::shape(), Shape::Scalar("u16")));
        assert!(matches!(Vec::<u16>::shape(), Shape::List(_)));
        assert!(matches!(Option::<u16>::shape(), Shape::Optional(_)));
    }
}
