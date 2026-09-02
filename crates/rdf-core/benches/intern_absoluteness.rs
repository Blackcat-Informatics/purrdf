// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Intern-time cost of the IR-boundary absoluteness invariant.
//!
//! Interning an IRI now parses it once, through
//! `purrdf_iri::BaseScope::resolve` on an empty scope, to prove it is absolute
//! (`crates/rdf-core/src/ir/absolute.rs`). The design claim this bench exists to make
//! *readable* is the store-once split:
//!
//! * a **miss** (the first time a given string is interned) pays one O(len) parse;
//! * a **hit** (every subsequent intern of the same string) pays nothing at all,
//!   because the interner returns from its hash lookup before the check is reached.
//!
//! That is why the check sits in the miss branch rather than at the head of
//! `intern_iri`: real workloads intern the same predicate and datatype IRIs over and
//! over, so paying per-intern instead of per-distinct-value would be a per-quad tax.
//!
//! This is a **report-only** bench, per repo policy: it records wall-clock time so the
//! cost can be read, and asserts no threshold, speedup, or regression bound.
//!
//! 1. `distinct_iris_all_misses` — N DISTINCT IRIs into a fresh builder. Every intern
//!    is a miss, so this is the worst case: the invariant's full cost, paid N times.
//! 2. `repeated_iri_all_hits` — ONE IRI interned N times. The first is a miss and the
//!    other N-1 are hits, so this reads as the hit path's cost and shows the check is
//!    absent from it.
//! 3. `realistic_mixed_vocabulary` — N quads' worth of interning over a small fixed
//!    predicate/datatype vocabulary plus a distinct subject per quad, i.e. the shape
//!    an actual parse produces: a few IRIs hit constantly, subjects miss once each.
//! 4. `global_dictionary_all_misses` — the same worst case on the OTHER term table,
//!    `GlobalDictionary`, whose `intern` is a fallible constructor because it has no
//!    freeze step at which a deferred verdict could be reported. The paged seal and
//!    compaction do NOT take this path: they use the crate-internal
//!    `reintern_validated`, whose whole purpose is to skip a parse that a
//!    already-validated source table has already done. That entry point is
//!    `pub(crate)` and so deliberately unreachable from a bench; this group is the
//!    cost it avoids paying, once per term per page per seal.
//!
//! Inputs are generated deterministically (no RNG, no time source), so the measured
//! set is identical across runs.

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use purrdf_core::RdfDatasetBuilder;
use purrdf_core::ir::{GlobalDictionary, TermValue};

/// Number of intern calls per measured iteration.
const N: u32 = 10_000;

/// The fixed vocabulary size in the mixed group — a handful of predicates and
/// datatypes, the way a real document reuses them.
const VOCABULARY: u32 = 16;

/// N distinct IRIs: every intern of this set is a MISS.
fn distinct_iris() -> Vec<String> {
    (0..N)
        .map(|i| format!("http://example.org/resource/{i}"))
        .collect()
}

/// Group 1: worst case — N distinct IRIs, so the absoluteness parse runs N times.
fn bench_distinct_iris_all_misses(c: &mut Criterion) {
    let iris = distinct_iris();
    let mut group = c.benchmark_group("intern_absoluteness");
    group.bench_function("distinct_iris_all_misses", |b| {
        b.iter_batched(
            || iris.clone(),
            |iris| {
                let mut builder = RdfDatasetBuilder::new();
                for iri in &iris {
                    black_box(builder.intern_iri(black_box(iri)));
                }
                black_box(builder)
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

/// Group 2: the hit path — ONE IRI interned N times, so exactly one parse happens and
/// the remaining N-1 interns must not reach the check at all.
fn bench_repeated_iri_all_hits(c: &mut Criterion) {
    let iri = "http://example.org/resource/0".to_owned();
    let mut group = c.benchmark_group("intern_absoluteness");
    group.bench_function("repeated_iri_all_hits", |b| {
        b.iter_batched(
            || iri.clone(),
            |iri| {
                let mut builder = RdfDatasetBuilder::new();
                for _ in 0..N {
                    black_box(builder.intern_iri(black_box(&iri)));
                }
                black_box(builder)
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

/// Group 3: the realistic shape — a small reused vocabulary plus one fresh subject per
/// quad, which is what a parse of an ordinary document actually produces.
fn bench_realistic_mixed_vocabulary(c: &mut Criterion) {
    let subjects = distinct_iris();
    let predicates: Vec<String> = (0..VOCABULARY)
        .map(|i| format!("http://example.org/ns#p{i}"))
        .collect();
    let mut group = c.benchmark_group("intern_absoluteness");
    group.bench_function("realistic_mixed_vocabulary", |b| {
        b.iter_batched(
            || (subjects.clone(), predicates.clone()),
            |(subjects, predicates)| {
                let mut builder = RdfDatasetBuilder::new();
                for (i, subject) in subjects.iter().enumerate() {
                    let s = builder.intern_iri(black_box(subject));
                    let p = builder.intern_iri(black_box(&predicates[i % predicates.len()]));
                    builder.push_quad(s, p, s, None);
                }
                black_box(builder)
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

/// Group 4: worst case on the `GlobalTermId` table — the validating public
/// constructor, N distinct IRIs, all misses. This is the per-term parse cost that the
/// paged seal and compaction avoid by routing through the crate-internal
/// `reintern_validated` instead.
fn bench_global_dictionary_all_misses(c: &mut Criterion) {
    let values: Vec<TermValue> = distinct_iris().into_iter().map(TermValue::Iri).collect();
    let mut group = c.benchmark_group("intern_absoluteness");
    group.bench_function("global_dictionary_all_misses", |b| {
        b.iter_batched(
            || values.clone(),
            |values| {
                let mut dictionary = GlobalDictionary::new();
                for value in &values {
                    black_box(
                        dictionary
                            .intern(black_box(value))
                            .expect("bench fixtures are absolute"),
                    );
                }
                black_box(dictionary)
            },
            BatchSize::LargeInput,
        );
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_distinct_iris_all_misses,
    bench_repeated_iri_all_hits,
    bench_realistic_mixed_vocabulary,
    bench_global_dictionary_all_misses
);
criterion_main!(benches);
