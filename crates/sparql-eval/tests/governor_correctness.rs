// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Differential correctness of the governed evaluator against an **ungoverned oracle**.
//!
//! # The one thing every test here does
//!
//! Run the same query over the same data twice — once ungoverned through
//! [`NativeSparqlEngine::query`], once governed under a budget — and check the governed
//! run's certificate against what the ungoverned run actually returned. The oracle is
//! never a hand-written expectation and never a second reading of the analysis: it is the
//! engine's own complete answer.
//!
//! That distinction is the whole point of this file. The prefix-monotonicity analysis and
//! the partial-lift channel can be — and were — exercised entirely by tests that assert
//! per-variant classifications written from the same mental model as the implementation,
//! and a systematically wrong analysis passes every one of them. Only an oracle can say
//! that `Certain` really means "every one of these rows is an answer".
//!
//! It works: writing these properties found a real unsoundness. `OPTIONAL` over a
//! truncated optional side was classified as an upper bound while emitting left rows
//! padded with unbound **in place of** the true pairings the cut had hidden — so the true
//! answers were missing from a result whose only licence is "a row absent from this is
//! definitively not an answer". See `binop::eval_left_join`.
//!
//! # Everything drives the public API
//!
//! No test here builds a [`GraphPattern`](purrdf_sparql_algebra::GraphPattern) by hand.
//! Queries are SPARQL text, parsed and planned by the production pipeline, because a plan
//! the pipeline cannot itself produce proves nothing about the surface a consumer reaches.
//!
//! # Where the other two properties live
//!
//! Two of this harness's properties need corpora that live outside this crate's
//! dependency graph, and the repository forbids adding a dependency to reach them:
//!
//! - `d0_governed_unbounded_is_byte_identical_to_ungoverned` walks the whole W3C SPARQL
//!   conformance corpus, which only `purrdf-sparql-conformance` can enumerate (it owns the
//!   suite and the manifest loader, and it *depends on this crate* — the edge cannot be
//!   reversed). It is
//!   `crates/sparql-conformance/tests/governor_correctness_corpus.rs`.
//! - `shacl_sparql_constraints_inherit_governors` drives SHACL-SPARQL, which lives in
//!   `purrdf-shapes` — again a crate that depends on this one. It is
//!   `crates/shapes/tests/governor_inheritance.rs`.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension, SparqlEngine, SparqlRequest,
    SparqlResult, TermId,
};
use purrdf_sparql_eval::{
    GovernedOutcome, NativeSparqlEngine, PartialAnswers, PartialSparqlResult, QueryGovernors,
};

/// The fixture namespace. PurRDF mints no vocabulary IRIs; these are test data.
const EX: &str = "http://example.org/";

// ---------------------------------------------------------------------------
// Generated datasets
// ---------------------------------------------------------------------------

/// A generated dataset shape.
///
/// Deliberately small. The properties here are about the *algebra* of partial answers,
/// and a five-subject graph exercises every operator's truncation behaviour while keeping
/// a whole-budget sweep (which re-runs the query once per candidate budget) affordable at
/// proptest's case counts.
#[derive(Debug, Clone, Copy)]
struct DataShape {
    /// How many `sN ex:p oN` edges exist.
    edges: usize,
    /// How many of those subjects also carry `sN ex:q zN` — the optional/subtracted side.
    optional_edges: usize,
    /// Whether `s0 ex:p o0` is asserted twice, so the answer is a genuine bag rather than
    /// a set and a multiset containment check can fail where a set one would not.
    duplicate_edge: bool,
    /// Whether the RDF 1.2 reification layer is populated: a reifier bound to the triple
    /// term `<<( sN ex:p oN )>>`, carrying an annotation. RDF 1.2 is a complete spec and
    /// the governor must certify partial answers over it exactly as it does over asserted
    /// triples, so it is generated data here rather than a separate special case.
    reified: bool,
    /// Whether the edges are also mirrored into a named graph, so `GRAPH ?g { … }` has
    /// something to bind.
    named_graph: bool,
}

fn data_shape() -> impl Strategy<Value = DataShape> {
    (
        1_usize..=5,
        0_usize..=4,
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(edges, optional_edges, duplicate_edge, reified, named_graph)| DataShape {
                edges,
                optional_edges: optional_edges.min(edges),
                duplicate_edge,
                reified,
                named_graph,
            },
        )
}

