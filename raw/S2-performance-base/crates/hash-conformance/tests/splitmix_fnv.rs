// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SplitMix64 and FNV-1a 64-bit: their frozen answers, replayed on every
//! target.
//!
//! # SplitMix64
//!
//! `tests/vectors/splitmix64_differential_vectors.txt` holds 10,000 draws per
//! seed of each of the three streams [`purrdf_hash::mix`] defines:
//!
//! * `next` — the published generator, [`splitmix64_next`] over a counter
//!   starting at the seed;
//! * `step` — the self-composed stream, [`splitmix64_step`] fed its own
//!   output, starting at the seed;
//! * `finalize` — [`splitmix64_finalize`] of `seed + index` (wrapping).
//!
//! Each record carries sixteen consecutive draws: stream, seed and the index
//! of the first draw, then the sixteen answers.
//!
//! # FNV-1a
//!
//! `tests/vectors/fnv1a64_differential_vectors.txt` holds, for each input,
//! three answers: [`fnv1a64`] of the input, [`fold`] of the input from the
//! state `0x10000001` (a caller-seeded fold), and [`fnv1a64`] of the input
//! followed by one `0xFF` separator byte (a sequence digest's element). The
//! inputs are structured, not only random: the empty input, every byte on its
//! own, every ordered pair drawn from the escape-relevant byte classes, the
//! UTF-8 of every Unicode scalar value (in blocks of 256 consecutive scalars,
//! the surrogate blocks excluded), the named boundary scalars on their own,
//! the repository IRI corpus, and every length `0..=256` of the SplitMix64
//! `next` stream's little-endian bytes.
//!
//! The oracle is purrdf-hash itself, so a replay proves stability across
//! targets, compilers and later edits; the reference test values in
//! `purrdf_hash::fnv` and `purrdf_hash::mix` prove correctness. To re-record
//! after a deliberate change of function, run this target with
//! `PURRDF_RECORD_MIX_FNV=1`.

use purrdf_hash::fnv::{BASIS, fnv1a64, fold};
use purrdf_hash::mix::{splitmix64_finalize, splitmix64_next, splitmix64_step};
use purrdf_hash_conformance::iri_corpus;
use purrdf_testkit::rng::splitmix64_bytes;
use purrdf_testkit::vectors::{VectorFile, decode_bytes, encode_bytes};

/// Draws recorded per stream and seed.
const DRAWS: u64 = 10_000;
/// Draws per record.
const PER_RECORD: u64 = 16;
/// The seeds every stream is recorded from: zero, one, a small arbitrary seed
/// and the all-ones word, so the counter wraps within the first draws.
const SEEDS: [u64; 4] = [0, 1, 0xC057, u64::MAX];
/// The state the seeded-fold answer starts from.
const SEEDED_FOLD_STATE: u64 = 0x1000_0001;
/// The separator byte the separated answer appends.
const SEPARATOR: u8 = 0xFF;

// --- SplitMix64 ----------------------------------------------------------------

/// The `count` draws of `stream` from `seed`, starting at draw `first`.
fn draws(stream: &str, seed: u64, first: u64, count: u64) -> Vec<u64> {
    match stream {
        "next" => {
            let mut state = seed;
            for _ in 0..first {
                let _ = splitmix64_next(&mut state);
            }
            (0..count).map(|_| splitmix64_next(&mut state)).collect()
        }
        "step" => {
            let mut state = seed;
            for _ in 0..first {
                state = splitmix64_step(state);
            }
            (0..count)
                .map(|_| {
                    state = splitmix64_step(state);
                    state
                })
                .collect()
        }
        "finalize" => (first..first + count)
            .map(|index| splitmix64_finalize(seed.wrapping_add(index)))
            .collect(),
        other => panic!("unknown SplitMix64 stream {other:?}"),
    }
}

/// The streams in record order.
const STREAMS: [&str; 3] = ["next", "step", "finalize"];

/// Every SplitMix64 record: stream, seed, first index, then the draws.
fn splitmix_records() -> Vec<Vec<String>> {
    let mut records = Vec::new();
    for stream in STREAMS {
        for seed in SEEDS {
            let all = draws(stream, seed, 0, DRAWS);
            for (chunk, values) in all.chunks(PER_RECORD as usize).enumerate() {
                let mut record = vec![
                    stream.to_owned(),
                    format!("{seed:016x}"),
                    (chunk as u64 * PER_RECORD).to_string(),
                ];
                record.extend(values.iter().map(|value| format!("{value:016x}")));
                records.push(record);
            }
        }
    }
    records
}

