// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Regenerates the golden runtime loss-ledger artifact
//! (`generated/shapes-loss-ledger.json`) drift-gated by
//! `crates/shapes/tests/loss_ledger_golden.rs`.
//!
//! Run via `cargo run -p purrdf-shapes --example gen_shapes_loss_ledger >
//! generated/shapes-loss-ledger.json`.
//!
//! `GOLDEN_SHAPES` below is a duplicate of the copy in
//! `crates/shapes/tests/loss_ledger_golden.rs` (integration tests cannot share
//! code with an example binary); keep the two in lock-step — a divergence is
//! still caught by that test's own drift check against the file this binary
//! writes.

use purrdf_shapes::json_schema::{Namespaces, compile};
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

fn main() {
    let ns = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )
    .expect("golden fixture namespace is valid");
    let ttl = format!("{PREFIXES}{GOLDEN_SHAPES}");
    let dataset = parse_turtle_to_dataset(&ttl).expect("golden fixture Turtle parses");
    let shapes = from_dataset(&dataset).expect("golden fixture shape parse");
    let compiled = compile(&shapes, &ns);
    print!("{}", compiled.losses.render_json());
}
