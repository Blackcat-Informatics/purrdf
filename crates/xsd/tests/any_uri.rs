// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The same native anyURI contract runs on native and WebAssembly targets.

use purrdf_xsd::range::{Cardinality, DataRange, Facet, Known, Satisfiability};
use purrdf_xsd::{XsdDatatype as D, parse, range, same_value, value_cmp, value_equal};

fn lexical_mapping_preserves_everything_except_xml_whitespace() {
    assert_eq!(D::from_iri(D::AnyUri.iri()), Some(D::AnyUri));
    for (lexical, expected) in [
        ("", ""),
        (" \t\n\r ", ""),
        (" \t../é/%2f#Fragment\r\n", "../é/%2f#Fragment"),
        ("HTTP://Example.org/%41", "HTTP://Example.org/%41"),
        (" a\t \r\nb ", "a b"),
        ("\u{a0}x\u{a0}", "\u{a0}x\u{a0}"),
        ("a<not-a-url>%zz", "a<not-a-url>%zz"),
    ] {
        let value = parse(lexical, D::AnyUri).unwrap();
        assert_eq!(value.datatype(), D::AnyUri);
        assert_eq!(value.canonical_lexical(), expected);
        assert_eq!(value.facet_length(), Some(expected.chars().count()));
        assert_eq!(
            purrdf_xsd::value::canonical_non_numeric(&value)
                .unwrap()
                .to_string(),
            expected
        );
    }
    for lexical in ["a\0b", "\u{b}", "\u{1f}", "\u{fffe}", "\u{ffff}"] {
        for datatype in [D::String, D::AnyUri] {
            assert!(
                parse(lexical, datatype).is_err(),
                "{datatype:?} {lexical:?}"
            );
        }
    }
    let text = " \té\r\n 文\u{a0} ";
    assert_eq!(parse(text, D::String).unwrap().canonical_lexical(), text);
}

fn primitive_identity_is_disjoint_and_unordered() {
    let uri = parse(" a\tb ", D::AnyUri).unwrap();
    let alias = parse("a b", D::AnyUri).unwrap();
    let other = parse("a%20b", D::AnyUri).unwrap();
    let text = parse("a b", D::String).unwrap();
    assert!(same_value(&uri, &alias));
    assert_eq!(value_equal(&uri, &alias), Some(true));
    assert_eq!(value_equal(&uri, &other), Some(false));
    assert_eq!(value_cmp(&uri, &alias), None);
    assert!(!same_value(&uri, &text));
    assert_eq!(value_equal(&uri, &text), None);
    assert_eq!(purrdf_xsd::effective_boolean_value(&uri), None);
    assert!(purrdf_xsd::value_unary_minus(&uri).is_err());
    assert!(purrdf_xsd::value_add(&uri, &alias).is_err());
}

fn ranges_use_collapsed_character_lengths_and_primitive_identity() {
    let uris = DataRange::Datatype(D::AnyUri);
    let strings = DataRange::Datatype(D::String);
    assert_eq!(
        range::satisfiability(&DataRange::And(vec![uris.clone(), strings.clone()])),
        Satisfiability::Empty
    );
    let empty = DataRange::Restriction {
        base: D::AnyUri,
        facets: vec![Facet::Length(0)],
    };
    assert_eq!(range::cardinality(&empty), Cardinality::Exactly(1));
    let value = parse(" \t ", D::AnyUri).unwrap();
    assert_eq!(range::contains(&empty, &value), Known::Yes);
    assert_eq!(
        range::cardinality(&DataRange::And(vec![
            empty,
            DataRange::Not(Box::new(DataRange::OneOf(vec![value])))
        ])),
        Cardinality::Exactly(0)
    );
    let three = DataRange::Restriction {
        base: D::AnyUri,
        facets: vec![Facet::Length(3)],
    };
    assert_eq!(
        range::contains(&three, &parse(" é\t文 ", D::AnyUri).unwrap()),
        Known::Yes
    );
    assert_eq!(
        range::contains(&three, &parse("é 文", D::String).unwrap()),
        Known::No
    );
    assert!(matches!(range::cardinality(&three), Cardinality::AtLeast(n) if n > 1_000_000));
    let values = DataRange::OneOf(vec![
        parse("\ta b ", D::AnyUri).unwrap(),
        parse("a b", D::AnyUri).unwrap(),
    ]);
    assert_eq!(range::cardinality(&values), Cardinality::Exactly(1));
    assert_eq!(range::containment(&values, &uris), Satisfiability::Empty);
    assert_eq!(
        range::containment(&values, &strings),
        Satisfiability::Inhabited
    );
}

purrdf_testkit::harness_main!(
    lexical_mapping_preserves_everything_except_xml_whitespace,
    primitive_identity_is_disjoint_and_unordered,
    ranges_use_collapsed_character_lengths_and_primitive_identity,
);
