// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One test per SEP-0009 production, one per PurRDF superset production, one per
//! ill-formed shape, and the canonical round-trip for every language-direction case.

use pretty_assertions::assert_eq;
use purrdf_cdt::{
    CdtEntry, CdtError, CdtKey, CdtLiteral, CdtTerm, CdtValue, TextDirection, XSD_BOOLEAN,
    XSD_DECIMAL, XSD_DOUBLE, XSD_INTEGER, XSD_STRING, parse_list, parse_map,
};

/// The items of a value that must be a list.
fn items(value: &CdtValue) -> &[CdtTerm] {
    value.as_list().expect("expected a list, got a map")
}

/// The entries of a value that must be a map.
fn entries(value: &CdtValue) -> &[CdtEntry] {
    value.as_map().expect("expected a map, got a list")
}

fn list_items(lexical: &str) -> Vec<CdtTerm> {
    items(&parse_list(lexical).expect("the lexical form is well formed")).to_vec()
}

/// A composite element, refused by the constructor only when it would break one of
/// the crate's three bounds — which no fixture in this file does.
fn composite(value: CdtValue) -> CdtTerm {
    CdtTerm::composite(value).expect("the fixture is within every bound")
}

/// A triple-term element, under the same standing as [`composite`].
fn triple(subject: CdtTerm, predicate: CdtTerm, object: CdtTerm) -> CdtTerm {
    CdtTerm::triple(subject, predicate, object).expect("the fixture is within every bound")
}

// ── [1] List ::= '[' (NonEmptyListContent)? ']' ────────────────────────────────

#[test]
fn production_1_list_is_bracketed_and_may_be_empty() {
    assert_eq!(parse_list("[]").unwrap(), CdtValue::empty_list());
    assert_eq!(list_items("[null]"), vec![CdtTerm::Null]);
    // Whitespace between the terminals is permitted and carries no meaning.
    assert_eq!(parse_list("  [  ]  ").unwrap(), CdtValue::empty_list());
    // The brackets are mandatory.
    assert!(matches!(
        parse_list("null"),
        Err(CdtError::Unexpected { offset: 0, .. })
    ));
}

// ── [2] NonEmptyListContent ::= ListElement (',' ListElement)* ─────────────────

#[test]
fn production_2_elements_are_comma_separated_and_ordered() {
    let parsed = list_items("[1, 2, 3]");
    assert_eq!(
        parsed,
        vec![
            CdtTerm::Literal(CdtLiteral::typed("1", XSD_INTEGER)),
            CdtTerm::Literal(CdtLiteral::typed("2", XSD_INTEGER)),
            CdtTerm::Literal(CdtLiteral::typed("3", XSD_INTEGER)),
        ]
    );
    // Order is significant: a list is a sequence, not a set.
    assert_ne!(list_items("[1,2]"), list_items("[2,1]"));
    // A separator with nothing after it is not content.
    assert!(parse_list("[1 2]").is_err());
}

// ── [3] ListElement ::= IRIREF | BLANK_NODE_LABEL | RDFLiteral | NumericLiteral
//                      | BooleanLiteral | NULL | List | Map ───────────────────

#[test]
fn production_3_every_list_element_alternative_is_admitted() {
    let parsed = list_items(
        "[<http://example.org/i>, _:b0, \"lit\", 1, 1.5, 1e0, true, null, [2], {\"k\": 3}]",
    );
    assert_eq!(parsed.len(), 10);
    assert_eq!(parsed[0], CdtTerm::Iri("http://example.org/i".into()));
    assert_eq!(parsed[1], CdtTerm::Blank("b0".into()));
    assert_eq!(
        parsed[2],
        CdtTerm::Literal(CdtLiteral::typed("lit", XSD_STRING))
    );
    assert_eq!(
        parsed[3],
        CdtTerm::Literal(CdtLiteral::typed("1", XSD_INTEGER))
    );
    assert_eq!(
        parsed[4],
        CdtTerm::Literal(CdtLiteral::typed("1.5", XSD_DECIMAL))
    );
    assert_eq!(
        parsed[5],
        CdtTerm::Literal(CdtLiteral::typed("1e0", XSD_DOUBLE))
    );
    assert_eq!(
        parsed[6],
        CdtTerm::Literal(CdtLiteral::typed("true", XSD_BOOLEAN))
    );
    assert_eq!(parsed[7], CdtTerm::Null);
    assert_eq!(
        parsed[8],
        composite(
            CdtValue::list(vec![CdtTerm::Literal(CdtLiteral::typed("2", XSD_INTEGER))])
                .expect("a one-element list is within every bound")
        )
    );
    assert!(matches!(&parsed[9], CdtTerm::Composite(inner) if inner.as_map().is_some()));
}

