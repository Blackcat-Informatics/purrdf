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

An import of the shapes document's own IRI (`--base`, its `file://` retrieval
IRI, or an in-document `@base`), or of an ontology already in the shapes graph
(`<X> a owl:Ontology`, or an ontology whose `owl:versionIRI` is `<X>`), needs
no pair.
Every other `owl:imports` no pair resolves is refused by name, exactly as it is
for `validate --shapes`, and a pair the closure never reaches is refused as
unused. The product is never packed from a smaller shapes graph than the one
named.

This did not always hold. `shacl pack` used to read the shapes document
through a route with no import table and no diagnostic channel at all, so an
unresolved `owl:imports` was dropped with nothing printed, and the resulting
product carried FEWER shapes than the document it was packed from — a decided,
well-formed, wrong verdict every time it was restored. Packing with `--import`
and validating the document with the same `--import` now reach the
byte-identical report, by construction: the two commands read and fold the
closure through the same function.

The Python, C-ABI and WebAssembly bindings pack through the same resolution with
their own import table — `imports=` in Python, `importIris` / `importDocuments`
in JavaScript, `import_iris` / `import_documents` / `import_count` in C — and a
closure that is not in hand is refused with the same typed import error their
validation functions raise. See
[The same rule on every host](shacl.md#the-same-rule-on-every-host).

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

### `--box-role-vocab` is resolved and recorded at pack time, the same way `validate --shapes` resolves it

The graph-box role annotation feature takes a caller-supplied vocabulary — six
term IRIs derived by concatenation from one namespace — and PurRDF mints no
vocabulary IRIs of its own, so there is no default: without `--box-role-vocab`
the feature is simply **inactive**, exactly as it always has been.

```bash
purrdf shacl pack \
  --shapes shapes.ttl \
  --box-role-vocab https://example.org/meta/ \
  --out shapes.purrshp
```

`--box-role-vocab NS` derives `NSgraphBoxRole`, `NSboxABox`, `NSboxTBox`,
`NSboxRBox`, `NSboxCBox` and `NSboxConfigBox`, records the namespace into the
product's identity (the `box-role-vocab` component — see
[The refusal dimensions](#the-refusal-dimensions)), and is the identical
derivation `validate --shapes --box-role-vocab NS` spends the flag on: a
product packed with the flag and a document validated with the same flag parse
to the same `box_role_vocab` and therefore restore or run under the identical
`vocabulary` component. Omitting the flag packs exactly as it always has: the
component records **absent**, and no role annotation is collected or stamped.

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

"Zero host-injected dependencies" is a statement about what a product *needs*,
not a ban on host wiring. A Rust host that injects native SPARQL functions or
custom aggregates packs and restores through the entry points described in
[Host-injected implementations](#host-injected-implementations); every other
caller, and every non-Rust binding, uses the plain path above.

## What a product carries, and what it does not

A product carries three sections, always all three:

| Section | Carries |
| --- | --- |
| identity | the preparation stage id, the profile id, and the parse inputs (base, prefix map, `sh:shapesGraph` IRI, box-role vocabulary) |
| dataset | the shapes dataset itself, in PurRDF's succinct pack container |
| model | the declarative SHACL model — node shapes, constraints, paths, node expressions, SHACL-AF rules, `sh:SPARQLTargetType` declarations — and, as its last field, the reusable class analysis derived from it |

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

**The shape analysis really is removed, not merely checked.** The class analysis —
a cycle-safe walk collecting every reachable `sh:class`, `sh:targetClass` and
`shnex:instancesOf` IRI and assigning each a binding-row position — is carried as
the last field of the model section, and `admit` reads it rather than walking the
shape tree again. It is bound rather than trusted: the `class-catalog` component of
the identity is the digest of exactly those pairs, positions included, so a carried
analysis that is not the one the product was packed with is refused. And it cannot
go stale across builds, because the preparation stage id is digested from the class
walk's own source — a build whose reachability rule differs cannot share a stage id
with the one that packed the product, so `admit` refuses it and `rebuild` re-derives
instead.

It shares the model section rather than taking a fourth section of its own so that
the section directory and the container format version did not have to move when it
started travelling; a fourth section would have made every product ever packed fail
to *open*, which is the one state `rebuild` cannot rescue.

**Nor is the provenance of a restore carried, for the same kind of reason.** A
product records the inputs that decided what its shapes *mean*; how a particular
preparation was *obtained* is a fact about one restore in one process, so a
product that encoded it would carry a value that is false for every reader except
the one that wrote it. It would also move the preparation stage id — a digest over
the model's meaning — for a change that alters no meaning at all, invalidating
every product ever written. The answer lives on the restored preparation instead,
where it is true: see [Which shapes produced this
verdict](#which-shapes-produced-this-verdict).

RDF 1.2 term identity survives the round trip intact, including quoted triples,
reifier bindings, statement annotations, and `rdf:dirLangString` literals whose
base direction is part of their identity (`"x"@en--ltr` and `"x"@en--rtl` are
two distinct terms).

**SPARQL functions the shapes graph declares are carried, and they are carried
by the dataset.** A SHACL-AF §5 `sh:SPARQLFunction` is an IRI, an ordered
`sh:parameter` list, a `sh:select`/`sh:ask` body and a `sh:returnType` — all of
it stated by the shapes graph — so a restore re-derives the declaration from the
shapes dataset the product already carries rather than from a second
transcription of it in the model. There is one parser for those declarations,
and a restore runs it, which is what keeps a restored function identical to the
parsed one. The SHACL 1.2 expression-bodied declarations
(`sh:ListParameterExpressionFunction`) are carried too, from the model, because
their bodies *are* node expressions.

One capability is refused at pack time rather than lost at restore, on
`unsupported-capability`:

- a declared function that nothing in the model reaches — an expression-bodied
  declaration called only from `sh:sparql` query text, say. The model is what
  carries those declarations, so one the model never reaches is not in it, and
  a product written from it would resolve that call site to nothing and
  validate green. Call the function from the shapes model and it packs.

An incomplete `owl:imports` closure is not an admission dimension: no product
exists yet when it is refused, so every host reports it as the typed import
error its validation functions raise, and the product is only ever packed from
the merged closure — see
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

Four flags that describe a shapes **parse** are refused by name against a
product rather than accepted and ignored, because that parse does not happen
here:

| Flag | Why it is refused | Where it belongs |
| --- | --- | --- |
| `--shapes-from` | names the syntax a shapes document is read as, and a product is not a document | not applicable |
| `--shapes-graph` | the product records its own `sh:shapesGraph` IRI | pass `--shapes-graph IRI` to `purrdf shacl pack` and re-pack |
| `--import` | the `owl:imports` closure is folded at pack time | pass `--import IRI=FILE` to `purrdf shacl pack` |
| `--box-role-vocab` | the product records its own box-role vocabulary (or its deliberate absence) | pass `--box-role-vocab NS` to `purrdf shacl pack` and re-pack |

Everything describing the **data** graph stays live: `--from`, `--base`,
`--format`, the governor flags (`--fuel`, `--deadline`,
`--max-intermediate-cells`, `--max-scratch-bytes`, `--max-remote-requests`), and
the positional `IN`/`OUT`.

### Which shapes produced this verdict

Every `validate` run writes one `shacl shapes-provenance` line to stderr, before
the verdict, naming where its shapes came from:

```console
$ purrdf validate --shapes-product shapes.purrshp data.ttl
shacl shapes-provenance restored-admitted 9f2c…64 lowercase hex…1b
shacl conforms false
shacl results 2
```

```console
$ purrdf validate --shapes shapes.ttl data.ttl
shacl shapes-provenance parsed
shacl conforms false
shacl results 2
```

The token is total — there is always one, and none of them means "unknown". A
restored product renders the artifact's own **input binding**, the same 64
hexadecimal digits `shacl explain` prints on its `identity-digest` line, so a
report and the artifact behind it are compared on one spelling:

```bash
purrdf validate --shapes-product shapes.purrshp data.ttl 2> receipt
awk '/^shacl shapes-provenance restored-/{print $3}' receipt   # the artifact
```

The three tokens are `parsed`, `restored-admitted <digest>` and
`restored-rebuilt <digest>`. The two restore tokens stay distinct because the
digest means something different on each: **admitted** says the product's binding
was checked against this process before anything reached a validator, while
**rebuilt** says it was read off the artifact and deliberately *not* checked —
see [`rebuild`, the forward-compatibility
path](#binding-a-restore-to-the-product-you-meant) for why checking it there
would refuse exactly the products that path rescues.

This answers the half of the question authentication does not. Admission asks
*may this process execute these bytes?*; the provenance line answers *which
artifact did this report come out of?*, which is the question a consumer has
afterwards, looking at a verdict in a log.

### Binding a restore to the product you meant

Every check above asks about **this process** — is this the build that wrote the
memo, are these the registries the product was prepared against, is the class
analysis the product carries the one its own identity pins. Not one of them asks
whether the file named
on the command line is the product you wanted, because nothing in a product states
which product was meant. So naming the wrong one is not an error:

```console
$ purrdf validate --shapes-product WRONG.purrshp data.ttl
shacl conforms true
```

That is a decided, well-formed, authenticated verdict about a shapes graph nobody
asked about, and it exits `0`. `--expect-identity HEX` is how you say which
product you meant:

```console
$ purrdf validate --shapes-product WRONG.purrshp \
    --expect-identity 9f2c…64 lowercase hex…1b data.ttl
shacl dimension shapes-graph
purrdf: --shapes-product WRONG.purrshp: shapes-graph: this product was prepared
from the shapes graph whose input binding is 4b81…, and the caller required the
product bound to 9f2c…; …
```

`HEX` is the product's **input binding**: the 64 hexadecimal digits of the
identity that covers the shapes dataset, the `sh:shapesGraph` IRI, the prefix map,
the base, the profile, the box-role vocabulary, all three registries and the class
catalog at once. Read it off the product you intend with either verb that prints
it — `shacl explain`'s `identity-digest` line, or `shacl verify`'s stdout — and
pass it back unchanged:

```bash
WANT=$(purrdf shacl explain shapes.purrshp | awk '/^identity-digest /{print $2}')
purrdf validate --shapes-product shapes.purrshp --expect-identity "$WANT" data.ttl
```

The check is the **first** thing the restore does, ahead of the profile and the
stage id: a caller who named the wrong artifact is told that, rather than sent to
re-pack a product that was never the one they wanted. It costs a 32-byte
comparison, because the identity digest is already decoded by the time the bytes
have opened.

A satisfied expectation changes nothing else. `--expect-identity` with the
product's own digest produces the report byte-identical to the run without the
flag — the flag changes the door, not the answer.

Two spellings are refused as usage errors (exit `2`) rather than accepted:
`--expect-identity` against `--shapes`, because a shapes document has no prepared
binding to require; and a value that is not 64 hexadecimal digits, which names no
dimension because no product was ever inspected. Case is not significant on the
way in, so a selector that passed through a manifest or a CI variable in upper
case still names its product.

The writer is byte-deterministic, so the binding is a property of the shapes graph
and its parse inputs, not of a particular pack run: re-packing the same shapes
graph under the same base and prefixes yields the same digest, which is what makes
the selector worth writing down in a deployment manifest beside the product it
names.

Under the hood there are two ways a product becomes a validator again, and they
are two entry points at one boundary rather than a flag on one:

- **`admit`** — the common path. It re-derives nothing expensive: verify the
  profile, verify the stage id, check the host-supplied registries, restore the
  dataset, decode the model and the class analysis carried with it, link it, and
  check the complete input binding — the carried analysis included.
- **`rebuild`** — the forward-compatibility path, taken when the stage id is one
  this build does not know. It ignores the model section entirely — the carried
  class analysis with it — and re-derives the shapes, and their analysis, from the
  dataset the product carries, under the product's own recorded parse inputs. That
  is what makes it the remedy: a product it must rescue is by definition one whose
  carried analysis came from a different reachability rule.

Every binding reaches both paths, not only Rust: the CLI's `validate
--shapes-product --rebuild`, Python's `ShapesProduct.rebuild()`, the C ABI's
`purrdf_shapes_product_rebuild`, and WebAssembly's
`shaclProductValidateToSarifRebuild` are the identical two-entry-point seam
described here, each restoring the SAME carried dataset `admit` would restore a
memo of — so rebuilding a CURRENT product (one whose stage id this build does
know) reaches the byte-identical report `admit` does, on every surface. `rebuild`
composes with the "which product did you mean" expectation too: `--rebuild
--expect-identity`, `rebuild_expecting`, and the identical Rust
`ShapesProductView::rebuild_expecting` all check the expectation FIRST, exactly as
`admit_expecting` does, so re-deriving from the dataset never bypasses the binding
a caller required.

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

// Or state WHICH product you meant, and fail closed when it is not that one.
// `wanted` is the digest `declared_identity().digest()` returns for the product
// you intend — the same value `shacl explain` prints. `admit_expecting` and
// `rebuild_expecting` check it FIRST, ahead of everything else either restore
// seam checks, so the choice of repair strategy never bypasses the binding.
let prepared = ShapesProduct::open(&product)?
    .admit_expecting(&ShapesProfile::CORE, &HostBindings::empty(), &wanted)?;
let prepared = ShapesProduct::open(&product)?
    .rebuild_expecting(&ShapesProfile::CORE, &HostBindings::empty(), &wanted)?;
```

`ShapesProduct::open` runs the container's cheap integrity tier — magic, format
version, section directory, canonical offsets, zero padding, every section's
SHA-256, the identity region, the trailer, the whole-container digest — and
admits nothing. Holding the resulting view is a statement about framing, never
about fitness, which is exactly what makes reading a refused product's declared
identity useful.

## Host-injected implementations

Everything above passes `HostBindings::empty()`, which is what a product of
`ShapesProfile::CORE` needs: every capability such a product exercises is
declared by the shapes graph itself. A Rust host may still wire native SPARQL
functions or custom aggregates of its own, and when it does, the product binds
**two** facts about that host rather than one:

* the registries' **declarations** — each injected entry's IRI, arity and
  volatility — which is all a fingerprint reproducible in another process can
  ever cover; and
* an **implementation identity**: an opaque byte string the caller uses to name
  the build those declarations resolve to, such as a release version or a commit
  digest.

The second is not belt-and-braces. Two builds of one host can register the same
IRI to two different closures that declare identical arity and volatility and
compute different answers — indistinguishable by every fact a registry can state
about itself. A binding over declarations alone would admit a product prepared
against one build under the other and validate green, which is the silent wrong
answer this codec exists to rule out.

So a preparation whose injected population is not empty **cannot be packed
without an identity at all**: the writer refuses on `unsupported-capability`
rather than emitting a product whose host half nothing could check. And a
restore that supplies the same declarations under a different identity is
refused on the registry dimension that carries it.

```rust,ignore
use purrdf::validate::{
    admit_shapes_product_with_implementations, prepared_to_product_with_implementations,
};

// The byte string that tells this build of the host's natives apart from every
// other build of them. PurRDF never interprets it.
const BUILD: &[u8] = b"example.org/host@1";

// `prepared` carries the host's natives in `Shapes::functions`.
let product = prepared_to_product_with_implementations(&prepared, BUILD)?;

// ...and the restore names the same build, beside the registries it wires.
let restored = admit_shapes_product_with_implementations(
    &product,
    &functions,
    &aggregates,
    &property_functions,
    BUILD,
)?;
```

What the mechanism does not do is verify the identity. PurRDF cannot read a
host's machine code and confirm the bytes name it; the identity is the caller's
claim about its own build, and what the codec enforces is that a restore
claiming a *different* one stops at the boundary. A host that spells two
different builds with one identity has told the binding they are the same build.

An absent identity is the empty byte string, and it encodes as nothing at all:
the three host rows stay the bare declaration fingerprints. That is why a
product with nothing injected — which is every product the paths above write —
is byte-identical to what a build that had never heard of implementation
identities would produce.

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
stage-id 7416f78d…
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

`identity box-role-vocab 0x00` above is the encoding of **absent** — the
product this example packed with no `--box-role-vocab`. A product packed with
the flag renders a longer `0x…` run instead (the six derived term IRIs,
length-prefixed and key-sorted), never `0x00`; the two are deliberately
distinguishable so a restore can never confuse "the feature is inactive" with
"the feature is active over a vocabulary of empty IRIs".

This is what makes a named refusal actionable rather than a log line. A restore
refused on `prefixes` is answered by reading which prefix map the product
actually carries and fixing the configuration — not by guessing, and not by
re-packing blindly. Comparing that answer across **two** products — the one you
have and the one you meant — is [`shacl diff`](#comparing-two-products), rather
than running `explain` twice and comparing the rendering by eye.

## Comparing two products

```console
$ purrdf shacl diff a.purrshp b.purrshp
diff-count 1
diff box-role-vocab 0x00 0x0108626f78…
```

`shacl diff` opens **both** products WITHOUT admitting either — the same
framing-and-integrity-only tier `shacl explain` reads through — and prints
every identity component whose value differs, one `diff <label> <value-in-a>
<value-in-b>` line per component, in `A`'s own component order. `diff-count 0`
and no `diff` lines means the two products declared identical identities.

This is what makes a NAMED refusal actionable between two artifacts rather than
just one. `shacl explain` answers "what does THIS product say it was compiled
from"; before `diff` existed, an operator whose restore was refused on a named
dimension had to run `explain` twice — on the product in hand and on the one
they meant to restore — and compare the rendering by eye to find which
declaration moved. `diff` is that comparison, done once, over the identical
decoded components `explain` renders, so the two can never disagree about what
a component's value is.

Exit codes mirror `shacl verify`'s certified/refused split rather than
`validate`'s conforms/non-conforms one, because a `diff` that finds a
difference is a **decided answer about two artifacts**, not a validation
verdict about data: exit `0` when the two identities are identical, exit `1`
when they differ (the difference is printed on stdout either way), and exit `2`
for a usage error.

`diff` never admits either side, so it works on a product whose preparation
stage id this build does not recognize — the stage id lives in the product's
separate preparation memo, not in the identity `diff` compares, and is exactly
the situation an operator reaches for a diff to make sense of: "`validate
--shapes-product` refuses this file outright; what, concretely, would change if
I re-packed it?"

## The refusal dimensions

Every refusal carries one of twenty stable kebab-case labels, printed to stderr
as `shacl dimension <label>`. **The dimension is what you branch on; the message
is what you read.** Matching on message text is not supported.

The dimensions are checked from the outside of the container inward, and the
first one that fails is the one reported — so the label always names the
outermost unmet precondition rather than a downstream symptom of it.

The twenty split into two groups by **which verb can report them**, and the split
is not a taxonomy: it is the cost decision this codec is built around. A restore
is the common path and must not canonicalize a shapes graph's blank nodes;
certification is the cold path that does. A dimension only one of the two can
reach is not something the other quietly skips — it is a statement that verb never
makes.

### Admit-time dimensions

Reported by `validate --shapes-product` on every restore, by `shacl explain` for
the structural rows it reaches while opening the bytes, and by `shacl verify`,
which opens the product before it certifies anything.

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
| `shapes-graph` | prepared under a different `sh:shapesGraph` IRI — or, under [`--expect-identity`](#binding-a-restore-to-the-product-you-meant), carrying an input binding that is not the one you required | expose the shapes under the same named graph and re-pack with that IRI, or point the expectation at the product you meant |
| `prefixes` | prepared against a different prefix map | pack and execute under the same prefixes — the map decides which IRI a prefixed name denotes |
| `base` | prepared against a different base IRI | pack and execute under the same base — relative references resolve against it |
| `vocabulary` | prepared under a different box-role vocabulary | pass the SAME `--box-role-vocab NS` (or none) to both `purrdf shacl pack` and the executing side; PurRDF mints no vocabulary IRIs and there is no default to fall back on |
| `function-registry` | prepared against a different SPARQL function registry, or against a different build of the host implementations behind it | wire the same functions into the executing host, under the same implementation identity |
| `aggregate-registry` | prepared against a different custom-aggregate registry, or against a different build of the host implementations behind it | wire the same aggregates into the executing host, under the same implementation identity |
| `property-function-registry` | prepared against a different property-function registry, or against a different build of the host implementations behind it | wire the same relations into the executing host, under the same implementation identity |
| `class-catalog` | the class analysis the product carries is not the one its own identity pins | discard and re-pack from the shapes graph — the section and the binding over it no longer describe one analysis |
| `unsupported-capability` | well-formed bytes asking for something this build cannot honour, or a preparation this build will not write a product for | see the message — a declared function nothing in the model reaches, a decode larger than the scratch ceiling in force, or host implementations the packer was given no identity for |
| `depth-limit` | a structure nests past the decoder's fixed ceiling | re-pack from a shapes graph this build parses; the ceiling is a stack guard, not a semantic limit |
| `malformed` | structurally invalid in a way no other dimension names | discard and re-pack |

Three of those are three genuinely different actions and that is the point of
naming them: `malformed` is a corrupt cache to discard, `format-version` is a
stale artifact to recompile, and `function-registry` is a configuration error in
your own process that re-packing will not fix.

### The certify-time dimension

| Dimension | What it means | What to do |
| --- | --- | --- |
| `dataset-identity` | the shapes dataset the product carries does not *canonicalize* to the digest its own binding claims | discard and re-pack from the shapes graph — the section and the binding over it no longer describe one dataset |

Only `shacl verify` reports this one. A restore does **not** re-canonicalize the
shapes dataset: it takes that component from the product's own binding, because
the computation is a graph isomorphism over the shapes graph's blank nodes and
would plausibly cost more than the shapes parse a product exists to eliminate.
The envelope's per-section SHA-256 and whole-container digest still cover those
bytes on every path, so a swapped section is refused as `section-digest` or
`container-digest`; what only certification establishes is that the bytes
canonicalize to what the binding claims.

So a product whose stored canonical digest has been tampered with **will open,
will admit, and will fail `shacl verify`.** That is a documented split pinned by
test, not a hole: if you need the statement, run `verify` — from a build step, a
release check or a conformance harness, never before every validation.

One failure carries **no** dimension: a shapes or data *document* that does not
parse never reached the admission boundary, so nothing was inspected and naming
a dimension for it would claim otherwise. On the CLI no `shacl dimension` line
is printed; in Python the exception's `.dimension` is `None`; in JavaScript
`dimension` is `undefined`.

## The command-line verbs

| Command | Does | Exit `0` | Exit `1` | Exit `2` |
| --- | --- | --- | --- | --- |
| `purrdf shacl pack --shapes FILE --out OUT [--base IRI] [--shapes-graph IRI] [--import IRI=FILE] [--box-role-vocab NS]` | parse, prepare, write the product | product written | the shapes did not parse, the graph declares something a product cannot carry, or an `owl:imports` is unresolved | bad flags, `--shapes -`, or a `--shapes-graph`/`--import` the shapes graph cannot resolve |
| `purrdf shacl verify [IN]` | corroborate the carried dataset against the claimed identity | prints the identity digest | refused, with `shacl dimension <label>` on stderr | bad flags |
| `purrdf shacl explain [IN]` | print what the product says it was compiled from | prints the `key value` rendering | the bytes are not a well-formed product | bad flags |
| `purrdf shacl diff A B` | compare two products' declared identities, without admitting either | the two identities are identical | the two identities differ (the `diff` lines are still printed, on stdout) | bad flags, or both `A` and `B` naming standard input |
| `purrdf validate --shapes-product FILE [--rebuild] [--expect-identity HEX] [IN]` | restore and validate — `--rebuild` re-derives from the carried dataset instead of admitting the memo | validation ran | product refused | bad flags, a parse flag passed against a product, `--rebuild` or `--expect-identity` without a product, or an `--expect-identity` that is not 64 hexadecimal digits |

`IN` defaults to `-` (standard input) for `verify` and `explain`. For
`validate`, `--shapes-product -` collides with the data graph's own default of
`-` and is refused by name: there is only one standard input. `diff`'s two
products, `A` and `B`, are both required positional paths with no default, for
the identical reason: naming standard input for both would give each product
part of one byte stream.

`validate --shapes FILE [--box-role-vocab NS]` accepts the identical flag on
the parse lane — see [Producing a product](#producing-a-product) above — and
refuses it by name against `--shapes-product`, in the four-flag table above.

`validate` additionally exits `3` when a governor budget trips, and always
writes `shacl shapes-provenance <token>` before the run and `shacl conforms
true` or `shacl conforms false` plus `shacl results <N>` after it, all on
stderr — including when the shapes came from a product.

## From Python

```python
from purrdf import shapes

prepared = shapes.Shapes(shapes_ttl, base="https://example.org/shapes").prepare()
product = prepared.to_product()           # bytes, byte-deterministic

view = shapes.ShapesProduct.open(product) # framing and integrity only
print(view.format_version())             # 1
print(view.stage_known())                # True
print(view.identity_digest())            # 64 lowercase hex characters
for label, value in view.identity_components():
    print(label, value)

restored = view.admit() if view.stage_known() else view.rebuild()
report = restored.validate_nt(data_nt)
print(report.conforms)

# Which artifact did that verdict come out of? Total — always an answer, never
# "unknown": "parsed", "restored-admitted <digest>" or "restored-rebuilt <digest>".
print(restored.provenance())             # restored-admitted 9f2c…1b

# …or say which product you meant, and fail closed when it is not that one.
restored = view.admit_expecting("9f2c…64 lowercase hex…1b")

# The same binding, over the rebuild seam: the expectation is checked FIRST,
# ahead of the re-derivation, so choosing to rebuild never bypasses it.
restored = view.rebuild_expecting("9f2c…64 lowercase hex…1b")
```

`PreparedShapes.provenance()` is the Python spelling of the CLI's `shacl
shapes-provenance` receipt, and renders the identical token — the digest in it is
the value `identity_digest()` reports, so it can be handed straight back to
`admit_expecting()`. A preparation built by `Shapes(...).prepare()` answers
`parsed`, because it was never restored from an artifact and none may be invented
for it.

`ShapesProduct.certify()` is the cold path; call it from a build step or a test.
Refusals raise `shapes.ShapesProductError`, whose `.dimension` carries the label
string (or `None` when no product was ever inspected).

The submodule is `purrdf.shapes`. `purrdf.shacl` is a back-compatible alias for
the same object and keeps working, but new code should spell the canonical name:
the alias predates the surface this page describes, and the two names resolving to
one module is the kind of thing that reads as two APIs to someone learning it.

`admit_expecting(expected_identity)` is `admit()` bound to the product you meant:
it takes the 64 hexadecimal digits `identity_digest()` reports, and raises
`ShapesProductError` with `.dimension == "shapes-graph"` when the product carries
a different binding. A selector that is not a digest raises a plain `ValueError`
instead — no product was opened, so nothing may be blamed on one.

`rebuild()` re-derives from the SAME carried dataset `admit()` restores a memo
of, so calling it on a CURRENT product (`stage_known() == True`) reaches the
byte-identical report — it is a second door onto one product, never a second,
divergent answer. `rebuild_expecting(expected_identity)` is its bound twin,
taking the identical `expected_identity` `admit_expecting` does: the expectation
is checked FIRST, ahead of the re-derivation, exactly as it is on
`admit_expecting`, so a caller who reaches for the forward-compatibility rescue
never loses the binding.

## From JavaScript / WebAssembly

```js
import {
  ready,
  shaclPackProduct,
  shaclProductExplain,
  shaclProductCertify,
  shaclProductValidateToSarif,
  shaclProductValidateToSarifRebuild,
  shaclProductValidateToSarifExpecting,
  shaclProductValidateToSarifRebuildExpecting,
} from "@blackcatinformatics/purrdf";

await ready(); // one-time async wasm instantiation

const product = shaclPackProduct(shapesTtl, "https://example.org/shapes");
console.log(shaclProductExplain(product));

try {
  const sarif = shaclProductValidateToSarif(product, dataNt);

  // …or, for a stage id this guest does not know, re-derive from the carried
  // dataset instead of admitting the memo — the remedy the `stage-id` refusal
  // above would name.
  const rebuilt = shaclProductValidateToSarifRebuild(product, dataNt);

  // …or say which product you meant, and fail closed when it is not that one.
  const bound = shaclProductValidateToSarifExpecting(product, dataNt, wantHex);

  // …or both: rebuild AND fail closed when it is not the product you meant.
  // The expectation is checked FIRST, ahead of the re-derivation, so the
  // rescue never bypasses the binding.
  const boundRebuilt = shaclProductValidateToSarifRebuildExpecting(
    product,
    dataNt,
    wantHex,
  );
} catch (refusal) {
  console.error(refusal.dimension, refusal.message);
  refusal.free();
}
```

A wasm guest has no retrieval IRI to derive a base from, so the host supplies
`shapesBase` explicitly; it is recorded in the product, and a restore resolves
the same relative references without the document.

`shaclProductValidateToSarifRebuild` re-derives from the SAME carried dataset
`shaclProductValidateToSarif` restores a memo of, so calling it on a CURRENT
product reaches the byte-identical report — it is a second door onto one
product, never a second, divergent answer.
`shaclProductValidateToSarifRebuildExpecting` is its bound twin, taking the
identical `expectIdentity` `shaclProductValidateToSarifExpecting` does: the
expectation is checked FIRST, ahead of the re-derivation, exactly as it is on
`shaclProductValidateToSarifExpecting`, so a host that reaches for the
forward-compatibility rescue never loses the binding.

`shaclProductValidateToSarifExpecting` takes the 64 hexadecimal digits
`shaclProductExplain` prints on its `identity-digest` line. A product carrying a
different binding rejects with `dimension === "shapes-graph"`; a selector that is
not a digest rejects with `dimension === undefined`, because no product was
inspected. `shaclProductValidateToSarifRebuildExpecting` carries the identical
selector and the identical refusal shape.

## From C

The C ABI exposes `purrdf_shapes_product_encode`, `purrdf_shapes_product_open`,
`purrdf_shapes_product_admit`, `purrdf_shapes_product_admit_expecting`,
`purrdf_shapes_product_rebuild`, `purrdf_shapes_product_rebuild_expecting` and
`purrdf_shapes_product_certify`, with `purrdf_shapes_product_error_dimension`
returning the refusal's stable label (or `NULL` when the shapes document simply
did not parse).

`purrdf_shapes_product_rebuild` is the forward-compatibility twin of
`purrdf_shapes_product_admit`: it re-derives the preparation from the shapes
dataset the product carries instead of admitting its memo, which is the remedy
`purrdf_shapes_product_admit`'s `stage-id` refusal names.
`purrdf_shapes_product_rebuild_expecting` is its bound twin, taking the
identical `expect_identity` `purrdf_shapes_product_admit_expecting` does: the
expectation is checked FIRST, ahead of the re-derivation, exactly as it is on
`purrdf_shapes_product_admit_expecting`, so a C host that reaches for the
forward-compatibility rescue never loses the binding.

`purrdf_shapes_product_admit_expecting` is `purrdf_shapes_product_admit` bound to
the product you meant: it takes the `identity-digest` value
`purrdf_shapes_product_open` renders, as a NUL-terminated C string, and returns
`PURRDF_STATUS_SHAPES_PRODUCT_ERROR` with the dimension `shapes-graph` when the
product carries a different binding. A selector that is not 64 hexadecimal digits
returns `PURRDF_STATUS_INVALID_ARGUMENT` and no dimension.
`purrdf_shapes_product_rebuild_expecting` carries the identical selector and the
identical refusal shape. See
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
