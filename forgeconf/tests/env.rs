//! Environment variable layer tests.
#![cfg(feature = "toml")]
#![allow(clippy::unwrap_used, reason = "tests fail loudly")]

use std::fs;

use forgeconf::{Config, ConfigError, ConfigFile, ConfigNode, ConfigSource, Table};

#[derive(Config, Debug, PartialEq)]
#[config(env_prefix = "APP_")]
struct Prefixed {
    #[config(default = 1)]
    port: u16,
    #[config(default = Server { port: 80, tls: false })]
    server: Server,
    #[config(default = Level::Info)]
    log_level: Level,
    #[config(default)]
    tags: Vec<String>,
    #[config(env = "CUSTOM_TOKEN", default = "none")]
    token: String,
    nickname: Option<String>,
}

#[derive(Config, Debug, PartialEq)]
#[config(env_prefix = "IGNORED_")]
struct Server {
    #[config(default = 80)]
    port: u16,
    #[config(default = false)]
    tls: bool,
}

#[derive(Config, Debug, PartialEq)]
#[config(rename_all = "lowercase")]
enum Level {
    Info,
    Debug,
}

fn load(vars: &[(&str, &str)]) -> Result<Prefixed, ConfigError> {
    Prefixed::loader().env_vars(vars.iter().copied()).load()
}

#[test]
fn prefix_and_path_give_the_variable_name() {
    let loaded =
        load(&[("APP_PORT", "9"), ("APP_SERVER_PORT", "8443"), ("APP_SERVER_TLS", "true")])
            .unwrap();
    assert_eq!(loaded.port, 9);
    assert_eq!(loaded.server, Server { port: 8443, tls: true });
}

#[test]
fn nested_struct_prefix_is_ignored() {
    let loaded = load(&[("IGNORED_SERVER_PORT", "1"), ("IGNORED_PORT", "2")]).unwrap();
    assert_eq!(loaded.server.port, 80);
    assert_eq!(loaded.port, 1);
}

#[test]
fn explicit_env_name_is_verbatim_without_prefix() {
    let loaded = load(&[("CUSTOM_TOKEN", "s3cret"), ("APP_CUSTOM_TOKEN", "wrong")]).unwrap();
    assert_eq!(loaded.token, "s3cret");
}

#[test]
fn unit_enum_option_and_default_leaves() {
    let loaded = load(&[("APP_LOG_LEVEL", "debug"), ("APP_NICKNAME", "bob")]).unwrap();
    assert_eq!(loaded.log_level, Level::Debug);
    assert_eq!(loaded.nickname.as_deref(), Some("bob"));
    assert_eq!(load(&[]).unwrap().nickname, None);
}

#[test]
fn list_is_split_on_commas_and_trimmed() {
    let loaded = load(&[("APP_TAGS", "a, b")]).unwrap();
    assert_eq!(loaded.tags, ["a", "b"]);
}

#[test]
fn empty_list_value_gives_an_empty_list() {
    let loaded = load(&[("APP_TAGS", "")]).unwrap();
    assert_eq!(loaded.tags, Vec::<String>::new());
}

#[test]
fn bad_value_error_names_the_variable() {
    let err = load(&[("APP_PORT", "x")]).unwrap_err();
    assert_eq!(err.to_string(), "port: expected u16, found \"x\" (env:APP_PORT)");
}

#[test]
fn nested_bad_value_error_has_the_full_path() {
    let err = load(&[("APP_SERVER_PORT", "x")]).unwrap_err();
    assert_eq!(err.to_string(), "server.port: expected u16, found \"x\" (env:APP_SERVER_PORT)");
}

#[test]
fn empty_variable_is_a_present_empty_value() {
    let err = load(&[("APP_PORT", "")]).unwrap_err();
    assert_eq!(err.to_string(), "port: expected u16, found \"\" (env:APP_PORT)");
}

#[derive(Config, Debug, PartialEq)]
struct Bare {
    #[config(default = 1)]
    port: u16,
    #[config(env = "BARE_HOST", default = "file-less")]
    host: String,
}

#[test]
fn without_a_prefix_only_explicit_names_are_read() {
    let loaded = Bare::loader()
        .env_vars([("PORT", "9"), ("BARE_PORT", "9"), ("BARE_HOST", "h")])
        .load()
        .unwrap();
    assert_eq!(loaded, Bare { port: 1, host: "h".into() });
}

#[derive(Config, Debug, PartialEq)]
#[config(env_prefix = "T_")]
struct Layered {
    #[config(default = 0)]
    port: u16,
    #[config(default = "base")]
    name: String,
}

struct Fixed(&'static str, &'static str);

impl ConfigSource for Fixed {
    fn load(&self) -> Result<ConfigNode, ConfigError> {
        let mut table = Table::new();
        table.insert(self.0.to_owned(), ConfigNode::scalar(self.1));
        Ok(ConfigNode::table(table))
    }
}

#[test]
fn env_beats_a_file_and_keeps_other_file_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    fs::write(&path, "port = 10\nname = \"file\"\n").unwrap();
    let loaded = Layered::loader()
        .file(ConfigFile::new(&path))
        .env_vars([("T_PORT", "11")])
        .load()
        .unwrap();
    assert_eq!(loaded, Layered { port: 11, name: "file".into() });
}

#[test]
fn env_beats_a_high_priority_file_and_a_default_source() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    fs::write(&path, "port = 10\n").unwrap();
    let loaded = Layered::loader()
        .file(ConfigFile::new(&path).priority(150))
        .source(Fixed("port", "100"))
        .env_vars([("T_PORT", "11")])
        .load()
        .unwrap();
    assert_eq!(loaded.port, 11);
}

#[test]
fn env_vars_map_replaces_the_process_environment() {
    // PATH exists in every test process; the map must hide it.
    #[derive(Config, Debug, PartialEq)]
    struct Path {
        #[config(env = "PATH", default = "unset")]
        value: String,
    }
    let loaded = Path::loader().env_vars([("OTHER", "x")]).load().unwrap();
    assert_eq!(loaded.value, "unset");
}
