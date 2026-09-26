// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Nested correlated `EXISTS`: what one outer row costs, counted, and what the answer is.
//!
//! [`crate::deferred_exists`] leaves a nested `EXISTS` body out of the per-row copy of the
//! pattern around it and substitutes it when it is evaluated. Two claims follow, and each
//! is checked here against an observer that could tell them apart from their failure:
//!
//! * **Cost.** One more outer row costs work proportional to the nesting depth: counted
//!   in tree nodes substituted, copied and analysed ([`crate::op_count`]), never in time.
//!   The same count taken with every body substituted in full
//!   ([`crate::deferred_exists::force_eager_substitution_for_test`], the evaluation as it
//!   was before) grows cubically, so the count can see the difference.
//! * **Answer.** The deferred evaluation answers exactly what the full substitution
//!   answers: against hand-pinned rows, against an independent reading of the query over
//!   the data, and against the full substitution itself over shapes that exercise every
//!   arm of the substitution a nested body can meet.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_algebra::{
    Expression, GraphPattern, NamedNode, NamedNodePattern, Query, QueryDataset, TermPattern,
    TriplePattern, Variable,
};

use crate::deferred_exists::force_eager_substitution_for_test;
use crate::engine::NativeSparqlEngine;
use crate::eval::EvalOptions;
use crate::governed::GovernedOutcome;
use crate::governor::QueryGovernors;
use crate::op_count::OpCounts;

const EX: &str = "http://example.org/";

/// A stack large enough for every depth these tests evaluate.
const BIG_STACK: usize = 512 * 1024 * 1024;

/// Run `body` on a fresh thread with [`BIG_STACK`] of stack.
fn on_big_stack<T: Send + 'static>(body: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(BIG_STACK)
        .spawn(body)
        .expect("spawn")
        .join()
        .expect("the evaluation thread returned")
}

/// An engine that evaluates on the calling thread, so the thread-local counters and the
/// thread-local eager override see the whole evaluation.
fn sequential_engine() -> NativeSparqlEngine {
    NativeSparqlEngine::default().with_eval_options(EvalOptions {
        force_sequential: true,
        ..EvalOptions::default()
    })
}

/// Every row of a SELECT result, each cell spelled, sorted.
fn rows_of(result: &SparqlResult) -> Vec<Vec<String>> {
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("a SELECT answers with solutions, got {result:?}");
    };
    let mut out: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| match cell {
                    None => "UNBOUND".to_owned(),
                    Some(TermValue::Iri(iri)) => iri.clone(),
                    Some(other) => format!("{other:?}"),
                })
                .collect()
        })
        .collect();
    out.sort();
    out
}

/// `query` over `ds`, evaluated sequentially: deferred, or with every nested body
/// substituted in full when `eager`.
fn answer(ds: &Arc<RdfDataset>, query: &str, eager: bool) -> Vec<Vec<String>> {
    let engine = sequential_engine();
    let prepared = engine
        .prepare_query(query, None)
        .unwrap_or_else(|e| panic!("prepare {query}: {e}"));
    let _eager = eager.then(force_eager_substitution_for_test);
    let result = engine
        .query_prepared(ds, &prepared, &[], crate::QueryOptions::EMPTY)
        .unwrap_or_else(|e| panic!("evaluate {query}: {e}"));
    rows_of(&result)
}

/// `query` over `ds`, deferred, with every row loop forked one row per worker: the
/// placeholders and the kept sites are then read from workers the window forked.
fn answer_forked(ds: &Arc<RdfDataset>, query: &str) -> Vec<Vec<String>> {
    let engine = NativeSparqlEngine::default();
    let prepared = engine
        .prepare_query(query, None)
        .unwrap_or_else(|e| panic!("prepare {query}: {e}"));
    let _parallel = crate::parallel::force_parallel_for_test(true);
    let _chunks = crate::parallel::force_chunk_size_for_test(1);
    let result = engine
        .query_prepared(ds, &prepared, &[], crate::QueryOptions::EMPTY)
        .unwrap_or_else(|e| panic!("evaluate {query}: {e}"));
    rows_of(&result)
}

