// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The shapes-graph IRI a host names — Python `shapes_graph=`, WebAssembly `shapesGraph`,
//! C `shapes_graph_iri`, the command line's `--shapes-graph` — reaches every entry point on
//! this boundary, graded on the W3C SHACL 1.0 test `sparql/pre-binding/shapesGraph-001`.
//!
//! That test's constraint selects its focus node only when `$shapesGraph` is bound and
//! `GRAPH $shapesGraph { $currentShape ex:property 42 }` reads the shapes graph. SHACL 1.0
//! pre-binds `$shapesGraph` and approves ONE result; SHACL 1.2 removed the pre-binding, so
//! with no IRI named `$shapesGraph` is an ordinary, unbound variable, `FILTER
//! bound($shapesGraph)` removes every solution and the data graph CONFORMS. Every entry
//! point is asked both questions, so the IRI is observed rather than assumed: with it, the
//! approved one result; without it, conforms.

use std::path::PathBuf;
use std::sync::Arc;

use purrdf_rdf::{SerializeGraph, parse_dataset, serialize_dataset};
use purrdf_shapes::ShapesImports;
use purrdf_shapes::engine::{self, PreparedShapes};
use purrdf_validate::{
    SarifOptions, ShapesError, lint_shapes_ttl_with_shapes_graph,
    pack_shapes_product_with_shapes_graph, validate_changes_to_sarif_string_with_shapes_graph,
    validate_to_sarif_string_with_shapes_graph, validate_with_shapes_product,
};

/// The vendored test file: both its shapes graph and its data graph.
fn fixture() -> (String, String, String) {
    let path = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/shacl/sparql/pre-binding/shapesGraph-001.ttl"
    ))
    .canonicalize()
    .expect("the vendored W3C test file");
    let base = format!("file://{}", path.display());
    let turtle = std::fs::read_to_string(&path).expect("the test file reads");
    let dataset =
        parse_dataset(turtle.as_bytes(), "text/turtle", Some(&base)).expect("the test parses");
    let bytes = serialize_dataset(&*dataset, "application/n-quads", SerializeGraph::Dataset)
        .expect("the data graph serializes");
    let data_nt = String::from_utf8(bytes).expect("N-Quads is UTF-8");
    (turtle, base, data_nt)
}

const FOCUS: &str =
    "http://datashapes.org/sh/tests/sparql/pre-binding/shapesGraph-001.test#InvalidResource";

/// `(conforms, results)` of a SARIF log, each result as its JSON text.
fn verdict(sarif: &str) -> (bool, Vec<String>) {
    let log: serde_json::Value = serde_json::from_str(sarif).expect("SARIF is JSON");
    let run = &log["runs"][0];
    let conforms = run["properties"]["shaclConforms"]
        .as_bool()
        .expect("the run states its verdict");
    // An empty `results` array is omitted from the log.
    let results = run["results"]
        .as_array()
        .into_iter()
        .flatten()
        .map(ToString::to_string)
        .collect();
    (conforms, results)
}

#[test]
fn shapes_graph_001_through_the_text_boundary() {
    let (shapes, base, data) = fixture();
    let options = SarifOptions::default();
    let run = |graph: Option<&str>, shapes_base: Option<&str>| {
        validate_to_sarif_string_with_shapes_graph(
            &shapes,
            shapes_base,
            graph,
            &data,
            &options,
            &[],
        )
    };

    // Named: the approved SHACL 1.0 result, exactly one, on ex:InvalidResource.
    let (conforms, focus) = verdict(&run(Some(&base), Some(&base)).expect("validates"));
    assert!(!conforms);
    assert_eq!(focus.len(), 1, "{focus:?}");
    assert!(focus[0].contains(FOCUS), "{focus:?}");
    // Unnamed: SHACL 1.2's ordinary variable — unbound, so the data graph conforms.
    let (conforms, focus) = verdict(&run(None, Some(&base)).expect("validates"));
    assert!(conforms);
    assert!(focus.is_empty(), "{focus:?}");
    // A relative IRI names what `sh:shapesGraph <shapesGraph-001.ttl>` in the document
    // would: resolved against the shapes document's base.
    let (conforms, _) =
        verdict(&run(Some("shapesGraph-001.ttl"), Some(&base)).expect("resolves against the base"));
    assert!(!conforms);
    // Any IRI names a graph; the constraint reads whichever one it is bound to.
    let (conforms, _) =
        verdict(&run(Some("https://example.org/shapes"), Some(&base)).expect("validates"));
    assert!(!conforms);
}

