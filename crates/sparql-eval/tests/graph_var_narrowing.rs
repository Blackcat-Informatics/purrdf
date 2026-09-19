// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `GRAPH ?g { ... }` over a many-graph dataset: the graph-major loop binds `?g` to
//! exactly the graphs SPARQL 1.1 §8.3/§18.6 says it must, and pays for a graph only
//! when that graph can answer.
//!
//! The loop evaluates its inner pattern once per named graph. Two things therefore
//! matter, and they pull in opposite directions:
//!
//! * **Nothing may be skipped that could answer.** `?g` ranges over EVERY named graph
//!   in the active dataset — a graph declared with no quads in it, and a graph named
//!   only by an RDF 1.2 reifier or annotation row, are named graphs like any other.
//!   Worse, a graph with zero base quads can still answer a pattern outright, through
//!   the statement layer alone. An over-eager skip here is a silent wrong answer.
//! * **Nothing should be paid for that cannot answer.** A graph holding no row of any
//!   kind cannot satisfy a pattern that needs a row, so driving it through a full
//!   inner algebra evaluation buys nothing.
//!
//! The guards below pin both directions:
//!
//! * **A — enumeration.** `GRAPH ?g {}` binds `?g` to every named graph, declared-empty
//!   and side-table-only included, identically over a paged and a single dataset.
//! * **B — parity.** `GRAPH ?g { ?s ?p ?o }` is byte-identical paged vs. single.
//! * **C — the over-skip trap.** A graph whose ONLY rows are a reifier row and an
//!   annotation row still answers patterns that match them, and still binds `?g`.
//! * **D — the shapes that answer with no data at all.** An unkeyed aggregate, a
//!   `BIND` over the empty group pattern, inline `VALUES`, and a zero-length property
//!   path each produce a row in a graph holding nothing, so each must still be
//!   evaluated in every graph.
//! * **E — evaluation count.** A counting view shows a row-free graph costing ZERO
//!   data probes, while the enumeration of A is untouched on the same fixture.
//! * **F — page touches.** Over a counting page provider, `GRAPH ?g { ... }` pulls
//!   exactly the pages that own a named graph, and no others.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use purrdf_core::{
    CountingDemandProvider, DatasetView, GraphMatch, InMemoryPageProvider, PageProvider,
    PagedDataset, QuadIds, QuadProbePlan, QuadRef, RdfDataset, RdfDatasetBuilder, RdfLiteral,
    RdfStoreCapabilities, SparqlResult, TermId, TermRef, TermValue,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

// ── shared fixture helpers ─────────────────────────────────────────────────────

const EX: &str = "http://example.org/";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// An `example.org` IRI value.
fn iri(name: &str) -> TermValue {
    TermValue::iri(format!("{EX}{name}"))
}

/// Intern one dataset-independent value into a builder, recursing for triple terms.
fn intern_value(b: &mut RdfDatasetBuilder, v: &TermValue) -> TermId {
    match v {
        TermValue::Iri(s) => b.intern_iri(s),
        TermValue::Blank { label, scope } => b.intern_blank(label, *scope),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => b.intern_literal(RdfLiteral {
            lexical_form: lexical_form.clone(),
            datatype: Some(datatype.clone()),
            language: language.clone(),
            direction: *direction,
        }),
        TermValue::Triple { s, p, o } => {
            let s = intern_value(b, s);
            let p = intern_value(b, p);
            let o = intern_value(b, o);
            b.intern_triple(s, p, o)
        }
    }
}

/// A quad in a named graph (`None` graph ⇒ the default graph).
type Quad = (TermValue, TermValue, TermValue, Option<TermValue>);

/// Everything one frozen dataset (or one page of one) is built from: base quads, named
/// graphs declared with no quads at all, and RDF 1.2 side-table rows scoped to a graph.
#[derive(Default, Clone)]
struct Fixture {
    /// Base quads, each with its own graph slot.
    quads: Vec<Quad>,
    /// Named graphs declared to exist while owning no quad.
    declared_graphs: Vec<TermValue>,
    /// `(reifier, reified triple, graph)` side-table rows.
    reifiers: Vec<(TermValue, TermValue, Option<TermValue>)>,
    /// `(reifier, predicate, object, graph)` side-table rows.
    annotations: Vec<(TermValue, TermValue, TermValue, Option<TermValue>)>,
}

