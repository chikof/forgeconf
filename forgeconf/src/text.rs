//! [`Text`]: an immutable string that keeps short text inline.

#![allow(
    unsafe_code,
    reason = "the inline buffer is read back as `str` without re-validating bytes that were \
              copied from a `&str` prefix"
)]

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// Longest text, in bytes, that [`Text`] stores without a heap allocation.
const INLINE_CAP: usize = 22;

/// An immutable UTF-8 string that stores up to 22 bytes inline.
///
/// Longer text lives in a `Box<str>`. Configuration trees hold mostly short
/// keys and scalars, so most values never touch the allocator. Comparison,
/// ordering and hashing are those of [`str`], which makes `Text` a valid
/// [`Borrow<str>`] map key.
///
/// ```
/// use forgeconf::Text;
///
/// let short = Text::from("localhost");
/// assert_eq!(short, "localhost");
/// assert_eq!(short.len(), 9);
///
/// let long = Text::from("a string that does not fit in twenty-two bytes");
/// assert_eq!(long.as_str(), "a string that does not fit in twenty-two bytes");
/// assert_eq!(long.into_string(), "a string that does not fit in twenty-two bytes");
/// ```
#[derive(Clone)]
pub struct Text(Repr);

#[derive(Clone)]
enum Repr {
    /// The first `len` bytes of `buf`, zero-padded: a `&str` prefix ending on a
    /// char boundary, never mutated afterwards.
    Inline {
        len: u8,
        buf: [u8; INLINE_CAP],
    },
    Heap(Box<str>),
}

const _: () = assert!(size_of::<Text>() == 24);

/// The zero-padded inline buffer as big-endian words: comparing them equals
/// comparing the padded bytes, which orders like the text once the lengths
/// break a tie.
fn words(buf: &[u8; INLINE_CAP]) -> (u128, u64) {
    let (head, tail) = buf.split_at(16);
    let mut hi = [0; 16];
    let mut lo = [0; 8];
    hi.copy_from_slice(head);
    if let Some(dst) = lo.get_mut(2..) {
        dst.copy_from_slice(tail);
    }
    (u128::from_be_bytes(hi), u64::from_be_bytes(lo))
}

impl Text {
    /// The inline form of `s`, or `None` when it is longer than 22 bytes.
    fn inline(s: &str) -> Option<Self> {
        let mut buf = [0; INLINE_CAP];
        buf.get_mut(..s.len())?.copy_from_slice(s.as_bytes());
        let len = u8::try_from(s.len()).ok()?;
        Some(Self(Repr::Inline { len, buf }))
    }

    /// The text as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Repr::Inline { len, buf } => {
                let bytes = buf.get(..usize::from(*len)).unwrap_or_default();
                // SAFETY: `bytes` is the prefix of a `&str` copied by
                // `Text::inline` that ends on a char boundary, and the buffer
                // is never mutated.
                unsafe { std::str::from_utf8_unchecked(bytes) }
            },
            Repr::Heap(boxed) => boxed,
        }
    }

    /// Convert into a [`String`]: moves the heap buffer, allocates for inline
    /// text.
    #[must_use]
    pub fn into_string(self) -> String {
        match self.0 {
            Repr::Heap(boxed) => boxed.into_string(),
            Repr::Inline { .. } => self.as_str().to_owned(),
        }
    }
}

impl Default for Text {
    fn default() -> Self {
        Self(Repr::Inline { len: 0, buf: [0; INLINE_CAP] })
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Text {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Self {
        Self::inline(s).unwrap_or_else(|| Self(Repr::Heap(s.into())))
    }
}

impl From<String> for Text {
    fn from(s: String) -> Self {
        Self::inline(&s).unwrap_or_else(|| Self(Repr::Heap(s.into_boxed_str())))
    }
}

impl From<Box<str>> for Text {
    fn from(s: Box<str>) -> Self {
        Self::inline(&s).unwrap_or(Self(Repr::Heap(s)))
    }
}

impl From<Text> for String {
    fn from(text: Text) -> Self {
        text.into_string()
    }
}

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Repr::Inline { len: a, buf: x }, Repr::Inline { len: b, buf: y }) => {
                a == b && words(x) == words(y)
            },
            _ => self.as_str() == other.as_str(),
        }
    }
}
impl Eq for Text {}

