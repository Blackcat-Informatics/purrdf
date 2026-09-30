// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Nested `LATERAL`: what one left row costs, counted, and what the answer is.
//!
//! [`crate::deferred_exists`] leaves a nested `LATERAL`'s right operand out of the per-row
//! copy of the pattern around it and substitutes it when the copied `LATERAL` evaluates
//! it. Two claims follow, each checked against an observer that could tell it from its
//! failure:
//!
//! * **Cost.** One more left row costs work proportional to the nesting depth, and the
//!   once-per-evaluation preparation copies the chain once: counted in tree nodes
//!   substituted and copied ([`crate::op_count`]), never in time. The same count taken
//!   with every operand substituted in full
//!   ([`crate::deferred_exists::force_eager_substitution_for_test`], the evaluation as it
//!   was before) grows cubically, so the count can see the difference.
//! * **Answer.** The deferred evaluation answers exactly what the full substitution
//!   answers: against rows read off the data by hand, and against the full substitution
//!   itself over generated chains one to six levels deep — varied left cardinalities, an
//!   empty left side, unbound variables, `OPTIONAL`, `FILTER` and `UNION` inside — run
//!   sequentially and forked, and under a charge ledger.

use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlResult, TermValue};
use purrdf_sparql_algebra::{
    Child, Expression, GraphPattern, NamedNode, NamedNodePattern, Query, QueryDataset, TermPattern,
    TriplePattern, Variable,
};

use crate::deferred_exists::force_eager_substitution_for_test;
use crate::engine::NativeSparqlEngine;
use crate::eval::EvalOptions;
use crate::op_count::OpCounts;

const EX: &str = "http://example.org/";

/// A stack large enough for every depth these tests evaluate, eager included.
const BIG_STACK: usize = 512 * 1024 * 1024;

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

/// `query` over `ds`, evaluated sequentially: deferred, or with every nested operand
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

/// The rows the root of `query`'s charge ledger reports, and the rows the explained
/// evaluation answered: the tracked substitution's own evaluation.
fn explained_rows(ds: &Arc<RdfDataset>, query: &str) -> (u64, usize) {
    let explanation = sequential_engine()
        .explain_query(ds, query, None)
        .unwrap_or_else(|e| panic!("explain {query}: {e}"));
    let root = explanation.ledger().first().expect("a root node");
    (root.rows, answer(ds, query, false).len())
}

