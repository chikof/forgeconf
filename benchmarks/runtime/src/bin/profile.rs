//! Runs one benchmark scenario in a loop for profilers that count work, such as
//! `valgrind --tool=callgrind`. Usage: `profile <scenario> <library>
//! [iterations]`.
#![allow(clippy::unwrap_used, reason = "a profiling driver fails loudly")]

use std::hint::black_box;

use forgeconf::{ConfigFile, FileFormat, Loader};
use forgeconf_benchmarks as rt;
use rt::{AppConfig, fc};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scenario = args.get(1).map_or("json-large", String::as_str);
    let lib = args.get(2).map_or("forgeconf", String::as_str);
    let n: u32 = args.get(3).and_then(|n| n.parse().ok()).unwrap_or(100);
    // SAFETY: single-threaded; nothing else has started.
    unsafe { rt::set_env() };
    let (small, large) = (rt::small(), rt::large());
    let file = rt::TempFile::new("app.toml", rt::SMALL_TOML);
    let mut work = 0usize;
    for _ in 0..n {
        let ok = match (scenario, lib) {
            ("toml-small", "forgeconf") => fc_str(&small.toml, FileFormat::Toml),
            ("toml-large", "forgeconf") => fc_str(&large.toml, FileFormat::Toml),
            ("json-small", "forgeconf") => fc_str(&small.json, FileFormat::Json),
            ("json-large", "forgeconf") => fc_str(&large.json, FileFormat::Json),
            ("yaml-small", "forgeconf") => fc_str(&small.yaml, FileFormat::Yaml),
            ("yaml-large", "forgeconf") => fc_str(&large.yaml, FileFormat::Yaml),
            ("toml-small", "serde") => toml::from_str::<AppConfig>(black_box(&small.toml)).is_ok(),
            ("toml-large", "serde") => toml::from_str::<AppConfig>(black_box(&large.toml)).is_ok(),
            ("json-small", "serde") => {
                serde_json::from_str::<AppConfig>(black_box(&small.json)).is_ok()
            },
            ("json-large", "serde") => {
                serde_json::from_str::<AppConfig>(black_box(&large.json)).is_ok()
            },
            ("layered", "forgeconf") => Loader::<fc::AppConfig>::new()
                .file(ConfigFile::new(black_box(file.path())))
                .load()
                .is_ok(),
            ("errors", "forgeconf") => {
                forgeconf::from_str::<fc::Strict>(black_box(rt::ERROR_TOML), FileFormat::Toml)
                    .map_err(|e| e.to_string())
                    .is_err()
            },
            ("cli", "forgeconf") => Loader::<fc::Cli>::new()
                .cli_args(black_box(rt::CLI_ARGV))
                .load()
                .is_ok(),
            ("yaml-small", "serde") => {
                serde_yaml::from_str::<AppConfig>(black_box(&small.yaml)).is_ok()
            },
            ("yaml-large", "serde") => {
                serde_yaml::from_str::<AppConfig>(black_box(&large.yaml)).is_ok()
            },
            ("layered", "confique") => {
                use confique::Config as _;
                rt::cq::AppConfig::builder()
                    .env()
                    .file(black_box(file.path()))
                    .load()
                    .is_ok()
            },
            ("errors", "serde") => toml::from_str::<rt::Strict>(black_box(rt::ERROR_TOML))
                .map_err(|e| e.to_string())
                .is_err(),
            ("cli", "clap") => {
                use clap::Parser as _;
                rt::cl::Cli::try_parse_from(black_box(rt::CLI_ARGV)).is_ok()
            },
            _ => panic!("unknown scenario {scenario} / {lib}"),
        };
        assert!(ok, "{scenario}/{lib} did not produce its expected outcome");
        work += 1;
    }
    println!("{scenario}/{lib}: {work} iterations");
}

fn fc_str(input: &str, format: FileFormat) -> bool {
    black_box(forgeconf::from_str::<fc::AppConfig>(black_box(input), format)).is_ok()
}
