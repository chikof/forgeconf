//! In-house parser for the YAML subset forgeconf supports.
//!
//! Supported: block mappings and sequences (space indentation), flow
//! collections, plain / single-quoted / double-quoted scalars, `|` and `>`
//! block scalars, comments, one optional `---` and one optional trailing `...`.
//! Anchors, aliases, tags, directives, complex and empty keys, merge keys and
//! further documents are rejected with `Unsupported`. Scalars stay text; only
//! the null forms become `Null`.

use std::iter::repeat_n;

use super::cursor::Cursor;
use crate::{ConfigError, ConfigNode, ErrorKind, Origin, Table, Text};

const FORMAT: &str = "yaml";

type Res<T> = Result<T, ConfigError>;

/// Parse YAML content into a [`ConfigNode`] tree. An empty document is an
/// empty table.
pub(crate) fn parse(input: &str, source: &str) -> Res<ConfigNode> {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let mut parser = Parser {
        cur: Cursor::new(input, source),
        content_pos: usize::MAX,
        content_indent: 0,
        tab: None,
    };
    parser.document()
}

fn syntax_at(origin: &Origin, message: impl Into<String>) -> ConfigError {
    ConfigError::new(ErrorKind::Syntax { format: FORMAT, message: message.into() })
        .with_origin(Some(origin))
}

fn unsupported_at(origin: &Origin, message: &str) -> ConfigError {
    ConfigError::new(ErrorKind::Unsupported(message.to_owned())).with_origin(Some(origin))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Marker {
    Start,
    End,
}

#[derive(Clone, Copy)]
enum Chomp {
    Strip,
    Clip,
    Keep,
}

enum Stop {
    Eol,
    Comment,
    Colon,
    Flow,
}

/// What a block-context node starts with.
enum Head {
    Key(Text, Origin),
    Scalar(ConfigNode),
}

struct Parser<'a> {
    cur: Cursor<'a>,
    /// Byte offset where [`Parser::next_content`] last stopped, so a second
    /// call without consuming anything returns the same answer.
    content_pos: usize,
    content_indent: usize,
    /// Where the first tab sits in the leading whitespace of the content line
    /// the parser is on. A tab may separate a scalar or flow collection from
    /// the margin, but never indent a block collection.
    tab: Option<Origin>,
}

