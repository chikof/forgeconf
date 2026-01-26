//! Error types for forgeconf.
//!
//! These errors use `miette::Diagnostic` for beautiful error reporting with
//! source code snippets.

#![allow(unused_assignments)] // Suppress false positives from miette's Diagnostic derive macro

use std::io;

use miette::{Diagnostic, SourceSpan};

/// Unified error returned by the runtime components.
#[derive(Diagnostic, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// The format chosen by the caller or inferred from the extension is not
    /// supported.
    #[error("unsupported file format '{0}'")]
    #[diagnostic(
        code(forgeconf::unsupported_format),
        help(
            "Forgeconf supports toml, yaml, and json formats. Make sure your file has the correct \
             extension."
        )
    )]
    UnsupportedFormat(String),

    /// File paths without an extension cannot be parsed automatically.
    #[error("configuration file is missing an extension")]
    #[diagnostic(
        code(forgeconf::missing_extension),
        help("Add a file extension like .toml, .yaml, or .json to your configuration file.")
    )]
    MissingExtension,

    /// Raised when a required value is not found during deserialization.
    #[error("missing required field '{field}'")]
    #[diagnostic(
        code(forgeconf::missing_field),
        help("Add the missing field '{field}' to your configuration file.")
    )]
    MissingValue {
        /// The field name that is missing
        field: String,
        /// Optional source span for where the error occurred
        #[label("expected '{field}' to be defined here")]
        span: Option<SourceSpan>,
    },

    /// Raised when a value cannot be converted into the expected target type.
    #[error("type mismatch for field '{field}'")]
    #[diagnostic(
        code(forgeconf::type_mismatch),
        help(
            "Expected a value of type '{expected}', but found '{found}'. Check your configuration \
             syntax."
        )
    )]
    TypeMismatch {
        /// The field name that has the wrong type
        field: String,
        /// The expected type
        expected: String,
        /// The found value or type
        found: String,
        /// Optional source span highlighting the problematic value
        #[label("expected {expected}, but found {found}")]
        span: Option<SourceSpan>,
    },

    /// Used to thread contextual errors when deserializing nested structs.
    #[error("error in nested section '{section}'")]
    #[diagnostic(
        code(forgeconf::nested_error),
        help("Check the '{section}' section of your configuration for errors.")
    )]
    Nested {
        /// The section name that failed
        section: String,
        /// The underlying error
        #[source]
        #[diagnostic_source]
        source: Box<dyn Diagnostic + Send + Sync>,
        /// Optional source span for the section
        #[label("error in this section")]
        span: Option<SourceSpan>,
    },

    /// IO errors propagated from the filesystem.
    #[error(transparent)]
    #[diagnostic(
        code(forgeconf::io_error),
        help("Check that the file exists and you have permission to read it.")
    )]
    Io(#[from] io::Error),

    /// TOML parsing failure.
    #[cfg(feature = "toml")]
    #[error("failed to parse TOML")]
    #[diagnostic(
        code(forgeconf::toml_parse_error),
        help(
            "Check your TOML syntax. Common issues include unquoted strings, missing commas, or \
             invalid escape sequences."
        )
    )]
    Toml {
        /// The underlying toml error
        #[source]
        source: toml::de::Error,
        /// Optional source span for where the parse error occurred
        #[label("parse error here")]
        span: Option<SourceSpan>,
    },

    /// JSON parsing failure.
    #[cfg(feature = "json")]
    #[error("failed to parse JSON")]
    #[diagnostic(
        code(forgeconf::json_parse_error),
        help(
            "Check your JSON syntax. Common issues include trailing commas, unquoted keys, or \
             missing brackets."
        )
    )]
    Json {
        /// The underlying json error
        #[source]
        source: jzon::Error,
        /// Optional source span for where the parse error occurred
        #[label("parse error here")]
        span: Option<SourceSpan>,
    },

    /// YAML parsing failure.
    #[cfg(feature = "yaml")]
    #[error("failed to parse YAML")]
    #[diagnostic(
        code(forgeconf::yaml_parse_error),
        help(
            "Check your YAML syntax. Common issues include incorrect indentation, missing colons, \
             or invalid characters."
        )
    )]
    Yaml {
        /// The underlying yaml error
        #[source]
        source: yaml_rust2::ScanError,
        /// Optional source span for where the parse error occurred
        #[label("parse error here")]
        span: Option<SourceSpan>,
    },
}

// Custom Debug implementation that uses miette's fancy formatting when the
// feature is enabled
impl std::fmt::Debug for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[cfg(feature = "miette")]
        {
            // When miette feature is enabled, delegate to miette's Diagnostic formatting
            use miette::GraphicalReportHandler;

            let mut output = String::new();
            let handler = GraphicalReportHandler::new();
            if handler
                .render_report(&mut output, self)
                .is_ok()
            {
                write!(f, "{}", output)
            } else {
                // If fancy rendering fails, fall back to Display
                write!(f, "{}", self)
            }
        }

        #[cfg(not(feature = "miette"))]
        {
            // Without miette feature, fall back to simple debug formatting
            // Use the Display implementation from thiserror::Error
            write!(f, "{}", self)
        }
    }
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
            span: None,
        }
    }

    /// Helper to produce a `TypeMismatch` error with a source span.
    pub fn mismatch_at(
        field: impl Into<String>,
        expected: impl Into<String>,
        found: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        ConfigError::TypeMismatch {
            field: field.into(),
            expected: expected.into(),
            found: found.into(),
            span: Some(span),
        }
    }

    /// Helper to wrap nested errors with additional context.
    pub fn nested(section: impl Into<String>, source: ConfigError) -> Self {
        ConfigError::Nested {
            section: section.into(),
            source: Box::new(source),
            span: None,
        }
    }

    /// Helper to wrap nested errors with additional context and a source span.
    pub fn nested_at(section: impl Into<String>, source: ConfigError, span: SourceSpan) -> Self {
        ConfigError::Nested {
            section: section.into(),
            source: Box::new(source),
            span: Some(span),
        }
    }

    /// Helper to surface missing values.
    pub fn missing(key: impl Into<String>) -> Self {
        ConfigError::MissingValue { field: key.into(), span: None }
    }

    /// Helper to surface missing values with a source span.
    pub fn missing_at(key: impl Into<String>, span: SourceSpan) -> Self {
        ConfigError::MissingValue { field: key.into(), span: Some(span) }
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
                found,
                span: None,
            } if field == "port" && expected == "u16" && found == "string"
        ));
    }

    #[test]
    fn nested_helper_wraps_inner_error() {
        let inner = ConfigError::missing("host");
        let err = ConfigError::nested("server", inner);
        assert!(matches!(
            err,
            ConfigError::Nested {
                section,
                span: None,
                ..
            } if section == "server"
        ));
    }

    #[test]
    fn missing_helper_sets_field_name() {
        let err = ConfigError::missing("database");
        assert!(matches!(
            err,
            ConfigError::MissingValue { ref field, span: None }
                if field == "database"
        ));
    }
}
