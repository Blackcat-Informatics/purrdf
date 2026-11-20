// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The reifier position constraint of a `reifies` row.
//!
//! The reifier of a `reifies` row is an IRI or a blank node. A row whose
//! reifier is a literal or a quoted triple is diagnosed `PositionConstraint`
//! and dropped; the reifier term itself is untouched, and a quoted triple stays
//! legal as the object of a `reifies` row.

use purrdf_gts::model::{Term, TermKind};
use purrdf_gts::reader::read;
use purrdf_gts::writer::Writer;

fn triple_term(s: usize, p: usize, o: usize) -> Term {
    Term {
        kind: TermKind::Triple,
        value: None,
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
        triple: Some((s, p, o)),
    }
}

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

/// An IRI reifier and a blank-node reifier are retained. A literal reifier, a
/// quoted triple that carries `tt`, a quoted triple whose `rf` names another
/// term and a bare quoted triple are each dropped with `PositionConstraint`. A
/// quoted triple as the OBJECT of a `reifies` row is retained: the constraint
/// binds the reifier position only.
///
/// One quoted-triple reifier is exempt: the indirect spelling older writers
/// emitted, a term with no `tt` whose `rf` is its own id. Files written that way
/// keep their binding, with no diagnostic.
#[test]
fn a_reifier_that_is_a_literal_or_quoted_triple_drops_its_row() {
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
    let bytes = writer.into_bytes();

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
