// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SPARQL operator, casting and grammar rules, from query text to answer.
//!
//! * NaN against a number: SPARQL §17.3 maps `=`, `<`, `>`, `<=`, `>=` on numerics to
//!   XPath F&O `op:numeric-equal`/`-less-than`/`-greater-than`, every one of which is
//!   `false` for a NaN operand, and `!=` to `fn:not(op:numeric-equal)`, so `true`. The
//!   answers are definite booleans, never an unbound type error.
//! * Casting (§17.5): a literal whose datatype has no row in the casting table — a
//!   non-XSD datatype, a language-tagged string — casts to no numeric, boolean or
//!   temporal target; casting any literal to `xsd:string` stays allowed.
//! * Keywords match case-insensitively except `a` (SPARQL 1.1 §19.8), so `TRUE` and
//!   `False` are the boolean literals.
//! * `GROUP BY` and `ORDER BY` need at least one condition, and a `HAVING` condition is
//!   a `Constraint`, never a bare variable or literal.
//!
//! Every refusal sits beside a neighbouring query that is valid and still answers.

use std::sync::Arc;

use purrdf_core::{
    RdfDataset, RdfDatasetBuilder, RdfLiteral, SparqlRequest, SparqlResult, TermValue,
};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const EX: &str = "http://example.org/";

/// `ex:s ex:p 1`, plus — with `with_nan` — `ex:n ex:p "NaN"^^xsd:double`.
fn dataset_with(with_nan: bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}s"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let literal = |builder: &mut RdfDatasetBuilder, lexical: &str, datatype: &str| {
        builder.intern_literal(RdfLiteral {
            lexical_form: lexical.to_owned(),
            datatype: Some(format!("{XSD}{datatype}")),
            language: None,
            direction: None,
        })
    };
    let one = literal(&mut builder, "1", "integer");
    builder.push_quad(s, p, one, None);
    if with_nan {
        let n = builder.intern_iri(&format!("{EX}n"));
        let nan = literal(&mut builder, "NaN", "double");
        builder.push_quad(n, p, nan, None);
    }
    builder.freeze().expect("a dataset")
}

fn evaluate_over(
    dataset: &RdfDataset,
    query: &str,
) -> Result<SparqlResult, purrdf_core::RdfDiagnostic> {
    NativeSparqlEngine::new().query_with_options_view(
        dataset,
        SparqlRequest {
            query: &format!("PREFIX xsd: <{XSD}> {query}"),
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions::EMPTY,
    )
}

fn evaluate(query: &str) -> Result<SparqlResult, purrdf_core::RdfDiagnostic> {
    evaluate_over(&dataset_with(false), query)
}

fn solutions(
    result: Result<SparqlResult, purrdf_core::RdfDiagnostic>,
    query: &str,
) -> Vec<Vec<Option<TermValue>>> {
    match result.unwrap_or_else(|e| panic!("evaluate `{query}`: {e:?}")) {
        SparqlResult::Solutions { rows, .. } => rows,
        other => panic!("expected solutions for `{query}`, got {other:?}"),
    }
}

fn rows(query: &str) -> Vec<Vec<Option<TermValue>>> {
    solutions(evaluate(query), query)
}

/// The one cell `SELECT (<expression> AS ?y) {}` binds, as `(lexical, datatype)`, or
/// `None` when the expression is an error and leaves `?y` unbound.
fn select(expression: &str) -> Option<(String, String)> {
    let rows = rows(&format!("SELECT ({expression} AS ?y) WHERE {{}}"));
    assert_eq!(rows.len(), 1, "{expression}");
    match rows[0][0].as_ref() {
        None => None,
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            ..
        }) => Some((lexical_form.clone(), datatype.clone())),
        Some(other) => panic!("`{expression}` bound a non-literal {other:?}"),
    }
}

fn boolean(expression: &str) -> Option<bool> {
    select(expression).map(|(lexical, datatype)| {
        assert_eq!(datatype, format!("{XSD}boolean"), "{expression}");
        match lexical.as_str() {
            "true" => true,
            "false" => false,
            other => panic!("`{expression}` bound the boolean lexical {other:?}"),
        }
    })
}

fn assert_parse_refused(query: &str) {
    let err = evaluate(query).expect_err(query);
    assert_eq!(err.code, "native-sparql-query-parse", "`{query}`: {err:?}");
}

