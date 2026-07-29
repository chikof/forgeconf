//! Shared types, fixtures and correctness checks for the comparison benchmarks.
//!
//! Every library deserializes into its own struct; each of those converts into
//! the plain [`AppConfig`] (or [`CliArgs`]) so one `assert_eq!` against a
//! hand-written expected value proves a bench did real work before it is timed.

use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Write as _};
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const SMALL_TOML: &str = include_str!("../../fixtures/app.toml");
pub const SMALL_JSON: &str = include_str!("../../fixtures/app.json");
pub const SMALL_YAML: &str = include_str!("../../fixtures/app.yaml");

/// TOML with a type error (`port`) and an unknown key (`hots`).
pub const ERROR_TOML: &str = "host = \"localhost\"\nport = \"abc\"\nhots = \"typo\"\n";

pub const ENV_PREFIX: &str = "APP";
pub const ENV_PORT: u16 = 9090;
pub const ENV_POOL_SIZE: u32 = 32;

/// Plain config struct: the serde users (serde, config, figment) deserialize
/// straight into it, and every other library converts into it for comparison.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct AppConfig {
    pub name: String,
    pub debug: bool,
    pub features: Vec<String>,
    pub server: Server,
    pub database: Database,
    pub limits: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Server {
    pub host: String,
    pub port: u16,
    pub workers: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Database {
    pub url: String,
    pub pool_size: u32,
    pub timeout_secs: u64,
}

/// Strict variant used by the error scenario.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strict {
    pub host: String,
    pub port: u16,
}

/// Normalised result of the CLI scenario.
#[derive(Debug, Clone, PartialEq)]
pub struct CliArgs {
    pub host: String,
    pub port: u16,
    pub workers: u32,
    pub debug: bool,
    pub features: Vec<String>,
}

/// forgeconf's view of the same data.
pub mod fc {
    use std::collections::BTreeMap;

    #[derive(Debug, forgeconf::Config)]
    #[config(env_prefix = "APP_")]
    pub struct AppConfig {
        pub name: String,
        pub debug: bool,
        pub features: Vec<String>,
        pub server: Server,
        pub database: Database,
        pub limits: BTreeMap<String, u64>,
    }

    #[derive(Debug, forgeconf::Config)]
    pub struct Server {
        pub host: String,
        pub port: u16,
        pub workers: u32,
    }

    #[derive(Debug, forgeconf::Config)]
    pub struct Database {
        pub url: String,
        pub pool_size: u32,
        pub timeout_secs: u64,
    }

    #[derive(Debug, forgeconf::Config)]
    #[config(deny_unknown_fields)]
    pub struct Strict {
        pub host: String,
        pub port: u16,
    }

    #[derive(Debug, forgeconf::Config)]
    pub struct Cli {
        #[config(default = "localhost")]
        pub host: String,
        #[config(default = 8080)]
        pub port: u16,
        #[config(default = 4)]
        pub workers: u32,
        #[config(default = false)]
        pub debug: bool,
        pub features: Vec<String>,
    }
}

/// confique's view of the same data. Env variables are named per field.
pub mod cq {
    use std::collections::BTreeMap;

    #[derive(Debug, confique::Config)]
    pub struct AppConfig {
        pub name: String,
        pub debug: bool,
        pub features: Vec<String>,
        #[config(nested)]
        pub server: Server,
        #[config(nested)]
        pub database: Database,
        pub limits: BTreeMap<String, u64>,
    }

    #[derive(Debug, confique::Config)]
    pub struct Server {
        pub host: String,
        #[config(env = "APP_SERVER_PORT")]
        pub port: u16,
        pub workers: u32,
    }

    #[derive(Debug, confique::Config)]
    pub struct Database {
        pub url: String,
        #[config(env = "APP_DATABASE_POOL_SIZE")]
        pub pool_size: u32,
        pub timeout_secs: u64,
    }
}

/// clap's view of the CLI scenario.
pub mod cl {
    use clap::Parser;

    #[derive(Debug, Parser)]
    #[command(name = "app")]
    pub struct Cli {
        #[arg(long, default_value = "localhost")]
        pub host: String,
        #[arg(long, default_value_t = 8080)]
        pub port: u16,
        #[arg(long, default_value_t = 4)]
        pub workers: u32,
        #[arg(long)]
        pub debug: bool,
        #[arg(long)]
        pub features: Vec<String>,
    }
}

