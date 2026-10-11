// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual packed and native Turtle/TriG output, on the default stack and wasm.

#[path = "support/turtle_chains.rs"]
mod inputs;

use purrdf_core::{BlankScope, RdfDatasetBuilder, render_canonical_turtle};
use purrdf_iri::vocab::rdf;
use purrdf_rdf::{SerializeGraph, parse_dataset, serialize_dataset};

fn prefixes() -> Vec<(String, String)> {
    vec![
        ("ex".into(), "http://example.org/".into()),
        ("rdf".into(), rdf::NS.into()),
    ]
}

fn receipt(label: &str, bytes: &[u8]) {
    let expected = match label {
        "packed/100000" => (
            33_094_118,
            "938071ed811f977dad0bb575dae44701c657a74fc12dea4af80518613201208b",
        ),
        "turtle/100000" => (
            4_878_216,
            "5ab357ea2c8db6c837529ec198b8b37359d7d4ad8f1ecfba721877b05dc6f630",
        ),
        "trig/100000" => (
            5_078_161,
            "f48e53d4b763e58c34c191f7d717e12abc9c5869468a8897c8e3d2f28f38085e",
        ),
        "packed/1000000" => (
            330_994_118,
            "8dcbafdf63c599dcc6bc781de652d70c6f7dae9cb35127dafb28a1b1a203f72a",
        ),
        "turtle/1000000" => (
            50_778_216,
            "fb165514a50a577964997db5bc344863d25928f8fe3ec232efa55ea3d7eb7384",
        ),
        "trig/1000000" => (
            52_778_161,
            "8ac01ee650c82b0054aa54fc03e75250346bf3728a7180e0198964bc5e373094",
        ),
        _ => panic!("unregistered output receipt: {label}"),
    };
    let digest = purrdf_hash::hex::encode(purrdf_hash::blake3::hash(bytes).as_bytes());
    assert_eq!((bytes.len(), digest.as_str()), expected, "{label}");
    purrdf_testkit::harness::print_line(&format!("{label}: {} {}", bytes.len(), digest));
}

fn deep(depth: usize) {
    {
        let dataset = inputs::chain(depth, false, false);
        let packed = render_canonical_turtle(&dataset, &prefixes());
        assert_eq!(packed.matches("ex:p ").count(), depth + 2);
        assert_eq!(packed.matches("[\n").count(), depth);
        assert_eq!(packed.matches(']').count(), depth);
        assert!(packed.contains("ex:note ex:end"));
        assert!(packed.contains("rdf:reifies <<( ex:end ex:p ex:end )>>"));
        assert!(
            packed
                .lines()
                .all(|line| line.bytes().take_while(|&b| b == b' ').count() <= 160)
        );
        receipt(&format!("packed/{depth}"), packed.as_bytes());
        drop(packed);
        let bytes =
            serialize_dataset(&*dataset, "text/turtle", SerializeGraph::DefaultGraph).unwrap();
        receipt(&format!("turtle/{depth}"), &bytes);
        let reparsed = parse_dataset(&bytes, "text/turtle", None).unwrap();
        assert_eq!(reparsed.quads().count(), depth + 1);
        assert_eq!(reparsed.reifier_quads().count(), 1);
        assert_eq!(reparsed.annotation_quads().count(), 1);
        assert_eq!(
            serialize_dataset(&*reparsed, "text/turtle", SerializeGraph::DefaultGraph).unwrap(),
            bytes
        );
    }
    {
        let dataset = inputs::chain(depth, true, false);
        let bytes =
            serialize_dataset(&*dataset, "application/trig", SerializeGraph::Dataset).unwrap();
        receipt(&format!("trig/{depth}"), &bytes);
        let reparsed = parse_dataset(&bytes, "application/trig", None).unwrap();
        assert_eq!(reparsed.quads().count(), depth + 1);
        assert_eq!(reparsed.named_graphs().count(), 1);
        let graph = reparsed.named_graphs().next().unwrap();
        assert!(matches!(
            reparsed.resolve(graph),
            purrdf_core::TermRef::Iri("http://example.org/graph")
        ));
        assert!(reparsed.quads().all(|q| q.g == Some(graph)));
        assert!(reparsed.reifier_quads().all(|q| q.g == Some(graph)));
        assert!(reparsed.annotation_quads().all(|q| q.g == Some(graph)));
        assert_eq!(reparsed.reifier_quads().count(), 1);
        assert_eq!(reparsed.annotation_quads().count(), 1);
        assert_eq!(
            serialize_dataset(&*reparsed, "application/trig", SerializeGraph::Dataset).unwrap(),
            bytes
        );
    }
}

