//! Hand-written SVG: horizontal bar panels and the forgeconf-vs-best chart.
//!
//! Every image draws its own opaque light background, so it reads the same on
//! GitHub's light and dark themes.

use std::fmt::Write;

use crate::fmt::{self, Verdict};

pub const WIDTH: f64 = 900.0;
const MARGIN: f64 = 20.0;
const GAP: f64 = 16.0;
const HEADER: f64 = 78.0;
const FOOTER: f64 = 34.0;
const ROW: f64 = 22.0;
const PANEL_HEAD: f64 = 48.0;
const PANEL_PAD: f64 = 10.0;
const VALUE_ROOM: f64 = 68.0;

const BG: &str = "#ffffff";
const PANEL: &str = "#f6f8fa";
const BORDER: &str = "#d0d7de";
const TEXT: &str = "#1f2328";
const MUTED: &str = "#59636e";
const ACCENT: &str = "#2f6fe4";
const NEUTRAL: &str = "#a7b0ba";
const GOOD: &str = "#2da44e";
const BAD: &str = "#cf222e";

pub struct Bar {
    pub label: String,
    pub value: f64,
    /// Whisker ends, in the same unit as `value`.
    pub range: Option<(f64, f64)>,
    pub accent: bool,
}

pub struct Panel {
    pub title: String,
    pub subtitle: String,
    pub bars: Vec<Bar>,
    pub format: fn(f64) -> String,
}

/// Body of an image: markup plus the height it occupies below the header.
pub struct Body {
    markup: String,
    height: f64,
}

fn text(out: &mut String, x: f64, y: f64, size: u32, fill: &str, extra: &str, content: &str) {
    let _ = write!(
        out,
        r#"<text x="{x:.1}" y="{y:.1}" font-size="{size}" fill="{fill}"{extra}>{}</text>"#,
        fmt::escape(content)
    );
}

fn panel_height(rows: usize) -> f64 {
    PANEL_HEAD + rows as f64 * ROW + PANEL_PAD
}