/// `query` over `ds`, deferred (sequentially, forked and explained), checked against the
/// full substitution; returns the rows.
fn answer_every_way(ds: &Arc<RdfDataset>, query: &str) -> Vec<Vec<String>> {
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
    let (ledger_rows, rows) = explained_rows(ds, query);
    assert_eq!(
        usize::try_from(ledger_rows).expect("row count fits"),
        rows,
        "the explained evaluation's root must report the rows the query answers: {query}"
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

/// `depth` nested `LATERAL`s, every level re-reading `?s <p> ?o` for the row around it —
/// the shape whose cost grew with the cube of `depth`.
fn same_row_nesting(depth: usize) -> String {
    let mut body = format!("?s <{EX}p> ?o");
    for _ in 0..depth {
        body = format!("?s <{EX}p> ?o LATERAL {{ {body} }}");
    }
    format!("SELECT ?s WHERE {{ {body} }}")
}

/// The counters an evaluation of `query` over `ds` bumped, and its rows.
fn counted(ds: &Arc<RdfDataset>, query: &str, eager: bool) -> (Vec<Vec<String>>, OpCounts) {
    let engine = sequential_engine();
    let prepared = engine.prepare_query(query, None).expect("prepare");
    let _eager = eager.then(force_eager_substitution_for_test);
    crate::op_count::reset();
    let result = engine
        .query_prepared(ds, &prepared, &[], crate::QueryOptions::EMPTY)
        .expect("evaluate");
    let counts = crate::op_count::read();
    (rows_of(&result), counts)
}

/// What one left row costs at `depth` (the counts of a two-row evaluation minus those of
/// a one-row one), and the one-row evaluation's own counts.
#[derive(Debug, Clone, Copy)]
struct Cost {
    per_row: OpCounts,
    once: OpCounts,
}

fn cost(depth: usize, eager: bool) -> Cost {
    purrdf_stack::on_stack(BIG_STACK, move || {
        let query = same_row_nesting(depth);
        let (one, c1) = counted(&edge_per_subject(1), &query, eager);
        assert_eq!(one, vec![vec![iri("s0")]]);
        let (two, c2) = counted(&edge_per_subject(2), &query, eager);
        assert_eq!(two, vec![vec![iri("s0")], vec![iri("s1")]]);
        Cost {
            per_row: OpCounts {
                substituted: c2.substituted - c1.substituted,
                cloned: c2.cloned - c1.cloned,
                analyzed: c2.analyzed - c1.analyzed,
                var_walked: c2.var_walked - c1.var_walked,
            },
            once: c1,
        }
    })
    .expect("spawn")
}

/// **One more left row of `d` nested `LATERAL`s costs `O(d)` counted work, and the
/// evaluation copies the chain once.**
///
/// Per extra left row, deferred: `2d - 1` substituted nodes — each nested level's own
/// `LATERAL` and left leaf, and the innermost leaf — and nothing copied. Once per
/// evaluation, the operand the outermost level defers is copied once to be cut into its
/// sites: `2d - 3` nodes. Measured:
///
/// | depth | per-row work, deferred | per-row work, full substitution (before) |
/// |------:|-----------------------:|-----------------------------------------:|
/// |    20 |                     39 |                                    3,060 |
/// |    40 |                     79 |                                   22,920 |
/// |    80 |                    159 |                                  177,040 |
///
/// The full substitution's count is the evaluator before this change: it copies every
/// remaining level at every level, each leaf wrapped in one more `VALUES` join per level
/// above it, so each doubling of the depth multiplies it by seven to eight — cubic —
/// where the deferred count doubles.
#[test]
fn nested_lateral_per_row_work_is_linear_in_depth() {
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
                substituted: 2 * d - 1,
                cloned: 0,
                analyzed: 0,
                var_walked: 0,
            },
            "one more left row at depth {depth} must substitute each level's own nodes once \
             and copy, analyse and walk nothing else"
        );
        assert_eq!(
            cost.once.cloned,
            2 * d - 3,
            "the deferred operand is copied exactly once per evaluation at depth {depth}"
        );
        assert_eq!(
            cost.once.substituted, cost.per_row.substituted,
            "the first left row substitutes exactly what every later one does; only the \
             cut is paid once"
        );
    }

    // The observer can see the difference: the same count, taken over the full
    // substitution, grows by far more than double per doubling of the depth.
    for pair in eager.windows(2) {
        assert!(
            pair[1].per_row.total() > 6 * pair[0].per_row.total(),
            "the full substitution's per-row work must grow super-linearly (cubically) with \
             depth, or this test could not tell the two apart: {pair:?}"
        );
    }
    assert_eq!(
        [
            eager[0].per_row.total(),
            eager[1].per_row.total(),
            eager[2].per_row.total()
        ],
        [3_060, 22_920, 177_040],
        "the full substitution still counts what the evaluator counted before the change"
    );
}

// ---------------------------------------------------------------------------
// Answer: read by hand
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

/// `depth` nested `LATERAL`s, level `k` stepping along `:next` from the node level `k - 1`
/// reached and requiring the step not to land on the outermost node `?x0`: every level
/// reads a binding of the level around it (a triple position) and the outermost row (an
/// expression position `depth` levels down).
fn walk_avoiding_start(depth: usize) -> String {
    let mut body = String::new();
    for k in (1..=depth).rev() {
        body = format!(
            "LATERAL {{ ?x{k} <{EX}next> ?x{next} FILTER(?x{next} != ?x0) {body} }}",
            next = k + 1
        );
    }
    format!(
        "SELECT ?x0 ?x{last} WHERE {{ ?x0 <{EX}next> ?x1 {body} }}",
        last = depth + 1
    )
}

