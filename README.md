# Forgeconf

Forgeconf is a small attribute macro and runtime for loading configuration files into strongly typed Rust structs. It is built for services that need predictable merge semantics, compile-time validation, and the ability to override values from the command line or the environment without sprinkling glue code throughout the application.

## Table of Contents

<!--toc:start-->

- [Forgeconf](#forgeconf)
  - [Highlights](#highlights)
  - [Install](#install)
  - [Quick start](#quick-start)
  - [Attribute reference](#attribute-reference)
    - [Field modifiers](#field-modifiers)
      - [Validators](#validators)
    - [Loader API](#loader-api)
  - [Format support](#format-support)
  - [Releasing](#releasing)
  - [License](#license)
  <!--toc:end-->

## Highlights

- 🧱 **Single source of truth** – annotate your struct once and Forgeconf generates the loader, builder, and conversion logic.
- 🧪 **Compile-time safety** – missing values or type mismatches become compile errors inside the generated code, so you fail fast during development.
- 🔌 **Composable sources** – merge any combination of files, CLI flags, and environment variables with explicit priorities.
- 🧩 **Nested structures** – annotate inner structs with `#[forgeconf]` and mark the field with `#[field(nested)]` to map configuration sub-sections without boilerplate.
- 🧷 **Format agnostic** – enable just the parsers you need through Cargo features (`toml`, `yaml`, `json`).

## Install

Add Forgeconf to your workspace:

```toml
[dependencies]
forgeconf = "0.3"
```

The crate enables TOML, YAML, and regex-powered validators by default. Add `json` if you want JSON support, or disable defaults to pick a subset:

```toml
[dependencies.forgeconf]
version = "0.3"
default-features = false
features = ["json", "regex"]
```

Disable `regex` if you want to skip the `regex` crate entirely, or re-enable it explicitly (as shown above) when using `validators::matches_regex`.

## Quick start

```rust,no_run
use forgeconf::{forgeconf, CliArguments, ConfigError};

#[forgeconf(config(path = "config/app.toml"))]
struct AppConfig {
    #[field(default = 8080)]
    port: u16,
    #[field(env = "APP_DATABASE_URL")]
    database_url: String,
}

fn main() -> Result<(), ConfigError> {
    let cfg = AppConfig::loader()
        .add_source(CliArguments::new().with_priority(200)) // merge `--key=value` CLI arguments
        .load()?;

    println!("listening on {}", cfg.port);
    println!("db url: {}", cfg.database_url);
    Ok(())
}
```

## Attribute reference

`#[forgeconf(...)]` accepts zero or more `config(...)` entries. Each entry takes:

| key        | type           | description                                  |
| ---------- | -------------- | -------------------------------------------- |
| `path`     | string (req.)  | Relative or absolute path to the file        |
| `format`   | `"toml" / ...` | Overrides format detection                   |
| `priority` | `u8`           | Higher numbers win when merging (default 10) |

### Field modifiers

Use `#[field(...)]` on struct fields to fine tune the behaviour:

| option        | type       | effect                                        |
| ------------- | ---------- | --------------------------------------------- |
| `name`        | string     | Rename the lookup key                         |
| `insensitive` | bool       | Perform case-insensitive lookups              |
| `env`         | string     | Pull from an environment variable first       |
| `cli`         | string     | Check `--<cli>=value` CLI flags before files  |
| `default`     | expression | Fall back to the provided literal/expression  |
| `optional`    | bool       | Treat `Option<T>` fields as optional          |
| `validate`    | expression | Invoke a validator after parsing (repeatable) |
| `nested`      | flag       | Treat the field as a nested `#[forgeconf]` struct, resolved from a sub-section of the same name |

All lookups resolve in the following order:

1. Field-level CLI override (`#[field(cli = "...")]`)
2. Field-level env override (`#[field(env = "...")]`)
3. Sources registered on the loader via `add_source`

#### Validators

Validators are plain expressions that evaluate to something callable with `(&T, &str)` and returning `Result<(), ConfigError>`. You can reference free functions, closures, or the helpers under `forgeconf::validators`:

```rust,no_run
fn ensure_https(value: &String, key: &str) -> Result<(), ConfigError> {
    if value.starts_with("https://") {
        Ok(())
    } else {
        Err(ConfigError::mismatch(key, "https url", value.clone()))
    }
}

#[forgeconf]
struct SecureConfig {
    #[field(validate = forgeconf::validators::range(1024, 65535))]
    port: u16,
    #[field(
        validate = ensure_https,
        validate = forgeconf::validators::len_range(12, 128),
        validate = forgeconf::validators::matches_regex(regex::Regex::new("^https://").unwrap()),
    )]
    endpoint: String,
}
```

The most common helpers:

- `non_empty()`, `min_len(n)`, `max_len(n)`, and `len_range(min, max)` – work with any type implementing `validators::HasLen` (Strings, Vecs, maps, sets, …).
- `range(min, max)` – enforce numeric/string bounds via `PartialOrd`.
- `one_of([..])` – restrict values to a predefined set.
- `matches_regex(regex::Regex)` – ensure the value matches a regular expression (enable the `regex` Cargo feature and add the [`regex`](https://crates.io/crates/regex) crate to your `Cargo.toml` when using this helper).

Each helper returns a closure that you can combine or wrap to build higher-level policies.

### Loader API

The generated `<Struct>Loader` exposes:

- `add_source(source)` – supply any custom `ConfigSource` (including `CliArguments`).
- `load()` – merges all sources (including any `config(...)` entries declared on the struct) and deserializes into the struct.

Config files declared with `#[forgeconf(config(path = "..."))]` are loaded automatically when you call `loader()` — no extra call needed. Use `add_source` to layer additional files or CLI arguments on top:

```rust
let cfg = AppConfig::loader()
    .add_source(forgeconf::ConfigFile::new("settings.override.toml"))
    .add_source(forgeconf::CliArguments::new().with_args(["--port=9090"]))
    .load()?;
```

## Format support

| Feature | Dependency   | File extensions |
| ------- | ------------ | --------------- |
| `toml`  | `toml` crate | `.toml`         |
| `yaml`  | `yaml-rust2` | `.yml`, `.yaml` |
| `json`  | `jzon`       | `.json`         |

Each parser lives behind a feature flag. Disable defaults if you want to ship with no parsers enabled.

## Releasing

The repository ships with `scripts/release.sh` to automate version bumps, changelog generation, tagging, and pushes. Requirements:

- `cargo set-version` (`cargo install cargo-edit`)
- `git-cliff`
- Rust nightly toolchain (for formatting) plus the regular stable toolchain

To publish a new release (for example `0.2.1`):

```bash
./scripts/release.sh 0.2.1
```

The script ensures the working tree is clean, bumps every crate in the workspace, regenerates `CHANGELOG.md` through `git-cliff`, runs formatting and tests, commits the results, tags the release (`v0.2.1`), and pushes both the branch and tag. Once the tag hits GitHub, the `release` workflow publishes the crates and attaches the same changelog to the GitHub Release entry.

## License

Forgeconf is released under the [MIT License](./LICENSE).
