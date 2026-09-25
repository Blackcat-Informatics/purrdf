// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Production-surface red/green demo for the reusable sound+complete loss
//! verification helpers (`purrdf::loss::{check_ledger_complete,
//! assert_ledger_complete, check_ledger_sound, assert_ledger_sound}`).
//!
//! `purrdf_shapes::json_schema::compile` is the one live runtime producer of a
//! [`purrdf::loss::LossLedger`] (`CompiledSchema::losses`): every test here
//! drives that REAL entry point over `example.org` SHACL fixtures — never a
//! hand-built ledger — and observes the helpers against its actual output.
//!
//! - `complete_*`: the correct expected-code set for a real lossy compile
//!   PASSES [`assert_ledger_complete`] (green); a deliberately incomplete set
//!   FAILS (red), both via `#[should_panic]` and via the `Result`-returning
//!   core.
//! - `sound_green_for_real_compile_output`: every code the real compile path
//!   records for `("shacl", "json-schema")` is inside the declared
//!   `profile_for` contract (the soundness RED case — a hand-built
//!   out-of-profile code — lives in `crates/rdf-core/src/loss.rs`'s own unit
//!   tests, since no real shapes emit path produces one).
//! - `lossless_shape_compiles_with_empty_ledger`: a shape with no
//!   unrepresentable construct compiles to an empty ledger.

use purrdf::loss::{assert_ledger_complete, assert_ledger_sound, check_ledger_complete};
use purrdf_shapes::json_schema::{CompiledSchema, Namespaces, SchemaCompileError, compile};
use purrdf_shapes::shapes::from_dataset;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = r"
    @prefix sh:  <http://www.w3.org/ns/shacl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex:  <https://example.org/> .
";

/// The fixture namespace table: `ex:` (`https://example.org/`) is the primary
/// prefix, exactly as a downstream caller would declare its own document
/// prefixes.
fn fixture_ns() -> Namespaces {
    Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )
    .expect("fixture namespace is valid")
}

/// Compile the given SHACL Turtle body (prefixed with [`PREFIXES`]) through
/// the real production entry point (`purrdf_shapes::json_schema::compile`),
/// exactly as a downstream consumer would call it.
fn compile_ttl(body: &str) -> CompiledSchema {
    let ttl = format!("{PREFIXES}{body}");
    let dataset = parse_turtle_to_dataset(&ttl, None).expect("Turtle parse");
    let shapes = from_dataset(&dataset).expect("shape parse");
    compile(&shapes, &fixture_ns()).expect("schema compilation")
}

/// A shape carrying a `sh:sparql` constraint (no JSON Schema equivalent) and a
/// second shape carrying a `sh:not` over a non-expressible inner (`sh:nodeKind
/// sh:Literal`) — two distinct, real loss codes recorded by one compile.
const LOSSY_SHAPES: &str = r#"
    ex:GuardedShape a sh:NodeShape ;
        sh:targetClass ex:Guarded ;
        sh:sparql [
            sh:select "SELECT $this WHERE { $this a <https://example.org/Guarded> . }" ;
        ] .

    ex:ThingShape a sh:NodeShape ;
        sh:targetClass ex:Thing ;
        sh:not [ sh:nodeKind sh:Literal ] .
"#;

#[test]
fn complete_green_with_the_correct_expected_codes() {
    let compiled = compile_ttl(LOSSY_SHAPES);
    // Green: both codes this compile actually records are declared expected.
    assert_ledger_complete(&compiled.losses, &["sh:sparql", "sh:not"]);
}

#[test]
#[should_panic(expected = "loss ledger incomplete")]
fn complete_red_when_a_real_recorded_code_is_omitted() {
    let compiled = compile_ttl(LOSSY_SHAPES);
    // Red: `sh:not` is a real code this compile DID record, but the caller's
    // expected set omits it — a silent loss must be flagged, not waved
    // through.
    assert_ledger_complete(&compiled.losses, &["sh:sparql"]);
}

#[test]
fn complete_red_via_result_core_names_the_missing_code() {
    let compiled = compile_ttl(LOSSY_SHAPES);
    let err = check_ledger_complete(
        &compiled.losses,
        &["sh:sparql", "sh:not", "sh:SPARQLTarget"],
    )
    .expect_err("sh:SPARQLTarget was never recorded by this compile");
    assert!(
        err.contains("sh:SPARQLTarget"),
        "error must name the missing code: {err}"
    );
}

