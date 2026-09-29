// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `purrdf-hash-conformance` — the frozen-vector suites of `purrdf-hash`.
//!
//! `purrdf-hash` is the workspace's zero-dependency root: every crate may
//! depend on it, `purrdf-testkit` included. Its vector replays need testkit's
//! vector format, deterministic stream and `harness = false` runner, so they
//! live in this unpublished crate instead of in `purrdf-hash`'s own tests,
//! where a dev-dependency on testkit would close a cycle.
//!
//! | Target | What it replays |
//! |---|---|
//! | `tests/digest_differential.rs` | MD5, SHA-1, SHA-3 and CRC-32 over every path the host runs |
//! | `tests/blake3.rs` | BLAKE3 streaming boundaries and random lengths, on every backend |
//! | `tests/hex.rs` | the frozen base16 encoding and digit tables through every entry point and encoding path, RFC 4648 vectors, reader refusals, `Digest32` |
//! | `tests/fixed_hasher.rs` | the table hasher's frozen self-vectors and its statistical quality |
//! | `tests/splitmix_fnv.rs` | SplitMix64's three streams and FNV-1a over structured corpora |
//! | `benches/hasher.rs` | the table hasher's latency per key class |
//!
//! The vector files are in `tests/vectors/`. Every test target runs natively
//! under `cargo test` and on `wasm32-unknown-unknown` under `make wasm-test`.

use purrdf_testkit::vectors::{VectorFile, decode_str};

/// The repository IRI corpus, `tests/vectors/corpus_iris.txt`: the 1,000 most
/// frequent absolute IRIs in the repository's RDF test corpora, most frequent
/// first.
///
/// # Panics
///
/// When the file fails its integrity check or a record is not an encoded
/// string: the corpus is a frozen input, and a damaged one must fail loudly.
#[must_use]
pub fn iri_corpus() -> Vec<String> {
    let file = VectorFile::parse(include_str!("../tests/vectors/corpus_iris.txt"))
        .unwrap_or_else(|error| panic!("corpus_iris.txt: {error}"));
    file.records()
        .iter()
        .map(|record| decode_str(record.fields[1]).expect("an encoded IRI"))
        .collect()
}
