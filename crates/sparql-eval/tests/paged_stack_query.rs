// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Real evaluator publication over independently sealed chronological generations.

#[path = "../examples/support/paged_stack.rs"]
mod fixture;
mod support;

use fixture::{carrier_pages, eager, seal};
use purrdf_core::term_fixture::iri;
use purrdf_core::{
    BlankScope, DatasetView, FallibleDatasetView, GlobalTermId, PackBuilder, PageFault,
    PageGeneration, PageId, PageMaterialization, PageProvider, PagedDataset, PagedQueryError,
    PagedQueryLimits, PagedStack, PagedStackEvidence, PagedStackSnapshot, QuadValues, RdfDataset,
    RdfDatasetBuilder, RdfTextDirection, SparqlRequest, StackPageOrigin, StopCause, TermBox,
    TermFactory, TermValue, ViewOperationStatus, canonical_paged_seal,
};
use purrdf_sparql_eval::{FallibleSparqlError, NativeSparqlEngine, QueryOptions};
use std::collections::BTreeSet;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use support::solutions;

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}
fn bound(n: usize) -> NonZeroUsize {
    NonZeroUsize::new(n).expect("positive page bound")
}
fn edge(s: &str, p: &str, o: TermValue) -> QuadValues {
    QuadValues::triple(iri(s), iri(p), o)
}
fn integer(n: &str) -> TermValue {
    TermValue::typed_literal(n, purrdf_xsd::datatype::XSD_INTEGER)
}

const QUERIES: &[&str] = &[
    "SELECT ?s ?o ?n ?a WHERE { ?s ex:knows ?o . ?o ex:name ?n . ?o ex:age ?a FILTER(?a >= 18) } ORDER BY ?s ?o",
    "SELECT ?s ?o ?n WHERE { ?s ex:knows ?o OPTIONAL { ?o ex:name ?n } } ORDER BY ?s ?o",
    "SELECT ?s ?o WHERE { { ?s ex:knows ?o } UNION { ?s ex:knows ?o } } ORDER BY ?s ?o",
    "SELECT DISTINCT ?s ?o WHERE { { ?s ex:knows ?o } UNION { ?s ex:knows ?o } } ORDER BY ?s ?o",
    "SELECT ?s ?o WHERE { ?s ex:knows ?o MINUS { ?o ex:name ?n } } ORDER BY ?s ?o",
    "SELECT ?s ?o WHERE { ?s ex:knows ?o FILTER NOT EXISTS { ?o ex:name ?n } } ORDER BY ?s ?o",
    "SELECT ?s (COUNT(?o) AS ?count) WHERE { ?s ex:knows ?o } GROUP BY ?s ORDER BY ?s",
    "SELECT ?s ?count WHERE { { SELECT ?s (COUNT(?o) AS ?count) WHERE { ?s ex:knows ?o } GROUP BY ?s } FILTER(?count > 0) } ORDER BY ?s",
    "SELECT ?o WHERE { ex:alice ex:knows+ ?o } ORDER BY ?o",
    "SELECT ?g ?s ?p ?o WHERE { GRAPH ?g { ?s ?p ?o } } ORDER BY ?g ?s ?p ?o",
    "SELECT ?g WHERE { GRAPH ?g {} } ORDER BY ?g",
    "ASK { GRAPH ex:empty {} }",
    "ASK { GRAPH ex:implicit {} }",
    "ASK { ex:alice ex:knows ex:bob }",
    "CONSTRUCT { ?s ex:linkedName ?n } WHERE { ?s ex:knows ?o . ?o ex:name ?n }",
    "SELECT ?s ?o WHERE { ?s ex:knows ?o } ORDER BY ?s ?o LIMIT 2 OFFSET 1",
    "SELECT ?s ?o WHERE { GRAPH ex:delta { ?s ex:name ?o } } ORDER BY ?s ?o",
    "ASK { GRAPH ex:delta {} }",
    "ASK { GRAPH ex:deltaEmpty {} }",
];

fn assert_same_result(
    actual: impl support::ResultView,
    expected: impl support::ResultView,
    query: &str,
) {
    match (
        actual.solutions_view(),
        expected.solutions_view(),
        actual.boolean_view(),
        expected.boolean_view(),
        actual.graph_view(),
        expected.graph_view(),
    ) {
        (Some((av, ar)), Some((ev, er)), ..) => {
            assert_eq!(av, ev, "variables: {query}");
            assert_eq!(ar, er, "ordered solution bags: {query}");
        }
        (_, _, Some(a), Some(e), ..) => assert_eq!(a, e, "ASK: {query}"),
        (_, _, _, _, Some(a), Some(e)) => assert_eq!(
            PackBuilder::build_bytes(a).unwrap(),
            PackBuilder::build_bytes(e).unwrap(),
            "CONSTRUCT carrier: {query}"
        ),
        _ => panic!("result shapes differ for {query}: {actual:?} != {expected:?}"),
    }
    drop(actual);
    drop(expected);
}

