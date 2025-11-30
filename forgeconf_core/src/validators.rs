use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::Hash;

#[cfg(feature = "regex")]
use regex::Regex;

use crate::ConfigError;

/// Trait for values whose length can be queried.
pub trait HasLen {
    fn len(&self) -> usize {
        0
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl HasLen for String {
    fn len(&self) -> usize {
        String::len(self)
    }
}

impl<T> HasLen for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }
}

impl<T> HasLen for VecDeque<T> {
    fn len(&self) -> usize {
        VecDeque::len(self)
    }
}

impl<K: Ord, V> HasLen for BTreeMap<K, V> {
    fn len(&self) -> usize {
        BTreeMap::len(self)
    }
}

impl<K: Ord> HasLen for BTreeSet<K> {
    fn len(&self) -> usize {
        BTreeSet::len(self)
    }
}

impl<K: Eq + Hash, V> HasLen for HashMap<K, V> {
    fn len(&self) -> usize {
        HashMap::len(self)
    }
}

impl<K: Eq + Hash> HasLen for HashSet<K> {
    fn len(&self) -> usize {
        HashSet::len(self)
    }
}

/// Ensure that a collection-like value is not empty.
pub fn non_empty<T>() -> impl Fn(&T, &str) -> Result<(), ConfigError>
where
    T: HasLen,
{
    |value, key| {
        if value.len() == 0 {
            Err(ConfigError::mismatch(key, "non-empty value", "empty value"))
        } else {
            Ok(())
        }
    }
}

/// Ensure the value length is at least `min`.
pub fn min_len<T>(min: usize) -> impl Fn(&T, &str) -> Result<(), ConfigError>
where
    T: HasLen,
{
    move |value, key| {
        let len = value.len();
        if len < min {
            Err(ConfigError::mismatch(key, format!("length >= {min}"), format!("length {len}")))
        } else {
            Ok(())
        }
    }
}

/// Ensure the value length does not exceed `max`.
pub fn max_len<T>(max: usize) -> impl Fn(&T, &str) -> Result<(), ConfigError>
where
    T: HasLen,
{
    move |value, key| {
        let len = value.len();
        if len > max {
            Err(ConfigError::mismatch(key, format!("length <= {max}"), format!("length {len}")))
        } else {
            Ok(())
        }
    }
}

/// Ensure the value length falls within the provided range.
pub fn len_range<T>(min: usize, max: usize) -> impl Fn(&T, &str) -> Result<(), ConfigError>
where
    T: HasLen,
{
    assert!(min <= max, "length lower bound must be less than or equal to the upper bound");
    move |value, key| {
        let len = value.len();
        if len < min || len > max {
            Err(ConfigError::mismatch(
                key,
                format!("length between {min} and {max}"),
                format!("length {len}"),
            ))
        } else {
            Ok(())
        }
    }
}

/// Ensure that a comparable scalar falls within the provided range.
pub fn range<T>(min: T, max: T) -> impl Fn(&T, &str) -> Result<(), ConfigError>
where
    T: PartialOrd + Display + Copy,
{
    assert!(min <= max, "range lower bound must be less than or equal to the upper bound");
    move |value, key| {
        if *value < min || *value > max {
            Err(ConfigError::mismatch(key, format!("between {min} and {max}"), value.to_string()))
        } else {
            Ok(())
        }
    }
}

/// Ensure the value is present in a pre-defined set.
pub fn one_of<T, I>(allowed: I) -> impl Fn(&T, &str) -> Result<(), ConfigError>
where
    T: PartialEq + Display + Clone + 'static,
    I: IntoIterator<Item = T>,
{
    let allowed: Vec<T> = allowed
        .into_iter()
        .collect();
    assert!(!allowed.is_empty(), "one_of requires at least one candidate value");
    let expected = format!(
        "one of [{}]",
        allowed
            .iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );

    move |value, key| {
        if allowed
            .iter()
            .any(|candidate| candidate == value)
        {
            Ok(())
        } else {
            Err(ConfigError::mismatch(key, expected.clone(), value.to_string()))
        }
    }
}

/// Ensure the textual value satisfies the provided regular expression.
#[cfg(feature = "regex")]
pub fn matches_regex<T>(pattern: Regex) -> impl Fn(&T, &str) -> Result<(), ConfigError>
where
    T: AsRef<str> + Display,
{
    let label = format!("match /{}/", pattern.as_str());
    move |value, key| {
        if pattern.is_match(value.as_ref()) {
            Ok(())
        } else {
            Err(ConfigError::mismatch(key, label.clone(), value.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_empty_is_rejected() {
        let validator = non_empty::<String>();
        let err = validator(&String::new(), "name").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch { field, expected, .. }
                if field == "name" && expected == "non-empty value"
        ));
    }

    #[test]
    fn range_accepts_value_in_bounds() {
        let validator = range(10_u32, 20);
        validator(&15, "port").unwrap();
    }

    #[test]
    fn range_rejects_out_of_bounds() {
        let validator = range(1_u8, 3_u8);
        let err = validator(&0, "retries").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch { field, expected, found }
                if field == "retries" && expected == "between 1 and 3" && found == "0"
        ));
    }

    #[test]
    fn min_len_flags_short_values() {
        let validator = min_len::<String>(3);
        let err = validator(&"ab".to_string(), "token").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch { field, expected, found }
                if field == "token" && expected == "length >= 3" && found == "length 2"
        ));
    }

    #[test]
    fn max_len_flags_long_values() {
        let validator = max_len::<Vec<u8>>(2);
        let err = validator(&vec![1, 2, 3], "bytes").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch { field, expected, found }
                if field == "bytes" && expected == "length <= 2" && found == "length 3"
        ));
    }

    #[test]
    fn len_range_accepts_bounds() {
        let validator = len_range::<String>(2, 4);
        validator(&"abcd".to_string(), "scope").unwrap();
    }

    #[test]
    fn one_of_accepts_member() {
        let validator = one_of(vec!["debug".to_string(), "info".to_string()]);
        validator(&"info".to_string(), "level").unwrap();
    }

    #[test]
    fn one_of_rejects_non_member() {
        let validator = one_of(vec!["debug".to_string(), "info".to_string()]);
        let err = validator(&"warn".to_string(), "level").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch { field, expected, found }
                if field == "level" && expected.contains("debug") && found == "warn"
        ));
    }

    #[cfg(feature = "regex")]
    #[test]
    fn matches_regex_enforces_pattern() {
        let validator = matches_regex::<String>(Regex::new(r"^[a-z0-9_]+$").unwrap());
        validator(&"valid_123".to_string(), "tag").unwrap();
        let err = validator(&"Invalid!".to_string(), "tag").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::TypeMismatch { field, expected, .. }
                if field == "tag" && expected.starts_with("match /")
        ));
    }
}