/// `query` over `ds`, deferred (sequentially and forked), checked against the full
/// substitution; returns the rows.
fn answer_both_ways(ds: &Arc<RdfDataset>, query: &str) -> Vec<Vec<String>> {
    let deferred = answer(ds, query, false);
    let eager = answer(ds, query, true);
    assert_eq!(
        deferred, eager,
        "the deferred substitution answered differently from the full one for {query}"
    );
    assert_eq!(
        answer_forked(ds, query),
        eager,
        "the deferred substitution, forked, answered differently from the full one for {query}"
    );
    deferred
}

fn iri(local: &str) -> String {
    format!("{EX}{local}")
}

// ---------------------------------------------------------------------------
// Cost
// ---------------------------------------------------------------------------

/// `:s0 … :s{n-1}`, each with one `:p` edge to its own `:o{i}`.
fn edge_per_subject(n: usize) -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&iri("p"));
    for i in 0..n {
        let s = b.intern_iri(&format!("{EX}s{i}"));
        let o = b.intern_iri(&format!("{EX}o{i}"));
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("freeze")
}

/// `depth` nested `FILTER EXISTS`, every level correlated with the one around it through
/// `?s` and `?o` — the shape whose cost grew with the cube of `depth`.
fn same_row_nesting(depth: usize) -> String {
    let mut body = format!("?s <{EX}p> ?o");
    for _ in 0..depth {
        body = format!("?s <{EX}p> ?o FILTER EXISTS {{ {body} }}");
    }
    format!("SELECT ?s WHERE {{ {body} }}")
}

/// The counters and the `EXISTS` preparations an evaluation of `query` over `ds` made.
fn counted(ds: &Arc<RdfDataset>, query: &str, eager: bool) -> (Vec<Vec<String>>, OpCounts, u64) {
    let engine = sequential_engine();
    let prepared = engine.prepare_query(query, None).expect("prepare");
    let _eager = eager.then(force_eager_substitution_for_test);
    crate::op_count::reset();
    crate::eval::PREPARED_EXISTS_BUILD_COUNT.with(|count| count.set(0));
    let result = engine
        .query_prepared(ds, &prepared, &[], crate::QueryOptions::EMPTY)
        .expect("evaluate");
    let counts = crate::op_count::read();
    let builds = crate::eval::PREPARED_EXISTS_BUILD_COUNT.with(std::cell::Cell::get);
    (rows_of(&result), counts, builds)
}

/// What one outer row costs at `depth`: the counts of a two-row evaluation minus those of
/// a one-row one (the two rows restrict the outer `EXISTS` differently, so its memo shares
/// nothing between them). Also the one-row evaluation's own counts and preparations —
/// the once-per-evaluation part.
#[derive(Debug, Clone, Copy)]
struct Cost {
    per_row: OpCounts,
    per_row_builds: u64,
    once: OpCounts,
    once_builds: u64,
}

fn cost(depth: usize, eager: bool) -> Cost {
    on_big_stack(move || {
        let query = same_row_nesting(depth);
        let (one, c1, b1) = counted(&edge_per_subject(1), &query, eager);
        assert_eq!(one, vec![vec![iri("s0")]]);
        let (two, c2, b2) = counted(&edge_per_subject(2), &query, eager);
        assert_eq!(two, vec![vec![iri("s0")], vec![iri("s1")]]);
        Cost {
            per_row: OpCounts {
                substituted: c2.substituted - c1.substituted,
                cloned: c2.cloned - c1.cloned,
                analyzed: c2.analyzed - c1.analyzed,
                var_walked: c2.var_walked - c1.var_walked,
            },
            per_row_builds: b2 - b1,
            once: c1,
            once_builds: b1,
        }
    })
}

