// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Numeric facets compare against their bounds exactly as written.
//!
//! The AST keeps a facet bound as an `i64` or an `f64` (ShExJ's JSON numbers), so a
//! bound like `100000000000000000001` or `0.30000000000000001` loses its digits there.
//! [`ExactSchema`] keeps them, and [`validate_exact`] gives the verdict the bound as
//! written implies, for both schema syntaxes and through imports. Each case sits
//! beside a neighbour that the lossy bound happened to decide correctly.

use purrdf_core::TermValue;
use purrdf_rdf::parse_dataset;
use purrdf_shex::{
    ConformanceStatus, ExactSchema, ShapeSelector, ValidationOptions, validate, validate_exact,
};

const EX: &str = "http://example.org/";

/// The verdict for `ex:x`, whose `ex:n` is `value`, against `ex:S { ex:n <facet> }`.
fn verdict(schema: &ExactSchema, value: &str) -> ConformanceStatus {
    let data = parse_dataset(
        format!("<{EX}x> <{EX}n> {value} .").as_bytes(),
        "text/turtle",
        None,
    )
    .expect("data parses");
    let map = vec![(
        TermValue::Iri(format!("{EX}x")),
        ShapeSelector::Label(format!("{EX}S")),
    )];
    validate_exact(schema, &data, &map, &ValidationOptions::default()).entries[0].status
}

fn shexc(facet: &str) -> ExactSchema {
    ExactSchema::parse_shexc(&format!("<{EX}S> {{ <{EX}n> {facet} }}"), None)
        .expect("schema parses")
}

fn shexj(facet: &str, lexeme: &str) -> ExactSchema {
    let doc = format!(
        r#"{{"@context":"http://www.w3.org/ns/shex.jsonld","type":"Schema","shapes":[
            {{"type":"Shape","id":"{EX}S","expression":{{"type":"TripleConstraint",
            "predicate":"{EX}n","valueExpr":{{"type":"NodeConstraint","{facet}":{lexeme}}}}}}}]}}"#
    );
    ExactSchema::parse_shexj(&doc, None).expect("schema parses")
}

use ConformanceStatus::{Conformant, Nonconformant};

#[test]
fn integer_bounds_past_i64_are_exact() {
    let schema = shexc("MININCLUSIVE 100000000000000000001");
    assert_eq!(verdict(&schema, "100000000000000000000"), Nonconformant);
    assert_eq!(verdict(&schema, "100000000000000000001"), Conformant);
    let schema = shexj("mininclusive", "100000000000000000001");
    assert_eq!(verdict(&schema, "100000000000000000000"), Nonconformant);
    assert_eq!(verdict(&schema, "100000000000000000001"), Conformant);
}

#[test]
fn decimal_bounds_are_exact() {
    let schema = shexc("MINEXCLUSIVE 0.1");
    assert_eq!(verdict(&schema, "0.1000000000000000000000001"), Conformant);
    assert_eq!(verdict(&schema, "0.1"), Nonconformant);
    let schema = shexc("MAXEXCLUSIVE 0.30000000000000001");
    assert_eq!(verdict(&schema, "0.3"), Conformant);
    assert_eq!(verdict(&schema, "0.30000000000000001"), Nonconformant);
    let schema = shexj("maxexclusive", "0.30000000000000001");
    assert_eq!(verdict(&schema, "0.3"), Conformant);
    // A double bound compares under the numeric promotion, as SPARQL's `<` does.
    let schema = shexc("MAXINCLUSIVE 1.5E0");
    assert_eq!(verdict(&schema, "1.5"), Conformant);
    assert_eq!(verdict(&schema, "1.5000000000000000001"), Conformant);
    assert_eq!(verdict(&schema, "1.6"), Nonconformant);
}

/// The plain AST path keeps its documented `f64` behaviour; the exact path is the one
/// the CLI and the Python binding validate through.
#[test]
fn the_lossy_ast_path_is_unchanged() {
    let exact = shexc("MININCLUSIVE 100000000000000000001");
    let data = parse_dataset(
        format!("<{EX}x> <{EX}n> 100000000000000000000 .").as_bytes(),
        "text/turtle",
        None,
    )
    .expect("data parses");
    let map = vec![(
        TermValue::Iri(format!("{EX}x")),
        ShapeSelector::Label(format!("{EX}S")),
    )];
    assert_eq!(
        validate(exact.schema(), &data, &map).entries[0].status,
        Conformant
    );
    assert_eq!(verdict(&exact, "100000000000000000000"), Nonconformant);
    // A code-built schema's i64/f64 bounds are exact as the values they are.
    let built = ExactSchema::from_schema(exact.into_schema());
    assert_eq!(verdict(&built, "100000000000000000000"), Conformant);
}

#[test]
fn bounds_survive_imports_and_nesting() {
    let root = ExactSchema::parse_shexc(
        &format!("IMPORT <{EX}lib>\n<{EX}S> {{ <{EX}n> @<{EX}T> }}\nstart = @<{EX}S>"),
        None,
    )
    .expect("root parses");
    let merged = root
        .resolve_imports(&|iri| {
            assert_eq!(iri, format!("{EX}lib"));
            ExactSchema::parse_shexc(
                &format!(
                    "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n\
                     <{EX}T> xsd:decimal MINEXCLUSIVE 0.1 MAXEXCLUSIVE 0.30000000000000001"
                ),
                None,
            )
        })
        .expect("imports resolve");
    assert_eq!(verdict(&merged, "0.3"), Conformant);
    assert_eq!(verdict(&merged, "0.30000000000000001"), Nonconformant);
    assert_eq!(verdict(&merged, "0.1"), Nonconformant);
}
