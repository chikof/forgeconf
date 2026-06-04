# Rust style

Opinionated and short. **[tool]** rules fail `check` (`scripts/check.sh`), so they are never up for debate. **[review]** rules are on the author and the reviewer. A rule that is not worth enforcing does not belong here.

Formatting is `rustfmt.toml`; lints are the tables in [Lint tables](#lint-tables); thresholds are `clippy.toml`.

## Formatting

- **[tool]** `cargo fmt --all --check`: `max_width = 100`, lowercase hex literals.
- **[review]** One import per path, grouped std, external crates, then `crate`/`super`. No glob imports except `use super::*` in a `mod test`.
- **[review]** Do not hand-format around rustfmt. If it produces something ugly, restructure the code.

## Naming

- **[review]** `UpperCamelCase` types and variants, `snake_case` functions, modules and variables, `SCREAMING_SNAKE_CASE` consts. Acronyms are words: `HttpClient`, not `HTTPClient`.
- **[tool]** `should_implement_trait`, `wrong_self_convention`: `as_` is a free borrow, `to_` an expensive conversion, `into_` consumes `self`, `is_`/`has_` return `bool`.
- **[review]** No `get_` prefix on getters. Name the thing: `len()`, `name()`.
- **[review]** Test module is `mod test` (singular), test names say the behavior: `rejects_empty_name`.

## Modules and files

- **[review]** One primary item per file, named after it: `Foo` lives in `foo.rs`. `lib.rs` and `mod.rs` hold `mod` declarations, re-exports and crate docs only.
- **[tool]** `unreachable_pub`: `pub` only where the API really exposes the item; use `pub(crate)` otherwise.
- **[review]** Unit tests sit in `mod test` at the bottom of the file. Integration tests use only the public API, in `tests/`.

## Types and API

- **[tool]** `ptr_arg`, `borrowed_box`: take `&str`, `&[T]`, `&Path`, not `&String`, `&Vec<T>`, `&PathBuf`, `&Box<T>`.
- **[tool]** `fn_params_excessive_bools`, `struct_excessive_bools`: replace several bools with an enum or a small config struct.
- **[tool]** `new_without_default`, `len_without_is_empty`: pair `new()` with `Default`, `len()` with `is_empty()`.
- **[tool]** `must_use_candidate`: pure functions returning a value say `#[must_use]`.
- **[tool]** `exhaustive_enums`, `exhaustive_structs`: libraries mark public types `#[non_exhaustive]` or keep fields private. Applications allow both (no team boundary).
- **[review]** Make invalid states unrepresentable: newtypes for IDs and units, enums over strings and flags, `NonZero*` over "0 means none".
- **[review]** Parse at the boundary, pass typed values inward. Derive `Debug` on everything; derive `Clone`, `PartialEq`, `Eq`, `Hash`, `Default` only when the semantics hold.
- **[review]** Prefer `impl Trait` and generics for arguments, concrete returns, `dyn Trait` only for real runtime polymorphism.

## Errors and panics

- **[tool]** `unwrap_used`: no `unwrap()` outside tests. `expect("why it cannot fail")` is for invariants the types cannot express.
- **[tool]** `unnecessary_unwrap`: after an `is_some()` check, use `if let` or `match`.
- **[tool]** `missing_errors_doc`, `missing_panics_doc`: every fallible public function documents `# Errors`, every panicking one `# Panics`.
- **[review]** Libraries: one public `Error` enum with `thiserror`, `#[source]` for causes, lowercase messages without a trailing period. Applications: `anyhow` with `.context("what we were doing")` at each boundary.
- **[review]** Panic only for caller bugs and broken invariants, never for bad input or I/O.
- **[review]** Do not swallow errors. `let _ =` needs a comment saying why ignoring is correct.

## Unsafe

- **[tool]** `unsafe_code = "deny"`: allow per module with `#![allow(unsafe_code)]`, a written reason, and review.
- **[tool]** `undocumented_unsafe_blocks`: every `unsafe` block has a `// SAFETY:` comment stating the invariant and why it holds.

## Docs and comments

- **[tool]** `missing_docs`, `RUSTDOCFLAGS="-D warnings"`: public items are documented and intra-doc links resolve.
- **[review]** The first doc line is one sentence in the third person. Add a doctest when the usage is not obvious; use `?` with a hidden `# Ok::<(), Error>(())` tail.
- **[review]** Comments explain why, not what. Delete commented-out code.

## Logging

- **[review]** `tracing` macros only; never `println!` or `eprintln!` for diagnostics (stdout is the program's result). Log fields, not formatted strings: `info!(path = %path.display(), "loaded")`.
- **[review]** Libraries emit events; only the binary installs a subscriber.

## Style

- **[tool]** `wildcard_imports`, `enum_glob_use`: import names explicitly.
- **[tool]** `needless_for_each`: write a `for` loop, not `.for_each(|x| ..)` with side effects.
- **[review]** Prefer iterators and `?` to index loops and nested `match`. Early return over deep nesting.
- **[review]** Dependencies: add one only when a few lines of std cannot do the job (`cargo add`, then `cargo deny check`).

## Memory layout

Size is a design property. Measure with `scripts/type-sizes.sh [filter]`, which prints each workspace type largest first. Sizes below are for 64-bit targets.

- **[tool]** `large_enum_variant` and `variant_size_differences`: the largest variant of an enum may not exceed `enum-variant-size-threshold = 128` bytes in `clippy.toml`. Box the big variant or the big field.
- **[tool]** `large_futures` (`future-size-threshold = 4096`): futures above 4 KiB are `Box::pin`ned before awaiting or spawning.
- **[tool]** `result_large_err` (`large-error-threshold = 128`, on by default): box an error type above 128 B.
- **[tool]** `trivially_copy_pass_by_ref` (`trivial-copy-size-limit = 16`): `Copy` types up to two words go by value. `large_types_passed_by_value` and `needless_pass_by_value` catch the opposite mistakes.
- **[tool]** `large_stack_frames`, `large_stack_arrays`, `large_const_arrays`: big buffers go on the heap or in a `static`.
- **[tool]** `box_collection`, `vec_box`, `redundant_allocation`, `boxed_local`, `unnecessary_box_returns`, `rc_buffer`, `rc_mutex`: no pointless indirection. Use `Rc<str>`, `Box<[T]>`, `Arc<[T]>` instead of `Rc<String>`, `Box<Vec<T>>`, `Arc<Vec<T>>`.
- **[tool]** `option_option`, `ref_option`, `ref_option_ref`: use a two-variant enum, and `Option<&T>` rather than `&Option<T>`.
- **[tool]** `zero_sized_map_values`, `inefficient_to_string`, `non_zero_suggestions`: use a set, avoid `to_string` on `&&str`, prefer `NonZero*`.
- **[tool]** `clippy.toml` sets `avoid-breaking-exported-api = false` in applications, so these lints also check `pub` signatures. The library `clippy.toml` keeps the default (public signatures are the API; private code is still linted).
- **[review]** Use niches: `Option<Box<T>>`, `Option<&T>`, `Option<NonZeroU32>` and `Option<String>` cost no extra byte. A `bool` or enum next to a pointer adds padding; order fields by size, or let rustc reorder (do not use `repr(C)` without need).
- **[review]** Immutable heap text is `Box<str>` (16 B), not `String` (24 B); `Box<[T]>` over `Vec<T>` when the length is fixed after build.
- **[review]** Enum discriminants with fieldless variants over `String` tags; `#[repr(u8)]` only when the layout is part of an interface.
- **[review]** Hot or collection-held types get a size pin below the type:
  `const _: () = assert!(size_of::<Foo>() == 24);`. A changed pin is justified in the task log.

## Lint tables

Application crates (cli, service, workspace) allow `exhaustive_enums` and `exhaustive_structs`; libraries warn. The tables are copied verbatim from each kind's `Cargo.toml`: in a workspace the same keys live under `[workspace.lints.rust]` and `[workspace.lints.clippy]` and members opt in with `[lints] workspace = true`. `check` runs clippy with `-D warnings`, so every `warn` is a failure. Never relax a lint to pass; fix the code or allow it at the item with a reason.

Application (cli, service, workspace):

```toml
[lints.rust]
unsafe_code = "deny"            # allow per module only with a SAFETY comment and review
missing_docs = "warn"           # public items are documented
unreachable_pub = "warn"        # pub only where the API really exposes it
variant_size_differences = "warn" # enum variants of very different size waste space; box the big one

[lints.clippy]
# imports
wildcard_imports = "warn"
enum_glob_use = "warn"
# ownership and signatures
borrowed_box = "warn"
ptr_arg = "warn"
fn_params_excessive_bools = "warn"
struct_excessive_bools = "warn"
# traits
should_implement_trait = "warn"
new_without_default = "warn"
wrong_self_convention = "warn"
len_without_is_empty = "warn"
# errors and panics
unwrap_used = "warn"
unnecessary_unwrap = "warn"
missing_panics_doc = "warn"
missing_errors_doc = "warn"
undocumented_unsafe_blocks = "warn"
# API surface
# Application crate: the exhaustive_* rules only matter across team boundaries.
exhaustive_enums = "allow"
exhaustive_structs = "allow"
must_use_candidate = "warn"
# style
needless_for_each = "warn"
# memory layout and niches (docs/rust-style.md `## Memory layout`, clippy.toml sets the thresholds)
large_enum_variant = "warn"
large_futures = "warn"
large_stack_frames = "warn"
large_stack_arrays = "warn"
large_const_arrays = "warn"
large_types_passed_by_value = "warn"
trivially_copy_pass_by_ref = "warn"
needless_pass_by_value = "warn"
box_collection = "warn"
vec_box = "warn"
redundant_allocation = "warn"
boxed_local = "warn"
unnecessary_box_returns = "warn"
rc_buffer = "warn"
rc_mutex = "warn"
option_option = "warn"
ref_option = "warn"
ref_option_ref = "warn"
zero_sized_map_values = "warn"
inefficient_to_string = "warn"
non_zero_suggestions = "warn"
```

Library (lib):

```toml
[lints.rust]
unsafe_code = "deny"            # allow per module only with a SAFETY comment and review
missing_docs = "warn"           # public items are documented
unreachable_pub = "warn"        # pub only where the API really exposes it
variant_size_differences = "warn" # enum variants of very different size waste space; box the big one

[lints.clippy]
# imports
wildcard_imports = "warn"
enum_glob_use = "warn"
# ownership and signatures
borrowed_box = "warn"
ptr_arg = "warn"
fn_params_excessive_bools = "warn"
struct_excessive_bools = "warn"
# traits
should_implement_trait = "warn"
new_without_default = "warn"
wrong_self_convention = "warn"
len_without_is_empty = "warn"
# errors and panics
unwrap_used = "warn"
unnecessary_unwrap = "warn"
missing_panics_doc = "warn"
missing_errors_doc = "warn"
undocumented_unsafe_blocks = "warn"
# API surface
exhaustive_enums = "warn"
exhaustive_structs = "warn"
must_use_candidate = "warn"
# style
needless_for_each = "warn"
# memory layout and niches (docs/rust-style.md `## Memory layout`, clippy.toml sets the thresholds)
large_enum_variant = "warn"
large_futures = "warn"
large_stack_frames = "warn"
large_stack_arrays = "warn"
large_const_arrays = "warn"
large_types_passed_by_value = "warn"
trivially_copy_pass_by_ref = "warn"
needless_pass_by_value = "warn"
box_collection = "warn"
vec_box = "warn"
redundant_allocation = "warn"
boxed_local = "warn"
unnecessary_box_returns = "warn"
rc_buffer = "warn"
rc_mutex = "warn"
option_option = "warn"
ref_option = "warn"
ref_option_ref = "warn"
zero_sized_map_values = "warn"
inefficient_to_string = "warn"
non_zero_suggestions = "warn"
```
