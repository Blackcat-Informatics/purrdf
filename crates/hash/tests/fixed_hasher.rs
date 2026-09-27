// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fixed-key table hasher: its frozen self-vectors, replayed on every
//! target, and its statistical quality, measured natively.
//!
//! # Vectors
//!
//! `tests/vectors/fixed_hasher_portable_vectors.txt` and
//! `tests/vectors/fixed_hasher_aes_vectors.txt` record what this crate's own
//! portable and AES functions answered when they were written. The oracle is
//! purrdf-hash itself, so a replay proves stability (across targets,
//! compilers and later edits), not correctness. The portable file replays
//! everywhere: natively, on i686, and on wasm32 in Node (`make wasm-test`),
//! where the folded multiply is built from 32-bit halves. The AES file replays
//! on builds whose target enables AES. The records drive `Hasher` methods
//! directly, never the standard library's `Hash` impls, whose byte streams
//! are not this crate's to freeze.
//!
//! To re-record after a deliberate change of function, run this target with
//! `PURRDF_RECORD_FIXED_HASHER=1`. The portable file can be recorded from any
//! build. The AES file needs an AES build. A change of function is a
//! deliberate act: nothing may persist these hashes, but it still needs a
//! reason written down.
//!
//! # Quality (native only)
//!
//! * **Avalanche:** for each input bit and output bit, the probability that
//!   flipping the input bit flips the output bit, over 2^20 random inputs,
//!   lies in 50% ± 1%.
//! * **Collisions:** there are no 64-bit collisions among integer keys in
//!   `0..2^24` and among the 2,796,417 256-bit keys with at most three set
//!   bits.
//! * **χ²:** the low 20 bits (a table's bucket index) and the top 7 bits
//!   (hashbrown's control byte) are uniform over IRIs and sequential
//!   integers.

use core::hash::{BuildHasher, Hash, Hasher};

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
use purrdf_hash::backend::AesFixedHasher;
use purrdf_hash::backend::{FIXED_HASHER_PATH, PortableFixedHasher};
use purrdf_hash::fixed::{FixedHasher, FixedState};
use purrdf_testkit::rng::Xoshiro256;
use purrdf_testkit::vectors::{VectorFile, decode_str, encode_str};

// --- inputs ------------------------------------------------------------------

/// The first `length` bytes of the little-endian `u64` stream of
/// `Xoshiro256::from_seed(seed)`: the recipe every vector header states.
fn stream(length: usize, seed: u64) -> Vec<u8> {
    let mut rng = Xoshiro256::from_seed(seed);
    let mut bytes = Vec::with_capacity(length + 8);
    while bytes.len() < length {
        bytes.extend_from_slice(&rng.next_u64().to_le_bytes());
    }
    bytes.truncate(length);
    bytes
}

/// The repository IRI corpus.
fn corpus() -> Vec<String> {
    let file = VectorFile::parse(include_str!("vectors/corpus_iris.txt"))
        .unwrap_or_else(|error| panic!("corpus_iris.txt: {error}"));
    file.records()
        .iter()
        .map(|record| decode_str(record.fields[1]).expect("an encoded IRI"))
        .collect()
}

fn one<H: Hasher + Default>(feed: impl FnOnce(&mut H)) -> u64 {
    let mut hasher = H::default();
    feed(&mut hasher);
    hasher.finish()
}

// --- the vector file ---------------------------------------------------------