impl PartialEq<str> for Text {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Text {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for Text {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Text {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.0, &other.0) {
            (Repr::Inline { len: a, buf: x }, Repr::Inline { len: b, buf: y }) => {
                words(x).cmp(&words(y)).then(a.cmp(b))
            },
            _ => self.as_str().cmp(other.as_str()),
        }
    }
}

impl Hash for Text {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state);
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeMap;
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    use super::*;

    fn is_inline(text: &Text) -> bool {
        matches!(text.0, Repr::Inline { .. })
    }

    fn hash_of<T: Hash + ?Sized>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn sizes_inline_and_heap_boundaries() {
        for (len, inline) in
            [(0, true), (1, true), (21, true), (22, true), (23, false), (200, false)]
        {
            let s = "x".repeat(len);
            let text = Text::from(s.as_str());
            assert_eq!(is_inline(&text), inline, "len {len}");
            assert_eq!(text.as_str(), s);
            assert_eq!(text.len(), len);
            assert_eq!(text.clone().into_string(), s);
            assert_eq!(Text::from(s.clone()).as_str(), s);
            assert_eq!(Text::from(s.clone().into_boxed_str()).as_str(), s);
            assert_eq!(is_inline(&Text::from(s)), inline, "owned len {len}");
        }
    }

    #[test]
    fn default_is_empty() {
        assert_eq!(Text::default(), "");
        assert!(Text::default().is_empty());
    }

    #[test]
    fn multibyte_char_across_the_boundary_goes_to_the_heap() {
        let ends_at_22 = format!("{}\u{20ac}", "a".repeat(19));
        assert_eq!(ends_at_22.len(), 22);
        let fits = Text::from(ends_at_22.as_str());
        assert!(is_inline(&fits));
        assert_eq!(fits, ends_at_22);

        let straddles = format!("{}\u{20ac}", "a".repeat(21));
        assert_eq!(straddles.len(), 24);
        let spills = Text::from(straddles.as_str());
        assert!(!is_inline(&spills));
        assert_eq!(spills, straddles);

        let starts_at_20 = format!("{}\u{20ac}b", "a".repeat(20));
        assert_eq!(starts_at_20.len(), 24);
        assert_eq!(Text::from(starts_at_20.as_str()), starts_at_20);
        assert_eq!(Text::from("\u{1f980}".repeat(5).as_str()).as_str(), "\u{1f980}".repeat(5));
    }

    #[test]
    fn ordering_and_hash_match_str() {
        let samples = [
            "",
            "a",
            "a\0",
            "ab",
            "b",
            "\u{20ac}",
            &"a".repeat(30),
            &"a".repeat(31),
            "abcdefghijklmnopqrstuvw",
        ];
        for a in samples {
            assert_eq!(hash_of(&Text::from(a)), hash_of(a));
            for b in samples {
                let (ta, tb) = (Text::from(a), Text::from(b));
                assert_eq!(ta.cmp(&tb), a.cmp(b));
                assert_eq!(ta.partial_cmp(&tb), a.partial_cmp(b));
                assert_eq!(ta == tb, a == b);
            }
        }
    }

    #[test]
    fn comparisons_with_str_and_string() {
        let text = Text::from("abc");
        assert_eq!(text, *"abc");
        assert_eq!(text, "abc");
        assert_eq!(text, String::from("abc"));
        assert_ne!(text, "abx");
        assert_eq!(String::from(text), "abc");
    }

    #[test]
    fn formats_like_str() {
        let text = Text::from("a\"b");
        assert_eq!(format!("{text:?}"), format!("{:?}", "a\"b"));
        assert_eq!(format!("{text}"), "a\"b");
        assert_eq!(format!("{text:>5}"), format!("{:>5}", "a\"b"));
    }

    #[test]
    fn btree_map_lookup_by_str() {
        let long = "k".repeat(40);
        let mut map: BTreeMap<Text, u32> = BTreeMap::new();
        map.insert("short".into(), 1);
        map.insert(long.clone().into(), 2);
        assert_eq!(map.get("short"), Some(&1));
        assert_eq!(map.get(long.as_str()), Some(&2));
        assert_eq!(map.get("missing"), None);
    }
}
