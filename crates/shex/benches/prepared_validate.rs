// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Report-only benchmark for the ShEx per-engine shape precompilation path.
//!
//! Wall-clock samples from the shared development host are not acceptance
//! evidence. Correctness is guarded by the shexTest conformance corpus and the
//! crate's unit tests; this target exists for controlled-host allocation and
//! profile collection of `crate::validate::Engine::new`'s `prepared_shapes`
//! step (`PreparedShape`, held behind an `Arc` and compiled once per
//! [`validate`] call rather than once per focus node — invariant triple-
//! expression compilation plus the forward/inverse predicate-and-direction
//! maps).
//!
//! [`validate`] is the crate's public shape-map validation entry point
//! (`crates/shex/src/validate.rs`). A single call precompiles every reachable
//! shape once (`Engine::new`) and then checks the whole shape map against that
//! shared, `Arc`-cloned preparation — so driving MANY focus nodes through ONE
//! call is exactly the workload the allocation change targets: the
//! precompile cost is amortized over `NODE_COUNT` focus nodes instead of
//! repeated per node.

use std::sync::Arc;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
use purrdf_shex::{ShapeSelector, parse_shexc, validate};

const NODE_COUNT: usize = 3_000;
const NS: &str = "http://example.org/ns#";

/// A schema whose single shape has five `EachOf`-partitioned triple
/// constraints (a nontrivial `PreparedShape::forward`/`inverse` map), and
/// `NODE_COUNT` focus nodes that each satisfy it — so `Engine::new`'s
/// (private) one-time compile is amortized across every entry in the shape
/// map passed to a single [`validate`] call.
fn fixture() -> (
    purrdf_shex::Schema,
    Arc<RdfDataset>,
    Vec<(TermValue, ShapeSelector)>,
) {
    let schema_src = format!(
        "<{NS}Widget> {{ \
            <{NS}code> LITERAL ; \
            <{NS}label> LITERAL ; \
            <{NS}owner> IRI ; \
            <{NS}status> LITERAL ; \
            <{NS}priority> LITERAL \
        }}"
    );
    let schema = parse_shexc(&schema_src, None).expect("fixture schema parses");

    let mut builder = RdfDatasetBuilder::new();
    let p_code = builder.intern_iri(&format!("{NS}code"));
    let p_label = builder.intern_iri(&format!("{NS}label"));
    let p_owner = builder.intern_iri(&format!("{NS}owner"));
    let p_status = builder.intern_iri(&format!("{NS}status"));
    let p_priority = builder.intern_iri(&format!("{NS}priority"));
    let owner = builder.intern_iri(&format!("{NS}team-alpha"));

    let mut map = Vec::with_capacity(NODE_COUNT);
    for idx in 0..NODE_COUNT {
        let subject_iri = format!("{NS}widget-{idx}");
        let subject = builder.intern_iri(&subject_iri);
        let code = builder.intern_literal(purrdf_core::RdfLiteral::simple(format!("W-{idx}")));
        builder.push_quad(subject, p_code, code, None);
        let label =
            builder.intern_literal(purrdf_core::RdfLiteral::simple(format!("Widget {idx}")));
        builder.push_quad(subject, p_label, label, None);
        builder.push_quad(subject, p_owner, owner, None);
        let status = builder.intern_literal(purrdf_core::RdfLiteral::simple("active"));
        builder.push_quad(subject, p_status, status, None);
        let priority = builder.intern_literal(purrdf_core::RdfLiteral::simple("normal"));
        builder.push_quad(subject, p_priority, priority, None);

        map.push((
            TermValue::iri(subject_iri),
            ShapeSelector::Label(format!("{NS}Widget")),
        ));
    }
    let data = builder.freeze().expect("freeze fixture dataset");
    (schema, data, map)
}

fn bench_prepared_validate(c: &mut Criterion) {
    let (schema, data, map) = fixture();

    let mut group = c.benchmark_group("shex_prepared_validate");
    group.throughput(Throughput::Elements(
        u64::try_from(NODE_COUNT).expect("fixture size fits u64"),
    ));
    group.bench_function("single_engine_3000_focus_nodes", |b| {
        b.iter(|| {
            let result = validate(black_box(&schema), black_box(&data), black_box(&map));
            assert_eq!(result.entries.len(), NODE_COUNT);
            assert!(result.all_conformant());
            black_box(result);
        });
    });
    group.finish();
}

criterion_group!(benches, bench_prepared_validate);
criterion_main!(benches);
