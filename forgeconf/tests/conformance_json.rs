//! JSONTestSuite conformance: `y_` parse, `n_` rejected, `i_` never panic.
#![cfg(feature = "json")]
#![allow(clippy::unwrap_used, reason = "tests fail loudly")]

use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

use forgeconf::{FileFormat, parse_str};

#[derive(Clone, Copy, Eq, PartialEq)]
enum Outcome {
    Accepted,
    Rejected,
    Panicked,
}

const SKIPPED: &[(&str, &str)] = &[
    (
        "y_object_duplicated_key.json",
        "RFC 8259 only says names SHOULD be unique; the parser deliberately rejects duplicate \
         keys instead of silently keeping one value",
    ),
    (
        "y_object_duplicated_key_and_value.json",
        "same deliberate duplicate-key rejection as y_object_duplicated_key.json",
    ),
];

fn suite_dir() -> PathBuf {
    let dir = std::env::var_os("FORGECONF_JSON_TEST_DIR")
        .expect("FORGECONF_JSON_TEST_DIR is unset; run inside `devenv shell`");
    PathBuf::from(dir)
}

fn outcome(bytes: &[u8]) -> Outcome {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return Outcome::Rejected;
    };
    match catch_unwind(AssertUnwindSafe(|| parse_str(text, FileFormat::Json))) {
        Ok(Ok(_)) => Outcome::Accepted,
        Ok(Err(_)) => Outcome::Rejected,
        Err(_) => Outcome::Panicked,
    }
}

fn cases(prefix: &str) -> Vec<(String, Outcome)> {
    let mut names: Vec<String> = fs::read_dir(suite_dir())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(prefix) && name.ends_with(".json"))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no `{prefix}*.json` files in {}", suite_dir().display());
    names
        .into_iter()
        .map(|name| {
            let bytes = fs::read(suite_dir().join(&name)).unwrap();
            let outcome = outcome(&bytes);
            (name, outcome)
        })
        .collect()
}

fn assert_all(prefix: &str, expected: Outcome) {
    let results = cases(prefix);
    for (skipped, _) in SKIPPED {
        assert!(
            !skipped.starts_with(prefix) || results.iter().any(|(name, _)| name == skipped),
            "skipped case `{skipped}` no longer exists in the suite"
        );
    }
    let failures: Vec<&str> = results
        .iter()
        .filter(|(name, got)| *got != expected && !SKIPPED.iter().any(|(skip, _)| skip == name))
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(
        failures.is_empty(),
        "{} of {} `{prefix}` cases wrong:\n{}",
        failures.len(),
        results.len(),
        failures.join("\n")
    );
}

#[test]
#[ignore = "conformance suite; run via `test-all`"]
fn json_y_cases_parse() {
    assert_all("y_", Outcome::Accepted);
}

#[test]
#[ignore = "conformance suite; run via `test-all`"]
fn json_n_cases_are_rejected() {
    assert_all("n_", Outcome::Rejected);
}

#[test]
#[ignore = "conformance suite; run via `test-all`"]
fn json_i_cases_never_panic() {
    let results = cases("i_");
    let count = |wanted: Outcome| results.iter().filter(|(_, got)| *got == wanted).count();
    println!(
        "i_ cases: {} accepted, {} rejected, {} panicked",
        count(Outcome::Accepted),
        count(Outcome::Rejected),
        count(Outcome::Panicked)
    );
    for (name, got) in &results {
        let label = match got {
            Outcome::Accepted => "accepted",
            Outcome::Rejected => "rejected",
            Outcome::Panicked => "PANICKED",
        };
        println!("{name}: {label}");
    }
    let panics: Vec<&str> = results
        .iter()
        .filter(|(_, got)| *got == Outcome::Panicked)
        .map(|(name, _)| name.as_str())
        .collect();
    assert!(panics.is_empty(), "{} `i_` cases panicked:\n{}", panics.len(), panics.join("\n"));
}
