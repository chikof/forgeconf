//! `.env` file parsing. The loader reads the file at `load()` time and uses
//! its values as a fallback below the real environment.

use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, io};

use crate::{ConfigError, ErrorKind, Origin};

/// Read and parse `path`; a missing file is an empty map. Later duplicates of
/// a key win.
pub(crate) fn load(path: &Path) -> Result<BTreeMap<String, String>, ConfigError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(source) => {
            return Err(ConfigError::new(ErrorKind::Io { path: path.to_path_buf(), source }));
        },
    };
    Ok(parse_dotenv(&text, &path.display().to_string())?
        .into_iter()
        .collect())
}

/// Parse dotenv `text` into ordered `(key, value)` pairs. `source` labels
/// syntax errors.
pub(crate) fn parse_dotenv(text: &str, source: &str) -> Result<Vec<(String, String)>, ConfigError> {
    let mut pairs = Vec::new();
    let mut pos = 0;

    while pos < text.len() {
        pos = skip_blanks(text, pos);
        match text[pos..].chars().next() {
            None => break,
            Some('\n') => pos += 1,
            Some('#') => pos = end_of_line(text, pos),
            Some(_) => {
                let (pair, next) = parse_entry(text, pos, source)?;
                pairs.push(pair);
                pos = next;
            },
        }
    }
    Ok(pairs)
}

fn skip_blanks(text: &str, pos: usize) -> usize {
    let rest = &text[pos..];
    pos + (rest.len() - rest.trim_start_matches([' ', '\t', '\r']).len())
}

/// Index of the `\n` ending the line that contains `pos`, or `text.len()`.
fn end_of_line(text: &str, pos: usize) -> usize {
    text[pos..].find('\n').map_or(text.len(), |i| pos + i)
}

fn syntax_error(text: &str, pos: usize, source: &str, message: &str) -> ConfigError {
    let before = &text[..pos];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let column = text[line_start..pos].chars().count() + 1;
    ConfigError::new(ErrorKind::Syntax {
        format: "dotenv",
        message: message.to_owned(),
    })
    .with_origin(Some(&Origin::new(
        source,
        u32::try_from(line).unwrap_or(u32::MAX),
        u32::try_from(column).unwrap_or(u32::MAX),
    )))
}

