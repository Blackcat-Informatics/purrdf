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

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) {
        if let Ok(Some(TermValue::Literal { lexical_form, .. })) = other.finish()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
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
    fn init(&self) -> Box<dyn AggregateAccumulator> {
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

    fn combine(&mut self, other: Box<dyn AggregateAccumulator>) {
        if let Ok(Some(TermValue::Literal { lexical_form, .. })) = other.finish()
            && let Ok(n) = lexical_form.parse::<i64>()
        {
            self.total += n;
        }
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
    fn init(&self) -> Box<dyn AggregateAccumulator> {
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
        aggregates: Some(registry),
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
}

// ── None ≡ Some(empty) ───────────────────────────────────────────────────────

#[test]
fn none_and_some_empty_registry_behave_identically() {
    let ds = dataset();
    // A query with no custom aggregate at all: `aggregates: None` and
    // `aggregates: Some(&AggregateRegistry::new())` must answer identically.
    let query = format!("SELECT ?s WHERE {{ ?s <{EX}val> ?v }} ORDER BY ?s");
    let none_options = QueryOptions::EMPTY;
    let empty_registry = AggregateRegistry::new();
    let empty_options = with_aggregates(&empty_registry);

    let via_none = run(&ds, &query, none_options);
    let via_empty = run(&ds, &query, empty_options);
    assert_eq!(rows(&via_none), rows(&via_empty));
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
