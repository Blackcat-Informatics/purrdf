// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Accounting pins for the per-row `EXISTS` and aggregate paths.
//!
//! The row loops of `FILTER`, `BIND` and inline aggregates take shorter routes
//! when the workspace reports no storage budget. These cases pin what those
//! routes must not change: the answers, the smallest bounded account each
//! query completes under (with one byte less refusing), the governor totals a
//! metered run reports, what a host's own governors charge the sequential aggregate
//! route, the run of a shared plan twice, that each fast path engages on the shape it
//! claims, and that an `EXISTS` scope collision is refused with its variable and
//! construct once a row binds it, with the neighbouring case accepted.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, ResourceDimension};
use purrdf_sparql_algebra::{
    Child, Expression, GraphPattern, Literal, NamedNode, NamedNodePattern, SparqlParser,
    TermPattern, TriplePattern, Variable,
};

use crate::SolutionTerm;
use crate::error::EvalError;
use crate::eval::{EvalCtx, EvaluatedOutcome, Outcome, eval_evaluated, evaluate_query_evaluated};
use crate::modifier::resident_aggregate_value;
use crate::scratch::ScratchInterner;
use crate::workspace::owned_row_regressions::account;
use crate::{GovernorState, QueryGovernors};

const EX: &str = "http://example.org/";
const XSD_FLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";

/// Twelve subjects, each with a float score and one of five statuses.
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let score = b.intern_iri(&format!("{EX}score")).unwrap();
    let status = b.intern_iri(&format!("{EX}status")).unwrap();
    for i in 0..12_u64 {
        let s = b.intern_iri(&format!("{EX}s{i}")).unwrap();
        let v = i.wrapping_mul(2_654_435_761) % 1000;
        let lit = b
            .intern_literal(RdfLiteral::typed(format!("{v}.0"), XSD_FLOAT.to_owned()))
            .unwrap();
        b.push_quad(s, score, lit, None).unwrap();
        let st = b.intern_iri(&format!("{EX}st{}", i % 5)).unwrap();
        b.push_quad(s, status, st, None).unwrap();
    }
    b.freeze().unwrap()
}

fn query(body: &str) -> purrdf_sparql_algebra::Query {
    SparqlParser::new()
        .parse_query(&format!("PREFIX ex: <{EX}>\n{body}"))
        .unwrap()
}

/// `FILTER NOT EXISTS` over a one-pattern body, correlated on `?x`.
const NOT_EXISTS: &str =
    "SELECT ?x ?s WHERE { ?x ex:score ?s . FILTER NOT EXISTS { ?x ex:status ex:st0 } }";
/// `BIND(EXISTS ...)`: the answer is kept as a column.
const BIND_EXISTS: &str =
    "SELECT ?x ?h WHERE { ?x ex:score ?s . BIND(EXISTS { ?x ex:status ex:st0 } AS ?h) }";
/// Two sites in one expression, with different bodies.
const TWO_SITES: &str = "SELECT ?x WHERE { ?x ex:score ?s . \
     FILTER(NOT EXISTS { ?x ex:status ex:st0 } && EXISTS { ?x ex:status ex:st1 }) }";
/// Two distinct sites with the same text, in two branches that bind different columns. One
/// preparation meeting several outer schemas needs a shared plan, which the two-run test
/// below covers.
const BRANCH_SCHEMAS: &str = "SELECT ?x ?s ?st WHERE { \
     { ?x ex:score ?s . FILTER NOT EXISTS { ?x ex:status ex:st2 } } UNION \
     { ?x ex:status ?st . FILTER NOT EXISTS { ?x ex:status ex:st2 } } }";
/// OPTIONAL leaves the correlated `?st` unbound on some rows of one FILTER.
const OPTIONAL_ROWS: &str = "SELECT ?x ?st WHERE { ?x ex:score ?s . \
     OPTIONAL { ?x ex:status ?st . FILTER(?st = ex:st3) } \
     FILTER NOT EXISTS { ?y ex:status ?st . FILTER(?y != ?x) } }";
