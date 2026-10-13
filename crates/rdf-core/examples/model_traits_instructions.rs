// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Instruction counts for the owned-model trait cases of `benches/model_traits.rs`,
//! read from the user-space retired-instruction counter (`perf stat -e
//! instructions:u`): independent of host load, where the wall-clock bench is
//! not. Run it pinned to one CPU so the process stays single-threaded on one
//! core; it starts no threads of its own.
//!
//! ```text
//! cargo run --release -p purrdf-core --example model_traits_instructions -- \
//!     report [--baseline OTHER_BUILD_OF_THIS_EXAMPLE] [--runs 5] [--iters 20000] [--cases a,b]
//! ```
//!
//! Each case is run twice at the same iteration count, once
//! performing the operation and once only preparing for it; the difference
//! divided by the count is the operation's instructions. The median of
//! `--runs` repetitions is reported. Three implementations are measured: the
//! model itself (`ours`), the independent compiler-derived model (`derived`),
//! and a second derived copy (`control`) that must agree with `derived` to
//! within 0.1%, or the measurement itself is not trusted. `ours` may cost at
//! most 1% more than `derived`, and, with `--baseline` naming this example
//! built from another revision, at most 1% more than that revision's `ours`.
//! The process exits non-zero when any gate fails.

use std::fmt::Write as _;
use std::hash::{Hash, Hasher};
use std::hint::black_box;
use std::process::{Command, ExitCode};

#[path = "../tests/support/model_terms.rs"]
mod model_terms;

const OPERATIONS: [&str; 5] = ["clone", "eq", "hash", "debug", "drop"];
const IMPLEMENTATIONS: [&str; 3] = ["ours", "derived", "control"];
/// The largest instruction increase `ours` may show over a reference.
const GATE: f64 = 0.01;
/// The largest disagreement the control may show with the derived model.
const CONTROL_GATE: f64 = 0.001;

/// Run `operation` on `term` `iterations` times, or only prepare for it.
fn run<T: Clone + Eq + Hash + std::fmt::Debug>(
    term: &T,
    operation: &str,
    iterations: usize,
    active: bool,
) {
    let other = term.clone();
    let mut rendered = String::new();
    match operation {
        "drop" => {
            let copies: Vec<T> = (0..iterations).map(|_| term.clone()).collect();
            for copy in copies {
                if active {
                    drop(black_box(copy));
                } else {
                    std::mem::forget(black_box(copy));
                }
            }
        }
        _ if !active => {}
        "clone" => {
            for _ in 0..iterations {
                drop(black_box(black_box(term).clone()));
            }
        }
        "eq" => {
            for _ in 0..iterations {
                black_box(black_box(term) == black_box(&other));
            }
        }
        "hash" => {
            for _ in 0..iterations {
                let mut state = purrdf_hash::fixed::FixedHasher::default();
                black_box(term).hash(&mut state);
                black_box(state.finish());
            }
        }
        "debug" => {
            for _ in 0..iterations {
                rendered.clear();
                write!(rendered, "{:?}", black_box(term)).expect("a String accepts formatting");
                black_box(rendered.len());
            }
        }
        other => panic!("unknown operation {other:?}"),
    }
}

/// The `measure` mode, the process whose instructions are counted.
fn measure(case: &str, operation: &str, implementation: &str, iterations: usize, active: bool) {
    let fixtures = model_terms::fixtures();
    let (_, term) = fixtures
        .iter()
        .find(|(label, _)| *label == case)
        .unwrap_or_else(|| panic!("unknown case {case:?}"));
    match implementation {
        "ours" => run(term, operation, iterations, active),
        "derived" | "control" => run(&model_terms::oracle(term), operation, iterations, active),
        other => panic!("unknown implementation {other:?}"),
    }
}

