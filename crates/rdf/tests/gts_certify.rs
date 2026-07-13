// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Streamable-compaction certificates (issue #89 Task 5): content projection
//! and refold-digest equivalence, `verify_compaction`, `compose`, and the
//! certifying authoring wrapper `compact_and_certify`.

use std::collections::HashMap;

use ed25519_dalek::SigningKey;
use purrdf_gts::compact::DictStrategy;
use purrdf_gts::model::{Term, TermKind};
use purrdf_gts::writer::Writer;
use purrdf_rdf::gts_certify::{
    CompactionCertificate, compact_and_certify, compose, verify_compaction,
};

const TIMESTAMP: &str = "2026-01-01T00:00:00Z";

/// A fixed, deterministic Ed25519 signing key (RFC 8032 signing is
/// deterministic per key + message, so tests stay byte-reproducible).
fn fixed_key(byte: u8) -> SigningKey {
    SigningKey::from_bytes(&[byte; 32])
}

fn iri_term(value: String) -> Term {
    Term {
        kind: TermKind::Iri,
        value: Some(value),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

fn literal_term(value: String) -> Term {
    Term {
        kind: TermKind::Literal,
        value: Some(value),
        datatype: None,
        lang: None,
        direction: None,
        reifier: None,
    }
}

/// A source GTS file carrying real RDF content — `n` `example.org` claims
/// (`<s{i}> <p> "claim {i}"`), authored as one signed terms frame and one
/// signed quads frame — so every frame a streamable compaction turns into
/// detached-signature provenance is exercised, not just blob frames.
///
/// When `mutate` names an index, that claim's object literal differs
/// (`"MUTATED claim {i}"`), producing a genuinely different source.
fn source_with_content(byte: u8, kid: &str, n: u32, mutate: Option<u32>) -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    w.sign_with(fixed_key(byte), kid);

    let mut terms = vec![iri_term("https://example.org/p".to_string())];
    let p = 0usize;
    let mut quads = Vec::new();
    for i in 0..n {
        let s = terms.len();
        terms.push(iri_term(format!("https://example.org/s{i}")));
        let o = terms.len();
        let text = if mutate == Some(i) {
            format!("MUTATED claim {i}")
        } else {
            format!("claim {i}")
        };
        terms.push(literal_term(text));
        quads.push((s, p, o, None));
    }
    w.add_terms(&terms);
    w.add_quads(&quads);
    w.into_bytes()
}

/// Like [`source_with_content`], plus `blob_n` compressible content blobs —
/// the corpus a pack dictionary strategy actually has something to train on.
fn source_with_content_and_blobs(byte: u8, kid: &str, quad_n: u32, blob_n: u32) -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    w.sign_with(fixed_key(byte), kid);

    let mut terms = vec![iri_term("https://example.org/p".to_string())];
    let p = 0usize;
    let mut quads = Vec::new();
    for i in 0..quad_n {
        let s = terms.len();
        terms.push(iri_term(format!("https://example.org/s{i}")));
        let o = terms.len();
        terms.push(literal_term(format!("claim {i}")));
        quads.push((s, p, o, None));
    }
    w.add_terms(&terms);
    w.add_quads(&quads);

    for i in 0..blob_n {
        let blob = format!(
            "<https://example.org/s{}> <https://example.org/p> \"blob claim {} about cats\" .\n",
            i % 37,
            i
        )
        .into_bytes();
        w.add_blob_owned(blob, Some("text/plain"), None);
    }
    w.into_bytes()
}

fn keyring(pairs: &[(&str, u8)]) -> HashMap<String, ed25519_dalek::VerifyingKey> {
    pairs
        .iter()
        .map(|&(kid, byte)| (kid.to_string(), fixed_key(byte).verifying_key()))
        .collect()
}