#[test]
fn sound_green_for_real_compile_output() {
    let compiled = compile_ttl(LOSSY_SHAPES);
    assert!(
        !compiled.losses.is_empty(),
        "fixture must actually record losses for this check to mean anything"
    );
    // Every code the real emitter recorded is inside the declared shapes
    // profile — nothing surprising reached the ledger.
    assert_ledger_sound(&compiled.losses, "shacl", "json-schema");
}

/// The SHACL 1.2 constraints the value-schema projection drops — the list
/// components, `sh:rootClass`, `sh:singleLine true`, a property-level
/// `sh:someValue` and a `sh:TripleTerm` node kind — each record their own code,
/// and every one of those codes is inside the declared profile.
///
/// The neighbours record nothing: `sh:singleLine false` checks nothing, and a
/// node-level `sh:someValue` is projected as `sh:node` is.
#[test]
fn shacl12_constraints_record_declared_codes() {
    let compiled = compile_ttl(
        r"
        ex:ListShape a sh:NodeShape ;
            sh:targetClass ex:Listed ;
            sh:property [ sh:path ex:items ; sh:minListLength 1 ; sh:maxListLength 3 ;
                          sh:uniqueMembers true ; sh:memberShape [ sh:nodeKind sh:IRI ] ] ;
            sh:property [ sh:path ex:kind ; sh:rootClass ex:Root ] ;
            sh:property [ sh:path ex:label ; sh:singleLine true ] ;
            sh:property [ sh:path ex:ref ; sh:someValue [ sh:nodeKind sh:IRI ] ] ;
            sh:property [ sh:path ex:quoted ; sh:nodeKind ( sh:IRI sh:TripleTerm ) ] .
        ",
    );
    let mut codes = recorded_codes(&compiled);
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec![
            "sh:maxListLength",
            "sh:memberShape",
            "sh:minListLength",
            "sh:nodeKind",
            "sh:rootClass",
            "sh:singleLine",
            "sh:someValue",
            "sh:uniqueMembers",
        ]
    );
    assert_ledger_sound(&compiled.losses, "shacl", "json-schema");

    let neighbours = compile_ttl(
        r"
        ex:QuietShape a sh:NodeShape ;
            sh:targetClass ex:Quiet ;
            sh:someValue [ sh:property [ sh:path ex:name ; sh:minCount 1 ] ] ;
            sh:property [ sh:path ex:label ; sh:singleLine false ] .
        ",
    );
    assert!(
        neighbours.losses.is_empty(),
        "{:?}",
        recorded_codes(&neighbours)
    );
    assert!(
        neighbours.schema_json.contains("\"required\""),
        "the node-level sh:someValue shape is projected: {}",
        neighbours.schema_json
    );
}

/// Every property pair — `sh:equals`, `sh:disjoint`, `sh:subsetOf`,
/// `sh:lessThan`, `sh:lessThanOrEquals`, over one IRI or any other path — records
/// its own declared code, on a property shape and on a node shape, and the
/// `$comment` names what was dropped.
#[test]
fn path_valued_pairs_and_subset_of_record_declared_codes() {
    let compiled = compile_ttl(
        r"
        ex:PairShape a sh:NodeShape ;
            sh:targetClass ex:Paired ;
            sh:equals ( ex:self ex:self ) ;
            sh:subsetOf ex:self ;
            sh:property [ sh:path ex:a ; sh:equals [ sh:inversePath ex:b ] ;
                          sh:disjoint ( ex:c ex:d ) ; sh:subsetOf ex:e ] ;
            sh:property [ sh:path ex:start ; sh:lessThan ( ex:next ex:start ) ;
                          sh:lessThanOrEquals [ sh:zeroOrOnePath ex:stop ] ] .
        ",
    );
    let mut codes = recorded_codes(&compiled);
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec![
            "sh:disjoint",
            "sh:equals",
            "sh:equals",
            "sh:lessThan",
            "sh:lessThanOrEquals",
            "sh:subsetOf",
            "sh:subsetOf",
        ]
    );
    assert_ledger_sound(&compiled.losses, "shacl", "json-schema");
    assert!(
        compiled
            .schema_json
            .contains("a sh:subsetOf <https://example.org/e> constraint on property"),
        "{}",
        compiled.schema_json
    );
}

