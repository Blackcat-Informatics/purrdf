// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Micro-benchmark isolating the SHACL `quads_for_pattern` seam.
//!
//! Report-only, `cargo bench -p purrdf-shapes --bench pattern_lookup`. Where
//! `validate.rs` sweeps the whole corpus end-to-end (parse + focus resolution +
//! every constraint), this bench drives ONLY the id-native pattern-lookup and
//! path-traversal path — [`purrdf_shapes::path::eval`] over a synthetic frozen
//! dataset — so the seam that item 1 rewired (indexed `quads_for_pattern` →
//! `QuadIds`, `TermId` frontier dedup, no per-quad owned-`Term` materialization)
//! is measured in isolation from parsing and constraint evaluation.
//!
//! Two shapes stress the two halves of the seam:
//! - `predicate_fanout` — one subject with many objects: a single indexed scan
//!   whose matched `QuadIds` are mapped to value-node ids (the per-quad
//!   materialization cost).
//! - `closure_chain` — a long `ex:next` chain walked by `zeroOrMore`: many
//!   sequential pattern lookups whose frontier dedups on `Copy` `TermId`.

use std::sync::Arc;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use purrdf::{RdfDataset, RdfDatasetBuilder};
use purrdf_shapes::path::eval;
use purrdf_shapes::shapes::Path;
use purrdf_shapes::term::{NamedNode, Term};

const FANOUT: usize = 2_000;
const CHAIN: usize = 2_000;

fn iri(local: &str) -> String {
    format!("http://example.org/ns#{local}")
}

/// A frozen dataset with a `FANOUT`-wide `ex:hub ex:p ex:oN` star and a
/// `CHAIN`-long `ex:cK ex:next ex:c{K+1}` chain (with a diamond re-entry so the
/// closure walk exercises the visited-set dedup).
fn fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&iri("p"));
    let next = b.intern_iri(&iri("next"));
    let hub = b.intern_iri(&iri("hub"));

    for idx in 0..FANOUT {
        let o = b.intern_iri(&iri(&format!("o{idx}")));
        b.push_quad(hub, p, o, None);
    }

    let mut prev = b.intern_iri(&iri("c0"));
    for idx in 1..CHAIN {
        let node = b.intern_iri(&iri(&format!("c{idx}")));
        b.push_quad(prev, next, node, None);
        prev = node;
    }
    // Diamond re-entry: c1 also points back near the head, so the closure walk
    // hits an already-visited id and must dedup rather than loop.
    let c1 = b.intern_iri(&iri("c1"));
    let c0 = b.intern_iri(&iri("c0"));
    b.push_quad(c1, next, c0, None);

    b.freeze().expect("freeze fixture")
}

fn bench_pattern_lookup(c: &mut Criterion) {
    let ds = fixture();
    let ds = ds.as_ref();

    let mut group = c.benchmark_group("shacl_pattern_lookup");

    // One indexed scan → map matched QuadIds to value-node ids.
    let hub = Term::NamedNode(NamedNode::new_unchecked(iri("hub")));
    let predicate = Path::Predicate(NamedNode::new_unchecked(iri("p")));
    group.bench_function("predicate_fanout", |bencher| {
        bencher.iter(|| {
            let values = eval(black_box(ds), black_box(&hub), black_box(&predicate));
            black_box(values);
        });
    });

    // Many sequential lookups with TermId frontier dedup.
    let head = Term::NamedNode(NamedNode::new_unchecked(iri("c0")));
    let closure = Path::ZeroOrMore(Box::new(Path::Predicate(NamedNode::new_unchecked(iri(
        "next",
    )))));
    group.bench_function("closure_chain", |bencher| {
        bencher.iter(|| {
            let values = eval(black_box(ds), black_box(&head), black_box(&closure));
            black_box(values);
        });
    });

    group.finish();
}

criterion_group!(benches, bench_pattern_lookup);
criterion_main!(benches);
