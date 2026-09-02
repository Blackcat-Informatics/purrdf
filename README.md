<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->
<p align="center"><a href="./README.zh-Hans.md">简体中文</a></p>

<p align="center">
  <a href="https://blackcatinformatics.ca/purrdf/">
    <img src="./docs/purrdf-logo.svg" alt="PurRDF logo — a black cat holding an RDF triple" width="128" height="128">
  </a>
</p>

<h1 align="center">PurRDF</h1>

<p align="center">
  <em>The RDF 1.2 toolkit with a purr: primitives, codecs, SPARQL, SHACL, ShEx, entailment, full-text search, GeoSPARQL, and graph transport.</em>
</p>

<p align="center">
  <strong>One RDF engine. One behavior. Every language.</strong>
</p>

<p align="center">
  <a href="https://github.com/Blackcat-Informatics/purrdf/actions/workflows/ci.yaml"><img src="https://github.com/Blackcat-Informatics/purrdf/actions/workflows/ci.yaml/badge.svg" alt="CI"></a>
  <a href="https://crates.io/crates/purrdf"><img src="https://img.shields.io/crates/v/purrdf.svg?label=crates.io" alt="crates.io"></a>
  <a href="https://pypi.org/project/purrdf/"><img src="https://img.shields.io/pypi/v/purrdf.svg?label=PyPI" alt="PyPI"></a>
  <a href="https://www.npmjs.com/package/@blackcatinformatics/purrdf"><img src="https://img.shields.io/npm/v/%40blackcatinformatics%2Fpurrdf.svg?label=npm" alt="npm"></a>
  <a href="https://doi.org/10.67342/pkg8gpp4no/v1"><img src="https://img.shields.io/badge/DOI-10.67342%2Fpkg8gpp4no%2Fv1-blue" alt="DOI: 10.67342/pkg8gpp4no/v1"></a>
  <a href="./LICENSING.md"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0"></a>
  <img src="https://img.shields.io/badge/MSRV-1.96-orange.svg" alt="MSRV 1.96">
</p>

<p align="center">
  <a href="https://blackcat-informatics.github.io/purrdf/playground/"><img src="https://img.shields.io/badge/RDF--1.2%20playground-try%20it%20live-brightgreen" alt="Try the RDF-1.2 playground in your browser"></a>
</p>

---

PurRDF is an [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/) toolkit —
primitives, codecs, SPARQL 1.1/1.2, SHACL, ShEx, entailment regimes, and the
GTS graph transport — implemented once in Rust and carried verbatim into
Python, WebAssembly/JavaScript, and C. Every published crate builds for
`wasm32-unknown-unknown`, so the engine that answers a query on a server
answers it, byte for byte, in a browser tab.

