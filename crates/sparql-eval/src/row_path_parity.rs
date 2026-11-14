// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The unbounded row path and the accounted row path answer alike, and a bounded run
//! keeps its accounting.
//!
//! A workspace that admits nothing builds its rows, projections and result cells
//! directly; a workspace with an operational account builds them through admitted
//! constructors. These tests hold both facts: the direct constructions equal the
//! admitted ones on every shape they take, and the account a bounded run needs is the
//! one it needed before the direct constructions existed.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_sparql_algebra::SparqlParser;

use crate::eval::{EvalCtx, EvaluatedOutcome, Outcome, evaluate_query_evaluated};
use crate::workspace::owned_row_regressions::{Refusal, account};

const EX: &str = "http://example.org/";
const INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

type Rows = (Vec<String>, Vec<Vec<Option<TermValue>>>);

/// Six subjects with an integer, an IRI and three plain literals each, so a query can
/// read two to six columns. With `statements`, one reifier and one annotation as well.
fn dataset(statements: bool) -> Arc<RdfDataset> {
    dataset_with(statements, statements)
}

/// The same dataset holding a reifier binding, an annotation, both or neither.
fn dataset_with(reifier: bool, annotation: bool) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let preds: Vec<_> = ["v", "w", "x", "y", "z"]
        .iter()
        .map(|name| b.intern_iri(&format!("{EX}{name}")).unwrap())
        .collect();
    for i in 0..6 {
        let s = b.intern_iri(&format!("{EX}s{i}")).unwrap();
        let number = b
            .intern_literal(RdfLiteral::typed(
                format!("{}", 3 + 2 * i),
                INTEGER.to_owned(),
            ))
            .unwrap();
        b.push_quad(s, preds[0], number, None).unwrap();
        let object = b.intern_iri(&format!("{EX}o{}", i % 3)).unwrap();
        b.push_quad(s, preds[1], object, None).unwrap();
        for (k, predicate) in preds[2..].iter().enumerate() {
            let text = b
                .intern_literal(RdfLiteral::simple(format!("t{i}-{k}")))
                .unwrap();
            b.push_quad(s, *predicate, text, None).unwrap();
        }
    }
    let confidence = b.intern_iri(&format!("{EX}confidence")).unwrap();
    if reifier || annotation {
        let s0 = b.intern_iri(&format!("{EX}s0")).unwrap();
        let v = b.intern_iri(&format!("{EX}v")).unwrap();
        let three = b
            .intern_literal(RdfLiteral::typed("3", INTEGER.to_owned()))
            .unwrap();
        let triple = b.intern_triple(s0, v, three).unwrap();
        let reifier_row = reifier;
        let reifier = b.intern_iri(&format!("{EX}r")).unwrap();
        b.intern_iri(RDF_REIFIES).unwrap();
        if reifier_row {
            b.push_reifier(reifier, triple).unwrap();
        }
        let high = b.intern_literal(RdfLiteral::simple("high")).unwrap();
        if annotation {
            b.push_annotation(reifier, confidence, high).unwrap();
        }
    }
    b.freeze().unwrap()
}

fn query(text: &str) -> purrdf_sparql_algebra::Query {
    SparqlParser::new()
        .parse_query(&format!("PREFIX ex: <{EX}>\n{text}"))
        .unwrap()
}

/// Evaluate and materialize on `ctx`, as the engine does for a SELECT.
fn run(text: &str, ctx: &mut EvalCtx<'_, RdfDataset>) -> Option<Rows> {
    let outcome = evaluate_query_evaluated(&query(text), ctx).ok()?;
    let EvaluatedOutcome::Complete(Outcome::Solutions(seq)) = outcome else {
        return None;
    };
    crate::eval::materialize_solutions(&seq, ctx).ok()
}

/// The answer on the unbounded workspace.
fn unbounded(text: &str, dataset: &Arc<RdfDataset>) -> Rows {
    let mut ctx = EvalCtx::new(dataset.as_ref());
    ctx.options.force_sequential = true;
    assert!(!ctx.growth.is_bounded());
    run(text, &mut ctx).expect("the unbounded run completes")
}

/// The answer under an account of `limit` bytes, if it completes.
fn bounded(text: &str, dataset: &Arc<RdfDataset>, limit: u64) -> Option<Rows> {
    let (workspace, _ledger, _) = account(limit);
    let mut ctx = EvalCtx::new(dataset.as_ref());
    ctx.options.force_sequential = true;
    ctx.growth = workspace.capability();
    assert!(ctx.growth.is_bounded());
    run(text, &mut ctx)
}