/// One record's streaming or terminal operation.
fn apply<H: Hasher + Default>(fields: &[&str], terminal: fn(u8, &[u8]) -> u64) -> u64 {
    let hex = |field: &str| u128::from_str_radix(field, 16).expect("a hexadecimal value");
    let decimal = |field: &str| field.parse::<usize>().expect("a decimal length");
    match fields[0] {
        "u8" => one::<H>(|h| h.write_u8(hex(fields[1]) as u8)),
        "u16" => one::<H>(|h| h.write_u16(hex(fields[1]) as u16)),
        "u32" => one::<H>(|h| h.write_u32(hex(fields[1]) as u32)),
        "u64" => one::<H>(|h| h.write_u64(hex(fields[1]) as u64)),
        "u128" => one::<H>(|h| h.write_u128(hex(fields[1]))),
        "usize" => one::<H>(|h| {
            h.write_usize(usize::try_from(hex(fields[1])).expect("a 32-bit usize"));
        }),
        "bytes" => {
            let data = stream(decimal(fields[1]), hex(fields[2]) as u64);
            one::<H>(|h| h.write(&data))
        }
        "terminal" => {
            let data = stream(decimal(fields[2]), hex(fields[3]) as u64);
            terminal(hex(fields[1]) as u8, &data)
        }
        "text" => {
            let text = decode_str(fields[1]).expect("an encoded text field");
            one::<H>(|h| h.write(text.as_bytes()))
        }
        "chain" => {
            // write_u32(a), write(bytes), write_u8(b), write_u64(c).
            let data = stream(decimal(fields[2]), hex(fields[3]) as u64);
            one::<H>(|h| {
                h.write_u32(hex(fields[1]) as u32);
                h.write(&data);
                h.write_u8(hex(fields[4]) as u8);
                h.write_u64(hex(fields[5]) as u64);
            })
        }
        "empty" => one::<H>(|_| {}),
        other => panic!("unknown vector operation {other:?}"),
    }
}

/// Every record's input fields, in file order.
fn vector_inputs() -> Vec<Vec<String>> {
    let mut records: Vec<Vec<String>> = vec![vec!["empty".to_owned()]];
    let mut rng = Xoshiro256::from_seed(0x6669_7865_6400_0001);
    let hex = |value: u128| format!("{value:x}");
    let edges: [u128; 6] = [0, 1, 0x7f, 0x80, 0xff, 0xffff_ffff];
    for (kind, bits) in [
        ("u8", 8),
        ("u16", 16),
        ("u32", 32),
        ("usize", 32),
        ("u64", 64),
        ("u128", 128),
    ] {
        let mask = if bits == 128 {
            u128::MAX
        } else {
            (1u128 << bits) - 1
        };
        let mut values: Vec<u128> = edges.iter().map(|edge| edge & mask).collect();
        values.push(mask);
        for _ in 0..64 {
            let value = (u128::from(rng.next_u64()) << 64) | u128::from(rng.next_u64());
            values.push(value & mask);
        }
        let mut seen = std::collections::BTreeSet::new();
        values.retain(|value| seen.insert(*value));
        for value in values {
            records.push(vec![kind.to_owned(), hex(value)]);
        }
    }
    // Every length 0..=300, two seeds each, then 200 longer lengths.
    for length in 0..=300usize {
        for salt in 0..2u64 {
            let seed = 0x7075_7272_6466_0000 | ((length as u64) << 1) | salt;
            records.push(vec![
                "bytes".to_owned(),
                length.to_string(),
                hex(seed.into()),
            ]);
            for tag in [0u8, 3] {
                records.push(vec![
                    "terminal".to_owned(),
                    hex(tag.into()),
                    length.to_string(),
                    hex(seed.into()),
                ]);
            }
        }
    }
    for _ in 0..200 {
        let length = 301 + rng.up_to(4096 - 301) as usize;
        let seed = rng.next_u64();
        records.push(vec![
            "bytes".to_owned(),
            length.to_string(),
            hex(seed.into()),
        ]);
    }
    for iri in corpus() {
        records.push(vec!["text".to_owned(), encode_str(&iri)]);
    }
    for _ in 0..256 {
        let length = rng.up_to(160) as usize;
        records.push(vec![
            "chain".to_owned(),
            hex(u128::from(rng.next_u64() as u32)),
            length.to_string(),
            hex(rng.next_u64().into()),
            hex(u128::from(rng.next_u64() as u8)),
            hex(rng.next_u64().into()),
        ]);
    }
    records
}

const PORTABLE_VECTORS: &str = include_str!("vectors/fixed_hasher_portable_vectors.txt");
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
const AES_VECTORS: &str = include_str!("vectors/fixed_hasher_aes_vectors.txt");

/// Replays `text` against `H`, returning the number of records.
fn replay<H: Hasher + Default>(name: &str, text: &str, terminal: fn(u8, &[u8]) -> u64) -> usize {
    let file = VectorFile::parse(text).unwrap_or_else(|error| panic!("{name}: {error}"));
    // Every operation's inputs precede the one answer field.
    let mut total = 0;
    for record in file.records() {
        let (inputs, answer) = record.fields.split_at(record.fields.len() - 1);
        let actual = format!("{:016x}", apply::<H>(inputs, terminal));
        assert_eq!(
            actual, answer[0],
            "{name}, line {}: {inputs:?} hashed differently",
            record.line
        );
        total += 1;
    }
    assert_eq!(
        total,
        vector_inputs().len(),
        "{name}: every record replayed"
    );
    total
}

