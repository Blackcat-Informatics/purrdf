<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# PurRDF prepared products: admission, three seams, and the guarantee's edges

A **prepared SHACL product** is a deterministic, versioned, authenticated byte
artifact carrying the declarative `Shapes` model plus its reusable analysis, so a
process can restore a prepared validator instead of re-parsing a Turtle shapes
graph. `purrdf shacl pack` writes one; `purrdf validate --shapes-product`
restores one; `purrdf shacl verify` and `purrdf shacl explain` are the surfaces
for deciding what a product in hand actually is.

This document records the decisions behind that surface which a reader would
otherwise take for oversights — three entry points where one flag would have
done, a version constant that is not a constant, a catalog carried as a digest
with no body, a section that will never exist — and states the limits of the
guarantees it makes.

All example IRIs use `example.org`. PurRDF mints no vocabulary IRIs.

---

## 1. Decoding is admission, not parsing

A product is compiled once and then handed around: cached on disk, shipped
between processes, restored after a restart. By the time the bytes come back they
are **untrusted**. Nothing in them is evidence of its own provenance, and the
validator loading them cannot assume it produced them, produced them at this
version, or produced them against the same caller-supplied configuration. So
decoding is not parsing — it is **admission**, and every field is a claim the
loader has to check before any of it reaches the validator.

That framing is not rhetoric; it decides the failure being designed against. A
parser's failure mode is a crash on bad bytes, which is loud. An admission
boundary's failure mode is quiet: a product prepared under one prefix map, one
base, one vocabulary or one function registry encodes decisions that are only
correct under those inputs. Loading it under different ones does not crash. It
validates, and returns a well-formed report about a shapes graph nobody asked
about. Nothing anywhere says a word.

Two consequences follow, and both are load-bearing.

**The container is authenticated, not merely checksummed.** The bytes are a
`purrdf_core::artifact` envelope under the magic `PURRSHP1`: fixed header, a
section directory in ascending kind order with a per-section SHA-256,
zero-verified alignment padding, an identity region under its own digest, and a
trailer restating the total length and sealing every byte ahead of it. The reader
recomputes every offset and refuses a buffer whose stored offsets disagree, so
there is no second admissible spelling of the same content. It fails closed at
the first inconsistency rather than collecting diagnostics — the totality law the
GTS reader uses is right for a transport log someone else wrote, and wrong here,
because continuing past an inconsistency means handing partially-verified bytes to
a layer that will treat them as certified.

**The identity is decodable, not a bare digest.** The envelope's identity region
carries eleven ordered, labelled components — the source dataset's canonical
digest, the shapes-graph IRI, the key-sorted prefix map, the base, the profile id,
the key-sorted box-role vocabulary, the two user-function populations, the
aggregate registry, the property-function registry, and the class catalog's
digest. The obvious alternative was one 32-byte hash over all of it. It is
smaller, it answers exactly one question — *do these match?* — and at the only
moment anyone looks at it, when the answer is no, it has nothing further to say.
A decodable identity lets a refusal name the component and both sides' values,
which is the difference between "digest mismatch" and "the base moved from
`https://example.org/a` to `https://example.org/b`". The operational failure
prevented is an unexplainable refusal a caller can only answer by rebuilding
everything and hoping.

The component **order is part of the format**: the check maps a position to a
refusal dimension, so reordering the rows is a breaking change to every product
already written, not a cosmetic edit.

## 2. Three seams, because the common case decides the shape

Performance in the common case is paramount; conformance testing may be slow,
because it is never part of normal usage. That standing rule is the entire reason
the reader has three entry points rather than one with a flag.

| Path | Tier | Re-derives | Reached when |
|---|---|---|---|
| `admit` | common — every restore | nothing expensive | the stage id is one this build knows |
| `rebuild` | forward compatibility | the whole shapes graph, from the carried dataset | the stage id is one this build does **not** know |
| `certify` | cold — never in normal usage | the shapes dataset's canonical identity | a verify subcommand, a conformance harness, a test |

