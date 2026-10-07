// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SHACL Core §4.3 range constraints compare numbers with the SPARQL operators, and
//! an `xsd:integer`/`xsd:decimal` value of any size compares exactly: never through a
//! double, and never refused for its length. A literal whose lexical form its own
//! datatype rejects has no value to compare, so it violates the constraint — beside
//! the well-formed neighbour of the same value, which conforms.

use purrdf_shapes::engine::{parse_shapes, validate_dataset_with_shapes_graph};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = "@prefix ex: <http://example.org/ns#> .\n\
    @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n";

/// Whether `ex:x ex:p {object}` conforms to `sh:{constraint} {bound}`.
fn conforms(constraint: &str, bound: &str, object: &str) -> bool {
    let shapes = format!(
        "{PREFIXES}ex:S a sh:NodeShape ; sh:targetNode ex:x ;\n\
         sh:property [ sh:path ex:p ; sh:{constraint} {bound} ] .\n"
    );
    let shapes = parse_shapes(&shapes, None).expect("shapes parse");
    let data = parse_turtle_to_dataset(&format!("{PREFIXES}ex:x ex:p {object} .\n"), None)
        .expect("data parses");
    let report = validate_dataset_with_shapes_graph(&data, &shapes, None).expect("validates");
    report.results.is_empty()
}

fn integer(lexical: &str) -> String {
    format!("\"{lexical}\"^^xsd:integer")
}

fn decimal(lexical: &str) -> String {
    format!("\"{lexical}\"^^xsd:decimal")
}

/// Integers one apart past the double's precision, past `i64` and past `i128` are
/// told apart, in every direction of every range constraint.
#[test]
fn integer_ranges_are_exact_at_any_size() {
    for (low, high) in [
        ("9007199254740992", "9007199254740993"),
        (
            "170141183460469231731687303715884105727",
            "170141183460469231731687303715884105728",
        ),
        (
            &format!("1{}", "0".repeat(60)) as &str,
            &format!("1{}1", "0".repeat(59)) as &str,
        ),
    ] {
        let (low, high) = (integer(low), integer(high));
        assert!(!conforms("minInclusive", &high, &low), "{low} < {high}");
        assert!(conforms("minInclusive", &low, &high), "{high} >= {low}");
        assert!(conforms("minInclusive", &low, &low));
        assert!(!conforms("minExclusive", &low, &low));
        assert!(conforms("minExclusive", &low, &high));
        assert!(!conforms("maxInclusive", &low, &high));
        assert!(conforms("maxInclusive", &high, &low));
        assert!(!conforms("maxExclusive", &high, &high));
        assert!(conforms("maxExclusive", &high, &low));
    }
}

/// A decimal past eighteen fractional digits compares by every digit, against an
/// integer or decimal bound.
#[test]
fn decimal_ranges_are_exact_past_eighteen_digits() {
    let just_above = decimal("5.0000000000000000000000000001");
    assert!(conforms("minExclusive", &integer("5"), &just_above));
    assert!(!conforms("maxInclusive", &integer("5"), &just_above));
    assert!(conforms(
        "maxInclusive",
        &decimal("5.0000000000000000000000000001"),
        &just_above
    ));
    assert!(!conforms(
        "maxExclusive",
        &decimal("5.0000000000000000000000000001"),
        &just_above
    ));
    // Trailing zeros are spelling: the value is five, inside both bounds.
    let five = decimal(&format!("5.{}", "0".repeat(40)));
    assert!(conforms("maxInclusive", &integer("5"), &five));
    assert!(conforms("minInclusive", &integer("5"), &five));
}

/// A literal its datatype rejects cannot be compared, so it violates every range
/// constraint; the well-formed literal of the same value beside it conforms.
#[test]
fn an_ill_typed_numeric_literal_violates_beside_its_well_formed_neighbour() {
    for (ill, well) in [
        (integer("1.5"), decimal("1.5")),
        (integer("12x"), integer("12")),
        (decimal("1.2.3"), decimal("1.23")),
        (
            "\"300\"^^xsd:byte".to_owned(),
            "\"127\"^^xsd:byte".to_owned(),
        ),
    ] {
        for (constraint, bound) in [("minInclusive", "0"), ("maxInclusive", "1000")] {
            assert!(
                !conforms(constraint, bound, &ill),
                "{ill} against sh:{constraint} {bound}"
            );
            assert!(
                conforms(constraint, bound, &well),
                "{well} against sh:{constraint} {bound}"
            );
        }
    }
}
