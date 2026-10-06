// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen preimages specified independently from the versioned byte grammar.

#[path = "../../../../tests/support/dataset_state_fixtures.rs"]
mod fixtures;

use std::sync::Arc;

use super::*;
use crate::{DatasetView, RdfDataset, RdfDatasetBuilder, RdfLiteral};

const VECTORS: &str = include_str!("../../../../tests/goldens/dataset-state-v1.txt");

fn fixture(name: &str) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    let q = builder.intern_iri("http://example.org/q");
    let g = builder.intern_iri("http://example.org/g");
    let h = builder.intern_iri("http://example.org/h");
    let s = builder.intern_blank("authored", BlankScope(19));
    let quoted = builder.intern_triple(s, p, h);
    match name {
        "empty_default" => {}
        "empty_iri_graph" => builder.declare_named_graph(g),
        "empty_blank_graph" => builder.declare_named_graph(s),
        "ordinary" => builder.push_quad(s, q, h, None),
        "reifier" => builder.push_reifier(s, quoted),
        "annotation" => builder.push_annotation(s, q, h),
        "ordinary_annotation" => {
            builder.push_quad(s, q, h, None);
            builder.push_annotation(s, q, h);
        }
        "nested_shared_graph" => {
            let nested = builder.intern_triple(s, p, quoted);
            builder.push_quad(s, q, nested, Some(s));
        }
        "list_shared_graph" | "map_shared_graph" | "embedded_list_shared_graph" => {
            let (lexical, datatype) = match name {
                "list_shared_graph" => (
                    "[ _:authored, [_:authored] ]".to_owned(),
                    purrdf_cdt::CDT_LIST,
                ),
                "map_shared_graph" => (
                    "{ _:authored: [_:authored] }".to_owned(),
                    purrdf_cdt::CDT_MAP,
                ),
                _ => (
                    format!("[\"[_:authored]\"^^<{}>, _:authored]", purrdf_cdt::CDT_LIST),
                    purrdf_cdt::CDT_LIST,
                ),
            };
            // The authored lexical labels bind to the one document scope.
            let s = builder.intern_blank("authored", BlankScope::DEFAULT);
            let object = builder.intern_literal(RdfLiteral::typed(lexical, datatype));
            builder.push_quad(s, q, object, Some(s));
        }
        "literal_exact_metadata" => {
            let object = builder.intern_literal(RdfLiteral {
                lexical_form: "a\0é".to_owned(),
                datatype: Some(purrdf_iri::vocab::rdf::DIR_LANG_STRING.to_owned()),
                language: Some("fr".to_owned()),
                direction: Some(RdfTextDirection::Rtl),
            });
            builder.push_quad(g, p, object, None);
        }
        _ => panic!("unknown frozen state fixture: {name}"),
    }
    builder.freeze().expect("valid frozen state")
}

#[test]
fn frozen_canonical_payloads_match_independent_typed_preimages() {
    let mut names = BTreeSet::new();
    for line in VECTORS.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split_whitespace().collect();
        let [name, payload_hex, digest_hex] = fields.as_slice() else {
            panic!("malformed frozen state vector: {line}");
        };
        assert!(names.insert(*name), "duplicate frozen state vector");
        let dataset = fixture(name);
        let mut reservation = dataset.reserve_workspace(0).unwrap();
        let mut captured = Captured::new(&*dataset, &mut reservation).unwrap();
        captured.collect().unwrap();
        let actual = captured.canonical_bytes(RDFC_CALL_LIMIT).unwrap();
        let expected = purrdf_hash::hex::decode_canonical(payload_hex).unwrap();
        assert_eq!(actual, expected, "{name}");
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"purrdf-core/dataset-state/v1");
        hasher.update(&expected);
        let expected_digest = Digest32::new(*hasher.finalize().as_bytes());
        assert_eq!(
            expected_digest.to_hex(),
            *digest_hex,
            "{name} pinned digest"
        );
        assert_eq!(
            DatasetStateDigest::from_view(&dataset).unwrap().as_bytes(),
            expected_digest.as_bytes(),
            "{name} public digest"
        );
    }
    assert_eq!(names.len(), 12, "the complete frozen vector inventory");
}

#[test]
fn discrete_refinement_uses_one_search_node_and_a_zero_node_budget_refuses() {
    let dataset = fixtures::anchored_blanks(256, false);
    let mut reservation = dataset.reserve_workspace(0).unwrap();
    let mut captured = Captured::new(&*dataset, &mut reservation).unwrap();
    captured.collect().unwrap();
    assert_eq!(
        captured.canonical_bytes(0),
        Err(DatasetStateError::SearchBudgetExceeded),
    );
    let bytes = captured
        .canonical_bytes(1)
        .expect("one discrete terminal is one search node");
    assert_eq!(bytes, captured.canonical_bytes(RDFC_CALL_LIMIT).unwrap());
    let renamed = fixtures::anchored_blanks(256, true);
    assert_eq!(
        DatasetStateDigest::from_view(&dataset).unwrap(),
        DatasetStateDigest::from_view(&renamed).unwrap(),
    );
}

