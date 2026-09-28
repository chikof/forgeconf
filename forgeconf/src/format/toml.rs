//! In-house TOML 1.0 parser.
//!
//! Values are built into a private [`TableSlot`] tree that carries the TOML
//! table-definition state (implicit, header, dotted), which the public
//! [`ConfigNode`] deliberately lacks. Scalars, inline tables and static
//! arrays are already finished nodes ("leaves"), so nothing can extend them
//! and they go into the output table as they are; only tables and arrays of
//! tables defined by headers or dotted keys stay builder slots until the end.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use super::cursor::{Cursor, MAX_DEPTH};
use crate::{ConfigError, ConfigNode, ErrorKind, Origin, Table};

const FORMAT: &str = "toml";

/// Parse TOML content into a [`ConfigNode`] tree.
pub(crate) fn parse(input: &str, source: &str) -> Result<ConfigNode, ConfigError> {
    let mut c = Cursor::new(input, source);
    c.eat("\u{feff}");
    let mut root = TableSlot::new(c.origin(), State::Header);
    // The `[header]` that ended the previous section, not yet applied.
    let mut header: Option<(Key<'_>, bool)> = None;
    // Levels `enter`ed for the current `[header]`; keeps nesting under
    // MAX_DEPTH.
    let mut header_depth = 0;

    loop {
        let table = match header.take() {
            None => &mut root,
            Some((key, is_array)) => {
                for _ in 0..header_depth {
                    c.leave();
                }
                header_depth = 0;
                for _ in 0..key.len() {
                    c.enter(FORMAT)?;
                    header_depth += 1;
                }
                let table = apply_header(&mut root, &key, is_array)?;
                end_of_expression(&mut c)?;
                table
            },
        };
        header = parse_section(&mut c, table)?;
        if header.is_none() {
            break;
        }
    }
    Ok(root.into_node())
}

/// Parse key/value lines into `table` up to the next `[header]`, which is
/// returned unapplied, or to the end of input.
fn parse_section<'a>(
    c: &mut Cursor<'a>,
    table: &mut TableSlot,
) -> Result<Option<(Key<'a>, bool)>, ConfigError> {
    loop {
        skip_ws(c);
        match c.peek() {
            None => return Ok(None),
            Some(b'#') => skip_comment(c)?,
            Some(b'[') => return parse_header(c).map(Some),
            Some(_) if eat_newline(c) => {},
            Some(_) => {
                let key = parse_key(c)?;
                skip_ws(c);
                expect_byte(c, b'=', "`=` after the key")?;
                skip_ws(c);
                for _ in &key.parents {
                    c.enter(FORMAT)?;
                }
                let value = parse_value(c)?;
                for _ in &key.parents {
                    c.leave();
                }
                insert_key_value(table, &key, value)?;
                end_of_expression(c)?;
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Table state
// ---------------------------------------------------------------------------

/// How a table came to exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    /// Created only as the parent path of a `[header]`; a later header may
    /// define it.
    Implicit,
    /// Defined by its own `[header]` or `[[header]]`.
    Header,
    /// Created by a dotted key; closed to headers, open to dotted keys.
    Dotted,
}

#[derive(Debug)]
struct TableSlot {
    /// Finished values, keyed by name.
    leaves: Leaves,
    /// Tables and arrays of tables still open to headers or dotted keys. A
    /// name is in `leaves` or in `children`, never in both.
    children: BTreeMap<String, Child>,
    state: State,
    origin: Origin,
}

#[derive(Debug)]
struct ArraySlot {
    items: Vec<TableSlot>,
    origin: Origin,
}

#[derive(Debug)]
enum Child {
    Table(TableSlot),
    /// `[[array.of.tables]]`; headers address its last element.
    Array(ArraySlot),
}

/// Small tables stay a map; one with this many entries becomes a list.
const MAP_MAX: usize = 12;

/// Up to this many unsorted entries are searched linearly.
const LINEAR_MAX: usize = 16;

/// Name-to-node entries of one table. Few entries go straight into a map.
/// Documents mostly list keys in sorted order, and the generators of large
/// ones always do, so past [`MAP_MAX`] entries are kept in a plain list while
/// they stay strictly ascending (one comparison per insert, no tree
/// rebalancing) and become a map again only once they do not.
#[derive(Debug)]
enum Leaves {
    List {
        items: Vec<(String, ConfigNode)>,
        sorted: bool,
    },
    Map(Table),
}

impl Leaves {
    fn get(&self, name: &str) -> Option<&ConfigNode> {
        match self {
            Self::Map(map) => map.get(name),
            Self::List { items, sorted: true } => {
                let i = items.binary_search_by(|(k, _)| k.as_str().cmp(name)).ok()?;
                items.get(i).map(|(_, node)| node)
            },
            Self::List { items, sorted: false } => items
                .iter()
                .find_map(|(k, node)| (k == name).then_some(node)),
        }
    }

    /// Add `name`; false, with nothing added, when it is already present.
    fn insert(&mut self, name: &str, node: ConfigNode) -> bool {
        if let Self::Map(map) = self {
            if map.len() != MAP_MAX {
                return Self::insert_map(map, name, node);
            }
            let items = std::mem::take(map).into_iter().collect();
            *self = Self::List { items, sorted: true };
        }
        if let Self::List { items, sorted } = self {
            if *sorted {
                match items.last().map(|(last, _)| last.as_str().cmp(name)) {
                    None | Some(Ordering::Less) => {
                        items.push((String::from(name), node));
                        return true;
                    },
                    Some(Ordering::Equal) => return false,
                    Some(Ordering::Greater) => *sorted = false,
                }
            }
            if items.len() < LINEAR_MAX {
                if items.iter().any(|(k, _)| k == name) {
                    return false;
                }
                items.push((String::from(name), node));
                return true;
            }
            *self = Self::Map(std::mem::take(items).into_iter().collect());
        }
        match self {
            Self::Map(map) => Self::insert_map(map, name, node),
            Self::List { .. } => false,
        }
    }

    fn insert_map(map: &mut Table, name: &str, node: ConfigNode) -> bool {
        match map.entry(String::from(name)) {
            Entry::Occupied(_) => false,
            Entry::Vacant(slot) => {
                slot.insert(node);
                true
            },
        }
    }

    fn into_table(self) -> Table {
        match self {
            Self::Map(map) => map,
            Self::List { items, .. } => items.into_iter().collect(),
        }
    }
}

impl TableSlot {
    fn new(origin: Origin, state: State) -> Self {
        Self {
            leaves: Leaves::Map(Table::new()),
            children: BTreeMap::new(),
            state,
            origin,
        }
    }

    /// Add a finished value; false when the name is taken.
    fn insert_leaf(&mut self, name: &str, node: ConfigNode) -> bool {
        if !self.children.is_empty() && self.children.contains_key(name) {
            return false;
        }
        self.leaves.insert(name, node)
    }

    fn into_node(self) -> ConfigNode {
        let mut table = self.leaves.into_table();
        if table.is_empty() {
            table = self
                .children
                .into_iter()
                .map(|(name, child)| (name, child.into_node()))
                .collect();
        } else {
            for (name, child) in self.children {
                table.insert(name, child.into_node());
            }
        }
        ConfigNode::table(table).with_origin(self.origin)
    }
}

impl Child {
    fn into_node(self) -> ConfigNode {
        match self {
            Self::Table(table) => table.into_node(),
            Self::Array(array) => {
                let items = array.items.into_iter().map(TableSlot::into_node).collect();
                ConfigNode::array(items).with_origin(array.origin)
            },
        }
    }

    /// The table a header addresses: the table itself, or the last element of
    /// an array of tables.
    fn last_table(&mut self) -> Option<&mut TableSlot> {
        match self {
            Self::Table(table) => Some(table),
            Self::Array(array) => array.items.last_mut(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Walking the parent path of a `[header]`.
    Header,
    /// Walking the parent path of a dotted key.
    Dotted,
}

/// Step into the child table `key.part(i)`, creating it when missing.
fn descend<'t>(
    cur: &'t mut TableSlot,
    key: &Key<'_>,
    i: usize,
    mode: Mode,
) -> Result<&'t mut TableSlot, ConfigError> {
    let part = key.part(i);
    if let Some(node) = cur.leaves.get(&part.name) {
        return Err(leaf_error(&part.origin, &key.prefix(i + 1), node));
    }
    if !cur.children.contains_key(&*part.name) {
        let created = match mode {
            Mode::Header => State::Implicit,
            Mode::Dotted => State::Dotted,
        };
        let table = TableSlot::new(part.origin.clone(), created);
        cur.children
            .insert(String::from(&*part.name), Child::Table(table));
    }
    match cur.children.get_mut(&*part.name) {
        Some(Child::Table(table)) => {
            if mode == Mode::Dotted {
                match table.state {
                    State::Header => {
                        return Err(error_at(
                            &part.origin,
                            format!("table \"{}\" is already defined", key.prefix(i + 1)),
                        ));
                    },
                    State::Implicit => table.state = State::Dotted,
                    State::Dotted => {},
                }
            }
            Ok(table)
        },
        Some(Child::Array(array)) => {
            if mode == Mode::Dotted {
                return Err(error_at(
                    &part.origin,
                    format!(
                        "cannot extend array of tables \"{}\" with a dotted key",
                        key.prefix(i + 1)
                    ),
                ));
            }
            array.items.last_mut().ok_or_else(|| {
                error_at(
                    &part.origin,
                    format!("array of tables \"{}\" is empty", key.prefix(i + 1)),
                )
            })
        },
        None => Err(error_at(&part.origin, "internal error: lost a table")),
    }
}

fn leaf_error(origin: &Origin, path: &str, node: &ConfigNode) -> ConfigError {
    let message = match node.kind() {
        "table" => format!("inline table \"{path}\" cannot be extended"),
        "array" => format!("static array \"{path}\" cannot be extended"),
        _ => format!("key \"{path}\" is already defined"),
    };
    error_at(origin, message)
}

/// Apply a `[header]`; returns the table the following key/value lines fill.
fn apply_header<'t>(
    root: &'t mut TableSlot,
    key: &Key<'_>,
    is_array: bool,
) -> Result<&'t mut TableSlot, ConfigError> {
    let mut cur = root;
    for i in 0..key.parents.len() {
        cur = descend(cur, key, i, Mode::Header)?;
    }
    let last = &key.last;
    let path = || key.prefix(key.len());
    if let Some(node) = cur.leaves.get(&last.name) {
        return Err(leaf_error(&last.origin, &path(), node));
    }
    let lost = || error_at(&last.origin, "internal error: empty array of tables");
    match cur.children.entry(String::from(&*last.name)) {
        Entry::Vacant(slot) => {
            let table = TableSlot::new(last.origin.clone(), State::Header);
            slot.insert(if is_array {
                Child::Array(ArraySlot {
                    items: vec![table],
                    origin: last.origin.clone(),
                })
            } else {
                Child::Table(table)
            })
            .last_table()
            .ok_or_else(lost)
        },
        Entry::Occupied(slot) => match (slot.into_mut(), is_array) {
            (Child::Table(table), false) if table.state == State::Implicit => {
                table.state = State::Header;
                table.origin = last.origin.clone();
                Ok(table)
            },
            (Child::Array(array), true) => {
                array
                    .items
                    .push(TableSlot::new(last.origin.clone(), State::Header));
                array.items.last_mut().ok_or_else(lost)
            },
            (Child::Table(_), false) => {
                Err(error_at(&last.origin, format!("table \"{}\" is already defined", path())))
            },
            (Child::Table(_), true) => Err(error_at(
                &last.origin,
                format!("table \"{}\" cannot be redefined as an array of tables", path()),
            )),
            (Child::Array(_), false) => Err(error_at(
                &last.origin,
                format!("array of tables \"{}\" cannot be redefined as a table", path()),
            )),
        },
    }
}

fn insert_key_value(
    table: &mut TableSlot,
    key: &Key<'_>,
    value: ConfigNode,
) -> Result<(), ConfigError> {
    let mut cur = table;
    for i in 0..key.parents.len() {
        cur = descend(cur, key, i, Mode::Dotted)?;
    }
    if cur.insert_leaf(&key.last.name, value) {
        Ok(())
    } else {
        Err(error_at(&key.last.origin, format!("duplicate key \"{}\"", key.prefix(key.len()))))
    }
}

fn error_at(origin: &Origin, message: impl Into<String>) -> ConfigError {
    ConfigError::new(ErrorKind::Syntax { format: FORMAT, message: message.into() })
        .with_origin(Some(origin))
}

// ---------------------------------------------------------------------------
// Keys and headers
// ---------------------------------------------------------------------------

struct KeyPart<'a> {
    name: Cow<'a, str>,
    origin: Origin,
}

/// A possibly dotted key; `parents` are the parts before the last one.
struct Key<'a> {
    parents: Vec<KeyPart<'a>>,
    last: KeyPart<'a>,
}