/// [`walk_avoiding_start`] read directly over [`NEXT`], with no evaluator: the walk from
/// each `x0` that takes `depth + 1` steps, none after the first landing on `x0`.
fn walk_avoiding_start_by_hand(depth: usize) -> Vec<Vec<String>> {
    fn next(x: &str) -> Option<&'static str> {
        NEXT.iter().find(|(from, _)| *from == x).map(|(_, to)| *to)
    }
    let mut rows = Vec::new();
    for (x0, x1) in NEXT {
        let mut at = x1;
        let mut reached = true;
        for _ in 0..depth {
            match next(at) {
                Some(step) if step != x0 => at = step,
                _ => {
                    reached = false;
                    break;
                }
            }
        }
        if reached {
            rows.push(vec![iri(x0), iri(at)]);
        }
    }
    rows.sort();
    rows
}

/// **Nested `LATERAL`s at depths 1–6 answer the rows read by hand.** Each depth is
/// checked against rows pinned here, against the independent reading
/// [`walk_avoiding_start_by_hand`], and against the full substitution. The pinned rows
/// differ between depths, so an evaluation that dropped the outermost binding at depth,
/// or kept a stale one, would be caught.
#[test]
fn walks_along_next_answer_the_rows_read_by_hand() {
    let ds = next_graph();
    let pinned: [(usize, &[(&str, &str)]); 6] = [
        (
            1,
            &[
                ("n0", "n2"),
                ("n1", "n0"),
                ("n2", "n1"),
                ("n3", "n1"),
                ("n4", "n0"),
            ],
        ),
        (2, &[("n3", "n2"), ("n4", "n1")]),
        (3, &[("n3", "n0"), ("n4", "n2")]),
        (4, &[("n3", "n1"), ("n4", "n0")]),
        (5, &[("n3", "n2"), ("n4", "n1")]),
        (6, &[("n3", "n0"), ("n4", "n2")]),
    ];
    for (depth, expected) in pinned {
        let expected: Vec<Vec<String>> = expected
            .iter()
            .map(|(x0, last)| vec![iri(x0), iri(last)])
            .collect();
        assert_eq!(
            walk_avoiding_start_by_hand(depth),
            expected,
            "the hand reading at depth {depth}"
        );
        let query = walk_avoiding_start(depth);
        assert_eq!(
            answer_every_way(&ds, &query),
            expected,
            "depth {depth}: {query}"
        );
    }
}

/// **A thousand nested `LATERAL`s answer the row the shape has.** The bench's shape over
/// four subjects, only the first of which has a `<q>`: the answer is that subject alone,
/// at any depth.
#[test]
fn a_thousand_nested_laterals_answer() {
    let rows = purrdf_stack::on_stack(BIG_STACK, || {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri(&iri("p"));
        let q = b.intern_iri(&iri("q"));
        for n in 1..=4 {
            let s = b.intern_iri(&iri(&format!("s{n}")));
            let o = b.intern_literal(RdfLiteral::simple(n.to_string()));
            b.push_quad(s, p, o, None);
            if n == 1 {
                b.push_quad(s, q, o, None);
            }
        }
        let ds: Arc<RdfDataset> = b.freeze().expect("freeze");
        let mut body = format!("?s <{EX}q> ?z");
        for _ in 0..1000 {
            body = format!("?s <{EX}p> ?o LATERAL {{ {body} }}");
        }
        answer(&ds, &format!("SELECT ?s WHERE {{ {body} }}"), false)
    })
    .expect("spawn");
    assert_eq!(rows, vec![vec![iri("s1")]]);
}

// ---------------------------------------------------------------------------
// Answer: the deferred substitution against the full one, generated
// ---------------------------------------------------------------------------