impl Parser<'_> {
    fn document(&mut self) -> Res<ConfigNode> {
        let start = self.cur.origin();
        let root = match self.next_content()? {
            None => None,
            Some(0) if self.cur.peek() == Some(b'%') => {
                return Err(unsupported_at(
                    &self.cur.origin(),
                    "yaml directives (%) are not supported",
                ));
            },
            Some(0) if self.marker() == Some(Marker::Start) => {
                self.cur.eat("---");
                self.skip_spaces();
                if self.at_line_end() {
                    Some(self.nested(-1, false)?)
                } else {
                    // A block collection cannot start on the `---` line.
                    let indent = self.column();
                    Some(match self.cur.peek() {
                        Some(b'[' | b'{') => self.node(indent, -1)?,
                        _ => self.value(-1, false)?,
                    })
                }
            },
            Some(0) if self.marker() == Some(Marker::End) => None,
            Some(indent) => Some(self.node(indent, -1)?),
        };
        match self.next_content()? {
            None => {},
            Some(0) if self.marker() == Some(Marker::End) => {
                self.cur.eat("...");
                if self.next_content()?.is_some() {
                    return Err(self.multiple_documents());
                }
            },
            Some(0) if self.marker() == Some(Marker::Start) => {
                return Err(self.multiple_documents());
            },
            Some(_) => {
                return Err(self
                    .cur
                    .error(FORMAT, "bad indentation or unexpected content"));
            },
        }
        Ok(match root {
            Some(node) if !node.is_null() => node,
            _ => ConfigNode::table(Table::new()).with_origin(start),
        })
    }

    fn multiple_documents(&self) -> ConfigError {
        unsupported_at(&self.cur.origin(), "yaml documents after the first are not supported")
    }

    // ---- cursor helpers -------------------------------------------------

    /// Zero-based column of the next character.
    fn column(&self) -> usize {
        usize::try_from(self.cur.origin().column())
            .unwrap_or(1)
            .saturating_sub(1)
    }

    /// Skip spaces and tabs; true when a tab was among them.
    fn skip_spaces(&mut self) -> bool {
        let mut tab = false;
        while let Some(byte @ (b' ' | b'\t')) = self.cur.peek() {
            tab |= byte == b'\t';
            self.cur.bump();
        }
        tab
    }

    fn no_tab_indent(&self) -> Res<()> {
        match &self.tab {
            Some(origin) => Err(syntax_at(origin, "tabs are not allowed for indentation")),
            None => Ok(()),
        }
    }

    fn comment_needs_space(&self) -> ConfigError {
        self.cur
            .error(FORMAT, "a comment needs whitespace before '#'")
    }

    fn skip_comment(&mut self) {
        while !matches!(self.cur.peek(), None | Some(b'\n' | b'\r')) {
            self.cur.bump();
        }
    }

    /// Consume one line break (`\n` or `\r\n`).
    fn eol(&mut self) {
        if !self.cur.eat("\r\n") {
            self.cur.bump();
        }
    }

    fn advance_to(&mut self, target: usize) {
        while self.cur.pos() < target && self.cur.bump().is_some() {}
    }

    fn advance(&mut self, bytes: usize) {
        self.advance_to(self.cur.pos().saturating_add(bytes));
    }

    /// True at end of input, a line break or a comment.
    fn at_line_end(&self) -> bool {
        matches!(self.cur.peek(), None | Some(b'\n' | b'\r' | b'#'))
    }

    fn is_seq_entry(&self) -> bool {
        self.cur.peek() == Some(b'-') && is_space_or_end(self.cur.peek_at(1))
    }

    fn marker(&self) -> Option<Marker> {
        marker_of(self.cur.rest())
    }

    fn doc_marker(&self, indent: usize) -> bool {
        indent == 0 && self.marker().is_some()
    }

    /// Finish the current line, skip blank and comment lines, and stop at the
    /// next content character. Returns its indentation, or `None` at end of
    /// input.
    fn next_content(&mut self) -> Res<Option<usize>> {
        if self.cur.pos() == self.content_pos {
            return Ok(Some(self.content_indent));
        }
        loop {
            let line_start = self.column() == 0;
            let mut spaced = line_start || matches!(self.cur.prev_byte(), Some(b' ' | b'\t'));
            self.tab = None;
            while let Some(byte @ (b' ' | b'\t')) = self.cur.peek() {
                if byte == b'\t' && line_start && self.tab.is_none() {
                    self.tab = Some(self.cur.origin());
                }
                spaced = true;
                self.cur.bump();
            }
            match self.cur.peek() {
                None => return Ok(None),
                Some(b'\n' | b'\r') => self.eol(),
                Some(b'#') if spaced => self.skip_comment(),
                Some(b'#') => return Err(self.comment_needs_space()),
                Some(_) if !line_start => {
                    let found = self.cur.rest().chars().next().unwrap_or(' ');
                    return Err(self.cur.error(FORMAT, format!("unexpected '{found}'")));
                },
                Some(_) => {
                    self.content_pos = self.cur.pos();
                    self.content_indent = self.column();
                    return Ok(Some(self.content_indent));
                },
            }
        }
    }

    fn reject_indicator(&self) -> Res<()> {
        let origin = self.cur.origin();
        match self.cur.peek() {
            Some(b'&') => Err(unsupported_at(&origin, "yaml anchors (&) are not supported")),
            Some(b'*') => Err(unsupported_at(&origin, "yaml aliases (*) are not supported")),
            Some(b'!') => Err(unsupported_at(&origin, "yaml tags (!) are not supported")),
            Some(b'?') if is_space_or_end(self.cur.peek_at(1)) => {
                Err(unsupported_at(&origin, "yaml complex keys (?) are not supported"))
            },
            Some(b'%' | b'@' | b'`') => {
                let found = self.cur.rest().chars().next().unwrap_or(' ');
                Err(syntax_at(&origin, format!("'{found}' cannot start a value")))
            },
            _ => Ok(()),
        }
    }

    // ---- block structure ------------------------------------------------

    /// Value on the lines after a `key:` or `-` whose own line is empty.
    /// `parent` is the indentation of the enclosing collection. A sequence may
    /// sit at the mapping's own indentation (`seq_at_parent`).
    fn nested(&mut self, parent: isize, seq_at_parent: bool) -> Res<ConfigNode> {
        let origin = self.cur.origin();
        match self.next_content()? {
            Some(indent) if exceeds(indent, parent) && !self.doc_marker(indent) => {
                self.node(indent, parent)
            },
            Some(indent) if seq_at_parent && to_isize(indent) == parent && self.is_seq_entry() => {
                self.sequence(indent)
            },
            _ => Ok(ConfigNode::null().with_origin(origin)),
        }
    }

    /// Node whose first character is at the cursor, at column `indent`.
    fn node(&mut self, indent: usize, parent: isize) -> Res<ConfigNode> {
        self.reject_indicator()?;
        match self.cur.peek() {
            Some(b'-') if self.is_seq_entry() => self.sequence(indent),
            Some(b'[' | b'{') => {
                let node = self.value(parent, true)?;
                self.skip_spaces();
                if self.colon_follows() {
                    return Err(unsupported_at(
                        &self.cur.origin(),
                        "yaml complex keys (?) are not supported",
                    ));
                }
                Ok(node)
            },
            Some(b'|' | b'>') => self.block_scalar(parent),
            _ => match self.head(parent)? {
                Head::Key(key, origin) => {
                    self.no_tab_indent()?;
                    self.mapping(indent, key, origin)
                },
                Head::Scalar(node) => Ok(node),
            },
        }
    }

    fn sequence(&mut self, indent: usize) -> Res<ConfigNode> {
        self.no_tab_indent()?;
        self.cur.enter(FORMAT)?;
        let origin = self.cur.origin();
        let mut items = Vec::new();
        loop {
            self.cur.bump();
            self.tab = self.skip_spaces().then(|| self.cur.origin());
            let parent = to_isize(indent);
            let item = if self.at_line_end() {
                self.nested(parent, false)?
            } else {
                let column = self.column();
                self.node(column, parent)?
            };
            items.push(item);
            let next = self.next_content()?;
            self.no_tab_indent()?;
            match next {
                Some(next) if next == indent && !self.doc_marker(next) && self.is_seq_entry() => {},
                Some(next) if next > indent => {
                    return Err(self
                        .cur
                        .error(FORMAT, "bad indentation of a sequence entry"));
                },
                _ => break,
            }
        }
        self.cur.leave();
        Ok(ConfigNode::array(items).with_origin(origin))
    }

    /// Block mapping at `indent`; the first key and its `:` are consumed.
    fn mapping(&mut self, indent: usize, first: Text, first_origin: Origin) -> Res<ConfigNode> {
        self.cur.enter(FORMAT)?;
        let origin = first_origin.clone();
        let parent = to_isize(indent);
        let mut table = Table::new();
        let (mut key, mut key_origin) = (first, first_origin);
        loop {
            if table.contains_key(&*key) {
                return Err(syntax_at(&key_origin, format!("duplicate key \"{key}\"")));
            }
            self.skip_spaces();
            let value = if self.at_line_end() {
                self.nested(parent, true)?
            } else {
                self.value(parent, false)?
            };
            table.insert(key, value);
            let next = self.next_content()?;
            self.no_tab_indent()?;
            match next {
                Some(next) if next == indent && !self.doc_marker(next) => {
                    (key, key_origin) = self.key()?;
                },
                Some(next) if next > indent => {
                    return Err(self.cur.error(FORMAT, "bad indentation of a mapping entry"));
                },
                _ => break,
            }
        }
        self.cur.leave();
        Ok(ConfigNode::table(table).with_origin(origin))
    }

    /// Block-context node start: a mapping key (consumed with its `:`) or a
    /// scalar.
    fn head(&mut self, parent: isize) -> Res<Head> {
        let origin = self.cur.origin();
        if matches!(self.cur.peek(), Some(b'"' | b'\'')) {
            let text = self.quoted(parent)?;
            if self.eat_key_colon() {
                self.single_line_key(&origin)?;
                return Ok(Head::Key(Text::from(text), origin));
            }
            return Ok(Head::Scalar(ConfigNode::scalar(text).with_origin(origin)));
        }
        if let Some(key) = self.plain_key()? {
            return Ok(Head::Key(key, origin));
        }
        Ok(Head::Scalar(self.plain_scalar(parent, false)?))
    }

    /// An implicit key cannot span lines.
    fn single_line_key(&self, start: &Origin) -> Res<()> {
        if self.cur.origin().line() == start.line() {
            return Ok(());
        }
        Err(syntax_at(start, "implicit mapping keys must stay on one line"))
    }

    /// Next key of a block mapping, with its `:` consumed.
    fn key(&mut self) -> Res<(Text, Origin)> {
        self.reject_indicator()?;
        let origin = self.cur.origin();
        if self.is_seq_entry() {
            return Err(syntax_at(&origin, "sequence entry inside a mapping"));
        }
        if matches!(self.cur.peek(), Some(b'[' | b'{')) {
            return Err(unsupported_at(&origin, "yaml complex keys (?) are not supported"));
        }
        if matches!(self.cur.peek(), Some(b'"' | b'\'')) {
            let text = self.quoted(-1)?;
            if !self.eat_key_colon() {
                return Err(syntax_at(&origin, "expected ':' after mapping key"));
            }
            self.single_line_key(&origin)?;
            return Ok((Text::from(text), origin));
        }
        match self.plain_key()? {
            Some(key) => Ok((key, origin)),
            None => Err(syntax_at(&origin, "expected a mapping key (\"key: value\")")),
        }
    }

    /// Consume a plain key and its `:` when the line holds one.
    fn plain_key(&mut self) -> Res<Option<Text>> {
        let origin = self.cur.origin();
        let rest = self.cur.rest();
        let (end, colon, stop) = scan_plain(rest, false);
        if !matches!(stop, Stop::Colon) {
            return Ok(None);
        }
        let key = rest.get(..end).unwrap_or("");
        check_plain_key(key, &origin)?;
        self.advance(colon + 1);
        Ok(Some(Text::from(key)))
    }

    fn colon_follows(&self) -> bool {
        self.cur.peek() == Some(b':') && is_space_or_end(self.cur.peek_at(1))
    }

    fn eat_key_colon(&mut self) -> bool {
        self.skip_spaces();
        if self.colon_follows() {
            self.cur.bump();
            return true;
        }
        false
    }

    // ---- values ---------------------------------------------------------

    /// Value starting at the cursor on the same line as its key or `-`, or
    /// inside a flow collection.
    fn value(&mut self, parent: isize, flow: bool) -> Res<ConfigNode> {
        self.reject_indicator()?;
        match self.cur.peek() {
            Some(b'[') => self.flow_seq(parent),
            Some(b'{') => self.flow_map(parent),
            Some(b'"' | b'\'') => {
                let origin = self.cur.origin();
                Ok(ConfigNode::scalar(self.quoted(parent)?).with_origin(origin))
            },
            Some(b'|' | b'>') if flow => Err(self
                .cur
                .error(FORMAT, "block scalars are not allowed inside flow collections")),
            Some(b'|' | b'>') => self.block_scalar(parent),
            Some(b'-') if self.dash_entry(flow) => Err(self
                .cur
                .error(FORMAT, "block sequence entries are not allowed in this context")),
            _ => self.plain_scalar(parent, flow),
        }
    }

    /// A `-` that cannot start a plain scalar: followed by a space or the end
    /// of the line, or inside flow collections also by a flow indicator.
    fn dash_entry(&self, flow: bool) -> bool {
        self.cur.peek() == Some(b'-')
            && (is_space_or_end(self.cur.peek_at(1))
                || (flow && self.cur.peek_at(1).is_some_and(is_flow_indicator)))
    }

    fn plain_scalar(&mut self, parent: isize, flow: bool) -> Res<ConfigNode> {
        let origin = self.cur.origin();
        let text = self.plain_text(parent, flow)?;
        Ok(plain_node(text, origin))
    }

    /// Text of a plain scalar, with multi-line folding.
    fn plain_text(&mut self, parent: isize, flow: bool) -> Res<String> {
        let mut text = String::new();
        loop {
            let rest = self.cur.rest();
            let (end, stop_at, stop) = scan_plain(rest, flow);
            text.push_str(rest.get(..end).unwrap_or(""));
            if !flow && matches!(stop, Stop::Colon) {
                self.advance(stop_at);
                return Err(self
                    .cur
                    .error(FORMAT, "mapping values are not allowed in this context"));
            }
            self.advance(end);
            if !matches!(stop, Stop::Eol) {
                break;
            }
            let Some((target, breaks)) = self.continuation(parent, flow) else { break };
            if breaks == 1 {
                text.push(' ');
            } else {
                text.extend(repeat_n('\n', breaks - 1));
            }
            self.advance_to(target);
        }
        Ok(text)
    }

    /// Where a plain scalar ending at the cursor continues on a later line:
    /// byte offset of the continuation text and the number of line breaks
    /// crossed.
    fn continuation(&self, parent: isize, flow: bool) -> Option<(usize, usize)> {
        let rest = self.cur.rest();
        let mut it = rest.trim_start_matches([' ', '\t']);
        let mut breaks = 0;
        loop {
            if it.is_empty() {
                return None;
            }
            it = strip_break(it);
            breaks += 1;
            let spaces = it.bytes().take_while(|&b| b == b' ').count();
            let body = it
                .get(spaces..)
                .unwrap_or("")
                .trim_start_matches([' ', '\t']);
            match body.bytes().next() {
                None | Some(b'#') => return None,
                Some(b'\n' | b'\r') => it = body,
                Some(first) => {
                    let next = body.as_bytes().get(1).copied();
                    // In flow context a line starting with `:` is a value
                    // indicator, so it ends the scalar (a key).
                    let ends = !exceeds(spaces, parent)
                        || (flow
                            && (is_flow_indicator(first)
                                || (first == b':'
                                    && (is_space_or_end(next)
                                        || next.is_some_and(is_flow_indicator)))));
                    if ends || (spaces == 0 && marker_of(body).is_some()) {
                        return None;
                    }
                    return Some((self.cur.pos() + rest.len() - body.len(), breaks));
                },
            }
        }
    }

    /// `parent` is the indentation of the enclosing collection; continuation
    /// lines must be indented deeper than it.
    fn quoted(&mut self, parent: isize) -> Res<String> {
        let origin = self.cur.origin();
        let double = self.cur.bump() == Some('"');
        let mut out = String::new();
        // Whitespace is held back: it is dropped before a line break.
        let mut pending = String::new();
        loop {
            let Some(ch) = self.cur.bump() else {
                let kind = if double { "double" } else { "single" };
                return Err(syntax_at(&origin, format!("unterminated {kind}-quoted string")));
            };
            match ch {
                ' ' | '\t' => pending.push(ch),
                '\n' | '\r' => {
                    pending.clear();
                    if ch == '\r' && self.cur.peek() == Some(b'\n') {
                        self.cur.bump();
                    }
                    self.fold_break(&mut out, parent)?;
                },
                '\'' if !double => {
                    out.push_str(&pending);
                    pending.clear();
                    if self.cur.peek() == Some(b'\'') {
                        self.cur.bump();
                        out.push('\'');
                    } else {
                        return Ok(out);
                    }
                },
                '"' if double => {
                    out.push_str(&pending);
                    return Ok(out);
                },
                '\\' if double => {
                    out.push_str(&pending);
                    pending.clear();
                    self.escape(&mut out, &origin, parent)?;
                },
                _ => {
                    out.push_str(&pending);
                    pending.clear();
                    out.push(ch);
                },
            }
        }
    }

    /// After a line break inside a quoted scalar: skip blank lines and leading
    /// whitespace, then fold the breaks (one break is a space, `n` breaks are
    /// `n - 1` newlines).
    fn fold_break(&mut self, out: &mut String, parent: isize) -> Res<()> {
        let mut breaks = 1;
        loop {
            self.check_quoted_line(parent)?;
            self.skip_spaces();
            if matches!(self.cur.peek(), Some(b'\n' | b'\r')) {
                self.eol();
                breaks += 1;
            } else {
                break;
            }
        }
        if breaks == 1 {
            out.push(' ');
        } else {
            out.extend(repeat_n('\n', breaks - 1));
        }
        Ok(())
    }

    /// At the start of a continuation line of a quoted scalar: a document
    /// marker ends the document and the text must be indented deeper than the
    /// enclosing collection (tabs do not count). Blank lines are exempt.
    fn check_quoted_line(&self, parent: isize) -> Res<()> {
        let (spaces, body) = line_body(self.cur.rest());
        if body.is_empty() || body.starts_with(['\n', '\r']) {
            return Ok(());
        }
        if marker_of(self.cur.rest()).is_some() {
            return Err(self
                .cur
                .error(FORMAT, "document marker inside a quoted scalar"));
        }
        if !exceeds(spaces, parent) {
            return Err(self
                .cur
                .error(FORMAT, "continuation line of a quoted scalar is not indented"));
        }
        Ok(())
    }

    /// Decode the escape after a consumed `\` in a double-quoted scalar.
    fn escape(&mut self, out: &mut String, origin: &Origin, parent: isize) -> Res<()> {
        let Some(ch) = self.cur.bump() else {
            return Err(syntax_at(origin, "unterminated double-quoted string"));
        };
        let decoded = match ch {
            '0' => '\0',
            'a' => '\u{07}',
            'b' => '\u{08}',
            't' | '\t' => '\t',
            'n' => '\n',
            'v' => '\u{0b}',
            'f' => '\u{0c}',
            'r' => '\r',
            'e' => '\u{1b}',
            ' ' => ' ',
            '"' => '"',
            '/' => '/',
            '\\' => '\\',
            'N' => '\u{85}',
            '_' => '\u{a0}',
            'L' => '\u{2028}',
            'P' => '\u{2029}',
            'x' => self.hex_escape(2, 'x')?,
            'u' => self.hex_escape(4, 'u')?,
            'U' => self.hex_escape(8, 'U')?,
            '\n' | '\r' => {
                if ch == '\r' && self.cur.peek() == Some(b'\n') {
                    self.cur.bump();
                }
                self.check_quoted_line(parent)?;
                self.skip_spaces();
                return Ok(());
            },
            other => {
                return Err(self
                    .cur
                    .error(FORMAT, format!("invalid escape sequence '\\{other}'")));
            },
        };
        out.push(decoded);
        Ok(())
    }

    fn hex_escape(&mut self, digits: u32, marker: char) -> Res<char> {
        let mut code = 0u32;
        for _ in 0..digits {
            let digit = self.cur.peek().and_then(|b| char::from(b).to_digit(16));
            let Some(digit) = digit else {
                return Err(self.cur.error(
                    FORMAT,
                    format!("escape '\\{marker}' needs {digits} hexadecimal digits"),
                ));
            };
            self.cur.bump();
            code = code * 16 + digit;
        }
        char::from_u32(code).ok_or_else(|| {
            self.cur
                .error(FORMAT, format!("escape '\\{marker}' is not a valid character"))
        })
    }

    /// `|` or `>` scalar; the cursor is on the indicator. `parent` is the
    /// indentation of the enclosing collection.
    fn block_scalar(&mut self, parent: isize) -> Res<ConfigNode> {
        let origin = self.cur.origin();
        let folded = self.cur.bump() == Some('>');
        let mut chomp = Chomp::Clip;
        let mut explicit = None;
        loop {
            match self.cur.peek() {
                Some(b'-') => chomp = Chomp::Strip,
                Some(b'+') => chomp = Chomp::Keep,
                Some(digit @ b'1'..=b'9') => explicit = Some(usize::from(digit - b'0')),
                _ => break,
            }
            self.cur.bump();
        }
        let header_end = self.cur.pos();
        self.skip_spaces();
        let spaced = self.cur.pos() > header_end;
        if self.cur.peek() == Some(b'#') {
            if !spaced {
                return Err(self.comment_needs_space());
            }
            self.skip_comment();
        }
        if self.cur.peek().is_some() {
            if !matches!(self.cur.peek(), Some(b'\n' | b'\r')) {
                return Err(self
                    .cur
                    .error(FORMAT, "unexpected content after block scalar header"));
            }
            self.eol();
        }

        let body = self.cur.rest();
        let mut indent = explicit.map(|digits| parent_floor(parent) + digits);
        // Widest space-only line before the indentation is known: the first
        // text line cannot be indented less than it.
        let mut widest_blank = 0;
        let mut lines: Vec<&str> = Vec::new();
        let mut rest = body;
        while !rest.is_empty() {
            let (line, after) = split_line(rest);
            let spaces = line.bytes().take_while(|&b| b == b' ').count();
            let blank = spaces == line.len();
            let width = match indent {
                Some(width) => width,
                None if blank => {
                    widest_blank = widest_blank.max(spaces);
                    usize::MAX
                },
                None if exceeds(spaces, parent) => {
                    if spaces < widest_blank {
                        return Err(self.cur.error(
                            FORMAT,
                            "a leading blank line is indented deeper than the first text line",
                        ));
                    }
                    indent = Some(spaces);
                    spaces
                },
                None => break,
            };
            if blank && spaces <= width {
                lines.push("");
            } else if spaces < width || (spaces == 0 && marker_of(line).is_some()) {
                break;
            } else {
                // Past the indentation, spaces are content, even alone on a
                // line.
                lines.push(line.get(width..).unwrap_or(""));
            }
            rest = after;
        }
        self.advance(body.len() - rest.len());
        if rest.trim_start_matches(' ').starts_with('\t') {
            return Err(self
                .cur
                .error(FORMAT, "tabs are not allowed for indentation"));
        }

        let trailing = lines
            .iter()
            .rev()
            .take_while(|line| line.is_empty())
            .count();
        let content = lines.get(..lines.len() - trailing).unwrap_or(&[]);
        let mut text = if folded { fold(content) } else { content.join("\n") };
        // The end of input also ends the last line.
        let final_break = !content.is_empty();
        match chomp {
            Chomp::Strip => {},
            Chomp::Clip => {
                if final_break {
                    text.push('\n');
                }
            },
            Chomp::Keep => {
                if final_break {
                    text.push('\n');
                }
                text.extend(repeat_n('\n', trailing));
            },
        }
        Ok(ConfigNode::scalar(text).with_origin(origin))
    }

    // ---- flow collections -----------------------------------------------

    /// Skip whitespace, line breaks and comments inside a flow collection
    /// that sits in a block collection indented `parent`.
    fn flow_ws(&mut self, parent: isize) -> Res<()> {
        let mut spaced = matches!(self.cur.prev_byte(), None | Some(b' ' | b'\t' | b'\n' | b'\r'));
        loop {
            match self.cur.peek() {
                Some(b' ' | b'\t') => {
                    self.cur.bump();
                    spaced = true;
                },
                Some(b'\n' | b'\r') => {
                    self.eol();
                    spaced = true;
                    self.flow_line(parent)?;
                },
                Some(b'#') if spaced => self.skip_comment(),
                Some(b'#') => return Err(self.comment_needs_space()),
                _ => return Ok(()),
            }
        }
    }

    /// At the start of a line inside a flow collection: a document marker is
    /// not allowed and text must be indented deeper than the enclosing block
    /// collection (tabs do not count).
    fn flow_line(&self, parent: isize) -> Res<()> {
        let (spaces, body) = line_body(self.cur.rest());
        if body.is_empty() || body.starts_with(['\n', '\r', '#']) {
            return Ok(());
        }
        if marker_of(self.cur.rest()).is_some() {
            return Err(self
                .cur
                .error(FORMAT, "document marker inside a flow collection"));
        }
        // A closing bracket may sit at the parent's own indentation:
        // `key: [\n  a,\n]` is common and unambiguous.
        if !exceeds(spaces, parent) && !body.starts_with([']', '}']) {
            return Err(self
                .cur
                .error(FORMAT, "flow collection line is not indented deeper than its parent"));
        }
        Ok(())
    }

    fn flow_seq(&mut self, parent: isize) -> Res<ConfigNode> {
        self.cur.enter(FORMAT)?;
        let origin = self.cur.origin();
        self.cur.bump();
        let mut items = Vec::new();
        let mut need_comma = false;
        loop {
            self.flow_ws(parent)?;
            match self.cur.peek() {
                None => return Err(syntax_at(&origin, "unterminated flow sequence")),
                Some(b']') => {
                    self.cur.bump();
                    break;
                },
                Some(b',') if need_comma => {
                    self.cur.bump();
                    need_comma = false;
                    continue;
                },
                Some(b',') => return Err(self.cur.error(FORMAT, "unexpected ','")),
                Some(_) if need_comma => {
                    return Err(self
                        .cur
                        .error(FORMAT, "expected ',' or ']' in flow sequence"));
                },
                Some(_) => {},
            }
            items.push(self.flow_seq_entry(parent)?);
            need_comma = true;
        }
        self.cur.leave();
        Ok(ConfigNode::array(items).with_origin(origin))
    }

    /// One entry of a flow sequence: a node, or a single `key: value` pair
    /// (a one-entry mapping).
    fn flow_seq_entry(&mut self, parent: isize) -> Res<ConfigNode> {
        let origin = self.cur.origin();
        self.reject_indicator()?;
        let (key, quoted) = match self.cur.peek() {
            Some(b'[' | b'{') => {
                let node = self.value(parent, true)?;
                self.flow_ws(parent)?;
                if self.cur.peek() == Some(b':') {
                    return Err(unsupported_at(
                        &self.cur.origin(),
                        "yaml complex keys (?) are not supported",
                    ));
                }
                return Ok(node);
            },
            Some(b'"' | b'\'') => (self.quoted(parent)?, true),
            Some(b'|' | b'>') => return self.value(parent, true),
            Some(b'-') if self.dash_entry(true) => return self.value(parent, true),
            _ => (self.plain_text(parent, true)?, false),
        };
        self.flow_ws(parent)?;
        if self.cur.peek() != Some(b':') {
            return Ok(if quoted {
                ConfigNode::scalar(key).with_origin(origin)
            } else {
                plain_node(key, origin)
            });
        }
        self.single_line_key(&origin)?;
        if !quoted {
            check_plain_key(&key, &origin)?;
        }
        self.cur.bump();
        self.flow_ws(parent)?;
        let value = match self.cur.peek() {
            None => return Err(syntax_at(&origin, "unterminated flow sequence")),
            Some(b',' | b']') => ConfigNode::null().with_origin(self.cur.origin()),
            Some(_) => self.value(parent, true)?,
        };
        let mut table = Table::new();
        table.insert(Text::from(key), value);
        Ok(ConfigNode::table(table).with_origin(origin))
    }

    fn flow_map(&mut self, parent: isize) -> Res<ConfigNode> {
        self.cur.enter(FORMAT)?;
        let origin = self.cur.origin();
        self.cur.bump();
        let mut table = Table::new();
        let mut need_comma = false;
        loop {
            self.flow_ws(parent)?;
            match self.cur.peek() {
                None => return Err(syntax_at(&origin, "unterminated flow mapping")),
                Some(b'}') => {
                    self.cur.bump();
                    break;
                },
                Some(b',') if need_comma => {
                    self.cur.bump();
                    need_comma = false;
                    continue;
                },
                Some(b',') => return Err(self.cur.error(FORMAT, "unexpected ','")),
                Some(_) if need_comma => {
                    return Err(self
                        .cur
                        .error(FORMAT, "expected ',' or '}' in flow mapping"));
                },
                Some(_) => {},
            }

            let key_origin = self.cur.origin();
            if matches!(self.cur.peek(), Some(b'[' | b'{')) {
                return Err(unsupported_at(&key_origin, "yaml complex keys (?) are not supported"));
            }
            self.reject_indicator()?;
            let quoted = matches!(self.cur.peek(), Some(b'"' | b'\''));
            let key = if quoted { self.quoted(parent)? } else { self.plain_text(parent, true)? };
            if !quoted {
                check_plain_key(&key, &key_origin)?;
            }
            if table.contains_key(key.as_str()) {
                return Err(syntax_at(&key_origin, format!("duplicate key \"{key}\"")));
            }

            self.flow_ws(parent)?;
            let value = if self.cur.peek() == Some(b':') {
                self.cur.bump();
                self.flow_ws(parent)?;
                match self.cur.peek() {
                    None => return Err(syntax_at(&origin, "unterminated flow mapping")),
                    Some(b',' | b'}') => ConfigNode::null().with_origin(self.cur.origin()),
                    Some(_) => self.value(parent, true)?,
                }
            } else {
                ConfigNode::null().with_origin(self.cur.origin())
            };
            table.insert(Text::from(key), value);
            need_comma = true;
        }
        self.cur.leave();
        Ok(ConfigNode::table(table).with_origin(origin))
    }
}

