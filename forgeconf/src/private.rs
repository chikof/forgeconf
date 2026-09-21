//! Helpers called by `#[derive(Config)]` output. Not public API.

use std::collections::BTreeMap;
use std::mem;

use crate::suggest::suggest;
use crate::{ConfigError, ConfigNode, ErrorKind, FromNode, Origin, Shape, Table, Warning};

/// The table behind `node`; null stands for an empty table so a struct whose
/// fields all have defaults loads from nothing.
pub fn table<'a>(node: &'a ConfigNode, type_name: &'static str) -> Result<&'a Table, ConfigError> {
    static EMPTY: Table = BTreeMap::new();
    if node.is_null() {
        return Ok(&EMPTY);
    }
    node.as_table().ok_or_else(|| {
        ConfigError::mismatch(format!("table for {type_name}"), node.kind())
            .with_origin(node.origin())
    })
}

/// The table of a node being consumed: fields are moved out of it, leaving
/// null placeholders so the key set stays intact for [`unknown_keys`].
#[derive(Debug)]
pub struct OwnedTable {
    table: Table,
    origin: Option<Origin>,
}

impl OwnedTable {
    /// Same acceptance as [`table`]: a table, or null as an empty table.
    ///
    /// # Errors
    ///
    /// A mismatch against `type_name` for any other kind.
    pub fn new(node: ConfigNode, type_name: &'static str) -> Result<Self, ConfigError> {
        match node.into_table() {
            Ok((table, origin)) => Ok(Self { table, origin }),
            Err(node) => {
                table(&node, type_name)?;
                Ok(Self { table: Table::new(), origin: node.origin })
            },
        }
    }

    /// The entries, including the placeholders of fields already taken.
    #[must_use]
    pub fn table(&self) -> &Table {
        &self.table
    }
}

/// What an externally tagged enum node holds.
#[derive(Debug)]
#[allow(clippy::exhaustive_enums, reason = "matched exhaustively by derive output")]
pub enum Single {
    /// A table with exactly one entry.
    Entry(String, ConfigNode),
    /// Anything that is not a table.
    Other(ConfigNode),
}

/// Split an externally tagged enum node into its only entry.
///
/// # Errors
///
/// A mismatch when `node` is a table with any other number of entries.
pub fn single_entry(node: ConfigNode) -> Result<Single, ConfigError> {
    match node.into_table() {
        Ok((mut table, origin)) => {
            let len = table.len();
            if len == 1 {
                if let Some((key, value)) = table.pop_first() {
                    return Ok(Single::Entry(key, value));
                }
            }
            Err(ConfigError::mismatch(
                "table with exactly one key",
                format!("table with {len} keys"),
            )
            .with_origin(origin.as_ref()))
        },
        Err(node) => Ok(Single::Other(node)),
    }
}

/// First present, non-null child among `keys`; exact match, or ASCII
/// case-insensitive when `insensitive`.
fn lookup<'a>(table: &'a Table, keys: &[&str], insensitive: bool) -> Option<&'a ConfigNode> {
    keys.iter().find_map(|key| {
        let found = if insensitive {
            table
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, child)| child)
                .find(|child| !child.is_null())
        } else {
            table.get(*key)
        };
        found.filter(|child| !child.is_null())
    })
}

/// Read one struct field. `keys[0]` is the canonical key, the rest are aliases;
/// errors are recorded under the canonical key.
pub fn field<T: FromNode>(
    parent: &ConfigNode,
    table: &Table,
    keys: &[&str],
    insensitive: bool,
    default: Option<fn() -> T>,
    validate: impl Fn(&T) -> Result<(), ConfigError>,
) -> Result<T, ConfigError> {
    let name = keys.first().copied().unwrap_or_default();
    if let Some(child) = lookup(table, keys, insensitive) {
        let value = T::from_node(child).map_err(|e| e.at_key(name))?;
        validate(&value).map_err(|e| e.with_origin(child.origin()).at_key(name))?;
        return Ok(value);
    }
    if let Some(default) = default {
        let value = default();
        validate(&value).map_err(|e| e.at_key(name))?;
        return Ok(value);
    }
    T::from_missing().ok_or_else(|| {
        ConfigError::missing()
            .at_key(name)
            .with_origin(parent.origin())
    })
}

