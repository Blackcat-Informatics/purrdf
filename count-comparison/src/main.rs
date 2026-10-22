// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Compare existing shared collector records; this program executes no workload.

use std::path::Path;

use purrdf_testkit::bench::counter::{CountUnit, read_report};

fn guard(home: &Path, record: &str, case: &str, lane: &str, workers: usize) -> String {
    let prefix = format!("{case}-{lane}-{workers}-");
    let mut guards = Vec::new();
    for entry in std::fs::read_dir(home.join("raw-counts").join(record)).unwrap() {
        let entry = entry.unwrap();
        if !entry.file_name().to_string_lossy().starts_with(&prefix) {
            continue;
        }
        for sample in 0..10 {
            let path = entry
                .path()
                .join(format!("sample-{sample}/workload.stdout"));
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            let line = text
                .lines()
                .find(|line| line.starts_with("GUARD "))
                .unwrap_or_else(|| panic!("{}: complete guard receipt missing", path.display()));
            guards.push(line.to_owned());
        }
    }
    assert_eq!(
        guards.len(),
        20,
        "{record}/{case}/{lane}/{workers}: ten samples from each allocator mode"
    );
    let first = guards.first().unwrap();
    assert!(
        guards.iter().all(|guard| guard == first),
        "full receipts differ within {record}/{case}/{lane}/{workers}"
    );
    first.clone()
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let [base, candidate] = args.as_slice() else {
        panic!(
            "expected BASE_RECORD CANDIDATE_RECORD; PURRDF_BENCH_HOME must select the report directory"
        );
    };
    let home = std::env::var_os("PURRDF_BENCH_HOME").expect("report home required");
    println!("case\tlane\tworkers\tunit\tbaseline\tcandidate\tdelta\tpercent\tinterval");
    for (case, lanes) in [
        ("numeric", &["ungoverned", "fuel", "scratch", "metered"][..]),
        (
            "bind",
            &["ungoverned", "stop_signal", "fuel_and_stop_signal"][..],
        ),
    ] {
        for lane in lanes {
            for workers in [1, 4, 32] {
                let baseline_guard = guard(Path::new(&home), base, case, lane, workers);
                assert_eq!(
                    baseline_guard,
                    guard(Path::new(&home), candidate, case, lane, workers),
                    "full cross-artifact receipt differs: {case}/{lane}/{workers}"
                );
                for unit in [
                    CountUnit::Instructions,
                    CountUnit::AllocationCalls,
                    CountUnit::RequestedBytes,
                    CountUnit::RetainedBytes,
                    CountUnit::PeakBytes,
                ] {
                    let workers = workers.to_string();
                    let components = ["governor_counts", case, lane, &workers, unit.name()];
                    let before = read_report(&components, base).expect("complete baseline report");
                    let after =
                        read_report(&components, candidate).expect("complete candidate report");
                    after
                        .check_compatible(&before)
                        .expect("matching units, work, workers, fixture and retention boundary");
                    assert_eq!(before.raw_samples().len(), 10);
                    assert_eq!(after.raw_samples().len(), 10);
                    let old = before.estimates().median;
                    let new = after.estimates().median;
                    let comparison = if before.raw_samples().iter().all(|value| *value > 0) {
                        let change = after
                            .compare(base, &before, 481)
                            .expect("shared count comparison");
                        format!(
                            "{}\t[{},{}]",
                            change.point * 100.0,
                            change.ci_low * 100.0,
                            change.ci_high * 100.0
                        )
                    } else {
                        "absolute-only\tn/a".to_owned()
                    };
                    println!(
                        "{case}\t{lane}\t{workers}\t{}\t{old}\t{new}\t{}\t{comparison}",
                        unit.name(),
                        new - old
                    );
                }
                eprintln!(
                    "GUARDS {case}/{lane}/{workers}: twenty complete receipts per artifact agree; production guards check every typed answer"
                );
            }
        }
    }
}
