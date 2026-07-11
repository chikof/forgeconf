//! `--help` text generated from the config [`Shape`].

use crate::Shape;
use crate::source::cli::long_name;
use crate::source::walk::{Leaf, leaves};

/// Widest left column; longer entries put the description on the next line.
const MAX_LEFT: usize = 32;
/// Gap between the left column and the description.
const GAP: usize = 2;

pub(crate) fn render_help(program: &str, shape: Shape) -> String {
    let mut out = format!("Usage: {program} [OPTIONS]\n\n");
    if let Shape::Struct(info) = shape {
        if let Some(doc) = info.docs().filter(|doc| !doc.is_empty()) {
            out.push_str(doc);
            out.push_str("\n\n");
        }
    }
    out.push_str("Options:\n");

    let mut rows: Vec<(String, String)> = leaves(shape)
        .iter()
        .filter(|leaf| !leaf.field.is_skip_cli())
        .map(|leaf| (left_column(leaf), description(leaf)))
        .collect();
    rows.push(("  -h, --help".to_owned(), "Print help".to_owned()));

    let width = rows
        .iter()
        .map(|(left, _)| left.chars().count())
        .max()
        .unwrap_or(0)
        .min(MAX_LEFT);
    let lines: Vec<String> = rows
        .into_iter()
        .map(|(left, desc)| {
            let len = left.chars().count();
            if desc.is_empty() {
                left
            } else if len > MAX_LEFT {
                format!("{left}\n{:indent$}{desc}", "", indent = MAX_LEFT + GAP)
            } else {
                format!("{left}{:pad$}{desc}", "", pad = width + GAP - len)
            }
        })
        .collect();
    out.push_str(&lines.join("\n"));
    out
}

fn left_column(leaf: &Leaf) -> String {
    let mut left = String::from("  ");
    match leaf.field.short_flag() {
        Some(short) => {
            left.push('-');
            left.push(short);
            left.push_str(", ");
        },
        None => left.push_str("    "),
    }
    left.push_str("--");
    left.push_str(&long_name(leaf));
    if let Some(label) = value_label(leaf.shape) {
        left.push_str(" <");
        left.push_str(&label);
        left.push('>');
        if matches!(leaf.shape, Shape::List(_)) {
            left.push_str("...");
        }
    }
    left
}

fn value_label(shape: Shape) -> Option<String> {
    match shape {
        Shape::Scalar(label) => Some(label.to_owned()),
        Shape::Enum(info) => Some(
            info.variants()
                .iter()
                .map(|variant| variant.key())
                .collect::<Vec<_>>()
                .join("|"),
        ),
        Shape::List(item) => {
            let mut item = item();
            while let Shape::Optional(inner) = item {
                item = inner();
            }
            value_label(item)
        },
        _ => None,
    }
}

fn description(leaf: &Leaf) -> String {
    let mut parts = Vec::new();
    if let Some(line) = leaf.field.docs().and_then(|doc| doc.lines().next()) {
        parts.push(line.trim().trim_end_matches('.').to_owned());
    }
    if !leaf.field.is_sensitive() {
        if let Some(default) = leaf.field.default_value() {
            parts.push(format!("[default: {default}]"));
        }
    }
    if let Some(env) = &leaf.env {
        parts.push(format!("[env: {env}]"));
    }
    if leaf.field.deprecation().is_some() {
        parts.push("[deprecated]".to_owned());
    }
    parts.retain(|part| !part.is_empty());
    parts.join(" ")
}