/// Parse one `KEY=value` entry starting at `pos`; returns it and the index
/// where parsing resumes.
fn parse_entry(
    text: &str,
    mut pos: usize,
    source: &str,
) -> Result<((String, String), usize), ConfigError> {
    // `export` is a prefix only when whitespace follows and the key is not
    // `export` itself (`export = x`).
    let has_prefix = text[pos..]
        .strip_prefix("export")
        .is_some_and(|after| after.starts_with([' ', '\t']));
    if has_prefix {
        let key_start = skip_blanks(text, pos + "export".len());
        if !text[key_start..].starts_with('=') {
            pos = key_start;
        }
    }

    let key_end = pos
        + text[pos..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
            .unwrap_or(text.len() - pos);
    let key = &text[pos..key_end];
    if key.is_empty() || key.starts_with(|c: char| c.is_ascii_digit() || c == '.') {
        return Err(syntax_error(text, pos, source, "key must match [A-Za-z_][A-Za-z0-9_.]*"));
    }

    let eq = skip_blanks(text, key_end);
    if !text[eq..].starts_with('=') {
        return Err(syntax_error(text, eq, source, "expected `=` after the key"));
    }
    let value_start = skip_blanks(text, eq + 1);

    let (value, next) = match text[value_start..].chars().next() {
        Some('"') => parse_double_quoted(text, value_start, source)?,
        Some('\'') => parse_single_quoted(text, value_start, source)?,
        _ => {
            let line_end = end_of_line(text, eq + 1);
            (unquoted(&text[eq + 1..line_end]), line_end)
        },
    };
    Ok(((key.to_owned(), value), next))
}

/// Trimmed value with a trailing ` #comment` removed. A `#` only starts a
/// comment when whitespace precedes it.
fn unquoted(raw: &str) -> String {
    let mut end = raw.len();
    let mut previous_blank = false;
    for (i, c) in raw.char_indices() {
        if c == '#' && previous_blank {
            end = i;
            break;
        }
        previous_blank = c.is_whitespace();
    }
    raw[..end].trim().to_owned()
}

/// After a quoted value only whitespace or a comment may follow on its line.
fn finish_quoted(
    text: &str,
    after: usize,
    source: &str,
    value: String,
) -> Result<(String, usize), ConfigError> {
    let rest_start = skip_blanks(text, after);
    match text[rest_start..].chars().next() {
        None | Some('\n' | '#') => Ok((value, rest_start)),
        Some(_) => {
            Err(syntax_error(text, rest_start, source, "unexpected text after the quoted value"))
        },
    }
}

fn parse_single_quoted(
    text: &str,
    start: usize,
    source: &str,
) -> Result<(String, usize), ConfigError> {
    let body = start + 1;
    let Some(len) = text[body..].find('\'') else {
        return Err(syntax_error(text, start, source, "unterminated single-quoted value"));
    };
    finish_quoted(text, body + len + 1, source, text[body..body + len].to_owned())
}

fn parse_double_quoted(
    text: &str,
    start: usize,
    source: &str,
) -> Result<(String, usize), ConfigError> {
    let mut value = String::new();
    let mut chars = text[start + 1..].char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return finish_quoted(text, start + 1 + i + 1, source, value),
            '\\' => match chars.next() {
                Some((_, 'n')) => value.push('\n'),
                Some((_, 't')) => value.push('\t'),
                Some((_, 'r')) => value.push('\r'),
                Some((_, escaped @ ('"' | '\\' | '$'))) => value.push(escaped),
                // Unknown escapes stay literal so `"C:\dir"` survives.
                Some((_, other)) => {
                    value.push('\\');
                    value.push(other);
                },
                None => break,
            },
            c => value.push(c),
        }
    }
    Err(syntax_error(text, start, source, "unterminated double-quoted value"))
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;

    fn pairs(input: &str) -> Vec<(String, String)> {
        parse_dotenv(input, "t.env").unwrap()
    }

    fn only(input: &str) -> String {
        let mut found = pairs(input);
        assert_eq!(found.len(), 1);
        found.remove(0).1
    }

    fn pair(key: &str, value: &str) -> (String, String) {
        (key.to_owned(), value.to_owned())
    }

    #[test]
    fn basic_key_value() {
        assert_eq!(
            pairs("PORT=3000\nHOST=localhost"),
            [pair("PORT", "3000"), pair("HOST", "localhost")]
        );
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        assert_eq!(pairs("# comment\n\n  # indented\nPORT=8080\n"), [pair("PORT", "8080")]);
    }

    #[test]
    fn export_prefix_needs_whitespace() {
        assert_eq!(
            pairs("export KEY=v\nexport_url=x\nexport\tTAB=1"),
            [pair("KEY", "v"), pair("export_url", "x"), pair("TAB", "1")]
        );
    }

    #[test]
    fn export_can_be_a_key() {
        assert_eq!(pairs("export = 1"), [pair("export", "1")]);
    }

    #[test]
    fn spaces_around_equals_and_trimmed_values() {
        assert_eq!(only("KEY  =   spaced value   "), "spaced value");
        assert_eq!(only("KEY="), "");
    }

    #[test]
    fn dots_are_allowed_in_keys() {
        assert_eq!(pairs("a.b_c=1"), [pair("a.b_c", "1")]);
    }

    #[test]
    fn double_quoted_escapes() {
        assert_eq!(only(r#"MSG="a\nb\tc\rd\"e\\f\$g""#), "a\nb\tc\rd\"e\\f$g");
    }

    #[test]
    fn double_quoted_unknown_escape_is_literal() {
        assert_eq!(only(r#"P="C:\dir""#), "C:\\dir");
    }

    #[test]
    fn double_quoted_may_span_lines() {
        assert_eq!(only("KEY=\"line1\nline2\"\n"), "line1\nline2");
    }

    #[test]
    fn single_quoted_is_literal_and_may_span_lines() {
        assert_eq!(only(r"MSG='a\nb'"), "a\\nb");
        assert_eq!(only("MSG='x\ny'"), "x\ny");
    }

    #[test]
    fn inline_comments() {
        assert_eq!(only("PORT=9000 # production"), "9000");
        assert_eq!(only("PORT= # nothing"), "");
        assert_eq!(only("COLOR=#fff"), "#fff");
        assert_eq!(only("URL=http://h/#frag"), "http://h/#frag");
        assert_eq!(only("T=\"abc#def\" # c"), "abc#def");
        assert_eq!(only("T='abc #def'"), "abc #def");
    }

    #[test]
    fn no_variable_expansion() {
        assert_eq!(only("A=${B}/x"), "${B}/x");
    }

    #[test]
    fn crlf_lines() {
        assert_eq!(pairs("A=1\r\nB=\"2\"\r\n"), [pair("A", "1"), pair("B", "2")]);
    }

    #[test]
    fn later_line_numbers_count_multiline_values() {
        let err = parse_dotenv("A=\"x\ny\"\n1BAD=z", "t.env").unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid dotenv: key must match [A-Za-z_][A-Za-z0-9_.]* (t.env:3:1)"
        );
    }

    #[test]
    fn invalid_key_reports_position() {
        let err = parse_dotenv("OK=1\n  -bad=2", "t.env").unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid dotenv: key must match [A-Za-z_][A-Za-z0-9_.]* (t.env:2:3)"
        );
    }

    #[test]
    fn missing_equals() {
        let err = parse_dotenv("JUST_A_WORD\n", "t.env").unwrap_err();
        assert_eq!(err.to_string(), "invalid dotenv: expected `=` after the key (t.env:1:12)");
    }

    #[test]
    fn unterminated_quotes() {
        let err = parse_dotenv("A=1\nB=\"open\nmore", "t.env").unwrap_err();
        assert_eq!(err.to_string(), "invalid dotenv: unterminated double-quoted value (t.env:2:3)");
        let err = parse_dotenv("B='open", "t.env").unwrap_err();
        assert_eq!(err.to_string(), "invalid dotenv: unterminated single-quoted value (t.env:1:3)");
    }

    #[test]
    fn text_after_quoted_value() {
        let err = parse_dotenv("A=\"x\" y", "t.env").unwrap_err();
        assert_eq!(
            err.to_string(),
            "invalid dotenv: unexpected text after the quoted value (t.env:1:7)"
        );
    }

    #[test]
    fn load_ignores_missing_file_and_last_duplicate_wins() {
        assert!(
            load(Path::new("/definitely/not/here.env"))
                .unwrap()
                .is_empty()
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        fs::write(&path, "A=1\nA=2\n").unwrap();
        assert_eq!(load(&path).unwrap().get("A").map(String::as_str), Some("2"));
    }

    #[test]
    fn load_reports_other_io_errors() {
        let dir = tempfile::tempdir().unwrap();
        let err = load(dir.path()).unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Io { .. }));
    }
}
