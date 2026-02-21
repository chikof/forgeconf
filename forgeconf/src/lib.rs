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
//!     unsafe { std::env::set_var("APP_DATABASE_URL", "postgres://override"); };
//!     let cfg = AppConfig::loader()
//!         .with_config() // load every `config(...)` entry
//!         .load()?;
//!     println!("listening on {}", cfg.port);
//!     println!("db url: {}", cfg.database_url);
//!     Ok(())
//! }
//! ```

#[cfg(feature = "cli")]
pub use forgeconf_core::CliArguments;
#[cfg(all(feature = "parse", feature = "json"))]
pub use forgeconf_core::parse_json;
#[cfg(feature = "parse")]
pub use forgeconf_core::parse_str;
#[cfg(all(feature = "parse", feature = "toml"))]
pub use forgeconf_core::parse_toml;
#[cfg(all(feature = "parse", feature = "yaml"))]
pub use forgeconf_core::parse_yaml;
#[cfg(feature = "validators")]
pub use forgeconf_core::validators;
pub use forgeconf_core::{
    ConfigBuilder,
    ConfigError,
    ConfigFile,
    ConfigNode,
    ConfigSource,
    FileFormat,
    FromNode,
    load_from_path,
    merge_nodes,
};
pub use forgeconf_macros::forgeconf;
