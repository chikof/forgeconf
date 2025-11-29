# Testing

Forgeconf uses Rust's built-in test runner. Execute the full suite with:

```
cargo test
```

Integration tests live under `forgeconf/tests` and cover the generated loader APIs. Unit tests for the runtime live next to their modules inside `forgeconf_core/src`.