#[test]
fn genuine_branching_refuses_its_search_budget_and_has_an_admitted_neighbor() {
    let dataset = fixtures::triangle_components(2);
    let mut reservation = dataset.reserve_workspace(0).unwrap();
    let mut captured = Captured::new(&*dataset, &mut reservation).unwrap();
    captured.collect().unwrap();
    assert_eq!(
        captured.canonical_bytes(1),
        Err(DatasetStateError::SearchBudgetExceeded),
    );
    assert!(captured.canonical_bytes(RDFC_CALL_LIMIT).is_ok());
}

fn incidence_fixture(asymmetric: bool) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let p = builder.intern_iri("http://example.org/p");
    let q = builder.intern_iri("http://example.org/q");
    let g = builder.intern_iri("http://example.org/g");
    let h = builder.intern_iri("http://example.org/h");
    let labels = ["left", "right"];
    let blanks = labels.map(|label| builder.intern_blank(label, BlankScope::DEFAULT));
    for (index, &blank) in blanks.iter().enumerate() {
        let other = blanks[1 - index];
        let label = labels[index];
        let other_label = labels[1 - index];
        builder.declare_named_graph(blank);
        let inner = builder.intern_triple(blank, p, other);
        let outer = builder.intern_triple(other, p, inner);
        builder.push_quad(g, p, outer, None);
        builder.push_quad(g, q, h, Some(blank));
        builder.push_reifier(blank, inner);
        builder.push_annotation(blank, q, h);
        let list = builder.intern_literal(RdfLiteral::typed(
            format!("[ _:{label}, [ _:{other_label}, _:{label} ] ]"),
            purrdf_cdt::CDT_LIST,
        ));
        builder.push_quad(g, p, list, None);
        let map = builder.intern_literal(RdfLiteral::typed(
            format!("{{ _:{label}: [ _:{other_label}, _:{label} ] }}"),
            purrdf_cdt::CDT_MAP,
        ));
        builder.push_annotation(g, q, map);
        let embedded = builder.intern_literal(RdfLiteral::typed(
            format!(
                "[\"[_:{label}, _:{other_label}]\"^^<{}> ]",
                purrdf_cdt::CDT_LIST
            ),
            purrdf_cdt::CDT_LIST,
        ));
        builder.push_quad(g, q, embedded, None);
    }
    builder.push_quad(g, p, h, None);
    if asymmetric {
        let object = builder.intern_literal(RdfLiteral::typed(
            format!("[\"{{_:left: [_:left]}}\"^^<{}> ]", purrdf_cdt::CDT_MAP),
            purrdf_cdt::CDT_LIST,
        ));
        builder.push_quad(g, p, object, None);
    }
    builder.freeze().unwrap()
}

#[test]
fn incident_transpositions_match_every_record_and_restore_labels() {
    let mut cases: Vec<_> = VECTORS
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let name = line.split_whitespace().next().unwrap();
            (name, fixture(name))
        })
        .collect();
    cases.extend([
        (
            "interchangeable_leaves",
            fixtures::interchangeable_leaves(8, false),
        ),
        ("directed_triangles", fixtures::triangle_components(2)),
        ("mixed_symmetric", incidence_fixture(false)),
        ("mixed_asymmetric", incidence_fixture(true)),
    ]);
    let mut accepted = false;
    let mut refused = false;
    let mut pairs = 0;
    for (name, dataset) in cases {
        let mut reservation = dataset.reserve_workspace(0).unwrap();
        let mut captured = Captured::new(&*dataset, &mut reservation).unwrap();
        captured.collect().unwrap();
        let incidence = captured.incidence();
        for reverse in [false, true] {
            let mut identity: Vec<_> = (0..captured.blank_count).collect();
            if reverse {
                identity.reverse();
            }
            let before = identity.clone();
            // A fresh ordinal changes every occurrence of this blank without
            // using incidence to choose the record. This proves its exactness.
            for (blank, incident) in incidence.iter().enumerate() {
                let original = identity[blank];
                for (index, &record) in captured.records.iter().enumerate() {
                    let bytes = captured.render(record, Labels::Ordinals(&identity));
                    identity[blank] = captured.blank_count;
                    let changed = captured.render(record, Labels::Ordinals(&identity));
                    identity[blank] = original;
                    assert_eq!(
                        incident.binary_search(&index).is_ok(),
                        bytes != changed,
                        "{name} blank {blank}, record {index}, reverse={reverse}",
                    );
                }
            }
            let original = captured.rendered_records(Labels::Ordinals(&identity));
            for a in 0..captured.blank_count {
                for b in a + 1..captured.blank_count {
                    identity.swap(a, b);
                    let expected =
                        captured.rendered_records(Labels::Ordinals(&identity)) == original;
                    identity.swap(a, b);
                    let actual = captured.automorphism(a, b, &incidence, &mut identity);
                    assert_eq!(actual, expected, "{name}: {a}/{b}, reverse={reverse}");
                    assert_eq!(identity, before, "labels restored after {name}: {a}/{b}");
                    accepted |= actual;
                    refused |= !actual;
                    pairs += 1;
                }
            }
        }
    }
    assert!(
        accepted && refused,
        "both automorphisms and invalid transpositions are exercised"
    );
    assert_eq!(pairs, 90, "every blank pair under both ordinal bijections");
    eprintln!("incident/full-record transposition comparisons: {pairs}");
}
