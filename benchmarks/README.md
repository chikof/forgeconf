# Benchmarks

These benchmarks compare forgeconf with [config], [figment], [confique], plain serde ([toml], [serde_json], [serde_yaml]) and, for the command line, [clap].

This directory is a separate cargo workspace with its own `Cargo.lock`. It is never published, and the forgeconf workspace excludes it. Every competitor depends on serde, which forgeconf bans from its own dependency graph.

## Run

Inside `devenv shell`:

```sh
bench                      # runtime and footprint
bench runtime              # criterion benches only; arguments after -- go to criterion
bench footprint            # build time, binary size, crate count
FOOTPRINT_RUNS=5 bench footprint
CARGO_BUILD_JOBS=1 bench footprint   # total compile work instead of wall time
```

`test-all benchmarks` builds every bench, runs each one once with its correctness checks, and verifies the footprint binaries without timing them.

## What is measured

All libraries load the same `AppConfig`: a name, a flag, a list, two nested tables and a map (`fixtures/app.toml`, `.json`, `.yaml`). The large documents add 1000 list entries and 1000 map entries.

| Group | Work in the timed region |
|---|---|
| `toml/*`, `json/*`, `yaml/*` | in-memory text to the typed struct |
| `layered/file+env` | read the TOML file from disk, apply two environment overrides, produce the struct |
| `cli/parse` | parse `--host 10.0.0.1 --port 9090 --debug --features a --features b` into a flat struct |
| `cli/help` | render the `--help` text |
| `errors/report` | load a document with a bad value and an unknown key, then format the error |

Every library's output is checked against the same expected value before any timing starts. A library that errors or returns the wrong data aborts the run, so it never reports a number.

The footprint crates (`footprint/*`) are minimal binaries that load the fixture file plus environment overrides, or parse the command line. Each one is built clean in release mode, stripped, and run to check its output.

## Results

Recorded on 2026-11-02 with rustc 1.101.0-nightly on a 16-core x86_64 Linux machine. The machine was not idle: load average was 2 to 4 from desktop processes, which is under the core count.

### Runtime

Each value is the criterion median in µs, averaged over two full runs. Between the two runs the medians moved by 3.6 % or less, except `yaml/large/serde` (10.5 %). Treat gaps under about 5 % as no measurable difference. The ratio column is relative to forgeconf, so lower than 1.00 means faster than forgeconf.

| Group | Library | µs | vs forgeconf |
|---|---|---|---|
| `toml/small` | forgeconf | 3.01 | 1.00 |
| | serde | 2.47 | 0.82 |
| | config | 5.69 | 1.89 |
| | figment | 7.99 | 2.65 |
| | confique | 2.52 | 0.84 |
| `toml/large` | forgeconf | 356 | 1.00 |
| | serde | 331 | 0.93 |
| | config | 607 | 1.71 |
| | figment | 855 | 2.40 |
| | confique | 334 | 0.94 |
| `json/small` | forgeconf | 2.10 | 1.00 |
| | serde | 0.40 | 0.19 |
| | config | 3.98 | 1.90 |
| | figment | 2.45 | 1.17 |
| `json/large` | forgeconf | 324 | 1.00 |
| | serde | 118 | 0.37 |
| | config | 507 | 1.57 |
| | figment | 345 | 1.07 |
| `yaml/small` | forgeconf | 2.87 | 1.00 |
| | serde | 5.44 | 1.90 |
| | config | 8.36 | 2.91 |
| | figment | 7.82 | 2.72 |
| | confique | 5.47 | 1.91 |
| `yaml/large` | forgeconf | 431 | 1.00 |
| | serde | 523 | 1.21 |
| | config | 819 | 1.90 |
| | figment | 789 | 1.83 |
| | confique | 547 | 1.27 |
| `layered/file+env` | forgeconf | 7.10 | 1.00 |
| | config | 29 | 4.15 |
| | figment | 29 | 4.09 |
| | confique | 3.90 | 0.55 |
| `cli/parse` | forgeconf | 1.62 | 1.00 |
| | clap | 3.11 | 1.92 |
| `cli/help` | forgeconf | 1.85 | 1.00 |
| | clap | 6.34 | 3.43 |
| `errors/report` | forgeconf | 0.95 | 1.00 |
| | serde | 0.75 | 0.79 |
| | config | 1.14 | 1.20 |
| | figment | 1.69 | 1.78 |

### Footprint

Clean release builds, median of five (wall time, 16 parallel jobs) and of three (one job, so the number is the total compile work). The crate count includes the binary itself.

| Library | Build s, 16 jobs | Build s, 1 job | Stripped KiB | Crates |
|---|---|---|---|---|
| forgeconf | 2.1 | 4.0 | 523 | 7 |
| config | 2.0 | 7.2 | 736 | 15 |
| figment | 4.0 | 16.3 | 1035 | 25 (26 with build deps) |
| confique | 2.3 | 8.4 | 671 | 19 |
| serde + toml | 2.1 | 6.4 | 641 | 14 |
| forgeconf (CLI only) | 1.9 | 3.5 | 456 | 7 |
| clap | 2.0 | 7.5 | 808 | 18 |

## Reading the numbers

- **Build time with 16 jobs.** It is nearly the same for everything except figment. Every library compiles `syn` for its derive macro, and that one crate is the longest chain [inference]. The one-job column shows the total compile work, where forgeconf needs about half of what the others need.
- **JSON.** forgeconf is 2.7 times slower than `serde_json` and about level with figment. forgeconf first builds a `ConfigNode` tree, which allocates a `String` for every value and a map for every table, then converts that tree into the struct. serde_json deserializes straight into the struct [inference: not profiled]. On the 2 µs small document this does not matter at start-up. It is the first place to look if parsing speed ever matters.
- **TOML and YAML.** forgeconf is within about 8 % of serde and confique for TOML, and faster than every serde-based option for YAML.
- **Layered loading.** confique is faster because it reads only the two variables that carry an `env` attribute. forgeconf looks up one variable for every field that `env_prefix` covers (9 here), then merges the layers and checks for deprecated keys. config and figment scan every `APP_*` variable and go through serde's `Value` types.
- **Errors.** forgeconf reports both problems (the bad value and the unknown key, each with a position). The serde-based libraries stop at the first one, so `errors/report` times different amounts of work.

## Known asymmetries

- **confique has no string loader.** In the `toml/*` and `yaml/*` groups, its layer is parsed by `toml` or `serde_yaml`, then passed to `builder().preloaded(...)`. confique also has no JSON support with the features used here, so it is absent from `json/*`.
- **Environment variable names differ.** config and figment use `APP_SERVER__PORT` (`__` separates nesting levels). forgeconf and confique use `APP_SERVER_PORT`. Each library uses its own documented convention, and all of them read the real process environment.
- **`serde_yaml` is deprecated upstream.** It is still what figment uses for YAML.
- **Single machine.** Every result comes from one machine with a desktop running. Rerun `bench` on the target hardware before you rely on a close result.

[config]: https://crates.io/crates/config
[figment]: https://crates.io/crates/figment
[confique]: https://crates.io/crates/confique
[toml]: https://crates.io/crates/toml
[serde_json]: https://crates.io/crates/serde_json
[serde_yaml]: https://crates.io/crates/serde_yaml
[clap]: https://crates.io/crates/clap