// ── [4] Map ::= '{' (NonEmptyMapContent)? '}' ──────────────────────────────────

#[test]
fn production_4_map_is_braced_and_may_be_empty() {
    assert_eq!(parse_map("{}").unwrap(), CdtValue::empty_map());
    assert_eq!(entries(&parse_map("{ \"a\" : 1 }").unwrap()).len(), 1);
    // A map lexical form is not a list lexical form and vice versa.
    assert!(parse_map("[]").is_err());
    assert!(parse_list("{}").is_err());
}

// ── [5] NonEmptyMapContent ::= MapEntry (',' MapEntry)* ────────────────────────

#[test]
fn production_5_entries_are_comma_separated() {
    let value = parse_map("{\"a\": 1, \"b\": 2, \"c\": 3}").unwrap();
    assert_eq!(entries(&value).len(), 3);
    // A map is unordered: authoring order does not reach the value.
    assert_eq!(
        parse_map("{\"a\": 1, \"b\": 2}").unwrap(),
        parse_map("{\"b\": 2, \"a\": 1}").unwrap()
    );
}

// ── [6] MapEntry ::= MapKey ':' MapValue ───────────────────────────────────────

#[test]
fn production_6_entry_pairs_a_key_with_a_value_through_a_colon() {
    let value = parse_map("{\"a\": <http://example.org/v>}").unwrap();
    assert_eq!(
        entries(&value)[0],
        CdtEntry {
            key: CdtKey::Literal(CdtLiteral::typed("a", XSD_STRING)),
            value: CdtTerm::Iri("http://example.org/v".into()),
        }
    );
    // The colon is mandatory.
    assert!(matches!(
        parse_map("{\"a\" 1}"),
        Err(CdtError::Unexpected { .. })
    ));
}

// ── [7] MapKey ::= IRIREF | RDFLiteral | NumericLiteral | BooleanLiteral ───────

#[test]
fn production_7_map_keys_admit_exactly_four_alternatives() {
    let value =
        parse_map("{<http://example.org/k>: 1, \"s\": 2, 3: 4, true: 5, \"l\"@en: 6}").unwrap();
    let keys: Vec<&CdtKey> = value.keys().collect();
    assert_eq!(keys.len(), 5);
    assert!(keys.contains(&&CdtKey::Iri("http://example.org/k".into())));
    assert!(keys.contains(&&CdtKey::Literal(CdtLiteral::typed("3", XSD_INTEGER))));
    assert!(keys.contains(&&CdtKey::Literal(CdtLiteral::typed("true", XSD_BOOLEAN))));
    assert!(keys.contains(&&CdtKey::Literal(CdtLiteral::lang("l", "en"))));
    // The alternatives a key does NOT have.
    assert!(parse_map("{_:b: 1}").is_err());
    assert!(parse_map("{[1]: 2}").is_err());
    assert!(parse_map("{{\"a\":1}: 2}").is_err());
    assert!(parse_map("{<<(<http://example.org/s> <http://example.org/p> 1)>>: 2}").is_err());
}

// ── [8] MapValue ::= … | NULL | List | Map ─────────────────────────────────────

#[test]
fn production_8_every_map_value_alternative_is_admitted() {
    let value = parse_map(
        "{\"a\": <http://example.org/i>, \"b\": _:b0, \"c\": \"lit\", \"d\": 1, \"e\": true, \
         \"f\": null, \"g\": [1], \"h\": {\"i\": 2}}",
    )
    .unwrap();
    let by_key = |name: &str| {
        entries(&value)
            .iter()
            .find(|entry| entry.key == CdtKey::Literal(CdtLiteral::typed(name, XSD_STRING)))
            .map(|entry| entry.value.clone())
            .expect("the key is present")
    };
    assert_eq!(by_key("a"), CdtTerm::Iri("http://example.org/i".into()));
    assert_eq!(by_key("b"), CdtTerm::Blank("b0".into()));
    assert_eq!(
        by_key("c"),
        CdtTerm::Literal(CdtLiteral::typed("lit", XSD_STRING))
    );
    assert_eq!(
        by_key("d"),
        CdtTerm::Literal(CdtLiteral::typed("1", XSD_INTEGER))
    );
    assert_eq!(
        by_key("e"),
        CdtTerm::Literal(CdtLiteral::typed("true", XSD_BOOLEAN))
    );
    assert_eq!(by_key("f"), CdtTerm::Null);
    assert!(matches!(by_key("g"), CdtTerm::Composite(_)));
    assert!(matches!(by_key("h"), CdtTerm::Composite(_)));
}

