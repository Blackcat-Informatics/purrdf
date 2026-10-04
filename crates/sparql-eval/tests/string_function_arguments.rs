// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The string built-ins' argument rules, from query text through the public engine.
//!
//! * `SUBSTR` is XPath `fn:substring` over `xsd:integer` arguments: it keeps the
//!   characters at positions `p` with `start <= p < start + length`, so a start at or
//!   below zero eats into the length rather than being clamped to one; arithmetic on
//!   the bounds never overflows; a supplied length that is unbound is an error, not an
//!   omitted length; and the result keeps the source's language tag and base direction.
//! * `CONTAINS`, `STRSTARTS` and `STRENDS` require argument compatibility (SPARQL 1.1
//!   §17.4.1.1, extended by RDF 1.2 to the base direction): a simple or `xsd:string`
//!   second argument goes with anything, two tagged arguments need the same language
//!   and the same direction, and a tagged second argument never goes with an untagged
//!   first one. An incompatible pair is an error, not `false`.
//! * `REGEX`'s flags, when supplied, are a simple literal; a supplied flags argument
//!   that is unbound or not a simple literal is an error, never "no flags".
//!
//! Every refusal is paired with a neighbouring call that must still answer.

use purrdf_core::term_fixture::{empty_dataset, one_quad};
use purrdf_core::{RdfDataset, RdfTextDirection, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// The one value `SELECT (<expression> AS ?y) {}` binds, `None` when it is unbound.
fn select(expression: &str) -> Option<TermValue> {
    select_over(
        &empty_dataset(),
        &format!("SELECT ({expression} AS ?y) WHERE {{}}"),
    )
}

fn select_over(dataset: &RdfDataset, query: &str) -> Option<TermValue> {
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            dataset,
            SparqlRequest {
                query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        )
        .unwrap_or_else(|e| panic!("evaluate `{query}`: {e}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions for `{query}`");
    };
    assert_eq!(rows.len(), 1, "{query}");
    rows[0][0].clone()
}

fn string(lexical: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: lexical.to_owned(),
        datatype: format!("{XSD}string"),
        language: None,
        direction: None,
    }
}

fn boolean(value: bool) -> TermValue {
    TermValue::Literal {
        lexical_form: value.to_string(),
        datatype: format!("{XSD}boolean"),
        language: None,
        direction: None,
    }
}

// ---------------------------------------------------------------------------
// SUBSTR
// ---------------------------------------------------------------------------

