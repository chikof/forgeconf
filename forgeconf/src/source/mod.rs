use std::path::Path;

use crate::{ConfigError, ConfigNode};

#[cfg(feature = "cli")]
pub(crate) mod cli;
#[cfg(feature = "dotenv")]
pub(crate) mod dotenv;
pub(crate) mod env;
mod file;
pub(crate) mod walk;

pub use file::ConfigFile;

/// A provider of configuration data, merged by [`Loader`](crate::Loader).
pub trait ConfigSource: Send + Sync {
    /// Read the source into a node tree.
    fn load(&self) -> Result<ConfigNode, ConfigError>;

    /// Higher priority sources override lower priority ones. Equal priorities
    /// keep insertion order, later wins.
    fn priority(&self) -> u8 {
        100
    }

    /// File the source reads from, if any. Used to watch for changes.
    fn watch_path(&self) -> Option<&Path> {
        None
    }
}
