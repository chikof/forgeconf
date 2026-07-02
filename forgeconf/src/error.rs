//! Error types for forgeconf.

use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::PathBuf;

use crate::Origin;

/// Error returned while loading or converting configuration.
///
/// Boxed so `Result<T, ConfigError>` stays small on the success path. Use
/// [`kind`](ConfigError::kind) to inspect what went wrong,
/// [`path`](ConfigError::path) for where in the tree, and
/// [`origin`](ConfigError::origin) for the file position.
pub struct ConfigError(Box<Inner>);

const _: () = assert!(size_of::<ConfigError>() == 8);

/// `path` is stored innermost-first: callers push segments while the error
/// travels up the tree.
struct Inner {
    kind: ErrorKind,
    path: Vec<PathSegment>,
    origin: Option<Origin>,
}

/// What went wrong.
#[derive(Debug)]
#[non_exhaustive]
pub enum ErrorKind {
    /// A required value is absent.
    Missing,
    /// A value cannot be converted into the expected type.
    Mismatch {
        /// Type label of the target.
        expected: Cow<'static, str>,
        /// The offending text.
        found: String,
    },
    /// A validator rejected the value.
    Invalid(String),
    /// A key that no field declares.
    UnknownKey {
        /// Closest declared key, when one is similar enough.
        suggestion: Option<String>,
    },
    /// An enum value that matches no variant.
    UnknownVariant {
        /// The offending text.
        found: String,
        /// Every accepted variant key.
        expected: &'static [&'static str],
    },
    /// The input is not valid in its format.
    Syntax {
        /// Format label such as `toml`.
        format: &'static str,
        /// Parser message.
        message: String,
    },
    /// A feature or format that is unavailable.
    Unsupported(String),
    /// A file could not be read.
    Io {
        /// The file that failed.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// Bad command-line input.
    Cli(String),
    /// The user asked for the help text.
    Help(String),
    /// Several independent errors.
    Multiple(Vec<ConfigError>),
}

/// One step on the way from the root to a value.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathSegment {
    /// A table key.
    Key(String),
    /// An array position.
    Index(usize),
}

impl ConfigError {
    /// Wrap `kind` with an empty path and no origin.
    pub fn new(kind: ErrorKind) -> Self {
        Self(Box::new(Inner { kind, path: Vec::new(), origin: None }))
    }

    /// A required value is absent.
    pub fn missing() -> Self {
        Self::new(ErrorKind::Missing)
    }

    /// A value cannot be converted into `expected`.
    pub fn mismatch(expected: impl Into<Cow<'static, str>>, found: impl Into<String>) -> Self {
        Self::new(ErrorKind::Mismatch {
            expected: expected.into(),
            found: found.into(),
        })
    }

