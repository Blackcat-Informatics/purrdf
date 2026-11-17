// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! An IRI holding DEL or a C1 control is refused at every ingress with the
//! shared diagnostic code, beside a neighbour holding U+00A0 that is accepted.
//! RFC 3987 `ucschar` starts at U+00A0, and the canonical document escapes
//! only what `IRIREF` forbids, so this refusal is what keeps a DEL or C1
//! scalar out of a canonical line.

use purrdf_core::ir::RdfDatasetBuilder;
use purrdf_rdf::{TermValue, parse_dataset};

/// The code `purrdf-iri` owns for a scalar an IRI may not hold.
const DISALLOWED: &str = "iri-disallowed-char";

/// DEL, the first and last C1 control, each raw and as a `UCHAR` escape the
/// grammar decodes to the same scalar; and the neighbour past the C1 block.
const REFUSED: [&str; 6] = [
    "\u{7f}", "\u{80}", "\u{9f}", "\\u007F", "\\u0080", "\\u009F",
];
const ACCEPTED: &str = "\u{a0}";

fn statement(inner: &str) -> String {
    format!("<http://example.org/a{inner}b> <http://example.org/p> <http://example.org/o> .")
}

#[test]
fn text_codecs_refuse_del_and_c1_and_accept_no_break_space() {
    for media_type in ["application/n-quads", "text/turtle"] {
        for inner in REFUSED {
            let err = parse_dataset(statement(inner).as_bytes(), media_type, None)
                .expect_err(&format!("{media_type} admitted {inner:?}"));
            assert_eq!(err.code, DISALLOWED, "{media_type} {inner:?}: {err}");
        }
        let dataset = parse_dataset(statement(ACCEPTED).as_bytes(), media_type, None)
            .unwrap_or_else(|e| panic!("{media_type} refused U+00A0: {e}"));
        let iri = format!("http://example.org/a{ACCEPTED}b");
        assert!(dataset.term_id_by_value(&TermValue::iri(&iri)).is_some());
    }
}

#[test]
fn the_builder_refuses_del_and_c1_and_accepts_no_break_space() {
    let freeze = |inner: &str| {
        let mut b = RdfDatasetBuilder::new();
        let s = b
            .intern_iri(&format!("http://example.org/a{inner}b"))
            .unwrap();
        let p = b.intern_iri("http://example.org/p").unwrap();
        let o = b.intern_iri("http://example.org/o").unwrap();
        b.push_quad(s, p, o, None).unwrap();
        b.freeze()
    };
    for inner in ["\u{7f}", "\u{80}", "\u{9f}"] {
        let err = freeze(inner).expect_err(&format!("builder admitted {inner:?}"));
        assert_eq!(err.code, DISALLOWED, "{inner:?}: {err}");
    }
    assert!(freeze(ACCEPTED).is_ok());
}
