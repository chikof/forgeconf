//! Runs the official yaml-test-suite against the in-house YAML subset parser.
#![cfg(all(feature = "yaml", feature = "json"))]
#![allow(clippy::unwrap_used, reason = "tests fail loudly")]

use std::collections::BTreeMap;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use forgeconf::{ConfigError, ConfigNode, ErrorKind, FileFormat, Value, parse_str};

/// Cases outside the declared subset that the parser cannot report as
/// `Unsupported`. Each entry is a case id (`ABCD` or `ABCD/01`) plus the
/// reason.
const SKIPPED: &[(&str, &str)] = &[];

/// Valid cases the suite ships without `in.json` although they hold data:
/// case id plus the expected data as JSON, transcribed from the case's
/// `test.event`.
const EXPECTED_WITHOUT_JSON: &[(&str, &str)] =
    &[("4ABK", r#"{"unquoted": "separate", "http://foo.com": null, "omitted value": null}"#)];

/// One suite case: a directory holding `in.yaml` and optionally `in.json` /
/// `error`.
struct Case {
    id: String,
    /// `None` when `in.yaml` is not valid UTF-8, which the loader rejects.
    yaml: Option<String>,
    invalid: bool,
    json: Option<String>,
}

fn suite_dir() -> PathBuf {
    let dir = std::env::var_os("FORGECONF_YAML_TEST_DIR")
        .expect("FORGECONF_YAML_TEST_DIR is unset; run inside `devenv shell`");
    PathBuf::from(dir)
}

fn sorted_entries(dir: &Path) -> Vec<(String, PathBuf)> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name().to_string_lossy().into_owned(), entry.path())
        })
        .collect();
    entries.sort();
    entries
}

fn load_case(id: String, dir: &Path) -> Case {
    Case {
        id,
        yaml: fs::read_to_string(dir.join("in.yaml")).ok(),
        invalid: dir.join("error").exists(),
        json: fs::read_to_string(dir.join("in.json")).ok(),
    }
}

fn cases() -> Vec<Case> {
    let root = suite_dir();
    let mut cases = Vec::new();
    for (name, path) in sorted_entries(&root) {
        // `name/` and `tags/` hold symlinks to the real case directories.
        let is_real_dir = fs::symlink_metadata(&path).unwrap().file_type().is_dir();
        if !is_real_dir || name.starts_with('.') || name == "name" || name == "tags" {
            continue;
        }
        if path.join("in.yaml").exists() {
            cases.push(load_case(name, &path));
            continue;
        }
        for (sub, sub_path) in sorted_entries(&path) {
            if sub_path.join("in.yaml").exists() {
                cases.push(load_case(format!("{name}/{sub}"), &sub_path));
            }
        }
    }
    assert!(!cases.is_empty(), "no cases found under {}", root.display());
    for (id, reason) in SKIPPED {
        assert!(
            cases.iter().any(|case| case.id == *id),
            "skip list entry {id} ({reason}) names no suite case"
        );
    }
    for (id, _) in EXPECTED_WITHOUT_JSON {
        assert!(
            cases
                .iter()
                .any(|case| case.id == *id && !case.invalid && case.json.is_none()),
            "{id} is not a valid suite case without in.json"
        );
    }
    cases
}

fn skipped(id: &str) -> bool {
    SKIPPED.iter().any(|(skip, _)| *skip == id)
}

/// What running the parser on one case produced.
enum Outcome {
    Parsed(Result<ConfigNode, ConfigError>),
    NotUtf8,
    Panicked,
}

fn run(case: &Case) -> Outcome {
    let Some(yaml) = &case.yaml else {
        return Outcome::NotUtf8;
    };
    match catch_unwind(AssertUnwindSafe(|| parse_str(yaml, FileFormat::Yaml))) {
        Ok(result) => Outcome::Parsed(result),
        Err(_) => Outcome::Panicked,
    }
}

fn is_unsupported(err: &ConfigError) -> bool {
    matches!(err.kind(), ErrorKind::Unsupported(_))
}

// ---- expected data: a tiny JSON reader that keeps strings, numbers, bools
// apart

#[derive(Debug)]
enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