const NAN_DOUBLE: &str = "\"NaN\"^^xsd:double";
const NAN_FLOAT: &str = "\"NaN\"^^xsd:float";

#[test]
fn nan_against_a_number_compares_false_and_differs_true() {
    for other in [
        "0.1e0",
        "1",
        "2.5",
        "\"1.0\"^^xsd:float",
        "\"INF\"^^xsd:double",
    ] {
        for (op, expected) in [
            ("=", false),
            ("!=", true),
            ("<", false),
            (">", false),
            ("<=", false),
            (">=", false),
        ] {
            for expression in [
                format!("{NAN_DOUBLE} {op} {other}"),
                format!("{other} {op} {NAN_DOUBLE}"),
                format!("{NAN_FLOAT} {op} {other}"),
            ] {
                assert_eq!(boolean(&expression), Some(expected), "{expression}");
            }
        }
    }
}

#[test]
fn nan_does_not_order_against_itself() {
    // `<=`/`>=` are the logical-or of a strict comparison and `op:numeric-equal`, both
    // false for NaN — the identical term is no exception.
    for nan in [NAN_DOUBLE, NAN_FLOAT] {
        for op in ["<", ">", "<=", ">="] {
            assert_eq!(
                boolean(&format!("{nan} {op} {nan}")),
                Some(false),
                "{nan} {op}"
            );
        }
    }
    assert_eq!(
        boolean(&format!("{NAN_DOUBLE} <= {NAN_FLOAT}")),
        Some(false)
    );
    // The neighbours: an ordinary identical term still orders against itself.
    assert_eq!(boolean("1 <= 1"), Some(true));
    assert_eq!(boolean("2.5 >= 2.5"), Some(true));
}

#[test]
fn nan_equals_nothing_not_even_itself_while_same_term_holds() {
    // SPARQL 1.2 §17.4.2.2: "The Operator Mapping for "=" is the function
    // op:numeric-equal which is defined to return false when comparing arguments
    // involving NaN. However, sameTerm(...NaN, ...NaN) is true."
    for (a, b) in [
        (NAN_DOUBLE, NAN_DOUBLE),
        (NAN_FLOAT, NAN_FLOAT),
        (NAN_FLOAT, NAN_DOUBLE),
    ] {
        assert_eq!(boolean(&format!("{a} = {b}")), Some(false), "{a} = {b}");
        assert_eq!(boolean(&format!("{a} != {b}")), Some(true), "{a} != {b}");
    }
    assert_eq!(
        boolean(&format!("sameTerm({NAN_DOUBLE}, {NAN_DOUBLE})")),
        Some(true)
    );
    assert_eq!(
        boolean(&format!("sameTerm({NAN_FLOAT}, {NAN_DOUBLE})")),
        Some(false)
    );
    assert_eq!(
        boolean(&format!("{NAN_DOUBLE} IN (1, {NAN_DOUBLE})")),
        Some(false)
    );
    assert_eq!(
        boolean(&format!("{NAN_DOUBLE} NOT IN (1, {NAN_DOUBLE})")),
        Some(true)
    );
    // The neighbours: an ordinary identical term is still equal, and in its own list.
    assert_eq!(boolean("1 = 1"), Some(true));
    assert_eq!(boolean("1 IN (2, 1)"), Some(true));
    // A stored NaN is not equal to itself, so `FILTER(?o = ?o)` drops its row and
    // keeps the ordinary one; `sameTerm` keeps both.
    let dataset = dataset_with(true);
    let query = "SELECT ?s WHERE { ?s ?p ?o FILTER(?o = ?o) }";
    let kept = solutions(evaluate_over(&dataset, query), query);
    assert_eq!(
        kept,
        vec![vec![Some(TermValue::Iri(format!("{EX}s")))]],
        "{query}"
    );
    let query = "SELECT ?s WHERE { ?s ?p ?o FILTER(sameTerm(?o, ?o)) }";
    assert_eq!(
        solutions(evaluate_over(&dataset, query), query).len(),
        2,
        "{query}"
    );
    let query = "SELECT ?s WHERE { ?s ?p ?o FILTER(?o != ?o) }";
    let kept = solutions(evaluate_over(&dataset, query), query);
    assert_eq!(
        kept,
        vec![vec![Some(TermValue::Iri(format!("{EX}n")))]],
        "{query}"
    );
}

