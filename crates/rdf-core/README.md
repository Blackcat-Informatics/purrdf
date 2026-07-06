<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-core` — RDF 1.2 Kernel

[![crates.io](https://img.shields.io/crates/v/purrdf-core.svg)](https://crates.io/crates/purrdf-core)
[![docs.rs](https://docs.rs/purrdf-core/badge.svg)](https://docs.rs/purrdf-core)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-core` is the ring-fenced RDF 1.2 kernel of the PurRDF toolkit — the
crate everything else in the workspace builds on. It owns:

- **The immutable, value-interned dataset IR** — `RdfDatasetBuilder` →
  `RdfDataset`, with every term stored once in a string arena addressed by
  copyable `TermId`s, triple terms in object position, reifier/annotation
  side-tables, base-direction literals, and copy-on-write mutation.
- **`DatasetView`** — the static, allocation-free read trait the SPARQL, SHACL,
  ShEx, and entailment engines all evaluate over.
- **Structured diagnostics** — typed `RdfDiagnostic`s with source locations;
  deliberately SARIF-free (the SARIF boundary is
  [`purrdf-validate`](https://crates.io/crates/purrdf-validate)).
- **RDFC-1.0** — W3C dataset canonicalization (`canonicalize`), plus dataset
  diff and isomorphism checks.
- **Store/backend traits** — the narrow parser-ingress, serializer-egress, and
  `SparqlEngine` seams concrete adapters implement in sibling crates.
- **Provenance and the loss ledger** — a generic provenance sidecar for the
  frozen IR and the machine-readable RDF↔GTS loss matrix, plus native FnO and
  SSSOM codecs.

The crate is deliberately dependency-strict: **no oxigraph, no PyO3** — the
whole workspace is oxigraph-free, and this crate is where that guarantee is
structural (a hygiene gate asserts the dependency tree). It is also
`wasm32-unknown-unknown`-clean, and its IR layer is file-IO-free.

## Usage

```sh
cargo add purrdf-core
```

```rust
use purrdf_core::{RdfDatasetBuilder, RdfLiteral};

// Intern terms once; quads are rows of copyable TermIds.
let mut b = RdfDatasetBuilder::new();
let cat = b.intern_iri("https://example.org/cat");
let says = b.intern_iri("https://example.org/says");
let meow = b.intern_literal(RdfLiteral::simple("meow"));
b.push_quad(cat, says, meow, None);

// Freeze into the immutable, indexed dataset the engines evaluate over.
let ds = b.freeze().expect("well-formed dataset");
assert_eq!(ds.quad_count(), 1);
```

Text codecs are *not* here — parsing and serialization live one layer up in
[`purrdf-rdf`](https://crates.io/crates/purrdf-rdf). This split keeps the
kernel small and its invariants enforceable at the crate boundary.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this kernel
through `purrdf-rdf`; depend on `purrdf-core` directly only when you are
building an engine or adapter over the IR itself.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
