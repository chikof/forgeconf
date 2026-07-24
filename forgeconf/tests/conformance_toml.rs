//! toml-test conformance: every `invalid` file is rejected, every `valid` file
//! matches its JSON.
#![cfg(all(feature = "toml", feature = "json"))]
#![allow(clippy::unwrap_used, reason = "tests fail loudly")]

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use forgeconf::{ConfigNode, FileFormat, Value, parse_str};

fn suite_dir() -> PathBuf {
    let dir = std::env::var_os("FORGECONF_TOML_TEST_DIR")
        .expect("FORGECONF_TOML_TEST_DIR is unset; run inside `devenv shell`");
    PathBuf::from(dir)
}

fn toml_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "toml") {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    assert!(!out.is_empty(), "no .toml files under {}", root.display());
    out
}

fn parse_toml(bytes: &[u8]) -> Result<Option<ConfigNode>, String> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Ok(None);
    };
    match catch_unwind(AssertUnwindSafe(|| parse_str(text, FileFormat::Toml))) {
        Ok(Ok(node)) => Ok(Some(node)),
        Ok(Err(err)) => Err(err.to_string()),
        Err(_) => Err("PANIC".to_owned()),
    }
}

fn report(kind: &str, total: usize, failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{} of {total} {kind} cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
#[ignore = "conformance suite; run via `test-all`"]
fn toml_invalid_files_are_rejected() {
    let root = suite_dir().join("invalid");
    let files = toml_files(&root);
    let mut failures = Vec::new();
    for path in &files {
        let name = path.strip_prefix(&root).unwrap().display();
        match parse_toml(&fs::read(path).unwrap()) {
            Err(msg) if msg == "PANIC" => failures.push(format!("{name}: panicked")),
            Err(_) | Ok(None) => {},
            Ok(Some(_)) => failures.push(format!("{name}: accepted")),
        }
    }
    report("invalid", files.len(), &failures);
}

#[test]
#[ignore = "conformance suite; run via `test-all`"]
fn toml_valid_files_match_expected_json() {
    let root = suite_dir().join("valid");
    let files = toml_files(&root);
    let mut failures = Vec::new();
    for path in &files {
        let name = path.strip_prefix(&root).unwrap().display().to_string();
        let ours = match parse_toml(&fs::read(path).unwrap()) {
            Ok(Some(node)) => node,
            Ok(None) => {
                failures.push(format!("{name}: not valid UTF-8"));
                continue;
            },
            Err(msg) => {
                failures.push(format!("{name}: rejected: {msg}"));
                continue;
            },
        };
        let expected_text = fs::read_to_string(path.with_extension("json")).unwrap();
        let expected = parse_str(&expected_text, FileFormat::Json).unwrap();
        if let Err(why) = compare(&ours, &expected, "") {
            failures.push(format!("{name}: {why}"));
        }
    }
    report("valid", files.len(), &failures);
}

fn leaf(node: &ConfigNode) -> Option<(&str, &str)> {
    let Value::Table(table) = node.value() else {
        return None;
    };
    if table.len() != 2 {
        return None;
    }
    let kind = table.get("type")?.as_str()?;
    let value = table.get("value")?.as_str()?;
    Some((kind, value))
}

fn compare(ours: &ConfigNode, expected: &ConfigNode, path: &str) -> Result<(), String> {
    if let Some((kind, value)) = leaf(expected) {
        let Value::Scalar(text) = ours.value() else {
            return Err(format!("{path}: expected {kind} scalar, got non-scalar"));
        };
        return compare_scalar(kind, text, value).map_err(|why| format!("{path}: {why}"));
    }
    match (ours.value(), expected.value()) {
        (Value::Table(a), Value::Table(b)) => {
            if a.keys().ne(b.keys()) {
                return Err(format!(
                    "{path}: keys {:?} != expected {:?}",
                    a.keys().collect::<Vec<_>>(),
                    b.keys().collect::<Vec<_>>()
                ));
            }
            for (key, node) in a {
                compare(node, &b[key], &format!("{path}.{key}"))?;
            }
            Ok(())
        },
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Err(format!("{path}: array length {} != expected {}", a.len(), b.len()));
            }
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                compare(x, y, &format!("{path}[{i}]"))?;
            }
            Ok(())
        },
        _ => Err(format!("{path}: structure mismatch")),
    }
}

fn compare_scalar(kind: &str, ours: &str, expected: &str) -> Result<(), String> {
    let mismatch = || format!("{kind} {ours:?} != expected {expected:?}");
    let equal = match kind {
        "string" | "bool" => ours == expected,
        "integer" => match (ours.parse::<i64>(), expected.parse::<i64>()) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        },
        "float" => match (ours.parse::<f64>(), expected.parse::<f64>()) {
            (Ok(a), Ok(b)) => (a.is_nan() && b.is_nan()) || a == b,
            _ => false,
        },
        "datetime" | "datetime-local" | "date-local" | "time-local" => {
            DateTime::parse(ours).is_some_and(|a| Some(a) == DateTime::parse(expected))
        },
        other => return Err(format!("unknown expected type {other:?}")),
    };
    if equal { Ok(()) } else { Err(mismatch()) }
}

#[derive(Debug, Eq, PartialEq)]
struct DateTime {
    date: Option<String>,
    time: Option<String>,
    fraction: String,
    offset: Option<String>,
}

impl DateTime {
    fn parse(raw: &str) -> Option<Self> {
        let text = raw.replace('z', "Z");
        let (date, rest) = if text.as_bytes().get(4) == Some(&b'-') {
            let (date, rest) = text.split_at_checked(10)?;
            let rest = match rest.as_bytes().first() {
                None => "",
                Some(b' ' | b'T' | b't') => &rest[1..],
                Some(_) => return None,
            };
            (Some(date.to_owned()), rest)
        } else {
            (None, text.as_str())
        };
        if rest.is_empty() {
            return Some(Self {
                date,
                time: None,
                fraction: String::new(),
                offset: None,
            });
        }
        let (clock, offset) = match rest.find(['Z', '+', '-']) {
            Some(i) => (&rest[..i], Some(rest[i..].to_owned())),
            None => (rest, None),
        };
        let (time, fraction) = match clock.split_once('.') {
            Some((time, frac)) => (time, frac.trim_end_matches('0').to_owned()),
            None => (clock, String::new()),
        };
        Some(Self {
            date,
            time: Some(time.to_owned()),
            fraction,
            offset,
        })
    }
}