fn matrix(snapshot: &PagedStackSnapshot, oracle: &RdfDataset, queries: &[&str]) {
    let engine = NativeSparqlEngine::new();
    for query in queries {
        let text = format!("PREFIX ex: <http://example.org/> {query}");
        let prepared = engine
            .prepare_query(&text, None)
            .expect("prepare consumer query");
        for prepared_entry in [false, true] {
            let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
            let complete = if prepared_entry {
                engine.query_prepared_fallible_view(&view, &prepared, &[], QueryOptions::EMPTY)
            } else {
                engine.query_fallible_view(&view, request(&text), QueryOptions::EMPTY)
            }
            .unwrap_or_else(|e| panic!("layered consumer failed: {e:?}: {text}"));
            let expected = engine
                .query_prepared_view(oracle, &prepared, &[], QueryOptions::EMPTY)
                .expect("independent eager input query");
            assert_same_result(complete.result, expected, &text);
            assert!(matches!(
                view.operation_status(),
                ViewOperationStatus::Ready { .. }
            ));
        }
    }
}

#[test]
fn chronological_layers_match_eager_bags_for_every_consumer_family() {
    let repeated = edge("alice", "knows", iri("bob"));
    let missing = edge("alice", "knows", iri("unnamed"));
    let first = vec![
        repeated.clone(),
        missing,
        edge("bob", "knows", iri("carol")),
    ];
    let implicit = QuadValues {
        g: Some(iri("implicit")),
        ..edge("named", "name", TermValue::simple_literal("Named"))
    };
    let second = vec![
        edge("bob", "name", TermValue::simple_literal("Bob")),
        repeated.clone(),
        edge("bob", "age", integer("25")),
        edge("carol", "name", TermValue::simple_literal("Carol")),
        edge("carol", "age", integer("17")),
        implicit.clone(),
    ];
    let base_a = seal(eager(first.clone(), &[]));
    let base_b = seal(eager(second.clone(), &[iri("empty")]));
    assert_ne!(
        base_a.dictionary().term_value(GlobalTermId::from_index(0)),
        base_b.dictionary().term_value(GlobalTermId::from_index(0)),
        "colliding local/global ordinal zero names different values"
    );
    let mut stack = PagedStack::new(vec![base_a, base_b]).unwrap();
    let mut values: BTreeSet<_> = first.into_iter().chain(second).collect();
    let original = stack.snapshot().unwrap();
    let original_oracle = eager(values.clone(), &[iri("empty")]);
    matrix(&original, &original_oracle, QUERIES);
    for old in [
        edge("bob", "name", TermValue::simple_literal("Bob")),
        edge("carol", "age", integer("17")),
    ] {
        assert_eq!(stack.remove(&old).unwrap(), values.remove(&old));
    }
    for new in [
        edge("bob", "name", TermValue::simple_literal("Robert")),
        edge("carol", "age", integer("18")),
        edge("alice", "knows", iri("dave")),
        edge("dave", "name", TermValue::simple_literal("Dave")),
        edge("dave", "age", integer("30")),
    ] {
        assert_eq!(stack.insert(new.clone()).unwrap(), values.insert(new));
    }
    stack.seal_head(bound(2)).unwrap();
    assert_eq!(stack.remove(&repeated).unwrap(), values.remove(&repeated));
    assert_eq!(stack.remove(&implicit).unwrap(), values.remove(&implicit));
    stack.seal_head(bound(1)).unwrap();
    let removed = stack.snapshot().unwrap();
    let removed_oracle = eager(values.clone(), &[iri("empty")]);
    matrix(&removed, &removed_oracle, QUERIES);
    assert_eq!(NativeSparqlEngine::new().query_fallible_view(&removed.query_view(PagedQueryLimits::UNBOUNDED), request("ASK { <http://example.org/alice> <http://example.org/knows> <http://example.org/bob> }"), QueryOptions::EMPTY).unwrap().result.boolean(), Some(false));
    assert_eq!(
        stack.insert(repeated.clone()).unwrap(),
        values.insert(repeated)
    );
    let delta_graph_row = QuadValues {
        g: Some(iri("delta")),
        ..edge("writer", "name", TermValue::simple_literal("Delta graph"))
    };
    assert_eq!(
        stack.insert(delta_graph_row.clone()).unwrap(),
        values.insert(delta_graph_row.clone())
    );
    assert!(stack.declare_named_graph(iri("deltaEmpty")).unwrap());
    let head = stack.snapshot().unwrap();
    let oracle = eager(values.clone(), &[iri("empty"), iri("deltaEmpty")]);
    matrix(&head, &oracle, QUERIES);
    let (_, witnesses) = solutions(
        NativeSparqlEngine::new()
            .query_fallible_view(
                &head.query_view(PagedQueryLimits::UNBOUNDED),
                request("SELECT ?s ?o WHERE { ?s <http://example.org/knows> ?o }"),
                QueryOptions::EMPTY,
            )
            .unwrap()
            .result,
    );
    assert_eq!(
        witnesses.len(),
        4,
        "nonempty set: repeated physical rows count once"
    );
    stack.seal_head(bound(3)).unwrap();
    matrix(&stack.snapshot().unwrap(), &oracle, QUERIES);
    matrix(&original, &original_oracle, QUERIES);
    matrix(&removed, &removed_oracle, QUERIES);
    for n in [1, 4] {
        let expected = canonical_paged_seal(oracle.as_ref(), bound(n)).unwrap();
        for retained in [&head, &stack.snapshot().unwrap()] {
            let folded = retained
                .compact(PagedQueryLimits::UNBOUNDED, bound(n))
                .unwrap();
            assert_eq!(carrier_pages(&folded), carrier_pages(&expected));
        }
    }
    assert_eq!(
        stack.remove(&delta_graph_row).unwrap(),
        values.remove(&delta_graph_row)
    );
    stack.seal_head(bound(1)).unwrap();
    let withdrawn = stack.snapshot().unwrap();
    let withdrawn_oracle = eager(values, &[iri("empty"), iri("deltaEmpty")]);
    matrix(&withdrawn, &withdrawn_oracle, QUERIES);
    assert_eq!(
        carrier_pages(
            &withdrawn
                .compact(PagedQueryLimits::UNBOUNDED, bound(2))
                .unwrap()
        ),
        carrier_pages(&canonical_paged_seal(withdrawn_oracle.as_ref(), bound(2)).unwrap())
    );
    matrix(&head, &oracle, QUERIES);
}

