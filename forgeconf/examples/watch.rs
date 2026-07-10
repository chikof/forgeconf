//! Reload on file changes.
//!
//! Loads a temp file once, rewrites it from a second thread, and prints each
//! reload the watcher delivers.
//!
//! Run with: cargo run --example watch --features watch
#![cfg_attr(not(all(feature = "watch", feature = "toml")), allow(dead_code))]

use forgeconf::Config;

#[derive(Debug, Config)]
struct AppConfig {
    /// Greeting to print.
    greeting: String,
}

#[cfg(all(feature = "watch", feature = "toml"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    use std::{fs, thread};

    use forgeconf::{ConfigFile, Loader};

    let path = std::env::temp_dir().join(format!("forgeconf-watch-{}.toml", std::process::id()));
    fs::write(&path, "greeting = \"hello\"\n")?;

    let loader = Loader::<AppConfig>::new().file(ConfigFile::new(&path));
    println!("loaded: {}", loader.load()?.greeting);
    let watcher = loader.watch(Duration::from_millis(100));

    let writer_path = path.clone();
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(500));
        fs::write(&writer_path, "greeting = \"hello again\"\n")
    });

    let deadline = Instant::now() + Duration::from_secs(3);
    while let Some(left) = deadline.checked_duration_since(Instant::now()) {
        match watcher.recv_timeout(left) {
            Some(Ok(config)) => println!("reloaded: {config:?}"),
            Some(Err(err)) => println!("reload failed: {err}"),
            None => break,
        }
    }

    writer.join().map_err(|_| "writer thread panicked")??;
    fs::remove_file(&path)?;
    Ok(())
}

#[cfg(not(all(feature = "watch", feature = "toml")))]
fn main() {
    println!("Run with: cargo run --example watch --features watch");
}