// ── [9] NULL ::= 'null' ────────────────────────────────────────────────────────

#[test]
fn production_9_null_is_a_first_class_element() {
    assert_eq!(list_items("[null]"), vec![CdtTerm::Null]);
    // Nulls are mutually indistinguishable.
    assert_eq!(parse_list("[null]").unwrap(), parse_list("[null]").unwrap());
    // …and are not the string "null".
    assert_ne!(
        parse_list("[null]").unwrap(),
        parse_list("[\"null\"]").unwrap()
    );
    // The spelling is exact.
    assert!(parse_list("[NULL]").is_err());
    assert!(parse_list("[nil]").is_err());
}

// ── [128s] RDFLiteral ::= String (LANGTAG | '^^' IRIREF)? ─────────────────────

#[test]
fn production_128s_rdf_literal_covers_all_four_string_forms_and_both_suffixes() {
    // All four `String` forms denote the same lexical value.
    for form in ["[\"abc\"]", "['abc']", "[\"\"\"abc\"\"\"]", "['''abc''']"] {
        assert_eq!(
            list_items(form),
            vec![CdtTerm::Literal(CdtLiteral::typed("abc", XSD_STRING))],
            "form {form} should denote the plain string \"abc\""
        );
    }
    // No suffix means xsd:string.
    assert_eq!(
        list_items("[\"x\"]"),
        vec![CdtTerm::Literal(CdtLiteral::plain("x"))]
    );
    // LANGTAG.
    assert_eq!(
        list_items("[\"x\"@en-GB]"),
        vec![CdtTerm::Literal(CdtLiteral::lang("x", "en-GB"))]
    );
    // '^^' IRIREF.
    assert_eq!(
        list_items("[\"7\"^^<http://www.w3.org/2001/XMLSchema#integer>]"),
        vec![CdtTerm::Literal(CdtLiteral::typed("7", XSD_INTEGER))]
    );
    // A long string may carry a raw newline; a short one may not.
    assert_eq!(
        list_items("[\"\"\"a\nb\"\"\"]"),
        vec![CdtTerm::Literal(CdtLiteral::plain("a\nb"))]
    );
    assert!(parse_list("[\"a\nb\"]").is_err());
}

// ── Superset 1: TripleTerm ::= '<<(' Element Element Element ')>>' ────────────

#[test]
fn superset_triple_term_is_admitted_as_an_element_and_a_value() {
    let expected = triple(
        CdtTerm::Iri("http://example.org/s".into()),
        CdtTerm::Iri("http://example.org/p".into()),
        CdtTerm::Literal(CdtLiteral::typed("1", XSD_INTEGER)),
    );
    assert_eq!(
        list_items("[<<(<http://example.org/s> <http://example.org/p> 1)>>]"),
        vec![expected]
    );
    let value = parse_map("{\"k\": <<(_:b <http://example.org/p> null)>>}").unwrap();
    assert_eq!(
        entries(&value)[0].value,
        triple(
            CdtTerm::Blank("b".into()),
            CdtTerm::Iri("http://example.org/p".into()),
            CdtTerm::Null,
        )
    );
    // A triple term nests composites, and survives the canonical round trip.
    let nested = parse_list("[<<(<http://example.org/s> <http://example.org/p> [1,2])>>]").unwrap();
    let canonical = nested.canonical_lexical();
    assert_eq!(
        parse_list(&canonical).unwrap().canonical_lexical(),
        canonical
    );
    // A triple term takes exactly three components.
    assert!(parse_list("[<<(<http://example.org/s> <http://example.org/p>)>>]").is_err());
    assert!(parse_list("[<<(1 2 3 4)>>]").is_err());
}

// ── Superset 2: LANGTAG gains RDF 1.2's '--' direction suffix ─────────────────

