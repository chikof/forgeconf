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
