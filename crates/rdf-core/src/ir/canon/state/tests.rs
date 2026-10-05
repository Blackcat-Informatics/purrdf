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
