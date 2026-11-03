//! Pure text helpers: number formatting, ratio labels, XML escaping, dates.

/// Escapes text for use in XML text nodes and attribute values.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

/// Three significant digits: `1.88`, `18.8`, `188`. Larger values keep all
/// integer digits.
pub fn sig3(x: f64) -> String {
    let mut decimals = 2;
    loop {
        let text = if decimals == 0 { format!("{}", x.round()) } else { format!("{x:.decimals$}") };
        let rounded: f64 = text.parse().unwrap_or(x);
        if decimals > 0 && rounded >= 10f64.powi(3 - decimals as i32) {
            decimals -= 1;
        } else {
            return text;
        }
    }
}

/// Divides by 1000 until the value fits below 1000 after rounding, picking the
/// matching unit.
fn scale<'a>(mut x: f64, units: &[&'a str]) -> (f64, &'a str) {
    let mut index = 0;
    while x >= 999.5 && index + 1 < units.len() {
        x /= 1000.0;
        index += 1;
    }
    (x, units[index])
}

fn with_unit(x: f64, unit: &str) -> String {
    if unit.is_empty() { sig3(x) } else { format!("{} {unit}", sig3(x)) }
}

/// Nanoseconds as `ns`, `µs` or `ms`.
pub fn time(ns: f64) -> String {
    let (x, unit) = scale(ns, &["ns", "µs", "ms", "s"]);
    with_unit(x, unit)
}

/// Plain counts with SI suffixes: `512`, `30.6 k`, `2.38 M`.
pub fn count(x: f64) -> String {
    let (x, unit) = scale(x, &["", "k", "M", "G"]);
    with_unit(x, unit)
}

pub fn seconds(s: f64) -> String {
    with_unit(s, "s")
}

pub fn kib(x: f64) -> String {
    with_unit(x, "KiB")
}

pub fn integer(x: f64) -> String {
    format!("{x:.0}")
}

/// How forgeconf compares with the best competitor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Verdict {
    Faster(f64),
    Slower(f64),
    /// Within 5 %: no measurable difference on a noisy machine.
    Level,
}

/// `ratio` is forgeconf's time divided by the competitor's.
pub fn verdict(ratio: f64) -> Verdict {
    let factor = ratio.max(1.0 / ratio);
    if factor < 1.05 {
        Verdict::Level
    } else if ratio < 1.0 {
        Verdict::Faster(factor)
    } else {
        Verdict::Slower(factor)
    }
}

pub fn ratio_label(ratio: f64) -> String {
    let factor = |f: f64| {
        if f >= 9.95 { format!("{f:.0}x") } else { format!("{f:.1}x") }
    };
    match verdict(ratio) {
        Verdict::Faster(f) => format!("{} faster", factor(f)),
        Verdict::Slower(f) => format!("{} slower", factor(f)),
        Verdict::Level => "about the same".to_owned(),
    }
}

/// Proleptic Gregorian date for days since 1970-01-01 (Hinnant's
/// civil_from_days).
pub fn civil_date(days: i64) -> String {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_xml_specials() {
        assert_eq!(escape(r#"a<b>&"c" 'd'"#), "a&lt;b&gt;&amp;&quot;c&quot; &apos;d&apos;");
        assert_eq!(escape("TOML → struct"), "TOML → struct");
    }

    #[test]
    fn three_significant_digits() {
        assert_eq!(sig3(1.876), "1.88");
        assert_eq!(sig3(18.76), "18.8");
        assert_eq!(sig3(187.6), "188");
        assert_eq!(sig3(9.996), "10.0");
        assert_eq!(sig3(99.96), "100");
        assert_eq!(sig3(1034.6), "1035");
    }

    #[test]
    fn time_units() {
        assert_eq!(time(590.4), "590 ns");
        assert_eq!(time(999.7), "1.00 µs");
        assert_eq!(time(1880.0), "1.88 µs");
        assert_eq!(time(185_000.0), "185 µs");
        assert_eq!(time(3_310_000.0), "3.31 ms");
    }

    #[test]
    fn count_units() {
        assert_eq!(count(512.0), "512");
        assert_eq!(count(9113.0), "9.11 k");
        assert_eq!(count(30550.0), "30.6 k");
        assert_eq!(count(2_375_108.0), "2.38 M");
    }

    #[test]
    fn footprint_units() {
        assert_eq!(seconds(2.2), "2.20 s");
        assert_eq!(kib(582.1), "582 KiB");
        assert_eq!(integer(7.0), "7");
    }

    #[test]
    fn ratio_labels() {
        assert_eq!(ratio_label(1.0 / 1.4), "1.4x faster");
        assert_eq!(ratio_label(3.5), "3.5x slower");
        assert_eq!(ratio_label(1.37 / 0.39), "3.5x slower");
        assert_eq!(ratio_label(1.0 / 12.3), "12x faster");
        assert_eq!(ratio_label(0.976), "about the same");
        assert_eq!(verdict(0.5), Verdict::Faster(2.0));
        assert_eq!(verdict(2.0), Verdict::Slower(2.0));
    }

    #[test]
    fn civil_dates() {
        assert_eq!(civil_date(0), "1970-01-01");
        assert_eq!(civil_date(20_392), "2025-10-31");
        assert_eq!(civil_date(11_016), "2000-02-29");
    }
}
