<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf">
    <img src="https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg" alt="PurRDF logo" width="120" height="120">
  </a>
</p>

<h1 align="center"><code>purrdf</code></h1>

<p align="center">
  <em>The RDF 1.2 toolkit with a purr: primitives, codecs, SPARQL, SHACL, ShEx, and graph transport.</em>
</p>

<p align="center">
  <a href="https://crates.io/crates/purrdf"><img src="https://img.shields.io/crates/v/purrdf.svg" alt="crates.io"></a>
  <a href="https://docs.rs/purrdf"><img src="https://docs.rs/purrdf/badge.svg" alt="docs.rs"></a>
  <a href="https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License"></a>
  <a href="https://github.com/Blackcat-Informatics/purrdf"><img src="https://img.shields.io/badge/repo-Blackcat--Informatics%2Fpurrdf-181717.svg" alt="Repository"></a>
</p>

`purrdf` is the umbrella crate of the [PurRDF](https://github.com/Blackcat-Informatics/purrdf)
workspace and the single dependency a downstream needs. It re-exports the RDF 1.2
implementation surface at the root and carries every other published crate under a
stable module (`purrdf::sparql`, `purrdf::shapes`, `purrdf::shex`, `purrdf::gts`,
`purrdf::entail`, `purrdf::validate`, …), so anything a consumer legitimately imports
is reachable from `purrdf` alone — never by reaching into a sub-crate.

## Why does this exist?

RDF tooling fragments along two axes. **Across languages**: every ecosystem has its
own parser with its own bugs and its own subset of the spec, so moving a graph
between a Rust service, a Python pipeline, and a browser silently changes what the
data means. **Across time**: [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/) —
triple terms, reifiers, base-direction literals — is where the standard is going,
and almost no incumbent library carries it.

PurRDF exists so that a graph is **the same graph everywhere**: a from-scratch,
dependency-light Rust core, carried verbatim into Python, WebAssembly/JavaScript,
and C. There are deliberately **no Cargo feature flags** anywhere in the workspace
(CI enforces this) — a data carrier must not have optional behavior, so every
consumer gets the same byte-identical semantics. PurRDF is a toolkit, not an
ontology: it mints no vocabulary IRIs, and domain vocabularies are always
caller-supplied configuration.

## What's inside

- **RDF 1.2 primitives** — an immutable, value-interned dataset IR (`TermId` space,
  string arena, copy-on-write mutation) with triple terms, reifier/annotation
  side-tables, and base-direction literals.
- **Native codecs** — first-party parsers/serializers for Turtle, TriG, N-Triples,
  N-Quads, RDF/XML, JSON-LD (star), and YAML-LD; byte-deterministic output.
- **Canonicalization** — W3C RDFC-1.0, tested against the W3C fixture suite.
- **SPARQL 1.1/1.2** — native parser → algebra → multiset evaluator (property
  paths, aggregates, EXISTS decorrelation, cost-based BGP planning), gated by the
  W3C conformance suite; results in SPARQL JSON/XML/CSV/TSV.
- **SHACL** — the complete SHACL Core feature set plus SHACL-SPARQL constraints
  and targets, on PurRDF's own engine.
- **ShEx 2.1** — ShExC/ShExJ schemas and shape-map validation, gated against the
  official shexTest suite.
- **Entailment** — RDFS / OWL-RL forward materialization plus query-directed
  OWL-Direct and RIF, entirely in interned `TermId` space. The umbrella
  `query_with_entailment` façade keeps query parsing and the selected regime
  together; RIF-XML imports stay caller-resolved and network-free.
- **GTS graph transport** — a single-file, content-addressed, append-only
  container for RDF 1.2 graphs: BLAKE3-chained CBOR segments, deterministic fold,
  COSE signing/encryption, pure-Rust crypto (wasm-friendly).
- **SARIF reporting** — diagnostics and SHACL reports rendered as byte-deterministic
  SARIF 2.1.0 for editors and CI.

## Quickstart

```sh
cargo add purrdf
```

```rust
use purrdf::{parse_dataset, serialize_dataset, RdfDatasetBuilder, RdfLiteral, SerializeGraph};

// Build a dataset in interned TermId space.
let mut b = RdfDatasetBuilder::new();
let alice = b.intern_iri("https://example.org/alice");
let knows = b.intern_iri("http://xmlns.com/foaf/0.1/knows");
let bob = b.intern_iri("https://example.org/bob");
let name = b.intern_iri("http://xmlns.com/foaf/0.1/name");
let hi = b.intern_literal(RdfLiteral::simple("Alice"));
b.push_quad(alice, knows, bob, None);
b.push_quad(alice, name, hi, None);
let ds = b.freeze().expect("freeze");

// Serialize to any native codec and parse back, losslessly.
let ttl = serialize_dataset(&ds, "text/turtle", SerializeGraph::Dataset).unwrap();
let back = parse_dataset(&ttl, "text/turtle", None).unwrap();
assert_eq!(back.quad_count(), 2);
```

Every engine is reachable through the same facade — for example ShEx and IRI
handling:

```rust
let iri = purrdf::iri::parse("https://example.org/cat").expect("valid IRI");
assert_eq!(iri.as_str(), "https://example.org/cat");

let schema = purrdf::shex::parse_shexc(
    "PREFIX ex: <https://example.org/>\nex:Cat { ex:says . }",
    None,
).expect("valid ShExC");
```

## Module map

| Module | Sub-crate(s) |
| --- | --- |
| (root) | [`purrdf-rdf`](https://crates.io/crates/purrdf-rdf) — core types, codecs, GTS/text adapters |
| `gts` | [`purrdf-gts`](https://crates.io/crates/purrdf-gts) + the RDF-level GTS adapter |
| `sparql` | [`purrdf-sparql-algebra`](https://crates.io/crates/purrdf-sparql-algebra) + [`purrdf-sparql-eval`](https://crates.io/crates/purrdf-sparql-eval) + [`purrdf-sparql-results`](https://crates.io/crates/purrdf-sparql-results) |
| `shapes` | [`purrdf-shapes`](https://crates.io/crates/purrdf-shapes) (SHACL) |
| `shex` | [`purrdf-shex`](https://crates.io/crates/purrdf-shex) (ShEx 2.1) |
| `entail` | [`purrdf-entail`](https://crates.io/crates/purrdf-entail) (RDFS / OWL-RL / OWL-Direct / RIF) |
| `validate` | [`purrdf-validate`](https://crates.io/crates/purrdf-validate) (SARIF 2.1.0 boundary) |
| `slice` | [`purrdf-slice`](https://crates.io/crates/purrdf-slice) (slice catalog) |
| `iri` / `xsd` / `events` | the zero-dependency foundation leaves |

The same engine ships to [PyPI](https://pypi.org/project/purrdf/) (`pip install purrdf`)
and [npm](https://www.npmjs.com/package/@blackcatinformatics/purrdf) as an RDF/JS-shaped
wasm package, plus a `libpurrdf` C ABI — all released in lockstep from one workspace
version.

## Part of PurRDF

Full documentation, conformance scoreboards (W3C SPARQL, SHACL, shexTest, RDFC-1.0,
frozen GTS vectors), benchmarks, and the crate map live in the
[PurRDF repository](https://github.com/Blackcat-Informatics/purrdf). MSRV follows the
workspace `rust-version` (currently 1.96, stable toolchain only).

## License

Licensed under either of

- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)

at your option.
