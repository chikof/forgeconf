//! In-house JSON parser (RFC 8259) producing [`ConfigNode`] trees.

use std::collections::btree_map::Entry as MapEntry;

use super::cursor::Cursor;
use crate::{ConfigError, ConfigNode, Table};

const FORMAT: &str = "json";

/// Objects up to this size are filled by inserting in document order, which
/// meets the first repeated key on the way; larger ones are sorted and built in
/// bulk.
const SMALL_TABLE: usize = 8;

/// The text being parsed, kept to rebuild the origin of an earlier key when a
/// duplicate is reported.
struct Doc<'a> {
    input: &'a str,
    source: &'a str,
    /// Members of every object being read, innermost last; an object owns the
    /// entries above its start, so no object allocates a list of its own.
    stack: Vec<Entry>,
}

/// A parsed object member, with the byte offset of its key.
struct Entry {
    key: String,
    node: ConfigNode,
    pos: usize,
}

/// Parse JSON content into a [`ConfigNode`] tree. Any JSON value is a valid
/// document; a leading UTF-8 BOM is skipped.
pub(crate) fn parse(input: &str, source: &str) -> Result<ConfigNode, ConfigError> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    // About one member per 32 bytes of text, within a bound: this avoids the
    // regrowth of the stack while it fills, without trusting huge inputs.
    let stack = Vec::with_capacity((input.len() / 32).min(4096));
    let mut doc = Doc { input, source, stack };
    let mut cursor = Cursor::new(input, source);
    skip_ws(&mut cursor);
    let node = parse_value(&mut cursor, &mut doc)?;
    skip_ws(&mut cursor);
    if !cursor.is_eof() {
        return Err(cursor.error(FORMAT, "unexpected content after the top-level value"));
    }
    Ok(node)
}

fn skip_ws(c: &mut Cursor<'_>) {
    while let Some(byte) = c.peek() {
        match byte {
            b' ' | b'\t' | b'\r' => c.advance_ascii(1),
            b'\n' => {
                c.bump();
            },
            _ => break,
        }
    }
}

fn parse_value(c: &mut Cursor<'_>, doc: &mut Doc<'_>) -> Result<ConfigNode, ConfigError> {
    let origin = c.origin();
    let node = match c.peek() {
        Some(b'{') => {
            c.enter(FORMAT)?;
            let node = parse_object(c, doc);
            c.leave();
            node?
        },
        Some(b'[') => {
            c.enter(FORMAT)?;
            let node = parse_array(c, doc);
            c.leave();
            node?
        },
        Some(b'"') => ConfigNode::scalar(parse_string(c)?),
        Some(b'-' | b'0'..=b'9') => ConfigNode::scalar(parse_number(c)?),
        Some(b't') if c.eat("true") => ConfigNode::scalar("true"),
        Some(b'f') if c.eat("false") => ConfigNode::scalar("false"),
        Some(b'n') if c.eat("null") => ConfigNode::null(),
        Some(_) => return Err(unexpected(c, "a value")),
        None => return Err(c.error(FORMAT, "unexpected end of input, expected a value")),
    };
    Ok(node.with_origin(origin))
}

fn unexpected(c: &Cursor<'_>, expected: &str) -> ConfigError {
    match c.rest().chars().next() {
        Some(ch) => c.error(FORMAT, format!("unexpected {ch:?}, expected {expected}")),
        None => c.error(FORMAT, format!("unexpected end of input, expected {expected}")),
    }
}

fn expect(c: &mut Cursor<'_>, byte: u8, expected: &str) -> Result<(), ConfigError> {
    if c.peek() == Some(byte) {
        c.advance_ascii(1);
        Ok(())
    } else {
        Err(unexpected(c, expected))
    }
}

