// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The custom-aggregate seam, end to end, from the vantage a host has: a query
//! text carrying `AGG(<iri>, args…)`, evaluated through the PUBLIC
//! [`NativeSparqlEngine`] entry points under a [`QueryOptions`] carrying an
//! [`AggregateRegistry`] — never reaching into the crate.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{
    AggregateAccumulator, AggregateRegistry, AlgebraicClass, Arity, CustomAggregate, EvalError,
    NativeSparqlEngine, QueryOptions, Volatility,
};

const EX: &str = "http://example.org/d/";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

const SUM_IRI: &str = "http://example.org/agg#sum";
const VOLATILE_SUM_IRI: &str = "http://example.org/agg#volatileSum";
const WEIGHTED_SUM_IRI: &str = "http://example.org/agg#weightedSum";
const UNREGISTERED_IRI: &str = "http://example.org/agg#nope";

// ── the example custom aggregates ───────────────────────────────────────────

/// A running integer sum over its single argument's lexical form. Non-numeric
/// arguments are simply ignored (never observed here, since every fixture
/// value is an `xsd:integer` literal).
struct SumAccumulator {
    total: i64,
}

impl AggregateAccumulator for SumAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.total.to_string(),
            XSD_INTEGER,
        )))
    }
}

/// `SUM`-alike, one argument, a declared determinism class the constructor picks —
/// used both as the ordinary `Stable` fixture and (registered under a second IRI)
/// as the `Volatile` fixture for the fork-gate determinism test.
struct SumAggregate {
    volatility: Volatility,
}

impl CustomAggregate for SumAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        self.volatility
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        0
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(SumAccumulator { total: 0 })
    }
}

/// A running sum of `value * weight` over a two-argument tuple — the fixture that
/// exercises DISTINCT deduping the FULL argument tuple rather than a single
/// column: two rows sharing `value` but differing in `weight` must both be
/// folded in.
struct WeightedSumAccumulator {
    total: i64,
}

impl AggregateAccumulator for WeightedSumAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        let (
            Some(TermValue::Literal {
                lexical_form: v, ..
            }),
            Some(TermValue::Literal {
                lexical_form: w, ..
            }),
        ) = (args.first(), args.get(1))
        else {
            return Ok(());
        };
        if let (Ok(v), Ok(w)) = (v.parse::<i64>(), w.parse::<i64>()) {
            self.total += v * w;
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.total.to_string(),
            XSD_INTEGER,
        )))
    }
}

struct WeightedSumAggregate;

impl CustomAggregate for WeightedSumAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(2)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        0
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(WeightedSumAccumulator { total: 0 })
    }
}

fn registry() -> AggregateRegistry {
    let mut registry = AggregateRegistry::new();
    registry.register(
        SUM_IRI,
        Arc::new(SumAggregate {
            volatility: Volatility::Stable,
        }),
    );
    registry.register(
        VOLATILE_SUM_IRI,
        Arc::new(SumAggregate {
            volatility: Volatility::Volatile,
        }),
    );
    registry.register(WEIGHTED_SUM_IRI, Arc::new(WeightedSumAggregate));
    registry
}

fn with_aggregates(registry: &AggregateRegistry) -> QueryOptions<'_> {
    QueryOptions {
        aggregates: registry,
        ..QueryOptions::EMPTY
    }
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

fn int_literal(b: &mut RdfDatasetBuilder, n: i64) -> purrdf_core::TermId {
    b.intern_literal(RdfLiteral::typed(n.to_string(), XSD_INTEGER.to_owned()))
}

/// `(cat1, s1, 1)`, `(cat1, s2, 2)`, `(cat1, s2, 2)` [duplicate value, distinct subject],
/// `(cat2, s3, 10)` — `ex:cat`/`ex:val` pairs, plus a `ex:weight` for the multi-arg case
/// mirroring `ex:val`'s subject-to-weight pairing (`s1`→5, `s2`→5, `s2`→7 twice for the
/// dedup fixture, `s3`→2).
fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let cat = b.intern_iri(&format!("{EX}cat"));
    let val = b.intern_iri(&format!("{EX}val"));
    let weight = b.intern_iri(&format!("{EX}weight"));
    let cat1 = b.intern_iri(&format!("{EX}cat1"));
    let cat2 = b.intern_iri(&format!("{EX}cat2"));

    let s1 = b.intern_iri(&format!("{EX}s1"));
    b.push_quad(s1, cat, cat1, None);
    let v1 = int_literal(&mut b, 1);
    b.push_quad(s1, val, v1, None);
    let w1 = int_literal(&mut b, 5);
    b.push_quad(s1, weight, w1, None);

    let s2 = b.intern_iri(&format!("{EX}s2"));
    b.push_quad(s2, cat, cat1, None);
    let v2 = int_literal(&mut b, 2);
    b.push_quad(s2, val, v2, None);
    let w2 = int_literal(&mut b, 5);
    b.push_quad(s2, weight, w2, None);

    let s2b = b.intern_iri(&format!("{EX}s2b"));
    b.push_quad(s2b, cat, cat1, None);
    // Same `?val` as s2 (2) — exercises single-column DISTINCT.
    let v2b = int_literal(&mut b, 2);
    b.push_quad(s2b, val, v2b, None);
    // Different `?weight` from s2 (7, not 5) — same `?val`, different `?weight`: the
    // (val, weight) TUPLE differs from s2's, so a full-tuple DISTINCT must keep BOTH.
    let w2b = int_literal(&mut b, 7);
    b.push_quad(s2b, weight, w2b, None);

    let s3 = b.intern_iri(&format!("{EX}s3"));
    b.push_quad(s3, cat, cat2, None);
    let v3 = int_literal(&mut b, 10);
    b.push_quad(s3, val, v3, None);
    let w3 = int_literal(&mut b, 2);
    b.push_quad(s3, weight, w3, None);

    b.freeze().expect("freeze")
}

