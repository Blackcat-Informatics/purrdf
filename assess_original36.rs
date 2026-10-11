// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Offline receipt projection; statistics use the original shipping bench home.
use purrdf_testkit::bench::{Change, Estimates, Verdict};
use std::{error::Error, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if !(1..=2).contains(&args.len()) {
        return Err(
            "expected the absolute Stage receipt directory and optional estimates directory".into(),
        );
    }
    let stage = PathBuf::from(&args[0]);
    let original = fs::read_to_string(stage.join("default-compact-current.log"))?;
    let lanes: Vec<_> = original
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let lane = fields.next()?;
            (fields.next()? == "time:").then_some(lane)
        })
        .collect();
    if lanes.len() != 36 {
        return Err(format!("expected all original 36 lanes, found {}", lanes.len()).into());
    }
    let estimates = args.get(1).map_or_else(
        || stage.join("default-alternating-estimates"),
        PathBuf::from,
    );
    let mut rejected = 0;
    println!("lane\toriginal_ns\tcurrent_ns\tchange_pct\tlow_pct\thigh_pct\tverdict");
    for (ordinal, lane) in lanes.iter().enumerate() {
        let root = estimates.join(lane);
        let (base_record, current_record) = if ordinal % 2 == 0 {
            ("original-main", "new")
        } else {
            ("new", "candidate-source")
        };
        let base = Estimates::from_json(&fs::read_to_string(
            root.join(base_record).join("estimates.json"),
        )?)?;
        let current = Estimates::from_json(&fs::read_to_string(
            root.join(current_record).join("estimates.json"),
        )?)?;
        let seed =
            purrdf_hash::mix::splitmix64_finalize(purrdf_hash::fnv::fnv1a64(lane.as_bytes()));
        let change = Change::between("original-main", &current, &base, seed);
        rejected += usize::from(change.verdict == Verdict::Regressed);
        println!(
            "{lane}\t{}\t{}\t{}\t{}\t{}\t{:?}",
            base.median,
            current.median,
            100.0 * change.point,
            100.0 * change.ci_low,
            100.0 * change.ci_high,
            change.verdict
        );
    }
    println!("COMPLETE lanes=36 significant_regressions={rejected}");
    if rejected != 0 {
        return Err(
            format!("default acceptance rejected: {rejected} significant regressions").into(),
        );
    }
    Ok(())
}
