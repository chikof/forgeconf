#![cfg(feature = "toml")]
//! `#[derive(Config)]` struct tests.
#![allow(clippy::unwrap_used, reason = "tests fail loudly")]

use std::fs;

use forgeconf::{Config, ConfigError, ConfigFile, FileFormat, from_str, validators};

fn load<T: forgeconf::FromNode>(input: &str) -> Result<T, ConfigError> {
    from_str(input, FileFormat::Toml)
}

fn message<T: forgeconf::FromNode>(input: &str) -> String {
    match load::<T>(input) {
        Ok(_) => panic!("expected an error for {input:?}"),
        Err(err) => err.to_string(),
    }
}

#[derive(Config, Debug, PartialEq)]
struct Defaults {
    #[config(default = "h")]
    host: String,
    #[config(default = 8080)]
    port: u16,
    #[config(default)]
    retries: u8,
    #[config(default = vec![1, 2])]
    ids: Vec<u8>,
}

#[test]
fn defaults_fill_absent_keys() {
    assert_eq!(
        load::<Defaults>("").unwrap(),
        Defaults {
            host: "h".into(),
            port: 8080,
            retries: 0,
            ids: vec![1, 2]
        }
    );
}

#[test]
fn present_keys_beat_defaults() {
    let loaded = load::<Defaults>("host = \"x\"\nport = 1\nretries = 3\nids = [9]\n").unwrap();
    assert_eq!(
        loaded,
        Defaults {
            host: "x".into(),
            port: 1,
            retries: 3,
            ids: vec![9]
        }
    );
}

#[test]
fn required_field_without_default_is_missing() {
    #[derive(Config, Debug)]
    struct Required {
        #[allow(dead_code)]
        port: u16,
    }
    assert_eq!(message::<Required>(""), "port: missing value (<string>:1:1)");
}

#[derive(Config, Debug, PartialEq)]
struct Aliased {
    #[config(alias = "port_number", alias = "p", default = 1)]
    port: u16,
}

#[test]
fn alias_is_read_when_key_is_absent() {
    assert_eq!(load::<Aliased>("port_number = 8080").unwrap(), Aliased { port: 8080 });
    assert_eq!(load::<Aliased>("p = 7").unwrap(), Aliased { port: 7 });
}

#[test]
fn key_beats_alias_when_both_present() {
    assert_eq!(load::<Aliased>("port_number = 2\nport = 3\np = 4").unwrap(), Aliased { port: 3 });
}

#[derive(Config, Debug, PartialEq)]
#[config(rename_all = "kebab-case")]
struct Kebab {
    listen_port: u16,
    #[config(rename = "Key")]
    api_key: String,
}

#[test]
fn rename_all_and_rename_set_the_keys() {
    assert_eq!(
        load::<Kebab>("listen-port = 1\nKey = \"k\"").unwrap(),
        Kebab { listen_port: 1, api_key: "k".into() }
    );
    assert_eq!(
        message::<Kebab>("listen_port = 1\napi_key = \"k\""),
        "2 configuration errors:\n  - listen-port: missing value (<string>:1:1)\n  - Key: missing \
         value (<string>:1:1)"
    );
}

#[derive(Config, Debug, PartialEq)]
#[config(rename_all = "camelCase")]
struct Camel {
    max_retries: u8,
}

#[test]
fn rename_all_camel_case() {
    assert_eq!(load::<Camel>("maxRetries = 4").unwrap(), Camel { max_retries: 4 });
}

#[derive(Config, Debug, PartialEq)]
#[config(deny_unknown_fields)]
struct Strict {
    #[config(insensitive, alias = "Level")]
    log_level: String,
    port: u16,
}

#[test]
fn insensitive_field_matches_any_ascii_case() {
    assert_eq!(
        load::<Strict>("LOG_LEVEL = \"debug\"\nport = 1").unwrap(),
        Strict { log_level: "debug".into(), port: 1 }
    );
}

