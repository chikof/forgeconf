//! Loader layer precedence and file handling tests.
#![cfg(feature = "toml")]
#![allow(clippy::unwrap_used, reason = "tests fail loudly")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use forgeconf::{
    Config,
    ConfigError,
    ConfigFile,
    ConfigNode,
    ConfigSource,
    ErrorKind,
    FileFormat,
    Loader,
    Table,
};

#[derive(Config, Debug, PartialEq)]
struct App {
    #[config(default = 1)]
    port: u16,
    #[config(default = "none")]
    database_url: String,
}

fn app(port: u16, database_url: &str) -> App {
    App { port, database_url: database_url.into() }
}

fn write(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents).unwrap();
    path
}

struct Fixed {
    priority: Option<u8>,
    node: ConfigNode,
}

impl Fixed {
    fn pairs(pairs: &[(&str, &str)]) -> Self {
        let table: Table = pairs
            .iter()
            .map(|(key, value)| ((*key).into(), ConfigNode::scalar(*value)))
            .collect();
        Self {
            priority: None,
            node: ConfigNode::table(table),
        }
    }

    fn null_port() -> Self {
        let mut table = Table::new();
        table.insert("port".into(), ConfigNode::null());
        Self {
            priority: None,
            node: ConfigNode::table(table),
        }
    }
}

impl ConfigSource for Fixed {
    fn load(&self) -> Result<ConfigNode, ConfigError> {
        Ok(self.node.clone())
    }

    fn priority(&self) -> u8 {
        self.priority.unwrap_or(100)
    }
}

#[test]
fn higher_priority_file_wins_per_key_and_lower_keys_survive() {
    let dir = tempfile::tempdir().unwrap();
    let high = write(dir.path(), "high.toml", "port = 5555\n");
    let loaded = App::loader()
        .file(ConfigFile::new(&high).priority(20))
        .file(ConfigFile::new("tests/fixtures/priority-base.toml"))
        .load()
        .unwrap();
    assert_eq!(loaded, app(5555, "postgres://base"));
}

#[test]
fn equal_priority_files_keep_insertion_order() {
    let dir = tempfile::tempdir().unwrap();
    let first = write(dir.path(), "first.toml", "port = 1\ndatabase_url = \"first\"\n");
    let second = write(dir.path(), "second.toml", "port = 2\n");
    let loaded = App::loader()
        .file(ConfigFile::new(&first))
        .file(ConfigFile::new(&second))
        .load()
        .unwrap();
    assert_eq!(loaded, app(2, "first"));
}

#[test]
fn custom_source_defaults_to_priority_100_and_beats_files() {
    let loaded = App::loader()
        .source(Fixed::pairs(&[("port", "7")]))
        .file(ConfigFile::new("tests/fixtures/priority-base.toml"))
        .load()
        .unwrap();
    assert_eq!(loaded, app(7, "postgres://base"));
}

#[test]
fn file_with_priority_above_100_beats_custom_source() {
    let loaded = App::loader()
        .source(Fixed::pairs(&[("port", "7")]))
        .file(ConfigFile::new("tests/fixtures/priority-base.toml").priority(150))
        .load()
        .unwrap();
    assert_eq!(loaded, app(3000, "postgres://base"));
}

#[test]
fn optional_missing_file_leaves_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = App::loader()
        .file(ConfigFile::new(dir.path().join("absent.toml")).optional())
        .load()
        .unwrap();
    assert_eq!(loaded, app(1, "none"));
}

#[test]
fn required_missing_file_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.toml");
    let err = App::loader()
        .file(ConfigFile::new(&path))
        .load()
        .unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::Io { .. }));
    let text = err.to_string();
    let prefix = format!("cannot read {}: ", path.display());
    assert!(text.starts_with(&prefix), "unexpected message: {text}");
}

#[test]
fn optional_file_still_reports_syntax_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "bad.toml", "port = = 1\n");
    let err = App::loader()
        .file(ConfigFile::new(&path).optional())
        .load()
        .unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::Syntax { format: "toml", .. }));
}

#[test]
fn explicit_null_source_keeps_the_lower_value() {
    let loaded = App::loader()
        .source(Fixed::null_port())
        .file(ConfigFile::new("tests/fixtures/priority-base.toml"))
        .load()
        .unwrap();
    assert_eq!(loaded, app(3000, "postgres://base"));
}

#[cfg(feature = "yaml")]
#[test]
fn yaml_null_in_a_higher_file_keeps_the_lower_value() {
    let dir = tempfile::tempdir().unwrap();
    let high = write(dir.path(), "high.yaml", "port: ~\ndatabase_url: from-yaml\n");
    let loaded = App::loader()
        .file(ConfigFile::new("tests/fixtures/priority-base.toml"))
        .file(ConfigFile::new(&high).priority(20))
        .load()
        .unwrap();
    assert_eq!(loaded, app(3000, "from-yaml"));
}