// ---- free helpers ---------------------------------------------------------

/// Number of leading spaces of the line at the start of `s`, and the text
/// after any further whitespace.
fn line_body(s: &str) -> (usize, &str) {
    let spaces = s.bytes().take_while(|&b| b == b' ').count();
    let body = s
        .get(spaces..)
        .unwrap_or("")
        .trim_start_matches([' ', '\t']);
    (spaces, body)
}

/// A plain scalar's text as a node: the null forms become `Null`.
fn plain_node(text: String, origin: Origin) -> ConfigNode {
    let node = if matches!(text.as_str(), "" | "~" | "null" | "Null" | "NULL") {
        ConfigNode::null()
    } else {
        ConfigNode::scalar(text)
    };
    node.with_origin(origin)
}

/// Plain keys the subset cannot represent: empty (null) keys and merge keys.
fn check_plain_key(key: &str, origin: &Origin) -> Res<()> {
    if key.is_empty() {
        return Err(unsupported_at(origin, "yaml empty mapping keys are not supported"));
    }
    if key == "<<" {
        return Err(unsupported_at(origin, "yaml merge keys (<<) are not supported"));
    }
    Ok(())
}

fn marker_of(s: &str) -> Option<Marker> {
    let kind = if s.starts_with("---") {
        Marker::Start
    } else if s.starts_with("...") {
        Marker::End
    } else {
        return None;
    };
    is_space_or_end(s.as_bytes().get(3).copied()).then_some(kind)
}