#[test]
fn faithful_repack_verifies_all_ok() {
    let source = source_with_content(1, "author", 5, None);
    let (pack, cert) = compact_and_certify(
        &source,
        DictStrategy::None,
        TIMESTAMP,
        false,
        (fixed_key(7), "pack".to_string()),
    )
    .expect("compact_and_certify succeeds on a clean signed source");

    let ring = keyring(&[("author", 1), ("pack", 7)]);
    let report = verify_compaction(&source, &pack, &ring).expect("verify_compaction succeeds");
    assert!(
        report.all_ok(),
        "a faithful repack must pass every check: {report:?}"
    );
    assert_eq!(
        cert.pre_refold_digest, cert.post_refold_digest,
        "a faithful repack's certificate carries equal pre/post digests"
    );
    assert!(
        !cert.detached_sig_roots.is_empty(),
        "the signed source carries detached signatures, so a root is bound"
    );
    assert_eq!(cert.packaging_kids, vec!["pack".to_string()]);
}

#[test]
fn content_mutation_breaks_refold_equivalence() {
    let source = source_with_content(1, "author", 5, None);
    // A genuinely different source: one claim's object literal differs.
    let mutated_source = source_with_content(1, "author", 5, Some(2));
    assert_ne!(
        source, mutated_source,
        "the mutated source must be byte-different from the original"
    );

    let (pack, _cert) = compact_and_certify(
        &source,
        DictStrategy::None,
        TIMESTAMP,
        false,
        (fixed_key(7), "pack".to_string()),
    )
    .expect("compact_and_certify succeeds");

    let ring = keyring(&[("author", 1), ("pack", 7)]);
    let report = verify_compaction(&mutated_source, &pack, &ring)
        .expect("verify_compaction still produces a report");
    assert!(
        !report.refold_equivalent,
        "a real content change (not the same pre) must flip refold_equivalent to false \
         (anti-tautology: this is not just comparing a digest to itself)"
    );
}

#[test]
fn codec_recompression_preserves_refold_equivalence() {
    let source = source_with_content_and_blobs(1, "author", 4, 48);

    let (trained, cert_trained) = compact_and_certify(
        &source,
        DictStrategy::Trained,
        TIMESTAMP,
        false,
        (fixed_key(7), "pack-trained".to_string()),
    )
    .expect("trained-dict compaction succeeds");
    let (raw, cert_raw) = compact_and_certify(
        &source,
        DictStrategy::RawContent,
        TIMESTAMP,
        false,
        (fixed_key(9), "pack-raw".to_string()),
    )
    .expect("raw-content-dict compaction succeeds");

    assert_ne!(
        trained, raw,
        "different dict strategies pin different dictionary bytes"
    );
    assert_eq!(
        cert_trained.pre_refold_digest, cert_raw.pre_refold_digest,
        "the SAME source folds to the same pre digest regardless of pack codec choice"
    );
    assert_eq!(
        cert_trained.post_refold_digest, cert_raw.post_refold_digest,
        "the fold is over decoded content, so codec re-compression does not change the \
         post digest either"
    );

    let ring = keyring(&[("author", 1), ("pack-trained", 7), ("pack-raw", 9)]);
    assert!(
        verify_compaction(&source, &trained, &ring)
            .expect("verify_compaction succeeds")
            .refold_equivalent
    );
    assert!(
        verify_compaction(&source, &raw, &ring)
            .expect("verify_compaction succeeds")
            .refold_equivalent
    );
}

#[test]
fn broken_seam_is_detected_without_erroring() {
    let source = source_with_content(1, "author", 5, None);
    let (pack, _cert) = compact_and_certify(
        &source,
        DictStrategy::None,
        TIMESTAMP,
        false,
        (fixed_key(7), "pack".to_string()),
    )
    .expect("compact_and_certify succeeds");

    // Corrupt one payload byte near the end of the pack (inside the trailing
    // signed `index` footer frame) — the frame's own content-id self-hash
    // mismatches, so the reader degrades it to a damaged opaque node (§7.6)
    // rather than aborting; every earlier frame (the actual content) still
    // folds cleanly, so this exercises exactly `seam_chain_ok` going false.
    let mut corrupted = pack.clone();
    let flip_at = corrupted.len() - 8;
    corrupted[flip_at] ^= 0xFF;
    assert_ne!(corrupted, pack);

    let ring = keyring(&[("author", 1), ("pack", 7)]);
    let report = verify_compaction(&source, &corrupted, &ring)
        .expect("a damaged trailing frame still yields a report, not an error");
    assert!(
        !report.seam_chain_ok,
        "a corrupted frame must be reported as a broken seam: {report:?}"
    );
}

