// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Synthetic Datalog programs whose answers are known ANALYTICALLY.
//!
//! # Why this exists
//!
//! "The output is byte-identical across a hundred runs" tests *determinism*, not
//! *correctness*: a systematically wrong evaluator passes that check a hundred times out
//! of a hundred. The corpus below closes that gap. Each workload is one classic
//! relational-core family — transitive closure, strongly connected components, same
//! generation, single-source reachability — over a graph whose shape is chosen so the
//! derived relation has a CLOSED FORM. The expected fact set is built here by that
//! construction, never by running the evaluator, so
//! `synth_corpus_matches_its_analytic_goldens` compares a formula against an engine rather
//! than an engine against itself.
//!
//! Every workload also carries `expected_rows`, the closed-form COUNT, checked alongside
//! the set: a construction bug that produced the wrong golden set would have to also
//! produce a matching wrong count to slip through both.
//!
//! # Why it is test-only
//!
//! These are fixtures, not substrate. Making them public would ship graph generators in a
//! released crate's API and documentation for the sake of one test module, so the corpus
//! is `#[cfg(test)]`, exactly like [`crate::test_support`]. A future benchmark or
//! conformance harness that needs them can lift the `cfg` deliberately rather than inherit
//! a public surface nobody asked for.
//!
//! # Determinism
//!
//! Every generator is pure: identical `n` yields an identical rule program, an identical
//! triple sequence (pushed in a fixed order) and an identical golden.

use std::collections::BTreeSet;

use crate::clause::{ClauseAtom, ClauseTerm, DlClause};
use crate::store::{Fact, RelationStore};

/// The IRI namespace root for synthetic nodes, predicates and rule names.
const BASE: &str = "https://example.org/synth";

/// One synthetic workload: a rule program, its EDB triples and the analytic golden.
#[derive(Debug, Clone)]
pub(crate) struct SynthWorkload {
    /// A short name, used in assertion messages.
    pub(crate) name: &'static str,
    /// The rule program, in authored order.
    pub(crate) rules: Vec<DlClause>,
    /// The EDB triples as `(subject, predicate, object)` LEXICAL SURFACES — the exact
    /// bytes a [`RelationStore`] interns, and the bytes a plan constant renders to.
    pub(crate) triples: Vec<(String, String, String)>,
    /// The analytically-known DERIVED facts (the least model minus the EDB).
    pub(crate) expected: BTreeSet<Fact>,
    /// The closed-form count of derived facts, cross-checking [`Self::expected`].
    pub(crate) expected_rows: u64,
}

impl SynthWorkload {
    /// A freshly seeded store holding this workload's EDB.
    ///
    /// Rebuilt per call because a [`RelationStore`] is consumed by an evaluation; the
    /// generators are pure, so two calls seed byte-identical stores.
    pub(crate) fn edb(&self) -> RelationStore {
        let mut store = RelationStore::new();
        for (subject, predicate, object) in &self.triples {
            store.insert(predicate, subject, object);
        }
        store
    }
}

/// The lexical surface an IRI is stored and compared under.
fn surface(iri: &str) -> String {
    format!("<{iri}>")
}

/// A node IRI `…/n{i}` — the synthetic graph vertices.
fn node(i: usize) -> String {
    format!("{BASE}/n{i}")
}

/// A variable term.
fn v(name: &str) -> ClauseTerm {
    ClauseTerm::var(name)
}

/// A rule `head :- body`, over binary atoms named by unbracketed predicate IRIs.
fn rule(head: (&str, &str, &str), body: &[(&str, &str, &str)]) -> DlClause {
    /// One atom, treating a `?`-prefixed argument as a variable and anything else as an
    /// IRI constant.
    fn atom(spec: (&str, &str, &str)) -> ClauseAtom {
        let term = |value: &str| {
            if value.starts_with('?') {
                v(value)
            } else {
                ClauseTerm::iri(value)
            }
        };
        ClauseAtom::positive(term(spec.0), spec.1, term(spec.2))
    }
    DlClause::datalog(atom(head), body.iter().copied().map(atom).collect())
}

/// A derived fact over IRI terms.
fn fact(subject: &str, predicate: &str, object: &str) -> Fact {
    Fact {
        subject: surface(subject),
        predicate: predicate.to_owned(),
        object: surface(object),
    }
}

/// The `edge` EDB predicate IRI.
fn edge_p() -> String {
    format!("{BASE}/edge")
}

/// The `path` IDB predicate IRI (the transitive closure of `edge`).
fn path_p() -> String {
    format!("{BASE}/path")
}