/// A relative IRI with no base in scope names no graph: refused by its diagnostic code,
/// beside the absolute neighbour that validates with no base at all.
#[test]
fn a_relative_shapes_graph_with_no_base_is_refused() {
    const SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:S a sh:NodeShape ; sh:targetNode ex:n ; ex:property 42 ;\n\
          sh:sparql [ sh:select \"SELECT $this WHERE { FILTER bound($shapesGraph) }\" ] .\n";
    let options = SarifOptions::default();
    let refused =
        validate_to_sarif_string_with_shapes_graph(SHAPES, None, Some("shapes"), "", &options, &[])
            .expect_err("no base to resolve `shapes` against");
    assert!(
        matches!(&refused, ShapesError::Invalid(message) if message.contains("iri-relative-no-base")),
        "{refused}"
    );
    let (conforms, focus) = verdict(
        &validate_to_sarif_string_with_shapes_graph(
            SHAPES,
            None,
            Some("http://example.org/shapes"),
            "",
            &options,
            &[],
        )
        .expect("an absolute IRI needs no base"),
    );
    assert!(!conforms);
    assert_eq!(focus.len(), 1);
    // The lint and pack entry points refuse the same value by the same code.
    for error in [
        lint_shapes_ttl_with_shapes_graph(SHAPES, None, Some("shapes"), &[])
            .expect_err("lint")
            .to_string(),
        match pack_shapes_product_with_shapes_graph(SHAPES, None, Some("shapes"), &[]) {
            Err(refusal) => refusal.to_string(),
            Ok(_) => panic!("pack must refuse a relative shapes graph with no base"),
        },
    ] {
        assert!(error.contains("iri-relative-no-base"), "{error}");
    }
    assert!(
        lint_shapes_ttl_with_shapes_graph(SHAPES, None, Some("http://example.org/shapes"), &[])
            .is_ok()
    );
}

#[test]
fn shapes_graph_001_through_the_change_path() {
    let (shapes, base, data) = fixture();
    let options = SarifOptions::default();
    for (graph, expected) in [(Some(base.as_str()), 1usize), (None, 0)] {
        let (sarif, _) = validate_changes_to_sarif_string_with_shapes_graph(
            &shapes,
            Some(&base),
            graph,
            "",
            Some(&data),
            None,
            &options,
            &[],
        )
        .expect("the change validates");
        let (conforms, focus) = verdict(&sarif);
        assert_eq!(focus.len(), expected, "{graph:?}: {focus:?}");
        assert_eq!(conforms, expected == 0);
    }
}

/// A `Shapes` parsed under the IRI carries it through every engine door — the plain
/// `validate_dataset`, a `PreparedShapes` binding, and a prepared product, which records it
/// in its identity — and one parsed without it exposes nothing through the same doors.
#[test]
fn shapes_graph_001_through_parsed_prepared_and_packed_shapes() {
    let (shapes_ttl, base, data_nt) = fixture();
    let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(&data_nt)
        .expect("the data graph parses");
    for (graph, expected) in [(Some(base.as_str()), 1usize), (None, 0)] {
        let shapes = engine::parse_shapes_with_graph(
            &shapes_ttl,
            Some(&base),
            None,
            graph,
            &ShapesImports::new(),
        )
        .expect("the shapes graph parses");
        assert_eq!(shapes.shapes_graph.as_deref(), graph);
        let direct = engine::validate_dataset(data.as_ref(), &shapes).expect("validates");
        assert_eq!(
            direct.results.len(),
            expected,
            "validate_dataset, {graph:?}"
        );

        let prepared = PreparedShapes::new(Arc::new(shapes));
        let projected = engine::project_dataset(data.as_ref()).expect("projects");
        for bound in [
            prepared
                .bind_projected_dataset(Arc::clone(&projected))
                .expect("binds"),
            prepared
                .bind_shared_dataset(Arc::clone(&projected))
                .expect("binds"),
        ] {
            assert_eq!(
                bound.validate().expect("validates").results.len(),
                expected,
                "PreparedShapes, {graph:?}"
            );
        }

        let product = pack_shapes_product_with_shapes_graph(&shapes_ttl, Some(&base), graph, &[])
            .expect("packs");
        let (conforms, focus) = verdict(
            &validate_with_shapes_product(&product, &data_nt, &SarifOptions::default())
                .expect("the product validates"),
        );
        assert_eq!(focus.len(), expected, "product, {graph:?}");
        assert_eq!(conforms, expected == 0);
    }
}
