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
    if a.is_ascii() && b.is_ascii() {
        if bound <= 1 {
            return within_one(a.as_bytes(), b.as_bytes(), bound);
        }
        within(a.bytes(), a_len, b.bytes(), b.len(), bound, rows)
    } else {
        within(a.chars(), a_len, b.chars(), b.chars().count(), bound, rows)
    }
}

/// `distance_within` for `bound <= 1`: one edit leaves a single differing
/// region between the common prefix and suffix, so no table is needed.
fn within_one(a: &[u8], b: &[u8], bound: usize) -> Option<usize> {
    if a == b {
        return Some(0);
    }
    if bound == 0 {
        return None;
    }
    let prefix = a.iter().zip(b).take_while(|(x, y)| x == y).count();
    let (a, b) = (&a[prefix..], &b[prefix..]);
    let suffix = a
        .iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    match (&a[..a.len() - suffix], &b[..b.len() - suffix]) {
        ([_], [_] | []) | ([], [_]) => Some(1),
        ([x, y], [u, v]) if x == v && y == u => Some(1),
        _ => None,
    }
}

fn within<T: Copy + PartialEq>(
    a: impl Iterator<Item = T>,
    a_len: usize,
    b: impl Iterator<Item = T> + Clone,
    b_len: usize,
    bound: usize,
    rows: &mut Vec<usize>,
) -> Option<usize> {
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
    let mut previous_a = None;
    for (i, ca) in a.enumerate() {
        row[0] = i + 1;
        let mut row_min = i + 1;
        let mut previous_b = None;
        for (j, cb) in b.clone().enumerate() {
            let mut best = (previous[j] + usize::from(ca != cb))
                .min(previous[j + 1] + 1)
                .min(row[j] + 1);
            if j > 0 && previous_b == Some(ca) && previous_a == Some(cb) {
                best = best.min(older[j - 1] + 1);
            }
            row[j + 1] = best;
            row_min = row_min.min(best);
            previous_b = Some(cb);
        }
        if row_min > bound && previous_min > bound {
            return None;
        }
        previous_min = row_min;
        previous_a = Some(ca);
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

    #[test]
    fn linear_path_agrees_with_the_table() {
        let mut words = vec![String::new()];
        let mut from = 0;
        for _ in 0..5 {
            let to = words.len();
            for i in from..to {
                for c in ["a", "b", "c"] {
                    words.push(format!("{}{c}", words[i]));
                }
            }
            from = to;
        }
        let mut rows = Vec::new();
        for a in &words {
            for b in &words {
                let table = within(a.bytes(), a.len(), b.bytes(), b.len(), usize::MAX, &mut rows)
                    .filter(|d| *d <= 1);
                for bound in [0, 1] {
                    let want = table.filter(|d| *d <= bound);
                    assert_eq!(within_one(a.as_bytes(), b.as_bytes(), bound), want, "{a} {b}");
                }
            }
        }
    }
}
