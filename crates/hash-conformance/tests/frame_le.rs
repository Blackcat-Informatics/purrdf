// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Length framing: the frozen answers of the little-endian `u64` length
//! prefix, replayed on every target.
//!
//! `tests/vectors/frame_le_vectors.txt` holds, for each input, the eight-byte
//! prefix a framed field opens with and the BLAKE3 digest of the whole framed
//! field (the prefix, then the input). The inputs are structured, not only
//! random: the empty input, every byte on its own, every ordered pair drawn
//! from the escape-relevant byte classes, the UTF-8 of the named boundary
//! scalars, the repository IRI corpus, every length `0..=256` of the
//! SplitMix64 `next` stream's little-endian bytes, and fills of one byte at the
//! lengths where a narrower length field would carry into its next byte.
//!
//! The file was recorded from the construction the framing specification
//! states — `u64_le(len(field)) ‖ field` — and [`purrdf_hash::frame`] answers
//! from it through every entry point: [`frame_le`] into a buffer,
//! [`frame_le_into`] into a streaming digest (statically and through
//! `&mut dyn Digest`, whole and split across two frames), and
//! [`frame_be_labelled`], whose lengths are the same eight bytes in the other
//! order. To re-record after a deliberate change of construction, run this
//! target with `PURRDF_RECORD_FRAME_LE=1`.

use purrdf_hash::frame::{frame_be_labelled, frame_le, frame_le_into};
use purrdf_hash::mix::splitmix64_next;
use purrdf_hash::{Digest, blake3};
use purrdf_hash_conformance::iri_corpus;
use purrdf_testkit::vectors::{VectorFile, decode_bytes, encode_bytes};

/// The frozen framing vectors.
const FRAME_VECTORS: &str = include_str!("vectors/frame_le_vectors.txt");

/// The escape-relevant byte classes the pair inputs are drawn from: C0
/// controls and the whitespace a grammar splits on, the quoting and escaping
/// punctuation, DEL, C1 and UTF-8 lead and continuation bytes, the bytes that
/// never appear in UTF-8, and ordinary letters and path separators.
const CLASS_BYTES: [u8; 24] = [
    0x00, 0x09, 0x0A, 0x0D, 0x1F, 0x20, 0x22, 0x23, 0x27, 0x3C, 0x3E, 0x5C, 0x7F, 0x80, 0x9F, 0xA0,
    0xC2, 0xE2, 0xEF, 0xF4, 0xFE, 0xFF, 0x61, 0x2F,
];

/// Boundary scalars recorded on their own: C0 and C1 edges, the line and
/// paragraph separators, the byte-order mark, the two noncharacters at the
/// end of the BMP and the last scalar.
const BOUNDARY_SCALARS: [u32; 10] = [
    0x0000, 0x001F, 0x0080, 0x009F, 0x2028, 0x2029, 0xFEFF, 0xFFFE, 0xFFFF, 0x10_FFFF,
];

/// The fill lengths: either side of every carry of a one-, two- and
/// three-byte length field, and one mebibyte.
const FILL_LENGTHS: [usize; 10] = [
    255,
    256,
    257,
    65_535,
    65_536,
    65_537,
    16_777_215,
    16_777_216,
    16_777_217,
    1 << 20,
];

/// The fill bytes.
const FILL_BYTES: [u8; 2] = [0x00, 0xFF];

/// The framed field the specification states: the input's length as eight
/// little-endian bytes, then the input.
fn specified_frame(input: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(8 + input.len());
    framed.extend_from_slice(
        &u64::try_from(input.len())
            .expect("a slice length fits u64")
            .to_le_bytes(),
    );
    framed.extend_from_slice(input);
    framed
}

/// The first `length` bytes of the little-endian SplitMix64 `next` stream
/// from the seed `length`.
fn stream_bytes(length: usize) -> Vec<u8> {
    let mut state = length as u64;
    let mut bytes = Vec::with_capacity(length + 8);
    while bytes.len() < length {
        bytes.extend_from_slice(&splitmix64_next(&mut state).to_le_bytes());
    }
    bytes.truncate(length);
    bytes
}

