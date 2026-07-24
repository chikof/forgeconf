# Testing

Forgeconf uses Rust's built-in test runner. Run the full suite with every feature enabled:

```
cargo test --workspace --all-features
```

To check a feature combination on its own (CI runs several):

```
cargo test -p forgeconf --no-default-features --features "json,yaml"
```

Integration tests live under `forgeconf/tests` (fixtures in `forgeconf/tests/fixtures`) and exercise `#[derive(Config)]` and `Loader` through the public API. Unit tests for the runtime live next to their modules inside `forgeconf/src`, and the derive macro has its own unit tests in `forgeconf_macros/src`. Doc tests cover the crate docs and the derive's `compile_fail` cases:

```
cargo test -p forgeconf --all-features --doc
```

The examples under `forgeconf/examples` are built by `cargo test` and must also run from the `forgeconf/` directory: `cargo run -q --example basic`.

The full local gate (format, clippy, docs, tests, dependency and spelling checks) is `scripts/check.sh`.

## Everything: `test-all`

`test-all` (inside `devenv shell`; the script is `scripts/test-all.sh`) runs every check below in order. Name steps to run only those, for example `test-all msrv conformance`.

| Step | What it runs |
|---|---|
| `check` | `scripts/check.sh`, the gate |
| `actionlint` | lints `.github/workflows/*.yml`, shell steps included |
| `msrv` | `cargo test --workspace --all-features` on Rust 1.85 (`$FORGECONF_MSRV_TOOLCHAIN`, kept off `PATH`) |
| `features` | `cargo hack test --feature-powerset` with `-D warnings`: every feature combination builds clean and passes |
| `conformance` | the official TOML 1.0 (toml-test), JSON (JSONTestSuite) and YAML (yaml-test-suite) suites |
| `examples` | runs every example and checks the `server` example's help, env, CLI and exit codes |
| `coverage` | `cargo llvm-cov`, failing under 80 % lines like CI |

The conformance tests live in `forgeconf/tests/conformance_{toml,json,yaml}.rs` and are `#[ignore]`d because they need the suite checkouts that devenv provides through `FORGECONF_TOML_TEST_DIR`, `FORGECONF_JSON_TEST_DIR` and `FORGECONF_YAML_TEST_DIR`. YAML cases outside the documented subset must fail with an `Unsupported` error; JSON cases with duplicate keys are skipped because forgeconf rejects duplicate keys on purpose.
