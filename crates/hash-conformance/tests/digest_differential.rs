// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Replays the frozen differential vectors in `tests/vectors/` against every
//! execution path this host can run.
//!
//! Each file records, for 14,097 inputs, the digest an earlier dependency of
//! this workspace answered (see each file's header and `crates/hash/PROVENANCE.md`). An
//! input is the first `length` bytes of the little-endian `u64` stream of
//! `Xoshiro256::from_seed(seed)`. A disagreement is a defect in purrdf-hash,
//! never a reason to edit a vector.
//!
//! Every record is also hashed in two streamed pieces, split at a point drawn
//! from its seed, which must agree with the one-shot digest.
//!
//! The target is `harness = false` on `purrdf_testkit`'s runner, so the same
//! full corpus runs natively under `cargo test`. `make wasm-test` selects only
//! the compact integer-lowering probe below: wasm32 has no additional MD5,
//! SHA-1, SHA-3 or CRC-32 backend, so it does not repeat their semantic corpora.
//! The probe uses frozen answers at block and padding boundaries to qualify
//! wasm integer operations and streamed length arithmetic.

use purrdf_hash::backend::{Crc32Backend, Sha1Backend};
use purrdf_hash::dispatch::{assert_required_available, host_advertises};
use purrdf_hash::hex::encode;
use purrdf_hash::md5::Md5;
use purrdf_hash::sha3::{Sha3_224, Sha3_256, Sha3_384, Sha3_512};
use purrdf_hash::{Backend, Digest};
use purrdf_testkit::rng::xoshiro256_bytes;
use purrdf_testkit::vectors::VectorFile;