/// `COUNT` of one variable per group.
const GROUP_COUNT: &str = "SELECT ?st (COUNT(?x) AS ?n) WHERE { ?x ex:status ?st } GROUP BY ?st";
/// `NOT EXISTS`, `GROUP BY` and `HAVING` in one query.
const GROUP_NOT_EXISTS: &str = "SELECT ?st (COUNT(?x) AS ?n) WHERE { ?x ex:score ?s ; ex:status ?st . \
     FILTER NOT EXISTS { ?x ex:status ex:st0 } } GROUP BY ?st HAVING (COUNT(?x) > 1)";
/// `DISTINCT`, `SAMPLE` and `MIN` read their argument variable too.
const GROUP_VALUES: &str = "SELECT ?st (COUNT(DISTINCT ?x) AS ?n) (MIN(?s) AS ?low) (SAMPLE(?x) AS ?one) \
     WHERE { ?x ex:score ?s ; ex:status ?st } GROUP BY ?st";

const QUERIES: [&str; 8] = [
    NOT_EXISTS,
    BIND_EXISTS,
    TWO_SITES,
    BRANCH_SCHEMAS,
    OPTIONAL_ROWS,
    GROUP_COUNT,
    GROUP_NOT_EXISTS,
    GROUP_VALUES,
];

/// The answer a query reports — its variables and each row's cells, a computed cell
/// by its value — without the storage the rows happen to be kept in.
fn render(outcome: &EvaluatedOutcome, ctx: &EvalCtx<'_, RdfDataset>) -> String {
    let EvaluatedOutcome::Complete(Outcome::Solutions(solutions)) = outcome else {
        panic!("every query here completes with solutions: {outcome:?}");
    };
    use std::fmt::Write as _;
    let mut out = format!("{:?}", solutions.schema.vars());
    for row in &solutions.rows {
        out.push('\n');
        for cell in row {
            match cell {
                None => out.push_str("- "),
                Some(SolutionTerm::Existing(id)) => write!(out, "{id:?} ").unwrap(),
                Some(SolutionTerm::Computed(sid)) => {
                    write!(out, "{:?} ", ctx.scratch.computed_value(*sid)).unwrap();
                }
            }
        }
    }
    out
}

/// The rendered answer of an unbounded, ungoverned run.
fn answer(dataset: &RdfDataset, text: &str) -> String {
    let mut ctx = EvalCtx::new(dataset);
    let outcome = evaluate_query_evaluated(&query(text), &mut ctx).unwrap();
    render(&outcome, &ctx)
}

/// The rendered answer under a bounded account of `limit` bytes, or `None` when the
/// account refuses the query.
fn bounded(dataset: &RdfDataset, text: &str, limit: u64) -> Option<String> {
    let (workspace, _ledger, _) = account(limit);
    let mut ctx = EvalCtx::new(dataset);
    ctx.growth = workspace.capability();
    let outcome = evaluate_query_evaluated(&query(text), &mut ctx).ok()?;
    Some(render(&outcome, &ctx))
}

/// The consumption a metered run reports, per dimension, with its rendered answer.
fn metered(dataset: &RdfDataset, text: &str) -> ([u64; 4], String) {
    let state = Arc::new(GovernorState::new(&QueryGovernors::METERED));
    let mut ctx = EvalCtx::new(dataset).with_governors(Arc::clone(&state));
    let outcome = evaluate_query_evaluated(&query(text), &mut ctx).unwrap();
    let evidence = state.evidence();
    assert_eq!(evidence.tripped(), None, "a metered run never refuses");
    (
        [
            ResourceDimension::Fuel,
            ResourceDimension::ScratchBytes,
            ResourceDimension::IntermediateCells,
            ResourceDimension::AnswerRows,
        ]
        .map(|dimension| evidence.consumed_in(dimension)),
        render(&outcome, &ctx),
    )
}

/// The smallest account each query completes under: a property of the bounded
/// (admitted) path, which the shorter unbounded routes leave untouched.
const BOUNDED_MINIMUM: [u64; 8] = [25147, 26499, 32349, 34191, 59323, 18598, 30641, 33902];

