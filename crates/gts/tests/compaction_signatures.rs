// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Detached-signature MMR binding + mandatory packaging head signature +
//! keyring (rotation-capable) verification (issue #89 Task 4).

use std::collections::HashMap;

use ed25519_dalek::SigningKey;
use purrdf_gts::compact::{
    CompactionParams, DictStrategy, compact_streamable, detached_signature_leaves,
    detached_signature_proof,
};
use purrdf_gts::mmr;
use purrdf_gts::model::{Graph, Signature};
use purrdf_gts::reader::read;
use purrdf_gts::stream;
use purrdf_gts::verify::verify_file_with_keyring;
use purrdf_gts::wire::blake3_256;
use purrdf_gts::writer::Writer;

/// A fixed, deterministic Ed25519 signing key (RFC 8032 signing is
/// deterministic per key + message, so tests stay byte-reproducible).
fn fixed_key(byte: u8) -> SigningKey {
    SigningKey::from_bytes(&[byte; 32])
}

/// A source GTS file whose every frame (including the blob frames a
/// streamable compaction turns into detached-signature provenance) is
/// COSE_Sign1-signed under `kid` with a fixed key.
fn source_signed(byte: u8, kid: &str, blob_count: u32) -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    w.sign_with(fixed_key(byte), kid);
    for i in 0..blob_count {
        let blob = format!("frame authorship payload {i}").into_bytes();
        w.add_blob_owned(blob, Some("text/plain"), None);
    }
    w.into_bytes()
}

/// A source GTS file with no signer configured — no frame carries a `sig`.
fn source_unsigned(blob_count: u32) -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    for i in 0..blob_count {
        let blob = format!("unsigned payload {i}").into_bytes();
        w.add_blob_owned(blob, Some("text/plain"), None);
    }
    w.into_bytes()
}

/// The literal value of the object of the first quad using `predicate_iri`.
fn object_literal(g: &Graph, predicate_iri: &str) -> Option<String> {
    let p = g
        .terms
        .iter()
        .position(|t| t.value.as_deref() == Some(predicate_iri))?;
    g.quads
        .iter()
        .find(|&&(_, pred, _, _)| pred == p)
        .and_then(|&(_, _, o, _)| g.terms[o].value.clone())
}

fn packaging_params<'a>(packaging_key: SigningKey, packaging_kid: &str) -> CompactionParams<'a> {
    CompactionParams {
        timestamp: "2026-01-01T00:00:00Z",
        seal_original: false,
        strategy: DictStrategy::None,
        content_digest: None,
        packaging_signer: Some((packaging_key, packaging_kid.to_string())),
    }
}

#[test]
fn detached_signatures_are_bound_under_the_mmr_root_and_prove_individually() {
    let source = source_signed(1, "kidA", 5);
    let packed_a = compact_streamable(&source, packaging_params(fixed_key(7), "pack"))
        .expect("compaction succeeds");
    let packed_b = compact_streamable(&source, packaging_params(fixed_key(7), "pack"))
        .expect("compaction succeeds");
    assert_eq!(
        packed_a, packed_b,
        "packaging-signed compaction is byte-deterministic"
    );

    // Fold the source exactly as `compact_streamable`'s refusal gate does, so
    // the detached-signature set is derived independently of the compactor.
    let source_graph = read(&source, true, None);
    let leaves = detached_signature_leaves(&source_graph);
    assert!(
        !leaves.is_empty(),
        "the signed source carries detached signatures"
    );
    let expected_root = mmr::root(&leaves);

    let packed_graph = read(&packed_a, true, None);
    let root_literal = object_literal(&packed_graph, stream::DETACHED_SIGNATURE_ROOT)
        .expect("stream:detachedSignatureRoot quad is present on the Compaction node");
    let parsed_root =
        mmr::parse_hex_32(&root_literal).expect("root literal parses as a 32-byte hex value");
    assert_eq!(
        parsed_root, expected_root,
        "the emitted root matches the independently derived MMR root"
    );

    // Every detached signature proves individually against the emitted root.
    for sig in &source_graph.signatures {
        let cose = sig
            .cose
            .as_deref()
            .expect("every source signature carries raw COSE bytes");
        let proof = detached_signature_proof(&source_graph, &sig.frame_id, cose)
            .expect("a present (frame_id, cose) pair has a selective inclusion proof");
        assert_eq!(
            proof.root, expected_root,
            "the proof targets the emitted root"
        );
        mmr::verify_proof(&proof).expect("the per-frame authorship proof verifies standalone");
    }

    // An absent (frame_id, cose) pair proves nothing.
    let absent = detached_signature_proof(&source_graph, &[0xffu8; 32], b"not-a-real-cose");
    assert!(
        absent.is_none(),
        "an absent (frame_id, cose) pair has no inclusion proof"
    );
}