impl Key<'_> {
    fn len(&self) -> usize {
        self.parents.len() + 1
    }

    fn part(&self, i: usize) -> &KeyPart<'_> {
        self.parents.get(i).unwrap_or(&self.last)
    }

    /// The first `n` parts joined with dots, for messages.
    fn prefix(&self, n: usize) -> String {
        let names: Vec<&str> = self
            .parents
            .iter()
            .chain(Some(&self.last))
            .take(n)
            .map(|p| &*p.name)
            .collect();
        names.join(".")
    }
}

fn parse_key<'a>(c: &mut Cursor<'a>) -> Result<Key<'a>, ConfigError> {
    let mut parents = Vec::new();
    let mut current = parse_key_part(c)?;
    loop {
        skip_ws(c);
        if !eat_byte(c, b'.') {
            break;
        }
        parents.push(std::mem::replace(&mut current, parse_key_part(c)?));
    }
    if parents.len() >= MAX_DEPTH {
        return Err(error_at(
            &parents[0].origin,
            format!("key nested deeper than {MAX_DEPTH} levels"),
        ));
    }
    Ok(Key { parents, last: current })
}

fn parse_key_part<'a>(c: &mut Cursor<'a>) -> Result<KeyPart<'a>, ConfigError> {
    skip_ws(c);
    let origin = c.origin();
    let name = match c.peek() {
        Some(b'"') => basic_string(c)?,
        Some(b'\'') => literal_string(c)?,
        Some(b) if is_bare_key(b) => {
            let rest = c.rest();
            let n = rest
                .bytes()
                .position(|b| !is_bare_key(b))
                .unwrap_or(rest.len());
            c.advance_flat(n);
            Cow::Borrowed(rest.get(..n).unwrap_or(""))
        },
        _ => return Err(c.error(FORMAT, "expected a key")),
    };
    Ok(KeyPart { name, origin })
}