/// Members are collected in document order and turned into the table in one
/// pass at the end. A duplicate key must still win over any error that comes
/// after its second occurrence, so every failure path checks the members read
/// so far (`fail`).
fn parse_object(c: &mut Cursor<'_>, doc: &mut Doc<'_>) -> Result<ConfigNode, ConfigError> {
    c.advance_ascii(1);
    let start = doc.stack.len();
    skip_ws(c);
    if c.peek() == Some(b'}') {
        c.advance_ascii(1);
        return Ok(ConfigNode::table(Table::new()));
    }
    loop {
        let members = start..doc.stack.len();
        skip_ws(c);
        if c.peek() != Some(b'"') {
            return Err(fail(doc, members, None, unexpected(c, "a string key")));
        }
        let pos = c.pos();
        let key = match parse_string(c) {
            Ok(key) => key,
            Err(err) => return Err(fail(doc, members, None, err)),
        };
        skip_ws(c);
        if let Err(err) = expect(c, b':', "`:`") {
            return Err(fail(doc, members, Some((key, pos)), err));
        }
        skip_ws(c);
        let node = match parse_value(c, doc) {
            Ok(node) => node,
            Err(err) => return Err(fail(doc, members, Some((key, pos)), err)),
        };
        doc.stack.push(Entry { key, node, pos });
        skip_ws(c);
        match c.peek() {
            Some(b',') => {
                c.advance_ascii(1);
            },
            Some(b'}') => {
                c.advance_ascii(1);
                return build_table(doc, start).map(ConfigNode::table);
            },
            _ => {
                let members = start..doc.stack.len();
                return Err(fail(doc, members, None, unexpected(c, "`,` or `}`")));
            },
        }
    }
}

fn by_key(a: &Entry, b: &Entry) -> std::cmp::Ordering {
    a.key.cmp(&b.key)
}

/// Among entries sorted stably by key, the repeated key whose second
/// occurrence comes first in the document.
fn earliest_duplicate(sorted: &[Entry]) -> Option<&Entry> {
    sorted
        .windows(2)
        .filter(|pair| pair[0].key == pair[1].key)
        .map(|pair| &pair[1])
        .min_by_key(|entry| entry.pos)
}

fn duplicate_key(input: &str, source: &str, key: &str, pos: usize) -> ConfigError {
    let mut at = Cursor::new(input, source);
    at.advance(pos);
    at.error(FORMAT, format!("duplicate key \"{key}\""))
}

/// Table from the members of one object (the stack entries from `start`),
/// rejecting repeated keys. Large objects are built in bulk, which drops a
/// repeated key silently; the lost entry is the only sign, and only then are
/// the keys re-read from the document to find the one to report.
fn build_table(doc: &mut Doc<'_>, start: usize) -> Result<Table, ConfigError> {
    if doc.stack.len().saturating_sub(start) <= SMALL_TABLE {
        let mut table = Table::new();
        for Entry { key, node, pos } in doc.stack.drain(start..) {
            match table.entry(key) {
                MapEntry::Occupied(slot) => {
                    return Err(duplicate_key(doc.input, doc.source, slot.key(), pos));
                },
                MapEntry::Vacant(slot) => {
                    slot.insert(node);
                },
            }
        }
        return Ok(table);
    }
    let positions: Vec<usize> = doc
        .stack
        .get(start..)
        .unwrap_or_default()
        .iter()
        .map(|entry| entry.pos)
        .collect();
    let table: Table = doc
        .stack
        .drain(start..)
        .map(|entry| (entry.key, entry.node))
        .collect();
    if table.len() == positions.len() {
        return Ok(table);
    }
    let mut entries: Vec<Entry> = positions
        .iter()
        .filter_map(|&pos| {
            let mut at = Cursor::new(doc.input, doc.source);
            at.advance(pos);
            let key = parse_string(&mut at).ok()?;
            Some(Entry { key, node: ConfigNode::null(), pos })
        })
        .collect();
    entries.sort_by(by_key);
    Err(match earliest_duplicate(&entries) {
        Some(entry) => duplicate_key(doc.input, doc.source, &entry.key, entry.pos),
        None => Cursor::new(doc.input, doc.source).error(FORMAT, "duplicate key"),
    })
}