#[test]
fn generated_mutations_and_retained_snapshots_keep_query_bags_and_page_identity() {
    let initial = edge("anchor", "knows", iri("anchor"));
    let mut values = BTreeSet::from([initial.clone()]);
    let mut stack = PagedStack::new(vec![seal(eager([initial], &[]))]).unwrap();
    let mut snapshots = Vec::new();
    let queries = [
        QUERIES[1],
        QUERIES[2],
        QUERIES[4],
        QUERIES[6],
        "SELECT ?o WHERE { ex:anchor ex:knows+ ?o } ORDER BY ?o",
    ];
    for step in 0..32 {
        let row = edge(
            &format!("s{}", (step * 13 + 3) % 7),
            "knows",
            iri(&format!("o{}", (step * 11) % 4)),
        );
        if step % 3 == 0 {
            assert_eq!(stack.remove(&row).unwrap(), values.remove(&row));
        } else {
            assert_eq!(stack.insert(row.clone()).unwrap(), values.insert(row));
        }
        let snapshot = stack.snapshot().unwrap();
        let oracle = eager(values.clone(), &[]);
        matrix(&snapshot, &oracle, &queries);
        let (_, path_rows) = solutions(NativeSparqlEngine::new().query_fallible_view(&snapshot.query_view(PagedQueryLimits::UNBOUNDED), request("SELECT ?o WHERE { <http://example.org/anchor> <http://example.org/knows>+ ?o }"), QueryOptions::EMPTY).unwrap().result);
        assert_eq!(
            path_rows,
            vec![vec![Some(iri("anchor"))]],
            "nonempty cycle closure survives generated edits"
        );
        if step % 7 == 0 {
            snapshots.push((snapshot, oracle));
            stack.seal_head(bound(1 + step % 3)).unwrap();
        }
    }
    for (snapshot, oracle) in snapshots {
        assert!(oracle.quad_count() > 0, "anchor survives every history");
        matrix(&snapshot, &oracle, &queries);
        for n in [1, 5] {
            assert_eq!(
                carrier_pages(
                    &snapshot
                        .compact(PagedQueryLimits::UNBOUNDED, bound(n))
                        .unwrap()
                ),
                carrier_pages(&canonical_paged_seal(oracle.as_ref(), bound(n)).unwrap())
            );
        }
    }
    let only = edge("deleted", "knows", iri("gone"));
    let mut empty = PagedStack::new(vec![seal(eager([only.clone()], &[]))]).unwrap();
    empty.remove(&only).unwrap();
    empty.seal_head(bound(1)).unwrap();
    matrix(
        &empty.snapshot().unwrap(),
        &purrdf_core::term_fixture::empty_dataset(),
        &[QUERIES[2], QUERIES[6], QUERIES[13]],
    );
    assert_eq!(
        carrier_pages(
            &empty
                .compact(PagedQueryLimits::UNBOUNDED, bound(2))
                .unwrap()
        ),
        [] as [Vec<u8>; 0]
    );
}