impl Fixture {
    /// Concatenate two fixtures — the "same content, one dataset" counterpart of
    /// splitting content across pages.
    fn merged(parts: &[Self]) -> Self {
        let mut all = Self::default();
        for part in parts {
            all.quads.extend(part.quads.iter().cloned());
            all.declared_graphs
                .extend(part.declared_graphs.iter().cloned());
            all.reifiers.extend(part.reifiers.iter().cloned());
            all.annotations.extend(part.annotations.iter().cloned());
        }
        all
    }
}

/// Freeze one fixture into a dataset (one page, or the single-dataset reference).
fn build(fixture: &Fixture) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    for (s, p, o, g) in &fixture.quads {
        let s = intern_value(&mut b, s);
        let p = intern_value(&mut b, p);
        let o = intern_value(&mut b, o);
        let g = g.as_ref().map(|g| intern_value(&mut b, g));
        b.push_quad(s, p, o, g);
    }
    for g in &fixture.declared_graphs {
        let g = intern_value(&mut b, g);
        b.declare_named_graph(g);
    }
    for (reifier, triple, g) in &fixture.reifiers {
        let reifier = intern_value(&mut b, reifier);
        let triple = intern_value(&mut b, triple);
        let g = g.as_ref().map(|g| intern_value(&mut b, g));
        b.push_reifier_in_graph(reifier, triple, g);
    }
    for (reifier, p, o, g) in &fixture.annotations {
        let reifier = intern_value(&mut b, reifier);
        let p = intern_value(&mut b, p);
        let o = intern_value(&mut b, o);
        let g = g.as_ref().map(|g| intern_value(&mut b, g));
        b.push_annotation_in_graph(reifier, p, o, g);
    }
    b.freeze().expect("fixture freeze")
}

/// Seal per-page fixtures into a `PagedDataset`.
fn paged(pages: &[Fixture]) -> PagedDataset {
    let frozen: Vec<Arc<RdfDataset>> = pages.iter().map(build).collect();
    PagedDataset::from_provider(Arc::new(InMemoryPageProvider::new(frozen))).expect("seal pages")
}

/// Destructure a `SparqlResult` into `(variables, rows)`, panicking on any other shape.
fn solutions(result: SparqlResult) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => (variables, rows),
        other => panic!("expected solutions, got {other:?}"),
    }
}

/// Run `query` over a single frozen dataset holding every page's content AND over the
/// paged view of the same content, assert the two agree exactly, and return the rows.
///
/// Agreement between the two backends is the strongest statement available here: the
/// paged view is the one whose graph index the narrowing consults, the single dataset is
/// the one whose named-graph set comes straight from the builder, and a skip that lost a
/// solution would have to lose it identically on both to escape.
fn rows_agreeing_across_backends(
    pages: &[Fixture],
    query: &str,
) -> (Vec<String>, Vec<Vec<Option<TermValue>>>) {
    let single = build(&Fixture::merged(pages));
    let view = paged(pages);
    let engine = NativeSparqlEngine::new();
    let prepared = engine.prepare_query(query, None).expect("prepare");

    let (single_vars, single_rows) = solutions(
        engine
            .query_prepared(&single, &prepared, &[], QueryOptions::EMPTY)
            .expect("single query"),
    );
    let (paged_vars, paged_rows) = solutions(
        engine
            .query_prepared_view(&view, &prepared, &[], QueryOptions::EMPTY)
            .expect("paged query"),
    );
    assert_eq!(single_vars, paged_vars, "same projected variables");
    assert_eq!(single_rows, paged_rows, "same rows: single == paged");
    (single_vars, single_rows)
}

/// The first column of every row, as `TermValue`s, panicking on an unbound cell.
fn first_column(rows: &[Vec<Option<TermValue>>]) -> Vec<TermValue> {
    rows.iter()
        .map(|r| r[0].clone().expect("first column is bound"))
        .collect()
}

