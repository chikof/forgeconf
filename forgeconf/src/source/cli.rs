//! Command-line source: flags derived from the config [`Shape`].

use std::ffi::OsString;
use std::path::Path;

use super::walk::{Leaf, leaves};
use crate::help::render_help;
use crate::suggest::suggest;
use crate::{ConfigError, ConfigNode, ErrorKind, Origin, Shape, Table, Text};

/// The long flag of `leaf`, without the leading `--`: the field's `flag`, else
/// the path joined by `.` with `_` turned into `-` in each segment.
pub(crate) fn long_name(leaf: &Leaf) -> String {
    match leaf.field.flag_name() {
        Some(flag) => flag.to_owned(),
        None => leaf
            .path
            .iter()
            .map(|segment| segment.replace('_', "-"))
            .collect::<Vec<_>>()
            .join("."),
    }
}

/// Parse `args` (program name first) into a node tree shaped like `shape`.
pub(crate) fn cli_node(shape: Shape, args: &[OsString]) -> Result<ConfigNode, ConfigError> {
    let leaves = leaves(shape)
        .into_iter()
        .filter(|leaf| !leaf.field.is_skip_cli())
        .collect::<Vec<_>>();
    let longs: Vec<String> = leaves.iter().map(long_name).collect();
    let program = args
        .first()
        .and_then(|arg| Path::new(arg).file_name())
        .map_or_else(|| "app".to_owned(), |name| name.to_string_lossy().into_owned());
    let help = || ConfigError::new(ErrorKind::Help(render_help(&program, shape)));

    // One slot per leaf: scalars keep the last value, lists collect all.
    let mut values: Vec<Vec<Text>> = vec![Vec::new(); leaves.len()];
    let mut tokens = args.iter().skip(1);
    while let Some(token) = tokens.next() {
        let token = token
            .to_str()
            .ok_or_else(|| cli_error("argument is not valid UTF-8"))?;
        let (index, inline, name) = if let Some(long) = token.strip_prefix("--") {
            if long.is_empty() {
                return Err(unexpected(token));
            }
            let (name, inline) = match long.split_once('=') {
                Some((name, value)) => (name, Some(value)),
                None => (long, None),
            };
            if name == "help" && inline.is_none() {
                return Err(help());
            }
            match longs.iter().position(|candidate| candidate == name) {
                Some(index) => (index, inline.map(Text::from), format!("--{name}")),
                None => {
                    let negated = match inline {
                        None => name.strip_prefix("no-"),
                        Some(_) => None,
                    }
                    .and_then(|rest| longs.iter().position(|candidate| candidate == rest))
                    .filter(|&index| matches!(leaves[index].shape, Shape::Bool));
                    match negated {
                        Some(index) => (index, Some(Text::from("false")), format!("--{name}")),
                        None => {
                            let candidates = longs.iter().map(String::as_str).chain(["help"]);
                            let mut message = format!("unknown flag --{name}");
                            if let Some(best) = suggest(name, candidates) {
                                message.push_str(&format!(", did you mean --{best}?"));
                            }
                            return Err(cli_error(message));
                        },
                    }
                },
            }
        } else if token.len() > 1 && token.starts_with('-') {
            let mut chars = token[1..].chars();
            let short = chars.next().unwrap_or('-');
            let rest = chars.as_str();
            if short == 'h' && rest.is_empty() {
                return Err(help());
            }
            let inline = match rest.strip_prefix('=') {
                Some(value) => Some(Text::from(value)),
                None if rest.is_empty() => None,
                None => {
                    return Err(cli_error(format!(
                        "combined short flags are not supported: {token}"
                    )));
                },
            };
            let index = leaves
                .iter()
                .position(|leaf| leaf.field.short_flag() == Some(short))
                .ok_or_else(|| cli_error(format!("unknown flag -{short}")))?;
            (index, inline, format!("-{short}"))
        } else {
            return Err(unexpected(token));
        };

        let value = match inline {
            Some(value) => value,
            None if matches!(leaves[index].shape, Shape::Bool) => Text::from("true"),
            None => tokens
                .next()
                .ok_or_else(|| cli_error(format!("flag {name} requires a value")))?
                .to_str()
                .ok_or_else(|| cli_error("argument is not valid UTF-8"))?
                .into(),
        };
        let slot = &mut values[index];
        if matches!(leaves[index].shape, Shape::List(_)) {
            slot.push(value);
        } else {
            *slot = vec![value];
        }
    }

    let mut root = Table::new();
    for ((leaf, long), values) in leaves.iter().zip(&longs).zip(values) {
        if values.is_empty() {
            continue;
        }
        let origin = Origin::new(format!("cli:--{long}"), 0, 0);
        let node = if matches!(leaf.shape, Shape::List(_)) {
            let items = values
                .into_iter()
                .map(|value| ConfigNode::scalar(value).with_origin(origin.clone()))
                .collect();
            ConfigNode::array(items)
        } else {
            ConfigNode::scalar(values.into_iter().next().unwrap_or_default())
        };
        insert(&mut root, &leaf.path, node.with_origin(origin));
    }
    Ok(ConfigNode::table(root))
}

fn insert(table: &mut Table, path: &[&str], node: ConfigNode) {
    match path {
        [] => {},
        [key] => {
            table.insert(Text::from(*key), node);
        },
        [key, rest @ ..] => {
            let child = table
                .entry(Text::from(*key))
                .or_insert_with(|| ConfigNode::table(Table::new()));
            if let crate::Value::Table(inner) = &mut child.value {
                insert(inner, rest, node);
            }
        },
    }
}

fn cli_error(message: impl Into<String>) -> ConfigError {
    ConfigError::new(ErrorKind::Cli(message.into()))
}

fn unexpected(token: &str) -> ConfigError {
    cli_error(format!("unexpected argument \"{token}\""))
}