fn run(ds: &Arc<RdfDataset>, query: &str, options: QueryOptions<'_>) -> SparqlResult {
    NativeSparqlEngine::new()
        .query_with_options_view(&**ds, request(query), options)
        .expect("query")
}

fn int_cell(row: &[Option<TermValue>], index: usize) -> i64 {
    match row[index].as_ref().expect("bound cell") {
        TermValue::Literal { lexical_form, .. } => lexical_form.parse().expect("integer literal"),
        other => panic!("expected an integer literal, got {other:?}"),
    }
}

fn rows(result: &SparqlResult) -> &[Vec<Option<TermValue>>] {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions, got {result:?}");
    };
    rows
}

// ── GROUP BY case ────────────────────────────────────────────────────────────

#[test]
fn group_by_case_sums_per_group() {
    let reg = registry();
    let ds = dataset();
    let query = format!(
        "SELECT ?cat (AGG(<{SUM_IRI}>, ?v) AS ?total) WHERE {{ \
         ?s <{EX}cat> ?cat . ?s <{EX}val> ?v }} GROUP BY ?cat ORDER BY ?cat"
    );
    let result = run(&ds, &query, with_aggregates(&reg));
    let rows = rows(&result);
    // cat1: s1(1) + s2(2) + s2b(2) = 5; cat2: s3(10).
    assert_eq!(rows.len(), 2);
    assert_eq!(int_cell(&rows[0], 1), 5, "cat1 sums to 5");
    assert_eq!(int_cell(&rows[1], 1), 10, "cat2 sums to 10");
}

// ── implicit single-group case ──────────────────────────────────────────────

#[test]
fn implicit_single_group_case_sums_everything() {
    let reg = registry();
    let ds = dataset();
    let query = format!("SELECT (AGG(<{SUM_IRI}>, ?v) AS ?total) WHERE {{ ?s <{EX}val> ?v }}");
    let result = run(&ds, &query, with_aggregates(&reg));
    let rows = rows(&result);
    assert_eq!(rows.len(), 1, "no GROUP BY: exactly one implicit group");
    // 1 + 2 + 2 + 10 = 15.
    assert_eq!(int_cell(&rows[0], 0), 15);
}

// ── DISTINCT (single-argument) ──────────────────────────────────────────────

#[test]
fn distinct_dedups_the_single_argument() {
    let reg = registry();
    let ds = dataset();
    let query =
        format!("SELECT (AGG(<{SUM_IRI}>, DISTINCT ?v) AS ?total) WHERE {{ ?s <{EX}val> ?v }}");
    let result = run(&ds, &query, with_aggregates(&reg));
    let rows = rows(&result);
    // Distinct values across all rows: {1, 2, 10} (the second `2`, from s2b, is
    // dropped) = 13.
    assert_eq!(int_cell(&rows[0], 0), 13);
}

// ── multi-argument tuple dedup ──────────────────────────────────────────────

#[test]
fn distinct_dedups_the_full_argument_tuple_not_a_single_column() {
    let reg = registry();
    let ds = dataset();
    let query = format!(
        "SELECT (AGG(<{WEIGHTED_SUM_IRI}>, DISTINCT ?v, ?w) AS ?total) WHERE {{ \
         ?s <{EX}val> ?v . ?s <{EX}weight> ?w }}"
    );
    let result = run(&ds, &query, with_aggregates(&reg));
    let result_rows = rows(&result);
    // Tuples: (1,5) from s1, (2,5) from s2, (2,7) from s2b [same `v` as s2, DIFFERENT
    // `w`, so NOT deduped against it], (10,2) from s3. Every tuple is distinct, so
    // DISTINCT drops nothing: 1*5 + 2*5 + 2*7 + 10*2 = 5 + 10 + 14 + 20 = 49.
    assert_eq!(int_cell(&result_rows[0], 0), 49);

    // Contrast: without DISTINCT the answer is identical here (no row repeats an
    // identical tuple in this fixture), which is the point — the dedup only ever
    // removes an EXACT tuple repeat, never a single-column collision.
    let query_no_distinct = format!(
        "SELECT (AGG(<{WEIGHTED_SUM_IRI}>, ?v, ?w) AS ?total) WHERE {{ \
         ?s <{EX}val> ?v . ?s <{EX}weight> ?w }}"
    );
    let result_no_distinct = run(&ds, &query_no_distinct, with_aggregates(&reg));
    assert_eq!(int_cell(&rows(&result_no_distinct)[0], 0), 49);
}

// ── empty group answers finish(init()) explicitly ───────────────────────────

#[test]
fn empty_group_answers_finish_of_init_explicitly() {
    let reg = registry();
    let ds = dataset();
    // No triple matches this predicate at all: the implicit single group is EMPTY,
    // and the custom aggregate must still answer (its accumulator's `finish(init())`)
    // rather than the row producing no solution at all.
    let query =
        format!("SELECT (AGG(<{SUM_IRI}>, ?v) AS ?total) WHERE {{ ?s <{EX}nonexistent> ?v }}");
    let result = run(&ds, &query, with_aggregates(&reg));
    let rows = rows(&result);
    assert_eq!(
        rows.len(),
        1,
        "an empty group is still exactly one answer row"
    );
    assert_eq!(
        int_cell(&rows[0], 0),
        0,
        "SumAccumulator's finish(init()) is total=0, answered explicitly"
    );
}

// ── unregistered-IRI prepare-time refusal ───────────────────────────────────

