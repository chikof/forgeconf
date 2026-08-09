/// Closest candidate to `input` by edit distance (over chars), when the
/// distance is at most `max(1, input.chars().count() / 3)`. Ties go to the
/// first candidate.
pub(crate) fn suggest<'a>(
    input: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    let input_len = input.chars().count();
    let mut bound = (input_len / 3).max(1);
    let mut rows = Vec::new();
    let mut best = None;
    for candidate in candidates {
        let Some(d) = distance_within(input, input_len, candidate, bound, &mut rows) else {
            continue;
        };
        best = Some(candidate);
        if d == 0 {
            break;
        }
        // Later candidates only win with a strictly smaller distance.
        bound = d - 1;
    }
    best
}

/// Levenshtein distance where swapping two adjacent chars costs 1, so the
/// common typo `prot` for `port` is within reach of a short key. `None` when
/// the distance exceeds `bound`. `rows` is scratch space reused across calls.
fn distance_within(
    a: &str,
    a_len: usize,
    b: &str,
    bound: usize,
    rows: &mut Vec<usize>,
) -> Option<usize> {
    let b_len = b.chars().count();
    if a_len.abs_diff(b_len) > bound {
        return None;
    }
    let width = b_len + 1;
    rows.clear();
    rows.resize(3 * width, 0);
    let (mut older, rest) = rows.split_at_mut(width);
    let (mut previous, mut row) = rest.split_at_mut(width);
    for (j, cell) in previous.iter_mut().enumerate() {
        *cell = j;
    }
    // Every cell of a row is at least the smaller of the two rows above it,
    // so once both exceed `bound` the final distance does too.
    let mut previous_min = 0;
    let mut previous_a = '\0';
    for (i, ca) in a.chars().enumerate() {
        row[0] = i + 1;
        let mut row_min = i + 1;
        let mut previous_b = '\0';
        for (j, cb) in b.chars().enumerate() {
            let mut best = (previous[j] + usize::from(ca != cb))
                .min(previous[j + 1] + 1)
                .min(row[j] + 1);
            if i > 0 && j > 0 && ca == previous_b && previous_a == cb {
                best = best.min(older[j - 1] + 1);
            }
            row[j + 1] = best;
            row_min = row_min.min(best);
            previous_b = cb;
        }
        if row_min > bound && previous_min > bound {
            return None;
        }
        previous_min = row_min;
        previous_a = ca;
        std::mem::swap(&mut older, &mut previous);
        std::mem::swap(&mut previous, &mut row);
    }
    let d = previous[b_len];
    (d <= bound).then_some(d)
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
        let d = |a: &str, b: &str| {
            distance_within(a, a.chars().count(), b, usize::MAX, &mut Vec::new()).unwrap()
        };
        assert_eq!((d("", ""), d("abc", ""), d("", "abc")), (0, 3, 3));
        assert_eq!((d("kitten", "sitting"), d("xy", "yx"), d("ca", "abc")), (3, 1, 3));
    }
}