#[test]
fn insensitive_match_is_not_an_unknown_key() {
    assert_eq!(
        load::<Strict>("Log_Level = \"info\"\nport = 2").unwrap(),
        Strict { log_level: "info".into(), port: 2 }
    );
    assert_eq!(
        load::<Strict>("LEVEL = \"warn\"\nport = 2").unwrap(),
        Strict { log_level: "warn".into(), port: 2 }
    );
}

#[test]
fn sensitive_to_case_fields_reject_other_cases() {
    assert_eq!(
        message::<Strict>("log_level = \"x\"\nPort = 2"),
        "2 configuration errors:\n  - port: missing value (<string>:1:1)\n  - Port: unknown key, \
         did you mean \"port\"? (<string>:2:8)"
    );
}

#[derive(Config, Debug, PartialEq)]
struct Runtime {
    hits: u32,
}

impl Default for Runtime {
    fn default() -> Self {
        Self { hits: 7 }
    }
}

#[derive(Config, Debug, PartialEq)]
struct Skipping {
    name: String,
    #[config(skip)]
    runtime: Runtime,
    #[config(skip)]
    cache: Vec<u8>,
}

#[test]
fn skipped_fields_are_default_and_never_read() {
    let loaded = load::<Skipping>("name = \"a\"\nruntime = { hits = 1 }\ncache = [1]").unwrap();
    assert_eq!(
        loaded,
        Skipping {
            name: "a".into(),
            runtime: Runtime { hits: 7 },
            cache: Vec::new()
        }
    );
}

#[derive(Config, Debug, PartialEq)]
struct Common {
    #[config(default = 1)]
    retries: u8,
}

#[derive(Config, Debug, PartialEq)]
struct Flat {
    name: String,
    #[config(flatten)]
    common: Common,
}

#[test]
fn flatten_reads_inner_keys_at_this_level() {
    assert_eq!(
        load::<Flat>("name = \"a\"\nretries = 5").unwrap(),
        Flat {
            name: "a".into(),
            common: Common { retries: 5 }
        }
    );
    assert_eq!(
        load::<Flat>("name = \"a\"").unwrap(),
        Flat {
            name: "a".into(),
            common: Common { retries: 1 }
        }
    );
}

#[test]
fn flatten_errors_use_the_inner_key() {
    assert_eq!(
        message::<Flat>("name = \"a\"\nretries = \"x\""),
        "retries: expected u8, found \"x\" (<string>:2:11)"
    );
}

#[derive(Config, Debug, PartialEq)]
struct Server {
    #[config(default = "0.0.0.0")]
    host: String,
    port: u16,
}

#[derive(Config, Debug, PartialEq)]
struct Nested {
    server: Server,
    #[config(default)]
    backup: Option<Server>,
    note: Option<String>,
}

#[test]
fn nested_struct_reads_a_subtable() {
    let loaded = load::<Nested>("[server]\nport = 9\n").unwrap();
    assert_eq!(
        loaded,
        Nested {
            server: Server { host: "0.0.0.0".into(), port: 9 },
            backup: None,
            note: None,
        }
    );
}

#[test]
fn option_fields_are_none_when_absent_and_some_when_present() {
    let loaded =
        load::<Nested>("note = \"n\"\n[server]\nport = 9\n[backup]\nhost = \"b\"\nport = 10\n")
            .unwrap();
    assert_eq!(loaded.note.as_deref(), Some("n"));
    assert_eq!(loaded.backup, Some(Server { host: "b".into(), port: 10 }));
}

#[test]
fn nested_errors_carry_the_full_path() {
    assert_eq!(
        message::<Nested>("[server]\nport = \"x\"\n"),
        "server.port: expected u16, found \"x\" (<string>:2:8)"
    );
}

#[derive(Config, Debug, PartialEq)]
struct Fleet {
    servers: Vec<Server>,
}

#[test]
fn vec_of_structs_from_array_of_tables() {
    let loaded =
        load::<Fleet>("[[servers]]\nport = 1\n\n[[servers]]\nhost = \"b\"\nport = 2\n").unwrap();
    assert_eq!(
        loaded.servers,
        vec![Server { host: "0.0.0.0".into(), port: 1 }, Server { host: "b".into(), port: 2 },]
    );
}

