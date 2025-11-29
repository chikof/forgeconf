# Contributing to Forgeconf

Thanks for your interest in the project! Forgeconf is intentionally small and opinionated, but community feedback and patches are very welcome. This short guide explains how to get involved.

## Reporting bugs or requesting features

- Open an issue on the tracker with a clear description, reproduction steps, and expected outcome.
- If you are proposing a feature, describe the problem you are trying to solve, the proposed API or behaviour, and any trade-offs you have considered.
- Please search existing issues before filing a new one to avoid duplicates.

## Sending pull requests

1. Fork the repository and create a branch describing your change.
2. Write tests that cover the behaviour you are adding or changing.
3. Run `cargo fmt` and `cargo test` locally before opening the PR.
4. Include a short summary of the change and any open questions in the PR description.

If you are working on a large feature, open an issue first so we can discuss the design and avoid wasted effort.

## Running the project

```
cargo fmt
cargo clippy --all-targets
cargo test
```

No additional tooling is required; everything builds with the stable toolchain.

## Community expectations

Please review the [Code of Conduct](./CODE_OF_CONDUCT.md). To report behaviour that violates the Code of Conduct, join the Discord server at <https://discord.gg/sejc7TnX6N> and contact a moderator.