#[test]
fn nan_against_a_non_number_stays_a_type_error() {
    // No operator-mapping row pairs a numeric with a string, so the comparison is an
    // error, exactly as for any other number.
    assert_eq!(boolean(&format!("{NAN_DOUBLE} < \"abc\"")), None);
    assert_eq!(boolean("1 < \"abc\""), None);
}

#[test]
fn nan_comparisons_filter_rows_and_answer_in() {
    let query = format!(
        "SELECT ?s WHERE {{ ?s ?p ?o FILTER({NAN_DOUBLE} != ?o) FILTER(!({NAN_DOUBLE} < ?o)) }}"
    );
    assert_eq!(rows(&query).len(), 1, "{query}");
    assert_eq!(boolean(&format!("{NAN_DOUBLE} IN (1, 2)")), Some(false));
    assert_eq!(boolean(&format!("{NAN_DOUBLE} NOT IN (1, 2)")), Some(true));
}

#[test]
fn a_cast_from_a_datatype_outside_the_table_is_an_error() {
    let custom = format!("\"1\"^^<{EX}customType>");
    for target in [
        "double", "float", "decimal", "integer", "int", "boolean", "dateTime", "date",
    ] {
        assert_eq!(select(&format!("xsd:{target}({custom})")), None, "{target}");
    }
    assert_eq!(
        select(&format!("xsd:double(\"1.5\"^^<{EX}customType>)")),
        None
    );
    assert_eq!(
        select(&format!(
            "xsd:dateTime(\"2020-01-01T00:00:00Z\"^^<{EX}customType>)"
        )),
        None
    );
    assert_eq!(select("xsd:double(\"1\"@en)"), None);
    assert_eq!(select("xsd:integer(\"1\"@en--ltr)"), None);
}

#[test]
fn a_cast_the_table_marks_never_allowed_is_an_error() {
    // Numeric/boolean sources cast to no temporal or other non-numeric target, and
    // temporal sources to no numeric or boolean target.
    assert_eq!(select("xsd:gYear(2020)"), None);
    assert_eq!(select("xsd:dateTime(1)"), None);
    assert_eq!(select("xsd:integer(\"2020\"^^xsd:gYear)"), None);
    assert_eq!(select("xsd:double(\"P1D\"^^xsd:duration)"), None);
    assert_eq!(
        select("xsd:boolean(\"2020-01-01T00:00:00Z\"^^xsd:dateTime)"),
        None
    );
}

#[test]
fn every_cast_the_table_allows_still_casts() {
    let double = format!("{XSD}double");
    let integer = format!("{XSD}integer");
    let boolean_dt = format!("{XSD}boolean");
    let string = format!("{XSD}string");
    let date_time = format!("{XSD}dateTime");
    // The `str` row: a simple literal and an explicit xsd:string decide by lexical form.
    assert_eq!(
        select("xsd:double(\"1.5\")"),
        Some(("1.5E0".to_owned(), double.clone()))
    );
    assert_eq!(
        select("xsd:double(\"1.5\"^^xsd:string)"),
        Some(("1.5E0".to_owned(), double.clone()))
    );
    assert_eq!(
        select("xsd:dateTime(\"2002-10-10T17:00:00Z\")"),
        Some(("2002-10-10T17:00:00Z".to_owned(), date_time.clone()))
    );
    assert_eq!(
        select("xsd:boolean(\"true\")"),
        Some(("true".to_owned(), boolean_dt.clone()))
    );
    // A type XPath derives from xsd:string takes the same row.
    assert_eq!(
        select("xsd:integer(\"12\"^^xsd:token)"),
        Some(("12".to_owned(), integer.clone()))
    );
    // The numeric and boolean rows.
    assert_eq!(select("xsd:double(1)"), Some(("1.0E0".to_owned(), double)));
    assert_eq!(
        select("xsd:integer(2.5)"),
        Some(("2".to_owned(), integer.clone()))
    );
    assert_eq!(select("xsd:integer(true)"), Some(("1".to_owned(), integer)));
    assert_eq!(
        select("xsd:boolean(0.0e0)"),
        Some(("false".to_owned(), boolean_dt))
    );
    assert_eq!(
        select("xsd:decimal(\"1.25\"^^xsd:float)"),
        Some(("1.25".to_owned(), format!("{XSD}decimal")))
    );
    // The dateTime row.
    assert_eq!(
        select("xsd:dateTime(\"2002-10-10T17:00:00Z\"^^xsd:dateTime)"),
        Some(("2002-10-10T17:00:00Z".to_owned(), date_time))
    );
    // A Gregorian target from a simple literal is untouched.
    assert_eq!(
        select("xsd:gYear(\"2020\")"),
        Some(("2020".to_owned(), format!("{XSD}gYear")))
    );
    // The `str` column is `Y` for every row, a custom datatype and an IRI included.
    assert_eq!(
        select(&format!("xsd:string(\"1.5\"^^<{EX}customType>)")),
        Some(("1.5".to_owned(), string.clone()))
    );
    assert_eq!(
        select("xsd:string(\"chat\"@fr)"),
        Some(("chat".to_owned(), string.clone()))
    );
    assert_eq!(
        select(&format!("xsd:string(<{EX}a>)")),
        Some((format!("{EX}a"), string))
    );
}

