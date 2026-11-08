<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# JSON-LD Contexts & Compaction

PurRDF has three explicit JSON-LD/YAML-LD serialization modes:

- `expanded` emits a deterministic empty-`@context` representation with `@type`
  entries written as IRI strings;
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

## Document expansion across hosts

Document expansion returns compact JSON text containing expanded document
structure. It preserves repeated and nested descriptions, unminted identifiers,
native RDF 1.2 terms and opaque JSON through the existing structural engine.
It constructs no RDF dataset. Supply original source bytes separately from a
closed version-one JSON request:

```json
{
  "version": 1,
  "format": "yamlld",
  "base": "https://example.org/document",
  "context": {"label": "https://example.org/label"},
  "yaml_ld": {"extract_all_scripts": false},
  "expansion": {"max_allocation_bytes": 34359738368},
  "output": {"max_bytes": 4294967296, "max_allocation_bytes": 34359738368}
}
```

| Field | Accepted value and default |
| --- | --- |
| `version` | Required integer `1`. |
| `format` | Required string resolved by the native format classifier; only JSON-LD and YAML-LD are accepted, including their existing aliases and media types. |
| `base` | Optional absolute input/document IRI string; omitted means no supplied base. |
| `context` | Optional bare JSON-LD context expression: object, context IRI string, array or null. Omission keeps default initialization; null is an explicit reset. |
| `registry` | Optional array of offline context records; default empty. |
| `yaml_ld` | Optional Basic YAML controls below; explicit presence requires YAML-LD source input. |
| `expansion` | Optional object accepting only `max_allocation_bytes`, default 32 GiB. |
| `output` | Optional object accepting only `max_bytes` (4 GiB) and `max_allocation_bytes` (32 GiB). |

Every record is closed. Unknown or duplicate fields, unsupported versions/formats,
incorrect types and non-applicable controls refuse before source parsing. Numeric
limits require nonnegative integer lexemes; fractions and exponent notation are
refused. Expansion and both output limits accept the full `u64` range, including
zero. Logical byte ceilings remain distinct from checked physical address/layout
conversions.

The request's `context` is the effective expression, not a context document.
An expression object containing `@context`, including an object in a context
array, is refused. A legitimate term definition may still contain its own scoped
`@context`. Registry documents retain the existing document-wrapper semantics.
Each registry record requires absolute `iri`, `format` (`json` or `yaml`) and
original `document` text. YAML records additionally require their own `yaml_ld`
object; JSON records refuse that field. Registry YAML controls remain independent
of the source format, so JSON-LD source may use a YAML context document. Contexts
and imports resolve only through that immutable offline registry.

Both source and registry `yaml_ld` objects accept only `extract_all_scripts`,
`syntax` and `projection`. Context and registry fields belong at the request's
top level. Omitted YAML controls use these native defaults:

| Control | Default |
| --- | --- |
| `extract_all_scripts` | `false` (construct the first document); true constructs all documents. |
| `syntax.max_input_bytes` | 16 MiB |
| `syntax.max_allocation_bytes` | 128 MiB |
| `syntax.max_occurrences` | 1,048,576 |
| `syntax.max_depth` | 128 |
| `syntax.aliases` | `true` |
| `syntax.max_documents` | 1,048,576 |
| `projection.max_allocation_bytes` | 128 MiB |
| `projection.max_nodes` | 1,048,576 |
| `projection.max_depth` | 128 |
| `projection.max_numeric_work` | 67,108,864 |
| `projection.max_numeric_bytes` | 32 MiB |

`max_occurrences`, `max_nodes` and `max_numeric_work` accept full-width `u64`;
other YAML numeric fields use the portable `0..=4294967295` range. Syntax covers
the complete YAML stream, including unselected documents; projection covers
retained syntax and selected construction. `max_serialization_bytes`, profiles
and extension controls are not request fields.