#[cfg(feature = "json")]
#[test]
fn json_null_in_a_higher_file_keeps_the_lower_value() {
    let dir = tempfile::tempdir().unwrap();
    let high = write(dir.path(), "high.json", "{\"port\": null, \"database_url\": \"from-json\"}");
    let loaded = App::loader()
        .file(ConfigFile::new("tests/fixtures/priority-base.toml"))
        .file(ConfigFile::new(&high).priority(20))
        .load()
        .unwrap();
    assert_eq!(loaded, app(3000, "from-json"));
}

#[test]
fn unknown_extension_cannot_be_inferred() {
    let err = App::loader()
        .file(ConfigFile::new("tests/fixtures/priority-override.cfg"))
        .load()
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "unsupported: cannot infer the format of tests/fixtures/priority-override.cfg; set it \
         with ConfigFile::format"
    );
}

#[test]
fn explicit_format_overrides_the_extension() {
    let loaded = App::loader()
        .file(ConfigFile::new("tests/fixtures/priority-override.cfg").format(FileFormat::Toml))
        .load()
        .unwrap();
    assert_eq!(loaded, app(5555, "postgres://override"));
}

#[cfg(not(feature = "yaml"))]
#[test]
fn disabled_format_extension_names_the_feature() {
    let err = App::loader()
        .file(ConfigFile::new("tests/fixtures/yaml-config.yaml"))
        .load()
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "unsupported: format yaml is disabled; enable the forgeconf `yaml` feature"
    );
}

#[derive(Config, Debug, PartialEq)]
struct Legacy {
    #[config(deprecated = "use port")]
    old_port: Option<u16>,
    #[config(alias = "legacy_name", deprecated)]
    name: Option<String>,
    #[config(default = 1)]
    port: u16,
}

fn warnings_of(path: &Path) -> (Legacy, Vec<String>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let loaded = Legacy::loader()
        .file(ConfigFile::new(path))
        .on_warning(move |warning| sink.lock().unwrap().push(warning.to_string()))
        .load()
        .unwrap();
    let warnings = seen.lock().unwrap().clone();
    (loaded, warnings)
}

#[test]
fn deprecated_keys_are_reported_through_on_warning() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "legacy.toml", "old_port = 80\nlegacy_name = \"n\"\n");
    let (loaded, warnings) = warnings_of(&path);
    assert_eq!(
        loaded,
        Legacy {
            old_port: Some(80),
            name: Some("n".into()),
            port: 1
        }
    );
    let p = path.display();
    assert_eq!(
        warnings,
        [
            format!("old_port: use port ({p}:1:12)"),
            format!("legacy_name: is deprecated ({p}:2:15)"),
        ]
    );
}

#[test]
fn absent_deprecated_keys_stay_silent() {
    let dir = tempfile::tempdir().unwrap();
    let path = write(dir.path(), "modern.toml", "port = 2\n");
    let (loaded, warnings) = warnings_of(&path);
    assert_eq!(loaded, Legacy { old_port: None, name: None, port: 2 });
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}

#[derive(Config, Debug, PartialEq)]
#[config(file = "tests/fixtures/basic.toml")]
struct Shorthand {
    port: u16,
    database_url: String,
}

#[test]
fn declared_shorthand_file_is_loaded_relative_to_the_crate() {
    assert_eq!(
        Shorthand::loader().load().unwrap(),
        Shorthand {
            port: 3000,
            database_url: "postgres://localhost/app".into()
        }
    );
}

#[derive(Config, Debug, PartialEq)]
#[config(
    file(path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/priority-base.toml")),
    file(path = "tests/fixtures/never-created.toml", optional, priority = 50),
    file(path = "tests/fixtures/priority-override.cfg", format = "toml", priority = 20)
)]
struct Declared {
    port: u16,
    database_url: String,
}

#[test]
fn declared_files_use_expr_paths_options_and_priority() {
    assert_eq!(
        Declared::loader().load().unwrap(),
        Declared {
            port: 5555,
            database_url: "postgres://override".into()
        }
    );
}

#[test]
fn added_sources_override_declared_files() {
    let loaded = Declared::loader()
        .source(Fixed::pairs(&[("port", "1")]))
        .load()
        .unwrap();
    assert_eq!(
        loaded,
        Declared {
            port: 1,
            database_url: "postgres://override".into()
        }
    );
}

#[test]
fn loader_new_has_no_sources() {
    assert_eq!(Loader::<App>::new().load().unwrap(), app(1, "none"));
    let err = Loader::<Shorthand>::new().load().unwrap_err();
    assert_eq!(
        err.to_string(),
        "2 configuration errors:\n  - port: missing value\n  - database_url: missing value"
    );
}
