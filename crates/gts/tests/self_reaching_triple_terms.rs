// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A triple term that reaches itself states no triple (§7.3, "Resolution MUST
//! terminate").
//!
//! The fold records every `reifies` row, including one that lets a triple term
//! reach itself. The term, not the row, degrades: [`SelfReaching::of`] names
//! it, [`Graph::resolved_triple_of`] answers `None` for it, and every walk over
//! resolved components (the segment union, the canonical writer and every
//! projection) takes that one answer, so none can recurse forever. In Rust a
//! blown stack ABORTS: no catchable panic, no diagnostic, just a dead process.
//!
//! A triple term reaches itself through any of its spellings:
//!
//! 1. `tt`, `dt` and `rf` name strictly earlier terms (§7.2), so none of those
//!    closes a loop on its own. An `rf` naming the term itself is a forward
//!    reference: it is diagnosed and treated as absent.
//! 2. A `reifies` row may name ANY term in its triple, so it is the one field
//!    that can close a loop. A triple term with no `tt` reaches that row through
//!    the earlier reifier its `rf` names; a triple term with neither states no
//!    triple and reaches nothing.
//!
//! Every case below is built through the real [`Writer`] and read back, so
//! these are claims about the wire and not about hand-assembled structures.

use purrdf_gts::model::{DiagnosticCause, Graph, ReifierRow, Term, TermKind};
use purrdf_gts::reader::{ReadOptions, StreamingSink, read, read_to_sink_with_options};
use purrdf_gts::resolution::SelfReaching;
use purrdf_gts::writer::Writer;
use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_testkit::error_capture::Capture;

#[expect(
    dead_code,
    reason = "this test uses only one of the two shared triple-term forms"
)]
#[path = "support/reifier_terms.rs"]
mod reifier_terms;
use reifier_terms::indirect_triple_term;

/// A plain second segment, so the reader takes the multi-segment union path.
fn plain_segment() -> Vec<u8> {
    let terms = vec![
        Term::iri("http://example.org/b"),
        Term::iri("http://example.org/q"),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&[(0, 1, 0, None)]);
    writer.into_bytes()
}

fn diagnostic_codes(graph: &Graph) -> Vec<&str> {
    graph.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

/// No term may resolve, transitively, back to itself once self-reaching terms
/// state no triple, and every term the fold reports as self-reaching is one.
///
/// This walks [`Graph::resolved_triple_of`], the accessor every consumer uses,
/// which answers `None` for a self-reaching term where `triple_of` would follow
/// the loop.
fn assert_resolution_terminates(graph: &Graph, self_reaching: &[usize]) {
    let reaching = SelfReaching::of(graph);
    for start in 0..graph.terms.len() {
        let mut stack: Vec<usize> = graph
            .resolved_triple_of(start, &reaching)
            .map_or_default(|spo| <[usize; 3]>::from(spo).to_vec());
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
            if let Some(spo) = graph.resolved_triple_of(tid, &reaching) {
                stack.extend(<[usize; 3]>::from(spo));
            }
        }
    }
    for tid in 0..graph.terms.len() {
        assert_eq!(
            reaching.contains(tid),
            self_reaching.contains(&tid),
            "term {tid} self-reaching: terms={:?} reifiers={:?}",
            graph.terms,
            graph.reifiers,
        );
        if reaching.contains(tid) {
            assert_eq!(
                graph.resolved_triple_of(tid, &reaching),
                None,
                "a self-reaching term states no triple",
            );
        }
    }
}

// --------------------------------------------------------------------------- //
// An `rf` naming the term's own id, which the text does not allow.
// --------------------------------------------------------------------------- //

