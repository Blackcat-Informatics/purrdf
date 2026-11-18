// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! U+FFFE and U+FFFF in a literal through the canonical entry points.
//!
//! The 5.0 canonical document spells them as `UCHAR` (N-Quads 1.2 section 3: a
//! character outside the XML 1.1 `Char` production), its digest is over binary
//! lines and is the same from every entry point, and the retained 4.x
//! `purrdf-rdfc12` output escapes them too (profile version 4), so a 4.x digest
//! recorded over the raw spelling no longer recomputes for such a dataset.

use purrdf_core::{
    CANON_CORPUS_POLICY, CanonicalizationControl, try_canonical_digest_view,
    try_canonicalize_digest_view,
};
use purrdf_rdf::rdfc12::{CanonHash, RDFC_V2_CORPUS_POLICY, try_canonicalize_with};
use purrdf_rdf::{TermValue, parse_dataset};

const SOURCE: &str =
    "<http://example.org/s> <http://example.org/p> \"a\\uFFFEb\\uFFFFc\\u0085\" .\n";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

#[test]
fn the_document_escapes_the_noncharacters_and_every_digest_entry_agrees() {
    let dataset = parse_dataset(SOURCE.as_bytes(), "application/n-quads", None).expect("parses");
    let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
    let (canonical, digest) =
        try_canonicalize_digest_view(&dataset, &mut control).expect("canonicalizes");
    assert_eq!(
        canonical.nquads(),
        "<http://example.org/s> <http://example.org/p> \"a\\uFFFEb\\uFFFFc\u{85}\" .\n"
    );

    let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
    let alone = try_canonical_digest_view(&dataset, &mut control).expect("digest");
    assert_eq!(alone.as_bytes(), digest.as_bytes());

    // The spelled line parses back to the same literal.
    let reparsed =
        parse_dataset(canonical.nquads().as_bytes(), "application/n-quads", None).expect("reads");
    let literal = TermValue::typed_literal("a\u{fffe}b\u{ffff}c\u{85}", XSD_STRING);
    assert!(dataset.term_id_by_value(&literal).is_some());
    assert!(reparsed.term_id_by_value(&literal).is_some());
}

#[test]
fn the_retained_4x_output_escapes_the_noncharacters_and_keeps_c1_raw() {
    let dataset = parse_dataset(SOURCE.as_bytes(), "application/n-quads", None).expect("parses");
    let canonical = try_canonicalize_with(
        &dataset,
        CanonHash::Sha256,
        &mut CanonicalizationControl::new(RDFC_V2_CORPUS_POLICY, None),
    )
    .expect("canonicalizes");
    assert_eq!(
        canonical.nquads(),
        "<http://example.org/s> <http://example.org/p> \"a\\uFFFEb\\uFFFFc\u{85}\" .\n"
    );
}