fn blank_chains_at_one_hundred_thousand() {
    deep(100_000);
}
fn blank_chains_at_one_million() {
    deep(1_000_000);
}

fn exact_guard_neighbors_and_permuted_interning() {
    for depth in [0, 1, 2, 31, 32, 33, 39, 40, 41] {
        let dataset = inputs::chain(depth, false, false);
        let actual = render_canonical_turtle(&dataset, &prefixes());
        let mut expected = format!(
            "@prefix rdf: <{}> .\n@prefix ex: <http://example.org/> .\n\nex:reifier\n    rdf:reifies <<( ex:end ex:p ex:end )>> ;\n    ex:note ex:end .\n\nex:root\n",
            rdf::NS
        );
        for level in 1..=depth {
            expected.push_str(&"    ".repeat(level.min(40)));
            expected.push_str("ex:p [\n");
        }
        expected.push_str(&"    ".repeat((depth + 1).min(40)));
        expected.push_str("ex:p ex:end");
        for level in (1..=depth).rev() {
            expected.push_str(" ;\n");
            expected.push_str(&"    ".repeat(level.min(40)));
            expected.push(']');
        }
        expected.push_str(" .\n");
        // The old layout adds four spaces per level. The deliberate whitespace
        // change starts only beyond level forty; every statement still renders.
        assert_eq!(actual, expected, "depth {depth}");
        let permuted = inputs::chain(depth, false, true);
        assert_eq!(render_canonical_turtle(&permuted, &prefixes()), actual);
        for (mime, selection) in [
            ("text/turtle", SerializeGraph::DefaultGraph),
            ("application/trig", SerializeGraph::Dataset),
        ] {
            assert_eq!(
                serialize_dataset(&*dataset, mime, selection).unwrap(),
                serialize_dataset(&*permuted, mime, selection).unwrap()
            );
        }
        let named = inputs::chain(depth, true, false);
        let named_permuted = inputs::chain(depth, true, true);
        assert_eq!(
            serialize_dataset(&*named, "application/trig", SerializeGraph::Dataset).unwrap(),
            serialize_dataset(
                &*named_permuted,
                "application/trig",
                SerializeGraph::Dataset
            )
            .unwrap()
        );
    }
}

fn cycles_shared_and_quoted_blanks_preserve_statements() {
    for source in [
        "_:a <http://example.org/p> _:b . _:b <http://example.org/p> _:a .",
        "_:a <http://example.org/p> _:a .",
        "<http://example.org/a> <http://example.org/p> _:b . <http://example.org/c> <http://example.org/p> _:b . _:b <http://example.org/p> <http://example.org/end> .",
        "<http://example.org/a> <http://example.org/p> _:b . <http://example.org/a> <http://example.org/q> <<( _:b <http://example.org/p> <http://example.org/end> )>> . _:b <http://example.org/p> <http://example.org/end> .",
        "<http://example.org/a> <http://example.org/p> ( <http://example.org/x> <http://example.org/y> ) .",
    ] {
        let original = parse_dataset(source.as_bytes(), "text/turtle", None).unwrap();
        let packed = render_canonical_turtle(&original, &prefixes());
        let reparsed = parse_dataset(packed.as_bytes(), "text/turtle", None).unwrap();
        assert_eq!(
            original.quads().count(),
            reparsed.quads().count(),
            "{packed}"
        );
        assert_eq!(render_canonical_turtle(&reparsed, &prefixes()), packed);
    }
}

