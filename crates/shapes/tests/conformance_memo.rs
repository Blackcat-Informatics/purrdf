// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **A shape named at two constraint sites is asked about a shared value node
//! once, not twice.**
//!
//! `sh:node`, `sh:and`, `sh:or`, `sh:xone` and `sh:qualifiedValueShape` all ask
//! the same question — "does this node conform to that shape?" — and the answer
//! is a pure function of the data graph, the node and the shape. When two
//! constraint sites name the SAME shape and their value sets overlap, the second
//! ask re-runs a traversal whose answer is already known.
//!
//! The evaluator memoizes those answers for the duration of one focus node's
//! validation. This file is the executable proof that the memo exists and fires,
//! because a cache is the easiest kind of code to ship broken: one that never
//! hits is indistinguishable from one that was never wired up, and every other
//! test in this crate passes either way.
//!
//! # How a memo hit is made visible
//!
//! Two shapes graphs of identical STRUCTURE and identical meaning, differing
//! only in whether the inner shape is reachable by name:
//!
//! * [`NAMED`] states the inner shape once as `ex:Inner` and refers to it from
//!   both property shapes. Both sites name the same shape, so the second ask is
//!   answered from the memo.
//! * [`INLINE`] writes the same constraints out twice as anonymous shapes. Two
//!   anonymous shapes are two DIFFERENT shapes — each belongs to exactly one
//!   site and can share an answer with nothing — so both asks run.
//!
//! Both graphs validate the same data, both must produce the same report, and
//! the named one must allocate strictly less. The inner shape carries
//! `sh:equals`, which is one of the few Core constraints whose CONFORMING
//! evaluation allocates (it builds the compared predicate's object set), so a
//! skipped traversal is visible in an allocation count rather than only in a
//! timing.
//!
//! Every IRI is under `example.org`: PurRDF mints no vocabulary IRIs.

use purrdf_alloc_probe::{CountingAllocator, WholeProcessWindow};
use purrdf_shapes::engine::validate_graphs;

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// The data graph: one focus node whose two paths lead to the SAME value node,
/// which is what makes the two constraint sites ask about one node.
const DATA: &str = concat!(
    "<http://example.org/purrdf/memo#focus> \
     <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> \
     <http://example.org/purrdf/memo#Focus> .\n",
    "<http://example.org/purrdf/memo#focus> <http://example.org/purrdf/memo#left> \
     <http://example.org/purrdf/memo#shared> .\n",
    "<http://example.org/purrdf/memo#focus> <http://example.org/purrdf/memo#right> \
     <http://example.org/purrdf/memo#shared> .\n",
    "<http://example.org/purrdf/memo#shared> <http://example.org/purrdf/memo#count> \
     \"7\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
    "<http://example.org/purrdf/memo#shared> <http://example.org/purrdf/memo#limit> \
     \"7\"^^<http://www.w3.org/2001/XMLSchema#integer> .\n",
);

/// The inner shape reached by NAME from both sites: one shape, one answer.
const NAMED: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/purrdf/memo#> .\n",
    "ex:Inner a sh:NodeShape ;
        sh:property [ sh:path ex:count ; sh:equals ex:limit ] .
     ex:Outer a sh:NodeShape ; sh:targetClass ex:Focus ;
        sh:property [ sh:path ex:left ; sh:node ex:Inner ] ;
        sh:property [ sh:path ex:right ; sh:node ex:Inner ] .",
);

/// The same constraints, written out twice as ANONYMOUS shapes: two shapes, two
/// answers, and nothing for a memo to share.
const INLINE: &str = concat!(
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n",
    "@prefix ex: <http://example.org/purrdf/memo#> .\n",
    "ex:Outer a sh:NodeShape ; sh:targetClass ex:Focus ;
        sh:property [ sh:path ex:left ; sh:node [
            a sh:NodeShape ;
            sh:property [ sh:path ex:count ; sh:equals ex:limit ]
        ] ] ;
        sh:property [ sh:path ex:right ; sh:node [
            a sh:NodeShape ;
            sh:property [ sh:path ex:count ; sh:equals ex:limit ]
        ] ] .",
);

/// Validate `shapes` against [`DATA`], requiring conformance, and return how many
/// allocations the validation made.
///
/// The measurement is taken after a warm-up run with the same arguments: the
/// regex cache, the class-membership index and the allocator's own arenas are
/// first-touch lazies, and charging them to whichever call ran first would make
/// the comparison a statement about start-up.
fn allocations_to_validate(shapes: &str, label: &str) -> u64 {
    let run = || {
        let report = validate_graphs(DATA, shapes, None)
            .unwrap_or_else(|error| panic!("{label}: the fixture must validate: {error}"));
        assert!(
            report.conforms && report.results.is_empty(),
            "{label}: the fixture must CONFORM, or the figure below describes a workload that \
             never reached the inner shape ({} result(s))",
            report.results.len()
        );
    };
    run();
    let window = WholeProcessWindow::open();
    run();
    window.close().allocations
}

/// **The two spellings mean the same thing, and the memoizable one costs less.**
///
/// The equality half is what makes the inequality half safe to assert: a memo
/// that returned a wrong answer would make the two reports disagree, and a memo
/// that never fired would make the two costs equal.
#[test]
fn a_named_inner_shape_is_evaluated_once_for_a_shared_value_node() {
    let named_report = validate_graphs(DATA, NAMED, None).expect("the named fixture validates");
    let inline_report = validate_graphs(DATA, INLINE, None).expect("the inline fixture validates");
    assert!(
        named_report.conforms && inline_report.conforms,
        "both spellings describe a conforming graph"
    );
    assert_eq!(
        named_report.results.len(),
        inline_report.results.len(),
        "the two spellings state the same constraints and must reach the same verdict"
    );

    let named = allocations_to_validate(NAMED, "named");
    let inline = allocations_to_validate(INLINE, "inline");
    assert!(
        named < inline,
        "naming the inner shape at both sites must let the second site reuse the first site's \
         conformance answer, so it must allocate strictly less than spelling the shape out twice \
         ({named} vs {inline}). Equal figures mean the conformance memo is not firing."
    );
}
