// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A triple term that reaches itself states no triple, and a caller-built GTS
//! term table must not be able to kill the process.
//!
//! GTS-SPEC §7.3 makes termination normative for EVERY walk of a triple term's
//! resolved components. This crate's walkers — the fold view's `render_term`, the
//! N-Triples/N-Quads/TriG writers, the RDF/XML and JSON-LD serializers, and the IR
//! fold's `SerInterner` — all recurse on those components with no depth bound and no
//! visited set, which is the right shape for a hot path and is only sound if the term
//! table they walk terminates. A stack overflow in Rust ABORTS: it is not a catchable
//! panic, so a self-reaching term is a process kill, not an error a caller can handle.
//!
//! A triple term that is reachable from its own components keeps an identity of its
//! own and projects as the fresh blank node an unbound triple term projects as, and
//! the `reifies` rows that name it still project. A graph a caller ASSEMBLED reaches
//! this crate through public doors: `purrdf_gts::model::Graph`'s fields are public,
//! `GtsFoldView::new` is public, and `dataset_from_gts_graph` /
//! `flattened_dataset_from_gts_graph` take a `&Graph` straight from the caller. Those
//! are the doors this suite stands in.
//!
//! A literal is not a triple term: a datatype chain that leads back to its own
//! literal is not degradable, and every door refuses it.

use purrdf_gts::model::{Graph, Term, TermKind};
use purrdf_rdf::gts::{dataset_from_gts_graph, flattened_dataset_from_gts_graph};
use purrdf_rdf::gts_view::GtsFoldView;
use purrdf_rdf::{SerializeGraph, serialize_dataset};

/// The diagnostic code the fold-time refusal reports.
const SELF_REACHING: &str = "gts-self-reaching-term";

