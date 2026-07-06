<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-rdf` — RDF 1.2 Implementation Layer

[![crates.io](https://img.shields.io/crates/v/purrdf-rdf.svg)](https://crates.io/crates/purrdf-rdf)
[![docs.rs](https://docs.rs/purrdf-rdf/badge.svg)](https://docs.rs/purrdf-rdf)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-rdf` is the RDF 1.2 implementation layer of the PurRDF toolkit: the
narrow waist between transport/runtime stores (such as the GTS container) and
consumers like SHACL, SPARQL, and SARIF reporting. It depends on and re-exports
the ring-fenced [`purrdf-core`](https://crates.io/crates/purrdf-core) kernel
(the interned IR, diagnostics, store traits, `DatasetView`, provenance, and the
loss ledger) and adds what the kernel deliberately leaves out:

- **Native text codecs** — first-party parsers/serializers for Turtle, TriG,
  N-Triples, N-Quads, and RDF/XML, plus JSON-LD (star) and YAML-LD; parsing
  can optionally record a source-position span table for diagnostics.
- **RDF 1.2 statement layer** — reifier bindings and annotations survive every
  star-capable round-trip; star-incapable projections drop them *loudly*, with
  the realized count handed to the loss ledger.
- **Canonicalization entry points** — W3C RDFC-1.0 (`canonicalize`) and
  canonical flat N-Quads over the frozen IR.
- **GTS adapters** — import/export between `RdfDataset` and the
  [`purrdf-gts`](https://crates.io/crates/purrdf-gts) container, including
  snapshot composition and content-chain verification.
- **Describe & normalization** — per-subject Symmetric-CBD extraction and a
  review-friendly Turtle normalizer.

The crate is PyO3-free and oxigraph-free (like the whole workspace), keeps
reporting structured but SARIF-free — callers translate `RdfDiagnostic`s into
SARIF via [`purrdf-validate`](https://crates.io/crates/purrdf-validate) — and
builds cleanly for `wasm32-unknown-unknown`.

## Usage

```sh
cargo add purrdf-rdf
```

```rust
use purrdf_rdf::{parse_dataset, serialize_dataset, SerializeGraph};

let turtle = br#"
    @prefix ex: <https://example.org/> .
    ex:cat ex:says "meow" .
"#;

// Parse into the frozen, value-interned RDF 1.2 dataset IR.
let ds = parse_dataset(turtle, "text/turtle", None).expect("valid Turtle");
assert_eq!(ds.quad_count(), 1);

// Serialize back out through any native codec — byte-deterministic output.
let nq = serialize_dataset(&ds, "application/n-quads", SerializeGraph::Dataset)
    .expect("serializes");
assert!(String::from_utf8(nq).unwrap().contains("meow"));
```

Malformed input is a typed `RdfDiagnostic` with a source location where the
codec can provide one — never a silent partial parse.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this entire
surface at its root and adds the SPARQL, SHACL, ShEx, and slice modules; depend
on `purrdf-rdf` directly only when you want the RDF layer alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
