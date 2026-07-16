<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Getting Started: JavaScript / WebAssembly

The npm package
[`@blackcatinformatics/purrdf`](https://www.npmjs.com/package/@blackcatinformatics/purrdf)
is the same Rust engine compiled to `wasm32` and surfaced through an
[RDF/JS](https://rdf.js.org/)-shaped API (`DataFactory`, `DatasetCore`,
`Stream`/`Sink`). It runs in the browser and in Node, entirely in memory.

```sh
npm install @blackcatinformatics/purrdf
```

Prefer to try it before installing anything? The
[RDF-1.2 playground](https://blackcat-informatics.github.io/purrdf/playground/)
runs this exact wasm build in your browser — parse, SPARQL, SHACL, serialize, and
canonicalize/compare RDF-1.2 graphs client-side, with no toolchain and no server.

## First dataset

Await `ready()` once before anything else — it performs the one-time async
wasm instantiation:

```js
import { ready, DataFactory, Dataset, QueryEngine } from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const f = new DataFactory();
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");

const ds = new Dataset();
ds.add(f.quad(f.namedNode("https://ex/s"), f.namedNode("https://ex/says"), rtl));

const nq = ds.serialize("nquads");           // directions survive the round-trip
const reparsed = Dataset.parse(nq, "nquads");

const engine = new QueryEngine();
const ask = engine.ask(reparsed, "ASK { <https://ex/s> <https://ex/says> ?msg }");
```

## The RDF 1.2 wedge

No incumbent RDF/JS library carries RDF 1.2 **quoted-triple terms** or
**directional literals**. PurRDF's `DataFactory` exposes both:

```js
// A quoted triple, usable as a subject/object (RDF-star / RDF 1.2).
const quoted = f.quotedTriple(
  f.namedNode("https://ex/alice"),
  f.namedNode("https://ex/knows"),
  f.namedNode("https://ex/bob"),
);

// A base-direction literal (rdf:dirLangString).
const hello = f.directionalLiteral("مرحبا", "ar", "rtl");
```

## API surface

- **`ready(bytesOrUrl?)`** — await once before anything else.
- **`DataFactory`** — `namedNode`, `blankNode`,
  `literal(value, languageOrDatatype?)`, `typedLiteral`,
  `directionalLiteral`, `variable`, `defaultGraph`, `quad`, `quotedTriple`,
  `fromTerm`, `fromQuad`.
- **`Dataset`** (RDF/JS `DatasetCore`) — `Dataset.parse(input, format, base?)`,
  `serialize(format)`, `add`/`delete`/`has`/`match`/`quads`/`size`, and
  iteration (`for (const quad of dataset)`). Formats: `turtle`, `ntriples`,
  `nquads`, `trig`, `rdfxml` (or their media types); `serialize` additionally
  accepts `jsonld`.
- **Graph identity** — `Dataset.canonicalize()` returns the RDFC-1.0 canonical,
  flat N-Quads for the graph; `Dataset.isomorphic(other)` decides RDF graph
  equality under blank-node relabeling (an exact oracle backed by full RDFC-1.0
  canonicalization).
- **Graph/tabular/research-object carriers** — `Dataset.project(profile, configJson)` returns
  canonical USTAR bytes and loss-ledger JSON; `liftProjection(...)` reconstructs
  RDF for the bidirectional profiles. See
  [Graph, Tabular & Research-Object Projections](../concepts/projections.md).
- **SPARQL** — `QueryEngine` keeps the native plan cache alive across calls and
  exposes typed `select` / `ask` / `construct` / `describe`, atomic `update`,
  and `queryRaw` serialization. `Dataset.query(...)` remains the compatibility
  raw-string helper.
- **SHACL** — `shaclValidateToSarif(shapesTtl, dataNt)` validates an N-Triples
  data graph against a Turtle shapes graph and returns a SARIF 2.1.0 report;
  `shaclEntail(shapesTtl, dataNt)` materializes the SHACL-AF `sh:rule`
  inferences as N-Triples.
- **`Sink`** — a streaming consumer (`push(quad)` / `finish() → Dataset`);
  `datasetToStream` / `streamToDataset` are the async RDF/JS Stream/Sink
  helpers.

More on the RDF/JS mapping in [RDF/JS in JavaScript](../interop/rdfjs.md).

## Scope and current limitations

- **In-memory only.** SPARQL queries run over the in-memory dataset;
  this package provides no network resolver, so remote `SERVICE` and `LOAD`
  fail explicitly.
- A quoted-triple term as a quad **object** currently round-trips only through
  **N-Quads** (a current native serializer limitation for the other formats).

## Building from source

The Rust cdylib lives in
[`crates/rdf-wasm`](https://github.com/Blackcat-Informatics/purrdf/tree/main/crates/rdf-wasm);
the published ESM package is generated from it:

```sh
make wasm-pkg        # release wasm + wasm-bindgen ESM bindings → js/pkg/
make wasm-pkg-test   # the above + TypeScript, Node, and packed-tarball gates
```

This requires the `wasm32-unknown-unknown` Rust target and a
`wasm-bindgen-cli` pinned to the crate's `wasm-bindgen` version.