/// Freeze `shape` into a dataset.
fn build_dataset(shape: DataShape) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    let n = builder.intern_iri(&format!("{EX}n"));
    let confidence = builder.intern_iri(&format!("{EX}confidence"));
    let graph = builder.intern_iri(&format!("{EX}g"));
    if shape.named_graph {
        builder.declare_named_graph(graph);
    }

    let mut edge_ids: Vec<(TermId, TermId)> = Vec::with_capacity(shape.edges);
    for index in 0..shape.edges {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let o = builder.intern_iri(&format!("{EX}o{index}"));
        let count = builder.intern_literal(RdfLiteral::typed(
            index.to_string(),
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        builder.push_quad(s, p, o, None);
        builder.push_quad(s, n, count, None);
        if shape.named_graph {
            builder.push_quad(s, p, o, Some(graph));
        }
        if index < shape.optional_edges {
            let z = builder.intern_iri(&format!("{EX}z{index}"));
            builder.push_quad(s, q, z, None);
        }
        edge_ids.push((s, o));
    }
    if shape.duplicate_edge {
        let (s, o) = edge_ids[0];
        builder.push_quad(s, p, o, None);
    }
    if shape.reified {
        // RDF 1.2: each edge's triple term gets a reifier carrying one annotation. The
        // reified statements are NOT re-asserted as plain quads — they are already
        // asserted above — so `?r rdf:reifies <<( ?s ex:p ?o )>>` reads the side-tables.
        for (index, &(s, o)) in edge_ids.iter().enumerate() {
            let statement = builder.intern_triple(s, p, o);
            let reifier = builder.intern_iri(&format!("{EX}r{index}"));
            let level = builder.intern_literal(RdfLiteral::simple(if index % 2 == 0 {
                "high"
            } else {
                "low"
            }));
            builder.push_reifier(reifier, statement);
            builder.push_annotation(reifier, confidence, level);
        }
    }
    builder.freeze().expect("the generated dataset is valid")
}

// ---------------------------------------------------------------------------
// Generated queries
// ---------------------------------------------------------------------------

/// One generated WHERE body, named by the algebra shape it produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Body {
    /// A single BGP.
    Bgp,
    /// A two-pattern join, so the join's right arm is a truncation site.
    Join,
    /// `OPTIONAL` — the antitone-looking position whose padding is the fabrication hazard.
    Optional,
    /// `OPTIONAL` with an inline `FILTER`, which is the fork-per-worker join lane.
    OptionalFiltered,
    /// `MINUS` — the genuinely antitone position, and the only one that yields an upper
    /// bound.
    Minus,
    /// `MINUS` whose subtracted side is itself a `MINUS`, so the interval algebra has to
    /// compose antitone with antitone and land back on a lower bound.
    DoubleMinus,
    /// `UNION`, whose two arms lose positional fidelity on opposite sides.
    Union,
    /// A plain `FILTER`.
    Filter,
    /// `FILTER EXISTS` — an opaque edge reached through an expression.
    FilterExists,
    /// `FILTER NOT EXISTS` — the same opaque edge, and the reason one classification must
    /// cover both.
    FilterNotExists,
    /// `BIND`.
    Bind,
    /// `GRAPH ?g { … }`.
    Graph,
    /// RDF 1.2: a reifier bound to a triple term.
    Reifier,
    /// RDF 1.2: a reifier bound to a triple term, joined to its annotation.
    ReifierAnnotated,
    /// `VALUES` joined against the data.
    Values,
}

impl Body {
    /// This body's SPARQL text, and the variables it binds.
    fn render(self) -> &'static str {
        match self {
            Self::Bgp => "?s <http://example.org/p> ?o",
            Self::Join => "?s <http://example.org/p> ?o . ?s <http://example.org/n> ?n",
            Self::Optional => {
                "?s <http://example.org/p> ?o OPTIONAL { ?s <http://example.org/q> ?z }"
            }
            Self::OptionalFiltered => {
                "?s <http://example.org/p> ?o \
                 OPTIONAL { ?s <http://example.org/q> ?z FILTER(?z != ?o) }"
            }
            Self::Minus => "?s <http://example.org/p> ?o MINUS { ?s <http://example.org/q> ?z }",
            Self::DoubleMinus => {
                "?s <http://example.org/p> ?o \
                 MINUS { ?s <http://example.org/q> ?z \
                         MINUS { ?s <http://example.org/n> ?z } }"
            }
            Self::Union => {
                "{ ?s <http://example.org/p> ?o } UNION { ?s <http://example.org/q> ?o }"
            }
            Self::Filter => "?s <http://example.org/p> ?o FILTER(?o != <http://example.org/o0>)",
            Self::FilterExists => {
                "?s <http://example.org/p> ?o FILTER EXISTS { ?s <http://example.org/q> ?z }"
            }
            Self::FilterNotExists => {
                "?s <http://example.org/p> ?o FILTER NOT EXISTS { ?s <http://example.org/q> ?z }"
            }
            Self::Bind => "?s <http://example.org/p> ?o BIND(STR(?s) AS ?b)",
            Self::Graph => "GRAPH ?g { ?s <http://example.org/p> ?o }",
            Self::Reifier => {
                "?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
                 <<( ?s <http://example.org/p> ?o )>>"
            }
            Self::ReifierAnnotated => {
                "?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
                 <<( ?s <http://example.org/p> ?o )>> . \
                 ?r <http://example.org/confidence> ?c"
            }
            Self::Values => {
                "?s <http://example.org/p> ?o \
                 VALUES ?o { <http://example.org/o0> <http://example.org/o1> }"
            }
        }
    }

    /// Every body, so the strategy cannot silently fall behind this enum.
    const ALL: [Self; 15] = [
        Self::Bgp,
        Self::Join,
        Self::Optional,
        Self::OptionalFiltered,
        Self::Minus,
        Self::DoubleMinus,
        Self::Union,
        Self::Filter,
        Self::FilterExists,
        Self::FilterNotExists,
        Self::Bind,
        Self::Graph,
        Self::Reifier,
        Self::ReifierAnnotated,
        Self::Values,
    ];
}

/// A solution-modifier tail applied above the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modifier {
    /// No modifier.
    None,
    /// `DISTINCT`, which is prefix-monotone and must stay certifiable.
    Distinct,
    /// A bare `ORDER BY`, which keeps the multiset bound and loses the positional one.
    OrderBy,
    /// `LIMIT`, a restricting `Slice` that selects by position.
    Limit,
    /// `OFFSET` + `LIMIT`.
    OffsetLimit,
    /// `ORDER BY` **and** `LIMIT` — the top-*k* interaction with no certified lower bound
    /// beneath the sort.
    OrderedLimit,
    /// `GROUP BY` with an aggregate, the opaque edge.
    Group,
    /// A bare aggregate with no grouping key.
    CountAll,
}

