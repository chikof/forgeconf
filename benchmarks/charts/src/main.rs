//! Renders the benchmark charts in `benchmarks/images/` from the files the
//! benchmark scripts leave in `benchmarks/target/criterion` and
//! `benchmarks/results`.
//!
//! Usage: `charts <path to benchmarks/>`; run it through `scripts/bench.sh
//! charts`.

mod data;
mod fmt;
mod svg;

use std::error::Error;
use std::fs;
use std::path::Path;

use data::{Criterion, Footprint, InstructionRow, LIBRARY, Meta};
use svg::{Bar, Panel, Ratio};

type Chart = Result<String, Box<dyn Error>>;

fn is_forgeconf(lib: &str) -> bool {
    lib.starts_with(LIBRARY)
}

fn subtitle(group: &str) -> &'static str {
    data::GROUPS
        .iter()
        .find(|(id, _)| *id == group)
        .map_or("", |(_, subtitle)| subtitle)
}

fn runtime(criterion: &Criterion, meta: &Meta) -> String {
    let panels: Vec<Panel> = criterion
        .groups
        .iter()
        .map(|(group, estimates)| Panel {
            title: (*group).to_owned(),
            subtitle: subtitle(group).to_owned(),
            bars: estimates
                .iter()
                .map(|e| Bar {
                    label: e.lib.clone(),
                    value: e.median,
                    range: Some((e.lo, e.hi)),
                    accent: is_forgeconf(&e.lib),
                })
                .collect(),
            format: fmt::time,
        })
        .collect();
    svg::document(
        "Runtime: median time per call",
        "Shorter is faster. Whiskers show the 95 % confidence interval of the median.",
        true,
        &svg::grid(&panels, 2, 76.0),
        &meta.footer(),
    )
}

fn relative(criterion: &Criterion, meta: &Meta) -> Chart {
    let mut rows = Vec::new();
    for (group, estimates) in &criterion.groups {
        let Some(ours) = estimates.iter().find(|e| e.lib == LIBRARY) else {
            continue;
        };
        let best = estimates
            .iter()
            .filter(|e| e.lib != LIBRARY)
            .min_by(|a, b| a.median.total_cmp(&b.median));
        if let Some(best) = best {
            rows.push(Ratio {
                group: (*group).to_owned(),
                rival: best.lib.clone(),
                ratio: ours.median / best.median,
            });
        }
    }
    if rows.is_empty() {
        return Err("no group has both forgeconf and a competitor".into());
    }
    Ok(svg::document(
        "forgeconf against the fastest competitor",
        "Median time ratio per group on a log scale. Left is faster than the best other library, \
         right is slower.",
        false,
        &svg::relative(&rows),
        &meta.footer(),
    ))
}

fn footprint(fp: &Footprint, meta: &Meta) -> String {
    let context = |key: &str| fp.context.get(key).map_or("?", String::as_str);
    let bars = |value: fn(&data::FootprintRow) -> f64, range: bool| -> Vec<Bar> {
        fp.rows
            .iter()
            .map(|r| Bar {
                label: r.lib.clone(),
                value: value(r),
                range: range.then_some((r.build_min, r.build_max)),
                accent: is_forgeconf(&r.lib),
            })
            .collect()
    };
    let panels = [
        Panel {
            title: format!("Clean release build, {} parallel jobs", context("jobs")),
            subtitle: format!(
                "Seconds, median of {} runs, whiskers show min to max",
                context("runs")
            ),
            bars: bars(|r| r.build_median, true),
            format: fmt::seconds,
        },
        Panel {
            title: "Stripped binary size".to_owned(),
            subtitle: "KiB of the release binary after `strip`".to_owned(),
            bars: bars(|r| r.kib, false),
            format: fmt::kib,
        },
        Panel {
            title: "Crates in the dependency tree".to_owned(),
            subtitle: "Normal dependencies, the binary's own crate included".to_owned(),
            bars: bars(|r| r.crates, false),
            format: fmt::integer,
        },
    ];
    svg::document(
        "Footprint of a minimal binary per library",
        "Shorter is smaller or faster. Each binary loads the same fixture, or parses the same \
         arguments.",
        true,
        &svg::grid(&panels, 1, 96.0),
        &meta.footer(),
    )
}