#[test]
fn unregistered_custom_aggregate_iri_is_refused_at_prepare_time_naming_the_iri() {
    let reg = registry();
    let engine = NativeSparqlEngine::new();
    let query =
        format!("SELECT (AGG(<{UNREGISTERED_IRI}>, ?v) AS ?total) WHERE {{ ?s <{EX}val> ?v }}");
    let options = with_aggregates(&reg);
    // `prepare_query_with_options` is the PREPARE-time entry — no dataset is even
    // supplied to it, so a success here proves the refusal (if any) did not wait
    // for evaluation.
    let error = engine
        .prepare_query_with_options(&query, None, options)
        .expect_err("an unregistered custom-aggregate IRI must be refused at prepare time");
    assert!(
        error.message.contains(UNREGISTERED_IRI),
        "the refusal must name the IRI: {}",
        error.message
    );
    // The planner admits BOTH a property-function call and a custom-aggregate call
    // in the same walk (`crate::property_fn_plan::plan_query`); this failure must
    // be attributed to the AGGREGATE seam, never reported under the
    // property-function code a caller matching on the documented contract would
    // otherwise be unable to see it under.
    assert_eq!(
        error.code, "native-sparql-aggregate-function",
        "an unregistered custom aggregate must report the aggregate code, not the \
         property-function code: {error:?}"
    );
}

/// The arity-mismatch twin of the unregistered-IRI refusal above: a REGISTERED
/// custom aggregate, called with the wrong positional-argument count, must also be
/// refused at prepare time under the aggregate code — not merely the unregistered
/// case.
#[test]
fn custom_aggregate_arity_mismatch_is_refused_at_prepare_time_under_the_aggregate_code() {
    let reg = registry();
    let engine = NativeSparqlEngine::new();
    // SUM_IRI is declared `Arity::Exact(1)`; two positional arguments is a mismatch.
    let query = format!("SELECT (AGG(<{SUM_IRI}>, ?v, ?v) AS ?total) WHERE {{ ?s <{EX}val> ?v }}");
    let options = with_aggregates(&reg);
    let error = engine
        .prepare_query_with_options(&query, None, options)
        .expect_err("an arity mismatch must be refused at prepare time");
    assert_eq!(
        error.code, "native-sparql-aggregate-function",
        "an arity mismatch on a registered custom aggregate must still report the \
         aggregate code: {error:?}"
    );
    assert!(
        error.message.contains(SUM_IRI),
        "the refusal must name the IRI: {}",
        error.message
    );
}

/// The regression control for both tests above: an unregistered PROPERTY-FUNCTION
/// call must still report the property-function code, never the aggregate code —
/// the fix that routes an aggregate failure to its own code must not have
/// widened to swallow the property-function seam too.
#[test]
fn unregistered_property_function_still_reports_the_property_function_code() {
    let engine = NativeSparqlEngine::new().with_parser_options(purrdf_sparql_eval::ParserOptions {
        extension_fn_namespaces: vec![],
        property_fn_namespaces: vec![format!("{EX}pf/")],
        property_fn_iris: Vec::new(),
    });
    let ds = dataset();
    let error = engine
        .query_with_options_view(
            &*ds,
            request(&format!("SELECT ?s WHERE {{ ?s <{EX}pf/nope> ?v }}")),
            QueryOptions::EMPTY,
        )
        .expect_err("nothing is registered under the configured namespace");
    assert_eq!(
        error.code, "native-sparql-property-function",
        "an unregistered property function must still report the property-function \
         code: {error:?}"
    );
}

// ── EMPTY ≡ any other empty registry ───────────────────────────────────────

/// `QueryOptions::aggregates` is `&AggregateRegistry`, never
/// `Option<&AggregateRegistry>` — there is no separate "no registry
/// configured" spelling for a query to distinguish from "an empty registry was
/// configured". A `None`-shaped call and a
/// `Some(&AggregateRegistry::new())`-shaped call answering differently is
/// therefore structurally impossible to even state: `None` does not
/// type-check as a `QueryOptions::aggregates` value at all. What remains
/// meaningful, and is what this test pins, is the weaker but still real
/// property that motivated `AggregateRegistry::EMPTY` being one canonical
/// shared constant rather than every call site minting its own empty registry:
/// [`QueryOptions::EMPTY`] (which carries `&AggregateRegistry::EMPTY`) and an
/// explicitly supplied, freshly built, still-empty [`AggregateRegistry::new`]
/// must answer identically, because both resolve every `AGG(<iri>, …)` IRI to
/// nothing.
#[test]
fn the_canonical_empty_registry_and_a_freshly_built_empty_registry_answer_identically() {
    let ds = dataset();
    let query = format!("SELECT ?s WHERE {{ ?s <{EX}val> ?v }} ORDER BY ?s");
    let canonical_empty_options = QueryOptions::EMPTY;
    let fresh_empty_registry = AggregateRegistry::new();
    let fresh_empty_options = with_aggregates(&fresh_empty_registry);

    let via_canonical = run(&ds, &query, canonical_empty_options);
    let via_fresh = run(&ds, &query, fresh_empty_options);
    assert_eq!(rows(&via_canonical), rows(&via_fresh));
}

// ── fork-gate determinism at scale ──────────────────────────────────────────

/// `n` subjects, each `ex:group<i % groups>` / `ex:val i`.
fn scaled_dataset(n: usize, groups: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let group_pred = b.intern_iri(&format!("{EX}group"));
    let val_pred = b.intern_iri(&format!("{EX}val"));
    for i in 0..n {
        let s = b.intern_iri(&format!("{EX}row{i}"));
        let g = b.intern_iri(&format!("{EX}group{}", i % groups));
        b.push_quad(s, group_pred, g, None);
        let v = int_literal(&mut b, i as i64);
        b.push_quad(s, val_pred, v, None);
    }
    b.freeze().expect("freeze")
}