#[test]
fn boolean_keywords_match_case_insensitively() {
    // W3C sparql10 expr-builtin `case-insensitive-booleans`.
    let answer = rows("SELECT (TRUE as ?t) (False as ?f) {}");
    let literal = |value: &str| {
        Some(TermValue::Literal {
            lexical_form: value.to_owned(),
            datatype: format!("{XSD}boolean"),
            language: None,
            direction: None,
        })
    };
    assert_eq!(answer, vec![vec![literal("true"), literal("false")]]);
    for (spelling, expected) in [
        ("true", true),
        ("TRUE", true),
        ("tRuE", true),
        ("false", false),
        ("FALSE", false),
        ("False", false),
    ] {
        assert_eq!(boolean(spelling), Some(expected), "{spelling}");
    }
    // In a VALUES block and a triple pattern too.
    assert_eq!(
        rows("SELECT ?x WHERE { VALUES ?x { TRUE fAlSe } }"),
        vec![vec![literal("true")], vec![literal("false")]]
    );
    assert_eq!(rows("SELECT * WHERE { ?s ?p TRUE }").len(), 0);
    // `a` is the one keyword that stays case-sensitive.
    assert_parse_refused("SELECT * WHERE { ?s A ?o }");
    assert_eq!(rows("SELECT * WHERE { ?s a ?o }").len(), 0);
}

#[test]
fn group_by_and_order_by_need_a_condition() {
    for query in [
        "SELECT * WHERE { ?s ?p ?o } GROUP BY",
        "SELECT * WHERE { ?s ?p ?o } ORDER BY",
        "SELECT * WHERE { ?s ?p ?o } ORDER BY LIMIT 1",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY HAVING (true)",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ORDER BY ?s",
        "SELECT ?s WHERE { { SELECT ?s WHERE { ?s ?p ?o } GROUP BY } }",
    ] {
        assert_parse_refused(query);
    }
    for query in [
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s",
        "SELECT ?k WHERE { ?s ?p ?o } GROUP BY (STR(?s) AS ?k)",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s ORDER BY ?s",
        "SELECT * WHERE { ?s ?p ?o } ORDER BY ?s",
        "SELECT * WHERE { ?s ?p ?o } ORDER BY DESC(?o)",
        "SELECT * WHERE { ?s ?p ?o } ORDER BY asc(?o) ?s LIMIT 1",
        "SELECT * WHERE { ?s ?p ?o } ORDER BY STR(?s)",
        "SELECT ?s WHERE { { SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s } }",
    ] {
        assert_eq!(rows(query).len(), 1, "{query}");
    }
}

#[test]
fn a_having_condition_is_a_constraint() {
    for query in [
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s HAVING",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s HAVING ?s",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s HAVING 1",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s HAVING true",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s HAVING (true) ?s",
    ] {
        assert_parse_refused(query);
    }
    for query in [
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s HAVING (true)",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s HAVING (COUNT(?o) > 0)",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s HAVING COUNT(?o)",
        "SELECT ?s WHERE { ?s ?p ?o } GROUP BY ?s HAVING bound(?s) (true)",
    ] {
        assert_eq!(rows(query).len(), 1, "{query}");
    }
}