/// **One more outer row of `d` nested correlated `EXISTS` costs `O(d)` counted work.**
///
/// Per extra outer row, deferred: `4d - 2` substituted nodes (each level's own four-node
/// body, less the two the innermost level lacks), and no copy, analysis, variable walk or
/// preparation at all — every level's body was prepared once, by the first row. Measured:
///
/// | depth | per-row work, deferred | per-row work, full substitution (before) | preparations per row, before |
/// |------:|-----------------------:|-----------------------------------------:|----------------------------:|
/// |    20 |                     78 |                                   26,773 |                          19 |
/// |    40 |                    158 |                                  315,343 |                          39 |
/// |    80 |                    318 |                                4,205,883 |                          79 |
///
/// The full substitution's count is the evaluator before this change (the same numbers
/// were read off the unchanged code): each doubling of the depth multiplies it by twelve
/// to thirteen — cubic — where the deferred count doubles.
///
/// The once-per-evaluation part is quadratic, and is paid once however many rows follow:
/// each of the `d` levels is prepared once, and a preparation normalizes, copies and
/// analyses its whole body, the levels below it included.
#[test]
fn nested_exists_per_row_work_is_linear_in_depth() {
    let depths = [20_usize, 40, 80];
    let deferred: Vec<Cost> = depths.iter().map(|&d| cost(d, false)).collect();
    let eager: Vec<Cost> = depths.iter().map(|&d| cost(d, true)).collect();
    for ((depth, deferred), eager) in depths.iter().zip(&deferred).zip(&eager) {
        eprintln!("depth {depth}: deferred {deferred:?}; full substitution {eager:?}");
    }

    for (&depth, cost) in depths.iter().zip(&deferred) {
        let d = u64::try_from(depth).expect("depth fits");
        assert_eq!(
            cost.per_row,
            OpCounts {
                substituted: 4 * d - 2,
                cloned: 0,
                analyzed: 0,
                var_walked: 0,
            },
            "one more outer row at depth {depth} must substitute each level's own body once \
             and copy, analyse and walk nothing else"
        );
        assert_eq!(
            cost.per_row_builds, 0,
            "every level is prepared once per evaluation, not once per outer row"
        );
        assert_eq!(
            cost.once_builds, d,
            "each of the {depth} EXISTS levels is prepared exactly once"
        );
        assert_eq!(
            cost.once.substituted, cost.per_row.substituted,
            "the first outer row substitutes exactly what every later one does; only the \
             preparation is paid once"
        );
    }

    // The observer can see the difference: the same count, taken over the full
    // substitution, grows by far more than double per doubling of the depth.
    for pair in eager.windows(2) {
        assert!(
            pair[1].per_row.total() > 8 * pair[0].per_row.total(),
            "the full substitution's per-row work must grow super-linearly (cubically) with \
             depth, or this test could not tell the two apart: {pair:?}"
        );
        assert!(pair[1].per_row_builds > pair[0].per_row_builds);
    }
    assert_eq!(
        [
            eager[0].per_row.total(),
            eager[1].per_row.total(),
            eager[2].per_row.total()
        ],
        [26_773, 315_343, 4_205_883],
        "the full substitution still counts what the evaluator counted before the change"
    );
}

// ---------------------------------------------------------------------------
// Answer: hand-pinned and independently read
// ---------------------------------------------------------------------------

/// A functional graph over `:n0 … :n4` along `:next`: the cycle `n0 → n1 → n2 → n0`,
/// and the tail `n4 → n3 → n0` into it.
const NEXT: [(&str, &str); 5] = [
    ("n0", "n1"),
    ("n1", "n2"),
    ("n2", "n0"),
    ("n3", "n0"),
    ("n4", "n3"),
];

fn next_graph() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let next = b.intern_iri(&iri("next"));
    for (from, to) in NEXT {
        let from = b.intern_iri(&iri(from));
        let to = b.intern_iri(&iri(to));
        b.push_quad(from, next, to, None);
    }
    b.freeze().expect("freeze")
}

/// `depth` levels of alternating `FILTER EXISTS` / `FILTER NOT EXISTS`, level `k` stepping
/// along `:next` from the node level `k - 1` reached and requiring the step not to land on
/// the outermost node `?x0`. Every level's answer depends on a binding of the level around
/// it (`?x{k-1}`, a triple position) and on the outermost row (`?x0`, an expression
/// position `depth` levels down): dropping or misplacing either substitution changes the
/// rows.
fn alternating_walk(depth: usize) -> String {
    let mut body = String::new();
    for k in (1..=depth).rev() {
        let keyword = if k % 2 == 1 { "EXISTS" } else { "NOT EXISTS" };
        body = format!(
            "FILTER {keyword} {{ ?x{k} <{EX}next> ?x{next} FILTER(?x{next} != ?x0) {body} }}",
            next = k + 1
        );
    }
    format!("SELECT ?x0 WHERE {{ ?x0 <{EX}next> ?x1 {body} }}")
}