struct JsonReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl JsonReader<'_> {
    fn ws(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }

    fn eat(&mut self, lit: &str) -> bool {
        if self.bytes[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> Json {
        self.ws();
        if self.eat("null") {
            return Json::Null;
        }
        if self.eat("true") {
            return Json::Bool(true);
        }
        if self.eat("false") {
            return Json::Bool(false);
        }
        match self.bytes[self.pos] {
            b'"' => Json::Str(self.string()),
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.ws();
                    if self.eat("]") {
                        break;
                    }
                    items.push(self.value());
                    self.ws();
                    self.eat(",");
                }
                Json::Arr(items)
            },
            b'{' => {
                self.pos += 1;
                let mut map = BTreeMap::new();
                loop {
                    self.ws();
                    if self.eat("}") {
                        break;
                    }
                    let key = self.string();
                    self.ws();
                    assert!(self.eat(":"), "expected ':' in expected JSON");
                    let value = self.value();
                    map.insert(key, value);
                    self.ws();
                    self.eat(",");
                }
                Json::Obj(map)
            },
            _ => {
                let start = self.pos;
                while self
                    .bytes
                    .get(self.pos)
                    .is_some_and(|b| b.is_ascii_digit() || b"+-.eE".contains(b))
                {
                    self.pos += 1;
                }
                assert!(self.pos > start, "bad expected JSON at byte {start}");
                Json::Num(String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned())
            },
        }
    }

    fn hex4(&mut self) -> u32 {
        let text = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).unwrap();
        self.pos += 4;
        u32::from_str_radix(text, 16).unwrap()
    }

    fn string(&mut self) -> String {
        assert_eq!(self.bytes[self.pos], b'"');
        self.pos += 1;
        let mut out = Vec::new();
        loop {
            let byte = self.bytes[self.pos];
            self.pos += 1;
            match byte {
                b'"' => break,
                b'\\' => {
                    let esc = self.bytes[self.pos];
                    self.pos += 1;
                    let ch = match esc {
                        b'n' => '\n',
                        b't' => '\t',
                        b'r' => '\r',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'/' => '/',
                        b'\\' => '\\',
                        b'"' => '"',
                        b'u' => {
                            let mut code = self.hex4();
                            if (0xD800..0xDC00).contains(&code) && self.eat("\\u") {
                                let low = self.hex4();
                                code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                            }
                            char::from_u32(code).unwrap_or('\u{fffd}')
                        },
                        other => panic!("bad escape \\{} in expected JSON", other as char),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                },
                other => out.push(other),
            }
        }
        String::from_utf8(out).unwrap()
    }
}

fn read_json_docs(text: &str) -> Vec<Json> {
    let mut reader = JsonReader { bytes: text.as_bytes(), pos: 0 };
    let mut docs = Vec::new();
    loop {
        reader.ws();
        if reader.pos >= reader.bytes.len() {
            return docs;
        }
        docs.push(reader.value());
    }
}

// ---- YAML 1.2 core schema scalar interpretation

fn core_bool(text: &str) -> Option<bool> {
    match text {
        "true" | "True" | "TRUE" => Some(true),
        "false" | "False" | "FALSE" => Some(false),
        _ => None,
    }
}

fn core_number(text: &str) -> Option<f64> {
    if let Some(oct) = text.strip_prefix("0o") {
        return u64::from_str_radix(oct, 8).ok().map(|n| n as f64);
    }
    if let Some(hex) = text.strip_prefix("0x") {
        return u64::from_str_radix(hex, 16).ok().map(|n| n as f64);
    }
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    let negative = text.starts_with('-');
    if matches!(unsigned, ".inf" | ".Inf" | ".INF") {
        return Some(if negative { f64::NEG_INFINITY } else { f64::INFINITY });
    }
    let plain = unsigned
        .bytes()
        .all(|b| b.is_ascii_digit() || b"+-.eE".contains(&b));
    let has_digit = unsigned.bytes().any(|b| b.is_ascii_digit());
    if plain && has_digit && !unsigned.starts_with(['e', 'E', '+', '-']) {
        return text.parse().ok();
    }
    None
}

// ---- comparison

fn describe(node: &ConfigNode) -> String {
    match node.value() {
        Value::Null => "null".into(),
        Value::Scalar(text) => format!("{text:?}"),
        Value::Array(_) => "array".into(),
        Value::Table(_) => "table".into(),
        _ => "other".into(),
    }
}

