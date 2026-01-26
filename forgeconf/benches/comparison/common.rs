/// Common traits and types shared across all benchmark implementations
use std::path::Path;

/// Result type for configuration loading operations
pub type ConfigResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Trait for simple configuration structures (flat config)
pub trait SimpleConfig: Sized {
    fn from_file(path: &Path) -> ConfigResult<Self>;
    fn from_str(content: &str) -> ConfigResult<Self>;
}

/// Trait for nested configuration structures (multi-level config)
pub trait NestedConfig: Sized {
    fn from_file(path: &Path) -> ConfigResult<Self>;
    fn from_str(content: &str) -> ConfigResult<Self>;
}

/// Trait for complex configuration structures (deep nesting with arrays)
pub trait ComplexConfig: Sized {
    fn from_file(path: &Path) -> ConfigResult<Self>;
    fn from_str(content: &str) -> ConfigResult<Self>;
}

/// Fixture paths
pub mod fixtures {
    pub const SIMPLE: &str = "benches/fixtures/simple.toml";
    pub const NESTED: &str = "benches/fixtures/nested.toml";
    pub const COMPLEX: &str = "benches/fixtures/complex.toml";
}