fn draw_panel(out: &mut String, x: f64, y: f64, w: f64, h: f64, label_w: f64, panel: &Panel) {
    let _ = write!(
        out,
        r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" rx="8" fill="{PANEL}" stroke="{BORDER}"/>"#
    );
    text(out, x + 12.0, y + 21.0, 13, TEXT, r#" font-weight="700""#, &panel.title);
    text(out, x + 12.0, y + 37.0, 11, MUTED, "", &panel.subtitle);

    let mut bars: Vec<&Bar> = panel.bars.iter().collect();
    bars.sort_by(|a, b| a.value.total_cmp(&b.value));
    let best = bars.first().map(|b| b.value);
    let extent = |b: &Bar| b.range.map_or(b.value, |(_, hi)| hi.max(b.value));
    let top = bars.iter().map(|b| extent(b)).fold(0.0, f64::max);
    let x0 = x + 12.0 + label_w;
    let plot_w = (w - 24.0 - label_w - VALUE_ROOM).max(1.0);
    let scale = if top > 0.0 { plot_w / top } else { 0.0 };

    for (i, bar) in bars.iter().enumerate() {
        let mid = y + PANEL_HEAD + i as f64 * ROW + ROW / 2.0;
        let bold = if Some(bar.value) == best { r#" font-weight="700""# } else { "" };
        let color = if bar.accent { ACCENT } else { NEUTRAL };
        text(out, x0 - 8.0, mid + 4.0, 12, TEXT, r#" text-anchor="end""#, &bar.label);
        let _ = write!(
            out,
            r#"<rect x="{x0:.1}" y="{:.1}" width="{:.1}" height="12" rx="2" fill="{color}"/>"#,
            mid - 6.0,
            (bar.value * scale).max(1.0)
        );
        let mut end = bar.value * scale;
        if let Some((lo, hi)) = bar.range {
            let (a, b) = (x0 + lo * scale, x0 + hi * scale);
            let _ = write!(
                out,
                r#"<path d="M{a:.1} {mid:.1}H{b:.1}M{a:.1} {:.1}V{:.1}M{b:.1} {:.1}V{:.1}" stroke="{TEXT}" stroke-width="1.2" fill="none"/>"#,
                mid - 4.0,
                mid + 4.0,
                mid - 4.0,
                mid + 4.0
            );
            end = end.max(hi * scale);
        }
        text(out, x0 + end + 6.0, mid + 4.0, 12, TEXT, bold, &(panel.format)(bar.value));
    }
}

/// Lays panels out in a grid with equal-height cells.
pub fn grid(panels: &[Panel], cols: usize, label_w: f64) -> Body {
    let cell_w = (WIDTH - 2.0 * MARGIN - (cols - 1) as f64 * GAP) / cols as f64;
    let mut markup = String::new();
    let mut y = 0.0;
    for row in panels.chunks(cols) {
        let height = panel_height(row.iter().map(|p| p.bars.len()).max().unwrap_or(0));
        for (i, panel) in row.iter().enumerate() {
            let x = MARGIN + i as f64 * (cell_w + GAP);
            draw_panel(&mut markup, x, y, cell_w, height, label_w, panel);
        }
        y += height + GAP;
    }
    Body { markup, height: y - GAP }
}

pub struct Ratio {
    pub group: String,
    pub rival: String,
    /// forgeconf's median divided by the rival's.
    pub ratio: f64,
}

/// Log-scale chart centred on 1x: left of centre is faster than the best
/// competitor.
pub fn relative(rows: &[Ratio]) -> Body {
    const LABEL_W: f64 = 150.0;
    const ROW_H: f64 = 34.0;
    const AXIS_H: f64 = 40.0;
    const LABEL_ROOM: f64 = 100.0;
    let left = MARGIN + LABEL_W;
    let right = WIDTH - MARGIN;
    let centre = (left + right) / 2.0;
    let half = (right - left) / 2.0 - LABEL_ROOM;

    let octaves = rows
        .iter()
        .map(|r| r.ratio.log2().abs())
        .fold(2.0_f64, f64::max)
        .ceil();
    let per_octave = half / octaves;
    let height = AXIS_H + rows.len() as f64 * ROW_H;

    let mut out = String::new();
    text(
        &mut out,
        centre - 10.0,
        12.0,
        11,
        GOOD,
        r#" text-anchor="end" font-weight="600""#,
        "← faster",
    );
    text(&mut out, centre + 10.0, 12.0, 11, BAD, r#" font-weight="600""#, "slower →");
    let n = octaves as i32;
    for k in -n..=n {
        let x = centre + f64::from(k) * per_octave;
        let tick = if k >= 0 {
            format!("{}x", 1u32 << k)
        } else {
            format!("{}x", 1.0 / f64::from(1u32 << -k))
        };
        let stroke = if k == 0 { MUTED } else { BORDER };
        let _ = write!(
            out,
            r#"<path d="M{x:.1} {:.1}V{height:.1}" stroke="{stroke}" stroke-width="1"/>"#,
            AXIS_H - 10.0
        );
        text(&mut out, x, AXIS_H - 14.0, 10, MUTED, r#" text-anchor="middle""#, &tick);
    }

    for (i, row) in rows.iter().enumerate() {
        let top = AXIS_H + i as f64 * ROW_H;
        let mid = top + ROW_H / 2.0;
        text(&mut out, MARGIN, mid - 1.0, 12, TEXT, r#" font-weight="700""#, &row.group);
        text(&mut out, MARGIN, mid + 11.0, 10, MUTED, "", &format!("vs {}", row.rival));
        let offset = row.ratio.log2() * per_octave;
        let verdict = fmt::verdict(row.ratio);
        let color = match verdict {
            Verdict::Faster(_) => GOOD,
            Verdict::Slower(_) => BAD,
            Verdict::Level => NEUTRAL,
        };
        let (x, width) = if offset < 0.0 { (centre + offset, -offset) } else { (centre, offset) };
        let _ = write!(
            out,
            r#"<rect x="{x:.1}" y="{:.1}" width="{:.1}" height="16" rx="2" fill="{color}"/>"#,
            mid - 8.0,
            width.max(2.0)
        );
        let label = fmt::ratio_label(row.ratio);
        if offset < 0.0 {
            text(
                &mut out,
                x - 6.0,
                mid + 4.0,
                12,
                TEXT,
                r#" text-anchor="end" font-weight="700""#,
                &label,
            );
        } else {
            text(&mut out, x + width + 6.0, mid + 4.0, 12, TEXT, r#" font-weight="700""#, &label);
        }
    }
    Body { markup: out, height }
}

/// Wraps a body in the svg root, background, header, optional legend and
/// footer.
pub fn document(title: &str, subtitle: &str, legend: bool, body: &Body, footer: &str) -> String {
    let height = HEADER + body.height + FOOTER;
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {WIDTH} {height:.0}" width="{WIDTH}" height="{height:.0}" role="img" aria-label="{}" font-family="system-ui, -apple-system, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif">"#,
        fmt::escape(title)
    );
    let _ = write!(out, "<title>{}</title>", fmt::escape(title));
    let _ = write!(
        out,
        r#"<rect width="{WIDTH}" height="{height:.0}" rx="12" fill="{BG}" stroke="{BORDER}"/>"#
    );
    text(&mut out, MARGIN, 36.0, 20, TEXT, r#" font-weight="700""#, title);
    text(&mut out, MARGIN, 58.0, 12, MUTED, "", subtitle);
    if legend {
        let x = WIDTH - MARGIN - 150.0;
        let _ = write!(
            out,
            r#"<rect x="{x:.1}" y="26" width="10" height="10" rx="2" fill="{ACCENT}"/>"#
        );
        text(&mut out, x + 15.0, 35.0, 11, TEXT, "", "forgeconf");
        let _ = write!(
            out,
            r#"<rect x="{:.1}" y="26" width="10" height="10" rx="2" fill="{NEUTRAL}"/>"#,
            x + 80.0
        );
        text(&mut out, x + 95.0, 35.0, 11, TEXT, "", "others");
    }
    let _ = write!(out, r#"<g transform="translate(0 {HEADER})">{}</g>"#, body.markup);
    text(&mut out, MARGIN, height - 14.0, 11, MUTED, "", footer);
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(title: &str) -> Panel {
        Panel {
            title: title.to_owned(),
            subtitle: "a < b & c".to_owned(),
            bars: vec![
                Bar {
                    label: "slow".to_owned(),
                    value: 30.0,
                    range: Some((28.0, 33.0)),
                    accent: false,
                },
                Bar {
                    label: "forgeconf".to_owned(),
                    value: 10.0,
                    range: None,
                    accent: true,
                },
            ],
            format: fmt::integer,
        }
    }

    #[test]
    fn panel_text_is_escaped_and_sorted() {
        let body = grid(&[panel("t&t")], 1, 80.0);
        let svg = document("T", "S", true, &body, "f");
        assert!(svg.contains("t&amp;t"));
        assert!(svg.contains("a &lt; b &amp; c"));
        assert!(svg.find(">forgeconf<").unwrap() < svg.find(">slow<").unwrap());
        assert!(svg.starts_with("<svg "));
    }

    #[test]
    fn relative_labels_both_directions() {
        let rows = [
            Ratio {
                group: "a".to_owned(),
                rival: "x".to_owned(),
                ratio: 0.5,
            },
            Ratio {
                group: "b".to_owned(),
                rival: "y".to_owned(),
                ratio: 3.5,
            },
        ];
        let svg = document("T", "S", false, &relative(&rows), "f");
        assert!(svg.contains("2.0x faster"));
        assert!(svg.contains("3.5x slower"));
    }
}
