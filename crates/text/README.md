<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-text` — Deterministic Full-Text Search over RDF Literals

[![crates.io](https://img.shields.io/crates/v/purrdf-text.svg)](https://crates.io/crates/purrdf-text)
[![docs.rs](https://docs.rs/purrdf-text/badge.svg)](https://docs.rs/purrdf-text)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0%20OR%20MulanPSL--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-text` is the in-memory full-text index of the PurRDF toolkit. It reads
RDF 1.2 literals out of a frozen dataset, tokenizes them by the Unicode word
boundaries of `UAX #29` over NFC-normalized text (`UAX #15`), and answers ranked
retrieval queries with BM25F scores.

## Caller-supplied IRIs

PurRDF mints no vocabulary. This crate exposes ranked retrieval to SPARQL
through the evaluator's property-function seam, and the predicate IRIs a query
calls it by are **configuration the caller supplies** — there is no default
namespace and no fabricated fallback. A configuration that names no IRI is a
typed error, not a guess. The same holds for the predicates whose literals are
indexed: the caller names them.

## Exact arithmetic, identical answers everywhere

Ranking is done in base-10 fixed-point integer arithmetic with twelve
fractional digits. No float enters the crate — the crate root carries
`#![deny(clippy::float_arithmetic)]`, so it cannot.

That is a correctness property, not an aesthetic one. BM25 needs a natural
logarithm, and a libm `ln` is free to differ by a unit in the last place
between implementations. One such difference is enough to swap the order of two
near-tied documents, so the same query over the same data could return rows in a
different order on a native build than in a WebAssembly build of the same
engine. The logarithm here is a fixed-length series over integers — a fixed
iteration count, never a convergence test — so its result is a pure function of
its input on every target.

The two halves of what that buys are proven in different places, so they are
claimed separately. The **ranking** — row order together with every score's
decimal lexical — is pinned by a single test body carrying both `#[test]` and
`#[wasm_bindgen_test]`, so `make wasm-test` executes it on
`wasm32-unknown-unknown` against the same expectations `cargo test` asserts
natively; that is the half a divergent `ln` could actually move. Byte identity
of the **serialized** answer — two independently built indexes queried through
the property-function seam, compared as SPARQL-JSON strings — is asserted
natively. Only the ranking claim is executed on both targets, so only it is
stated for both.

## Versioned BM25F, ranked within a partition

Ranking uses one fielded BM25F path, including the single-field case. The
immutable `RankingProfile` binds up to sixteen named fields, each field's
weight and length normalization, total predicate routing, exact arithmetic,
query aggregation, input bounds and the score bound into a fingerprint.
`k1 = 1.2` is fixed. `RankingProfile::single_field()` explicitly routes every
selected predicate to one field with weight one and `b = 0.75`.

`TextIndex::from_dataset_with_ranking` builds under an explicit profile;
`with_ranking_profile` reweights or remaps an existing index from retained
predicate-level token facts without re-tokenizing. Analyzer identity is
separate from ranking identity. The pure `PreparedCorpus` and `PreparedQuery`
APIs expose the same scoring implementation for other stores, binding cached
IDFs to validated corpus statistics and the immutable profile.

The maximum score is exactly `65_536 * 10^12` raw units and needs 56 bits.
Scores above that exact maximum are refused even if they fit in 56 bits.
See [the ranking contract](RANKING.md) for the bound proof, operation order,
independent arithmetic reference and vocabulary-search ownership decision.

Corpus statistics are computed per `(graph, language)` partition, so a score is a
number relative to one corpus. `?rank` is therefore the 1-based position of a
document **within its own partition**, never a global position; a rank spanning
partitions would order numbers computed against different corpora. Rows are
emitted in `(partition key ASC, rank ASC)` order, and a query wanting one ranked
list binds `?lang` and `?graph` or uses a single-partition index.

The Rust field is named `Scored::partition_rank` for that reason: nothing about
the *value* records which corpus produced it, so an answer over a three-language
index carries three rows of rank 1 and `LIMIT 10` over it is the first ten of an
interleaving rather than the ten best documents.

Within a partition the order is `(score DESC, document id ASC)`. Document ids are
assigned only after sorting on `(graph, subject, language)`, so ascending id is
ascending canonical order and the tie-break is reproducible across independently
built indexes. Because ids are distinct, the order is strict and total.

A score reaches a consumer as an `xsd:decimal` of fixed width, so two rows can
report the same `?score` while carrying different `?rank` — whether their scores
are exactly equal or merely equal once rounded. `ORDER BY ?rank` is the
reproducing idiom; `ORDER BY DESC(?score)` can disagree with it.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C (the GTS container itself reaches Python and C, not the wasm package). Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate; depend on `purrdf-text`
directly when you want the index and its arithmetic on their own.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.98, stable toolchain only).

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
