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
since ~2021 (Chrome/Edge 91+, Firefox 89+, Safari 16.4+) and Node ≥ 16.

## Quickstart

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

## API surface

- `ready(bytesOrUrl?)` — one-time async wasm instantiation.
- `DataFactory` — `namedNode`, `blankNode`, `literal`, `typedLiteral`,
  `directionalLiteral`, `variable`, `defaultGraph`, `quad`, `quotedTriple`,
  `fromTerm`, `fromQuad`.
- `Dataset` — `Dataset.parse(input, format, base?)`, `serialize(format)`,
  `add` / `delete` / `has` / `match` / `quads` / `size`, iteration.
  Formats: `turtle`, `ntriples`, `nquads`, `trig`, `rdfxml`.
- `Sink`, `datasetToStream`, `streamToDataset` — the async RDF/JS
  Stream/Sink primitives over the synchronous engine surface.
- SPARQL evaluation over the in-memory dataset (no server required).

Full typings ship in `index.d.ts`.

## Scope

In-memory only, by design: no persistent store and no network I/O inside the
wasm module. For the container transport (GTS), SHACL validation, SPARQL
result serializers, and the rest of the toolkit, see the
[main repository](https://github.com/Blackcat-Informatics/purrdf).

## Supply chain

Published from GitHub Actions via npm trusted publishing with sigstore
provenance, a GitHub build-provenance attestation, and an SPDX SBOM per
release.

## License

[MIT OR Apache-2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSING.md),
© 2026 Blackcat Informatics® Inc.