fn tied_guarded_branches_keep_order_under_permutation() {
    let build = |reverse: bool| {
        let mut builder = RdfDatasetBuilder::new();
        let root = builder.intern_iri("http://example.org/root");
        let predicate = builder.intern_iri("http://example.org/p");
        for branch in if reverse { ["z", "a"] } else { ["a", "z"] } {
            let mut next = builder.intern_iri(&format!("http://example.org/end{branch}"));
            for level in 0..100 {
                let cell = builder.intern_blank(&format!("{branch}{level}"), BlankScope::DEFAULT);
                builder.push_quad(cell, predicate, next, None);
                next = cell;
            }
            builder.push_quad(root, predicate, next, None);
        }
        builder.freeze().unwrap()
    };
    let a = render_canonical_turtle(&build(false), &prefixes());
    let b = render_canonical_turtle(&build(true), &prefixes());
    assert_eq!(a, b);
    // The root's two objects share one predicate spelling.
    assert_eq!(a.matches("ex:p ").count(), 201);
    assert!(a.contains("ex:enda") && a.contains("ex:endz"));
    assert!(
        a.lines()
            .all(|line| line.bytes().take_while(|&b| b == b' ').count() <= 160)
    );
}

fn a_long_collection_keeps_every_member() {
    let mut builder = RdfDatasetBuilder::new();
    let root = builder.intern_iri("http://example.org/root");
    let predicate = builder.intern_iri("http://example.org/p");
    let first = builder.intern_iri(rdf::FIRST);
    let rest = builder.intern_iri(rdf::REST);
    let nil = builder.intern_iri(rdf::NIL);
    let member = builder.intern_iri("http://example.org/member");
    let mut next = nil;
    for level in 0..100_000 {
        let cell = builder.intern_blank(&format!("cell{level}"), BlankScope::DEFAULT);
        builder.push_quad(cell, first, member, None);
        builder.push_quad(cell, rest, next, None);
        next = cell;
    }
    builder.push_quad(root, predicate, next, None);
    let dataset = builder.freeze().unwrap();
    let packed = render_canonical_turtle(&dataset, &prefixes());
    assert_eq!(packed.matches("ex:member").count(), 100_000);
    assert!(packed.contains("ex:p ( "));
    let reparsed = parse_dataset(packed.as_bytes(), "text/turtle", None).unwrap();
    assert_eq!(reparsed.quads().count(), 200_001);
}

fn continuation_objects_saturate_at_the_same_indent_guard() {
    let mut builder = RdfDatasetBuilder::new();
    let root = builder.intern_iri("http://example.org/root");
    let predicate = builder.intern_iri("http://example.org/p");
    let a = builder.intern_iri("http://example.org/a");
    let z = builder.intern_iri("http://example.org/z");
    let mut next = builder.intern_blank("bottom", BlankScope::DEFAULT);
    builder.push_quad(next, predicate, a, None);
    builder.push_quad(next, predicate, z, None);
    for level in 1..39 {
        let cell = builder.intern_blank(&format!("cell{level}"), BlankScope::DEFAULT);
        builder.push_quad(cell, predicate, next, None);
        next = cell;
    }
    builder.push_quad(root, predicate, next, None);
    let packed = render_canonical_turtle(&builder.freeze().unwrap(), &prefixes());
    assert!(packed.contains(&format!(
        "{}ex:p ex:a ,\n{}ex:z ;\n",
        " ".repeat(160),
        " ".repeat(160)
    )));
    assert_eq!(packed.matches("ex:p ").count(), 40);
}

purrdf_testkit::harness_main!(
    blank_chains_at_one_hundred_thousand,
    blank_chains_at_one_million,
    exact_guard_neighbors_and_permuted_interning,
    cycles_shared_and_quoted_blanks_preserve_statements,
    tied_guarded_branches_keep_order_under_permutation,
    a_long_collection_keeps_every_member,
    continuation_objects_saturate_at_the_same_indent_guard,
);
