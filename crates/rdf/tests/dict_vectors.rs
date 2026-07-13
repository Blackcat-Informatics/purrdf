// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drift-guard tests for the two frozen dict-compaction corpus vectors
//! (Task 7, Part B): `vectors/30-dict-rawcontent.gts` (raw-content
//! in-band pack dictionary — fully cross-platform deterministic) and
//! `vectors/31-dict-trained.gts` (FastCOVER-trained in-band pack dictionary —
//! deterministic on the authoring platform but not guaranteed byte-identical
//! cross-platform, since FastCOVER's scoring involves transcendental floating
//! point; see `crates/gts/src/dict.rs`).
//!
//! Both vectors are frozen by `crates/rdf/src/bin/gen_dict_vectors.rs`; this
//! file duplicates that binary's exact fixed-source builder (a `[[bin]]`
//! target exposes no library surface a test could import) so a fresh
//! regeneration here always starts from the SAME bytes the frozen vectors
//! were authored from.
//!
//! Neither vector carries a `<id>.expected.json` cross-engine oracle: that
//! JSON is produced by gmeow-gts's `vectors.py` generator, which is not
//! vendored in this repository. `crates/gts/tests/frozen_canonical_bytes.rs`
//! still covers both (canonical-CBOR byte-exactness of every frozen `.gts`
//! item); the tests here are the purrdf-local functional/drift guard on top
//! of that (see `docs/GTS-CONFORMANCE.md` §2).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ciborium::value::Value;
use ed25519_dalek::SigningKey;
use purrdf_gts::compact::DictStrategy;
use purrdf_gts::model::Graph;
use purrdf_gts::reader::read;
use purrdf_gts::wire::{iter_items, map_get};
use purrdf_gts::writer::Writer;
use purrdf_rdf::gts_certify::{compact_and_certify, refold_digest, verify_compaction};

const TIMESTAMP: &str = "2026-01-01T00:00:00Z";

/// The fixed authorship signing key (`kid` "authorA") — matches
/// `gen_dict_vectors::authorship_key`.
fn authorship_key() -> SigningKey {
    SigningKey::from_bytes(&[3u8; 32])
}

/// The fixed packaging signing key (`kid` "pack") — matches
/// `gen_dict_vectors::packaging_key`.
fn packaging_key() -> SigningKey {
    SigningKey::from_bytes(&[7u8; 32])
}

/// Exactly `gen_dict_vectors::fixed_source`: 40 content-blob frames of
/// repeated structure, signed under the fixed authorship key, closed with an
/// `index` footer.
fn fixed_source() -> Vec<u8> {
    let mut w = Writer::new("purrdf.gts");
    w.sign_with(authorship_key(), "authorA");
    for i in 0..40u32 {
        let blob = format!(
            "<https://example.org/s{}> <https://example.org/p> \"dict vector claim {} about cats\" .\n",
            i % 7,
            i
        )
        .into_bytes();
        w.add_blob_owned(blob, Some("text/plain"), None);
    }
    w.add_index();
    w.into_bytes()
}

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vectors")
}

fn read_vector(name: &str) -> Vec<u8> {
    std::fs::read(vectors_dir().join(name)).unwrap_or_else(|err| panic!("read {name}: {err}"))
}

fn keyring() -> HashMap<String, ed25519_dalek::VerifyingKey> {
    HashMap::from([
        ("authorA".to_string(), authorship_key().verifying_key()),
        ("pack".to_string(), packaging_key().verifying_key()),
    ])
}

