// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! An IRI holding DEL or a C1 control is refused at every ingress, beside a
//! neighbour holding U+00A0 that is accepted.
//!
//! RFC 3987 `ucschar` starts at U+00A0, so DEL and U+0080 to U+009F are not IRI
//! characters. The canonical form writes them raw (it escapes exactly what the
//! `IRIREF` production excludes), so the canonical form can never be asked to
//! write one only because every ingress refuses it first. This pins that
//! instead of assuming it: if an entry point ever admits such an IRI, a canonical
//! document would carry a raw DEL or C1 control and this test names the entry point.
//!
//! Ingresses covered: N-Quads (raw and `UCHAR`-escaped), Turtle (raw and
//! `UCHAR`-escaped) and the programmatic builder, which records the refusal when
//! the IRI is interned and reports it when the dataset is frozen, so no frozen
//! dataset holds such an IRI. The GTS importer interns every IRI through the same
//! builder call (`RdfDatasetBuilder::intern_iri`).

use purrdf_rdf::{RdfDatasetBuilder, parse_dataset};

const REFUSED: [(&str, &str); 4] = [
    ("DEL", "\u{7f}"),
    ("the first C1 control", "\u{80}"),
    ("NEL", "\u{85}"),
    ("the last C1 control", "\u{9f}"),
];

fn nquads(iri_body: &str) -> String {
    format!("<urn:ex:s> <urn:ex:p> <urn:ex:{iri_body}> .\n")
}

fn turtle(iri_body: &str) -> String {
    format!("<urn:ex:s> <urn:ex:p> <urn:ex:{iri_body}> .\n")
}

fn escaped(c: &str) -> String {
    format!("\\u{:04X}", c.chars().next().expect("one scalar") as u32)
}

#[test]
fn n_quads_refuses_del_and_c1_in_an_iri_and_accepts_the_neighbour() {
    for (name, c) in REFUSED {
        assert!(
            parse_dataset(
                nquads(&format!("a{c}b")).as_bytes(),
                "application/n-quads",
                None
            )
            .is_err(),
            "N-Quads admitted {name} raw in an IRI"
        );
        assert!(
            parse_dataset(
                nquads(&format!("a{}b", escaped(c))).as_bytes(),
                "application/n-quads",
                None
            )
            .is_err(),
            "N-Quads admitted {name} as a UCHAR in an IRI"
        );
    }
    parse_dataset(nquads("a\u{a0}b").as_bytes(), "application/n-quads", None)
        .expect("U+00A0 is ucschar and an IRI may hold it");
}

#[test]
fn turtle_refuses_del_and_c1_in_an_iri_and_accepts_the_neighbour() {
    for (name, c) in REFUSED {
        assert!(
            parse_dataset(turtle(&format!("a{c}b")).as_bytes(), "text/turtle", None).is_err(),
            "Turtle admitted {name} raw in an IRI"
        );
        assert!(
            parse_dataset(
                turtle(&format!("a{}b", escaped(c))).as_bytes(),
                "text/turtle",
                None
            )
            .is_err(),
            "Turtle admitted {name} as a UCHAR in an IRI"
        );
    }
    parse_dataset(turtle("a\u{a0}b").as_bytes(), "text/turtle", None)
        .expect("U+00A0 is ucschar and an IRI may hold it");
}

#[test]
fn the_programmatic_builder_refuses_del_and_c1_in_an_iri_and_accepts_the_neighbour() {
    let frozen_with = |object: &str| {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri("urn:ex:s").expect("subject");
        let p = builder.intern_iri("urn:ex:p").expect("predicate");
        let o = builder
            .intern_iri(object)
            .expect("interning defers the refusal");
        builder.push_quad(s, p, o, None).expect("quad");
        builder.freeze()
    };
    for (name, c) in REFUSED {
        assert!(
            frozen_with(&format!("urn:ex:a{c}b")).is_err(),
            "the builder froze a dataset holding {name} in an IRI"
        );
    }
    frozen_with("urn:ex:a\u{a0}b").expect("U+00A0 is ucschar and an IRI may hold it");
}
