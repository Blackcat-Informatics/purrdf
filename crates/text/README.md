<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-text` — Deterministic Full-Text Search over RDF Literals

[![crates.io](https://img.shields.io/crates/v/purrdf-text.svg)](https://crates.io/crates/purrdf-text)
[![docs.rs](https://docs.rs/purrdf-text/badge.svg)](https://docs.rs/purrdf-text)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-text` is the in-memory full-text index of the PurRDF toolkit. It reads
RDF 1.2 literals out of a frozen dataset, tokenizes them by the Unicode word
boundaries of `UAX #29` over NFC-normalized text (`UAX #15`), and answers ranked
retrieval queries with BM25 scores.

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
its input on every target, and a ranking is reproducible byte for byte.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate; depend on `purrdf-text`
directly when you want the index and its arithmetic on their own.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