fn portable_vectors_are_reproduced() {
    let count = purrdf_testkit::harness::without_host_clock_or_entropy(|| {
        replay::<PortableFixedHasher>(
            "portable",
            PORTABLE_VECTORS,
            PortableFixedHasher::hash_terminal,
        )
    });
    purrdf_testkit::harness::print_line(&format!("fixed hasher: portable, {count} records"));
}

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
fn aes_vectors_are_reproduced() {
    let count = replay::<AesFixedHasher>("aes", AES_VECTORS, AesFixedHasher::hash_terminal);
    purrdf_testkit::harness::print_line(&format!("fixed hasher: aes, {count} records"));
}

/// `FixedHasher` and `FixedState` answer as the path this build names.
fn the_selected_function_answers_its_own_vectors() {
    let text = match FIXED_HASHER_PATH {
        "portable" => PORTABLE_VECTORS,
        #[cfg(all(
            any(target_arch = "x86_64", target_arch = "aarch64"),
            target_endian = "little",
            target_feature = "aes"
        ))]
        "aes" => AES_VECTORS,
        other => panic!("unknown path {other:?}"),
    };
    replay::<FixedHasher>(FIXED_HASHER_PATH, text, FixedHasher::hash_terminal);
    let state = FixedState::new();
    assert_eq!(state, FixedState::default());
    let copy = state;
    let text = "http://example.org/s";
    // `build_hasher` starts where `FixedHasher::default` does.
    assert_eq!(
        copy.build_hasher().finish(),
        FixedHasher::default().finish()
    );
    assert_eq!(state.hash_one(text), one::<FixedHasher>(|h| text.hash(h)));
    assert_eq!(
        FixedState::new().hash_one(7u32),
        FixedState::new().hash_one(7u32)
    );
    purrdf_testkit::harness::print_line(&format!(
        "fixed hasher: this build selects {FIXED_HASHER_PATH}"
    ));
}

/// Writes the vector files instead of replaying them, when asked to.
#[cfg(not(target_arch = "wasm32"))]
fn record_vectors_when_asked() {
    if std::env::var_os("PURRDF_RECORD_FIXED_HASHER").as_deref() != Some("1".as_ref()) {
        return;
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors");
    let write = |file: &str, path: &str, answer: &dyn Fn(&[&str]) -> u64| {
        let mut recorder = purrdf_testkit::vectors::Recorder::new();
        let comments = [
            format!("FixedHasher, {path} function: self-recorded answers (frozen)."),
            String::new(),
            format!(
                "Answers: purrdf-hash's own `purrdf_hash::backend::{}FixedHasher`, recorded",
                if path == "aes" { "Aes" } else { "Portable" }
            ),
            "by `tests/fixed_hasher.rs` with PURRDF_RECORD_FIXED_HASHER=1. A self-vector"
                .to_owned(),
            "proves stability across targets and time, not correctness; no third-party".to_owned(),
            "implementation or output was consulted.".to_owned(),
            String::new(),
            "Each record is one operation; streaming ones use a fresh hasher and `finish`:"
                .to_owned(),
            "  empty                       nothing written".to_owned(),
            "  u8|u16|u32|usize|u64|u128 V write_<kind>(V), V hexadecimal".to_owned(),
            "  bytes L S                   write(first L bytes of the little-endian u64".to_owned(),
            "                              stream of Xoshiro256::from_seed(S)), S hex".to_owned(),
            "  terminal T L S              hash_terminal(T, first L stream bytes), T hex"
                .to_owned(),
            "  text T                      write(T's UTF-8 bytes); T from corpus_iris.txt"
                .to_owned(),
            "  chain A L S B C             write_u32(A), write(bytes L S), write_u8(B),".to_owned(),
            "                              write_u64(C)".to_owned(),
            String::new(),
            "Fields: operation, inputs, then the 64-bit hash (16 lowercase hex digits).".to_owned(),
        ];
        for comment in &comments {
            recorder.comment(comment).expect("a one-line comment");
        }
        recorder
            .header(
                "oracle",
                &format!(
                    "purrdf-hash {} (fixed::{path}, self-recorded)",
                    env!("CARGO_PKG_VERSION")
                ),
            )
            .expect("an oracle header");
        for inputs in vector_inputs() {
            let fields: Vec<&str> = inputs.iter().map(String::as_str).collect();
            let hash = format!("{:016x}", answer(&fields));
            let mut record = inputs.clone();
            record.push(hash);
            recorder.record(&record).expect("an encodable record");
        }
        std::fs::write(dir.join(file), recorder.render()).expect("the vector file is written");
        purrdf_testkit::harness::print_line(&format!("recorded {file}"));
    };
    write("fixed_hasher_portable_vectors.txt", "portable", &|fields| {
        apply::<PortableFixedHasher>(fields, PortableFixedHasher::hash_terminal)
    });
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    ))]
    write("fixed_hasher_aes_vectors.txt", "aes", &|fields| {
        apply::<AesFixedHasher>(fields, AesFixedHasher::hash_terminal)
    });
}

