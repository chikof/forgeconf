//! Byte cursor with line/column tracking shared by the in-house parsers.

use std::sync::Arc;

use crate::{ConfigError, ErrorKind, Origin};

/// Deepest allowed nesting of arrays, tables and objects. Deeper input is a
/// `Syntax` error instead of a stack overflow.
pub(crate) const MAX_DEPTH: usize = 128;

/// Position-tracking reader over a `&str`.
///
/// `line` and `column` are 1-based; the column counts characters, not bytes.
pub(crate) struct Cursor<'a> {
    src: &'a str,
    pos: usize,
    line: u32,
    column: u32,
    source: Arc<str>,
    depth: usize,
}

impl<'a> Cursor<'a> {
    /// Start at the first character of `src`; `source` labels errors.
    pub(crate) fn new(src: &'a str, source: impl Into<Arc<str>>) -> Self {
        Self {
            src,
            pos: 0,
            line: 1,
            column: 1,
            source: source.into(),
            depth: 0,
        }
    }

    /// Next byte, without consuming it.
    pub(crate) fn peek(&self) -> Option<u8> {
        self.peek_at(0)
    }

    /// Byte `n` positions ahead of the next one.
    pub(crate) fn peek_at(&self, n: usize) -> Option<u8> {
        self.src.as_bytes().get(self.pos.checked_add(n)?).copied()
    }

    /// Unconsumed input.
    pub(crate) fn rest(&self) -> &'a str {
        self.src.get(self.pos..).unwrap_or("")
    }

    /// Byte offset of the next character.
    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    /// Input from byte offset `start` (a value of [`Cursor::pos`]) up to the
    /// current position.
    #[cfg(any(feature = "toml", feature = "json"))]
    pub(crate) fn slice_from(&self, start: usize) -> &'a str {
        self.src.get(start..self.pos).unwrap_or("")
    }

    /// True when all input is consumed.
    #[cfg(any(feature = "toml", feature = "json"))]
    pub(crate) fn is_eof(&self) -> bool {
        self.pos >= self.src.len()
    }

    /// Consume and return the next character.
    pub(crate) fn bump(&mut self) -> Option<char> {
        let ch = self.src.get(self.pos..)?.chars().next()?;
        self.pos += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    /// Consume `prefix` when the input continues with it.
    pub(crate) fn eat(&mut self, prefix: &str) -> bool {
        let Some(rest) = self.src.get(self.pos..) else { return false };
        if !rest.starts_with(prefix) {
            return false;
        }
        for _ in prefix.chars() {
            self.bump();
        }
        true
    }

    /// Current position as an [`Origin`].
    pub(crate) fn origin(&self) -> Origin {
        Origin::new(Arc::clone(&self.source), self.line, self.column)
    }

    /// A `Syntax` error for `format` at the current position.
    pub(crate) fn error(&self, format: &'static str, msg: impl Into<String>) -> ConfigError {
        ConfigError::new(ErrorKind::Syntax { format, message: msg.into() })
            .with_origin(Some(&self.origin()))
    }

    /// Descend one nesting level; fails past [`MAX_DEPTH`].
    pub(crate) fn enter(&mut self, format: &'static str) -> Result<(), ConfigError> {
        if self.depth >= MAX_DEPTH {
            return Err(self.error(format, format!("nesting deeper than {MAX_DEPTH} levels")));
        }
        self.depth += 1;
        Ok(())
    }

    /// Ascend one nesting level.
    pub(crate) fn leave(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn tracks_line_and_column() {
        let mut cursor = Cursor::new("ab\ncé", "t.json");
        assert_eq!(cursor.origin(), Origin::new("t.json", 1, 1));
        assert_eq!(cursor.bump(), Some('a'));
        assert_eq!(cursor.origin(), Origin::new("t.json", 1, 2));
        cursor.bump();
        assert_eq!(cursor.bump(), Some('\n'));
        assert_eq!(cursor.origin(), Origin::new("t.json", 2, 1));
        cursor.bump();
        assert_eq!(cursor.bump(), Some('é'));
        assert_eq!(cursor.origin(), Origin::new("t.json", 2, 3));
        assert_eq!(cursor.bump(), None);
        assert_eq!(cursor.peek(), None);
    }

    #[test]
    fn peek_does_not_consume() {
        let cursor = Cursor::new("xyz", "s");
        assert_eq!(
            (cursor.peek(), cursor.peek_at(2), cursor.peek_at(3)),
            (Some(b'x'), Some(b'z'), None)
        );
        assert_eq!(cursor.origin(), Origin::new("s", 1, 1));
    }

    #[test]
    fn eat_consumes_only_on_match() {
        let mut cursor = Cursor::new("true\nx", "s");
        assert!(!cursor.eat("false"));
        assert_eq!(cursor.origin(), Origin::new("s", 1, 1));
        assert!(cursor.eat("true\n"));
        assert_eq!(cursor.origin(), Origin::new("s", 2, 1));
        assert!(!cursor.eat("xy"));
    }

    #[test]
    fn error_carries_origin() {
        let mut cursor = Cursor::new("a\nb", "app.toml");
        cursor.bump();
        cursor.bump();
        let err = cursor.error("toml", "unexpected b");
        assert_eq!(err.to_string(), "invalid toml: unexpected b (app.toml:2:1)");
    }

    #[test]
    fn depth_limit_is_enforced() {
        let mut cursor = Cursor::new("", "s");
        for _ in 0..MAX_DEPTH {
            cursor.enter("json").unwrap();
        }
        let err = cursor.enter("json").unwrap_err();
        assert_eq!(err.to_string(), "invalid json: nesting deeper than 128 levels (s:1:1)");
        cursor.leave();
        cursor.enter("json").unwrap();
    }
}
