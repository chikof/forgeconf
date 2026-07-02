# Forgeconf Examples

Run every example from the `forgeconf/` directory: the fixtures are read relative to it.

```bash
cd forgeconf
cargo run --example <name>
```

All examples build with the default features (`toml`, `cli`). Parts that need `json` or `yaml` are compiled only when those features are on, so use `--all-features` to see everything.

## Examples

| Example            | Run                                                      | Shows                                                                                   |
| ------------------ | -------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `basic`            | `cargo run --example basic`                              | `#[derive(Config)]` with a declared file, a default and an optional override file       |
| `nested`           | `cargo run --example nested`                             | Nested structs, `Option<Struct>` sections, `deny_unknown_fields`                        |
| `validators`       | `cargo run --example validators`                         | Built-in and custom validators, a struct-level `validate`, error accumulation           |
| `enums`            | `cargo run --example enums`                              | Unit, externally tagged and internally tagged (`tag = "type"`) enums, unknown variants  |
| `multiple_formats` | `cargo run --example multiple_formats --all-features`    | The same struct from TOML, YAML and JSON, as files and as strings (`from_str`)          |
| `error_handling`   | `cargo run --example error_handling`                     | `ConfigError::kind()`, `errors()`, `path()`, `origin()`, redaction of `sensitive` values |

## Fixtures

`fixtures/` holds the files the examples read:

- `basic.toml`: flat configuration
- `nested.toml`: nested `[database]` and `[logging]` sections
- `validators.toml`: values that satisfy every validator
- `format.toml`, `format.yaml`, `format.json`: the same data in three formats

## Further reading

- The top-level [README](../../README.md) for the attribute reference, loader priorities and supported types.
- The integration tests in [`../tests`](../tests) for more usage patterns.