impl Modifier {
    /// Every modifier, so the strategy cannot silently fall behind this enum.
    const ALL: [Self; 8] = [
        Self::None,
        Self::Distinct,
        Self::OrderBy,
        Self::Limit,
        Self::OffsetLimit,
        Self::OrderedLimit,
        Self::Group,
        Self::CountAll,
    ];
}

/// A generated query: a body, a modifier, and whether the projection is explicit.
#[derive(Debug, Clone, Copy)]
struct QueryShape {
    body: Body,
    modifier: Modifier,
}

impl QueryShape {
    /// This shape's SPARQL text.
    fn render(self) -> String {
        let body = self.body.render();
        match self.modifier {
            Modifier::None => format!("SELECT * WHERE {{ {body} }}"),
            Modifier::Distinct => format!("SELECT DISTINCT ?s WHERE {{ {body} }}"),
            Modifier::OrderBy => format!("SELECT * WHERE {{ {body} }} ORDER BY ?s"),
            Modifier::Limit => format!("SELECT * WHERE {{ {body} }} LIMIT 3"),
            Modifier::OffsetLimit => format!("SELECT * WHERE {{ {body} }} OFFSET 1 LIMIT 2"),
            Modifier::OrderedLimit => format!("SELECT * WHERE {{ {body} }} ORDER BY ?s LIMIT 2"),
            Modifier::Group => {
                format!("SELECT ?s (COUNT(*) AS ?k) WHERE {{ {body} }} GROUP BY ?s")
            }
            Modifier::CountAll => format!("SELECT (COUNT(*) AS ?k) WHERE {{ {body} }}"),
        }
    }
}

fn query_shape() -> impl Strategy<Value = QueryShape> {
    (
        proptest::sample::select(&Body::ALL[..]),
        proptest::sample::select(&Modifier::ALL[..]),
    )
        .prop_map(|(body, modifier)| QueryShape { body, modifier })
}

// ---------------------------------------------------------------------------
// Generated budgets
// ---------------------------------------------------------------------------

/// Which ceiling a generated budget engages.
///
/// Deliberately no deadline: a wall clock is not a property this machine can assert
/// anything about beyond the deadline governor's own latching, which
/// `governed_query.rs` covers with a scripted clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dimension {
    /// Abstract execution steps — the ceiling that can land anywhere in the plan.
    Fuel,
    /// The answer cap, which cuts at the root.
    AnswerRows,
    /// The intermediate-cell peak, which is observed once a bag exists.
    IntermediateCells,
}

impl Dimension {
    const ALL: [Self; 3] = [Self::Fuel, Self::AnswerRows, Self::IntermediateCells];

    /// The kernel dimension this engages.
    const fn resource(self) -> ResourceDimension {
        match self {
            Self::Fuel => ResourceDimension::Fuel,
            Self::AnswerRows => ResourceDimension::AnswerRows,
            Self::IntermediateCells => ResourceDimension::IntermediateCells,
        }
    }

    /// `QueryGovernors::UNBOUNDED` with this dimension capped at `ceiling`.
    fn governors(self, ceiling: u64) -> QueryGovernors {
        match self {
            Self::Fuel => QueryGovernors::UNBOUNDED.with_fuel(ceiling),
            Self::AnswerRows => QueryGovernors::UNBOUNDED.with_max_answers(ceiling),
            Self::IntermediateCells => {
                QueryGovernors::UNBOUNDED.with_max_intermediate_cells(ceiling)
            }
        }
    }
}

/// A generated budget: which dimension, and what fraction of the query's measured cost.
///
/// The fraction is generated rather than the ceiling itself, because an absolute number is
/// meaningless without knowing what the query costs — a fuel budget of 40 is "no work at
/// all" for one generated query and "the whole thing twice over" for another. The
/// harness measures the true cost with [`QueryGovernors::METERED`] and scales.
#[derive(Debug, Clone, Copy)]
struct Budget {
    dimension: Dimension,
    /// Numerator over 256.
    fraction: u8,
}

impl Budget {
    /// The ceiling this budget names for a query whose measured cost is `cost`.
    const fn ceiling(self, cost: u64) -> u64 {
        cost.saturating_mul(self.fraction as u64) / 256
    }
}

fn budget() -> impl Strategy<Value = Budget> {
    (proptest::sample::select(&Dimension::ALL[..]), any::<u8>()).prop_map(
        |(dimension, fraction)| Budget {
            dimension,
            fraction,
        },
    )
}

/// One generated trial: a dataset, a query, and a budget.
#[derive(Debug, Clone, Copy)]
struct Trial {
    data: DataShape,
    query: QueryShape,
    budget: Budget,
}

fn trial() -> impl Strategy<Value = Trial> {
    (data_shape(), query_shape(), budget()).prop_map(|(data, query, budget)| Trial {
        data,
        query,
        budget,
    })
}

// ---------------------------------------------------------------------------
// The oracle, and how a result is compared to it
// ---------------------------------------------------------------------------

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

/// The **ungoverned** answer: the ordinary query path, with no governor anywhere near it.
///
/// This — not a governed run under `UNBOUNDED` — is the oracle, because the claim being
/// checked is about what the engine returns when nobody is governing it.
fn oracle(dataset: &Arc<RdfDataset>, query: &str) -> Result<SparqlResult, String> {
    NativeSparqlEngine::new()
        .query(dataset, request(query))
        .map_err(|diagnostic| diagnostic.to_string())
}

/// The governed answer under `governors`.
fn governed(
    dataset: &Arc<RdfDataset>,
    query: &str,
    governors: &QueryGovernors,
) -> Result<GovernedOutcome, String> {
    NativeSparqlEngine::new()
        .query_governed(dataset, request(query), governors)
        .map_err(|diagnostic| diagnostic.to_string())
}

