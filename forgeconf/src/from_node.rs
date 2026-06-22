use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::num::{
    NonZeroI8,
    NonZeroI16,
    NonZeroI32,
    NonZeroI64,
    NonZeroI128,
    NonZeroIsize,
    NonZeroU8,
    NonZeroU16,
    NonZeroU32,
    NonZeroU64,
    NonZeroU128,
    NonZeroUsize,
};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crate::duration::parse_duration;
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

macro_rules! impl_list {
    ($($ty:ident<$($bound:path),*>),* $(,)?) => {
        $(
            impl<T: FromNode $(+ $bound)*> FromNode for $ty<T> {
                fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
                    collect_items(node)
                }

                fn from_missing() -> Option<Self> {
                    Some($ty::new())
                }

                fn shape() -> Shape {
                    Shape::List(T::shape)
                }
            }
        )*
    };
}

impl_list!(Vec<>, VecDeque<>, HashSet<Eq, Hash>, BTreeSet<Ord>);

macro_rules! impl_map {
    ($($ty:ident<$($bound:path),*>),* $(,)?) => {
        $(
            impl<K: FromStr $(+ $bound)*, V: FromNode> FromNode for $ty<K, V> {
                fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
                    collect_entries(node)
                }

                fn from_missing() -> Option<Self> {
                    Some($ty::new())
                }

                fn shape() -> Shape {
                    Shape::Map(V::shape)
                }
            }
        )*
    };
}

impl_map!(HashMap<Eq, Hash>, BTreeMap<Ord>);

impl<T: FromNode, const N: usize> FromNode for [T; N] {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        let len = array_items(node)?.len();
        let mismatch = || {
            ConfigError::mismatch(format!("array of {N} items"), len.to_string())
                .with_origin(node.origin())
        };
        if len != N {
            return Err(mismatch());
        }
        collect_items::<T, Vec<T>>(node)?
            .try_into()
            .map_err(|_| mismatch())
    }

    fn shape() -> Shape {
        Shape::List(T::shape)
    }
}

macro_rules! impl_pointer {
    ($($ptr:ident),*) => {
        $(
            impl<T: FromNode> FromNode for $ptr<T> {
                fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
                    T::from_node(node).map($ptr::new)
                }

                fn from_missing() -> Option<Self> {
                    T::from_missing().map($ptr::new)
                }

                fn shape() -> Shape {
                    T::shape()
                }
            }
        )*
    };
}

impl_pointer!(Box, Arc);

impl FromNode for Box<str> {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        scalar_text(node, "string").map(Box::from)
    }

    fn shape() -> Shape {
        Shape::Scalar("string")
    }
}

impl FromNode for Arc<str> {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        scalar_text(node, "string").map(Arc::from)
    }

    fn shape() -> Shape {
        Shape::Scalar("string")
    }
}

impl FromNode for PathBuf {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        scalar_text(node, "path").map(PathBuf::from)
    }

    fn shape() -> Shape {
        Shape::Scalar("path")
    }
}

macro_rules! impl_from_str {
    ($label:literal: $($ty:ty),*) => {
        $(
            impl FromNode for $ty {
                fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
                    parse_scalar(node, $label)
                }

                fn shape() -> Shape {
                    Shape::Scalar($label)
                }
            }
        )*
    };
}

impl_from_str!("ip address": IpAddr, Ipv4Addr, Ipv6Addr);
impl_from_str!("socket address": SocketAddr, SocketAddrV4, SocketAddrV6);

impl FromNode for Duration {
    fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
        let text = scalar_text(node, "duration")?;
        parse_duration(text)
            .ok_or_else(|| ConfigError::mismatch("duration", text).with_origin(node.origin()))
    }

    fn shape() -> Shape {
        Shape::Scalar("duration")
    }
}

macro_rules! impl_non_zero {
    ($($nz:ident => $int:ident),* $(,)?) => {
        $(
            impl FromNode for $nz {
                fn from_node(node: &ConfigNode) -> Result<Self, ConfigError> {
                    let value: $int = parse_scalar(node, stringify!($int))?;
                    $nz::new(value).ok_or_else(|| {
                        let text = node.as_str().unwrap_or_default();
                        ConfigError::mismatch(concat!("non-zero ", stringify!($int)), text)
                            .with_origin(node.origin())
                    })
                }

                fn shape() -> Shape {
                    Shape::Scalar(stringify!($int))
                }
            }
        )*
    };
}

