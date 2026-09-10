// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `Shapes` exposes the frozen dataset it already retains, without reparsing.

use std::sync::Arc;

use purrdf_shapes::engine::parse_shapes;
use purrdf_shapes::shapes::{Shapes, from_dataset};
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;

const SHAPES_TTL: &str = r"
@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .

ex:PersonShape
    a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [
        sh:path ex:name ;
        sh:minCount 1 ;
    ] .
";

/// The accessor hands back the very allocation it was given — no copy, no reparse.
///
/// A deep copy would be invisible to any behavioural assertion: the shapes would
/// validate identically and only the memory cost would differ. Pointer identity
/// is the only thing that actually distinguishes sharing from copying, which is
/// why this is asserted rather than inferred from equal contents.
#[test]
fn the_accessor_returns_the_same_allocation_it_was_built_from() {
    let dataset = parse_turtle_to_dataset(SHAPES_TTL, None).expect("shapes parse");
    let shapes = from_dataset(&dataset).expect("shapes build");

    assert!(
        Arc::ptr_eq(shapes.dataset(), &dataset),
        "the retained dataset must be the caller's, not a clone of it"
    );

    // A caller wanting independent retention pays one refcount bump, explicitly.
    let retained = Arc::clone(shapes.dataset());
    let before = Arc::strong_count(&dataset);
    drop(shapes);
    assert!(
        Arc::ptr_eq(&retained, &dataset),
        "the dataset outlives the Shapes that exposed it"
    );
    assert!(before > 1, "the clone held a reference while Shapes lived");
}

/// The text entry point retains the dataset it parsed, so no consumer needs to
/// reparse the source merely to recover it.
#[test]
fn the_text_entry_point_retains_its_parsed_dataset() {
    let shapes = parse_shapes(SHAPES_TTL, None).expect("shapes parse");
    let retained = shapes.dataset();

    let independently = parse_turtle_to_dataset(SHAPES_TTL, None).expect("second parse");
    assert!(
        !Arc::ptr_eq(retained, &independently),
        "a second parse is a different allocation; the point is not to need one"
    );
    assert_eq!(
        retained.quad_count(),
        independently.quad_count(),
        "the retained dataset is the same content a reparse would produce"
    );
}

/// Exposing the dataset does not disturb how the document's prefixes resolve.
///
/// `parse_shapes` reads prefixes from the source text separately from the RDF it
/// interns, so a change to what it retains could plausibly shift shape identity.
/// The shapes parsed with the accessor present must still name the same targets.
#[test]
fn exposing_the_dataset_leaves_prefix_resolution_alone() {
    let from_text = parse_shapes(SHAPES_TTL, None).expect("shapes from text");

    let dataset = parse_turtle_to_dataset(SHAPES_TTL, None).expect("dataset");
    let from_graph = from_dataset(&dataset).expect("shapes from dataset");

    let names = |shapes: &Shapes| {
        let mut ids: Vec<String> = shapes
            .node_shapes
            .iter()
            .map(|shape| shape.id.to_string())
            .collect();
        ids.sort();
        ids
    };

    assert_eq!(names(&from_text), names(&from_graph));
    assert!(
        names(&from_text)
            .iter()
            .any(|id| id.contains("http://example.org/PersonShape")),
        "the prefixed shape name still resolves against the document base: {:?}",
        names(&from_text)
    );
}