/// Move the first present, non-null child among `keys` out of `table`; same
/// matching as [`lookup`].
fn take(table: &mut Table, keys: &[&str], insensitive: bool) -> Option<ConfigNode> {
    keys.iter().find_map(|key| {
        let child = if insensitive {
            table
                .iter_mut()
                .filter(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, child)| child)
                .find(|child| !child.is_null())
        } else {
            table.get_mut(*key).filter(|child| !child.is_null())
        };
        child.map(|child| mem::replace(child, ConfigNode::null()))
    })
}

#[inline(always)]
fn field_owned_with<T: FromNode, V: Fn(&T) -> Result<(), ConfigError>>(
    parent: &mut OwnedTable,
    keys: &[&str],
    insensitive: bool,
    default: Option<fn() -> T>,
    validate: Option<V>,
) -> Result<T, ConfigError> {
    let name = keys.first().copied().unwrap_or_default();
    if let Some(child) = take(&mut parent.table, keys, insensitive) {
        let origin = validate.as_ref().and_then(|_| child.origin.clone());
        let value = T::from_node_owned(child).map_err(|e| e.at_key(name))?;
        if let Some(validate) = validate {
            validate(&value).map_err(|e| e.with_origin(origin.as_ref()).at_key(name))?;
        }
        return Ok(value);
    }
    if let Some(default) = default {
        let value = default();
        if let Some(validate) = validate {
            validate(&value).map_err(|e| e.at_key(name))?;
        }
        return Ok(value);
    }
    T::from_missing().ok_or_else(|| {
        ConfigError::missing()
            .at_key(name)
            .with_origin(parent.origin.as_ref())
    })
}

/// [`field`] for a table being consumed, without validators.
///
/// # Errors
///
/// The same errors as [`field`].
pub fn field_owned<T: FromNode>(
    parent: &mut OwnedTable,
    keys: &[&str],
    insensitive: bool,
    default: Option<fn() -> T>,
) -> Result<T, ConfigError> {
    field_owned_with(parent, keys, insensitive, default, None::<fn(&T) -> Result<(), ConfigError>>)
}

/// [`field`] for a table being consumed, with a validator.
///
/// # Errors
///
/// The same errors as [`field`].
pub fn field_owned_validated<T: FromNode>(
    parent: &mut OwnedTable,
    keys: &[&str],
    insensitive: bool,
    default: Option<fn() -> T>,
    validate: impl Fn(&T) -> Result<(), ConfigError>,
) -> Result<T, ConfigError> {
    field_owned_with(parent, keys, insensitive, default, Some(validate))
}

/// Accumulates field errors so one load reports every problem.
#[derive(Debug, Default)]
pub struct Collector {
    errors: Vec<ConfigError>,
}

impl Collector {
    /// An empty collector.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Unwrap an `Ok`, or record the error and return `None`.
    pub fn take<T>(&mut self, result: Result<T, ConfigError>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.push(error);
                None
            },
        }
    }

    /// Record an error.
    pub fn push(&mut self, error: ConfigError) {
        error.into_leaves(&mut self.errors);
    }

    /// `Ok(value)` when nothing was recorded, otherwise the combined error.
    pub fn finish<T>(self, value: T) -> Result<T, ConfigError> {
        if self.errors.is_empty() { Ok(value) } else { Err(self.into_error()) }
    }

    /// The recorded errors as one: the error itself when there is exactly one,
    /// [`ErrorKind::Multiple`] otherwise.
    #[must_use]
    pub fn into_error(mut self) -> ConfigError {
        if self.errors.len() == 1 {
            return self.errors.remove(0);
        }
        ConfigError::new(ErrorKind::Multiple(self.errors))
    }
}

/// Report every key of `table` missing from `known`, with a did-you-mean hint.
/// `insensitive` lists the names of case-insensitive fields; a key equal to one
/// of them ignoring ASCII case is known too.
pub fn unknown_keys(table: &Table, known: &[&str], insensitive: &[&str], errors: &mut Collector) {
    for (key, child) in table {
        if known.contains(&key.as_str())
            || insensitive
                .iter()
                .any(|name| name.eq_ignore_ascii_case(key))
        {
            continue;
        }
        let suggestion = suggest(key, known.iter().copied()).map(str::to_owned);
        errors.push(
            ConfigError::new(ErrorKind::UnknownKey { suggestion })
                .at_key(key)
                .with_origin(child.origin()),
        );
    }
}

/// The text of a scalar node, for enum variant names and tag values.
pub fn scalar<'a>(node: &'a ConfigNode, expected: &'static str) -> Result<&'a str, ConfigError> {
    node.as_str()
        .ok_or_else(|| ConfigError::mismatch(expected, node.kind()).with_origin(node.origin()))
}

