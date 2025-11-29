use std::path::Path;
use std::str::FromStr;

use crate::{ConfigError, ConfigNode};

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "yaml")]
mod yaml;

/// Supported on-disk formats.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileFormat {
    Toml,
    Yaml,
    Json,
}

impl FileFormat {
    /// Returns the lower-case identifier.
    pub fn label(&self) -> &'static str {
        match self {
            FileFormat::Toml => "toml",
            FileFormat::Yaml => "yaml",
            FileFormat::Json => "json",
        }
    }

    /// All recognised extensions for the format.
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            FileFormat::Toml => &["toml"],
            FileFormat::Yaml => &["yaml", "yml"],
            FileFormat::Json => &["json"],
        }
    }
}

impl FromStr for FileFormat {
    type Err = ConfigError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let lower = s.to_lowercase();
        match lower.as_str() {
            "toml" => Ok(FileFormat::Toml),
            "yaml" | "yml" => Ok(FileFormat::Yaml),
            "json" => Ok(FileFormat::Json),
            other => Err(ConfigError::UnsupportedFormat(other.into())),
        }
    }
}

/// Parse a configuration file into a [`ConfigNode`].
pub fn load_from_path(
    path: impl AsRef<Path>,
    explicit: Option<FileFormat>,
) -> Result<ConfigNode, ConfigError> {
    let path = path.as_ref();
    let format = match explicit {
        Some(fmt) => fmt,
        None => infer_from_extension(path)?,
    };

    if !format_supported(format) {
        return Err(ConfigError::UnsupportedFormat(
            format
                .label()
                .into(),
        ));
    }

    let contents = std::fs::read_to_string(path)?;
    parse_str(&contents, format)
}

/// Parse an in-memory string.
pub fn parse_str(input: &str, format: FileFormat) -> Result<ConfigNode, ConfigError> {
    match format {
        FileFormat::Toml => {
            #[cfg(feature = "toml")]
            {
                toml::parse(input)
            }
            #[cfg(not(feature = "toml"))]
            {
                Err(ConfigError::UnsupportedFormat("toml (feature disabled)".into()))
            }
        },
        FileFormat::Yaml => {
            #[cfg(feature = "yaml")]
            {
                yaml::parse(input)
            }
            #[cfg(not(feature = "yaml"))]
            {
                Err(ConfigError::UnsupportedFormat("yaml (feature disabled)".into()))
            }
        },
        FileFormat::Json => {
            #[cfg(feature = "json")]
            {
                json::parse(input)
            }
            #[cfg(not(feature = "json"))]
            {
                Err(ConfigError::UnsupportedFormat("json (feature disabled)".into()))
            }
        },
    }
}

fn infer_from_extension(path: &Path) -> Result<FileFormat, ConfigError> {
    let Some(ext) = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|s| s.to_lowercase())
    else {
        return Err(ConfigError::MissingExtension);
    };

    ext.parse()
}

fn format_supported(format: FileFormat) -> bool {
    match format {
        FileFormat::Toml => cfg!(feature = "toml"),
        FileFormat::Yaml => cfg!(feature = "yaml"),
        FileFormat::Json => cfg!(feature = "json"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn load_from_path_requires_extension() {
        let dir = tempdir().unwrap();
        let path = dir
            .path()
            .join("config");
        fs::write(&path, "port = 7000").unwrap();

        let err = load_from_path(&path, None).unwrap_err();
        assert!(matches!(err, ConfigError::MissingExtension));
    }

    #[test]
    fn load_from_path_uses_explicit_format() {
        let dir = tempdir().unwrap();
        let path = dir
            .path()
            .join("override.cfg");
        fs::write(&path, "port = 6100").unwrap();

        let node = load_from_path(&path, Some(FileFormat::Toml)).unwrap();
        let table = node
            .as_table()
            .unwrap();
        assert_eq!(
            table
                .get("port")
                .unwrap()
                .to_string(),
            "6100"
        );
    }
}
