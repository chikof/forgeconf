use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long, default_value = "127.0.0.1")]
    host: String,
    #[arg(long, default_value_t = 8080)]
    port: u16,
    #[arg(long, default_value_t = 4)]
    #[allow(dead_code, reason = "the schema mirrors the forgeconf twin")]
    workers: u32,
    #[arg(long)]
    debug: bool,
    #[arg(long)]
    features: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    println!(
        "host={} port={} debug={} features={}",
        cli.host,
        cli.port,
        cli.debug,
        cli.features.len()
    );
}