/// What a metered run of each query reports: fuel, scratch bytes, intermediate
/// cells and answer rows.
const METERED_TOTALS: [[u64; 4]; 8] = [
    [65, 153, 24, 9],
    [71, 153, 36, 12],
    [61, 153, 24, 3],
    [150, 153, 60, 20],
    [694, 153, 36, 0],
    [65, 784, 24, 5],
    [121, 1328, 36, 4],
    [133, 2318, 36, 5],
];

#[test]
fn bounded_accounts_complete_at_the_pinned_minimum_and_refuse_one_byte_below() {
    let dataset = dataset();
    for (text, minimum) in QUERIES.into_iter().zip(BOUNDED_MINIMUM) {
        let expected = answer(&dataset, text);
        assert_eq!(
            bounded(&dataset, text, minimum).as_deref(),
            Some(expected.as_str()),
            "{text}: completes with the unbounded answer at {minimum} bytes"
        );
        assert_eq!(
            bounded(&dataset, text, minimum - 1),
            None,
            "{text}: refused one byte below"
        );
    }
}

#[test]
fn metered_runs_report_the_pinned_totals_and_the_unbounded_answer() {
    let dataset = dataset();
    for (text, totals) in QUERIES.into_iter().zip(METERED_TOTALS) {
        let (consumed, rendered) = metered(&dataset, text);
        assert_eq!(consumed, totals, "{text}: metered totals");
        assert_eq!(rendered, answer(&dataset, text), "{text}: metered answer");
    }
}

/// A stop signal that counts its polls and fires, and stays fired, from poll `fire_at`.
#[derive(Debug)]
struct StopFrom {
    polls: std::sync::atomic::AtomicU64,
    fire_at: u64,
}

impl crate::StopSignal for StopFrom {
    fn poll(&self) -> Option<purrdf_core::StopCause> {
        let seen = self
            .polls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        (seen >= self.fire_at).then_some(purrdf_core::StopCause::Cancelled)
    }
}

/// What a host's governors report for one run: the answer when it completes, the fuel,
/// scratch, intermediate cells and answer rows consumed, and the stop polls made.
struct Host {
    rendered: Option<String>,
    totals: [u64; 4],
    polls: u64,
}

/// Run `text` under the governors of a host that sets a fuel, scratch, cell and answer
/// ceiling and a stop signal that fires from poll `fire_at`, as a SHACL host does. A
/// ceiling on cells keeps GROUP BY on its sequential, suspendable aggregate route.
fn host_run(dataset: &RdfDataset, text: &str, scratch: u64, fire_at: u64) -> Host {
    let stop = Arc::new(StopFrom {
        polls: std::sync::atomic::AtomicU64::new(0),
        fire_at,
    });
    let governors = QueryGovernors::UNBOUNDED
        .with_fuel(1 << 40)
        .with_max_scratch_bytes(scratch)
        .with_max_intermediate_cells(1 << 40)
        .with_max_answers(1 << 40)
        .with_stop_signal(Arc::clone(&stop) as Arc<dyn crate::StopSignal>);
    let state = Arc::new(GovernorState::new(&governors));
    let mut ctx = EvalCtx::new(dataset).with_governors(Arc::clone(&state));
    let outcome = evaluate_query_evaluated(&query(text), &mut ctx).ok();
    let evidence = state.evidence();
    let rendered = outcome
        .filter(|_| evidence.tripped().is_none())
        .map(|outcome| render(&outcome, &ctx));
    Host {
        rendered,
        totals: [
            ResourceDimension::Fuel,
            ResourceDimension::ScratchBytes,
            ResourceDimension::IntermediateCells,
            ResourceDimension::AnswerRows,
        ]
        .map(|dimension| evidence.consumed_in(dimension)),
        polls: stop.polls.load(std::sync::atomic::Ordering::Relaxed),
    }
}

