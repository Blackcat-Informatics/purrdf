// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The GTS identities computed under a hash domain, frozen: the MMR leaf, parent and
//! root domains through the language-neutral proof vector, and the segment-heads
//! aggregate through a frozen corpus file. A moved value is a changed published
//! identity.

use std::path::PathBuf;

use purrdf_gts::mmr::{Proof, prove, root, verify_proof};
use purrdf_gts::replication::{heads_json, inventory};

fn vector(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../vectors")
        .join(relative)
}

/// The four frame ids the proof vector commits: `[1; 32]` through `[4; 32]`.
fn frame_ids() -> Vec<Vec<u8>> {
    (1..=4u8).map(|byte| vec![byte; 32]).collect()
}

#[test]
fn the_mmr_proof_vector_is_what_this_implementation_proves() {
    let text = std::fs::read_to_string(vector("proofs/mmr-basic-proof.json")).expect("the vector");
    let frozen = Proof::from_json(&text).expect("the vector parses");
    verify_proof(&frozen).expect("the frozen proof verifies");
    let ids = frame_ids();
    assert_eq!(
        root(&ids),
        frozen.root,
        "the MMR root over the vector's frame ids"
    );
    assert_eq!(
        prove(&ids, frozen.leaf_index).expect("the leaf is in range"),
        frozen,
        "the proof, leaf and interior hashes included"
    );
}

#[test]
fn the_mmr_proof_vector_with_a_bad_root_is_refused() {
    let text = std::fs::read_to_string(vector("proofs/mmr-basic-proof-bad-root.json"))
        .expect("the vector");
    let frozen = Proof::from_json(&text).expect("the vector parses");
    assert!(
        verify_proof(&frozen).is_err(),
        "a proof against a wrong root must not verify"
    );
}

#[test]
fn the_segment_heads_aggregate_of_a_frozen_file_is_frozen() {
    let bytes = std::fs::read(vector("01-minimal.gts")).expect("the vector");
    let report = heads_json(&inventory(&bytes));
    assert!(
        report.contains(
            "\"digest\":\"9fef66c297fa8a187373c8e547c582ad30aedfab5f782651348fdcd29538ab81\""
        ),
        "the segment-heads aggregate moved: {report}"
    );
}