`ShapesProduct::open` sits underneath all three. It runs the envelope's cheap
integrity tier and decodes the product's self-description, and it **admits
nothing** — holding the resulting view is a statement about framing and integrity,
never about fitness. That is what makes reading a product's declared identity
useful: a caller can see what a product says it was compiled from, in order to
decide what to do about it, without any of it reaching a validator.

The split between `admit` and `certify` is the one that must not erode.
Certification re-canonicalizes the shapes dataset, which is a graph-isomorphism
computation over its blank nodes. Putting that on the restore path would very
plausibly cost more than the shapes parse this whole feature exists to eliminate —
a product slower than the thing it replaces, with every test still passing,
visible only in a benchmark nobody ran. So certification is not reachable from
`admit` by construction rather than by convention: `admit` takes the identity's
dataset component from the product's own binding verbatim instead of recomputing
it. Two tests pin that property from opposite sides — a product whose stored
canonical digest has been tampered with (every section digest left intact) **must
open, must admit, and must fail certification**. If someone quietly moves the
canonicalization onto the common path, those tests fail.

What `admit` does still establish about the dataset is worth stating precisely,
because it is easy to over-read. The envelope's per-section SHA-256 and
whole-container digest both cover the dataset's bytes, so the section cannot be
swapped without the container refusing first. What is deliberately *not*
established is that those bytes canonicalize to the digest the identity claims.
That is the one statement only `certify` makes.

### 2.1 `rebuild` is a seam, not a mode flag

The writer has exactly **one** path. A product always carries every section, so
there is no "with dataset" and "without dataset" variant to choose between, and no
writer-side switch anyone can set wrong. The section directory is total: the
identity section, the dataset section and the model section are present in every
product, any of them possibly zero-length.

The reader has **two total entry points at one boundary**. A reader that meets a
stage id it does not know refuses `admit` — the memo was written against a model
this build no longer has — and can still `rebuild` the preparation from the
authenticated primitive the product carries.

This is the same discipline the retrieval-composition record states for its
pipeline, read in the other direction: a flag that changes what a stage produces
is a second behaviour the first must be kept in agreement with, while a seam is a
different, equally-supported place to enter the same boundary. A
`rebuild: bool` on one entry point would be one function with two internal
regimes, and the regime nobody exercises is where the refusals go unrun.

**`rebuild` is not a text fallback.** No RDF text is parsed and nothing is
re-read from disk. The shapes dataset travels *inside* the product, under the
envelope's per-section SHA-256 and whole-container digest, and rebuilding
re-derives the shapes from that under the product's own recorded parse inputs. The
rejected alternative — carry only the compiled model and tell callers to keep the
source document around for the day the format moves — makes forward compatibility
depend on a file the product does not own and cannot authenticate. That is not a
fallback; it is an instruction to hope.

Rebuild deliberately checks neither the stage id nor the identity. Every identity
component it could compare against is a claim made by a build whose model is not
this one's, so checking them would refuse precisely the products this path exists
to rescue. What binds a rebuilt preparation is the derivation itself. The
**profile** *is* checked, because the profile moves only when the format's meaning
is redefined, at which point the carried dataset no longer means what this build
would make of it.

Carrying the dataset is not only the forward-compatibility answer. SHACL-SPARQL
exposes the shapes graph itself as a named graph — `$shapesGraph` is bound for
every `sh:select` body — so a product that dropped the dataset would restore a
validator that still loads, still verifies, and answers `GRAPH $shapesGraph { … }`
with zero rows. The report would be well formed and would describe a shapes graph
nobody asked about. The dataset also carries RDF 1.2 term identity across the
boundary for free: quoted triples, reifier bindings, statement annotations, and
`rdf:dirLangString` literals whose base direction is part of their identity
(`"x"@en--ltr` and `"x"@en--rtl` are two distinct terms) all survive because the
pack codec already round-trips every one of them. Re-serializing the shapes graph
as N-Quads and reparsing it would have been a second codec for that identity, and
a second codec is a second place for a term kind to go missing.