#[test]
fn typed_rdf_values_and_graph_scoped_retractions_match_explicit_eager_tables() {
    let blank = TermValue::Blank {
        label: "same".into(),
        scope: BlankScope(51),
    };
    let nested = TermValue::Triple {
        s: TermBox::new(blank.clone()),
        p: TermBox::new(iri("p")),
        o: TermBox::new(purrdf_core::term_fixture::triple_chain(2)),
    };
    let direction = TermValue::Literal {
        lexical_form: "مرحبا".into(),
        datatype: purrdf_core::vocab::rdf::DIR_LANG_STRING.into(),
        language: Some("ar".into()),
        direction: Some(RdfTextDirection::Rtl),
    };
    let declaration = QuadValues {
        s: iri("r"),
        p: TermValue::iri(purrdf_core::vocab::rdf::REIFIES),
        o: nested.clone(),
        g: Some(iri("g")),
    };
    let note = QuadValues {
        s: iri("r"),
        p: iri("note"),
        o: direction.clone(),
        g: Some(iri("g")),
    };
    let ordinary = [
        QuadValues::triple(blank, iri("nested"), nested),
        QuadValues::triple(
            TermValue::Blank {
                label: "same".into(),
                scope: BlankScope(52),
            },
            iri("label"),
            direction,
        ),
        QuadValues {
            g: Some(iri("elsewhere")),
            ..note.clone()
        },
    ];
    let mut typed = RdfDatasetBuilder::new();
    let r = typed.intern_value(&declaration.s);
    let t = typed.intern_value(&declaration.o);
    let g = typed.intern_value(declaration.g.as_ref().unwrap());
    typed.push_reifier_in_graph(r, t, Some(g));
    let p = typed.intern_value(&note.p);
    let o = typed.intern_value(&note.o);
    typed.push_annotation_in_graph(r, p, o, Some(g));
    let typed = typed.freeze().unwrap();
    let ordinary_page = eager(ordinary.clone(), &[]);
    let mut expected = RdfDatasetBuilder::new();
    for page in [&typed, &ordinary_page] {
        purrdf_core::ir::import::DatasetImporter::new(&mut expected, page.as_ref()).append();
    }
    let expected = expected.freeze().unwrap();
    let mut stack = PagedStack::new(vec![seal(typed), seal(ordinary_page)]).unwrap();
    let before = stack.snapshot().unwrap();
    let queries = [
        "SELECT ?s ?p ?o WHERE { ?s ?p ?o } ORDER BY ?p ?s ?o",
        "SELECT ?g ?s ?p ?o WHERE { GRAPH ?g { ?s ?p ?o } } ORDER BY ?g ?p ?o",
        "SELECT ?o WHERE { GRAPH ex:g { ex:r ex:note ?o } }",
        "SELECT ?o WHERE { GRAPH ex:g { ex:r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ?o } }",
        "CONSTRUCT { ?s ?p ?o } WHERE { GRAPH ex:g { ?s ?p ?o } }",
    ];
    matrix(&before, &expected, &queries);
    let (_, rows) = solutions(NativeSparqlEngine::new().query_fallible_view(&before.query_view(PagedQueryLimits::UNBOUNDED), request("SELECT ?o WHERE { GRAPH <http://example.org/g> { <http://example.org/r> <http://example.org/note> ?o } }"), QueryOptions::EMPTY).unwrap().result);
    assert_eq!(
        rows,
        vec![vec![Some(note.o.clone())]],
        "directional value is published intact"
    );
    stack.remove(&declaration).unwrap();
    stack.seal_head(bound(1)).unwrap();
    let after = stack.snapshot().unwrap();
    let expected_after = eager(ordinary.iter().cloned().chain([note.clone()]), &[]);
    matrix(&after, &expected_after, &queries);
    matrix(&before, &expected, &queries);
    for (snapshot, oracle) in [(&before, &expected), (&after, &expected_after)] {
        for n in [1, 3] {
            assert_eq!(
                carrier_pages(
                    &snapshot
                        .compact(PagedQueryLimits::UNBOUNDED, bound(n))
                        .unwrap()
                ),
                carrier_pages(&canonical_paged_seal(oracle.as_ref(), bound(n)).unwrap())
            );
        }
    }
    let new_declaration = QuadValues {
        g: Some(iri("elsewhere")),
        ..declaration
    };
    assert!(stack.insert(new_declaration.clone()).unwrap());
    let mut promoted_builder = RdfDatasetBuilder::new();
    let ordinary_expected = eager(ordinary[..2].iter().cloned().chain([note.clone()]), &[]);
    purrdf_core::ir::import::DatasetImporter::new(
        &mut promoted_builder,
        ordinary_expected.as_ref(),
    )
    .append();
    let r = promoted_builder.intern_value(&new_declaration.s);
    let triple = promoted_builder.intern_value(&new_declaration.o);
    let graph = promoted_builder.intern_value(new_declaration.g.as_ref().unwrap());
    promoted_builder.push_reifier_in_graph(r, triple, Some(graph));
    let p = promoted_builder.intern_value(&note.p);
    let o = promoted_builder.intern_value(&note.o);
    promoted_builder.push_annotation_in_graph(r, p, o, Some(graph));
    let promoted_oracle = promoted_builder.freeze().unwrap();
    let promoted_head = stack.snapshot().unwrap();
    stack.seal_head(bound(1)).unwrap();
    let promoted_sealed = stack.snapshot().unwrap();
    for snapshot in [&promoted_head, &promoted_sealed] {
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        assert_eq!(
            view.quads().count(),
            3,
            "only matching graph primary is promoted"
        );
        assert_eq!(view.annotation_quads().count(), 1);
        assert_eq!(view.reifier_quads().count(), 1);
        matrix(snapshot, &promoted_oracle, &queries);
        for n in [1, 3] {
            assert_eq!(
                carrier_pages(
                    &snapshot
                        .compact(PagedQueryLimits::UNBOUNDED, bound(n))
                        .unwrap()
                ),
                carrier_pages(&canonical_paged_seal(promoted_oracle.as_ref(), bound(n)).unwrap())
            );
        }
    }
    matrix(&after, &expected_after, &queries);
}

