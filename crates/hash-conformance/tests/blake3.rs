// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen public-API oracle answers, replayed with unrelated update boundaries.
use purrdf_hash::Backend as _;
use purrdf_hash::blake3::{Backend, Hasher, hash};
use purrdf_hash::dispatch::{assert_required_available, host_advertises};
use purrdf_hash::hex::Lower;
use purrdf_testkit::vectors::VectorFile;

const VECTORS: &str = include_str!("vectors/blake3_differential_vectors.txt");

fn required_backends_are_available() {
    assert!(Backend::Portable.is_available());
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    assert_eq!(Backend::selected(), Backend::Wasm128);
    #[cfg(all(target_arch = "wasm32", not(target_feature = "simd128")))]
    assert_eq!(Backend::selected(), Backend::Portable);
    #[cfg(all(
        target_arch = "aarch64",
        target_feature = "neon",
        target_endian = "little"
    ))]
    assert_eq!(Backend::selected(), Backend::Neon);
    #[cfg(target_arch = "x86_64")]
    assert!(Backend::Sse2.is_available());

    assert_required_available("blake3", expected_here);
}

/// Whether this host is expected to run a BLAKE3 path: its architecture, and
/// the processor features it advertises independently of the detection under
/// test.
fn expected_here(backend: Backend) -> bool {
    let x86_64 = cfg!(target_arch = "x86_64");
    match backend {
        Backend::Portable => true,
        Backend::Sse2 => x86_64 || (cfg!(target_arch = "x86") && host_advertises(&["sse2"])),
        Backend::Ssse3 => x86_64 && host_advertises(&["ssse3"]),
        Backend::Avx2 => x86_64 && host_advertises(&["avx2"]),
        Backend::Avx512 => x86_64 && host_advertises(&["avx512f", "avx512vl"]),
        Backend::Neon => cfg!(all(
            target_arch = "aarch64",
            target_feature = "neon",
            target_endian = "little"
        )),
        Backend::Wasm128 => cfg!(all(target_arch = "wasm32", target_feature = "simd128")),
    }
}

fn corpus() -> Vec<u8> {
    (0..=1_048_576).map(|i| (i % 251) as u8).collect()
}

fn frozen_answers_match() {
    let data = corpus();
    let file = VectorFile::parse(VECTORS).expect("frozen vector integrity");
    let count = file
        .replay(1, |fields| {
            let len: usize = fields[0].parse().expect("length");
            vec![Lower(hash(&data[..len]).as_bytes()).to_string()]
        })
        .expect("default dispatch matches independent answers");
    assert_eq!(count, 8214);
    for backend in Backend::all_available() {
        let count = file
            .replay(1, |fields| {
                let len: usize = fields[0].parse().expect("length");
                vec![
                    Lower(
                        backend
                            .hash(&data[..len])
                            .expect("available path")
                            .as_bytes(),
                    )
                    .to_string(),
                ]
            })
            .unwrap_or_else(|error| panic!("{backend:?}: {error:?}"));
        assert_eq!(count, 8214);
    }
}

fn streaming_answers_match() {
    replay_stream::<16384>();
    replay_stream::<1024>();
}

fn replay_stream<const BUFFER: usize>() {
    let data = corpus();
    let file = VectorFile::parse(VECTORS).expect("frozen vector integrity");
    for backend in core::iter::once(None).chain(Backend::all_available().map(Some)) {
        let mut h = backend.map_or_else(purrdf_hash::blake3::Streaming::<BUFFER>::new, |b| {
            b.streaming::<BUFFER>().expect("available path")
        });
        for widths in [&[1, 63, 65, 1023, 1025, 16383, 16385][..], &[65536][..]] {
            let count = file
                .replay(1, |fields| {
                    let len: usize = fields[0].parse().expect("length");
                    h.reset();
                    let mut offset = 0;
                    let mut step = 0;
                    while offset < len {
                        let end = (offset + widths[step % widths.len()]).min(len);
                        h.update(&data[offset..end]);
                        h.update(&[]);
                        offset = end;
                        step += 1;
                    }
                    let first = h.finalize();
                    assert_eq!(first, h.finalize(), "finalize preserves state");
                    vec![Lower(first.as_bytes()).to_string()]
                })
                .unwrap_or_else(|error| panic!("{backend:?}, buffer={BUFFER}: {error:?}"));
            assert_eq!(count, 8214);
        }
    }
}