/// A `k:3` term with no `tt` whose `rf` names its own id, and a row keyed by that
/// id: what such a file yields. §7.2 requires `rf` to name a term-id strictly
/// less than the term's own id, so the `rf` is a `ForwardReference` and is
/// treated as absent; the term then states no triple (§7.3, third resolution
/// rule). The row's reifier is a quoted triple, which §7.4 does not allow, so it
/// is dropped with `PositionConstraint`. The quad using the term is kept, and
/// the fold terminates on one segment and through the union. The neighbouring
/// file spells the same binding with an earlier IRI reifier and folds with no
/// diagnostic.
#[test]
fn a_term_whose_rf_names_itself_is_a_forward_reference_and_its_row_drops() {
    // Terms a, p and r, then the triple term T at 3 and a quad using it. `rf` is
    // T's reifier and `reifier` keys the row binding <<( a p a )>>.
    let file = |rf: usize, reifier: usize| {
        let mut writer = Writer::new("purrdf-test");
        writer.add_terms(&[
            Term::iri("http://example.org/a"),
            Term::iri("http://example.org/p"),
            Term::iri("http://example.org/r"),
            indirect_triple_term(rf),
        ]);
        writer.add_quads(&[(0, 1, 3, None)]);
        writer.add_reifies(&[(reifier, (0, 1, 0), None)]);
        writer.into_bytes()
    };

    let self_named = file(3, 3);
    let mut two_segments = self_named.clone();
    two_segments.extend_from_slice(&plain_segment());
    for graph in [
        read(&self_named, false, None),
        read(&two_segments, true, None),
    ] {
        assert_eq!(
            diagnostic_codes(&graph),
            ["ForwardReference", "PositionConstraint"],
            "{:?}",
            graph.diagnostics,
        );
        assert!(graph.reifiers.is_empty(), "the row keyed by the term drops");
        let term = graph
            .terms
            .iter()
            .position(|t| t.kind == TermKind::Triple)
            .expect("the term is kept");
        assert_eq!(
            graph.terms[term].reifier, None,
            "its rf is treated as absent"
        );
        assert_eq!(graph.triple_of(term), None, "it states no triple");
        assert!(
            graph.quads.iter().any(|&(_, _, o, _)| o == term),
            "its quad is kept"
        );
        assert_resolution_terminates(&graph, &[]);
    }

    let earlier_reifier = read(&file(2, 2), false, None);
    assert!(
        earlier_reifier.diagnostics.is_empty(),
        "{:?}",
        earlier_reifier.diagnostics
    );
    assert_eq!(earlier_reifier.reifiers, vec![(2, (0, 1, 0), None)]);
    assert_eq!(earlier_reifier.triple_of(3), Some((0, 1, 0)));
}

/// A consumer can act on both diagnostics a self-naming `rf` produces: the
/// forward reference names the term and the `rf` it carried, and the dropped row
/// names the frame, the reifier and the row's triple.
#[test]
fn the_diagnostics_of_a_self_naming_rf_carry_typed_payloads() {
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&[
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
        Term::iri("http://example.org/r"),
        indirect_triple_term(3),
    ]);
    writer.add_quads(&[(0, 1, 3, None)]);
    writer.add_reifies(&[(3, (0, 1, 0), None)]);
    let graph = read(&writer.into_bytes(), false, None);
    let payload = |code: &str| {
        let diagnostic = graph
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap_or_else(|| panic!("{code} is reported: {:?}", graph.diagnostics));
        let Some(DiagnosticCause::Native(cause)) = diagnostic.cause.as_ref() else {
            panic!("{code} keeps its typed cause: {diagnostic:?}");
        };
        let error: &dyn StructuredError = &***cause;
        (diagnostic.frame_index, Capture::read(error))
    };

    let (frame, forward) = payload("ForwardReference");
    assert_eq!(forward.code, "gts-forward-reference");
    assert!(
        matches!(forward.field("term"), ErrorValue::Signed(3)),
        "term"
    );
    assert!(
        matches!(forward.field("reifier"), ErrorValue::Signed(3)),
        "reifier"
    );
    assert!(
        matches!(forward.field("datatype"), ErrorValue::Absent),
        "datatype"
    );
    assert_eq!(frame, Some(1), "the terms frame");

    let (frame, row) = payload("PositionConstraint");
    assert_eq!(row.code, "gts-reifier-kind");
    assert!(matches!(row.field("id"), ErrorValue::Unsigned(3)), "id");
    assert_eq!(frame, Some(3), "the reifies frame");
}

