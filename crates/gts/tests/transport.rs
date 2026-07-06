// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Writer → reader transport round-trip: terms and quads survive a fold intact.

use purrdf_gts::model::{Term, TermKind};
use purrdf_gts::reader::read;
use purrdf_gts::writer::Writer;

#[test]
fn writer_reader_round_trips_terms_and_quads() {
    let terms = vec![
        Term {
            kind: TermKind::Iri,
            value: Some("https://example.test/s".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Iri,
            value: Some("https://example.test/p".to_string()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
        },
        Term {
            kind: TermKind::Literal,
            value: Some("Cat".to_string()),
            datatype: None,
            lang: Some("en".to_string()),
            direction: None,
            reifier: None,
        },
    ];
    let quads = vec![(0, 1, 2, None)];

    let mut writer = Writer::new("purrdf.gts");
    writer.add_terms(&terms);
    writer.add_quads(&quads);

    let graph = read(&writer.to_bytes(), true, None);
    assert_eq!(graph.terms, terms);
    assert_eq!(graph.quads, quads);
}