// ── the shared many-graph fixture ───────────────────────────────────────────────

/// Page 0 — a named graph with ordinary base quads, plus TWO named graphs declared to
/// exist while owning nothing at all.
fn page_with_data_and_declared_empties() -> Fixture {
    Fixture {
        quads: vec![
            (iri("alice"), iri("p"), iri("o1"), Some(iri("gdata"))),
            (iri("bob"), iri("p"), iri("o2"), Some(iri("gdata"))),
        ],
        declared_graphs: vec![iri("gempty1"), iri("gempty2")],
        ..Fixture::default()
    }
}

/// Page 1 — a named graph holding NO base quad, named only by the RDF 1.2 side tables:
/// one reifier row and one annotation about that reifier. `:gside` is a named graph, its
/// rows are real data, and a pattern can match them with the quad table empty.
fn page_with_side_table_only_graph() -> Fixture {
    let reified = TermValue::Triple {
        s: Box::new(iri("alice")),
        p: Box::new(iri("p")),
        o: Box::new(iri("o1")),
    };
    Fixture {
        reifiers: vec![(iri("r1"), reified, Some(iri("gside")))],
        annotations: vec![(
            iri("r1"),
            iri("certainty"),
            TermValue::simple_literal("high"),
            Some(iri("gside")),
        )],
        ..Fixture::default()
    }
}

/// Page 2 — default-graph content only, so it belongs to NO named graph and a
/// `GRAPH ?g` query has no reason to read it.
fn page_with_default_graph_only() -> Fixture {
    Fixture {
        quads: vec![(iri("carol"), iri("p"), iri("o3"), None)],
        ..Fixture::default()
    }
}

/// The three pages above, which together carry four named graphs: one with base quads
/// (`:gdata`), two declared empty (`:gempty1`, `:gempty2`), and one named only by side
/// tables (`:gside`).
fn many_graph_pages() -> Vec<Fixture> {
    vec![
        page_with_data_and_declared_empties(),
        page_with_side_table_only_graph(),
        page_with_default_graph_only(),
    ]
}

/// Every named graph the fixture above declares, ascending by IRI (the `ORDER BY ?g`
/// order every enumeration query below asks for).
fn every_named_graph() -> Vec<TermValue> {
    vec![iri("gdata"), iri("gempty1"), iri("gempty2"), iri("gside")]
}

// ── A — enumeration: `?g` ranges over every named graph ─────────────────────────

/// Guard A — enumeration. `GRAPH ?g {}` must bind `?g` to all four named graphs
/// this fixture declares: the one with base quads, both declared-empty ones, and
/// the one named only by a reifier/annotation row.
#[test]
fn graph_var_binds_every_named_graph_including_empty_and_side_table_only() {
    let (vars, rows) = rows_agreeing_across_backends(
        &many_graph_pages(),
        "SELECT ?g WHERE { GRAPH ?g {} } ORDER BY ?g",
    );
    assert_eq!(vars, vec!["g"]);
    // All four: the graph with quads, BOTH declared-empty graphs, and the graph named
    // only by a reifier/annotation row. The empty group pattern answers with the
    // one-row identity table in every graph, so this is exactly the enumeration.
    assert_eq!(
        first_column(&rows),
        every_named_graph(),
        "GRAPH ?g {{}} enumerates every named graph"
    );
}

// ── B — parity on the whole-graph scan ──────────────────────────────────────────

/// Guard B — parity. `GRAPH ?g { ?s ?p ?o }` must be byte-identical between the
/// paged and single-dataset backends, and must contribute only the graphs that
/// hold an unconstrained-triple-pattern row: the declared-empty graphs answer
/// nothing here even though they still enumerate under guard A.
#[test]
fn graph_var_whole_graph_scan_agrees_across_backends() {
    let (vars, rows) = rows_agreeing_across_backends(
        &many_graph_pages(),
        "SELECT ?g ?s ?p ?o WHERE { GRAPH ?g { ?s ?p ?o } } ORDER BY ?g ?s ?p ?o",
    );
    assert_eq!(vars, vec!["g", "s", "p", "o"]);
    // `:gdata`'s two quads, plus `:gside`'s reifier row and annotation row surfacing as
    // the virtual triples they denote. The declared-empty graphs contribute nothing —
    // an unconstrained triple pattern needs a row, and they have none.
    let graphs = first_column(&rows);
    assert_eq!(
        graphs,
        vec![iri("gdata"), iri("gdata"), iri("gside"), iri("gside")],
        "two base quads in :gdata, two statement-layer rows in :gside"
    );
}