/// [`alternating_walk`] read directly over [`NEXT`], with no evaluator: the body at level
/// `k` holds for `(x0, x)` iff the step `x → y` exists, `y ≠ x0`, and level `k + 1`
/// (negated at even levels) holds for `(x0, y)`.
fn alternating_walk_by_hand(depth: usize) -> Vec<Vec<String>> {
    fn next(x: &str) -> Option<&'static str> {
        NEXT.iter().find(|(from, _)| *from == x).map(|(_, to)| *to)
    }
    fn level(k: usize, depth: usize, x0: &str, x: &str) -> bool {
        if k > depth {
            return true;
        }
        let body = next(x).is_some_and(|y| y != x0 && level(k + 1, depth, x0, y));
        if k % 2 == 1 { body } else { !body }
    }
    let mut rows: Vec<Vec<String>> = NEXT
        .iter()
        .filter(|(x0, x1)| level(1, depth, x0, x1))
        .map(|(x0, _)| vec![iri(x0)])
        .collect();
    rows.sort();
    rows
}

/// **Nested `EXISTS`/`NOT EXISTS` at depths 1–6 answer the pinned rows.** Each depth is
/// checked three ways: against rows worked out by hand, against the independent reading
/// [`alternating_walk_by_hand`], and against the full substitution. The pinned rows differ
/// between depths, so an evaluation that dropped the outermost binding at depth (every
/// `?x0` comparison an error, every level false) or kept a stale one would be caught.
#[test]
fn alternating_nested_exists_answers_the_pinned_rows() {
    let ds = next_graph();
    let all = ["n0", "n1", "n2", "n3", "n4"];
    let cycle = ["n0", "n1", "n2"];
    let pinned: [(usize, &[&str]); 6] = [
        (1, &all),
        (2, &cycle),
        (3, &cycle),
        (4, &all),
        (5, &all),
        (6, &cycle),
    ];
    for (depth, expected) in pinned {
        let expected: Vec<Vec<String>> = expected.iter().map(|n| vec![iri(n)]).collect();
        assert_eq!(
            alternating_walk_by_hand(depth),
            expected,
            "the hand reading at depth {depth}"
        );
        let query = alternating_walk(depth);
        assert_eq!(
            answer_both_ways(&ds, &query),
            expected,
            "depth {depth}: {query}"
        );
    }
}

/// **Depth 200 answers, inside a governor bound.** Two hundred alternating levels over the
/// same graph, governed with a fuel ceiling linear in the depth, complete with the rows
/// the independent reading gives.
#[test]
fn two_hundred_nested_levels_answer_within_a_governor_bound() {
    const DEPTH: usize = 200;
    let (rows, evidence) = on_big_stack(|| {
        let ds = next_graph();
        let query = alternating_walk(DEPTH);
        // A hundred units of fuel a level, for five outer rows: a bound linear in the depth
        // (the evaluation spends under ten thousand in all).
        let governors = QueryGovernors::UNBOUNDED.with_fuel(100 * DEPTH as u64);
        let outcome = sequential_engine()
            .query_governed(
                &ds,
                SparqlRequest {
                    query: &query,
                    base_iri: None,
                    substitutions: &[],
                },
                crate::QueryOptions::EMPTY,
                &governors,
            )
            .expect("the governed evaluation answers");
        let GovernedOutcome::Complete {
            result, evidence, ..
        } = outcome
        else {
            panic!("two hundred levels must complete inside the bound: {outcome:?}");
        };
        (rows_of(&result), evidence)
    });
    eprintln!("depth {DEPTH}: {evidence:?}");
    assert_eq!(rows, alternating_walk_by_hand(DEPTH));
}

// ---------------------------------------------------------------------------
// Answer: the deferred substitution against the full one
// ---------------------------------------------------------------------------

