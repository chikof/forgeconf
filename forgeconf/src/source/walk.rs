//! Leaf walk over a config [`Shape`], shared by the environment and CLI
//! sources.

use crate::{FieldInfo, Shape};

/// A field that can be set from a single environment variable or flag.
pub(crate) struct Leaf {
    /// Keys from the root to this field; `flatten` adds no segment.
    pub(crate) path: Vec<&'static str>,
    #[cfg_attr(not(feature = "cli"), allow(dead_code, reason = "read by the CLI source and help"))]
    pub(crate) field: &'static FieldInfo,
    /// The field's shape with `Optional` removed: `Scalar`, `Bool`, a unit-only
    /// `Enum`, or a `List` of those.
    pub(crate) shape: Shape,
    /// Variable name: the field's `env`, else the root prefix plus the path.
    pub(crate) env: Option<String>,
}

/// Every settable leaf of `shape`, in declaration order. Lists of tables, maps
/// and enums with data are skipped: a flat string cannot describe them.
pub(crate) fn leaves(shape: Shape) -> Vec<Leaf> {
    let mut out = Vec::new();
    if let Shape::Struct(info) = shape {
        let mut path = Vec::new();
        walk(info.fields(), info.prefix(), &mut path, &mut out);
    }
    out
}

fn walk(
    fields: &'static [FieldInfo],
    prefix: Option<&'static str>,
    path: &mut Vec<&'static str>,
    out: &mut Vec<Leaf>,
) {
    for field in fields {
        let shape = unwrap_optional(field.shape());
        if let Shape::Struct(info) = shape {
            if !field.is_flatten() {
                path.push(field.key());
            }
            walk(info.fields(), prefix, path, out);
            if !field.is_flatten() {
                path.pop();
            }
            continue;
        }
        if !is_leaf(shape) {
            continue;
        }
        path.push(field.key());
        let env = match field.env_var() {
            Some(name) => Some(name.to_owned()),
            None => prefix.map(|prefix| env_name(prefix, path)),
        };
        out.push(Leaf { path: path.clone(), field, shape, env });
        path.pop();
    }
}

fn unwrap_optional(shape: Shape) -> Shape {
    match shape {
        Shape::Optional(inner) => unwrap_optional(inner()),
        other => other,
    }
}

fn is_leaf(shape: Shape) -> bool {
    match shape {
        Shape::List(item) => is_scalar(unwrap_optional(item())),
        other => is_scalar(other),
    }
}

fn is_scalar(shape: Shape) -> bool {
    match shape {
        Shape::Scalar(_) | Shape::Bool => true,
        Shape::Enum(info) => info.variants().iter().all(|v| v.payload().is_none()),
        _ => false,
    }
}

fn env_name(prefix: &str, path: &[&str]) -> String {
    let mut name = String::from(prefix);
    for (i, segment) in path.iter().enumerate() {
        if i > 0 {
            name.push('_');
        }
        name.push_str(segment);
    }
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
        .collect()
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;
    use crate::{EnumInfo, FromNode, StructInfo, VariantInfo};

    static LOG: EnumInfo =
        EnumInfo::new("Log", &[VariantInfo::new("debug"), VariantInfo::new("info")]);
    static DATA: EnumInfo =
        EnumInfo::new("Data", &[VariantInfo::new("x").data(<u8 as FromNode>::shape)]);
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
            FieldInfo::new("flat", || Shape::Struct(&INNER)).flatten(),
            FieldInfo::new("tags", <Vec<String> as FromNode>::shape),
            FieldInfo::new("log", || Shape::Enum(&LOG)),
            FieldInfo::new("data", || Shape::Enum(&DATA)),
            FieldInfo::new("map", <std::collections::BTreeMap<String, u8> as FromNode>::shape),
        ],
    )
    .env_prefix("app-");

    #[test]
    fn walks_leaves_with_env_names() {
        let found: Vec<(String, Option<String>)> = leaves(Shape::Struct(&ROOT))
            .into_iter()
            .map(|leaf| (leaf.path.join("."), leaf.env))
            .collect();
        let expected = [
            ("port", Some("APP_PORT")),
            ("server.max_conn", Some("APP_SERVER_MAX_CONN")),
            ("server.token", Some("TOKEN")),
            ("max_conn", Some("APP_MAX_CONN")),
            ("token", Some("TOKEN")),
            ("tags", Some("APP_TAGS")),
            ("log", Some("APP_LOG")),
        ]
        .map(|(path, env)| (path.to_owned(), env.map(str::to_owned)));
        assert_eq!(found, expected);
    }

    #[test]
    fn no_prefix_means_only_explicit_names() {
        static BARE: StructInfo = StructInfo::new(
            "Bare",
            &[
                FieldInfo::new("a", <u8 as FromNode>::shape),
                FieldInfo::new("b", <u8 as FromNode>::shape).env("B_VAR"),
            ],
        );
        let envs: Vec<Option<String>> = leaves(Shape::Struct(&BARE))
            .into_iter()
            .map(|leaf| leaf.env)
            .collect();
        assert_eq!(envs, [None, Some("B_VAR".to_owned())]);
    }
}
