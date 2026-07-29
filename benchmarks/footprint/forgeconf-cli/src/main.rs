use forgeconf::{Config, Loader};

#[derive(Config)]
struct Cli {
    #[config(default = "127.0.0.1")]
    host: String,
    #[config(default = 8080)]
    port: u16,
    #[config(default = 4)]
    #[allow(dead_code, reason = "the schema mirrors the clap twin")]
    workers: u32,
    #[config(default = false)]
    debug: bool,
    #[config(default)]
    features: Vec<String>,
}

fn main() {
    let cli = Loader::<Cli>::new().cli().load_or_exit();
    println!(
        "host={} port={} debug={} features={}",
        cli.host,
        cli.port,
        cli.debug,
        cli.features.len()
    );
}