#[test]
fn vec_of_structs_error_names_the_index() {
    assert_eq!(
        message::<Fleet>("[[servers]]\nport = 1\n\n[[servers]]\nport = \"x\"\n"),
        "servers[1].port: expected u16, found \"x\" (<string>:5:8)"
    );
}

#[allow(clippy::trivially_copy_pass_by_ref, reason = "validators receive `&T`")]
fn even(value: &u16) -> Result<(), ConfigError> {
    if value % 2 == 0 { Ok(()) } else { Err(ConfigError::invalid("must be even")) }
}

#[derive(Config, Debug, PartialEq)]
struct Validated {
    #[config(validate = validators::range(1024..=65535), validate = even)]
    port: u16,
    #[config(validate = validators::non_empty(), validate = validators::len(..=3))]
    code: String,
    #[config(default = 0, validate = validators::range(1..=9))]
    level: u8,
}

#[test]
fn field_validators_accept_good_values() {
    assert_eq!(
        load::<Validated>("port = 2000\ncode = \"ab\"\nlevel = 3").unwrap(),
        Validated { port: 2000, code: "ab".into(), level: 3 }
    );
}

#[test]
fn field_validator_error_carries_path_and_origin() {
    assert_eq!(
        message::<Validated>("port = 80\ncode = \"ab\"\nlevel = 3"),
        "port: must be between 1024 and 65535 (<string>:1:8)"
    );
}

#[test]
fn field_validators_run_in_declaration_order() {
    assert_eq!(
        message::<Validated>("port = 2001\ncode = \"\"\nlevel = 3"),
        "2 configuration errors:\n  - port: must be even (<string>:1:8)\n  - code: must not be \
         empty (<string>:2:8)"
    );
    assert_eq!(
        message::<Validated>("port = 2000\ncode = \"abcd\"\nlevel = 3"),
        "code: length must be at most 3 (<string>:2:8)"
    );
}

#[test]
fn default_values_are_validated_without_origin() {
    assert_eq!(
        message::<Validated>("port = 2000\ncode = \"ab\""),
        "level: must be between 1 and 9"
    );
}

fn ordered(window: &Window) -> Result<(), ConfigError> {
    if window.lo > window.hi { Err(ConfigError::invalid("lo must not exceed hi")) } else { Ok(()) }
}

fn always_fails(_: &Guarded) -> Result<(), ConfigError> {
    Err(ConfigError::invalid("struct validator ran"))
}

#[derive(Config, Debug, PartialEq)]
#[config(validate = ordered)]
struct Window {
    lo: u32,
    hi: u32,
}

#[derive(Config, Debug, PartialEq)]
#[config(validate = always_fails)]
struct Guarded {
    #[allow(dead_code)]
    a: u32,
}

#[test]
fn struct_validator_sees_the_loaded_value() {
    assert_eq!(load::<Window>("lo = 1\nhi = 2").unwrap(), Window { lo: 1, hi: 2 });
    assert_eq!(message::<Window>("lo = 5\nhi = 2"), "lo must not exceed hi");
}

#[test]
fn struct_validator_runs_only_when_every_field_loaded() {
    assert_eq!(message::<Guarded>("a = 1"), "struct validator ran");
    assert_eq!(message::<Guarded>("a = \"x\""), "a: expected u32, found \"x\" (<string>:1:5)");
    assert_eq!(message::<Guarded>(""), "a: missing value (<string>:1:1)");
}

#[derive(Config, Debug, PartialEq)]
#[config(deny_unknown_fields)]
struct Closed {
    host: String,
    port: u16,
}

#[test]
fn unknown_key_suggests_the_closest_field() {
    assert_eq!(
        message::<Closed>("host = \"a\"\nport = 1\nprot = 2"),
        "prot: unknown key, did you mean \"port\"? (<string>:3:8)"
    );
}

#[test]
fn unknown_key_without_a_close_match_has_no_hint() {
    assert_eq!(
        message::<Closed>("host = \"a\"\nport = 1\nzzzzzzzz = 2"),
        "zzzzzzzz: unknown key (<string>:3:12)"
    );
}

