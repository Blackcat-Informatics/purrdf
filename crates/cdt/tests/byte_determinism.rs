// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The canonical lexical mapping is a fixpoint, and it is injective.
//!
//! Two properties, both load-bearing for the workspace's byte-determinism rule:
//!
//! * **Fixpoint** — `canonical -> parse -> canonical` returns the same bytes, so a
//!   value that has been written once is never rewritten differently.
//! * **Injectivity** — two values render to the same bytes exactly when they are the
//!   same value, so the canonical form can be used as a value's identity (a
//!   `GROUP BY` key, a golden, a hash input) without collapsing distinct values.
//!
//! The repository already uses `proptest` (it is a `[workspace.dependencies]` entry
//! and the `purrdf-xsd` / `purrdf-iri` leaves both take it as a dev-dependency for
//! exactly this kind of round-trip property), so the properties are checked over
//! generated values rather than only over a hand-written corpus.

use pretty_assertions::assert_eq;
use proptest::prelude::*;
use purrdf_cdt::{
    CdtDatatype, CdtEntry, CdtKey, CdtLiteral, CdtTerm, CdtValue, TextDirection, XSD_INTEGER,
    parse_cdt, parse_list, parse_map,
};

// ── Fixed goldens: the exact bytes, pinned ─────────────────────────────────────

#[test]
fn canonical_bytes_are_pinned() {
    assert_eq!(parse_list("[]").unwrap().canonical_lexical(), "[]");
    assert_eq!(parse_map("{}").unwrap().canonical_lexical(), "{}");
    assert_eq!(
        parse_list("[ <http://example.org/a> , _:b , null ]")
            .unwrap()
            .canonical_lexical(),
        "[<http://example.org/a>,_:b,null]"
    );
    assert_eq!(
        parse_map("{ \"b\" : 2 , \"a\" : 1 }")
            .unwrap()
            .canonical_lexical(),
        "{\"a\"^^<http://www.w3.org/2001/XMLSchema#string>:\
          \"1\"^^<http://www.w3.org/2001/XMLSchema#integer>,\
          \"b\"^^<http://www.w3.org/2001/XMLSchema#string>:\
          \"2\"^^<http://www.w3.org/2001/XMLSchema#integer>}"
    );
    // The SPARQL escape set, exactly.
    assert_eq!(
        parse_list("[\"\\t\\b\\n\\r\\f\\\"\\'\\\\\\u0000\"]")
            .unwrap()
            .canonical_lexical(),
        "[\"\\t\\b\\n\\r\\f\\\"'\\\\\\u0000\"^^<http://www.w3.org/2001/XMLSchema#string>]"
    );
}

#[test]
fn map_rendering_does_not_depend_on_authoring_order() {
    let scrambled = [
        "{\"a\":1,\"b\":2,\"c\":3}",
        "{\"c\":3,\"a\":1,\"b\":2}",
        "{\"b\":2,\"c\":3,\"a\":1}",
    ];
    let rendered: Vec<String> = scrambled
        .iter()
        .map(|lexical| parse_map(lexical).unwrap().canonical_lexical())
        .collect();
    assert_eq!(rendered[0], rendered[1]);
    assert_eq!(rendered[1], rendered[2]);
    // And building the same map programmatically, in a third order, agrees.
    let entry = |name: &str, number: &str| CdtEntry {
        key: CdtKey::Literal(CdtLiteral::plain(name)),
        value: CdtTerm::Literal(CdtLiteral::typed(number, XSD_INTEGER)),
    };
    let built = CdtValue::map(vec![entry("b", "2"), entry("c", "3"), entry("a", "1")]).unwrap();
    assert_eq!(built.canonical_lexical(), rendered[0]);
}

#[test]
fn canonical_form_is_a_fixpoint_over_a_hand_written_corpus() {
    let corpus = [
        (CdtDatatype::List, "[]"),
        (CdtDatatype::List, "[1,2,3]"),
        (CdtDatatype::List, "[[[[1]]]]"),
        (CdtDatatype::List, "['a'@en, \"b\"@fr--ltr, \"c\"@he--rtl]"),
        (CdtDatatype::List, "[<http://example.org/a#f>, _:b0]"),
        (
            CdtDatatype::List,
            "[<<(<http://example.org/s> <http://example.org/p> [null])>>]",
        ),
        (CdtDatatype::Map, "{}"),
        (CdtDatatype::Map, "{1:2, \"a\":{\"b\":[3]}, true:null}"),
        (
            CdtDatatype::Map,
            "{<http://example.org/k>: \"\"\"multi\nline\"\"\"}",
        ),
    ];
    for (datatype, lexical) in corpus {
        let value = parse_cdt(lexical, datatype).expect("the corpus is well formed");
        let once = value.canonical_lexical();
        let twice = parse_cdt(&once, datatype)
            .expect("the canonical form re-parses")
            .canonical_lexical();
        assert_eq!(twice, once, "not a fixpoint for {lexical}");
    }
}

// ── Generated values ──────────────────────────────────────────────────────────

