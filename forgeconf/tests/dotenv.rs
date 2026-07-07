#![cfg(feature = "dotenv")]

use std::fs;
use std::path::{Path, PathBuf};

use forgeconf::{Config, ConfigError, ErrorKind};

#[derive(Config, Debug, PartialEq)]
#[config(env_prefix = "APP_")]
struct App {
    #[config(default = 1)]
    port: u16,
    #[config(default = "none")]
    url: String,
    #[config(env = "export_url", default = "none")]
    exported: String,
    #[config(default = "")]
    message: String,
    #[config(default = "")]
    token: String,
}

fn write_env(dir: &Path, contents: &str) -> PathBuf {
    let path = dir.join(".env");
    fs::write(&path, contents).unwrap();
    path
}

fn load(contents: &str) -> Result<App, ConfigError> {
    let dir = tempfile::tempdir().unwrap();
    let path = write_env(dir.path(), contents);
    App::loader()
        .env_vars::<[(&str, &str); 0], _, _>([])
        .dotenv(path)
        .load()
}

#[test]
fn values_come_from_the_file() {
    let app = load("APP_PORT=8080\nAPP_URL=postgres://h/db\n").unwrap();
    assert_eq!((app.port, app.url.as_str()), (8080, "postgres://h/db"));
}

#[test]
fn export_prefix_is_stripped_only_before_whitespace() {
    let app = load("export APP_PORT=7\nexport_url=x\n").unwrap();
    assert_eq!(app.port, 7);
    assert_eq!(app.exported, "x");
}

#[test]
fn comments_blank_lines_and_inline_comments() {
    let app = load("# header\n\nAPP_PORT=9000 # production\n  # indented\nAPP_URL=a#b\n").unwrap();
    assert_eq!((app.port, app.url.as_str()), (9000, "a#b"));
}

#[test]
fn multiline_double_quoted_value() {
    let app = load("APP_MESSAGE=\"line1\nline2\\tend\"\nAPP_PORT=3\n").unwrap();
    assert_eq!(app.message, "line1\nline2\tend");
    assert_eq!(app.port, 3);
}

#[test]
fn single_quoted_value_is_literal() {
    let app = load("APP_TOKEN='a\\nb $HOME'\n").unwrap();
    assert_eq!(app.token, "a\\nb $HOME");
}

#[test]
fn no_variable_expansion() {
    let app = load("APP_URL=${HOME}/x\n").unwrap();
    assert_eq!(app.url, "${HOME}/x");
}

#[test]
fn env_vars_map_wins_over_dotenv() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_env(dir.path(), "APP_PORT=1111\nAPP_URL=from-file\n");
    let app = App::loader()
        .env_vars([("APP_PORT", "2222")])
        .dotenv(path)
        .load()
        .unwrap();
    assert_eq!((app.port, app.url.as_str()), (2222, "from-file"));
}

#[test]
fn missing_dotenv_file_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let app = App::loader()
        .env_vars::<[(&str, &str); 0], _, _>([])
        .dotenv(dir.path().join("absent.env"))
        .load()
        .unwrap();
    assert_eq!(app.port, 1);
}

#[test]
fn unreadable_dotenv_is_an_io_error() {
    let dir = tempfile::tempdir().unwrap();
    let err = App::loader()
        .env_vars::<[(&str, &str); 0], _, _>([])
        .dotenv(dir.path())
        .load()
        .unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::Io { .. }));
}

#[test]
fn syntax_error_text_has_path_line_and_column() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_env(dir.path(), "APP_PORT=1\n  9bad=2\n");
    let err = App::loader()
        .env_vars::<[(&str, &str); 0], _, _>([])
        .dotenv(&path)
        .load()
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        format!("invalid dotenv: key must match [A-Za-z_][A-Za-z0-9_.]* ({}:2:3)", path.display())
    );
}

#[test]
fn unterminated_quote_is_a_syntax_error() {
    let err = load("APP_URL=\"never closed\n").unwrap_err();
    assert!(
        err.to_string()
            .starts_with("invalid dotenv: unterminated double-quoted value (")
    );
}

#[test]
fn dotenv_is_reread_on_every_load() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_env(dir.path(), "APP_PORT=1\n");
    let loader = App::loader()
        .env_vars::<[(&str, &str); 0], _, _>([])
        .dotenv(&path);
    assert_eq!(loader.load().unwrap().port, 1);
    fs::write(&path, "APP_PORT=2\n").unwrap();
    assert_eq!(loader.load().unwrap().port, 2);
}
