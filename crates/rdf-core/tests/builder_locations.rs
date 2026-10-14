// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source locations stay associated with deduplicated builder rows.

use purrdf_core::{BlankScope, RdfDatasetBuilder, RdfLocation, RdfQuad, RdfTerm};

fn assert_owned_duplicate_location(append_another_row: bool) {
    let mut builder = RdfDatasetBuilder::new();
    let quad = RdfQuad::new(
        RdfTerm::iri("https://example.org/located"),
        "https://example.org/p",
        RdfTerm::iri("https://example.org/value"),
    );
    let location = RdfLocation::file("duplicate.ttl").with_line(7);
    builder.push_owned_quad(&quad).unwrap();
    builder
        .push_owned_quad_scoped(&quad.with_location(location.clone()), BlankScope::DEFAULT)
        .unwrap();
    if append_another_row {
        builder
            .push_owned_quad(&RdfQuad::new(
                RdfTerm::iri("https://example.org/next"),
                "https://example.org/p",
                RdfTerm::iri("https://example.org/value"),
            ))
            .unwrap();
    }
    let graph = builder.freeze().expect("valid graph");
    assert_eq!(graph.quad_count(), if append_another_row { 2 } else { 1 });
    if append_another_row {
        let next = graph
            .owned_quads()
            .find(|quad| quad.subject == RdfTerm::iri("https://example.org/next"))
            .expect("the next row exists");
        assert_eq!(next.location, None, "a duplicate cannot locate another row");
    }
    let located = graph
        .owned_quads()
        .find(|quad| quad.subject == RdfTerm::iri("https://example.org/located"))
        .expect("the located row exists");
    assert_eq!(located.location, Some(location));
}

#[test]
fn owned_duplicate_location_survives_at_end() {
    assert_owned_duplicate_location(false);
}

#[test]
fn owned_duplicate_location_does_not_move_to_the_next_row() {
    assert_owned_duplicate_location(true);
}

#[test]
fn unrealized_location_handle_cannot_alias_a_later_row() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri("https://example.org/first").unwrap();
    let p = builder.intern_iri("https://example.org/p").unwrap();
    let o = builder.intern_iri("https://example.org/value").unwrap();
    builder.push_quad(s, p, o, None).unwrap();
    let stale = builder.next_quad_handle().unwrap();
    builder.push_quad(s, p, o, None).unwrap();
    builder.attach_location(stale, RdfLocation::file("duplicate.ttl").with_line(7));
    let next = builder.intern_iri("https://example.org/next").unwrap();
    builder.push_quad(next, p, o, None).unwrap();
    let graph = builder.freeze().expect("valid graph");
    assert_eq!(graph.quad_count(), 2);
    assert!(graph.owned_quads().all(|quad| quad.location.is_none()));
}

#[test]
fn returned_handles_follow_deduplicated_graph_qualified_rows() {
    let mut builder = RdfDatasetBuilder::new();
    let s = builder.intern_iri("https://example.org/s").unwrap();
    let p = builder.intern_iri("https://example.org/p").unwrap();
    let o = builder.intern_iri("https://example.org/value").unwrap();
    let g = builder.intern_iri("https://example.org/g").unwrap();
    let first = builder.push_quad_with_handle(s, p, o, None).unwrap();
    let duplicate = builder.push_quad_with_handle(s, p, o, None).unwrap();
    let named = builder.push_quad_with_handle(s, p, o, Some(g)).unwrap();
    assert_eq!(first, duplicate);
    assert_ne!(first, named);
    let default_location = RdfLocation::file("default.ttl").with_line(3);
    let named_location = RdfLocation::file("named.ttl").with_line(5);
    builder.attach_location(duplicate, default_location.clone());
    builder.attach_location(named, named_location.clone());
    let graph = builder.freeze().expect("valid graph");
    assert_eq!(graph.quad_count(), 2);
    for quad in graph.owned_quads() {
        let expected = if quad.graph_name.is_some() {
            &named_location
        } else {
            &default_location
        };
        assert_eq!(quad.location.as_ref(), Some(expected));
    }
}
