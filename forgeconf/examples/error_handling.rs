//! Error handling.
//!
//! A failed load returns one `ConfigError`. Every bad field is collected, so a
//! single run reports all of them: `errors()` iterates the individual errors
//! and `kind()` says what went wrong. Each error knows its key path and, for
//! file input, the line and column it came from.
//!
//! Run with: cargo run --example error_handling
#![allow(dead_code, reason = "values are only printed through Debug or never read")]

use forgeconf::{Config, ConfigError, ErrorKind};

#[derive(Debug, Config)]
#[config(deny_unknown_fields)]
struct ServerConfig {
    #[config(default = "localhost")]
    host: String,
    port: u16,
    #[config(validate = forgeconf::validators::range(1..=64))]
    workers: u8,
    mode: Mode,
}

#[derive(Debug, Config)]
#[config(rename_all = "lowercase")]
enum Mode {
    Fast,
    Safe,
}

/// `sensitive` fields are redacted in errors; the derive implements `Debug`
/// for such structs, so it must not be derived again.
#[derive(Config)]
struct Credentials {
    #[config(sensitive)]
    pin: u16,
}

fn describe(error: &ConfigError) -> String {
    match error.kind() {
        ErrorKind::Missing => "required value is absent".to_owned(),
        ErrorKind::Mismatch { expected, found } => {
            format!("wrong type, expected {expected}, found {found:?}")
        },
        ErrorKind::Invalid(message) => format!("validator rejected it: {message}"),
        ErrorKind::UnknownKey { suggestion: Some(key) } => format!("unknown key, try {key:?}"),
        ErrorKind::UnknownKey { suggestion: None } => "unknown key".to_owned(),
        ErrorKind::UnknownVariant { found, expected } => {
            format!("variant {found:?} is not one of {expected:?}")
        },
        ErrorKind::Syntax { format, message } => format!("not valid {format}: {message}"),
        ErrorKind::Io { path, .. } => format!("cannot read {}", path.display()),
        _ => "other error".to_owned(),
    }
}

fn report(title: &str, error: &ConfigError) {
    println!("--- {title}");
    for item in error.errors() {
        let path = if item.path().is_empty() { "<root>".to_owned() } else { item.path() };
        match item.origin() {
            Some(origin) => println!("  {path} [{origin}]: {}", describe(item)),
            None => println!("  {path}: {}", describe(item)),
        }
    }
    println!("full report:\n{error}\n");
}

#[cfg(feature = "toml")]
fn main() {
    use forgeconf::{ConfigFile, FileFormat, from_str};

    let several = "port = \"abc\"\nworkers = 100\nmode = \"turbo\"\nhots = \"x\"\n";
    let error = from_str::<ServerConfig>(several, FileFormat::Toml).unwrap_err();
    report("several bad fields at once", &error);

    let error = from_str::<ServerConfig>("port = 1\nworkers = 2\n", FileFormat::Toml).unwrap_err();
    report("missing value", &error);

    let error = from_str::<ServerConfig>("port = = 1\n", FileFormat::Toml).unwrap_err();
    report("syntax error", &error);

    let error = from_str::<Credentials>("pin = \"hunter2\"\n", FileFormat::Toml).unwrap_err();
    report("sensitive value", &error);

    let error = ServerConfig::loader()
        .file(ConfigFile::new("examples/fixtures/does-not-exist.toml"))
        .load()
        .unwrap_err();
    report("missing file", &error);

    // `fn main() -> Result<(), ConfigError>` prints the same readable report,
    // because `Debug` for `ConfigError` writes the Display text.
}

#[cfg(not(feature = "toml"))]
fn main() {
    eprintln!("this example needs the `toml` feature");
}