/// `err` for a failed object whose members are `members` on the stack, unless
/// a duplicate key precedes it: the document is then rejected for the
/// duplicate, as soon as its key is read.
fn fail(
    doc: &mut Doc<'_>,
    members: std::ops::Range<usize>,
    pending: Option<(String, usize)>,
    err: ConfigError,
) -> ConfigError {
    doc.stack.truncate(members.end);
    if let Some((key, pos)) = pending {
        doc.stack.push(Entry { key, node: ConfigNode::null(), pos });
    }
    let entries = doc.stack.get_mut(members.start..).unwrap_or_default();
    entries.sort_by(by_key);
    match earliest_duplicate(entries) {
        Some(entry) => duplicate_key(doc.input, doc.source, &entry.key, entry.pos),
        None => err,
    }
}

fn parse_array(c: &mut Cursor<'_>, doc: &mut Doc<'_>) -> Result<ConfigNode, ConfigError> {
    c.advance_ascii(1);
    let mut items = Vec::new();
    skip_ws(c);
    if c.peek() == Some(b']') {
        c.advance_ascii(1);
        return Ok(ConfigNode::array(items));
    }
    loop {
        skip_ws(c);
        items.push(parse_value(c, doc)?);
        skip_ws(c);
        match c.peek() {
            Some(b',') => {
                c.advance_ascii(1);
            },
            Some(b']') => {
                c.advance_ascii(1);
                return Ok(ConfigNode::array(items));
            },
            _ => return Err(unexpected(c, "`,` or `]`")),
        }
    }
}

/// Length of the leading run of `bytes` free of `"`, `\` and control bytes,
/// and whether the run is ASCII, found eight bytes at a time. In `has_zero`,
/// the lowest flagged byte is always a real match, so the first set bit is
/// exact.
fn string_run(bytes: &[u8]) -> (usize, bool) {
    const LOW: u64 = 0x0101_0101_0101_0101;
    const HIGH: u64 = 0x8080_8080_8080_8080;
    let has_zero = |x: u64| x.wrapping_sub(LOW) & !x & HIGH;
    let mut at = 0;
    let mut seen = 0;
    while let Some(chunk) = bytes.get(at..).and_then(<[u8]>::first_chunk::<8>) {
        let word = u64::from_le_bytes(*chunk);
        let hits = has_zero(word ^ (LOW * u64::from(b'"')))
            | has_zero(word ^ (LOW * u64::from(b'\\')))
            | (word.wrapping_sub(LOW * 0x20) & !word & HIGH);
        if hits != 0 {
            let len = (hits.trailing_zeros() / 8) as usize;
            let run = word & ((1 << (len * 8)) - 1);
            return (at + len, (seen | run) & HIGH == 0);
        }
        seen |= word;
        at += 8;
    }
    let tail = bytes.get(at..).unwrap_or_default();
    let len = tail
        .iter()
        .position(|&b| b == b'"' || b == b'\\' || b < 0x20)
        .unwrap_or(tail.len());
    let ascii = seen & HIGH == 0 && tail.get(..len).is_some_and(<[u8]>::is_ascii);
    (at + len, ascii)
}

fn parse_string(c: &mut Cursor<'_>) -> Result<String, ConfigError> {
    c.advance_ascii(1);
    let rest = c.rest();
    let bytes = rest.as_bytes();
    let (len, ascii) = string_run(bytes);
    if bytes.get(len) == Some(&b'"') {
        let text = rest.get(..len).unwrap_or("").to_owned();
        if ascii {
            c.advance_ascii(len + 1);
        } else {
            c.advance_flat(len + 1);
        }
        return Ok(text);
    }
    parse_string_slow(c)
}

/// The string after its opening quote when it holds an escape or is malformed.
fn parse_string_slow(c: &mut Cursor<'_>) -> Result<String, ConfigError> {
    let mut out = String::new();
    loop {
        let rest = c.rest();
        let bytes = rest.as_bytes();
        let len = string_run(bytes).0;
        out.push_str(rest.get(..len).unwrap_or(""));
        match bytes.get(len) {
            Some(b'"') => {
                c.advance_flat(len + 1);
                return Ok(out);
            },
            Some(b'\\') => {
                c.advance_flat(len + 1);
                out.push(parse_escape(c)?);
            },
            Some(_) => {
                c.advance_flat(len);
                return Err(c.error(FORMAT, "control character in string"));
            },
            None => {
                c.advance_flat(len);
                return Err(c.error(FORMAT, "unterminated string"));
            },
        }
    }
}
/// Decode the escape after a consumed backslash.
fn parse_escape(c: &mut Cursor<'_>) -> Result<char, ConfigError> {
    let Some(byte) = c.peek() else {
        return Err(c.error(FORMAT, "unterminated string"));
    };
    let simple = match byte {
        b'"' => '"',
        b'\\' => '\\',
        b'/' => '/',
        b'b' => '\u{8}',
        b'f' => '\u{c}',
        b'n' => '\n',
        b'r' => '\r',
        b't' => '\t',
        b'u' => {
            c.bump();
            return parse_unicode_escape(c);
        },
        _ => return Err(unexpected(c, "a valid escape character")),
    };
    c.bump();
    Ok(simple)
}