// --- contract ----------------------------------------------------------------

/// Integers are zero-extended; `usize` hashes as a `u64` on every target.
fn integers_share_one_word() {
    for value in [0u64, 1, 0xff, 0xdead_beef, u64::from(u32::MAX)] {
        let as_u64 = one::<FixedHasher>(|h| h.write_u64(value));
        assert_eq!(
            one::<FixedHasher>(|h| h.write_usize(value as usize)),
            as_u64
        );
        assert_eq!(one::<FixedHasher>(|h| h.write_u32(value as u32)), as_u64);
    }
    // Length is part of a slice's hash, and state carries between writes.
    assert_ne!(
        one::<FixedHasher>(|h| h.write(b"")),
        one::<FixedHasher>(|_| {})
    );
    assert_ne!(
        one::<FixedHasher>(|h| h.write(&[0; 20])),
        one::<FixedHasher>(|h| h.write(&[0; 21]))
    );
    assert_ne!(
        one::<FixedHasher>(|h| {
            h.write_u64(1);
            h.write_u64(2);
        }),
        one::<FixedHasher>(|h| {
            h.write_u64(2);
            h.write_u64(1);
        })
    );
}

/// The two functions differ only on slices longer than 16 bytes.
#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
fn paths_share_integers_and_short_slices() {
    let mut rng = Xoshiro256::from_seed(0x7368_6172_6564);
    for _ in 0..1 << 16 {
        let word = rng.next_u64();
        assert_eq!(
            one::<PortableFixedHasher>(|h| h.write_u64(word)),
            one::<AesFixedHasher>(|h| h.write_u64(word))
        );
        let data = stream(rng.up_to(16) as usize, rng.next_u64());
        assert_eq!(
            one::<PortableFixedHasher>(|h| h.write(&data)),
            one::<AesFixedHasher>(|h| h.write(&data))
        );
        let long = stream(17 + rng.up_to(200) as usize, rng.next_u64());
        assert_ne!(
            one::<PortableFixedHasher>(|h| h.write(&long)),
            one::<AesFixedHasher>(|h| h.write(&long))
        );
    }
}

