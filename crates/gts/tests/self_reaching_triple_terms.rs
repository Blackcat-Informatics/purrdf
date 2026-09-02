// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A folded quoted-triple term must never resolve back to itself (§7.3,
//! "Resolution MUST terminate").
//!
//! [`Graph::triple_of`] is the one place a folded quoted triple's components
//! are resolved, and every consumer of a quoted triple recurses through it —
//! the segment union, the canonical writer, and every projection to text
//! (N-Quads, Turtle, TriG, RDF/XML). None of those walks carries a bound of
//! its own. A term that resolved, transitively, back to itself would make each
//! of them recurse forever, and in Rust a blown stack ABORTS: no catchable
//! panic, no diagnostic, just a dead process. `lib.rs` promises the opposite —
//! the reader is total.
//!
//! §7.3 permits two conforming strategies, and this engine takes the first:
//! REFUSE the binding at fold time (the other is to accept it and give the
//! term a sentinel identity that states no triple). Because the refusal is the
//! whole defence, it has to cover every spelling `triple_of` reads:
//!
//! 1. `tt` — the term's own components — must name already-introduced terms.
//! 2. `dt` and `rf` must too (`rf` may also name the term itself).
//! 3. A `reifies` row may name ANY term, so it is the one field that can close
//!    a loop, and the fold refuses a row that would. A triple term reaches
//!    that row either through the `rf` it names or, when it names none,
//!    through its OWN id — and BOTH spellings must be refused. Missing the
//!    second admitted a file that folded with no diagnostic at all and then
//!    killed the first consumer that rendered it.
//!
//! Every case below is built through the real [`Writer`] and read back, so
//! these are claims about the wire and not about hand-assembled structures.

use purrdf_gts::model::{Graph, Term, TermKind};
use purrdf_gts::reader::read;
use purrdf_gts::writer::Writer;

fn iri(value: &str) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value.to_string()),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

/// The legacy indirect spelling: components come from the `reifies` row bound
/// to `reifier`.
fn indirect_triple_term(reifier: usize) -> Term {
    Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: Some(reifier),
        triple: None,
    }
}

/// The IMPLICIT self-bound spelling: a `k:3` term carrying neither `tt` nor
/// `rf`, whose binding is keyed by its own term id (§7.1).
fn implicit_self_bound_triple_term() -> Term {
    Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    }
}

/// A plain second segment, so the reader takes the multi-segment union path.
fn plain_segment() -> Vec<u8> {
    let terms = vec![iri("http://example.org/b"), iri("http://example.org/q")];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&[(0, 1, 0, None)]);
    writer.into_bytes()
}