/// A `Stable` custom aggregate over a dataset large enough to cross the fork-join
/// per-group threshold answers correctly and identically across two runs — proving
/// the aggregate is safely usable on the parallel per-group path (same answer as
/// the sequential semantics would give).
#[test]
fn stable_custom_aggregate_is_deterministic_at_parallel_scale() {
    const N: usize = 4000;
    const GROUPS: usize = 2000; // comfortably above PARALLEL_MIN_ROWS (1024)
    let reg = registry();
    let ds = scaled_dataset(N, GROUPS);
    let query = format!(
        "SELECT ?g (AGG(<{SUM_IRI}>, ?v) AS ?total) WHERE {{ \
         ?s <{EX}group> ?g . ?s <{EX}val> ?v }} GROUP BY ?g ORDER BY ?g"
    );
    let first = run(&ds, &query, with_aggregates(&reg));
    let second = run(&ds, &query, with_aggregates(&reg));
    assert_eq!(
        rows(&first).len(),
        GROUPS,
        "one row per distinct group value"
    );
    assert_eq!(
        rows(&first),
        rows(&second),
        "the parallel-eligible path must be deterministic across runs"
    );
    // `group0`'s rows are i in {0, GROUPS, 2*GROUPS, ...}; check the total directly
    // for one group as a correctness spot-check, not merely a determinism one.
    let expected_group0: i64 = (0..N as i64).step_by(GROUPS).sum();
    assert_eq!(int_cell(&rows(&first)[0], 1), expected_group0);
}

/// The `Volatile` twin of the above: at the SAME scale, a `Volatile` custom
/// aggregate must be forced sequential (never forked across groups — see
/// `crate::parallel::aggregate_is_unsafe`) and still answer correctly and
/// identically across two runs. Black-box, this test cannot observe that no fork
/// happened; what it proves is that forcing it sequential does not change or
/// destabilize the answer relative to the `Stable` case above (same fixture, same
/// arithmetic, same expected totals).
#[test]
fn volatile_custom_aggregate_is_still_correct_and_deterministic_at_scale() {
    const N: usize = 4000;
    const GROUPS: usize = 2000;
    let reg = registry();
    let ds = scaled_dataset(N, GROUPS);
    let query = format!(
        "SELECT ?g (AGG(<{VOLATILE_SUM_IRI}>, ?v) AS ?total) WHERE {{ \
         ?s <{EX}group> ?g . ?s <{EX}val> ?v }} GROUP BY ?g ORDER BY ?g"
    );
    let first = run(&ds, &query, with_aggregates(&reg));
    let second = run(&ds, &query, with_aggregates(&reg));
    assert_eq!(rows(&first).len(), GROUPS);
    assert_eq!(rows(&first), rows(&second));
    let expected_group0: i64 = (0..N as i64).step_by(GROUPS).sum();
    assert_eq!(int_cell(&rows(&first)[0], 1), expected_group0);
}

// ── the first-party statistical aggregate set (purrdf_sparql_eval::stat_agg) ──
//
// Same seam, same public engine entry points — the difference from every test
// above is that these ten aggregates are never registered by hand: a host
// calls `AggregateRegistry::register_statistical_aggregates` once, and gets
// the whole closed set through the string surface.

const STAT_NS: &str = "http://example.org/agg/";

fn statistical_registry() -> AggregateRegistry {
    let mut registry = AggregateRegistry::new();
    registry.register_statistical_aggregates(STAT_NS);
    registry
}

fn stat_lex(row: &[Option<TermValue>], index: usize) -> String {
    match row[index].as_ref().expect("bound cell") {
        TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
        other => panic!("expected a literal, got {other:?}"),
    }
}

/// `ex:g1` has values {1, 2, 3, 4} (four rows across four subjects); `ex:g2`
/// has values {10, 10, 20} (a repeat, for `MODE`/`DISTINCT`).
fn stat_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let cat = b.intern_iri(&format!("{EX}cat"));
    let val = b.intern_iri(&format!("{EX}val"));
    let g1 = b.intern_iri(&format!("{EX}g1"));
    let g2 = b.intern_iri(&format!("{EX}g2"));

    for (i, v) in [1, 2, 3, 4].into_iter().enumerate() {
        let s = b.intern_iri(&format!("{EX}g1s{i}"));
        b.push_quad(s, cat, g1, None);
        let vt = int_literal(&mut b, v);
        b.push_quad(s, val, vt, None);
    }
    for (i, v) in [10, 10, 20].into_iter().enumerate() {
        let s = b.intern_iri(&format!("{EX}g2s{i}"));
        b.push_quad(s, cat, g2, None);
        let vt = int_literal(&mut b, v);
        b.push_quad(s, val, vt, None);
    }
    b.freeze().expect("freeze")
}

#[test]
fn group_by_query_computes_several_statistical_members_per_group() {
    let reg = statistical_registry();
    let ds = stat_dataset();
    let query = format!(
        "SELECT ?cat \
         (AGG(<{STAT_NS}MEDIAN>, ?v) AS ?median) \
         (AGG(<{STAT_NS}STDDEV_POP>, ?v) AS ?stddevPop) \
         (AGG(<{STAT_NS}MODE>, ?v) AS ?mode) \
         (AGG(<{STAT_NS}FIRST>, ?v) AS ?first) \
         (AGG(<{STAT_NS}LAST>, ?v) AS ?last) \
         WHERE {{ ?s <{EX}cat> ?cat . ?s <{EX}val> ?v }} GROUP BY ?cat ORDER BY ?cat"
    );
    let result = run(&ds, &query, with_aggregates(&reg));
    let result_rows = rows(&result);
    assert_eq!(result_rows.len(), 2);

    // g1: {1, 2, 3, 4} — median 2.5, population stddev ~1.118, mode/first/last
    // over an all-distinct group are the smallest / earliest / latest value.
    assert_eq!(stat_lex(&result_rows[0], 1), "2.5");
    let g1_stddev_pop: f64 = stat_lex(&result_rows[0], 2).parse().expect("double");
    assert!((g1_stddev_pop - 1.118_033_988_75).abs() < 1e-9);
    assert_eq!(
        stat_lex(&result_rows[0], 3),
        "1",
        "MODE tie-break: smallest"
    );
    assert_eq!(stat_lex(&result_rows[0], 4), "1", "FIRST row seen");
    assert_eq!(stat_lex(&result_rows[0], 5), "4", "LAST row seen");

    // g2: {10, 10, 20} — MODE is 10 (the repeated value).
    assert_eq!(stat_lex(&result_rows[1], 3), "10");
}

