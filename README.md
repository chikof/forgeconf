# Forgeconf

Forgeconf loads configuration files into strongly typed Rust structs. Derive `Config`, point it at one or more files, and get merged, validated values back with every error reported at once, including its key path and the file line and column it came from.

Forgeconf has no third-party runtime dependencies: the TOML, JSON and YAML parsers are in-house, and the derive macro needs only `syn`, `quote` and `proc-macro2`.

## Table of Contents

<!--toc:start-->

- [Forgeconf](#forgeconf)
  - [Install](#install)
  - [Quick start](#quick-start)
  - [Derive attributes](#derive-attributes)
  - [Loader and priorities](#loader-and-priorities)
  - [Supported types](#supported-types)
  - [Validators](#validators)
  - [Errors](#errors)
  - [Features](#features)
  - [Performance](#performance)
  - [Examples](#examples)
  - [Releasing](#releasing)
  - [License](#license)
  <!--toc:end-->

## Install

```toml
[dependencies]
forgeconf = "1"
```

Default features are `toml` and `cli`. Enable `json` or `yaml` for those formats, or disable defaults to pick a subset:

```toml
[dependencies.forgeconf]
version = "1"
default-features = false
features = ["json", "yaml"]
```

The minimum supported Rust version is 1.85.

## Quick start

```rust,no_run
use forgeconf::Config;

#[derive(Debug, Config)]
#[config(file = "config/app.toml")]
struct AppConfig {
    #[config(default = 8080)]
    port: u16,
    database_url: String,
}

fn main() -> Result<(), forgeconf::ConfigError> {
    let cfg = AppConfig::loader().load()?;
    println!("listening on {}", cfg.port);
    println!("db url: {}", cfg.database_url);
    Ok(())
}
```

`Config` is both a trait and its derive, like `serde::Serialize`. `AppConfig::loader()` returns a `Loader<AppConfig>` preloaded with the declared files. Returning `Result<(), ConfigError>` from `main` prints the full error report.

## Derive attributes

The helper attribute is `config`. Invalid attributes produce spanned compile errors, all at once. Generic types, lifetimes, unions and unit structs are rejected.

**Struct**

| attribute                   | effect                                                                                                              |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| `file = "path"`             | Add a config file (repeatable, declaration order)                                                                   |
| `file(path = EXPR, ...)`    | Long form: `format = "toml"/"json"/"yaml"/"yml"`, `priority = u8` (default 10), `optional` (missing file is empty) |
| `env_prefix = "APP_"`       | Prefix for derived environment variable names                                                                       |
| `rename_all = "..."`        | Key style: `lowercase`, `UPPERCASE`, `snake_case`, `kebab-case`, `camelCase`, `PascalCase`, `SCREAMING_SNAKE_CASE`, `SCREAMING-KEBAB-CASE` |
| `deny_unknown_fields`       | Unknown keys are errors, with a "did you mean" hint                                                                 |
| `validate = EXPR`           | `fn(&Self) -> Result<(), ConfigError>`, runs after every field loaded (repeatable)                                  |

**Field**

| attribute                    | effect                                                                             |
| ---------------------------- | ---------------------------------------------------------------------------------- |
| `rename = "key"`             | Explicit key, wins over `rename_all`                                               |
| `alias = "key"`              | Extra accepted key (repeatable); the primary key wins when several are present     |
| `default` / `default = EXPR` | Value when the key is absent; bare `default` uses `Default::default()`; a string literal goes through `Into` |
| `insensitive`                | Match the key ignoring ASCII case                                                  |
| `validate = EXPR`            | `fn(&FieldType) -> Result<(), ConfigError>` (repeatable)                           |
| `sensitive`                  | Redact the value in errors; the derive also implements `Debug` (prints `***`)      |
| `deprecated` / `deprecated = "msg"` | Warn once when the key is present                                           |
| `skip`                       | Not loaded; the field is `Default::default()`                                      |
| `flatten`                    | The field type's keys live at this level                                           |
| `env = "VAR"`                | Explicit environment variable name                                                 |
| `flag = "name"`, `short = 'c'`, `skip_cli` | Command-line flag names, or hide the field from the command line     |

`Option<T>` and `Vec<T>` fields are optional without `default`: `None` and an empty list.

**Enum and variant**

| attribute                    | effect                                                                              |
| ---------------------------- | ----------------------------------------------------------------------------------- |
| `rename_all = "..."`         | Key style for variant names                                                         |
| `tag = "type"`               | Internally tagged: a table whose `type` key selects the variant                     |
| `rename = "key"`, `alias = "key"` | On a variant: its key and extra accepted keys                                  |

Without `tag`, a unit variant is a plain string and a data variant is a table with exactly one key naming the variant. A newtype struct (`struct Port(u16);`) loads as its inner type.

The full reference is on the derive in the [`forgeconf_macros`](https://docs.rs/forgeconf_macros) docs.

## Loader and priorities

```rust,ignore
let cfg = AppConfig::loader()
    .file(ConfigFile::new("settings.override.toml").optional())
    .load()?;
```

`Loader` methods: `file(ConfigFile)`, `source(impl ConfigSource)`, `on_warning(handler)` (deprecations; the default prints `warning: ...` to stderr), `env_vars(map)` (replace the process environment, for tests) and `load()`. `ConfigFile::new(path)` takes `.format(FileFormat)`, `.priority(u8)` and `.optional()`; the format comes from the extension otherwise.

Sources merge in ascending priority; equal priorities keep insertion order and the later one wins:

| source                                   | priority |
| ---------------------------------------- | -------- |
| declared or added files (`ConfigFile`)   | 10       |
| custom `ConfigSource` (default)          | 100      |
| environment variables                    | 200      |
| command-line flags                       | 250      |

Tables merge key by key, arrays are replaced rather than appended, and a null value never overrides a lower source. `forgeconf::from_str::<T>(text, FileFormat::Toml)` converts a string directly.

Environment variables, command-line flags, `.env` files and hot reload are layered on top of this and are all derived from the struct's shape:

| capability | feature            | description                                                                          |
| ---------- | ------------------ | ------------------------------------------------------------------------------------ |
| environment | always on         | `env = "VAR"` per field, or names derived from `env_prefix`                          |
| CLI        | `cli` (default)    | Generated flags and `--help`; `Loader::cli()` and `Loader::load_or_exit()`           |
| dotenv     | `dotenv`           | `Loader::dotenv(path)` reads a `.env` file below the real environment                |
| watch      | `watch`            | `Loader::watch(interval)` polls the config files and sends fresh values              |

## Supported types

- Strings and chars: `String`, `char`, `Box<str>`, `Arc<str>`, `PathBuf`.
- Numbers and bools: all integer and float types, `NonZero*`, `bool` (`true/false/1/0/yes/no/on/off`, case-insensitive).
- `std::time::Duration`: seconds (`30`, `1.5`) or units (`250ms`, `1h30m`, `2d 4h`).
- Network: `IpAddr`, `Ipv4Addr`, `Ipv6Addr`, `SocketAddr` and its v4/v6 forms.
- Collections: `Vec`, `VecDeque`, `HashSet`, `BTreeSet`, `[T; N]`, `HashMap<K, V>`, `BTreeMap<K, V>` (keys implement `FromStr`).
- Wrappers: `Option<T>`, `Box<T>`, `Arc<T>`.
- Your own structs and enums with `#[derive(Config)]`, and `ConfigNode` for raw pass-through.
- Anything else: implement `FromNode`.

## Validators

A validator is any callable taking `&T` and returning `Result<(), ConfigError>`: a function, a closure, or a helper from `forgeconf::validators`.

```rust,ignore
use forgeconf::{Config, ConfigError};

fn ensure_https(url: &String) -> Result<(), ConfigError> {
    if url.starts_with("https://") { Ok(()) } else { Err(ConfigError::invalid("must be https")) }
}

#[derive(Config)]
struct Secure {
    #[config(validate = forgeconf::validators::range(1024..=65535))]
    port: u16,
    #[config(validate = ensure_https, validate = forgeconf::validators::len(12..=128))]
    endpoint: String,
}
```

Helpers: `non_empty()`, `len(range)` for anything with a length (`String`, `Vec`, maps, sets), `range(range)` for `PartialOrd` values, and `one_of(values)`.

## Errors

Every field is checked and all failures come back together as one `ConfigError`:

```text
2 configuration errors:
  - port: expected u16, found "abc" (config/app.toml:1:8)
  - hots: unknown key, did you mean "host"? (config/app.toml:2:1)
```

Use `error.kind()` (`ErrorKind::Missing`, `Mismatch`, `Invalid`, `UnknownKey`, `UnknownVariant`, `Syntax`, `Io`, ...), `error.path()` (`server.port`, `servers[2].host`), `error.origin()` (file, line, column) and `error.errors()` to iterate the individual errors.

## Features

| feature  | default | description                                         |
| -------- | ------- | --------------------------------------------------- |
| `toml`   | yes     | TOML 1.0 parser, `.toml` files                      |
| `cli`    | yes     | Command-line source with generated `--help`         |
| `json`   | no      | JSON parser, `.json` files                          |
| `yaml`   | no      | YAML (common subset) parser, `.yaml` / `.yml` files |
| `dotenv` | no      | `.env` file support                                 |
| `watch`  | no      | Polling hot reload                                  |

## Performance

forgeconf has no dependencies besides its derive macro. Loading a typical file plus environment variables takes a few microseconds, so in a normal program it is a tiny fraction of process start-up. [`benchmarks/`](./benchmarks) compares speed, build time, binary size and crate count with config, figment, confique, serde and clap.

Settings in your own application that make forgeconf, and the rest of your code, faster:

- `lto = true` (or `"fat"`) and `codegen-units = 1` in `[profile.release]`. In our benchmarks this removed 1 % to 11 % of the instructions forgeconf runs.
- `panic = "abort"`, if your application does not rely on unwinding.
- Profile-guided optimisation, or a faster global allocator, if configuration is loaded very often, for example in tests.

forgeconf does not set any of these for you: they are decisions for the final binary.

## Examples

Runnable examples live in [`forgeconf/examples`](./forgeconf/examples): `basic`, `nested`, `validators`, `enums`, `multiple_formats` and `error_handling`.

## Releasing

The repository ships with `scripts/release.sh` to automate version bumps, changelog generation, tagging, and pushes. Requirements:

- `cargo set-version` (`cargo install cargo-edit`)
- `git-cliff`
- Rust nightly toolchain (for formatting) plus the regular stable toolchain

To publish a new release (for example `1.0.1`):

```bash
./scripts/release.sh 1.0.1
```

The script ensures the working tree is clean, bumps every crate in the workspace, regenerates `CHANGELOG.md` through `git-cliff`, runs formatting and tests, commits the results, tags the release (`v1.0.1`), and pushes both the branch and tag. Once the tag hits GitHub, the `release` workflow publishes the crates and attaches the same changelog to the GitHub Release entry.

## License

Forgeconf is released under the [MIT License](./LICENSE).
