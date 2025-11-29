# Forgeconf

Forgeconf is a small attribute macro and runtime for loading configuration files into strongly typed Rust structs. It is built for services that need predictable merge semantics, compile-time validation, and the ability to override values from the command line or the environment without sprinkling glue code throughout the application.

## Highlights

- 🧱 **Single source of truth** – annotate your struct once and Forgeconf generates the loader, builder, and conversion logic.
- 🧪 **Compile-time safety** – missing values or type mismatches become compile errors inside the generated code, so you fail fast during development.
- 🔌 **Composable sources** – merge any combination of files, CLI flags, and environment variables with explicit priorities.
- 🧩 **Nested structures** – nested structs can be annotated with `#[forgeconf]` as well, enabling deeply nested configuration trees without boilerplate.
- 🧷 **Format agnostic** – enable just the parsers you need through Cargo features (`toml`, `yaml`, `json`).

## Install

Add Forgeconf to your workspace:

```toml
[dependencies]
forgeconf = "0.1"
```

The crate enables TOML and YAML parsing by default. Add `json` if you want JSON support, or disable defaults to pick a subset:

```toml
[dependencies.forgeconf]
version = "0.1"
default-features = false
features = ["json"]
```

## Quick start

```rust,no_run
use forgeconf::{forgeconf, ConfigError};

#[forgeconf(config(path = "config/app.toml"))]
struct AppConfig {
    #[field(default = 8080)]
    port: u16,
    #[field(env = "APP_DATABASE_URL")]
    database_url: String,
}

fn main() -> Result<(), ConfigError> {
    let cfg = AppConfig::loader()
        .with_config() // load every `config(...)` entry
        .with_cli(200) // merge `--key=value` CLI arguments
        .load()?;

    println!("listening on {}", cfg.port);
    println!("db url: {}", cfg.database_url);
    Ok(())
}
```

## Attribute reference

`#[forgeconf(...)]` accepts zero or more `config(...)` entries. Each entry takes:

| key        | type            | description                                   |
|------------|-----------------|-----------------------------------------------|
| `path`     | string (req.)   | Relative or absolute path to the file         |
| `format`   | `"toml" / ...`  | Overrides format detection                    |
| `priority` | `u8`            | Higher numbers win when merging (default 10)  |

### Field modifiers

Use `#[field(...)]` on struct fields to fine tune the behaviour:

| option        | type         | effect |
|---------------|--------------|--------|
| `name`        | string       | Rename the lookup key                         |
| `insensitive` | bool         | Perform case-insensitive lookups              |
| `env`         | string       | Pull from an environment variable first       |
| `cli`         | string       | Check `--<cli>=value` CLI flags before files  |
| `default`     | expression   | Fall back to the provided literal/expression  |
| `optional`    | bool         | Treat `Option<T>` fields as optional          |

All lookups resolve in the following order:

1. Field-level CLI override (`#[field(cli = "...")]`)
2. Field-level env override (`#[field(env = "...")]`)
3. Sources registered on the loader (`with_cli`, `with_config`, or `add_source`)

### Loader API

The generated `<Struct>Loader` exposes:

- `with_config()` – loads every `config(...)` entry from the attribute.
- `with_cli(priority)` – merges parsed CLI arguments at the provided priority.
- `add_source(source)` – supply any custom `ConfigSource`.
- `load()` – merges the queued sources and deserializes the struct.

You can construct sources manually using items re-exported from the crate:

```rust
let cfg = AppConfig::loader()
    .add_source(forgeconf::ConfigFile::new("settings.toml"))
    .add_source(forgeconf::CliArguments::new().with_args(["--port=9090"]))
    .load()?;
```

## Format support

| Feature | Dependency   | File extensions         |
|---------|--------------|-------------------------|
| `toml`  | `toml` crate | `.toml`                 |
| `yaml`  | `yaml-rust2` | `.yml`, `.yaml`         |
| `json`  | `jzon`       | `.json`                 |

Each parser lives behind a feature flag. Disable defaults if you want to ship with no parsers enabled.

## License

Forgeconf is released under the [MIT License](./LICENSE).