/// Decode `XXXX` after `\u`, combining a surrogate pair into one scalar.
fn parse_unicode_escape(c: &mut Cursor<'_>) -> Result<char, ConfigError> {
    let high = parse_hex4(c)?;
    let code = match high {
        0xD800..=0xDBFF => {
            if !c.eat("\\u") {
                return Err(c.error(FORMAT, "unpaired high surrogate in string"));
            }
            let low = parse_hex4(c)?;
            if !(0xDC00..=0xDFFF).contains(&low) {
                return Err(c.error(FORMAT, "high surrogate not followed by a low surrogate"));
            }
            0x10000 + ((high - 0xD800) << 10) + (low - 0xDC00)
        },
        0xDC00..=0xDFFF => return Err(c.error(FORMAT, "unpaired low surrogate in string")),
        _ => high,
    };
    char::from_u32(code).ok_or_else(|| c.error(FORMAT, "invalid unicode escape"))
}

fn parse_hex4(c: &mut Cursor<'_>) -> Result<u32, ConfigError> {
    let mut value = 0;
    for _ in 0..4 {
        let digit = c
            .rest()
            .chars()
            .next()
            .and_then(|ch| ch.to_digit(16))
            .ok_or_else(|| unexpected(c, "a hex digit"))?;
        c.bump();
        value = value * 16 + digit;
    }
    Ok(value)
}

/// Validate a number against the JSON grammar and return its exact text.
fn parse_number(c: &mut Cursor<'_>) -> Result<String, ConfigError> {
    let rest = c.rest();
    let bytes = rest.as_bytes();
    let digits = |from: usize| {
        bytes
            .iter()
            .skip(from)
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let mut end = usize::from(bytes.first() == Some(&b'-'));
    match bytes.get(end) {
        Some(b'0') => end += 1,
        Some(b'1'..=b'9') => end += 1 + digits(end + 1),
        _ => return Err(number_error(c, end)),
    }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        match digits(end) {
            0 => return Err(number_error(c, end)),
            n => end += n,
        }
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        end += 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        match digits(end) {
            0 => return Err(number_error(c, end)),
            n => end += n,
        }
    }
    c.advance_ascii(end);
    Ok(rest.get(..end).unwrap_or("").to_owned())
}

