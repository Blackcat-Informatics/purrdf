// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Keeps the root `README.md` Rust quickstart honest: this test is the same
//! code, verbatim. If it stops compiling or passing, fix the README too.

use purrdf::{RdfDatasetBuilder, RdfLiteral, SerializeGraph, parse_dataset, serialize_dataset};

#[test]
fn readme_quickstart_round_trips() {
    // Build a dataset in interned TermId space.
    let mut b = RdfDatasetBuilder::new();
    let alice = b.intern_iri("https://example.org/alice");
    let knows = b.intern_iri("http://xmlns.com/foaf/0.1/knows");
    let bob = b.intern_iri("https://example.org/bob");
    let name = b.intern_iri("http://xmlns.com/foaf/0.1/name");
    let hi = b.intern_literal(RdfLiteral::simple("Alice"));
    b.push_quad(alice, knows, bob, None);
    b.push_quad(alice, name, hi, None);
    let ds = b.freeze().expect("freeze");

    // Serialize to any native codec and parse back, losslessly.
    let ttl = serialize_dataset(&ds, "text/turtle", SerializeGraph::Dataset).unwrap();
    let back = parse_dataset(&ttl, "text/turtle", None).unwrap();
    assert_eq!(back.quad_count(), 2);
}