fn is_bare_key(b: u8) -> bool {
    BARE_KEY[usize::from(b)]
}

static BARE_KEY: [bool; 256] = {
    let mut table = [false; 256];
    let mut i = 0;
    while i < 256 {
        let b = i as u8;
        table[i] = b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
        i += 1;
    }
    table
};

/// Parse `[key]` or `[[key]]`; the flag is true for an array of tables.
fn parse_header<'a>(c: &mut Cursor<'a>) -> Result<(Key<'a>, bool), ConfigError> {
    let is_array = c.eat("[[");
    if !is_array {
        c.skip_flat();
    }
    let key = parse_key(c)?;
    skip_ws(c);
    if is_array {
        expect(c, "]]", "`]]` to close the header")?;
    } else {
        expect_byte(c, b']', "`]` to close the header")?;
    }
    Ok((key, is_array))
}

// ---------------------------------------------------------------------------
// Whitespace, comments, newlines
// ---------------------------------------------------------------------------

fn skip_ws(c: &mut Cursor<'_>) {
    if !matches!(c.peek(), Some(b' ' | b'\t')) {
        return;
    }
    let n = c
        .rest()
        .bytes()
        .take_while(|b| matches!(b, b' ' | b'\t'))
        .count();
    c.advance_flat(n);
}

fn eat_newline(c: &mut Cursor<'_>) -> bool {
    match c.peek() {
        Some(b'\n') => {
            c.bump();
            true
        },
        Some(b'\r') if c.peek_at(1) == Some(b'\n') => {
            c.advance(2);
            true
        },
        _ => false,
    }
}

fn at_newline(c: &Cursor<'_>) -> bool {
    c.peek() == Some(b'\n') || (c.peek() == Some(b'\r') && c.peek_at(1) == Some(b'\n'))
}

fn peek_char(c: &Cursor<'_>) -> Option<char> {
    c.rest().chars().next()
}

/// Control characters other than tab are not allowed in comments and strings.
fn is_control(ch: char) -> bool {
    (ch < ' ' && ch != '\t') || ch == '\u{7f}'
}

/// Consume a comment up to, not including, the line break.
fn skip_comment(c: &mut Cursor<'_>) -> Result<(), ConfigError> {
    let n = c
        .rest()
        .bytes()
        .position(|b| CONTROL[usize::from(b)])
        .unwrap_or(c.rest().len());
    c.advance_flat(n);
    if c.peek().is_none_or(|b| b == b'\n') || at_newline(c) {
        Ok(())
    } else {
        Err(c.error(FORMAT, "control character in comment"))
    }
}

fn skip_ws_newlines_comments(c: &mut Cursor<'_>) -> Result<(), ConfigError> {
    loop {
        skip_ws(c);
        if c.peek() == Some(b'#') {
            skip_comment(c)?;
        } else if !eat_newline(c) {
            return Ok(());
        }
    }
}

/// After a header or key/value pair only a comment and the line end may follow.
fn end_of_expression(c: &mut Cursor<'_>) -> Result<(), ConfigError> {
    skip_ws(c);
    if c.peek() == Some(b'#') {
        skip_comment(c)?;
    }
    if c.is_eof() || at_newline(c) {
        Ok(())
    } else {
        Err(c.error(FORMAT, "expected a newline after the expression"))
    }
}

fn expect(c: &mut Cursor<'_>, token: &str, what: &str) -> Result<(), ConfigError> {
    if c.eat(token) { Ok(()) } else { Err(c.error(FORMAT, format!("expected {what}"))) }
}

fn eat_byte(c: &mut Cursor<'_>, byte: u8) -> bool {
    let found = c.peek() == Some(byte);
    if found {
        c.skip_flat();
    }
    found
}

fn expect_byte(c: &mut Cursor<'_>, byte: u8, what: &str) -> Result<(), ConfigError> {
    if eat_byte(c, byte) { Ok(()) } else { Err(c.error(FORMAT, format!("expected {what}"))) }
}

// ---------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------

fn parse_value(c: &mut Cursor<'_>) -> Result<ConfigNode, ConfigError> {
    let origin = c.origin();
    let node = match c.peek() {
        Some(b'"') => {
            let text = if c.rest().starts_with("\"\"\"") {
                ml_basic_string(c)?
            } else {
                basic_string(c)?.into_owned()
            };
            ConfigNode::scalar(text)
        },
        Some(b'\'') => {
            let text = if c.rest().starts_with("'''") {
                ml_literal_string(c)?
            } else {
                literal_string(c)?.into_owned()
            };
            ConfigNode::scalar(text)
        },
        Some(b'[') => parse_array(c)?,
        Some(b'{') => parse_inline_table(c)?,
        _ => parse_scalar(c)?,
    };
    Ok(node.with_origin(origin))
}

fn parse_array(c: &mut Cursor<'_>) -> Result<ConfigNode, ConfigError> {
    c.enter(FORMAT)?;
    c.skip_flat();
    let mut items = Vec::new();
    loop {
        skip_ws_newlines_comments(c)?;
        if eat_byte(c, b']') {
            break;
        }
        items.push(parse_value(c)?);
        skip_ws_newlines_comments(c)?;
        if eat_byte(c, b',') {
            continue;
        }
        if eat_byte(c, b']') {
            break;
        }
        return Err(c.error(FORMAT, "expected `,` or `]` in array"));
    }
    c.leave();
    Ok(ConfigNode::array(items))
}