#[test]
fn superset_directional_literals_are_admitted_in_both_directions() {
    assert_eq!(
        list_items("[\"hello\"@en--ltr]"),
        vec![CdtTerm::Literal(CdtLiteral::dir_lang(
            "hello",
            "en",
            TextDirection::Ltr
        ))]
    );
    assert_eq!(
        list_items("[\"\u{645}\u{631}\u{62d}\u{628}\u{627}\"@ar--rtl]"),
        vec![CdtTerm::Literal(CdtLiteral::dir_lang(
            "\u{645}\u{631}\u{62d}\u{628}\u{627}",
            "ar",
            TextDirection::Rtl
        ))]
    );
    // The direction is a separate component, not part of the language tag.
    let CdtTerm::Literal(literal) = &list_items("[\"x\"@en-GB--rtl]")[0] else {
        panic!("expected a literal");
    };
    assert_eq!(literal.language.as_deref(), Some("en-GB"));
    assert_eq!(literal.direction, Some(TextDirection::Rtl));
    // Only the two RDF 1.2 directions exist.
    assert!(matches!(
        parse_list("[\"x\"@en--up]"),
        Err(CdtError::BadLanguageTag { .. })
    ));
}

// ── Ill-formed shapes ─────────────────────────────────────────────────────────

#[test]
fn ill_formed_unterminated_list() {
    let error = parse_list("[1, 2").unwrap_err();
    assert!(matches!(error, CdtError::UnexpectedEnd { .. }), "{error:?}");
    assert_eq!(error.offset(), 5);
}

#[test]
fn ill_formed_unterminated_map() {
    let error = parse_map("{\"a\": 1").unwrap_err();
    assert!(matches!(error, CdtError::UnexpectedEnd { .. }), "{error:?}");
    assert_eq!(error.offset(), 7);
}

#[test]
fn ill_formed_duplicate_map_key() {
    let error = parse_map("{\"a\": 1, \"a\": 2}").unwrap_err();
    let CdtError::DuplicateMapKey { offset, key } = error else {
        panic!("expected a duplicate-key error, got {error:?}");
    };
    assert_eq!(offset, 9);
    assert_eq!(key, "\"a\"^^<http://www.w3.org/2001/XMLSchema#string>");
    // The distinctness rule is on the key TERM, so the shorthand spelling of a key
    // collides with its explicit spelling — they are one and the same RDF term.
    assert!(matches!(
        parse_map("{1: \"a\", \"1\"^^<http://www.w3.org/2001/XMLSchema#integer>: \"b\"}"),
        Err(CdtError::DuplicateMapKey { .. })
    ));
    // …but a different LEXICAL form of the same value stays a different key.
    assert!(
        parse_map("{1: \"a\", \"01\"^^<http://www.w3.org/2001/XMLSchema#integer>: \"b\"}").is_ok()
    );
}

/// A map built programmatically reports the duplicate key too, and its offset points
/// at the key inside the canonical form the map would have had.
#[test]
fn a_programmatic_duplicate_map_key_carries_the_same_diagnostic() {
    let entry = |key: &str, value: &str| CdtEntry {
        key: CdtKey::Literal(CdtLiteral::plain(key)),
        value: CdtTerm::Literal(CdtLiteral::plain(value)),
    };
    let error = CdtValue::map(vec![entry("a", "1"), entry("a", "2")])
        .expect_err("two entries under one key are not a map");
    let CdtError::DuplicateMapKey { offset, key } = error else {
        panic!("expected a duplicate-key error, got {error:?}");
    };
    let rendered = "\"a\"^^<http://www.w3.org/2001/XMLSchema#string>";
    assert_eq!(key, rendered);
    // `{` + the first entry (`key` `:` `value` `,`).
    let first_value = "\"1\"^^<http://www.w3.org/2001/XMLSchema#string>";
    assert_eq!(offset, 1 + rendered.len() + 1 + first_value.len() + 1);

    // A map whose keys are pairwise distinct is built, sorted into key order.
    let ok = CdtValue::map(vec![entry("b", "2"), entry("a", "1")]).expect("distinct keys");
    assert_eq!(
        ok.canonical_lexical(),
        format!(
            "{{{rendered}:{first_value},\"b\"^^<http://www.w3.org/2001/XMLSchema#string>:\"2\"^^<http://www.w3.org/2001/XMLSchema#string>}}"
        )
    );
}