    /// A validator rejected the value.
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Invalid(message.into()))
    }

    /// Record that the error happened under table key `key`. Applied to every
    /// child of [`ErrorKind::Multiple`]; ignored by [`ErrorKind::Help`].
    #[must_use]
    pub fn at_key(self, key: &str) -> Self {
        self.push(&|| PathSegment::Key(key.to_owned()))
    }

    /// Record that the error happened at array position `index`.
    #[must_use]
    pub fn at_index(self, index: usize) -> Self {
        self.push(&|| PathSegment::Index(index))
    }

    /// Set the origin unless one is already set. Applied to every child of
    /// [`ErrorKind::Multiple`]; ignored by [`ErrorKind::Help`].
    #[must_use]
    pub fn with_origin(mut self, origin: Option<&Origin>) -> Self {
        match &mut self.0.kind {
            ErrorKind::Multiple(children) => {
                *children = std::mem::take(children)
                    .into_iter()
                    .map(|child| child.with_origin(origin))
                    .collect();
            },
            ErrorKind::Help(_) => {},
            _ => {
                if self.0.origin.is_none() {
                    self.0.origin = origin.cloned();
                }
            },
        }
        self
    }

    /// Replace every offending value in the report with `***`.
    #[must_use]
    pub fn redacted(mut self) -> Self {
        match &mut self.0.kind {
            ErrorKind::Mismatch { found, .. } => *found = String::from("***"),
            ErrorKind::Multiple(children) => {
                *children = std::mem::take(children)
                    .into_iter()
                    .map(ConfigError::redacted)
                    .collect();
            },
            _ => {},
        }
        self
    }

    /// What went wrong.
    pub fn kind(&self) -> &ErrorKind {
        &self.0.kind
    }

    /// Where the value came from, when known.
    pub fn origin(&self) -> Option<&Origin> {
        self.0.origin.as_ref()
    }

    /// Dotted path from the root: `server.port`, `servers[2].host`, or an empty
    /// string at the root.
    pub fn path(&self) -> String {
        let mut out = String::new();
        self.write_path(&mut out);
        out
    }

    /// Whether the user asked for the help text.
    pub fn is_help(&self) -> bool {
        matches!(self.0.kind, ErrorKind::Help(_))
    }

    /// Every leaf error: flattens [`ErrorKind::Multiple`] recursively and
    /// yields `self` for any other kind.
    pub fn errors(&self) -> impl Iterator<Item = &ConfigError> {
        let mut out = Vec::new();
        self.flatten_into(&mut out);
        out.into_iter()
    }

    fn push(mut self, segment: &dyn Fn() -> PathSegment) -> Self {
        match &mut self.0.kind {
            ErrorKind::Multiple(children) => {
                *children = std::mem::take(children)
                    .into_iter()
                    .map(|child| child.push(segment))
                    .collect();
            },
            ErrorKind::Help(_) => {},
            _ => self.0.path.push(segment()),
        }
        self
    }

    /// Move every leaf error into `out`, flattening nested `Multiple`.
    pub(crate) fn into_leaves(mut self, out: &mut Vec<ConfigError>) {
        match &mut self.0.kind {
            ErrorKind::Multiple(children) => {
                for child in std::mem::take(children) {
                    child.into_leaves(out);
                }
            },
            _ => out.push(self),
        }
    }

    fn flatten_into<'a>(&'a self, out: &mut Vec<&'a ConfigError>) {
        match &self.0.kind {
            ErrorKind::Multiple(children) => {
                for child in children {
                    child.flatten_into(out);
                }
            },
            _ => out.push(self),
        }
    }

    fn write_path(&self, out: &mut String) {
        for segment in self.0.path.iter().rev() {
            match segment {
                PathSegment::Key(key) => {
                    if !out.is_empty() {
                        out.push('.');
                    }
                    out.push_str(key);
                },
                PathSegment::Index(index) => {
                    out.push('[');
                    out.push_str(&index.to_string());
                    out.push(']');
                },
            }
        }
    }
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0.kind {
            ErrorKind::Help(text) => return f.write_str(text),
            ErrorKind::Multiple(_) => {
                let leaves: Vec<&ConfigError> = self.errors().collect();
                write!(f, "{} configuration errors:", leaves.len())?;
                for leaf in leaves {
                    write!(f, "\n  - {leaf}")?;
                }
                return Ok(());
            },
            _ => {},
        }

        let path = self.path();
        if !path.is_empty() {
            write!(f, "{path}: ")?;
        }
        match &self.0.kind {
            ErrorKind::Missing => f.write_str("missing value")?,
            ErrorKind::Mismatch { expected, found } => {
                write!(f, "expected {expected}, found \"{found}\"")?;
            },
            ErrorKind::Invalid(message) | ErrorKind::Cli(message) => f.write_str(message)?,
            ErrorKind::UnknownKey { suggestion } => {
                f.write_str("unknown key")?;
                if let Some(suggestion) = suggestion {
                    write!(f, ", did you mean \"{suggestion}\"?")?;
                }
            },
            ErrorKind::UnknownVariant { found, expected } => {
                write!(f, "unknown variant \"{found}\", expected one of: {}", expected.join(", "))?;
            },
            ErrorKind::Syntax { format, message } => write!(f, "invalid {format}: {message}")?,
            ErrorKind::Unsupported(message) => write!(f, "unsupported: {message}")?,
            ErrorKind::Io { path, source } => {
                write!(f, "cannot read {}: {source}", path.display())?;
            },
            ErrorKind::Help(_) | ErrorKind::Multiple(_) => {},
        }
        if let Some(origin) = &self.0.origin {
            write!(f, " ({origin})")?;
        }
        Ok(())
    }
}