### 2.2 Certification sits on the producer

There is exactly one canonicalization in the whole codec that is not on a cold
path, and it is on the **writer**: `to_product` certifies the shapes dataset's
canonical identity before it states it, because a product cannot honestly state
an identity it has not established.

The asymmetry is the point. One producer, many restores. Paying canonicalization
once in a build tool that already owns the shapes document is a different budget
line from paying it on every process start in every consumer, and the consumer is
the party this feature exists to relieve. The producer refuses *before the bytes*
for the same reason: every capability check runs before the container is
assembled, so a shapes graph declaring something a product cannot carry is refused
with nothing emitted. Writing a product no `admit` could ever accept would move a
writer-side defect into a reader-side mystery, at some later date, on some other
machine.

## 3. The stage identity is derived, and that is what a hand-bumped constant gets wrong

`STAGE_ID` is a content-derived capability digest over the whole declarative model
plus the tables the model's meaning depends on. It is **never hand-incremented**.
`crates/shapes/tests/product_model_census.rs` computes it from the live sources
with `syn` and pins the result; the shipped constant is that digest, read from the
one place products are actually written under rather than transcribed beside it.

A hand-maintained version counter is precisely how an authenticated cache serves
stale-but-verified wrong answers. The mechanism is worth spelling out, because the
failure is invisible from every angle that normally reports one: someone changes
what a model type *means* without changing its shape, the counter is not bumped
because nothing looked different, the product's bytes still verify against every
digest they carry, the counter still matches — and the meaning moved underneath
both. Every integrity check in the container passes. The answer is wrong. Here the
digest **is** the meaning, so the two cannot separate.

The model's own byte stream therefore carries no format-version byte of its own,
deliberately. A second hand-maintained counter would reintroduce exactly the
defect the derivation removes. The container framing carries the one format
version this format needs.

The stage id sits *below* the profile id in coarseness. The profile moves only
when the profile's meaning is redefined, at which point every product written
under the old meaning must stop opening — which is exactly what changing it does.
The stage id moves whenever the model's content moves, and a product carrying one
this build does not know is not refused outright: it is the `rebuild` case.

## 4. The census closure, and the types an enumeration had missed

A codec that writes a model out and reads it back invites one quiet failure: a
variant nobody taught the writer about is skipped, the bytes still verify, the
reader still loads, and a shape that should have constrained the data simply is
not there. Nothing is broken enough to notice.

"Never silently omit a shape" is therefore structural rather than promised in a
comment. The census reads the declarative model out of `crates/shapes/src/**/*.rs`
with `syn` and reports every type, variant and field a codec has to handle. The
codec's own totality is then enforced by the compiler — wildcard-free matches in
the encoder, exhaustive struct construction in the decoder — so a new variant or
field does not build until the codec handles it.

The decision worth recording is that the census list is **not** a hand-maintained
enumeration. It is checked for equality against the transitive closure of the
model reachable from `Shapes` through public fields, so the list cannot go stale
in either direction: a reachable type missing from it fails, and a row that the
model no longer reaches fails too.

This is not hypothetical tidiness. The closure turned up types a hand-written
enumeration had missed, and they were load-bearing ones: `SparqlTargetType` and
`TargetTypeParam`, reached through `Shapes::target_types`, which carry the
`sh:SPARQLTargetType` declarations; and `Rule`, `RuleBody` and `RuleSchedule`,
which carry the SHACL-AF rules. A product built against the enumeration would have
verified, restored, and validated with the rules missing. An enumeration silently
omits exactly the types nobody happened to think of, which is the same set as the
types nobody will think to check.

