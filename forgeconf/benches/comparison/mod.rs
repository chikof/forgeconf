/// Module declarations for benchmark implementations
pub mod common;
pub mod config_impl;
pub mod forgeconf_impl;

// Re-export common traits and types
pub use common::{ComplexConfig, ConfigResult, NestedConfig, SimpleConfig};
