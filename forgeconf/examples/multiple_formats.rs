//! Multiple formats, from files and from strings.
//!
//! The same struct loads from TOML, YAML and JSON. Files pick the format from
//! their extension; strings name it with `FileFormat`. Only the formats whose
//! Cargo feature is on are compiled in.
//!
//! Run with: cargo run --example multiple_formats --all-features
#![cfg_attr(
    not(any(feature = "toml", feature = "yaml", feature = "json")),
    allow(dead_code, unused_imports)
)]

use forgeconf::{Config, ConfigError, ConfigFile, FileFormat};

#[derive(Debug, Config)]
struct Package {
    name: String,
    version: String,
    enabled: bool,
}

#[derive(Debug, Config)]
struct Api {
    endpoint: String,
    timeout: u32,
    #[config(default = 3)]
    retries: u8,
}

fn load_file(path: &str) -> Result<Package, ConfigError> {
    Package::loader().file(ConfigFile::new(path)).load()
}

fn show_file(label: &str, result: Result<Package, ConfigError>) {
    match result {
        Ok(p) => println!("{label}: {} v{} (enabled: {})", p.name, p.version, p.enabled),
        Err(error) => println!("{label} failed: {error}"),
    }
}

fn show_string(label: &str, input: &str, format: FileFormat) {
    match forgeconf::from_str::<Api>(input, format) {
        Ok(api) => {
            println!("{label}: {} timeout={}s retries={}", api.endpoint, api.timeout, api.retries)
        },
        Err(error) => println!("{label} failed: {error}"),
    }
}

fn main() {
    println!("From files:");
    #[cfg(feature = "toml")]
    show_file("toml", load_file("examples/fixtures/format.toml"));
    #[cfg(feature = "yaml")]
    show_file("yaml", load_file("examples/fixtures/format.yaml"));
    #[cfg(feature = "json")]
    show_file("json", load_file("examples/fixtures/format.json"));

    println!("\nFrom strings:");
    #[cfg(feature = "toml")]
    show_string("toml", "endpoint = \"https://api.example.com\"\ntimeout = 30\n", FileFormat::Toml);
    #[cfg(feature = "yaml")]
    show_string(
        "yaml",
        "endpoint: https://api.example.com\ntimeout: 30\nretries: 5\n",
        FileFormat::Yaml,
    );
    #[cfg(feature = "json")]
    show_string(
        "json",
        r#"{"endpoint": "https://api.example.com", "timeout": 30, "retries": 1}"#,
        FileFormat::Json,
    );

    #[cfg(not(any(feature = "toml", feature = "yaml", feature = "json")))]
    eprintln!("this example needs the `toml`, `yaml` or `json` feature");
}