The closure's frontier is derived rather than declared: the walk crosses public
fields and stops at types the crate does not define, so `Arc`, `BTreeMap`,
`String` and the registry types outside `crates/shapes/src` are leaves because the
scan never sees a definition for them. `pub(crate)` fields are deliberately not
crossed — the retained dataset and the parse provenance are reconstructed rather
than carried, and a caller cannot name them at all.

One row is admitted to the census that no public field reaches, and only on a
stated reason. `SparqlCallForm` is the lowering table for SHACL 1.2 Node
Expressions `sparql:<NAME>` calls; its *output* is baked into query text at
shapes-load, so no field of a parsed `Shapes` holds one — but the table decides
what that baked text says. Changing `sparql:add` from an infix `+` to anything
else changes the meaning of every prepared product carrying a lowered call while
leaving every reachable field byte-identical. That is precisely the stale meaning
the stage id exists to catch, so the form set is censused and digested.

The census is also proved not to be asleep. A synthetic source carrying an
undeclared reachable type must be flagged; a synthetic added variant must appear
in the census rows *and* must move the stage id; `#[cfg(test)]` items must not be
seen at all. The mirror checks are there too, because a census that fired on prose
would be re-pinned reflexively and stop being read: a reworded doc comment must
**not** move the stage id.

## 5. A refusal names a dimension, and the label set is a pinned contract

A refusal is a claim too, so it carries the exact thing that failed.
`ProductDimension` is the closed set of twenty ways a candidate product can fail
admission, ordered from the outside of the container inward — first the bytes are a
product at all (`magic` through `trailer`), then their contents are intact
(`section-digest`, `container-digest`), then the identity they were prepared
against matches the identity supplied for execution (`dataset-identity` through
`class-catalog`), and only then the residual structural refusals
(`unsupported-capability`, `depth-limit`, `malformed`). A refusal reports the
*first* dimension that fails, so the message always describes the outermost unmet
precondition rather than a downstream symptom of it.

The rejected alternative was a single opaque error carrying a string. It types
perfectly well, and it collapses "these bytes are garbage" into "you supplied a
different function registry than the one this product was prepared against", so
the only available recovery is the pessimistic one. Callers then do the
predictable thing and match on substrings of the message, which converts every
wording improvement into a silent behaviour change downstream. The three
dimensions above are three different actions: a `malformed` product is a corrupt
cache to discard and rebuild, a `format-version` mismatch is a stale artifact to
recompile, a `function-registry` mismatch is a configuration error in the caller's
own code that rebuilding will not fix.

The kebab-case labels are a **pinned contract**. A frozen conformance corpus
records them as refusal discriminants and every language binding carries them
across its own boundary unchanged, so renaming one is a breaking change to that
corpus rather than a cosmetic edit. They are deliberately distinct from the prose
a refusal renders: the dimension is what a caller branches on, the message is what
a human reads, and matching on message text is not supported.

The messages themselves are prescriptive — they name the action that resolves the
refusal, not only the condition that caused it — and the action genuinely differs
per dimension, which is why the text is per-dimension rather than generic. Two
`#[non_exhaustive]` error vocabularies reach this mapping from below, and both
wildcard arms land on `malformed`, fail-closed, rather than being attributed to
whichever component happens to sit first.

The binding boundary every language surface routes through is honest about the one
place there is no dimension. A shapes *document* that does not parse never reached
the admission boundary at all, so no dimension names it; inventing one would claim
a product was inspected when none was ever written. That case is its own arm, and
its dimension is `None`.

### 5.1 Refusals were executed in pairs

Over-refusal is the mirror of a silent drop and the harder of the two to notice,
because every test still passes and the strictness *looks* correct right up until
someone restores a product that should open and doesn't. So every refusal on this
boundary is executed alongside a neighbouring valid case: the corrupt product is
refused on its named dimension, **and** the unmodified product still admits.

