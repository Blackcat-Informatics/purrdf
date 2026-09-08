// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native literal shape admission and compatible implied datatype expansion.

use purrdf_core::{RdfDatasetBuilder, RdfLiteral, RdfTextDirection};

#[test]
fn invalid_literal_shapes_cannot_cross_native_validation() {
    for literal in [
        RdfLiteral {
            direction: Some(RdfTextDirection::Ltr),
            ..RdfLiteral::simple("x")
        },
        RdfLiteral::typed("x", RdfLiteral::language_datatype_iri(None)),
        RdfLiteral::typed(
            "x",
            RdfLiteral::language_datatype_iri(Some(RdfTextDirection::Rtl)),
        ),
        RdfLiteral::language_tagged("x", ""),
    ] {
        let mut builder = RdfDatasetBuilder::new();
        builder.intern_literal(literal);
        assert_eq!(
            builder
                .freeze()
                .expect_err("invalid shape must not publish")
                .code,
            "rdf-ir-literal-shape"
        );
    }
}

#[test]
fn native_implied_datatype_policy_remains_compatible() {
    let mut builder = RdfDatasetBuilder::new();
    for direction in [
        None,
        Some(RdfTextDirection::Ltr),
        Some(RdfTextDirection::Rtl),
    ] {
        let id = builder.intern_literal(RdfLiteral {
            lexical_form: "x".to_owned(),
            datatype: Some("https://example.org/ignored-explicit-type".to_owned()),
            language: Some("EN".to_owned()),
            direction,
        });
        let purrdf_core::TermRef::Literal {
            datatype,
            language,
            direction: stored,
            ..
        } = builder.resolve(id)
        else {
            panic!("literal")
        };
        assert_eq!(language, Some("en"));
        assert_eq!(stored, direction);
        assert_eq!(
            builder.resolve(datatype),
            purrdf_core::TermRef::Iri(RdfLiteral::language_datatype_iri(direction))
        );
    }
    builder
        .freeze()
        .expect("implied datatype expansion is valid");
}
