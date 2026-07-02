use std::io;
use std::path::{Path, PathBuf};

use super::ConfigSource;
use crate::format::load_from_path;
use crate::{ConfigError, ConfigNode, ErrorKind, FileFormat, Table};

/// Source backed by a configuration file on disk.
///
/// The format is inferred from the extension unless [`format`](Self::format)
/// is set. Files default to priority 10 and are required.
#[derive(Clone, Debug)]
pub struct ConfigFile {
    path: PathBuf,
    format: Option<FileFormat>,
    priority: u8,
    optional: bool,
}

impl ConfigFile {
    /// A required file at `path` with priority 10.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            format: None,
            priority: 10,
            optional: false,
        }
    }

    /// Parse the file as `format` instead of inferring it from the extension.
    #[must_use]
    pub fn format(mut self, format: FileFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Override the merge priority; higher wins.
    #[must_use]
    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Treat a missing file as an empty table instead of an error.
    #[must_use]
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

impl ConfigSource for ConfigFile {
    fn load(&self) -> Result<ConfigNode, ConfigError> {
        match load_from_path(&self.path, self.format) {
            Err(err) if self.optional && is_not_found(&err) => Ok(ConfigNode::table(Table::new())),
            result => result,
        }
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn watch_path(&self) -> Option<&Path> {
        Some(&self.path)
    }
}

fn is_not_found(err: &ConfigError) -> bool {
    matches!(err.kind(), ErrorKind::Io { source, .. } if source.kind() == io::ErrorKind::NotFound)
}

#[cfg(all(test, feature = "toml"))]
mod test {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn optional_missing_file_is_empty_table() {
        let node = ConfigFile::new("/definitely/not/here.toml")
            .optional()
            .load()
            .unwrap();
        assert_eq!(node, ConfigNode::table(Table::new()));
        assert!(node.origin().is_none());
    }

    #[test]
    fn required_missing_file_is_io_error() {
        let err = ConfigFile::new("/definitely/not/here.toml")
            .load()
            .unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Io { .. }));
    }

    #[test]
    fn optional_file_with_syntax_error_still_fails() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("bad.toml");
        fs::write(&path, "port = = 1").unwrap();
        let err = ConfigFile::new(&path).optional().load().unwrap_err();
        assert!(matches!(err.kind(), ErrorKind::Syntax { .. }));
    }

    #[test]
    fn builders_set_priority_and_watch_path() {
        let file = ConfigFile::new("a.toml")
            .priority(42)
            .format(FileFormat::Toml);
        assert_eq!(ConfigSource::priority(&file), 42);
        assert_eq!(file.watch_path(), Some(Path::new("a.toml")));
        assert_eq!(ConfigSource::priority(&ConfigFile::new("a.toml")), 10);
    }
}