Two ceilings were set with that failure in mind rather than by picking a round
number.

The model codec's `MAX_DEPTH` is 128 — twice the evaluator's own recursion
ceiling. The factor is the point: a ceiling *below* the evaluator's would refuse
products the evaluator could have run. Authored SHACL nests in single digits; 128
is a stack guard, not a second semantic limit. It is unconditional rather than
budgeted because stack exhaustion is an abort no caller can handle.

The governed-decode refusal estimates a decode's size as the **sum of the declared
section lengths and nothing cleverer**, because the decoded form of a section is
never smaller than its bytes. An estimator that guessed high would refuse work
that would have fit. A lower bound can only ever refuse a decode that genuinely
could not have fit. It is also reported as a refusal at admission rather than
charged as consumption, since nothing has run yet and a "consumed" figure would
promise a number something actually spent.

## 6. The shipped profile is named, explicitly selected, and mints nothing

There is one profile, `purrdf-shacl-core-v1`, and it is selected by name. It is
not a default, and there is no fallback: a caller names `ShapesProfile::CORE` and
cannot mint a profile of its own, because a profile a caller can spell is a claim
about a product rather than a fact about it. A product prepared under any other
profile is refused, not best-effort opened.

The profile is SHACL Core + SHACL-SPARQL + SHACL-AF with **zero host-injected
dependencies**: every capability a product of this profile can exercise is
declared by the shapes graph itself. That is a statement about what a product
*needs*, not a ban on host bindings. An empty host always suffices to restore a
product of this profile's own capabilities; a host that additionally wires custom
aggregates or native functions may still do so, and the product's identity binds
whichever it was prepared against, so a restore under different ones is refused
rather than silently executed. What the profile genuinely excludes is the one
binding no shapes graph can describe: a product of this profile is prepared
against the *empty* property-function registry, because `sh:sparql` bodies calling
host relations depend on wiring the shapes graph cannot state and a product
therefore cannot carry.

**It mints no vocabulary, and the reason is that its host vocabulary is honestly
empty rather than defaulted.** The box-role vocabulary is caller-supplied
configuration with no fabricated default, so the product treats it as an identity
component in its own right, with its own `vocabulary` refusal. That is not
garnish. A product that omitted it would restore a `Shapes` carrying no
vocabulary, which validates *differently* from the parsed one — every box-role
list stays empty and no role lookup happens — and no other check in the codec
catches it, because the model, the dataset and every registry are identical. Its
encoding distinguishes absent from present-and-empty, because "the box-role
feature is inactive" and "the feature is active over a vocabulary of empty IRIs"
are two different parses, and an encoding that conflated them would let one
product open against the other's inputs. The same discrimination applies to the
shapes-graph IRI: naming no graph and naming the empty IRI are two
configurations, and only one of them binds `$shapesGraph`.

Installing is also distinguished from fingerprinting, and the distinction is
load-bearing. Both restore seams **install** the host's registries into the
restored shapes rather than merely checking them, because the validation entry
points read those fields directly. A restore that fingerprinted them and left the
restored value's own empty registries in place would pass every check and then
fail at evaluation with "no custom aggregate is registered" — a capability loss no
fingerprint can catch, because the fingerprints agreed.

## 7. The bounded guarantee: shapes and reusable analysis, never targets

**A product reproduces the shapes and the reusable analysis. It does not reproduce
targets, and there is no section for them.**

Target resolution is a property of a *data graph*. A product that cached resolved
targets would validate a new snapshot against the focus nodes of an old one: the
report would be well formed, every finding in it would be correct, and the rows
about everything added since the product was written would simply be absent. That
is the short-bag-reported-as-complete failure, and no amount of container
integrity detects it, because the bytes would be perfectly intact. Targets are
data-dependent by design and are resolved per bound dataset, every time.