/// The IRI form of each pair is no more expressible than a path form: JSON Schema
/// cannot relate one property's values to another's. Each records exactly its
/// own code against its shape and leaves a `$comment`, rather than vanishing from
/// the schema; the same shapes without the pairs record nothing.
#[test]
fn iri_valued_pairs_record_their_declared_codes() {
    let with_pairs = compile_ttl(
        r"
        ex:PairShape a sh:NodeShape ;
            sh:targetClass ex:Paired ;
            sh:property [ sh:path ex:a ; sh:minCount 1 ; sh:equals ex:b ; sh:disjoint ex:c ] ;
            sh:property [ sh:path ex:start ; sh:datatype xsd:integer ;
                          sh:lessThan ex:end ; sh:lessThanOrEquals ex:stop ] .
        ",
    );
    let mut codes = recorded_codes(&with_pairs);
    codes.sort_unstable();
    assert_eq!(
        codes,
        vec![
            "sh:disjoint",
            "sh:equals",
            "sh:lessThan",
            "sh:lessThanOrEquals"
        ]
    );
    assert_eq!(
        with_pairs
            .losses
            .render_json()
            .matches("https://example.org/PairShape")
            .count(),
        4,
        "each pair's loss is recorded against its shape: {}",
        with_pairs.losses.render_json()
    );
    assert_ledger_sound(&with_pairs.losses, "shacl", "json-schema");
    assert!(
        with_pairs
            .schema_json
            .contains("a sh:equals <https://example.org/b> constraint on property"),
        "{}",
        with_pairs.schema_json
    );
    let without_pairs = compile_ttl(
        r"
        ex:PairShape a sh:NodeShape ;
            sh:targetClass ex:Paired ;
            sh:property [ sh:path ex:a ; sh:minCount 1 ] ;
            sh:property [ sh:path ex:start ; sh:datatype xsd:integer ] .
        ",
    );
    assert!(
        without_pairs.losses.is_empty(),
        "{:?}",
        recorded_codes(&without_pairs)
    );
}

#[test]
fn lossless_shape_compiles_with_empty_ledger() {
    let compiled = compile_ttl(
        r"
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ] .
        ",
    );
    assert!(
        compiled.losses.is_empty(),
        "a shape with no unrepresentable construct must compile losslessly, got {:?}",
        compiled.losses
    );
    // Soundness holds vacuously over an empty ledger too.
    assert_ledger_sound(&compiled.losses, "shacl", "json-schema");
}

/// Every loss code a real compile records, as `&str`, for assertions.
fn recorded_codes(compiled: &CompiledSchema) -> Vec<&str> {
    compiled
        .losses
        .entries()
        .iter()
        .map(|entry| entry.code.as_ref())
        .collect()
}

/// Unicode categories are expanded faithfully and no longer create a loss.
#[test]
fn category_pattern_translates_without_a_dialect_loss() {
    let compiled = compile_ttl(
        r#"
        ex:CategoryPatternShape a sh:NodeShape ;
            sh:targetClass ex:CategoryPattern ;
            sh:property [ sh:path ex:code ; sh:pattern "^\\p{L}+$" ] .
        "#,
    );
    assert_eq!(recorded_codes(&compiled), [] as [&str; 0]);
    assert!(!compiled.schema_json.contains("\\\\p{L}"));
    assert_ledger_sound(&compiled.losses, "shacl", "json-schema");
}

/// An unsupported source fails the complete emission, rather than installing
/// a pattern which the validator cannot enforce.
#[test]
fn rejected_pattern_is_a_typed_emission_failure() {
    let dataset = parse_turtle_to_dataset(
        &format!(
            "{PREFIXES}{}",
            r#"
        ex:RejectedPatternShape a sh:NodeShape ;
            sh:targetClass ex:RejectedPattern ;
            sh:property [ sh:path ex:code ; sh:pattern "(a)\\1" ] .
    "#
        ),
        None,
    )
    .expect("fixture parse");
    let shapes = from_dataset(&dataset).expect("shape parse");
    assert!(matches!(
        compile(&shapes, &fixture_ns()),
        Err(SchemaCompileError::Pattern { .. })
    ));
}
