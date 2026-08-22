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
    let walked: Result<(), std::convert::Infallible> =
        visit(shape, true, &mut |path, field, shape, env| {
            out.push(Leaf {
                path: path.to_vec(),
                field,
                shape,
                env: env.map(str::to_owned),
            });
            Ok(())
        });
    walked.unwrap_or(());
    out
}

/// Call `f(path, field, shape, env)` for every leaf of `shape`, like
/// [`leaves`] but without allocating per leaf. `env` is the variable name:
/// the field's `env`, else the root prefix plus the path; `None` when `names`
/// is false or the field has neither. It is only valid during the call.
pub(crate) fn visit<E>(
    shape: Shape,
    names: bool,
    f: &mut impl FnMut(&[&'static str], &'static FieldInfo, Shape, Option<&str>) -> Result<(), E>,
) -> Result<(), E> {
    let Shape::Struct(info) = shape else { return Ok(()) };
    let mut name = String::new();
    let prefixed = names && info.prefix().is_some();
    if let (true, Some(prefix)) = (prefixed, info.prefix()) {
        name.reserve(64);
        push_mapped(&mut name, prefix);
    }
    let mut walker = Walker {
        path: Vec::with_capacity(8),
        name,
        prefixed,
        f,
    };
    walker.fields(info.fields())
}

struct Walker<'f, F> {
    path: Vec<&'static str>,
    /// Prefix plus the mapped path so far; kept only when `prefixed`.
    name: String,
    prefixed: bool,
    f: &'f mut F,
}

impl<F, E> Walker<'_, F>
where
    F: FnMut(&[&'static str], &'static FieldInfo, Shape, Option<&str>) -> Result<(), E>,
{
    fn fields(&mut self, fields: &'static [FieldInfo]) -> Result<(), E> {
        for field in fields {
            let shape = unwrap_optional(field.shape());
            if let Shape::Struct(info) = shape {
                if field.is_flatten() {
                    self.fields(info.fields())?;
                } else {
                    let mark = self.push(field.key());
                    self.fields(info.fields())?;
                    self.pop(mark);
                }
                continue;
            }
            if !is_leaf(shape) {
                continue;
            }
            let mark = self.push(field.key());
            let env = match field.env_var() {
                Some(name) => Some(name),
                None if self.prefixed => Some(self.name.as_str()),
                None => None,
            };
            (self.f)(&self.path, field, shape, env)?;
            self.pop(mark);
        }
        Ok(())
    }

    /// Descend into `key`; returns what [`pop`](Self::pop) needs to undo it.
    fn push(&mut self, key: &'static str) -> usize {
        self.path.push(key);
        let mark = self.name.len();
        if self.prefixed {
            if self.path.len() > 1 {
                self.name.push('_');
            }
            push_mapped(&mut self.name, key);
        }
        mark
    }

    fn pop(&mut self, mark: usize) {
        self.path.pop();
        self.name.truncate(mark);
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

/// Append `text` upper-cased, every character that is not ASCII alphanumeric
/// replaced by `_`.
fn push_mapped(out: &mut String, text: &str) {
    for c in text.chars() {
        out.push(if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' });
    }
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
