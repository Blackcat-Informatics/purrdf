// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shallow owned-model fixtures shared by the native compatibility tests and bench.

use purrdf_core::{RdfLiteral, RdfLocation, RdfTerm, RdfTextDirection, RdfTriple};

/// An independent compiler-derived model for bounded compatibility and latency oracles.
pub(crate) mod derived {
    use purrdf_core::{RdfLiteral, RdfLocation};

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) enum RdfTerm {
        Iri(String),
        BlankNode(String),
        Literal(RdfLiteral),
        Triple(Box<RdfTriple>),
    }

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct RdfTriple {
        pub(crate) subject: RdfTerm,
        pub(crate) predicate: String,
        pub(crate) object: RdfTerm,
        pub(crate) location: Option<RdfLocation>,
    }
}

/// Project a bounded fixture to the independent model, outside timed operations.
pub(crate) fn oracle(term: &RdfTerm) -> derived::RdfTerm {
    match term {
        RdfTerm::Iri(value) => derived::RdfTerm::Iri(value.clone()),
        RdfTerm::BlankNode(value) => derived::RdfTerm::BlankNode(value.clone()),
        RdfTerm::Literal(value) => derived::RdfTerm::Literal(value.clone()),
        RdfTerm::Triple(triple) => derived::RdfTerm::Triple(Box::new(derived::RdfTriple {
            subject: oracle(&triple.subject),
            predicate: triple.predicate.clone(),
            object: oracle(&triple.object),
            location: triple.location.clone(),
        })),
    }
}

/// Every leaf kind and two shallow triple shapes, with authored metadata preserved.
pub(crate) fn fixtures() -> Vec<(&'static str, RdfTerm)> {
    let literal = RdfLiteral {
        lexical_form: "a \"quote\"\nλ".into(),
        datatype: Some("http://example.org/type".into()),
        language: Some("en-US".into()),
        direction: Some(RdfTextDirection::Rtl),
    };
    let location = RdfLocation {
        path: Some("a\nfile".into()),
        line: Some(u64::MAX),
        column: Some(u32::MAX),
        logical: Some("adapter".into()),
        subject: Some("subject".into()),
        gts_term_id: Some(1),
        gts_quad_index: Some(2),
        gts_reifier_id: Some(3),
        gts_frame_index: Some(4),
        gts_segment_index: Some(5),
    };
    let triple = RdfTerm::Triple(Box::new(RdfTriple {
        subject: RdfTerm::iri("http://example.org/s"),
        predicate: "http://example.org/p".into(),
        object: RdfTerm::literal(literal.clone()),
        location: Some(location),
    }));
    vec![
        ("iri", RdfTerm::iri("http://example.org/λ\n\"")),
        ("blank", RdfTerm::blank_node("same")),
        ("literal", RdfTerm::literal(literal)),
        ("simple", RdfTerm::literal(RdfLiteral::simple(""))),
        (
            "typed",
            RdfTerm::literal(RdfLiteral::typed("7", "http://example.org/t")),
        ),
        (
            "language",
            RdfTerm::literal(RdfLiteral::language_tagged("word", "en")),
        ),
        ("triple", triple.clone()),
        (
            "nested",
            RdfTerm::Triple(Box::new(RdfTriple {
                subject: triple,
                predicate: "http://example.org/q".into(),
                object: RdfTerm::triple(RdfTriple::new(
                    RdfTerm::blank_node("same"),
                    "http://example.org/r",
                    RdfTerm::literal(RdfLiteral {
                        lexical_form: String::new(),
                        datatype: Some(String::new()),
                        language: Some(String::new()),
                        direction: Some(RdfTextDirection::Ltr),
                    }),
                )),
                location: Some(RdfLocation::default()),
            })),
        ),
    ]
}