/// What a host-governed run of each query consumes: fuel, scratch bytes, intermediate
/// cells and answer rows.
const HOST_TOTALS: [[u64; 4]; 8] = [
    [65, 153, 24, 9],
    [71, 153, 36, 12],
    [61, 153, 24, 3],
    [150, 153, 60, 20],
    [694, 153, 36, 0],
    [65, 784, 24, 5],
    [121, 1328, 36, 4],
    [133, 2318, 36, 5],
];

/// The stop polls a host-governed run of each query makes.
const HOST_POLLS: [u64; 8] = [23, 23, 28, 44, 292, 20, 41, 36];

/// **A host-governed run is charged, polled and stopped at its pinned values.** Under a
/// host's ceilings GROUP BY takes its sequential route, so each query must consume the
/// pinned fuel, scratch, cells and rows, make the pinned number of stop polls, complete
/// at its own scratch and refuse one byte below, stop when the signal fires on a poll it
/// makes, and complete when the signal fires only after its last poll.
#[test]
fn host_governed_runs_report_the_pinned_totals_polls_and_scratch_boundary() {
    let dataset = dataset();
    for (index, text) in QUERIES.into_iter().enumerate() {
        let expected = answer(&dataset, text);
        let run = host_run(&dataset, text, 1 << 40, u64::MAX);
        assert_eq!(
            run.rendered.as_deref(),
            Some(expected.as_str()),
            "{text}: answer"
        );
        assert_eq!(
            run.totals, HOST_TOTALS[index],
            "{text}: fuel, scratch, cells, rows"
        );
        assert_eq!(run.polls, HOST_POLLS[index], "{text}: stop polls");
        // The scratch ceiling: complete at what was consumed, refused one byte below.
        let scratch = run.totals[1];
        assert_eq!(
            host_run(&dataset, text, scratch, u64::MAX)
                .rendered
                .as_deref(),
            Some(expected.as_str()),
            "{text}: completes at {scratch} bytes of scratch"
        );
        if scratch > 0 {
            assert_eq!(
                host_run(&dataset, text, scratch - 1, u64::MAX).rendered,
                None,
                "{text}: refused one byte below"
            );
        }
        // The stop signal: observed on the first poll and part-way through, the run is
        // stopped; fired only after its last poll, it completes with the same answer.
        for fire_at in [1, run.polls.div_ceil(2)] {
            assert_eq!(
                host_run(&dataset, text, 1 << 40, fire_at).rendered,
                None,
                "{text}: stopped at poll {fire_at} of {}",
                run.polls
            );
        }
        assert_eq!(
            host_run(&dataset, text, 1 << 40, run.polls + 1)
                .rendered
                .as_deref(),
            Some(expected.as_str()),
            "{text}: completes when the stop fires after its last poll"
        );
    }
}

/// Aggregate shapes whose value path differs: `SUM` over IRIs (an error, so an unbound
/// result), `AVG` and `SUM` of floats, `GROUP_CONCAT` and an unbound argument.
const ROUTE_QUERIES: [&str; 4] = [
    "SELECT ?st (SUM(?x) AS ?t) WHERE { ?x ex:status ?st } GROUP BY ?st",
    "SELECT ?st (AVG(?s) AS ?a) (SUM(?s) AS ?t) WHERE { ?x ex:score ?s ; ex:status ?st } GROUP BY ?st",
    "SELECT ?st (GROUP_CONCAT(?s; SEPARATOR=\",\") AS ?g) WHERE { ?x ex:score ?s ; ex:status ?st } GROUP BY ?st",
    "SELECT ?x (COUNT(?o) AS ?n) WHERE { ?x ex:score ?s OPTIONAL { ?x ex:status ?o FILTER(?o = ex:st1) } } GROUP BY ?x",
];