/// One solution row, canonicalized as its variable-to-value bindings.
///
/// Keyed by variable name rather than by column position on purpose: a partial result's
/// projection can legitimately carry its columns in a different order from the complete
/// one, and a positional comparison would report that as a wrong answer. Unbound cells are
/// rendered explicitly, because "bound to nothing" and "no such column" are different
/// facts and `OPTIONAL` is exactly where confusing them hides a bug.
type Row = Vec<(String, String)>;

/// The rows of a solutions result, canonicalized, in result order.
fn rows_of(result: &SparqlResult) -> Option<Vec<Row>> {
    match result {
        SparqlResult::Solutions {
            variables, rows, ..
        } => Some(
            rows.iter()
                .map(|row| {
                    let mut binding: Row = variables
                        .iter()
                        .zip(row.iter())
                        .map(|(variable, cell)| {
                            (
                                variable.clone(),
                                cell.as_ref().map_or_else(
                                    || "UNBOUND".to_owned(),
                                    |value| format!("{value:?}"),
                                ),
                            )
                        })
                        .collect();
                    binding.sort();
                    binding
                })
                .collect(),
        ),
        SparqlResult::Boolean(_) | SparqlResult::Graph(_) => None,
    }
}

/// `rows` as a multiset.
fn bag(rows: &[Row]) -> BTreeMap<Row, usize> {
    let mut counts: BTreeMap<Row, usize> = BTreeMap::new();
    for row in rows {
        *counts.entry(row.clone()).or_default() += 1;
    }
    counts
}

/// Whether every row of `part` appears in `whole` **at least as many times** — multiset
/// containment, which is the containment this algebra is defined over. A set-level check
/// would pass a partial result that invented a duplicate of a real answer.
fn bag_contains(whole: &[Row], part: &[Row]) -> Result<(), String> {
    let available = bag(whole);
    for (row, needed) in bag(part) {
        let have = available.get(&row).copied().unwrap_or(0);
        if have < needed {
            return Err(format!(
                "row {row:?} appears {needed}× in the partial answer but only {have}× in \
                 the ungoverned answer"
            ));
        }
    }
    Ok(())
}

/// The certified rows of a governed outcome, when it certifies a **lower** bound — either
/// because it completed (a complete answer is a tight lower bound on itself) or because
/// the certificate says [`PartialAnswers::Certain`].
fn certain_rows(outcome: &GovernedOutcome) -> Option<Vec<Row>> {
    match outcome {
        GovernedOutcome::Complete { result, .. } => rows_of(result),
        GovernedOutcome::BudgetExhausted(exhausted) => match &exhausted.partial {
            PartialAnswers::Certain(partial) => rows_of(partial.result()),
            PartialAnswers::AtMost(_) | PartialAnswers::Unknown(_) => None,
        },
    }
}

/// The certified partial of a governed outcome, when it is a lower bound *and* the
/// execution actually tripped — i.e. excluding the complete case.
fn certain_partial(outcome: &GovernedOutcome) -> Option<&PartialSparqlResult> {
    match outcome {
        GovernedOutcome::Complete { .. } => None,
        GovernedOutcome::BudgetExhausted(exhausted) => match &exhausted.partial {
            PartialAnswers::Certain(partial) => Some(partial),
            PartialAnswers::AtMost(_) | PartialAnswers::Unknown(_) => None,
        },
    }
}

/// The measured cost of `query` over `dataset` in `dimension`, with nothing bounded.
fn measured_cost(dataset: &Arc<RdfDataset>, query: &str, dimension: Dimension) -> Option<u64> {
    let metered = governed(dataset, query, &QueryGovernors::METERED).ok()?;
    match &metered {
        GovernedOutcome::Complete { evidence, .. } => {
            Some(evidence.consumed_in(dimension.resource()))
        }
        // METERED bounds nothing, so it cannot trip; a trip here would mean the metering
        // ceiling itself was reached, which no execution that fits in memory can do.
        GovernedOutcome::BudgetExhausted(_) => None,
    }
}

/// How many *generated* trials a property runs, on top of the deterministic grid below.
///
/// Above proptest's default, because the interesting region is narrow: a trial only says
/// anything when its budget lands strictly inside the query's cost. The coverage
/// assertions at the end of each property are what hold this number honest — though it is
/// [`grid_trials`], not this, that makes those assertions deterministic.
const CASES: u32 = 768;

/// Two fixed dataset shapes for the deterministic grid: one plain, one carrying the
/// RDF 1.2 reification layer and a named graph.
const GRID_SHAPES: [DataShape; 2] = [
    DataShape {
        edges: 4,
        optional_edges: 2,
        duplicate_edge: true,
        reified: false,
        named_graph: false,
    },
    DataShape {
        edges: 5,
        optional_edges: 3,
        duplicate_edge: false,
        reified: true,
        named_graph: true,
    },
];

/// The budget fractions the grid sweeps, from nothing to just under the full cost.
const GRID_FRACTIONS: [u8; 9] = [0, 32, 64, 96, 128, 160, 192, 224, 255];

