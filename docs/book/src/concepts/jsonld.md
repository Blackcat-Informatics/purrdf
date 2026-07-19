<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# JSON-LD Contexts & Compaction

PurRDF has three explicit JSON-LD/YAML-LD serialization modes:

- `expanded` preserves the byte-frozen empty-`@context` representation;
- `context` compiles a caller prefix map or local JSON-LD 1.1 context once and
  applies normative compaction;
- `derived` deterministically assigns neutral `ns0`, `ns1`, ... aliases solely
  from absolute IRIs in the dataset. It never invents a vocabulary or infers
  `@vocab`.

An RDF dataset does not retain prefix declarations from Turtle, JSON-LD, or
another source syntax. Supply them explicitly when those declarations are
application policy. All configured surfaces consume the same closed version-1
JSON document and reject duplicate members, unknown fields, invalid contexts,
network-only context references, cycles, and resource-limit excesses before
emitting output.

```json
{
  "version": 1,
  "mode": "context",
  "prefixes": {
    "ex": "https://example.org/",
    "schema": "https://schema.org/"
  },
  "yaml_schema_url": "https://example.org/purrdf.schema.json"
}
```

## Rust

```rust,ignore
use purrdf::{
    JsonLdSerializeOptions, parse_dataset,
    serialize_dataset_to_jsonld_with_options,
};

let dataset = parse_dataset(
    b"<https://example.org/alice> <https://schema.org/name> \"Alice\" .",
    "application/n-triples",
    None,
)?;
let options = JsonLdSerializeOptions::prefixes([
    ("ex", "https://example.org/"),
    ("schema", "https://schema.org/"),
])?;
let jsonld = serialize_dataset_to_jsonld_with_options(&dataset, &options)?;
# Ok::<(), purrdf::RdfDiagnostic>(())
```

Keep a `CompiledJsonLdContext` (or `JsonLdSerializeOptions::compiled`) when the
same application context is used for many datasets. `JsonLdContextRegistry`
resolves context IRIs and `@import` only from caller-supplied immutable local
documents; PurRDF never performs network context loading.

## CLI

Write the versioned document to a file, then pass it to any RDF-producing CLI
path:

```sh
purrdf --jsonld-options context.json convert --from turtle --to jsonld input.ttl output.jsonld
```

The option is rejected for non-JSON-LD/YAML-LD output, canonical output,
non-graph SPARQL results, and carrier projections rather than being ignored.

## Python

```python
import json
import purrdf

options = json.dumps({
    "version": 1,
    "mode": "context",
    "prefixes": {"ex": "https://example.org/"},
})
context = purrdf.CompiledJsonLdContext(options)
text = purrdf.serialize_jsonld(
    nquads,
    format=purrdf.RdfFormat.N_QUADS,
    output_format="jsonld",
    context=context,
)
```

`Store.dump`, `MutableDataset.dump`, immutable `RdfDataset`, and the RDFLib
compatibility `Graph.serialize` surface accept the same configuration. Prefixes
explicitly bound on a compatibility graph become its caller context.

## JavaScript and WebAssembly

```js
const options = JSON.stringify({
  version: 1,
  mode: "context",
  prefixes: { ex: "https://example.org/" },
});
const context = new CompiledJsonLdContext(options);
const text = dataset.serializeWithContext("jsonld", context);
```

Use `serializeConfigured` for one-shot requests. `QueryEngine` exposes matching
configured methods for CONSTRUCT and DESCRIBE graph results, and the playground
worker accepts the same options document on its serialization path.

## C

Compile options with `purrdf_jsonld_context_compile`, reuse the returned
`PurrdfJsonLdContext` with `purrdf_serialize_jsonld_configured`, then release it
with `purrdf_jsonld_context_free`. The serializer accepts exactly one options
byte slice or compiled handle. Buffers and errors retain the normal libpurrdf
ownership rules.

YAML-LD uses the same context lens. Its optional schema URL changes only the
deterministic YAML language-server header; it does not change RDF semantics.
