// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent lane identity across unequal message, block and tree lengths.

use super::{OFFICIAL_CONTEXT, OFFICIAL_KEY};
use purrdf_hash::Backend as _;
use purrdf_hash::blake3::{self, Backend, DeriveKeyContext, Hash};

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn independent_batches_match_portable_through_two_chunks() {
    let data = super::corpus();
    let context = DeriveKeyContext::new(OFFICIAL_CONTEXT);
    // The permutation mixes lengths within each SIMD group while visiting
    // every length exactly once. Shift each start to exercise unaligned loads.
    let inputs: Vec<_> = (0..2049)
        .map(|i| {
            let len = i * 787 % 2049;
            let start = i % 31;
            &data[start..start + len]
        })
        .collect();
    compare_modes(&inputs, &context, Backend::all_available());
}

fn modes(inputs: &[&[u8]], context: &DeriveKeyContext, backend: Option<Backend>) -> [Vec<Hash>; 3] {
    let mut out = core::array::from_fn(|_| vec![Hash::ZERO; inputs.len()]);
    if let Some(backend) = backend {
        backend.hash_batch(inputs, &mut out[0]).unwrap().unwrap();
        backend
            .keyed_hash_batch(OFFICIAL_KEY, inputs, &mut out[1])
            .unwrap()
            .unwrap();
        backend
            .derive_key_batch(context, inputs, &mut out[2])
            .unwrap()
            .unwrap();
    } else {
        blake3::hash_batch(inputs, &mut out[0]).unwrap();
        blake3::keyed_hash_batch(OFFICIAL_KEY, inputs, &mut out[1]).unwrap();
        context.derive_batch(inputs, &mut out[2]).unwrap();
    }
    out
}

fn compare_modes(
    inputs: &[&[u8]],
    context: &DeriveKeyContext,
    paths: impl IntoIterator<Item = Backend>,
) {
    let expected = modes(inputs, context, Some(Backend::Portable));
    assert_eq!(modes(inputs, context, None), expected, "default dispatch");
    for path in paths {
        assert_eq!(modes(inputs, context, Some(path)), expected, "{path:?}");
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn independent_batches_replay_all_admitted_official_vectors() {
    let context = DeriveKeyContext::new(OFFICIAL_CONTEXT);
    let data = super::corpus();
    let available: Vec<_> = Backend::all_available().collect();
    replay_official_batches(&data, &context, &available, 16);
    // These mixed groups force the three- and four-chunk reductions, which
    // must coexist with a single-block root in the very same group.
    let boundaries = [4096, 0, 3073, 1025, 2049, 1, 3072, 2048, 1024, 65, 64, 63];
    let mixed: Vec<_> = boundaries
        .map(|n| &data[..n])
        .into_iter()
        .cycle()
        .take(33)
        .collect();
    compare_modes(&mixed, &context, available);
}

fn replay_official_batches(
    data: &[u8],
    context: &DeriveKeyContext,
    paths: &[Backend],
    max_width: usize,
) {
    use purrdf_hash::hex::Lower;
    use purrdf_testkit::vectors::VectorFile;

    let vectors = VectorFile::load(super::OFFICIAL);
    let records: Vec<_> = vectors
        .records()
        .iter()
        .filter_map(|record| {
            let len = record.fields[0]
                .parse::<usize>()
                .expect("official vector length");
            (len <= blake3::MAX_BATCH_MESSAGE).then(|| (&data[..len], &record.fields[1..]))
        })
        .collect();
    // Every full width and partial width, including force-selected SIMD.
    for width in 1..=max_width {
        for batch in records.chunks(width) {
            let inputs: Vec<_> = batch.iter().map(|(data, _)| *data).collect();
            for &path in paths {
                let outputs = modes(&inputs, context, Some(path));
                for (lane, (_, expected)) in batch.iter().enumerate() {
                    for (mode, out) in outputs.iter().enumerate() {
                        assert_eq!(
                            Lower(out[lane].as_bytes()).to_string(),
                            expected[mode][..64]
                        );
                    }
                    assert_eq!(context.derive(inputs[lane]), *outputs[2][lane].as_bytes());
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn refusal_leaves_every_batch_slot_intact() {
    use blake3::{BatchError, MAX_BATCH_MESSAGE, hash_batch};

    let boundary = vec![1; MAX_BATCH_MESSAGE + 1];
    let mut out = [Hash::ZERO; 2];
    assert_eq!(
        hash_batch(&[b"valid", &boundary], &mut out),
        Err(BatchError::MessageLength {
            index: 1,
            bytes: MAX_BATCH_MESSAGE + 1
        })
    );
    assert_eq!(
        hash_batch(&[b"valid"], &mut out),
        Err(BatchError::OutputCount {
            messages: 1,
            slots: 2
        })
    );
    assert_eq!(out, [Hash::ZERO; 2]);
    hash_batch(&[b"valid", &boundary[..MAX_BATCH_MESSAGE]], &mut out).unwrap();
    assert_eq!(
        out,
        [
            blake3::hash(b"valid"),
            blake3::hash(&boundary[..MAX_BATCH_MESSAGE])
        ]
    );
}

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
pub(super) fn mixed_message_batch_simd_dispatch() {
    assert_eq!(Backend::selected(), Backend::Wasm128);
    let data = super::corpus();
    let context = DeriveKeyContext::new(OFFICIAL_CONTEXT);
    replay_official_batches(&data, &context, &[Backend::Wasm128], 4);
    let lengths = [4096, 0, 3073, 1025, 65];
    compare_modes(
        &lengths.map(|len| &data[1..1 + len]),
        &context,
        [Backend::Wasm128],
    );
}
