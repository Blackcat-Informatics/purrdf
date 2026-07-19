<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# @blackcatinformatics/purrdf

**PurRDF** — an in-memory RDF 1.2 engine for JavaScript, compiled to
WebAssembly from the [purrdf](https://github.com/Blackcat-Informatics/purrdf)
Rust workspace, with an idiomatic [RDF/JS](https://rdf.js.org/)
(`DataFactory` / `DatasetCore` / `Stream`) API.

It is the same engine, byte-for-byte behavior, that ships as the `purrdf`
Rust crates, the `purrdf` PyPI package, and `libpurrdf` — PurRDF's rule is
**one engine, one behavior, every language**.

> **Try it live** — the [RDF-1.2 playground](https://blackcat-informatics.github.io/purrdf/playground/)
> runs this package in your browser: parse, SPARQL, SHACL, serialize, and
> canonicalize/compare RDF-1.2 graphs client-side, with no install.

## Why this instead of an incumbent RDF/JS library?

No incumbent RDF/JS library carries the RDF 1.2 features:

- **Quoted-triple terms** (RDF-star / RDF 1.2 triple terms), usable in the
  object position;
- **Base-direction literals** (`rdf:dirLangString`) — language *plus*
  `ltr`/`rtl` direction;
- Byte-deterministic serializers backed by W3C conformance corpora
  (SPARQL 1.1, RDFC-1.0 canonicalization fixtures) in the parent workspace.

## Install

```sh
npm install @blackcatinformatics/purrdf
```

Runs in Node ≥ 18 and modern browsers (ESM, `web`-target wasm-bindgen).

The wasm artifact is built with WebAssembly SIMD (`+simd128`) for higher parse
throughput, so it requires a runtime with wasm SIMD support: every major browser
since ~2021 (Chrome/Edge 91+, Firefox 89+, Safari 16.4+) and Node ≥ 18.

## Quickstart

```js
import { ready, DataFactory, Dataset, QueryEngine } from "@blackcatinformatics/purrdf";

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

const engine = new QueryEngine();
const names = engine.select(
  reparsed,
  "SELECT ?message WHERE { <https://ex/s> <https://ex/says> ?message }",
);
console.log(names.rows.take(0)?.message.value);
```

Configured JSON-LD/YAML-LD calls the same Rust context engine as native PurRDF:

```js
import { CompiledJsonLdContext } from "@blackcatinformatics/purrdf";

const options = JSON.stringify({
  version: 1,
  mode: "context",
  prefixes: { ex: "https://ex/" },
});
const context = new CompiledJsonLdContext(options);
const jsonld = reparsed.serializeWithContext("jsonld", context);
```

`serializeConfigured` handles one-shot expanded, context, registry-backed, or
derived requests. Matching `QueryEngine.queryRawConfigured` and
`queryRawWithContext` methods serialize CONSTRUCT/DESCRIBE graph results.

## Graph, tabular, and research-object projection archives

Projection and lift run entirely in memory through the native Rust engine. The
caller supplies strict profile-tagged JSON; PurRDF does not fabricate vocabulary,
identity, or resource limits.

```js
import { Dataset, liftProjection, ready } from "@blackcatinformatics/purrdf";

await ready();
const config = JSON.stringify({
  profile: "lpg-csv",
  config: {
    rdf_type: "https://example.org/type",
    scope: { mode: "all" },
    limits: {
      max_artifacts: 16,
      max_artifact_bytes: 1_000_000,
      max_total_bytes: 4_000_000,
      max_archive_bytes: 5_000_000,
      max_term_depth: 16,
    },
    execution_limits: {
      max_input_records: 1_000,
      max_model_records: 1_000,
      max_nodes: 1_000,
      max_edges: 1_000,
    },
  },
});
const dataset = Dataset.parse(
  "@prefix ex: <https://example.org/> . ex:alice ex:knows ex:bob .",
  "turtle",
);
const projected = dataset.project("lpg-csv", config);
const lifted = liftProjection(projected.archive, "lpg-csv", config);
const roundTrip = lifted.takeDataset();
console.log(roundTrip?.size, JSON.parse(projected.lossLedgerJson));
```

`lpg-csv`, `neo4j-csv`, `open-cypher`, `graphml`, `csvw-exact`, `croissant-1.1`,
`ro-crate-1.3`, `datacite-4.6`, `dcat-3`, and `frictionless-data-package-1` are
bidirectional. `csvw-terms`, `obo-graphs`, and `skos` are write-only,
loss-ledgered views and are excluded from the `LiftProfile` TypeScript union.
Research-object contexts, vocabularies, identities, and
profiles are mandatory caller configuration. Archives are canonical
deterministic USTAR bytes. Package/lift objects own wasm memory; call `free()`
when finished, and remember that `takeDataset()` transfers its dataset exactly
once. A runnable Node example is
[`projection-roundtrip.mjs`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-wasm/js/examples/projection-roundtrip.mjs).

## API surface

- `ready(bytesOrUrl?)` — one-time async wasm instantiation.
- `DataFactory` — `namedNode`, `blankNode`, `literal`, `typedLiteral`,
  `directionalLiteral`, `variable`, `defaultGraph`, `quad`, `quotedTriple`,
  `fromTerm`, `fromQuad`.
- `Dataset` — `Dataset.parse(input, format, base?)`, `serialize(format)`,
  `serializeConfigured(format, optionsJson)`, `serializeWithContext(format, context)`,
  `add` / `delete` / `has` / `match` / `quads` / `size`, iteration.
  Formats: `turtle`, `ntriples`, `nquads`, `trig`, `rdfxml` (`serialize` also `jsonld`).
- `Dataset.canonicalize()` / `Dataset.isomorphic(other)` — RDFC-1.0 canonical N-Quads
  and RDF graph-identity (isomorphism under blank-node relabeling).
- `Dataset.project(profile, configJson)` / `liftProjection(archive, profile,
  configJson)` — canonical graph/tabular/research-object USTAR carriers with structured,
  always-computed loss-ledger JSON.
- `Dataset.visualModel(options?)` / `visualExport(options?)` /
  `visualSvg(options?)` — the renderer-neutral RDF 1.2 model, complete semantic
  scene and deterministic geometry, or self-contained SVG paired with that export.
  Returned objects are structured-clone-safe and preserve triple-term identity,
  assertion graphs, reifier/annotation graph context, nesting, and diagnostics.
- `QueryEngine` — a reusable SPARQL execution context with a native plan cache,
  typed `select` / `ask` / `construct` / `describe` helpers, atomic `update`,
  and `queryRaw` serialization for SPARQL Results JSON/XML/CSV/TSV plus graph
  formats. SELECT `rows` are a single-owner iterable; use `take(index)`,
  `toArray()`, or iteration, and call `free()` when abandoning unconsumed rows.
  `Dataset.query(...)` remains as the compatibility raw-string helper.
- `shaclValidateToSarif(shapesTtl, dataNt)` / `shaclEntail(shapesTtl, dataNt)` — SHACL
  validation to a SARIF 2.1.0 report and SHACL-AF `sh:rule` entailment to N-Triples.
- `Sink`, `datasetToStream`, `streamToDataset` — the async RDF/JS
  Stream/Sink primitives over the synchronous engine surface.
- SPARQL evaluation over the in-memory dataset (no server required).

Full typings ship in `index.d.ts`.

```js
const { svg, export: graph } = reparsed.visualSvg({
  mode: "compact",
  vocabulary: [{ prefix: "ex", namespace: "https://ex/" }],
  svg: { title: "RDF 1.2 graph", embedMetadata: true },
});

console.log(graph.model.statements, graph.model.relations);
document.querySelector("#graph").innerHTML = svg;
```

Compact mode keeps ordinary asserted RDF as directed predicate-labelled edges and
promotes statements only when they need identity. Incidence mode exposes exact
subject/predicate/object ports. Table mode scales statement inspection without
discarding the same underlying model.

## Scope

In-memory only, by design: no persistent store and no network I/O inside the
wasm module. This package provides no network resolver, so remote `SERVICE`
and `LOAD` fail explicitly. For the container transport (GTS), native APIs, and the
rest of the toolkit, see the
[main repository](https://github.com/Blackcat-Informatics/purrdf).

## Supply chain

Published from GitHub Actions via npm trusted publishing with sigstore
provenance, a GitHub build-provenance attestation, and an SPDX SBOM per
release.

## License

[MIT OR Apache-2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSING.md),
© 2026 Blackcat Informatics® Inc.