/// The error for a missing digit `offset` bytes into the number.
fn number_error(c: &mut Cursor<'_>, offset: usize) -> ConfigError {
    c.advance_ascii(offset);
    unexpected(c, "a digit")
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;
    use crate::{ErrorKind, Origin, Value};

    /// Rebuild `node` without origins so trees compare by value.
    fn strip(node: &ConfigNode) -> ConfigNode {
        match node.value() {
            Value::Null => ConfigNode::null(),
            Value::Scalar(text) => ConfigNode::scalar(text.as_str()),
            Value::Array(items) => ConfigNode::array(items.iter().map(strip).collect()),
            Value::Table(table) => {
                ConfigNode::table(table.iter().map(|(k, v)| (k.clone(), strip(v))).collect())
            },
        }
    }

    fn parsed(input: &str) -> ConfigNode {
        strip(&parse(input, "<string>").unwrap())
    }

    fn nested(depth: usize) -> String {
        format!("{}{}", "[".repeat(depth), "]".repeat(depth))
    }

    #[test]
    fn parses_nested_object() {
        let input = r#"{"server": {"host": "localhost", "port": 8080, "tls": false},
            "tags": ["a", "b"], "none": null, "ratio": -0.5, "empty": {}, "list": []}"#;
        let expected = ConfigNode::table(Table::from([
            (
                "server".to_owned(),
                ConfigNode::table(Table::from([
                    ("host".to_owned(), ConfigNode::scalar("localhost")),
                    ("port".to_owned(), ConfigNode::scalar("8080")),
                    ("tls".to_owned(), ConfigNode::scalar("false")),
                ])),
            ),
            (
                "tags".to_owned(),
                ConfigNode::array(vec![ConfigNode::scalar("a"), ConfigNode::scalar("b")]),
            ),
            ("none".to_owned(), ConfigNode::null()),
            ("ratio".to_owned(), ConfigNode::scalar("-0.5")),
            ("empty".to_owned(), ConfigNode::table(Table::new())),
            ("list".to_owned(), ConfigNode::array(Vec::new())),
        ]));
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn nodes_carry_origin_of_first_byte() {
        let node = parse("{\n  \"a\": [1, true]\n}", "app.json").unwrap();
        assert_eq!(node.origin(), Some(&Origin::new("app.json", 1, 1)));
        let list = node.get("a").unwrap();
        assert_eq!(list.origin(), Some(&Origin::new("app.json", 2, 8)));
        let items = list.as_array().unwrap();
        assert_eq!(items[0].origin(), Some(&Origin::new("app.json", 2, 9)));
        assert_eq!(items[1].origin(), Some(&Origin::new("app.json", 2, 12)));
    }

    #[test]
    fn top_level_scalar_is_allowed() {
        assert_eq!(parsed("  42 \n"), ConfigNode::scalar("42"));
        assert_eq!(parsed("null"), ConfigNode::null());
    }

    #[test]
    fn decodes_escapes() {
        assert_eq!(
            parsed(r#""\"\\\/\b\f\n\r\t\u0041""#),
            ConfigNode::scalar("\"\\/\u{8}\u{c}\n\r\tA")
        );
    }

    #[test]
    fn decodes_surrogate_pair() {
        assert_eq!(parsed(r#""\ud83d\ude00""#), ConfigNode::scalar("😀"));
    }

    #[test]
    fn keeps_raw_non_ascii_text() {
        assert_eq!(parsed("\"héllo 😀\""), ConfigNode::scalar("héllo 😀"));
    }

    #[test]
    fn rejects_lone_surrogates() {
        for input in [r#""\ud83d""#, r#""\ude00""#, r#""\ud83dx""#, r#""\ud83d\u0041""#] {
            let err = parse(input, "<string>").unwrap_err();
            assert!(matches!(err.kind(), ErrorKind::Syntax { format: "json", .. }), "{input}");
        }
    }

    #[test]
    fn rejects_raw_control_character() {
        let err = parse("\"a\u{1}b\"", "<string>").unwrap_err();
        assert_eq!(err.to_string(), "invalid json: control character in string (<string>:1:3)");
    }

    #[test]
    fn string_scan_is_exact_at_every_offset() {
        for len in 0..40 {
            let body = "é".repeat(len / 2) + &"x".repeat(len % 2);
            let text = format!("[\"{body}\", 1]");
            let node = parse(&text, "<string>").unwrap();
            assert_eq!(node.as_array().unwrap()[0].as_str(), Some(body.as_str()), "{len}");
            let second = node.as_array().unwrap()[1].origin().unwrap();
            assert_eq!(second.column() as usize, body.chars().count() + 6, "{len}");
            let escaped = format!("\"{body}\\n{body}\"");
            let node = parse(&escaped, "<string>").unwrap();
            assert_eq!(node.as_str(), Some(format!("{body}\n{body}").as_str()), "{len}");
            let control = format!("\"{body}\u{1}\"");
            let err = parse(&control, "<string>").unwrap_err();
            let column = body.chars().count() + 2;
            assert_eq!(
                err.to_string(),
                format!("invalid json: control character in string (<string>:1:{column})"),
                "{len}"
            );
        }
    }

    #[test]
    fn rejects_unterminated_string() {
        let err = parse("{\"a", "<string>").unwrap_err();
        assert_eq!(err.to_string(), "invalid json: unterminated string (<string>:1:4)");
    }

    #[test]
    fn keeps_number_text_verbatim() {
        assert_eq!(parsed("1.5e10"), ConfigNode::scalar("1.5e10"));
        assert_eq!(parsed("-0"), ConfigNode::scalar("-0"));
        assert_eq!(parsed("1E+2"), ConfigNode::scalar("1E+2"));
        assert_eq!(parsed("12345678901234567890"), ConfigNode::scalar("12345678901234567890"));
    }

    #[test]
    fn rejects_invalid_numbers() {
        for input in ["01", "+1", "1.", ".5", "1e", "1e+", "-", "0x10", "1.5.2"] {
            assert!(parse(input, "<string>").is_err(), "{input}");
        }
    }

    #[test]
    fn duplicate_key_reports_origin_of_second_key() {
        let err = parse("{\n  \"a\": 1,\n  \"a\": 2}", "<string>").unwrap_err();
        assert_eq!(err.to_string(), "invalid json: duplicate key \"a\" (<string>:3:3)");
    }

    #[test]
    fn earliest_duplicate_wins_over_later_errors_and_keys() {
        let cases = [
            ("{\"b\": 1, \"a\": 1, \"b\": 2, \"a\": 2}", "duplicate key \"b\" (<string>:1:18)"),
            ("{\"a\": 1, \"a\": @}", "duplicate key \"a\" (<string>:1:10)"),
            ("{\"a\": 1, \"a\" 2}", "duplicate key \"a\" (<string>:1:10)"),
            ("{\"a\": 1, \"a\": 2, \"b\": }", "duplicate key \"a\" (<string>:1:10)"),
            ("{\"a\": 1, \"a\": 2 \"b\"}", "duplicate key \"a\" (<string>:1:10)"),
            (
                "{\"x\": {\"a\": 1, \"a\": 2}, \"y\": 1, \"y\": 2}",
                "duplicate key \"a\" (<string>:1:16)",
            ),
        ];
        for (input, message) in cases {
            let err = parse(input, "<string>").unwrap_err();
            assert_eq!(err.to_string(), format!("invalid json: {message}"), "{input}");
        }
    }

    #[test]
    fn duplicate_in_bulk_object_reports_earliest_second_key() {
        let ascending: Vec<String> = (0..20).map(|i| format!("\"k{i:02}\": 0")).collect();
        let mut descending = ascending.clone();
        descending.reverse();
        for members in [ascending, descending] {
            let text = format!("{{{}, \"k0\\u0035\": 1, \"k07\": 2}}", members.join(", "));
            let at = text.find("\"k0\\u0035\"").unwrap() + 1;
            let err = parse(&text, "<string>").unwrap_err();
            assert_eq!(
                err.to_string(),
                format!("invalid json: duplicate key \"k05\" (<string>:1:{at})")
            );
        }
    }

    #[test]
    fn rejects_trailing_commas() {
        assert!(parse("[1,]", "<string>").is_err());
        assert!(parse("{\"a\": 1,}", "<string>").is_err());
    }

    #[test]
    fn rejects_trailing_content() {
        let err = parse("{} x", "<string>").unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid json: unexpected content after the top-level value (<string>:1:4)"
        );
    }

    #[test]
    fn rejects_empty_input_and_bad_literals() {
        assert!(parse("", "<string>").is_err());
        assert!(parse("tr", "<string>").is_err());
        assert!(parse("nul", "<string>").is_err());
        assert!(parse("{\"a\" 1}", "<string>").is_err());
        assert!(parse("{a: 1}", "<string>").is_err());
        assert!(parse("[1 2]", "<string>").is_err());
    }

    #[test]
    fn depth_limit() {
        assert!(parse(&nested(128), "<string>").is_ok());
        let err = parse(&nested(129), "<string>").unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Syntax { format: "json", .. }));
    }

    #[test]
    fn skips_bom() {
        assert_eq!(
            parsed("\u{feff}{\"a\": 1}"),
            ConfigNode::table(Table::from([("a".to_owned(), ConfigNode::scalar("1"))]))
        );
    }
}
