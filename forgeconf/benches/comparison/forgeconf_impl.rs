use std::path::Path;

/// Forgeconf benchmark implementations
///
/// This module contains all forgeconf-specific configuration structures
/// for the three benchmark scenarios: simple, nested, and complex.
use forgeconf::forgeconf;

use super::common::{ComplexConfig, ConfigResult, NestedConfig, SimpleConfig};

// ============================================================================
// Simple Configuration
// ============================================================================

#[forgeconf]
pub struct ForgeconfSimple {
    app_name: String,
    port: u16,
    host: String,
    debug: bool,
    max_connections: u32,
}

impl SimpleConfig for ForgeconfSimple {
    fn from_file(path: &Path) -> ConfigResult<Self> {
        let node = forgeconf::load_from_path(path, Some(forgeconf::FileFormat::Toml))?;
        Ok(Self::load_from(&node)?)
    }

    fn from_str(content: &str) -> ConfigResult<Self> {
        Ok(Self::parse_toml(content)?)
    }
}

// ============================================================================
// Nested Configuration
// ============================================================================

#[forgeconf]
struct ForgeconfServer {
    host: String,
    port: u16,
    max_connections: u32,
    timeout_ms: u32,
}

#[forgeconf]
struct ForgeconfDatabase {
    url: String,
    pool_size: u32,
    max_overflow: u32,
    timeout: u32,
}

#[forgeconf]
struct ForgeconfLogging {
    level: String,
    format: String,
    output: String,
}

#[forgeconf]
struct ForgeconfCache {
    enabled: bool,
    ttl: u32,
    max_size: u32,
}

#[forgeconf]
struct ForgeconfFeatures {
    feature_a: bool,
    feature_b: bool,
    feature_c: bool,
}

#[forgeconf]
pub struct ForgeconfNested {
    app_name: String,
    version: String,
    server: ForgeconfServer,
    database: ForgeconfDatabase,
    logging: ForgeconfLogging,
    cache: ForgeconfCache,
    features: ForgeconfFeatures,
}

impl NestedConfig for ForgeconfNested {
    fn from_file(path: &Path) -> ConfigResult<Self> {
        let node = forgeconf::load_from_path(path, Some(forgeconf::FileFormat::Toml))?;
        Ok(Self::load_from(&node)?)
    }

    fn from_str(content: &str) -> ConfigResult<Self> {
        Ok(Self::parse_toml(content)?)
    }
}

// ============================================================================
// Complex Configuration
// ============================================================================

#[forgeconf]
struct ForgeconfEndpoint {
    path: String,
    methods: Vec<String>,
    auth_required: bool,
    rate_limit: u32,
}

#[forgeconf]
struct ForgeconfComplexServer {
    host: String,
    port: u16,
    max_connections: u32,
    timeout_ms: u32,
    workers: u32,
    endpoints: Vec<ForgeconfEndpoint>,
}

#[forgeconf]
struct ForgeconfReplicas {
    read_1: String,
    read_2: String,
    read_3: String,
}

#[forgeconf]
struct ForgeconfComplexDatabase {
    url: String,
    pool_size: u32,
    max_overflow: u32,
    timeout: u32,
    replicas: ForgeconfReplicas,
}

#[forgeconf]
struct ForgeconfSentinel {
    host: String,
    port: u16,
}

#[forgeconf]
struct ForgeconfRedis {
    host: String,
    port: u16,
    db: u32,
    password: String,
    sentinels: Vec<ForgeconfSentinel>,
}

#[forgeconf]
struct ForgeconfComplexCache {
    enabled: bool,
    ttl: u32,
    max_size: u32,
    redis: ForgeconfRedis,
}

#[forgeconf]
struct ForgeconfSinks {
    console: bool,
    file: bool,
    syslog: bool,
}

#[forgeconf]
struct ForgeconfComplexLogging {
    level: String,
    format: String,
    output: String,
    sinks: ForgeconfSinks,
}

#[forgeconf]
struct ForgeconfMetricsTags {
    service: String,
    version: String,
    environment: String,
}

#[forgeconf]
struct ForgeconfMetrics {
    enabled: bool,
    port: u16,
    path: String,
    tags: ForgeconfMetricsTags,
}

#[forgeconf]
struct ForgeconfComplexFeatures {
    feature_a: bool,
    feature_b: bool,
    feature_c: bool,
    feature_d: bool,
    feature_e: bool,
}

#[forgeconf]
pub struct ForgeconfComplex {
    app_name: String,
    version: String,
    environment: String,
    server: ForgeconfComplexServer,
    database: ForgeconfComplexDatabase,
    cache: ForgeconfComplexCache,
    logging: ForgeconfComplexLogging,
    metrics: ForgeconfMetrics,
    features: ForgeconfComplexFeatures,
}

impl ComplexConfig for ForgeconfComplex {
    fn from_file(path: &Path) -> ConfigResult<Self> {
        let node = forgeconf::load_from_path(path, Some(forgeconf::FileFormat::Toml))?;
        Ok(Self::load_from(&node)?)
    }

    fn from_str(content: &str) -> ConfigResult<Self> {
        Ok(Self::parse_toml(content)?)
    }
}
