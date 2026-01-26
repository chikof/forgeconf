use std::path::Path;

/// Config crate benchmark implementations
///
/// This module contains all config-crate-specific configuration structures
/// for the three benchmark scenarios: simple, nested, and complex.
use config::{Config, File};
use serde::Deserialize;

use super::common::{ComplexConfig, ConfigResult, NestedConfig, SimpleConfig};

// ============================================================================
// Simple Configuration
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ConfigSimple {
    app_name: String,
    port: u16,
    host: String,
    debug: bool,
    max_connections: u32,
}

impl SimpleConfig for ConfigSimple {
    fn from_file(path: &Path) -> ConfigResult<Self> {
        let config = Config::builder()
            .add_source(File::from(path))
            .build()?;
        Ok(config.try_deserialize()?)
    }

    fn from_str(content: &str) -> ConfigResult<Self> {
        let config = Config::builder()
            .add_source(File::from_str(content, config::FileFormat::Toml))
            .build()?;
        Ok(config.try_deserialize()?)
    }
}

// ============================================================================
// Nested Configuration
// ============================================================================

#[derive(Debug, Deserialize)]
struct ConfigServer {
    host: String,
    port: u16,
    max_connections: u32,
    timeout_ms: u32,
}

#[derive(Debug, Deserialize)]
struct ConfigDatabase {
    url: String,
    pool_size: u32,
    max_overflow: u32,
    timeout: u32,
}

#[derive(Debug, Deserialize)]
struct ConfigLogging {
    level: String,
    format: String,
    output: String,
}

#[derive(Debug, Deserialize)]
struct ConfigCache {
    enabled: bool,
    ttl: u32,
    max_size: u32,
}

#[derive(Debug, Deserialize)]
struct ConfigFeatures {
    feature_a: bool,
    feature_b: bool,
    feature_c: bool,
}

#[derive(Debug, Deserialize)]
pub struct ConfigNested {
    app_name: String,
    version: String,
    server: ConfigServer,
    database: ConfigDatabase,
    logging: ConfigLogging,
    cache: ConfigCache,
    features: ConfigFeatures,
}

impl NestedConfig for ConfigNested {
    fn from_file(path: &Path) -> ConfigResult<Self> {
        let config = Config::builder()
            .add_source(File::from(path))
            .build()?;
        Ok(config.try_deserialize()?)
    }

    fn from_str(content: &str) -> ConfigResult<Self> {
        let config = Config::builder()
            .add_source(File::from_str(content, config::FileFormat::Toml))
            .build()?;
        Ok(config.try_deserialize()?)
    }
}

// ============================================================================
// Complex Configuration
// ============================================================================

#[derive(Debug, Deserialize)]
struct ConfigEndpoint {
    path: String,
    methods: Vec<String>,
    auth_required: bool,
    rate_limit: u32,
}

#[derive(Debug, Deserialize)]
struct ConfigComplexServer {
    host: String,
    port: u16,
    max_connections: u32,
    timeout_ms: u32,
    workers: u32,
    endpoints: Vec<ConfigEndpoint>,
}

#[derive(Debug, Deserialize)]
struct ConfigReplicas {
    read_1: String,
    read_2: String,
    read_3: String,
}

#[derive(Debug, Deserialize)]
struct ConfigComplexDatabase {
    url: String,
    pool_size: u32,
    max_overflow: u32,
    timeout: u32,
    replicas: ConfigReplicas,
}

#[derive(Debug, Deserialize)]
struct ConfigSentinel {
    host: String,
    port: u16,
}

#[derive(Debug, Deserialize)]
struct ConfigRedis {
    host: String,
    port: u16,
    db: u32,
    password: String,
    sentinels: Vec<ConfigSentinel>,
}

#[derive(Debug, Deserialize)]
struct ConfigComplexCache {
    enabled: bool,
    ttl: u32,
    max_size: u32,
    redis: ConfigRedis,
}

#[derive(Debug, Deserialize)]
struct ConfigSinks {
    console: bool,
    file: bool,
    syslog: bool,
}

#[derive(Debug, Deserialize)]
struct ConfigComplexLogging {
    level: String,
    format: String,
    output: String,
    sinks: ConfigSinks,
}

#[derive(Debug, Deserialize)]
struct ConfigMetricsTags {
    service: String,
    version: String,
    environment: String,
}

#[derive(Debug, Deserialize)]
struct ConfigMetrics {
    enabled: bool,
    port: u16,
    path: String,
    tags: ConfigMetricsTags,
}

#[derive(Debug, Deserialize)]
struct ConfigComplexFeatures {
    feature_a: bool,
    feature_b: bool,
    feature_c: bool,
    feature_d: bool,
    feature_e: bool,
}

#[derive(Debug, Deserialize)]
pub struct ConfigComplex {
    app_name: String,
    version: String,
    environment: String,
    server: ConfigComplexServer,
    database: ConfigComplexDatabase,
    cache: ConfigComplexCache,
    logging: ConfigComplexLogging,
    metrics: ConfigMetrics,
    features: ConfigComplexFeatures,
}

impl ComplexConfig for ConfigComplex {
    fn from_file(path: &Path) -> ConfigResult<Self> {
        let config = Config::builder()
            .add_source(File::from(path))
            .build()?;
        Ok(config.try_deserialize()?)
    }

    fn from_str(content: &str) -> ConfigResult<Self> {
        let config = Config::builder()
            .add_source(File::from_str(content, config::FileFormat::Toml))
            .build()?;
        Ok(config.try_deserialize()?)
    }
}