/// Replays `text`, answering each record with `digest` of its input. `digest`
/// receives the input and a split point in `0..=input.len()`.
fn replay(name: &str, text: &str, mut digest: impl FnMut(&[u8], usize) -> String) {
    let file = VectorFile::parse(text).unwrap_or_else(|error| panic!("{name}: {error:?}"));
    let replayed = file
        .replay(2, |fields| {
            let length: usize = fields[0].parse().expect("a decimal length");
            let seed = u64::from_str_radix(fields[1], 16).expect("a hexadecimal seed");
            let data = xoshiro256_bytes(length, seed);
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
    encode(one_shot)
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
    for backend in Sha1Backend::all_available() {
        let mut hasher = backend.hasher().expect("an available path");
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
    for backend in Crc32Backend::all_available() {
        let mut hasher = backend.hasher().expect("an available path");
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

/// Whether this host is expected to run a SHA-1 path: its architecture, and
/// the processor features it advertises independently of the detection
/// under test.
fn sha1_expected_here(backend: Sha1Backend) -> bool {
    match backend {
        Sha1Backend::Portable => true,
        Sha1Backend::X86Sha => {
            cfg!(target_arch = "x86_64") && host_advertises(&["sha_ni", "ssse3", "sse4_1"])
        }
        Sha1Backend::Aarch64Sha1 => cfg!(target_arch = "aarch64") && host_advertises(&["sha1"]),
    }
}

/// Whether this host is expected to run a CRC-32 path, as
/// [`sha1_expected_here`].
fn crc32_expected_here(backend: Crc32Backend) -> bool {
    match backend {
        Crc32Backend::Portable => true,
        Crc32Backend::X86Pclmulqdq => {
            cfg!(target_arch = "x86_64") && host_advertises(&["pclmulqdq", "sse4_1"])
        }
        Crc32Backend::Aarch64Crc32 => cfg!(target_arch = "aarch64") && host_advertises(&["crc32"]),
        Crc32Backend::Aarch64Pmull => {
            cfg!(target_arch = "aarch64") && host_advertises(&["pmull", "crc32"])
        }
    }
}

/// Every SHA-1 and CRC-32 path `PURRDF_REQUIRE_SIMD_PATHS` requires is
/// available, and a required hardware path is the one the public API
/// selects (except `aarch64-pmull`, available but never selected). The
/// replays above run every available path, so a path required here is also a
/// path they executed.
fn required_sha1_and_crc32_paths_are_available_and_selected() {
    for backend in assert_required_available("sha1", sha1_expected_here) {
        if backend != Sha1Backend::Portable {
            assert_eq!(
                Sha1Backend::selected(),
                backend,
                "sha1:{} is required but not selected",
                backend.name()
            );
        }
    }
    for backend in assert_required_available("crc32", crc32_expected_here) {
        if !matches!(backend, Crc32Backend::Portable | Crc32Backend::Aarch64Pmull) {
            assert_eq!(
                Crc32Backend::selected(),
                backend,
                "crc32:{} is required but not selected",
                backend.name()
            );
        }
    }
}

/// Small independent-answer boundaries for wasm integer lowering: the u64
/// SHA-3 lanes and rotations, and digest block/padding length arithmetic.
/// Every algorithm keeps its complete differential corpus in the native suite.
fn integer_digest_lowering_matches_frozen_boundaries() {
    const LENGTHS: [usize; 20] = [
        0, 1, 55, 56, 57, 63, 64, 65, 71, 72, 73, 103, 104, 105, 135, 136, 137, 143, 144, 145,
    ];
    let mut md5 = Md5::new();
    let mut sha1 = purrdf_hash::sha1::Sha1::new();
    let mut sha3_224 = Sha3_224::new();
    let mut sha3_256 = Sha3_256::new();
    let mut sha3_384 = Sha3_384::new();
    let mut sha3_512 = Sha3_512::new();
    let mut crc32 = purrdf_hash::crc32::Crc32::new();
    let cases: [(&str, &str, &mut dyn Digest); 7] = [
        (
            "md5",
            include_str!("vectors/md5_differential_vectors.txt"),
            &mut md5,
        ),
        (
            "sha1",
            include_str!("vectors/sha1_differential_vectors.txt"),
            &mut sha1,
        ),
        (
            "sha3-224",
            include_str!("vectors/sha3_224_differential_vectors.txt"),
            &mut sha3_224,
        ),
        (
            "sha3-256",
            include_str!("vectors/sha3_256_differential_vectors.txt"),
            &mut sha3_256,
        ),
        (
            "sha3-384",
            include_str!("vectors/sha3_384_differential_vectors.txt"),
            &mut sha3_384,
        ),
        (
            "sha3-512",
            include_str!("vectors/sha3_512_differential_vectors.txt"),
            &mut sha3_512,
        ),
        (
            "crc32",
            include_str!("vectors/crc32_differential_vectors.txt"),
            &mut crc32,
        ),
    ];
    let mut checked = 0;
    for (name, text, hasher) in cases {
        let file = VectorFile::parse(text).expect("frozen vector integrity");
        for length in LENGTHS {
            // The frozen prefix records each length 0..=4096 exactly once,
            // with the high-word seed its header specifies.
            let record = file.records().get(length).expect("the boundary record");
            assert_eq!(record.fields[0], length.to_string());
            let seed = u64::from_str_radix(record.fields[1], 16).expect("the frozen seed");
            assert_eq!(seed, 0x7075_7272_6466_0000 + u64::try_from(length).unwrap());
            let data = xoshiro256_bytes(length, seed);
            hasher.reset();
            hasher.update(&data);
            let mut answer = [0_u8; purrdf_hash::MAX_OUTPUT_LEN];
            let answer_len = hasher.finalize_reset(&mut answer);
            let actual = one_shot_and_streamed(&answer[..answer_len], hasher, &data, length / 2);
            assert_eq!(actual, record.fields[2], "{name}, length {length}");
            checked += 1;
        }
    }
    assert_eq!(checked, 7 * LENGTHS.len());
    purrdf_testkit::harness::print_line(&format!(
        "digest integer lowering: {checked} frozen boundaries"
    ));
}

purrdf_testkit::harness_main!(
    integer_digest_lowering_matches_frozen_boundaries,
    md5_vectors_are_reproduced,
    required_sha1_and_crc32_paths_are_available_and_selected,
    sha1_vectors_are_reproduced_on_every_path,
    sha3_224_vectors_are_reproduced,
    sha3_256_vectors_are_reproduced,
    sha3_384_vectors_are_reproduced,
    sha3_512_vectors_are_reproduced,
    crc32_vectors_are_reproduced_on_every_path,
);
