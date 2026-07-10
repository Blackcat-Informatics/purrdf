<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# purrdf (wasm) — RDF 1.2 in the browser & Node, the RDF/JS way

`purrdf` is a `wasm32`, **in-memory** RDF 1.2 engine compiled from the oxigraph-free
[`purrdf`](../purrdf) umbrella crate and surfaced to JavaScript/TypeScript through the
[RDF/JS](https://rdf.js.org/) community spec (`DataFactory`, `DatasetCore`,
`Stream`/`Sink`). It is parcel **P10** of the purrdf program
([`docs/design/PurRDF-PLAN.md`](../../docs/design/PurRDF-PLAN.md)).

This crate (`purrdf-wasm`) is the Rust cdylib; the published npm/ESM package lives
in [`js/`](./js/) and is named **`@blackcatinformatics/purrdf`**.

> **Try it live** — the [RDF-1.2 playground](https://blackcat-informatics.github.io/purrdf/playground/)
> runs this exact wasm build in your browser: parse, SPARQL, SHACL, serialize, and
> canonicalize/compare graphs client-side, no toolchain and no server.

## The RDF-1.2 wedge

No incumbent RDF/JS library carries RDF-1.2 **quoted-triple terms** or **directional
literals**. purrdf's `DataFactory` exposes both — the deliberate "overcome, don't
inherit" extension to stock RDF/JS:

```js
import { ready, DataFactory, Dataset } from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const f = new DataFactory();

// A quoted triple, usable as a subject/object (RDF-star / RDF 1.2).
const quoted = f.quotedTriple(
  f.namedNode("https://ex/alice"),
  f.namedNode("https://ex/knows"),
  f.namedNode("https://ex/bob"),
);

// A base-direction literal (rdf:dirLangString).
const rtl = f.directionalLiteral("مرحبا", "ar", "rtl");

const ds = new Dataset();
ds.add(f.quad(f.namedNode("https://ex/s"), f.namedNode("https://ex/says"), rtl));
ds.add(f.quad(f.namedNode("https://ex/stmt"), f.namedNode("https://ex/asserts"), quoted));

// Quoted triples + directions survive a round-trip through N-Quads.
const nq = ds.serialize("nquads");
const reparsed = Dataset.parse(nq, "nquads");
```

## API

- **`ready(bytesOrUrl?)`** — await once before anything else (instantiates the wasm).
- **`DataFactory`** — `namedNode`, `blankNode`, `literal(value, languageOrDatatype?)`,
  `typedLiteral`, `directionalLiteral`, `variable`, `defaultGraph`, `quad`,
  `quotedTriple`, `fromTerm`, `fromQuad`.
- **`Dataset`** (RDF/JS `DatasetCore`) — `Dataset.parse(input, format, base?)`,
  `serialize(format)`, `add`/`delete`/`has`/`match`/`quads`/`size`, and iteration
  (`for (const quad of dataset)`). Formats: `turtle`, `ntriples`, `nquads`, `trig`,
  `rdfxml` (or their media types); `serialize` additionally accepts `jsonld`.
- **Graph identity** — `Dataset.canonicalize()` returns the RDFC-1.0 canonical, flat
  N-Quads for the graph; `Dataset.isomorphic(other)` decides RDF graph equality under
  blank-node relabeling (an exact oracle backed by full RDFC-1.0 canonicalization).
- **Visualization** — `Dataset.visualModel(options?)` returns the renderer-neutral
  RDF 1.2 statement model; `visualExport` adds the semantic scene, deterministic
  geometry, hashes, diagnostics, and element index; `visualSvg` returns a
  self-contained SVG paired with that complete export. All results are plain,
  structured-clone-safe objects.
- **SHACL** — `shaclValidateToSarif(shapesTtl, dataNt)` validates an N-Triples data
  graph against a Turtle shapes graph and returns a SARIF 2.1.0 report;
  `shaclEntail(shapesTtl, dataNt)` materializes the SHACL-AF `sh:rule` inferences as
  N-Triples.
- **`Sink`** — a streaming consumer (`push(quad)` / `finish() → Dataset`) over the
  `purrdf-events` ingestion protocol; **`datasetToStream`** / **`streamToDataset`**
  are the async RDF/JS Stream/Sink helpers.

```js
const { svg, export: graph } = reparsed.visualSvg({
  mode: "compact",
  vocabulary: [{ prefix: "ex", namespace: "https://ex/" }],
  svg: { title: "RDF 1.2 claim graph" },
});

console.log(graph.model.statements, graph.model.relations);
document.querySelector("#graph").innerHTML = svg;
```

Use `mode: "incidence"` to inspect exact subject/predicate/object ports and nested
triple terms, or `mode: "table"` for one row per structural statement. A quoted
triple is never rendered as asserted unless an assertion occurrence is present.

## Scope

- **In-memory only** — the oxigraph `Store` (RocksDB) and the logic engine do not
  compile to wasm and are excluded by design. SPARQL query runs offline over the
  in-memory dataset; this package provides no network resolver, so remote
  `SERVICE` and `LOAD` fail explicitly.
- Text codecs ride purrdf's native codecs — no Store dependency and no
  `purrdf-gts` RDF-codec feature.
- A quoted-triple term as a quad **object** currently round-trips only through
  **N-Quads** (a current native serializer limitation for the other formats).

## Building

```sh
make wasm-pkg        # release wasm + wasm-bindgen ESM bindings → js/pkg/
make wasm-pkg-test   # the above + TypeScript, Node, and packed-tarball gates
```

Requires the `wasm32-unknown-unknown` Rust target and `wasm-bindgen-cli` (pinned to
the crate's `wasm-bindgen` version, `0.2.125`).

## License

MIT OR Apache-2.0 (the `purrdf` engine); the `purrdf-events` ingestion protocol it
depends on is permissive (MIT OR Apache-2.0).