/// A graph with IRIs, literals, a named graph and blank nodes, for the differential
/// corpus below.
fn mixed_graph() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&iri("p"));
    let q = b.intern_iri(&iri("q"));
    let r = b.intern_iri(&iri("r"));
    let label = b.intern_iri(&iri("label"));
    let g = b.intern_iri(&iri("g"));
    let a = b.intern_iri(&iri("a"));
    let bb = b.intern_iri(&iri("b"));
    let c = b.intern_iri(&iri("c"));
    let d = b.intern_iri(&iri("d"));
    let blank1 = b.intern_blank("b1", BlankScope::default());
    let blank2 = b.intern_blank("b2", BlankScope::default());
    let one = b.intern_literal(purrdf_core::RdfLiteral::simple("one"));
    let two = b.intern_literal(purrdf_core::RdfLiteral::simple("two"));
    for (s, o) in [
        (a, bb),
        (bb, c),
        (c, a),
        (d, a),
        (blank1, a),
        (blank2, blank1),
    ] {
        b.push_quad(s, p, o, None);
    }
    for (s, o) in [(a, c), (bb, blank1), (blank1, bb), (c, d)] {
        b.push_quad(s, q, o, None);
    }
    for (s, o) in [(a, one), (bb, two), (blank1, one)] {
        b.push_quad(s, label, o, None);
    }
    b.push_quad(a, r, bb, Some(g));
    b.push_quad(blank1, r, c, Some(g));
    b.freeze().expect("freeze")
}

/// **The deferred substitution answers what the full substitution answers**, over nested
/// bodies that meet every arm of the substitution walk a carried binding can reach: a
/// triple position, an expression, `BOUND`, `BIND`, `VALUES`, `OPTIONAL`, `MINUS`, a
/// sub-`SELECT` that does and one that does not project the variable, `GRAPH ?g`, a
/// `LATERAL` around a nested `EXISTS`, `EXISTS` inside `BIND` and inside `IF`, blank-node
/// bindings (which have no expression constant and travel as one-row `VALUES`), and
/// literal ones. Each query is also checked for a non-trivial answer, so agreement is not
/// the agreement of two empty results.
#[test]
fn deferred_substitution_matches_the_full_substitution() {
    let ds = mixed_graph();
    let p = format!("<{EX}p>");
    let q = format!("<{EX}q>");
    let r = format!("<{EX}r>");
    let label = format!("<{EX}label>");
    let queries = [
        // Triple positions, two levels down.
        format!(
            "SELECT ?a ?b WHERE {{ ?a {p} ?b FILTER EXISTS {{ ?b {p} ?c \
             FILTER EXISTS {{ ?c {p} ?a }} }} }}"
        ),
        // Expression positions only, through NOT EXISTS.
        format!(
            "SELECT ?a WHERE {{ ?a {p} ?b FILTER NOT EXISTS {{ ?x {q} ?y \
             FILTER(?x = ?b) FILTER NOT EXISTS {{ ?y {p} ?z FILTER(?z = ?a) }} }} }}"
        ),
        // BOUND of a carried variable, and of one the body binds.
        format!(
            "SELECT ?a WHERE {{ ?a {p} ?b FILTER EXISTS {{ ?b {q} ?c \
             FILTER EXISTS {{ OPTIONAL {{ ?c {label} ?l }} FILTER(BOUND(?a) && !BOUND(?l)) }} }} }}"
        ),
        // BIND and VALUES inside the nested body.
        format!(
            "SELECT ?a WHERE {{ ?a {p} ?b FILTER EXISTS {{ ?b {p} ?c \
             FILTER EXISTS {{ BIND(?a AS ?k) VALUES ?v {{ {label} {q} }} ?k ?v ?w }} }} }}"
        ),
        // MINUS inside the nested body.
        format!(
            "SELECT ?a WHERE {{ ?a {p} ?b FILTER EXISTS {{ ?b {p} ?c \
             FILTER NOT EXISTS {{ ?c {p} ?d MINUS {{ ?d {q} ?a }} }} }} }}"
        ),
        // A sub-SELECT that projects the carried variable, and one that hides it.
        format!(
            "SELECT ?a WHERE {{ ?a {p} ?b FILTER EXISTS {{ ?b {p} ?c \
             FILTER EXISTS {{ SELECT ?a WHERE {{ ?a {q} ?e }} }} }} }}"
        ),
        format!(
            "SELECT ?a WHERE {{ ?a {p} ?b FILTER EXISTS {{ ?b {p} ?c \
             FILTER EXISTS {{ SELECT ?c WHERE {{ ?a {q} ?c }} }} }} }}"
        ),
        // GRAPH ?g, the graph variable carried from the outer row.
        format!(
            "SELECT ?a ?g WHERE {{ GRAPH ?g {{ ?a {r} ?b }} FILTER EXISTS {{ ?b {p} ?c \
             FILTER EXISTS {{ GRAPH ?g {{ ?x {r} ?c }} }} }} }}"
        ),
        // A LATERAL whose right side holds a nested EXISTS correlated with its left.
        format!(
            "SELECT ?a ?c WHERE {{ ?a {p} ?b LATERAL {{ ?b {p} ?c \
             FILTER EXISTS {{ ?c {p} ?d FILTER EXISTS {{ ?d {q} ?e FILTER(?e != ?a) }} }} }} }}"
        ),
        // EXISTS inside BIND and inside IF, nested.
        format!(
            "SELECT ?a ?f WHERE {{ ?a {p} ?b BIND(EXISTS {{ ?b {p} ?c \
             FILTER(IF(EXISTS {{ ?c {q} ?a }}, true, false)) }} AS ?f) }}"
        ),
        // Blank-node bindings reaching an expression position only, two levels down.
        format!(
            "SELECT ?a WHERE {{ ?a {p} ?b FILTER(isBlank(?a)) FILTER EXISTS {{ ?b {label} ?l \
             FILTER EXISTS {{ ?x {label} ?l FILTER(!sameTerm(?x, ?a)) }} }} }}"
        ),
        // Literal bindings carried into a triple position.
        format!(
            "SELECT ?a ?l WHERE {{ ?a {label} ?l FILTER EXISTS {{ ?a {p} ?b \
             FILTER NOT EXISTS {{ ?b {label} ?l }} }} }}"
        ),
        // Four levels, each binding a new variable the innermost reads.
        format!(
            "SELECT ?a WHERE {{ ?a {p} ?b FILTER EXISTS {{ ?b {p} ?c \
             FILTER EXISTS {{ ?c {p} ?d FILTER EXISTS {{ ?d {p} ?e \
             FILTER(?e = ?a || ?e = ?b) }} }} }} }}"
        ),
    ];
    for query in &queries {
        let rows = on_big_stack({
            let ds = Arc::clone(&ds);
            let query = query.clone();
            move || answer_both_ways(&ds, &query)
        });
        assert!(
            !rows.is_empty(),
            "the corpus query must answer something: {query}"
        );
    }
}

