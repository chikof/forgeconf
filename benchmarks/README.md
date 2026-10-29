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

Each value is the criterion median in µs, averaged over two full runs taken after the performance work (including the inline `Text` type). Between the two runs the forgeconf medians moved by 4.7 % or less. Treat gaps under about 5 % as no measurable difference. The ratio column is relative to forgeconf, so lower than 1.00 means faster than forgeconf.

| Group | Library | µs | vs forgeconf |
|---|---|---|---|
| `toml/small` | forgeconf | 1.88 | 1.00 |
|  | serde | 2.49 | 1.33 |
|  | config | 5.67 | 3.02 |
|  | figment | 7.58 | 4.04 |
|  | confique | 2.54 | 1.35 |
| `toml/large` | forgeconf | 185 | 1.00 |
|  | serde | 331 | 1.79 |
|  | config | 607 | 3.29 |
|  | figment | 809 | 4.38 |
|  | confique | 336 | 1.82 |
| `json/small` | forgeconf | 1.37 | 1.00 |
|  | serde | 0.39 | 0.28 |
|  | config | 4.03 | 2.94 |
|  | figment | 2.46 | 1.79 |
| `json/large` | forgeconf | 140 | 1.00 |
|  | serde | 118 | 0.85 |
|  | config | 498 | 3.57 |
|  | figment | 340 | 2.43 |
| `yaml/small` | forgeconf | 2.64 | 1.00 |
|  | serde | 5.61 | 2.12 |
|  | config | 8.14 | 3.08 |
|  | figment | 7.76 | 2.94 |
|  | confique | 5.82 | 2.20 |
| `yaml/large` | forgeconf | 375 | 1.00 |
|  | serde | 534 | 1.42 |
|  | config | 813 | 2.16 |
|  | figment | 772 | 2.06 |
|  | confique | 520 | 1.39 |
| `layered/file+env` | forgeconf | 4.49 | 1.00 |
|  | config | 29 | 6.50 |
|  | figment | 31 | 6.82 |
|  | confique | 4.60 | 1.03 |
| `cli/parse` | forgeconf | 1.18 | 1.00 |
|  | clap | 3.13 | 2.66 |
| `cli/help` | forgeconf | 1.76 | 1.00 |
|  | clap | 6.15 | 3.49 |
| `errors/report` | forgeconf | 0.59 | 1.00 |
|  | serde | 0.75 | 1.28 |
|  | config | 1.16 | 1.96 |
|  | figment | 1.69 | 2.85 |

### Footprint

Clean release builds, median of five (wall time, 16 parallel jobs) and of three (one job, so the number is the total compile work). The crate count includes the binary itself. The forgeconf rows were re-measured after the performance work (16 jobs, 3 runs); the one-job column for forgeconf predates it. The inline string type and the faster parsers added about 59 KiB to the forgeconf binary.

| Library | Build s, 16 jobs | Build s, 1 job | Stripped KiB | Crates |
|---|---|---|---|---|
| forgeconf | 2.2 | 4.0 | 582 | 7 |
| config | 2.0 | 7.2 | 736 | 15 |
| figment | 4.0 | 16.3 | 1035 | 25 (26 with build deps) |
| confique | 2.3 | 8.4 | 671 | 19 |
| serde + toml | 2.1 | 6.4 | 641 | 14 |
| forgeconf (CLI only) | 1.9 | 3.5 | 467 | 7 |
| clap | 2.0 | 7.5 | 808 | 18 |

## Reading the numbers

- **forgeconf is the fastest in every group except JSON.** `serde_json` deserializes straight into the struct and stays ahead: 0.39 µs against 1.37 µs on the small document, 118 µs against 140 µs on the large one. forgeconf builds an intermediate tree (now with short strings stored inline, so most values and keys no longer allocate), then moves it into the struct. On `json/large` callgrind counts fewer instructions for forgeconf than for serde_json (2.38 M against 2.60 M), yet serde_json is faster in wall time: building and walking the `BTreeMap` tree costs memory traffic and cache misses that an instruction count does not show [inference: no cycle-level profiler runs here]. Beating serde_json on JSON would need a direct typed parse path that skips the tree.
- **Layered loading is level with confique.** 4.49 µs against 4.60 µs is inside the run-to-run spread. About a quarter of forgeconf's work there is `getenv`: nine lookups, each scanning the whole environment. confique pays the same for the same nine variables.
- **Build time with 16 jobs.** It is nearly the same for everything except figment. Every library compiles `syn` for its derive macro, and that one crate is the longest chain [inference]. The one-job column shows the total compile work, where forgeconf needs about half of what the others need.
- **Errors.** forgeconf reports both problems (the bad value and the unknown key, each with a position). The serde-based libraries stop at the first one, so `errors/report` times more work for forgeconf, and forgeconf is still faster.
- **Where the speed came from.** Byte-wise scanning in every parser, building tables in bulk, an allocation-free environment walk, error text written straight into the formatter, moving values out of the parsed tree instead of cloning them, and the `Text` type, which stores strings of up to 22 bytes inline. Each change was kept only if `bench instructions` improved and no scenario regressed by more than 1 %.

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
