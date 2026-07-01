// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property-based SHACL conformance monotonicity (#787, T6 of #781).
//!
//! # The property
//!
//! For the **monotone constraint fragment**, adding data triples (shapes fixed)
//! never removes an existing violation:
//!
//! ```text
//! result_tuples(validate(data))  ⊆  result_tuples(validate(data ∪ extra))
//! ```
//!
//! # Why only a fragment
//!
//! SHACL is **not** globally monotone under data addition. The fragment exercised
//! here is chosen so the ⊆ relation actually holds:
//!
//! * `sh:datatype`, `sh:pattern` — check an *intrinsic* property of a value that
//!   already exists; adding triples cannot un-make a bad value.
//! * `sh:maxCount` — an upper bound; adding values can only push further over.
//!
//! Deliberately **excluded** because they are *satisfiable by addition* (adding
//! data can remove a violation, breaking ⊆): `sh:minCount` (a new value can reach
//! the minimum) and `sh:class` (a new `rdf:type` triple can make a value an
//! instance). `sh:not`, `sh:maxCount`-dual and similar non-monotone components are
//! likewise out of scope.
//!
//! The validation entry point ([`validate_graphs`]) and the canonical result-set
//! comparator ([`ValidationReport::result_tuples`]) are re-used, not re-minted.

use proptest::prelude::*;
use purrdf_shapes::engine::validate_graphs;

/// Shapes over the monotone fragment only: a value-typed property with a datatype
/// constraint, a lexical pattern, and an upper-bound cardinality.
const SHAPES_TTL: &str = r#"
@prefix sh:  <http://www.w3.org/ns/shacl#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix ex:  <https://example.org/> .

ex:ThingShape a sh:NodeShape ;
    sh:targetClass ex:Thing ;
    sh:property [
        sh:path ex:val ;
        sh:datatype xsd:integer ;
        sh:pattern "^[0-9]+$" ;
        sh:maxCount 2 ;
    ] .
"#;

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const EX: &str = "https://example.org/";

/// A single generated data assertion, rendered as one N-Triples line.
#[derive(Clone, Debug)]
enum Fact {
    /// `ex:s{n}` is an `ex:Thing` (becomes a validation target).
    Typed(u8),
    /// `ex:s{n} ex:val "<int>"^^xsd:integer` — satisfies datatype + pattern.
    IntValue(u8, i32),
    /// `ex:s{n} ex:val "<text>"` — a plain string: violates datatype + pattern.
    StrValue(u8, String),
}

fn fact_to_nt(fact: &Fact) -> String {
    match fact {
        Fact::Typed(s) => {
            format!("<{EX}s{s}> <{RDF_TYPE}> <{EX}Thing> .")
        }
        Fact::IntValue(s, n) => {
            format!("<{EX}s{s}> <{EX}val> \"{n}\"^^<http://www.w3.org/2001/XMLSchema#integer> .")
        }
        Fact::StrValue(s, text) => {
            format!("<{EX}s{s}> <{EX}val> \"{text}\" .")
        }
    }
}

fn to_ntriples(facts: &[Fact]) -> String {
    facts.iter().map(fact_to_nt).collect::<Vec<_>>().join("\n")
}

fn arb_fact() -> impl Strategy<Value = Fact> {
    prop_oneof![
        (0u8..4).prop_map(Fact::Typed),
        // Non-negative only: the shape's `sh:pattern "^[0-9]+$"` rejects a leading
        // minus sign, so a negative would unintentionally violate the pattern and
        // contradict this generator's "satisfies datatype + pattern" intent.
        (0u8..4, 0..i32::MAX).prop_map(|(s, n)| Fact::IntValue(s, n)),
        // Lowercase letters only: no N-Triples escaping needed, and never a valid
        // xsd:integer lexical form (always a datatype + pattern violation).
        (0u8..4, "[a-z]{1,4}").prop_map(|(s, t)| Fact::StrValue(s, t)),
    ]
}

fn config() -> ProptestConfig {
    let cases = std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(64);
    ProptestConfig {
        cases,
        failure_persistence: None,
        ..ProptestConfig::default()
    }
}

/// Non-vacuity guard: the shapes must actually fire, otherwise the monotonicity
/// property would pass trivially on always-empty reports.
#[test]
fn shapes_detect_violations() {
    let data = format!("<{EX}s0> <{RDF_TYPE}> <{EX}Thing> .\n<{EX}s0> <{EX}val> \"oops\" .");
    let report = validate_graphs(&data, SHAPES_TTL).expect("validate");
    assert!(
        !report.result_tuples().is_empty(),
        "a string value under sh:datatype xsd:integer must produce a violation",
    );
}

proptest! {
    #![proptest_config(config())]

    /// Adding data never removes a violation in the monotone fragment.
    #[test]
    fn conformance_is_monotone_under_data_addition(
        base in prop::collection::vec(arb_fact(), 0..8),
        extra in prop::collection::vec(arb_fact(), 0..8),
    ) {
        let base_nt = to_ntriples(&base);
        let mut combined = base;
        combined.extend(extra);
        let combined_nt = to_ntriples(&combined);

        let base_report = validate_graphs(&base_nt, SHAPES_TTL)
            .expect("base validation should not error");
        let combined_report = validate_graphs(&combined_nt, SHAPES_TTL)
            .expect("combined validation should not error");

        let base_tuples = base_report.result_tuples();
        let combined_tuples = combined_report.result_tuples();

        prop_assert!(
            base_tuples.is_subset(&combined_tuples),
            "monotonicity violated — a base violation vanished after adding data:\n\
             base    = {:#?}\n\
             combined = {:#?}",
            base_tuples,
            combined_tuples,
        );
    }
}