// --- quality -----------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod quality {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{Hasher, Xoshiro256, corpus, one};

    /// Samples per avalanche cell.
    const SAMPLES: usize = 1 << 20;
    /// The samples are drawn in this many fixed chunks, one seed each, so the
    /// counts do not depend on how many threads share the work.
    const CHUNKS: usize = 64;

    /// Runs `work(chunk)` for every chunk on a few threads and merges the
    /// results with `merge`.
    fn in_chunks<T: Send>(
        chunks: usize,
        work: impl Fn(usize) -> T + Sync,
        mut merge: impl FnMut(T),
    ) {
        let threads = std::thread::available_parallelism()
            .map_or(1, std::num::NonZero::get)
            .min(8);
        let next = AtomicUsize::new(0);
        let done = Mutex::new(Vec::with_capacity(chunks));
        std::thread::scope(|scope| {
            for _ in 0..threads {
                scope.spawn(|| {
                    loop {
                        let chunk = next.fetch_add(1, Ordering::Relaxed);
                        if chunk >= chunks {
                            break;
                        }
                        let result = work(chunk);
                        done.lock().expect("no worker panicked").push(result);
                    }
                });
            }
        });
        for result in done.into_inner().expect("no worker panicked") {
            merge(result);
        }
    }

    /// Flip probability per (input bit, output bit) for `hash` over inputs of
    /// `len` bytes; panics when any cell leaves ½ ± the tolerance. Returns the
    /// worst deviation from one half and the tolerance applied.
    ///
    /// The tolerance is 1% over 2^20 random inputs. An input of one or two
    /// bytes has only 256 or 65,536 values, so repeating them 2^20 times adds
    /// no information: such lengths are enumerated exhaustively instead, and
    /// each cell is then an exact proportion over `2^(8·len) / 2` independent
    /// input pairs. Even an ideal random function scatters around ½ with
    /// σ = ½/√pairs there (4.4% for one byte, 0.28% for two), which ±1%
    /// cannot allow for one byte, so the tolerance is max(1%, 5σ).
    fn avalanche(name: &str, len: usize, hash: &(dyn Fn(&[u8]) -> u64 + Sync)) -> (f64, f64) {
        let bits = 8 * len;
        let exhaustive = bits <= 16;
        let samples = if exhaustive { 1usize << bits } else { SAMPLES };
        let per_chunk = samples.div_ceil(CHUNKS);
        let mut counts = vec![0u32; bits * 64];
        in_chunks(
            CHUNKS,
            |chunk| {
                let mut local = vec![0u32; bits * 64];
                let mut rng = Xoshiro256::from_seed(
                    0x6176_616c_0000_0000 ^ ((len as u64) << 16) ^ chunk as u64,
                );
                let mut input = vec![0u8; len];
                let first = chunk * per_chunk;
                for sample in first..(first + per_chunk).min(samples) {
                    if exhaustive {
                        input.copy_from_slice(&sample.to_le_bytes()[..len]);
                    } else {
                        for byte in input.chunks_mut(8) {
                            let word = rng.next_u64().to_le_bytes();
                            byte.copy_from_slice(&word[..byte.len()]);
                        }
                    }
                    let base = hash(&input);
                    for bit in 0..bits {
                        input[bit / 8] ^= 1 << (bit % 8);
                        let diff = base ^ hash(&input);
                        input[bit / 8] ^= 1 << (bit % 8);
                        let row = &mut local[bit * 64..bit * 64 + 64];
                        for (out, count) in row.iter_mut().enumerate() {
                            *count += ((diff >> out) & 1) as u32;
                        }
                    }
                }
                local
            },
            |local| {
                for (total, part) in counts.iter_mut().zip(local) {
                    *total += part;
                }
            },
        );
        let tolerance = if exhaustive {
            let pairs = (samples / 2) as f64;
            (5.0 * 0.5 / pairs.sqrt()).max(0.01)
        } else {
            0.01
        };
        let mut worst = 0.0f64;
        for (cell, &count) in counts.iter().enumerate() {
            let p = f64::from(count) / samples as f64;
            let deviation = (p - 0.5).abs();
            assert!(
                deviation <= tolerance,
                "{name}, {len} bytes: input bit {} flips output bit {} with p = {p:.4} \
                 (tolerance ±{tolerance:.4})",
                cell / 64,
                cell % 64
            );
            worst = worst.max(deviation);
        }
        (worst, tolerance)
    }

    /// The byte lengths the slice avalanche runs: every length whose packing
    /// or block schedule differs from its neighbours (1–16 byte by byte, each
    /// side of the 16/32/64-byte boundaries, overlapping and whole final
    /// blocks) up to 128.
    const LENGTHS: [usize; 29] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 24, 31, 32, 33, 48, 63, 64, 65,
        96, 100, 127, 128,
    ];

    pub(super) fn avalanche_integers<H: Hasher + Default>(path: &str) {
        let (u32_worst, _) = avalanche(&format!("{path} u32"), 4, &|bytes| {
            let word = u32::from_le_bytes(bytes.try_into().expect("four bytes"));
            one::<H>(|h| h.write_u32(word))
        });
        let (u64_worst, _) = avalanche(&format!("{path} u64"), 8, &|bytes| {
            let word = u64::from_le_bytes(bytes.try_into().expect("eight bytes"));
            one::<H>(|h| h.write_u64(word))
        });
        purrdf_testkit::harness::print_line(&format!(
            "avalanche {path}: u32 worst |p − ½| = {u32_worst:.5}, u64 {u64_worst:.5} \
             (2^20 samples per cell)"
        ));
    }

    pub(super) fn avalanche_slices<H: Hasher + Default>(path: &str, lengths: &[usize]) {
        for &len in lengths {
            let (worst, tolerance) = avalanche(&format!("{path} bytes"), len, &|bytes| {
                one::<H>(|h| h.write(bytes))
            });
            purrdf_testkit::harness::print_line(&format!(
                "avalanche {path}: {len:>3} bytes, worst |p − ½| = {worst:.5} (tolerance {tolerance:.4})"
            ));
        }
    }

    pub(super) fn short_lengths() -> Vec<usize> {
        LENGTHS.iter().copied().filter(|&len| len <= 16).collect()
    }

    pub(super) fn long_lengths() -> Vec<usize> {
        LENGTHS.iter().copied().filter(|&len| len > 16).collect()
    }

    fn assert_distinct(name: &str, mut hashes: Vec<u64>) {
        let total = hashes.len();
        hashes.sort_unstable();
        let collisions = hashes.windows(2).filter(|pair| pair[0] == pair[1]).count();
        assert_eq!(
            collisions, 0,
            "{name}: 64-bit collisions among {total} keys"
        );
        purrdf_testkit::harness::print_line(&format!("{name}: {total} keys, 0 collisions"));
    }

    pub(super) fn no_integer_collisions<H: Hasher + Default>(path: &str) {
        let n = 1u64 << 24;
        assert_distinct(
            &format!("{path} u32 0..2^24"),
            (0..n)
                .map(|v| one::<H>(|h| h.write_u32(v as u32)))
                .collect(),
        );
        assert_distinct(
            &format!("{path} u64 0..2^24"),
            (0..n).map(|v| one::<H>(|h| h.write_u64(v))).collect(),
        );
        assert_distinct(
            &format!("{path} u64 (0..2^24)·2^40"),
            (0..n).map(|v| one::<H>(|h| h.write_u64(v << 40))).collect(),
        );
    }

    pub(super) fn no_sparse_collisions<H: Hasher + Default>(path: &str) {
        let hash = |bits: &[usize]| {
            let mut key = [0u8; 32];
            for &bit in bits {
                key[bit / 8] |= 1 << (bit % 8);
            }
            one::<H>(|h| h.write(&key))
        };
        let mut hashes = Vec::with_capacity(2_796_417);
        hashes.push(hash(&[]));
        for a in 0..256 {
            hashes.push(hash(&[a]));
            for b in a + 1..256 {
                hashes.push(hash(&[a, b]));
                for c in b + 1..256 {
                    hashes.push(hash(&[a, b, c]));
                }
            }
        }
        assert_eq!(hashes.len(), 2_796_417);
        assert_distinct(&format!("{path} sparse 256-bit keys"), hashes);
    }

    pub(super) fn terminal_hash_quality(path: &str, hash: fn(u8, &[u8]) -> u64) {
        for len in [2, 16, 17, 24, 31, 32, 33] {
            let (worst, tolerance) =
                avalanche(&format!("{path} terminal"), len, &|bytes| hash(0, bytes));
            purrdf_testkit::harness::print_line(&format!(
                "avalanche {path} terminal: {len:>2} bytes, worst |p − ½| = {worst:.5} (tolerance {tolerance:.4})"
            ));
        }

        let mut sparse = Vec::with_capacity(2_796_417);
        let sparse_hash = |bits: &[usize]| {
            let mut key = [0u8; 32];
            for &bit in bits {
                key[bit / 8] |= 1 << (bit % 8);
            }
            hash(0, &key)
        };
        sparse.push(sparse_hash(&[]));
        for a in 0..256 {
            sparse.push(sparse_hash(&[a]));
            for b in a + 1..256 {
                sparse.push(sparse_hash(&[a, b]));
                for c in b + 1..256 {
                    sparse.push(sparse_hash(&[a, b, c]));
                }
            }
        }
        assert_distinct(&format!("{path} terminal sparse 256-bit keys"), sparse);

        let cross_length: Vec<u64> = (2..=128)
            .flat_map(|len| {
                let iri = format!("a:{}", "x".repeat(len - 2));
                [hash(0, iri.as_bytes()), hash(3, iri.as_bytes())]
            })
            .collect();
        assert_distinct(&format!("{path} terminal tags and lengths"), cross_length);

        let iris = corpus();
        let hashes: Vec<u64> = iris.iter().map(|iri| hash(0, iri.as_bytes())).collect();
        assert_distinct(&format!("{path} terminal corpus IRIs"), hashes.clone());
        assert_uniform(&format!("{path} terminal corpus IRIs"), &hashes, 7);
        let mut minted = Vec::with_capacity(iris.len() * 8192);
        let mut key = Vec::with_capacity(256);
        for iri in &iris {
            for n in 0..8192u32 {
                key.clear();
                key.extend_from_slice(iri.as_bytes());
                key.extend_from_slice(format!("{{{n}}}").as_bytes());
                minted.push(hash(0, &key));
            }
        }
        assert_uniform(
            &format!("{path} terminal corpus IRIs × 0..8192"),
            &minted,
            20,
        );
    }

    /// The Wilson–Hilferty z of a χ² statistic with `df` degrees of freedom:
    /// approximately standard normal under the uniform hypothesis.
    fn chi_squared_z(counts: &[u64]) -> f64 {
        let total: u64 = counts.iter().sum();
        let expected = total as f64 / counts.len() as f64;
        let statistic: f64 = counts
            .iter()
            .map(|&count| {
                let d = count as f64 - expected;
                d * d / expected
            })
            .sum();
        let df = (counts.len() - 1) as f64;
        let shape = 2.0 / (9.0 * df);
        ((statistic / df).cbrt() - (1.0 - shape)) / shape.sqrt()
    }

    /// χ² of the low 20 bits and of the top 7 bits of `hashes`; each z must
    /// stay below 5 (a false alarm about once in 3.5 million runs).
    fn assert_uniform(name: &str, hashes: &[u64], low_bits: u32) {
        let mut low = vec![0u64; 1 << low_bits];
        let mut top = [0u64; 128];
        for &hash in hashes {
            low[(hash & ((1 << low_bits) - 1)) as usize] += 1;
            top[(hash >> 57) as usize] += 1;
        }
        let (z_low, z_top) = (chi_squared_z(&low), chi_squared_z(&top));
        purrdf_testkit::harness::print_line(&format!(
            "χ² {name}: {} keys, low {low_bits} bits z = {z_low:.2}, top 7 bits z = {z_top:.2}",
            hashes.len()
        ));
        assert!(
            z_low < 5.0,
            "{name}: the low {low_bits} bits are not uniform (z = {z_low:.2})"
        );
        assert!(
            z_top < 5.0,
            "{name}: the top 7 bits are not uniform (z = {z_top:.2})"
        );
    }

    pub(super) fn uniform_over_iris_and_integers<H: Hasher + Default>(path: &str) {
        let iris = corpus();
        let hashes: Vec<u64> = iris
            .iter()
            .map(|iri| one::<H>(|h| h.write(iri.as_bytes())))
            .collect();
        // 1,000 keys cannot fill 2^20 bins; they must still be distinct, and
        // at most a birthday handful may share their low 20 bits (about 0.48
        // expected pairs; more than 5 has probability below 10^-5).
        let mut low: Vec<u64> = hashes.iter().map(|hash| hash & 0xF_FFFF).collect();
        low.sort_unstable();
        let shared = low.windows(2).filter(|pair| pair[0] == pair[1]).count();
        assert!(
            shared <= 5,
            "{path}: {shared} IRI pairs share their low 20 bits"
        );
        let mut distinct = hashes.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            iris.len(),
            "{path}: every corpus IRI hashes apart"
        );
        // Seven low bits (128 bins, ~7.8 per bin) and the top seven over the
        // corpus itself.
        assert_uniform(&format!("{path} corpus IRIs"), &hashes, 7);
        // The corpus with 8,192 numeric suffixes each, as minted instance
        // IRIs are: 8,192,000 keys, ~7.8 per low-20-bit bin. The suffix is
        // `{n}`: a brace cannot occur in a corpus IRI (see its header), so
        // every key is distinct. A bare numeric suffix would not be:
        // `…/s1` + `1` is `…/s` + `11`, and duplicated keys inflate χ².
        let mut minted = Vec::with_capacity(iris.len() * 8192);
        let mut key = Vec::with_capacity(256);
        for iri in &iris {
            for n in 0..8192u32 {
                key.clear();
                key.extend_from_slice(iri.as_bytes());
                key.extend_from_slice(format!("{{{n}}}").as_bytes());
                minted.push(one::<H>(|h| h.write(&key)));
            }
        }
        assert_uniform(&format!("{path} corpus IRIs × 0..8192"), &minted, 20);
        let sequential: Vec<u64> = (0..1u64 << 24)
            .map(|v| one::<H>(|h| h.write_u64(v)))
            .collect();
        assert_uniform(&format!("{path} u64 0..2^24"), &sequential, 20);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn portable_avalanche_integers() {
    quality::avalanche_integers::<PortableFixedHasher>("portable");
}