/// **Each aggregate route yields the accounted run's answer and one scratch charge.**
/// The inline route, the sequential route a cell ceiling forces and a bounded account
/// must agree on these value shapes, where an error, a float sum, a concatenation or an
/// unbound argument reaches the value copy.
#[test]
fn the_sequential_and_inline_aggregate_routes_answer_as_the_accounted_run() {
    let dataset = dataset();
    for text in ROUTE_QUERIES {
        let inline = answer(&dataset, text);
        let accounted = bounded(&dataset, text, 1 << 40);
        let sequential = host_run(&dataset, text, 1 << 40, u64::MAX);
        assert_eq!(
            accounted.as_deref(),
            Some(inline.as_str()),
            "{text}: accounted run"
        );
        assert_eq!(
            sequential.rendered.as_deref(),
            Some(inline.as_str()),
            "{text}: sequential route"
        );
        // The scratch each route charges is one number.
        assert_eq!(
            metered(&dataset, text).0[1],
            sequential.totals[1],
            "{text}: scratch charged"
        );
    }
}

/// Hand-built algebra the parser would refuse: `FILTER NOT EXISTS { BIND("x" AS ?o) }`
/// over rows of which only the last binds `?o`.
fn collision_pattern(with_binding_row: bool) -> GraphPattern {
    let triple = |object: TermPattern| GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(Variable::new("x")),
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(format!("{EX}score"))),
            object,
        }],
    };
    let unbound = triple(TermPattern::Variable(Variable::new("w")));
    let inner = if with_binding_row {
        GraphPattern::union(unbound, triple(TermPattern::Variable(Variable::new("o"))))
    } else {
        unbound
    };
    let body = GraphPattern::Extend {
        inner: Child::new(GraphPattern::Bgp {
            patterns: Vec::new(),
        }),
        variable: Variable::new("o"),
        expression: Expression::Literal(Literal::new_simple("x")),
    };
    GraphPattern::Filter {
        expr: Expression::Not(Child::new(Expression::Exists(Child::new(body)))),
        inner: Child::new(inner),
    }
}

#[test]
fn exists_scope_collision_is_refused_once_a_row_binds_the_variable() {
    let dataset = dataset();
    // Twelve rows leave `?o` unbound; every one of them is answered.
    let mut ctx = EvalCtx::new(dataset.as_ref());
    let kept = eval_evaluated(&collision_pattern(false), &mut ctx).unwrap();
    assert_eq!(
        kept.rows().len(),
        0,
        "BIND always yields a row, so NOT EXISTS keeps none"
    );
    // The same twelve rows, then twelve that bind `?o`: the first of those refuses.
    let mut ctx = EvalCtx::new(dataset.as_ref());
    match eval_evaluated(&collision_pattern(true), &mut ctx) {
        Err(EvalError::ExistsScopeCollision { variable, intro }) => {
            assert_eq!(variable.as_str(), "o");
            assert_eq!(intro, "BIND target");
        }
        other => panic!("expected the scope collision, got {other:?}"),
    }
}

/// A view whose value reads are refused by the allocator, as an exhausted one would.
struct RefusingReads;

impl purrdf_core::DatasetView for RefusingReads {
    type Id = purrdf_core::TermId;
    type ReadError = std::convert::Infallible;
    type TermGuard<'a> = purrdf_core::TermRef<'a>;
    type ProbePlan = ();
    fn quads(&self) -> impl Iterator<Item = purrdf_core::QuadIds> + '_ {
        std::iter::empty()
    }
    fn resolve(&self, _: purrdf_core::TermId) -> Result<Self::TermGuard<'_>, Self::ReadError> {
        Ok(purrdf_core::TermRef::Iri("http://example.org/original"))
    }
    fn term_id_by_value(
        &self,
        _: &purrdf_core::TermValue,
    ) -> Result<Option<purrdf_core::TermId>, Self::ReadError> {
        Ok(None)
    }
    fn capabilities(&self) -> purrdf_core::RdfStoreCapabilities {
        purrdf_core::RdfStoreCapabilities::default()
    }
    fn term_count(&self) -> u64 {
        1
    }
    fn probe_plan(&self, _: bool, _: bool, _: bool, _: purrdf_core::GraphMatch) {}
    fn quads_for_pattern_with_plan(
        &self,
        (): &(),
        _: Option<purrdf_core::TermId>,
        _: Option<purrdf_core::TermId>,
        _: Option<purrdf_core::TermId>,
        _: purrdf_core::GraphMatch,
    ) -> impl Iterator<Item = purrdf_core::QuadIds> + '_ {
        std::iter::empty()
    }
    fn term_value_with_storage<E>(
        &self,
        _: purrdf_core::TermId,
        _: impl Fn(purrdf_core::TermLookupError<Self::ReadError>) -> E,
        storage_error: impl Fn(std::collections::TryReserveError) -> E,
    ) -> Result<purrdf_core::TermValue, E> {
        Err(storage_error(
            Vec::<u8>::new()
                .try_reserve(usize::MAX)
                .expect_err("a request past the address space is refused"),
        ))
    }
}