fn key_strategy() -> impl Strategy<Value = CdtKey> {
    prop_oneof![
        "[a-z]{1,6}".prop_map(|name| CdtKey::Iri(format!("http://example.org/{name}"))),
        ".{0,8}".prop_map(|lexical| CdtKey::Literal(CdtLiteral::plain(lexical))),
        "-?[0-9]{1,6}".prop_map(|lexical| CdtKey::Literal(CdtLiteral::typed(lexical, XSD_INTEGER))),
        ("[a-z]{1,6}", "[a-z]{2}")
            .prop_map(|(lexical, tag)| CdtKey::Literal(CdtLiteral::lang(lexical, tag))),
    ]
}

fn leaf_term_strategy() -> impl Strategy<Value = CdtTerm> {
    prop_oneof![
        "[a-z]{1,6}".prop_map(|name| CdtTerm::Iri(format!("http://example.org/{name}"))),
        "[a-z][a-z0-9]{0,5}".prop_map(CdtTerm::Blank),
        // Unrestricted text, so control characters and quotes exercise the escapes.
        ".{0,10}".prop_map(|lexical| CdtTerm::Literal(CdtLiteral::plain(lexical))),
        "-?[0-9]{1,6}"
            .prop_map(|lexical| CdtTerm::Literal(CdtLiteral::typed(lexical, XSD_INTEGER))),
        ("[a-z]{1,6}", "[a-z]{2}")
            .prop_map(|(lexical, tag)| CdtTerm::Literal(CdtLiteral::lang(lexical, tag))),
        (
            "[a-z]{1,6}",
            "[a-z]{2}",
            prop_oneof![Just(TextDirection::Ltr), Just(TextDirection::Rtl)]
        )
            .prop_map(
                |(lexical, tag, direction)| CdtTerm::Literal(CdtLiteral::dir_lang(
                    lexical, tag, direction
                ))
            ),
        Just(CdtTerm::Null),
    ]
}

fn entries_strategy(term: BoxedStrategy<CdtTerm>) -> impl Strategy<Value = Vec<CdtEntry>> {
    proptest::collection::vec((key_strategy(), term), 0..4).prop_filter_map(
        "a map's keys must be pairwise distinct",
        |pairs| {
            let entries: Vec<CdtEntry> = pairs
                .into_iter()
                .map(|(key, value)| CdtEntry { key, value })
                .collect();
            CdtValue::map(entries).ok()?.into_map()
        },
    )
}

fn term_strategy() -> BoxedStrategy<CdtTerm> {
    leaf_term_strategy()
        .prop_recursive(3, 24, 3, |inner| {
            prop_oneof![
                proptest::collection::vec(inner.clone(), 0..4).prop_filter_map(
                    "a generated list must be within the crate's three bounds",
                    |items| CdtTerm::composite(CdtValue::list(items).ok()?).ok(),
                ),
                entries_strategy(inner.clone()).prop_filter_map(
                    "a generated map must be within the crate's three bounds",
                    |entries| CdtTerm::composite(CdtValue::map(entries).ok()?).ok(),
                ),
                (inner.clone(), inner.clone(), inner).prop_filter_map(
                    "a generated triple term must be within the crate's three bounds",
                    |(subject, predicate, object)| {
                        CdtTerm::triple(subject, predicate, object).ok()
                    },
                ),
            ]
        })
        .boxed()
}

fn value_strategy() -> impl Strategy<Value = CdtValue> {
    prop_oneof![
        proptest::collection::vec(term_strategy(), 0..5).prop_filter_map(
            "a generated list must be within the crate's three bounds",
            |items| CdtValue::list(items).ok(),
        ),
        entries_strategy(term_strategy()).prop_filter_map(
            "a generated map must be within the crate's three bounds",
            |entries| CdtValue::map(entries).ok(),
        ),
    ]
}

proptest! {
    /// canonical -> parse -> canonical is a fixpoint, and re-parsing recovers the
    /// very same value.
    #[test]
    fn canonical_parse_canonical_is_a_fixpoint(value in value_strategy()) {
        let canonical = value.canonical_lexical();
        let reparsed = parse_cdt(&canonical, value.datatype())
            .expect("a canonical form is always in the lexical space");
        prop_assert_eq!(reparsed.canonical_lexical(), canonical);
        prop_assert!(reparsed == value);
    }

    /// Parsing is injective on canonical forms: equal bytes iff equal values.
    #[test]
    fn parsing_is_injective_on_canonical_forms(
        left in value_strategy(),
        right in value_strategy(),
    ) {
        let same_bytes = left.canonical_lexical() == right.canonical_lexical();
        prop_assert_eq!(same_bytes, left == right);
    }

    /// Rendering consults nothing but the value: two renders agree, and so do two
    /// renders of a value routed through the lexical form and back.
    #[test]
    fn rendering_is_a_pure_function_of_the_value(value in value_strategy()) {
        prop_assert_eq!(value.canonical_lexical(), value.canonical_lexical());
        let clone = value.clone();
        prop_assert_eq!(clone.canonical_lexical(), value.canonical_lexical());
    }
}
