// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! An IRI holding DEL or a C1 control is refused at every ingress, beside a
//! neighbour holding U+00A0 that is accepted. RFC 3987 `ucschar` starts at
//! U+00A0, and the canonical IRI writer escapes only what `IRIREF` forbids, so
//! this refusal is what keeps a DEL or C1 scalar out of a canonical line.

use purrdf_core::ir::RdfDatasetBuilder;
use purrdf_rdf::{TermValue, parse_dataset};

/// The refused spellings (DEL, the first and last C1 control) and the accepted
/// neighbour that follows the C1 block.
const REFUSED: [&str; 3] = ["\u{7f}", "\u{80}", "\u{9f}"];
const ACCEPTED: &str = "\u{a0}";

fn iri(inner: &str) -> String {
    format!("http://example.org/a{inner}b")
}

#[test]
fn text_codecs_refuse_del_and_c1_and_accept_no_break_space() {
    for media_type in ["application/n-quads", "text/turtle"] {
        let statement = |inner: &str| {
            format!(
                "<{}> <http://example.org/p> <http://example.org/o> .",
                iri(inner)
            )
        };
        for inner in REFUSED {
            assert!(
                parse_dataset(statement(inner).as_bytes(), media_type, None).is_err(),
                "{media_type} admitted {inner:?}"
            );
        }
        let dataset = parse_dataset(statement(ACCEPTED).as_bytes(), media_type, None)
            .unwrap_or_else(|e| panic!("{media_type} refused U+00A0: {e}"));
        assert!(
            dataset
                .term_id_by_value(&TermValue::iri(&iri(ACCEPTED)))
                .is_some()
        );
    }
}

#[test]
fn the_builder_refuses_del_and_c1_and_accepts_no_break_space() {
    let freeze = |inner: &str| {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&iri(inner)).unwrap();
        let p = b.intern_iri("http://example.org/p").unwrap();
        let o = b.intern_iri("http://example.org/o").unwrap();
        b.push_quad(s, p, o, None).unwrap();
        b.freeze()
    };
    for inner in REFUSED {
        assert!(freeze(inner).is_err(), "builder admitted {inner:?}");
    }
    assert!(freeze(ACCEPTED).is_ok());
}
