// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! XSD constructor casts between the numeric types, and from them to `xsd:string`, from
//! query text (SPARQL 1.1 §17.5, which defers to the casting rules of XPath and XQuery
//! Functions and Operators 3.1 §19).
//!
//! A cast from a typed numeric or boolean literal converts the source's VALUE: the
//! float `0.1` is `0.100000001490116119384765625`, so `xsd:double` of it is that number,
//! not the double nearest the digits `0.1`. Only a string source is read lexically. A
//! numeric value cast to `xsd:string` is written by the cast-to-string rule: plain
//! decimal notation for magnitudes in `[0.000001, 1000000)`, scientific notation with a
//! mandatory digit after the point outside it, and a float at single precision.

use std::sync::Arc;

use purrdf_core::{RdfDataset, RdfDatasetBuilder, SparqlRequest, SparqlResult, TermValue};
use purrdf_sparql_eval::{NativeSparqlEngine, QueryOptions};
use purrdf_xsd::numeric::{canonical_double, canonical_float};

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

/// The one cell `SELECT (<expression> AS ?y) {}` binds, `None` when the expression is
/// an error and leaves `?y` unbound.
fn select(expression: &str) -> Option<(String, String)> {
    let dataset: Arc<RdfDataset> = RdfDatasetBuilder::new().freeze().expect("an empty dataset");
    let query = format!("PREFIX xsd: <{XSD}>\nSELECT ({expression} AS ?y) WHERE {{}}");
    let result = NativeSparqlEngine::new()
        .query_with_options_view(
            &*dataset,
            SparqlRequest {
                query: &query,
                base_iri: None,
                substitutions: &[],
            },
            QueryOptions::EMPTY,
        )
        .unwrap_or_else(|e| panic!("evaluate `{query}`: {e}"));
    let SparqlResult::Solutions { rows, .. } = result else {
        panic!("expected solutions");
    };
    assert_eq!(rows.len(), 1, "{expression}");
    match rows[0][0].as_ref() {
        None => None,
        Some(TermValue::Literal {
            lexical_form,
            datatype,
            ..
        }) => Some((lexical_form.clone(), datatype.clone())),
        Some(other) => panic!("{expression}: expected a literal, got {other:?}"),
    }
}

fn typed(lexical: &str, datatype: &str) -> Option<(String, String)> {
    Some((lexical.to_owned(), format!("{XSD}{datatype}")))
}

fn check(expression: &str, expected: Option<(String, String)>) {
    let cast = Cast {
        actual: select(expression),
        expected,
    };
    assert_eq!(cast.actual, cast.expected, "{expression}");
}

/// What a cast bound against what it should have bound.
struct Cast {
    actual: Option<(String, String)>,
    expected: Option<(String, String)>,
}

