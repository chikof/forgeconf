//! Readers for the files the benchmark scripts leave behind.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::fmt;

type Result<T, E = Box<dyn Error>> = std::result::Result<T, E>;

/// Criterion groups in display order, with the subtitle shown under each panel
/// title.
pub const GROUPS: [(&str, &str); 10] = [
    ("toml/small", "TOML → struct, small document"),
    ("toml/large", "TOML → struct, 1000-entry document"),
    ("json/small", "JSON → struct, small document"),
    ("json/large", "JSON → struct, 1000-entry document"),
    ("yaml/small", "YAML → struct, small document"),
    ("yaml/large", "YAML → struct, 1000-entry document"),
    ("layered/file+env", "TOML file + env overrides → struct"),
    ("cli/parse", "argv → flat struct"),
    ("cli/help", "render the --help text"),
    ("errors/report", "bad value + unknown key → error text"),
];

pub const LIBRARY: &str = "forgeconf";

pub struct Estimate {
    pub lib: String,
    pub median: f64,
    pub lo: f64,
    pub hi: f64,
}

pub struct Criterion {
    /// `(group id, estimates)` in [`GROUPS`] order, groups without data left
    /// out.
    pub groups: Vec<(&'static str, Vec<Estimate>)>,
}

fn field(value: &Value, path: &[&str]) -> Option<f64> {
    path.iter().try_fold(value, |v, key| v.get(key))?.as_f64()
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?)
}

pub fn criterion(benchmarks: &Path) -> Result<Criterion> {
    let root = benchmarks.join("target/criterion");
    let missing =
        || format!("no criterion results under {}; run `scripts/bench.sh runtime`", root.display());
    let mut found: HashMap<String, Vec<Estimate>> = HashMap::new();
    for group_dir in fs::read_dir(&root).map_err(|_| missing())? {
        let group_dir = group_dir?.path();
        if !group_dir.is_dir() {
            continue;
        }
        for bench_dir in fs::read_dir(&group_dir)? {
            let new = bench_dir?.path().join("new");
            let (info, estimates) = (new.join("benchmark.json"), new.join("estimates.json"));
            if !info.is_file() || !estimates.is_file() {
                continue;
            }
            let info = read_json(&info)?;
            let estimates = read_json(&estimates)?;
            let (Some(group), Some(lib)) = (
                info.get("group_id").and_then(Value::as_str),
                info.get("function_id").and_then(Value::as_str),
            ) else {
                continue;
            };
            let median = |path: &[&str]| field(&estimates, path);
            let (Some(point), Some(lo), Some(hi)) = (
                median(&["median", "point_estimate"]),
                median(&["median", "confidence_interval", "lower_bound"]),
                median(&["median", "confidence_interval", "upper_bound"]),
            ) else {
                return Err(format!(
                    "{}: no median estimate; rerun `scripts/bench.sh runtime`",
                    new.join("estimates.json").display()
                )
                .into());
            };
            found.entry(group.to_owned()).or_default().push(Estimate {
                lib: lib.to_owned(),
                median: point,
                lo,
                hi,
            });
        }
    }
    let groups: Vec<_> = GROUPS
        .iter()
        .filter_map(|(id, _)| found.remove(*id).map(|estimates| (*id, estimates)))
        .collect();
    if groups.is_empty() {
        return Err(missing().into());
    }
    Ok(Criterion { groups })
}

pub struct FootprintRow {
    pub lib: String,
    pub build_median: f64,
    pub build_min: f64,
    pub build_max: f64,
    pub kib: f64,
    pub crates: f64,
}

pub struct Footprint {
    pub context: HashMap<String, String>,
    pub rows: Vec<FootprintRow>,
}

/// Parses `# key=value<TAB>key=value...` comment lines into a map.
fn context_line(line: &str, into: &mut HashMap<String, String>) {
    for pair in line.trim_start_matches('#').trim().split('\t') {
        if let Some((key, value)) = pair.split_once('=') {
            into.insert(key.trim().to_owned(), value.trim().to_owned());
        }
    }
}