/// Sorted `(digest, decoded bytes)` for every blob in `g` — an order- and
/// codec-independent content identity, unaffected by which in-band
/// dictionary compressed the frames.
fn decoded_blobs(g: &Graph) -> Vec<(String, Vec<u8>)> {
    let mut out: Vec<(String, Vec<u8>)> = g
        .blobs
        .iter()
        .map(|(digest, entry)| {
            (
                digest.clone(),
                entry
                    .decoded_vec()
                    .unwrap_or_else(|err| panic!("blob {digest} decodes: {err}")),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Whether the file's header item (the first CBOR item, §3.1) carries a
/// non-empty `"dct"` map (§5) — the functional signal that a pack dictionary
/// was actually pinned in-band, not merely that some codec ran.
fn header_carries_dct_entry(bytes: &[u8]) -> bool {
    let (items, _torn) = iter_items(bytes);
    let Some((_, first)) = items.first() else {
        return false;
    };
    let inner = match first {
        Value::Tag(_, inner) => inner.as_ref(),
        other => other,
    };
    let Value::Map(entries) = inner else {
        return false;
    };
    matches!(map_get(entries, "dct"), Some(Value::Map(dct)) if !dct.is_empty())
}

#[test]
fn rawcontent_vector_is_byte_identical_to_a_fresh_regeneration() {
    let frozen = read_vector("30-dict-rawcontent.gts");
    let source = fixed_source();

    let (regenerated, _cert) = compact_and_certify(
        &source,
        DictStrategy::RawContent,
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("raw-content dict compaction succeeds over the fixed source");

    assert_eq!(
        regenerated, frozen,
        "the raw-content dict producer has no platform-dependent floating point, so a \
         fresh regeneration from the SAME fixed source must be byte-identical to the \
         frozen vector"
    );

    let folded = read(&frozen, true, None);
    assert!(
        folded.diagnostics.is_empty(),
        "the frozen raw-content vector must fold cleanly: {:?}",
        folded.diagnostics
    );
    assert_eq!(
        folded.blobs.len(),
        40,
        "every content blob survives compaction"
    );

    let ring = keyring();
    let report = verify_compaction(&source, &frozen, &ring).expect("verify_compaction succeeds");
    assert!(
        report.all_ok(),
        "the frozen raw-content vector must independently verify (incl. the carried \
         stream:detachedSignatureRoot over the signed source): {report:?}"
    );
}

#[test]
fn trained_vector_folds_cleanly_decodes_and_carries_a_dct_entry() {
    let frozen = read_vector("31-dict-trained.gts");
    let source = fixed_source();

    let folded = read(&frozen, true, None);
    assert!(
        folded.diagnostics.is_empty(),
        "the frozen trained-dict vector must fold cleanly: {:?}",
        folded.diagnostics
    );
    assert_eq!(
        folded.blobs.len(),
        40,
        "every content blob survives compaction"
    );
    for (digest, entry) in &folded.blobs {
        entry.decoded_vec().unwrap_or_else(|err| {
            panic!("blob {digest} decodes against the pinned in-band trained dictionary: {err}")
        });
    }
    assert!(
        header_carries_dct_entry(&frozen),
        "the trained-dict vector's header must pin a named, non-empty \"dct\" entry (§5)"
    );

    let ring = keyring();
    let report = verify_compaction(&source, &frozen, &ring).expect("verify_compaction succeeds");
    assert!(
        report.all_ok(),
        "the frozen trained-dict vector must independently verify (incl. the carried \
         stream:detachedSignatureRoot over the signed source): {report:?}"
    );
}

#[test]
fn trained_vector_folds_identically_to_a_fresh_regeneration() {
    let frozen = read_vector("31-dict-trained.gts");
    let source = fixed_source();

    let (regenerated, _cert) = compact_and_certify(
        &source,
        DictStrategy::Trained,
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("trained dict compaction succeeds over the fixed source");

    let frozen_fold = read(&frozen, true, None);
    let regenerated_fold = read(&regenerated, true, None);
    assert!(
        frozen_fold.diagnostics.is_empty(),
        "frozen vector folds cleanly"
    );
    assert!(
        regenerated_fold.diagnostics.is_empty(),
        "freshly regenerated pack folds cleanly"
    );

    // FastCOVER's dict bytes — and therefore the header/pack bytes — are
    // deliberately NOT asserted byte-equal here (cross-platform FP; see the
    // module docs). The FOLD is asserted identical instead: the same decoded
    // blob content and the same RDFC-1.0 content-refold digest, regardless of
    // which dictionary bytes compressed the frames on this platform.
    assert_eq!(
        decoded_blobs(&frozen_fold),
        decoded_blobs(&regenerated_fold),
        "a fresh trained-dict regeneration must decode to the SAME blob content as the \
         frozen vector, even if the trained dictionary bytes differ cross-platform"
    );
    assert_eq!(
        refold_digest(&frozen_fold).expect("frozen content-refold digest"),
        refold_digest(&regenerated_fold).expect("regenerated content-refold digest"),
        "fold-equality: the RDFC-1.0 content-refold digest must agree"
    );

    // Anti-tautology: the trained and raw-content strategies genuinely pin
    // different bytes over the SAME source — this is not a vacuous
    // byte-equality check against a strategy that ignores dict choice.
    let (raw_regenerated, _raw_cert) = compact_and_certify(
        &source,
        DictStrategy::RawContent,
        TIMESTAMP,
        false,
        (packaging_key(), "pack".to_string()),
    )
    .expect("raw-content dict compaction succeeds");
    assert_ne!(
        regenerated, raw_regenerated,
        "sanity: trained vs raw-content dict strategies pin different bytes"
    );
}