/// The lexical form an XSD cast of `source` to `xsd:{target}` binds — asserting the
/// bound literal is of the target datatype — or `None` for a cast error.
fn cast(target: &str, source: &str) -> Option<String> {
    select(&format!("xsd:{target}({source})")).map(|(lexical, datatype)| {
        assert_eq!(datatype, format!("{XSD}{target}"), "xsd:{target}({source})");
        lexical
    })
}

#[test]
fn date_time_casts_keep_their_components_and_timezone() {
    // XPath F&O 3.1 §19.3: dateTime to date, time and the Gregorian types keeps the
    // components the target has, timezone included.
    let dt = "\"2002-10-10T17:30:05.5+05:00\"^^xsd:dateTime";
    assert_eq!(cast("date", dt), Some("2002-10-10+05:00".to_owned()));
    assert_eq!(cast("time", dt), Some("17:30:05.5+05:00".to_owned()));
    assert_eq!(cast("gYear", dt), Some("2002+05:00".to_owned()));
    assert_eq!(cast("gYearMonth", dt), Some("2002-10+05:00".to_owned()));
    assert_eq!(cast("gMonth", dt), Some("--10+05:00".to_owned()));
    assert_eq!(cast("gMonthDay", dt), Some("--10-10+05:00".to_owned()));
    assert_eq!(cast("gDay", dt), Some("---10+05:00".to_owned()));
    let utc = "\"-0044-03-15T12:00:00Z\"^^xsd:dateTime";
    assert_eq!(cast("date", utc), Some("-0044-03-15Z".to_owned()));
    assert_eq!(cast("gYear", utc), Some("-0044Z".to_owned()));
    let local = "\"2002-10-10T17:00:00\"^^xsd:dateTime";
    assert_eq!(cast("date", local), Some("2002-10-10".to_owned()));
    assert_eq!(cast("time", local), Some("17:00:00".to_owned()));
    // date to dateTime is midnight of that date, with its timezone.
    let d = "\"2002-10-10-05:00\"^^xsd:date";
    assert_eq!(
        cast("dateTime", d),
        Some("2002-10-10T00:00:00-05:00".to_owned())
    );
    assert_eq!(cast("gYearMonth", d), Some("2002-10-05:00".to_owned()));
    assert_eq!(cast("gMonthDay", d), Some("--10-10-05:00".to_owned()));
    assert_eq!(
        cast("gDay", "\"2002-10-10\"^^xsd:date"),
        Some("---10".to_owned())
    );
    // The neighbours the lexical path already served: same-type casts and simple
    // literals.
    assert_eq!(
        cast("date", "\"2002-10-10Z\"^^xsd:date"),
        Some("2002-10-10Z".to_owned())
    );
    assert_eq!(
        cast("dateTime", "\"2002-10-10T00:00:00\""),
        Some("2002-10-10T00:00:00".to_owned())
    );
    assert_eq!(cast("gYear", "\"2002\""), Some("2002".to_owned()));
}

#[test]
fn duration_and_binary_casts_convert_by_value() {
    let dur = "\"P1Y2M3DT4H\"^^xsd:duration";
    assert_eq!(cast("yearMonthDuration", dur), Some("P1Y2M".to_owned()));
    assert_eq!(cast("dayTimeDuration", dur), Some("P3DT4H".to_owned()));
    assert_eq!(
        cast("duration", "\"P3DT4H\"^^xsd:dayTimeDuration"),
        Some("P3DT4H".to_owned())
    );
    // The two binary spaces share spellings that mean different bytes: "abcd" is
    // three bytes as base64 and two as hex. The cast keeps the bytes.
    assert_eq!(
        cast("hexBinary", "\"abcd\"^^xsd:base64Binary"),
        Some("69B71D".to_owned())
    );
    assert_eq!(
        cast("base64Binary", "\"69B71D\"^^xsd:hexBinary"),
        Some("abcd".to_owned())
    );
    assert_eq!(
        cast("hexBinary", "\"0fb7\"^^xsd:hexBinary"),
        Some("0FB7".to_owned())
    );
}

