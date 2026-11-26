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
//! Native Rust runs this complete frozen corpus on the shared harness.
//! WASM execution is reserved for actual target dispatch and SIMD kernels.

use purrdf_hash::backend::{Crc32Backend, Sha1Backend};
use purrdf_hash::dispatch::{assert_required_available, host_advertises};
use purrdf_hash::hex::{decode_canonical, encode};
use purrdf_hash::md5::Md5;
use purrdf_hash::sha2::{
    Sha224, Sha256, Sha256Backend, Sha256Batch, Sha256BatchBackend, Sha384, Sha512, Sha512Backend,
};
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

fn sha2_selected_answers(data: &[u8]) -> [Vec<u8>; 4] {
    [
        Sha224::digest(data).to_vec(),
        Sha256::digest(data).to_vec(),
        Sha384::digest(data).to_vec(),
        Sha512::digest(data).to_vec(),
    ]
}

fn sha2_portable_dispatch_preserves_the_frozen_boundary() {
    let file = VectorFile::parse(include_str!("vectors/sha2_differential_vectors.txt"))
        .expect("frozen vector checksum");
    let [length, seed, a, b, c, d] = file.records()[129].fields.as_slice() else {
        panic!("SHA-2 vectors contain length, seed and four digests");
    };
    assert_eq!(*length, "129");
    let seed = u64::from_str_radix(seed, 16).expect("seed in hex");
    let data = xoshiro256_bytes(129, seed);
    let answers = [a, b, c, d].map(|field| decode_canonical(field).expect("frozen digest"));
    assert_eq!(sha2_selected_answers(&data), answers);
    if cfg!(target_arch = "wasm32") {
        assert!(Sha256Backend::all_available().eq([Sha256Backend::Portable]));
        assert!(Sha512Backend::all_available().eq([Sha512Backend::Portable]));
        assert!(!Sha256Batch::is_parallel());
    }
}

fn sha2_vectors_are_reproduced_on_every_path() {
    assert_required_available("sha256", |path| match path {
        Sha256Backend::Portable => true,
        Sha256Backend::X86ShaAvx2 => {
            cfg!(target_arch = "x86_64") && host_advertises(&["sha_ni", "ssse3", "avx2"])
        }
        Sha256Backend::Aarch64Sha2 => {
            cfg!(all(target_arch = "aarch64", target_endian = "little"))
                && host_advertises(&["sha2", "asimd"])
        }
    });
    assert_required_available("sha512", |path| match path {
        Sha512Backend::Portable => true,
        Sha512Backend::X86Sha512 => {
            cfg!(target_arch = "x86_64") && host_advertises(&["sha512", "avx2"])
        }
        Sha512Backend::X86Avx512Vl => {
            cfg!(target_arch = "x86_64")
                && host_advertises(&["avx512f", "avx512vl", "bmi2", "ssse3", "sse4_1"])
        }
        Sha512Backend::Aarch64Sha512 => {
            cfg!(all(target_arch = "aarch64", target_endian = "little"))
                && host_advertises(&["sha512", "sha3", "asimd"])
        }
    });

    let mut streams: Vec<(usize, &'static str, Box<dyn Digest>)> = Vec::new();
    for path in Sha256Backend::all_available() {
        streams.push((
            0,
            path.name(),
            Box::new(Sha224::with_backend(path).expect("available")),
        ));
        streams.push((
            1,
            path.name(),
            Box::new(Sha256::with_backend(path).expect("available")),
        ));
    }
    for path in Sha512Backend::all_available() {
        streams.push((
            2,
            path.name(),
            Box::new(Sha384::with_backend(path).expect("available")),
        ));
        streams.push((
            3,
            path.name(),
            Box::new(Sha512::with_backend(path).expect("available")),
        ));
    }
    let file = VectorFile::parse(include_str!("vectors/sha2_differential_vectors.txt"))
        .expect("frozen vector checksum");
    assert_eq!(file.records().len(), 258);
    for (length, record) in (0..=256).chain([1 << 20]).zip(file.records()) {
        let [len, seed, a, b, c, d] = record.fields.as_slice() else {
            panic!("SHA-2 vectors contain length, seed and four digests");
        };
        assert_eq!(length, len.parse().expect("decimal length"));
        let seed = u64::from_str_radix(seed, 16).expect("seed in hex");
        let data = xoshiro256_bytes(length, seed);
        let answers = [a, b, c, d].map(|field| decode_canonical(field).expect("frozen digest"));
        assert_eq!(
            sha2_selected_answers(&data),
            answers,
            "selected one-shot at {length} bytes"
        );
        let last_split = if length <= 256 { length.min(129) } else { 0 };
        for (index, name, hasher) in &mut streams {
            for split in 0..=last_split {
                hasher.update(&data[..split]);
                hasher.update(&[]);
                hasher.update(&data[split..]);
                let mut out = [0xa5; purrdf_hash::MAX_OUTPUT_LEN];
                let written = hasher.finalize_reset(&mut out);
                assert_eq!(
                    &out[..written],
                    &answers[*index],
                    "{name} at {length} bytes split at {split}"
                );
                assert!(out[written..].iter().all(|&byte| byte == 0xa5));
            }
        }
    }
}

fn sha256_batch_keeps_lane_order_and_reuses_the_accepted_boundary() {
    assert_required_available("sha256-batch", |path| match path {
        Sha256BatchBackend::Portable => true,
        Sha256BatchBackend::X86Avx512 => {
            cfg!(target_arch = "x86_64") && host_advertises(&["avx512f", "avx512bw"])
        }
    });

    for path in Sha256BatchBackend::all_available() {
        check_sha256_batch(|batch| batch.finish_reset_with_backend(path).expect("available"));
    }
    check_sha256_batch(Sha256Batch::finish_reset);
}

fn check_sha256_batch(
    mut drain: impl FnMut(&mut Sha256Batch) -> ([[u8; 32]; Sha256Batch::CAPACITY], usize),
) {
    let mut batch = Sha256Batch::new();
    let expected = core::array::from_fn(|lane| {
        let data = xoshiro256_bytes(lane * 32, lane as u64);
        let spans: [_; 2] = data.split_at(data.len() / 2).into();
        assert!(batch.push(spans));
        Sha256::digest(&data)
    });
    assert!(!batch.push([b"full".as_slice()]));
    assert_eq!(drain(&mut batch), (expected, Sha256Batch::CAPACITY));

    let too_long = [0xc3; Sha256Batch::MAX_INPUT_LEN + 1];
    let (left, right) = too_long.split_at(301);
    assert!(!batch.push([left, right]));
    assert!(batch.push([left, &right[..right.len() - 1]]));
    let (result, count) = drain(&mut batch);
    assert_eq!(count, 1);
    assert_eq!(result[0], Sha256::digest(&too_long[..too_long.len() - 1]));
}

purrdf_testkit::harness_main!(
    md5_vectors_are_reproduced,
    required_sha1_and_crc32_paths_are_available_and_selected,
    sha1_vectors_are_reproduced_on_every_path,
    sha2_vectors_are_reproduced_on_every_path,
    sha2_portable_dispatch_preserves_the_frozen_boundary,
    sha256_batch_keeps_lane_order_and_reuses_the_accepted_boundary,
    sha3_224_vectors_are_reproduced,
    sha3_256_vectors_are_reproduced,
    sha3_384_vectors_are_reproduced,
    sha3_512_vectors_are_reproduced,
    crc32_vectors_are_reproduced_on_every_path,
);
