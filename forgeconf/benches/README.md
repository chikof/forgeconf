# Forgeconf Benchmarks

This directory contains benchmarks comparing forgeconf with other Rust configuration crates using a clean, modular architecture.

## Quick Start

```bash
cd forgeconf

# Run all benchmarks
cargo bench --bench comparison

# Run specific scenarios
cargo bench --bench comparison simple
cargo bench --bench comparison nested
cargo bench --bench comparison complex

# View HTML report
xdg-open ../target/criterion/report/index.html
```

## Project Structure

```
forgeconf/benches/
├── main.rs                      # Main benchmark runner (~200 lines)
├── comparison/
│   ├── mod.rs                   # Module declarations
│   ├── common.rs                # Shared traits and types
│   ├── forgeconf_impl.rs        # Forgeconf implementations (~220 lines)
│   ├── config_impl.rs           # Config crate implementations (~230 lines)
│   └── yourcrate_impl.rs        # Add your crate here!
├── fixtures/
│   ├── simple.toml              # Simple flat config
│   ├── nested.toml              # Nested multi-level config
│   └── complex.toml             # Complex with arrays
└── README.md                    # This file
```

## Benchmark Scenarios

### 1. Simple Configuration
- **Fixture**: `fixtures/simple.toml`
- **Size**: 5 flat fields (app_name, port, host, debug, max_connections)
- **Use Case**: Basic application settings

### 2. Nested Configuration
- **Fixture**: `fixtures/nested.toml`
- **Size**: ~25 fields across 5 nested sections (server, database, logging, cache, features)
- **Use Case**: Typical microservice configuration

### 3. Complex Configuration
- **Fixture**: `fixtures/complex.toml`
- **Size**: ~50+ fields with arrays and deep nesting
- **Use Case**: Enterprise application with endpoints, replicas, sentinels

## Adding a New Crate

The modular structure makes it easy to add new crates for comparison. Each crate gets its own implementation file.

### Step 1: Add Dependency

Edit `Cargo.toml`:

```toml
[dev-dependencies]
criterion = "0.5"
config = "0.14"
serde = { version = "1.0", features = ["derive"] }
yourcrate = "x.y.z"  # Add this
```

### Step 2: Create Implementation File

Create `benches/comparison/yourcrate_impl.rs`:

```rust
/// YourCrate benchmark implementations

use std::path::Path;
use super::common::{ComplexConfig, ConfigResult, NestedConfig, SimpleConfig};

// Import your crate
use yourcrate::YourConfigLoader;

// ============================================================================
// Simple Configuration
// ============================================================================

pub struct YourCrateSimple {
    app_name: String,
    port: u16,
    host: String,
    debug: bool,
    max_connections: u32,
}

impl SimpleConfig for YourCrateSimple {
    fn from_file(path: &Path) -> ConfigResult<Self> {
        // Load using your crate's API
        // Example: YourConfigLoader::from_file(path)?.parse()
        todo!()
    }

    fn from_str(content: &str) -> ConfigResult<Self> {
        // Parse using your crate's API
        // Example: YourConfigLoader::from_str(content)?.parse()
        todo!()
    }
}

// ============================================================================
// Nested Configuration
// ============================================================================

// Define nested structs matching fixtures/nested.toml
// (See forgeconf_impl.rs or config_impl.rs for examples)

pub struct YourCrateNested {
    // ... fields matching nested.toml
}

impl NestedConfig for YourCrateNested {
    fn from_file(path: &Path) -> ConfigResult<Self> {
        todo!()
    }

    fn from_str(content: &str) -> ConfigResult<Self> {
        todo!()
    }
}

// ============================================================================
// Complex Configuration
// ============================================================================

// Define complex structs matching fixtures/complex.toml
// (See forgeconf_impl.rs or config_impl.rs for examples)

pub struct YourCrateComplex {
    // ... fields matching complex.toml
}

impl ComplexConfig for YourCrateComplex {
    fn from_file(path: &Path) -> ConfigResult<Self> {
        todo!()
    }

    fn from_str(content: &str) -> ConfigResult<Self> {
        todo!()
    }
}
```

### Step 3: Register Module

Edit `benches/comparison/mod.rs`:

```rust
pub mod common;
pub mod config_impl;
pub mod forgeconf_impl;
pub mod yourcrate_impl;  // Add this line

pub use common::{ComplexConfig, ConfigResult, NestedConfig, SimpleConfig};
```