/// Every combination of body, modifier, budget dimension and budget fraction over
/// [`GRID_SHAPES`].
///
/// # Why a grid *and* a random search
///
/// They answer different questions and neither substitutes for the other. The random
/// search is the falsifier: it reaches shapes nobody enumerated and it shrinks a failure
/// to something readable. But its *coverage* is a die roll, and a property whose
/// interesting case is rare — an upper bound needs a trip inside a subtracted arm
/// specifically — can go a whole run without reaching it once. A coverage assertion that
/// is itself random is a test that intermittently proves nothing and says so only
/// sometimes, which is worse than one that never proved anything at all.
///
/// So the grid runs first and is exhaustive over the enumerated space, which makes the
/// coverage counts at the end of each property deterministic; the random search then runs
/// on top of it for reach.
fn grid_trials() -> Vec<Trial> {
    let mut trials = Vec::new();
    for data in GRID_SHAPES {
        for body in Body::ALL {
            for modifier in Modifier::ALL {
                for dimension in Dimension::ALL {
                    for fraction in GRID_FRACTIONS {
                        trials.push(Trial {
                            data,
                            query: QueryShape { body, modifier },
                            budget: Budget {
                                dimension,
                                fraction,
                            },
                        });
                    }
                }
            }
        }
    }
    trials
}

/// Run `check` over the deterministic grid and then over generated trials, failing the
/// test on the first counterexample (proptest shrinks the generated ones).
fn for_each_trial(check: impl Fn(Trial) -> Result<(), TestCaseError>) {
    for input in grid_trials() {
        if let Err(error) = check(input) {
            panic!("enumerated trial {input:?} failed: {error}");
        }
    }
    let mut runner = TestRunner::new(Config {
        cases: CASES,
        failure_persistence: None,
        ..Config::default()
    });
    runner
        .run(&trial(), check)
        .unwrap_or_else(|error| panic!("{error}"));
}

// ---------------------------------------------------------------------------
// 1. The lower bound
// ---------------------------------------------------------------------------

#[test]
fn certain_answers_are_always_a_subset_of_the_ungoverned_answers() {
    // The claim `PartialAnswers::Certain` makes, checked against the only thing that can
    // falsify it: every row a governed run certifies must appear — with at least that
    // multiplicity — in the answer the ungoverned run returned for the same query over
    // the same data.
    let certified = AtomicUsize::new(0);
    let nonempty = AtomicUsize::new(0);

    for_each_trial(|trial| {
        let dataset = build_dataset(trial.data);
        let query = trial.query.render();
        let Ok(truth) = oracle(&dataset, &query) else {
            // A query the ungoverned engine rejects has no answer to bound. The governed
            // run must reject it too, which `d6_precedence_order` in `governed_query.rs`
            // pins; here there is simply nothing to compare.
            return Ok(());
        };
        let Some(truth_rows) = rows_of(&truth) else {
            return Ok(());
        };
        let Some(cost) = measured_cost(&dataset, &query, trial.budget.dimension) else {
            return Ok(());
        };
        let governors = trial.budget.dimension.governors(trial.budget.ceiling(cost));
        let outcome = governed(&dataset, &query, &governors)
            .map_err(|e| TestCaseError::fail(format!("governed run failed: {e}")))?;

        let Some(partial) = certain_partial(&outcome) else {
            return Ok(());
        };
        let Some(partial_rows) = rows_of(partial.result()) else {
            return Ok(());
        };
        certified.fetch_add(1, Ordering::Relaxed);
        if !partial_rows.is_empty() {
            nonempty.fetch_add(1, Ordering::Relaxed);
        }
        bag_contains(&truth_rows, &partial_rows).map_err(|why| {
            TestCaseError::fail(format!(
                "certified partial answer is not contained in the ungoverned answer\n\
                 query:   {query}\n\
                 data:    {:?}\n\
                 budget:  {:?} at {}/256 of {cost}\n\
                 {why}",
                trial.data, trial.budget.dimension, trial.budget.fraction
            ))
        })?;
        Ok(())
    });

    assert!(
        certified.load(Ordering::Relaxed) > 0,
        "no generated trial produced a certified lower bound, so this property proved \
         nothing"
    );
    assert!(
        nonempty.load(Ordering::Relaxed) > 0,
        "every certified lower bound was empty; an empty bag is contained in anything, so \
         the containment check never ran on a row"
    );
}

// ---------------------------------------------------------------------------
// 2. The upper bound
// ---------------------------------------------------------------------------

#[test]
fn possible_answers_always_contain_the_ungoverned_answers() {
    // The dual claim, and the harder one. `AtMost` licenses exactly one reading — "a row
    // absent from this result is definitively not an answer" — so it is falsified by a
    // single true answer the result omits.
    //
    // This property is what caught the `OPTIONAL` unsoundness. Truncating the optional
    // side was classified antitone, which says the output GROWS; in fact the padded rows
    // it emitted REPLACED the true pairings the cut had hidden, so the true answers were
    // missing and the licence was false. The classification is now bag-monotone (see
    // `soundness::visit_pattern_parts`'s `LeftJoin` arm), and the operator suppresses the
    // padding, which is why this property no longer sees `OPTIONAL` at all: `MINUS` — the
    // one position that really does subtract less — is where the upper bound now lives.
    let bounded = AtomicUsize::new(0);

    for_each_trial(|trial| {
        let dataset = build_dataset(trial.data);
        let query = trial.query.render();
        let Ok(truth) = oracle(&dataset, &query) else {
            return Ok(());
        };
        let Some(truth_rows) = rows_of(&truth) else {
            return Ok(());
        };
        let Some(cost) = measured_cost(&dataset, &query, trial.budget.dimension) else {
            return Ok(());
        };
        let governors = trial.budget.dimension.governors(trial.budget.ceiling(cost));
        let outcome = governed(&dataset, &query, &governors)
            .map_err(|e| TestCaseError::fail(format!("governed run failed: {e}")))?;

        let GovernedOutcome::BudgetExhausted(exhausted) = &outcome else {
            return Ok(());
        };
        let PartialAnswers::AtMost(partial) = &exhausted.partial else {
            return Ok(());
        };
        let Some(partial_rows) = rows_of(partial.result()) else {
            return Ok(());
        };
        bounded.fetch_add(1, Ordering::Relaxed);
        assert!(
            !partial.is_positional_prefix(),
            "an upper bound is not a prefix of the answer, so it can never claim to be \
             the answer's first rows"
        );
        bag_contains(&partial_rows, &truth_rows).map_err(|why| {
            TestCaseError::fail(format!(
                "the ungoverned answer is NOT contained in the upper bound, so a row \
                 absent from it is not 'definitively not an answer'\n\
                 query:   {query}\n\
                 data:    {:?}\n\
                 budget:  {:?} at {}/256 of {cost}\n\
                 {why}",
                trial.data, trial.budget.dimension, trial.budget.fraction
            ))
        })?;
        Ok(())
    });

    // Non-vacuity has to be checked here rather than trusted, because the generator's
    // reach over the antitone positions is exactly what this property depends on: a
    // change that stopped producing `AtMost` at all would otherwise turn this test green
    // by proving nothing.
    assert!(
        bounded.load(Ordering::Relaxed) > 0,
        "no generated trial produced an upper bound, so this property proved nothing; \
         MINUS with a trip in its subtracted arm is what reaches it"
    );
}

