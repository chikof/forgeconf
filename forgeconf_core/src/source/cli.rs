use std::collections::BTreeMap;
use std::env;

use super::ConfigSource;
use crate::{ConfigError, ConfigNode};

/// Pulls overrides from `std::env::args`.
pub struct CliArguments {
    priority: u8,
    args: Option<Vec<String>>,
}

impl CliArguments {
    pub fn new() -> Self {
        Self { priority: u8::MAX, args: None }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Provide a fixed set of arguments; useful for tests.
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args = Some(
            args.into_iter()
                .map(Into::into)
                .collect(),
        );
        self
    }

    fn args(&self) -> Vec<String> {
        if let Some(custom) = &self.args {
            custom.clone()
        } else {
            env::args()
                .skip(1)
                .collect()
        }
    }
}

impl Default for CliArguments {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigSource for CliArguments {
    fn priority(&self) -> u8 {
        self.priority
    }

    fn load(&self) -> Result<ConfigNode, ConfigError> {
        let mut tree = BTreeMap::new();
        for arg in self.args() {
            if let Some((key, value)) = parse_flag(&arg) {
                insert_path(&mut tree, key, value);
            }
        }

        Ok(ConfigNode::Table(tree))
    }
}

fn parse_flag(arg: &str) -> Option<(&str, &str)> {
    let stripped = arg.strip_prefix("--")?;
    let mut split = stripped.splitn(2, '=');
    let key = split
        .next()?
        .trim();
    let value = split
        .next()?
        .trim();

    if key.is_empty() { None } else { Some((key, value)) }
}

fn insert_path(tree: &mut BTreeMap<String, ConfigNode>, path: &str, value: &str) {
    let segments: Vec<&str> = path
        .split('.')
        .filter(|segment| !segment.is_empty())
        .collect();

    if segments.is_empty() {
        return;
    }

    insert_segments(tree, &segments, value);
}

fn insert_segments(tree: &mut BTreeMap<String, ConfigNode>, segments: &[&str], value: &str) {
    if let Some((head, tail)) = segments.split_first() {
        if tail.is_empty() {
            tree.insert((*head).to_string(), ConfigNode::Scalar(value.to_owned()));
        } else {
            let branch = tree
                .entry((*head).to_string())
                .or_insert_with(ConfigNode::empty_table);
            let map = as_table(branch);
            insert_segments(map, tail, value);
        }
    }
}

fn as_table(node: &mut ConfigNode) -> &mut BTreeMap<String, ConfigNode> {
    if !matches!(node, ConfigNode::Table(_)) {
        *node = ConfigNode::empty_table();
    }

    match node {
        ConfigNode::Table(map) => map,
        _ => unreachable!("ensured above"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_path_builds_nested_tables() {
        let mut tree = BTreeMap::new();
        insert_path(&mut tree, "db.host", "localhost");
        insert_path(&mut tree, "db.port", "5432");

        let db = tree
            .get("db")
            .unwrap();
        let table = db
            .as_table()
            .unwrap();
        assert_eq!(
            table
                .get("host")
                .unwrap()
                .to_string(),
            "localhost"
        );
        assert_eq!(
            table
                .get("port")
                .unwrap()
                .to_string(),
            "5432"
        );
    }

    #[test]
    fn cli_arguments_can_override_values() {
        let cli = CliArguments::new().with_args(["--server.port=9000"]);
        let node = cli
            .load()
            .unwrap();
        let table = node
            .as_table()
            .unwrap();
        let server = table
            .get("server")
            .unwrap()
            .as_table()
            .unwrap();
        assert_eq!(
            server
                .get("port")
                .unwrap()
                .to_string(),
            "9000"
        );
    }
}
