<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Prepared Shapes Products

A shapes graph is parsed, linked and analyzed before a single focus node is
looked at, and that work is identical on every validation of the same document.
A **prepared shapes product** does it once and writes the result out as bytes: a
deterministic, versioned, authenticated artifact carrying the compiled SHACL
model plus its reusable analysis, so a process can restore a prepared validator
instead of re-parsing Turtle.

`purrdf shacl pack` writes one. `purrdf validate --shapes-product` restores one
instead of parsing a shapes document. `purrdf shacl verify` and
`purrdf shacl explain` are the admission surface for those bytes.

> **A product that comes back from disk is untrusted, and decoding it is
> admission rather than parsing.** Nothing in a product is evidence of its own
> provenance. The stage id, the profile and the complete input binding are all
> checked before any of it reaches the validator, and a mismatch is **refused on
> a named dimension** rather than validated under the wrong configuration. The
> failure that design exists to prevent is silent: a product prepared under one
> prefix map, base, vocabulary or function registry does not crash when restored
> under different ones — it validates, and returns a well-formed report about a
> shapes graph nobody asked for.

The design record behind this surface is
[`docs/design/purrdf-prepared-products.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-prepared-products.md).

## Producing a product

```bash
purrdf shacl pack \
  --shapes shapes.ttl \
  --base https://example.org/shapes \
  --out shapes.purrshp
```

`--shapes` and `--out` are required; `--out -` writes the raw bytes to stdout.
A receipt goes to stderr *after* the artifact, so it never contaminates a piped
product:

```console
$ purrdf shacl pack --shapes shapes.ttl --out shapes.purrshp
shacl product bytes 4213
```

`--shapes` must be a **path**, not `-`. A product records the base its shapes
graph was parsed under, and standard input has no retrieval IRI to derive one
from; passing `--shapes -` is a usage error telling you to give a path or name
the base with `--base`. When `--base` is omitted, the document's own `file://`
retrieval IRI is derived — the same base `validate --shapes` parses it under, so
packing and direct validation cannot disagree about what a relative reference
resolves to.

Turtle is the accepted shapes syntax because it is the one syntax carrying a
`@prefix`/`PREFIX` map recoverable from source text, and that map is the
fallback prefix environment every SHACL-AF `sh:select` body resolves against.
The product records it.

### `owl:imports` is folded at pack time, the same way `validate --shapes` folds it

`shacl pack` accepts the identical `--import IRI=FILE` table
`validate --shapes` does, resolves the same transitive closure, and packs the
merged graph — see [`owl:imports` in a shapes graph](shacl.md#owlimports-in-a-shapes-graph)
for the closure semantics, which are shared code, not a parallel
re-implementation:

```bash
purrdf shacl pack \
  --shapes root.ttl \
  --import https://example.org/lib=lib.ttl \
  --out shapes.purrshp
```

Naming **no** `--import` at all is not a refusal: each unresolved
`owl:imports` is reported on stderr as a `shacl warning` line and the product
is packed from the root graph alone — a shapes document may legitimately carry
an ontology header whose imports are irrelevant to its shapes. Naming **any**
pair makes the closure mandatory, exactly as it does for `validate --shapes`:
an `owl:imports` no pair resolves is refused by name, and a pair the closure
never reaches is refused as unused.

This did not always hold. `shacl pack` used to read the shapes document
through a route with no import table and no diagnostic channel at all, so an
unresolved `owl:imports` was dropped with nothing printed, and the resulting
product carried FEWER shapes than the document it was packed from — a decided,
well-formed, wrong verdict every time it was restored. Packing with `--import`
and validating the document with the same `--import` now reach the
byte-identical report, by construction: the two commands read and fold the
closure through the same function.

The Python, C-ABI and WebAssembly bindings call a lower-level, text-only pack
entry point that has no `--import` table at all, and refuse rather than fold
— see [What a product carries, and what it does not](#what-a-product-carries-and-what-it-does-not).

### `--shapes-graph` is resolved and recorded at pack time, the same way `validate --shapes` resolves it

SHACL-SPARQL exposes the shapes graph itself as a named graph — a `sh:select`
body may read `GRAPH $shapesGraph { … }` — and `--shapes-graph IRI` names what
that graph is called, overriding a `sh:shapesGraph` the document declares:

```bash
purrdf shacl pack \
  --shapes shapes.ttl \
  --shapes-graph https://example.org/shapes \
  --out shapes.purrshp
```

A relative value resolves against the SAME base the shapes document parses
under — the one `--base` names, or the document's own `file://` retrieval
IRI — which is the identical derivation `validate --shapes --shapes-graph`
spends the flag on. That agreement is the whole point: a product packed with
`--shapes-graph IRI` and a document validated with
`--shapes --shapes-graph IRI` expose `$shapesGraph` under the same absolute
IRI and reach the byte-identical report. Before this flag existed, `shacl
pack` had no way to record ANY override at all, so a shapes graph whose
SHACL-SPARQL bodies depend on `$shapesGraph` validated one answer through
`--shapes` and a different one through a restored product, with nothing on
the command line able to close the gap.

Omitting `--shapes-graph` packs exactly as it always has: the product records
whatever `sh:shapesGraph` the shapes graph itself declares, or none at all.

The writer is **byte-deterministic**. No wall clock, no randomness and no
hash-iteration order reach it, so two runs over the same document, base and
import table produce identical bytes and a content-addressed cache key over a
product is stable.

From Rust:

```rust,ignore
use purrdf::shapes::engine::{parse_shapes, PreparedShapes};
use purrdf::shapes::product::ShapesProfile;
use std::sync::Arc;

let shapes = parse_shapes(shapes_ttl, Some("https://example.org/shapes"))?;
let prepared = PreparedShapes::new(Arc::new(shapes));
let product: Vec<u8> = prepared.to_product(&ShapesProfile::CORE)?;
```

`ShapesProfile::CORE` is named explicitly and there is no default. It is the one
profile this build implements — SHACL Core plus SHACL-SPARQL plus SHACL-AF with
zero host-injected dependencies — and a caller cannot mint a profile of its own,
because a profile a caller can spell would be a claim about a product rather
than a fact about it.

## What a product carries, and what it does not

A product carries three sections, always all three:

| Section | Carries |
| --- | --- |
| identity | the preparation stage id, the profile id, and the parse inputs (base, prefix map, `sh:shapesGraph` IRI, box-role vocabulary) |
| dataset | the shapes dataset itself, in PurRDF's succinct pack container |
| model | the declarative SHACL model — node shapes, constraints, paths, node expressions, SHACL-AF rules, `sh:SPARQLTargetType` declarations |

Each section carries its own SHA-256; the envelope adds an identity-region
digest and a whole-container digest in a sealed trailer that also restates the
total length, so a product truncated anywhere or appended to anywhere is
refused.

The shapes **dataset** travels inside the product for two independent reasons.
SHACL-SPARQL exposes the shapes graph as a named graph — `$shapesGraph` is bound
for every `sh:select` body — so a product that dropped it would restore a
validator that answers `GRAPH $shapesGraph { … }` with zero rows and reports
that as success. And it is what `rebuild` re-derives from when a product was
written by another build; see [Restoring a product](#restoring-a-product).

**Targets are not carried, and there is no section for them.** Target resolution
is a property of a *data graph*: a product that cached resolved targets would
validate a new snapshot against the focus nodes of an old one, producing a
report in which every finding is correct and everything added since the product
was written is simply absent. Targets are resolved per bound dataset, every
time. What a product removes from the restore path is the parse, the model
construction and the shape analysis — reusable preparation and per-dataset
binding are different lines in the budget.

RDF 1.2 term identity survives the round trip intact, including quoted triples,
reifier bindings, statement annotations, and `rdf:dirLangString` literals whose
base direction is part of their identity (`"x"@en--ltr` and `"x"@en--rtl` are
two distinct terms).

Two capabilities are refused at pack time rather than lost at restore, both on
`unsupported-capability`:

- a shapes graph declaring a `sh:SPARQLFunction`, because only the SHACL 1.2
  expression-bodied declarations survive a restore and a product carrying one
  would resolve every call site of that function to nothing and validate
  green;
- an `owl:imports` this pack call has no way to resolve — but ONLY through the
  Python, C-ABI and WebAssembly bindings' lower-level, text-only entry point,
  which carries no `--import` table and no place to print a warning. The CLI's
  `purrdf shacl pack --import` is different: it folds the closure or reports
  each unresolved import on stderr exactly as `validate --shapes` does, and
  only refuses when an `--import` pair itself is unusable — see
  [`owl:imports` in a shapes graph](shacl.md#owlimports-in-a-shapes-graph).

## Shipping a product

A product is an artifact of the PurRDF build that wrote it, not a stable
interchange format. Ship it beside the binary that will restore it — in the same
container image, the same release bundle, the same build cache entry — and treat
its bytes as a cache key rather than as a document.

Two version facts govern whether a restore succeeds:

- The **container format version** is refused outright when it is not the one
  this build decodes. Re-pack with the build that will execute the product.
- The **preparation stage id** is a content-derived digest over the declarative
  model and the tables its meaning depends on. It is never hand-incremented, so
  it cannot fall out of step with what the model means. A product carrying a
  stage id this build does not know is *not* dead: `rebuild` restores it.

Because the writer is deterministic, a build system can address products by
content: pack, hash the bytes, and skip the work when the hash is already
present. `purrdf shacl explain` reads a product's own identity digest back out
without admitting anything, which is the cheap way to ask "is this the product
for this configuration?" before deciding to restore it.

## Restoring a product

```bash
purrdf validate --shapes-product shapes.purrshp --format sarif data.ttl
```

`--shapes` and `--shapes-product` are mutually exclusive, and exactly one is
required. The verdict is the identical one `--shapes shapes.ttl` reaches: the
same engine entry point runs, over the same restored shapes.

Three flags that describe a shapes **parse** are refused by name against a
product rather than accepted and ignored, because that parse does not happen
here:

| Flag | Why it is refused | Where it belongs |
| --- | --- | --- |
| `--shapes-from` | names the syntax a shapes document is read as, and a product is not a document | not applicable |
| `--shapes-graph` | the product records its own `sh:shapesGraph` IRI | pass `--shapes-graph IRI` to `purrdf shacl pack` and re-pack |
| `--import` | the `owl:imports` closure is folded at pack time | pass `--import IRI=FILE` to `purrdf shacl pack` |

Everything describing the **data** graph stays live: `--from`, `--base`,
`--format`, the governor flags (`--fuel`, `--deadline`,
`--max-intermediate-cells`, `--max-scratch-bytes`, `--max-remote-requests`), and
the positional `IN`/`OUT`.

Under the hood there are two ways a product becomes a validator again, and they
are two entry points at one boundary rather than a flag on one:

- **`admit`** — the common path. It re-derives nothing expensive: verify the
  profile, verify the stage id, check the host-supplied registries, restore the
  dataset, decode the model, link it, re-derive the class catalog, and check the
  complete input binding.
- **`rebuild`** — the forward-compatibility path, taken when the stage id is one
  this build does not know. It ignores the model section and re-derives the
  shapes from the dataset the product carries, under the product's own recorded
  parse inputs.

**`rebuild` is not a Turtle fallback.** No RDF text is parsed and no file is
read; the dataset travels inside the product under the envelope's digests. The
alternative — carrying only the compiled model and telling callers to keep the
source document for the day the format moves — would make forward compatibility
depend on a file the product does not own and cannot authenticate.

```rust,ignore
use purrdf::shapes::product::{HostBindings, ShapesProduct, ShapesProfile, STAGE_ID};

// Ask what the bytes are, without admitting them.
let known = ShapesProduct::open(&product)?.stage_id() == &STAGE_ID;

// Both restore seams consume the view, so re-open to take one.
let view = ShapesProduct::open(&product)?;
let prepared = if known {
    view.admit(&ShapesProfile::CORE, &HostBindings::empty())?
} else {
    view.rebuild(&ShapesProfile::CORE, &HostBindings::empty())?
};
```

`ShapesProduct::open` runs the container's cheap integrity tier — magic, format
version, section directory, canonical offsets, zero padding, every section's
SHA-256, the identity region, the trailer, the whole-container digest — and
admits nothing. Holding the resulting view is a statement about framing, never
about fitness, which is exactly what makes reading a refused product's declared
identity useful.

## Corroborating a product

```console
$ purrdf shacl verify shapes.purrshp
9f2c…64 lowercase hex…1b
```

`verify` is the codec's **cold** path and is deliberately not reachable from a
restore. It re-canonicalizes the shapes graph's blank nodes and compares the
result against the digest the product's identity claims — a graph-isomorphism
computation that can cost more than the shapes parse a product exists to
eliminate. Call it from a build step, a release check or a conformance harness;
never before every validation.

What `admit` establishes about the dataset is that the section is the one the
container sealed — the per-section SHA-256 and the whole-container digest both
cover its bytes, so a swapped section is refused. What only `verify` establishes
is that those bytes *canonicalize* to the digest the identity records. A product
whose stored canonical digest has been tampered with will open, will admit, and
will fail `verify`; that split is pinned by test and is not an accident of
layering.

It prints the product's 64-character lowercase-hex identity digest on stdout and
exits `0`. A refusal names its dimension on stderr and exits `1`.

## Reading a product without admitting it

```console
$ purrdf shacl explain shapes.purrshp
format-version 1
stage-id 5b7a53e6…
stage-known true
identity-digest 9f2c…
identity-components 11
identity source-dataset 0x1f0a…
identity shapes-graph "(present) https://example.org/shapes"
identity doc-prefixes 0x02657808…
identity base "(present) https://example.org/shapes"
identity profile "purrdf-shacl-core-v1"
identity box-role-vocab 0x00
identity user-functions-declared 0x4c1d…
identity user-functions-injected 0x4c1d…
identity aggregate-registry 0xb7e0…
identity property-function-registry 0xb7e0…
identity class-catalog 0x63aa…
parse-base https://example.org/shapes
parse-shapes-graph https://example.org/shapes
parse-prefixes 3
parse-prefix ex https://example.org/
parse-prefix sh http://www.w3.org/ns/shacl#
parse-prefix xsd http://www.w3.org/2001/XMLSchema#
```

Deterministic `key value` lines in a fixed order, terminated by a newline, so a
consumer can split on whitespace without a parser. An identity component's value
is rendered as `"text"` when it is printable UTF-8 and `0x…` lowercase hex
otherwise.

`stage-known` is the fact to act on: `false` says this build's `admit` will
refuse these bytes and `rebuild` is the path that still restores them.

This is what makes a named refusal actionable rather than a log line. A restore
refused on `prefixes` is answered by reading which prefix map the product
actually carries and fixing the configuration — not by guessing, and not by
re-packing blindly.

## The refusal dimensions

Every refusal carries one of twenty stable kebab-case labels, printed to stderr
as `shacl dimension <label>` by `shacl verify`, `shacl explain` and
`validate --shapes-product` alike. **The dimension is what you branch on; the
message is what you read.** Matching on message text is not supported.

The dimensions are checked from the outside of the container inward, and the
first one that fails is the one reported — so the label always names the
outermost unmet precondition rather than a downstream symptom of it.

| Dimension | What it means | What to do |
| --- | --- | --- |
| `magic` | these bytes never were a prepared shapes product | hand the loader a file written by `purrdf shacl pack` |
| `format-version` | the container format is not the one this build decodes | re-pack with the PurRDF build that will execute it |
| `stage-id` | the model memo was written against a model this build no longer has | restore with `rebuild`, or re-pack |
| `profile` | prepared under a different preparation profile | re-pack with the build that will execute it |
| `truncated` | the bytes end inside a structure they declared | re-pack — this is an interrupted write, not corruption in place |
| `trailer` | the trailer is absent or inconsistent with the bytes ahead of it | discard and re-pack |
| `section-digest` | a section no longer matches its recorded digest | discard and re-pack — the product is corrupt in place |
| `container-digest` | the whole-container digest disagrees with the bytes | discard and re-pack — something outside the section bodies moved |
| `dataset-identity` | prepared from a different shapes dataset | re-pack from the same shapes graph the execution loads |
| `shapes-graph` | prepared under a different `sh:shapesGraph` IRI | expose the shapes under the same named graph, or re-pack with that IRI |
| `prefixes` | prepared against a different prefix map | pack and execute under the same prefixes — the map decides which IRI a prefixed name denotes |
| `base` | prepared against a different base IRI | pack and execute under the same base — relative references resolve against it |
| `vocabulary` | prepared under a different box-role vocabulary | supply the same vocabulary; PurRDF mints no vocabulary IRIs and there is no default to fall back on |
| `function-registry` | prepared against a different SPARQL function registry | wire the same functions into the executing host |
| `aggregate-registry` | prepared against a different custom-aggregate registry | wire the same aggregates into the executing host |
| `property-function-registry` | prepared against a different property-function registry | wire the same relations into the executing host |
| `class-catalog` | the pinned class analysis is not the one this build re-derives | re-pack with the build that will execute it |
| `unsupported-capability` | well-formed bytes asking for something this build cannot honour | see the message — a `sh:SPARQLFunction` declaration, or a decode larger than the scratch ceiling in force |
| `depth-limit` | a structure nests past the decoder's fixed ceiling | re-pack from a shapes graph this build parses; the ceiling is a stack guard, not a semantic limit |
| `malformed` | structurally invalid in a way no other dimension names | discard and re-pack |

Three of those are three genuinely different actions and that is the point of
naming them: `malformed` is a corrupt cache to discard, `format-version` is a
stale artifact to recompile, and `function-registry` is a configuration error in
your own process that re-packing will not fix.

One failure carries **no** dimension: a shapes or data *document* that does not
parse never reached the admission boundary, so nothing was inspected and naming
a dimension for it would claim otherwise. On the CLI no `shacl dimension` line
is printed; in Python the exception's `.dimension` is `None`; in JavaScript
`dimension` is `undefined`.

## The command-line verbs

| Command | Does | Exit `0` | Exit `1` | Exit `2` |
| --- | --- | --- | --- | --- |
| `purrdf shacl pack --shapes FILE --out OUT [--base IRI] [--shapes-graph IRI] [--import IRI=FILE]` | parse, prepare, write the product | product written | the shapes did not parse, or the graph declares something a product cannot carry | bad flags, `--shapes -`, or a `--shapes-graph`/`--import` the shapes graph cannot resolve |
| `purrdf shacl verify [IN]` | corroborate the carried dataset against the claimed identity | prints the identity digest | refused, with `shacl dimension <label>` on stderr | bad flags |
| `purrdf shacl explain [IN]` | print what the product says it was compiled from | prints the `key value` rendering | the bytes are not a well-formed product | bad flags |
| `purrdf validate --shapes-product FILE [IN]` | restore and validate | validation ran | product refused | bad flags, or a parse flag passed against a product |

`IN` defaults to `-` (standard input) for `verify` and `explain`. For
`validate`, `--shapes-product -` collides with the data graph's own default of
`-` and is refused by name: there is only one standard input.

`validate` additionally exits `3` when a governor budget trips, and always
writes `shacl conforms true|false` and `shacl results <N>` to stderr — including
when the shapes came from a product.

## From Python

```python
from purrdf import shacl

prepared = shacl.Shapes(shapes_ttl, base="https://example.org/shapes").prepare()
product = prepared.to_product()          # bytes, byte-deterministic

view = shacl.ShapesProduct.open(product) # framing and integrity only
print(view.format_version())             # 1
print(view.stage_known())                # True
print(view.identity_digest())            # 64 lowercase hex characters
for label, value in view.identity_components():
    print(label, value)

restored = view.admit() if view.stage_known() else view.rebuild()
report = restored.validate_nt(data_nt)
print(report.conforms)
```

`ShapesProduct.certify()` is the cold path; call it from a build step or a test.
Refusals raise `shacl.ShapesProductError`, whose `.dimension` carries the label
string (or `None` when no product was ever inspected).

## From JavaScript / WebAssembly

```js
import {
  ready,
  shaclPackProduct,
  shaclProductExplain,
  shaclProductCertify,
  shaclProductValidateToSarif,
} from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const product = shaclPackProduct(shapesTtl, "https://example.org/shapes");
console.log(shaclProductExplain(product));

try {
  const sarif = shaclProductValidateToSarif(product, dataNt);
} catch (refusal) {
  console.error(refusal.dimension, refusal.message);
  refusal.free();
}
```

A wasm guest has no retrieval IRI to derive a base from, so the host supplies
`shapesBase` explicitly; it is recorded in the product, and a restore resolves
the same relative references without the document.

## From C

The C ABI exposes `purrdf_shapes_product_encode`, `purrdf_shapes_product_open`,
`purrdf_shapes_product_admit` and `purrdf_shapes_product_certify`, with
`purrdf_shapes_product_error_dimension` returning the refusal's stable label (or
`NULL` when the shapes document simply did not parse). See
[`crates/rdf-capi/include/purrdf.h`](https://github.com/Blackcat-Informatics/purrdf/blob/main/crates/rdf-capi/include/purrdf.h)
and [Getting Started: C](../getting-started/c.md).

## Related

- [SHACL](shacl.md) — the validator itself, its coverage, and the report model.
- [`docs/design/purrdf-prepared-products.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-prepared-products.md)
  — the design record: why admission rather than parsing, why three seams, the
  derived stage identity, the census closure, and the bounded guarantee.
- [Codecs & Determinism](../concepts/codecs.md) — the byte-determinism rules
  every PurRDF writer follows, including this one.
- [Base IRIs & Relative References](../concepts/base-iris.md) — what `--base`
  records and why a product refuses to guess it.
- [Performance](../project/performance.md) — the report-only benches that
  decompose the pack and restore phases.
