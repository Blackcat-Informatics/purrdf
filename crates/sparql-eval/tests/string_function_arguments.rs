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
//! * `REGEX`'s and `REPLACE`'s flags, when supplied, are a simple literal; a supplied
//!   flags argument that is unbound or not a simple literal is an error, never "no
//!   flags". Their pattern (and `REPLACE`'s replacement) is a simple literal too; the
//!   text they search may be any string.
//! * A string a built-in derives from a string argument keeps its language tag and
//!   base direction (`UCASE`, `LCASE`, `REPLACE`, `SUBSTR`, `STRBEFORE`, `STRAFTER`,
//!   and `CONCAT` when every argument shares them), and `STRLANGDIR`, like `STRLANG`,
//!   refuses a lexical form that already carries a tag.
//!
//! Every refusal is paired with a neighbouring call that must still answer.

use purrdf_core::term_fixture::{build_page, empty_dataset, one_quad};
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

// ---------------------------------------------------------------------------
// facets of the result
// ---------------------------------------------------------------------------

fn tagged(lexical: &str, language: &str, direction: Option<RdfTextDirection>) -> TermValue {
    TermValue::Literal {
        lexical_form: lexical.to_owned(),
        datatype: format!(
            "{RDF}{}",
            if direction.is_some() {
                "dirLangString"
            } else {
                "langString"
            }
        ),
        language: Some(language.to_owned()),
        direction,
    }
}