/// Every input: its kind, its encoded field, and its bytes. A `fill` input is
/// spelled `BYTE*LENGTH` (the byte in hexadecimal, the length in decimal).
fn inputs() -> Vec<(&'static str, String, Vec<u8>)> {
    let mut inputs = Vec::new();
    let mut bytes = |kind: &'static str, data: Vec<u8>| {
        inputs.push((kind, encode_bytes(&data), data));
    };
    bytes("bytes", Vec::new());
    for byte in 0..=u8::MAX {
        bytes("bytes", vec![byte]);
    }
    for first in CLASS_BYTES {
        for second in CLASS_BYTES {
            bytes("bytes", vec![first, second]);
        }
    }
    for code in BOUNDARY_SCALARS {
        let scalar = char::from_u32(code).expect("a scalar value");
        bytes("bytes", scalar.to_string().into_bytes());
    }
    for iri in iri_corpus() {
        bytes("bytes", iri.into_bytes());
    }
    for length in 0..=256 {
        bytes("stream", stream_bytes(length));
    }
    for byte in FILL_BYTES {
        for length in FILL_LENGTHS {
            inputs.push((
                "fill",
                format!("{}*{length}", purrdf_hash::hex::encode(&[byte])),
                vec![byte; length],
            ));
        }
    }
    inputs
}

/// The bytes a record's `kind` and encoded `input` stand for.
fn decode_input(kind: &str, input: &str) -> Vec<u8> {
    match kind {
        "bytes" | "stream" => decode_bytes(input).expect("an encoded byte string"),
        "fill" => {
            let (byte, length) = input.split_once('*').expect("a BYTE*LENGTH fill");
            let byte = purrdf_hash::hex::decode(byte).expect("a hexadecimal byte");
            let length: usize = length.parse().expect("a decimal length");
            vec![byte[0]; length]
        }
        other => panic!("unknown framing input kind {other:?}"),
    }
}

/// The two answers for a framed field: its eight-byte prefix and the BLAKE3
/// digest of the whole framed field, both in lowercase hexadecimal.
fn answers(framed: &[u8]) -> Vec<String> {
    vec![
        purrdf_hash::hex::encode(&framed[..8]),
        purrdf_hash::hex::encode(blake3::hash(framed).as_bytes()),
    ]
}

/// The file answers the specified construction for every input, in order.
fn the_vectors_are_the_specified_construction() {
    let file = VectorFile::parse(FRAME_VECTORS).unwrap_or_else(|error| panic!("frame_le: {error}"));
    let replayed = file
        .replay(2, |fields| {
            answers(&specified_frame(&decode_input(fields[0], fields[1])))
        })
        .unwrap_or_else(|mismatch| panic!("frame_le: {mismatch}"));
    let expected = inputs();
    assert_eq!(replayed, expected.len(), "every input replayed");
    for (record, (kind, input, _)) in file.records().iter().zip(&expected) {
        assert_eq!(
            (record.fields[0], record.fields[1]),
            (*kind, input.as_str())
        );
    }
}

/// [`frame_le`] appends exactly the frozen framed field, after whatever the
/// buffer already holds.
fn frame_le_reproduces_the_vectors() {
    let file = VectorFile::parse(FRAME_VECTORS).unwrap_or_else(|error| panic!("frame_le: {error}"));
    let replayed = file
        .replay(2, |fields| {
            let input = decode_input(fields[0], fields[1]);
            let mut out = vec![0xA5];
            frame_le(&mut out, &input);
            assert_eq!(out[0], 0xA5, "the buffer's earlier bytes are kept");
            assert_eq!(out[9..], input[..], "the field follows its prefix");
            answers(&out[1..])
        })
        .unwrap_or_else(|mismatch| panic!("frame_le: {mismatch}"));
    assert_eq!(replayed, inputs().len());
}

/// [`frame_le_into`] streams exactly the frozen framed field into a digest,
/// through a generic and a `dyn` digest alike.
fn frame_le_into_reproduces_the_vectors() {
    let file =
        VectorFile::parse(FRAME_VECTORS).unwrap_or_else(|error| panic!("frame_le_into: {error}"));
    let replayed = file
        .replay(2, |fields| {
            let input = decode_input(fields[0], fields[1]);
            let mut hasher = blake3::Hasher::new();
            frame_le_into(&mut hasher, &input);
            let mut through_dyn = blake3::RecordHasher::new();
            frame_le_into(&mut through_dyn as &mut dyn Digest, &input);
            let digest = hasher.finalize();
            assert_eq!(
                digest,
                through_dyn.finalize(),
                "a dyn digest absorbs the same bytes"
            );
            let mut framed = Vec::new();
            frame_le(&mut framed, &input);
            vec![
                purrdf_hash::hex::encode(&framed[..8]),
                purrdf_hash::hex::encode(digest.as_bytes()),
            ]
        })
        .unwrap_or_else(|mismatch| panic!("frame_le_into: {mismatch}"));
    assert_eq!(replayed, inputs().len());
}