struct ControlledProvider {
    pages: Vec<Arc<RdfDataset>>,
    generation: AtomicU64,
    count: AtomicU64,
    reads: AtomicUsize,
    mode: AtomicUsize,
    charge: u64,
    descriptor_reads: AtomicUsize,
    drift_after: AtomicUsize,
    drift_kind: AtomicUsize,
}

#[test]
fn native_primary_and_virtual_reifier_overlap_reaches_the_same_consumer_bag() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_value(&iri("r"));
    let p = builder.intern_value(&TermValue::iri(purrdf_core::vocab::rdf::REIFIES));
    let o = builder.intern_value(&purrdf_core::term_fixture::triple_chain(1));
    builder.push_quad(s, p, o, None);
    builder.push_reifier(s, o);
    let oracle = builder.freeze().unwrap();
    let snapshot = PagedStack::new(vec![seal(Arc::clone(&oracle))])
        .unwrap()
        .snapshot()
        .unwrap();
    let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    assert_eq!(view.quads().count(), 1);
    assert_eq!(view.reifier_quads().count(), 1);
    matrix(
        &snapshot,
        &oracle,
        &[
            "SELECT ?s ?p ?o WHERE { ?s ?p ?o } ORDER BY ?s ?p ?o",
            "SELECT (COUNT(*) AS ?count) WHERE { ?s ?p ?o }",
        ],
    );
    assert_eq!(
        carrier_pages(
            &snapshot
                .compact(PagedQueryLimits::UNBOUNDED, bound(1))
                .unwrap()
        ),
        carrier_pages(&canonical_paged_seal(oracle.as_ref(), bound(1)).unwrap())
    );
}
impl ControlledProvider {
    fn new(pages: Vec<Arc<RdfDataset>>, charge: u64) -> Self {
        Self {
            count: AtomicU64::new(u64::try_from(pages.len()).unwrap()),
            pages,
            generation: AtomicU64::new(23),
            reads: AtomicUsize::new(0),
            mode: AtomicUsize::new(0),
            charge,
            descriptor_reads: AtomicUsize::new(0),
            drift_after: AtomicUsize::new(0),
            drift_kind: AtomicUsize::new(0),
        }
    }
}
impl PageProvider for ControlledProvider {
    fn page_count(&self) -> u64 {
        let actual = self.count.load(Ordering::Relaxed);
        let checkpoint = self.descriptor_reads.fetch_add(1, Ordering::Relaxed) + 1;
        if checkpoint == self.drift_after.load(Ordering::Relaxed) {
            match self.drift_kind.load(Ordering::Relaxed) {
                1 => {
                    self.generation.fetch_add(1, Ordering::Relaxed);
                }
                2 => {
                    self.count.fetch_add(1, Ordering::Relaxed);
                }
                _ => {}
            }
        }
        actual
    }
    fn generation(&self) -> PageGeneration {
        PageGeneration(self.generation.load(Ordering::Relaxed))
    }
    fn materialize(&self, id: PageId) -> Result<PageMaterialization, PageFault> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        match self.mode.load(Ordering::Relaxed) {
            1 => Err(PageFault::provider(id, "consumer source refusal")),
            2 => Err(PageFault::cancelled(id, "consumer cancellation")),
            3 => Err(PageFault::deadline_exceeded(id, "consumer deadline")),
            4 => Err(PageFault::invalid_data(id, "consumer corrupt carrier")),
            5 => Ok(PageMaterialization::new(
                eager([edge("object", "p", iri("base"))], &[]),
                PageGeneration(23),
                self.charge,
            )),
            _ => Ok(PageMaterialization::new(
                Arc::clone(&self.pages[usize::try_from(id.0).unwrap()]),
                PageGeneration(23),
                self.charge,
            )),
        }
    }
}