fn snapshots_and_reset_preserve_message_boundaries() {
    let data = corpus();
    let mut h = Hasher::new();
    for len in [
        0, 1, 63, 64, 65, 1023, 1024, 1025, 16383, 16384, 16385, 65536,
    ] {
        h.reset();
        h.update(&data[..len]);
        let prefix = h.finalize();
        assert_eq!(prefix, hash(&data[..len]));
        let mut fork = h.clone();
        h.update(&data[len..=len]);
        assert_eq!(h.finalize(), hash(&data[..=len]));
        assert_eq!(fork.finalize(), prefix);
        fork.update(&data[len..len + 65]);
        assert_eq!(fork.finalize(), hash(&data[..len + 65]));
        h.reset();
        h.update(b"abc");
        assert_eq!(h.finalize(), hash(b"abc"));
        h.reset();
        assert_eq!(h.finalize(), hash(b""));
    }
}

fn random_inputs_cover_irregular_trees_and_alignment() {
    let file = VectorFile::parse(include_str!("vectors/blake3_random_vectors.txt"))
        .expect("frozen vector integrity");
    let count = file
        .replay(2, |fields| {
            let len: usize = fields[0].parse().expect("length");
            let seed = u64::from_str_radix(fields[1], 16).expect("seed");
            let offset = (seed % 32) as usize;
            let mut buffer = vec![0; offset + len];
            let mut state = seed;
            for byte in &mut buffer[offset..] {
                *byte = purrdf_testkit::rng::xorshift64_next(&mut state) as u8;
            }
            let data = &buffer[offset..];
            let expected = Backend::Portable
                .hash(data)
                .expect("portable always exists");
            assert_eq!(
                hash(data),
                expected,
                "default dispatch, len={len}, seed={seed:x}"
            );
            for backend in Backend::all_available() {
                assert_eq!(
                    backend.hash(data).expect("available path"),
                    expected,
                    "{backend:?}, len={len}, seed={seed:x}"
                );
                let mut stream = backend.hasher().expect("available path");
                let mut position = 0;
                let mut step = 0;
                let widths = [
                    (seed % 1025) as usize + 1,
                    ((seed >> 16) % 16385) as usize + 1,
                ];
                while position < len {
                    let end = (position + widths[step % 2]).min(len);
                    std::io::Write::write_all(&mut stream, &data[position..end])
                        .expect("infallible in-memory hash writer");
                    std::io::Write::flush(&mut stream).expect("flush preserves the message");
                    position = end;
                    step += 1;
                }
                assert_eq!(
                    stream.finalize(),
                    expected,
                    "stream {backend:?}, len={len}, seed={seed:x}"
                );
            }
            vec![Lower(expected.as_bytes()).to_string()]
        })
        .expect("independent random oracle answers");
    assert_eq!(count, 10000);
}

// A small independent-answer slice of the frozen corpus, suitable for Miri.
// The full frozen corpus above remains the native conformance requirement.
fn streaming_boundary_answers() {
    let data: Vec<u8> = (0..16385).map(|i| (i % 251) as u8).collect();
    let cases = [
        (
            0usize,
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        ),
        (
            65usize,
            "de1e5fa0be70df6d2be8fffd0e99ceaa8eb6e8c93a63f2d8d1c30ecb6b263dee",
        ),
        (
            1025usize,
            "d00278ae47eb27b34faecf67b4fe263f82d5412916c1ffd97c8cb7fb814b8444",
        ),
        (
            16383usize,
            "7529418ecb789a30254899f229522ccde05234d2019c5ec5072bc4685a57658d",
        ),
        (
            16384usize,
            "f875d6646de28985646f34ee13be9a576fd515f76b5b0a26bb324735041ddde4",
        ),
        (
            16385usize,
            "1dabe216be2578830263b049de1639f39f05a4da616b9b78c7a5e4e41662fd1f",
        ),
    ];
    for backend in Backend::all_available() {
        for (len, expected) in cases {
            let mut stream = backend.hasher().expect("available path");
            for part in data[..len].chunks(63) {
                stream.update(part);
            }
            assert_eq!(
                Lower(stream.finalize().as_bytes()).to_string(),
                expected,
                "{backend:?}, len={len}"
            );
            let mut record = backend.record_hasher().expect("available path");
            for part in data[..len].chunks(65) {
                record.update(part);
            }
            assert_eq!(
                Lower(record.finalize().as_bytes()).to_string(),
                expected,
                "{backend:?}, len={len}"
            );
            stream.reset();
            record.reset();
            assert_eq!(stream.finalize(), record.finalize());
        }
    }
}

