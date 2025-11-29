//! Forgeconf - ergonomic configuration loading for Rust structs.
//!
//! The `#[forgeconf]` attribute builds a dedicated loader that merges
//! configuration files, CLI flags, and environment variables in a predictable
//! order while checking type safety at compile time.
//!
//! # Example
//! ```
//! use forgeconf::{forgeconf, ConfigError};
//!
//! #[forgeconf(config(path = "tests/fixtures/basic.toml"))]
//! struct AppConfig {
//!     #[field(default = 8080)]
//!     port: u16,
//!     #[field(env = "APP_DATABASE_URL")]
//!     database_url: String,
//! }
//!
//! fn main() -> Result<(), ConfigError> {
//!     std::env::set_var("APP_DATABASE_URL", "postgres://override");
//!     let cfg = AppConfig::loader()
//!         .with_config() // load every `config(...)` entry
//!         .load()?;
//!     println!("listening on {}", cfg.port);
//!     println!("db url: {}", cfg.database_url);
//!     Ok(())
//! }
//! ```

pub use forgeconf_core::{
    merge_nodes,
    CliArguments,
    ConfigBuilder,
    ConfigError,
    ConfigFile,
    ConfigNode,
    ConfigSource,
    FileFormat,
    FromNode,
};
pub use forgeconf_macros::forgeconf;