fn operational(
    error: FallibleSparqlError<PagedQueryError, PagedStackEvidence>,
) -> (PagedQueryError, PagedStackEvidence) {
    assert!(
        error.diagnostic().is_none(),
        "storage failure cannot become a query diagnostic"
    );
    assert!(
        error.partial_answers().is_none(),
        "storage failure cannot publish partial answers"
    );
    match error {
        FallibleSparqlError::Operational { error, evidence } => (error, evidence),
        other => panic!("expected Operational, got {other:?}"),
    }
}

#[test]
fn aggregate_admission_and_cache_receipts_are_exact_at_the_public_boundary() {
    let a = Arc::new(ControlledProvider::new(
        vec![eager([edge("a", "p", iri("b"))], &[])],
        11,
    ));
    let b = Arc::new(ControlledProvider::new(
        vec![eager([edge("b", "q", iri("c"))], &[])],
        17,
    ));
    let mut stack = PagedStack::new(vec![
        Arc::new(PagedDataset::from_provider(a.clone()).unwrap()),
        Arc::new(PagedDataset::from_provider(b.clone()).unwrap()),
    ])
    .unwrap();
    stack.insert(edge("head", "h", iri("value"))).unwrap();
    let snapshot = stack.snapshot().unwrap();
    a.reads.store(0, Ordering::Relaxed);
    b.reads.store(0, Ordering::Relaxed);
    let engine = NativeSparqlEngine::new();
    let query = "SELECT ?s ?p ?o WHERE { ?s ?p ?o } ORDER BY ?s ?p ?o";
    let unbounded = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
    let complete = engine
        .query_fallible_view(&unbounded, request(query), QueryOptions::EMPTY)
        .unwrap();
    assert_eq!(solutions(complete.result).1.len(), 3);
    let receipt = complete.evidence;
    assert_eq!(receipt.pages.consumed_pages, 3);
    assert_eq!(receipt.pages.consumed_bytes, 28 + receipt.head_bytes);
    assert!(receipt.head_bytes > 0);
    assert_eq!(
        receipt.requested_origins,
        vec![
            StackPageOrigin::Head,
            StackPageOrigin::Sealed {
                layer: 1,
                page: PageId(0)
            },
            StackPageOrigin::Sealed {
                layer: 0,
                page: PageId(0)
            }
        ]
    );
    assert_eq!(
        (
            a.reads.load(Ordering::Relaxed),
            b.reads.load(Ordering::Relaxed)
        ),
        (1, 1)
    );
    let prepared = engine.prepare_query(query, None).unwrap();
    let reread = engine
        .query_prepared_fallible_view(&unbounded, &prepared, &[], QueryOptions::EMPTY)
        .unwrap();
    assert_eq!(*reread.evidence, *receipt);
    assert_eq!(
        (
            a.reads.load(Ordering::Relaxed),
            b.reads.load(Ordering::Relaxed)
        ),
        (1, 1),
        "cache rereads neither materialize nor charge"
    );
    let exact = snapshot.query_view(PagedQueryLimits::new(3, receipt.pages.consumed_bytes));
    assert_eq!(
        *engine
            .query_fallible_view(&exact, request(query), QueryOptions::EMPTY)
            .unwrap()
            .evidence,
        *receipt
    );
    for limits in [
        PagedQueryLimits::new(0, 0),
        PagedQueryLimits::new(2, u64::MAX),
        PagedQueryLimits::new(3, receipt.pages.consumed_bytes - 1),
    ] {
        a.reads.store(0, Ordering::Relaxed);
        b.reads.store(0, Ordering::Relaxed);
        let view = snapshot.query_view(limits);
        let (error, evidence) = operational(
            engine
                .query_fallible_view(&view, request(query), QueryOptions::EMPTY)
                .expect_err("one refused page prevents publication"),
        );
        assert!(matches!(
            error,
            PagedQueryError::PageBudgetExceeded { .. } | PagedQueryError::ByteBudgetExceeded { .. }
        ));
        assert_eq!(
            evidence.pages.requested_pages.len(),
            usize::try_from(evidence.pages.consumed_pages + 1).unwrap(),
            "refused address remains requested"
        );
        assert_eq!(
            u64::try_from(a.reads.load(Ordering::Relaxed) + b.reads.load(Ordering::Relaxed))
                .unwrap(),
            evidence.pages.consumed_pages.saturating_sub(1),
            "rejected source is never materialized"
        );
        assert!(
            snapshot.compact(limits, bound(1)).is_err(),
            "no partially folded artifact"
        );
    }
}

