// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::collections::BTreeSet;
use std::path::Path;
use purrdf_testkit::bench::counter::{CountReport, read_report};

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args[0].as_str() {
        "compare" => {
            assert_eq!(args.len(), 7, "compare CASE LANE THREADS UNIT NEW BASE");
            let components = ["governor_counts", &args[1], &args[2], &args[3], &args[4]];
            let new = read_report(&components, &args[5]).expect("new explicit-unit record");
            let base = read_report(&components, &args[6]).expect("base explicit-unit record");
            new.check_compatible(&base).expect("matching measurement contexts");
            let delta = new.estimates().median - base.estimates().median;
            match new.compare(&args[6], &base, 478) {
                Ok(change) => println!("{}/{}/{}\t{}\t{}->{}\tbase={}\tnew={}\tdelta={}\tpercent={}\tci=[{},{}]\t{:?}",
                    args[1], args[2], args[3], args[4], args[6], args[5], base.estimates().median, new.estimates().median,
                    delta, 100.0 * change.point, 100.0 * change.ci_low, 100.0 * change.ci_high, change.verdict),
                Err(error) => {
                    assert!(base.raw_samples().iter().any(|sample| *sample <= 0), "unexpected refusal: {error}");
                    println!("{}/{}/{}\t{}\t{}->{}\tbase={}\tnew={}\tabsolute_delta={}\trelative_refusal={error}",
                        args[1], args[2], args[3], args[4], args[6], args[5], base.estimates().median, new.estimates().median, delta);
                }
            }
        }
        "ratio" => {
            assert_eq!(args.len(), 4, "ratio THREADS RECORD MAXIMUM");
            let plain = read_report(&["governor_counts", "numeric", "ungoverned", &args[1], "instructions"], &args[2]).unwrap();
            let metered = read_report(&["governor_counts", "numeric", "metered", &args[1], "instructions"], &args[2]).unwrap();
            metered.check_compatible(&plain).unwrap();
            let ratio = metered.estimates().median / plain.estimates().median;
            println!("numeric/{}/{}\tplain={}\tmetered={}\tratio={ratio}\tlimit={}", args[2], args[1], plain.estimates().median, metered.estimates().median, args[3]);
            assert!(ratio <= args[3].parse::<f64>().unwrap());
        }
        "guards" => {
            assert!(args.len() >= 6, "guards CASE LANE THREADS RECORD RECORD...");
            let home = std::env::var_os("PURRDF_BENCH_HOME").unwrap();
            let prefix = format!("{}-{}-{}-", args[1], args[2], args[3]);
            let mut expected = None;
            for record in &args[4..] {
                let mut guards = BTreeSet::new();
                let mut samples = 0;
                for run in std::fs::read_dir(Path::new(&home).join("raw-counts").join(record)).unwrap() {
                    let run = run.unwrap();
                    if !run.file_name().to_str().unwrap().starts_with(&prefix) { continue; }
                    for sample in std::fs::read_dir(run.path()).unwrap() {
                        let sample = sample.unwrap();
                        if !sample.file_name().to_str().unwrap().starts_with("sample-") { continue; }
                        let text = std::fs::read_to_string(sample.path().join("workload.stdout")).unwrap();
                        let found: Vec<_> = text.lines().filter(|line| line.starts_with("GUARD ")).collect();
                        assert_eq!(found.len(), 1, "one full guard receipt per sample");
                        guards.insert(found[0].to_owned());
                        samples += 1;
                    }
                }
                assert!(samples >= 10, "all fixed samples must be present for {record}");
                assert_eq!(guards.len(), 1, "all complete typed/evidence guards within {record} must agree");
                let guard = guards.into_iter().next().unwrap();
                if let Some(previous) = &expected { assert_eq!(&guard, previous, "full cross-artifact GovernorEvidence/fixture/answers differ: {record}"); }
                else { expected = Some(guard); }
                println!("GUARDS {}/{}/{} record={record} samples={samples} full_receipt_equality=PASS", args[1], args[2], args[3]);
            }
        }
        "inspect" => {
            assert_eq!(args.len(), 2);
            let text = std::fs::read_to_string(&args[1]).unwrap();
            let report = CountReport::from_json(&text).unwrap();
            println!("{:?}", report.context());
        }
        other => panic!("unknown action {other}"),
    }
}
