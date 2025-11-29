use std::path::{Path, PathBuf};

use super::ConfigSource;
use crate::parser::load_from_path;
use crate::{ConfigError, ConfigNode, FileFormat};

/// Source backed by a configuration file on disk.
pub struct ConfigFile {
    path: PathBuf,
    format: Option<FileFormat>,
    priority: u8,
}

impl ConfigFile {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            path: path
                .as_ref()
                .to_path_buf(),
            format: None,
            priority: 10,
        }
    }

    pub fn with_format(mut self, format: FileFormat) -> Self {
        self.format = Some(format);
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

impl ConfigSource for ConfigFile {
    fn priority(&self) -> u8 {
        self.priority
    }

    fn load(&self) -> Result<ConfigNode, ConfigError> {
        load_from_path(&self.path, self.format)
    }
}
