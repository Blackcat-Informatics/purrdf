<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-sparql-results` — SPARQL Results Serialization

[![crates.io](https://img.shields.io/crates/v/purrdf-sparql-results.svg)](https://crates.io/crates/purrdf-sparql-results)
[![docs.rs](https://docs.rs/purrdf-sparql-results/badge.svg)](https://docs.rs/purrdf-sparql-results)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-sparql-results` is the results boundary of the PurRDF SPARQL stack: the
canonical authority for turning a `SparqlResult` (SELECT solutions, ASK
boolean, or CONSTRUCT graph) into the four W3C SPARQL Results formats — JSON
(SRJ), XML, CSV, and TSV — plus an additive, provenance-carrying PurRDF
extension where the format can carry one. JSON and XML documents can also be
read back (`from_json`, `from_xml`).

Behavior worth knowing before you pick a format:

- **Byte-deterministic output** — the same result always serializes to the
  same bytes.
- **The support matrix is enforced, not fudged** — XML rejects CONSTRUCT
  graphs, and CSV/TSV reject both ASK booleans and CONSTRUCT graphs, each as a
  typed `Error::Format`.
- **Lossy projections are flagged** — CSV/TSV have no extension point, so a
  populated provenance is trimmed at the exit gate and
  `SerializeOutcome::provenance_dropped` is set; the drop is never silent.

The crate depends only on [`purrdf-core`](https://crates.io/crates/purrdf-core)
and stays wasm-clean; term and N-Triples syntax come exclusively from the
kernel's emit primitives, so there is one term-syntax authority in the
workspace.

## Usage

```sh
cargo add purrdf-sparql-results
```

```rust
use purrdf_sparql_results::{serialize, ResultProvenance, SparqlResultsFormat};

// `result` is the SparqlResult produced by purrdf-sparql-eval (or any engine
// implementing the purrdf-core SparqlEngine seam).
let outcome = serialize(&result, SparqlResultsFormat::Json, &ResultProvenance::default())
    .expect("SELECT serializes to SRJ");

assert!(!outcome.provenance_dropped);
let json = String::from_utf8(outcome.bytes).unwrap();
```

Per-format writers (`to_json`, `to_xml`, `to_csv`, `to_tsv`) and readers
(`from_json`, `from_json_boolean`, `from_xml`, `from_xml_boolean`) are also
exported directly.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
under `purrdf::sparql`; depend on `purrdf-sparql-results` directly only when
you want the results boundary alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