/// `stat_dataset`'s g2 group ({10, 10, 20}) proves MODE finds A repeat, but 10 is
/// SIMULTANEOUSLY the mode, the minimum, and the first row seen — so it cannot
/// discriminate "found the genuine mode" from "defaulted to MIN" or "defaulted to
/// FIRST". This dataset's repeated value (20) is deliberately neither the min (10)
/// nor the max (30) nor the first (`g3s0` = 10) nor the last (`g3s3` = 30) row, so
/// this test can only pass for the right reason.
#[test]
fn mode_finds_the_genuine_repeat_not_a_coincidental_stand_in() {
    let reg = statistical_registry();
    let mut b = RdfDatasetBuilder::new();
    let val = b.intern_iri(&format!("{EX}val"));
    for (i, v) in [10, 20, 20, 30].into_iter().enumerate() {
        let s = b.intern_iri(&format!("{EX}g3s{i}"));
        let vt = int_literal(&mut b, v);
        b.push_quad(s, val, vt, None);
    }
    let ds = b.freeze().expect("freeze");
    let query = format!("SELECT (AGG(<{STAT_NS}MODE>, ?v) AS ?mode) WHERE {{ ?s <{EX}val> ?v }}");
    let result = run(&ds, &query, with_aggregates(&reg));
    assert_eq!(
        stat_lex(&rows(&result)[0], 0),
        "20",
        "MODE must be the genuinely repeated value, distinct from MIN (10), MAX (30), \
         the first row (10), and the last row (30)"
    );
}

#[test]
fn percentile_named_scalarval_form_end_to_end() {
    let reg = statistical_registry();
    let ds = stat_dataset();
    let query = format!(
        "SELECT (AGG(<{STAT_NS}PERCENTILE>, ?v; P=0.5) AS ?p) \
         WHERE {{ ?s <{EX}cat> <{EX}g1> . ?s <{EX}val> ?v }}"
    );
    let result = run(&ds, &query, with_aggregates(&reg));
    // P=0.5 over {1,2,3,4} is the same interpolated median as MEDIAN itself.
    assert_eq!(stat_lex(&rows(&result)[0], 0), "2.5");
}

/// `P=0.5` alone is MEDIAN's own path at zero interpolation (an even split needing
/// no weighting between unequal neighbors) — it cannot prove PERCENTILE's
/// interpolation arithmetic runs at all. `P=0.1` over g1's {1,2,3,4} lands at
/// `rank = P * (n-1) = 0.3`, strictly between the two smallest values (1 and 2),
/// so the answer (1.3) is only reachable by actually interpolating: it is not
/// MEDIAN (2.5), not MIN (1), not MAX (4), and not an unweighted average of any
/// two data points.
#[test]
fn percentile_at_a_genuinely_interpolating_fraction() {
    let reg = statistical_registry();
    let ds = stat_dataset();
    let query = format!(
        "SELECT (AGG(<{STAT_NS}PERCENTILE>, ?v; P=0.1) AS ?p) \
         WHERE {{ ?s <{EX}cat> <{EX}g1> . ?s <{EX}val> ?v }}"
    );
    let result = run(&ds, &query, with_aggregates(&reg));
    assert_eq!(stat_lex(&rows(&result)[0], 0), "1.3");
}

#[test]
fn percentile_out_of_range_p_is_unbound_not_a_hard_error() {
    let reg = statistical_registry();
    let ds = stat_dataset();
    let query = format!(
        "SELECT (AGG(<{STAT_NS}PERCENTILE>, ?v; P=1.5) AS ?p) \
         WHERE {{ ?s <{EX}cat> <{EX}g1> . ?s <{EX}val> ?v }}"
    );
    let result = run(&ds, &query, with_aggregates(&reg));
    assert!(
        rows(&result)[0][0].is_none(),
        "P outside [0, 1] poisons the fold to unbound, never a query-aborting error"
    );
}

/// A missing required scalarval (`PERCENTILE` declares `P`) is refused at
/// PREPARE time, under the aggregate diagnostic code — never a runtime poison.
#[test]
fn percentile_missing_p_is_refused_at_prepare_time() {
    let reg = statistical_registry();
    let engine = NativeSparqlEngine::new();
    let query =
        format!("SELECT (AGG(<{STAT_NS}PERCENTILE>, ?v) AS ?p) WHERE {{ ?s <{EX}val> ?v }}");
    let error = engine
        .prepare_query_with_options(&query, None, with_aggregates(&reg))
        .expect_err("a missing required scalarval must be refused at prepare time");
    assert_eq!(error.code, "native-sparql-aggregate-function");
    assert!(error.message.contains('P'), "{}", error.message);
}

