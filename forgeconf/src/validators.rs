//! Reusable field validators.
//!
//! Every function returns a closure `Fn(&T) -> Result<(), ConfigError>` that
//! can be used in `#[field(validate = ...)]`. The error carries only the
//! message; the caller adds the key path.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::fmt::Display;
use std::hash::Hash;
use std::ops::{Bound, RangeBounds};

use crate::ConfigError;

/// Values whose length can be queried.
pub trait HasLen {
    /// Number of elements (bytes for strings).
    fn len(&self) -> usize;

    /// `true` when [`len`](HasLen::len) is zero.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl HasLen for str {
    fn len(&self) -> usize {
        str::len(self)
    }
}

impl HasLen for String {
    fn len(&self) -> usize {
        String::len(self)
    }
}

impl<T> HasLen for [T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
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

impl<K, V> HasLen for BTreeMap<K, V> {
    fn len(&self) -> usize {
        BTreeMap::len(self)
    }
}

impl<K, V> HasLen for HashMap<K, V> {
    fn len(&self) -> usize {
        HashMap::len(self)
    }
}

impl<T> HasLen for BTreeSet<T> {
    fn len(&self) -> usize {
        BTreeSet::len(self)
    }
}

impl<T: Eq + Hash> HasLen for HashSet<T> {
    fn len(&self) -> usize {
        HashSet::len(self)
    }
}

/// Reject empty strings and collections.
pub fn non_empty<T: HasLen + ?Sized>() -> impl Fn(&T) -> Result<(), ConfigError> {
    |value| {
        if value.is_empty() { Err(ConfigError::invalid("must not be empty")) } else { Ok(()) }
    }
}

/// Require the length to fall inside `range`.
pub fn len<T: HasLen + ?Sized>(
    range: impl RangeBounds<usize>,
) -> impl Fn(&T) -> Result<(), ConfigError> {
    let bounds = (range.start_bound().cloned(), range.end_bound().cloned());
    let message = format!("length must be {}", describe(&bounds));
    move |value| {
        if bounds.contains(&value.len()) { Ok(()) } else { Err(ConfigError::invalid(&message)) }
    }
}

/// Require the value to fall inside `range`.
pub fn range<T: PartialOrd + Display + Clone>(
    range: impl RangeBounds<T>,
) -> impl Fn(&T) -> Result<(), ConfigError> {
    let bounds = (range.start_bound().cloned(), range.end_bound().cloned());
    let message = format!("must be {}", describe(&bounds));
    move |value| {
        if bounds.contains(value) { Ok(()) } else { Err(ConfigError::invalid(&message)) }
    }
}

/// Require the value to equal one of `allowed`.
pub fn one_of<T: PartialEq + Display>(
    allowed: impl IntoIterator<Item = T>,
) -> impl Fn(&T) -> Result<(), ConfigError> {
    let allowed: Vec<T> = allowed.into_iter().collect();
    let list: Vec<String> = allowed.iter().map(ToString::to_string).collect();
    let message = format!("must be one of: {}", list.join(", "));
    move |value| {
        if allowed.contains(value) { Ok(()) } else { Err(ConfigError::invalid(&message)) }
    }
}

fn describe<T: Display>(bounds: &(Bound<T>, Bound<T>)) -> String {
    match bounds {
        (Bound::Included(a), Bound::Included(b)) => format!("between {a} and {b}"),
        (Bound::Included(a), Bound::Excluded(b)) => format!("at least {a} and less than {b}"),
        (Bound::Excluded(a), Bound::Included(b)) => format!("greater than {a} and at most {b}"),
        (Bound::Excluded(a), Bound::Excluded(b)) => format!("greater than {a} and less than {b}"),
        (Bound::Included(a), Bound::Unbounded) => format!("at least {a}"),
        (Bound::Excluded(a), Bound::Unbounded) => format!("greater than {a}"),
        (Bound::Unbounded, Bound::Included(b)) => format!("at most {b}"),
        (Bound::Unbounded, Bound::Excluded(b)) => format!("less than {b}"),
        (Bound::Unbounded, Bound::Unbounded) => "anything".to_owned(),
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;

    #[test]
    fn non_empty_rejects_empty() {
        let validator = non_empty::<String>();
        assert_eq!(validator(&String::new()).unwrap_err().to_string(), "must not be empty");
        validator(&"x".to_owned()).unwrap();
        non_empty::<[u8]>()(&[1]).unwrap();
        assert!(non_empty::<str>()("").is_err());
    }

    #[test]
    fn range_inclusive_message() {
        let validator = range(1024..=65535_u16);
        validator(&1024).unwrap();
        validator(&65535).unwrap();
        let err = validator(&80).unwrap_err().at_key("port");
        assert_eq!(err.to_string(), "port: must be between 1024 and 65535");
    }

    #[test]
    fn range_open_ends() {
        assert_eq!(range(5_u8..)(&4).unwrap_err().to_string(), "must be at least 5");
        assert_eq!(range(..=5_u8)(&6).unwrap_err().to_string(), "must be at most 5");
        assert_eq!(range(..5_u8)(&5).unwrap_err().to_string(), "must be less than 5");
        assert_eq!(
            range(1_u8..5)(&5).unwrap_err().to_string(),
            "must be at least 1 and less than 5"
        );
        range(1_u8..5)(&4).unwrap();
        range::<u8>(..)(&0).unwrap();
    }

    #[test]
    fn range_exclusive_start() {
        let validator = range((Bound::Excluded(0.0_f64), Bound::Unbounded));
        assert_eq!(validator(&0.0).unwrap_err().to_string(), "must be greater than 0");
        validator(&0.1).unwrap();
    }

    #[test]
    #[allow(clippy::reversed_empty_ranges, reason = "the empty range is the case under test")]
    fn empty_range_never_passes() {
        let validator = range(5_u8..3);
        assert!(validator(&4).is_err());
        assert!(validator(&5).is_err());
        assert!(validator(&3).is_err());
    }

    #[test]
    fn len_messages() {
        let both = len::<str>(2..=4);
        both("ab").unwrap();
        assert_eq!(both("a").unwrap_err().to_string(), "length must be between 2 and 4");
        assert_eq!(len::<str>(3..)("ab").unwrap_err().to_string(), "length must be at least 3");
        assert_eq!(
            len::<[u8]>(..=2)(&[1, 2, 3]).unwrap_err().to_string(),
            "length must be at most 2"
        );
        assert_eq!(
            len::<Vec<u8>>(..2)(&vec![1, 2]).unwrap_err().to_string(),
            "length must be less than 2"
        );
    }

    #[test]
    fn one_of_messages() {
        let validator = one_of(["debug".to_owned(), "info".to_owned()]);
        validator(&"info".to_owned()).unwrap();
        let err = validator(&"warn".to_owned()).unwrap_err();
        assert_eq!(err.to_string(), "must be one of: debug, info");
    }

    #[test]
    fn one_of_empty_never_passes() {
        assert!(one_of(Vec::<u8>::new())(&1).is_err());
    }
}
