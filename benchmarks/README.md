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

Versions (from `Cargo.lock`): config 0.15.27, figment 0.10.19, confique 0.4.0, serde 1.0.229, toml 1.1.8, serde_json 1.0.154, serde_yaml 0.9.34, clap 4.6.7, criterion 0.8.2.

### Runtime

Each value is the criterion median in µs, averaged over two full runs taken after the performance work. Between the two runs the medians moved by 4.3 % or less for forgeconf and by up to 10 % for some competitors. Treat gaps under about 5 % as no measurable difference. The ratio column is relative to forgeconf, so lower than 1.00 means faster than forgeconf.

| Group | Library | µs | vs forgeconf |
|---|---|---|---|
| `toml/small` | forgeconf | 1.96 | 1.00 |
| | serde | 2.49 | 1.27 |
| | config | 5.67 | 2.90 |
| | figment | 7.79 | 3.98 |
| | confique | 2.55 | 1.30 |
| `toml/large` | forgeconf | 200 | 1.00 |
| | serde | 333 | 1.67 |
| | config | 612 | 3.06 |
| | figment | 824 | 4.12 |
| | confique | 337 | 1.69 |
| `json/small` | forgeconf | 1.39 | 1.00 |
| | serde | 0.40 | 0.29 |
| | config | 3.92 | 2.83 |
| | figment | 2.48 | 1.79 |
| `json/large` | forgeconf | 152 | 1.00 |
| | serde | 120 | 0.78 |
| | config | 501 | 3.29 |
| | figment | 342 | 2.24 |
| `yaml/small` | forgeconf | 2.71 | 1.00 |
| | serde | 5.49 | 2.02 |
| | config | 8.19 | 3.02 |
| | figment | 7.54 | 2.78 |
| | confique | 5.51 | 2.03 |
| `yaml/large` | forgeconf | 384 | 1.00 |
| | serde | 512 | 1.33 |
| | config | 814 | 2.12 |
| | figment | 756 | 1.97 |
| | confique | 519 | 1.35 |
| `layered/file+env` | forgeconf | 4.63 | 1.00 |
| | config | 29 | 6.31 |
| | figment | 30 | 6.43 |
| | confique | 4.58 | 0.99 |
| `cli/parse` | forgeconf | 1.22 | 1.00 |
| | clap | 3.17 | 2.59 |
| `cli/help` | forgeconf | 1.81 | 1.00 |
| | clap | 6.26 | 3.45 |
| `errors/report` | forgeconf | 0.57 | 1.00 |
| | serde | 0.72 | 1.28 |
| | config | 1.14 | 2.01 |
| | figment | 1.68 | 2.97 |

### Footprint

Clean release builds, median of five (wall time, 16 parallel jobs) and of three (one job, so the number is the total compile work). The crate count includes the binary itself. The forgeconf rows were re-measured after the performance work (16 jobs, 3 runs); the one-job column for forgeconf predates it.

| Library | Build s, 16 jobs | Build s, 1 job | Stripped KiB | Crates |
|---|---|---|---|---|
| forgeconf | 2.2 | 4.0 | 555 | 7 |
| config | 2.0 | 7.2 | 736 | 15 |
| figment | 4.0 | 16.3 | 1035 | 25 (26 with build deps) |
| confique | 2.3 | 8.4 | 671 | 19 |
| serde + toml | 2.1 | 6.4 | 641 | 14 |
| forgeconf (CLI only) | 1.9 | 3.5 | 464 | 7 |
| clap | 2.0 | 7.5 | 808 | 18 |

## Reading the numbers

- **forgeconf is the fastest in every group except JSON.** For JSON, `serde_json` deserializes straight into the struct and stays ahead: 0.40 µs against 1.39 µs on the small document, 120 µs against 152 µs on the large one. forgeconf builds an intermediate tree first, with one allocation for every value and key and a position on every node, and then moves that tree into the struct. Callgrind counts on `json/large` put the parser at about 44 % of the work, the conversion at 24 % and freeing the tree at 18 %. Closing the rest of the gap needs a cheaper value representation (for example inline short strings) or a direct typed parse path, both of which change public types.
- **Layered loading is a tie with confique.** 4.63 µs against 4.58 µs is inside the run-to-run spread. About a quarter of forgeconf's work there is `getenv`: nine lookups, each scanning the whole environment, which is large inside `devenv shell`. confique pays the same for the same nine variables.
- **Build time with 16 jobs.** It is nearly the same for everything except figment. Every library compiles `syn` for its derive macro, and that one crate is the longest chain [inference]. The one-job column shows the total compile work, where forgeconf needs about half of what the others need.
- **Errors.** forgeconf reports both problems (the bad value and the unknown key, each with a position). The serde-based libraries stop at the first one, so `errors/report` times more work for forgeconf, and forgeconf is still faster.
- **Where the speed came from.** Byte-wise scanning in every parser, building tables in bulk, an allocation-free environment walk, error text written straight into the formatter, and moving values out of the parsed tree instead of cloning them. Each change was kept only if `bench instructions` improved and no scenario regressed by more than 1 %.

## Known asymmetries

- **confique has no string loader.** In the `toml/*` and `yaml/*` groups, its layer is parsed by `toml` or `serde_yaml`, then passed to `builder().preloaded(...)`. confique also has no JSON support with the features used here, so it is absent from `json/*`.
- **Environment variable names differ.** config and figment use `APP_SERVER__PORT` (`__` separates nesting levels). forgeconf and confique use `APP_SERVER_PORT`. Each library uses its own documented convention, and all of them read the real process environment. confique has no prefix option, so it names a variable on each of the nine fields that forgeconf's `env_prefix` covers. An earlier version named only two, which made it look faster than it is.
- **`serde_yaml` is deprecated upstream.** It is still what figment uses for YAML.
- **Single machine.** Every result comes from one machine with a desktop running. Rerun `bench` on the target hardware before you rely on a close result.

[config]: https://crates.io/crates/config
[figment]: https://crates.io/crates/figment
[confique]: https://crates.io/crates/confique
[toml]: https://crates.io/crates/toml
[serde_json]: https://crates.io/crates/serde_json
[serde_yaml]: https://crates.io/crates/serde_yaml
[clap]: https://crates.io/crates/clap