#[test]
fn source_faults_are_operational_sticky_and_refuse_compacted_artifacts() {
    for mode in 1..=5 {
        let provider = Arc::new(ControlledProvider::new(
            vec![eager([edge("base", "p", iri("object"))], &[])],
            19,
        ));
        let mut stack = PagedStack::new(vec![Arc::new(
            PagedDataset::from_provider(provider.clone()).unwrap(),
        )])
        .unwrap();
        stack.insert(edge("head", "h", iri("value"))).unwrap();
        stack.declare_named_graph(iri("empty")).unwrap();
        let snapshot = stack.snapshot().unwrap();
        provider.reads.store(0, Ordering::Relaxed);
        provider.mode.store(mode, Ordering::Relaxed);
        let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
        let engine = NativeSparqlEngine::new();
        let (error, evidence) = operational(
            engine
                .query_fallible_view(
                    &view,
                    request("SELECT * WHERE { ?s ?p ?o }"),
                    QueryOptions::EMPTY,
                )
                .expect_err("failure after head cannot publish head"),
        );
        match (mode, &error) {
            (1, PagedQueryError::Provider { .. })
            | (4 | 5, PagedQueryError::InvalidData { .. }) => {}
            (
                2,
                PagedQueryError::Stopped {
                    cause: StopCause::Cancelled,
                    ..
                },
            )
            | (
                3,
                PagedQueryError::Stopped {
                    cause: StopCause::Deadline,
                    ..
                },
            ) => {}
            _ => panic!("wrong operational root: {mode}: {error:?}"),
        }
        assert_eq!(provider.reads.load(Ordering::Relaxed), 1);
        assert_eq!(
            evidence.pages.consumed_pages, 1,
            "head admitted, failed sealed page not consumed"
        );
        assert_eq!(view.quads().count(), 0);
        assert_eq!(view.named_graphs().count(), 0);
        let prepared = engine.prepare_query("SELECT * WHERE {}", None).unwrap();
        let (sticky, sticky_evidence) = operational(
            engine
                .query_prepared_fallible_view(&view, &prepared, &[], QueryOptions::EMPTY)
                .expect_err("constant query cannot escape the failure latch"),
        );
        assert_eq!(sticky, error);
        assert_eq!(sticky_evidence, evidence);
        assert!(
            snapshot
                .compact(PagedQueryLimits::UNBOUNDED, bound(1))
                .is_err()
        );
    }
}

#[test]
fn constants_and_head_queries_check_skipped_and_zero_page_source_descriptors() {
    for zero_pages in [false, true] {
        for count_drift in [false, true] {
            let pages = if zero_pages {
                vec![]
            } else {
                vec![eager([edge("unrelated", "p", iri("object"))], &[])]
            };
            let provider = Arc::new(ControlledProvider::new(pages, 13));
            let mut stack = PagedStack::new(vec![
                seal(eager([edge("untouched", "p", iri("unrelated"))], &[])),
                Arc::new(PagedDataset::from_provider(provider.clone()).unwrap()),
            ])
            .unwrap();
            stack.insert(edge("head", "h", iri("value"))).unwrap();
            let snapshot = stack.snapshot().unwrap();
            provider.reads.store(0, Ordering::Relaxed);
            if count_drift {
                provider.count.fetch_add(1, Ordering::Relaxed);
            } else {
                provider.generation.fetch_add(1, Ordering::Relaxed);
            }
            for query in [
                "SELECT (1 AS ?constant) WHERE {}",
                "SELECT ?o WHERE { <http://example.org/head> <http://example.org/h> ?o }",
            ] {
                let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
                let engine = NativeSparqlEngine::new();
                let prepared = engine.prepare_query(query, None).unwrap();
                for error in [
                    engine
                        .query_fallible_view(&view, request(query), QueryOptions::EMPTY)
                        .unwrap_err(),
                    engine
                        .query_prepared_fallible_view(&view, &prepared, &[], QueryOptions::EMPTY)
                        .unwrap_err(),
                ] {
                    let (root, evidence) = operational(error);
                    let PagedQueryError::SourceSnapshot { source, error } = root else {
                        panic!("full source vector drift was not preserved: {root:?}");
                    };
                    assert_eq!(source, 1);
                    assert_eq!(evidence.sources.len(), 2);
                    assert!(if count_drift {
                        matches!(*error, PagedQueryError::PageCountMismatch { .. })
                    } else {
                        matches!(*error, PagedQueryError::StaleGeneration { .. })
                    });
                    assert_eq!(evidence.pages.requested_pages, [] as [PageId; 0]);
                    assert_eq!(evidence.pages.consumed_bytes, 0);
                }
                assert_eq!(view.quads().count(), 0);
            }
            assert_eq!(
                provider.reads.load(Ordering::Relaxed),
                0,
                "descriptor checks do not fetch skipped content"
            );
            assert!(
                snapshot
                    .compact(PagedQueryLimits::UNBOUNDED, bound(1))
                    .is_err()
            );
        }
    }
}

