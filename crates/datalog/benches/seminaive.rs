// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Bench targets are not public API: `criterion_group!` expands to a `pub fn`,
// which would otherwise trip the workspace `missing_docs` lint.
#![allow(missing_docs)]

//! Semi-naive join benchmarks.
//!
//! The evaluator had no bench of its own — the nearest was `purrdf-entail`'s chase
//! bench, which drives it through a regime's rule table and therefore measures the
//! rule table too. These three drive `compile` + `evaluate` directly, so a change to
//! the join has a number that belongs to the join.
//!
//! Each isolates a different axis of the per-solution cost, because a join's price
//! is not one number:
//!
//! * `fanout` — a two-atom body over a dense middle column. The intermediate relation
//!   is `|p| * |q| / |m|` solutions wide, so this is the fan-out the binary join
//!   materializes and the only one of the three whose cost is dominated by the count
//!   of intermediate solutions rather than by their shape.
//! * `frame_width` — a chain body from 2 to 12 atoms over an EDB holding exactly one
//!   chain, so precisely one solution survives at every level. The intermediate COUNT
//!   is therefore pinned at one and the fan-out this measures is nil; what grows is
//!   the number of join levels and the width of the frame each level copies. Those
//!   two are the PER-SOLUTION cost, which is the half a flattening change moves —
//!   deliberately separated here from the per-count cost `fanout` measures, because a
//!   single figure mixing them cannot say which half a change moved.
//! * `recursion` — transitive closure over a chain, so the fixpoint runs many rounds
//!   and the delta scan is exercised rather than a single pass.
//!
//! Report-only, per this repository's rule: benches exist so a later change has a
//! number to move, never so a speedup can be asserted. Nothing here fails a build.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause};
use purrdf_datalog::seminaive::{Evaluation, compile, evaluate};
use purrdf_datalog::store::RelationStore;

/// The fixture namespace. A bench mints no vocabulary of its own, and a
/// reserved-for-documentation authority is the only one it may put in a term.
const EX: &str = "https://example.org/";

/// The lexical surface an IRI is stored under.
fn surface(name: &str) -> String {
    format!("<{EX}{name}>")
}

fn var(name: &str) -> ClauseTerm {
    ClauseTerm::var(name)
}

/// `subject predicate object` with both terminals as variables.
fn atom(subject: &str, predicate: &str, object: &str) -> ClauseAtom {
    ClauseAtom::positive(var(subject), format!("{EX}{predicate}"), var(object))
}

/// Run one program to fixpoint, panicking rather than reporting a partial answer —
/// a bench that silently measured a budget refusal would be measuring nothing.
fn run(rules: Vec<DlClause>, edb: RelationStore) -> Evaluation {
    let exe = compile(rules).expect("the fixture program compiles");
    evaluate(&exe, edb).expect("the fixture program stays inside every ceiling")
}

/// `p(s_i, m_j)` and `q(m_j, o_k)` over `width` middles, so the join's intermediate
/// relation is `width * width` solutions before the head is formed.
fn fanout_store(width: usize) -> RelationStore {
    let mut store = RelationStore::new();
    for i in 0..width {
        for j in 0..width {
            store.insert(
                &surface(&format!("s{i}")),
                &surface("p"),
                &surface(&format!("m{j}")),
                RelationStore::DEFAULT_GRAPH,
            );
            store.insert(
                &surface(&format!("m{i}")),
                &surface("q"),
                &surface(&format!("o{j}")),
                RelationStore::DEFAULT_GRAPH,
            );
        }
    }
    store
}

/// `r(?s, ?o) :- p(?s, ?m), q(?m, ?o)` — the two-atom body whose fan-out the binary
/// join materializes.
fn fanout_rules() -> Vec<DlClause> {
    vec![DlClause::datalog(
        atom("?s", "r", "?o"),
        vec![atom("?s", "p", "?m"), atom("?m", "q", "?o")],
    )]
}

/// A body of `pairs` atoms chained through distinct variables, so the frame carries
/// `pairs + 1` variables and exactly one solution survives.
///
/// The EDB holds one chain, so the answer set is a single row at every width. That is
/// the point: the count is fixed and only the frame's width moves.
fn frame_rules(pairs: usize) -> Vec<DlClause> {
    let body: Vec<ClauseAtom> = (0..pairs)
        .map(|i| atom(&format!("?v{i}"), "p", &format!("?v{}", i + 1)))
        .collect();
    vec![DlClause::datalog(
        atom("?v0", "r", &format!("?v{pairs}")),
        body,
    )]
}

fn frame_store(pairs: usize) -> RelationStore {
    let mut store = RelationStore::new();
    for i in 0..pairs {
        store.insert(
            &surface(&format!("n{i}")),
            &surface("p"),
            &surface(&format!("n{}", i + 1)),
            RelationStore::DEFAULT_GRAPH,
        );
    }
    store
}

/// `edge` chain of `n` links, closed transitively — `n * (n + 1) / 2` derived facts
/// over `n` rounds.
fn chain_store(n: usize) -> RelationStore {
    let mut store = RelationStore::new();
    for i in 0..n {
        store.insert(
            &surface(&format!("n{i}")),
            &surface("p"),
            &surface(&format!("n{}", i + 1)),
            RelationStore::DEFAULT_GRAPH,
        );
    }
    store
}

fn closure_rules() -> Vec<DlClause> {
    vec![
        DlClause::datalog(atom("?s", "q", "?o"), vec![atom("?s", "p", "?o")]),
        DlClause::datalog(
            atom("?s", "q", "?o"),
            vec![atom("?s", "p", "?m"), atom("?m", "q", "?o")],
        ),
    ]
}

fn fanout(c: &mut Criterion) {
    let mut group = c.benchmark_group("seminaive_fanout");
    for width in [8_usize, 24, 48] {
        let store = fanout_store(width);
        group.bench_with_input(BenchmarkId::from_parameter(width), &width, |bch, _| {
            bch.iter(|| run(fanout_rules(), store.clone()));
        });
    }
    group.finish();
}

fn frame_width(c: &mut Criterion) {
    let mut group = c.benchmark_group("seminaive_frame_width");
    for pairs in [2_usize, 6, 12] {
        let store = frame_store(pairs);
        group.bench_with_input(BenchmarkId::from_parameter(pairs), &pairs, |bch, _| {
            bch.iter(|| run(frame_rules(pairs), store.clone()));
        });
    }
    group.finish();
}

fn recursion(c: &mut Criterion) {
    let mut group = c.benchmark_group("seminaive_recursion");
    for n in [16_usize, 48, 96] {
        let store = chain_store(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |bch, _| {
            bch.iter(|| run(closure_rules(), store.clone()));
        });
    }
    group.finish();
}

criterion_group!(benches, fanout, frame_width, recursion);
criterion_main!(benches);
