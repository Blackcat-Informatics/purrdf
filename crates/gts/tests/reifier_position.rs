// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The reifier position constraint of a `reifies` row.
//!
//! The reifier of a `reifies` row is an IRI or a blank node. A row whose
//! reifier is a literal or a quoted triple is diagnosed `PositionConstraint`
//! and dropped; the reifier term itself is untouched, and a quoted triple stays
//! legal as the object of a `reifies` row.

use purrdf_gts::model::{ReifierRow, Term, TermKind};
use purrdf_gts::reader::{ReadOptions, StreamingSink, read, read_to_sink_with_options};
use purrdf_gts::writer::Writer;

#[path = "support/reifier_terms.rs"]
mod reifier_terms;
use reifier_terms::{indirect_triple_term, triple_term};

/// One file holding every kind of reifier: ids 3, 4 and 10 are kept IRI and
/// blank-node reifiers, 8 is the exempt spelling, and 5, 6, 7 and 9 are refused.
fn mixed_reifier_file() -> Vec<u8> {
    let terms = vec![
        Term::iri("http://example.org/a"),       // 0
        Term::iri("http://example.org/related"), // 1
        Term::iri("http://example.org/b"),       // 2
        Term::iri("http://example.org/r1"),      // 3
        Term::blank("b0"),                       // 4
        Term::literal("x", None),                // 5
        triple_term(0, 1, 2),                    // 6 carries `tt`
        indirect_triple_term(3),                 // 7 `rf` names another term
        indirect_triple_term(8),                 // 8 `rf` is its own id
        Term {
            kind: TermKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        }, // 9 neither `tt` nor `rf`
        Term::iri("http://example.org/r2"),      // 10
    ];
    let mut writer = Writer::new("purrdf-test");
    writer.add_terms(&terms);
    writer.add_reifies(&[
        (3, (0, 1, 2), None),
        (4, (0, 1, 2), None),
        (5, (0, 1, 2), None),
        (6, (0, 1, 2), None),
        (7, (0, 1, 2), None),
        (8, (0, 1, 2), None),
        (9, (0, 1, 2), None),
        (10, (0, 1, 6), None),
    ]);
    writer.into_bytes()
}

/// An IRI reifier and a blank-node reifier are retained. A literal reifier, a
/// quoted triple that carries `tt`, a quoted triple whose `rf` names another
/// term and a bare quoted triple are each dropped with `PositionConstraint`. A
/// quoted triple as the OBJECT of a `reifies` row is retained: the constraint
/// binds the reifier position only.
///
/// One quoted-triple reifier is exempt: the indirect spelling stored files hold,
/// a term with no `tt` whose `rf` is its own id. Its binding is kept with no
/// diagnostic.
#[test]
fn a_reifier_that_is_a_literal_or_quoted_triple_drops_its_row() {
    let bytes = mixed_reifier_file();

    let graph = read(&bytes, false, None);
    assert_eq!(
        graph.reifiers,
        vec![
            (3, (0, 1, 2), None),
            (4, (0, 1, 2), None),
            (8, (0, 1, 2), None),
            (10, (0, 1, 6), None),
        ],
        "the literal and non-exempt quoted-triple reifier rows are dropped"
    );
    let codes: Vec<&str> = graph.diagnostics.iter().map(|d| d.code.as_str()).collect();
    assert_eq!(codes, ["PositionConstraint"; 4]);
    assert_eq!(
        graph.triple_of(8),
        Some((0, 1, 2)),
        "the exempt term still states its own triple"
    );
    assert_eq!(
        graph.terms[6].kind,
        TermKind::Triple,
        "a dropped row leaves the reifier term itself in the fold"
    );
}

/// The streaming reader decides each `reifies` row against the same terms and
/// reports the same rows and diagnostics as the materializing reader, so an event
/// importer never sees a row the materializing fold refused.
#[test]
fn the_streaming_reader_applies_the_same_reifier_position_rule() {
    #[derive(Default)]
    struct Rows(Vec<ReifierRow>);
    impl StreamingSink for Rows {
        fn reifier(&mut self, _segment_index: u64, reifier: ReifierRow) {
            self.0.push(reifier);
        }
    }

    let bytes = mixed_reifier_file();
    let folded = read(&bytes, false, None);
    let mut rows = Rows::default();
    let streamed = read_to_sink_with_options(&bytes, ReadOptions::new(false, None), &mut rows);

    assert_eq!(rows.0, folded.reifiers);
    let codes = |diagnostics: &[purrdf_gts::model::Diagnostic]| -> Vec<String> {
        diagnostics.iter().map(|d| d.code.clone()).collect()
    };
    assert_eq!(codes(&streamed.diagnostics), codes(&folded.diagnostics));
    assert_eq!(codes(&streamed.diagnostics), ["PositionConstraint"; 4]);
}
