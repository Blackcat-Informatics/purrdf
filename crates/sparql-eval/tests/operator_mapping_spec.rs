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
//! * Keywords match case-insensitively except `a` (SPARQL 1.1 §19.3), so `TRUE` and
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

/// The `(lexical, datatype)` an XSD cast of `source` to `xsd:{target}` binds.
fn cast(target: &str, source: &str) -> Option<(String, String)> {
    select(&format!("xsd:{target}({source})"))
}

fn typed(lexical: &str, datatype: &str) -> Option<(String, String)> {
    Some((lexical.to_owned(), format!("{XSD}{datatype}")))
}

#[test]
fn date_time_casts_keep_their_components_and_timezone() {
    // XPath F&O 3.1 §19.3: dateTime to date, time and the Gregorian types keeps the
    // components the target has, timezone included.
    let dt = "\"2002-10-10T17:30:05.5+05:00\"^^xsd:dateTime";
    assert_eq!(cast("date", dt), typed("2002-10-10+05:00", "date"));
    assert_eq!(cast("time", dt), typed("17:30:05.5+05:00", "time"));
    assert_eq!(cast("gYear", dt), typed("2002+05:00", "gYear"));
    assert_eq!(cast("gYearMonth", dt), typed("2002-10+05:00", "gYearMonth"));
    assert_eq!(cast("gMonth", dt), typed("--10+05:00", "gMonth"));
    assert_eq!(cast("gMonthDay", dt), typed("--10-10+05:00", "gMonthDay"));
    assert_eq!(cast("gDay", dt), typed("---10+05:00", "gDay"));
    let utc = "\"-0044-03-15T12:00:00Z\"^^xsd:dateTime";
    assert_eq!(cast("date", utc), typed("-0044-03-15Z", "date"));
    assert_eq!(cast("gYear", utc), typed("-0044Z", "gYear"));
    let local = "\"2002-10-10T17:00:00\"^^xsd:dateTime";
    assert_eq!(cast("date", local), typed("2002-10-10", "date"));
    assert_eq!(cast("time", local), typed("17:00:00", "time"));
    // date to dateTime is midnight of that date, with its timezone.
    let d = "\"2002-10-10-05:00\"^^xsd:date";
    assert_eq!(
        cast("dateTime", d),
        typed("2002-10-10T00:00:00-05:00", "dateTime")
    );
    assert_eq!(cast("gYearMonth", d), typed("2002-10-05:00", "gYearMonth"));
    assert_eq!(cast("gMonthDay", d), typed("--10-10-05:00", "gMonthDay"));
    assert_eq!(
        cast("gDay", "\"2002-10-10\"^^xsd:date"),
        typed("---10", "gDay")
    );
    // The neighbours the lexical path already served: same-type casts and simple
    // literals.
    assert_eq!(
        cast("date", "\"2002-10-10Z\"^^xsd:date"),
        typed("2002-10-10Z", "date")
    );
    assert_eq!(
        cast("dateTime", "\"2002-10-10T00:00:00\""),
        typed("2002-10-10T00:00:00", "dateTime")
    );
    assert_eq!(cast("gYear", "\"2002\""), typed("2002", "gYear"));
}

#[test]
fn duration_and_binary_casts_convert_by_value() {
    let dur = "\"P1Y2M3DT4H\"^^xsd:duration";
    assert_eq!(
        cast("yearMonthDuration", dur),
        typed("P1Y2M", "yearMonthDuration")
    );
    assert_eq!(
        cast("dayTimeDuration", dur),
        typed("P3DT4H", "dayTimeDuration")
    );
    assert_eq!(
        cast("duration", "\"P3DT4H\"^^xsd:dayTimeDuration"),
        typed("P3DT4H", "duration")
    );
    // The two binary spaces share spellings that mean different bytes: "abcd" is
    // three bytes as base64 and two as hex. The cast keeps the bytes.
    assert_eq!(
        cast("hexBinary", "\"abcd\"^^xsd:base64Binary"),
        typed("69B71D", "hexBinary")
    );
    assert_eq!(
        cast("base64Binary", "\"69B71D\"^^xsd:hexBinary"),
        typed("abcd", "base64Binary")
    );
    assert_eq!(
        cast("hexBinary", "\"0fb7\"^^xsd:hexBinary"),
        typed("0FB7", "hexBinary")
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