/// **Transitive closure** over a length-`n` linear chain `v0 → v1 → … → vn`.
///
/// A chain of `n` edges over `n + 1` nodes has closure `{ (vi, vj) : i < j }`, of size
/// `C(n+1, 2) = n(n+1)/2` — that is the only IDB predicate, so it is the whole golden.
///
/// # Panics
///
/// Panics if `n == 0`: an empty chain derives nothing, so it is not an oracle.
pub(crate) fn transitive_closure(n: usize) -> SynthWorkload {
    assert!(n >= 1, "transitive_closure needs n >= 1");
    let edge = edge_p();
    let path = path_p();

    let triples: Vec<(String, String, String)> = (0..n)
        .map(|i| (surface(&node(i)), edge.clone(), surface(&node(i + 1))))
        .collect();

    let rules = vec![
        rule(("?s", &path, "?o"), &[("?s", &edge, "?o")]),
        rule(
            ("?s", &path, "?o"),
            &[("?s", &edge, "?m"), ("?m", &path, "?o")],
        ),
    ];

    let expected: BTreeSet<Fact> = (0..=n)
        .flat_map(|i| ((i + 1)..=n).map(move |j| (i, j)))
        .map(|(i, j)| fact(&node(i), &path, &node(j)))
        .collect();
    let expected_rows = (n as u64) * (n as u64 + 1) / 2;
    assert_eq!(expected.len() as u64, expected_rows);

    SynthWorkload {
        name: "transitive_closure",
        rules,
        triples,
        expected,
        expected_rows,
    }
}

/// **Strongly connected** — one directed `n`-cycle `v0 → v1 → … → v(n-1) → v0`.
///
/// In an `n`-cycle every node reaches every node (itself included, around the loop), so
/// `path` is the complete `n²` relation and mutual reachability (`same_component`) is
/// complete too. Both are IDB, so the golden has `2n²` facts.
///
/// # Panics
///
/// Panics if `n == 0`.
pub(crate) fn strongly_connected(n: usize) -> SynthWorkload {
    assert!(n >= 1, "strongly_connected needs n >= 1");
    let edge = edge_p();
    let path = path_p();
    let same = format!("{BASE}/same_component");

    let triples: Vec<(String, String, String)> = (0..n)
        .map(|i| (surface(&node(i)), edge.clone(), surface(&node((i + 1) % n))))
        .collect();

    let rules = vec![
        rule(("?s", &path, "?o"), &[("?s", &edge, "?o")]),
        rule(
            ("?s", &path, "?o"),
            &[("?s", &edge, "?m"), ("?m", &path, "?o")],
        ),
        rule(
            ("?s", &same, "?o"),
            &[("?s", &path, "?o"), ("?o", &path, "?s")],
        ),
    ];

    let expected: BTreeSet<Fact> = (0..n)
        .flat_map(|i| (0..n).map(move |j| (i, j)))
        .flat_map(|(i, j)| {
            [
                fact(&node(i), &path, &node(j)),
                fact(&node(i), &same, &node(j)),
            ]
        })
        .collect();
    let expected_rows = 2 * (n as u64) * (n as u64);
    assert_eq!(expected.len() as u64, expected_rows);

    SynthWorkload {
        name: "strongly_connected",
        rules,
        triples,
        expected,
        expected_rows,
    }
}

/// **Same generation** over a two-level tree: a root with `n` children, each of which has
/// `n` children of its own.
///
/// The `n` parents all share the root, so every ordered parent pair is same-generation:
/// `n²` facts. Every grandchild pair is same-generation too, because their parents are:
/// `(n²)² = n⁴` facts. The levels are disjoint, so the golden has `n² + n⁴` facts.
///
/// # Panics
///
/// Panics if `n == 0`.
pub(crate) fn same_generation(n: usize) -> SynthWorkload {
    assert!(n >= 1, "same_generation needs n >= 1");
    let parent = format!("{BASE}/parent");
    let sg = format!("{BASE}/same_gen");
    let root = format!("{BASE}/root");
    let level_1 = |i: usize| format!("{BASE}/p{i}");
    let level_2 = |i: usize, j: usize| format!("{BASE}/c{i}_{j}");

    let mut triples: Vec<(String, String, String)> = Vec::new();
    for i in 0..n {
        triples.push((surface(&level_1(i)), parent.clone(), surface(&root)));
        for j in 0..n {
            triples.push((
                surface(&level_2(i, j)),
                parent.clone(),
                surface(&level_1(i)),
            ));
        }
    }

    let rules = vec![
        rule(
            ("?x", &sg, "?y"),
            &[("?x", &parent, "?p"), ("?y", &parent, "?p")],
        ),
        rule(
            ("?x", &sg, "?y"),
            &[
                ("?x", &parent, "?a"),
                ("?a", &sg, "?b"),
                ("?y", &parent, "?b"),
            ],
        ),
    ];

    let mut expected: BTreeSet<Fact> = BTreeSet::new();
    for i in 0..n {
        for j in 0..n {
            expected.insert(fact(&level_1(i), &sg, &level_1(j)));
        }
    }
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                for l in 0..n {
                    expected.insert(fact(&level_2(i, j), &sg, &level_2(k, l)));
                }
            }
        }
    }
    let square = (n as u64) * (n as u64);
    let expected_rows = square + square * square;
    assert_eq!(expected.len() as u64, expected_rows);

    SynthWorkload {
        name: "same_generation",
        rules,
        triples,
        expected,
        expected_rows,
    }
}

