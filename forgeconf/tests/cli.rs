#![cfg(all(feature = "cli", feature = "toml"))]
//! CLI argument layer tests.
#![allow(clippy::unwrap_used, reason = "tests fail loudly")]

use std::ffi::OsString;
use std::fs;

use forgeconf::{Config, ConfigError, ConfigFile, ErrorKind};

#[derive(Config, Debug, PartialEq)]
struct Server {
    #[config(default = 10)]
    max_conn: u32,
}

#[derive(Config, Debug, PartialEq)]
struct Full {
    #[config(default = 8080, short = 'p')]
    port: u16,
    #[config(default = false, short = 'v')]
    verbose: bool,
    #[config(default = false, short = 'q')]
    quiet: bool,
    #[config(default)]
    tags: Vec<String>,
    #[config(default = "none", flag = "label", short = 'l')]
    name: String,
    #[config(default = 1, skip_cli)]
    hidden: u8,
    #[config(default)]
    server: Server,
}

fn run(args: &[&str]) -> Result<Full, ConfigError> {
    Full::loader()
        .cli_args(["app"].into_iter().chain(args.iter().copied()))
        .load()
}

fn ok(args: &[&str]) -> Full {
    run(args).unwrap()
}

fn fail(args: &[&str]) -> String {
    run(args).unwrap_err().to_string()
}

#[test]
fn no_flags_keep_defaults() {
    assert_eq!(
        ok(&[]),
        Full {
            port: 8080,
            verbose: false,
            quiet: false,
            tags: Vec::new(),
            name: "none".into(),
            hidden: 1,
            server: Server { max_conn: 10 },
        }
    );
}

#[test]
fn long_flag_takes_equals_or_next_token() {
    assert_eq!(ok(&["--port=9000"]).port, 9000);
    assert_eq!(ok(&["--port", "9001"]).port, 9001);
}

#[test]
fn next_token_is_taken_verbatim_even_with_a_dash() {
    assert_eq!(ok(&["--label", "--port"]).name, "--port");
    assert_eq!(ok(&["--label", "-h"]).name, "-h");
}

#[test]
fn explicit_flag_name_replaces_the_field_name() {
    assert_eq!(ok(&["--label=x"]).name, "x");
    assert_eq!(fail(&["--name=x"]), "unknown flag --name");
}

#[test]
fn nested_fields_use_dotted_dashed_names() {
    assert_eq!(ok(&["--server.max-conn", "5"]).server, Server { max_conn: 5 });
    assert_eq!(
        fail(&["--server.max_conn", "5"]),
        "unknown flag --server.max_conn, did you mean --server.max-conn?"
    );
}

#[test]
fn bool_flags_need_no_value() {
    assert!(ok(&["--verbose"]).verbose);
    assert!(ok(&["--verbose", "--port", "1"]).verbose);
    assert!(!ok(&["--verbose", "--no-verbose"]).verbose);
    assert!(!ok(&["--verbose=false"]).verbose);
    assert!(ok(&["--verbose=true"]).verbose);
}

#[test]
fn bool_flag_does_not_swallow_the_next_token() {
    assert_eq!(fail(&["--verbose", "yes"]), "unexpected argument \"yes\"");
}

#[test]
fn short_flags() {
    assert_eq!(ok(&["-p", "9002"]).port, 9002);
    assert_eq!(ok(&["-p=9003"]).port, 9003);
    assert!(ok(&["-v"]).verbose);
    assert!(!ok(&["-v=false"]).verbose);
    assert_eq!(ok(&["-l", "-v"]).name, "-v");
}

#[test]
fn combined_short_flags_are_rejected() {
    assert_eq!(fail(&["-vq"]), "combined short flags are not supported: -vq");
    assert_eq!(fail(&["-p8080"]), "combined short flags are not supported: -p8080");
}

#[test]
fn unknown_short_flag() {
    assert_eq!(fail(&["-x"]), "unknown flag -x");
}

#[test]
fn list_flags_append_on_repeat() {
    assert_eq!(ok(&["--tags", "a", "--tags=b", "--tags", "c"]).tags, ["a", "b", "c"]);
}

#[test]
fn scalar_repeat_keeps_the_last_value() {
    assert_eq!(ok(&["--port", "1", "-p", "2", "--port=3"]).port, 3);
}

#[test]
fn unknown_flag_suggests_the_closest_name() {
    assert_eq!(fail(&["--prot", "1"]), "unknown flag --prot, did you mean --port?");
    assert_eq!(fail(&["--zzzzzz"]), "unknown flag --zzzzzz");
}

#[test]
fn skip_cli_fields_have_no_flag() {
    assert_eq!(fail(&["--hidden", "2"]), "unknown flag --hidden");
}

#[test]
fn missing_value() {
    assert_eq!(fail(&["--port"]), "flag --port requires a value");
    assert_eq!(fail(&["-p"]), "flag -p requires a value");
}