/// Append a [`Warning`] for every present deprecated key under `node`.
pub fn collect_deprecations(node: &ConfigNode, shape: Shape, out: &mut Vec<Warning>) {
    walk(node, shape, &Path::Root, out);
}

/// The keys and indices from the root down to the node being visited, as a
/// chain of stack frames: it allocates nothing until a warning needs its text.
enum Path<'a> {
    Root,
    Key(&'a Path<'a>, &'a str),
    Index(&'a Path<'a>, usize),
}

impl Path<'_> {
    /// Same format as [`ConfigError::path`]: `servers[2].host`.
    fn render(&self) -> String {
        let mut chain = Vec::new();
        let mut at = self;
        while let Self::Key(parent, _) | Self::Index(parent, _) = at {
            chain.push(at);
            at = parent;
        }
        let mut out = String::new();
        for segment in chain.into_iter().rev() {
            match segment {
                Self::Key(_, key) => {
                    if !out.is_empty() {
                        out.push('.');
                    }
                    out.push_str(key);
                },
                Self::Index(_, index) => out.push_str(&format!("[{index}]")),
                Self::Root => {},
            }
        }
        out
    }
}

/// Whether values of `shape` can hold fields that carry a deprecation.
fn has_fields(shape: Shape) -> bool {
    match shape {
        Shape::Struct(_) => true,
        Shape::Optional(inner) | Shape::List(inner) | Shape::Map(inner) => has_fields(inner()),
        _ => false,
    }
}