fn compare(expected: &Json, actual: &ConfigNode, path: &str) -> Result<(), String> {
    let mismatch = |what: &str| Err(format!("{path}: expected {what}, got {}", describe(actual)));
    match expected {
        Json::Null => {
            if actual.is_null() {
                Ok(())
            } else {
                mismatch("null")
            }
        },
        Json::Bool(want) => match actual.as_str().and_then(core_bool) {
            Some(got) if got == *want => Ok(()),
            _ => mismatch(&want.to_string()),
        },
        Json::Num(text) => {
            let want: f64 = text.parse().unwrap();
            match actual.as_str().and_then(core_number) {
                Some(got) if got == want => Ok(()),
                _ => mismatch(text),
            }
        },
        Json::Str(want) => {
            if actual.as_str() == Some(want.as_str()) {
                Ok(())
            } else {
                mismatch(&format!("{want:?}"))
            }
        },
        Json::Arr(items) => {
            let Some(got) = actual.as_array() else {
                return mismatch("array");
            };
            if got.len() != items.len() {
                return Err(format!("{path}: expected {} items, got {}", items.len(), got.len()));
            }
            for (index, (want, node)) in items.iter().zip(got).enumerate() {
                compare(want, node, &format!("{path}[{index}]"))?;
            }
            Ok(())
        },
        Json::Obj(map) => {
            let Some(got) = actual.as_table() else {
                return mismatch("table");
            };
            let want_keys: Vec<_> = map.keys().collect();
            let got_keys: Vec<_> = got.keys().collect();
            if want_keys != got_keys {
                return Err(format!("{path}: expected keys {want_keys:?}, got {got_keys:?}"));
            }
            for (key, want) in map {
                compare(want, &got[key], &format!("{path}.{key}"))?;
            }
            Ok(())
        },
    }
}

/// An empty document parses to an empty table; the suite expects `null`.
fn compare_root(expected: &Json, actual: &ConfigNode) -> Result<(), String> {
    if matches!(expected, Json::Null) && actual.as_table().is_some_and(|t| t.is_empty()) {
        return Ok(());
    }
    compare(expected, actual, "$")
}

// ---- tests

#[test]
#[ignore = "conformance suite; run via `test-all`"]
fn yaml_suite_rejects_invalid_documents() {
    let mut failures = Vec::new();
    let mut rejected = 0usize;
    let mut skip_count = 0usize;
    let mut total = 0usize;
    for case in cases().iter().filter(|case| case.invalid) {
        total += 1;
        if skipped(&case.id) {
            skip_count += 1;
            continue;
        }
        match run(case) {
            Outcome::Panicked => failures.push(format!("{}: parser panicked", case.id)),
            Outcome::NotUtf8 | Outcome::Parsed(Err(_)) => rejected += 1,
            Outcome::Parsed(Ok(node)) => {
                failures.push(format!("{}: invalid YAML accepted as {}", case.id, describe(&node)));
            },
        }
    }
    println!("yaml invalid: {total} cases, {rejected} rejected, {skip_count} skipped");
    assert!(
        failures.is_empty(),
        "{} of {total} invalid YAML cases were not rejected:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
#[ignore = "conformance suite; run via `test-all`"]
fn yaml_suite_valid_documents_match_expected_data() {
    let mut failures = Vec::new();
    let (mut equal, mut unsupported, mut skip_count, mut total) = (0usize, 0usize, 0usize, 0usize);
    for case in cases().iter().filter(|case| !case.invalid) {
        total += 1;
        if skipped(&case.id) {
            skip_count += 1;
            continue;
        }
        let json = case.json.as_deref().or_else(|| {
            EXPECTED_WITHOUT_JSON
                .iter()
                .find_map(|(id, json)| (*id == case.id).then_some(*json))
        });
        let docs = json.map(read_json_docs);
        match run(case) {
            Outcome::Panicked => failures.push(format!("{}: parser panicked", case.id)),
            Outcome::NotUtf8 => failures.push(format!("{}: valid case is not UTF-8", case.id)),
            Outcome::Parsed(Err(err)) if is_unsupported(&err) => unsupported += 1,
            Outcome::Parsed(Err(err)) => failures.push(format!("{}: rejected: {err}", case.id)),
            Outcome::Parsed(Ok(node)) => {
                let empty_table = node.as_table().is_some_and(|t| t.is_empty());
                let verdict = match docs.as_deref() {
                    // Product rule: an empty document (nothing but comments,
                    // whitespace or `...`) is an empty table, not zero
                    // documents.
                    Some([]) if empty_table => Ok(()),
                    Some([]) => Err(format!("0-document stream accepted as {}", describe(&node))),
                    Some([doc]) => compare_root(doc, &node),
                    Some(many) => Err(format!(
                        "accepted a {}-document stream as {}",
                        many.len(),
                        describe(&node)
                    )),
                    None if empty_table => Ok(()),
                    None => Err(format!("no data expected, got {}", describe(&node))),
                };
                match verdict {
                    Ok(()) => equal += 1,
                    Err(why) => failures.push(format!("{}: {why}", case.id)),
                }
            },
        }
    }
    println!(
        "yaml valid: {total} cases, {equal} accepted-equal, {unsupported} unsupported, \
         {skip_count} skipped"
    );
    assert!(
        failures.is_empty(),
        "{} of {total} valid YAML cases failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
