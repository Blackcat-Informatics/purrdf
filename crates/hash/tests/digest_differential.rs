// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Replays the frozen differential vectors in `tests/vectors/` against every
//! execution path this host can run.
//!
//! Each file records, for 14,097 inputs, the digest an earlier dependency of
//! this workspace answered (see each file's header and `PROVENANCE.md`). An
//! input is the first `length` bytes of the little-endian `u64` stream of
//! `Xoshiro256::from_seed(seed)`. A disagreement is a defect in this crate,
//! never a reason to edit a vector.
//!
//! Every record is also hashed in two streamed pieces, split at a point drawn
//! from its seed, which must agree with the one-shot digest.
//!
//! The target is `harness = false` on `purrdf_testkit`'s runner, so the same
//! cases run natively under `cargo test` and on `wasm32-unknown-unknown` in
//! Node (`make wasm-test`), where only the portable paths exist.

use purrdf_hash::Digest;
use purrdf_hash::backend::{Crc32Backend, Sha1Backend};
use purrdf_hash::md5::Md5;
use purrdf_hash::sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};
use purrdf_testkit::rng::Xoshiro256;
use purrdf_testkit::vectors::VectorFile;

/// The recipe every file's header states.
fn input(length: usize, seed: u64) -> Vec<u8> {
    let mut rng = Xoshiro256::from_seed(seed);
    let mut bytes = Vec::with_capacity(length + 8);
    while bytes.len() < length {
        bytes.extend_from_slice(&rng.next_u64().to_le_bytes());
    }
    bytes.truncate(length);
    bytes
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, byte| {
        let _ = write!(out, "{byte:02x}");
        out
    })
}

/// Replays `text`, answering each record with `digest` of its input. `digest`
/// receives the input and a split point in `0..=input.len()`.
fn replay(name: &str, text: &str, mut digest: impl FnMut(&[u8], usize) -> String) {
    let file = VectorFile::parse(text).unwrap_or_else(|error| panic!("{name}: {error:?}"));
    let replayed = file
        .replay(2, |fields| {
            let length: usize = fields[0].parse().expect("a decimal length");
            let seed = u64::from_str_radix(fields[1], 16).expect("a hexadecimal seed");
            let data = input(length, seed);
            let split = (seed as usize) % (length + 1);
            vec![digest(&data, split)]
        })
        .unwrap_or_else(|mismatch| panic!("{name}: {mismatch}"));
    assert_eq!(replayed, 14_097, "{name}: every record replayed");
}

/// The one-shot digest, checked against `hasher` fed in two pieces.
fn one_shot_and_streamed(
    one_shot: &[u8],
    hasher: &mut dyn Digest,
    data: &[u8],
    split: usize,
) -> String {
    hasher.reset();
    hasher.update(&data[..split]);
    hasher.update(&data[split..]);
    let mut streamed = [0u8; purrdf_hash::MAX_OUTPUT_LEN];
    let len = hasher.finalize_reset(&mut streamed);
    assert_eq!(
        &streamed[..len],
        one_shot,
        "streamed with a split at {split}"
    );
    hex(one_shot)
}

fn md5_vectors_are_reproduced() {
    let mut hasher = Md5::new();
    replay(
        "md5",
        include_str!("vectors/md5_differential_vectors.txt"),
        |data, split| one_shot_and_streamed(&Md5::digest(data), &mut hasher, data, split),
    );
}

fn sha1_vectors_are_reproduced_on_every_path() {
    let text = include_str!("vectors/sha1_differential_vectors.txt");
    let mut ran = 0;
    for backend in Sha1Backend::ALL {
        let Some(mut hasher) = backend.hasher() else {
            continue;
        };
        replay(backend.name(), text, |data, split| {
            let one_shot = backend.digest(data).expect("an available path");
            one_shot_and_streamed(&one_shot, &mut hasher, data, split)
        });
        purrdf_testkit::harness::print_line(&format!("sha1: {} reproduced", backend.name()));
        ran += 1;
    }
    assert!(ran >= 1);
    // The public entry point is whichever path is selected; replay it too.
    let mut hasher = purrdf_hash::sha1::Sha1::new();
    replay("sha1 (selected)", text, |data, split| {
        one_shot_and_streamed(
            &purrdf_hash::sha1::Sha1::digest(data),
            &mut hasher,
            data,
            split,
        )
    });
}

fn sha3_224_vectors_are_reproduced() {
    let mut hasher = Sha3_224::new();
    replay(
        "sha3-224",
        include_str!("vectors/sha3_224_differential_vectors.txt"),
        |data, split| one_shot_and_streamed(&Sha3_224::digest(data), &mut hasher, data, split),
    );
}

fn sha3_256_vectors_are_reproduced() {
    let mut hasher = Sha3_256::new();
    replay(
        "sha3-256",
        include_str!("vectors/sha3_256_differential_vectors.txt"),
        |data, split| one_shot_and_streamed(&Sha3_256::digest(data), &mut hasher, data, split),
    );
}

fn sha3_384_vectors_are_reproduced() {
    let mut hasher = Sha3_384::new();
    replay(
        "sha3-384",
        include_str!("vectors/sha3_384_differential_vectors.txt"),
        |data, split| one_shot_and_streamed(&Sha3_384::digest(data), &mut hasher, data, split),
    );
}

fn sha3_512_vectors_are_reproduced() {
    let mut hasher = Sha3_512::new();
    replay(
        "sha3-512",
        include_str!("vectors/sha3_512_differential_vectors.txt"),
        |data, split| one_shot_and_streamed(&Sha3_512::digest(data), &mut hasher, data, split),
    );
}

fn crc32_vectors_are_reproduced_on_every_path() {
    let text = include_str!("vectors/crc32_differential_vectors.txt");
    let mut ran = 0;
    for backend in Crc32Backend::ALL {
        let Some(mut hasher) = backend.hasher() else {
            continue;
        };
        replay(backend.name(), text, |data, split| {
            let one_shot = backend.checksum(data).expect("an available path");
            one_shot_and_streamed(&one_shot.to_be_bytes(), &mut hasher, data, split)
        });
        purrdf_testkit::harness::print_line(&format!("crc32: {} reproduced", backend.name()));
        ran += 1;
    }
    assert!(ran >= 1);
    let mut hasher = purrdf_hash::crc32::Crc32::new();
    replay("crc32 (selected)", text, |data, split| {
        let one_shot = purrdf_hash::crc32::Crc32::checksum(data).to_be_bytes();
        one_shot_and_streamed(&one_shot, &mut hasher, data, split)
    });
}

purrdf_testkit::harness_main!(
    md5_vectors_are_reproduced,
    sha1_vectors_are_reproduced_on_every_path,
    sha3_224_vectors_are_reproduced,
    sha3_256_vectors_are_reproduced,
    sha3_384_vectors_are_reproduced,
    sha3_512_vectors_are_reproduced,
    crc32_vectors_are_reproduced_on_every_path,
);