// ── C — the over-skip trap: a graph whose only rows are side-table rows ─────────

/// Guard C — the over-skip trap. `:gside` owns zero base quads, so "no quads in
/// this graph" would wrongly skip the one graph that can answer through its
/// annotation row (matched by `<...certainty>`) and, separately, its reifier row
/// (matched through `rdf:reifies`); both must still bind `?g` to `:gside`.
#[test]
fn graph_var_answers_from_a_graph_whose_only_rows_are_statement_layer_rows() {
    // The annotation row, matched by its own predicate. `:gside` owns ZERO base quads,
    // so "no quads in this graph" would have skipped the one graph that answers.
    let (vars, rows) = rows_agreeing_across_backends(
        &many_graph_pages(),
        &format!("SELECT ?g ?r ?c WHERE {{ GRAPH ?g {{ ?r <{EX}certainty> ?c }} }} ORDER BY ?g ?r"),
    );
    assert_eq!(vars, vec!["g", "r", "c"]);
    assert_eq!(rows.len(), 1, "the annotation row answers");
    assert_eq!(rows[0][0].as_ref().expect("?g bound"), &iri("gside"));
    assert_eq!(rows[0][1].as_ref().expect("?r bound"), &iri("r1"));
    assert_eq!(
        rows[0][2].as_ref().expect("?c bound"),
        &TermValue::simple_literal("high")
    );

    // The reifier row, matched through `rdf:reifies`. Same graph, same trap, the other
    // side table.
    let (_vars, rows) = rows_agreeing_across_backends(
        &many_graph_pages(),
        &format!("SELECT ?g ?r WHERE {{ GRAPH ?g {{ ?r <{RDF_REIFIES}> ?t }} }} ORDER BY ?g ?r"),
    );
    assert_eq!(rows.len(), 1, "the reifier row answers");
    assert_eq!(rows[0][0].as_ref().expect("?g bound"), &iri("gside"));
    assert_eq!(rows[0][1].as_ref().expect("?r bound"), &iri("r1"));
}

// ── D — shapes that answer with no data underneath them ─────────────────────────

/// Guard D — the shapes that answer with no data underneath them: an unkeyed
/// aggregate, `BIND` over the empty group pattern, inline `VALUES`, and a
/// zero-length property path each produce a row with no data probe at all, so
/// every named graph — including the declared-empty ones — must still answer each
/// of the four.
#[test]
fn graph_var_keeps_evaluating_shapes_that_answer_without_any_row() {
    // An UNKEYED aggregate: `COUNT` over zero rows is one row holding zero, so every
    // named graph — the empty ones included — contributes a row. A `Group` with no
    // grouping key is therefore never a candidate for skipping.
    let (vars, rows) = rows_agreeing_across_backends(
        &many_graph_pages(),
        &format!(
            "SELECT ?g ?c WHERE {{ GRAPH ?g {{ SELECT (COUNT(?s) AS ?c) WHERE {{ ?s <{EX}p> ?o }} }} }} ORDER BY ?g"
        ),
    );
    assert_eq!(vars, vec!["g", "c"]);
    assert_eq!(
        first_column(&rows),
        every_named_graph(),
        "an unkeyed aggregate answers in every named graph"
    );

    // `BIND` over the empty group pattern: one row per graph, data or no data.
    let (_vars, rows) = rows_agreeing_across_backends(
        &many_graph_pages(),
        "SELECT ?g ?one WHERE { GRAPH ?g { BIND(1 AS ?one) } } ORDER BY ?g",
    );
    assert_eq!(
        first_column(&rows),
        every_named_graph(),
        "BIND over the empty group pattern answers in every named graph"
    );

    // Inline `VALUES` carries its own rows and reads no graph at all.
    let (_vars, rows) = rows_agreeing_across_backends(
        &many_graph_pages(),
        "SELECT ?g ?v WHERE { GRAPH ?g { VALUES ?v { 1 } } } ORDER BY ?g",
    );
    assert_eq!(
        first_column(&rows),
        every_named_graph(),
        "inline VALUES answers in every named graph"
    );

    // A zero-length property path from a BOUND subject binds that subject to itself
    // without reading an edge, so `?` (and `*`) are never skipped either.
    let (_vars, rows) = rows_agreeing_across_backends(
        &many_graph_pages(),
        &format!("SELECT ?g ?o WHERE {{ GRAPH ?g {{ <{EX}alice> <{EX}q>? ?o }} }} ORDER BY ?g"),
    );
    assert_eq!(
        first_column(&rows),
        every_named_graph(),
        "a zero-length path match answers in every named graph"
    );
}

