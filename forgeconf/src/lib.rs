//! Forgeconf - ergonomic configuration loading for Rust structs.
//!
//! `#[derive(Config)]` describes a configuration struct. [`Loader`] merges
//! files and other sources in a predictable priority order and converts the
//! result, reporting every bad field at once with its path and origin.
//!
//! # Example
//! ```
//! # #[cfg(feature = "toml")]
//! # fn main() -> Result<(), forgeconf::ConfigError> {
//! use forgeconf::Config;
//!
//! #[derive(Config)]
//! #[config(file = "examples/fixtures/basic.toml")]
//! struct AppConfig {
//!     host: String,
//!     port: u16,
//!     #[config(default = 4)]
//!     workers: u8,
//! }
//!
//! let cfg = AppConfig::loader().load()?;
//! assert_eq!(cfg.host, "localhost");
//! assert_eq!(cfg.port, 8080);
//! assert_eq!(cfg.workers, 4); // not in the file: the default applies
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "toml"))]
//! # fn main() {}
//! ```
//!
//! # Features
//!
//! `default = ["toml", "cli"]`. Parsers: `toml`, `json`, `yaml`. Sources:
//! `cli`, `dotenv`. `watch` reloads on file changes. The crate has no
//! third-party runtime dependency besides its own proc-macro crate.

extern crate self as forgeconf;

#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;
mod duration;
mod error;
mod format;
mod from_node;
mod loader;
mod merge;
mod node;
mod origin;
mod shape;
mod source;
mod suggest;
pub mod validators;

pub use error::{ConfigError, ErrorKind, PathSegment, Warning};
/// Derive macro for configuration types, next to the [`Config`] trait it
/// implements.
///
/// The helper attribute is `config`. Struct attributes: `file`, `env_prefix`,
/// `rename_all`, `deny_unknown_fields`, `validate`. Field attributes: `rename`,
/// `alias`, `default`, `env`, `flag`, `short`, `skip_cli`, `insensitive`,
/// `validate`, `sensitive`, `deprecated`, `skip`, `flatten`. Enums take
/// `rename_all` and `tag`; variants take `rename` and `alias`. See the
/// [`forgeconf_macros`](https://docs.rs/forgeconf_macros) crate docs for the
/// full reference.
///
/// Generic types and lifetimes are rejected:
///
/// ```compile_fail
/// use forgeconf::Config;
/// #[derive(Config)]
/// struct Wrapper<T> {
///     value: T,
/// }
/// ```
///
/// So is `deny_unknown_fields` together with a `flatten` field:
///
/// ```compile_fail
/// use forgeconf::Config;
/// #[derive(Config)]
/// struct Inner {
///     a: u8,
/// }
/// #[derive(Config)]
/// #[config(deny_unknown_fields)]
/// struct Strict {
///     #[config(flatten)]
///     inner: Inner,
/// }
/// ```
pub use forgeconf_macros::Config;
pub use format::{FileFormat, from_str, parse_str};
pub use from_node::FromNode;
pub use loader::{Config, Loader};
pub use node::{ConfigNode, Table, Value};
pub use origin::Origin;
pub use shape::{EnumInfo, FieldInfo, Shape, StructInfo, VariantInfo};
pub use source::{ConfigFile, ConfigSource};
