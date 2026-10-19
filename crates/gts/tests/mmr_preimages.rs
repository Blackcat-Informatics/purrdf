// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Portable identities for the public fixed-state MMR path.

use purrdf_gts::mmr::{MmrPeak, MmrPeaks, Proof, prove, root, verify_proof};
use purrdf_gts::wire::{blake3_256, canonical};
use purrdf_lex::cbor::Value;

fn streamed_mmr_roots_preserve_frozen_proofs_and_variable_frame_ids() {
    let frozen = Proof::from_json(include_str!("../../../vectors/proofs/mmr-basic-proof.json"))
        .expect("frozen proof");
    let ids: Vec<Vec<u8>> = (1..=4_u8).map(|byte| vec![byte; 32]).collect();
    verify_proof(&frozen).expect("frozen proof verifies");
    assert_eq!(root(&ids), frozen.root);
    assert_eq!(
        prove(&ids, frozen.leaf_index).expect("covered leaf"),
        frozen
    );
    for length in [0, 23, 24, 255, 256, 65_535, 65_536] {
        let bytes = vec![0x91; length];
        // Independent tree encoder retains the published one-leaf preimage.
        let leaf = blake3_256(&canonical(&Value::Array(vec![
            purrdf_hash::Domain::new(b"gts-mmr-leaf-v1").as_str().into(),
            0_u64.into(),
            Value::Bytes(bytes.clone()),
        ])));
        let expected = blake3_256(&canonical(&Value::Array(vec![
            purrdf_hash::Domain::new(b"gts-mmr-root-v1").as_str().into(),
            1_u64.into(),
            Value::Array(vec![Value::Array(vec![
                0_u64.into(),
                Value::Bytes(leaf.to_vec()),
            ])]),
        ])));
        let mut frontier = MmrPeaks::default();
        frontier.push(&bytes).expect("frame fits");
        assert_eq!(frontier.root(), expected);
        assert_eq!(root(&[bytes]), expected);
    }
    // The frontier count is u64 even on the 32-bit target, and a full frontier
    // has 64 peaks: both integer and array heads must retain their wide forms.
    for count in [1_u64 << 32, 1_u64 << 63, u64::MAX] {
        let peaks: Vec<MmrPeak> = (0..64)
            .rev()
            .filter(|height| count & (1_u64 << height) != 0)
            .map(|height| MmrPeak {
                height,
                hash: vec![0x91; 32],
            })
            .collect();
        let expected = blake3_256(&canonical(&Value::Array(vec![
            purrdf_hash::Domain::new(b"gts-mmr-root-v1").as_str().into(),
            count.into(),
            Value::Array(
                peaks
                    .iter()
                    .map(|peak| {
                        Value::Array(vec![
                            (peak.height as u64).into(),
                            Value::Bytes(peak.hash.clone()),
                        ])
                    })
                    .collect(),
            ),
        ])));
        let frontier = MmrPeaks::from_parts(count, peaks).expect("valid wide frontier");
        assert_eq!(frontier.root(), expected);
    }
}

purrdf_testkit::harness_main!(streamed_mmr_roots_preserve_frozen_proofs_and_variable_frame_ids);
