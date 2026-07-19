use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

/// Where a configuration value came from.
///
/// Lines and columns are 1-based. Line `0` means the source has no position
/// (environment variables, command-line flags).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    source: Arc<str>,
    line: u32,
    column: u32,
}

const _: () = assert!(size_of::<Origin>() == 24);

impl Origin {
    /// Create an origin for `source` at `line`:`column`.
    pub fn new(source: impl Into<Arc<str>>, line: u32, column: u32) -> Self {
        Self { source: source.into(), line, column }
    }

    /// File path or source label.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// 1-based line, or `0` when the source has no position.
    #[must_use]
    pub fn line(&self) -> u32 {
        self.line
    }

    /// 1-based column, or `0` when the source has no position.
    #[must_use]
    pub fn column(&self) -> u32 {
        self.column
    }
}

impl Display for Origin {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.source)
        } else {
            write!(f, "{}:{}:{}", self.source, self.line, self.column)
        }
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;

    #[test]
    fn display_with_position() {
        assert_eq!(Origin::new("app.toml", 3, 8).to_string(), "app.toml:3:8");
    }

    #[test]
    fn display_without_position() {
        assert_eq!(Origin::new("env", 0, 0).to_string(), "env");
    }

    #[test]
    fn getters_return_fields() {
        let origin = Origin::new("a.json", 2, 5);
        assert_eq!((origin.source(), origin.line(), origin.column()), ("a.json", 2, 5));
    }
}
