// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! RFC-3986 §5.4 reference-resolution conformance.
//!
//! Source of truth: RFC-3986 §5.4.1 (normal examples) and §5.4.2 (abnormal
//! examples), the normative table with base `http://a/b/c/d;p?q`. This is the
//! canonical committed ground truth for [`purrdf_iri::Iri::resolve`] — it is the
//! same table every conformant URI library is measured against.

use pretty_assertions::assert_eq;
use purrdf_iri::parse;

const BASE: &str = "http://a/b/c/d;p?q";

fn resolve(reference: &str) -> String {
    parse(BASE)
        .expect("base parses")
        .resolve(reference)
        .unwrap_or_else(|e| panic!("resolve({reference:?}) failed: {e}"))
        .as_str()
        .to_owned()
}

#[test]
fn rfc3986_5_4_1_normal_examples() {
    let cases: &[(&str, &str)] = &[
        ("g:h", "g:h"),
        ("g", "http://a/b/c/g"),
        ("./g", "http://a/b/c/g"),
        ("g/", "http://a/b/c/g/"),
        ("/g", "http://a/g"),
        ("//g", "http://g"),
        ("?y", "http://a/b/c/d;p?y"),
        ("g?y", "http://a/b/c/g?y"),
        ("#s", "http://a/b/c/d;p?q#s"),
        ("g#s", "http://a/b/c/g#s"),
        ("g?y#s", "http://a/b/c/g?y#s"),
        (";x", "http://a/b/c/;x"),
        ("g;x", "http://a/b/c/g;x"),
        ("g;x?y#s", "http://a/b/c/g;x?y#s"),
        ("", "http://a/b/c/d;p?q"),
        (".", "http://a/b/c/"),
        ("./", "http://a/b/c/"),
        ("..", "http://a/b/"),
        ("../", "http://a/b/"),
        ("../g", "http://a/b/g"),
        ("../..", "http://a/"),
        ("../../", "http://a/"),
        ("../../g", "http://a/g"),
    ];
    for (reference, expected) in cases {
        assert_eq!(&resolve(reference), expected, "ref = {reference:?}");
    }
}

#[test]
fn rfc3986_5_4_2_abnormal_examples() {
    let cases: &[(&str, &str)] = &[
        // Extra "../" that would back up past the root are ignored.
        ("../../../g", "http://a/g"),
        ("../../../../g", "http://a/g"),
        // Dot-segments where a complete path segment was expected.
        ("/./g", "http://a/g"),
        ("/../g", "http://a/g"),
        ("g.", "http://a/b/c/g."),
        (".g", "http://a/b/c/.g"),
        ("g..", "http://a/b/c/g.."),
        ("..g", "http://a/b/c/..g"),
        // Nonsensical but legal dot-segment sequences.
        ("./../g", "http://a/b/g"),
        ("./g/.", "http://a/b/c/g/"),
        ("g/./h", "http://a/b/c/g/h"),
        ("g/../h", "http://a/b/c/h"),
        ("g;x=1/./y", "http://a/b/c/g;x=1/y"),
        ("g;x=1/../y", "http://a/b/c/y"),
        // Dot-segments only matter in the path, not in query or fragment.
        ("g?y/./x", "http://a/b/c/g?y/./x"),
        ("g?y/../x", "http://a/b/c/g?y/../x"),
        ("g#s/./x", "http://a/b/c/g#s/./x"),
        ("g#s/../x", "http://a/b/c/g#s/../x"),
        // Strict resolution: a same-scheme reference is NOT treated as relative.
        ("http:g", "http:g"),
    ];
    for (reference, expected) in cases {
        assert_eq!(&resolve(reference), expected, "ref = {reference:?}");
    }
}

#[test]
fn resolve_requires_absolute_base() {
    // A relative base (no scheme) cannot resolve a reference.
    let rel = parse("/b/c/d").expect("relative ref parses");
    let err = rel
        .resolve("x")
        .expect_err("relative base must be rejected");
    assert_eq!(
        format!("{err}"),
        "base IRI is not absolute (no scheme): \"/b/c/d\""
    );
}
