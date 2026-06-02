#![allow(dead_code)]

use forgeconf::forgeconf;

// Primary key "port" with alias "port_number" — fixture uses "port" so primary
// wins.
#[forgeconf(config(path = "tests/fixtures/alias.toml"))]
struct AliasConfig {
    #[field(alias = "port_number")]
    port: u16,
    database_url: String,
}

// Fixture only has the alias key, not the primary — alias fallback must kick
// in.
#[forgeconf(config(path = "tests/fixtures/alias_only.toml"))]
struct AliasFallbackConfig {
    #[field(alias = "port_number")]
    port: u16,
    database_url: String,
}

// Multiple aliases — first alias present in fixture is used.
#[forgeconf(config(path = "tests/fixtures/alias_only.toml"))]
struct MultiAliasConfig {
    #[field(alias = "port_number", alias = "p")]
    port: u16,
    database_url: String,
}

// Shorthand #[alias = "..."] syntax.
#[forgeconf(config(path = "tests/fixtures/alias_only.toml"))]
struct AliasShorthandConfig {
    #[alias = "port_number"]
    port: u16,
    database_url: String,
}

#[test]
fn primary_key_takes_precedence_over_alias() {
    let cfg = AliasConfig::loader().load().unwrap();
    assert_eq!(cfg.port, 3000);
}

#[test]
fn alias_used_when_primary_absent() {
    let cfg = AliasFallbackConfig::loader().load().unwrap();
    assert_eq!(cfg.port, 8080);
}

#[test]
fn multi_alias_fallback_chain() {
    let cfg = MultiAliasConfig::loader().load().unwrap();
    assert_eq!(cfg.port, 8080);
}

#[test]
fn alias_shorthand_syntax() {
    let cfg = AliasShorthandConfig::loader().load().unwrap();
    assert_eq!(cfg.port, 8080);
}
