use crate::merge::merge;
use crate::{ConfigError, ConfigNode, Table};

#[cfg(feature = "cli")]
mod cli;
#[cfg(feature = "dotenv")]
mod dotenv;
mod file;
#[cfg(feature = "watch")]
mod watch;

#[cfg(feature = "cli")]
pub use cli::{CliArgsSource, CliArguments};
#[cfg(feature = "dotenv")]
pub use dotenv::DotenvSource;
pub use file::ConfigFile;
#[cfg(feature = "watch")]
#[allow(unused_imports)]
pub use watch::WatchSource;

/// Trait implemented by configuration sources (files, CLI, etc).
pub trait ConfigSource: Send + Sync {
    /// Higher priority sources override lower priority ones.
    fn priority(&self) -> u8 {
        0
    }

    /// Load configuration data from the source.
    fn load(&self) -> Result<ConfigNode, ConfigError>;
}

/// Builder that merges a set of [`ConfigSource`] instances.
#[derive(Default)]
pub struct ConfigBuilder {
    sources: Vec<Box<dyn ConfigSource>>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_source<S>(mut self, source: S) -> Self
    where
        S: ConfigSource + 'static,
    {
        self.sources.push(Box::new(source));
        self
    }

    pub fn load(mut self) -> Result<ConfigNode, ConfigError> {
        self.sources.sort_by_key(|source| source.priority());

        let mut merged = ConfigNode::table(Table::new());
        for source in self.sources {
            merge(&mut merged, source.load()?);
        }

        Ok(merged)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Clone)]
    struct StaticSource {
        priority: u8,
        node: ConfigNode,
    }

    impl StaticSource {
        fn table(priority: u8, key: &str, value: &str) -> Self {
            let mut map = Table::new();
            map.insert(key.to_string(), ConfigNode::scalar(value));
            Self { priority, node: ConfigNode::table(map) }
        }
    }

    impl ConfigSource for StaticSource {
        fn priority(&self) -> u8 {
            self.priority
        }

        fn load(&self) -> Result<ConfigNode, ConfigError> {
            Ok(self.node.clone())
        }
    }

    #[test]
    fn config_builder_honors_source_priority() {
        let node = ConfigBuilder::new()
            .add_source(StaticSource::table(5, "service", "base"))
            .add_source(StaticSource::table(200, "service", "override"))
            .load()
            .unwrap();

        assert_eq!(node.get("service").and_then(ConfigNode::as_str), Some("override"));
    }

    #[test]
    fn config_builder_keeps_lower_sources_under_null_overlay() {
        let node = ConfigBuilder::new()
            .add_source(StaticSource::table(5, "service", "base"))
            .add_source(StaticSource { priority: 200, node: ConfigNode::null() })
            .load()
            .unwrap();

        assert_eq!(node.get("service").and_then(ConfigNode::as_str), Some("base"));
    }
}