Request JSON uses the existing 10 MiB options ceiling and separate 128 MiB
record-construction account. Registry/context compilation, source decoding,
structural expansion, output/writer construction, host input/ABI transfer and
result-handle storage retain their separate authorities. The output byte ceiling
is enforced before each governed append, including escaping and UTF-8; the output
allocation ceiling covers newly owned text and traversal storage. No unbounded
intermediate JSON string is constructed. These limits do not claim total
operation-memory accounting. Native diagnostics and storage causes are retained.

Rust callers use `purrdf_validate::document_expansion::expand(source, request_json)`.
`request(request_json)` compiles a reusable request, and
`expand_decoded(source, &request)` avoids decoding it again. The CLI decodes and
compiles the request before acquiring source bytes; `-` selects stdin/stdout,
and request and source cannot share stdin:

```sh
purrdf document-expand --request expand.json input.yamlld expanded.jsonld
```

C callers use
`purrdf_document_expand(source, source_len, request_json, request_len, out_buffer, out_error)`.
Both pointer/length inputs are required. Valid output slots are cleared before
work, aliased output/error slots are refused, and failures publish no buffer.
Read successful bytes with `purrdf_buffer_data`, then call `purrdf_buffer_free`;
the output lives independently of source/request storage. Errors use the existing
diagnostic and native-cause accessors and `purrdf_error_free` lifetime.

Python exposes `purrdf.expand_document(source: bytes, *, request_json: bytes) -> str`.
Immutable input owners remain borrowed while native work runs outside the GIL.
JavaScript exposes `expandDocument(source: Uint8Array, requestJson: Uint8Array): string`.
Its captured non-shared buffers receive admitted native copies; request admission
and compilation precede source capture. Detached/shared buffers refuse. Both
interfaces return JSON text and retain existing diagnostic/cause error fields.
The JavaScript native cause retains its existing `YamlSourceCause` owner and
`free()` lifetime.

## Document compaction and flattening across hosts

Compaction and flattening accept the same original source bytes and common
version-one request fields as expansion. They use the native structural engine
without constructing an RDF dataset. Each operation has a closed request grammar:
expansion refuses the additional fields below, and compaction refuses `flattening`.

```json
{
  "version": 1,
  "format": "jsonld",
  "base": "https://example.org/source/document",
  "output_context": {"label": "https://example.org/label"},
  "output_context_base": "https://example.org/contexts/",
  "output_base": {"mode": "input"},
  "compaction": {
    "compact_arrays": true,
    "compact_to_relative": true,
    "max_allocation_bytes": 34359738368
  }
}
```

| Additional field | Compaction and flattening meaning |
| --- | --- |
| `output_context` | Compaction requires an explicit bare expression, including a valid null reset. Flattening accepts omission or null to return the flattened expanded array; every non-null expression, including `{}`, selects ordinary compaction. |
| `output_context_base` | Optional absolute retrieval IRI string or null. Omission selects the original request `base`; explicit null selects no retrieval URL. |
| `output_base` | Optional closed record whose required `mode` is `input`, `none` or `iri`. Default `input` uses the effective document base retained by expansion. `none` supplies no initial semantic output base. `iri` requires an absolute `iri` string; other modes refuse that field. A null record is invalid. |
| `compaction` | Optional closed object accepting only boolean `compact_arrays` and `compact_to_relative`, both defaulting to true, and `max_allocation_bytes`, defaulting to 32 GiB. |
| `flattening` | Flattening only: an optional closed object accepting `max_allocation_bytes`, defaulting to 32 GiB, for native node-map construction. |

Both new allocation limits accept the full `u64` range and refuse before governed
construction. Expansion, flattening, compaction and serialized output keep
independent allowances. Context compilation and scoped state retain their existing
native limits. Moving an expanded or flattened value into another phase does not
refund it into that phase's account or establish an aggregate operation-memory cap.

The base roles are separate. Request `context` and `base` interpret source bytes.
The output context supplies emitted terms and obeys the same bare-expression
boundary: an expression object containing `@context` refuses, while term-scoped
contexts and offline registry documents retain their existing semantics.
Boolean and numeric output expressions refuse during request decoding, including
inside expression arrays.