// ---------------------------------------------------------------------------
// 4. Budget monotonicity
// ---------------------------------------------------------------------------

#[test]
fn a_larger_budget_never_yields_fewer_certain_answers() {
    // Raising a ceiling may not take an answer away. Without this the certificate would
    // still be sound row-by-row and useless in practice: a caller who raised the budget
    // and got a smaller answer could not tell a governor from a bug.
    //
    // The comparison runs against the ungoverned oracle too, at the top of the ladder:
    // the largest budget is the query's full measured cost, so the last rung is the
    // complete answer and every rung below is checked into it.
    let compared = AtomicUsize::new(0);

    for_each_trial(|trial| {
        let dataset = build_dataset(trial.data);
        let query = trial.query.render();
        if oracle(&dataset, &query).is_err() {
            return Ok(());
        }
        let dimension = trial.budget.dimension;
        let Some(cost) = measured_cost(&dataset, &query, dimension) else {
            return Ok(());
        };

        // A ladder of ceilings from nothing to the full measured cost. Every adjacent
        // pair is a (smaller, larger) budget, and the property is checked on all of them
        // rather than on one generated pair, because the interesting rungs are the ones
        // where the trip point moves from one operator to another.
        let rungs: Vec<u64> = (0..=8).map(|step| cost * step / 8).collect();
        let mut previous: Option<(u64, Vec<Row>)> = None;
        for ceiling in rungs {
            let outcome = governed(&dataset, &query, &dimension.governors(ceiling))
                .map_err(|e| TestCaseError::fail(format!("governed run failed: {e}")))?;
            let Some(rows) = certain_rows(&outcome) else {
                // A rung that certifies no lower bound (an upper bound, or a withheld
                // one) says nothing about the lower bound's growth, and the ladder
                // continues past it: the NEXT rung is still compared against the last
                // rung that did certify one.
                continue;
            };
            if let Some((smaller, before)) = &previous {
                compared.fetch_add(1, Ordering::Relaxed);
                bag_contains(&rows, before).map_err(|why| {
                    TestCaseError::fail(format!(
                        "raising the ceiling from {smaller} to {ceiling} LOST a certified \
                         answer\n\
                         query: {query}\n\
                         data:  {:?}\n\
                         dim:   {dimension:?}\n\
                         at {smaller}: {before:?}\n\
                         at {ceiling}: {rows:?}\n\
                         {why}",
                        trial.data
                    ))
                })?;
            }
            previous = Some((ceiling, rows));
        }
        Ok(())
    });

    assert!(
        compared.load(Ordering::Relaxed) > 0,
        "no generated trial produced two comparable rungs, so this property proved nothing"
    );
}

// ---------------------------------------------------------------------------
// 5. Prefix stability — the resumption contract
// ---------------------------------------------------------------------------