#[test]
fn a_float_cast_to_double_keeps_the_floats_value() {
    let widened = f64::from(0.1_f32);
    check(
        r#"xsd:double("0.1"^^xsd:float)"#,
        typed(&canonical_double(widened), "double"),
    );
    assert_eq!(canonical_double(widened), "1.0000000149011612E-1");
    // A float's integral value is its own: 16777217 is not a float.
    check(
        r#"xsd:double("16777217"^^xsd:float)"#,
        typed("1.6777216E7", "double"),
    );
    check(r#"xsd:double("-0"^^xsd:float)"#, typed("-0.0E0", "double"));
    check(r#"xsd:double("INF"^^xsd:float)"#, typed("INF", "double"));
    check(r#"xsd:double("NaN"^^xsd:float)"#, typed("NaN", "double"));
}

#[test]
fn a_double_cast_to_float_rounds_the_doubles_value_once() {
    // The double is exactly `1 + 2^-24`, an f32 halfway point, which rounds to the even
    // `1.0`; the decimal digits read as a float would round up to `1 + 2^-23` instead.
    check(
        r#"xsd:float("1.00000005960464477539062500001"^^xsd:double)"#,
        typed("1.0E0", "float"),
    );
    check(
        r#"xsd:float("0.1"^^xsd:double)"#,
        typed(&canonical_float(0.1), "float"),
    );
    check(r#"xsd:float("1e300"^^xsd:double)"#, typed("INF", "float"));
    check(
        r#"xsd:float("-1e-300"^^xsd:double)"#,
        typed("-0.0E0", "float"),
    );
}

/// The values that separate round-to-nearest from the mantissa truncation F&O 3.1
/// §19.1.2.1 words the narrowing as: PurRDF rounds, deliberately.
#[test]
fn a_double_cast_to_float_rounds_to_nearest_not_by_truncation() {
    // `1 + 1.5 × 2^-24` is above the halfway point between `1` and `1 + 2^-23`:
    // rounding gives `1 + 2^-23`, truncation would give `1`.
    check(
        r#"xsd:float("1.0000000894069671630859375"^^xsd:double)"#,
        typed("1.0000001E0", "float"),
    );
    // Exactly halfway between the largest float and 2^128: ties to even overflow to
    // INF, where truncation would give the largest float.
    check(
        r#"xsd:float("3.4028235677973366e38"^^xsd:double)"#,
        typed("INF", "float"),
    );
    check(
        r#"xsd:float("-3.4028235677973366e38"^^xsd:double)"#,
        typed("-INF", "float"),
    );
    // Just below the halfway point stays finite.
    check(
        r#"xsd:float("3.4028235677973362e38"^^xsd:double)"#,
        typed("3.4028235E38", "float"),
    );
    // The float subnormal band: the nearest subnormal, not a flush to zero.
    check(
        r#"xsd:float("1e-40"^^xsd:double)"#,
        typed("1.0E-40", "float"),
    );
    check(
        r#"xsd:float("1.401298464324817e-45"^^xsd:double)"#,
        typed(&canonical_float(f32::from_bits(1)), "float"),
    );
}

#[test]
fn exact_sources_cast_to_float_and_double_round_once() {
    check(
        r#"xsd:float("16777217"^^xsd:integer)"#,
        typed("1.6777216E7", "float"),
    );
    check(
        r#"xsd:double("9007199254740993"^^xsd:integer)"#,
        typed("9.007199254740992E15", "double"),
    );
    check(r#"xsd:float("0.1"^^xsd:decimal)"#, typed("1.0E-1", "float"));
    check(
        r#"xsd:double("0.1"^^xsd:decimal)"#,
        typed("1.0E-1", "double"),
    );
    check(r#"xsd:double("12"^^xsd:byte)"#, typed("1.2E1", "double"));
}

#[test]
fn a_float_or_double_cast_to_decimal_is_the_closest_decimal() {
    // The float 0.1 is 0.100000001490116119384765625; 18 fractional digits keep
    // 0.100000001490116119.
    check(
        r#"xsd:decimal("0.1"^^xsd:float)"#,
        typed("0.100000001490116119", "decimal"),
    );
    // The double 0.1 is 0.1000000000000000055511151231257827…, which rounds up.
    check(
        r#"xsd:decimal("0.1"^^xsd:double)"#,
        typed("0.100000000000000006", "decimal"),
    );
    // 3 × 2^-19 = 0.0000057220458984375 is a tie at the 19th digit: the one closer to
    // zero is chosen.
    check(
        r#"xsd:decimal("0.0000057220458984375"^^xsd:double)"#,
        typed("0.000005722045898437", "decimal"),
    );
    check(
        r#"xsd:decimal("-0.0000057220458984375"^^xsd:double)"#,
        typed("-0.000005722045898437", "decimal"),
    );
    check(
        r#"xsd:decimal("1e30"^^xsd:double)"#,
        typed("1000000000000000019884624838656", "decimal"),
    );
    check(r#"xsd:decimal("2.5"^^xsd:float)"#, typed("2.5", "decimal"));
    check(r#"xsd:decimal("1e-30"^^xsd:double)"#, typed("0", "decimal"));
    check(r#"xsd:decimal("-0"^^xsd:double)"#, typed("0", "decimal"));
    // No decimal value: an error, not a fabricated number.
    check(r#"xsd:decimal("NaN"^^xsd:double)"#, None);
    check(r#"xsd:decimal("INF"^^xsd:float)"#, None);
    check(r#"xsd:decimal("-INF"^^xsd:double)"#, None);
    check(r#"xsd:decimal("1e300"^^xsd:double)"#, None);
}

#[test]
fn a_float_or_double_cast_to_integer_truncates_its_value() {
    // 16777217 read as a float is 16777216.
    check(
        r#"xsd:integer("16777217"^^xsd:float)"#,
        typed("16777216", "integer"),
    );
    check(r#"xsd:integer("2.9"^^xsd:float)"#, typed("2", "integer"));
    check(r#"xsd:integer("-2.9"^^xsd:double)"#, typed("-2", "integer"));
    check(
        r#"xsd:integer("1e30"^^xsd:double)"#,
        typed("1000000000000000019884624838656", "integer"),
    );
    check(r#"xsd:byte("127.9"^^xsd:double)"#, typed("127", "byte"));
    check(r#"xsd:byte("128"^^xsd:double)"#, None);
    check(r#"xsd:integer("NaN"^^xsd:float)"#, None);
    check(r#"xsd:integer("INF"^^xsd:double)"#, None);
    check(r#"xsd:integer("1e40"^^xsd:double)"#, None);
    // The integer range ends at 2^127: just inside it on either side converts.
    check(
        r#"xsd:integer("1.7e38"^^xsd:double)"#,
        typed("169999999999999998061923293023115935744", "integer"),
    );
    check(
        r#"xsd:integer("-1.7014118346046923e38"^^xsd:double)"#,
        typed("-170141183460469231731687303715884105728", "integer"),
    );
    check(r#"xsd:integer("1.7014118346046923e38"^^xsd:double)"#, None);
}

#[test]
fn integer_and_decimal_sources_cast_to_decimal_exactly() {
    let max = i128::MAX.to_string();
    check(
        &format!(r#"xsd:decimal("{max}"^^xsd:integer)"#),
        typed(&max, "decimal"),
    );
    check(
        r#"xsd:decimal("12345678901234567.123456789012345678"^^xsd:decimal)"#,
        typed("12345678901234567.123456789012345678", "decimal"),
    );
    check(r#"xsd:decimal("+007"^^xsd:integer)"#, typed("7", "decimal"));
    check(
        r#"xsd:integer("12345678901234567.9"^^xsd:decimal)"#,
        typed("12345678901234567", "integer"),
    );
}

#[test]
fn booleans_and_numerics_cast_both_ways() {
    check("xsd:integer(true)", typed("1", "integer"));
    check("xsd:decimal(false)", typed("0", "decimal"));
    check("xsd:double(true)", typed("1.0E0", "double"));
    check("xsd:float(false)", typed("0.0E0", "float"));
    check(
        r#"xsd:boolean("0.0"^^xsd:float)"#,
        typed("false", "boolean"),
    );
    check(
        r#"xsd:boolean("NaN"^^xsd:double)"#,
        typed("false", "boolean"),
    );
    check(
        r#"xsd:boolean("-0.5"^^xsd:decimal)"#,
        typed("true", "boolean"),
    );
    check(r#"xsd:boolean("2"^^xsd:integer)"#, typed("true", "boolean"));
}

#[test]
fn a_string_source_is_still_read_lexically() {
    check(r#"xsd:double("0.1")"#, typed("1.0E-1", "double"));
    check(
        r#"xsd:float("0.1"^^xsd:string)"#,
        typed(&canonical_float(0.1), "float"),
    );
    check(
        r#"xsd:float("1.00000005960464477539062500001")"#,
        typed(&canonical_float(1.000_000_1), "float"),
    );
    check(r#"xsd:decimal("0.1")"#, typed("0.1", "decimal"));
    check(r#"xsd:integer("  7")"#, None);
    check(r#"xsd:integer("1.5")"#, None);
    check(r#"xsd:double("+INF")"#, None);
}

#[test]
fn a_double_cast_to_string_follows_the_cast_to_string_rule() {
    for (lexical, expected) in [
        ("0.1", "0.1"),
        ("1e7", "1.0E7"),
        ("12345678", "1.2345678E7"),
        ("1e6", "1.0E6"),
        ("999999", "999999"),
        ("123456.5", "123456.5"),
        ("1.5e-6", "0.0000015"),
        ("1e-7", "1.0E-7"),
        ("-1.25e20", "-1.25E20"),
        ("100", "100"),
        ("-2.50", "-2.5"),
        ("0", "0"),
        ("-0", "-0"),
        ("NaN", "NaN"),
        ("INF", "INF"),
        ("-INF", "-INF"),
    ] {
        check(
            &format!(r#"xsd:string("{lexical}"^^xsd:double)"#),
            typed(expected, "string"),
        );
    }
    // The double written `1e-6` is a little below one millionth, so it is outside the
    // plain-notation range; its successor is inside it.
    check(
        r#"xsd:string("1e-6"^^xsd:double)"#,
        typed("1.0E-6", "string"),
    );
    check(
        r#"xsd:string("0.0000010000000000000002"^^xsd:double)"#,
        typed("0.0000010000000000000002", "string"),
    );
}

#[test]
fn a_float_cast_to_string_is_written_at_single_precision() {
    for (lexical, expected) in [
        ("0.1", "0.1"),
        ("3.4028235E38", "3.4028235E38"),
        ("1e7", "1.0E7"),
        ("16777217", "1.6777216E7"),
        ("123456.7", "123456.7"),
        ("1e-10", "1.0E-10"),
        ("-0", "-0"),
        ("INF", "INF"),
    ] {
        check(
            &format!(r#"xsd:string("{lexical}"^^xsd:float)"#),
            typed(expected, "string"),
        );
    }
}

#[test]
fn integer_decimal_and_boolean_casts_to_string_are_canonical() {
    check(r#"xsd:string("+007"^^xsd:integer)"#, typed("7", "string"));
    check(r#"xsd:string("1.0"^^xsd:decimal)"#, typed("1", "string"));
    check(
        r#"xsd:string("-0.50"^^xsd:decimal)"#,
        typed("-0.5", "string"),
    );
    check(r#"xsd:string("1"^^xsd:boolean)"#, typed("true", "string"));
}

#[test]
fn str_of_a_numeric_literal_is_its_lexical_form_unchanged() {
    check(
        r#"STR("10000000.0"^^xsd:double)"#,
        Some(("10000000.0".to_owned(), format!("{XSD}string"))),
    );
    check(
        r#"STR("0.1"^^xsd:float)"#,
        Some(("0.1".to_owned(), format!("{XSD}string"))),
    );
    check(
        r#"STR("+007"^^xsd:integer)"#,
        Some(("+007".to_owned(), format!("{XSD}string"))),
    );
}
