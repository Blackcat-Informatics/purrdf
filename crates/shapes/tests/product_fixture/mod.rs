// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! **The one shapes graph the prepared-product determinism proofs compile
//! against, on every target.**
//!
//! This module is the PREIMAGE of the committed golden artifacts under
//! `tests/fixtures/`. It is shared rather than copied because the wasm proof's
//! whole claim is that the bytes it produces equal the bytes the native run
//! produced: two transcriptions of "the same" fixture that drifted by a
//! whitespace character would fail that comparison while the codec was perfectly
//! target-independent, and the failure would look exactly like the divergence the
//! test exists to catch.
//!
//! It lives in a subdirectory because Cargo compiles every top-level `tests/*.rs`
//! file as its own test binary; a sibling `product_fixture.rs` would become a
//! third, empty test target.
//!
//! # Why the fixture looks like that
//!
//! The determinism claim that matters is not "encoding twice in a row agrees with
//! itself" — a serializer that walked a hash map in whatever order that map
//! happened to have would pass that trivially, because it is the same map both
//! times. The claim is that encoding is a pure function of the *shapes graph*,
//! independent of how the process that parsed it laid its tables out in memory.
//! That divergence only becomes observable when the fixture actually populates
//! the map-shaped state a parse builds, so this one deliberately carries:
//!
//! * **several shapes**, so the shape index has something to order;
//! * **several prefixes**, so the document prefix map does;
//! * **four distinct target kinds** (`sh:targetClass`, `sh:targetNode`,
//!   `sh:targetSubjectsOf`, `sh:targetObjectsOf`), so the target tables do;
//! * **a box-role vocabulary**, so the caller-supplied configuration travels too.
//!
//! Everything here is under `example.org`: PurRDF mints no vocabulary IRIs, and
//! the box-role vocabulary in particular is caller-supplied configuration with no
//! fabricated default.
//!
//! Nothing in this module touches the filesystem, a clock or a source of
//! randomness, so it compiles and runs unchanged on `wasm32-unknown-unknown`. The
//! golden artifacts are reached with `include_bytes!`, which resolves at compile
//! time on every target.

#![allow(
    dead_code,
    reason = "one shared fixture serves two test binaries; each uses the part of it \
              that its own claim needs, and splitting the module per consumer would \
              reintroduce the transcription drift it exists to prevent"
)]

use std::sync::Arc;

use purrdf::RdfDataset;
use purrdf_shapes::engine::{PreparedShapes, parse_shapes_with_config};
use purrdf_shapes::model::BoxRoleVocab;
use purrdf_shapes::product::{HostBindings, ShapesProfile};
use purrdf_shapes::report::ValidationReport;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

/// The namespace the caller-supplied box-role vocabulary is derived from.
///
/// Configuration, not an ontology PurRDF owns: the six role IRIs are whatever the
/// caller says they are, and this fixture says `example.org`.
pub(crate) const ROLE_NS: &str = "http://example.org/roles#";

/// The shapes graph the golden artifacts were prepared from.
///
/// **Frozen alongside the artifacts.** Editing this text changes the product the
/// fixture encodes to, which is exactly what the golden comparisons are watching
/// for; a change here is a change to the preimage of a committed byte string and
/// has to be made as deliberately as a change to the bytes themselves.
pub(crate) const SHAPES: &str = r"
@prefix sh:    <http://www.w3.org/ns/shacl#> .
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
@prefix ex:    <http://example.org/ns#> .
@prefix org:   <http://example.org/org#> .
@prefix place: <http://example.org/place#> .
@prefix roles: <http://example.org/roles#> .

ex:PersonShape a sh:NodeShape ;
    roles:graphBoxRole roles:boxTBox ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ; sh:datatype xsd:string ] ;
    sh:property [ sh:path ex:age  ; sh:maxCount 1 ; sh:datatype xsd:integer ] .

ex:OrgShape a sh:NodeShape ;
    roles:graphBoxRole roles:boxABox ;
    sh:targetNode org:acme, org:globex ;
    sh:property [ sh:path org:label ; sh:minCount 1 ] .

ex:PlaceShape a sh:NodeShape ;
    sh:targetSubjectsOf place:locatedIn ;
    sh:property [ sh:path place:name ; sh:minCount 1 ] .

ex:AcquaintanceShape a sh:NodeShape ;
    sh:targetObjectsOf ex:knows ;
    sh:nodeKind sh:IRI .
