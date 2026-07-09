<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# RDF/JS in JavaScript

In JavaScript, PurRDF does not invent its own API shape: the npm package
[`@blackcatinformatics/purrdf`](https://www.npmjs.com/package/@blackcatinformatics/purrdf)
implements the [RDF/JS](https://rdf.js.org/) community specifications —
`DataFactory`, `DatasetCore`, and `Stream`/`Sink` — over the wasm-compiled
native engine. Code written against RDF/JS interfaces works with PurRDF terms
and datasets.

## The data factory

`DataFactory` covers the standard RDF/JS term constructors — `namedNode`,
`blankNode`, `literal(value, languageOrDatatype?)`, `variable`,
`defaultGraph`, `quad`, `fromTerm`, `fromQuad` — plus the deliberate RDF 1.2
extensions no incumbent RDF/JS library carries:

```js
import { ready, DataFactory } from "@blackcatinformatics/purrdf";
await ready();
const f = new DataFactory();

// RDF/JS standard surface.
const q = f.quad(
  f.namedNode("https://ex/alice"),
  f.namedNode("https://ex/knows"),
  f.namedNode("https://ex/bob"),
);

// RDF 1.2 extensions: quoted triple terms and base-direction literals.
const quoted = f.quotedTriple(q.subject, q.predicate, q.object);
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");
```

`typedLiteral` is a convenience alongside the spec's overloaded `literal`.

## Datasets

`Dataset` implements RDF/JS `DatasetCore`: `add`, `delete`, `has`, `match`,
`size`, and iteration (`for (const quad of dataset)`), plus parsing and
serialization through the native codecs:

```js
const ds = Dataset.parse("<https://ex/s> <https://ex/p> <https://ex/o> .", "ntriples");
for (const quad of ds.match(null, f.namedNode("https://ex/p"), null)) {
  console.log(quad.subject.value);
}
const trig = ds.serialize("trig");
```

Accepted format names: `turtle`, `ntriples`, `nquads`, `trig`, `rdfxml`, or
their media types. Because these are the native codecs, output is
byte-deterministic and identical to what the Rust, Python, and C surfaces
emit ([Codecs & Determinism](../concepts/codecs.md)).

## SPARQL

Use `QueryEngine` when running more than one query or when the caller wants
typed results instead of raw strings. The engine owns the native SPARQL plan
cache and returns package-root terms and datasets:

```js
import { QueryEngine } from "@blackcatinformatics/purrdf";

const engine = new QueryEngine();
const result = engine.select(
  ds,
  "PREFIX ex: <https://ex/> SELECT ?o WHERE { ex:s ex:p ?o }",
);
console.log(result.rows[0].o?.value);

const graph = engine.construct(
  ds,
  "PREFIX ex: <https://ex/> CONSTRUCT { ex:copy ex:p ?o } WHERE { ex:s ex:p ?o }",
);
```

`QueryEngine.queryRaw(...)` serializes SELECT/ASK results as SPARQL Results
JSON/XML/CSV/TSV and graph results through the same graph formats accepted by
`Dataset.serialize`. `QueryEngine.update(...)` applies SPARQL UPDATE atomically:
the dataset changes only after the whole update succeeds.

## Streams and sinks

The package speaks the async RDF/JS Stream/Sink protocol over the
`purrdf-events` ingestion seam:

- **`Sink`** — a streaming consumer: `push(quad)` per quad, `finish()` returns
  the accumulated `Dataset`.
- **`datasetToStream(dataset)`** / **`streamToDataset(stream)`** — bridge
  between a `Dataset` and RDF/JS streams, for piping into or out of other
  RDF/JS tooling.

## Scope notes

- The engine is **in-memory**; there is no persistent store in the wasm
  build. SPARQL runs over the in-memory dataset; this package provides no
  network resolver, so remote `SERVICE` and `LOAD` fail explicitly.
- A quoted-triple term as a quad **object** currently round-trips only
  through **N-Quads** (a current native serializer limitation for the other
  formats).

## Related

- [Getting Started: JavaScript / WebAssembly](../getting-started/javascript.md)
- [RDF 1.2 Features](../concepts/rdf12.md) — what the extensions mean.
- [`crates/rdf-wasm`](https://github.com/Blackcat-Informatics/purrdf/tree/main/crates/rdf-wasm)
  — the Rust cdylib behind the package.
