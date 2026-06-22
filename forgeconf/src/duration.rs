use std::time::Duration;

const NANOS_PER_SEC: u128 = 1_000_000_000;

/// Parse a human-written duration.
///
/// Accepts a plain integer (seconds), a plain decimal such as `1.5`
/// (seconds), or one or more `<integer><unit>` parts like `1h30m` or
/// `2d 4h`, with units `ns`, `us`, `µs`, `ms`, `s`, `m`, `h` and `d`.
/// Returns `None` for empty input, unknown units, and overflow.
pub(crate) fn parse_duration(s: &str) -> Option<Duration> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.bytes().all(|b| b.is_ascii_digit()) {
        return s.parse().ok().map(Duration::from_secs);
    }
    parse_decimal_seconds(s).or_else(|| parse_parts(s))
}

fn parse_decimal_seconds(s: &str) -> Option<Duration> {
    let (whole, frac) = s.split_once('.')?;
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || !digits(frac) {
        return None;
    }
    Duration::try_from_secs_f64(s.parse().ok()?).ok()
}

fn parse_parts(s: &str) -> Option<Duration> {
    let mut rest = s;
    let mut total: u128 = 0;
    loop {
        rest = rest.trim_start();
        if rest.is_empty() {
            break;
        }
        let digits_end = rest.find(|c: char| !c.is_ascii_digit())?;
        let amount: u128 = rest[..digits_end].parse().ok()?;
        rest = &rest[digits_end..];
        let unit_end = rest
            .find(|c: char| c.is_ascii_digit() || c.is_whitespace())
            .unwrap_or(rest.len());
        let nanos_per_unit = match &rest[..unit_end] {
            "ns" => 1,
            "us" | "µs" => 1_000,
            "ms" => 1_000_000,
            "s" => NANOS_PER_SEC,
            "m" => 60 * NANOS_PER_SEC,
            "h" => 3_600 * NANOS_PER_SEC,
            "d" => 86_400 * NANOS_PER_SEC,
            _ => return None,
        };
        rest = &rest[unit_end..];
        total = total.checked_add(amount.checked_mul(nanos_per_unit)?)?;
    }
    let secs = u64::try_from(total / NANOS_PER_SEC).ok()?;
    // The remainder is below 1e9, so it always fits in u32.
    Some(Duration::new(secs, (total % NANOS_PER_SEC) as u32))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn plain_integer_is_seconds() {
        assert_eq!(parse_duration("30"), Some(Duration::from_secs(30)));
        assert_eq!(parse_duration(" 7 "), Some(Duration::from_secs(7)));
    }

    #[test]
    fn plain_decimal_is_seconds() {
        assert_eq!(parse_duration("1.5"), Some(Duration::from_millis(1500)));
    }

    #[test]
    fn compound_parts() {
        assert_eq!(parse_duration("1h30m"), Some(Duration::from_secs(5400)));
        assert_eq!(parse_duration("2d 4h"), Some(Duration::from_secs(2 * 86_400 + 4 * 3_600)));
        assert_eq!(parse_duration("1m 1s 1ms"), Some(Duration::new(61, 1_000_000)));
    }

    #[test]
    fn sub_second_units() {
        assert_eq!(parse_duration("250ms"), Some(Duration::from_millis(250)));
        assert_eq!(parse_duration("5us"), Some(Duration::from_micros(5)));
        assert_eq!(parse_duration("5µs"), Some(Duration::from_micros(5)));
        assert_eq!(parse_duration("9ns"), Some(Duration::from_nanos(9)));
    }

    #[test]
    fn rejects_invalid() {
        assert_eq!(parse_duration("5x"), None);
        assert_eq!(parse_duration(""), None);
        assert_eq!(parse_duration("   "), None);
        assert_eq!(parse_duration("h"), None);
        assert_eq!(parse_duration("5"), Some(Duration::from_secs(5)));
        assert_eq!(parse_duration("1.5h"), None);
        assert_eq!(parse_duration(".5"), None);
        assert_eq!(parse_duration("1."), None);
        assert_eq!(parse_duration("-1s"), None);
    }

    #[test]
    fn rejects_overflow() {
        assert_eq!(parse_duration("99999999999999999999999999999999999999999"), None);
        assert_eq!(parse_duration("999999999999999999999d"), None);
        assert_eq!(parse_duration("18446744073709551616"), None);
        assert_eq!(parse_duration("1e400"), None);
    }
}
