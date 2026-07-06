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

## First dataset

Await `ready()` once before anything else — it performs the one-time async
wasm instantiation:

```js
import { ready, DataFactory, Dataset } from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const f = new DataFactory();
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");

const ds = new Dataset();
ds.add(f.quad(f.namedNode("https://ex/s"), f.namedNode("https://ex/says"), rtl));

const nq = ds.serialize("nquads");           // directions survive the round-trip
const reparsed = Dataset.parse(nq, "nquads");
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
  `nquads`, `trig`, `rdfxml` (or their media types).
- **`Sink`** — a streaming consumer (`push(quad)` / `finish() → Dataset`);
  `datasetToStream` / `streamToDataset` are the async RDF/JS Stream/Sink
  helpers.

More on the RDF/JS mapping in [RDF/JS in JavaScript](../interop/rdfjs.md).

## Scope and current limitations

- **In-memory only.** SPARQL queries run over the in-memory dataset;
  federation and remote graph loading are native-only.
- A quoted-triple term as a quad **object** currently round-trips only through
  **N-Quads** (a current native serializer limitation for the other formats).

## Building from source

The Rust cdylib lives in
[`crates/rdf-wasm`](https://github.com/Blackcat-Informatics/purrdf/tree/main/crates/rdf-wasm);
the published ESM package is generated from it:

```sh
make wasm-pkg        # release wasm + wasm-bindgen ESM bindings → js/pkg/
make wasm-pkg-test   # the above + the Node real-execution round-trip suite
```

This requires the `wasm32-unknown-unknown` Rust target and a
`wasm-bindgen-cli` pinned to the crate's `wasm-bindgen` version.