// ── E — evaluation count: a row-free graph costs no data probe ──────────────────

/// A `DatasetView` that forwards every call to a frozen [`RdfDataset`] and counts the
/// planned pattern probes — the one call the BGP matcher makes to read data.
///
/// The count is the number of inner-pattern evaluations that actually reached the quad
/// table, which is what "a graph that cannot answer is not driven through a full inner
/// evaluation" means in observable terms. The emptiness probe the narrowing itself runs
/// goes through `quads_for_pattern` and is deliberately NOT counted here: this counter
/// answers "how many graphs were evaluated", not "how many iterators were made".
struct ProbeCountingView {
    inner: Arc<RdfDataset>,
    planned_probes: AtomicUsize,
}

impl ProbeCountingView {
    /// Wraps `inner` with the counter zeroed, so every measurement starts from a
    /// known baseline of zero planned probes.
    fn new(inner: Arc<RdfDataset>) -> Self {
        Self {
            inner,
            planned_probes: AtomicUsize::new(0),
        }
    }

    /// The number of `quads_for_pattern_with_plan` calls so far — one per inner
    /// pattern evaluation that actually reached the quad table, per the type doc.
    fn planned_probes(&self) -> usize {
        self.planned_probes.load(Ordering::Relaxed)
    }
}

impl DatasetView for ProbeCountingView {
    type Id = TermId;
    type ProbePlan = QuadProbePlan;

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads(&*self.inner)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn quad_refs(&self) -> impl Iterator<Item = QuadRef<'_>> + '_ {
        DatasetView::quad_refs(&*self.inner)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn resolve(&self, id: TermId) -> TermRef<'_> {
        DatasetView::resolve(&*self.inner, id)
    }