#[cfg(not(target_arch = "wasm32"))]
fn portable_avalanche_short_slices() {
    quality::avalanche_slices::<PortableFixedHasher>("portable", &quality::short_lengths());
}

#[cfg(not(target_arch = "wasm32"))]
fn portable_avalanche_long_slices() {
    quality::avalanche_slices::<PortableFixedHasher>("portable", &quality::long_lengths());
}

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
fn aes_avalanche_long_slices() {
    quality::avalanche_slices::<AesFixedHasher>("aes", &quality::long_lengths());
}

#[cfg(not(target_arch = "wasm32"))]
fn portable_has_no_integer_collisions() {
    quality::no_integer_collisions::<PortableFixedHasher>("portable");
}

#[cfg(not(target_arch = "wasm32"))]
fn portable_has_no_sparse_key_collisions() {
    quality::no_sparse_collisions::<PortableFixedHasher>("portable");
}

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
fn aes_has_no_sparse_key_collisions() {
    quality::no_sparse_collisions::<AesFixedHasher>("aes");
}

#[cfg(not(target_arch = "wasm32"))]
fn portable_is_uniform() {
    quality::uniform_over_iris_and_integers::<PortableFixedHasher>("portable");
}

#[cfg(not(target_arch = "wasm32"))]
fn portable_terminal_hash_quality() {
    quality::terminal_hash_quality("portable", PortableFixedHasher::hash_terminal);
}

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
fn aes_terminal_hash_quality() {
    quality::terminal_hash_quality("aes", AesFixedHasher::hash_terminal);
}

