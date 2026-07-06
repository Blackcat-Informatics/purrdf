<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-sparql-eval` — Native SPARQL Evaluator

[![crates.io](https://img.shields.io/crates/v/purrdf-sparql-eval.svg)](https://crates.io/crates/purrdf-sparql-eval)
[![docs.rs](https://docs.rs/purrdf-sparql-eval/badge.svg)](https://docs.rs/purrdf-sparql-eval)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-sparql-eval` is the native, RDF 1.2-first **multiset SPARQL evaluator**
of the PurRDF toolkit. It consumes the
[`purrdf-sparql-algebra`](https://crates.io/crates/purrdf-sparql-algebra)
front-end and evaluates over the
[`purrdf-core`](https://crates.io/crates/purrdf-core) IR's `DatasetView`
**entirely in interned `TermId` space** — constants resolve to a dataset id
once, solutions are a single integer compare apart, and computed FILTER/BIND
values that already exist in the dataset are promoted to the interned id at
mint time.

Design pillars:

- **Multiset (bag) semantics** — solutions are a bag, preserved until
  `DISTINCT`/`REDUCED`.
- **Property paths in-engine** — the full path algebra (`* + ? / | ^ !()`)
  evaluated over the same indexed surface, wasm-safe.
- **Query features** — aggregates, `EXISTS` decorrelation, cost-based BGP
  planning (with an `explain_query` introspection API), SPARQL UPDATE, and a
  host-injectable `SERVICE` transport so federation stays wasm-portable.
- **Hard-fail** — an out-of-scope algebra node or unimplemented builtin is a
  typed `EvalError::Unsupported`, never a partial or wrong answer.

The engine is gated by the W3C SPARQL 1.1 conformance suite (run through the
workspace harness), carries zero oxigraph-family dependencies, and builds for
`wasm32-unknown-unknown`.

## Usage

```sh
cargo add purrdf-sparql-eval
```

```rust
use purrdf_core::{RdfDatasetBuilder, RdfLiteral, SparqlEngine, SparqlRequest, SparqlResult};
use purrdf_sparql_eval::NativeSparqlEngine;

// A tiny dataset in interned TermId space.
let mut b = RdfDatasetBuilder::new();
let cat = b.intern_iri("https://example.org/cat");
let says = b.intern_iri("https://example.org/says");
let meow = b.intern_literal(RdfLiteral::simple("meow"));
b.push_quad(cat, says, meow, None);
let ds = b.freeze().expect("freeze");

// Evaluate through the SparqlEngine seam; parsed plans are memoized.
let engine = NativeSparqlEngine::new();
let result = engine.query(&ds, SparqlRequest {
    query: "SELECT ?what WHERE { <https://example.org/cat> <https://example.org/says> ?what }",
    base_iri: None,
    substitutions: &[],
}).expect("evaluates");

if let SparqlResult::Solutions { rows, .. } = result {
    assert_eq!(rows.len(), 1);
}
```

Serialize results to SPARQL JSON/XML/CSV/TSV with the sibling
[`purrdf-sparql-results`](https://crates.io/crates/purrdf-sparql-results) crate.

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
under `purrdf::sparql`; depend on `purrdf-sparql-eval` directly only when you
want the evaluator alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
