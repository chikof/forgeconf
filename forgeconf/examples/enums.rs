//! Enums as configuration values.
//!
//! - unit enums are plain strings (`log = "warn"`)
//! - externally tagged enums are tables with exactly one key (`[storage.disk]`)
//! - internally tagged enums (`tag = "type"`) select the variant with a key in
//!   the same table (`[auth]` with `type = "token"`)
//!
//! Run with: cargo run --example enums
#![allow(dead_code, reason = "values are only printed through Debug or never read")]

use forgeconf::Config;

#[derive(Debug, Config)]
#[config(rename_all = "lowercase")]
enum Log {
    Debug,
    Info,
    Warn,
}

/// Externally tagged: the single table key names the variant.
#[derive(Debug, Config)]
#[config(rename_all = "kebab-case")]
enum Storage {
    InMemory,
    /// Newtype variant: the value is the inner type.
    Remote(String),
    /// Struct variant: the value is a table of fields.
    Disk {
        path: std::path::PathBuf,
        #[config(default = 64)]
        max_mb: u32,
    },
}

/// Internally tagged: the `type` key picks the variant.
#[derive(Debug, Config)]
#[config(tag = "type", rename_all = "lowercase")]
enum Auth {
    None,
    Token {
        value: String,
    },
    Basic {
        user: String,
        #[config(alias = "pass")]
        password: String,
    },
}

#[derive(Debug, Config)]
struct AppConfig {
    log: Log,
    storage: Storage,
    auth: Auth,
    #[config(default)]
    mirrors: Vec<Storage>,
}

#[cfg(feature = "toml")]
fn main() {
    use forgeconf::{FileFormat, from_str};

    let input = r#"
        log = "warn"
        mirrors = [
            { remote = "s3://backup" },
            { disk = { path = "/mnt/mirror", max_mb = 128 } },
            "in-memory",
        ]

        [storage.disk]
        path = "/var/lib/app"

        [auth]
        type = "basic"
        user = "admin"
        pass = "secret"
    "#;
    match from_str::<AppConfig>(input, FileFormat::Toml) {
        Ok(config) => println!("{config:#?}"),
        Err(error) => println!("{error}"),
    }

    // Unknown names report the accepted ones; `storage` was a string, so only
    // the unit variants are listed.
    let bad = "log = \"loud\"\nstorage = \"nowhere\"\nauth = { type = \"oauth\" }\n";
    match from_str::<AppConfig>(bad, FileFormat::Toml) {
        Ok(config) => println!("unexpected: {config:?}"),
        Err(error) => println!("\n{error}"),
    }
}

#[cfg(not(feature = "toml"))]
fn main() {
    eprintln!("this example needs the `toml` feature");
}