fn splitmix64_vectors_are_reproduced() {
    let file = VectorFile::parse(include_str!("vectors/splitmix64_differential_vectors.txt"))
        .unwrap_or_else(|error| panic!("splitmix64: {error:?}"));
    // Each record is answered from scratch by re-running its stream up to its
    // first index, so a record is independent of the records before it; the
    // cumulative run below is checked against the same file.
    let replayed = file
        .replay(3, |fields| {
            let seed = u64::from_str_radix(fields[1], 16).expect("a hexadecimal seed");
            let first: u64 = fields[2].parse().expect("a decimal index");
            draws(fields[0], seed, first, PER_RECORD)
                .iter()
                .map(|value| format!("{value:016x}"))
                .collect()
        })
        .unwrap_or_else(|mismatch| panic!("splitmix64: {mismatch}"));
    let expected = STREAMS.len() as u64 * SEEDS.len() as u64 * DRAWS.div_ceil(PER_RECORD);
    assert_eq!(replayed as u64, expected, "every record replayed");
    let body: Vec<Vec<&str>> = file.records().iter().map(|r| r.fields.clone()).collect();
    let computed = splitmix_records();
    assert_eq!(body.len(), computed.len());
    for (frozen, fresh) in body.iter().zip(&computed) {
        assert_eq!(frozen, fresh, "the cumulative streams agree with the file");
    }
}

// --- FNV-1a -----------------------------------------------------------------

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

/// The UTF-8 of every scalar value in `first..=last`, in order.
fn scalar_block(first: u32, last: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4];
    for code in first..=last {
        if let Some(scalar) = char::from_u32(code) {
            bytes.extend_from_slice(scalar.encode_utf8(&mut buffer).as_bytes());
        }
    }
    bytes
}

/// Every FNV-1a input: its kind, its encoded field, and its bytes. A
/// `scalars` input is spelled `FIRST-LAST` in hexadecimal and stands for the
/// UTF-8 of every scalar in that range; every other input is its bytes.
fn fnv_inputs() -> Vec<(&'static str, String, Vec<u8>)> {
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
        bytes("stream", splitmix64_bytes(length, length as u64));
    }
    for block in 0..=0x10FF_u32 {
        let first = block << 8;
        let last = first | 0xFF;
        if (0xD800..=0xDFFF).contains(&first) {
            continue;
        }
        inputs.push((
            "scalars",
            format!("{first:04x}-{last:04x}"),
            scalar_block(first, last),
        ));
    }
    inputs
}

/// The bytes a record's `kind` and encoded `input` stand for.
fn decode_input(kind: &str, input: &str) -> Vec<u8> {
    match kind {
        "bytes" | "stream" => decode_bytes(input).expect("an encoded byte string"),
        "scalars" => {
            let (first, last) = input.split_once('-').expect("a FIRST-LAST range");
            let first = u32::from_str_radix(first, 16).expect("a hexadecimal scalar");
            let last = u32::from_str_radix(last, 16).expect("a hexadecimal scalar");
            scalar_block(first, last)
        }
        other => panic!("unknown FNV input kind {other:?}"),
    }
}

/// The three answers for `data`: plain, seeded fold, separated.
fn fnv_answers(data: &[u8]) -> Vec<String> {
    let plain = fnv1a64(data);
    vec![
        format!("{plain:016x}"),
        format!("{:016x}", fold(SEEDED_FOLD_STATE, data)),
        format!("{:016x}", fold(plain, &[SEPARATOR])),
    ]
}

fn fnv1a64_vectors_are_reproduced() {
    let file = VectorFile::parse(include_str!("vectors/fnv1a64_differential_vectors.txt"))
        .unwrap_or_else(|error| panic!("fnv1a64: {error:?}"));
    let replayed = file
        .replay(2, |fields| fnv_answers(&decode_input(fields[0], fields[1])))
        .unwrap_or_else(|mismatch| panic!("fnv1a64: {mismatch}"));
    assert_eq!(replayed, fnv_inputs().len(), "every input replayed");
}

