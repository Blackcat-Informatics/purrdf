// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Falsifiable acceptance tests for the certified-projection verifier
//! (`purrdf_core::ir::pack::certify`, Task 7 of the succinct-pack-codec
//! feature): [`verify_pack`] must independently reconstruct a pack's dataset,
//! recompute its RDFC-1.0 digest, and bind it to the SOURCE dataset's own
//! canonical identity — catching a tampered digest header field that the
//! per-section SHA-256 checks [`PackView::from_bytes`] already runs cannot see,
//! while still deferring to those section checks for section-body corruption.

use std::sync::Arc;

use purrdf_core::ir::pack::certify::{PackDigest, verify_pack};
use purrdf_core::ir::pack::container::{PackBuilder, PackError, PackView};
use purrdf_core::{CanonHash, RdfDataset, RdfDatasetBuilder, RdfLiteral, try_canonicalize_with};
use sha2::{Digest, Sha256};

/// A rich fixture (default + named graphs, literals with facets, blanks,
/// triple terms, reifiers, annotations) — deliberately varied so the
/// reconstruction path exercises every component `canon.rs` folds into the
/// RDFC-1.0 digest (base quads, reifier bindings, statement annotations).
fn rich_fixture() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();

    let s1 = b.intern_iri("http://example.org/s1");
    let p1 = b.intern_iri("http://example.org/p1");
    let o1 = b.intern_iri("http://example.org/o1");
    b.push_quad(s1, p1, o1, None);

    let s2 = b.intern_iri("http://example.org/s2");
    let p2 = b.intern_iri("http://example.org/p2");
    let lit_en = b.intern_literal(RdfLiteral {
        lexical_form: "hello".to_owned(),
        datatype: None,
        language: Some("en".to_owned()),
        direction: None,
    });
    b.push_quad(s2, p2, lit_en, None);

    let lit_typed = b.intern_literal(RdfLiteral {
        lexical_form: "42".to_owned(),
        datatype: Some("http://www.w3.org/2001/XMLSchema#integer".to_owned()),
        language: None,
        direction: None,
    });
    let p2b = b.intern_iri("http://example.org/p2b");
    b.push_quad(s2, p2b, lit_typed, None);

    let lit_dir = b.intern_literal(RdfLiteral {
        lexical_form: "ltr text".to_owned(),
        datatype: None,
        language: Some("en".to_owned()),
        direction: Some(purrdf_core::RdfTextDirection::Ltr),
    });
    let p2c = b.intern_iri("http://example.org/p2c");
    b.push_quad(s2, p2c, lit_dir, None);

    let blank1 = b.intern_blank("blank1", purrdf_core::BlankScope::default());
    let blank2 = b.intern_blank("blank2", purrdf_core::BlankScope::default());
    let p3 = b.intern_iri("http://example.org/p3");
    b.push_quad(blank1, p3, blank2, None);

    let triple1 = b.intern_triple(s1, p1, o1);
    let s4 = b.intern_iri("http://example.org/s4");
    let p4 = b.intern_iri("http://example.org/p4");
    b.push_quad(s4, p4, triple1, None);

    // A second, side-table-only triple term (never a base quad's S/P/O).
    let s6 = b.intern_iri("http://example.org/s6");
    let p6 = b.intern_iri("http://example.org/p6");
    let o6 = b.intern_iri("http://example.org/o6");
    let triple2 = b.intern_triple(s6, p6, o6);

    let g1 = b.intern_iri("http://example.org/g1");
    let g2 = b.intern_iri("http://example.org/g2");
    let s5 = b.intern_iri("http://example.org/s5");
    let p5 = b.intern_iri("http://example.org/p5");
    let o5 = b.intern_iri("http://example.org/o5");
    b.push_quad(s5, p5, o5, Some(g1));
    b.push_quad(s5, p5, triple1, Some(g2));

    let r1 = b.intern_iri("http://example.org/r1");
    let r2 = b.intern_iri("http://example.org/r2");
    b.push_reifier(r1, triple1);
    // r2 binds a side-table-only triple term, inside a named graph that owns
    // no base quad of its own.
    let g3 = b.intern_iri("http://example.org/g3");
    b.push_reifier_in_graph(r2, triple2, Some(g3));

    let ap1 = b.intern_iri("http://example.org/ap1");
    let ao1 = b.intern_iri("http://example.org/ao1");
    b.push_annotation(r1, ap1, ao1);
    let ap2 = b.intern_iri("http://example.org/ap2");
    let ao2 = b.intern_iri("http://example.org/ao2");
    b.push_annotation_in_graph(r1, ap2, ao2, Some(g2));
    let ap3 = b.intern_iri("http://example.org/ap3");
    let ao3 = b.intern_iri("http://example.org/ao3");
    b.push_annotation(r2, ap3, ao3);

    b.freeze().expect("rich fixture is a valid dataset")
}