/// `toml-small` becomes `toml/small`, `layered` becomes `layered/file+env`.
fn scenario_group(scenario: &str) -> String {
    match scenario {
        "layered" => "layered/file+env".to_owned(),
        "errors" => "errors/report".to_owned(),
        "cli" => "cli/parse".to_owned(),
        other => other.replacen('-', "/", 1),
    }
}

fn instructions(rows: &[InstructionRow], meta: &Meta) -> String {
    let mut panels: Vec<Panel> = Vec::new();
    for row in rows {
        let bar = Bar {
            label: row.lib.clone(),
            value: row.count,
            range: None,
            accent: is_forgeconf(&row.lib),
        };
        if let Some(panel) = panels
            .iter_mut()
            .find(|p| p.title == scenario_group(&row.scenario))
        {
            panel.bars.push(bar);
        } else {
            let group = scenario_group(&row.scenario);
            panels.push(Panel {
                subtitle: subtitle(&group).to_owned(),
                title: group,
                bars: vec![bar],
                format: fmt::count,
            });
        }
    }
    svg::document(
        "Instructions per iteration under callgrind",
        "Shorter is fewer instructions. Counts are deterministic, so small differences are real.",
        true,
        &svg::grid(&panels, 3, 68.0),
        &meta.footer(),
    )
}

fn run(benchmarks: &Path) -> Result<(), Box<dyn Error>> {
    let criterion = data::criterion(benchmarks);
    let footprint_data = data::footprint(benchmarks);
    let instruction_data = data::instructions(benchmarks);
    let meta = Meta::resolve(footprint_data.as_ref().ok());

    let charts: Vec<(&str, Chart)> = vec![
        (
            "runtime.svg",
            criterion
                .as_ref()
                .map(|c| runtime(c, &meta))
                .map_err(|e| e.to_string().into()),
        ),
        (
            "runtime-relative.svg",
            match &criterion {
                Ok(c) => relative(c, &meta),
                Err(e) => Err(e.to_string().into()),
            },
        ),
        (
            "footprint.svg",
            footprint_data
                .as_ref()
                .map(|f| footprint(f, &meta))
                .map_err(|e| e.to_string().into()),
        ),
        (
            "instructions.svg",
            instruction_data
                .as_ref()
                .map(|rows| instructions(rows, &meta))
                .map_err(|e| e.to_string().into()),
        ),
    ];

    let out_dir = benchmarks.join("images");
    fs::create_dir_all(&out_dir)?;
    let mut written = 0;
    for (name, chart) in charts {
        match chart {
            Ok(svg) => {
                let path = out_dir.join(name);
                fs::write(&path, svg).map_err(|e| format!("{}: {e}", path.display()))?;
                println!("wrote {}", path.display());
                written += 1;
            },
            Err(e) => eprintln!("warning: skipping {name}: {e}"),
        }
    }
    if written == 0 {
        return Err("no chart could be made: run `scripts/bench.sh runtime footprint \
                    instructions` first"
            .into());
    }
    Ok(())
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (Some(dir), None) = (args.next(), args.next()) else {
        eprintln!("usage: charts <path to benchmarks/>");
        std::process::exit(2);
    };
    if let Err(e) = run(Path::new(&dir)) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_names_map_to_group_ids() {
        assert_eq!(scenario_group("toml-small"), "toml/small");
        assert_eq!(scenario_group("layered"), "layered/file+env");
        assert_eq!(scenario_group("errors"), "errors/report");
        assert_eq!(scenario_group("cli"), "cli/parse");
    }
}