#[test]
fn unknown_keys_are_ignored_without_deny_unknown_fields() {
    assert_eq!(
        load::<Server>("port = 1\nextra = 2").unwrap(),
        Server { host: "0.0.0.0".into(), port: 1 }
    );
}

#[derive(Config, Debug, PartialEq)]
struct Two {
    port: u16,
    retries: u8,
}

#[test]
fn every_bad_field_is_reported_at_once() {
    assert_eq!(
        message::<Two>("port = \"abc\"\nretries = \"x\""),
        "2 configuration errors:\n  - port: expected u16, found \"abc\" (<string>:1:8)\n  - \
         retries: expected u8, found \"x\" (<string>:2:11)"
    );
}

#[test]
fn errors_nested_in_several_fields_flatten_into_one_report() {
    #[derive(Config, Debug)]
    struct Outer {
        #[allow(dead_code)]
        a: Two,
        #[allow(dead_code)]
        b: Two,
    }
    assert_eq!(
        message::<Outer>("[a]\nport = \"x\"\nretries = 1\n[b]\nport = 1\nretries = \"y\"\n"),
        "2 configuration errors:\n  - a.port: expected u16, found \"x\" (<string>:2:8)\n  - \
         b.retries: expected u8, found \"y\" (<string>:6:11)"
    );
}

#[derive(Config)]
struct Secrets {
    user: String,
    #[config(sensitive)]
    pin: u16,
    #[config(sensitive)]
    token: String,
}

#[test]
fn sensitive_values_are_redacted_in_errors() {
    assert_eq!(
        message::<Secrets>("user = \"u\"\npin = \"abc\"\ntoken = \"t\""),
        "pin: expected u16, found \"***\" (<string>:2:7)"
    );
}

#[test]
fn sensitive_values_are_redacted_in_debug() {
    let secrets = load::<Secrets>("user = \"bob\"\npin = 1234\ntoken = \"hunter2\"").unwrap();
    assert_eq!(format!("{secrets:?}"), "Secrets { user: \"bob\", pin: ***, token: *** }");
    assert_eq!(secrets.pin, 1234);
    assert_eq!(secrets.user, "bob");
    assert_eq!(secrets.token, "hunter2");
}

#[derive(Config, Debug, PartialEq)]
struct Port(#[config(validate = validators::range(1024..=65535))] u16);

#[derive(Config, Debug, PartialEq)]
struct Holder {
    port: Port,
}

#[test]
fn newtype_struct_delegates_and_validates() {
    assert_eq!(load::<Holder>("port = 8080").unwrap(), Holder { port: Port(8080) });
    assert_eq!(
        message::<Holder>("port = 80"),
        "port: must be between 1024 and 65535 (<string>:1:8)"
    );
    assert_eq!(message::<Holder>("port = \"x\""), "port: expected u16, found \"x\" (<string>:1:8)");
    assert_eq!(message::<Holder>(""), "port: missing value (<string>:1:1)");
}

#[test]
fn non_table_root_is_a_mismatch() {
    assert_eq!(
        forgeconf::FromNode::from_node(&forgeconf::ConfigNode::scalar("x"))
            .map(|_: Defaults| ())
            .unwrap_err()
            .to_string(),
        "expected table for Defaults, found \"scalar\""
    );
}

#[test]
fn file_load_reports_syntax_type_and_unknown_keys_with_file_origins() {
    #[derive(Config, Debug)]
    #[config(deny_unknown_fields)]
    struct S {
        #[config(default = "h")]
        #[allow(dead_code)]
        host: String,
        #[allow(dead_code)]
        port: u16,
    }

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    fs::write(&path, "port = \"abc\"\nhots = \"x\"\n").unwrap();

    let err = S::loader().file(ConfigFile::new(&path)).load().unwrap_err();
    // Origins point at the value, not the key: `hots = "x"` reports column 8.
    let p = path.display();
    assert_eq!(
        err.to_string(),
        format!(
            "2 configuration errors:\n  - port: expected u16, found \"abc\" ({p}:1:8)\n  - hots: \
             unknown key, did you mean \"host\"? ({p}:2:8)"
        )
    );
}
