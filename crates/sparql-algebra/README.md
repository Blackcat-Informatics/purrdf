<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

# `purrdf-sparql-algebra` — SPARQL 1.1/1.2 Parser & Query Algebra

[![crates.io](https://img.shields.io/crates/v/purrdf-sparql-algebra.svg)](https://crates.io/crates/purrdf-sparql-algebra)
[![docs.rs](https://docs.rs/purrdf-sparql-algebra/badge.svg)](https://docs.rs/purrdf-sparql-algebra)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
[![Repository](https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg)](https://github.com/Blackcat-Informatics/purrdf)

`purrdf-sparql-algebra` is the native SPARQL 1.1/1.2 front-end of the PurRDF
toolkit: a pure-Rust, wasm-clean crate that parses query and update text into a
PurRDF-owned, RDF 1.2-native query algebra (`Query`/`GraphPattern` and
`Update`/`GraphUpdateOperation`). Parse and algebra **only** — evaluation lives
in the downstream [`purrdf-sparql-eval`](https://crates.io/crates/purrdf-sparql-eval)
crate. It builds only on the two zero-dependency foundation leaves
[`purrdf-iri`](https://crates.io/crates/purrdf-iri) and
[`purrdf-xsd`](https://crates.io/crates/purrdf-xsd).

Covered surface:

- **Query** — the four query forms (SELECT/ASK/CONSTRUCT/DESCRIBE), basic graph
  patterns, `OPTIONAL`, `UNION`, `MINUS`, `GRAPH`, `FILTER`/`BIND`/`VALUES`,
  property paths, `GROUP BY`/aggregates, `EXISTS`/`NOT EXISTS`, solution
  modifiers, and RDF 1.2 quoted triple terms (`<<( s p o )>>`).
- **Update** — `INSERT DATA`/`DELETE DATA`, the `DELETE`/`INSERT … WHERE`
  family (`WITH`/`USING`, `DELETE WHERE`), `LOAD`, and
  `CLEAR`/`DROP`/`CREATE`/`ADD`/`MOVE`/`COPY`.

Anything outside this surface — and every malformed query — is a typed
`ParseError`, never a silently-degraded or partial parse.

## Usage

```sh
cargo add purrdf-sparql-algebra
```

```rust
use purrdf_sparql_algebra::{Query, SparqlParser};

let parser = SparqlParser::new().with_base_iri("https://example.org/");
let query = parser.parse_query(
    "PREFIX ex: <https://example.org/>
     SELECT ?food WHERE { ex:cat ex:eats ?food } ORDER BY ?food",
).expect("valid SPARQL");

assert!(matches!(query, Query::Select { .. }));
```

| Module | Responsibility |
| --- | --- |
| `lexer` / `parser` | Tokenize and parse SPARQL text. |
| `ast` | Parsed term, triple, quad, and literal syntax nodes. |
| `algebra` | Evaluable query/update algebra consumed by `purrdf-sparql-eval`. |
| `serialize` | Query-pattern serialization helpers. |
| `error` | Typed parse and unsupported-surface failures. |

## Part of PurRDF

This crate is one member of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace — an RDF 1.2 toolkit with native codecs, SPARQL, SHACL, ShEx,
entailment, and the GTS graph transport, carried into Python, WebAssembly, and
C. Most applications should depend on the umbrella
[`purrdf`](https://crates.io/crates/purrdf) crate, which re-exports this crate
under `purrdf::sparql`; depend on `purrdf-sparql-algebra` directly only when
you want the parser/algebra layer alone.

There are deliberately no Cargo feature flags anywhere in the workspace. MSRV
follows the workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
