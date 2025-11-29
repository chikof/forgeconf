use std::io;

use thiserror::Error;

/// Unified error returned by the runtime components.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The format chosen by the caller or inferred from the extension is not
    /// supported.
    #[error("unsupported file format '{0}'")]
    UnsupportedFormat(String),

    /// File paths without an extension cannot be parsed automatically.
    #[error("configuration file is missing an extension")]
    MissingExtension,

    /// Raised when a required value is not found during deserialization.
    #[error("missing value for '{0}'")]
    MissingValue(String),

    /// Raised when a value cannot be converted into the expected target type.
    #[error("type mismatch for '{field}': expected {expected}, found {found}")]
    TypeMismatch {
        field: String,
        expected: String,
        found: String,
    },

    /// Used to thread contextual errors when deserializing nested structs.
    #[error("nested section '{section}' failed: {source}")]
    Nested {
        section: String,
        #[source]
        source: Box<ConfigError>,
    },

    /// IO errors propagated from the filesystem.
    #[error(transparent)]
    Io(#[from] io::Error),

    /// TOML parsing failure.
    #[cfg(feature = "toml")]
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),

    /// JSON parsing failure.
    #[cfg(feature = "json")]
    #[error("json parse error: {0}")]
    Json(#[from] jzon::Error),

    /// YAML parsing failure.
    #[cfg(feature = "yaml")]
    #[error("yaml parse error: {0}")]
    Yaml(#[from] yaml_rust2::ScanError),
}

impl ConfigError {
    /// Helper to produce a `TypeMismatch` error.
    pub fn mismatch(
        field: impl Into<String>,
        expected: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        ConfigError::TypeMismatch {
            field: field.into(),
            expected: expected.into(),
            found: found.into(),
        }
    }

    /// Helper to wrap nested errors with additional context.
    pub fn nested(section: impl Into<String>, source: ConfigError) -> Self {
        ConfigError::Nested {
            section: section.into(),
            source: Box::new(source),
        }
    }

    /// Helper to surface missing values.
    pub fn missing(key: impl Into<String>) -> Self {
        ConfigError::MissingValue(key.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatch_helper_builds_type_mismatch() {
        let err = ConfigError::mismatch("port", "u16", "string");
        assert!(matches!(
            err,
            ConfigError::TypeMismatch {
                field,
                expected,
                found
            } if field == "port" && expected == "u16" && found == "string"
        ));
    }

    #[test]
    fn nested_helper_wraps_inner_error() {
        let inner = ConfigError::missing("host");
        let err = ConfigError::nested("server", inner);
        assert!(matches!(
            err,
            ConfigError::Nested { section, source }
                if section == "server" && matches!(*source, ConfigError::MissingValue(ref key) if key == "host")
        ));
    }

    #[test]
    fn missing_helper_sets_field_name() {
        let err = ConfigError::missing("database");
        assert!(matches!(err, ConfigError::MissingValue(ref key) if key == "database"));
    }
}
