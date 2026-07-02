#![cfg(feature = "toml")]

use forgeconf::{Config, ConfigError, ConfigNode, FileFormat, FromNode, from_str};

fn load<T: FromNode>(input: &str) -> Result<T, ConfigError> {
    from_str(input, FileFormat::Toml)
}

fn message<T: FromNode>(input: &str) -> String {
    match load::<T>(input) {
        Ok(_) => panic!("expected an error for {input:?}"),
        Err(err) => err.to_string(),
    }
}

#[derive(Config, Debug, PartialEq)]
#[config(rename_all = "lowercase")]
enum Log {
    Debug,
    Info,
    #[config(alias = "warning")]
    Warn,
    #[config(rename = "err")]
    Error,
}

#[derive(Config, Debug, PartialEq)]
struct Logging {
    log: Log,
}

#[test]
fn unit_variants_match_renamed_keys() {
    assert_eq!(load::<Logging>("log = \"warn\"").unwrap(), Logging { log: Log::Warn });
    assert_eq!(load::<Logging>("log = \"debug\"").unwrap(), Logging { log: Log::Debug });
}

#[test]
fn unit_variant_alias_and_rename() {
    assert_eq!(load::<Logging>("log = \"warning\"").unwrap(), Logging { log: Log::Warn });
    assert_eq!(load::<Logging>("log = \"err\"").unwrap(), Logging { log: Log::Error });
    assert_eq!(
        message::<Logging>("log = \"error\""),
        "log: unknown variant \"error\", expected one of: debug, info, warn, err (<string>:1:7)"
    );
}

#[derive(Config, Debug, PartialEq)]
#[config(rename_all = "snake_case")]
enum Mode {
    ShutDown,
    KeepRunning,
}

#[test]
fn snake_case_variants() {
    assert_eq!(Mode::from_node(&ConfigNode::scalar("shut_down")).unwrap(), Mode::ShutDown);
    assert_eq!(Mode::from_node(&ConfigNode::scalar("keep_running")).unwrap(), Mode::KeepRunning);
}

#[derive(Config, Debug, PartialEq)]
#[config(rename_all = "lowercase")]
enum Ab {
    A,
    B,
}

#[test]
fn unknown_variant_message_lists_every_key() {
    let err = Ab::from_node(&ConfigNode::scalar("x")).unwrap_err();
    assert_eq!(err.to_string(), "unknown variant \"x\", expected one of: a, b");
}

#[test]
fn non_scalar_for_unit_enum_is_a_mismatch() {
    assert_eq!(
        message::<Holder>("ab = [1]"),
        "ab: expected one of: a, b, found \"array\" (<string>:1:6)"
    );
}

#[derive(Config, Debug, PartialEq)]
struct Holder {
    ab: Ab,
}

#[derive(Config, Debug, PartialEq)]
struct Inner {
    a: u8,
}

#[derive(Config, Debug, PartialEq)]
#[config(rename_all = "lowercase")]
enum Sink {
    Stdout,
    File {
        path: String,
        #[config(default = false)]
        append: bool,
    },
    Tcp(u16),
    Custom(Inner),
}

#[derive(Config, Debug, PartialEq)]
struct Output {
    sink: Sink,
}

#[test]
fn externally_tagged_struct_variant() {
    assert_eq!(
        load::<Output>("sink = { file = { path = \"x\" } }").unwrap(),
        Output {
            sink: Sink::File { path: "x".into(), append: false }
        }
    );
    assert_eq!(
        load::<Output>("[sink.file]\npath = \"y\"\nappend = true\n").unwrap(),
        Output {
            sink: Sink::File { path: "y".into(), append: true }
        }
    );
}

#[test]
fn externally_tagged_newtype_variants() {
    assert_eq!(load::<Output>("sink = { tcp = 9000 }").unwrap(), Output { sink: Sink::Tcp(9000) });
    assert_eq!(
        load::<Output>("sink = { custom = { a = 3 } }").unwrap(),
        Output { sink: Sink::Custom(Inner { a: 3 }) }
    );
}

#[test]
fn unit_variant_next_to_data_variants() {
    assert_eq!(load::<Output>("sink = \"stdout\"").unwrap(), Output { sink: Sink::Stdout });
}

#[test]
fn externally_tagged_errors_carry_the_variant_in_the_path() {
    assert_eq!(
        message::<Output>("sink = { file = {} }"),
        "sink.file.path: missing value (<string>:1:17)"
    );
    assert_eq!(
        message::<Output>("sink = { tcp = \"x\" }"),
        "sink.tcp: expected u16, found \"x\" (<string>:1:16)"
    );
}

#[test]
fn unknown_externally_tagged_variant_lists_data_keys() {
    assert_eq!(
        message::<Output>("sink = { udp = 1 }"),
        "sink: unknown variant \"udp\", expected one of: file, tcp, custom (<string>:1:16)"
    );
    assert_eq!(
        message::<Output>("sink = \"nope\""),
        "sink: unknown variant \"nope\", expected one of: stdout (<string>:1:8)"
    );
}

#[test]
fn externally_tagged_table_needs_exactly_one_key() {
    assert_eq!(
        message::<Output>("sink = { tcp = 1, file = { path = \"x\" } }"),
        "sink: expected table with exactly one key, found \"table with 2 keys\" (<string>:1:8)"
    );
}

#[derive(Config, Debug, PartialEq)]
#[config(tag = "type", rename_all = "snake_case")]
enum Source {
    Memory,
    HttpServer { url: String },
    Inline(Inner),
}

#[derive(Config, Debug, PartialEq)]
struct Origin {
    source: Source,
}

#[test]
fn internally_tagged_variants() {
    assert_eq!(
        load::<Origin>("[source]\ntype = \"memory\"\n").unwrap(),
        Origin { source: Source::Memory }
    );
    assert_eq!(
        load::<Origin>("[source]\ntype = \"http_server\"\nurl = \"u\"\n").unwrap(),
        Origin {
            source: Source::HttpServer { url: "u".into() }
        }
    );
    assert_eq!(
        load::<Origin>("[source]\ntype = \"inline\"\na = 4\n").unwrap(),
        Origin { source: Source::Inline(Inner { a: 4 }) }
    );
}

#[test]
fn internally_tagged_missing_and_unknown_tag() {
    assert_eq!(
        message::<Origin>("[source]\nurl = \"u\"\n"),
        "source.type: missing value (<string>:1:2)"
    );
    assert_eq!(
        message::<Origin>("[source]\ntype = \"ftp\"\n"),
        "source.type: unknown variant \"ftp\", expected one of: memory, http_server, inline \
         (<string>:2:8)"
    );
}

#[test]
fn internally_tagged_variant_field_errors_have_no_variant_segment() {
    assert_eq!(
        message::<Origin>("[source]\ntype = \"http_server\"\n"),
        "source.url: missing value (<string>:1:2)"
    );
}
