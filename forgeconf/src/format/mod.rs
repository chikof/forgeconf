use std::path::Path;

use crate::{ConfigError, ConfigNode, ErrorKind, FromNode};

#[allow(dead_code, reason = "used by in-house parsers from Phase 3")]
mod cursor;
#[cfg(feature = "json")]
mod json;
#[cfg(feature = "toml")]
mod toml;
#[cfg(feature = "yaml")]
mod yaml;

/// Supported on-disk formats. A variant exists only when its Cargo feature is
/// enabled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FileFormat {
    /// TOML, behind the `toml` feature.
    #[cfg(feature = "toml")]
    Toml,
    /// JSON, behind the `json` feature.
    #[cfg(feature = "json")]
    Json,
    /// YAML, behind the `yaml` feature.
    #[cfg(feature = "yaml")]
    Yaml,
}

impl FileFormat {
    /// The format for a file extension (case-insensitive, without the dot).
    /// `None` for unknown extensions and for formats whose feature is off.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            #[cfg(feature = "toml")]
            "toml" => Some(Self::Toml),
            #[cfg(feature = "json")]
            "json" => Some(Self::Json),
            #[cfg(feature = "yaml")]
            "yaml" | "yml" => Some(Self::Yaml),
            _ => None,
        }
    }

    /// Lower-case identifier.
    pub fn label(self) -> &'static str {
        match self {
            #[cfg(feature = "toml")]
            Self::Toml => "toml",
            #[cfg(feature = "json")]
            Self::Json => "json",
            #[cfg(feature = "yaml")]
            Self::Yaml => "yaml",
        }
    }
}

/// Read and parse a configuration file into a [`ConfigNode`].
///
/// The format is `explicit` when given, otherwise inferred from the extension.
pub fn load_from_path(
    path: impl AsRef<Path>,
    explicit: Option<FileFormat>,
) -> Result<ConfigNode, ConfigError> {
    let path = path.as_ref();
    let format = match explicit {
        Some(format) => format,
        None => infer_from_extension(path)?,
    };
    let contents = std::fs::read_to_string(path)
        .map_err(|source| ConfigError::new(ErrorKind::Io { path: path.to_path_buf(), source }))?;
    parse_str(&contents, format)
}

/// Parse an in-memory string.
#[cfg_attr(
    not(any(feature = "toml", feature = "json", feature = "yaml")),
    allow(unused_variables, reason = "no parser is compiled in")
)]
pub fn parse_str(input: &str, format: FileFormat) -> Result<ConfigNode, ConfigError> {
    // Selecting a disabled format is impossible: its variant does not exist.
    match format {
        #[cfg(feature = "toml")]
        FileFormat::Toml => toml::parse(input),
        #[cfg(feature = "json")]
        FileFormat::Json => json::parse(input),
        #[cfg(feature = "yaml")]
        FileFormat::Yaml => yaml::parse(input),
    }
}

/// Parse `input` and convert it into `T`.
pub fn from_str<T: FromNode>(input: &str, format: FileFormat) -> Result<T, ConfigError> {
    T::from_node(&parse_str(input, format)?)
}

fn infer_from_extension(path: &Path) -> Result<FileFormat, ConfigError> {
    let ext = path.extension().and_then(|ext| ext.to_str());
    if let Some(format) = ext.and_then(FileFormat::from_extension) {
        return Ok(format);
    }
    let message = match ext.map(str::to_ascii_lowercase).as_deref() {
        Some(disabled @ ("toml" | "json" | "yaml" | "yml")) => {
            let name = if disabled == "yml" { "yaml" } else { disabled };
            format!("format {name} is disabled; enable the forgeconf `{name}` feature")
        },
        _ => {
            format!("cannot infer the format of {}; set it with ConfigFile::format", path.display())
        },
    };
    Err(ConfigError::new(ErrorKind::Unsupported(message)))
}

#[cfg(test)]
mod test {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn unknown_extension_cannot_be_inferred() {
        let err = infer_from_extension(Path::new("conf.cfg")).unwrap_err();
        assert_eq!(
            err.to_string(),
            "unsupported: cannot infer the format of conf.cfg; set it with ConfigFile::format"
        );
    }

    #[test]
    fn missing_extension_cannot_be_inferred() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config");
        fs::write(&path, "port = 7000").unwrap();
        let err = load_from_path(&path, None).unwrap_err();
        assert!(
            matches!(err.kind(), ErrorKind::Unsupported(msg) if msg.starts_with("cannot infer"))
        );
    }

    #[cfg(not(feature = "yaml"))]
    #[test]
    fn disabled_format_is_reported() {
        let err = infer_from_extension(Path::new("a.yml")).unwrap_err();
        assert_eq!(
            err.to_string(),
            "unsupported: format yaml is disabled; enable the forgeconf `yaml` feature"
        );
        assert_eq!(FileFormat::from_extension("yaml"), None);
    }

    #[cfg(feature = "toml")]
    mod toml_enabled {
        use super::*;

        #[test]
        fn extension_maps_to_format() {
            assert_eq!(FileFormat::from_extension("TOML"), Some(FileFormat::Toml));
            assert_eq!(FileFormat::Toml.label(), "toml");
            assert_eq!(FileFormat::from_extension("txt"), None);
        }

        #[test]
        fn explicit_format_overrides_extension() {
            let dir = tempdir().unwrap();
            let path = dir.path().join("override.cfg");
            fs::write(&path, "port = 6100").unwrap();

            let node = load_from_path(&path, Some(FileFormat::Toml)).unwrap();
            assert_eq!(node.get("port").and_then(ConfigNode::as_str), Some("6100"));
        }

        #[test]
        fn missing_file_reports_path() {
            let err = load_from_path("/definitely/not/here.toml", None).unwrap_err();
            assert!(
                matches!(err.kind(), ErrorKind::Io { path, .. } if path.ends_with("here.toml"))
            );
            assert!(
                err.to_string()
                    .starts_with("cannot read /definitely/not/here.toml: ")
            );
        }

        #[test]
        fn from_str_converts_into_target() {
            let node: ConfigNode = from_str("port = 80", FileFormat::Toml).unwrap();
            assert_eq!(node.get("port").and_then(ConfigNode::as_str), Some("80"));
        }
    }
}