/// The smallest account, in 8 byte steps, under which `text` completes.
fn smallest_completing_limit(text: &str, dataset: &Arc<RdfDataset>) -> u64 {
    let base = crate::workspace::QueryWorkspace::<Refusal>::control_bytes().unwrap();
    let (mut low, mut high) = (base / 8, (base + 4 * 1024 * 1024) / 8);
    assert!(bounded(text, dataset, high * 8).is_some());
    while low < high {
        let mid = low + (high - low) / 2;
        if bounded(text, dataset, mid * 8).is_some() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    low * 8
}

/// Queries covering the constructions that gained a direct path: a join with a FILTER
/// (bound rows, kept-row clones, projection), a repeated variable, a row wider than the
/// inline capacity, and a nested triple pattern.
const SHAPES: [&str; 5] = [
    "SELECT ?s ?v WHERE { ?s ex:v ?v ; ex:w ?o . FILTER(?v > 5) }",
    "SELECT ?s WHERE { ?s ex:v ?v ; ex:w ?o . ?s ex:x ?t . ?s ex:x ?t }",
    "SELECT * WHERE { ?s ex:v ?v ; ex:w ?o ; ex:x ?x ; ex:y ?y ; ex:z ?z }",
    "SELECT ?s ?z WHERE { ?s ex:v ?v ; ex:z ?z . ?s ex:w ?s }",
    "SELECT ?r ?c WHERE { ?r ex:confidence ?c . ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> << ?s ex:v ?v >> }",
];

/// What each shape needed on the accounted path before the direct paths existed, in the
/// order of [`SHAPES`]: the smallest account that completes, and the largest one below it
/// that does not (one step of 8 bytes under).
const PINNED_SMALLEST_LIMIT: [u64; 5] = [11544, 8048, 12144, 7240, 6064];

#[test]
fn the_accounted_row_path_needs_the_account_it_always_needed() {
    let data = dataset(true);
    let measured: Vec<u64> = SHAPES
        .iter()
        .map(|shape| smallest_completing_limit(shape, &data))
        .collect();
    assert_eq!(measured, PINNED_SMALLEST_LIMIT);
    for (shape, smallest) in SHAPES.iter().zip(measured) {
        assert!(
            bounded(shape, &data, smallest - 8).is_none(),
            "one step below the smallest account refuses: {shape}"
        );
    }
}

#[test]
fn the_direct_row_path_answers_as_the_accounted_row_path_does() {
    for shape in SHAPES {
        let data = dataset(true);
        let direct = unbounded(shape, &data);
        let limit = smallest_completing_limit(shape, &data);
        let accounted = bounded(shape, &data, limit).expect("completes at its smallest account");
        assert_eq!(direct, accounted, "{shape}");
        if shape.starts_with("SELECT *") || shape.contains("FILTER") {
            assert!(!direct.1.is_empty(), "{shape} has rows");
        }
    }
}

/// The answer on the unbounded workspace under a stop signal that never fires and a
/// cell ceiling nothing reaches: every stage, the last one included, scans into the
/// row sink instead of the retained cursor.
fn unbounded_into_the_sink(text: &str, dataset: &Arc<RdfDataset>) -> Rows {
    let governors = crate::QueryGovernors::UNBOUNDED
        .with_max_intermediate_cells(1 << 40)
        .with_stop_signal(Arc::new(crate::CancellationFlag::new()));
    let mut ctx = EvalCtx::new(dataset.as_ref())
        .with_governors(Arc::new(crate::governor::GovernorState::new(&governors)));
    ctx.options.force_sequential = true;
    assert!(!ctx.growth.is_bounded());
    assert!(ctx.stop_signal().is_some());
    run(text, &mut ctx).expect("the governed unbounded run completes")
}

/// INVARIANT: a candidate binds to the same row whichever scan loop offers it. The
/// sink loop and the retained cursor share one binder, and its direct form for a
/// pattern with no nested position equals the general one: a constant, a slot the
/// driving row binds, a variable repeated inside one pattern (`?s ex:w ?s`, which
/// must reject every candidate here) and a nested triple pattern all answer alike.
#[test]
fn the_sink_scan_binds_as_the_cursor_scan_does() {
    let repeated = "SELECT ?s ?o WHERE { ?s ex:w ?o . ?o ex:w ?o }";
    let data = dataset(true);
    for shape in SHAPES.into_iter().chain([repeated]) {
        let cursor = unbounded(shape, &data);
        let sink = unbounded_into_the_sink(shape, &data);
        assert_eq!(cursor, sink, "{shape}");
        let limit = smallest_completing_limit(shape, &data);
        assert_eq!(bounded(shape, &data, limit), Some(sink), "{shape}");
    }
    // The neighbour that binds: a variable repeated inside one pattern is not refused
    // when the candidate does agree with itself.
    let mut b = RdfDatasetBuilder::new();
    let knows = b.intern_iri(&format!("{EX}w")).unwrap();
    let narcissus = b.intern_iri(&format!("{EX}n")).unwrap();
    let echo = b.intern_iri(&format!("{EX}e")).unwrap();
    b.push_quad(narcissus, knows, narcissus, None).unwrap();
    b.push_quad(echo, knows, narcissus, None).unwrap();
    let data = b.freeze().unwrap();
    let shape = "SELECT ?s WHERE { ?s ex:w ?s }";
    let rows = unbounded_into_the_sink(shape, &data);
    assert_eq!(rows.1, vec![vec![Some(TermValue::Iri(format!("{EX}n")))]]);
    assert_eq!(unbounded(shape, &data), rows);
}

#[test]
fn the_statement_layer_is_skipped_only_when_the_view_certifies_it_empty() {
    use crate::bgp::VIRTUAL_SOURCES;
    let shape = "SELECT ?r ?c WHERE { ?r ex:confidence ?c }";
    let without = dataset(false);
    let with = dataset(true);
    assert!(purrdf_core::DatasetView::statement_layer_is_empty(
        without.as_ref()
    ));
    assert!(!purrdf_core::DatasetView::statement_layer_is_empty(
        with.as_ref()
    ));

    // The neighbour with no reifier and no annotation: the same rows as the accounted
    // run, and the virtual source was never opened.
    VIRTUAL_SOURCES.with(|opened| opened.set(0));
    let rows = unbounded(shape, &without);
    assert_eq!(rows.1.len(), 0);
    assert_eq!(VIRTUAL_SOURCES.with(std::cell::Cell::get), 0);
    let limit = smallest_completing_limit(shape, &without);
    VIRTUAL_SOURCES.with(|opened| opened.set(0));
    assert_eq!(bounded(shape, &without, limit), Some(rows));
    assert!(
        VIRTUAL_SOURCES.with(std::cell::Cell::get) > 0,
        "a bounded run still opens the source it is charged for"
    );

    // The dataset that holds the statements: the virtual rows are still returned.
    VIRTUAL_SOURCES.with(|opened| opened.set(0));
    let rows = unbounded(shape, &with);
    assert_eq!(rows.1.len(), 1, "the annotation is answered");
    assert!(VIRTUAL_SOURCES.with(std::cell::Cell::get) > 0);
}

/// Float and double literals of every ordering class a direct comparison must keep:
/// negative, zero of both signs, equal across the two types, infinities and NaN.
fn float_dataset() -> Arc<RdfDataset> {
    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
    let mut b = RdfDatasetBuilder::new();
    let f = b.intern_iri(&format!("{EX}f")).unwrap();
    let values = [
        ("-1.5", "float"),
        ("-0", "float"),
        ("0", "double"),
        ("1.5", "float"),
        ("1.5", "double"),
        ("3", "float"),
        ("INF", "float"),
        ("-INF", "double"),
        ("NaN", "float"),
        ("0.1", "float"),
        ("0.1", "double"),
    ];
    for (i, (lexical, kind)) in values.iter().enumerate() {
        let s = b.intern_iri(&format!("{EX}f{i}")).unwrap();
        let o = b
            .intern_literal(RdfLiteral::typed(*lexical, format!("{XSD}{kind}")))
            .unwrap();
        b.push_quad(s, f, o, None).unwrap();
    }
    b.freeze().unwrap()
}

const FLOAT_SHAPES: [&str; 4] = [
    "SELECT ?s WHERE { ?s ex:f ?a . ?t ex:f ?b . FILTER(?a < ?b) }",
    "SELECT ?s WHERE { ?s ex:f ?a . ?t ex:f ?b . FILTER(?a >= ?b) }",
    "SELECT ?s WHERE { ?s ex:f ?a . FILTER(?a > \"1.5\"^^<http://www.w3.org/2001/XMLSchema#double>) }",
    "SELECT ?s WHERE { ?s ex:f ?a . FILTER(?a <= \"0\"^^<http://www.w3.org/2001/XMLSchema#float>) }",
];

/// The smallest completing account of each float shape, in order.
const FLOAT_PINNED_LIMIT: [u64; 4] = [37808, 37808, 13584, 13880];

#[test]
fn float_comparison_is_the_same_directly_and_under_an_account() {
    let data = float_dataset();
    let mut pinned = Vec::new();
    for shape in FLOAT_SHAPES {
        let direct = unbounded(shape, &data);
        let limit = smallest_completing_limit(shape, &data);
        let accounted = bounded(shape, &data, limit).expect("completes at its smallest account");
        assert_eq!(direct, accounted, "{shape}");
        assert!(
            bounded(shape, &data, limit - 8).is_none(),
            "one step below refuses: {shape}"
        );
        pinned.push(limit);
    }
    assert_eq!(pinned, FLOAT_PINNED_LIMIT);
}

#[test]
fn either_statement_table_alone_makes_the_layer_non_empty() {
    use purrdf_core::DatasetView;
    for (reifier, annotation) in [(false, false), (true, false), (false, true), (true, true)] {
        let data = dataset_with(reifier, annotation);
        assert_eq!(
            data.statement_layer_is_empty(),
            !(reifier || annotation),
            "reifier {reifier}, annotation {annotation}"
        );
    }
}