#[cfg(all(
    any(target_arch = "x86_64", target_arch = "aarch64"),
    target_endian = "little",
    target_feature = "aes"
))]
fn aes_is_uniform() {
    quality::uniform_over_iris_and_integers::<AesFixedHasher>("aes");
}

purrdf_testkit::harness_main!(
    #[cfg(not(target_arch = "wasm32"))]
    record_vectors_when_asked,
    portable_vectors_are_reproduced,
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    ))]
    aes_vectors_are_reproduced,
    the_selected_function_answers_its_own_vectors,
    integers_share_one_word,
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    ))]
    paths_share_integers_and_short_slices,
    #[cfg(not(target_arch = "wasm32"))]
    portable_avalanche_integers,
    #[cfg(not(target_arch = "wasm32"))]
    portable_avalanche_short_slices,
    #[cfg(not(target_arch = "wasm32"))]
    portable_avalanche_long_slices,
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    ))]
    aes_avalanche_long_slices,
    #[cfg(not(target_arch = "wasm32"))]
    portable_has_no_integer_collisions,
    #[cfg(not(target_arch = "wasm32"))]
    portable_has_no_sparse_key_collisions,
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    ))]
    aes_has_no_sparse_key_collisions,
    #[cfg(not(target_arch = "wasm32"))]
    portable_is_uniform,
    #[cfg(not(target_arch = "wasm32"))]
    portable_terminal_hash_quality,
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    ))]
    aes_terminal_hash_quality,
    #[cfg(all(
        any(target_arch = "x86_64", target_arch = "aarch64"),
        target_endian = "little",
        target_feature = "aes"
    ))]
    aes_is_uniform,
);