#[test]
fn certified_prefix_is_stable_across_budget_increases() {
    // `PartialSparqlResult::is_positional_prefix` is the only thing that makes a governed
    // query resumable: it promises that re-running under a larger budget returns THESE
    // rows, in THIS order, first. Multiset containment is not that promise — a bag-only
    // lower bound satisfies containment while returning its rows somewhere else entirely
    // — so this property checks positions, and only where the certificate claims them.
    let compared = AtomicUsize::new(0);
    let against_complete = AtomicUsize::new(0);

    for_each_trial(|trial| {
        let dataset = build_dataset(trial.data);
        let query = trial.query.render();
        let Ok(truth) = oracle(&dataset, &query) else {
            return Ok(());
        };
        let Some(truth_rows) = rows_of(&truth) else {
            return Ok(());
        };
        let dimension = trial.budget.dimension;
        let Some(cost) = measured_cost(&dataset, &query, dimension) else {
            return Ok(());
        };

        let mut previous: Option<(u64, Vec<Row>)> = None;
        for step in 0..=8_u64 {
            let ceiling = cost * step / 8;
            let outcome = governed(&dataset, &query, &dimension.governors(ceiling))
                .map_err(|e| TestCaseError::fail(format!("governed run failed: {e}")))?;
            let Some(partial) = certain_partial(&outcome) else {
                continue;
            };
            if !partial.is_positional_prefix() {
                // The certificate declines the positional claim here, so there is nothing
                // to hold stable. The rung is dropped from the chain rather than compared
                // loosely — a property that silently weakened itself on the awkward cases
                // would be the failure this file exists to prevent.
                previous = None;
                continue;
            }
            let Some(rows) = rows_of(partial.result()) else {
                continue;
            };

            // Against the ungoverned answer, which is the budget increase taken to its
            // limit: a positional prefix must be a prefix of the true output, in order.
            against_complete.fetch_add(1, Ordering::Relaxed);
            if !truth_rows.starts_with(&rows) {
                return Err(TestCaseError::fail(format!(
                    "a certified POSITIONAL prefix is not a prefix of the ungoverned \
                     answer\n\
                     query:  {query}\n\
                     data:   {:?}\n\
                     dim:    {dimension:?} at {ceiling}\n\
                     partial: {rows:?}\n\
                     truth:   {truth_rows:?}",
                    trial.data
                )));
            }

            if let Some((smaller, before)) = &previous {
                compared.fetch_add(1, Ordering::Relaxed);
                if !rows.starts_with(before) {
                    return Err(TestCaseError::fail(format!(
                        "the rows certified at {smaller} are not a positional prefix of \
                         those certified at {ceiling}, so a caller cannot page through \
                         this query by raising the ceiling\n\
                         query: {query}\n\
                         data:  {:?}\n\
                         at {smaller}: {before:?}\n\
                         at {ceiling}: {rows:?}",
                        trial.data
                    )));
                }
            }
            previous = Some((ceiling, rows));
        }
        Ok(())
    });

    assert!(
        against_complete.load(Ordering::Relaxed) > 0,
        "no generated trial certified a positional prefix, so this property proved nothing"
    );
    assert!(
        compared.load(Ordering::Relaxed) > 0,
        "no generated trial certified a positional prefix at two different budgets, so \
         the stability half of this property proved nothing"
    );
}

// ---------------------------------------------------------------------------
// 7. The fork-per-worker lane's meter
// ---------------------------------------------------------------------------

/// A chain wide enough that the row loops really run on rayon.
///
/// `crate::parallel::PARALLEL_MIN_ROWS` gates the fork, and `chunk_size_for` derives the
/// chunk geometry from `rayon::current_num_threads()`, so an input this size is split
/// into a genuinely different number of differently-sized chunks by each pool below. A
/// test that quietly ran the sequential branch would prove nothing about the fork.
const PARALLEL_ROWS: usize = 1_500;

/// `PARALLEL_ROWS` subjects on `ex:p`, half of them also on `ex:q`.
fn wide_dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri(&format!("{EX}p"));
    let q = builder.intern_iri(&format!("{EX}q"));
    for index in 0..PARALLEL_ROWS {
        let s = builder.intern_iri(&format!("{EX}s{index}"));
        let o = builder.intern_iri(&format!("{EX}o{index}"));
        builder.push_quad(s, p, o, None);
        if index % 2 == 0 {
            builder.push_quad(s, q, o, None);
        }
    }
    builder.freeze().expect("the wide dataset is valid")
}

/// What one run of a governed query observed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Observation {
    /// Fuel the run reported spending.
    fuel: u64,
    /// The governor that stopped it, rendered.
    tripped: Option<String>,
    /// What the rows in hand claimed, and what they were.
    answer: String,
}

fn observe(dataset: &Arc<RdfDataset>, query: &str, governors: &QueryGovernors) -> Observation {
    let outcome = governed(dataset, query, governors).expect("governed run");
    let answer = match &outcome {
        GovernedOutcome::Complete { result, .. } => {
            format!("complete {:?}", rows_of(result))
        }
        GovernedOutcome::BudgetExhausted(exhausted) => match &exhausted.partial {
            PartialAnswers::Certain(partial) => {
                format!("certain {:?}", rows_of(partial.result()))
            }
            PartialAnswers::AtMost(partial) => format!("at-most {:?}", rows_of(partial.result())),
            PartialAnswers::Unknown(barrier) => format!("withheld at {barrier}"),
        },
    };
    Observation {
        fuel: outcome.evidence().consumed_in(ResourceDimension::Fuel),
        tripped: outcome.tripped().map(|governor| format!("{governor:?}")),
        answer,
    }
}