/// A `k:3` term with neither `tt` nor `rf` states no triple (§7.1, third
/// resolution rule) and is no reifier: the `reifies` row naming it as reifier is
/// dropped with `PositionConstraint`, while the term itself stays a distinct
/// term in every quad that uses it as subject or object.
#[test]
fn a_bare_triple_term_loses_its_reifier_row_but_keeps_its_quads() {
    let bare = Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: None,
    };
    let terms = vec![
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
        bare,
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&[(2, 1, 0, None), (0, 1, 2, None)]);
    writer.add_reifies(&[(2, (0, 1, 0), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    let codes: Vec<&str> = graph.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["PositionConstraint"], "{:?}", graph.diagnostics);
    assert!(
        graph.reifiers.is_empty(),
        "the row naming it as reifier is dropped"
    );
    assert_eq!(graph.quads, vec![(2, 1, 0, None), (0, 1, 2, None)]);
    assert_eq!(graph.terms[2].kind, TermKind::Triple);
    assert_eq!(graph.triple_of(2), None, "it states no triple");
}

// --------------------------------------------------------------------------- //
// The explicit `rf` spelling.
// --------------------------------------------------------------------------- //

/// Term 2 reads its components from reifier 0, and the row bound to reifier 0
/// has term 2 as its subject.
#[test]
fn a_binding_that_makes_an_rf_triple_term_contain_itself_degrades_the_term() {
    let terms = vec![
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
        indirect_triple_term(0),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(0, (2, 1, 1), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    assert_eq!(graph.reifiers, vec![(0, (2, 1, 1), None)]);
    assert_resolution_terminates(&graph, &[2]);
}

/// A two-hop loop: two triple terms whose rows name each other. The search
/// follows the chain, so both terms degrade — a check that only asked "does
/// this row name its own anchor" would catch neither.
#[test]
fn a_two_hop_binding_loop_degrades_both_terms() {
    let terms = vec![
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
        indirect_triple_term(0),
        indirect_triple_term(1),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    // Row one is innocent on its own: reifier 1 is unbound when it is read.
    // Row two closes the loop; both rows stay.
    writer.add_reifies(&[(0, (3, 1, 1), None), (1, (2, 1, 1), None)]);
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
    assert_eq!(
        graph.reifiers,
        vec![(0, (3, 1, 1), None), (1, (2, 1, 1), None)],
    );
    assert_resolution_terminates(&graph, &[2, 3]);
}

/// A term resolves through the FIRST row bound to its reifier, so a later row
/// that names a term reading that reifier closes no loop: the neighbouring
/// valid case of the loops above, introduced across frame boundaries.
#[test]
fn a_later_binding_of_a_shared_reifier_closes_no_loop() {
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&[
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
    ]);
    writer.add_terms(&[indirect_triple_term(0)]); // 2
    writer.add_reifies(&[(0, (1, 1, 1), None)]); // innocent: term 2 = <<( p p p )>>
    writer.add_terms(&[indirect_triple_term(0)]); // 3, sharing reifier 0
    writer.add_reifies(&[(0, (3, 1, 1), None)]); // a second row; the first still wins
    let bytes = writer.into_bytes();

    let graph = read(&bytes, false, None);
    assert_eq!(
        graph.reifiers,
        vec![(0, (1, 1, 1), None), (0, (3, 1, 1), None)],
        "both bindings are retained",
    );
    assert_eq!(graph.triple_of(3), Some((1, 1, 1)));
    assert_resolution_terminates(&graph, &[]);
}

/// The streaming reader folds each frame with a fresh fold state. It must
/// stream the same rows, and report the same diagnostics, as the materializing
/// reader, for rows that close a loop as for rows that do not.
#[test]
fn the_streaming_reader_streams_the_rows_the_materializing_reader_keeps() {
    #[derive(Default)]
    struct Rows(Vec<ReifierRow>);
    impl StreamingSink for Rows {
        fn reifier(&mut self, _segment_index: u64, reifier: ReifierRow) {
            self.0.push(reifier);
        }
    }

    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&[
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
    ]);
    writer.add_terms(&[indirect_triple_term(0)]); // 2
    writer.add_reifies(&[(0, (1, 1, 1), None)]); // innocent: term 2 = <<( p p p )>>
    writer.add_terms(&[indirect_triple_term(0)]); // 3, sharing reifier 0
    writer.add_reifies(&[(0, (3, 1, 1), None)]); // a second row for reifier 0
    writer.add_terms(&[indirect_triple_term(1)]); // 4, reading reifier 1
    writer.add_reifies(&[(1, (4, 1, 1), None)]); // 4 contains itself
    let bytes = writer.into_bytes();

    let mut rows = Rows::default();
    let streamed = read_to_sink_with_options(&bytes, ReadOptions::new(false, None), &mut rows);
    let streamed_codes: Vec<&str> = streamed
        .diagnostics
        .iter()
        .map(|d| d.code.as_str())
        .collect();
    assert_eq!(
        streamed_codes,
        diagnostic_codes(&read(&bytes, false, None)),
        "the streaming fold reports what the materializing fold reports",
    );
    assert_eq!(
        rows.0,
        vec![
            (0, (1, 1, 1), None),
            (0, (3, 1, 1), None),
            (1, (4, 1, 1), None)
        ],
        "every binding is streamed, the loop-closing one included",
    );
}

// --------------------------------------------------------------------------- //
// The id-ordering rules that make `tt`/`rf` acyclic by construction.
// --------------------------------------------------------------------------- //

/// A `tt` naming its OWN term id is a forward reference — the term is not
/// introduced until its row is read — so it is reported and dropped.
#[test]
fn a_tt_naming_its_own_term_is_dropped_as_a_forward_reference() {
    let terms = vec![
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
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
    assert_resolution_terminates(&graph, &[]);
}

/// An `rf` naming a term after itself is dropped the same way, so a reifier
/// chain can only ever run downwards.
#[test]
fn an_rf_naming_a_later_term_is_dropped_as_a_forward_reference() {
    let terms = vec![
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
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
    assert_resolution_terminates(&graph, &[]);
}

// --------------------------------------------------------------------------- //
// The same degradation on the other ways a fold is reached.
// --------------------------------------------------------------------------- //

/// A snapshot frame shifts its local ids into the outer space and
/// re-dispatches through the same handlers, so it degrades the same way.
#[test]
fn a_binding_loop_inside_a_snapshot_degrades_the_term() {
    let mut graph = Graph::default();
    graph.terms.push(Term::iri("http://example.org/a")); // 0
    graph.terms.push(Term::iri("http://example.org/p")); // 1
    graph.terms.push(indirect_triple_term(0)); // 2
    graph.reifiers.push((0, (2, 1, 1), None));

    let payload = graph.snapshot_payload();
    let mut writer = Writer::new("purrdf-test");
    writer.add_frame("snapshot", Some(payload), None, None, None);
    let bytes = writer.into_bytes();

    let folded = read(&bytes, false, None);
    assert!(folded.diagnostics.is_empty(), "{:?}", folded.diagnostics);
    assert_eq!(folded.reifiers, vec![(0, (2, 1, 1), None)]);
    assert_resolution_terminates(&folded, &[2]);
}

/// The multi-segment fold of a file holding a self-reaching term terminates,
/// keeps the row and the rest of the file, and gives the term an identity of its
/// own that states no triple. This is the path the segment union walks.
#[test]
fn the_multi_segment_fold_of_a_self_reaching_term_terminates() {
    let terms = vec![
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
        indirect_triple_term(0),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[(0, (2, 1, 1), None)]);
    writer.add_quads(&[(0, 1, 2, None)]);
    let mut data = writer.into_bytes();
    data.extend_from_slice(&plain_segment());

    let graph = read(&data, true, None);
    assert_eq!(graph.segment_heads.len(), 2, "both segments folded");
    assert_eq!(graph.reifiers.len(), 1, "the row survives the union");
    // The union maps a self-reaching term without walking its reifier, so in
    // the union it states no triple directly rather than through a loop.
    let term = graph
        .terms
        .iter()
        .position(|t| t.kind == TermKind::Triple)
        .expect("the term survives the union");
    assert_eq!(graph.terms[term].reifier, None);
    assert_resolution_terminates(&graph, &[]);
    assert_eq!(graph.quads.len(), 2, "both segments' quads survive");
    assert!(
        graph
            .terms
            .iter()
            .any(|t| t.value.as_deref() == Some("http://example.org/b")),
        "the second segment's terms survive the union",
    );
}

/// A triple term `R` bound through the IRI reifier `r`, and a term `T` whose `tt`
/// is `(a, p, o)` and whose `rf` names `R`, with `r`'s row binding
/// `<<( T p o )>>`, in a file of two segments (`r`'s row binds `<<( a p o )>>`
/// instead unless `rf_loops`).
fn reifier_of_a_term_bound_through_its_own_reifier(rf_loops: bool) -> Vec<u8> {
    let mut tagged = Term::triple_term((0, 1, 2));
    tagged.reifier = Some(4);
    let terms = vec![
        Term::iri("http://example.org/a"), // 0
        Term::iri("http://example.org/p"), // 1
        Term::iri("http://example.org/o"), // 2
        Term::iri("http://example.org/r"), // 3: r
        indirect_triple_term(3),           // 4: R, bound through r
        tagged,                            // 5: T, `tt` and `rf` naming R
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    let subject = if rf_loops { 5 } else { 0 };
    writer.add_reifies(&[(3, (subject, 1, 2), None)]);
    writer.add_quads(&[(5, 1, 2, None)]);
    let mut data = writer.into_bytes();
    data.extend_from_slice(&plain_segment());
    data
}

/// §7.3 makes termination bind every walk, the segment union included. A term's
/// `rf` is not part of its identity (§7.1: `tt` is authoritative and `rf` only
/// descriptive), yet the union maps it, and here it closes a loop the
/// components do not: `T`'s `rf` is `R`, and `R`'s binding has `T` as subject.
/// Neither term is reachable from its own components, so neither degrades: the
/// union terminates, `T` keeps its `tt` and its `rf`, and `r` keeps its row.
/// The neighbouring file, whose row does not name `T`, unions as before, with
/// `R` mapped ahead of the term that names it.
#[test]
fn the_multi_segment_union_terminates_on_a_loop_through_a_reifier() {
    for rf_loops in [true, false] {
        let graph = read(
            &reifier_of_a_term_bound_through_its_own_reifier(rf_loops),
            true,
            None,
        );
        assert!(graph.diagnostics.is_empty(), "{:?}", graph.diagnostics);
        assert_eq!(graph.segment_heads.len(), 2, "both segments folded");
        let id = |iri: &str| {
            graph
                .terms
                .iter()
                .position(|t| t.value.as_deref() == Some(iri))
                .unwrap_or_else(|| panic!("{iri} is in the union"))
        };
        let (a, p, o) = (
            id("http://example.org/a"),
            id("http://example.org/p"),
            id("http://example.org/o"),
        );
        let tagged = graph
            .terms
            .iter()
            .position(|t| t.triple == Some((a, p, o)))
            .expect("T keeps its tt");
        let reifier = graph.terms[tagged].reifier.expect("T keeps its rf");
        let r = id("http://example.org/r");
        assert_eq!(graph.terms[reifier].reifier, Some(r), "R reads r");
        let subject = if rf_loops { tagged } else { a };
        assert_eq!(graph.reifiers, vec![(r, (subject, p, o), None)]);
        assert_eq!(graph.triple_of(reifier), Some((subject, p, o)));
        assert!(graph.quads.contains(&(tagged, p, o, None)));
        assert_eq!(graph.quads.len(), 2, "both segments' quads survive");
        assert_resolution_terminates(&graph, &[]);
        if !rf_loops {
            assert!(reifier < tagged, "R is mapped ahead of the term naming it");
        }
    }
}

/// Re-authoring a file with a self-reaching term through the canonical writer
/// terminates, and reading the result back gives the same term and the same
/// row: the degradation is not something a round trip loses.
#[test]
fn a_self_reaching_term_survives_a_round_trip_through_the_canonical_writer() {
    let terms = vec![
        Term::iri("http://example.org/a"),
        Term::iri("http://example.org/p"),
        indirect_triple_term(0),
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&[(0, 1, 2, None)]);
    writer.add_reifies(&[(0, (2, 1, 1), None)]);
    let folded = read(&writer.into_bytes(), false, None);

    let rewritten = Writer::deterministic(&folded, "purrdf-test")
        .expect("re-author the folded graph")
        .into_bytes();
    let refolded = read(&rewritten, false, None);
    assert_eq!(
        refolded.reifiers.len(),
        1,
        "the row survives the round trip"
    );
    assert_resolution_terminates(&refolded, &[2]);
}

/// A self-reaching term that carries its own `tt`, which names a deeper triple
/// term that is not on the loop: the canonical writer must order the deeper
/// term before it, so the re-authored file reads back with no diagnostics and
/// the same `tt`.
#[test]
fn a_self_reaching_term_with_tt_survives_a_round_trip_through_the_canonical_writer() {
    let terms = vec![
        Term::iri("http://example.org/a"), // 0
        Term::iri("http://example.org/p"), // 1
        Term::iri("http://example.org/b"), // 2
        Term::iri("http://example.org/R"), // 3
        Term::triple_term((0, 1, 2)),      // 4: the deepest term
        Term::triple_term((0, 1, 4)),      // 5: off the loop
        indirect_triple_term(3),           // 6: bound by the row keyed on 3
        Term::triple_term((5, 1, 6)),      // 7: reaches 5 and 6
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_quads(&[(0, 1, 7, None)]);
    // 6 = <<( 7 p p )>>, so 6 -> 7 -> 6.
    writer.add_reifies(&[(3, (7, 1, 1), None)]);
    let folded = read(&writer.into_bytes(), false, None);
    assert!(folded.diagnostics.is_empty(), "{:?}", folded.diagnostics);
    assert_resolution_terminates(&folded, &[6, 7]);

    let rewritten = Writer::deterministic(&folded, "purrdf-test")
        .expect("re-author")
        .into_bytes();
    let refolded = read(&rewritten, false, None);
    assert!(
        refolded.diagnostics.is_empty(),
        "{:?}",
        refolded.diagnostics
    );
    assert_eq!(refolded.reifiers.len(), 1, "the row survives");
    assert_eq!(
        refolded
            .terms
            .iter()
            .filter(|term| term.kind == TermKind::Triple && term.triple.is_some())
            .count(),
        3,
        "every tt survives the re-authoring",
    );
    let mut degraded: Vec<usize> = (0..refolded.terms.len())
        .filter(|&tid| refolded.terms[tid].kind == TermKind::Triple)
        .filter(|&tid| {
            refolded
                .resolved_triple_of(tid, &SelfReaching::of(&refolded))
                .is_none()
        })
        .collect();
    degraded.sort_unstable();
    assert_eq!(degraded.len(), 2, "the two loop members state no triple");
}
