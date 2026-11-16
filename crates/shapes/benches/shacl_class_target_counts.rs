// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

// Bench targets are not public API, so the workspace `missing_docs` lint is
// not asked of their items.
#![allow(missing_docs)]

//! Retired user-mode instructions of class-targeted SHACL validation.
//!
//! Linux only (needs `perf`). Each sample is a fresh child process run under
//! `perf stat -e instructions:u` through the test kit's counter protocol: the
//! child builds the fixture, runs one warm validation, enables the counter,
//! validates once and disables it. Fixture build, parsing and the warm run are
//! outside the counted window; worker threads are included. The workload is
//! `support/class_targets.rs`, shared with the `shacl_focus_class_targets` group
//! of `validate.rs`. `PURRDF_CLASS_TARGET_SAMPLES` sets the samples per row (3 by
//! default). Output lines are `count <subjects> samples=<n,...>`.

#[cfg(target_os = "linux")]
#[allow(dead_code, reason = "each target uses part of the shared workload")]
#[path = "support/class_targets.rs"]
mod fixture;

/// Subjects per row and the number of class-targeted shapes they spread over.
#[cfg(target_os = "linux")]
const SUBJECTS: &[usize] = &[18_100, 181_000];
#[cfg(target_os = "linux")]
const CLASSES: usize = 183;

#[cfg(target_os = "linux")]
fn child(subjects: usize) {
    use purrdf_testkit::bench::counter::{CounterCommand, CounterControl};
    let fx = fixture::fixture(subjects, CLASSES);
    let _ = fixture::validate(&fx);
    // The global pool's workers exist before the counter is enabled.
    rayon::broadcast(|_| ());
    let mut control = CounterControl::required().expect("admitted instruction counter");
    control.command(CounterCommand::Enable).expect("enable");
    let results = fixture::validate(&fx);
    control.command(CounterCommand::Disable).expect("disable");
    println!("witness={results}");
}

#[cfg(target_os = "linux")]
fn parent() {
    use purrdf_testkit::bench::counter::collect_instructions;
    let samples: usize = std::env::var("PURRDF_CLASS_TARGET_SAMPLES").map_or(3, |raw| {
        raw.parse().expect("the sample count is an integer")
    });
    let executable = std::env::current_exe().expect("the bench binary path");
    let home =
        std::env::temp_dir().join(format!("shacl_class_target_counts_{}", std::process::id()));
    std::fs::create_dir_all(&home).expect("the scratch directory");
    for &subjects in SUBJECTS {
        let mut counts = Vec::new();
        for sample in 0..samples {
            let directory = home.join(format!("{subjects}_{sample}"));
            let args = ["--child".into(), subjects.to_string().into()];
            let taken = collect_instructions(&executable, &args, &directory)
                .unwrap_or_else(|error| panic!("{subjects}: {error}"));
            let stdout = std::fs::read_to_string(directory.join("workload.stdout"))
                .expect("the child's output");
            assert_eq!(
                stdout.trim(),
                "witness=0",
                "{subjects}: a conforming fixture produced results"
            );
            counts.push(taken.instructions);
        }
        let list: Vec<String> = counts.iter().map(u64::to_string).collect();
        println!("count {subjects} samples={}", list.join(","));
    }
    let _ = std::fs::remove_dir_all(&home);
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("--child") {
            child(args[2].parse().expect("the size"));
        } else {
            parent();
        }
    }
    #[cfg(not(target_os = "linux"))]
    panic!("native instruction counting requires Linux perf");
}