impl From<fc::AppConfig> for AppConfig {
    fn from(c: fc::AppConfig) -> Self {
        Self {
            name: c.name,
            debug: c.debug,
            features: c.features,
            server: Server {
                host: c.server.host,
                port: c.server.port,
                workers: c.server.workers,
            },
            database: Database {
                url: c.database.url,
                pool_size: c.database.pool_size,
                timeout_secs: c.database.timeout_secs,
            },
            limits: c.limits,
        }
    }
}

impl From<cq::AppConfig> for AppConfig {
    fn from(c: cq::AppConfig) -> Self {
        Self {
            name: c.name,
            debug: c.debug,
            features: c.features,
            server: Server {
                host: c.server.host,
                port: c.server.port,
                workers: c.server.workers,
            },
            database: Database {
                url: c.database.url,
                pool_size: c.database.pool_size,
                timeout_secs: c.database.timeout_secs,
            },
            limits: c.limits,
        }
    }
}

impl From<fc::Cli> for CliArgs {
    fn from(c: fc::Cli) -> Self {
        Self {
            host: c.host,
            port: c.port,
            workers: c.workers,
            debug: c.debug,
            features: c.features,
        }
    }
}

impl From<cl::Cli> for CliArgs {
    fn from(c: cl::Cli) -> Self {
        Self {
            host: c.host,
            port: c.port,
            workers: c.workers,
            debug: c.debug,
            features: c.features,
        }
    }
}

/// One document in all three formats with the value it must parse to.
pub struct Doc {
    pub toml: String,
    pub json: String,
    pub yaml: String,
    pub expected: AppConfig,
}

/// The value of `fixtures/app.*`.
pub fn small_expected() -> AppConfig {
    AppConfig {
        name: "inventory".into(),
        debug: false,
        features: ["search", "export", "audit"].map(String::from).to_vec(),
        server: Server {
            host: "0.0.0.0".into(),
            port: 8080,
            workers: 8,
        },
        database: Database {
            url: "postgres://app:secret@db.internal:5432/inventory".into(),
            pool_size: 16,
            timeout_secs: 30,
        },
        limits: [("upload", 1_048_576), ("requests", 1000), ("sessions", 250)]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    }
}

/// The checked-in fixtures.
pub fn small() -> Doc {
    Doc {
        toml: SMALL_TOML.to_owned(),
        json: SMALL_JSON.to_owned(),
        yaml: SMALL_YAML.to_owned(),
        expected: small_expected(),
    }
}

/// The small fixture plus 1000 extra `features` (`feat0000`..) and 1000 extra
/// `limits` (`key0000`..), generated in plain block style every parser accepts.
pub fn large() -> Doc {
    let mut expected = small_expected();
    expected
        .features
        .extend((0..1000).map(|i| format!("feat{i:04}")));
    expected
        .limits
        .extend((0..1000_u32).map(|i| (format!("key{i:04}"), u64::from(i) * 7 + 1)));
    Doc {
        toml: render_toml(&expected),
        json: render_json(&expected),
        yaml: render_yaml(&expected),
        expected,
    }
}

fn render_toml(c: &AppConfig) -> String {
    let mut s = format!("name = {:?}\ndebug = {}\nfeatures = [\n", c.name, c.debug);
    for f in &c.features {
        let _ = writeln!(s, "  {f:?},");
    }
    let _ = write!(
        s,
        "]\n\n[server]\nhost = {:?}\nport = {}\nworkers = {}\n\n[database]\nurl = {:?}\npool_size \
         = {}\ntimeout_secs = {}\n\n[limits]\n",
        c.server.host,
        c.server.port,
        c.server.workers,
        c.database.url,
        c.database.pool_size,
        c.database.timeout_secs
    );
    for (k, v) in &c.limits {
        let _ = writeln!(s, "{k} = {v}");
    }
    s
}

