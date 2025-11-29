use crate::{ConfigError, ConfigNode};

mod cli;
mod file;

pub use cli::CliArguments;
pub use file::ConfigFile;

/// Trait implemented by configuration sources (files, CLI, etc).
pub trait ConfigSource: Send + Sync {
    /// Higher priority sources override lower priority ones.
    fn priority(&self) -> u8 {
        0
    }

    /// Load configuration data from the source.
    fn load(&self) -> Result<ConfigNode, ConfigError>;
}

/// Combine two configuration trees, where values from `overlay` take
/// precedence.
pub fn merge_nodes(base: ConfigNode, overlay: ConfigNode) -> ConfigNode {
    match (base, overlay) {
        (ConfigNode::Table(mut left), ConfigNode::Table(right)) => {
            for (key, value) in right {
                match left.remove(&key) {
                    Some(existing) => {
                        let merged = merge_nodes(existing, value);
                        left.insert(key, merged);
                    },
                    None => {
                        left.insert(key, value);
                    },
                }
            }
            ConfigNode::Table(left)
        },
        (_, other) => other,
    }
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
        self.sources
            .push(Box::new(source));
        self
    }

    pub fn load(mut self) -> Result<ConfigNode, ConfigError> {
        self.sources
            .sort_by_key(|source| source.priority());

        let mut merged = ConfigNode::empty_table();
        for source in self.sources {
            let value = source.load()?;
            merged = merge_nodes(merged, value);
        }

        Ok(merged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_nodes_prefers_overlay() {
        use std::collections::BTreeMap;

        let mut left = BTreeMap::new();
        left.insert("port".into(), ConfigNode::Scalar("8080".into()));

        let mut right = BTreeMap::new();
        right.insert("port".into(), ConfigNode::Scalar("9090".into()));
        right.insert("host".into(), ConfigNode::Scalar("0.0.0.0".into()));

        let merged = merge_nodes(ConfigNode::Table(left), ConfigNode::Table(right));
        let table = merged
            .as_table()
            .unwrap();
        assert_eq!(
            table
                .get("port")
                .unwrap()
                .to_string(),
            "9090"
        );
        assert!(table.contains_key("host"));
    }
}
