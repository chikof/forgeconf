use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::ConfigSource;
use crate::{ConfigError, ConfigNode};

/// Source that reads a `.env` file and injects key-value pairs into the
/// configuration tree.
///
/// Keys are lowercased by default so `DATABASE_URL=...` maps to the field
/// `database_url`. Call [`preserve_case`](DotenvSource::preserve_case) to
/// keep keys as written.
///
/// Missing files are silently ignored unless
/// [`required`](DotenvSource::required) is called.
pub struct DotenvSource {
    path: PathBuf,
    lowercase: bool,
    required: bool,
    priority: u8,
}

impl DotenvSource {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
            lowercase: true,
            required: false,
            priority: 5,
        }
    }

    /// Keep keys exactly as written in the file (e.g. `DATABASE_URL`).
    pub fn preserve_case(mut self) -> Self {
        self.lowercase = false;
        self
    }

    /// Return an error if the file does not exist instead of producing an
    /// empty table.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

impl ConfigSource for DotenvSource {
    fn priority(&self) -> u8 {
        self.priority
    }

    fn load(&self) -> Result<ConfigNode, ConfigError> {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !self.required => {
                return Ok(ConfigNode::empty_table());
            },
            Err(e) => {
                return Err(ConfigError::Io(e));
            },
        };

        let mut map = BTreeMap::new();
        for (raw_key, value) in parse_dotenv(&content) {
            let key = if self.lowercase { raw_key.to_lowercase() } else { raw_key };
            map.insert(key, ConfigNode::Scalar(value));
        }

        Ok(ConfigNode::Table(map))
    }
}

fn parse_dotenv(content: &str) -> Vec<(String, String)> {
    let mut pairs = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // skip comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line
            .strip_prefix("export")
            .map(str::trim_start)
            .unwrap_or(line);

        let Some((key, rest)) = line.split_once('=') else { continue };
        let key = key.trim().to_string();
        if key.is_empty() || key.contains(char::is_whitespace) {
            continue;
        }

        let value = parse_value(rest.trim());
        pairs.push((key, value));
    }

    pairs
}

fn parse_value(raw: &str) -> String {
    if raw.starts_with('"') {
        // double-quoted: handle escape sequences, stop at closing "
        let inner = raw.get(1..).unwrap_or("");
        let mut out = String::new();
        let mut chars = inner.chars();
        loop {
            match chars.next() {
                None | Some('"') => break,
                Some('\\') => match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some(c) => out.push(c),
                    None => break,
                },
                Some(c) => out.push(c),
            }
        }
        out
    } else if raw.starts_with('\'') {
        // single-quoted: literal, no escapes
        let inner = raw.get(1..).unwrap_or("");
        inner
            .find('\'')
            .map(|i| inner[..i].to_string())
            .unwrap_or_else(|| inner.to_string())
    } else {
        // unquoted: strip inline comment
        raw.split_once(" #")
            .map(|(v, _)| v.trim_end())
            .unwrap_or(raw)
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(input: &str) -> Vec<(String, String)> {
        parse_dotenv(input)
    }

    #[test]
    fn basic_key_value() {
        let p = pairs("PORT=3000\nHOST=localhost");
        assert_eq!(p, vec![("PORT".into(), "3000".into()), ("HOST".into(), "localhost".into())]);
    }

    #[test]
    fn skips_comments_and_blank_lines() {
        let p = pairs("# comment\n\nPORT=8080");
        assert_eq!(p, vec![("PORT".into(), "8080".into())]);
    }

    #[test]
    fn strips_export_prefix() {
        let p = pairs("export DATABASE_URL=postgres://localhost/app");
        assert_eq!(p, vec![("DATABASE_URL".into(), "postgres://localhost/app".into())]);
    }

    #[test]
    fn double_quoted_with_escapes() {
        let p = pairs(r#"MSG="hello\nworld""#);
        assert_eq!(p[0].1, "hello\nworld");
    }

    #[test]
    fn single_quoted_literal() {
        let p = pairs("MSG='don\\'t escape'");
        assert_eq!(p[0].1, "don\\");
    }

    #[test]
    fn inline_comment_stripped() {
        let p = pairs("PORT=9000 # production port");
        assert_eq!(p[0].1, "9000");
    }

    #[test]
    fn double_quoted_inline_hash_not_stripped() {
        let p = pairs(r#"TOKEN="abc#def""#);
        assert_eq!(p[0].1, "abc#def");
    }
}