/// A resident aggregate run copies each survivor without the admission frame, and a
/// refused read buffer is the typed error the admitted route reports, never a panic.
/// The neighbouring readable value is copied unchanged.
#[test]
fn a_refused_resident_read_is_the_typed_allocation_error() {
    let mut builder = RdfDatasetBuilder::new();
    let id = builder.intern_iri("http://example.org/original").unwrap();
    let refused = resident_aggregate_value(
        &RefusingReads,
        &ScratchInterner::default(),
        SolutionTerm::Existing(id),
        |error| match error {},
    )
    .expect_err("the refused buffer is an error");
    assert!(matches!(
        refused,
        EvalError::AllocationFailed {
            construct: "resolved term value"
        }
    ));

    let dataset = builder.freeze().unwrap();
    let (value, _) = resident_aggregate_value(
        dataset.as_ref(),
        &ScratchInterner::default(),
        SolutionTerm::Existing(id),
        EvalError::source_read,
    )
    .expect("a readable value is copied")
    .into_parts();
    assert_eq!(
        value,
        purrdf_core::TermValue::Iri("http://example.org/original".to_owned())
    );
}

/// A site whose child is a per-row definition is decided once per row, with one extra
/// decision at most: the request for an answer in place that learns it needs a child.
/// Rows alternate between a restriction already answered (memoized) and a fresh one, so
/// a flag cleared by a memoized answer would ask in place again on the next fresh row
/// and decide it twice. The neighbouring probe site answers the same rows within the
/// same bound.
#[test]
fn a_definition_site_is_not_decided_again_in_place_after_a_memoized_answer() {
    let dataset = dataset();
    let rows = 10_u64;
    let text = "SELECT ?st WHERE { \
                VALUES ?st { ex:st0 ex:st1 ex:st0 ex:st2 ex:st0 ex:st3 ex:st0 ex:st4 ex:st0 ex:st5 } \
                FILTER NOT EXISTS { ?y ex:status ?st } }";
    for strategy in [
        crate::expr::ForcedExistsStrategy::Definition,
        crate::expr::ForcedExistsStrategy::Probe,
    ] {
        let _guard = crate::expr::force_exists_strategy_for_test(strategy);
        let mut ctx = EvalCtx::new(dataset.as_ref()).with_eval_options(crate::eval::EvalOptions {
            force_sequential: true,
            ..crate::eval::EvalOptions::default()
        });
        crate::eval::EXISTS_DECISION_COUNT.with(|count| count.set(0));
        evaluate_query_evaluated(&query(text), &mut ctx).unwrap();
        let decisions = crate::eval::EXISTS_DECISION_COUNT.with(std::cell::Cell::get);
        assert!(
            decisions <= rows + 1,
            "{decisions} decisions for {rows} rows under the forced strategy"
        );
    }
}