/// User-space instructions retired by one `measure` run of `binary`, read from
/// the hardware counter through `perf stat`.
fn collected(binary: &str, arguments: &[String]) -> u64 {
    let output = Command::new("perf")
        .args(["stat", "-x", ",", "-e", "instructions:u", binary, "measure"])
        .args(arguments)
        .output()
        .expect("perf runs");
    assert!(output.status.success(), "perf stat failed: {output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr
        .lines()
        .find_map(|line| {
            let mut fields = line.split(',');
            let count = fields.next()?;
            (fields.nth(1)? == "instructions:u").then(|| count.parse().ok())?
        })
        .unwrap_or_else(|| panic!("no instruction count in perf output:\n{stderr}"))
}

/// The median per-operation instructions of one case on `binary`.
fn per_operation(
    binary: &str,
    case: &str,
    operation: &str,
    implementation: &str,
    iterations: usize,
    runs: usize,
) -> f64 {
    let mut samples: Vec<f64> = (0..runs)
        .map(|_| {
            let arguments = |active: &str| {
                [
                    case,
                    operation,
                    implementation,
                    &iterations.to_string(),
                    active,
                ]
                .map(str::to_owned)
                .to_vec()
            };
            let active = collected(binary, &arguments("1"));
            let idle = collected(binary, &arguments("0"));
            #[expect(
                clippy::cast_precision_loss,
                reason = "instruction counts are far below 2^52"
            )]
            let per = (active as f64 - idle as f64) / iterations as f64;
            per
        })
        .collect();
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}

fn change(new: f64, base: f64) -> f64 {
    new / base - 1.0
}

fn report(arguments: &[String]) -> ExitCode {
    let mut baseline = None;
    let mut runs = 5;
    let mut iterations = 20_000;
    let mut cases: Option<Vec<String>> = None;
    let mut rest = arguments.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().expect("every flag takes a value");
        match flag.as_str() {
            "--baseline" => baseline = Some(value.clone()),
            "--runs" => runs = value.parse().expect("--runs is a count"),
            "--iters" => iterations = value.parse().expect("--iters is a count"),
            "--cases" => cases = Some(value.split(',').map(str::to_owned).collect()),
            other => panic!("unknown flag {other:?}"),
        }
    }
    let current = std::env::current_exe().expect("the example knows its own path");
    let current = current.to_str().expect("a UTF-8 path");
    let mut failed = false;
    println!(
        "| case | op | ours | derived | control | ours vs derived | control vs derived |{}",
        if baseline.is_some() {
            " baseline ours | ours vs baseline |"
        } else {
            ""
        }
    );
    println!(
        "|---|---|---|---|---|---|---|{}",
        if baseline.is_some() { "---|---|" } else { "" }
    );
    for (case, _) in model_terms::fixtures() {
        if cases
            .as_ref()
            .is_some_and(|cases| !cases.iter().any(|name| name == case))
        {
            continue;
        }
        for operation in OPERATIONS {
            let [ours, derived, control] = IMPLEMENTATIONS
                .map(|which| per_operation(current, case, operation, which, iterations, runs));
            let paired = change(ours, derived);
            let noise = change(control, derived);
            let mut verdict = |delta: f64, limit: f64, symmetric: bool| {
                let bad = if symmetric {
                    delta.abs() > limit
                } else {
                    delta > limit
                };
                failed |= bad;
                format!("{:+.3}%{}", 100.0 * delta, if bad { " FAIL" } else { "" })
            };
            let mut line = format!(
                "| {case} | {operation} | {ours:.1} | {derived:.1} | {control:.1} | {} | {} |",
                verdict(paired, GATE, false),
                verdict(noise, CONTROL_GATE, true),
            );
            if let Some(baseline) = &baseline {
                let base = per_operation(baseline, case, operation, "ours", iterations, runs);
                let delta = verdict(change(ours, base), GATE, false);
                write!(line, " {base:.1} | {delta} |").expect("a String accepts formatting");
            }
            println!("{line}");
        }
    }
    if failed {
        println!("GATE FAILED");
        ExitCode::FAILURE
    } else {
        println!("gate passed");
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.split_first() {
        Some((mode, rest)) if mode == "measure" => {
            let [case, operation, implementation, iterations, active] = rest else {
                panic!("measure CASE OP IMPL ITERS ACTIVE")
            };
            measure(
                case,
                operation,
                implementation,
                iterations.parse().expect("ITERS is a count"),
                active == "1",
            );
            ExitCode::SUCCESS
        }
        Some((mode, rest)) if mode == "report" => report(rest),
        _ => {
            eprintln!(
                "usage: model_traits_instructions report [--baseline BIN] [--runs N] [--iters N]"
            );
            ExitCode::FAILURE
        }
    }
}