#[test]
fn a_temporal_or_binary_cast_the_table_forbids_is_an_error() {
    for (target, source) in [
        ("time", "\"2002-10-10\"^^xsd:date"),
        ("date", "\"17:00:00\"^^xsd:time"),
        ("dateTime", "\"17:00:00\"^^xsd:time"),
        ("gYear", "\"17:00:00\"^^xsd:time"),
        ("date", "\"2002\"^^xsd:gYear"),
        ("dateTime", "\"2002-10\"^^xsd:gYearMonth"),
        ("gMonth", "\"2002-10\"^^xsd:gYearMonth"),
        ("gYear", "\"2020\"^^xsd:hexBinary"),
        ("date", "\"P1D\"^^xsd:duration"),
        ("hexBinary", "\"2002-10-10\"^^xsd:date"),
        ("dayTimeDuration", "\"2002-10-10\"^^xsd:date"),
        // An ill-typed source has no value to cast.
        ("date", "\"not a date\"^^xsd:dateTime"),
    ] {
        assert_eq!(cast(target, source), None, "xsd:{target}({source})");
    }
}

/// The `str` row is every type derived from `xsd:string`, not `xsd:string` alone, and
/// `xsd:dateTimeStamp` is the `dT` row: both cast to the calendar types, as they do to
/// the numeric ones. A `xsd:dateTimeStamp` spelling without its required timezone
/// holds no value, and the `N` neighbours stay errors.
#[test]
fn string_derived_and_date_time_stamp_sources_cast_to_calendar_targets() {
    let instant = "2002-10-10T17:30:05Z";
    for datatype in ["token", "normalizedString", "NCName", "language"] {
        let source = format!("\"{instant}\"^^xsd:{datatype}");
        assert_eq!(
            cast("dateTime", &source),
            Some(instant.to_owned()),
            "{source}"
        );
        assert_eq!(
            cast("date", &source),
            None,
            "{source}: the spelling is no date"
        );
    }
    assert_eq!(
        cast("date", "\"2002-10-10\"^^xsd:token"),
        Some("2002-10-10".to_owned())
    );
    assert_eq!(
        cast("gYear", "\"2002\"^^xsd:token"),
        Some("2002".to_owned())
    );
    let stamp = "\"2002-10-10T17:30:05.5+05:00\"^^xsd:dateTimeStamp";
    assert_eq!(
        cast("dateTime", stamp),
        Some("2002-10-10T17:30:05.5+05:00".to_owned())
    );
    assert_eq!(cast("date", stamp), Some("2002-10-10+05:00".to_owned()));
    assert_eq!(cast("gYear", stamp), Some("2002+05:00".to_owned()));
    assert_eq!(
        cast("time", &format!("\"{instant}\"^^xsd:dateTimeStamp")),
        Some("17:30:05Z".to_owned())
    );
    // No timezone: not a dateTimeStamp value at all.
    assert_eq!(
        cast("dateTime", "\"2002-10-10T17:30:05\"^^xsd:dateTimeStamp"),
        None
    );
    // The dateTime row casts to no numeric, boolean, duration or binary target.
    assert_eq!(cast("integer", stamp), None);
    assert_eq!(cast("boolean", stamp), None);
    assert_eq!(cast("dayTimeDuration", stamp), None);
    assert_eq!(cast("hexBinary", stamp), None);
}

/// XPath F&O 3.1 §19.1: `xsd:anyURI` casts to `xsd:string` and nothing else — its
/// lexical form spelling a date does not make it one. `xsd:QName` and `xsd:NOTATION`
/// likewise. The `xsd:string` neighbour still casts.
#[test]
fn an_any_uri_casts_to_string_alone() {
    for (target, source) in [
        ("date", "\"2024-01-01\"^^xsd:anyURI"),
        ("dateTime", "\"2024-01-01T00:00:00Z\"^^xsd:anyURI"),
        ("gYear", "\"2024\"^^xsd:anyURI"),
        ("integer", "\"1\"^^xsd:anyURI"),
        ("hexBinary", "\"0F\"^^xsd:anyURI"),
        ("dayTimeDuration", "\"PT1S\"^^xsd:anyURI"),
        ("date", "\"2024-01-01\"^^xsd:QName"),
        ("date", "\"2024-01-01\"^^xsd:NOTATION"),
    ] {
        assert_eq!(cast(target, source), None, "xsd:{target}({source})");
    }
    assert_eq!(
        cast("string", "\"2024-01-01\"^^xsd:anyURI"),
        Some("2024-01-01".to_owned())
    );
    assert_eq!(
        cast("string", "\"ex:a\"^^xsd:QName"),
        Some("ex:a".to_owned())
    );
}