The line that follows from this is the one to keep in mind when reading a
benchmark decomposition: **reusable preparation and per-dataset binding are
different lines in the budget.** What a product moves off the restore path is the
parse, the model construction and the shape analysis. What stays is everything
that depends on the data.

The class catalog sits exactly on that line and is handled accordingly. It is a
pure derivation of the shape tree — a cycle-safe walk collecting every reachable
`sh:class`, `sh:targetClass` and `shnex:instancesOf` IRI and assigning each a
position — so the product pins its **digest and carries no catalog body at all**.
A restore re-derives the catalog from the model it has already decoded and
compares. The walk is in memory, with no I/O and no SPARQL, and it is dominated by
the model decode that has to happen on the same path anyway. Carrying the body
instead would add bytes, add a second decoder, and — the actual defect — make a
stale analysis servable: a product written by a build whose class walk had a bug,
or simply a different reachability rule, would restore *that build's* catalog and
validate against it, verified and wrong. Pinning the digest makes a product whose
analysis no longer matches this build's walk unservable, which is the outcome
worth having. The position is folded into the digest as well as the IRI, because
the position is what a validation plan indexes its resolved row by, so two
catalogs over the same classes with different assignments are different analyses.

One capability is refused at pack time rather than lost at restore. The model
carries the SHACL 1.2 expression-bodied function declarations and nothing else
about the function registry, so a `sh:SPARQLFunction` declaration has no home in
it. A product carrying one would restore a shapes graph whose call sites resolve
to nothing and **validate green**. The writer therefore compares the declared
populations of the parsed and reassembled registries and refuses on
`unsupported-capability` — a total statement of the condition rather than a probe
for the one case known today, so a future declared-function kind that assembly
cannot reproduce fails without anyone remembering to add a branch.

## 8. Determinism, and the target-agreement arrangement

The writer is a pure function of the preparation and the profile. No
hash-iteration order, no wall clock and no randomness reach it — sections are
sorted by kind, every hash-ordered container in the model is written key-sorted,
and the identity is an ordered list — so two calls over equal inputs produce
byte-identical buffers, and insertion order is not an input.

That matters more than it usually does here, because the model types deliberately
do not implement structural equality: they embed a compiled regex and a shared
handle, so a round trip cannot be witnessed with `==`. The witness is the bytes.
**This codec supplies the canonical form, and hence the equality relation the
types themselves lack**: two values are equal exactly when they encode alike.

Determinism across *targets* is asserted the same way the geometry crate asserts
its own, and for the same reason — two runs on one target cannot tell a codec that
is target-independent from one that merely agrees with whichever target it was
last compiled for. One test body carries two attributes: natively an ordinary
`#[test]`, and on `wasm32-unknown-unknown` a `wasm_bindgen_test` run by the
`make wasm-test` lane. Both arms assert against a golden byte string committed
under `tests/fixtures/` and produced by a *native* build, so the native run is not
a weaker version of the wasm one — it is the other half of the comparison. Nothing
in that test opens a file; the shapes graph, the data graph and the golden product
are compile-time constants, because a wasm32 test needing a filesystem would be
proving something about the runner's shims rather than about the codec.

The codec itself touches no filesystem, no clock, no thread and no source of
randomness at any layer — envelope, identity, dataset section, model section — and
is `wasm32-unknown-unknown`-clean by construction. Every multi-byte field is
decoded from an explicit byte-slice copy rather than a pointer cast, so a caller's
buffer need not be aligned to any field's native alignment.

## 9. The standing tension: three authenticated containers

This is recorded because it is a real cost, not because it is resolved.

The workspace now holds **three** implementations of "fixed magic, fixed version,
section directory with per-section digests, sealed trailer, fail-closed reader":
the new generic envelope (`PURRAEND`), the dataset pack container (`PURRPCK1`),
and the embedding wire format (`PURREMB1`). The generic envelope exists precisely
because integrity discipline transcribed N times drifts N ways, and the copy that
drifts is always the one nobody re-reads — so the obvious follow-through is to
re-point the two pre-existing hand-rolls at it.

