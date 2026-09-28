// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen public-API oracle answers, replayed with unrelated update boundaries.
use purrdf_hash::blake3::{Backend, Hasher, hash};
use purrdf_hash::hex::Lower;
use purrdf_testkit::vectors::VectorFile;

const VECTORS: &str = include_str!("vectors/blake3_differential_vectors.txt");

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
    for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
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
    for backend in core::iter::once(None).chain(
        Backend::ALL
            .into_iter()
            .filter(|b| b.is_available())
            .map(Some),
    ) {
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
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                *byte = state as u8;
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
            for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
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
// The full corpus above remains the native and wasm conformance requirement.
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
    for backend in Backend::ALL.into_iter().filter(|b| b.is_available()) {
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

purrdf_testkit::harness_main!(
    caller_scheduled_trees_match_frozen_answers,
    random_inputs_cover_irregular_trees_and_alignment,
    streaming_boundary_answers,
    frozen_answers_match,
    streaming_answers_match,
    snapshots_and_reset_preserve_message_boundaries,
);