fn walk(node: &ConfigNode, shape: Shape, path: &Path<'_>, out: &mut Vec<Warning>) {
    match shape {
        Shape::Struct(info) => {
            let Some(table) = node.as_table() else { return };
            for field in info.fields() {
                if field.is_flatten() {
                    walk(node, field.shape(), path, out);
                    continue;
                }
                if field.deprecation().is_none() && !has_fields(field.shape()) {
                    continue;
                }
                let Some((key, child)) = std::iter::once(field.key())
                    .chain(field.alias_keys().iter().copied())
                    .find_map(|key| {
                        table
                            .get(key)
                            .filter(|child| !child.is_null())
                            .map(|child| (key, child))
                    })
                else {
                    continue;
                };
                let path = Path::Key(path, key);
                if let Some(message) = field.deprecation() {
                    out.push(Warning::new(path.render(), message, child.origin().cloned()));
                }
                walk(child, field.shape(), &path, out);
            }
        },
        Shape::Optional(inner) => walk(node, inner(), path, out),
        Shape::List(inner) => {
            let Some(items) = node.as_array() else { return };
            let inner = inner();
            if !has_fields(inner) {
                return;
            }
            for (index, item) in items.iter().enumerate() {
                walk(item, inner, &Path::Index(path, index), out);
            }
        },
        Shape::Map(inner) => {
            let Some(table) = node.as_table() else { return };
            let inner = inner();
            if !has_fields(inner) {
                return;
            }
            for (key, value) in table {
                walk(value, inner, &Path::Key(path, key), out);
            }
        },
        _ => {},
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;
    use crate::{FieldInfo, Origin, StructInfo};

    fn at(line: u32) -> Origin {
        Origin::new("app.toml", line, 1)
    }

    fn node(pairs: &[(&str, ConfigNode)]) -> ConfigNode {
        ConfigNode::table(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), v.clone()))
                .collect(),
        )
    }

    fn read<T: FromNode>(
        node: &ConfigNode,
        keys: &[&str],
        insensitive: bool,
        default: Option<fn() -> T>,
    ) -> Result<T, ConfigError> {
        let table = table(node, "T")?;
        field(node, table, keys, insensitive, default, |_| Ok(()))
    }

    #[test]
    fn table_accepts_table_and_null() {
        assert!(table(&node(&[]), "S").unwrap().is_empty());
        assert!(table(&ConfigNode::null(), "S").unwrap().is_empty());
    }

    #[test]
    fn table_rejects_other_kinds_with_origin() {
        let err = table(&ConfigNode::scalar("x").with_origin(at(3)), "Server").unwrap_err();
        assert_eq!(err.to_string(), "expected table for Server, found \"scalar\" (app.toml:3:1)");
    }

    #[test]
    fn field_prefers_earlier_key_over_alias() {
        let n = node(&[("old", ConfigNode::scalar("1")), ("port", ConfigNode::scalar("2"))]);
        assert_eq!(read::<u16>(&n, &["port", "old"], false, None).unwrap(), 2);
        assert_eq!(read::<u16>(&n, &["gone", "old"], false, None).unwrap(), 1);
    }

    #[test]
    fn field_skips_null_and_falls_through_to_alias() {
        let n = node(&[("port", ConfigNode::null()), ("old", ConfigNode::scalar("7"))]);
        assert_eq!(read::<u16>(&n, &["port", "old"], false, None).unwrap(), 7);
    }

    #[test]
    fn field_insensitive_matches_ascii_case() {
        let n = node(&[("PoRt", ConfigNode::scalar("9"))]);
        assert_eq!(read::<u16>(&n, &["port"], true, None).unwrap(), 9);
        assert_eq!(
            read::<u16>(&n, &["port"], false, None)
                .unwrap_err()
                .to_string(),
            "port: missing value"
        );
    }

    #[test]
    fn field_default_then_from_missing_then_missing() {
        let empty = node(&[]);
        assert_eq!(read::<u16>(&empty, &["port"], false, Some(|| 80)).unwrap(), 80);
        assert_eq!(read::<Option<u16>>(&empty, &["port"], false, None).unwrap(), None);
        let null = node(&[("port", ConfigNode::null())]);
        assert_eq!(read::<Vec<u16>>(&null, &["port"], false, None).unwrap(), Vec::<u16>::new());

        let parent = node(&[]).with_origin(at(1));
        let err = read::<u16>(&parent, &["port", "p"], false, None).unwrap_err();
        assert_eq!(err.to_string(), "port: missing value (app.toml:1:1)");
    }

    #[test]
    fn field_default_wins_over_from_missing() {
        let empty = node(&[]);
        let value = read::<Option<u16>>(&empty, &["port"], false, Some(|| Some(1))).unwrap();
        assert_eq!(value, Some(1));
    }

    #[test]
    fn field_errors_carry_key_and_origin() {
        let n = node(&[("old", ConfigNode::scalar("abc").with_origin(at(4)))]);
        let err = read::<u16>(&n, &["port", "old"], false, None).unwrap_err();
        assert_eq!(err.path(), "port");
        assert_eq!(err.origin(), Some(&at(4)));
    }

    #[test]
    fn validator_runs_on_loaded_and_default_values() {
        let reject = |v: &u16| {
            if *v > 100 { Err(ConfigError::invalid("too big")) } else { Ok(()) }
        };
        let n = node(&[("port", ConfigNode::scalar("500").with_origin(at(2)))]);
        let t = table(&n, "T").unwrap();
        let err = field::<u16>(&n, t, &["port"], false, None, reject).unwrap_err();
        assert_eq!(err.to_string(), "port: too big (app.toml:2:1)");

        let empty = node(&[]);
        let t = table(&empty, "T").unwrap();
        let err = field::<u16>(&empty, t, &["port"], false, Some(|| 500), reject).unwrap_err();
        assert_eq!(err.to_string(), "port: too big");

        let t = table(&n, "T").unwrap();
        assert!(
            field::<Option<u16>>(&empty, table(&empty, "T").unwrap(), &["x"], false, None, |_| Ok(
                ()
            ))
            .is_ok()
        );
        assert!(field::<u16>(&n, t, &["port"], false, None, |_| Ok(())).is_ok());
    }

    #[test]
    fn collector_flattens_and_counts() {
        let mut c = Collector::new();
        assert_eq!(c.take::<u8>(Ok(1)), Some(1));
        assert_eq!(c.take::<u8>(Err(ConfigError::missing().at_key("a"))), None);
        c.push(ConfigError::new(ErrorKind::Multiple(vec![
            ConfigError::missing().at_key("b"),
            ConfigError::new(ErrorKind::Multiple(vec![ConfigError::missing().at_key("c")])),
        ])));
        let err = c.finish(()).unwrap_err();
        let ErrorKind::Multiple(children) = err.kind() else { panic!("expected Multiple") };
        assert_eq!(children.len(), 3);
        assert!(
            children
                .iter()
                .all(|e| !matches!(e.kind(), ErrorKind::Multiple(_)))
        );
        assert_eq!(
            err.to_string(),
            "3 configuration errors:\n  - a: missing value\n  - b: missing value\n  - c: missing \
             value"
        );
    }

    #[test]
    fn collector_single_error_is_unwrapped_and_empty_is_ok() {
        let mut c = Collector::new();
        c.push(ConfigError::missing().at_key("a"));
        let err = c.into_error();
        assert!(matches!(err.kind(), ErrorKind::Missing));
        assert_eq!(err.path(), "a");
        assert_eq!(Collector::new().finish(5).unwrap(), 5);
    }

    #[test]
    fn unknown_keys_suggest_and_carry_origin() {
        let n = node(&[
            ("prot", ConfigNode::scalar("1").with_origin(at(5))),
            ("port", ConfigNode::scalar("1")),
            ("zzzzzzzz", ConfigNode::scalar("1")),
        ]);
        let mut c = Collector::new();
        unknown_keys(table(&n, "T").unwrap(), &["host", "port"], &[], &mut c);
        let err = c.into_error();
        assert_eq!(
            err.to_string(),
            "2 configuration errors:\n  - prot: unknown key, did you mean \"port\"? \
             (app.toml:5:1)\n  - zzzzzzzz: unknown key"
        );
    }

    #[test]
    fn scalar_reads_text_or_reports_kind() {
        assert_eq!(scalar(&ConfigNode::scalar("a"), "string").unwrap(), "a");
        let err = scalar(&node(&[]).with_origin(at(2)), "variant name").unwrap_err();
        assert_eq!(err.to_string(), "expected variant name, found \"table\" (app.toml:2:1)");
    }

    #[test]
    fn unknown_keys_accept_insensitive_matches() {
        let n = node(&[
            ("HOST", ConfigNode::scalar("a")),
            ("Alias", ConfigNode::scalar("a")),
            ("PORT", ConfigNode::scalar("1")),
        ]);
        let mut c = Collector::new();
        unknown_keys(
            table(&n, "T").unwrap(),
            &["host", "alias", "port"],
            &["host", "alias"],
            &mut c,
        );
        let err = c.into_error();
        assert_eq!(err.to_string(), "PORT: unknown key");
    }

    fn leaf() -> Shape {
        Shape::Scalar("string")
    }

    static CHILD: StructInfo = StructInfo::new(
        "Child",
        &[FieldInfo::new("host", leaf).deprecated("is deprecated"), FieldInfo::new("port", leaf)],
    );
    static FLAT: StructInfo =
        StructInfo::new("Flat", &[FieldInfo::new("flat_old", leaf).deprecated("gone")]);

    fn child_shape() -> Shape {
        Shape::Struct(&CHILD)
    }
    fn list_shape() -> Shape {
        Shape::List(child_shape)
    }
    fn map_shape() -> Shape {
        Shape::Map(child_shape)
    }
    fn opt_shape() -> Shape {
        Shape::Optional(child_shape)
    }
    fn flat_shape() -> Shape {
        Shape::Struct(&FLAT)
    }

    static ROOT: StructInfo = StructInfo::new(
        "Root",
        &[
            FieldInfo::new("child", opt_shape),
            FieldInfo::new("items", list_shape),
            FieldInfo::new("by_name", map_shape),
            FieldInfo::new("old", leaf)
                .aliases(&["older"])
                .deprecated("use new"),
            FieldInfo::new("flat", flat_shape).flatten(),
        ],
    );

    fn warn(root: &ConfigNode) -> Vec<String> {
        let mut out = Vec::new();
        collect_deprecations(root, Shape::Struct(&ROOT), &mut out);
        out.iter().map(Warning::to_string).collect()
    }

    #[test]
    fn deprecations_cover_keys_aliases_lists_maps_and_flatten() {
        let host = |line| node(&[("host", ConfigNode::scalar("h").with_origin(at(line)))]);
        let root = node(&[
            ("older", ConfigNode::scalar("x").with_origin(at(1))),
            ("child", host(2)),
            ("items", ConfigNode::array(vec![node(&[("port", ConfigNode::scalar("1"))]), host(3)])),
            ("by_name", node(&[("a", host(4))])),
            ("flat_old", ConfigNode::scalar("y")),
        ]);
        assert_eq!(
            warn(&root),
            [
                "child.host: is deprecated (app.toml:2:1)",
                "items[1].host: is deprecated (app.toml:3:1)",
                "by_name.a.host: is deprecated (app.toml:4:1)",
                "older: use new (app.toml:1:1)",
                "flat_old: gone",
            ]
        );
    }

    #[test]
    fn deprecations_skip_absent_and_null_keys() {
        let root = node(&[
            ("old", ConfigNode::null()),
            ("child", node(&[("port", ConfigNode::scalar("1"))])),
        ]);
        assert!(warn(&root).is_empty());
        assert!(warn(&ConfigNode::scalar("not a table")).is_empty());
    }
}