/// Two streamed frames digest the two appended frames: the framing composes,
/// and a split of the same bytes into two fields digests differently.
fn streamed_frames_compose() {
    for (_, _, input) in inputs().iter().filter(|(kind, _, _)| *kind != "fill") {
        let split = input.len() / 2;
        let (head, tail) = input.split_at(split);
        let mut streamed = blake3::RecordHasher::new();
        frame_le_into(&mut streamed, head);
        frame_le_into(&mut streamed, tail);
        let mut appended = Vec::new();
        frame_le(&mut appended, head);
        frame_le(&mut appended, tail);
        assert_eq!(streamed.finalize(), blake3::hash(&appended));
        if !input.is_empty() {
            let mut whole = Vec::new();
            frame_le(&mut whole, input);
            frame_le(&mut whole, &[]);
            assert_ne!(whole, appended, "a field boundary moves the framed bytes");
        }
    }
}

/// [`frame_be_labelled`] frames its label and its value with the frozen
/// lengths, read big-endian.
fn frame_be_labelled_reproduces_the_vectors() {
    let file = VectorFile::parse(FRAME_VECTORS)
        .unwrap_or_else(|error| panic!("frame_be_labelled: {error}"));
    for record in file.records() {
        let input = decode_input(record.fields[0], record.fields[1]);
        let Ok(label) = core::str::from_utf8(&input) else {
            continue;
        };
        let mut prefix = purrdf_hash::hex::decode(record.fields[2]).expect("a hexadecimal prefix");
        prefix.reverse();
        let value: Vec<u8> = input.iter().rev().copied().collect();
        let mut out = Vec::new();
        frame_be_labelled(&mut out, label, &value);
        let mut expected = prefix.clone();
        expected.extend_from_slice(&input);
        expected.extend_from_slice(&prefix);
        expected.extend_from_slice(&value);
        assert_eq!(out, expected, "a {}-byte label and value", input.len());
    }
}

/// Writes the vector file instead of replaying it, when asked to.
#[cfg(not(target_arch = "wasm32"))]
fn record_vectors_when_asked() {
    if std::env::var_os("PURRDF_RECORD_FRAME_LE").as_deref() != Some("1".as_ref()) {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
    let mut recorder = purrdf_testkit::vectors::Recorder::new();
    for comment in [
        "Length framing: the frozen answers of u64_le(len(field)) || field.",
        "",
        "Answers: the construction the framing specification states, recorded by",
        "`tests/frame_le.rs` with PURRDF_RECORD_FRAME_LE=1. Every framing in the",
        "workspace answers from it, so these are the bytes each caller's preimage",
        "and wire encoding carries.",
        "",
        "Inputs, by kind:",
        "  bytes   the encoded byte string: the empty input, every byte, every",
        "          ordered pair of 24 escape-relevant bytes, ten boundary scalars'",
        "          UTF-8, and the 1,000 IRIs of corpus_iris.txt",
        "  stream  the first N bytes (N = 0..=256) of the little-endian",
        "          splitmix64_next stream from the seed N, encoded",
        "  fill    BYTE*LENGTH: LENGTH copies of the hexadecimal BYTE, at either",
        "          side of each carry of a one-, two- and three-byte length and",
        "          at one mebibyte",
        "",
        "Fields: kind, input, the eight-byte prefix (16 lowercase hex digits), and",
        "the BLAKE3-256 digest of the framed field (64 lowercase hex digits).",
    ] {
        recorder.comment(comment).expect("a one-line comment");
    }
    recorder
        .header("oracle", "the specified construction (self-recorded)")
        .expect("an oracle header");
    for (kind, input, data) in inputs() {
        let mut record = vec![kind.to_owned(), input];
        record.extend(answers(&specified_frame(&data)));
        recorder.record(&record).expect("an encodable record");
    }
    std::fs::write(dir.join("frame_le_vectors.txt"), recorder.render())
        .expect("the vector file is written");
    purrdf_testkit::harness::print_line("recorded the length-framing vectors");
}

purrdf_testkit::harness_main!(
    #[cfg(not(target_arch = "wasm32"))]
    record_vectors_when_asked,
    the_vectors_are_the_specified_construction,
    frame_le_reproduces_the_vectors,
    frame_le_into_reproduces_the_vectors,
    streamed_frames_compose,
    frame_be_labelled_reproduces_the_vectors,
);
