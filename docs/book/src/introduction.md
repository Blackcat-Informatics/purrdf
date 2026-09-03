<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Introduction

**PurRDF** is an [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/) toolkit:
primitives, codecs, SPARQL, SHACL, ShEx, entailment, and graph transport,
implemented once in Rust and carried verbatim into Python, WebAssembly/JavaScript,
and C, with one exception stated up front: the GTS container reaches Rust, the CLI
(as an input format), Python and C, and is not exposed by the wasm/JavaScript
package. Every published crate builds for `wasm32-unknown-unknown`, so the engine
that answers a query on a server answers it, byte for byte, in a browser tab.
It is developed by Blackcat Informatics® Inc. and published under
MIT OR Apache-2.0.

> **One RDF engine. One behavior. Every language.**

**What it removes from your architecture.** The three jobs that usually keep a
PostgreSQL instance running beside a triple store — ranked full-text search,
spatial predicates, and vector similarity — are answered inside PurRDF:
in-process, over the same dataset, from the same SPARQL query, with no second
database and no sync job. Each answer is exact and deterministic, natively and
on wasm32. This is not a projection: these query surfaces have already removed
a whole PostgreSQL requirement from one RDF project. See
[One engine instead of three databases](#one-engine-instead-of-three-databases)
below.

## Why does PurRDF exist?

RDF tooling fragments along two axes.

**Across languages**: every ecosystem has its own parser, with its own bugs, its
own corner-case interpretations, and its own subset of the spec. Move a graph
from a Rust service to a Python pipeline to a browser and you have silently
changed what the data means three times.

**Across time**: RDF 1.2 — triple terms, reifiers, base-direction literals — is
the current revision of the standard, and almost no incumbent library carries it.

PurRDF exists so that a graph is **the same graph everywhere**. It is a
from-scratch, dependency-light Rust core — parser to SPARQL engine to SHACL
validator to binary transport — exposed through native bindings rather than
reimplemented per language.

## What's inside

- **RDF 1.2 primitives** — an immutable, value-interned dataset IR (`TermId`
  space, string arena, copy-on-write mutation), with triple terms in object
  position, reifier/annotation side-tables, and base-direction literals.
  See [The Interned Dataset IR](concepts/interned-dataset.md).
- **Native codecs** — first-party parsers/serializers for Turtle, TriG,
  N-Triples, N-Quads, RDF/XML, TriX, HexTuples, JSON-LD (star), and YAML-LD,
  with byte-deterministic output. See [Codecs & Determinism](concepts/codecs.md).
  Every syntax resolves relative IRI references through one RFC 3986 layer,
  and a relative reference with no base in scope is a hard error.
  See [Base IRIs & Relative References](concepts/base-iris.md).
- **Canonicalization** — W3C RDFC-1.0 on the RDF 1.1 subset; over RDF 1.2
  constructs, either the flat form (RDFC-1.0 over `rdf:reifies` triples) or the
  first-party `purrdf-rdfc12` profile, named apart — plus dataset diff and
  isomorphism. See [Canonicalization & Diff](concepts/canonicalization.md).
- **Projections & carriers** — deterministic LPG, CSVW, OBO Graphs,
  dataset-description, and research-object projections, each with a located
  loss ledger.
  See [Graph, Tabular & Research-Object Projections](concepts/projections.md).
- **SPARQL 1.1/1.2** — native parser → algebra → multiset evaluator, with full
  Update, SEP-0002 temporal arithmetic, `LATERAL` (SEP-0006), the SEP-0008
  SHA-3 hash builtins (`SHA3-224`/`256`/`384`/`512`), quad templates that
  `CONSTRUCT` into named graphs (a first-party extension, not a SPARQL 1.2
  feature), the SEP-0009 composite datatypes (`cdt:List`/`cdt:Map`, `FOLD`,
  `UNFOLD` — with one stated divergence: PurRDF admits RDF 1.2 triple terms
  and directional language-tagged literals as composite elements, a lexical
  superset a conformant SEP-0009 reader will call ill-formed), caller-registered
  aggregates and property functions (including
  path witnesses that bind a traversal hop by hop), governed execution with
  per-node explain receipts, and a `SERVICE` seam — a host-injected resolver
  carrying per-service context; no HTTP client and no resolver ship, so
  `SERVICE` and `LOAD` fail by name on every shipped surface unless written
  `SILENT` — gated by the W3C conformance suites.
  See [SPARQL](sparql/querying.md).
- **SPARQL extensions outside `purrdf-core`** — deterministic full-text search with
  exact fixed-point BM25 ([Full-Text Search](sparql/full-text.md)), exact and
  float-free GeoSPARQL 1.1 ([GeoSPARQL](sparql/geosparql.md)), and
  nearest-neighbour search over a PURREMB embedding space
  ([Embedding Nearest Neighbours](sparql/embedding-knn.md)) — each a consumer
  of the extension seams, registered under IRIs the caller supplies.
- **SHACL and ShEx** — native validators for both shape languages; the SHACL
  engine covers Core, SHACL-SPARQL and SHACL-AF, aligned with the SHACL 1.2
  node-expression and rule-layering drafts. See [Validation](validation/shacl.md).
- **Entailment** — Simple/RDF/RDFS/OWL-RL/D materialization (all 78 OWL 2 RL
  rules implemented — rule-table coverage, distinct from entailment
  conformance, where the OWL 2 RL entailment tests score 27 of 27 positive and
  23 of 23 negative on this vendored W3C corpus), an OWL-Direct tableau, and RIF-Core rules, with a reasoning
  report on every closure. See [Entailment](entailment.md), evaluated on the
  [Datalog fixpoint engine](datalog.md).
- **GTS graph transport** — a single-file, content-addressed, append-only
  container for RDF 1.2 graphs and binary payloads.
  See [GTS Graph Transport](gts.md).
- **Slices, mappings, and provenance** — a slice catalog, an explicit RDF↔GTS
  loss ledger, SSSOM, and FnO. See [Slices, Mappings & Provenance](slices.md).

## One engine instead of three databases

An RDF project that needs ranked text search, spatial predicates, or
nearest-neighbour search has usually run PostgreSQL beside its triple store
for exactly those three jobs. PurRDF answers all three from SPARQL, over the
dataset already in memory, through the evaluator's caller-keyed extension seams
— and each page below opens with what its surface replaces and where it stops.

| You needed | Usually from | Now inside PurRDF | Where it stops |
| --- | --- | --- | --- |
| Ranked full-text search | PostgreSQL `tsvector`/`tsquery` | [Full-Text Search](sparql/full-text.md): `purrdf-text`, an inverted index over RDF 1.2 literals with BM25 ranking in exact `i128` fixed point and no floating point in the crate. | BM25 ranking, not a Lucene: no stemming, no stop-word lists, no query dialect; an in-memory index built once over a frozen dataset. |
| Spatial predicates | PostGIS | [GeoSPARQL](sparql/geosparql.md): `purrdf-geo`, GeoSPARQL 1.1 with WKT and GeoJSON as exact rationals and every Simple Features, Egenhofer and RCC8 relation over an exact DE-9IM; no GEOS, no PROJ. | Topological predicates, accessors and exactly computable measures over vector geometry, not a PostGIS: no CRS transform, no ellipsoidal geodesic, no buffers, no concave hull, no overlay set operations, no raster — each unimplemented function hard-errors by name (`geof:convexHull` is implemented). |
| Vector similarity | pgvector | [Embedding Nearest Neighbours](sparql/embedding-knn.md): exact top-k over a PURREMB embedding space, binary64 in a pinned accumulation order. | Exact scan bounded by a caller-supplied `KnnGuard`, three metrics, no approximate index; PurRDF computes no embeddings — the vectors come from a PURREMB artifact the caller fills, which PurRDF itself writes (`EmbeddingBuilder`, `EmbeddingStreamWriter`; Rust only) and opens fail-closed. |

All three are pure functions of their input on every target — fixed point,
exact rationals, or a pinned binary64 order, with canonical tie-breaks — and
the claim is executed rather than argued: the text and kNN determinism tests
run the same body natively and on `wasm32-unknown-unknown`, and
`make geo-determinism` compares the two targets byte for byte. They are
Rust-host seams: a host registers an index or space under its own IRIs, and
that host may itself be compiled to wasm32. The shipped npm package and Python
wheel do not yet expose these three relations.

## Two design rules worth knowing on day one

**No feature flags — ever.** There are deliberately no Cargo feature flags
anywhere in the workspace, and CI enforces this. A data carrier must not have
optional behavior: optionality changes semantics per consumer, so every
consumer gets the same byte-identical semantics instead.

**PurRDF is a toolkit, not an ontology — it mints no vocabulary IRIs.** Every
vocabulary the library reads or writes is caller-supplied configuration with no
fabricated default. A feature exercised without its vocabulary hard-errors or
stays inactive; it never invents an IRI for you. (Test fixtures use
`example.org`.)

The full invariant list is in
[Design Rules & Invariants](project/design-rules.md).

## What the version number commits to

From 1.0.0 the suite follows semantic versioning in full: a breaking change
bumps the major version, a minor bump is additive, a patch bump is bugfix-only,
and the crates.io, PyPI and npm packages ship one workspace version in
lockstep. The one exception is the C ABI (`purrdf.h`), which carries its own
`0.x` ABI version, bumped on every exported-signature change, and is not
frozen. See [Versioning & Releases](project/releases.md).

## Why RDF 1.2?

RDF 1.2 (and SPARQL 1.2) add first-class statement-level metadata to the data
model: **triple terms** that can appear in object position, **reifiers** that
name occurrences of a triple, and **base-direction literals**
(`rdf:dirLangString`) for bidirectional text. PurRDF treats these as core data
model, not an extension: they flow through the IR, the codecs, SPARQL, SHACL
(a scoped SHACL 1.2 feature), the RDF/JS surface, and the GTS transport.
See [RDF 1.2 Features](concepts/rdf12.md).

## Where PurRDF sits

PurRDF is the library layer of a small family of linked-data projects: it is
the data backbone of the
[GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology) stack and the
reference home of the Rust [GTS](gts.md) engine — but it assumes nothing about
your ontology or application.

## How to read this book

- New users: start with [Getting Started](getting-started/rust.md) in your
  language, then read the [Concepts](concepts/interned-dataset.md) chapters.
- Engine users: jump to [SPARQL](sparql/querying.md),
  [Validation](validation/shacl.md), or [Entailment](entailment.md).
- Integrators: see [Interop](interop/rdflib.md) and
  [GTS Graph Transport](gts.md).
- Contributors: read the [Project](project/design-rules.md) chapters, then
  [AGENTS.md](https://github.com/Blackcat-Informatics/purrdf/blob/main/AGENTS.md)
  in the repository.

API reference documentation lives on [docs.rs/purrdf](https://docs.rs/purrdf);
the repository is
[github.com/Blackcat-Informatics/purrdf](https://github.com/Blackcat-Informatics/purrdf).
