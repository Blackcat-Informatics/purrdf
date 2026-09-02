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
  <em>The RDF 1.2 toolkit with a purr: primitives, codecs, SPARQL, SHACL, ShEx, entailment, full-text search, GeoSPARQL, and graph transport.</em>
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
  N-Quads, RDF/XML, TriX, HexTuples, JSON-LD (star), and YAML-LD; byte-deterministic
  output. Every syntax resolves relative IRI references through the one RFC 3986
  layer in `purrdf::iri` (`BaseIri`/`BaseScope`), and a relative reference with
  no base in scope is a hard error rather than a term.
- **Canonicalization** — W3C RDFC-1.0, tested against the W3C fixture suite.
- **SPARQL 1.1/1.2** — native parser → algebra → multiset evaluator (property
  paths, aggregates, EXISTS/NOT EXISTS answered by a memoized existence probe
  where a prepare-time proof licenses it and by the per-row definition
  otherwise, cost-based BGP planning, the SEP-0009 composite datatypes,
  governed execution), gated by the W3C conformance suites; results in SPARQL
  JSON/XML/CSV/TSV. Caller-keyed extension seams — scalar functions, property
  functions (path witnesses and embedding kNN ship in-crate), custom aggregates,
  and a `SERVICE` resolver with per-service context — under `purrdf::sparql`.
- **Full-text search** — `purrdf::text`: an in-memory inverted index over RDF
  1.2 literals with exact fixed-point BM25 ranking and no floating point, so
  the same query ranks identically natively and on wasm32; reached from SPARQL
  as property functions under the caller's IRIs.
- **GeoSPARQL 1.1** — `purrdf::geo`: exact, float-free WKT/GeoJSON geometry,
  every Simple Features, Egenhofer and RCC8 relation over an exact DE-9IM, the
  `geof:` family on the scalar seam and spatial-relation rewrite on the
  property-function seam, with the OGC vocabulary supplied by the caller.
- **SHACL** — the complete SHACL Core feature set, SHACL-SPARQL constraints and
  targets, and SHACL-AF (node expressions and SHACL Rules, aligned with the
  SHACL 1.2 node-expression and rule-layering drafts), on PurRDF's own engine.
- **ShEx 2.1** — ShExC/ShExJ schemas and shape-map validation, gated against the
  official shexTest suite.
- **Entailment** — Simple / RDF / RDFS / OWL 2 RL / D forward materialization
  (all 78 OWL 2 RL rules of OWL 2 Profiles §4.3 Tables 4–9 — *rule-table
  coverage*, which is not entailment conformance: on this vendored W3C corpus
  of OWL 2 RL entailment tests this chase scores **27 of 27 positive and 23 of 23
  negative**, the latter meaning no unsoundness was found; all 18 RDF + RDFS
  patterns, the four existential ones firing through the restricted chase with
  their surrogate blank nodes withheld at the materialization boundary) plus
  query-directed OWL-Direct and RIF, entirely in interned
  `TermId` space. Every closure comes back with a `ReasoningReport` saying which
  rules fired, which did not, which boundaries the run met, and the contract hash
  of the calculus it ran, and disclosing on an `extension` line the one rule that
  fires without a specification table behind it (`ext-eq-diff-sym`, symmetry of
  `owl:differentFrom` under OWL 2 RL — counted in neither figure above and named by
  `extensions(regime)`). The umbrella `query_with_entailment` façade keeps query
  parsing and the selected regime together; RIF-XML imports stay caller-resolved
  and network-free.
- **GTS graph transport** — a single-file, content-addressed, append-only
  container for RDF 1.2 graphs: BLAKE3-chained CBOR segments, deterministic fold,
  COSE signing/encryption, pure-Rust crypto (wasm-friendly).
- **SARIF reporting** — diagnostics and SHACL reports rendered as byte-deterministic
  SARIF 2.1.0 for editors and CI.
- **Statement-centric visualization** — renderer-neutral RDF 1.2 models and scenes,
  deterministic compact/incidence/table layouts, and semantic SVG with embedded,
  round-trippable assertion, reifier, annotation, graph, and dialect metadata.

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

### The document base

Both RDF legs take the same document base — the trailing `Option<&str>` of
`parse_dataset` and of `serialize_dataset_to_format` — so the Rust surface is exactly
as capable as the Python, WebAssembly, and C ones:

```rust
use purrdf::{parse_dataset, serialize_dataset_to_format, NativeRdfFormat};

let base = "https://example.org/base/";

// Ingress: a relative subject resolves against the base.
let doc = "<rel> <https://example.org/p> <https://example.org/o> .\n";
let ds = parse_dataset(doc.as_bytes(), "text/turtle", Some(base)).unwrap();
assert_eq!(ds.quad_count(), 1);

// Egress: Turtle can express a base, so it is written and relativized against.
let out = serialize_dataset_to_format(&ds, NativeRdfFormat::Turtle, Some(base)).unwrap();
let turtle = String::from_utf8(out.bytes).unwrap();
assert!(turtle.contains("@base <https://example.org/base/> ."));

// With no base in scope a relative reference hard-fails — none is ever fabricated.
let err = parse_dataset(doc.as_bytes(), "text/turtle", None).unwrap_err();
assert_eq!(err.code, "iri-relative-no-base");
```

A syntax that cannot express a base (N-Triples, N-Quads, TriX, HexTuples) emits
absolute IRIs instead of erroring — the format registry decides, once, for the whole
workspace. `purrdf::iri` carries `BaseIri`, `BaseScope`, `BaseOrigin`, and `IriError`
for building and interpreting a base, so naming one never costs a second dependency.

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
| `columnar` | [`purrdf-columnar`](https://crates.io/crates/purrdf-columnar) (five-table Parquet codec) |
| `gts` | [`purrdf-gts`](https://crates.io/crates/purrdf-gts) + the RDF-level GTS adapter |
| `sparql` | [`purrdf-sparql-algebra`](https://crates.io/crates/purrdf-sparql-algebra) + [`purrdf-sparql-eval`](https://crates.io/crates/purrdf-sparql-eval) + [`purrdf-sparql-results`](https://crates.io/crates/purrdf-sparql-results) |
| `shapes` | [`purrdf-shapes`](https://crates.io/crates/purrdf-shapes) (SHACL) |
| `shex` | [`purrdf-shex`](https://crates.io/crates/purrdf-shex) (ShEx 2.1) |
| `entail` | [`purrdf-entail`](https://crates.io/crates/purrdf-entail) (Simple / RDF / RDFS / OWL-RL / D / OWL-Direct / RIF) |
| `geo` | [`purrdf-geo`](https://crates.io/crates/purrdf-geo) (GeoSPARQL 1.1: exact WKT/GeoJSON geometry, the `geof:` family, query rewrite) |
| `text` | [`purrdf-text`](https://crates.io/crates/purrdf-text) (inverted index, exact fixed-point BM25 ranking) |
| `validate` | [`purrdf-validate`](https://crates.io/crates/purrdf-validate) (SARIF 2.1.0 boundary) |
| `slice` | [`purrdf-slice`](https://crates.io/crates/purrdf-slice) (slice catalog) |
| `viz` | RDF 1.2 semantic projection, deterministic layout, and SVG export |
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