/// **Reachability** — single-source reachability from `v0` along a length-`n` chain.
///
/// The source is a CONSTANT in both rules, so the search is seeded and extended from that
/// one node: `reach(v0, vi)` holds for exactly `i = 1 … n`, giving `n` facts.
///
/// # Panics
///
/// Panics if `n == 0`.
pub(crate) fn reachability(n: usize) -> SynthWorkload {
    assert!(n >= 1, "reachability needs n >= 1");
    let edge = edge_p();
    let reach = format!("{BASE}/reach");
    let source = node(0);

    let triples: Vec<(String, String, String)> = (0..n)
        .map(|i| (surface(&node(i)), edge.clone(), surface(&node(i + 1))))
        .collect();

    let rules = vec![
        rule((&source, &reach, "?o"), &[(&source, &edge, "?o")]),
        rule(
            (&source, &reach, "?o"),
            &[(&source, &reach, "?m"), ("?m", &edge, "?o")],
        ),
    ];

    let expected: BTreeSet<Fact> = (1..=n).map(|i| fact(&source, &reach, &node(i))).collect();
    let expected_rows = n as u64;
    assert_eq!(expected.len() as u64, expected_rows);

    SynthWorkload {
        name: "reachability",
        rules,
        triples,
        expected,
        expected_rows,
    }
}

/// Every workload of the corpus, at several scales.
///
/// Small scales are deliberate: an analytic oracle proves the SHAPE of the answer, and the
/// shapes here (a chain, a cycle, a two-level tree) are exercised completely at `n` in the
/// single digits. Growing `n` would grow the runtime quartically without testing anything
/// the small cases leave untested.
pub(crate) fn all() -> Vec<SynthWorkload> {
    let mut workloads = Vec::new();
    for n in [1usize, 2, 5, 8] {
        workloads.push(transitive_closure(n));
    }
    for n in [1usize, 2, 3, 5] {
        workloads.push(strongly_connected(n));
    }
    for n in [1usize, 2, 3] {
        workloads.push(same_generation(n));
    }
    for n in [1usize, 4, 7] {
        workloads.push(reachability(n));
    }
    workloads
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every generator's golden set and closed-form count agree, and the EDB it emits is
    /// the size the construction implies. This checks the ORACLE, so a bug in the oracle
    /// cannot silently excuse a bug in the evaluator.
    #[test]
    fn every_generator_is_self_consistent() {
        for workload in all() {
            assert_eq!(
                workload.expected.len() as u64,
                workload.expected_rows,
                "{}: golden set size vs closed form",
                workload.name
            );
            assert!(
                !workload.triples.is_empty(),
                "{}: an empty EDB is not an oracle",
                workload.name
            );
            let edb = workload.edb();
            assert_eq!(
                edb.row_count(),
                workload.triples.len(),
                "{}: the triple list has no duplicates",
                workload.name
            );
            // The golden names only DERIVED facts, so no golden fact is already seeded.
            let seeded: BTreeSet<Fact> = edb.facts_sorted().into_iter().collect();
            assert!(
                workload.expected.is_disjoint(&seeded),
                "{}: the golden must be the derived facts alone",
                workload.name
            );
        }
    }

    /// The generators are pure: the same scale yields the same program, EDB and golden.
    #[test]
    fn generators_are_pure() {
        for (left, right) in all().into_iter().zip(all()) {
            assert_eq!(left.name, right.name);
            assert_eq!(left.rules, right.rules);
            assert_eq!(left.triples, right.triples);
            assert_eq!(left.expected, right.expected);
            assert_eq!(left.expected_rows, right.expected_rows);
        }
    }

    /// The closed forms are the ones the module docs state.
    #[test]
    fn closed_forms_match_the_documented_formulas() {
        assert_eq!(transitive_closure(8).expected_rows, 8 * 9 / 2);
        assert_eq!(strongly_connected(5).expected_rows, 2 * 25);
        assert_eq!(same_generation(3).expected_rows, 9 + 81);
        assert_eq!(reachability(7).expected_rows, 7);
    }
}