";

/// The data graph the restore proofs validate against.
///
/// Every shape in [`SHAPES`] has both a conforming and a violating focus node
/// here, so a restore that silently lost a shape shows up as a report that got
/// *shorter* rather than as a report that never had anything to say.
pub(crate) const DATA: &str = r#"
@prefix xsd:   <http://www.w3.org/2001/XMLSchema#> .
@prefix ex:    <http://example.org/ns#> .
@prefix org:   <http://example.org/org#> .
@prefix place: <http://example.org/place#> .

ex:alice a ex:Person ; ex:name "Alice" ; ex:age "34"^^xsd:integer ; ex:knows ex:bob .
ex:bob   a ex:Person ; ex:knows "not an iri" .

org:acme   org:label "Acme" .
org:globex a ex:Organization .

place:harbour place:locatedIn place:city ; place:name "Harbour" .
place:quay    place:locatedIn place:city .
"#;

/// The prepared product this release writes for [`SHAPES`], committed as a golden.
///
/// Reached with `include_bytes!` so the comparison works identically on a target
/// with no filesystem.
pub(crate) const GOLDEN: &[u8] = include_bytes!("../fixtures/prepared-shapes-core.product");

/// Parse [`SHAPES`] into a fresh shapes graph.
///
/// A function rather than a cached value on purpose: the stability proof needs two
/// *independent* parses, each with its own tables.
pub(crate) fn prepared() -> PreparedShapes {
    let vocab = BoxRoleVocab::for_namespace(ROLE_NS);
    let shapes =
        parse_shapes_with_config(SHAPES, None, Some(vocab)).expect("the fixture shapes parse");
    PreparedShapes::new(Arc::new(shapes))
}

/// Encode [`SHAPES`] as a prepared product under the only profile this build
/// implements.
pub(crate) fn encode() -> Vec<u8> {
    prepared()
        .to_product(&ShapesProfile::CORE)
        .expect("the fixture is representable as a product")
}

/// Freeze [`DATA`] into a data graph.
pub(crate) fn data() -> Arc<RdfDataset> {
    parse_turtle_to_dataset(DATA, None).expect("the fixture data parses")
}

/// Validate [`DATA`] with `prepared` and render the report as canonical
/// N-Triples.
///
/// The report's RDF form is the comparison surface rather than a field-by-field
/// walk: two restores agree exactly when the graphs they produce are the same
/// bytes, and a restore that dropped a constraint shows up as a missing result
/// rather than as a field nobody thought to compare.
pub(crate) fn report_nt(prepared: &PreparedShapes) -> String {
    report(prepared).to_ntriples()
}

/// Validate [`DATA`] with `prepared`.
pub(crate) fn report(prepared: &PreparedShapes) -> ValidationReport {
    prepared
        .bind_shared_dataset(data())
        .expect("the data graph binds")
        .validate()
        .expect("validation runs")
}

/// The report a freshly parsed [`SHAPES`] produces over [`DATA`] — the answer
/// every restore path must reproduce.
///
/// Asserted non-vacuous by [`assert_non_vacuous`]: a comparison against an empty
/// report would pass for a restore that lost every shape.
pub(crate) fn expected_report_nt() -> String {
    let expected = report_nt(&prepared());
    assert_non_vacuous(&expected);
    expected
}

/// Refuse a report that could not distinguish a working restore from a broken one.
///
/// Each of the four shapes must have found its violating focus node. Without this
/// the equality assertions below are satisfied by two identically empty reports,
/// which is precisely the shape a restore that silently dropped the whole shapes
/// graph would produce.
pub(crate) fn assert_non_vacuous(report_nt: &str) {
    for focus in [
        "http://example.org/ns#bob",
        "http://example.org/org#globex",
        "http://example.org/place#quay",
    ] {
        assert!(
            report_nt.contains(focus),
            "the fixture report must name {focus}, or every equality assertion over it is \
             vacuous: {report_nt}"
        );
    }
    assert!(
        report_nt.contains("not an iri"),
        "the fixture report must carry the node-kind violation, or the object-target shape \
         contributes nothing: {report_nt}"
    );
}

/// The host bindings every product of [`ShapesProfile::CORE`] restores under.
pub(crate) fn host() -> HostBindings<'static> {
    HostBindings::empty()
}