/// A quoted triple in the ORIGINAL indirect spelling: it names a reifier id and the
/// statement layer supplies that id's components.
fn indirect_triple_term(reifier: usize) -> Term {
    Term {
        kind: TermKind::Triple,
        value: Some(String::new()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: Some(reifier),
        triple: None,
    }
}

/// A quoted triple with neither `tt` nor `rf`. It states no triple (§7.3, third
/// resolution rule): an absent `rf` names no reifier, not the term's own id.
fn bare_triple_term() -> Term {
    Term {
        kind: TermKind::Triple,
        value: Some(String::new()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

fn literal_with_datatype(lexical: &str, datatype: usize) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(lexical.to_owned()),
        datatype: Some(datatype),
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

/// Assert that every public door into this crate refuses `graph`, and says why.
///
/// The three doors are checked together on purpose. They are separate entry points
/// over three different downstream walkers — the fold view's renderer, the IR fold's
/// `SerInterner`, and (through the dataset the fold would have produced) the text,
/// RDF/XML and JSON-LD serializers — so a fix that closed only one of them would
/// leave the others live.
fn every_door_refuses(build: impl Fn() -> Graph, what: &str) {
    // `Graph` is not `Clone` and two of the three doors take it by value, so each
    // door gets its own build of the same table.
    let err = GtsFoldView::new(build())
        .err()
        .unwrap_or_else(|| panic!("GtsFoldView::new must refuse {what}"));
    assert_eq!(err.code, SELF_REACHING, "GtsFoldView::new refused {what}");

    let graph = build();
    let err = dataset_from_gts_graph(&graph)
        .err()
        .unwrap_or_else(|| panic!("dataset_from_gts_graph must refuse {what}"));
    assert_eq!(
        err.code, SELF_REACHING,
        "dataset_from_gts_graph refused {what}"
    );

    let err = flattened_dataset_from_gts_graph(&graph)
        .err()
        .unwrap_or_else(|| panic!("flattened_dataset_from_gts_graph must refuse {what}"));
    assert_eq!(
        err.code, SELF_REACHING,
        "flattened_dataset_from_gts_graph refused {what}"
    );
}

/// The N-Quads of `graph` through the dataset bridge, sorted.
fn nquads(graph: &Graph) -> Vec<String> {
    let dataset = dataset_from_gts_graph(graph).expect("a self-reaching term degrades");
    let bytes = serialize_dataset(&dataset, "application/n-quads", SerializeGraph::Dataset)
        .expect("the dataset serializes");
    let mut lines: Vec<String> = String::from_utf8(bytes)
        .expect("N-Quads are UTF-8")
        .lines()
        .map(str::to_owned)
        .collect();
    lines.sort();
    lines
}

/// Assert that every public door admits `build()` and that the fold view renders
/// each term of `degraded` as the blank node of an unbound triple term. The two
/// dataset bridges are only required to admit the graph here; the tests that
/// follow assert what the dataset bridge renders.
fn every_door_degrades(build: impl Fn() -> Graph, degraded: &[usize], what: &str) {
    let view = GtsFoldView::new(build())
        .unwrap_or_else(|error| panic!("GtsFoldView::new must admit {what}: {error:?}"));
    for &tid in degraded {
        assert_eq!(
            view.nq_token(tid),
            format!("_:unbound_triple_{tid}"),
            "term {tid} of {what} states no triple"
        );
    }
    let graph = build();
    dataset_from_gts_graph(&graph)
        .unwrap_or_else(|error| panic!("dataset_from_gts_graph must admit {what}: {error:?}"));
    flattened_dataset_from_gts_graph(&graph).unwrap_or_else(|error| {
        panic!("flattened_dataset_from_gts_graph must admit {what}: {error:?}")
    });
}

/// The minimal kill: one self-describing triple term whose own `(s, p, o)` names itself.
/// Term ids are all in range, so a range-only check waves it straight through. The term
/// is the blank node an unbound triple term is, and the quad that uses it stays.
#[test]
fn a_triple_term_that_names_itself_states_no_triple() {
    let build = || {
        let mut graph = Graph::default();
        graph.terms.push(Term::iri("https://example.org/s")); // 0
        graph.terms.push(Term::iri("https://example.org/p")); // 1
        graph.terms.push(Term::triple_term((2, 1, 0))); // 2 — names itself
        graph.quads.push((0, 1, 2, None));
        graph
    };
    every_door_degrades(build, &[2], "a triple term whose components name itself");
    assert_eq!(
        nquads(&build()),
        ["<https://example.org/s> <https://example.org/p> _:unbound_triple_2 ."]
    );
}

/// The loop does not have to be a self-edge. Two triple terms that name each other
/// reach themselves in two steps, which a check that only compared a term to its own
/// id would miss. Both degrade; a term that merely CONTAINS one does not, and renders
/// around it.
#[test]
fn two_triple_terms_that_name_each_other_both_state_no_triple() {
    let build = || {
        let mut graph = Graph::default();
        graph.terms.push(Term::iri("https://example.org/p")); // 0
        graph.terms.push(Term::triple_term((2, 0, 2))); // 1 — reaches 2
        graph.terms.push(Term::triple_term((1, 0, 1))); // 2 — reaches 1
        graph.terms.push(Term::triple_term((1, 0, 0))); // 3 — contains 1, is not on the loop
        graph
    };
    every_door_degrades(build, &[1, 2], "a two-step cycle between triple terms");
    let view = GtsFoldView::new(build()).expect("the loop degrades");
    assert_eq!(
        view.nq_token(3),
        "<<( _:unbound_triple_1 <https://example.org/p> <https://example.org/p> )>>",
        "a term that contains a self-reaching term keeps its own components"
    );
}

/// The loop can close through a literal's DATATYPE. A literal is not a triple term, so
/// it cannot degrade, and `render_literal` follows the datatype edge: that shape is
/// refused.
#[test]
fn a_literal_datatype_that_reaches_its_own_term_is_refused() {
    every_door_refuses(
        || {
            let mut graph = Graph::default();
            graph.terms.push(literal_with_datatype("x", 0));
            graph
        },
        "a literal whose datatype is itself",
    );
}

#[test]
fn a_datatype_cycle_through_a_triple_term_is_refused() {
    every_door_refuses(
        || {
            let mut graph = Graph::default();
            graph.terms.push(Term::iri("https://example.org/p")); // 0
            graph.terms.push(literal_with_datatype("x", 2)); // 1 — datatype is the triple
            graph.terms.push(Term::triple_term((1, 0, 1))); // 2 — reaches the literal
            graph
        },
        "a datatype edge closing a loop through a triple term",
    );
}

/// The ORIGINAL indirect spelling closes the same loop through the statement layer:
/// term 1 names reifier `<r>`, and `<r>`'s binding has term 1 as its own subject. The
/// term degrades and the row still projects as `<r> rdf:reifies <<( ... )>>`.
#[test]
fn a_reifier_binding_that_reaches_its_own_triple_term_keeps_its_row() {
    let build = || {
        let mut graph = Graph::default();
        graph.terms.push(Term::iri("https://example.org/p")); // 0
        graph.terms.push(indirect_triple_term(2)); // 1
        graph.terms.push(Term::iri("https://example.org/r")); // 2 — the reifier
        graph.reifiers.push((2, (1, 0, 0), None));
        graph
    };
    every_door_degrades(build, &[1], "a reifier binding naming its own triple term");
    assert_eq!(
        nquads(&build()),
        [
            "<https://example.org/r> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> \
          <<( _:unbound_triple_1 <https://example.org/p> <https://example.org/p> )>> ."
        ]
    );
}

/// A term with neither `tt` nor `rf` states no triple, so a row keyed by its own
/// id does not bind it. The fold view renders the term as the blank node of an
/// unbound triple term, and the dataset bridges refuse it at once as unbound,
/// whether it is the row's reifier alone or a quad also uses it. Reading that row
/// as the term's binding made the term contain itself, and the bridge then walked
/// it until storage ran out.
#[test]
fn a_bare_triple_term_is_not_bound_by_a_row_keyed_by_its_own_id() {
    for used_by_a_quad in [false, true] {
        let build = || {
            let mut graph = Graph::default();
            graph.terms.push(Term::iri("https://example.org/p")); // 0
            graph.terms.push(bare_triple_term()); // 1
            graph.reifiers.push((1, (1, 0, 0), None));
            if used_by_a_quad {
                graph.quads.push((1, 0, 0, None));
            }
            graph
        };
        let view = GtsFoldView::new(build()).expect("the fold view admits the graph");
        assert_eq!(view.nq_token(1), "_:unbound_triple_1");
        let graph = build();
        let err = dataset_from_gts_graph(&graph).expect_err("no row binds the bare term");
        assert_eq!(err.code, "native-codec-unbound-triple-term", "{err:?}");
        let err = flattened_dataset_from_gts_graph(&graph).expect_err("no row binds the bare term");
        assert_eq!(err.code, "native-codec-unbound-triple-term", "{err:?}");
    }
}

/// `tt` takes precedence over the reifier indirection, so a graph whose `tt` closes a
/// loop degrades even when the reifier row it also carries is perfectly sound.
#[test]
fn a_self_reaching_tt_states_no_triple_beside_a_sound_reifier_row() {
    every_door_degrades(
        || {
            let mut graph = Graph::default();
            graph.terms.push(Term::iri("https://example.org/s")); // 0
            graph.terms.push(Term::iri("https://example.org/p")); // 1
            graph.terms.push(Term::iri("https://example.org/o")); // 2
            graph.terms.push(Term::iri("https://example.org/r")); // 3
            let mut term = Term::triple_term((4, 1, 2));
            term.reifier = Some(3);
            graph.terms.push(term); // 4 — `tt` reaches itself; `rf` would not
            graph.reifiers.push((3, (0, 1, 2), None));
            graph
        },
        &[4],
        "a self-reaching `tt` beside a sound reifier row",
    );
}

/// `<<( s says <<( s p o )>> )>>` — nested, and perfectly acyclic. RDF 1.2 nests a
/// triple term only in the OBJECT slot, so that is where the inner one sits.
fn nested_triple_term_graph() -> Graph {
    let mut graph = Graph::default();
    graph.terms.push(Term::iri("https://example.org/s")); // 0
    graph.terms.push(Term::iri("https://example.org/p")); // 1
    graph.terms.push(Term::iri("https://example.org/o")); // 2
    graph.terms.push(Term::triple_term((0, 1, 2))); // 3 — <<( s p o )>>
    graph.terms.push(Term::iri("https://example.org/says")); // 4
    graph.terms.push(Term::triple_term((0, 4, 3))); // 5 — <<( s says <<( s p o )>> )>>
    graph.quads.push((0, 1, 5, None));
    graph
}

/// Degradation is not a ban on nesting. A legitimately nested quoted triple —
/// the shape the check exists to keep walkable — still constructs and still renders,
/// so the guard costs the honest graph nothing.
#[test]
fn legitimately_nested_triple_terms_still_construct_and_render() {
    let graph = nested_triple_term_graph();

    let view = GtsFoldView::new(nested_triple_term_graph())
        .expect("a nested but acyclic table terminates");
    assert_eq!(
        view.nq_token(5),
        "<<( <https://example.org/s> <https://example.org/says> \
         <<( <https://example.org/s> <https://example.org/p> <https://example.org/o> )>> )>>",
        "the nested quoted triple renders to its leaves"
    );
    assert_eq!(
        dataset_from_gts_graph(&graph)
            .expect("a nested but acyclic table folds")
            .quads()
            .count(),
        1,
        "the one base quad survives the fold"
    );
}

/// A quoted triple naming component id `9`, which no term occupies.
fn dangling_component_graph() -> Graph {
    let mut graph = Graph::default();
    graph.terms.push(Term::iri("https://example.org/p")); // 0
    graph.terms.push(Term::triple_term((9, 0, 9))); // 1 — 9 names no term
    graph.quads.push((0, 0, 1, None));
    graph
}

/// A component id that names no term is out of THIS check's scope: it closes no loop,
/// so the graph is admitted and the resolvers report the dangling id themselves. The
/// termination check must not start rejecting range errors under a misleading code.
#[test]
fn a_dangling_component_id_is_not_reported_as_a_loop() {
    let graph = dangling_component_graph();

    GtsFoldView::new(dangling_component_graph()).expect("a dangling id closes no loop");
    let err = dataset_from_gts_graph(&graph).expect_err("the fold reports the dangling id");
    assert_ne!(
        err.code, SELF_REACHING,
        "a range error must not be reported as a loop: {err:?}"
    );
}
