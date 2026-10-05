// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Report-only shallow owned-term Clone, equality, Hash and Debug latency.

use std::hash::{Hash, Hasher};
use std::hint::black_box;

use purrdf_testkit::bench::{Bench, BenchmarkGroup, bench_group, bench_main};

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
        // The old compiler-derived model and the new implementation are measured
        // adjacently in this same binary; projection and copies stay outside timing.
        let derived = model_terms::oracle(term);
        let derived_other = derived.clone();
        let other = term.clone();
        let mut group = c.benchmark_group(format!("owned_model/{label}"));
        clone(&mut group, "clone", term);
        clone(&mut group, "derived_clone", &derived);
        equality(&mut group, "eq", term, &other);
        equality(&mut group, "derived_eq", &derived, &derived_other);
        hash(&mut group, "hash", term);
        hash(&mut group, "derived_hash", &derived);
        debug(&mut group, "debug", term);
        debug(&mut group, "derived_debug", &derived);
        group.finish();
    }
}

bench_group!(benches, traits);
bench_main!(benches);
