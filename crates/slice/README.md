<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-slice` — Slice Catalog for Vocabulary Repositories

[![crates.io](https://img.shields.io/crates/v/purrdf-slice.svg)](https://crates.io/crates/purrdf-slice)
[![docs.rs](https://docs.rs/purrdf-slice/badge.svg)](https://docs.rs/purrdf-slice)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-slice` is the native slice catalog of the PurRDF toolkit: tooling for
ontology/vocabulary repositories that are organized as *slices* — directories
of authored RDF (`slices/<group>/<name>/`) each described by a `manifest.ttl`.
It discovers slice anatomy from manifests, inventories slice-local artifacts
with content-addressed IDs, and exposes ownership and dependency facts to
validation, documentation, mapping, and pipeline tooling.

- **Catalog** — manifest-based discovery (`SliceCatalog::discover`), typed
  slice metadata (`SliceRecord`, `SliceTier`), and artifact roles. Slice
  identity comes from the manifest, not the directory name.
- **Ownership & dependencies** — term-ownership analysis (every declared term
  has exactly one owning slice), dependency edges with evidence, forbidden-edge
  rules (extension slices depend only on core), and machine-applicable fix
  suggestions.
- **Content addressing** — deterministic artifact digests and cache keys for
  incremental pipelines.
- **Emitters** — native support for projection/mapping emitters and lints
  (prefix maps, JSON-LD contexts, FnO function catalogs, claim views).

True to the PurRDF rule that the toolkit mints no vocabulary IRIs, every
term the slice framework reads or emits belongs to the **caller's** vocabulary:
a `SliceVocab` is caller-constructed (it has no `Default`) and threaded through
every public entry point.

## Usage

```sh
cargo add purrdf-slice
```

```rust
use std::path::Path;
use purrdf_slice::{SliceCatalog, SliceVocab};

// Your vocabulary namespace — PurRDF fabricates none.
let vocab = SliceVocab::for_namespace("https://example.org/vocab/");
assert_eq!(vocab.slice_class(), "https://example.org/vocab/Slice");

// Discover every slice under the repository root from its manifest.ttl.
let catalog = SliceCatalog::discover(Path::new("slices"), vocab)
    .expect("slices discovered");
for slice in catalog.records() {
    println!("{} ({:?})", slice.manifest.slice_iri, slice.manifest.tier);
}
```

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
as `purrdf::slice`; depend on `purrdf-slice` directly only when you want the
slice framework alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