**What it removes from your architecture.** The three jobs that usually keep
a PostgreSQL instance running beside a triple store — ranked full-text search,
spatial predicates, and vector similarity — are answered inside PurRDF:
in-process, over the same dataset, from the same SPARQL query, with no second
database, no sync job, and no question split between SPARQL and SQL. Each
answer is exact and deterministic — the same rows, in the same order, with the
same score lexicals, natively and on wasm32 — which is a guarantee a Postgres
stack does not make across machines. This is not a projection: these query
surfaces have already removed a whole PostgreSQL requirement from one RDF
project. The verified capability table, the boundary of each surface, and an
architectural before/after are in
[One engine instead of three databases](#one-engine-instead-of-three-databases).

## Why does this exist?

RDF tooling fragments along two axes.

**Across languages**: every ecosystem has its own parser, with its own bugs, its own
corner-case interpretations, and its own subset of the spec. Move a graph from a Rust
service to a Python pipeline to a browser and you have silently changed what the data
means three times.

**Across time**: [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/) — triple terms,
reifiers, base-direction literals — is the current revision of the standard, and
almost no incumbent library carries it.

PurRDF exists so that a graph is **the same graph everywhere**. It is a from-scratch,
dependency-light Rust core — parser to SPARQL engine to SHACL validator to binary
transport — carried verbatim into Python, WebAssembly/JavaScript, and C. There are
deliberately **no Cargo feature flags** anywhere in the workspace (CI enforces this):
a data carrier must not have optional behavior, so every consumer gets the same
byte-identical semantics.

PurRDF is the data backbone of the [GMEOW](https://github.com/Blackcat-Informatics/gmeow-ontology)
stack and the reference home of the [GTS](./docs/GTS-SPEC.md) graph-transport engine,
but it assumes nothing about your ontology or application.

## One engine instead of three databases

An RDF project that needs ranked text search, spatial predicates, or
nearest-neighbour search has usually run PostgreSQL beside its triple store for
exactly those three jobs. PurRDF answers all three from SPARQL, over the
dataset already in memory, through the evaluator's caller-keyed
property-function and scalar-function seams. Every capability below is one you
can open a crate and find; every boundary is one its tests pin.

| You needed | Usually from | Now inside PurRDF | Where it stops |
| --- | --- | --- | --- |
| Ranked full-text search | PostgreSQL `tsvector`/`tsquery` | [`purrdf-text`](./crates/text/): an inverted index over RDF 1.2 literals (annotation layer included), Unicode case folding and word-boundary segmentation, and BM25 ranking in exact `i128` base-10 fixed point with **no floating point in the crate** (`#![deny(clippy::float_arithmetic)]`). Two relations: `?doc <iri> ( "needle" ?score ?rank ?lang ?matched )` for ranked retrieval and `?doc <iri> ( "term" ?lang ?position )` for term occurrence, from which phrase and proximity compose in plain SPARQL. | BM25 ranking, not a Lucene: no stemming, no stop-word lists, no query dialect; `k1` and `b` are fixed constants; the index is in-memory and built once over a frozen dataset (`TextIndex::from_dataset`). |
| Spatial predicates | PostGIS | [`purrdf-geo`](./crates/geo/): GeoSPARQL 1.1 (OGC 22-047r1) — WKT and GeoJSON literals read as exact rationals, every Simple Features, Egenhofer and RCC8 relation decided over an exact DE-9IM, the `geof:` functions on the scalar seam and the Query Rewrite relations (`?a geo:sfWithin ?b` between features) on the property-function seam. No GEOS, no PROJ, no float arithmetic; the one float boundary is the `xsd:double` result literal. | Topological predicates, accessors, and the exactly computable measures and constructors over vector geometry, not a PostGIS: `geof:transform` hard-errors by name (there is no CRS database), a `metric*` measure answers only in a CRS the caller declared in metres (there is no ellipsoidal geodesic), and `buffer`, `concaveHull`, `boundingCircle`, the overlay set operations (`intersection`/`union`/`difference`/`symDifference`) and the GML/KML/DGGS encodings are registered and hard-error by name. No raster, no persistent spatial index. |
| Vector similarity | pgvector | Embedding kNN in [`purrdf-sparql-eval`](./crates/sparql-eval/): `?neighbour <space> ( ?seed k ?distance )` over a [PURREMB](./docs/PURREMB.md) embedding space (`EmbeddingSpace::from_artifact`, `EmbeddingKnnRelation`), exact top-k under the metric the artifact declares, in binary64 with a pinned accumulation order and no fused multiply-add, ties broken by content-derived row order. | Exact search — every candidate is scored, there is no pruning and no approximate index — bounded by a caller-supplied `KnnGuard` (largest space, largest `k`; refusals, not clamps). Three metrics: cosine, negative dot, squared Euclidean. PurRDF computes no embeddings and runs no ANN payload: the vectors arrive in a PURREMB artifact the caller produced. |

Two more capabilities landed on the same seam in this release: **path
witnesses**, a property function that binds a traversal hop by hop with every
traversed statement as an RDF 1.2 triple term, and the **SEP-0009 composite
datatypes** (`cdt:List`/`cdt:Map`, with `FOLD` and `UNFOLD`). One divergence in
the latter is stated rather than hidden: PurRDF admits RDF 1.2 triple terms and
directional language-tagged literals as composite elements, a lexical superset
that a conformant SEP-0009 reader will call ill-formed, emitted only for values
SEP-0009 cannot express at all.

**Deterministic, and therefore portable.** A Postgres stack gives one answer
per build: `ts_rank` and pgvector distances are floating point, and PostGIS
predicates run on GEOS's floating-point geometry. PurRDF's three surfaces are pure
functions of their input on every target — BM25 in `i128` fixed point with a
fixed-iteration integer logarithm, geometry as exact rationals with integer
DE-9IM decisions, kNN in binary64 with one sequential accumulation order — and
every ordering is canonical: document ids are assigned after sorting on
`(graph, subject, language)`, spatial rows sort in `TermValue`'s total order,
and kNN ties break on the content-derived `TargetId`. The claim is executed,
not argued: the text and kNN determinism tests are one body carrying both
`#[test]` and `#[wasm_bindgen_test]`, run natively by `cargo test` and on
`wasm32-unknown-unknown` by `make wasm-test`, and `make geo-determinism` runs
the same corpus on both targets and compares bytes.

**In the browser.** All three crates are `wasm32-unknown-unknown`-clean and
their determinism tests execute there — which none of the three Postgres
extensions can do at all. They are Rust-host seams: a host builds an index or
opens a space and registers it under its own IRIs, and that host may itself be
compiled to wasm32 (that is how the wasm tests run). The shipped
`@blackcatinformatics/purrdf` npm package and the `purrdf` Python wheel do not
yet expose these three relations; the data-shaped property functions (frozen
tables, graph-backed tables, path witnesses) are what cross those boundaries
today.

**Illustrative before/after** (the project is real; it is not named here):

- *Before* — a triple store for the graph, and a PostgreSQL instance beside it
  for `tsvector`/`tsquery` over labels and abstracts, PostGIS
  `ST_Within`/`ST_Intersects` over feature geometries, and pgvector `<->` over
  document embeddings: three copies of the data, one sync job to keep them
  aligned, and every question that spanned them written half in SPARQL and
  half in SQL.
- *After* — one PurRDF dataset; one `PropertyFunctionRegistry` holding a
  `TextSearchRelation`, the `geo:` Query Rewrite relations, and an
  `EmbeddingKnnRelation`, each under the project's own IRIs; one SPARQL query
  joining all three through basic graph patterns; and no PostgreSQL. The answer
  is the same on the server and in the browser.

```sparql
PREFIX ex:  <https://example.org/>
PREFIX geo: <http://www.opengis.net/ont/geosparql#>

SELECT ?doc ?score ?distance WHERE {
  ?doc ex:search ( "harbour dredging" ?score ?rank ?lang ?matched ) .
  ?doc ex:locatedIn ?feature .
  ?feature geo:sfWithin ex:PortDistrict .
  ?doc ex:nearest ( ex:doc-42 5 ?distance )
}
ORDER BY ?rank
```

Each predicate above is the caller's: PurRDF mints no vocabulary, so
`ex:search`, `ex:nearest` and the CRS behind `geo:sfWithin` are registrations
the host supplies, and a query naming an IRI nobody registered is an ordinary
triple pattern.

## What's inside

- **RDF 1.2 primitives** — an immutable, value-interned dataset IR (`TermId` space,
  string arena, copy-on-write mutation), with triple terms in object position,
  reifier/annotation side-tables, and base-direction literals (`rdf:dirLangString`).
- **Native codecs** — first-party parsers/serializers for **Turtle, TriG, N-Triples,
  N-Quads, RDF/XML, TriX, HexTuples, JSON-LD (star), and YAML-LD**, plus bidirectional
  OKF Markdown bundles with caller-supplied vocabulary; byte-deterministic output.
- **One base-resolution layer** — every codec, SPARQL, ShEx and SHACL resolve a
  relative IRI reference through the single RFC 3986 implementation in
  `purrdf-iri` (`BaseIri`/`BaseScope`), on RFC 3986 §5.1's precedence chain: an
  in-document directive (`@base`/`BASE`/`xml:base`/`@context.@base`), else a
  caller-supplied base, else the document's retrieval IRI (the `file://` IRI of a
  file the CLI opened), else the hard error `iri-relative-no-base`. A relative
  IRI never enters a graph unresolved, and syntaxes that can express a base
  (Turtle, TriG, RDF/XML, JSON-LD, YAML-LD) write and relativize against one on
  the way out. See [Base IRIs & Relative References](./docs/book/src/concepts/base-iris.md).
- **Canonicalization** — W3C **RDFC-1.0** dataset canonicalization, tested against the
  W3C fixture suite.
- **Projections & carriers** — deterministic graph, tabular, and
  research-object projections: a canonical LPG model, W3C-gated CSVW
  (**270/270** RDF conversions, **282/282** validation cases), OBO Graphs and
  SKOS views, native DCAT/VoID dataset descriptions, and RO-Crate 1.3 /
  Croissant 1.1 / DataCite 4.6 / DCAT 3 / Frictionless carriers — every lossy
  step reported through a located loss ledger — plus the five-table
  byte-deterministic Parquet codec in `purrdf-columnar`.
- **SPARQL 1.1/1.2** — native parser → algebra → multiset evaluator over the
  interned IR: all four query forms plus full SPARQL Update, property paths,
  cost-based BGP planning, and the enforced `VERSION` declaration (including
  the `1.2-basic` profile). `EXISTS`/`NOT EXISTS` runs on SEP-0007's
  defensible substitution semantics (`Replace`/`PrjMap`, a JOIN rather than a
  term rewrite, plus its Part 3 assignment restriction), answered by a
  memoized existence probe where a prepare-time proof licenses it and by the
  per-row definition otherwise. The 1.2 surface includes temporal arithmetic
  (SEP-0002: instants, durations, and the five Gregorian partial-date types,
  plus duration `SUM`/`AVG` and `ADJUST`) and `LATERAL` (SEP-0006, with
  Jena's scope rule), the SEP-0008 SHA-3 builtins, and the SEP-0009 composite
  datatypes (`cdt:List`/`cdt:Map`, the fifteen-function library, the `FOLD`
  aggregate and the `UNFOLD` graph pattern, evaluated in the closed-leaf
  `purrdf-cdt` crate and gated by the vendored `awslabs/SPARQL-CDTs` corpus).
  One deliberate divergence is stated rather than hidden: PurRDF admits RDF 1.2
  triple terms and directional language-tagged literals as composite elements,
  a lexical superset that a conformant SEP-0009 reader will call ill-formed,
  emitted only for values SEP-0009 cannot express at all. Three caller-keyed
  extension seams — scalar functions, property functions (magic predicates),
  and custom aggregates via `AGG(<iri>, …)` with a ten-aggregate statistical
  set under a caller-supplied namespace — plus `SERVICE` federation through a
  host-injectable `ServiceResolver` that carries **per-service context**
  (headers, credentials, timeouts, capabilities; deny by default) and whose
  wire format is the deterministic serializer, round-trip-swept over the
  823-item vendored corpus (update requests included). A host scalar function
  on the native seam carries SPARQL's expression-error channel: a per-solution
  domain error eliminates the row under `FILTER` or leaves the variable unbound
  under `BIND`/`SELECT` instead of aborting the query. Gated by the full W3C
  SPARQL 1.1 + 1.2 evaluation corpus: **862 passing**, 5 ledgered
  upstream-errata fixtures. Results in SPARQL JSON/XML/CSV/TSV.
- **Out-of-core SPARQL extensions** — capability that arrives through those
  seams as sibling crates, each registered under IRIs the caller supplies (PurRDF
  mints none) and each byte-identical natively and on `wasm32-unknown-unknown`:
  - **Full-text search** (`purrdf-text`) — an in-memory inverted index over RDF
    1.2 literals (the annotation layer included), Unicode normalization and
    full case folding (UAX 15, UAX 21) followed by word-boundary segmentation
    (UAX 29), and BM25 ranking in exact
    base-10 fixed point with **no floating point in the crate** (denied by
    lint), so the ranking is a pure function of its input on every target. Two
    property functions: ranked retrieval
    (`?doc <iri> ( "needle" ?score ?rank ?lang ?matched )`) and term
    occurrence (`?doc <iri> ( "term" ?lang ?position )`), from which phrase and
    proximity queries compose in plain SPARQL.
  - **GeoSPARQL 1.1** (`purrdf-geo`, OGC 22-047r1) — WKT and GeoJSON literals
    read as exact rationals, every topological relation of the Simple Features,
    Egenhofer and RCC8 families over an exact DE-9IM, the accessors and the
    exactly-computable measures and constructors, with no GEOS, no PROJ and no
    float arithmetic (the one float boundary is the `xsd:double` result
    literal). The `geof:` family lands on the scalar seam and the spatial
    relations rewrite through the property-function seam; `geof:transform`,
    the buffers, the concave hull, the overlay set operations and the
    GML/KML/DGGS encodings are registered but **hard-error by name** rather
    than answering a default (`geof:convexHull` is implemented), and
    a `metric*` measure answers only in a CRS the caller declared in metres
    (there is no ellipsoidal geodesic).
  - **Embedding kNN** — nearest-neighbour search over a
    [PURREMB](./docs/PURREMB.md) embedding space as a property function
    (`?neighbour <space> ( ?seed k ?distance )`): an exact search under the
    metric the artifact declares, binary64 in a pinned accumulation order, and
    governor charges proportional to the candidates actually scanned.
  - **Path witnesses** — a property function that binds the *derivation* of a
    traversal, not just its endpoints:
    `?start <iri> ( ?end ?pathId ?len ?step ?node ?edge )`, one row per hop,
    with every traversed statement an RDF 1.2 triple term that joins straight
    back into the dataset; every simple-prefix walk or one shortest witness per
    pair, a content-derived path identifier, and hop limits the caller must
    state. Reachable from the CLI (`--path-relation`) and Python
    (`path_relations`); the reference vectors were re-executed against a real
    Virtuoso `OPTION(TRANSITIVE …)` instance, not transcribed from its manual.
- **Governed execution** — every query/update entry point has a governed twin
  running under caller-set ceilings (fuel, answer rows, intermediate cells,
  scratch bytes, remote requests, deadline) that trips with certified rows
  rather than a wrong answer, and `--explain` returns a per-algebra-node charge
  ledger beside the cost planner's estimates. The normative charge schedule and
  the frozen 50-case governor corpus live in
  [`docs/SPARQL-GOVERNOR-PROFILE.md`](./docs/SPARQL-GOVERNOR-PROFILE.md).
- **SHACL validation** — a native validator with the complete SHACL Core feature
  set (all constraint components, full property paths, qualified value shapes,
  property pairs), SHACL-SPARQL constraints/targets on the native engine, the
  complete SHACL-AF surface (node expressions, expression constraints,
  user-defined SPARQL functions and target types, and SHACL Rules materialized
  as a new dataset), aligned with the SHACL 1.2 Node Expressions
  (`shnex:`), SPARQL Extensions and SPARQL 1.2 RL Working Drafts — both the AF
  and the 1.2 spelling of a node expression parse to one representation, and
  rules run as `sh:order` strata with `once`/`general` partitioning — plus
  scoped SHACL 1.2 support for reifier shapes. None of that is a claim of full
  SHACL 1.2 conformance. **129/129 passing** on the vendored W3C test suite,
  zero ledgered. The answer is the W3C validation report as a frozen RDF
  dataset (`ValidationReport::to_dataset()`), so any syntax — and the CLI's
  `validate --format` — is a serialization of that dataset rather than a text
  round-trip, with the report's minted blank nodes kept distinct from every
  blank node the data graph carries.
- **ShEx 2.1** — a from-scratch ShExC + ShExJ schema layer and validator gated
  against the official shexTest suite: **1,105/1,105 attempted validation tests,
  zero expected-failures** (imports and semantic actions included), 99/99 negative
  syntax, 14/14 negative structure. See [`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md).
- **Entailment** — Simple/RDF/RDFS/OWL-RL/D forward materialization over a
  deterministic semi-naive fixpoint (**all 78 OWL 2 RL rules** of OWL 2 Profiles
  §4.3 Tables 4–9 — *rule-table coverage*, which is not the same claim as
  entailment conformance: on this vendored W3C corpus of OWL 2 RL entailment
  tests the chase scores **27 of 27 positive and 23 of 23 negative**, the latter meaning no
  unsoundness was found; all 18 RDF + RDFS patterns, the four existential ones
  firing through the restricted chase with their surrogate blank nodes withheld at
  the materialization boundary), an open-world OWL-Direct
  SHOIQ(D) hypertableau, and RIF-Core rules. **Every closure comes back with a reasoning
  report** naming what fired, what did not, the boundaries met, the budget
  consumed, and the contract hash of the calculus that ran — so an incomplete
  answer can never be delivered as a complete one. One rule fires that no
  specification table states — `ext-eq-diff-sym`, symmetry of `owl:differentFrom`
  under `owl-rl` — and it is in neither rule count above; `extensions(regime)`
  names it, and every report discloses it on an `extension` line. Per-rule
  inventory:
  [`docs/book/src/entailment-rules.md`](./docs/book/src/entailment-rules.md).
- **GTS graph transport** — a single-file, content-addressed, append-only container
  for RDF 1.2 graphs and the binaries they reference: BLAKE3-chained CBOR segments,
  deterministic fold, COSE signing/encryption, pure-Rust crypto (wasm-friendly).
  Spec in [`docs/GTS-SPEC.md`](./docs/GTS-SPEC.md), frozen cross-language conformance
  vectors in [`vectors/`](./vectors/).
- **Slices, mappings, and provenance** — a manifest-based slice catalog with
  content-addressed artifact IDs, an explicit RDF↔GTS **loss ledger**
  ([`generated/rdf-loss-matrix.json`](./generated/rdf-loss-matrix.json)), SSSOM
  mapping TSV support, and an FnO function-catalog codec.
- **Zero-dependency foundations** — `purrdf-iri` (RFC 3987/3986) and `purrdf-xsd`
  (XSD 1.1 value space) have no runtime dependencies at all; `purrdf-events` (the
  object-safe ingestion seam) has none either, and `purrdf-cdt` is a `no_std`
  closed leaf over exactly those two.

## Quickstart

### Rust

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

### Python

```sh
pip install purrdf
```

```python
import purrdf

quads = purrdf.parse(
    '<https://example.org/alice> <http://xmlns.com/foaf/0.1/name> "Alice" .',
    purrdf.RdfFormat.TURTLE,
)

from purrdf import shapes, shex

report = shapes.validate(shapes_ttl=my_shapes, data_nt=my_data)
print(report["conforms"])

results = shex.validate(my_schema_shexc, my_data_ttl,
                        [("https://example.org/alice", "https://example.org/PersonShape")])
print(all(entry["conformant"] for entry in results))
```

Every parse entry point takes an optional `base=` for a document that spells
relative IRIs (`shapes.validate` takes `shapes_base=`); with none in scope a
relative reference raises rather than being mis-parsed. `Store.query` and its
governed/update twins register property functions as data — frozen tables,
tables read from the store's own graph, and `path_relations` traversals — with
the GIL released for the whole evaluation.

The Python package also ships an [rdflib compatibility layer](./bindings/python/python/src/purrdf/compat/rdflib/)
(`from purrdf.compat.rdflib import Graph`) and GTS relational exports
(`gts_to_sqlite`, `gts_to_duckdb`, `gts_to_parquet`).

For a literal, zero-change `import rdflib`, install the opt-in extra:

```bash
pip install purrdf[rdflib]
```

This pulls in the separate [`purrdf-rdflib`](./bindings/python-rdflib-shadow/)
distribution, whose top-level `rdflib` package re-exports the compat surface, so
existing third-party code doing `import rdflib` / `from rdflib.namespace import RDF`
transparently runs on purrdf. **Caveat:** that shadow claims the `rdflib` import
name and must never be installed alongside the genuine
[`rdflib`](https://pypi.org/project/rdflib/) — the two cannot co-inhabit one
environment. It is a separate distribution (never bundled into the main `purrdf`
wheel) precisely so environments that need the real rdflib simply omit it.

### JavaScript / WebAssembly

An [RDF/JS](https://rdf.js.org/)-shaped API (`DataFactory` / `Dataset` / `Stream`)
over the same engine, including the RDF 1.2 features no incumbent RDF/JS library
carries — quoted triple terms and base-direction literals:

```js
import { ready, DataFactory, Dataset } from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const f = new DataFactory();
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");

const ds = new Dataset();
ds.add(f.quad(f.namedNode("https://ex/s"), f.namedNode("https://ex/says"), rtl));

const nq = ds.serialize("nquads");           // directions survive the round-trip
const reparsed = Dataset.parse(nq, "nquads"); // Dataset.parse(input, format, base?)
```

The same browser bundle also exposes SHACL validation (`shaclValidateToSarif`,
`shaclEntail`, each taking an optional `shapesBase`), entailment-regime
materialization, governed SPARQL with explain receipts, and RDFC-1.0 graph
identity (`Dataset.canonicalize()`, `Dataset.isomorphic()`). See
[`crates/rdf-wasm`](./crates/rdf-wasm/) (`make wasm-pkg` builds the ESM
package).

### C

`libpurrdf` ([`crates/rdf-capi`](./crates/rdf-capi/)) exposes parse, serialize,
pattern iteration, copy-on-write mutation, SPARQL, SHACL validation/entailment,
and GTS round-trips behind a panic-safe C ABI with a committed, reproducible
header ([`include/purrdf.h`](./crates/rdf-capi/include/purrdf.h)) that CI checks
for drift. Built with cargo-c: `make capi-build`.

## Crate map

| Crate | What it is |
| --- | --- |
| [`purrdf`](./crates/purrdf/) | Umbrella facade: the RDF surface at the root, `slice` and `shapes` as modules. Start here. |
| [`purrdf-rdf`](./crates/rdf/) | RDF 1.2 implementation: native codecs, GTS adapters, describe, canonicalization entry points. |
| [`purrdf-core`](./crates/rdf-core/) | The kernel: interned IR, diagnostics, store traits, provenance, loss ledger, RDFC-1.0. |
| [`purrdf-columnar`](./crates/columnar/) | Bidirectional, byte-deterministic five-table Parquet codec for RDF 1.2 and content-addressed blobs. |
| [`purrdf-gts`](./crates/gts/) | GTS container engine: reader, writer, fold, verify, COSE sign/encrypt. |
| [`purrdf-sparql-algebra`](./crates/sparql-algebra/) | SPARQL 1.1/1.2 parser → query algebra AST. |
| [`purrdf-sparql-eval`](./crates/sparql-eval/) | Multiset SPARQL evaluator in interned `TermId` space, with the caller-keyed extension seams (scalar functions, property functions — including the path-witness and embedding-kNN relations — custom aggregates, and the per-service `ServiceResolver`) and the execution governors. |
| [`purrdf-sparql-results`](./crates/sparql-results/) | SPARQL results JSON/XML/CSV/TSV, plus a provenance-carrying extension. |
| [`purrdf-cdt`](./crates/cdt/) | SEP-0009 SPARQL composite datatypes (`cdt:List`/`cdt:Map`): the value space, an iterative bounded lexical scanner, canonical spelling, and the fifteen-function library. A `no_std` closed leaf over `purrdf-iri` + `purrdf-xsd`; reached through the evaluator, not re-exported by the umbrella. |
| [`purrdf-shapes`](./crates/shapes/) | SHACL validation engine (full Core + SHACL-SPARQL + SHACL-AF, including SHACL Rules). |
| [`purrdf-shex`](./crates/shex/) | ShEx 2.1: ShExC/ShExJ schemas and validation. |
| [`purrdf-entail`](./crates/entail/) | Entailment regimes: the RDF/RDFS/OWL-RL/D chase, an OWL-Direct tableau, and RIF-Core rules — each closure returned with a reasoning report. |
| [`purrdf-geo`](./crates/geo/) | GeoSPARQL 1.1: exact, float-free WKT and GeoJSON geometry, the `geof:` function family over the scalar seam, and feature-level query rewrite over the property-function seam — all under caller-supplied IRIs. |
| [`purrdf-datalog`](./crates/datalog/) | The fixpoint substrate beneath the chase: a columnar relation store and a deterministic semi-naive evaluator over the DL-clause IR. Not re-exported by the umbrella. |
| [`purrdf-text`](./crates/text/) | Deterministic full-text search over RDF 1.2 literals: an in-memory inverted index and exact fixed-point BM25 ranking, reached from SPARQL through caller-supplied property-function IRIs. |
| [`purrdf-validate`](./crates/validate/) | The shared host boundary: SARIF 2.1.0 diagnostics and the entailment-regime string surface the Python/wasm/C bindings call. |
| [`purrdf-slice`](./crates/slice/) | Slice catalog: manifests, typed artifacts, ownership/dependency analysis. |
| [`purrdf-iri`](./crates/iri/) | Zero-dependency IRI/URI parsing, normalization, CURIEs, and the workspace's single RFC 3986 base-resolution layer (`BaseIri`/`BaseScope`). |
| [`purrdf-xsd`](./crates/xsd/) | Zero-dependency XSD 1.1 value space with SPARQL numeric promotion. |
| [`purrdf-events`](./crates/rdf-events/) | Zero-dependency object-safe RDF event sink/source seam. |
| [`purrdf-wasm`](./crates/rdf-wasm/) | The wasm32 engine behind the `purrdf` ESM package. |
| [`purrdf-capi`](./crates/rdf-capi/) | `libpurrdf` C ABI (unpublished; built via cargo-c). |
| [`purrdf-cli`](./crates/cli/) | The `purrdf` command-line tool: `convert`, `query`, `update`, `reason`, `entails`, `consistency`, `validate`, `shex`, `describe`, `project`, `lift`, `pack`, `verify` (unpublished). |
| [`purrdf-sparql-conformance`](./crates/sparql-conformance/) | W3C SPARQL, entailment-regime, and OWL 2 conformance harnesses (unpublished). |

## Documentation

- **[RDF-1.2 playground](https://blackcat-informatics.github.io/purrdf/playground/)** —
  a zero-install browser console: parse, query (SPARQL), validate (SHACL),
  serialize, and canonicalize/compare RDF-1.2 (quoted triples, directional
  literals) entirely client-side over the wasm build. No toolchain, no server.
- **[The PurRDF Book](https://blackcat-informatics.github.io/purrdf/)** — the
  user guide: getting started in each language, concepts, and every engine
  (source in [`docs/book/`](./docs/book/), `make book` builds it locally).
- **API reference** — [docs.rs/purrdf](https://docs.rs/purrdf) for the umbrella
  crate; every member crate links its own docs.rs page from the crate map above.
- **Specs & reports** — [GTS spec](./docs/GTS-SPEC.md),
  [RDF 1.2 canonicalization profile](./docs/RDF12-CANON-PROFILE.md),
  [PURREMB embedding companion](./docs/PURREMB.md),
  [SPARQL execution governor profile](./docs/SPARQL-GOVERNOR-PROFILE.md),
  [conformance scoreboard](./docs/CONFORMANCE.md),
  [benchmarks](./docs/BENCHMARKS.md), [release process](./docs/RELEASE.md).
- **Design notes** — why the out-of-core engines answer identically on every
  target: [full-text scoring](./docs/design/purrdf-text-scoring.md),
  [GeoSPARQL exactness](./docs/design/purrdf-geo-exactness.md),
  [embedding kNN](./docs/design/purrdf-embedding-knn.md).

## Fast by measurement, not by assertion

The IR keeps every term **once** in a string arena addressed by copyable
`NonZeroU32` ids, hashes with fixed-key `ahash` everywhere hot, and freezes datasets
into `Box<[QuadRow]>` tables with lazy ordinal permutation indexes (~4 bytes/quad
per axis). Performance claims are backed by criterion benchmarks rather than
adjectives — `crates/rdf-core/benches/ir_layout.rs` measures AoS vs. SoA vs.
predicate-adjacency layouts (allocation counts, high-water mark, end-to-end
latency), and the shipped layout is whichever wins. Run them with `make bench`.

There is also a report-only Python harness that times the native-backed
`purrdf.compat.rdflib` drop-in against the real `rdflib` on parse, serialize,
SPARQL, and triple-pattern iteration over a deterministic `example.org` corpus
(`make bench-python`). Methodology, how to run, and a representative
(host-dependent) results table live in [`docs/BENCHMARKS.md`](./docs/BENCHMARKS.md).
Numbers vary by host — reproduce locally rather than trusting a fixed multiplier.

## Conformance

Every engine is gated by its official test suite, vendored and frozen in-repo —
full scoreboard and how-to-run in [`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md):

| Engine | Suite | Result |
| --- | --- | --- |
| ShEx 2.1 validation | shexTest v2.1.0 (`vectors/shexTest/`) | **1,105 / 1,105** attempted, 0 xfail |
| ShEx schemas / negative syntax / structure | shexTest v2.1.0 | **425/425 · 99/99 · 14/14** |
| SHACL | W3C data-shapes (`vectors/shacl/`) | **129 / 129**, 0 ledgered |
| SHACL (first-party frozen corpus) | `crates/shapes/corpus/` | **70 / 70** |
| SHACL Rules | DASH + first-party (`vectors/shacl/af/rules/`) | **19 / 19** |
| Syntax codecs | W3C rdf-tests round-trip | **264 / 264** |
| SPARQL 1.1/1.2 | full W3C sparql11 + sparql12 + first-party, via `purrdf-sparql-conformance` | **862** pass · 5 ledgered (upstream errata) |
| SPARQL CDT (SEP-0009) | vendored `awslabs/SPARQL-CDTs` (`vectors/sparql-cdt/`) | **658 / 658**, 0 ledgered — see the lexical-space divergence in [`docs/CONFORMANCE.md`](./docs/CONFORMANCE.md) |
| SPARQL execution governors | first-party frozen corpus (`vectors/sparql-governors/`) | **50 / 50**, 0 ledgered |
| Entailment (SPARQL regimes) | W3C sparql11 `entailment/` group | **70 / 70**, 0 ledgered |
| Entailment (OWL 2 DL consistency) | vendored W3C OWL 2 suite | **258 / 262** agreeing, 4 ledgered, 0 unledgered |
| Entailment (OWL 2 RL, W3C entailment tests) | vendored W3C OWL 2 entailment suite | **50 / 50** agreeing, 0 ledgered, 0 unledgered — negative lane **23 / 23** (no unsoundness), positive lane **27 / 27** |
| RDFC-1.0 | W3C canonicalization fixtures | green |
| GTS | frozen cross-language vectors (`vectors/`) | **38 / 39** fold byte-exactly into their committed expectation, 1 ledgered divergence |

## How capability grows

SPARQL breadth grows through caller-keyed extension seams — scalar functions,
property functions, custom aggregates, and the host-injected service resolver
— so new capability lands as composition through a seam, never as a Cargo
feature flag and never as a vocabulary PurRDF mints itself. Quad-form
`CONSTRUCT`, the SEP-0008 SHA-3 builtins, the SEP-0009 composite datatypes,
deterministic full-text search, path witnesses, embedding kNN and GeoSPARQL 1.1
all arrived that way: out-of-core, under caller-supplied IRIs, byte-identical
on every target, and under the same conformance discipline as everything
above.

## Development

```sh
make metadata   # regenerate + verify generated artifacts
make check      # fmt, build, tests, hygiene gates
make bench      # criterion benchmarks
```

Releases are tag-driven with OIDC trusted publishing (crates.io and PyPI), with
build-provenance attestations and SPDX SBOMs — see [`docs/RELEASE.md`](./docs/RELEASE.md).

## Versioning & MSRV

**Pre-1.0 semver policy.** While the version is `0.x`, a **minor** bump
(`0.x` → `0.(x+1)`) may include breaking API changes; a **patch** bump
(`0.x.y` → `0.x.(y+1)`) is bugfix-only and API-compatible. All three published
surfaces — the crates.io crate suite, the PyPI `purrdf` package, and the npm
`@blackcatinformatics/purrdf` package — share **one** workspace version and are
released in lockstep. That coherence is enforced in CI: a version-coherence check
fails the build if the three version sources disagree.

**MSRV policy.** The supported minimum Rust is `rust-version` in the root
`Cargo.toml` (currently **1.96**) on the **stable** channel, enforced by a dedicated
CI MSRV job, and release artifacts are built on stable. Raising the MSRV is a
notable change recorded in the changelog and, pre-1.0, rides a minor bump. The
README MSRV badge is maintained by hand and must be bumped together with
`rust-version`.

Contributors run a dated nightly (`rust-toolchain.toml`) for its sharper clippy and
rustdoc lint surface, but the workspace contains **no nightly-only features** — the
MSRV job is what proves that on every change. Building PurRDF needs nothing beyond
stable 1.96.

## The GMEOW family

PurRDF is the library layer of a small family of linked-data projects:

- [`gmeow-ontology`](https://github.com/Blackcat-Informatics/gmeow-ontology) — the
  GMEOW reasoning-centric super-vocabulary and its publishing toolchain (PurRDF's
  primary consumer).
- [`gmeow-gts`](https://github.com/Blackcat-Informatics/gmeow-gts) — the GTS
  specification and its multi-language engines; PurRDF hosts the Rust engine.

Extraction history and source commits: [`PROVENANCE.md`](./PROVENANCE.md).
Brand assets and usage: [`docs/BRAND.md`](./docs/BRAND.md).

## License

Licensed under either of [Apache License 2.0](./LICENSE-APACHE) or
[MIT license](./LICENSE-MIT) at your option, as described in
[`LICENSING.md`](./LICENSING.md).

If you use PurRDF in research, please cite it — see [`CITATION.cff`](./CITATION.cff).