fn caller_scheduled_trees_match_frozen_answers() {
    use purrdf_hash::blake3::{Join, hash_with_join};
    struct Inline;
    impl Join for Inline {
        fn join<A: Send, B: Send>(
            &self,
            left: impl FnOnce() -> A + Send,
            right: impl FnOnce() -> B + Send,
        ) -> (A, B) {
            (left(), right())
        }
    }
    let data = corpus();
    let file = VectorFile::parse(VECTORS).expect("frozen vector integrity");
    let count = file
        .replay(1, |fields| {
            let len: usize = fields[0].parse().expect("length");
            for grain in [0, 2048, 16384, 131_072, usize::MAX] {
                assert_eq!(
                    hash_with_join(&data[..len], grain, &Inline),
                    hash(&data[..len])
                );
            }
            vec![Lower(hash_with_join(&data[..len], 2048, &Inline).as_bytes()).to_string()]
        })
        .expect("caller-scheduled frozen answers");
    assert_eq!(count, 8214);
}

const OFFICIAL: &str = include_str!("vectors/blake3_official_vectors.txt");
const OFFICIAL_KEY: &[u8; 32] = b"whats the Elvish word for friend";
const OFFICIAL_CONTEXT: &str = "BLAKE3 2019-12-27 16:29:52 test vectors context";

fn official_modes_and_extended_output() {
    use purrdf_hash::blake3::{derive_key, derive_key_xof, hash_xof, keyed_hash, keyed_hash_xof};
    let data = corpus();
    let vectors = VectorFile::load(OFFICIAL);
    assert_eq!(vectors.records().len(), 35);
    let count = vectors
        .replay(1, |fields| {
            let input = &data[..fields[0].parse::<usize>().expect("input length")];
            let mut answers = [[0; 131]; 3];
            hash_xof(input, &mut answers[0]);
            keyed_hash_xof(OFFICIAL_KEY, input, &mut answers[1]);
            derive_key_xof(OFFICIAL_CONTEXT, input, &mut answers[2]);
            assert_eq!(hash(input).as_bytes(), &answers[0][..32]);
            assert_eq!(
                keyed_hash(OFFICIAL_KEY, input).as_bytes(),
                &answers[1][..32]
            );
            assert_eq!(derive_key(OFFICIAL_CONTEXT, input), answers[2][..32]);
            answers
                .iter()
                .map(|bytes| Lower(bytes).to_string())
                .collect()
        })
        .expect("official full output, default dispatch");
    assert_eq!(count, 35);
    replay_official_streams::<1024>(&vectors, &data);
    replay_official_streams::<16384>(&vectors, &data);
}

fn replay_official_streams<const BUFFER: usize>(vectors: &VectorFile<'_>, data: &[u8]) {
    use purrdf_hash::blake3::Streaming;
    for backend in Backend::all_available() {
        let count = vectors
            .replay(1, |fields| {
                let input = &data[..fields[0].parse::<usize>().expect("input length")];
                let mut answers = Vec::new();
                for mode in 0..3 {
                    let mut stream = match mode {
                        0 => backend.streaming::<BUFFER>(),
                        1 => backend.keyed_streaming::<BUFFER>(OFFICIAL_KEY),
                        _ => backend.derive_key_streaming::<BUFFER>(OFFICIAL_CONTEXT),
                    }
                    .expect("available backend");
                    let mut default = match mode {
                        0 => Streaming::<BUFFER>::new(),
                        1 => Streaming::<BUFFER>::new_keyed(OFFICIAL_KEY),
                        _ => Streaming::<BUFFER>::new_derive_key(OFFICIAL_CONTEXT),
                    };
                    // Whole-input updates exercise the existing batch/subtree paths;
                    // subsequent passes cross unrelated compression/buffer boundaries.
                    stream.update(input);
                    let mut expected = [0; 131];
                    stream.finalize_xof().fill(&mut expected);
                    for grain in [63, 2053] {
                        stream.reset();
                        for part in input.chunks(grain) {
                            stream.update(part);
                        }
                        assert_eq!(stream.finalize().as_bytes(), &expected[..32]);
                        let snapshot = stream.clone();
                        let mut reader = snapshot.finalize_xof();
                        let mut actual = [0; 131];
                        reader.fill(&mut actual[..31]);
                        reader.fill(&mut []);
                        reader.fill(&mut actual[31..65]);
                        reader.fill(&mut actual[65..]);
                        assert_eq!(
                            actual, expected,
                            "{backend:?}, mode={mode}, buffer={BUFFER}"
                        );
                        for position in [0, 31, 32, 63, 64, 65, 127, 128, 131] {
                            reader.set_position(position);
                            let mut tail = vec![0; 131 - position as usize];
                            reader.clone().fill(&mut tail);
                            assert_eq!(tail, expected[position as usize..]);
                            assert_eq!(reader.position(), position);
                        }
                    }
                    default.update(input);
                    assert_eq!(default.finalize(), stream.finalize());
                    let mut default_output = [0; 131];
                    default.finalize_xof().fill(&mut default_output);
                    assert_eq!(default_output, expected);
                    answers.push(Lower(&expected).to_string());
                }
                answers
            })
            .unwrap_or_else(|error| panic!("{backend:?}, buffer={BUFFER}: {error:?}"));
        assert_eq!(count, 35);
    }
}

