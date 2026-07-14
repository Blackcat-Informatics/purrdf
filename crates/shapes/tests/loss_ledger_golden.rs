// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Golden drift gate for the public **runtime** loss-ledger schema
//! ([`purrdf::loss::LossLedger::render_json`]) over a REAL
//! [`purrdf_shapes::json_schema::compile`] output — never a hand-built ledger.
//!
//! `generated/rdf-loss-matrix.json` / `generated/transcode-loss-matrix.json`
//! (drift-gated in `crates/rdf-core/src/loss.rs`) pin the **contract**
//! (bare-array) shape; this file pins the **runtime** (`{ "schema_version": 1,
//! "losses": [...] }`) shape the shapes emitter (and future `--loss-ledger`
//! consumers, see the crate root docs) actually produce, so downstream
//! consumers can rely on a stable, versioned envelope.
//!
//! Regenerate the committed artifact with:
//!
//! ```text
//! cargo run -p purrdf-shapes --example gen_shapes_loss_ledger > generated/shapes-loss-ledger.json
//! ```
//!
//! `GOLDEN_SHAPES` below is duplicated (not shared code) in
//! `crates/shapes/examples/gen_shapes_loss_ledger.rs` — the regen path — so
//! keep the two copies in lock-step; a drift between them is still caught
//! (the regenerated file would then fail this test's own drift check).

use purrdf::loss::assert_ledger_sound;
use purrdf_shapes::json_schema::{CompiledSchema, Namespaces, compile};
use purrdf_shapes::shapes::from_dataset;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const PREFIXES: &str = r"
    @prefix sh:  <http://www.w3.org/ns/shacl#> .
    @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    @prefix ex:  <https://example.org/> .
";

/// A FIXED, `example.org` SHACL document exercising a representative slice of
/// the `("shacl", "json-schema")` loss profile through the real emitter:
/// node- and property-level `sh:sparql`, a node-level `sh:not` over a
/// non-expressible inner, a property-level (value-position) `sh:not`, a
/// node-level `sh:expression`, and a shape targeted only via SHACL-AF
/// `sh:SPARQLTarget` (`sh:SPARQLTarget`).
const GOLDEN_SHAPES: &str = r#"
    ex:GuardedShape a sh:NodeShape ;
        sh:targetClass ex:Guarded ;
        sh:sparql [
            sh:select "SELECT $this WHERE { $this a <https://example.org/Guarded> . }" ;
        ] ;
        sh:expression true ;
        sh:not [ sh:nodeKind sh:Literal ] ;
        sh:property [
            sh:path ex:tag ;
            sh:sparql [
                sh:select "SELECT $this ?value WHERE { $this <https://example.org/tag> ?value . }" ;
            ] ;
        ] ;
        sh:property [
            sh:path ex:label ;
            sh:not [ sh:datatype xsd:integer ] ;
        ] .

    ex:SparqlTargetedShape a sh:NodeShape ;
        sh:target [
            a sh:SPARQLTarget ;
            sh:select "SELECT ?this WHERE { ?this a <https://example.org/Guarded> . }" ;
        ] ;
        sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ] .
"#;

/// The fixture namespace table: `ex:` (`https://example.org/`) is the primary
/// prefix, exactly as a downstream caller would declare its own document
/// prefixes.
fn golden_ns() -> Namespaces {
    Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )
    .expect("golden fixture namespace is valid")
}

/// Compile [`GOLDEN_SHAPES`] through the real production entry point.
fn compile_golden() -> CompiledSchema {
    let ttl = format!("{PREFIXES}{GOLDEN_SHAPES}");
    let dataset = parse_turtle_to_dataset(&ttl).expect("golden fixture Turtle parses");
    let shapes = from_dataset(&dataset).expect("golden fixture shape parse");
    compile(&shapes, &golden_ns())
}

/// Drift gate: the committed `generated/shapes-loss-ledger.json` must
/// byte-equal a fresh [`CompiledSchema::losses`] `render_json()` render of the
/// real emitter output over [`GOLDEN_SHAPES`].
#[test]
fn shapes_loss_ledger_has_not_drifted() {
    let compiled = compile_golden();
    assert!(
        !compiled.losses.is_empty(),
        "the golden fixture must actually record real losses for this gate to mean anything"
    );
    let rendered = compiled.losses.render_json();

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../generated/shapes-loss-ledger.json");
    let on_disk =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert_eq!(
        on_disk, rendered,
        "generated/shapes-loss-ledger.json is stale; regenerate via `cargo run -p \
         purrdf-shapes --example gen_shapes_loss_ledger > generated/shapes-loss-ledger.json`"
    );
}

/// The runtime ledger's public schema is pinned at `schema_version: 1` — the
/// stable envelope a future `--loss-ledger` flag (and any other downstream
/// consumer) can rely on.
#[test]
fn shapes_loss_ledger_schema_version_is_pinned() {
    let compiled = compile_golden();
    let rendered = compiled.losses.render_json();
    assert!(
        rendered.starts_with("{\n  \"schema_version\": 1,\n  \"losses\": ["),
        "runtime ledger envelope must open with a pinned schema_version: 1, got: {rendered}"
    );
    assert!(rendered.ends_with("]\n}\n"), "got: {rendered}");
}

/// Every code the real emitter records over [`GOLDEN_SHAPES`] must be inside
/// the declared `("shacl", "json-schema")` profile — soundness over a real,
/// multi-code compile (not just the single-shape fixture in
/// `loss_ledger_soundness.rs`).
#[test]
fn golden_shapes_ledger_is_sound() {
    let compiled = compile_golden();
    assert_ledger_sound(&compiled.losses, "shacl", "json-schema");
}
