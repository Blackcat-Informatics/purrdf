// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native `XXH64` seed, stripe, alignment and non-consuming prefix boundaries.
//! Expected digests come from `twox-hash` 2.1.5, recorded by `gen_xxh64_vectors`.

use core::hash::Hasher as _;

use purrdf_hash::xxh64::{Xxh64, hash};
use purrdf_testkit::rng::splitmix64_bytes;
use purrdf_testkit::vectors::VectorFile;

fn independent_vectors_cover_seed_stripe_and_streaming_boundaries() {
    let vectors = VectorFile::load(include_str!("vectors/xxh64_differential_vectors.txt"));
    let replayed = vectors
        .replay(2, |fields| {
            let seed = fields[0].parse().expect("a frozen seed");
            let length = fields[1].parse().expect("a frozen byte length");
            let input = splitmix64_bytes(length, 0);
            let expected = hash(seed, &input);
            for chunk_size in [1, 7, 17, 31, 32, 33, 128, 4096] {
                let mut state = Xxh64::with_seed(seed);
                state.write(&[]);
                for chunk in input.chunks(chunk_size) {
                    state.write(chunk);
                    state.update(&[]);
                }
                assert_eq!(
                    state.finish(),
                    expected,
                    "seed {seed}, length {length}, chunks {chunk_size}"
                );
            }
            let split = length / 2;
            let mut state = Xxh64::with_seed(seed);
            state.update(&input[..split]);
            let prefix = state.finish();
            assert_eq!(prefix, hash(seed, &input[..split]));
            state.update(&[]);
            assert_eq!(state.finish(), prefix);
            state.update(&input[split..]);
            assert_eq!(state.finish(), expected);

            let mut backing = vec![0; length + 8];
            for offset in 0..8 {
                let bytes = &mut backing[offset..offset + length];
                bytes.copy_from_slice(&input);
                assert_eq!(hash(seed, bytes), expected, "alignment class {offset}");
            }
            vec![format!("{expected:016x}")]
        })
        .expect("every native digest agrees with the independent oracle");
    assert_eq!(
        replayed, 304,
        "the complete independent seed and length corpus"
    );
}

purrdf_testkit::harness_main!(independent_vectors_cover_seed_stripe_and_streaming_boundaries);
