//! Runtime primitives exposed to the macro-generated code.

mod error;
mod node;
mod parser;
mod source;

pub use error::ConfigError;
pub use node::{ConfigNode, FromNode};
pub use parser::{load_from_path, FileFormat};
pub use source::{merge_nodes, CliArguments, ConfigBuilder, ConfigFile, ConfigSource};