pub fn parse_footprint(text: &str) -> Result<Footprint> {
    let mut context = HashMap::new();
    let mut rows = Vec::new();
    let mut header_seen = false;
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        if line.starts_with('#') {
            context_line(line, &mut context);
        } else if !header_seen {
            header_seen = true;
        } else {
            let cols: Vec<&str> = line.split('\t').collect();
            let [lib, med, min, max, kib, normal, _build] = cols[..] else {
                return Err(format!("footprint.tsv: expected 7 columns, got `{line}`").into());
            };
            let num = |s: &str| -> Result<f64> {
                s.parse()
                    .map_err(|_| format!("footprint.tsv: `{s}` is not a number").into())
            };
            rows.push(FootprintRow {
                lib: lib.to_owned(),
                build_median: num(med)?,
                build_min: num(min)?,
                build_max: num(max)?,
                kib: num(kib)?,
                crates: num(normal)?,
            });
        }
    }
    if rows.is_empty() {
        return Err("footprint.tsv has no rows".into());
    }
    Ok(Footprint { context, rows })
}

pub fn footprint(benchmarks: &Path) -> Result<Footprint> {
    let path = benchmarks.join("results/footprint.tsv");
    let text = fs::read_to_string(&path)
        .map_err(|_| format!("no {}; run `scripts/bench.sh footprint`", path.display()))?;
    parse_footprint(&text)
}

pub struct InstructionRow {
    pub scenario: String,
    pub lib: String,
    pub count: f64,
}

pub fn parse_instructions(text: &str) -> Result<Vec<InstructionRow>> {
    let mut rows = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let cols: Vec<&str> = line.split('\t').collect();
        let [scenario, lib, count] = cols[..] else {
            return Err(format!("instructions.tsv: expected 3 columns, got `{line}`").into());
        };
        rows.push(InstructionRow {
            scenario: scenario.to_owned(),
            lib: lib.to_owned(),
            count: count
                .parse()
                .map_err(|_| format!("instructions.tsv: `{count}` is not a number"))?,
        });
    }
    if rows.is_empty() {
        return Err("instructions.tsv has no rows".into());
    }
    Ok(rows)
}

pub fn instructions(benchmarks: &Path) -> Result<Vec<InstructionRow>> {
    let path = benchmarks.join("results/instructions.tsv");
    let text = fs::read_to_string(&path)
        .map_err(|_| format!("no {}; run `scripts/bench.sh instructions`", path.display()))?;
    parse_instructions(&text)
}

/// Toolchain and machine facts printed in every chart footer.
pub struct Meta {
    pub rustc: String,
    pub cores: String,
    pub date: String,
}

impl Meta {
    /// Prefers the values recorded with the footprint run, else asks the
    /// machine.
    pub fn resolve(footprint: Option<&Footprint>) -> Meta {
        let recorded = |key: &str| footprint.and_then(|f| f.context.get(key)).cloned();
        Meta {
            rustc: recorded("rustc").unwrap_or_else(|| {
                Command::new("rustc")
                    .arg("-V")
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
                    .unwrap_or_else(|| "rustc unknown".to_owned())
            }),
            cores: recorded("cores").unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map_or_else(|_| "?".to_owned(), |n| n.to_string())
            }),
            date: recorded("date").unwrap_or_else(|| {
                let secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                fmt::civil_date((secs / 86_400) as i64)
            }),
        }
    }

    pub fn footer(&self) -> String {
        format!("{} · {} cores · {}", self.rustc, self.cores, self.date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_footprint_tsv() {
        let text = [
            "# rustc=rustc 1.0 (abc)\tcores=16\tdate=2026-11-03\truns=3\tjobs=8",
            "library\tmedian\tmin\tmax\tkib\tnormal\tbuild",
            "forgeconf\t2.195\t2.1\t2.3\t582.1\t7\t7",
        ]
        .join("\n");
        let fp = parse_footprint(&text).unwrap();
        assert_eq!(fp.context["rustc"], "rustc 1.0 (abc)");
        assert_eq!(fp.context["jobs"], "8");
        assert_eq!(fp.rows.len(), 1);
        assert_eq!(fp.rows[0].build_max, 2.3);
        assert_eq!(fp.rows[0].crates, 7.0);
    }

    #[test]
    fn rejects_short_rows() {
        assert!(parse_footprint("h\nforgeconf\t1\n").is_err());
        assert!(parse_instructions("toml-small\tforgeconf\n").is_err());
    }

    #[test]
    fn parses_instructions_tsv() {
        let rows = parse_instructions("toml-small\tforgeconf\t30550\ncli\tclap\t60197\n").unwrap();
        assert_eq!(rows[1].lib, "clap");
        assert_eq!(rows[1].count, 60197.0);
    }
}