#[test]
fn delayed_drift_at_the_last_checkpoint_refuses_cached_guarded_results_and_fold() {
    let provider = Arc::new(ControlledProvider::new(
        vec![eager(
            [edge("a", "p", iri("b")), edge("c", "p", iri("d"))],
            &[],
        )],
        31,
    ));
    let snapshot = PagedStack::new(vec![Arc::new(
        PagedDataset::from_provider(provider.clone()).unwrap(),
    )])
    .unwrap()
    .snapshot()
    .unwrap();
    let engine = NativeSparqlEngine::new();
    let text = "SELECT ?s ?o WHERE { ?s <http://example.org/p> ?o } ORDER BY ?s";
    let prepared = engine.prepare_query(text, None).unwrap();
    for operation in 0..3 {
        for drift_kind in [1, 2] {
            provider.generation.store(23, Ordering::Relaxed);
            provider.count.store(1, Ordering::Relaxed);
            provider.drift_after.store(0, Ordering::Relaxed);
            let view = snapshot.query_view(PagedQueryLimits::UNBOUNDED);
            assert_eq!(
                view.quads().count(),
                2,
                "admit the exact pages before measuring cached publication"
            );
            provider.descriptor_reads.store(0, Ordering::Relaxed);
            match operation {
                0 => assert_eq!(
                    solutions(
                        engine
                            .query_fallible_view(&view, request(text), QueryOptions::EMPTY)
                            .unwrap()
                            .result
                    )
                    .1
                    .len(),
                    2
                ),
                1 => assert_eq!(
                    solutions(
                        engine
                            .query_prepared_fallible_view(
                                &view,
                                &prepared,
                                &[],
                                QueryOptions::EMPTY
                            )
                            .unwrap()
                            .result
                    )
                    .1
                    .len(),
                    2
                ),
                _ => assert_eq!(
                    canonical_paged_seal(&view, bound(1)).unwrap().page_count(),
                    2
                ),
            }
            let checkpoints = provider.descriptor_reads.load(Ordering::Relaxed);
            assert!(checkpoints >= 2);
            // Observe the production call sequence rather than copying it into a
            // test wrapper: mutate just after its penultimate healthy checkpoint.
            provider.descriptor_reads.store(0, Ordering::Relaxed);
            provider.drift_kind.store(drift_kind, Ordering::Relaxed);
            provider
                .drift_after
                .store(checkpoints - 1, Ordering::Relaxed);
            let reads_before = provider.reads.load(Ordering::Relaxed);
            let error = match operation {
                0 => {
                    operational(
                        engine
                            .query_fallible_view(&view, request(text), QueryOptions::EMPTY)
                            .expect_err("final drift must refuse ordinary publication"),
                    )
                    .0
                }
                1 => {
                    operational(
                        engine
                            .query_prepared_fallible_view(
                                &view,
                                &prepared,
                                &[],
                                QueryOptions::EMPTY,
                            )
                            .expect_err("final drift must refuse prepared publication"),
                    )
                    .0
                }
                _ => match canonical_paged_seal(&view, bound(1))
                    .expect_err("final drift must refuse a built fold")
                {
                    purrdf_core::CanonicalPagedError::Read(error) => error,
                    other => panic!("expected final checkpoint error, got {other:?}"),
                },
            };
            let PagedQueryError::SourceSnapshot { source, error } = error else {
                panic!("delayed source drift must retain its qualified cause");
            };
            assert_eq!(source, 0);
            if drift_kind == 1 {
                assert!(matches!(
                    *error,
                    PagedQueryError::StaleGeneration {
                        expected: PageGeneration(23),
                        actual: PageGeneration(24),
                        ..
                    }
                ));
            } else {
                assert!(matches!(
                    *error,
                    PagedQueryError::PageCountMismatch {
                        expected: 1,
                        actual: 2
                    }
                ));
                assert_eq!(
                    provider.descriptor_reads.load(Ordering::Relaxed),
                    checkpoints
                );
            }
            assert_eq!(
                provider.reads.load(Ordering::Relaxed),
                reads_before,
                "delayed cached refusal must not depend on another page admission"
            );
            assert_eq!(view.quads().next(), None);
            eprintln!(
                "delayed operation={operation} drift={drift_kind} healthy_checkpoints={checkpoints}; complete result/artifact refused"
            );
        }
    }
}