/// An unrecognized scalarval name is refused at prepare time too, naming the
/// aggregate — the sibling refusal to a missing one.
#[test]
fn percentile_unknown_scalarval_name_is_refused_at_prepare_time() {
    let reg = statistical_registry();
    let engine = NativeSparqlEngine::new();
    let query =
        format!("SELECT (AGG(<{STAT_NS}PERCENTILE>, ?v; Q=0.5) AS ?p) WHERE {{ ?s <{EX}val> ?v }}");
    let error = engine
        .prepare_query_with_options(&query, None, with_aggregates(&reg))
        .expect_err("an unrecognized scalarval name must be refused at prepare time");
    assert_eq!(error.code, "native-sparql-aggregate-function");
}

/// A duplicate scalarval name is refused at prepare time.
#[test]
fn percentile_duplicate_scalarval_is_refused_at_prepare_time() {
    let reg = statistical_registry();
    let engine = NativeSparqlEngine::new();
    let query = format!(
        "SELECT (AGG(<{STAT_NS}PERCENTILE>, ?v; P=0.5; P=0.9) AS ?p) WHERE {{ ?s <{EX}val> ?v }}"
    );
    let error = engine
        .prepare_query_with_options(&query, None, with_aggregates(&reg))
        .expect_err("a duplicate scalarval name must be refused at prepare time");
    assert_eq!(error.code, "native-sparql-aggregate-function");
}

/// A wrong-typed scalarval value (a string where `PERCENTILE`'s `P` declares
/// `Numeric`) is refused at prepare time.
#[test]
fn percentile_wrong_typed_scalarval_is_refused_at_prepare_time() {
    let reg = statistical_registry();
    let engine = NativeSparqlEngine::new();
    let query = format!(
        "SELECT (AGG(<{STAT_NS}PERCENTILE>, ?v; P=\"high\") AS ?p) WHERE {{ ?s <{EX}val> ?v }}"
    );
    let error = engine
        .prepare_query_with_options(&query, None, with_aggregates(&reg))
        .expect_err("a wrong-typed scalarval value must be refused at prepare time");
    assert_eq!(error.code, "native-sparql-aggregate-function");
}

#[test]
fn topk_end_to_end() {
    let reg = statistical_registry();
    let ds = stat_dataset();
    let query = format!(
        "SELECT (AGG(<{STAT_NS}TOPK>, ?v; K=2) AS ?top) \
         WHERE {{ ?s <{EX}cat> <{EX}g1> . ?s <{EX}val> ?v }}"
    );
    let result = run(&ds, &query, with_aggregates(&reg));
    assert_eq!(stat_lex(&rows(&result)[0], 0), "4 3");
}

#[test]
fn distinct_interacts_with_a_statistical_member() {
    let reg = statistical_registry();
    let ds = stat_dataset();
    // g2 is {10, 10, 20}: DISTINCT sees {10, 20}, so its interpolated median
    // (the mean of a two-element DISTINCT set) is (10+20)/2 = 15, not the
    // three-element non-distinct median (10).
    let query = format!(
        "SELECT (AGG(<{STAT_NS}MEDIAN>, DISTINCT ?v) AS ?median) \
         WHERE {{ ?s <{EX}cat> <{EX}g2> . ?s <{EX}val> ?v }}"
    );
    let distinct_result = run(&ds, &query, with_aggregates(&reg));
    assert_eq!(stat_lex(&rows(&distinct_result)[0], 0), "15");

    let query_plain = format!(
        "SELECT (AGG(<{STAT_NS}MEDIAN>, ?v) AS ?median) \
         WHERE {{ ?s <{EX}cat> <{EX}g2> . ?s <{EX}val> ?v }}"
    );
    let plain_result = run(&ds, &query_plain, with_aggregates(&reg));
    assert_eq!(stat_lex(&rows(&plain_result)[0], 0), "10");
}

/// `ex:mval` over five subjects: values {1, 2, 3, 4, 5} — chosen (rather than
/// reusing `stat_dataset`'s g1={1,2,3,4}) so BOTH the population and sample
/// moments land on an exact, clean decimal: `Σx=15`, `Σx²=55`, numerator
/// `Σx² − (Σx)²/n = 55 − 45 = 10`, population denom 5 → `VAR_POP = 2`, sample
/// denom 4 → `VARIANCE = 2.5`.
fn moments_dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let mval = b.intern_iri(&format!("{EX}mval"));
    for (i, v) in [1, 2, 3, 4, 5].into_iter().enumerate() {
        let s = b.intern_iri(&format!("{EX}m{i}"));
        let vt = int_literal(&mut b, v);
        b.push_quad(s, mval, vt, None);
    }
    b.freeze().expect("freeze")
}

/// The three moment-based members `STDDEV`/`VARIANCE`/`VAR_POP` had no
/// string-surface (`AGG(<iri>, …)`) coverage in this file — only
/// `STDDEV_POP` did (see `group_by_query_computes_several_statistical_members_per_group`).
/// `VARIANCE`/`VAR_POP` are exact decimal division (see `crate::stat_agg`'s
/// `MomentsAccumulator::finish`), so their answers are pinned as exact
/// strings; `STDDEV`'s final `sqrt` step is inexact `f64`, so it is compared
/// with a tolerance exactly like `STDDEV_POP` is elsewhere in this file.
#[test]
fn moments_stddev_variance_and_var_pop_end_to_end() {
    let reg = statistical_registry();
    let ds = moments_dataset();
    let query = format!(
        "SELECT (AGG(<{STAT_NS}VARIANCE>, ?v) AS ?variance) \
         (AGG(<{STAT_NS}VAR_POP>, ?v) AS ?varPop) \
         (AGG(<{STAT_NS}STDDEV>, ?v) AS ?stddev) \
         WHERE {{ ?s <{EX}mval> ?v }}"
    );
    let result = run(&ds, &query, with_aggregates(&reg));
    let result_rows = rows(&result);
    assert_eq!(result_rows.len(), 1);
    assert_eq!(
        stat_lex(&result_rows[0], 0),
        "2.5",
        "sample variance over {{1,2,3,4,5}}: 10/4"
    );
    assert_eq!(
        stat_lex(&result_rows[0], 1),
        "2",
        "population variance over {{1,2,3,4,5}}: 10/5, integer-valued decimal has no point"
    );
    let stddev: f64 = stat_lex(&result_rows[0], 2).parse().expect("double");
    assert!(
        (stddev - 2.5_f64.sqrt()).abs() < 1e-9,
        "sample stddev = sqrt(sample variance) = sqrt(2.5), got {stddev}"
    );
}