#[test]
fn positional_and_double_dash_are_rejected() {
    assert_eq!(fail(&["x"]), "unexpected argument \"x\"");
    assert_eq!(fail(&["--port", "1", "extra"]), "unexpected argument \"extra\"");
    assert_eq!(fail(&["--"]), "unexpected argument \"--\"");
    assert_eq!(fail(&["-"]), "unexpected argument \"-\"");
}

#[cfg(unix)]
#[test]
fn non_utf8_argument() {
    use std::os::unix::ffi::OsStringExt;

    let args = vec![OsString::from("app"), OsString::from_vec(vec![b'-', b'-', 0xff])];
    let err = Full::loader().cli_args(args).load().unwrap_err();
    assert_eq!(err.to_string(), "argument is not valid UTF-8");
    assert!(matches!(err.kind(), ErrorKind::Cli(_)));
}

#[test]
fn bad_value_reports_the_flag_as_origin() {
    assert_eq!(fail(&["--port", "abc"]), "port: expected u16, found \"abc\" (cli:--port)");
}

#[test]
fn cli_beats_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    fs::write(&path, "port = 1\nname = \"file\"\ntags = [\"f\"]\n[server]\nmax_conn = 2\n")
        .unwrap();
    let cfg = Full::loader()
        .file(ConfigFile::new(&path))
        .cli_args(["app", "--port", "9", "--tags", "c"])
        .load()
        .unwrap();
    assert_eq!(cfg.port, 9);
    assert_eq!(cfg.name, "file");
    assert_eq!(cfg.tags, ["c"]);
    assert_eq!(cfg.server, Server { max_conn: 2 });
}

#[test]
fn without_cli_args_the_process_arguments_are_ignored() {
    assert_eq!(Full::loader().load().unwrap().port, 8080);
}

/// Small demo.
#[allow(dead_code, reason = "only the generated help is checked")]
#[derive(Config, Debug)]
#[config(env_prefix = "APP_")]
struct Small {
    /// Port to listen on.
    #[config(default = 8080, short = 'p')]
    port: u16,
    /// Enable verbose output.
    #[config(default = false)]
    verbose: bool,
}

fn help_of<T: Config>(args: &[&str]) -> String {
    let err = T::loader()
        .cli_args(args.iter().copied())
        .load()
        .err()
        .unwrap();
    assert!(err.is_help());
    err.to_string()
}

#[test]
fn help_lists_every_flag() {
    let expected = "\
Usage: app [OPTIONS]

Small demo.

Options:
  -p, --port <u16>  Port to listen on [default: 8080] [env: APP_PORT]
      --verbose     Enable verbose output [default: false] [env: APP_VERBOSE]
  -h, --help        Print help";
    assert_eq!(help_of::<Small>(&["app", "--help"]), expected);
    assert_eq!(help_of::<Small>(&["/usr/bin/app", "-h"]), expected);
}

#[test]
fn help_uses_the_program_file_name() {
    assert!(help_of::<Small>(&["/opt/bin/serve", "-h"]).starts_with("Usage: serve [OPTIONS]\n"));
}

#[derive(Config, Debug)]
enum Level {
    Low,
    High,
}

#[allow(dead_code, reason = "only the generated help is checked")]
#[derive(Config)]
struct Wide {
    /// Secret token.
    #[config(sensitive, default = "hunter2")]
    token: String,
    level: Level,
    /// Names to greet.
    #[config(default)]
    names: Vec<String>,
    /// Old knob.
    #[config(deprecated = "use level", default = 1)]
    knob: u8,
    /// Shown on its own line because the flag is long.
    #[config(flag = "a-very-long-flag-name-for-testing", default = false)]
    long: bool,
    #[config(skip_cli, default = 1)]
    hidden: u8,
}

#[test]
fn help_formats_labels_wrapping_and_markers() {
    let expected = "\
Usage: app [OPTIONS]

Options:
      --token <string>            Secret token
      --level <Low|High>
      --names <string>...         Names to greet
      --knob <u8>                 Old knob [default: 1] [deprecated]
      --a-very-long-flag-name-for-testing
                                  Shown on its own line because the flag is long [default: false]
  -h, --help                      Print help";
    assert_eq!(help_of::<Wide>(&["app", "--help"]), expected);
}

#[test]
fn help_wins_over_later_flags_only_when_first() {
    assert!(help_of::<Small>(&["app", "--help", "--bogus"]).starts_with("Usage:"));
    let err = Small::loader()
        .cli_args(["app", "--bogus", "--help"])
        .load()
        .unwrap_err();
    assert_eq!(err.to_string(), "unknown flag --bogus");
}

impl Default for Server {
    fn default() -> Self {
        Server { max_conn: 10 }
    }
}
