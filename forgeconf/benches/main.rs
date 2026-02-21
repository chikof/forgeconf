//! Benchmark suite for comparing forgeconf with other config crates
//!
//! This benchmark compares loading performance across three scenarios:
//! 1. Simple: Flat configuration with basic types
//! 2. Nested: Multi-level configuration with nested structs
//! 3. Complex: Deep nesting with arrays and complex structures
//!
//! Each scenario tests:
//! - File loading (from disk)
//! - String parsing (from in-memory string)
//!
//! ## Adding a New Crate
//!
//! To add a new crate for comparison:
//! 1. Add the crate to `Cargo.toml` under `[dev-dependencies]`
//! 2. Create a new module file: `benches/comparison/yourcrate_impl.rs`
//! 3. Implement the three trait structs: `YourCrateSimple`,
//`YourCrateNested`, !    `YourCrateComplex`
//! 4. Add the module to `benches/comparison/mod.rs`
//! 5. Import and use in the benchmark functions below
//!
//! See `comparison/forgeconf_impl.rs` or `comparison/config_impl.rs` for
//! examples.

#![allow(dead_code, unused_imports)]

use std::fs;
use std::hint::black_box;
use std::path::Path;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

mod comparison;

use comparison::common::fixtures;
// Import implementations
use comparison::config_impl::{ConfigComplex, ConfigNested, ConfigSimple};
use comparison::forgeconf_impl::{ForgeconfComplex, ForgeconfNested, ForgeconfSimple};
use comparison::{ComplexConfig, NestedConfig, SimpleConfig};

// ============================================================================
// Simple Configuration Benchmarks
// ============================================================================

fn bench_simple_from_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_from_file");
    let path = Path::new(fixtures::SIMPLE);

    group.bench_with_input(BenchmarkId::new("forgeconf", "simple"), &path, |b, path| {
        b.iter(|| {
            let config = ForgeconfSimple::from_file(black_box(path)).unwrap();
            black_box(config);
        });
    });

    group.bench_with_input(BenchmarkId::new("config", "simple"), &path, |b, path| {
        b.iter(|| {
            let config = ConfigSimple::from_file(black_box(path)).unwrap();
            black_box(config);
        });
    });

    // To add a new crate, add another bench_with_input here:
    // group.bench_with_input(BenchmarkId::new("yourcrate", "simple"), &path, |b,
    // path| {     b.iter(|| {
    //         let config = YourCrateSimple::from_file(black_box(path)).unwrap();
    //         black_box(config);
    //     });
    // });

    group.finish();
}

fn bench_simple_from_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_from_str");
    let content = fs::read_to_string(fixtures::SIMPLE).expect("Failed to read fixture");

    group.bench_with_input(BenchmarkId::new("forgeconf", "simple"), &content, |b, content| {
        b.iter(|| {
            let config = ForgeconfSimple::from_str(black_box(content)).unwrap();
            black_box(config);
        });
    });

    group.bench_with_input(BenchmarkId::new("config", "simple"), &content, |b, content| {
        b.iter(|| {
            let config = ConfigSimple::from_str(black_box(content)).unwrap();
            black_box(config);
        });
    });

    group.finish();
}

// ============================================================================
// Nested Configuration Benchmarks
// ============================================================================

fn bench_nested_from_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_from_file");
    let path = Path::new(fixtures::NESTED);

    group.bench_with_input(BenchmarkId::new("forgeconf", "nested"), &path, |b, path| {
        b.iter(|| {
            let config = ForgeconfNested::from_file(black_box(path)).unwrap();
            black_box(config);
        });
    });

    group.bench_with_input(BenchmarkId::new("config", "nested"), &path, |b, path| {
        b.iter(|| {
            let config = ConfigNested::from_file(black_box(path)).unwrap();
            black_box(config);
        });
    });

    group.finish();
}

fn bench_nested_from_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("nested_from_str");
    let content = fs::read_to_string(fixtures::NESTED).expect("Failed to read fixture");

    group.bench_with_input(BenchmarkId::new("forgeconf", "nested"), &content, |b, content| {
        b.iter(|| {
            let config = ForgeconfNested::from_str(black_box(content)).unwrap();
            black_box(config);
        });
    });

    group.bench_with_input(BenchmarkId::new("config", "nested"), &content, |b, content| {
        b.iter(|| {
            let config = ConfigNested::from_str(black_box(content)).unwrap();
            black_box(config);
        });
    });

    group.finish();
}

// ============================================================================
// Complex Configuration Benchmarks
// ============================================================================

fn bench_complex_from_file(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_from_file");
    let path = Path::new(fixtures::COMPLEX);

    group.bench_with_input(BenchmarkId::new("forgeconf", "complex"), &path, |b, path| {
        b.iter(|| {
            let config = ForgeconfComplex::from_file(black_box(path)).unwrap();
            black_box(config);
        });
    });

    group.bench_with_input(BenchmarkId::new("config", "complex"), &path, |b, path| {
        b.iter(|| {
            let config = ConfigComplex::from_file(black_box(path)).unwrap();
            black_box(config);
        });
    });

    group.finish();
}

fn bench_complex_from_str(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_from_str");
    let content = fs::read_to_string(fixtures::COMPLEX).expect("Failed to read fixture");

    group.bench_with_input(BenchmarkId::new("forgeconf", "complex"), &content, |b, content| {
        b.iter(|| {
            let config = ForgeconfComplex::from_str(black_box(content)).unwrap();
            black_box(config);
        });
    });

    group.bench_with_input(BenchmarkId::new("config", "complex"), &content, |b, content| {
        b.iter(|| {
            let config = ConfigComplex::from_str(black_box(content)).unwrap();
            black_box(config);
        });
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_simple_from_file,
    bench_simple_from_str,
    bench_nested_from_file,
    bench_nested_from_str,
    bench_complex_from_file,
    bench_complex_from_str,
);

criterion_main!(benches);