fn derived_context_trees_match_across_backends() {
    use purrdf_hash::blake3::{derive_key, derive_key_xof};
    let context = "应用-specific context🦀".repeat(2049);
    let data = corpus();
    let mut portable = Backend::Portable
        .derive_key_streaming::<1024>(&context)
        .expect("portable");
    for length in [1025, 16_384, 262_145, 1_048_576] {
        portable.reset();
        portable.update(&data[..length]);
        let mut expected = [0; 197];
        portable.finalize_xof().fill(&mut expected);
        assert_eq!(derive_key(&context, &data[..length]), expected[..32]);
        let mut one_shot = [0; 197];
        derive_key_xof(&context, &data[..length], &mut one_shot);
        assert_eq!(one_shot, expected);
        for backend in Backend::all_available() {
            let mut stream = backend
                .derive_key_streaming::<16384>(&context)
                .expect("available");
            for part in data[..length].chunks(997) {
                stream.update(part);
            }
            let mut actual = [0; 197];
            stream.finalize_xof().fill(&mut actual);
            assert_eq!(actual, expected, "{backend:?}, length={length}");
        }
    }
}

fn xof_large_output_and_seek_boundaries() {
    use std::io::Read as _;
    let data = corpus();
    for mode in 0..3 {
        let mut portable = match mode {
            0 => Backend::Portable.hasher(),
            1 => Backend::Portable.keyed_streaming(OFFICIAL_KEY),
            _ => Backend::Portable.derive_key_streaming(OFFICIAL_CONTEXT),
        }
        .expect("portable");
        portable.update(&data[..262_145]);
        let mut expected = vec![0; 1_048_579];
        portable.finalize_xof().fill(&mut expected);
        for backend in Backend::all_available() {
            let mut stream = match mode {
                0 => backend.hasher(),
                1 => backend.keyed_streaming(OFFICIAL_KEY),
                _ => backend.derive_key_streaming(OFFICIAL_CONTEXT),
            }
            .expect("available backend");
            stream.update(&data[..262_145]);
            let mut reader = stream.finalize_xof();
            let mut actual = vec![0; expected.len()];
            for part in actual.chunks_mut(4093) {
                reader.read_exact(part).expect("read output");
            }
            assert_eq!(actual, expected, "{backend:?}, mode={mode}");
            reader.set_position(u64::MAX - 1);
            assert_eq!(reader.read(&mut [0; 2]).expect("last byte"), 1);
            assert_eq!(reader.position(), u64::MAX);
            assert_eq!(reader.read(&mut [0; 1]).expect("end of output"), 0);
            reader.fill(&mut []);
            for position in [(1_u64 << 38) - 1, 1_u64 << 38, u64::MAX - 128] {
                let mut oracle = portable.finalize_xof();
                oracle.set_position(position);
                reader.set_position(position);
                let mut want = [0; 127];
                let mut got = [0; 127];
                oracle.fill(&mut want);
                reader.fill(&mut got);
                assert_eq!(got, want);
            }
        }
    }
}

purrdf_testkit::harness_main!(
    derived_context_trees_match_across_backends,
    official_modes_and_extended_output,
    xof_large_output_and_seek_boundaries,
    required_backends_are_available,
    caller_scheduled_trees_match_frozen_answers,
    random_inputs_cover_irregular_trees_and_alignment,
    streaming_boundary_answers,
    frozen_answers_match,
    streaming_answers_match,
    snapshots_and_reset_preserve_message_boundaries,
);