// ---------------------------------------------------------------------------
// Answer: layers that disagree
// ---------------------------------------------------------------------------

fn var(name: &str) -> Variable {
    Variable::new(name)
}

fn triple(s: &str, p: &str, o: &str) -> GraphPattern {
    GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(var(s)),
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri(p))),
            object: TermPattern::Variable(var(o)),
        }],
    }
}

fn filter_exists(inner: GraphPattern, body: GraphPattern) -> GraphPattern {
    GraphPattern::Filter {
        expr: Expression::Exists(Box::new(body)),
        inner: Box::new(inner),
    }
}

/// **Layers that disagree are substituted one at a time, as before.** Hand-built algebra
/// (the parser refuses it): the second level's body rebinds `?v` with `BIND`, after the
/// first level bound it from the data. The third level is then owed `?v` twice, with two
/// values — the collision SEP-0007 leaves undefined — and the deferred evaluation must
/// fall back to the full, layer-by-layer substitution and answer what it answers. The data
/// gives the first level `?v = :k` for `:x1` (the layers agree) and `?v = :other` for `:x2`
/// (they disagree), so the fallback is observed on a row where its answer differs from a
/// merged row's: the full substitution joins the third level with both values and finds
/// nothing for `:x2`.
#[test]
fn disagreeing_layers_answer_as_the_full_substitution_does() {
    let mut b = RdfDatasetBuilder::new();
    let tag = b.intern_iri(&iri("tag"));
    let q = b.intern_iri(&iri("q"));
    let r = b.intern_iri(&iri("r"));
    let x1 = b.intern_iri(&iri("x1"));
    let x2 = b.intern_iri(&iri("x2"));
    let k = b.intern_iri(&iri("k"));
    let other = b.intern_iri(&iri("other"));
    let z = b.intern_iri(&iri("z"));
    b.push_quad(x1, tag, x1, None);
    b.push_quad(x2, tag, x2, None);
    b.push_quad(x1, q, k, None);
    b.push_quad(x2, q, other, None);
    b.push_quad(k, r, z, None);
    b.push_quad(other, r, z, None);
    let ds: Arc<RdfDataset> = b.freeze().expect("freeze");

    // ?x :tag ?w FILTER EXISTS {
    //   ?y :q ?v FILTER(?y = ?x)
    //   FILTER EXISTS { BIND(:k AS ?v) FILTER EXISTS { ?v :r ?z } } }
    //
    // The first level compares the outer ?x in an expression over a variable its own body
    // binds, which keeps it off the memoized probe: the probe would evaluate the body as
    // written, outside any substituted window, where the second level's own admission
    // check refuses the rebinding before anything is substituted.
    let third = triple("v", "r", "z");
    let second = filter_exists(
        GraphPattern::Extend {
            inner: Box::new(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
            variable: var("v"),
            expression: Expression::NamedNode(NamedNode::new_unchecked(iri("k"))),
        },
        third,
    );
    let first = filter_exists(
        GraphPattern::Filter {
            expr: Expression::Equal(
                Box::new(Expression::Variable(var("y"))),
                Box::new(Expression::Variable(var("x"))),
            ),
            inner: Box::new(triple("y", "q", "v")),
        },
        second,
    );
    let outer = filter_exists(triple("x", "tag", "w"), first);
    let query = Query::Select {
        pattern: GraphPattern::Project {
            inner: Box::new(outer),
            variables: vec![var("x")],
        },
        dataset: QueryDataset::default(),
        base_iri: None,
        version: None,
    };
    let prepared = crate::PreparedQuery::rewritten(query, crate::QueryOptions::EMPTY)
        .expect("no registry to fingerprint");
    let run = |eager: bool| {
        let _eager = eager.then(force_eager_substitution_for_test);
        let result = sequential_engine()
            .query_prepared(&ds, &prepared, &[], crate::QueryOptions::EMPTY)
            .expect("evaluate");
        rows_of(&result)
    };
    let eager = run(true);
    assert_eq!(
        eager,
        vec![vec![iri("x1")]],
        "the full substitution keeps :x1 (its layers agree) and drops :x2 (they do not)"
    );
    assert_eq!(run(false), eager);
}

// ---------------------------------------------------------------------------
// Answer: the charge ledger's own evaluation
// ---------------------------------------------------------------------------

/// **An explained evaluation answers each outer row with its own substitution.** Under a
/// charge ledger the substitution walk is tracked, and a nested `EXISTS` site used to be
/// prepared once from the FIRST outer row's substituted copy and reused for every later
/// row — with the first row's constants baked in. The nested body here compares against
/// the outer `?a` in an expression, so reusing the first row's copy answers every row as
/// `:s1`: three rows where the query has two. The explanation's root reports the rows the
/// evaluation produced, and must agree with the ordinary evaluation.
#[test]
fn an_explained_nested_exists_substitutes_each_outer_row() {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&iri("p"));
    let q = b.intern_iri(&iri("q"));
    let y = b.intern_iri(&iri("y"));
    for i in 1..=3 {
        let s = b.intern_iri(&format!("{EX}s{i}"));
        let o = b.intern_iri(&format!("{EX}o{i}"));
        b.push_quad(s, p, o, None);
        if i < 3 {
            b.push_quad(s, q, y, None);
        }
    }
    let ds: Arc<RdfDataset> = b.freeze().expect("freeze");
    let query = format!(
        "PREFIX : <{EX}> SELECT ?a WHERE {{ VALUES ?a {{ :s1 :s2 :s3 }} \
         FILTER EXISTS {{ ?s :p ?o FILTER(?s = ?a) \
         FILTER EXISTS {{ ?x :q ?y FILTER(?x = ?a) }} }} }}"
    );
    let rows = answer(&ds, &query, false);
    assert_eq!(rows, vec![vec![iri("s1")], vec![iri("s2")]]);
    let explanation = sequential_engine()
        .explain_query(&ds, &query, None)
        .expect("explain");
    let root = explanation.ledger().first().expect("a root node");
    assert_eq!(
        root.rows,
        2,
        "the explained evaluation must produce the query's two rows: {}",
        explanation.render()
    );
}