fn is_space_or_end(byte: Option<u8>) -> bool {
    byte.is_none_or(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
}

fn is_flow_indicator(byte: u8) -> bool {
    matches!(byte, b',' | b'[' | b']' | b'{' | b'}')
}

/// `indent` is deeper than the enclosing collection at `parent` (`-1` for the
/// document level).
fn exceeds(indent: usize, parent: isize) -> bool {
    parent < 0 || indent > parent.unsigned_abs()
}

fn parent_floor(parent: isize) -> usize {
    if parent < 0 { 0 } else { parent.unsigned_abs() }
}

fn to_isize(indent: usize) -> isize {
    isize::try_from(indent).unwrap_or(isize::MAX)
}

fn strip_break(s: &str) -> &str {
    s.strip_prefix("\r\n")
        .or_else(|| s.strip_prefix(['\n', '\r']))
        .unwrap_or(s)
}

/// Split at the first line break: the line, and the text after the break (the
/// whole input when there is no break).
fn split_line(s: &str) -> (&str, &str) {
    let end = s.find(['\n', '\r']).unwrap_or(s.len());
    let line = s.get(..end).unwrap_or(s);
    let after = s.get(end..).unwrap_or("");
    (line, strip_break(after))
}

/// Scan one plain-scalar line. Returns the byte length of the text with
/// trailing whitespace trimmed, the byte index where scanning stopped, and why.
fn scan_plain(s: &str, flow: bool) -> (usize, usize, Stop) {
    let bytes = s.as_bytes();
    let mut end = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        match byte {
            b'\n' | b'\r' => return (end, i, Stop::Eol),
            b' ' | b'\t' => {},
            b'#' if i > 0 && matches!(bytes.get(i - 1), Some(b' ' | b'\t')) => {
                return (end, i, Stop::Comment);
            },
            b':' if is_space_or_end(bytes.get(i + 1).copied())
                || (flow && bytes.get(i + 1).copied().is_some_and(is_flow_indicator)) =>
            {
                return (end, i, Stop::Colon);
            },
            _ if flow && is_flow_indicator(byte) => return (end, i, Stop::Flow),
            _ => end = i + 1,
        }
    }
    (end, bytes.len(), Stop::Eol)
}