#[test]
fn a_local_name_outside_the_closed_set_is_refused_at_prepare_time() {
    let reg = statistical_registry();
    let engine = NativeSparqlEngine::new();
    let query =
        format!("SELECT (AGG(<{STAT_NS}NOT_A_MEMBER>, ?v) AS ?x) WHERE {{ ?s <{EX}val> ?v }}");
    let options = with_aggregates(&reg);
    let error = engine
        .prepare_query_with_options(&query, None, options)
        .expect_err("a local name outside the closed set is simply unregistered");
    assert!(error.message.contains("NOT_A_MEMBER"));
}

#[test]
fn statistical_aggregates_are_deterministic_at_parallel_scale() {
    const N: usize = 4000;
    const GROUPS: usize = 2000;
    let reg = statistical_registry();
    let ds = scaled_dataset(N, GROUPS);
    let query = format!(
        "SELECT ?g (AGG(<{STAT_NS}FIRST>, ?v) AS ?f) (AGG(<{STAT_NS}MEDIAN>, ?v) AS ?m) WHERE {{ \
         ?s <{EX}group> ?g . ?s <{EX}val> ?v }} GROUP BY ?g ORDER BY ?g"
    );
    let first_run = run(&ds, &query, with_aggregates(&reg));
    let second_run = run(&ds, &query, with_aggregates(&reg));
    assert_eq!(rows(&first_run).len(), GROUPS);
    assert_eq!(
        rows(&first_run),
        rows(&second_run),
        "the parallel-eligible path must be deterministic across runs"
    );
}

// ── zero-arity custom aggregate: unrepresentable, so it can never row-count ────

/// A `CustomAggregate` whose declared [`Arity`] admits ZERO arguments and whose
/// `finish` returns a sentinel utterly unlike a row count (`-999`), so if this
/// accumulator's `init`/`step`/`finish` ever ran in place of a row count, this
/// value — not the group's cardinality — would come back.
///
/// It never gets the chance: `AGG(<iri>)` (zero positional args) is refused at
/// PARSE time (`parse_agg_call`'s "one or more" rule), and even a caller who
/// skips the SPARQL parser entirely and builds the algebra by hand cannot
/// construct the `AggregateExpression` this accumulator would need —
/// [`purrdf_sparql_algebra::AggregateExpression::new`] refuses an empty `args`
/// for anything but `COUNT`. A registry is free to declare `Arity::Exact(0)`
/// (checked below), but that declaration can never be exercised: the call site
/// that would supply zero arguments cannot exist as a value.
struct ZeroArityAggregate;

impl AggregateAccumulator for ZeroArityAggregate {
    fn step(&mut self, _args: &[TermValue]) -> Result<(), EvalError> {
        panic!("a zero-arity custom aggregate can never be constructed, so `step` can never run");
    }

    fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        panic!(
            "a zero-arity custom aggregate can never be constructed, so `combine` can never run"
        );
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal("-999", XSD_INTEGER)))
    }
}

impl CustomAggregate for ZeroArityAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(0)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        0
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(Self)
    }
}

/// The gap the shipped CLI actually hit: `SUM(*)`/`AVG(*)`/… silently
/// answered a GROUP's ROW COUNT instead of erroring or running the named
/// function, because the evaluator used to decide "this is `COUNT(*)`" by
/// asking whether `args` was empty rather than by reading `function`. The same
/// shape applied to a registered zero-arity [`CustomAggregate`]: nothing kept
/// `AggregateFunction::Custom` with an empty `args` from existing, so it too
/// would have hit the row-count branch and silently bypassed the registry
/// (and its state-bound metering) entirely.
///
/// This proves that hole is closed at the type level, not merely patched at
/// the one call site the regression was found in: a registry that declares a
/// custom aggregate's arity as `Arity::Exact(0)` — legal on its own terms —
/// still cannot be reached, because no `AggregateExpression` naming it with an
/// empty `args` can ever be built, from inside `purrdf-sparql-eval` or out.
#[test]
fn zero_arity_custom_aggregate_cannot_be_constructed_and_therefore_never_row_counts() {
    const ZERO_ARITY_IRI: &str = "http://example.org/agg#zeroArity";
    let mut reg = registry();
    reg.register(ZERO_ARITY_IRI, Arc::new(ZeroArityAggregate));
    // The registry itself is untroubled by the zero-arity declaration.
    assert!(reg.resolve(ZERO_ARITY_IRI).is_some());

    // The SPARQL surface refuses `AGG(<iri>)` with no positional arguments —
    // this is `crates/sparql-algebra`'s own `agg_call_requires_at_least_one_argument`
    // pinned again here, from the public evaluator entry point, against a
    // registry that WOULD happily run the call if it were ever handed one.
    let engine = NativeSparqlEngine::new();
    let query = format!("SELECT (AGG(<{ZERO_ARITY_IRI}>) AS ?x) WHERE {{ ?s <{EX}val> ?v }}");
    engine
        .prepare_query_with_options(&query, None, with_aggregates(&reg))
        .expect_err("AGG(<iri>) with zero positional args is a hard parse-time syntax error");

    // And even bypassing the SPARQL parser entirely — building the algebra node
    // directly, as an embedder driving `purrdf-sparql-eval` as a library would —
    // the checked constructor refuses the same shape: there is no way, anywhere
    // in this crate's public surface, to build an `AggregateExpression` naming
    // `AggregateFunction::Custom` with an empty `args`.
    let err = purrdf_sparql_algebra::AggregateExpression::new(
        purrdf_sparql_algebra::AggregateFunction::Custom(
            purrdf_sparql_algebra::NamedNode::new_unchecked(ZERO_ARITY_IRI),
        ),
        Vec::new(),
        Vec::new(),
        false,
    )
    .expect_err("a zero-arity Custom aggregate must be unrepresentable, registry or not");
    assert!(matches!(
        err.function(),
        purrdf_sparql_algebra::AggregateFunction::Custom(iri) if iri.as_str() == ZERO_ARITY_IRI
    ));
}