/// A graph with IRIs, literals, a named graph and blank nodes.
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
    let one = b.intern_literal(RdfLiteral::simple("one"));
    let two = b.intern_literal(RdfLiteral::simple("two"));
    for (s, o) in [
        (a, bb),
        (bb, c),
        (c, a),
        (d, a),
        (blank1, a),
        (blank2, blank1),
        (a, c),
    ] {
        b.push_quad(s, p, o, None);
    }
    for (s, o) in [(a, c), (bb, blank1), (blank1, bb), (c, d), (d, d)] {
        b.push_quad(s, q, o, None);
    }
    for (s, o) in [(a, one), (bb, two), (blank1, one), (d, two)] {
        b.push_quad(s, label, o, None);
    }
    b.push_quad(a, r, bb, Some(g));
    b.push_quad(blank1, r, c, Some(g));
    b.freeze().expect("freeze")
}

/// A deterministic choice source: SplitMix64 over a seed.
struct Choices(purrdf_testkit::rng::SplitMix64);

impl Choices {
    fn pick(&mut self, n: u64) -> u64 {
        self.0.below(n)
    }
}

/// The left side of the outermost `LATERAL`: every cardinality from none to all of the
/// `:p` edges, and rows that leave `?v1` or `?l` unbound.
fn outer(choices: &mut Choices) -> String {
    match choices.pick(6) {
        // Every `:p` edge.
        0 => format!("?v0 <{EX}p> ?v1"),
        // No row at all.
        1 => format!("?v0 <{EX}none> ?v1"),
        // One row.
        2 => format!("VALUES (?v0 ?v1) {{ (<{EX}a> <{EX}b>) }}"),
        // Rows, one of which leaves `?v1` unbound.
        3 => format!("VALUES (?v0 ?v1) {{ (<{EX}a> <{EX}b>) (<{EX}c> <{EX}a>) (<{EX}d> UNDEF) }}"),
        // Every `:p` edge, `?l` bound only where the object has a label.
        4 => format!("?v0 <{EX}p> ?v1 OPTIONAL {{ ?v1 <{EX}label> ?l }}"),
        // Every `:q` edge, a literal-bearing row set.
        _ => format!("?v0 <{EX}q> ?v1 OPTIONAL {{ ?v0 <{EX}label> ?l }}"),
    }
}

/// Level `k`'s own group, the level below it (if any) nested after it: a correlated step
/// from `?v{k}`, and around it a choice of `OPTIONAL`, `FILTER` and `UNION`, several of
/// which read the outermost row or a variable left unbound above.
fn level(choices: &mut Choices, k: usize, below: &str) -> String {
    let (here, next) = (format!("?v{k}"), format!("?v{}", k + 1));
    let step = match choices.pick(4) {
        0 => format!("{here} <{EX}p> {next}"),
        1 => format!("{here} <{EX}q> {next}"),
        2 => format!("{{ {here} <{EX}p> {next} }} UNION {{ {here} <{EX}q> {next} }}"),
        _ => format!("GRAPH ?g{k} {{ {here} <{EX}r> {next} }}"),
    };
    let optional = match choices.pick(3) {
        0 => format!(" OPTIONAL {{ {next} <{EX}label> ?m{k} }}"),
        1 => format!(" OPTIONAL {{ {next} <{EX}q> ?m{k} FILTER({next} != ?v0) }}"),
        _ => String::new(),
    };
    let filter = match choices.pick(6) {
        0 => format!(" FILTER({next} != ?v0)"),
        1 => " FILTER(!BOUND(?l) || ?l != \"two\")".to_owned(),
        2 => format!(" FILTER(isIRI({next}) || isBlank({next}))"),
        3 => format!(" FILTER(!sameTerm({next}, {here}))"),
        4 => " FILTER(BOUND(?v1))".to_owned(),
        _ => String::new(),
    };
    let below = if below.is_empty() {
        String::new()
    } else {
        format!(" LATERAL {{ {below} }}")
    };
    format!("{step}{optional}{filter}{below}")
}

/// A generated chain `depth` levels deep, below [`outer`].
fn generated(seed: u64, depth: usize) -> String {
    let mut choices = Choices(purrdf_testkit::rng::SplitMix64::new(seed));
    let outer = outer(&mut choices);
    let mut body = String::new();
    for k in (1..=depth).rev() {
        body = level(&mut choices, k, &body);
    }
    format!("SELECT * WHERE {{ {outer} LATERAL {{ {body} }} }}")
}