/// A number, a boolean and a calendar value are no binary value, whatever their
/// spelling (`2020` spells two bytes of hex): the table marks every such pair `N`.
/// The binary and string neighbours still cast.
#[test]
fn numbers_booleans_and_calendar_values_cast_to_no_binary_or_duration() {
    for (target, source) in [
        ("hexBinary", "2020"),
        ("hexBinary", "\"2020\"^^xsd:integer"),
        ("base64Binary", "\"abcd\"^^xsd:integer"),
        ("hexBinary", "true"),
        ("hexBinary", "\"2020\"^^xsd:gYear"),
        ("base64Binary", "\"2020\"^^xsd:gYear"),
        ("duration", "1"),
        ("dayTimeDuration", "1.5"),
        ("yearMonthDuration", "false"),
    ] {
        assert_eq!(cast(target, source), None, "xsd:{target}({source})");
    }
    assert_eq!(cast("hexBinary", "\"2020\""), Some("2020".to_owned()));
    assert_eq!(
        cast("hexBinary", "\"2020\"^^xsd:hexBinary"),
        Some("2020".to_owned())
    );
    assert_eq!(cast("duration", "\"P1D\""), Some("P1D".to_owned()));
}

/// A duration subtype keeps only its own component, so a cast that has none of it is
/// the zero of the target: `P1Y` has no day-time part and `PT5S` no year-month part.
#[test]
fn a_duration_cast_with_no_component_of_the_target_is_its_zero() {
    assert_eq!(
        cast("dayTimeDuration", "\"P1Y\"^^xsd:duration"),
        Some("PT0S".to_owned())
    );
    assert_eq!(
        cast("yearMonthDuration", "\"PT5S\"^^xsd:duration"),
        Some("P0M".to_owned())
    );
    assert_eq!(
        cast("dayTimeDuration", "\"P1Y\"^^xsd:yearMonthDuration"),
        Some("PT0S".to_owned())
    );
    assert_eq!(
        cast("yearMonthDuration", "\"P1D\"^^xsd:dayTimeDuration"),
        Some("P0M".to_owned())
    );
}

/// The gate admits every numeric datatype XSD derives, and the boolean, to every
/// numeric and boolean target — the neighbours of the refusals above.
#[test]
fn derived_numeric_and_boolean_sources_cast_to_numeric_and_boolean_targets() {
    assert_eq!(cast("integer", "\"1\"^^xsd:int"), Some("1".to_owned()));
    assert_eq!(
        cast("integer", "\"7\"^^xsd:unsignedByte"),
        Some("7".to_owned())
    );
    assert_eq!(
        cast("double", "\"3\"^^xsd:nonNegativeInteger"),
        Some("3.0E0".to_owned())
    );
    assert_eq!(cast("decimal", "\"-2\"^^xsd:short"), Some("-2".to_owned()));
    assert_eq!(cast("boolean", "1"), Some("true".to_owned()));
    assert_eq!(cast("boolean", "0.0"), Some("false".to_owned()));
    assert_eq!(cast("decimal", "true"), Some("1".to_owned()));
    assert_eq!(cast("integer", "false"), Some("0".to_owned()));
    assert_eq!(cast("float", "\"1\"^^xsd:long"), Some("1.0E0".to_owned()));
}

/// NaN against a string is the same type error a number against a string is, for
/// `=` and `!=` as for `<` — no operator-mapping row pairs them. Against an IRI it
/// is the same `RDFterm-equal` answer a number gets: two different terms.
#[test]
fn nan_equality_against_a_non_number_answers_as_a_number_does() {
    for operator in ["=", "!=", ">", ">="] {
        let nan = select(&format!("{NAN_DOUBLE} {operator} \"abc\""));
        assert_eq!(nan, None, "NaN {operator} \"abc\"");
        assert_eq!(nan, select(&format!("1 {operator} \"abc\"")), "{operator}");
    }
    assert_eq!(
        boolean(&format!("{NAN_DOUBLE} = <{EX}a>")),
        boolean(&format!("1 = <{EX}a>"))
    );
    assert_eq!(boolean(&format!("{NAN_DOUBLE} = <{EX}a>")), Some(false));
    assert_eq!(boolean(&format!("{NAN_DOUBLE} != <{EX}a>")), Some(true));
}

