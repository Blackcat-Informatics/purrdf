// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Retired user-mode instructions per row of one frozen-dataset workload.
//!
//! `cargo bench -p purrdf-shapes --bench frozen_eval_counts [-- <id filter>]`
//! (needs `perf` and Linux). Each sample is a fresh child process run under
//! `perf stat -e instructions:u` through the test kit's counter protocol: the
//! child builds the fixture and the row's setup, runs a warm row once, enables
//! the counter, runs the measured work once, disables it and prints the
//! witness. Fixture build, parsing and warm-up are outside the counted window;
//! worker threads are included. `PURRDF_FROZEN_EVAL_COUNT_SAMPLES` sets the
//! samples per row (3 by default). The routine size is 22000 quads; `PURRDF_FROZEN_EVAL_QUADS`
//! adds one, and a size above 4.4 million also needs
//! `PURRDF_FROZEN_EVAL_ALLOW_LARGE=1`, refused before anything is built.
//!
//! The parent first verifies every row of the size in process (rows of one query
//! or shape set must agree across the freeze variants) and compares each child's witness with its own.
//! Output lines are `count <quads> <row id> samples=<n,...> witness=<count>`.

#[cfg(target_os = "linux")]
#[path = "support/frozen_eval.rs"]
mod fixture;

#[cfg(target_os = "linux")]
fn child(quads: usize, id: &str) {
    use fixture::{Detail, Fixtures};
    use purrdf_testkit::bench::counter::{CounterCommand, CounterControl};
    let fx = Fixtures::new(quads);
    let rows = fixture::rows();
    let row = rows
        .iter()
        .find(|row| row.id == id)
        .unwrap_or_else(|| panic!("no row {id}"));
    let mut op = fixture::op(row, &fx);
    if row.spec.repeatable() {
        let _ = op.run(Detail::Count);
    }
    // The global pool's workers exist before the counter is enabled.
    rayon::broadcast(|_| ());
    let mut control = CounterControl::required().expect("admitted instruction counter");
    control.command(CounterCommand::Enable).expect("enable");
    let witness = op.run(Detail::Count);
    control.command(CounterCommand::Disable).expect("disable");
    println!("witness={}", witness.count);
}

#[cfg(target_os = "linux")]
fn parent() {
    use fixture::Fixtures;
    use purrdf_testkit::bench::counter::collect_instructions;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let filters: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    let samples: usize = std::env::var("PURRDF_FROZEN_EVAL_COUNT_SAMPLES").map_or(3, |raw| {
        raw.parse().expect("the sample count is an integer")
    });
    let executable = std::env::current_exe().expect("the bench binary path");
    let home = std::env::temp_dir().join(format!("frozen_eval_counts_{}", std::process::id()));
    std::fs::create_dir_all(&home).expect("the scratch directory");
    for quads in fixture::sizes("PURRDF_FROZEN_EVAL_QUADS") {
        let fx = Fixtures::new(quads);
        let rows = fixture::rows();
        let verified = fixture::verify(&fx, &rows);
        for (n, (id, expected)) in verified.iter().enumerate() {
            if !filters.is_empty() && !filters.iter().any(|f| id.contains(f.as_str())) {
                continue;
            }
            let mut counts = Vec::new();
            for sample in 0..samples {
                let directory = home.join(format!("{quads}_{n}_{sample}"));
                let args = [
                    "--child".into(),
                    quads.to_string().into(),
                    id.clone().into(),
                ];
                let taken = collect_instructions(&executable, &args, &directory)
                    .unwrap_or_else(|error| panic!("{id}: {error}"));
                let stdout = std::fs::read_to_string(directory.join("workload.stdout"))
                    .expect("the child's output");
                // A bind reports its results only to the equality check, which
                // validates outside the counted window; every other row's child
                // must return what the verified run returned.
                if !id.starts_with("shacl_bind/") {
                    assert_eq!(
                        stdout.trim(),
                        format!("witness={}", expected.count),
                        "{id}: the child did different work from the verified run"
                    );
                }
                counts.push(taken.instructions);
            }
            let list: Vec<String> = counts.iter().map(u64::to_string).collect();
            println!(
                "count {quads} {id} samples={} witness={}",
                list.join(","),
                expected.count
            );
        }
    }
    let _ = std::fs::remove_dir_all(&home);
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("--child") {
            child(args[2].parse().expect("the size"), &args[3]);
        } else {
            parent();
        }
    }
    #[cfg(not(target_os = "linux"))]
    panic!("native instruction counting requires Linux perf");
}