/// A prepared plan keeps what its `EXISTS` sites learned between runs. The second run of
/// each query answers and is charged exactly as the first, which began with nothing
/// learned.
#[test]
fn a_second_run_of_one_prepared_plan_answers_and_is_charged_as_the_first() {
    let dataset = dataset();
    let engine = crate::NativeSparqlEngine::default();
    for text in QUERIES.into_iter().chain(ROUTE_QUERIES) {
        let prepared = engine
            .prepare_query(&format!("PREFIX ex: <{EX}>\n{text}"), None)
            .expect("prepares");
        let mut runs = Vec::new();
        for _ in 0..2 {
            let outcome = engine
                .query_prepared_governed_view(
                    &*dataset,
                    &prepared,
                    &[],
                    &crate::QueryOptions::EMPTY,
                    &QueryGovernors::METERED,
                )
                .expect("completes");
            let crate::GovernedOutcome::Complete {
                result, evidence, ..
            } = outcome
            else {
                panic!("{text}: a metered run completes");
            };
            runs.push((
                format!("{result:?}"),
                [
                    ResourceDimension::Fuel,
                    ResourceDimension::ScratchBytes,
                    ResourceDimension::IntermediateCells,
                    ResourceDimension::AnswerRows,
                ]
                .map(|dimension| evidence.consumed_in(dimension)),
            ));
        }
        assert_eq!(runs[0], runs[1], "{text}: the second run of one plan");
    }
}

/// A comparison `FILTER`: its boolean result is the interned boolean the shortcut reads.
const COMPARISON: &str = "SELECT ?x ?s WHERE { ?x ex:score ?s FILTER(?s > 5.5e0) }";

/// The four witnesses a resident run raises, in the order of their declaration, for one
/// sequential run of `text` (under the host's governors when `host`) over an unbounded or
/// an accounted workspace.
fn engaged(dataset: &RdfDataset, text: &str, accounted: bool, host: bool) -> [u64; 4] {
    let counters = [
        &crate::eval::IN_PLACE_ANSWER_COUNT,
        &crate::eval::SITE_FACTS_READ_COUNT,
        &crate::eval::BOOLEAN_SHORTCUT_COUNT,
        &crate::eval::LONE_VARIABLE_READ_COUNT,
    ];
    for counter in counters {
        counter.with(|count| count.set(0));
    }
    let mut ctx = EvalCtx::new(dataset).with_eval_options(crate::eval::EvalOptions {
        force_sequential: true,
        ..crate::eval::EvalOptions::default()
    });
    let (workspace, _ledger, _) = account(1 << 40);
    if accounted {
        ctx.growth = workspace.capability();
    }
    if host {
        let state = Arc::new(GovernorState::new(
            &QueryGovernors::UNBOUNDED.with_max_intermediate_cells(1 << 40),
        ));
        ctx = ctx.with_governors(state);
    }
    evaluate_query_evaluated(&query(text), &mut ctx).expect("completes");
    counters.map(|counter| counter.with(std::cell::Cell::get))
}

/// Each fast path engages on the shape it claims, and none does on an accounted
/// workspace, which keeps the original routes.
#[test]
fn each_resident_fast_path_engages_on_its_shape_and_not_on_an_accounted_workspace() {
    let dataset = dataset();
    let [in_place, facts, ..] = engaged(&dataset, NOT_EXISTS, false, false);
    assert!(in_place > 0, "an EXISTS row is resumed in place");
    assert!(facts > 0, "the site is read through its learned facts");
    let [_, _, boolean, _] = engaged(&dataset, COMPARISON, false, false);
    assert!(
        boolean > 0,
        "the boolean a comparison yields is read as itself"
    );
    let [in_place, ..] = engaged(&dataset, BIND_EXISTS, false, false);
    assert!(in_place > 0, "a BIND of EXISTS is resumed in place too");
    // The lone-variable read, on the inline route and on the sequential one.
    let [.., inline] = engaged(&dataset, GROUP_COUNT, false, false);
    assert!(
        inline > 0,
        "the inline aggregate reads its argument from the row"
    );
    let [.., sequential] = engaged(&dataset, GROUP_COUNT, false, true);
    assert!(
        sequential > 0,
        "the sequential aggregate reads its argument from the row"
    );
    // The neighbour: an accounted workspace takes none of them.
    for text in [NOT_EXISTS, BIND_EXISTS, GROUP_COUNT] {
        assert_eq!(
            engaged(&dataset, text, true, false),
            [0; 4],
            "{text}: accounted"
        );
    }
}
