//! Server configuration layered from a file, the environment and the command
//! line.

use std::time::Duration;

use forgeconf::Config;

/// Demo server configuration.
#[allow(dead_code, reason = "the fields are only printed through Debug")]
#[derive(Debug, Config)]
#[config(
    file(path = "examples/fixtures/server.toml", optional),
    env_prefix = "SERVER_",
    deny_unknown_fields
)]
struct ServerConfig {
    /// Address to bind.
    #[config(default = "127.0.0.1")]
    host: String,
    /// Port to listen on.
    #[config(default = 8080, short = 'p', validate = forgeconf::validators::range(1024..=65535))]
    port: u16,
    /// Request timeout.
    #[config(default = Duration::from_secs(30))]
    timeout: Duration,
    /// Log level.
    #[config(default = Log::Info)]
    log: Log,
    /// Tags to attach.
    #[config(default)]
    tags: Vec<String>,
}

#[derive(Debug, Config)]
#[config(rename_all = "lowercase")]
enum Log {
    Debug,
    Info,
    Warn,
}

#[cfg(feature = "cli")]
fn main() {
    let cfg = ServerConfig::loader().cli().load_or_exit();
    println!("{cfg:#?}");
}

#[cfg(not(feature = "cli"))]
fn main() {
    eprintln!("this example needs the `cli` feature");
}
