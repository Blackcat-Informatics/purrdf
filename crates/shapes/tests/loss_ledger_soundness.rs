// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
use purrdf_shapes::json_schema::{CompiledSchema, Namespaces, compile};
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
    let dataset = parse_turtle_to_dataset(&ttl).expect("Turtle parse");
    let shapes = from_dataset(&dataset).expect("shape parse");
    compile(&shapes, &fixture_ns())
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
