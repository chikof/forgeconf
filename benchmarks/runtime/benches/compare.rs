//! Criterion comparison of forgeconf with config, figment, confique, serde and
//! clap.
#![allow(
    clippy::result_large_err,
    reason = "figment::Error is figment's own error type; boxing it would add work to its numbers"
)]

use std::error::Error;
use std::fmt::{Debug, Display};
use std::hint::black_box;
use std::path::Path;
use std::time::Duration;

use clap::{CommandFactory, Parser};
use confique::Config as _;
use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, criterion_group};
use figment::Figment;
use figment::providers::{Env, Format, Json, Toml, Yaml};
use forgeconf::{ConfigFile, FileFormat, Loader};
use forgeconf_benchmarks as rt;
use rt::{AppConfig, CliArgs, Doc, cl, cq, fc};

type Group<'a> = BenchmarkGroup<'a, WallTime>;

fn run<T, X, E>(g: &mut Group<'_>, lib: &str, expected: &X, f: impl Fn() -> Result<T, E>)
where
    X: From<T> + Debug + PartialEq,
    E: Display,
{
    rt::check(lib, expected, f());
    g.bench_function(lib, |b| b.iter(|| black_box(f())));
}

fn run_err<T, E: Display>(g: &mut Group<'_>, lib: &str, f: impl Fn() -> Result<T, E>) {
    rt::check_err(lib, f());
    g.bench_function(lib, |b| b.iter(|| black_box(f().err().map(|e| e.to_string()))));
}

fn run_text(g: &mut Group<'_>, lib: &str, f: impl Fn() -> String) {
    rt::check_help(lib, &f());
    g.bench_function(lib, |b| b.iter(|| black_box(f())));
}

fn config_str(s: &str, format: config::FileFormat) -> Result<AppConfig, config::ConfigError> {
    config::Config::builder()
        .add_source(config::File::from_str(s, format))
        .build()?
        .try_deserialize()
}

fn toml_group(c: &mut Criterion, name: &str, doc: &Doc) {
    let s = doc.toml.as_str();
    let mut g = c.benchmark_group(name);
    run::<fc::AppConfig, _, _>(&mut g, "forgeconf", &doc.expected, || {
        forgeconf::from_str(black_box(s), FileFormat::Toml)
    });
    run::<AppConfig, _, _>(&mut g, "serde", &doc.expected, || toml::from_str(black_box(s)));
    run(&mut g, "config", &doc.expected, || config_str(black_box(s), config::FileFormat::Toml));
    run::<AppConfig, _, _>(&mut g, "figment", &doc.expected, || {
        Figment::from(Toml::string(black_box(s))).extract()
    });
    // confique has no string loader; its documented way to feed pre-parsed
    // data is `builder().preloaded(layer)`, with the layer parsed by `toml`.
    // The parse is therefore serde's, plus confique's merge and defaults.
    run::<cq::AppConfig, _, _>(
        &mut g,
        "confique",
        &doc.expected,
        || -> Result<_, Box<dyn Error>> {
            let layer = toml::from_str(black_box(s))?;
            Ok(cq::AppConfig::builder().preloaded(layer).load()?)
        },
    );
    g.finish();
}

fn json_group(c: &mut Criterion, name: &str, doc: &Doc) {
    let s = doc.json.as_str();
    let mut g = c.benchmark_group(name);
    run::<fc::AppConfig, _, _>(&mut g, "forgeconf", &doc.expected, || {
        forgeconf::from_str(black_box(s), FileFormat::Json)
    });
    run::<AppConfig, _, _>(&mut g, "serde", &doc.expected, || serde_json::from_str(black_box(s)));
    run(&mut g, "config", &doc.expected, || config_str(black_box(s), config::FileFormat::Json));
    run::<AppConfig, _, _>(&mut g, "figment", &doc.expected, || {
        Figment::from(Json::string(black_box(s))).extract()
    });
    // confique has no JSON loader (only toml/yaml/json5 features): omitted.
    g.finish();
}

fn yaml_group(c: &mut Criterion, name: &str, doc: &Doc) {
    let s = doc.yaml.as_str();
    let mut g = c.benchmark_group(name);
    run::<fc::AppConfig, _, _>(&mut g, "forgeconf", &doc.expected, || {
        forgeconf::from_str(black_box(s), FileFormat::Yaml)
    });
    run::<AppConfig, _, _>(&mut g, "serde", &doc.expected, || serde_yaml::from_str(black_box(s)));
    run(&mut g, "config", &doc.expected, || config_str(black_box(s), config::FileFormat::Yaml));
    run::<AppConfig, _, _>(&mut g, "figment", &doc.expected, || {
        Figment::from(Yaml::string(black_box(s))).extract()
    });
    // Same asymmetry as TOML: confique's layer is parsed by `serde_yaml`.
    run::<cq::AppConfig, _, _>(
        &mut g,
        "confique",
        &doc.expected,
        || -> Result<_, Box<dyn Error>> {
            let layer = serde_yaml::from_str(black_box(s))?;
            Ok(cq::AppConfig::builder().preloaded(layer).load()?)
        },
    );
    g.finish();
}

fn parse(c: &mut Criterion) {
    let (small, large) = (rt::small(), rt::large());
    toml_group(c, "toml/small", &small);
    toml_group(c, "toml/large", &large);
    json_group(c, "json/small", &small);
    json_group(c, "json/large", &large);
    yaml_group(c, "yaml/small", &small);
    yaml_group(c, "yaml/large", &large);
}

// Every library reads the real process env and pays the file read. config and
// figment scan every `APP_*` variable, forgeconf and confique only the ones
// their schema names; that is inherent to each design.
fn layered(c: &mut Criterion) {
    let file = rt::TempFile::new("app.toml", rt::SMALL_TOML);
    let path: &Path = file.path();
    let expected = rt::layered_expected();
    let mut g = c.benchmark_group("layered/file+env");
    run::<fc::AppConfig, _, _>(&mut g, "forgeconf", &expected, || {
        Loader::<fc::AppConfig>::new()
            .file(ConfigFile::new(black_box(path)))
            .load()
    });
    run(&mut g, "config", &expected, || -> Result<AppConfig, config::ConfigError> {
        config::Config::builder()
            .add_source(config::File::from(black_box(path)))
            .add_source(
                config::Environment::with_prefix(rt::ENV_PREFIX)
                    .prefix_separator("_")
                    .separator("__"),
            )
            .build()?
            .try_deserialize()
    });
    run::<AppConfig, _, _>(&mut g, "figment", &expected, || {
        Figment::from(Toml::file(black_box(path)))
            .merge(Env::prefixed("APP_").split("__"))
            .extract()
    });
    run::<cq::AppConfig, _, _>(&mut g, "confique", &expected, || {
        cq::AppConfig::builder().env().file(black_box(path)).load()
    });
    g.finish();
}

fn cli_parse(c: &mut Criterion) {
    let expected = rt::cli_expected();
    let mut g = c.benchmark_group("cli/parse");
    run::<fc::Cli, CliArgs, _>(&mut g, "forgeconf", &expected, || {
        Loader::<fc::Cli>::new()
            .cli_args(black_box(rt::CLI_ARGV))
            .load()
    });
    run::<cl::Cli, CliArgs, _>(&mut g, "clap", &expected, || {
        cl::Cli::try_parse_from(black_box(rt::CLI_ARGV))
    });
    g.finish();
}

fn cli_help(c: &mut Criterion) {
    let mut g = c.benchmark_group("cli/help");
    run_text(&mut g, "forgeconf", || {
        match Loader::<fc::Cli>::new()
            .cli_args(black_box(["app", "--help"]))
            .load()
        {
            Err(err) if err.is_help() => err.to_string(),
            other => panic!("forgeconf: expected a help error, got {other:?}"),
        }
    });
    run_text(&mut g, "clap", || black_box(cl::Cli::command()).render_help().to_string());
    g.finish();
}

fn errors(c: &mut Criterion) {
    let s = rt::ERROR_TOML;
    let mut g = c.benchmark_group("errors/report");
    run_err(&mut g, "forgeconf", || {
        forgeconf::from_str::<fc::Strict>(black_box(s), FileFormat::Toml)
    });
    run_err(&mut g, "serde", || toml::from_str::<rt::Strict>(black_box(s)));
    run_err(&mut g, "config", || -> Result<rt::Strict, config::ConfigError> {
        config::Config::builder()
            .add_source(config::File::from_str(black_box(s), config::FileFormat::Toml))
            .build()?
            .try_deserialize()
    });
    run_err(&mut g, "figment", || {
        Figment::from(Toml::string(black_box(s))).extract::<rt::Strict>()
    });
    g.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(3))
        .warm_up_time(Duration::from_secs(1));
    targets = parse, layered, cli_parse, cli_help, errors
}

fn main() {
    // SAFETY: first statement of main; criterion has spawned no thread yet.
    unsafe { rt::set_env() };
    benches();
    Criterion::default().configure_from_args().final_summary();
}