#[test]
fn detached_signature_leaves_sort_by_frame_id_then_cose_for_rotation_cosigners() {
    let frame_a = vec![1u8; 32];
    let frame_b = vec![2u8; 32];
    // Two co-signatures over the SAME frame (a rotation window where both the
    // old and new key sign the same content) plus one signature over a
    // different frame. `frame_id` alone cannot disambiguate the co-signers —
    // the `cose` tie-break is required for a stable leaf order.
    let g = Graph {
        signatures: vec![
            Signature {
                frame_id: frame_b.clone(),
                kid: None,
                status: "unverified".to_string(),
                cose: Some(vec![9, 9]),
            },
            Signature {
                frame_id: frame_a.clone(),
                kid: None,
                status: "unverified".to_string(),
                cose: Some(vec![2, 0]),
            },
            Signature {
                frame_id: frame_a.clone(),
                kid: None,
                status: "unverified".to_string(),
                cose: Some(vec![1, 0]),
            },
        ],
        ..Graph::default()
    };

    let mut expected_pairs = vec![
        (frame_a.clone(), vec![1u8, 0]),
        (frame_a, vec![2u8, 0]),
        (frame_b, vec![9u8, 9]),
    ];
    expected_pairs.sort();
    let expected_leaves: Vec<Vec<u8>> = expected_pairs
        .into_iter()
        .map(|(mut frame_id, cose)| {
            frame_id.extend_from_slice(&cose);
            blake3_256(&frame_id)
        })
        .collect();

    let leaves_first_call = detached_signature_leaves(&g);
    let leaves_second_call = detached_signature_leaves(&g);
    assert_eq!(
        leaves_first_call, leaves_second_call,
        "leaf order is deterministic across repeated calls"
    );
    assert_eq!(
        leaves_first_call, expected_leaves,
        "leaves are ordered by (frame_id, cose), not signature insertion order"
    );
}

#[test]
fn an_unsigned_source_emits_no_detached_signature_root_and_folds_cleanly() {
    let source = source_unsigned(4);
    let packed = compact_streamable(&source, packaging_params(fixed_key(7), "pack"))
        .expect("compaction of an unsigned source succeeds");

    let packed_graph = read(&packed, true, None);
    assert!(
        packed_graph.diagnostics.is_empty(),
        "an unsigned-source pack folds cleanly: {:?}",
        packed_graph.diagnostics
    );
    assert!(
        object_literal(&packed_graph, stream::DETACHED_SIGNATURE_ROOT).is_none(),
        "no detachedSignatureRoot quad is emitted for an empty detached-signature set"
    );

    let source_graph = read(&source, true, None);
    assert!(
        detached_signature_leaves(&source_graph).is_empty(),
        "an unsigned source has no detached-signature leaves"
    );
}

#[test]
fn the_packaging_head_signature_is_distinct_from_carried_authorship_signatures() {
    let source = source_signed(1, "kidA", 3);
    let packaging_kid = "pack";
    let packed = compact_streamable(&source, packaging_params(fixed_key(7), packaging_kid))
        .expect("compaction succeeds");

    let packed_graph = read(&packed, true, None);
    assert_eq!(
        packed_graph.signatures.len(),
        1,
        "the compacted pack's folded `signatures` carries ONLY the packaging \
         (index/head) signature — authorship signatures live in detached \
         provenance quads, not as frame `sig` fields"
    );
    let packaging_sig = &packed_graph.signatures[0];
    assert_eq!(
        Some(&packaging_sig.frame_id),
        packed_graph.segment_heads.last(),
        "the packaging signature authenticates the index/head frame"
    );

    let public = fixed_key(7).verifying_key();
    let mut keyring = HashMap::new();
    keyring.insert(packaging_kid.to_string(), public);
    let result = verify_file_with_keyring(&packed, &keyring);
    assert_eq!(
        result.valid, 1,
        "the packaging signature verifies under its key"
    );
    assert_eq!(result.signed, 1);
    assert!(result.ok, "verification succeeds: {:?}", result.errors);
}

#[test]
fn keyring_rotation_flips_the_packaging_signature_from_unverified_to_valid() {
    let source = source_signed(1, "kidA", 3);
    let packaging_kid = "pack-v2";
    let packed = compact_streamable(&source, packaging_params(fixed_key(9), packaging_kid))
        .expect("compaction succeeds");

    // A keyring that has rotated PAST the packaging key (missing "pack-v2")
    // leaves the packaging signature unverified.
    let stale_keyring: HashMap<String, ed25519_dalek::VerifyingKey> =
        HashMap::from([("pack-v1".to_string(), fixed_key(1).verifying_key())]);
    let stale = verify_file_with_keyring(&packed, &stale_keyring);
    assert!(
        !stale.ok,
        "a keyring missing the packaging kid must not verify"
    );
    assert_eq!(stale.unverified, 1);
    assert_eq!(stale.valid, 0);

    // A keyring carrying both the retired and the current packaging key
    // (rotation-capable) resolves the signature.
    let rotated_keyring: HashMap<String, ed25519_dalek::VerifyingKey> = HashMap::from([
        ("pack-v1".to_string(), fixed_key(1).verifying_key()),
        (packaging_kid.to_string(), fixed_key(9).verifying_key()),
    ]);
    let rotated = verify_file_with_keyring(&packed, &rotated_keyring);
    assert!(
        rotated.ok,
        "a keyring carrying the current packaging key verifies: {:?}",
        rotated.errors
    );
    assert_eq!(rotated.valid, 1);
    assert_eq!(rotated.unverified, 0);
}
