//! In-house JSON parser (RFC 8259) producing [`ConfigNode`] trees.

use super::cursor::Cursor;
use crate::{ConfigError, ConfigNode, ErrorKind, Origin, Table};

const FORMAT: &str = "json";

/// Parse JSON content into a [`ConfigNode`] tree. Any JSON value is a valid
/// document; a leading UTF-8 BOM is skipped.
pub(crate) fn parse(input: &str, source: &str) -> Result<ConfigNode, ConfigError> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut cursor = Cursor::new(input, source);
    skip_ws(&mut cursor);
    let node = parse_value(&mut cursor)?;
    skip_ws(&mut cursor);
    if !cursor.is_eof() {
        return Err(cursor.error(FORMAT, "unexpected content after the top-level value"));
    }
    Ok(node)
}

fn skip_ws(c: &mut Cursor<'_>) {
    while matches!(c.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        c.bump();
    }
}

fn parse_value(c: &mut Cursor<'_>) -> Result<ConfigNode, ConfigError> {
    let origin = c.origin();
    let node = match c.peek() {
        Some(b'{') => {
            c.enter(FORMAT)?;
            let node = parse_object(c);
            c.leave();
            node?
        },
        Some(b'[') => {
            c.enter(FORMAT)?;
            let node = parse_array(c);
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
        c.bump();
        Ok(())
    } else {
        Err(unexpected(c, expected))
    }
}

fn parse_object(c: &mut Cursor<'_>) -> Result<ConfigNode, ConfigError> {
    c.bump();
    let mut table = Table::new();
    skip_ws(c);
    if c.peek() == Some(b'}') {
        c.bump();
        return Ok(ConfigNode::table(table));
    }
    loop {
        skip_ws(c);
        if c.peek() != Some(b'"') {
            return Err(unexpected(c, "a string key"));
        }
        let key_origin = c.origin();
        let key = parse_string(c)?;
        if table.contains_key(&key) {
            return Err(duplicate_key(&key, &key_origin));
        }
        skip_ws(c);
        expect(c, b':', "`:`")?;
        skip_ws(c);
        let value = parse_value(c)?;
        table.insert(key, value);
        skip_ws(c);
        match c.peek() {
            Some(b',') => {
                c.bump();
            },
            Some(b'}') => {
                c.bump();
                return Ok(ConfigNode::table(table));
            },
            _ => return Err(unexpected(c, "`,` or `}`")),
        }
    }
}

fn duplicate_key(key: &str, origin: &Origin) -> ConfigError {
    ConfigError::new(ErrorKind::Syntax {
        format: FORMAT,
        message: format!("duplicate key \"{key}\""),
    })
    .with_origin(Some(origin))
}

fn parse_array(c: &mut Cursor<'_>) -> Result<ConfigNode, ConfigError> {
    c.bump();
    let mut items = Vec::new();
    skip_ws(c);
    if c.peek() == Some(b']') {
        c.bump();
        return Ok(ConfigNode::array(items));
    }
    loop {
        skip_ws(c);
        items.push(parse_value(c)?);
        skip_ws(c);
        match c.peek() {
            Some(b',') => {
                c.bump();
            },
            Some(b']') => {
                c.bump();
                return Ok(ConfigNode::array(items));
            },
            _ => return Err(unexpected(c, "`,` or `]`")),
        }
    }
}

fn parse_string(c: &mut Cursor<'_>) -> Result<String, ConfigError> {
    c.bump();
    let mut out = String::new();
    loop {
        match c.peek() {
            None => return Err(c.error(FORMAT, "unterminated string")),
            Some(b'"') => {
                c.bump();
                return Ok(out);
            },
            Some(b'\\') => {
                c.bump();
                out.push(parse_escape(c)?);
            },
            Some(byte) if byte < 0x20 => {
                return Err(c.error(FORMAT, "control character in string"));
            },
            Some(_) => match c.bump() {
                Some(ch) => out.push(ch),
                None => return Err(c.error(FORMAT, "unterminated string")),
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
    let start = c.pos();
    if c.peek() == Some(b'-') {
        c.bump();
    }
    match c.peek() {
        Some(b'0') => {
            c.bump();
        },
        Some(b'1'..=b'9') => skip_digits(c),
        _ => return Err(unexpected(c, "a digit")),
    }
    if c.peek() == Some(b'.') {
        c.bump();
        require_digit(c)?;
        skip_digits(c);
    }
    if matches!(c.peek(), Some(b'e' | b'E')) {
        c.bump();
        if matches!(c.peek(), Some(b'+' | b'-')) {
            c.bump();
        }
        require_digit(c)?;
        skip_digits(c);
    }
    Ok(c.slice_from(start).to_owned())
}

fn require_digit(c: &Cursor<'_>) -> Result<(), ConfigError> {
    if matches!(c.peek(), Some(b'0'..=b'9')) { Ok(()) } else { Err(unexpected(c, "a digit")) }
}

fn skip_digits(c: &mut Cursor<'_>) {
    while matches!(c.peek(), Some(b'0'..=b'9')) {
        c.bump();
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;
    use crate::Value;

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
