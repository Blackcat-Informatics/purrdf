// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One import-key policy at every insertion site, seen through the reasoning boundary
//! ([`premise_import_map`]) and the shapes boundary ([`ShapesImports`]): both are built on
//! `ImportMap::try_insert`, so a key one refuses the other refuses, and the neighbour that
//! is valid (two distinct absolute IRIs) is accepted by both.

use purrdf_shapes::ShapesImports;
use purrdf_validate::premise_import_map;

const A: &str = "http://example.org/a";
const B: &str = "http://example.org/b";
const DOC: &str = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";
const TTL: &str = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n";

#[test]
fn two_distinct_absolute_keys_are_accepted_by_both_boundaries() {
    let map = premise_import_map(&[(A, DOC), (B, DOC)], &[]).expect("distinct keys");
    assert_eq!(map.len(), 2);
    ShapesImports::from_turtle(&[(A, TTL), (B, TTL)]).expect("distinct keys");
}

#[test]
fn a_repeated_key_is_refused_by_both_boundaries() {
    let error = premise_import_map(&[(A, DOC), (A, DOC)], &[]).expect_err("a repeated key");
    assert!(error.contains("twice") && error.contains(A), "{error}");
    let error = ShapesImports::from_turtle(&[(A, TTL), (A, TTL)]).expect_err("a repeated key");
    assert!(error.to_string().contains("twice"), "{error}");
}

#[test]
fn a_relative_or_empty_key_is_refused_by_both_boundaries() {
    for key in ["lib", "../lib"] {
        let error = premise_import_map(&[(key, DOC)], &[]).expect_err("a non-absolute key");
        assert!(error.contains("import key <"), "{key:?}: {error}");
        let error = ShapesImports::from_turtle(&[(key, TTL)]).expect_err("a non-absolute key");
        assert!(
            error.to_string().contains("import key <"),
            "{key:?}: {error}"
        );
    }
}

#[test]
fn the_empty_key_is_refused_by_both_boundaries() {
    let error = premise_import_map(&[("", DOC)], &[]).expect_err("the empty key");
    assert!(error.contains("empty ontology IRI"), "{error}");
    let error = ShapesImports::from_turtle(&[("", TTL)]).expect_err("the empty key");
    assert!(error.to_string().contains("empty ontology IRI"), "{error}");
}