    /// Forwards to the inner view unchanged and deliberately UNCOUNTED — per the
    /// type doc, the narrowing's own emptiness probe goes through this method, and
    /// `planned_probes` must answer "how many graphs were evaluated", not "how many
    /// iterators were made".
    fn quads_for_pattern(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::quads_for_pattern(&*self.inner, s, p, o, g)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn term_id_by_value(&self, value: &TermValue) -> Option<TermId> {
        DatasetView::term_id_by_value(&*self.inner, value)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn capabilities(&self) -> RdfStoreCapabilities {
        DatasetView::capabilities(&*self.inner)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn len_hint(&self) -> Option<usize> {
        DatasetView::len_hint(&*self.inner)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn probe_plan(
        &self,
        s_bound: bool,
        p_bound: bool,
        o_bound: bool,
        g: GraphMatch,
    ) -> QuadProbePlan {
        DatasetView::probe_plan(&*self.inner, s_bound, p_bound, o_bound, g)
    }

    /// The ONE counted call: increments `planned_probes` before forwarding, because
    /// this is the method the BGP matcher calls once it has committed to actually
    /// reading data for the current graph — see the type doc for why this, and not
    /// `quads_for_pattern`, is the right method to count.
    fn quads_for_pattern_with_plan(
        &self,
        plan: &QuadProbePlan,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> impl Iterator<Item = QuadIds> + '_ {
        self.planned_probes.fetch_add(1, Ordering::Relaxed);
        DatasetView::quads_for_pattern_with_plan(&*self.inner, plan, s, p, o, g)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn cardinality_estimate(
        &self,
        s: Option<TermId>,
        p: Option<TermId>,
        o: Option<TermId>,
        g: GraphMatch,
    ) -> usize {
        DatasetView::cardinality_estimate(&*self.inner, s, p, o, g)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn term_count(&self) -> usize {
        DatasetView::term_count(&*self.inner)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn stats_fingerprint(&self) -> u64 {
        DatasetView::stats_fingerprint(&*self.inner)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn reifier_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::reifier_quads(&*self.inner)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn reifier_quads_of(&self, reifier: TermId) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::reifier_quads_of(&*self.inner, reifier)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn annotation_quads(&self) -> impl Iterator<Item = QuadIds> + '_ {
        DatasetView::annotation_quads(&*self.inner)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn annotations_of_with_graph(
        &self,
        reifier: TermId,
    ) -> impl Iterator<Item = (TermId, TermId, Option<TermId>)> + '_ {
        DatasetView::annotations_of_with_graph(&*self.inner, reifier)
    }

    /// Forwards to the inner view unchanged; not part of what this type measures.
    fn named_graphs(&self) -> impl Iterator<Item = TermId> + '_ {
        DatasetView::named_graphs(&*self.inner)
    }
}

/// Named graphs in the counting fixture that own base quads.
const GRAPHS_WITH_DATA: usize = 2;
/// Named graphs in the counting fixture declared to exist while owning nothing.
const GRAPHS_DECLARED_EMPTY: usize = 20;

/// Two named graphs with one quad each, plus twenty declared-empty ones — the shape
/// where the per-graph loop's cost is dominated by graphs that cannot answer.
fn counting_fixture() -> Fixture {
    let quads = (0..GRAPHS_WITH_DATA)
        .map(|i| {
            (
                iri(&format!("s{i}")),
                iri("p"),
                iri(&format!("o{i}")),
                Some(iri(&format!("gdata{i}"))),
            )
        })
        .collect();
    let declared_graphs = (0..GRAPHS_DECLARED_EMPTY)
        .map(|i| iri(&format!("gempty{i:02}")))
        .collect();
    Fixture {
        quads,
        declared_graphs,
        ..Fixture::default()
    }
}

/// Guard E — evaluation count. Over twenty declared-empty graphs plus two with
/// data, a pattern needing a row drives exactly `GRAPHS_WITH_DATA` graphs through
/// `quads_for_pattern_with_plan` — never the declared-empty ones — while the same
/// fixture's `GRAPH ?g {}` enumeration is untouched and still binds all
/// twenty-two.
#[test]
fn a_row_free_graph_costs_no_data_probe_yet_still_enumerates() {
    let view = ProbeCountingView::new(build(&counting_fixture()));
    let engine = NativeSparqlEngine::new();

    // The pattern needs a row, so a graph with no rows provably answers nothing.
    let prepared = engine
        .prepare_query(
            &format!("SELECT ?g ?s WHERE {{ GRAPH ?g {{ ?s <{EX}p> ?o }} }} ORDER BY ?g"),
            None,
        )
        .expect("prepare");
    let (_vars, rows) = solutions(
        engine
            .query_prepared_view(&view, &prepared, &[], QueryOptions::EMPTY)
            .expect("counted query"),
    );

    // The answer is the two graphs that hold a quad.
    assert_eq!(
        first_column(&rows),
        vec![iri("gdata0"), iri("gdata1")],
        "only the graphs holding a quad answer this pattern"
    );

    // The measurement: exactly one planned probe per graph that could answer. Driving
    // every named graph through the inner evaluation would have made
    // `GRAPHS_WITH_DATA + GRAPHS_DECLARED_EMPTY` of them.
    assert_eq!(
        view.planned_probes(),
        GRAPHS_WITH_DATA,
        "a graph holding no row is never driven through the inner pattern"
    );

    // The other direction, on the SAME fixture: the enumeration itself is untouched.
    // Every named graph still binds `?g`, so the narrowing above bought its saving
    // without losing a single graph.
    let prepared = engine
        .prepare_query("SELECT ?g WHERE { GRAPH ?g {} } ORDER BY ?g", None)
        .expect("prepare");
    let (_vars, rows) = solutions(
        engine
            .query_prepared_view(&view, &prepared, &[], QueryOptions::EMPTY)
            .expect("enumeration query"),
    );
    assert_eq!(
        rows.len(),
        GRAPHS_WITH_DATA + GRAPHS_DECLARED_EMPTY,
        "every named graph still enumerates"
    );
}

/// Every named graph row-free at once — the case where the per-graph loop contributes
/// no block at all, so the result's COLUMNS come from the fallback rather than from any
/// graph's evaluation. The answer is empty, and its shape must still be the inner
/// pattern's columns plus the graph variable, in that order.
#[test]
fn a_result_whose_every_graph_was_row_free_still_carries_the_right_columns() {
    let pages = vec![
        Fixture {
            // Default-graph content, so the dataset is not degenerate: it holds quads,
            // just none in any NAMED graph.
            quads: vec![(iri("alice"), iri("p"), iri("o1"), None)],
            declared_graphs: vec![iri("gempty1"), iri("gempty2"), iri("gempty3")],
            ..Fixture::default()
        },
        Fixture::default(),
    ];

    for (query, expected) in [
        (
            "SELECT ?s ?p ?o ?g WHERE { GRAPH ?g { ?s ?p ?o } }",
            vec!["s", "p", "o", "g"],
        ),
        (
            "SELECT * WHERE { GRAPH ?g { ?s ?p ?o } }",
            vec!["g", "s", "p", "o"],
        ),
    ] {
        let (vars, rows) = rows_agreeing_across_backends(&pages, query);
        assert!(
            rows.is_empty(),
            "no named graph holds a row, so nothing answers: {query}"
        );
        assert_eq!(
            vars, expected,
            "the projected columns are unchanged: {query}"
        );
    }

    // And the neighbouring query that DOES answer on the same fixture: the enumeration
    // still reaches every declared-empty graph.
    let (_vars, rows) =
        rows_agreeing_across_backends(&pages, "SELECT ?g WHERE { GRAPH ?g {} } ORDER BY ?g");
    assert_eq!(
        first_column(&rows),
        vec![iri("gempty1"), iri("gempty2"), iri("gempty3")],
        "every declared-empty graph still enumerates"
    );
}

// ── F — page touches: only the pages owning a named graph are pulled ────────────

/// Guard F — page touches. Over six pages, three carrying one named graph each and
/// three carrying default-graph-only content, `GRAPH ?g { ... }` must pull only
/// the three pages that own a named graph — the per-graph page index proves the
/// other three can be skipped without materializing them to find out.
#[test]
fn graph_var_materializes_only_the_pages_that_own_a_named_graph() {
    // Six pages: three carry one named graph each, three carry default-graph content
    // only. A `GRAPH ?g { ... }` query has no reason to read the latter three, and the
    // per-graph page index is what lets it avoid them WITHOUT materializing anything to
    // find out.
    let provider = Arc::new(CountingDemandProvider::new(vec![
        Box::new(|| {
            build(&Fixture {
                quads: vec![(iri("a"), iri("p"), iri("o"), Some(iri("g0")))],
                ..Fixture::default()
            })
        }),
        Box::new(|| {
            build(&Fixture {
                quads: vec![(iri("b"), iri("p"), iri("o"), Some(iri("g1")))],
                ..Fixture::default()
            })
        }),
        Box::new(|| {
            build(&Fixture {
                quads: vec![(iri("c"), iri("p"), iri("o"), Some(iri("g2")))],
                ..Fixture::default()
            })
        }),
        Box::new(|| {
            build(&Fixture {
                quads: vec![(iri("d"), iri("p"), iri("o"), None)],
                ..Fixture::default()
            })
        }),
        Box::new(|| {
            build(&Fixture {
                quads: vec![(iri("e"), iri("p"), iri("o"), None)],
                ..Fixture::default()
            })
        }),
        Box::new(|| {
            build(&Fixture {
                quads: vec![(iri("f"), iri("p"), iri("o"), None)],
                ..Fixture::default()
            })
        }),
    ]));
    let view =
        PagedDataset::from_provider(provider.clone() as Arc<dyn PageProvider>).expect("seal pages");
    let hits_after_seal = provider.hits();
    assert_eq!(
        hits_after_seal,
        view.page_count(),
        "the seal pass pulls each of the six pages once"
    );

    let engine = NativeSparqlEngine::new();
    let prepared = engine
        .prepare_query(
            &format!("SELECT ?g ?s WHERE {{ GRAPH ?g {{ ?s <{EX}p> ?o }} }} ORDER BY ?g"),
            None,
        )
        .expect("prepare");
    let (_vars, rows) = solutions(
        engine
            .query_prepared_view(&view, &prepared, &[], QueryOptions::EMPTY)
            .expect("paged graph-var query"),
    );
    assert_eq!(
        first_column(&rows),
        vec![iri("g0"), iri("g1"), iri("g2")],
        "each named graph answers from its own page"
    );

    // The measurement: three pages pulled, one per named graph. The three
    // default-graph-only pages belong to no named graph and were never materialized —
    // a graph-blind inner scan would have read all six.
    assert_eq!(
        provider.hits() - hits_after_seal,
        3,
        "only the three pages owning a named graph are materialized"
    );
}

/// Graph-scoped page eviction must not erase the graph it retains, as seen from SPARQL.
///
/// A graph declared empty owns no row in any stream, so a keep-set built only from "which
/// pages hold rows in `g`" is empty for it — and evicting on that answer deletes the graph
/// outright. `GRAPH ?g` enumerates declared-empty graphs, so the deletion is observable as
/// a lost solution on the production query surface, not merely as missing metadata.
#[test]
fn retaining_a_declared_empty_graph_keeps_it_bound_by_graph_var() {
    let pages = [
        page_with_data_and_declared_empties(),
        page_with_side_table_only_graph(),
        page_with_default_graph_only(),
    ];
    let view = paged(&pages);
    let engine = NativeSparqlEngine::new();
    let query = "SELECT ?g WHERE { GRAPH ?g { FILTER(true) } }";
    let prepared = engine.prepare_query(query, None).expect("prepare");

    let (_, rows) = solutions(
        engine
            .query_prepared_view(&view, &prepared, &[], QueryOptions::EMPTY)
            .expect("whole-dataset enumeration"),
    );
    let before = first_column(&rows);
    assert!(
        before.contains(&iri("gempty1")),
        "sanity: the whole dataset binds the declared-empty graph"
    );

    let gempty1 = view
        .term_id_by_value(&iri("gempty1"))
        .expect("gempty1 is interned");
    let retained = view.retain_graph(gempty1);

    let (_, rows) = solutions(
        engine
            .query_prepared_view(&retained, &prepared, &[], QueryOptions::EMPTY)
            .expect("enumeration after eviction"),
    );
    let after = first_column(&rows);
    assert!(
        after.contains(&iri("gempty1")),
        "retaining a declared-empty graph must leave it bound by GRAPH ?g, not delete it"
    );

    // The neighbouring valid case: eviction is still a real decision. `gside` owns rows
    // on a different page, so retaining it must NOT keep the declared-empty page.
    let gside = view
        .term_id_by_value(&iri("gside"))
        .expect("gside interned");
    let (_, rows) = solutions(
        engine
            .query_prepared_view(
                &view.retain_graph(gside),
                &prepared,
                &[],
                QueryOptions::EMPTY,
            )
            .expect("enumeration after retaining a graph that owns rows"),
    );
    let side_only = first_column(&rows);
    assert!(
        side_only.contains(&iri("gside")),
        "the retained graph is still bound"
    );
    assert!(
        !side_only.contains(&iri("gempty1")),
        "a graph whose only page was evicted is gone: retention is not 'keep everything'"
    );
}