`output_context_base` resolves relative offline output-context references. Its
default uses the original input document URL, before an in-document `@base`
changes or clears the semantic base. `output_base` selects the initial semantic
base for output-context compilation. Its `input` default uses the actual effective
document base, including root in-document base or null changes. The output
context's own base or null is then processed over that initial base.
`compact_to_relative: false` disables document-relative output selection;
vocabulary and prefix selection retain their native meaning.

When flattening has no non-null output context, there is no compaction phase.
Its compaction controls and output-base records still require the closed grammar,
valid types and ranges, and absolute IRI strings. A zero compaction allocation
limit therefore does not prevent context-free flattening. Empty `{}` is non-null
and does run compaction. Compaction applies its root finishing exactly once,
including the native context and graph envelope; a null compaction context resets
the context and emits no `@context` member.

Decidable request checks and input-context compilation precede source acquisition.
Output-context compilation follows expansion because it depends on the actual
effective base. No provisional base is substituted for that metadata.

| Operation | Rust operation | CLI command | C function | Python function | JavaScript function |
| --- | --- | --- | --- | --- | --- |
| Compact | `document::Operation::Compact` | `document-compact` | `purrdf_document_compact` | `compact_document` | `compactDocument` |
| Flatten | `document::Operation::Flatten` | `document-flatten` | `purrdf_document_flatten` | `flatten_document` | `flattenDocument` |

Rust callers use `purrdf_validate::document::execute(source, request_json, operation)`.
`document::request(request_json, operation)` returns a reusable request for
`document::execute_decoded(source, &request)`. Existing expansion functions and
their reusable request API remain available.

```sh
purrdf document-compact --request compact.json input.jsonld compacted.jsonld
purrdf document-flatten --request flatten.json input.yamlld flattened.jsonld
```

CLI stdin/path protection and output publication follow document expansion.
The new C functions use the same source/request pointer-length pairs and
buffer/error slots as `purrdf_document_expand`, with the same one-buffer ownership
and free functions. Python functions accept `source: bytes` and keyword-only
`request_json: bytes`, returning `str` while retaining the borrowed-input lifetime.
JavaScript functions accept two `Uint8Array` arguments and return JSON text;
admitted capture, retained native causes and their `free()` lifetime are unchanged.

Native orchestration can use `expand_jsonld_document_with_context` or
`expand_yamlld_document_with_context` to retain `JsonLdExpandedDocument::document_base`
alongside the expanded `value`. `CompiledJsonLdContext::compile_with_bases` reuses
the existing compiler with independent retrieval and initial semantic bases.
`flatten_expanded` exposes the existing normative node-map construction; the
existing value-only expansion and flattening entry points retain their behavior.

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
than earlier carrier output. The untyped no-base, no-options expanded byte baseline
is retained. Node types use IRI strings in both JSON-LD and YAML-LD. RDF JSON
literals keep typed lexical forms whenever raw JSON coercion would
change their spelling, collection shape or annotation identity.

## Document framing across hosts

Framing accepts original source bytes, original frame bytes and separate
version-one request JSON. It calls the native two-document byte adapter directly;
that adapter owns both expansions, the node map, framing, compaction and root
finishing. No RDF dataset or additional expansion is introduced.

```json
{
  "version": 1,
  "format": "jsonld",
  "base": "https://example.org/source/document",
  "context": {"label": "https://example.org/label"},
  "frame_context": {"id": "@id"},
  "frame_retrieval_base": "https://example.org/frames/frame",
  "output_base": {"mode": "input"},
  "framing": {"embed": "@once", "omit_graph": true}
}
```

The closed request reuses `version`, `format`, `base`, `context`, `registry`,
`yaml_ld`, `expansion` and `output` from document expansion. One format governs
both documents: JSON-LD or YAML-LD, including the native classifier's aliases.
One Basic `yaml_ld` selection/syntax/projection record governs both YAML
documents; each parse has its own account. There are no `frame_format` or
`frame_yaml_ld` fields and no mixed-format projection.