#[test]
fn filter_exists_fuel_is_invariant_under_worker_count() {
    // The fork-per-worker lane, which is the one with NO ordered per-item ledger. Its
    // workers share one `Arc<GovernorState>` and charge it directly, so anything they
    // charge lands in atomics whose total is a function of the chunk geometry — and the
    // chunk geometry is derived from `rayon::current_num_threads()`.
    //
    // `FILTER EXISTS` is the only expression that can charge from inside such a worker:
    // ordinary expression evaluation spends nothing (the row's whole cost is charged
    // before the loop, on the main thread), while an expression-embedded `EXISTS` calls
    // back into whole-pattern evaluation. Each chunk forks its own child whose EXISTS memo
    // is a snapshot taken at fork time, so the inner pattern was being re-evaluated once
    // per chunk and the reported fuel scaled with the thread count: measured on this
    // fixture, one worker reported 13507 and eight reported 57036.
    //
    // That is now fixed at the source rather than papered over in the certificate: a
    // governed execution does not fork a row loop whose expression can re-enter
    // evaluation (`EvalCtx::may_fork_row_loop`). Every other governed expression keeps
    // full parallelism, because it cannot charge. So the assertion below is the strong
    // one — the reported fuel is EXACT, not merely bounded — and the trip and the
    // certified rows are exact with it.
    //
    // The sequential guard is deliberately not engaged: no forced-parallel or
    // forced-sequential override, and `PARALLEL_ROWS` is above the parallel threshold, so
    // each pool really does split the row loop differently.
    let dataset = wide_dataset();
    let filter_exists =
        format!("SELECT ?s WHERE {{ ?s <{EX}p> ?o . FILTER EXISTS {{ ?a <{EX}q> ?b }} }}");
    // The companion case: the same shape with no EXISTS, which keeps the fork. Both must
    // be exact, and holding them side by side is what shows the guarantee is not being
    // bought by refusing to parallelize anything.
    let plain_filter = format!("SELECT ?s WHERE {{ ?s <{EX}p> ?o . FILTER(?o != <{EX}o0>) }}");

    for query in [&filter_exists, &plain_filter] {
        let mut per_pool: Vec<(usize, Observation, Observation)> = Vec::new();
        for threads in [1_usize, 2, 3, 5, 8] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("building a fixed-size pool");
            let observed = pool.install(|| {
                assert_eq!(
                    rayon::current_num_threads(),
                    threads,
                    "the pool must actually be the size this iteration asked for"
                );
                // Metered: the reported cost of the whole query, which is the number a
                // caller uses to size the next budget. If this moves with the worker
                // count, every budget derived from it is machine-specific.
                let metered = observe(&dataset, query, &QueryGovernors::METERED);
                // And a budget set from it, aimed at half the measured cost, so the run
                // really trips and the trip point is observable too.
                let half = QueryGovernors::UNBOUNDED.with_fuel(metered.fuel / 2);
                let bounded = observe(&dataset, query, &half);
                (metered, bounded)
            });
            per_pool.push((threads, observed.0, observed.1));
        }

        let (_, first_metered, first_bounded) = &per_pool[0];
        assert!(
            first_metered.fuel > 0,
            "{query}: the fixture must cost something to measure"
        );
        assert!(
            first_bounded.tripped.is_some(),
            "{query}: the fixture must actually trip, or invariance is vacuous"
        );
        for (threads, metered, bounded) in &per_pool {
            assert_eq!(
                metered, first_metered,
                "{query}: the METERED cost moved at {threads} worker(s); a budget sized \
                 from a machine-specific measurement is not a budget"
            );
            assert_eq!(
                bounded, first_bounded,
                "{query}: the governed run moved at {threads} worker(s); the trip point \
                 and the certified rows must be a pure function of the query, the data, \
                 and the budget, never of the machine"
            );
        }
    }
}

#[test]
fn a_governed_union_reports_one_outcome_however_it_is_scheduled() {
    // `UNION` is the only operator that starts both of its arms at once, and both arms
    // are whole patterns, so both charge the shared `GovernorState` — from two threads.
    // Whichever rayon happened to schedule first drained the budget, and this exact query
    // under nine fuel produced SEVEN distinct outcomes across sixty runs of one process:
    // the certified answer was five rows or none, and the reported consumption was 10, 12,
    // 13 or 14. No certificate can repair that, because the two arms disagreed about how
    // much budget there was.
    //
    // Repetition is the only way to observe a race, and it is a sound way: the property
    // under test is that ONE outcome exists, so a second distinct observation falsifies it
    // outright. A run that happens not to race proves nothing and fails nothing, which is
    // why the sweep is over several budgets rather than one.
    let dataset = build_dataset(DataShape {
        edges: 5,
        optional_edges: 3,
        duplicate_edge: true,
        reified: false,
        named_graph: true,
    });
    let query = format!("SELECT * WHERE {{ {{ ?s <{EX}p> ?o }} UNION {{ ?s <{EX}q> ?o }} }}");

    for fuel in [4_u64, 9, 13, 17, 20, 25, 30] {
        let governors = QueryGovernors::UNBOUNDED.with_fuel(fuel);
        let first = observe(&dataset, &query, &governors);
        for attempt in 1..64 {
            assert_eq!(
                observe(&dataset, &query, &governors),
                first,
                "attempt {attempt} at {fuel} fuel disagreed with the first run: one query, \
                 one dataset and one budget must have exactly one outcome"
            );
        }
    }
}

#[test]
fn an_ungoverned_run_keeps_the_fork_it_always_had() {
    // The other half of the rule above, and the one that keeps it honest: refusing the
    // fork is conditional on the execution being GOVERNED. An ungoverned `FILTER EXISTS`
    // has no meter to be exact about, so it must still take the parallel path — otherwise
    // the exactness guarantee would have been bought by slowing down every query that
    // never asked for a governor.
    //
    // Observable without timing: the parallel path forks one child context per chunk, and
    // each child's EXISTS memo starts as a snapshot, so the inner pattern is evaluated
    // once per chunk. Under `METERED` — which IS governed — that count is one. The two
    // runs must therefore return the same ANSWER while only the governed one is obliged
    // to report a machine-independent cost.
    let dataset = wide_dataset();
    let query = format!("SELECT ?s WHERE {{ ?s <{EX}p> ?o . FILTER EXISTS {{ ?a <{EX}q> ?b }} }}");

    let ungoverned = oracle(&dataset, &query).expect("ungoverned run");
    let ungoverned_rows = rows_of(&ungoverned).expect("a SELECT returns solutions");
    assert_eq!(
        ungoverned_rows.len(),
        PARALLEL_ROWS,
        "the EXISTS holds for every row, so the filter admits all of them"
    );

    let metered = governed(&dataset, &query, &QueryGovernors::METERED).expect("metered run");
    let GovernedOutcome::Complete { result, .. } = &metered else {
        panic!("METERED bounds nothing and cannot trip: {metered:?}");
    };
    assert_eq!(
        rows_of(result).expect("a SELECT returns solutions"),
        ungoverned_rows,
        "governing a query must not change its answer"
    );
}