They were left alone, deliberately. Both of their byte layouts are pinned by
frozen golden vectors, and a re-point that preserved every byte would be a large
change with no observable effect, while a re-point that did not preserve every
byte would invalidate corpora this repository treats as frozen. Neither is worth
doing on the way past. The generic envelope took the pack container's layout laws
intact and removed its domain knowledge, so the three agree on the rules even
where they do not share the code — but "agree today" is a weaker property than
"cannot disagree", and that is the debt. A fourth prepared product should
instantiate the envelope; it should not be a fourth transcription.

## 10. The allocation profile, as measured facts

The benches in `crates/shapes/benches/shacl_product_reuse.rs` and
`shacl_product_alloc.rs` are **report-only**. Nothing in them is compared against a
baseline or gated on, and no threshold is asserted anywhere — a prepared product
is a structural change (a parse that happens once at build time instead of once
per process start) and needs no invented speedup to be worth making. The figures
below are recorded because the interesting risk in a "cache the parse" feature is
not that restoring is slow, it is that the *producer* has a peak nobody budgeted
for, and because an allocation count is a fact about the code rather than about
the host it ran on.

Restoring the dataset pack for the fixture's 3.8 KB section moved from **523
allocations to 21**, and from 104,527 requested bytes to 43,411. The bitmap-triple
and side-table readers were already at zero, so the whole cost sat in the
dictionary decode and the re-intern; the largest single find was a double parse
across a crate boundary, with every IRI in the dictionary parsed once to check
absoluteness during decode and parsed again at the builder's store-once boundary.
Splitting the IRI parser into an allocation-free scan plus owned materialization
removed a fifth of the total on its own and benefits every intern miss in the
workspace. The remaining floor is one allocation per retained structure rather
than per term, so it is flat in pack size. No emitted bytes changed: the frozen
product golden and every pack vector are untouched.

The whole `admit` path over the same fixture moved from **1046 allocations to
544**. What remains is dominated by the model decode, which sits at **333
allocations — its structural floor**: the declarative model is an owned-string term
model, so decoding it allocates once per owned string it materializes, and that
term is not removable without changing what the model *is*. The honest reading is
that the container, the dataset restore and the linking are no longer where the
allocations are; the term representation is.

Four quantities are reported separately and should not be conflated: allocation
*count*, requested bytes (allocator traffic, including memory freed again within
the phase), retained bytes (what the phase's result still holds), and peak live
bytes. A phase that allocates and frees a large buffer repeatedly has high traffic
and a modest peak; a phase that assembles one large structure has the reverse.
That distinction is exactly what separates the producer's encode — which re-packs
and canonicalizes the shapes dataset, and is the pipeline's real peak-memory event
— from the restore paths.

One byte-count fact deliberately does not live only in a bench log. The encoded
artifact's length for the determinism fixture is an **asserted constant** in
`crates/shapes/tests/product_determinism.rs`, so a codec change that doubled the
artifact fails the build rather than quietly changing a number nobody re-reads.

## What this document does not claim

* It does not claim a speedup. The benches assert no timing, name no competitor
  and compare no two identifiers; the product's correctness rests on the
  determinism, refusal and restore-equivalence tests, which never look at a clock.
* It does not claim `admit` establishes the shapes dataset's canonical identity.
  It establishes that the dataset section is the one the container sealed. Only
  `certify` makes the canonicalization statement, and it is a separate verb for
  exactly that reason.
* It does not claim a product is an interchange format. A product is an artifact
  of the PurRDF build that wrote it: a container-version mismatch is refused
  outright, and a stage-id mismatch is served by `rebuild` rather than by
  best-effort decoding of a memo whose model this build no longer has.
* It does not claim target coverage survives a round trip, because targets are
  deliberately not carried. See §7.