| Framing field | Accepted value and default |
| --- | --- |
| `frame_context` | Initial bare frame context expression. Omission means `{}`; explicit null remains null. Expression-position wrappers and scalar booleans/numbers refuse; legitimate scoped term definitions remain valid. |
| `frame_retrieval_base` | Absolute frame retrieval IRI string, or null. Omission and null both select no retrieval URL. The source base is never substituted. |
| `output_base` | The existing closed `mode: input`, `mode: none` or `mode: iri` record, defaulting to input. Only iri mode accepts and requires an absolute `iri`. |
| `compaction` | The existing closed compaction controls, with `compact_arrays` and `compact_to_relative` true and `max_allocation_bytes` 32 GiB by default. |
| `node_map` | Closed object accepting only `max_allocation_bytes`, default 32 GiB. |
| `framing` | Closed native policy and construction controls in the table below. |

`frame_context` compiles against exactly `frame_retrieval_base` and the shared
immutable offline registry. Its terms interpret the pattern but are not
implicitly emitted. The native fallback fills only an absent, non-explicit frame
base; an explicit frame-context `@base`, including null, survives.

Output context comes exclusively from the frame document's own `@context`:
missing emits `{}`, and explicit null remains null. Standalone `output_context`
and `output_context_base` request fields refuse. Native framing's base law is
preserved: output-base input selects the effective expanded source base only
when relative compaction is enabled; its false case selects none. Explicit none
and IRI modes keep their native meanings. The frame's own output context then
compiles over that initial semantic base with the independent frame retrieval
URL. This does not change standalone compaction or flattening base selection.

| `framing` control | Default |
| --- | --- |
| `embed` | `"@once"`; only `"@once"`, `"@always"` and `"@never"` are accepted. |
| `explicit`, `require_all`, `omit_default`, `frame_default` | `false` |
| `omit_graph` | `true` |
| `max_allocation_bytes` | 32 GiB |
| `max_work` | 1,073,741,824 deterministic native work units |
| `max_emitted_nodes` | 16,777,216 native subject occurrences |

Boolean aliases and local overrides in the frame document remain the native
engine's responsibility; the request's embed control requires one of the three
strings. Numeric phase controls accept the full `u64` range. Expansion's allowance
applies independently to source and frame expansion. Node-map, framing,
compaction and serialized output keep separate authorities; output still defaults
to 4 GiB of bytes and 32 GiB of construction. Context/registry storage, lexical
decoding, bridges and handles retain their existing accounts. These controls do
not claim an aggregate operation-memory bound.

Rust callers use `purrdf_validate::document::frame(source, frame, request_json)`.
For reuse, decode with `document::request(request_json, document::Operation::Frame)`
and call `document::frame_decoded(source, frame, &request)`. Decidable closed-record,
range, IRI and context checks finish before either document is acquired by a host.

```sh
purrdf document-frame --request framing.json source.yamlld frame.yamlld framed.jsonld
```

The CLI permits at most one stdin reader across request, source and frame, and
applies the existing path/global-option protections. C exposes
`purrdf_document_frame(source, source_len, frame, frame_len, request_json, request_len, out_buffer, out_error)`.
All three borrowed pointer/length pairs receive the existing null/addressable
length checks, and each document receives its native byte bound. Valid output
slots are cleared before work; aliased slots refuse. Success publishes one buffer
with the existing buffer/error free functions and original native causes.

Python exposes `frame_document(source: bytes, frame: bytes, *, request_json: bytes) -> str`;
all immutable owners remain borrowed while native work runs outside the GIL.
JavaScript exposes `frameDocument(source: Uint8Array, frame: Uint8Array, requestJson: Uint8Array): string`.
Request capture and compilation precede separately admitted source/frame captures.
Returned JSON text and retained native-cause ownership, including `free()`, follow
the other document operations.

## Native document framing