#[test]
fn missing_authorship_key_fails_signatures_verify_but_full_keyring_succeeds() {
    let source = source_with_content(1, "author", 5, None);
    let (pack, _cert) = compact_and_certify(
        &source,
        DictStrategy::None,
        TIMESTAMP,
        false,
        (fixed_key(7), "pack".to_string()),
    )
    .expect("compact_and_certify succeeds");

    // A keyring that resolves the packaging key but has rotated past (or
    // never had) the original authorship key.
    let partial_ring = keyring(&[("pack", 7)]);
    let partial =
        verify_compaction(&source, &pack, &partial_ring).expect("verify_compaction succeeds");
    assert!(
        !partial.signatures_verify,
        "a keyring missing the authorship kid must fail signatures_verify"
    );
    assert!(
        partial.packaging_sig_ok,
        "the packaging signature is independent of the missing authorship key"
    );

    // Full (rotation-capable) keyring resolves both.
    let full_ring = keyring(&[("author", 1), ("pack", 7)]);
    let full = verify_compaction(&source, &pack, &full_ring).expect("verify_compaction succeeds");
    assert!(
        full.signatures_verify,
        "the full keyring resolves the carried authorship signatures: {full:?}"
    );
}

#[test]
fn compose_chains_matching_certificates_and_rejects_mismatched_ones() {
    let source = source_with_content(1, "author", 3, None);
    let (pack1, cert_ab) = compact_and_certify(
        &source,
        DictStrategy::None,
        TIMESTAMP,
        false,
        (fixed_key(7), "pack1".to_string()),
    )
    .expect("first compaction succeeds");
    let (_pack2, cert_bc) = compact_and_certify(
        &pack1,
        DictStrategy::None,
        TIMESTAMP,
        false,
        (fixed_key(9), "pack2".to_string()),
    )
    .expect("second compaction (of the first pack) succeeds");

    // `pack1` is the shared intermediate: cert_ab's post digest must equal
    // cert_bc's pre digest (content_projection strips ALL provenance layers,
    // including the first compaction's own, so re-compacting a pack yields
    // the SAME content digest as the original source).
    assert_eq!(cert_ab.post_refold_digest, cert_bc.pre_refold_digest);

    let composed = compose(&cert_ab, &cert_bc).expect("B is the shared intermediate for both");
    assert_eq!(composed.pre_refold_digest, cert_ab.pre_refold_digest);
    assert_eq!(composed.post_refold_digest, cert_bc.post_refold_digest);
    assert_eq!(
        composed.packaging_kids,
        vec!["pack1".to_string(), "pack2".to_string()]
    );
    assert_eq!(
        composed.detached_sig_roots.len(),
        cert_ab.detached_sig_roots.len() + cert_bc.detached_sig_roots.len(),
        "compose accretes (concatenates) the detached-signature roots of both certificates"
    );

    // A certificate whose pre digest is unrelated does not compose. (Content
    // must genuinely differ, not just the signing key — a different key over
    // the SAME subject/object content would still fold to the same digest,
    // since signatures never enter the content projection.)
    let other_source = source_with_content(2, "author2", 3, Some(0));
    let (_other_pack, cert_other) = compact_and_certify(
        &other_source,
        DictStrategy::None,
        TIMESTAMP,
        false,
        (fixed_key(11), "pack3".to_string()),
    )
    .expect("unrelated compaction succeeds");
    assert!(
        compose(&cert_ab, &cert_other).is_none(),
        "mismatched pre/post digests must not compose"
    );
}

#[test]
fn certificate_canonical_cbor_round_trips_and_is_deterministic() {
    let source = source_with_content(1, "author", 4, None);
    let (_pack, cert) = compact_and_certify(
        &source,
        DictStrategy::None,
        TIMESTAMP,
        false,
        (fixed_key(7), "pack".to_string()),
    )
    .expect("compact_and_certify succeeds");

    let first = cert.to_canonical_cbor();
    let second = cert.to_canonical_cbor();
    assert_eq!(
        first, second,
        "encoding the same certificate twice must be byte-identical"
    );

    let round_tripped =
        CompactionCertificate::from_canonical_cbor(&first).expect("canonical CBOR parses back");
    assert_eq!(
        round_tripped, cert,
        "from_canonical_cbor(to_canonical_cbor(cert)) must be identity"
    );
}