#[test]
fn string_producers_keep_the_language_tag_and_base_direction() {
    let rtl = Some(RdfTextDirection::Rtl);
    let ltr = Some(RdfTextDirection::Ltr);
    for (expression, expected) in [
        (r#"UCASE("abc"@en--rtl)"#, tagged("ABC", "en", rtl)),
        (r#"LCASE("ABC"@ar--ltr)"#, tagged("abc", "ar", ltr)),
        (r#"UCASE("abc"@en)"#, tagged("ABC", "en", None)),
        (r#"UCASE("abc")"#, string("ABC")),
        (
            r#"REPLACE("abc"@en--rtl, "b", "x")"#,
            tagged("axc", "en", rtl),
        ),
        (r#"REPLACE("abc"@en, "b", "x")"#, tagged("axc", "en", None)),
        (r#"REPLACE("abc", "b", "x")"#, string("axc")),
        (
            r#"CONCAT("ab"@en--rtl, "c"@en--rtl)"#,
            tagged("abc", "en", rtl),
        ),
        (r#"CONCAT("ab"@en--rtl, "c"@en--ltr)"#, string("abc")),
        (r#"CONCAT("ab"@en--rtl, "c"@en)"#, string("abc")),
        (r#"SUBSTR("abc"@en--ltr, 2)"#, tagged("bc", "en", ltr)),
        (
            r#"STRLANGDIR("abc", "en", "rtl")"#,
            tagged("abc", "en", rtl),
        ),
        // ENCODE_FOR_URI returns a simple literal by definition (§17.4.3.11).
        (r#"ENCODE_FOR_URI("a b"@en--rtl)"#, string("a%20b")),
    ] {
        assert_eq!(select(expression), Some(expected), "{expression}");
    }
}

/// `STRLANGDIR`, like `STRLANG`, takes a lexical form that carries no tag: re-tagging an
/// already tagged string would silently discard its language and direction.
#[test]
fn strlangdir_refuses_a_tagged_lexical_form() {
    for expression in [
        r#"STRLANGDIR("abc"@fr, "en", "rtl")"#,
        r#"STRLANGDIR("abc"@fr--ltr, "en", "rtl")"#,
        r#"STRLANG("abc"@fr, "en")"#,
    ] {
        assert_eq!(select(expression), None, "{expression}");
    }
    let xsd_string = format!(r#"STRLANGDIR("abc"^^<{XSD}string>, "en", "rtl")"#);
    assert_eq!(
        select(&xsd_string),
        Some(tagged("abc", "en", Some(RdfTextDirection::Rtl)))
    );
}

#[test]
fn replace_supplied_flags_must_be_a_bound_simple_literal() {
    for expression in [
        r#"REPLACE("abc", "B", "x", ?unbound)"#,
        r#"REPLACE("abc", "b", "x", ?unbound)"#,
        r#"REPLACE("abc", "b", "x", 1/0)"#,
        r#"REPLACE("abc", "b", "x", 1)"#,
        r#"REPLACE("abc", "B", "x", "i"@en)"#,
    ] {
        assert_eq!(select(expression), None, "{expression}");
    }
    for (expression, expected) in [
        (r#"REPLACE("abc", "B", "x")"#, "abc"),
        (r#"REPLACE("abc", "B", "x", "")"#, "abc"),
        (r#"REPLACE("abc", "B", "x", "i")"#, "axc"),
    ] {
        assert_eq!(select(expression), Some(string(expected)), "{expression}");
    }
    for (flags, expected) in [
        (r#""i""#, Some(string("axc"))),
        ("1", None),
        ("UNDEF", None),
    ] {
        let query = format!(
            "SELECT (REPLACE(\"abc\", ?p, \"x\", ?f) AS ?y) \
             WHERE {{ VALUES (?p ?f) {{ (\"B\" {flags}) }} }}"
        );
        assert_eq!(select_over(&empty_dataset(), &query), expected, "{query}");
    }
}

#[test]
fn regex_and_replace_patterns_are_simple_literals() {
    // The refusals: a tagged pattern or replacement.
    for expression in [
        r#"REGEX("abc", "b"@en)"#,
        r#"REGEX("abc"@en, "b"@en)"#,
        r#"REGEX("abc", "b"@en--rtl)"#,
        r#"REPLACE("abc", "b"@en, "x")"#,
        r#"REPLACE("abc"@en, "b"@en, "x")"#,
        r#"REPLACE("abc", "b", "x"@en)"#,
        r#"REPLACE("abc", "b", "x"@en--ltr)"#,
    ] {
        assert_eq!(select(expression), None, "{expression}");
    }
    // The neighbours: a plain or xsd:string pattern against tagged text.
    for (expression, expected) in [
        (r#"REGEX("abc"@en, "b")"#.to_owned(), boolean(true)),
        (r#"REGEX("abc"@en--rtl, "b")"#.to_owned(), boolean(true)),
        (
            format!(r#"REGEX("abc"@en, "b"^^<{XSD}string>)"#),
            boolean(true),
        ),
        (
            r#"REPLACE("abc"@en--rtl, "b", "x")"#.to_owned(),
            tagged("axc", "en", Some(RdfTextDirection::Rtl)),
        ),
        (
            format!(r#"REPLACE("abc", "b"^^<{XSD}string>, "x"^^<{XSD}string>)"#),
            string("axc"),
        ),
    ] {
        assert_eq!(select(&expression), Some(expected), "{expression}");
    }
    // The same over values bound per row.
    for (pattern, expected) in [(r#""b""#, Some(boolean(true))), (r#""b"@en"#, None)] {
        let query =
            format!("SELECT (REGEX(\"abc\"@en, ?p) AS ?y) WHERE {{ VALUES ?p {{ {pattern} }} }}");
        assert_eq!(select_over(&empty_dataset(), &query), expected, "{query}");
    }
}

// ---------------------------------------------------------------------------
// simple-literal-only arguments
// ---------------------------------------------------------------------------

/// The hash built-ins take a simple literal or `xsd:string` (SPARQL 1.1 §17.4.6, and
/// SEP-0008 for SHA-3): a language-tagged or directional argument is an error, while a
/// plain and an `xsd:string` argument hash to the same digest.
#[test]
fn hash_built_ins_take_only_simple_literals() {
    for (function, digest_of_abc) in [
        ("MD5", "900150983cd24fb0d6963f7d28e17f72"),
        ("SHA1", "a9993e364706816aba3e25717850c26c9cd0d89d"),
        (
            "SHA256",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            "SHA384",
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed\
             8086072ba1e7cc2358baeca134c825a7",
        ),
        (
            "SHA512",
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
        ),
        (
            "SHA3-256",
            "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532",
        ),
    ] {
        for argument in [r#""abc""#.to_owned(), format!(r#""abc"^^<{XSD}string>"#)] {
            let expression = format!("{function}({argument})");
            assert_eq!(
                select(&expression),
                Some(string(digest_of_abc)),
                "{expression}"
            );
        }
        for argument in [
            r#""abc"@en"#,
            r#""abc"@en--rtl"#,
            "1",
            "<http://example.org/abc>",
        ] {
            let expression = format!("{function}({argument})");
            assert_eq!(select(&expression), None, "{expression}");
        }
        // The same over a value bound per row.
        for (argument, expected) in [
            (r#""abc""#, Some(string(digest_of_abc))),
            (r#""abc"@en"#, None),
            (r#""abc"@en--ltr"#, None),
        ] {
            let query =
                format!("SELECT ({function}(?v) AS ?y) WHERE {{ VALUES ?v {{ {argument} }} }}");
            assert_eq!(select_over(&empty_dataset(), &query), expected, "{query}");
        }
    }
    for function in ["SHA3-224", "SHA3-384", "SHA3-512"] {
        assert!(
            select(&format!(r#"{function}("abc")"#)).is_some(),
            "{function}"
        );
        assert_eq!(
            select(&format!(r#"{function}("abc"^^<{XSD}string>)"#)),
            select(&format!(r#"{function}("abc")"#)),
            "{function}"
        );
        assert_eq!(
            select(&format!(r#"{function}("abc"@en)"#)),
            None,
            "{function}"
        );
        assert_eq!(
            select(&format!(r#"{function}("abc"@en--rtl)"#)),
            None,
            "{function}"
        );
    }
}

/// The language and direction arguments of `STRLANG` and `STRLANGDIR` are simple
/// literals (§17.4.2.4 and SPARQL 1.2), as is `LANGMATCHES`'s tag and range
/// (§17.4.3.13), `BNODE`'s label (§17.4.2.9) and `IRI`'s string form (§17.4.2.8).
#[test]
fn simple_literal_arguments_refuse_tagged_strings() {
    for expression in [
        r#"STRLANG("abc", "en"@fr)"#,
        r#"STRLANG("abc", "en"@fr--ltr)"#,
        r#"STRLANGDIR("abc", "en"@fr, "rtl")"#,
        r#"STRLANGDIR("abc", "en", "rtl"@en)"#,
        r#"LANGMATCHES("en"@fr, "*")"#,
        r#"LANGMATCHES("en", "*"@en)"#,
        r#"LANGMATCHES("en"@fr--rtl, "en")"#,
        r#"BNODE("x"@en)"#,
        r#"BNODE("x"@en--rtl)"#,
        r#"IRI("http://example.org/x"@en)"#,
        r#"URI("http://example.org/x"@en--ltr)"#,
        "IRI(1)",
    ] {
        assert_eq!(select(expression), None, "{expression}");
    }
    let rtl = Some(RdfTextDirection::Rtl);
    for (expression, expected) in [
        (
            r#"STRLANG("abc", "en")"#.to_owned(),
            tagged("abc", "en", None),
        ),
        (
            format!(r#"STRLANG("abc", "en"^^<{XSD}string>)"#),
            tagged("abc", "en", None),
        ),
        (
            format!(r#"STRLANGDIR("abc", "en"^^<{XSD}string>, "rtl"^^<{XSD}string>)"#),
            tagged("abc", "en", rtl),
        ),
        (r#"LANGMATCHES("en", "*")"#.to_owned(), boolean(true)),
        (
            r#"LANGMATCHES(LANG("x"@en--rtl), "EN")"#.to_owned(),
            boolean(true),
        ),
        (
            format!(r#"LANGMATCHES("en-GB"^^<{XSD}string>, "en")"#),
            boolean(true),
        ),
        (r#"LANGMATCHES("fr", "en")"#.to_owned(), boolean(false)),
        (r#"isBLANK(BNODE("x"))"#.to_owned(), boolean(true)),
        (
            format!(r#"isBLANK(BNODE("x"^^<{XSD}string>))"#),
            boolean(true),
        ),
        (
            r#"IRI("http://example.org/x")"#.to_owned(),
            TermValue::Iri("http://example.org/x".to_owned()),
        ),
        (
            format!(r#"IRI("http://example.org/x"^^<{XSD}string>)"#),
            TermValue::Iri("http://example.org/x".to_owned()),
        ),
        (
            "URI(<http://example.org/x>)".to_owned(),
            TermValue::Iri("http://example.org/x".to_owned()),
        ),
    ] {
        assert_eq!(select(&expression), Some(expected), "{expression}");
    }
    // Under a base, a non-string or tagged literal still makes no IRI; a string does.
    for (argument, expected) in [
        ("1", None),
        (r#""x"@en"#, None),
        (
            r#""x""#,
            Some(TermValue::Iri("http://example.org/x".to_owned())),
        ),
    ] {
        let query = format!("BASE <http://example.org/> SELECT (IRI({argument}) AS ?y) WHERE {{}}");
        assert_eq!(select_over(&empty_dataset(), &query), expected, "{query}");
    }
    // LANGMATCHES over values bound per row.
    for (tag, expected) in [(r#""en""#, Some(boolean(true))), (r#""en"@fr"#, None)] {
        let query =
            format!("SELECT (LANGMATCHES(?t, \"*\") AS ?y) WHERE {{ VALUES ?t {{ {tag} }} }}");
        assert_eq!(select_over(&empty_dataset(), &query), expected, "{query}");
    }
}

// ---------------------------------------------------------------------------
// LANGMATCHES over empty tags and ranges
// ---------------------------------------------------------------------------

/// `LANGMATCHES` is false when the tag, the range or both are empty, and `"*"` matches
/// only a non-empty tag (SPARQL 1.1 §17.4.3.13, SPARQL 1.2 §17.4.3.11).
#[test]
fn langmatches_is_false_for_an_empty_tag_or_range() {
    for (expression, expected) in [
        (r#"LANGMATCHES("", "*")"#, false),
        (r#"LANGMATCHES("", "")"#, false),
        (r#"LANGMATCHES("en", "")"#, false),
        (r#"LANGMATCHES("", "en")"#, false),
        (r#"LANGMATCHES(LANG("abc"), "*")"#, false),
        (
            r#"LANGMATCHES(LANG("abc"^^<http://www.w3.org/2001/XMLSchema#string>), "*")"#,
            false,
        ),
        (r#"!LANGMATCHES(LANG("abc"), "*")"#, true),
        // The neighbours: a non-empty tag still matches `*`, a prefix and its case.
        (r#"LANGMATCHES(LANG("abc"@en), "*")"#, true),
        (r#"LANGMATCHES(LANG("abc"@ar--rtl), "*")"#, true),
        (r#"LANGMATCHES("en-US", "en")"#, true),
        (r#"LANGMATCHES("EN-us", "en-US")"#, true),
        (r#"LANGMATCHES("en-gb", "EN")"#, true),
        (r#"LANGMATCHES("en", "*")"#, true),
        (r#"LANGMATCHES("english", "en")"#, false),
    ] {
        assert_eq!(select(expression), Some(boolean(expected)), "{expression}");
    }
    // A LANG over a non-literal is an error, and so is the call over it.
    assert_eq!(
        select("LANGMATCHES(LANG(<http://example.org/abc>), \"*\")"),
        None
    );
}

/// The data of the W3C `sparql10/expr-builtin` `q-langMatches-*` tests
/// (`data-langMatches.ttl`).
fn lang_matches_data() -> std::sync::Arc<RdfDataset> {
    let ex = |local: &str| TermValue::iri(format!("http://example.org/#{local}"));
    build_page(&[
        (ex("x"), ex("p1"), TermValue::simple_literal("abc")),
        (ex("x"), ex("p2"), TermValue::iri("http://example.org/abc")),
        (ex("x"), ex("p3"), TermValue::lang_literal("abc", "en")),
        (ex("x"), ex("p4"), TermValue::lang_literal("abc", "en-gb")),
        (ex("x"), ex("p5"), TermValue::lang_literal("abc", "fr")),
    ])
}

/// Every `?p` local name `query` binds over [`lang_matches_data`], sorted.
fn lang_matches_rows(filter: &str) -> Vec<String> {
    let query =
        format!("PREFIX : <http://example.org/#> SELECT ?p {{ :x ?p ?v . FILTER {filter} . }}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*lang_matches_data(),
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        )
        .unwrap_or_else(|e| panic!("evaluate `{query}`: {e}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions for `{query}`");
    };
    let mut names: Vec<String> = rows
        .into_iter()
        .map(|row| match &row[0] {
            Some(TermValue::Iri(iri)) => iri.trim_start_matches("http://example.org/#").to_owned(),
            other => panic!("expected an IRI, got {other:?}"),
        })
        .collect();
    names.sort();
    names
}

/// The W3C `q-langMatches-1` .. `-4` queries and their expected results
/// (`result-langMatches-*.ttl`). `-3` keeps the three tagged literals and not the
/// simple one, whose `LANG` is empty; `-4` negates it, so the simple literal is the
/// one row (the IRI's `LANG` is an error, which drops its row from both).
#[test]
fn w3c_lang_matches_queries_answer_their_expected_rows() {
    for (name, filter, expected) in [
        (
            "q-langMatches-1",
            r#"langMatches(lang(?v), "en-GB")"#,
            &["p4"][..],
        ),
        (
            "q-langMatches-2",
            r#"langMatches(lang(?v), "en")"#,
            &["p3", "p4"][..],
        ),
        (
            "q-langMatches-3",
            r#"langMatches(lang(?v), "*")"#,
            &["p3", "p4", "p5"][..],
        ),
        (
            "q-langMatches-4",
            r#"(! langMatches(lang(?v), "*"))"#,
            &["p1"][..],
        ),
    ] {
        assert_eq!(lang_matches_rows(filter), expected, "{name}");
    }
}