fn render_json(c: &AppConfig) -> String {
    let features = c
        .features
        .iter()
        .map(|f| format!("{f:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let limits = c
        .limits
        .iter()
        .map(|(k, v)| format!("{k:?}: {v}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{{\"name\": {:?}, \"debug\": {}, \"features\": [{features}], \"server\": {{\"host\": \
         {:?}, \"port\": {}, \"workers\": {}}}, \"database\": {{\"url\": {:?}, \"pool_size\": {}, \
         \"timeout_secs\": {}}}, \"limits\": {{{limits}}}}}",
        c.name,
        c.debug,
        c.server.host,
        c.server.port,
        c.server.workers,
        c.database.url,
        c.database.pool_size,
        c.database.timeout_secs
    )
}

fn render_yaml(c: &AppConfig) -> String {
    let mut s = format!("name: {}\ndebug: {}\nfeatures:\n", c.name, c.debug);
    for f in &c.features {
        let _ = writeln!(s, "  - {f}");
    }
    let _ = write!(
        s,
        "server:\n  host: {}\n  port: {}\n  workers: {}\ndatabase:\n  url: {}\n  pool_size: {}\n  \
         timeout_secs: {}\nlimits:\n",
        c.server.host,
        c.server.port,
        c.server.workers,
        c.database.url,
        c.database.pool_size,
        c.database.timeout_secs
    );
    for (k, v) in &c.limits {
        let _ = writeln!(s, "  {k}: {v}");
    }
    s
}

/// The small fixture with the environment overrides applied.
pub fn layered_expected() -> AppConfig {
    let mut c = small_expected();
    c.server.port = ENV_PORT;
    c.database.pool_size = ENV_POOL_SIZE;
    c
}

pub const CLI_ARGV: [&str; 10] = [
    "app",
    "--host",
    "10.0.0.1",
    "--port",
    "9090",
    "--debug",
    "--features",
    "a",
    "--features",
    "b",
];

pub fn cli_expected() -> CliArgs {
    CliArgs {
        host: "10.0.0.1".into(),
        port: 9090,
        workers: 4,
        debug: true,
        features: vec!["a".into(), "b".into()],
    }
}

/// Export the overrides under every library's naming convention: forgeconf and
/// confique use `APP_SERVER_PORT`, config and figment split on `__`.
///
/// # Safety
/// Call once at program start, before any thread exists: `set_var` is not
/// thread-safe.
pub unsafe fn set_env() {
    let vars = [
        ("APP_SERVER_PORT", ENV_PORT.to_string()),
        ("APP_DATABASE_POOL_SIZE", ENV_POOL_SIZE.to_string()),
        ("APP_SERVER__PORT", ENV_PORT.to_string()),
        ("APP_DATABASE__POOL_SIZE", ENV_POOL_SIZE.to_string()),
    ];
    for (key, value) in vars {
        // SAFETY: the caller guarantees the process is still single-threaded.
        unsafe { std::env::set_var(key, value) };
    }
}

/// A file in a unique temp directory, removed on drop.
pub struct TempFile(PathBuf);

impl TempFile {
    pub fn new(name: &str, contents: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("forgeconf-bench-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join(name);
        std::fs::write(&path, contents).expect("write temp file");
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(dir) = self.0.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

/// Panic unless `got` is `Ok` and converts to `expected`.
pub fn check<T, X, E>(lib: &str, expected: &X, got: Result<T, E>)
where
    X: From<T> + Debug + PartialEq,
    E: Display,
{
    match got {
        Ok(value) => assert_eq!(&X::from(value), expected, "{lib}: wrong value"),
        Err(err) => panic!("{lib}: expected Ok, got error: {err}"),
    }
}

/// Panic unless `got` is an error whose text names the bad value or key.
pub fn check_err<T, E: Display>(lib: &str, got: Result<T, E>) {
    match got {
        Ok(_) => panic!("{lib}: expected an error, got Ok"),
        Err(err) => {
            let text = err.to_string();
            assert!(
                text.contains("abc") || text.contains("hots"),
                "{lib}: unhelpful error: {text}"
            );
        },
    }
}

/// Panic unless `text` looks like the generated help of [`CliArgs`].
pub fn check_help(lib: &str, text: &str) {
    for needle in ["Usage", "--host", "--port", "--workers", "--debug", "--features"] {
        assert!(text.contains(needle), "{lib}: help lacks {needle:?}:\n{text}");
    }
}
