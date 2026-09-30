// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! A shape-map selector expands to nodes in `TermValue`'s total order, in which
//! base direction and blank-node scope distinguish terms.
//!
//! The order is read through the public entry points a caller uses,
//! `resolve_shape_map` and `validate_shape_map`, over a graph whose selected
//! objects span every term kind: IRIs, blank nodes of several scopes, plain,
//! language-tagged and directional literals, and a triple term. The expectation is
//! computed with `TermValue::cmp` and also pinned as a literal sequence, so a change
//! of the order is a visible diff. The retired string sort key tied terms differing
//! only in direction or scope, so the distinctions are asserted, not just the order.
//!
//! Fixtures use `example.org` throughout; every IRI is fixture configuration.

use std::sync::Arc;

use purrdf_core::{
    BlankScope, RdfDataset, RdfDatasetBuilder, RdfLiteral, RdfTextDirection, TermBox, TermValue,
};
use purrdf_shex::{
    ShapeSelector, ValidationOptions, parse_shape_map, parse_shexc, resolve_shape_map,
    validate_shape_map,
};

const SUBJECT: &str = "http://example.org/s";
const PREDICATE: &str = "http://example.org/p";
const SHAPE: &str = "http://example.org/S";

fn directional(lexical: &str, tag: &str, direction: RdfTextDirection) -> RdfLiteral {
    let mut literal = RdfLiteral::language_tagged(lexical, tag);
    literal.direction = Some(direction);
    literal
}

/// `<s> <p>` each of a mixed-kind object set, plus `<s2> <p>` a repeat of one of them.
fn data() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let s = b.intern_iri(SUBJECT);
    let p = b.intern_iri(PREDICATE);
    let mut objects = vec![
        b.intern_iri("http://example.org/o2"),
        b.intern_iri("http://example.org/o1"),
        b.intern_blank("b", BlankScope(2)),
        b.intern_blank("b", BlankScope(1)),
        b.intern_blank("b", BlankScope::DEFAULT),
        b.intern_blank("a", BlankScope(1)),
        b.intern_literal(RdfLiteral::simple("x")),
        b.intern_literal(RdfLiteral::language_tagged("x", "en")),
        b.intern_literal(directional("x", "en", RdfTextDirection::Rtl)),
        b.intern_literal(directional("x", "en", RdfTextDirection::Ltr)),
        b.intern_literal(RdfLiteral::typed(
            "7",
            "http://www.w3.org/2001/XMLSchema#integer",
        )),
    ];
    let (ts, tp, to) = (
        b.intern_iri("http://example.org/ts"),
        b.intern_iri("http://example.org/tp"),
        b.intern_iri("http://example.org/to"),
    );
    objects.push(b.intern_triple(ts, tp, to));
    // Inserted in a scrambled order, twice for one of them: dedup must hold.
    for &o in objects.iter().rev().chain(objects.iter().take(1)) {
        b.push_quad(s, p, o, None);
    }
    b.freeze().expect("freeze")
}

/// The terms the fixture selects, spelled as values.
fn expected_terms() -> Vec<TermValue> {
    let lit = |lexical: &str, tag: &str, direction: RdfTextDirection| {
        TermValue::from_rdf_term(&purrdf_core::RdfTerm::literal(directional(
            lexical, tag, direction,
        )))
    };
    vec![
        TermValue::iri("http://example.org/o2"),
        TermValue::iri("http://example.org/o1"),
        TermValue::Blank {
            label: "b".into(),
            scope: BlankScope(2),
        },
        TermValue::Blank {
            label: "b".into(),
            scope: BlankScope(1),
        },
        TermValue::Blank {
            label: "b".into(),
            scope: BlankScope::DEFAULT,
        },
        TermValue::Blank {
            label: "a".into(),
            scope: BlankScope(1),
        },
        TermValue::simple_literal("x"),
        TermValue::lang_literal("x", "en"),
        lit("x", "en", RdfTextDirection::Rtl),
        lit("x", "en", RdfTextDirection::Ltr),
        TermValue::typed_literal("7", "http://www.w3.org/2001/XMLSchema#integer"),
        TermValue::Triple {
            s: TermBox::new(TermValue::iri("http://example.org/ts")),
            p: TermBox::new(TermValue::iri("http://example.org/tp")),
            o: TermBox::new(TermValue::iri("http://example.org/to")),
        },
    ]
}

#[test]
fn a_selector_expands_in_term_value_order() {
    let data = data();
    let map = parse_shape_map(
        &format!("{{<{SUBJECT}> <{PREDICATE}> FOCUS}}@<{SHAPE}>"),
        None,
    )
    .expect("shape map parses");
    let selected: Vec<TermValue> = resolve_shape_map(&map, &data)
        .into_iter()
        .map(|(node, _)| node)
        .collect();

    let mut by_cmp = expected_terms();
    by_cmp.sort();
    assert_eq!(selected, by_cmp, "the expansion is TermValue's total order");
    assert_eq!(selected.len(), 12, "every distinct term once, none tied");

    // The pinned sequence: a change of the order shows here.
    assert_eq!(PINNED_ORDER.to_vec(), describe(&selected));
}

#[test]
fn a_validation_report_lists_nodes_in_term_value_order() {
    let data = data();
    let schema = parse_shexc(&format!("<{SHAPE}> {{}}"), None).expect("schema parses");
    let report = validate_shape_map(
        &schema,
        &data,
        &format!("{{<{SUBJECT}> <{PREDICATE}> FOCUS}}@<{SHAPE}>"),
        None,
        &ValidationOptions::default(),
    )
    .expect("the map validates");
    let nodes: Vec<TermValue> = report.entries.iter().map(|e| e.node.clone()).collect();
    let mut by_cmp = expected_terms();
    by_cmp.sort();
    assert_eq!(nodes, by_cmp, "the report follows the expansion order");
    assert!(
        report
            .entries
            .iter()
            .all(|entry| entry.shape == ShapeSelector::Label(SHAPE.to_owned())),
        "every association keeps its shape"
    );
}

/// A stable, human-readable spelling of a term, independent of `Ord`.
fn describe(terms: &[TermValue]) -> Vec<String> {
    terms
        .iter()
        .map(|term| match term {
            TermValue::Iri(iri) => format!("iri {iri}"),
            TermValue::Blank { label, scope } => format!("blank {label} scope {}", scope.ordinal()),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => format!("literal {lexical_form:?} {datatype} lang={language:?} dir={direction:?}"),
            TermValue::Triple { .. } => "triple".to_owned(),
        })
        .collect()
}

/// IRIs, then literals (by datatype, language, lexical form, direction), then blank
/// nodes (by label, then scope), then triple terms.
const PINNED_ORDER: [&str; 12] = [
    "iri http://example.org/o1",
    "iri http://example.org/o2",
    "literal \"x\" http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString lang=Some(\"en\") dir=Some(Ltr)",
    "literal \"x\" http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString lang=Some(\"en\") dir=Some(Rtl)",
    "literal \"x\" http://www.w3.org/1999/02/22-rdf-syntax-ns#langString lang=Some(\"en\") dir=None",
    "literal \"7\" http://www.w3.org/2001/XMLSchema#integer lang=None dir=None",
    "literal \"x\" http://www.w3.org/2001/XMLSchema#string lang=None dir=None",
    "blank a scope 1",
    "blank b scope 0",
    "blank b scope 1",
    "blank b scope 2",
    "triple",
];
