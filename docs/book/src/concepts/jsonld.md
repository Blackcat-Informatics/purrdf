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

Context and derived modes accept `"compact_arrays": false` to retain eligible
singleton collections as arrays in both JSON-LD and YAML-LD. Omission and explicit
`true` preserve the existing output. Scalar keyword positions and list item arrays
keep their required shapes. RDF JSON literals use the lossless typed lexical form
when arrays are retained. Expanded mode rejects this field, including `true`.
In Rust, use `options.with_compact_arrays(false)?` and `options.compact_arrays()`.

## Rust

```rust
use purrdf::{
    JsonLdSerializeOptions, SerializeGraph, parse_dataset, serialize_dataset,
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
assert!(jsonld.contains("\"schema:name\""));
let reopened = parse_dataset(jsonld.as_bytes(), "application/ld+json", None)?;
assert_eq!(
    serialize_dataset(&reopened, "application/n-quads", SerializeGraph::Dataset)?,
    serialize_dataset(&dataset, "application/n-quads", SerializeGraph::Dataset)?,
);
# Ok::<(), purrdf::RdfDiagnostic>(())
```

Keep a `CompiledJsonLdContext` (or `JsonLdSerializeOptions::compiled`) when the
same application context is used for many datasets. `JsonLdContextRegistry`
resolves context IRIs and `@import` only from caller-supplied immutable local
documents; PurRDF never performs network context loading.

## Native document compaction

`compact_expanded_with_context` compacts lexical JSON values before RDF lowering.
`compact_jsonld_with_context` and `compact_yamlld_with_context` first expand input
bytes with a compiled input context, then compact with a distinct compiled output
context. Both return an owned lexical JSON value; YAML input keeps its existing
selection options and expansion limits. Opaque JSON and native numeric lexemes
remain document values rather than taking an RDF round trip.

Pass `JsonLdCompactionPolicy` with `compact_arrays` and `compact_to_relative`;
both default to `true`. Relative selection affects document-position IRIs under
the compiled base without altering the retained output context. The compact
operation attaches a non-null context, turns an empty root array into an object,
and envelopes other remaining arrays with the context's `@graph` alias.

Flattening with an output context composes through that same root operation:

```rust
use purrdf::native_codecs::jsonld::{
    JsonLdCompactionLimits, JsonLdCompactionPolicy, JsonLdExpansionLimits,
    JsonLdFlattenLimits, compact_expanded_with_context, flatten_jsonld_with_context,
};
# fn example(bytes: &[u8], input: &purrdf::CompiledJsonLdContext,
#     output: &purrdf::CompiledJsonLdContext) -> Result<(), purrdf::RdfDiagnostic> {
let flattened = flatten_jsonld_with_context(
    bytes, input, &JsonLdExpansionLimits::DEFAULT, &JsonLdFlattenLimits::DEFAULT,
)?;
let compacted = compact_expanded_with_context(
    &flattened, output, &JsonLdCompactionPolicy::DEFAULT,
    &JsonLdCompactionLimits::DEFAULT,
)?;
# let _ = compacted;
# Ok(())
# }
```

`JsonLdCompactionLimits::max_allocation_bytes` defaults to 32 GiB. It admits newly
owned document output, retained context copies, walk frames and compactor scratch
before construction. Borrowed input and initial compiled contexts are not charged
again. Context compilation, scoped context state and native IRI/inverse-selection
scratch retain their separate semantic limits; those are per compilation, not an
aggregate retained-byte bound. This is a document-construction account, not a cap
on total operation memory. Expansion and flattening keep their own accounts.

Configured RDF writers project their prepared RDF nodes into this same compactor,
retaining streamed envelopes and one-node residency. Lists, reifiers, reverse
relocation and lossless graph ownership remain RDF preparation. A selected custom
graph index consumes the actual mapping property's first compacted key and retains
residual values, graph identifiers and other metadata. It does not substitute
semantic `@index` metadata for that property.

The shared value-compaction rules can emit plain strings for untyped literals even
under prefix-only property spellings. Ordinary type/index maps now use the native
document algorithm as well. Multiple lists retain explicit list objects when one
list container would combine their identities. If an immutable context shadows
every lossless spelling, compaction returns an actionable error. These configured
outputs can be more compact
than earlier carrier output. Frozen no-base, no-options expanded bytes are retained,
and RDF JSON literals keep typed lexical forms whenever raw JSON coercion would
change their spelling, collection shape or annotation identity.

## CLI

Write the versioned document to a file, then pass it to any RDF-producing CLI
path:

```sh
purrdf --jsonld-options context.json convert --from turtle --to jsonld input.ttl output.jsonld
```

The option is rejected for non-JSON-LD/YAML-LD output, canonical output,
non-graph SPARQL results, and carrier projections rather than being ignored.

## YAML-LD semantic translation

Select `yamlld` on the CLI or `application/ld+yaml` in the native format API.
The reader targets the [YAML-LD 1.0 Working Draft of 24 October
2026](https://www.w3.org/TR/2026/WD-yaml-ld-10-20261024/). It constructs the first
document by default; explicit whole-stream selection and admission bounds are
described in [YAML-LD input options](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/YAML_INPUT_OPTIONS.md).
Input controls are independent of the serialization context described here.

Using the `context.json` above, translate an ordinary RDF dataset with an
explicit output base:

```sh
purrdf --jsonld-options context.json convert --from nquads --to yamlld \
  --base https://example.org/ input.nq output.yamlld
```

The shared JSON-LD carrier supplies expanded, caller-context and derived-context
YAML output. The generic serializer carries the caller's base into that context;
it does not recover a prefix map from the input dataset. For fixed dataset,
mode, context, base and options, the output is deterministic. The optional
`yaml_schema_url` selects the YAML language-server header, independently of RDF
meaning.

This conversion preserves RDF semantics rather than authored YAML presentation.
Comments, quoting, anchors and original layout require the separate
[exact YAML source workflow](codecs.md#exact-yaml-source-reconstruction).

## RDF 1.2 carrier additions

The shared JSON-LD/YAML-LD carrier represents [RDF 1.2](rdf12.md) triple terms
with `@triple`. Its value has three members: `@subject` and `@object` are typed
carrier terms, and `@predicate` is an IRI string. For example:

```yaml
'@id': https://example.org/alice
https://example.org/reports:
  '@triple':
    '@subject':
      '@id': https://example.org/cat
    '@predicate': https://example.org/says
    '@object':
      '@value': meow
  '@annotation':
    '@id': https://example.org/assertion
    https://example.org/confidence:
      '@value': high
```

`@annotation` carries one annotation node or an array of nodes on a value.
Together these additions retain recursive triple terms and statement annotation
relationships through RDF 1.2 round trips. They are carrier extensions, not terms
defined by the pinned YAML-LD Basic specification, and are separate from its
informative Extended YAML profile. Standard JSON-LD context, language and
`@direction` constructs retain their ordinary meanings; the
[context modes](#json-ld-contexts--compaction) above apply to both carriers.

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