/// **The deferred substitution answers what the full substitution answers** over
/// generated chains one to six levels deep: every left cardinality from none upwards,
/// rows with unbound variables, `OPTIONAL`, `FILTER`, `UNION` and `GRAPH ?g` inside, and
/// levels that read the outermost row. Each is also run forked and explained. Enough of
/// them answer something that the agreement is not the agreement of empty results.
#[test]
fn generated_nested_laterals_match_the_full_substitution() {
    let ds = mixed_graph();
    let mut queries = 0_usize;
    let mut nonempty = 0_usize;
    let mut empty_outer = 0_usize;
    for depth in 1..=6 {
        for seed in 0..24_u64 {
            let query = generated(
                seed * 7919 + u64::try_from(depth).expect("depth fits"),
                depth,
            );
            let rows = purrdf_stack::on_stack(BIG_STACK, {
                let ds = Arc::clone(&ds);
                let query = query.clone();
                move || answer_every_way(&ds, &query)
            })
            .expect("spawn");
            queries += 1;
            nonempty += usize::from(!rows.is_empty());
            empty_outer += usize::from(query.contains("none"));
        }
    }
    eprintln!("{queries} queries, {nonempty} with rows, {empty_outer} over an empty left side");
    assert!(
        nonempty * 3 >= queries,
        "at least a third of the generated chains must answer something ({nonempty} of \
         {queries})"
    );
    assert!(empty_outer > 0, "the generator reaches an empty left side");
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

/// **Layers that disagree are substituted one at a time, as before.** Hand-built algebra
/// (the parser refuses it): the nested `LATERAL`'s left side rebinds `?v` with `BIND`,
/// after the outer left side bound it from the data. Its right side is then owed `?v`
/// twice, with two values — the collision SEP-0007 leaves undefined — and the deferred
/// evaluation must fall back to the layer-by-layer substitution and answer what the full
/// substitution answers. The data gives `:x1` the value the `BIND` writes (the layers
/// agree) and `:x2` another (they do not), so the fallback is observed on a row where its
/// answer differs from a merged row's.
#[test]
fn disagreeing_layers_answer_as_the_full_substitution_does() {
    let mut b = RdfDatasetBuilder::new();
    let q = b.intern_iri(&iri("q"));
    let r = b.intern_iri(&iri("r"));
    let x1 = b.intern_iri(&iri("x1"));
    let x2 = b.intern_iri(&iri("x2"));
    let k = b.intern_iri(&iri("k"));
    let other = b.intern_iri(&iri("other"));
    let z = b.intern_iri(&iri("z"));
    b.push_quad(x1, q, k, None);
    b.push_quad(x2, q, other, None);
    b.push_quad(k, r, z, None);
    b.push_quad(other, r, z, None);
    let ds: Arc<RdfDataset> = b.freeze().expect("freeze");

    // ?x :q ?v LATERAL { BIND(:k AS ?v) LATERAL { ?v :r ?z } }
    let nested = GraphPattern::Lateral {
        left: Child::new(GraphPattern::Extend {
            inner: Child::new(GraphPattern::Bgp {
                patterns: Vec::new(),
            }),
            variable: var("v"),
            expression: Expression::NamedNode(NamedNode::new_unchecked(iri("k"))),
        }),
        right: Child::new(triple("v", "r", "z")),
    };
    let outer = GraphPattern::Lateral {
        left: Child::new(triple("x", "q", "v")),
        right: Child::new(nested),
    };
    let query = Query::Select {
        pattern: GraphPattern::Project {
            inner: Child::new(outer),
            variables: vec![var("x"), var("z")],
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
        vec![vec![iri("x1"), iri("z")]],
        "the full substitution keeps :x1 (its layers agree) and drops :x2 (they do not)"
    );
    assert_eq!(run(false), eager);
}