### Step 4: Add to Benchmarks

Edit `benches/main.rs` and add imports at the top:

```rust
// Add to imports section
use comparison::yourcrate_impl::{YourCrateComplex, YourCrateNested, YourCrateSimple};
```

Then add benchmark cases in each of the 6 benchmark functions. Example for `bench_simple_from_file`:

```rust
fn bench_simple_from_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_from_file");
    let path = Path::new(fixtures::SIMPLE);

    // ... existing benchmarks ...

    // ADD YOUR CRATE HERE
    group.bench_with_input(BenchmarkId::new("yourcrate", "simple"), &path, |b, path| {
        b.iter(|| {
            let config = YourCrateSimple::from_file(black_box(path)).unwrap();
            black_box(config);
        });
    });

    group.finish();
}
```

Repeat for all 6 functions:
- `bench_simple_from_file`
- `bench_simple_from_str`
- `bench_nested_from_file`
- `bench_nested_from_str`
- `bench_complex_from_file`
- `bench_complex_from_str`

### Step 5: Run and Verify

```bash
cd forgeconf

# Test compilation
cargo bench --bench comparison --no-run

# Run benchmarks for your crate only
cargo bench --bench comparison yourcrate

# Run all benchmarks
cargo bench --bench comparison
```

## Benefits of Modular Structure

### ✅ Scalability
- Each crate gets its own ~200-300 line implementation file
- Main benchmark runner stays clean (~200 lines)
- No single file grows too large

### ✅ Maintainability
- Changes to one crate don't affect others
- Easy to update or remove crate implementations
- Clear separation of concerns

### ✅ Readability
- Each file has a single, clear purpose
- Easy to find and review specific implementations
- Good for code reviews

### ✅ Parallel Development
- Multiple people can add different crates simultaneously
- No merge conflicts in implementation code
- Only minor conflicts in `main.rs` (easily resolved)

## Common Trait Interface

All implementations must satisfy these traits (defined in `comparison/common.rs`):

```rust
pub trait SimpleConfig: Sized {
    fn from_file(path: &Path) -> ConfigResult<Self>;
    fn from_str(content: &str) -> ConfigResult<Self>;
}

pub trait NestedConfig: Sized {
    fn from_file(path: &Path) -> ConfigResult<Self>;
    fn from_str(content: &str) -> ConfigResult<Self>;
}

pub trait ComplexConfig: Sized {
    fn from_file(path: &Path) -> ConfigResult<Self>;
    fn from_str(content: &str) -> ConfigResult<Self>;
}
```

This ensures:
- Fair comparison (same interface)
- Consistent benchmarking methodology
- Each crate uses its native API internally

## File Size Comparison

### Before (Single File)
- `comparison.rs`: ~600 lines (growing with each crate)

### After (Modular Structure)
- `main.rs`: ~200 lines (stays constant)
- `comparison/mod.rs`: ~10 lines
- `comparison/common.rs`: ~35 lines
- `comparison/forgeconf_impl.rs`: ~220 lines
- `comparison/config_impl.rs`: ~230 lines
- **Per additional crate**: ~200-300 lines in separate file

## Tips for Implementation

1. **Match Fixtures Exactly**: Ensure your structs match the TOML structure
2. **Use Native APIs**: Leverage your crate's strengths
3. **Error Handling**: Use `.unwrap()` in benchmarks (testing happy path)
4. **Documentation**: Add comments explaining non-obvious patterns
5. **Test First**: Verify your implementation loads correctly before benchmarking

## Viewing Results

Criterion generates detailed HTML reports with charts and statistical analysis:

```bash
# After running benchmarks
open ../target/criterion/report/index.html  # macOS
xdg-open ../target/criterion/report/index.html  # Linux
start ../target/criterion/report/index.html  # Windows
```

The report includes:
- Mean execution time
- Standard deviation
- Throughput measurements
- Historical comparisons
- Violin plots and regression charts

## CI Integration

For continuous benchmarking in CI:

```bash
# Save baseline
cargo bench --bench comparison -- --save-baseline main

# Compare against baseline
cargo bench --bench comparison -- --baseline main
```

## Further Reading

- Check `comparison/forgeconf_impl.rs` for a complete example
- Review `comparison/config_impl.rs` for serde-based approach
- Read Criterion docs: https://bheisler.github.io/criterion.rs/book/
