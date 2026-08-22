//! Environment variables mapped onto the shape's leaves.

use std::ffi::OsString;

use super::walk;
use crate::{ConfigError, ConfigNode, Origin, Shape, Table};

/// A node tree with one scalar per leaf whose variable `vars` knows.
///
/// `vars` returns `None` for an unset variable and `Err` for one that is not
/// valid UTF-8.
pub(crate) fn env_node(
    shape: Shape,
    vars: &dyn Fn(&str) -> Option<Result<String, OsString>>,
) -> Result<ConfigNode, ConfigError> {
    let mut root = Table::new();
    walk::visit(shape, true, &mut |path, _, leaf_shape, env| {
        let Some(name) = env else { return Ok(()) };
        let Some(value) = vars(name) else { return Ok(()) };
        match value {
            Ok(text) => {
                insert(&mut root, path, leaf_node(leaf_shape, name, &text));
                Ok(())
            },
            Err(_) => {
                let mut err =
                    ConfigError::invalid(format!("environment variable {name} is not valid UTF-8"));
                for key in path.iter().rev() {
                    err = err.at_key(key);
                }
                Err(err)
            },
        }
    })?;
    Ok(ConfigNode::table(root))
}

fn leaf_node(shape: Shape, name: &str, text: &str) -> ConfigNode {
    let origin = Origin::new(format!("env:{name}"), 0, 0);
    if matches!(shape, Shape::List(_)) {
        let items = if text.is_empty() {
            Vec::new()
        } else {
            text.split(',')
                .map(|part| ConfigNode::scalar(part.trim()).with_origin(origin.clone()))
                .collect()
        };
        ConfigNode::array(items).with_origin(origin)
    } else {
        ConfigNode::scalar(text).with_origin(origin)
    }
}

fn insert(table: &mut Table, path: &[&str], node: ConfigNode) {
    let Some((key, rest)) = path.split_first() else { return };
    if rest.is_empty() {
        table.insert((*key).to_owned(), node);
        return;
    }
    let child = table
        .entry((*key).to_owned())
        .or_insert_with(|| ConfigNode::table(Table::new()));
    // A `flatten` collision can leave a non-table here; the later leaf wins.
    if child.as_table().is_none() {
        *child = ConfigNode::table(Table::new());
    }
    if let crate::Value::Table(inner) = &mut child.value {
        insert(inner, rest, node);
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use std::collections::BTreeMap;
    #[cfg(unix)]
    use std::os::unix::ffi::OsStringExt;

    use super::*;
    use crate::{FieldInfo, FromNode, StructInfo};

    static INNER: StructInfo = StructInfo::new(
        "Inner",
        &[
            FieldInfo::new("max_conn", <u16 as FromNode>::shape),
            FieldInfo::new("token", <String as FromNode>::shape).env("TOKEN"),
        ],
    );
    static ROOT: StructInfo = StructInfo::new(
        "Root",
        &[
            FieldInfo::new("port", <Option<u16> as FromNode>::shape),
            FieldInfo::new("server", || Shape::Struct(&INNER)),
            FieldInfo::new("tags", <Vec<String> as FromNode>::shape),
            FieldInfo::new("map", <BTreeMap<String, u8> as FromNode>::shape),
        ],
    )
    .env_prefix("APP_");

    fn lookup<'a>(
        pairs: &'a [(&'a str, &'a str)],
    ) -> impl Fn(&str) -> Option<Result<String, OsString>> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| Ok((*value).to_owned()))
        }
    }

    fn scalar_at<'a>(node: &'a ConfigNode, path: &[&str]) -> Option<&'a str> {
        let mut current = node;
        for key in path {
            current = current.get(key)?;
        }
        current.as_str()
    }

    #[test]
    fn builds_nested_tables_with_env_origin() {
        let pairs = [("APP_PORT", "80"), ("APP_SERVER_MAX_CONN", "5"), ("TOKEN", "t")];
        let node = env_node(Shape::Struct(&ROOT), &lookup(&pairs)).unwrap();
        assert_eq!(scalar_at(&node, &["port"]), Some("80"));
        assert_eq!(scalar_at(&node, &["server", "max_conn"]), Some("5"));
        assert_eq!(scalar_at(&node, &["server", "token"]), Some("t"));
        let origin = node.get("port").and_then(ConfigNode::origin).unwrap();
        assert_eq!(origin.to_string(), "env:APP_PORT");
    }

    #[test]
    fn absent_variables_leave_no_keys() {
        let node = env_node(Shape::Struct(&ROOT), &lookup(&[])).unwrap();
        assert_eq!(node, ConfigNode::table(Table::new()));
    }

    #[test]
    fn lists_split_on_commas_and_trim() {
        let pairs = [("APP_TAGS", "a, b ,c")];
        let node = env_node(Shape::Struct(&ROOT), &lookup(&pairs)).unwrap();
        let items: Vec<&str> = node
            .get("tags")
            .and_then(ConfigNode::as_array)
            .unwrap()
            .iter()
            .filter_map(ConfigNode::as_str)
            .collect();
        assert_eq!(items, ["a", "b", "c"]);
    }

    #[test]
    fn empty_list_value_is_an_empty_array() {
        let pairs = [("APP_TAGS", "")];
        let node = env_node(Shape::Struct(&ROOT), &lookup(&pairs)).unwrap();
        assert_eq!(node.get("tags").and_then(ConfigNode::as_array), Some(&[][..]));
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8_value_is_an_error_at_the_path() {
        let vars = |name: &str| {
            (name == "APP_SERVER_MAX_CONN").then(|| Err(OsString::from_vec(vec![0xff, 0xfe])))
        };
        let err = env_node(Shape::Struct(&ROOT), &vars).unwrap_err();
        assert_eq!(
            err.to_string(),
            "server.max_conn: environment variable APP_SERVER_MAX_CONN is not valid UTF-8"
        );
    }
}