fn parse_inline_table(c: &mut Cursor<'_>) -> Result<ConfigNode, ConfigError> {
    c.enter(FORMAT)?;
    let mut table = TableSlot::new(c.origin(), State::Header);
    c.skip_flat();
    skip_ws(c);
    if !eat_byte(c, b'}') {
        loop {
            let key = parse_key(c)?;
            skip_ws(c);
            expect_byte(c, b'=', "`=` after the key")?;
            skip_ws(c);
            for _ in &key.parents {
                c.enter(FORMAT)?;
            }
            let value = parse_value(c)?;
            for _ in &key.parents {
                c.leave();
            }
            insert_key_value(&mut table, &key, value)?;
            skip_ws(c);
            if eat_byte(c, b',') {
                skip_ws(c);
                if c.peek() == Some(b'}') {
                    return Err(c.error(FORMAT, "trailing comma in inline table"));
                }
                continue;
            }
            if eat_byte(c, b'}') {
                break;
            }
            return Err(c.error(FORMAT, "expected `,` or `}` in inline table"));
        }
    }
    c.leave();
    Ok(table.into_node())
}

/// Booleans, numbers and date-times: everything that is not a string,
/// array or inline table.
fn parse_scalar(c: &mut Cursor<'_>) -> Result<ConfigNode, ConfigError> {
    let origin = c.origin();
    let start = c.pos();
    take_token(c);
    // A date and a time may be separated by one space.
    if c.peek() == Some(b' ')
        && c.peek_at(1).is_some_and(|b| b.is_ascii_digit())
        && is_date(c.slice_from(start).as_bytes())
    {
        c.skip_flat();
        take_token(c);
    }
    let text = c.slice_from(start);
    if text.is_empty() {
        return Err(error_at(&origin, "expected a value"));
    }
    scalar_text(text)
        .map(ConfigNode::scalar)
        .map_err(|message| error_at(&origin, message))
}

fn take_token(c: &mut Cursor<'_>) {
    let rest = c.rest();
    let n = rest
        .bytes()
        .position(|b| TOKEN_END[usize::from(b)])
        .unwrap_or(rest.len());
    c.advance_flat(n);
}

// ---------------------------------------------------------------------------
// Strings
// ---------------------------------------------------------------------------

/// Length of the run of bytes that can be copied verbatim: everything up to
/// the closing `quote`, a backslash when `escapes` is set, or a control
/// character other than tab.
fn plain_run(rest: &str, quote: u8, escapes: bool) -> usize {
    rest.bytes()
        .position(|b| b == quote || (escapes && b == b'\\') || CONTROL[usize::from(b)])
        .unwrap_or(rest.len())
}

/// Control characters other than tab, which strings and comments reject.
static CONTROL: [bool; 256] = {
    let mut table = [false; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = (i < 0x20 && i != 0x09) || i == 0x7f;
        i += 1;
    }
    table
};

/// Bytes that end a bare value token.
static TOKEN_END: [bool; 256] = {
    let mut table = [false; 256];
    let ends = *b" \t\r\n,]}#";
    let mut i = 0;
    while i < ends.len() {
        table[ends[i] as usize] = true;
        i += 1;
    }
    table
};

/// Consume the plain run at the cursor and return it.
fn take_plain_run<'a>(c: &mut Cursor<'a>, quote: u8, escapes: bool) -> &'a str {
    let rest = c.rest();
    let n = plain_run(rest, quote, escapes);
    c.advance_flat(n);
    rest.get(..n).unwrap_or("")
}

/// Error for the byte that ended a plain run inside a single-line string.
fn string_stop(c: &Cursor<'_>) -> ConfigError {
    match c.peek() {
        None => c.error(FORMAT, "unterminated string"),
        Some(b'\n' | b'\r') => c.error(FORMAT, "newline in single-line string"),
        Some(_) => c.error(FORMAT, "control character in string"),
    }
}

fn basic_string<'a>(c: &mut Cursor<'a>) -> Result<Cow<'a, str>, ConfigError> {
    c.skip_flat();
    let first = take_plain_run(c, b'"', true);
    if c.peek() == Some(b'"') {
        c.skip_flat();
        return Ok(Cow::Borrowed(first));
    }
    let mut out = String::from(first);
    loop {
        match c.peek() {
            Some(b'"') => {
                c.skip_flat();
                return Ok(Cow::Owned(out));
            },
            Some(b'\\') => {
                c.skip_flat();
                out.push(escape(c)?);
            },
            _ => return Err(string_stop(c)),
        }
        out.push_str(take_plain_run(c, b'"', true));
    }
}

fn literal_string<'a>(c: &mut Cursor<'a>) -> Result<Cow<'a, str>, ConfigError> {
    c.skip_flat();
    let text = take_plain_run(c, b'\'', false);
    if c.peek() == Some(b'\'') {
        c.skip_flat();
        Ok(Cow::Borrowed(text))
    } else {
        Err(string_stop(c))
    }
}

fn ml_basic_string(c: &mut Cursor<'_>) -> Result<String, ConfigError> {
    c.eat("\"\"\"");
    eat_newline(c);
    let mut out = String::new();
    loop {
        let Some(ch) = peek_char(c) else {
            return Err(c.error(FORMAT, "unterminated multi-line string"));
        };
        match ch {
            '"' => {
                if ml_quotes(c, b'"', &mut out)? {
                    return Ok(out);
                }
            },
            '\\' => {
                c.bump();
                if line_ending_backslash(c) {
                    while c.peek().is_some_and(|b| matches!(b, b' ' | b'\t' | b'\n'))
                        || at_newline(c)
                    {
                        c.bump();
                    }
                } else {
                    out.push(escape(c)?);
                }
            },
            '\n' => {
                c.bump();
                out.push('\n');
            },
            '\r' if c.peek_at(1) == Some(b'\n') => {
                c.eat("\r\n");
                out.push_str("\r\n");
            },
            ch if is_control(ch) => return Err(c.error(FORMAT, "control character in string")),
            _ => {
                c.bump();
                out.push(ch);
            },
        }
    }
}

fn ml_literal_string(c: &mut Cursor<'_>) -> Result<String, ConfigError> {
    c.eat("'''");
    eat_newline(c);
    let mut out = String::new();
    loop {
        let Some(ch) = peek_char(c) else {
            return Err(c.error(FORMAT, "unterminated multi-line string"));
        };
        match ch {
            '\'' => {
                if ml_quotes(c, b'\'', &mut out)? {
                    return Ok(out);
                }
            },
            '\n' => {
                c.bump();
                out.push('\n');
            },
            '\r' if c.peek_at(1) == Some(b'\n') => {
                c.eat("\r\n");
                out.push_str("\r\n");
            },
            ch if is_control(ch) => return Err(c.error(FORMAT, "control character in string")),
            _ => {
                c.bump();
                out.push(ch);
            },
        }
    }
}

/// Handle a run of `quote` bytes inside a multi-line string. Returns true when
/// the run closes the string; one or two quotes may precede the delimiter.
fn ml_quotes(c: &mut Cursor<'_>, quote: u8, out: &mut String) -> Result<bool, ConfigError> {
    let mut run = 0;
    while c.peek_at(run) == Some(quote) {
        run += 1;
    }
    let (content, closes) = match run {
        1 | 2 => (run, false),
        3..=5 => (run - 3, true),
        _ => return Err(c.error(FORMAT, "too many quotes in multi-line string")),
    };
    for _ in 0..content {
        c.bump();
        out.push(char::from(quote));
    }
    if closes {
        c.eat(if quote == b'"' { "\"\"\"" } else { "'''" });
    }
    Ok(closes)
}