fn diagnostic_codes(graph: &Graph) -> Vec<&str> {
    graph.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

/// No folded term may resolve, transitively, back to itself.
///
/// This walks [`Graph::triple_of`] — the accessor every consumer uses — rather
/// than `term_triple`, which does not read the implicit self-bound spelling
/// and so cannot see a loop written in it.
fn assert_no_term_reaches_itself(graph: &Graph) {
    for start in 0..graph.terms.len() {
        let mut stack: Vec<usize> = graph
            .triple_of(start)
            .map(|spo| <[usize; 3]>::from(spo).to_vec())
            .unwrap_or_default();
        let mut seen = vec![false; graph.terms.len()];
        while let Some(tid) = stack.pop() {
            assert_ne!(
                tid, start,
                "term {start} resolves through itself: terms={:?} reifiers={:?}",
                graph.terms, graph.reifiers,
            );
            if std::mem::replace(&mut seen[tid], true) {
                continue;
            }
            if let Some(spo) = graph.triple_of(tid) {
                stack.extend(<[usize; 3]>::from(spo));
            }
        }
    }
}

// --------------------------------------------------------------------------- //
// The implicit self-bound spelling — the one the refusal used to miss.
// --------------------------------------------------------------------------- //

/// The regression. A `k:3` term with no `tt` and no `rf` takes its components
/// from the row keyed by its OWN id, so a row naming that term as its own
/// subject makes the term contain itself.
///
/// This file folded with an EMPTY diagnostic list and left
/// `triple_of(2) == Some((2, 1, 1))`, which aborted the first consumer that
/// rendered term 2. The refusal anchored only on terms naming the reifier
/// through `rf`, and this term names nothing.
#[test]
fn a_row_keyed_by_an_implicit_self_bound_terms_own_id_is_refused() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        implicit_self_bound_triple_term(),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&[(0, 1, 2, None)]);
    writer.add_reifies(&[(2, (2, 1, 1), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert_eq!(
        diagnostic_codes(&graph),
        vec!["DamagedFrame"],
        "the loop must be REPORTED, not folded in silence: {:?}",
        graph.diagnostics,
    );
    assert!(
        graph.diagnostics[0].detail.contains("recursive"),
        "{:?}",
        graph.diagnostics,
    );
    assert!(
        graph.reifiers.is_empty(),
        "the self-reaching binding is not recorded: {:?}",
        graph.reifiers,
    );
    assert_eq!(
        graph.triple_of(2),
        None,
        "term 2 is left stating no triple rather than one containing itself",
    );
    assert_no_term_reaches_itself(&graph);
}

/// The implicit spelling is only refused when it LOOPS: a row keyed by an
/// implicit self-bound term's own id that names other terms is ordinary RDF
/// 1.2 and must still fold.
#[test]
fn an_implicit_self_bound_term_binding_other_terms_still_folds() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        implicit_self_bound_triple_term(),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(2, (0, 1, 0), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    assert_eq!(graph.reifiers, vec![(2, (0, 1, 0), None)]);
    assert_eq!(
        graph.triple_of(2),
        Some((0, 1, 0)),
        "the implicit self-bound spelling still resolves through its own id",
    );
    assert_no_term_reaches_itself(&graph);
}

/// A loop that runs THROUGH an implicit self-bound term rather than closing on
/// it directly. The traversal has to follow the implicit spelling, not just
/// anchor on it.
#[test]
fn a_loop_routed_through_an_implicit_self_bound_term_is_refused() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        implicit_self_bound_triple_term(), // 2, bound by the row keyed on 2
        indirect_triple_term(0),           // 3, bound by the row keyed on 0
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    // Innocent on its own: term 2 = <<( 3 p p )>>, and term 3 is unbound.
    writer.add_reifies(&[(2, (3, 1, 1), None)]);
    // Closes it: term 3 = <<( 2 p p )>>, so 3 -> 2 -> 3.
    writer.add_reifies(&[(0, (2, 1, 1), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert!(
        diagnostic_codes(&graph).contains(&"DamagedFrame"),
        "{:?}",
        graph.diagnostics,
    );
    assert_eq!(
        graph.reifiers,
        vec![(2, (3, 1, 1), None)],
        "only the loop-closing row is refused",
    );
    assert_no_term_reaches_itself(&graph);
}

// --------------------------------------------------------------------------- //
// The explicit `rf` spelling.
// --------------------------------------------------------------------------- //

/// Term 2 reads its components from reifier 0, and the row bound to reifier 0
/// has term 2 as its subject.
#[test]
fn a_binding_that_makes_an_rf_triple_term_contain_itself_is_refused() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        indirect_triple_term(0),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(0, (2, 1, 1), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert_eq!(
        diagnostic_codes(&graph),
        vec!["DamagedFrame"],
        "{:?}",
        graph.diagnostics,
    );
    assert_eq!(graph.reifiers, [] as [_; 0]);
    assert_eq!(graph.triple_of(2), None);
    assert_no_term_reaches_itself(&graph);
}

/// A two-hop loop: two triple terms whose rows name each other. The refusal
/// traverses, so it catches this too — a check that only asked "does this row
/// name its own anchor" would not.
#[test]
fn a_two_hop_binding_loop_is_refused() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        indirect_triple_term(0),
        indirect_triple_term(1),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    // Row one is innocent on its own: reifier 1 is unbound when it is read.
    // Row two closes the loop, and that is where the refusal must land.
    writer.add_reifies(&[(0, (3, 1, 1), None), (1, (2, 1, 1), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert_eq!(
        diagnostic_codes(&graph),
        vec!["DamagedFrame"],
        "{:?}",
        graph.diagnostics,
    );
    assert_eq!(
        graph.reifiers,
        vec![(0, (3, 1, 1), None)],
        "the innocent row is kept and only the loop-closing row refused",
    );
    assert_no_term_reaches_itself(&graph);
}

/// The loop is refused however late it is attempted: introducing the terms in
/// separate frames, and the rows after them, does not get one past the check.
#[test]
fn a_binding_loop_is_refused_across_frame_boundaries() {
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&[iri("http://example.org/a"), iri("http://example.org/p")]);
    writer.add_terms(&[indirect_triple_term(0)]); // 2
    writer.add_reifies(&[(0, (1, 1, 1), None)]); // innocent: term 2 = <<( p p p )>>
    writer.add_terms(&[indirect_triple_term(0)]); // 3, sharing reifier 0
    writer.add_reifies(&[(0, (3, 1, 1), None)]); // would make 3 contain itself
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert!(
        diagnostic_codes(&graph).contains(&"DamagedFrame"),
        "{:?}",
        graph.diagnostics,
    );
    assert_eq!(
        graph.reifiers,
        vec![(0, (1, 1, 1), None)],
        "only the innocent binding survives",
    );
    assert_no_term_reaches_itself(&graph);
}

/// An explicitly self-bound term (`rf == its own id`) is the same hazard
/// spelled a third way, and is refused too.
#[test]
fn an_explicitly_self_bound_term_naming_itself_is_refused() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        indirect_triple_term(2), // rf == this term's own id, which §7.1 allows
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(2, (2, 1, 1), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert!(
        diagnostic_codes(&graph).contains(&"DamagedFrame"),
        "{:?}",
        graph.diagnostics,
    );
    assert_no_term_reaches_itself(&graph);
}

// --------------------------------------------------------------------------- //
// The id-ordering rules that make `tt`/`rf` acyclic by construction.
// --------------------------------------------------------------------------- //

/// A `tt` naming its OWN term id is a forward reference — the term is not
/// introduced until its row is read — so it is reported and dropped.
#[test]
fn a_tt_naming_its_own_term_is_dropped_as_a_forward_reference() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        Term {
            kind: TermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: Some((2, 1, 1)), // 2 is this very term
        },
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert_eq!(diagnostic_codes(&graph), vec!["ForwardReference"]);
    assert_eq!(graph.terms[2].triple, None, "the self-naming tt is dropped");
    assert_no_term_reaches_itself(&graph);
}

/// An `rf` naming a term after itself is dropped the same way, so a reifier
/// chain can only ever run downwards (or self-bind, handled above).
#[test]
fn an_rf_naming_a_later_term_is_dropped_as_a_forward_reference() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        indirect_triple_term(3), // 3 does not exist yet
        indirect_triple_term(2),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert_eq!(diagnostic_codes(&graph), vec!["ForwardReference"]);
    assert_eq!(graph.terms[2].reifier, None, "the forward rf is dropped");
    assert_eq!(graph.terms[3].reifier, Some(2), "the backward rf survives");
    assert_no_term_reaches_itself(&graph);
}

// --------------------------------------------------------------------------- //
// The same refusals on the other ways a fold is reached.
// --------------------------------------------------------------------------- //

/// A snapshot frame shifts its local ids into the outer space and
/// re-dispatches through the same handlers, so it gets the same refusal.
#[test]
fn a_binding_loop_inside_a_snapshot_is_refused() {
    let mut graph = Graph::default();
    graph.terms.push(iri("http://example.org/a")); // 0
    graph.terms.push(iri("http://example.org/p")); // 1
    graph.terms.push(implicit_self_bound_triple_term()); // 2
    graph.reifiers.push((2, (2, 1, 1), None));

    let payload = graph.snapshot_payload();
    let mut writer = Writer::new("purrdf-test");
    writer.add_frame("snapshot", Some(payload), None, None, None);
    let bytes = writer.into_bytes();

    let folded = read(&bytes, false, None);
    assert!(
        !folded.diagnostics.is_empty(),
        "the loop is refused inside a snapshot, and reported",
    );
    assert_no_term_reaches_itself(&folded);
}

/// The multi-segment fold of a file that ATTEMPTED the loop terminates, keeps
/// the rest of the file, and leaves no self-reaching term. This is the path
/// the segment union walks, and it has no bound of its own.
#[test]
fn the_multi_segment_fold_of_an_attempted_loop_terminates() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        implicit_self_bound_triple_term(),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(2, (2, 1, 1), None)]);
    writer.add_quads(&[(0, 1, 2, None)]);
    let mut data = writer.into_bytes();
    data.extend_from_slice(&plain_segment());

    let graph = read(&data, true, None);
    assert_eq!(graph.segment_heads.len(), 2, "both segments folded");
    assert_no_term_reaches_itself(&graph);
    assert_eq!(graph.quads.len(), 2, "both segments' quads survive");
    assert!(
        graph
            .terms
            .iter()
            .any(|t| t.value.as_deref() == Some("http://example.org/b")),
        "the second segment's terms survive the union",
    );
}

/// Re-authoring a refused file through the canonical writer and reading it
/// back stays clean: the refusal is not something a round trip can launder.
#[test]
fn a_refused_loop_does_not_come_back_through_a_round_trip() {
    let terms = vec![
        iri("http://example.org/a"),
        iri("http://example.org/p"),
        implicit_self_bound_triple_term(),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&[(0, 1, 2, None)]);
    writer.add_reifies(&[(2, (2, 1, 1), None)]);
    let folded = read(&writer.into_bytes(), false, None);

    let rewritten = Writer::deterministic(&folded, "purrdf-test")
        .expect("re-author the folded graph")
        .into_bytes();
    let refolded = read(&rewritten, false, None);
    assert_no_term_reaches_itself(&refolded);
}