#[test]
fn ill_formed_relative_iriref() {
    let error = parse_list("[<relative/path>]").unwrap_err();
    let CdtError::NotAbsoluteIri { offset, iri, .. } = error else {
        panic!("expected a relative-IRI error, got {error:?}");
    };
    assert_eq!(offset, 1);
    assert_eq!(iri, "relative/path");
    // An absolute IRI at the same position is accepted, so the test is about
    // absoluteness and not about the position.
    assert!(parse_list("[<http://example.org/path>]").is_ok());
}

#[test]
fn ill_formed_bad_escape() {
    let error = parse_list("[\"a\\qb\"]").unwrap_err();
    assert!(matches!(error, CdtError::BadEscape { .. }), "{error:?}");
    assert_eq!(error.offset(), 3);
    // A truncated UCHAR is a bad escape too, not a silently short code point.
    assert!(matches!(
        parse_list("[\"\\u12\"]"),
        Err(CdtError::BadEscape { .. })
    ));
    // A surrogate code point is not a Unicode scalar value.
    assert!(matches!(
        parse_list("[\"\\uD800\"]"),
        Err(CdtError::BadEscape { .. })
    ));
    // The eight legal ECHARs and both UCHAR widths do decode.
    assert_eq!(
        list_items("[\"\\t\\b\\n\\r\\f\\\"\\'\\\\\\u0041\\U00000042\"]"),
        vec![CdtTerm::Literal(CdtLiteral::plain(
            "\t\u{8}\n\r\u{c}\"'\\AB"
        ))]
    );
}

#[test]
fn ill_formed_trailing_comma() {
    let error = parse_list("[1,]").unwrap_err();
    assert!(matches!(error, CdtError::Unexpected { .. }), "{error:?}");
    assert_eq!(error.offset(), 3);
    assert!(parse_map("{\"a\": 1,}").is_err());
    // A leading comma is no better.
    assert!(parse_list("[,1]").is_err());
}

#[test]
fn ill_formed_null_as_a_map_key() {
    let error = parse_map("{null: 1}").unwrap_err();
    assert!(matches!(error, CdtError::Unexpected { .. }), "{error:?}");
    assert_eq!(error.offset(), 1);
    // `null` remains legal as a map VALUE — the restriction is on the key only.
    assert!(parse_map("{\"a\": null}").is_ok());
}

// ── Canonical round trips, one per language-direction case ────────────────────

/// parse -> canonical -> parse -> canonical must be byte-identical.
fn assert_canonical_round_trip(lexical: &str) {
    let first = parse_list(lexical).expect("the lexical form is well formed");
    let canonical = first.canonical_lexical();
    let second = parse_list(&canonical).expect("the canonical form re-parses");
    assert_eq!(second.canonical_lexical(), canonical);
    assert_eq!(second, first);
}

#[test]
fn round_trip_language_tag_with_no_direction() {
    assert_canonical_round_trip("[\"hello\"@en]");
    assert_eq!(
        parse_list("[\"hello\"@en]").unwrap().canonical_lexical(),
        "[\"hello\"@en]"
    );
}

#[test]
fn round_trip_language_tag_with_ltr_direction() {
    assert_canonical_round_trip("[\"hello\"@en--ltr]");
    assert_eq!(
        parse_list("[\"hello\"@en--ltr]")
            .unwrap()
            .canonical_lexical(),
        "[\"hello\"@en--ltr]"
    );
}

#[test]
fn round_trip_language_tag_with_rtl_direction() {
    let lexical = "[\"\u{645}\u{631}\u{62d}\u{628}\u{627}\"@ar--rtl]";
    assert_canonical_round_trip(lexical);
    assert_eq!(parse_list(lexical).unwrap().canonical_lexical(), lexical);
}

#[test]
fn canonical_form_always_spells_shorthands_explicitly() {
    assert_eq!(
        parse_list("[1, 1.5, 1e0, true, \"s\", null]")
            .unwrap()
            .canonical_lexical(),
        "[\"1\"^^<http://www.w3.org/2001/XMLSchema#integer>,\
          \"1.5\"^^<http://www.w3.org/2001/XMLSchema#decimal>,\
          \"1e0\"^^<http://www.w3.org/2001/XMLSchema#double>,\
          \"true\"^^<http://www.w3.org/2001/XMLSchema#boolean>,\
          \"s\"^^<http://www.w3.org/2001/XMLSchema#string>,\
          null]"
    );
}