fn is_indented(line: &str) -> bool {
    line.starts_with([' ', '\t'])
}

/// Fold the lines of a `>` scalar: a lone break between two ordinary lines is a
/// space, blank lines are kept as newlines, and more-indented lines keep their
/// breaks.
fn fold(lines: &[&str]) -> String {
    let mut out = String::new();
    let mut prev: Option<&str> = None;
    let mut blanks = 0;
    for &line in lines {
        if line.is_empty() {
            blanks += 1;
            continue;
        }
        match prev {
            None => out.extend(repeat_n('\n', blanks)),
            Some(before) if is_indented(before) || is_indented(line) => {
                out.extend(repeat_n('\n', blanks + 1));
            },
            Some(_) if blanks == 0 => out.push(' '),
            Some(_) => out.extend(repeat_n('\n', blanks)),
        }
        out.push_str(line);
        prev = Some(line);
        blanks = 0;
    }
    out
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;
    use crate::merge::merge;
    use crate::{ConfigNode, Value};

    fn strip(node: &ConfigNode) -> ConfigNode {
        match node.value() {
            Value::Null => ConfigNode::null(),
            Value::Scalar(text) => ConfigNode::scalar(text.clone()),
            Value::Array(items) => ConfigNode::array(items.iter().map(strip).collect()),
            Value::Table(table) => ConfigNode::table(
                table
                    .iter()
                    .map(|(key, value)| (key.clone(), strip(value)))
                    .collect(),
            ),
        }
    }

    fn s(text: &str) -> ConfigNode {
        ConfigNode::scalar(text)
    }

    fn arr(items: Vec<ConfigNode>) -> ConfigNode {
        ConfigNode::array(items)
    }

    fn map(entries: Vec<(&str, ConfigNode)>) -> ConfigNode {
        ConfigNode::table(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }

    fn parsed(input: &str) -> ConfigNode {
        match parse(input, "<string>") {
            Ok(node) => strip(&node),
            Err(error) => panic!("parse failed: {error}\n{input}"),
        }
    }

    fn failure(input: &str) -> String {
        match parse(input, "<string>") {
            Ok(node) => panic!("expected an error, got {node:?}"),
            Err(error) => error.to_string(),
        }
    }

    #[test]
    fn nested_mappings_and_sequences_of_mappings() {
        let input = "\
# service config
service:
  name: forgeconf   # trailing comment
  enabled: true
  ports:
    - 80
    - 443
servers:
  - host: a.example.com
    port: 1
    tags: [x, y]
  - host: b.example.com
    port: 2
  -
    host: c.example.com
empty:
tail: end
";
        let expected = map(vec![
            (
                "service",
                map(vec![
                    ("name", s("forgeconf")),
                    ("enabled", s("true")),
                    ("ports", arr(vec![s("80"), s("443")])),
                ]),
            ),
            (
                "servers",
                arr(vec![
                    map(vec![
                        ("host", s("a.example.com")),
                        ("port", s("1")),
                        ("tags", arr(vec![s("x"), s("y")])),
                    ]),
                    map(vec![("host", s("b.example.com")), ("port", s("2"))]),
                    map(vec![("host", s("c.example.com"))]),
                ]),
            ),
            ("empty", ConfigNode::null()),
            ("tail", s("end")),
        ]);
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn sequence_at_mapping_indent_and_nested_sequences() {
        let input = "list:\n- a\n- - b\n  - c\n- d: 1\nafter: x\n";
        let expected = map(vec![
            ("list", arr(vec![s("a"), arr(vec![s("b"), s("c")]), map(vec![("d", s("1"))])])),
            ("after", s("x")),
        ]);
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn flow_collections() {
        let input = "\
a: [1, 2, [3, 4], {k: v}]
b: {x: 1, y: [a, b], z: , w}
c: [
  one,   # comment
  'two',
  \"three\",
]
d: {
  p: q,
  r: multi
    line
}
e: []
f: {}
";
        let expected = map(vec![
            ("a", arr(vec![s("1"), s("2"), arr(vec![s("3"), s("4")]), map(vec![("k", s("v"))])])),
            (
                "b",
                map(vec![
                    ("x", s("1")),
                    ("y", arr(vec![s("a"), s("b")])),
                    ("z", ConfigNode::null()),
                    ("w", ConfigNode::null()),
                ]),
            ),
            ("c", arr(vec![s("one"), s("two"), s("three")])),
            ("d", map(vec![("p", s("q")), ("r", s("multi line"))])),
            ("e", arr(vec![])),
            ("f", map(vec![])),
        ]);
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn flow_collection_as_document() {
        assert_eq!(parsed("{a: 1, b: [x]}"), map(vec![("a", s("1")), ("b", arr(vec![s("x")]))]));
        assert_eq!(parsed("[a, b]"), arr(vec![s("a"), s("b")]));
    }

    #[test]
    fn literal_block_scalar_keeps_newlines() {
        let input = "text: |\n  line one\n    indented\n\n  line two\nnext: 1\n";
        let expected =
            map(vec![("text", s("line one\n  indented\n\nline two\n")), ("next", s("1"))]);
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn folded_block_scalar_folds_and_strips() {
        let input = "text: >-\n  a\n  b\n\n  c\n\nnext: 1\n";
        assert_eq!(parsed(input), map(vec![("text", s("a b\nc")), ("next", s("1"))]));
    }

    #[test]
    fn block_scalar_chomping_and_indent_indicator() {
        let input = "\
clip: |
  x

strip: |-
  x

keep: |+
  x

indent: |2
    y
seq:
  - >
    p
    q
";
        let expected = map(vec![
            ("clip", s("x\n")),
            ("strip", s("x")),
            ("keep", s("x\n\n")),
            ("indent", s("  y\n")),
            ("seq", arr(vec![s("p q\n")])),
        ]);
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn anchor_is_unsupported_with_position() {
        assert_eq!(
            failure("a: &x 1"),
            "unsupported: yaml anchors (&) are not supported (<string>:1:4)"
        );
    }

    #[test]
    fn alias_tag_complex_key_and_merge_key_are_unsupported() {
        assert_eq!(
            failure("a: 1\nb: *x\n"),
            "unsupported: yaml aliases (*) are not supported (<string>:2:4)"
        );
        assert_eq!(
            failure("a: !!str 1\n"),
            "unsupported: yaml tags (!) are not supported (<string>:1:4)"
        );
        assert_eq!(
            failure("? a\n: b\n"),
            "unsupported: yaml complex keys (?) are not supported (<string>:1:1)"
        );
        assert_eq!(
            failure("base: 1\nthis:\n  <<: 2\n"),
            "unsupported: yaml merge keys (<<) are not supported (<string>:3:3)"
        );
    }

    #[test]
    fn tab_indentation_is_a_syntax_error() {
        assert_eq!(
            failure("a:\n\tb: 1\n"),
            "invalid yaml: tabs are not allowed for indentation (<string>:2:1)"
        );
    }

    #[test]
    fn empty_documents_are_empty_tables() {
        assert_eq!(parsed(""), map(vec![]));
        assert_eq!(parsed("\n  \n"), map(vec![]));
        assert_eq!(parsed("# only a comment\n# another\n"), map(vec![]));
        assert_eq!(parsed("---\n"), map(vec![]));
    }

    #[test]
    fn null_value_does_not_override_lower_source() {
        let mut base = parse("port: 80\nhost: a\n", "low.yaml").unwrap();
        let overlay = parse("port: ~\nhost: null\n", "high.yaml").unwrap();
        merge(&mut base, overlay);
        assert_eq!(strip(&base), map(vec![("host", s("a")), ("port", s("80"))]));
    }

    #[test]
    fn null_forms() {
        let input = "a: ~\nb: null\nc: Null\nd: NULL\ne:\nf: 'null'\ng: nulls\n";
        let expected = map(vec![
            ("a", ConfigNode::null()),
            ("b", ConfigNode::null()),
            ("c", ConfigNode::null()),
            ("d", ConfigNode::null()),
            ("e", ConfigNode::null()),
            ("f", s("null")),
            ("g", s("nulls")),
        ]);
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn second_document_fails() {
        assert_eq!(
            failure("a: 1\n---\nb: 2\n"),
            "unsupported: yaml documents after the first are not supported (<string>:2:1)"
        );
        assert_eq!(
            failure("---\na: 1\n...\n---\nb: 2\n"),
            "unsupported: yaml documents after the first are not supported (<string>:4:1)"
        );
    }

    #[test]
    fn document_markers() {
        assert_eq!(parsed("---\na: 1\n...\n"), map(vec![("a", s("1"))]));
        assert_eq!(parsed("# c\n--- # c\na: 1\n"), map(vec![("a", s("1"))]));
        assert_eq!(parsed("--- text\n"), s("text"));
        assert_eq!(parsed("a: 1\n...\n# done\n"), map(vec![("a", s("1"))]));
    }

    #[test]
    fn duplicate_key_fails() {
        assert_eq!(
            failure("a: 1\nb: 2\na: 3\n"),
            "invalid yaml: duplicate key \"a\" (<string>:3:1)"
        );
        assert_eq!(failure("x: {a: 1, a: 2}"), "invalid yaml: duplicate key \"a\" (<string>:1:11)");
    }

    #[test]
    fn quoted_scalars_and_escapes() {
        let input = r#"
single: 'it''s: fine # not a comment'
double: "tab\there \"quoted\" \\ \/ \x41\u00e9\U0001F600 \0\a\b\v\f\r\e end"
folded: "one
  two

  three"
single_folded: 'a
   b'
joined: "abc\
   def"
"key: q": x
"#;
        let expected = map(vec![
            ("single", s("it's: fine # not a comment")),
            (
                "double",
                s("tab\there \"quoted\" \\ / A\u{e9}\u{1F600} \0\u{7}\u{8}\u{b}\u{c}\r\u{1b} end"),
            ),
            ("folded", s("one two\nthree")),
            ("single_folded", s("a b")),
            ("joined", s("abcdef")),
            ("key: q", s("x")),
        ]);
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn invalid_escape_is_a_syntax_error() {
        assert_eq!(
            failure("a: \"\\q\"\n"),
            "invalid yaml: invalid escape sequence '\\q' (<string>:1:7)"
        );
        assert_eq!(
            failure("a: \"abc\n"),
            "invalid yaml: unterminated double-quoted string (<string>:1:4)"
        );
    }

    #[test]
    fn plain_scalars_stay_text() {
        let input = "\
yes: yes
url: http://example.com:8080/path?a=1&b=2
time: 12:30:00
path: C:\\temp
hash: a#b
comment: a #b
multi: first
  second

  third
dash: -1
star: a*b
";
        let expected = map(vec![
            ("yes", s("yes")),
            ("url", s("http://example.com:8080/path?a=1&b=2")),
            ("time", s("12:30:00")),
            ("path", s("C:\\temp")),
            ("hash", s("a#b")),
            ("comment", s("a")),
            ("multi", s("first second\nthird")),
            ("dash", s("-1")),
            ("star", s("a*b")),
        ]);
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn nested_value_misuse_is_an_error() {
        assert_eq!(
            failure("a: b: c\n"),
            "invalid yaml: mapping values are not allowed in this context (<string>:1:5)"
        );
        assert_eq!(
            failure("a:\n  b: 1\n c: 2\n"),
            "invalid yaml: bad indentation of a mapping entry (<string>:3:2)"
        );
        assert_eq!(
            failure("a: - b\n"),
            "invalid yaml: block sequence entries are not allowed in this context (<string>:1:4)"
        );
        assert_eq!(
            failure("a: [1, 2\n"),
            "invalid yaml: unterminated flow sequence (<string>:1:4)"
        );
        assert_eq!(failure("a: 'x' y\n"), "invalid yaml: unexpected 'y' (<string>:1:8)");
    }

    #[test]
    fn crlf_line_endings() {
        let input = "a:\r\n  - x\r\n  - y\r\nb: |\r\n  t\r\n";
        assert_eq!(parsed(input), map(vec![("a", arr(vec![s("x"), s("y")])), ("b", s("t\n"))]));
    }

    #[test]
    fn origins_point_at_first_byte() {
        let node = parse("a:\n  b: 1\n  c: [x]\n", "f.yaml").unwrap();
        let table = node.as_table().unwrap();
        let at = |n: &ConfigNode| n.origin().unwrap().to_string();
        assert_eq!(at(&node), "f.yaml:1:1");
        let inner = table.get("a").unwrap();
        assert_eq!(at(inner), "f.yaml:2:3");
        assert_eq!(at(inner.get("b").unwrap()), "f.yaml:2:6");
        assert_eq!(at(inner.get("c").unwrap()), "f.yaml:3:6");
    }

    #[test]
    fn nesting_is_limited() {
        let deep = format!("{}1{}", "[".repeat(200), "]".repeat(200));
        let message = failure(&deep);
        assert!(message.starts_with("invalid yaml: nesting deeper than"), "{message}");
    }

    #[test]
    fn fixture_file_parses() {
        let input = include_str!("../../tests/fixtures/yaml-config.yaml");
        let expected = map(vec![
            ("service", map(vec![("name", s("forgeconf")), ("enabled", s("true"))])),
            ("replicas", arr(vec![s("db-a"), s("db-b")])),
            ("port", s("8123")),
        ]);
        assert_eq!(parsed(input), expected);
    }

    #[test]
    fn directives_and_empty_keys_are_unsupported() {
        assert_eq!(
            failure("%YAML 1.2\n---\na: 1\n"),
            "unsupported: yaml directives (%) are not supported (<string>:1:1)"
        );
        assert_eq!(
            failure(": a\n"),
            "unsupported: yaml empty mapping keys are not supported (<string>:1:1)"
        );
        assert_eq!(
            failure("[: x]\n"),
            "unsupported: yaml empty mapping keys are not supported (<string>:1:2)"
        );
        assert_eq!(
            failure("{ : x }\n"),
            "unsupported: yaml empty mapping keys are not supported (<string>:1:3)"
        );
    }

    #[test]
    fn flow_sequence_single_pair_entries() {
        assert_eq!(
            parsed("[a: b, c, \"d\": e, 'f':g, h:]\n"),
            arr(vec![
                map(vec![("a", s("b"))]),
                s("c"),
                map(vec![("d", s("e"))]),
                map(vec![("f", s("g"))]),
                map(vec![("h", ConfigNode::null())]),
            ])
        );
        assert_eq!(
            parsed("k: [\n  implicit: value,\n ]\n"),
            map(vec![("k", arr(vec![map(vec![("implicit", s("value"))])]))])
        );
        assert_eq!(
            failure("[[a]: b]\n"),
            "unsupported: yaml complex keys (?) are not supported (<string>:1:5)"
        );
    }

    #[test]
    fn flow_key_continues_on_the_line_of_its_colon() {
        assert_eq!(parsed("{foo\n: bar}\n"), map(vec![("foo", s("bar"))]));
        assert_eq!(parsed("k: {\n k\n :\n v\n }\n"), map(vec![("k", map(vec![("k", s("v"))]))]));
    }

    #[test]
    fn flow_lines_stay_indented_and_inside_the_document() {
        assert_eq!(
            failure("a: [x,\ny]\n"),
            "invalid yaml: flow collection line is not indented deeper than its parent \
             (<string>:2:1)"
        );
        assert_eq!(
            failure("- [\n\tfoo,\n foo\n ]\n"),
            "invalid yaml: flow collection line is not indented deeper than its parent \
             (<string>:2:1)"
        );
        assert_eq!(
            failure("[\n--- ,\n...\n]\n"),
            "invalid yaml: document marker inside a flow collection (<string>:2:1)"
        );
        assert_eq!(parsed("a: [\n  x,\n]\n"), map(vec![("a", arr(vec![s("x")]))]));
    }

    #[test]
    fn dash_is_not_a_plain_scalar_in_flow_context() {
        assert_eq!(
            failure("[-]\n"),
            "invalid yaml: block sequence entries are not allowed in this context (<string>:1:2)"
        );
        assert_eq!(
            failure("- [-, -]\n"),
            "invalid yaml: block sequence entries are not allowed in this context (<string>:1:4)"
        );
        assert_eq!(parsed("[-a, -1]\n"), arr(vec![s("-a"), s("-1")]));
    }

    #[test]
    fn tabs_separate_but_never_indent() {
        assert_eq!(parsed("\t[\n\t]\n"), arr(vec![]));
        assert_eq!(parsed("\t{}\n"), map(vec![]));
        assert_eq!(parsed("foo:\n \tbar\n"), map(vec![("foo", s("bar"))]));
        assert_eq!(parsed("a:\tb\n"), map(vec![("a", s("b"))]));
        assert_eq!(
            failure("-\t-\n"),
            "invalid yaml: tabs are not allowed for indentation (<string>:1:3)"
        );
        assert_eq!(
            failure("- \t-\n"),
            "invalid yaml: tabs are not allowed for indentation (<string>:1:4)"
        );
        assert_eq!(
            failure("a: 1\n\tb: 2\n"),
            "invalid yaml: tabs are not allowed for indentation (<string>:2:1)"
        );
    }

    #[test]
    fn comments_need_leading_whitespace() {
        assert_eq!(parsed("a:   # c\n  b\n"), map(vec![("a", s("b"))]));
        assert_eq!(parsed("- # c\n- x\n"), arr(vec![ConfigNode::null(), s("x")]));
        for input in ["k: \"v\"# c\n", "[a, b]#c\n", "[a,#c\n b]\n", "k: >#c\n  x\n"] {
            let message = failure(input);
            assert!(
                message.starts_with("invalid yaml: a comment needs whitespace before '#'"),
                "{input:?}: {message}"
            );
        }
    }

    #[test]
    fn block_collections_cannot_start_on_the_document_marker_line() {
        assert_eq!(
            failure("--- a: b\n"),
            "invalid yaml: mapping values are not allowed in this context (<string>:1:6)"
        );
        assert_eq!(
            failure("--- - a\n"),
            "invalid yaml: block sequence entries are not allowed in this context (<string>:1:5)"
        );
        assert_eq!(parsed("--- text\n"), s("text"));
        assert_eq!(parsed("--- [a]\n"), arr(vec![s("a")]));
        assert_eq!(parsed("--- |\n x\n"), s("x\n"));
    }

    #[test]
    fn quoted_scalars_obey_indentation_markers_and_single_line_keys() {
        assert_eq!(parsed("a: \"x\n  y\"\n"), map(vec![("a", s("x y"))]));
        assert_eq!(
            failure("a: \"x\ny\"\n"),
            "invalid yaml: continuation line of a quoted scalar is not indented (<string>:2:1)"
        );
        assert_eq!(
            failure("a: \"x\n\ty\"\n"),
            "invalid yaml: continuation line of a quoted scalar is not indented (<string>:2:1)"
        );
        assert_eq!(
            failure("---\n\"\n---\n\"\n"),
            "invalid yaml: document marker inside a quoted scalar (<string>:3:1)"
        );
        assert_eq!(
            failure("'\n...\n'\n"),
            "invalid yaml: document marker inside a quoted scalar (<string>:2:1)"
        );
        assert_eq!(
            failure("\"a\n b\": 1\n"),
            "invalid yaml: implicit mapping keys must stay on one line (<string>:1:1)"
        );
        assert_eq!(
            failure("- - \"bar\nbar\": x\n"),
            "invalid yaml: continuation line of a quoted scalar is not indented (<string>:2:1)"
        );
    }

    #[test]
    fn block_scalar_blank_and_whitespace_lines() {
        assert_eq!(
            parsed("foo: |\n  x\n   \nbar: 1\n"),
            map(vec![("foo", s("x\n \n")), ("bar", s("1"))])
        );
        assert_eq!(parsed("foo: |\n  x\n   "), map(vec![("foo", s("x\n \n"))]));
        assert_eq!(parsed("foo: |\n \t\nbar: 1\n"), map(vec![("foo", s("\t\n")), ("bar", s("1"))]));
        assert_eq!(parsed("- >\n \t\n detected\n"), arr(vec![s("\t\ndetected\n")]));
        assert_eq!(parsed("--- |+\n ab\n \n  \n...\n"), s("ab\n\n \n"));
        assert_eq!(parsed("|\n \n  \n  literal\n   \n  \n  text\n"), s("\n\nliteral\n \n\ntext\n"));
    }

    #[test]
    fn block_scalar_rejects_bad_leading_blanks_and_tab_lines() {
        for input in [
            "a: >\n \n  \n   \n invalid\n",
            "a: |\n     \n  more\n  text\n",
            "a: >\n  \n   \n # comment\n",
        ] {
            let message = failure(input);
            assert!(
                message.starts_with(
                    "invalid yaml: a leading blank line is indented deeper than the first text \
                     line"
                ),
                "{input:?}: {message}"
            );
        }
        assert_eq!(
            failure("foo: |\n\t\nbar: 1\n"),
            "invalid yaml: tabs are not allowed for indentation (<string>:2:1)"
        );
    }
}