// Debug prints the Display text so `fn main() -> Result<(), ConfigError>` shows
// the readable report instead of a struct dump.
impl fmt::Debug for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.0.kind {
            ErrorKind::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// A non-fatal finding, such as a deprecated key that is still in use.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Warning {
    path: String,
    message: String,
    origin: Option<Origin>,
}

impl Warning {
    pub(crate) fn new(
        path: impl Into<String>,
        message: impl Into<String>,
        origin: Option<Origin>,
    ) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
            origin,
        }
    }

    /// Dotted path of the value the warning is about.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Human-readable description.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Where the value came from, when known.
    pub fn origin(&self) -> Option<&Origin> {
        self.origin.as_ref()
    }
}

impl Display for Warning {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)?;
        if let Some(origin) = &self.origin {
            write!(f, " ({origin})")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;

    use super::*;

    #[test]
    fn display_missing() {
        assert_eq!(ConfigError::missing().to_string(), "missing value");
        assert_eq!(ConfigError::missing().at_key("port").to_string(), "port: missing value");
    }

    #[test]
    fn display_mismatch_with_path_and_origin() {
        let err = ConfigError::mismatch("u16", "abc")
            .at_key("port")
            .at_key("server")
            .with_origin(Some(&Origin::new("app.toml", 3, 8)));
        assert_eq!(err.to_string(), "server.port: expected u16, found \"abc\" (app.toml:3:8)");
    }

    #[test]
    fn display_invalid() {
        let err = ConfigError::invalid("must be even").at_key("port");
        assert_eq!(err.to_string(), "port: must be even");
    }

    #[test]
    fn display_unknown_key() {
        let plain = ConfigError::new(ErrorKind::UnknownKey { suggestion: None }).at_key("prot");
        assert_eq!(plain.to_string(), "prot: unknown key");
        let hinted = ConfigError::new(ErrorKind::UnknownKey { suggestion: Some("port".into()) })
            .at_key("prot");
        assert_eq!(hinted.to_string(), "prot: unknown key, did you mean \"port\"?");
    }

    #[test]
    fn display_unknown_variant() {
        let err = ConfigError::new(ErrorKind::UnknownVariant {
            found: "loud".into(),
            expected: &["debug", "info", "warn"],
        })
        .at_key("level");
        assert_eq!(
            err.to_string(),
            "level: unknown variant \"loud\", expected one of: debug, info, warn"
        );
    }

    #[test]
    fn display_syntax_with_origin() {
        let err = ConfigError::new(ErrorKind::Syntax {
            format: "toml",
            message: "unexpected end of input".into(),
        })
        .with_origin(Some(&Origin::new("app.toml", 7, 1)));
        assert_eq!(err.to_string(), "invalid toml: unexpected end of input (app.toml:7:1)");
    }

    #[test]
    fn display_unsupported() {
        let err = ConfigError::new(ErrorKind::Unsupported("format yaml is disabled".into()));
        assert_eq!(err.to_string(), "unsupported: format yaml is disabled");
    }

    #[test]
    fn display_io_and_source() {
        let err = ConfigError::new(ErrorKind::Io {
            path: PathBuf::from("/etc/app.toml"),
            source: io::Error::new(io::ErrorKind::NotFound, "no such file"),
        });
        assert_eq!(err.to_string(), "cannot read /etc/app.toml: no such file");
        assert_eq!(err.source().map(ToString::to_string).as_deref(), Some("no such file"));
        assert!(ConfigError::missing().source().is_none());
    }

    #[test]
    fn display_cli() {
        let err = ConfigError::new(ErrorKind::Cli("unknown flag --x".into()));
        assert_eq!(err.to_string(), "unknown flag --x");
    }

    #[test]
    fn display_help_ignores_path_and_origin() {
        let err = ConfigError::new(ErrorKind::Help("Usage: app".into()))
            .at_key("ignored")
            .with_origin(Some(&Origin::new("cli", 0, 0)));
        assert_eq!(err.to_string(), "Usage: app");
        assert!(err.is_help());
        assert_eq!(err.path(), "");
    }

    #[test]
    fn display_multiple_flattens_children() {
        let nested = ConfigError::new(ErrorKind::Multiple(vec![
            ConfigError::missing().at_key("host"),
            ConfigError::mismatch("u16", "abc").at_key("port"),
        ]));
        let err = ConfigError::new(ErrorKind::Multiple(vec![nested, ConfigError::invalid("bad")]));
        assert_eq!(
            err.to_string(),
            "3 configuration errors:\n  - host: missing value\n  - port: expected u16, found \
             \"abc\"\n  - bad"
        );
        assert_eq!(err.errors().count(), 3);
    }

    #[test]
    fn multiple_prefixes_every_child() {
        let err = ConfigError::new(ErrorKind::Multiple(vec![
            ConfigError::missing().at_key("host"),
            ConfigError::mismatch("u16", "abc").at_key("port"),
        ]))
        .at_key("server")
        .with_origin(Some(&Origin::new("app.toml", 2, 1)));
        let paths: Vec<String> = err.errors().map(ConfigError::path).collect();
        assert_eq!(paths, ["server.host", "server.port"]);
        assert!(
            err.errors()
                .all(|e| e.origin() == Some(&Origin::new("app.toml", 2, 1)))
        );
    }

    #[test]
    fn errors_yields_self_for_single() {
        let err = ConfigError::missing();
        assert_eq!(err.errors().count(), 1);
    }

    #[test]
    fn redacted_replaces_found() {
        let err = ConfigError::mismatch("u16", "hunter2")
            .at_key("password")
            .redacted();
        assert_eq!(err.to_string(), "password: expected u16, found \"***\"");
    }

    #[test]
    fn redacted_reaches_children() {
        let err = ConfigError::new(ErrorKind::Multiple(vec![ConfigError::mismatch("u16", "x")]))
            .redacted();
        assert_eq!(err.to_string(), "1 configuration errors:\n  - expected u16, found \"***\"");
    }

    #[test]
    fn index_segments_render_without_dot() {
        let err = ConfigError::missing()
            .at_key("host")
            .at_index(2)
            .at_key("servers");
        assert_eq!(err.path(), "servers[2].host");
    }

    #[test]
    fn with_origin_keeps_first_origin() {
        let err = ConfigError::missing()
            .with_origin(Some(&Origin::new("a.toml", 1, 1)))
            .with_origin(Some(&Origin::new("b.toml", 2, 2)));
        assert_eq!(err.origin(), Some(&Origin::new("a.toml", 1, 1)));
    }

    #[test]
    fn debug_matches_display() {
        let err = ConfigError::mismatch("u16", "abc").at_key("port");
        assert_eq!(format!("{err:?}"), err.to_string());
    }

    #[test]
    fn warning_display() {
        let warning = Warning::new("old_key", "deprecated", Some(Origin::new("app.toml", 4, 1)));
        assert_eq!(warning.to_string(), "old_key: deprecated (app.toml:4:1)");
        assert_eq!(Warning::new("k", "m", None).to_string(), "k: m");
        assert_eq!((warning.path(), warning.message()), ("old_key", "deprecated"));
    }
}
