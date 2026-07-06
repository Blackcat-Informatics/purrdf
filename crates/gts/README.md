<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-gts` — GTS Graph-Transport Container Engine

[![crates.io](https://img.shields.io/crates/v/purrdf-gts.svg)](https://crates.io/crates/purrdf-gts)
[![docs.rs](https://docs.rs/purrdf-gts/badge.svg)](https://docs.rs/purrdf-gts)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-gts` is the GTS (Graph Transport Substrate) container engine of the
PurRDF toolkit: a single-file, content-addressed, append-only format for
shipping RDF 1.2 graphs — and the binary blobs they reference — between
systems. A GTS file is a CBOR Sequence of segments, each an append-only log of
frames chained by BLAKE3 content id; the reader verifies the chain and folds
the log into a container graph, degrading undecodable frames to opaque nodes
instead of aborting — **the reader is total**.

The crate owns the wire-format machinery:

- **`reader`** — chain-verified reading and deterministic folding of GTS bytes
  into the container graph model (quads, reifiers, annotations, blobs).
- **`writer`** — frame authoring and byte-deterministic single-segment
  snapshots (`Writer::deterministic`), with optional per-frame COSE signing.
- **`model`** — the folded transport-graph rows and fold diagnostics.
- **`verify`, `cose`, `openpgp`, `policy`** — integrity, signature, and
  trust-policy checks; encryption and signing use pure-Rust crypto, so the
  whole engine stays wasm-friendly.
- **`files`, `tar`, `stream`** — content/file transport helpers and streaming
  state.

Both this engine and its sibling implementations are gated against the same
frozen, language-neutral conformance vectors, byte-exact. The format is
specified in
[`docs/GTS-SPEC.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/GTS-SPEC.md).

## Usage

```sh
cargo add purrdf-gts
```

```rust
use purrdf_gts::reader;

// Fold GTS bytes into the container graph model, verifying the BLAKE3 chain.
let graph = reader::read(&bytes, /* allow_segments */ true, /* expected_head */ None);

// The fold is total: quads, reifiers, annotations, and blobs are all rows,
// and anything undecodable is preserved as an opaque node plus a diagnostic.
println!("{} quads, {} blobs", graph.quads.len(), graph.blobs.len());
```

RDF text formats and the `RdfDataset` import/export path deliberately live one
layer up: use the umbrella [`purrdf`](https://crates.io/crates/purrdf) crate
(its `gts` module combines this engine with the RDF-level adapter) for
RDF-facing GTS work.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate; depend on `purrdf-gts`
directly only when you want the container engine alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