/// The header's `rdfc_digest` field: offset 32, length 32 bytes (see
/// `ir::pack::container`'s module docs table) — sits inside the fixed 64-byte
/// header, entirely OUTSIDE the section directory's per-section SHA-256
/// coverage (that directory starts at offset 64).
const RDFC_DIGEST_HEADER_OFFSET: usize = 32;

#[test]
fn verify_pack_certifies_a_round_tripped_pack_against_the_source_digest() {
    let dataset = rich_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    let certified = verify_pack(&bytes).expect("a freshly built pack must verify");

    // The certificate must bind the pack to the SOURCE dataset's own RDFC-1.0
    // identity: canonicalize the source directly and compare.
    let source_canon =
        try_canonicalize_with(&dataset, CanonHash::Sha256).expect("source canonicalizes");
    let source_digest: [u8; 32] = Sha256::digest(source_canon.nquads.as_bytes()).into();

    assert_eq!(
        certified.as_bytes(),
        &source_digest,
        "verify_pack's certified digest must equal the source dataset's own RDFC-1.0 digest"
    );

    // And it must equal the header's stored value (what PackView::rdfc_digest
    // already exposed pre-Task-7) — verify_pack just corroborates it.
    let view = PackView::from_bytes(&bytes).expect("opens");
    assert_eq!(certified.as_bytes(), &view.rdfc_digest());
}

#[test]
fn verify_pack_rejects_a_tampered_digest_header_field() {
    let dataset = rich_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    let mut corrupted = bytes.clone();
    corrupted[RDFC_DIGEST_HEADER_OFFSET] ^= 0xFF;

    // The digest field is not covered by any section's SHA-256, so from_bytes
    // still opens the (structurally intact) pack successfully.
    assert!(
        PackView::from_bytes(&corrupted).is_ok(),
        "a flipped digest-header byte must NOT be caught by from_bytes's section checks"
    );

    // The independent recompute must catch it.
    let err = verify_pack(&corrupted).expect_err("a tampered digest header must fail verify_pack");
    match err {
        PackError::RdfcDigestMismatch { expected, computed } => {
            assert_ne!(expected, computed, "mismatch must report differing digests");
            assert_eq!(
                expected,
                {
                    let mut d = [0u8; 32];
                    d.copy_from_slice(
                        &corrupted[RDFC_DIGEST_HEADER_OFFSET..RDFC_DIGEST_HEADER_OFFSET + 32],
                    );
                    d
                },
                "the reported `expected` digest must be the tampered header value"
            );
        }
        other => panic!("expected RdfcDigestMismatch, got {other:?}"),
    }

    // A correctly-built pack does not trip this path.
    assert!(verify_pack(&bytes).is_ok());
}

#[test]
fn verify_pack_defers_to_section_integrity_for_body_corruption() {
    let dataset = rich_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");

    // Flip a byte strictly inside a section body (well past the fixed
    // header + directory, so it lands inside SOME section's bytes).
    let flip_at = bytes.len() - 1;
    let mut corrupted = bytes;
    corrupted[flip_at] ^= 0xFF;

    let err =
        verify_pack(&corrupted).expect_err("a flipped section byte must be caught by from_bytes");
    assert!(
        matches!(err, PackError::SectionDigestMismatch { .. }),
        "expected SectionDigestMismatch (from_bytes's defense), got {err:?}"
    );
}

#[test]
fn verify_pack_is_deterministic_across_two_builds() {
    let dataset = rich_fixture();
    let bytes_a = PackBuilder::build_bytes(&dataset).expect("builds");
    let bytes_b = PackBuilder::build_bytes(&dataset).expect("builds");

    let digest_a = verify_pack(&bytes_a).expect("verifies");
    let digest_b = verify_pack(&bytes_b).expect("verifies");

    assert_eq!(
        digest_a, digest_b,
        "two builds of the same dataset must verify to the same PackDigest"
    );
}

#[test]
fn pack_digest_to_hex_is_64_lowercase_hex_chars() {
    let dataset = rich_fixture();
    let bytes = PackBuilder::build_bytes(&dataset).expect("builds");
    let digest = verify_pack(&bytes).expect("verifies");

    let hex = digest.to_hex();
    assert_eq!(hex.len(), 64, "a SHA-256 digest hex-encodes to 64 chars");
    assert!(
        hex.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "to_hex must be lowercase hex: {hex}"
    );

    let redecoded = PackDigest::to_hex(&digest);
    assert_eq!(
        redecoded, hex,
        "to_hex is a pure function of the digest bytes"
    );
}
