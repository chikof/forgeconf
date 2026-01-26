//! Runtime primitives exposed to the macro-generated code.

mod error;
mod node;
mod parser;
mod source;
#[cfg(feature = "validators")]
pub mod validators;

pub use error::ConfigError;
pub use node::{ConfigNode, FromNode};
#[cfg(all(feature = "parse", feature = "json"))]
pub use parser::parse_json;
#[cfg(feature = "parse")]
pub use parser::parse_str;
#[cfg(all(feature = "parse", feature = "toml"))]
pub use parser::parse_toml;
#[cfg(all(feature = "parse", feature = "yaml"))]
pub use parser::parse_yaml;
pub use parser::{load_from_path, FileFormat};
#[cfg(feature = "cli")]
pub use source::CliArguments;
pub use source::{merge_nodes, ConfigBuilder, ConfigFile, ConfigSource};