/// After a `\` in a multi-line basic string: true when only blanks precede
/// the line break.
fn line_ending_backslash(c: &Cursor<'_>) -> bool {
    let mut n = 0;
    while matches!(c.peek_at(n), Some(b' ' | b'\t')) {
        n += 1;
    }
    c.peek_at(n) == Some(b'\n') || (c.peek_at(n) == Some(b'\r') && c.peek_at(n + 1) == Some(b'\n'))
}

/// Decode the escape after a consumed backslash.
fn escape(c: &mut Cursor<'_>) -> Result<char, ConfigError> {
    let Some(ch) = peek_char(c) else {
        return Err(c.error(FORMAT, "unterminated string"));
    };
    let decoded = match ch {
        'b' => '\u{8}',
        't' => '\t',
        'n' => '\n',
        'f' => '\u{c}',
        'r' => '\r',
        '"' => '"',
        '\\' => '\\',
        'u' => return unicode_escape(c, 4),
        'U' => return unicode_escape(c, 8),
        other => return Err(c.error(FORMAT, format!("invalid escape sequence \"\\{other}\""))),
    };
    c.bump();
    Ok(decoded)
}

fn unicode_escape(c: &mut Cursor<'_>, digits: usize) -> Result<char, ConfigError> {
    let origin = c.origin();
    c.bump();
    let mut code: u32 = 0;
    for _ in 0..digits {
        let digit = peek_char(c).and_then(|ch| ch.to_digit(16));
        let Some(digit) = digit else {
            return Err(c.error(FORMAT, format!("expected {digits} hex digits in unicode escape")));
        };
        c.bump();
        code = code << 4 | digit;
    }
    char::from_u32(code)
        .ok_or_else(|| error_at(&origin, format!("U+{code:04X} is not a unicode scalar value")))
}

// ---------------------------------------------------------------------------
// Numbers, booleans, date-times
// ---------------------------------------------------------------------------

/// Validate a bare token and return the text to store.
fn scalar_text(text: &str) -> Result<String, String> {
    if matches!(text, "true" | "false") {
        return Ok(text.to_owned());
    }
    // Plain decimal integers short of i64's range are already in canonical
    // form: no sign, underscores or leading zero.
    if matches!(text.len(), 1..=18)
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text.len() == 1 || !text.starts_with('0'))
    {
        return Ok(text.to_owned());
    }
    let bytes = text.as_bytes();
    let starts_date = bytes.get(4) == Some(&b'-') && bytes[..4].iter().all(u8::is_ascii_digit);
    let starts_time = bytes.get(2) == Some(&b':') && bytes[..2].iter().all(u8::is_ascii_digit);
    if starts_date || starts_time {
        return if valid_datetime(bytes) {
            Ok(text.to_owned())
        } else {
            Err(format!("invalid date or time \"{text}\""))
        };
    }
    number_text(text)
}

fn digits_ok(s: &str, radix: u32) -> bool {
    !s.is_empty()
        && !s.starts_with('_')
        && !s.ends_with('_')
        && !s.contains("__")
        && s.chars().all(|ch| ch == '_' || ch.is_digit(radix))
}

fn number_text(text: &str) -> Result<String, String> {
    let invalid = || format!("invalid number \"{text}\"");
    let (negative, body) = match text.as_bytes().first() {
        Some(b'+') => (false, &text[1..]),
        Some(b'-') => (true, &text[1..]),
        _ => (false, text),
    };
    if matches!(body, "inf" | "nan") {
        return Ok(text.to_owned());
    }
    let radix = if body.len() == text.len() {
        [("0x", 16), ("0o", 8), ("0b", 2)]
            .into_iter()
            .find_map(|(prefix, radix)| text.strip_prefix(prefix).map(|digits| (digits, radix)))
    } else {
        None
    };
    if let Some((digits, radix)) = radix {
        if !digits_ok(digits, radix) {
            return Err(invalid());
        }
        let cleaned: String = digits.chars().filter(|&ch| ch != '_').collect();
        return u64::from_str_radix(&cleaned, radix)
            .ok()
            .filter(|&n| n <= i64::MAX as u64)
            .map(|n| n.to_string())
            .ok_or_else(|| format!("integer \"{text}\" is out of range for i64"));
    }

    let int_end = body.find(['.', 'e', 'E']).unwrap_or(body.len());
    let (int_part, rest) = body.split_at(int_end);
    if !digits_ok(int_part, 10) {
        return Err(invalid());
    }
    if int_part.len() > 1 && int_part.starts_with('0') {
        return Err(format!("leading zeros are not allowed in \"{text}\""));
    }
    if rest.is_empty() {
        let cleaned: String = int_part.chars().filter(|&ch| ch != '_').collect();
        let signed = if negative { format!("-{cleaned}") } else { cleaned };
        return signed
            .parse::<i64>()
            .map(|n| n.to_string())
            .map_err(|_| format!("integer \"{text}\" is out of range for i64"));
    }

    let (fraction, exponent) = match rest.strip_prefix('.') {
        Some(after) => match after.find(['e', 'E']) {
            Some(i) => (Some(&after[..i]), Some(&after[i + 1..])),
            None => (Some(after), None),
        },
        None => (None, rest.get(1..)),
    };
    if fraction.is_some_and(|f| !digits_ok(f, 10)) {
        return Err(invalid());
    }
    if let Some(exp) = exponent {
        let exp = exp.strip_prefix(['+', '-']).unwrap_or(exp);
        if !digits_ok(exp, 10) {
            return Err(invalid());
        }
    }
    Ok(text.chars().filter(|&ch| ch != '_').collect())
}

fn two_digits(b: &[u8], i: usize) -> Option<u32> {
    let (tens, ones) = (*b.get(i)?, *b.get(i + 1)?);
    if tens.is_ascii_digit() && ones.is_ascii_digit() {
        Some(u32::from(tens - b'0') * 10 + u32::from(ones - b'0'))
    } else {
        None
    }
}

fn is_date(b: &[u8]) -> bool {
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' || !b[..4].iter().all(u8::is_ascii_digit) {
        return false;
    }
    let year = b[..4]
        .iter()
        .fold(0u32, |acc, d| acc * 10 + u32::from(d - b'0'));
    let (Some(month), Some(day)) = (two_digits(b, 5), two_digits(b, 8)) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

/// Validate `HH:MM:SS[.frac]` at the start of `b`; returns its length.
fn time_len(b: &[u8]) -> Option<usize> {
    if two_digits(b, 0)? > 23 || b.get(2) != Some(&b':') || two_digits(b, 3)? > 59 {
        return None;
    }
    if b.get(5) != Some(&b':') || two_digits(b, 6)? > 60 {
        return None;
    }
    if b.get(8) != Some(&b'.') {
        return Some(8);
    }
    let fraction = b[9..].iter().take_while(|d| d.is_ascii_digit()).count();
    if fraction == 0 { None } else { Some(9 + fraction) }
}