`native_codecs::jsonld::frame_value_with_context` frames a lexical JSON document;
`frame_jsonld_with_context` and `frame_yamlld_with_context` decode input and frame
bytes before calling the same operation. YAML uses the existing first/all-document
selection in `YamlLdOptions`. The algorithm targets [JSON-LD 1.1 Framing,
Recommendation of 16 July 2020, sections 4–5.3](https://www.w3.org/TR/2020/REC-json-ld11-framing-20200716/).
It expands through the structural walker, retains the original graph-local node
map, frames iteratively, and uses the shared compactor. It does not lower through
RDF or reconstruct the map from flattened output.

The context arguments have separate roles. `input_context` interprets the input.
`frame_context` supplies initial pattern terms and the offline registry;
`frame_retrieval_base` resolves frame references and local/registered contexts.
The frame's own `@context` is compiled independently for output using that retrieval
base. A missing frame context emits an empty context; explicit null emits null.
Initial pattern terms are not implicitly emitted. Registry lookup remains offline.
`JsonLdFramingBase::Input`, `None` and `Iri` select the initial output compaction
base: the expanded input document base, no base, or an
explicit absolute IRI. The frame's own output context is then processed over
that initial base, with its separate retrieval URL for context lookup. Its
explicit `@base`, including null, takes effect. Output selection does not change
the emitted context document or the initial pattern context's active meaning.

`JsonLdFramingPolicy` defaults to `JsonLdEmbed::Once`, with `explicit`,
`require_all`, `omit_default` and `frame_default` false, and `omit_graph` true.
Frame-local `@embed`, `@explicit`, `@requireAll` and `@omitDefault` override
the operation defaults for that frame. Implicit child frames inherit the effective
local flags; explicit child frames begin with operation defaults before applying
their own controls. Embedding admits exactly `@once`, `@always` and `@never`;
true aliases `@once`, false aliases `@never`. Ancestor cycles retain references
under every policy. Traversal follows lexical subject and property order.
The nested `JsonLdCompactionPolicy` retains both defaults, `compact_arrays: true`
and `compact_to_relative: true`, and keyword aliases select output spellings.
Root finishing follows section 5.1's concrete sequence: ordinary compaction first,
then a forced graph envelope only when `omit_graph` is false. Consequently,
`compact_arrays: false` retains a root array and its ordinary graph envelope even
for one result with `omit_graph: true`. The REC's general omitGraph prose describes
multiple-node envelopes but does not supply an additional true-case unwrapping
step after compaction.

Framing uses the merged graph by default. `frame_default: true`, or a top-level
frame property expanding to `@graph`, selects the default graph; a graph subframe
enters the corresponding named graph. The REC API initializes its graph name to
false and immediately tests for `@merged`. This implementation reads that internal
inconsistency using sections 4.1 and 5.3, which specify the merged/default choice.

`JsonLdFramingLimits` preserves separate `expansion`, `node_map` and `compaction`
construction accounts. Its new framing allowance defaults to 32 GiB and admits
new graph views, output, work frames, typed default markers and scratch before
allocation. Inputs and compiled contexts are borrowed. Context compilation,
scope state and resolver scratch retain their separate authority; there is no
aggregate operation-memory claim. `max_work` defaults to 2^30 deterministic
units, charging subject/property scans, matching, inherited subframes, recursive
matching and embedding. Frame expansion and context compilation precede that
counter. `max_emitted_nodes` defaults to 2^24 subject occurrences, including
identifier-only references. Limits refuse before governed work and return no
partial document. Native storage diagnostics retain their original cause.

Native RDF 1.2 triple terms and attached/detached annotations are atomic framing
values. Ordinary property wildcards and exclusions can retain or exclude them;
patterns inspecting `@triple`, its component controls, or `@annotation` internals
are explicitly refused. Identifier references inside native terms count when
pruning singly occurring blank-node identifiers, but cleanup never changes the
atomic values. Opaque `@json` payloads are copied and compared as whole values,
never interpreted or rewritten as JSON-LD. Private preservation markers occupy
separate typed state; ordinary user strings, including `@null`, and real JSON
null cannot become cleanup instructions.

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
