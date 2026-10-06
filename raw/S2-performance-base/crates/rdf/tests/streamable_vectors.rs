// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Drift-guard tests for the frozen streamable-compaction corpus vector
//! `vectors/25b-streamable-compacted.gts` — the streamable compaction of
//! `vectors/25-streamable-source.gts` (which this file only READS; it is a
//! separately frozen top-level GTS vector, never regenerated here).
//!
//! Frozen by `crates/rdf/src/bin/gen_streamable_vectors.rs`; this file
//! duplicates that binary's exact fixed timestamp/packaging key so a fresh
//! regeneration here always starts from the SAME parameters the frozen
//! vector was authored from (a `[[bin]]` target exposes no library surface a
//! test could import — matches the `dict_vectors.rs` pattern for
//! `30-dict-rawcontent`/`31-dict-trained`).
//!
//! `crates/gts/tests/frozen_canonical_bytes.rs` separately covers this vector
//! for canonical-CBOR byte-exactness; the tests here are the purrdf-local
//! functional/drift guard on top of that (see `docs/GTS-CONFORMANCE.md` §2).

use purrdf_gts::compact::DictPlan;
use purrdf_gts::reader::read;
use purrdf_rdf::FastMap;
use purrdf_rdf::gts_certify::{compact_and_certify, verify_compaction};

#[path = "support/vectors.rs"]
mod vectors;
use purrdf_rdf::gts_dict_vectors::streamable_packaging_key;
use vectors::{header_carries_dct_entry, read_vector};

const TIMESTAMP: &str = "2026-01-01T00:00:00Z";

#[test]
fn frozen_vector_folds_cleanly_and_carries_the_source_content() {
    let frozen = read_vector("25b-streamable-compacted.gts");
    let g = read(&frozen, true, None);
    assert!(
        g.diagnostics.is_empty(),
        "the frozen streamable-compaction vector must fold cleanly: {:?}",
        g.diagnostics
    );
    assert_eq!(
        g.blobs.len(),
        2,
        "both of the source's content blobs survive compaction"
    );
    assert_eq!(
        g.segment_heads.len(),
        1,
        "a streamable compaction is exactly one segment"
    );
    assert!(
        g.segment_streamable[0].claimed,
        "the compacted pack must declare layout = \"streamable\""
    );
}

#[test]
fn frozen_vector_is_byte_identical_to_a_fresh_regeneration() {
    let source = read_vector("25-streamable-source.gts");
    let frozen = read_vector("25b-streamable-compacted.gts");

    let (regenerated, _cert) = compact_and_certify(
        &source,
        DictPlan::undicted(),
        TIMESTAMP,
        false,
        (streamable_packaging_key(), "pack".to_string()),
    )
    .expect("streamable compaction over the frozen source succeeds");

    assert_eq!(
        regenerated, frozen,
        "streamable compaction with DictPlan::undicted() has no platform-dependent \
         floating point (unlike 31-dict-trained's FastCOVER training), so a fresh \
         regeneration from the SAME frozen source must be byte-identical to the \
         frozen vector"
    );
}

#[test]
fn frozen_vector_independently_verifies_the_facets_this_repo_can_check() {
    let source = read_vector("25-streamable-source.gts");
    let frozen = read_vector("25b-streamable-compacted.gts");

    // This repo does not vendor the private OR public half of the "vector-key"
    // keypair `25-streamable-source.gts`'s own frame signatures are signed
    // under (that source is a separately frozen cross-engine oracle) — so
    // `signatures_verify` can never independently verify here and is
    // deliberately not asserted. Every other §10.1 preservation facet is
    // checkable with only the packaging key this binary controls.
    let mut keyring = FastMap::default();
    keyring.insert(
        "pack".to_string(),
        streamable_packaging_key().verifying_key(),
    );

    let report = verify_compaction(&source, &frozen, &keyring).expect("verify_compaction succeeds");
    assert!(
        report.refold_equivalent,
        "compaction must preserve content (RDFC-1.0 refold equivalence): {report:?}"
    );
    assert!(
        report.seam_chain_ok,
        "the compacted pack must fold with an intact, untorn hash chain: {report:?}"
    );
    assert!(
        report.signatures_bound,
        "the source's detached signatures must be bound under the pack's \
         stream:detachedSignatureRoot MMR commitment: {report:?}"
    );
    assert!(
        report.packaging_sig_ok,
        "the pack's own mandatory packaging (index/head) signature must verify \
         under the fixed packaging key: {report:?}"
    );
    assert!(
        report.suppressions_ok,
        "every suppression present in the source must be carried forward, and the \
         effective (post-suppression) digest must agree pre/post: {report:?}"
    );
}

#[test]
fn frozen_vector_pins_no_pack_dictionary() {
    // `25-streamable-source.gts` carries only two tiny content blobs (10 and 100
    // decoded bytes) — not the repeated-structure corpus a pack dictionary has
    // anything real to train on (that is what `30-dict-rawcontent`/
    // `31-dict-trained` freeze). `gen_streamable_vectors` deliberately compacts
    // with `DictPlan::undicted()`, so the frozen pack carries no `"dct"` header
    // entry and no zstd-compressed frames.
    let frozen = read_vector("25b-streamable-compacted.gts");

    // Authoritative proof (GTS-SPEC §5): the pack HEADER must carry no
    // `"dct"` map at all. Counting zstd frames alone cannot prove this — a
    // pack could carry a pinned `"dct"` entry while every blob still uses an
    // identity (uncompressed) frame, so this header-level check is the real
    // "no in-band pack dictionary is pinned" assertion.
    assert!(
        !header_carries_dct_entry(&frozen),
        "25b-streamable-compacted.gts header must carry no \"dct\" entry \
         (DictPlan::undicted pins no in-band pack dictionary)"
    );

    // Separate codec-behavior check: with no dictionary pinned, this frozen
    // pack's two tiny blobs also happen to carry no zstd-compressed frames.
    // This is a fact about THIS vector's codec choice, not by itself proof
    // of "no dictionary" (an inert pinned dictionary next to all-identity
    // frames would also count zero) — kept alongside, not instead of, the
    // header-level assertion above.
    const ZSTD_FRAME_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];
    let zstd_frames = frozen
        .windows(ZSTD_FRAME_MAGIC.len())
        .filter(|window| *window == ZSTD_FRAME_MAGIC)
        .count();
    assert_eq!(
        zstd_frames, 0,
        "25b-streamable-compacted.gts must carry no zstd-compressed frames \
         (DictPlan::undicted, no in-band pack dictionary)"
    );
}