// ── registry INSTANCE identity: a plan must not silently cross registries ──
//
// `AGG(<iri>, …)` is admitted (arity checked, registration confirmed) at PREPARE
// time against one registry. A plan's identity must be tied to WHICH registry
// instance it was admitted against — not merely to what that registry declares —
// because two independently built registries can register the same IRI to two
// entirely different accumulators while describing identically.

/// A `PRODUCT`-alike accumulator: multiplies rather than sums. Declares EXACTLY
/// the same [`Arity`]/[`Volatility`]/[`AlgebraicClass`]/state-bound as
/// [`SumAggregate`] (`registry()`'s `SUM_IRI` registration) — nothing about its
/// DECLARATION can distinguish it from a SUM — but its computed answer is
/// completely different.
struct ProductAccumulator {
    total: i64,
}

impl AggregateAccumulator for ProductAccumulator {
    fn step(&mut self, args: &[TermValue]) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = args.first()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total *= n;
        }
        Ok(())
    }

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
        if let Some(TermValue::Literal { lexical_form, .. }) = other.finish()?
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total *= n;
        }
        Ok(())
    }

    fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
        self
    }

    fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
        Ok(Some(TermValue::typed_literal(
            self.total.to_string(),
            XSD_INTEGER,
        )))
    }
}

struct ProductAggregate;

impl CustomAggregate for ProductAggregate {
    fn arity(&self) -> Arity {
        Arity::Exact(1)
    }
    fn volatility(&self) -> Volatility {
        Volatility::Stable
    }
    fn algebraic_class(&self) -> AlgebraicClass {
        AlgebraicClass::Commutative
    }
    fn state_bound(&self) -> u64 {
        0
    }
    fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
        Box::new(ProductAccumulator { total: 1 })
    }
}

/// The exact reproduction: registry A binds `SUM_IRI` to a SUM; registry B binds
/// the SAME IRI to a PRODUCT with identical declared arity, volatility, algebraic
/// class, and state bound. A query is prepared under A (admitted, arity-checked)
/// and handed to `query_prepared_view` under B. This must be a typed refusal —
/// NEVER a silently-computed answer under B's different implementation.
#[test]
fn a_plan_prepared_under_one_registry_refuses_to_execute_under_a_different_registry_with_identical_declarations()
 {
    let mut registry_a = AggregateRegistry::new();
    registry_a.register(
        SUM_IRI,
        Arc::new(SumAggregate {
            volatility: Volatility::Stable,
        }),
    );
    let mut registry_b = AggregateRegistry::new();
    registry_b.register(SUM_IRI, Arc::new(ProductAggregate));

    // The reproduction only means what it claims if the two registries' DECLARED
    // metadata is byte-identical for this IRI — confirm that first.
    assert_eq!(
        registry_a.describe().expect("no panic"),
        registry_b.describe().expect("no panic"),
        "the two registries must declare identically for this to be a meaningful \
         reproduction of the declaration-only fingerprint gap"
    );

    let ds = dataset();
    let engine = NativeSparqlEngine::new();
    // dataset()'s `ex:val` values are {1, 2, 2, 10}: SUM = 15, PRODUCT = 40 — a
    // PRODUCT answer here could only come from silently running under registry B.
    let query = format!("SELECT (AGG(<{SUM_IRI}>, ?v) AS ?total) WHERE {{ ?s <{EX}val> ?v }}");

    let prepared = engine
        .prepare_query_with_options(&query, None, with_aggregates(&registry_a))
        .expect("registry A admits and prepares the call");

    let error = engine
        .query_prepared_view(&*ds, &prepared, &[], with_aggregates(&registry_b))
        .expect_err(
            "a plan prepared under registry A must be REFUSED under registry B, never silently \
             executed against B's different accumulator",
        );
    assert_eq!(
        error.code, "native-sparql-aggregate-function",
        "the refusal must be attributable to the aggregate-registry identity check: {error:?}"
    );
}

/// The non-regression twin: executing a plan under the SAME registry instance it
/// was prepared under must still work — the identity check above must not
/// produce a false refusal for the ordinary case.
#[test]
fn a_plan_prepared_and_executed_under_the_same_registry_instance_still_works() {
    let reg = registry();
    let ds = dataset();
    let engine = NativeSparqlEngine::new();
    let query = format!("SELECT (AGG(<{SUM_IRI}>, ?v) AS ?total) WHERE {{ ?s <{EX}val> ?v }}");

    let prepared = engine
        .prepare_query_with_options(&query, None, with_aggregates(&reg))
        .expect("the registry admits and prepares the call");
    let result = engine
        .query_prepared_view(&*ds, &prepared, &[], with_aggregates(&reg))
        .expect("the SAME registry instance must be accepted at execution");
    assert_eq!(int_cell(&rows(&result)[0], 0), 15, "1 + 2 + 2 + 10");
}