fn valid_offset(b: &[u8]) -> bool {
    match b {
        [] | [b'Z' | b'z'] => true,
        [b'+' | b'-', ..] => {
            b.len() == 6
                && b[3] == b':'
                && two_digits(b, 1).is_some_and(|h| h < 24)
                && two_digits(b, 4).is_some_and(|m| m < 60)
        },
        _ => false,
    }
}

/// Lexical validation of the four TOML date-time forms.
fn valid_datetime(b: &[u8]) -> bool {
    if b.get(2) == Some(&b':') {
        return time_len(b) == Some(b.len());
    }
    if b.len() == 10 {
        return is_date(b);
    }
    if b.len() < 11 || !is_date(&b[..10]) || !matches!(b[10], b'T' | b't' | b' ') {
        return false;
    }
    let time = &b[11..];
    time_len(time).is_some_and(|n| valid_offset(&time[n..]))
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;
    use crate::{Table, Value};

    fn s(text: &str) -> ConfigNode {
        ConfigNode::scalar(text)
    }
    #[test]
    fn tables_of_every_size_keep_keys_and_reject_duplicates() {
        for len in 1..40_usize {
            let orders: [Vec<usize>; 3] = [
                (0..len).collect(),
                (0..len).rev().collect(),
                (0..len).map(|i| (i * 7) % len).collect(),
            ];
            for order in orders {
                let mut seen = std::collections::BTreeSet::new();
                let order: Vec<usize> = order.into_iter().filter(|i| seen.insert(*i)).collect();
                let text: String = order.iter().map(|i| format!("k{i:03} = {i}\n")).collect();
                let node = parse(&text, "t").unwrap();
                let table = node.as_table().unwrap();
                assert_eq!(table.len(), order.len(), "{len}");
                for i in &order {
                    assert_eq!(
                        node.get(&format!("k{i:03}")).unwrap().as_str(),
                        Some(i.to_string().as_str())
                    );
                }
                for dup in &order {
                    let err = parse(&format!("{text}k{dup:03} = 0\n"), "t").unwrap_err();
                    assert!(
                        err.to_string()
                            .contains(&format!("duplicate key \"k{dup:03}\"")),
                        "{err}"
                    );
                }
            }
        }
    }

    #[test]
    fn columns_count_characters_across_flat_runs() {
        let doc = parse("\"é\" = \"ü€\" # ñ\n\"kéy\" = 42 # ö", "t").unwrap();
        let at = |key: &str| {
            let origin = doc.get(key).unwrap().origin().unwrap();
            (origin.line(), origin.column())
        };
        assert_eq!(at("é"), (1, 7));
        assert_eq!(at("kéy"), (2, 9));
        let err = parse("a = 1 # ü\nb = 08", "t").unwrap_err();
        assert!(err.to_string().ends_with("(t:2:5)"), "{err}");
    }

    fn t(entries: Vec<(&str, ConfigNode)>) -> ConfigNode {
        ConfigNode::table(
            entries
                .into_iter()
                .map(|(k, v)| (k.to_owned(), v))
                .collect(),
        )
    }

    fn a(items: Vec<ConfigNode>) -> ConfigNode {
        ConfigNode::array(items)
    }

    fn bare(node: &ConfigNode) -> ConfigNode {
        match node.value() {
            Value::Null => ConfigNode::null(),
            Value::Scalar(text) => ConfigNode::scalar(text.clone()),
            Value::Array(items) => ConfigNode::array(items.iter().map(bare).collect()),
            Value::Table(table) => ConfigNode::table(
                table
                    .iter()
                    .map(|(k, v)| (k.clone(), bare(v)))
                    .collect::<Table>(),
            ),
        }
    }

    fn ok(input: &str) -> ConfigNode {
        match parse(input, "<string>") {
            Ok(node) => bare(&node),
            Err(err) => panic!("{input:?} should parse, got: {err}"),
        }
    }

    fn err(input: &str) -> String {
        match parse(input, "<string>") {
            Ok(node) => panic!("{input:?} should fail, parsed {node:?}"),
            Err(err) => err.to_string(),
        }
    }

    fn scalar(input: &str) -> String {
        ok(&format!("k = {input}"))
            .get("k")
            .and_then(ConfigNode::as_str)
            .unwrap_or("")
            .to_owned()
    }

    #[test]
    fn empty_and_comment_only_documents_are_empty_tables() {
        assert_eq!(ok(""), t(vec![]));
        assert_eq!(ok("# note\r\n\r\n   # more\n"), t(vec![]));
    }

    #[test]
    fn tables_and_arrays_of_tables_build_a_tree() {
        let input = "\
name = \"app\"
[server]
port = 80
[server.tls]
on = true
[[user]]
id = 1
[[user]]
id = 2
[user.meta]
admin = false
";
        assert_eq!(
            ok(input),
            t(vec![
                ("name", s("app")),
                ("server", t(vec![("port", s("80")), ("tls", t(vec![("on", s("true"))]))])),
                (
                    "user",
                    a(vec![
                        t(vec![("id", s("1"))]),
                        t(vec![("id", s("2")), ("meta", t(vec![("admin", s("false"))]))]),
                    ])
                ),
            ])
        );
    }

    #[test]
    fn nodes_carry_the_origin_of_their_first_byte() {
        let node = parse("a = 1\n[t]\n  b = \"x\"\n", "f.toml").unwrap();
        let origin =
            |n: Option<&ConfigNode>| n.and_then(ConfigNode::origin).map(ToString::to_string);
        assert_eq!(origin(node.get("a")), Some("f.toml:1:5".into()));
        assert_eq!(origin(node.get("t")), Some("f.toml:2:2".into()));
        assert_eq!(origin(node.get("t").and_then(|t| t.get("b"))), Some("f.toml:3:7".into()));
    }

    #[test]
    fn dotted_and_quoted_keys_build_nested_tables() {
        assert_eq!(
            ok("a.b.c = 1\na.b.d = 2\n\"x y\".'z' = 3\n"),
            t(vec![
                ("a", t(vec![("b", t(vec![("c", s("1")), ("d", s("2"))]))])),
                ("x y", t(vec![("z", s("3"))])),
            ])
        );
        assert_eq!(
            ok("[ a . b ]\nc = 1\n"),
            t(vec![("a", t(vec![("b", t(vec![("c", s("1"))]))]))])
        );
    }

    #[test]
    fn integers_are_emitted_as_decimal_text() {
        assert_eq!(scalar("0xff"), "255");
        assert_eq!(scalar("0xDEAD_beef"), "3735928559");
        assert_eq!(scalar("0o17"), "15");
        assert_eq!(scalar("0b1010_1010"), "170");
        assert_eq!(scalar("1_000"), "1000");
        assert_eq!(scalar("+5"), "5");
        assert_eq!(scalar("-17"), "-17");
        assert_eq!(scalar("0"), "0");
        assert_eq!(scalar("-9223372036854775808"), "-9223372036854775808");
        assert_eq!(scalar("9223372036854775807"), "9223372036854775807");
        assert_eq!(scalar("0x7fff_ffff_ffff_ffff"), "9223372036854775807");
    }

    #[test]
    fn floats_drop_underscores_and_otherwise_stay_verbatim() {
        assert_eq!(scalar("3.14"), "3.14");
        assert_eq!(scalar("+1.0"), "+1.0");
        assert_eq!(scalar("-0.01"), "-0.01");
        assert_eq!(scalar("5e+22"), "5e+22");
        assert_eq!(scalar("1e06"), "1e06");
        assert_eq!(scalar("6.626e-34"), "6.626e-34");
        assert_eq!(scalar("224_617.445_991"), "224617.445991");
        assert_eq!(scalar("inf"), "inf");
        assert_eq!(scalar("-inf"), "-inf");
        assert_eq!(scalar("+nan"), "+nan");
    }

    #[test]
    fn booleans_are_scalars() {
        assert_eq!(scalar("true"), "true");
        assert_eq!(scalar("false"), "false");
    }

    #[test]
    fn basic_strings_decode_escapes() {
        assert_eq!(
            scalar(r#""a\tb\n\"q\" \\ \u00e9 \U0001F600 \b\f\r""#),
            "a\tb\n\"q\" \\ é 😀 \u{8}\u{c}\r"
        );
    }

    #[test]
    fn literal_strings_keep_backslashes() {
        assert_eq!(scalar(r"'C:\path\n'"), r"C:\path\n");
    }

    #[test]
    fn multi_line_basic_string_trims_first_newline_and_continuations() {
        assert_eq!(scalar("\"\"\"\nRoses\nare red\"\"\""), "Roses\nare red");
        assert_eq!(scalar("\"\"\"a \\\n   b \\  \n\n  c\"\"\""), "a b c");
        assert_eq!(scalar("\"\"\"he said \"\"\"\"\""), "he said \"\"");
        assert_eq!(scalar("\"\"\"\r\nx\r\ny\"\"\""), "x\r\ny");
    }

    #[test]
    fn multi_line_literal_string_is_raw() {
        assert_eq!(scalar("'''\nC:\\a\\n\n  b'''"), "C:\\a\\n\n  b");
        assert_eq!(scalar("'''it''s'''"), "it''s");
    }

    #[test]
    fn date_times_are_kept_verbatim() {
        for text in [
            "1979-05-27T07:32:00Z",
            "1979-05-27T00:32:00.999999-07:00",
            "1979-05-27 07:32:00Z",
            "1979-05-27t07:32:00+01:30",
            "1979-05-27T07:32:00",
            "1979-05-27 07:32:00.5",
            "1979-05-27",
            "07:32:00",
            "00:32:00.999999",
            "2024-02-29",
        ] {
            assert_eq!(scalar(text), text);
        }
        assert_eq!(
            ok("a = 1979-05-27 07:32:00 # c\nb = [1979-05-27, 07:32:00]\n"),
            t(vec![
                ("a", s("1979-05-27 07:32:00")),
                ("b", a(vec![s("1979-05-27"), s("07:32:00")]))
            ])
        );
    }

    #[test]
    fn malformed_date_times_fail() {
        for text in [
            "1979-13-01",
            "1979-02-29",
            "1979-05-27T25:00:00",
            "1979-05-27T07:32",
            "1979-05-27T07:32:00.",
            "07:32:00Z",
            "1979-05-27T07:32:00+0100",
            "1979-5-27",
        ] {
            assert!(err(&format!("k = {text}")).contains("invalid"), "{text}");
        }
    }

    #[test]
    fn arrays_allow_comments_trailing_comma_and_mixed_types() {
        assert_eq!(
            ok("k = [\n  1, # one\n  \"two\",\n  [3.0],\n  {x = 4},\n]\n"),
            t(vec![("k", a(vec![s("1"), s("two"), a(vec![s("3.0")]), t(vec![("x", s("4"))])]))])
        );
        assert_eq!(ok("k = []"), t(vec![("k", a(vec![]))]));
    }

    #[test]
    fn inline_tables_support_dotted_keys() {
        assert_eq!(
            ok("p = { x = 1, y.z = 2, \"q\" = {} }"),
            t(vec![("p", t(vec![("x", s("1")), ("y", t(vec![("z", s("2"))])), ("q", t(vec![]))]))])
        );
    }

    #[test]
    fn redefining_a_table_fails_with_origin() {
        assert_eq!(
            err("[a]\nx = 1\n[a]\n"),
            "invalid toml: table \"a\" is already defined (<string>:3:2)"
        );
        assert_eq!(
            err("[a.b]\n[a.b]"),
            "invalid toml: table \"a.b\" is already defined (<string>:2:4)"
        );
    }

    #[test]
    fn duplicate_keys_fail() {
        assert_eq!(err("a = 1\na = 2"), "invalid toml: duplicate key \"a\" (<string>:2:1)");
        assert_eq!(
            err("a = 1\na.b = 2"),
            "invalid toml: key \"a\" is already defined (<string>:2:1)"
        );
        assert_eq!(err("a.b = 1\na.b = 2"), "invalid toml: duplicate key \"a.b\" (<string>:2:3)");
        assert!(err("[a]\nx = 1\nx = 2").contains("duplicate key"));
        assert!(err("a = { x = 1, x = 2 }").contains("duplicate key"));
    }

    #[test]
    fn headers_cannot_redefine_tables_made_by_dotted_keys() {
        // TOML 1.0: dotted keys define every table before the last part, so a
        // later [a] header for such a table is invalid.
        assert_eq!(
            err("a.b = 1\n[a]\nc = 2"),
            "invalid toml: table \"a\" is already defined (<string>:2:2)"
        );
        assert_eq!(
            err("a.b = 1\n[a.b]"),
            "invalid toml: key \"a.b\" is already defined (<string>:2:4)"
        );
        assert!(err("[fruit]\napple.color = \"red\"\n[fruit.apple]").contains("already defined"));
        assert!(
            err("[fruit]\napple.taste.sweet = true\n[fruit.apple.taste]")
                .contains("already defined")
        );
    }

    #[test]
    fn sub_tables_may_extend_dotted_tables_and_super_tables_follow() {
        assert_eq!(
            ok("[fruit]\napple.color = \"red\"\n[fruit.apple.texture]\nsmooth = true\n"),
            t(vec![(
                "fruit",
                t(vec![(
                    "apple",
                    t(vec![("color", s("red")), ("texture", t(vec![("smooth", s("true"))]))])
                )])
            )])
        );
        assert_eq!(
            ok("[x.y.z.w]\n[x]\nk = 1\n"),
            t(vec![(
                "x",
                t(vec![("k", s("1")), ("y", t(vec![("z", t(vec![("w", t(vec![]))]))]))])
            )])
        );
    }

    #[test]
    fn dotted_keys_cannot_extend_header_tables() {
        assert!(err("[a.b.c]\nz = 9\n[a]\nb.c.t = 1").contains("already defined"));
        assert!(err("[a.b.c.d]\nz = 9\n[a]\nb.c.d.k.t = 1").contains("already defined"));
        assert!(err("[a]\n[b]\n[a.c]\n[b.d]\n[a]\n").contains("already defined"));
    }

    #[test]
    fn inline_tables_and_static_arrays_are_frozen() {
        assert_eq!(
            err("a = { b = 1 }\n[a.c]"),
            "invalid toml: inline table \"a\" cannot be extended (<string>:2:2)"
        );
        assert!(err("a = { b = 1 }\n[a]").contains("inline table"));
        assert!(err("a = { b = 1 }\na.c = 2").contains("inline table"));
        assert!(err("a = { b = { c = 1 } }\n[a.b.d]").contains("inline table"));
        assert!(err("a = [1]\n[[a]]").contains("static array"));
        assert!(err("a = [{ b = 1 }]\n[a.c]").contains("static array"));
        assert!(err("a = [1]\na.b = 1").contains("static array"));
    }

    #[test]
    fn array_of_tables_conflicts_fail() {
        assert!(err("[a]\n[[a]]").contains("array of tables"));
        assert!(err("[[a]]\n[a]").contains("array of tables"));
        assert!(err("[[t.a]]\n[t]\na.b = 1").contains("dotted key"));
        assert!(err("[[a]]\n[a.b]\n[a.b]").contains("already defined"));
    }

    #[test]
    fn leading_zeros_fail() {
        assert_eq!(
            err("a = 01"),
            "invalid toml: leading zeros are not allowed in \"01\" (<string>:1:5)"
        );
        assert!(err("a = -012").contains("leading zeros"));
        assert!(err("a = 00").contains("leading zeros"));
        assert!(err("a = 01.5").contains("leading zeros"));
    }

    #[test]
    fn out_of_range_integers_fail() {
        assert!(err("a = 9223372036854775808").contains("out of range"));
        assert!(err("a = -9223372036854775809").contains("out of range"));
        assert!(err("a = 0xffff_ffff_ffff_ffff").contains("out of range"));
        assert!(err("a = 0x8000000000000000").contains("out of range"));
    }

    #[test]
    fn malformed_numbers_fail() {
        for text in [
            "1__0", "_1", "1_", "1_.5", "1._5", "1.", ".5", "1e", "1e_2", "+0x1", "0X1", "0xg",
            "0x", "0b2", "0o8", "-inf_", "Inf", "NaN", "++1", "1.2.3", "1-2", "abc", "0x_1",
        ] {
            let message = err(&format!("a = {text}"));
            assert!(message.starts_with("invalid toml: "), "{text}: {message}");
        }
    }

    #[test]
    fn invalid_strings_fail() {
        assert!(err("a = \"x\ny\"").contains("newline in single-line string"));
        assert!(err("a = 'x\ny'").contains("newline in single-line string"));
        assert!(err("a = \"unterminated").contains("unterminated"));
        assert!(err("a = \"\\q\"").contains("invalid escape"));
        assert!(err("a = \"\\e\"").contains("invalid escape"));
        assert!(err("a = \"\\x41\"").contains("invalid escape"));
        assert!(err("a = \"\\ud800\"").contains("not a unicode scalar value"));
        assert!(err("a = \"\\U00110000\"").contains("not a unicode scalar value"));
        assert!(err("a = \"\\u12\"").contains("hex digits"));
        assert!(err("a = \"\u{1}\"").contains("control character"));
        assert!(err("a = \"\"\"x\\ y\"\"\"").contains("invalid escape"));
        assert!(err("a = \"\"\"x").contains("unterminated"));
        assert!(err("a = \"\"\"x\"\"\"\"\"\"").contains("too many quotes"));
    }

    #[test]
    fn structural_errors_fail() {
        assert!(err("a = 1 b = 2").contains("expected a newline"));
        assert!(err("a =").contains("expected a value"));
        assert!(err("a = \n").contains("expected a value"));
        assert!(err("= 1").contains("expected a key"));
        assert!(err("a 1").contains("expected `=`"));
        assert!(err("[a").contains("expected `]`"));
        assert!(err("[[a]").contains("expected `]]`"));
        assert!(err("[a] b").contains("expected a newline"));
        assert!(err("k = [1 2]").contains("expected `,` or `]`"));
        assert!(err("k = [,]").contains("expected a value"));
        assert!(err("k = [1,,2]").contains("expected a value"));
        assert!(err("k = [1").contains("expected `,` or `]`"));
        assert!(err("k = { a = 1, }").contains("trailing comma"));
        assert!(err("k = { a = 1\n}").contains("expected `,` or `}`"));
        assert!(err("k = {\n}").contains("expected a key"));
        assert!(err("# bad \u{1}\n").contains("control character in comment"));
        assert!(err("a = 1\r b = 2").contains("expected a newline"));
        assert!(err("a = 1__").starts_with("invalid toml"));
        assert!(err("[a]]").contains("expected a newline"));
        assert!(err("[[a] ]").contains("expected `]]`"));
        assert!(err("[ [a]]").contains("expected a key"));
    }

    #[test]
    fn nesting_deeper_than_128_fails() {
        let nested = |n: usize| format!("k = {}{}", "[".repeat(n), "]".repeat(n));
        assert!(parse(&nested(128), "<string>").is_ok());
        assert_eq!(
            err(&nested(129)),
            format!("invalid toml: nesting deeper than 128 levels (<string>:1:{})", 5 + 128)
        );
        let inline = |n: usize| format!("k = {}1{}", "{a=".repeat(n), "}".repeat(n));
        assert!(parse(&inline(128), "<string>").is_ok());
        assert!(err(&inline(129)).contains("nesting deeper than 128 levels"));
        let header = |n: usize| format!("[{}]\n", vec!["a"; n].join("."));
        assert!(parse(&header(128), "<string>").is_ok());
        assert!(err(&header(129)).contains("deeper than 128 levels"));
        let dotted = |n: usize| format!("{} = 1\n", vec!["a"; n].join("."));
        assert!(parse(&dotted(128), "<string>").is_ok());
        assert!(err(&dotted(129)).contains("deeper than 128 levels"));
    }

    #[test]
    fn leading_bom_is_skipped() {
        assert_eq!(ok("\u{feff}a = 1"), t(vec![("a", s("1"))]));
    }

    #[test]
    fn crlf_line_endings_parse() {
        assert_eq!(
            ok("a = 1\r\n[t]\r\nb = [\r\n  2,\r\n]\r\n"),
            t(vec![("a", s("1")), ("t", t(vec![("b", a(vec![s("2")]))]))])
        );
    }
}