impl_non_zero!(
    NonZeroU8 => u8, NonZeroU16 => u16, NonZeroU32 => u32, NonZeroU64 => u64,
    NonZeroU128 => u128, NonZeroUsize => usize,
    NonZeroI8 => i8, NonZeroI16 => i16, NonZeroI32 => i32, NonZeroI64 => i64,
    NonZeroI128 => i128, NonZeroIsize => isize,
);

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

/// Items of an array node; null is an empty list.
fn array_items(node: &ConfigNode) -> Result<&[ConfigNode], ConfigError> {
    match node.value() {
        Value::Array(items) => Ok(items),
        Value::Null => Ok(&[]),
        _ => Err(ConfigError::mismatch("array", node.kind()).with_origin(node.origin())),
    }
}

fn collect_items<T: FromNode, C: FromIterator<T>>(node: &ConfigNode) -> Result<C, ConfigError> {
    array_items(node)?
        .iter()
        .enumerate()
        .map(|(index, item)| T::from_node(item).map_err(|err| err.at_index(index)))
        .collect()
}

fn collect_entries<K, V, C>(node: &ConfigNode) -> Result<C, ConfigError>
where
    K: FromStr,
    V: FromNode,
    C: FromIterator<(K, V)>,
{
    let table = match node.value() {
        Value::Table(table) => table,
        Value::Null => return Ok(std::iter::empty().collect()),
        _ => return Err(ConfigError::mismatch("table", node.kind()).with_origin(node.origin())),
    };
    table
        .iter()
        .map(|(key, child)| {
            let parsed = key.parse::<K>().map_err(|_| {
                let expected = std::any::type_name::<K>()
                    .rsplit("::")
                    .next()
                    .unwrap_or("key");
                ConfigError::mismatch(expected, key.as_str())
                    .at_key(key)
                    .with_origin(child.origin())
            })?;
            let value = V::from_node(child).map_err(|err| err.at_key(key))?;
            Ok((parsed, value))
        })
        .collect()
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

    fn table_of(entries: &[(&str, &str)]) -> ConfigNode {
        ConfigNode::table(
            entries
                .iter()
                .map(|(key, value)| ((*key).to_owned(), ConfigNode::scalar(*value)))
                .collect(),
        )
    }

    #[test]
    fn hash_map_from_table() {
        let map = HashMap::<String, u16>::from_node(&table_of(&[("a", "1"), ("b", "2")])).unwrap();
        assert_eq!(map.len(), 2);
        assert_eq!(map["a"], 1);
        assert_eq!(map["b"], 2);
        assert_eq!(HashMap::<String, u16>::from_missing(), Some(HashMap::new()));
        assert!(
            HashMap::<String, u16>::from_node(&ConfigNode::null())
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn btree_map_key_parse_failure_is_at_key() {
        let node = table_of(&[("x", "hello")]);
        let err = BTreeMap::<u16, String>::from_node(&node)
            .unwrap_err()
            .at_key("ports");
        assert_eq!(err.path(), "ports.x");
        assert_eq!(err.to_string(), "ports.x: expected u16, found \"x\"");
    }

    #[test]
    fn map_value_error_is_at_key() {
        let err = BTreeMap::<String, u8>::from_node(&table_of(&[("a", "300")])).unwrap_err();
        assert_eq!(err.to_string(), "a: expected u8, found \"300\"");
    }

    #[test]
    fn map_from_scalar_is_mismatch() {
        let err = BTreeMap::<String, u8>::from_node(&ConfigNode::scalar("x")).unwrap_err();
        assert_eq!(err.to_string(), "expected table, found \"scalar\"");
    }

    #[test]
    fn sets_and_deque() {
        let node = ConfigNode::array(vec![
            ConfigNode::scalar("2"),
            ConfigNode::scalar("1"),
            ConfigNode::scalar("2"),
        ]);
        assert_eq!(BTreeSet::<u8>::from_node(&node).unwrap(), BTreeSet::from([1, 2]));
        assert_eq!(HashSet::<u8>::from_node(&node).unwrap(), HashSet::from([1, 2]));
        assert_eq!(VecDeque::<u8>::from_node(&node).unwrap(), VecDeque::from([2, 1, 2]));
        assert_eq!(BTreeSet::<u8>::from_missing(), Some(BTreeSet::new()));
    }

    #[test]
    fn fixed_array_checks_length() {
        let two = ConfigNode::array(vec![ConfigNode::scalar("1"), ConfigNode::scalar("2")]);
        let err = <[u8; 3]>::from_node(&two).unwrap_err();
        assert_eq!(err.to_string(), "expected array of 3 items, found \"2\"");
        let three = ConfigNode::array(vec![
            ConfigNode::scalar("1"),
            ConfigNode::scalar("2"),
            ConfigNode::scalar("3"),
        ]);
        assert_eq!(<[u8; 3]>::from_node(&three).unwrap(), [1, 2, 3]);
    }

    #[test]
    fn non_zero_rejects_zero() {
        let err = NonZeroU16::from_node(&ConfigNode::scalar("0")).unwrap_err();
        assert_eq!(err.to_string(), "expected non-zero u16, found \"0\"");
        assert_eq!(
            NonZeroU16::from_node(&ConfigNode::scalar("7"))
                .unwrap()
                .get(),
            7
        );
        assert_eq!(
            NonZeroI8::from_node(&ConfigNode::scalar("-3"))
                .unwrap()
                .get(),
            -3
        );
        let err = NonZeroU8::from_node(&ConfigNode::scalar("abc")).unwrap_err();
        assert_eq!(err.to_string(), "expected u8, found \"abc\"");
        assert!(matches!(NonZeroU16::shape(), Shape::Scalar("u16")));
    }

    #[test]
    fn duration_cases() {
        let parse = |text: &str| Duration::from_node(&ConfigNode::scalar(text));
        assert_eq!(parse("1h30m").unwrap(), Duration::from_secs(5400));
        assert_eq!(parse("250ms").unwrap(), Duration::from_millis(250));
        assert_eq!(parse("1.5").unwrap(), Duration::from_millis(1500));
        assert_eq!(parse("5x").unwrap_err().to_string(), "expected duration, found \"5x\"");
        assert!(parse("").is_err());
        assert!(parse("99999999999999999999d").is_err());
    }

    #[test]
    fn path_and_strings() {
        assert_eq!(
            PathBuf::from_node(&ConfigNode::scalar("/etc/app.toml")).unwrap(),
            PathBuf::from("/etc/app.toml")
        );
        assert_eq!(&*Box::<str>::from_node(&ConfigNode::scalar("hi")).unwrap(), "hi");
        assert_eq!(&*Arc::<str>::from_node(&ConfigNode::scalar("hi")).unwrap(), "hi");
        assert!(matches!(PathBuf::shape(), Shape::Scalar("path")));
    }

    #[test]
    fn network_addresses() {
        let addr = SocketAddr::from_node(&ConfigNode::scalar("127.0.0.1:8080")).unwrap();
        assert_eq!(addr, SocketAddr::from(([127, 0, 0, 1], 8080)));
        assert_eq!(
            IpAddr::from_node(&ConfigNode::scalar("::1")).unwrap(),
            IpAddr::from(Ipv6Addr::LOCALHOST)
        );
        let err = Ipv4Addr::from_node(&ConfigNode::scalar("nope")).unwrap_err();
        assert_eq!(err.to_string(), "expected ip address, found \"nope\"");
        let err = SocketAddrV4::from_node(&ConfigNode::scalar("1.2.3.4")).unwrap_err();
        assert_eq!(err.to_string(), "expected socket address, found \"1.2.3.4\"");
    }

    #[test]
    fn box_and_arc_delegate() {
        assert_eq!(*Box::<u8>::from_node(&ConfigNode::scalar("4")).unwrap(), 4);
        assert_eq!(*Arc::<Vec<u8>>::from_missing().unwrap(), Vec::<u8>::new());
        assert!(Box::<u8>::from_missing().is_none());
        assert!(matches!(Arc::<u8>::shape(), Shape::Scalar("u8")));
        assert!(matches!(BTreeMap::<String, u8>::shape(), Shape::Map(_)));
    }
}