#[test]
fn substr_keeps_the_xpath_interval_for_starts_at_or_below_zero() {
    for (expression, expected) in [
        // The neighbouring valid calls: the ordinary one-based interval.
        (r#"SUBSTR("12345", 2, 3)"#, "234"),
        (r#"SUBSTR("12345", 1, 3)"#, "123"),
        (r#"SUBSTR("12345", 2)"#, "2345"),
        (r#"SUBSTR("12345", 5, 1)"#, "5"),
        (r#"SUBSTR("12345", 6, 1)"#, ""),
        (r#"SUBSTR("12345", 1, 0)"#, ""),
        (r#"SUBSTR("a😀é\u0000z", 2, 3)"#, "😀é\0"),
        // fn:substring("12345", 0, 3) is "12": positions 0, 1 and 2, of which only 1
        // and 2 exist.
        (r#"SUBSTR("12345", 0, 3)"#, "12"),
        (r#"SUBSTR("12345", -1, 3)"#, "1"),
        (r#"SUBSTR("12345", -3, 5)"#, "1"),
        (r#"SUBSTR("12345", -3, 4)"#, ""),
        (r#"SUBSTR("12345", 5, -3)"#, ""),
        (r#"SUBSTR("12345", -2)"#, "12345"),
        (r#"SUBSTR("12345", 0)"#, "12345"),
        (r#"SUBSTR("", 1, 1)"#, ""),
    ] {
        assert_eq!(select(expression), Some(string(expected)), "{expression}");
    }
}

#[test]
fn substr_bounds_do_not_overflow() {
    let i64_max = i64::MAX;
    let i64_min = i64::MIN;
    let i128_max = i128::MAX;
    // `-170…728` written bare is the negation of a literal one past `i128::MAX`; the
    // typed literal spells the minimum itself.
    let i128_min = format!("\"{}\"^^<{XSD}integer>", i128::MIN);
    for (expression, expected) in [
        (format!(r#"SUBSTR("12345", 2, {i64_max})"#), "2345"),
        (format!(r#"SUBSTR("12345", {i64_min}, {i64_max})"#), ""),
        (format!(r#"SUBSTR("12345", {i64_min})"#), "12345"),
        (format!(r#"SUBSTR("12345", {i64_max}, 1)"#), ""),
        (format!(r#"SUBSTR("12345", 2, {i128_max})"#), "2345"),
        (format!(r#"SUBSTR("12345", {i128_max}, {i128_max})"#), ""),
        (format!(r#"SUBSTR("12345", {i128_min}, {i128_max})"#), ""),
        (format!(r#"SUBSTR("12345", {i128_min}, {i128_min})"#), ""),
        (format!(r#"SUBSTR("12345", {i128_min})"#), "12345"),
        // A hugely negative start whose length reaches back into the string keeps the
        // prefix the interval covers: positions up to (but not including) 3.
        (
            format!(
                r#"SUBSTR("12345", -{i64_max}, {})"#,
                i128::from(i64_max) + 3
            ),
            "12",
        ),
    ] {
        assert_eq!(select(&expression), Some(string(expected)), "{expression}");
    }
}

#[test]
fn substr_supplied_unbound_length_is_an_error_not_an_omitted_length() {
    // The refusals: a length that is supplied and unbound, or an error.
    assert_eq!(select(r#"SUBSTR("12345", 1, ?unbound)"#), None);
    assert_eq!(select(r#"SUBSTR("12345", 1, 1/0)"#), None);
    assert_eq!(select(r#"SUBSTR("12345", ?unbound)"#), None);
    // The refusals SPARQL's signature already made: non-integer arguments.
    assert_eq!(select(r#"SUBSTR("12345", 1.5, 3)"#), None);
    assert_eq!(select(r#"SUBSTR("12345", "1", 3)"#), None);
    assert_eq!(select(r#"SUBSTR("12345", 1, "3")"#), None);
    // The neighbours: an omitted length, and a length bound through a variable.
    assert_eq!(select(r#"SUBSTR("12345", 2)"#), Some(string("2345")));
    let query = r#"SELECT (SUBSTR("12345", 1, ?n) AS ?y) WHERE { VALUES ?n { 2 } }"#;
    assert_eq!(select_over(&empty_dataset(), query), Some(string("12")));
    // A derived integer type is an integer.
    assert_eq!(
        select(&format!(
            r#"SUBSTR("12345", "2"^^<{XSD}unsignedByte>, "2"^^<{XSD}long>)"#
        )),
        Some(string("23"))
    );
}

#[test]
fn substr_keeps_the_language_tag_and_base_direction() {
    assert_eq!(
        select(r#"SUBSTR("a😀é"@en--rtl, 2, 1)"#),
        Some(TermValue::Literal {
            lexical_form: "😀".to_owned(),
            datatype: format!("{RDF}dirLangString"),
            language: Some("en".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        })
    );
    assert_eq!(
        select(r#"SUBSTR("chat"@fr, 0, 3)"#),
        Some(TermValue::Literal {
            lexical_form: "ch".to_owned(),
            datatype: format!("{RDF}langString"),
            language: Some("fr".to_owned()),
            direction: None,
        })
    );
}

// ---------------------------------------------------------------------------
// CONTAINS / STRSTARTS / STRENDS
// ---------------------------------------------------------------------------

/// Each `(first, second)` pair, its expected answer for the three predicates with the
/// needle `"b"` against the haystack `"abc"`: `CONTAINS` holds, `STRSTARTS` and
/// `STRENDS` do not, so a compatible pair answers `true`, `false`, `false`.
const PAIRS: &[(&str, &str, bool)] = &[
    // Compatible: a simple or xsd:string second argument goes with anything.
    (r#""abc""#, r#""b""#, true),
    (r#""abc""#, r#""b"^^xsd:string"#, true),
    (r#""abc"@en"#, r#""b""#, true),
    (r#""abc"@en--rtl"#, r#""b""#, true),
    (r#""abc"@en--ltr"#, r#""b"^^xsd:string"#, true),
    // Compatible: the same language (compared without case) and the same direction.
    (r#""abc"@en"#, r#""b"@en"#, true),
    (r#""abc"@en"#, r#""b"@EN"#, true),
    (r#""abc"@en--rtl"#, r#""b"@en--rtl"#, true),
    (r#""abc"@EN--ltr"#, r#""b"@en--ltr"#, true),
    // Incompatible: a tagged second argument against an untagged first one.
    (r#""abc""#, r#""b"@en"#, false),
    (r#""abc"^^xsd:string"#, r#""b"@en--rtl"#, false),
    // Incompatible: different languages.
    (r#""abc"@en"#, r#""b"@fr"#, false),
    (r#""abc"@en--rtl"#, r#""b"@fr--rtl"#, false),
    // Incompatible: the same language, different directions.
    (r#""abc"@en--rtl"#, r#""b"@en--ltr"#, false),
    (r#""abc"@en--rtl"#, r#""b"@en"#, false),
    (r#""abc"@en"#, r#""b"@en--rtl"#, false),
];

#[test]
fn substring_predicates_require_compatible_arguments_including_direction() {
    for &(first, second, compatible) in PAIRS {
        for (function, holds) in [("CONTAINS", true), ("STRSTARTS", false), ("STRENDS", false)] {
            let expression = format!("{function}({first}, {second})");
            let query = format!("PREFIX xsd: <{XSD}> SELECT ({expression} AS ?y) WHERE {{}}");
            let expected = compatible.then(|| boolean(holds));
            assert_eq!(
                select_over(&empty_dataset(), &query),
                expected,
                "{expression}"
            );
        }
    }
}

/// The same rule over values that arrive from the data rather than from query text,
/// so the per-row path is held to it too.
#[test]
fn substring_predicates_check_compatibility_on_bound_values() {
    for &(first, second, compatible) in PAIRS {
        let query = format!(
            "PREFIX xsd: <{XSD}> SELECT (CONTAINS(?h, ?n) AS ?y) \
             WHERE {{ VALUES (?h ?n) {{ ({first} {second}) }} }}"
        );
        let expected = compatible.then(|| boolean(true));
        assert_eq!(select_over(&empty_dataset(), &query), expected, "{query}");
    }
    // A stored directional literal against a stored and a written needle.
    let dataset = one_quad(
        "http://example.org/s",
        "http://example.org/p",
        "http://example.org/o",
    );
    for (needle, expected) in [
        (r#""b"@en--rtl"#, Some(boolean(true))),
        (r#""z"@en--rtl"#, Some(boolean(false))),
        (r#""b""#, Some(boolean(true))),
        (r#""b"@en--ltr"#, None),
        (r#""b"@en"#, None),
    ] {
        let query = format!(
            "SELECT (STRSTARTS(STRLANGDIR(\"bcd\", \"en\", \"rtl\"), {needle}) AS ?y) \
             WHERE {{ ?s ?p ?o }}"
        );
        assert_eq!(select_over(&dataset, &query), expected, "{query}");
    }
}

#[test]
fn substring_predicates_answer_false_for_a_compatible_miss() {
    assert_eq!(
        select(r#"CONTAINS("abc"@en--rtl, "z"@en--rtl)"#),
        Some(boolean(false))
    );
    assert_eq!(select(r#"STRENDS("abc"@en, "c"@en)"#), Some(boolean(true)));
    assert_eq!(select(r#"STRSTARTS("abc"@en, "a")"#), Some(boolean(true)));
    // STR and LANG read as plain strings, whatever the operand's tag.
    assert_eq!(
        select(r#"CONTAINS(STR("abc"@en), "b")"#),
        Some(boolean(true))
    );
    assert_eq!(
        select(r#"STRSTARTS(LANG("abc"@en--rtl), "e")"#),
        Some(boolean(true))
    );
}

#[test]
fn strbefore_and_strafter_share_the_compatibility_rule_and_keep_the_direction() {
    let rtl = |lexical: &str| TermValue::Literal {
        lexical_form: lexical.to_owned(),
        datatype: format!("{RDF}dirLangString"),
        language: Some("en".to_owned()),
        direction: Some(RdfTextDirection::Rtl),
    };
    for (expression, expected) in [
        (r#"STRBEFORE("abc"@en--rtl, "b")"#, Some(rtl("a"))),
        (r#"STRAFTER("abc"@en--rtl, "b")"#, Some(rtl("c"))),
        (r#"STRBEFORE("abc"@en--rtl, "b"@EN--rtl)"#, Some(rtl("a"))),
        (r#"STRBEFORE("abc"@en--rtl, "z")"#, Some(string(""))),
        (r#"STRBEFORE("abc"@en--rtl, "b"@en--ltr)"#, None),
        (r#"STRAFTER("abc"@en--rtl, "b"@en)"#, None),
        (r#"STRAFTER("abc"@en, "b"@en--ltr)"#, None),
        (r#"STRAFTER("abc", "b"@en)"#, None),
    ] {
        assert_eq!(select(expression), expected, "{expression}");
    }
}

// ---------------------------------------------------------------------------
// REGEX flags
// ---------------------------------------------------------------------------

#[test]
fn regex_supplied_flags_must_be_a_bound_simple_literal() {
    // The refusals: a supplied flags argument that is unbound, an error, not a string,
    // or a tagged string.
    for expression in [
        r#"REGEX("abc", "b", ?unbound)"#,
        r#"REGEX("abc", "B", ?unbound)"#,
        r#"REGEX("abc", "b", 1/0)"#,
        r#"REGEX("abc", "b", 1)"#,
        r#"REGEX("abc", "b", <http://example.org/i>)"#,
        r#"REGEX("abc", "B", "i"@en)"#,
        r#"REGEX("abc", "B", STR(?unbound))"#,
    ] {
        assert_eq!(select(expression), None, "{expression}");
    }
    // The neighbours: omitted flags, empty flags, valid flags, and flags bound per row.
    for (expression, expected) in [
        (r#"REGEX("abc", "b")"#, true),
        (r#"REGEX("abc", "B")"#, false),
        (r#"REGEX("abc", "B", "")"#, false),
        (r#"REGEX("abc", "B", "i")"#, true),
        (
            r#"REGEX("abc", "B", "i"^^<http://www.w3.org/2001/XMLSchema#string>)"#,
            true,
        ),
        (r#"REGEX("abc", "B", STR("i"))"#, true),
        (r#"REGEX("abc"@en, "B", "i")"#, true),
    ] {
        assert_eq!(select(expression), Some(boolean(expected)), "{expression}");
    }
    for (flags, expected) in [
        (r#""i""#, Some(boolean(true))),
        (r#""""#, Some(boolean(false))),
        ("1", None),
        (r#""i"@en"#, None),
    ] {
        let query = format!(
            "SELECT (REGEX(\"abc\", ?p, ?f) AS ?y) WHERE {{ VALUES (?p ?f) {{ (\"B\" {flags}) }} }}"
        );
        assert_eq!(select_over(&empty_dataset(), &query), expected, "{query}");
    }
    // An unbound flags cell in the same position.
    let query = "SELECT (REGEX(\"abc\", ?p, ?f) AS ?y) WHERE { VALUES (?p ?f) { (\"b\" UNDEF) } }";
    assert_eq!(select_over(&empty_dataset(), query), None, "{query}");
}