/// The comparison fix leaves ordering alone: `ORDER BY` sorts NaN after every
/// number (before them descending), whatever order the rows arrive in, and `MIN` /
/// `MAX` follow that order — so NaN is the maximum and never the minimum.
#[test]
fn nan_sorts_after_every_number_and_min_max_follow_the_sort() {
    let lexicals = |query: &str| -> Vec<String> {
        rows(query)
            .into_iter()
            .flatten()
            .map(|cell| match cell {
                Some(TermValue::Literal { lexical_form, .. }) => lexical_form,
                other => panic!("{query}: {other:?}"),
            })
            .collect()
    };
    for values in [
        "2 \"NaN\"^^xsd:double 1 \"-INF\"^^xsd:double",
        "\"NaN\"^^xsd:double \"-INF\"^^xsd:double 2 1",
        "1 2 \"-INF\"^^xsd:double \"NaN\"^^xsd:double",
    ] {
        assert_eq!(
            lexicals(&format!(
                "SELECT ?o WHERE {{ VALUES ?o {{ {values} }} }} ORDER BY ?o"
            )),
            ["-INF", "1", "2", "NaN"],
            "{values}"
        );
        assert_eq!(
            lexicals(&format!(
                "SELECT ?o WHERE {{ VALUES ?o {{ {values} }} }} ORDER BY DESC(?o)"
            )),
            ["NaN", "2", "1", "-INF"],
            "{values}"
        );
        assert_eq!(
            lexicals(&format!(
                "SELECT (MIN(?o) AS ?m) (MAX(?o) AS ?x) WHERE {{ VALUES ?o {{ {values} }} }}"
            )),
            ["-INF", "NaN"],
            "{values}"
        );
    }
}

/// A triple term compares componentwise under `=`, each numeric component under
/// `op:numeric-equal`, so a triple term holding a NaN equals nothing, itself
/// included — the identical term as well as a distinct spelling of it. `sameTerm`
/// still says it is itself. The neighbours without a NaN stay equal, by identity and
/// by value (`1` against `1.0`).
#[test]
fn a_triple_term_holding_nan_is_unequal_to_itself() {
    let nan = format!("<<( <{EX}a> <{EX}b> {NAN_DOUBLE} )>>");
    let nested = format!("<<( <{EX}a> <{EX}b> {nan} )>>");
    for term in [&nan, &nested] {
        assert_eq!(boolean(&format!("{term} = {term}")), Some(false), "{term}");
        assert_eq!(boolean(&format!("{term} != {term}")), Some(true), "{term}");
        assert_eq!(
            boolean(&format!("{term} IN ({term})")),
            Some(false),
            "{term}"
        );
        assert_eq!(
            boolean(&format!("sameTerm({term}, {term})")),
            Some(true),
            "{term}"
        );
    }
    // The same term bound twice, from the data, through a variable: still unequal.
    let query = format!("SELECT ?t WHERE {{ BIND({nan} AS ?t) FILTER(?t = ?t) }}");
    assert!(rows(&query).is_empty(), "{query}");
    // A distinct spelling of the same NaN (float against double) is unequal too.
    assert_eq!(
        boolean(&format!("{nan} = <<( <{EX}a> <{EX}b> {NAN_FLOAT} )>>")),
        Some(false)
    );
    // Neighbours: no NaN, so identical and value-equal triple terms are equal.
    let one = format!("<<( <{EX}a> <{EX}b> 1 )>>");
    assert_eq!(boolean(&format!("{one} = {one}")), Some(true));
    assert_eq!(boolean(&format!("{one} IN ({one})")), Some(true));
    assert_eq!(
        boolean(&format!("{one} = <<( <{EX}a> <{EX}b> 1.0 )>>")),
        Some(true)
    );
    let query = format!("SELECT ?t WHERE {{ BIND({one} AS ?t) FILTER(?t = ?t) }}");
    assert_eq!(rows(&query).len(), 1, "{query}");
}
