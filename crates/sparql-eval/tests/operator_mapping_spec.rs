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

fn dataset() -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri(&format!("{EX}s"));
    let p = builder.intern_iri(&format!("{EX}p"));
    let one = builder.intern_literal(RdfLiteral {
        lexical_form: "1".to_owned(),
        datatype: Some(format!("{XSD}integer")),
        language: None,
        direction: None,
    });
    builder.push_quad(s, p, one, None);
    builder.freeze().expect("a dataset")
}

fn evaluate(query: &str) -> Result<SparqlResult, purrdf_core::RdfDiagnostic> {
    let dataset = dataset();
    NativeSparqlEngine::new().query_with_options_view(
        &*dataset,
        SparqlRequest {
            query: &format!("PREFIX xsd: <{XSD}> {query}"),
            base_iri: None,
            substitutions: &[],
        },
        QueryOptions::EMPTY,
    )
}

fn rows(query: &str) -> Vec<Vec<Option<TermValue>>> {
    match evaluate(query).unwrap_or_else(|e| panic!("evaluate `{query}`: {e:?}")) {
        SparqlResult::Solutions { rows, .. } => rows,
        other => panic!("expected solutions for `{query}`, got {other:?}"),
    }
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
    // The neighbours: an ordinary identical term still orders against itself, and
    // `sameValue`'s NaN carve-out keeps `=` between two NaNs true.
    assert_eq!(boolean("1 <= 1"), Some(true));
    assert_eq!(boolean("2.5 >= 2.5"), Some(true));
    assert_eq!(boolean(&format!("{NAN_DOUBLE} = {NAN_DOUBLE}")), Some(true));
    assert_eq!(boolean(&format!("{NAN_DOUBLE} = {NAN_FLOAT}")), Some(true));
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
