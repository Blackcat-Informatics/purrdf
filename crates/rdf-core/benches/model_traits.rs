// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only shallow owned-term Clone, equality, Hash and Debug latency.

use std::hash::{Hash, Hasher};
use std::hint::black_box;

use purrdf_testkit::bench::{Bench, BenchmarkGroup, Change, bench_group, bench_main};

#[path = "../tests/support/model_terms.rs"]
mod model_terms;

fn clone<T: Clone>(group: &mut BenchmarkGroup<'_>, name: &str, term: &T) {
    group.bench_function(name, |b| b.iter(|| black_box(black_box(term).clone())));
}

fn equality<T: Eq>(group: &mut BenchmarkGroup<'_>, name: &str, term: &T, other: &T) {
    group.bench_function(name, |b| {
        b.iter(|| black_box(black_box(term) == black_box(other)));
    });
}

fn hash<T: Hash>(group: &mut BenchmarkGroup<'_>, name: &str, term: &T) {
    group.bench_function(name, |b| {
        b.iter(|| {
            let mut state = purrdf_hash::fixed::FixedHasher::default();
            black_box(term).hash(&mut state);
            black_box(state.finish())
        });
    });
}

fn debug<T: std::fmt::Debug>(group: &mut BenchmarkGroup<'_>, name: &str, term: &T) {
    let mut rendered = String::with_capacity(format!("{term:?}").len());
    group.bench_function(name, |b| {
        b.iter(|| {
            use std::fmt::Write as _;
            rendered.clear();
            write!(rendered, "{:?}", black_box(term)).expect("a String accepts formatting");
            black_box(rendered.len())
        });
    });
}

fn traits(c: &mut Bench) {
    let fixtures = model_terms::fixtures();
    for (label, term) in &fixtures {
        // The compiler-derived model and the implementation are measured
        // adjacently in this same binary; projection and copies stay outside
        // timing. The control measures a second, separately allocated copy of
        // the derived model after both: the same code timed again, so its
        // verdict is the run's own noise floor for that pair.
        let derived = model_terms::oracle(term);
        let derived_other = derived.clone();
        let control = model_terms::oracle(term);
        let control_other = control.clone();
        let other = term.clone();
        let mut group = c.benchmark_group(format!("owned_model/{label}"));
        clone(&mut group, "derived_clone", &derived);
        clone(&mut group, "clone", term);
        clone(&mut group, "control_clone", &control);
        equality(&mut group, "derived_eq", &derived, &derived_other);
        equality(&mut group, "eq", term, &other);
        equality(&mut group, "control_eq", &control, &control_other);
        hash(&mut group, "derived_hash", &derived);
        hash(&mut group, "hash", term);
        hash(&mut group, "control_hash", &control);
        debug(&mut group, "derived_debug", &derived);
        debug(&mut group, "debug", term);
        debug(&mut group, "control_debug", &control);
        group.finish();
    }

    // Reuse the harness's bootstrap law on this run's adjacent pairs. Its normal
    // saved-record lines compare historical runs, not the independent derive.
    let outcomes = c.outcomes();
    for outcome in &outcomes {
        let Some((prefix, operation)) = outcome.id.rsplit_once('/') else {
            continue;
        };
        if operation.starts_with("derived_") {
            continue;
        }
        let (kind, operation) = operation
            .strip_prefix("control_")
            .map_or(("paired", operation), |operation| ("control", operation));
        let derived_id = format!("{prefix}/derived_{operation}");
        let derived = outcomes.iter().find(|item| item.id == derived_id);
        if let (Some(new), Some(base)) = (
            outcome.estimates.as_ref(),
            derived.and_then(|item| item.estimates.as_ref()),
        ) {
            let change = Change::between(
                &derived_id,
                new,
                base,
                purrdf_hash::fnv::fnv1a64(outcome.id.as_bytes()),
            );
            println!(
                "{kind} {}: [{:+.4}% {:+.4}% {:+.4}%] against independent derive: {}",
                outcome.id,
                100.0 * change.ci_low,
                100.0 * change.point,
                100.0 * change.ci_high,
                change.verdict.sentence(),
            );
        }
    }
}

bench_group!(benches, traits);
bench_main!(benches);
