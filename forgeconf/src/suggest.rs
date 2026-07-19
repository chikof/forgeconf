/// Closest candidate to `input` by edit distance (over chars), when the
/// distance is at most `max(1, input.chars().count() / 3)`. Ties go to the
/// first candidate.
pub(crate) fn suggest<'a>(
    input: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let input: Vec<char> = input.chars().collect();
    let limit = (input.len() / 3).max(1);
    let mut best: Option<(usize, &'a str)> = None;
    for candidate in candidates {
        let d = distance(&input, candidate);
        if d <= limit && best.is_none_or(|(b, _)| d < b) {
            best = Some((d, candidate));
        }
    }
    best.map(|(_, candidate)| candidate)
}

/// Levenshtein distance where swapping two adjacent chars costs 1, so the
/// common typo `prot` for `port` is within reach of a short key.
fn distance(a: &[char], b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut older = vec![0; b.len() + 1];
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    let mut row = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let mut best = (previous[j] + usize::from(ca != cb))
                .min(previous[j + 1] + 1)
                .min(row[j] + 1);
            if i > 0 && j > 0 && *ca == b[j - 1] && a[i - 1] == *cb {
                best = best.min(older[j - 1] + 1);
            }
            row[j + 1] = best;
        }
        std::mem::swap(&mut older, &mut previous);
        std::mem::swap(&mut previous, &mut row);
    }
    previous[b.len()]
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used, reason = "tests assert on known-good fixtures")]

    use super::*;

    #[test]
    fn transposition_is_found() {
        assert_eq!(suggest("prot", ["host", "port"]), Some("port"));
        assert_eq!(suggest("hots", ["port", "host"]), Some("host"));
    }

    #[test]
    fn far_input_has_no_suggestion() {
        assert_eq!(suggest("database_url", ["port", "host"]), None);
        assert_eq!(suggest("x", std::iter::empty()), None);
    }

    #[test]
    fn closest_wins_and_ties_take_first() {
        assert_eq!(suggest("zorpt", ["zoprt", "zorxx"]), Some("zoprt"));
        assert_eq!(suggest("cat", ["bat", "hat"]), Some("bat"));
        assert_eq!(suggest("abcdef", ["abcdxx", "abcdef"]), Some("abcdef"));
    }

    #[test]
    fn counts_chars_not_bytes() {
        assert_eq!(suggest("pört", ["port"]), Some("port"));
    }

    #[test]
    fn distance_basics() {
        let d = |a: &str, b: &str| distance(&a.chars().collect::<Vec<_>>(), b);
        assert_eq!((d("", ""), d("abc", ""), d("", "abc")), (0, 3, 3));
        assert_eq!((d("kitten", "sitting"), d("xy", "yx"), d("ca", "abc")), (3, 1, 3));
    }
}