/// A split input folds to the whole input's answer, at every split of every
/// structured byte input, so a caller may feed a digest in pieces.
fn fnv_fold_composes_over_every_split() {
    for (_, _, data) in fnv_inputs()
        .iter()
        .filter(|(kind, _, _)| *kind != "scalars")
    {
        let whole = fnv1a64(data);
        for split in 0..=data.len() {
            let (head, tail) = data.split_at(split);
            assert_eq!(fold(fold(BASIS, head), tail), whole);
        }
    }
}

/// Writes the vector files instead of replaying them, when asked to.
#[cfg(not(target_arch = "wasm32"))]
fn record_vectors_when_asked() {
    if std::env::var_os("PURRDF_RECORD_MIX_FNV").as_deref() != Some("1".as_ref()) {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
    let oracle = format!("purrdf-hash {} (self-recorded)", env!("CARGO_PKG_VERSION"));

    let mut recorder = purrdf_testkit::vectors::Recorder::new();
    for comment in [
        "SplitMix64: self-recorded answers (frozen).",
        "",
        "Answers: purrdf_hash::mix, recorded by `tests/splitmix_fnv.rs` with",
        "PURRDF_RECORD_MIX_FNV=1. Every other SplitMix64 in the workspace answers",
        "from this module, so these draws are what each of its callers sees.",
        "",
        "Streams, 10,000 draws each from the seeds 0, 1, 0xc057 and 0xffffffffffffffff:",
        "  next      splitmix64_next over a counter starting at the seed",
        "  step      splitmix64_step fed its own output, starting at the seed",
        "  finalize  splitmix64_finalize(seed + index), wrapping",
        "",
        "Fields: stream, seed (hex), index of the first draw (decimal), then sixteen",
        "consecutive draws (16 lowercase hex digits each).",
    ] {
        recorder.comment(comment).expect("a one-line comment");
    }
    recorder
        .header("oracle", &oracle)
        .expect("an oracle header");
    for record in splitmix_records() {
        recorder.record(&record).expect("an encodable record");
    }
    std::fs::write(
        dir.join("splitmix64_differential_vectors.txt"),
        recorder.render(),
    )
    .expect("the vector file is written");

    let mut recorder = purrdf_testkit::vectors::Recorder::new();
    for comment in [
        "FNV-1a 64-bit: self-recorded answers (frozen).",
        "",
        "Answers: purrdf_hash::fnv, recorded by `tests/splitmix_fnv.rs` with",
        "PURRDF_RECORD_MIX_FNV=1. Every other FNV-1a in the workspace answers from",
        "this module, so these digests are what each of its callers sees.",
        "",
        "Inputs, by kind:",
        "  bytes    the encoded byte string: the empty input, every byte, every",
        "           ordered pair of 24 escape-relevant bytes, ten boundary scalars'",
        "           UTF-8, and the 1,000 IRIs of corpus_iris.txt",
        "  stream   the first N bytes (N = 0..=256) of the little-endian",
        "           splitmix64_next stream from the seed N, encoded",
        "  scalars  FIRST-LAST (hex): the UTF-8 of every scalar value in the range,",
        "           in order; every block of 256 scalars except the surrogates'",
        "",
        "Fields: kind, input, fnv1a64(input), fold(0x10000001, input),",
        "fnv1a64(input followed by one 0xff byte); answers are 16 lowercase hex digits.",
    ] {
        recorder.comment(comment).expect("a one-line comment");
    }
    recorder
        .header("oracle", &oracle)
        .expect("an oracle header");
    for (kind, input, data) in fnv_inputs() {
        let mut record = vec![kind.to_owned(), input];
        record.extend(fnv_answers(&data));
        recorder.record(&record).expect("an encodable record");
    }
    std::fs::write(
        dir.join("fnv1a64_differential_vectors.txt"),
        recorder.render(),
    )
    .expect("the vector file is written");
    purrdf_testkit::harness::print_line("recorded the SplitMix64 and FNV-1a vectors");
}

purrdf_testkit::harness_main!(
    #[cfg(not(target_arch = "wasm32"))]
    record_vectors_when_asked,
    splitmix64_vectors_are_reproduced,
    fnv1a64_vectors_are_reproduced,
    fnv_fold_composes_over_every_split,
);
