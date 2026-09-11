<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# PurRDF shared view carriers: the view-backed pipeline bundle and GTS ingestion

A PurRDF pipeline moves a *carrier* — one RDF surface plus the out-of-band
material that travels with it — from one stage to the next. There are two
carriers, and they are interchangeable by content:

- `PipelineBundle` owns one frozen `RdfDataset`. Folding a named graph into it
  unions a new dataset: a deep copy, paid once per accumulation.
- `PipelineViewBundle` owns an `Arc<CompositeDatasetView>`. Folding a named
  graph into it appends a retained source through `CompositeDatasetView::extend`.
  No source row is copied and no whole-surface materialization occurs; what an
  IRI-named placement does charge is **one graph-dictionary freeze per append**,
  over the appended contribution's own dictionary rather than over the
  accumulated surface. The bases it retains are charged once to a shared
  `RetentionLedger` however many carriers hold them.

Both delegate their entire non-dataset state — lookaside, blobs, provenance,
typed handles, the digest fold, the per-graph memo and the pin check — to one
internal implementation, so the two carriers cannot drift into disagreeing about
identity. On the output side the same consolidation happened again: the frozen
`SnapshotBuilder::add_dataset[_scoped]` surface and the new
`SnapshotBuilder::add_view[_scoped]` surface both run through a single ingestion
body, so a refusal, a checkpoint and the poison flag mean the same thing on both.

This document records the decisions behind that surface which a reader would
otherwise take for oversights: the graphs the wire format does not carry, the
refusal that looks like over-strictness, the second identity that is not a
replacement for the first, and the one place where copying still happens.

All example IRIs use `example.org`. PurRDF mints no vocabulary IRIs; every
vocabulary term is caller-supplied configuration.

---

## 1. Adoption path

The flat surfaces keep working, unchanged, and are not deprecated.

`PipelineBundle` constructs, accumulates, digests and pins exactly as before.
`SnapshotBuilder::add_dataset` and `SnapshotBuilder::add_dataset_scoped` keep
their signatures (`Result<(), String>`, the shape the Python producer's binding
is frozen against) and their semantics, including the `default_graph_name` and
`scope` partitioning hooks. What changed underneath them is that they now
delegate: a frozen `RdfDataset` is itself a `DatasetView` and a
`FallibleDatasetView`, so the flat entry point runs the same ingestion body the
view entry point runs, rather than being a second definition of "ingest a
dataset". A flat caller that recompiles and changes nothing emits the same
`purrdf.gts` bytes: the frozen GTS vectors are the regression proof for that, and
a dedicated test asserts that the view path and the flat path emit **byte-equal**
containers for the same content — byte equality on the emitted container, not
isomorphism, so anything that renamed a blank scope would show up.

A caller that wants the view path opts in per call site:

1. Construct a view carrier with one of the three entry points.
   `PipelineViewBundle::from_dataset` composes a single frozen base as one
   graph-preserving source and records it as the sole owner of every graph it
   addresses. `PipelineViewBundle::from_view` adopts an already-composed
   `CompositeDatasetView` (owned or shared) and registers each of its native and
   delta sources with the ledger. `PipelineViewBundle::from_delta` rides a
   `DeltaDatasetView` as a composed source without compacting its base,
   registering both the base and the delta overlay.
2. Fold contributions in with `PipelineViewBundle::accumulate_named_graph`, whose
   contract matches `PipelineBundle::accumulate_named_graph` — same containment
   rule, same pin preservation, same resulting digests — without the deep copy.
3. Serialize with `SnapshotBuilder::add_view` or
   `SnapshotBuilder::add_view_scoped`, which take any `FallibleDatasetView`
   directly: no temporary `RdfDataset`, no text round trip, no per-row owned term
   reconstruction.

The two carriers convert in both directions at explicit, accounted boundaries:
`PipelineBundle::to_view_bundle` / `into_view_bundle` and
`PipelineViewBundle::to_flat_bundle` / `into_flat_bundle`. Every pinned handle is
re-verified across the conversion rather than assumed to survive it.

Adoption is therefore incremental and reversible. There is no flag day, no
workspace-wide switch, and no call site that must move because another one did.

### One source-compatibility note: the comparison helpers

`datasets_isomorphic` and `dataset_diff` became generic over **two** view types
(`fn datasets_isomorphic<A: DatasetView, B: DatasetView>(a: &A, b: &B)`, and the
same shape for `dataset_diff`), because comparing a composed view against a flat
dataset is exactly the question a caller mid-adoption asks, and the old
single-type form could not express it. An ordinary call — two values, inference
picks the types — compiles unchanged. What needs an edit is code that named the
old single-type form explicitly: coercing either function to a concrete `fn`
pointer, or turbofishing it with one type argument. Both now need the two types
spelled out (`datasets_isomorphic::<RdfDataset, RdfDataset>`, or a `fn(&A, &B) ->
bool` pointer type with both arguments given). Nothing about the answer changed;
only the arity of the type parameter list did.

## 2. Declaration-only graphs at GTS output

A `DatasetView` may address a named graph that holds no row — a *declaration-only*
graph. `RdfDataset` preserves such declarations, `CompositeDatasetView` preserves
them through `GraphPlacement::Preserve`, and `PipelineViewBundle::named_graph_iris`
reports them. The carrier is not where they are lost.

The GTS `dist` snapshot payload is frozen, and it has four tables: terms, quads,
reifiers and annotations. It has no declarations slot. A graph name with no row
therefore has nowhere in the wire bytes to be written to.

The ruling is that ingestion **omits** such a graph and **states** the omission.

- The graph's IRI is not interned. Interning it would add a term row, which would
  shift every later term id and therefore shift `snapshot_content_id`. Content
  ids stay stable precisely because the omission is total rather than partial.
- The omission is observable, not silent. `add_view` and `add_view_scoped` return
  an `IngestReport`, and `IngestReport` carries
  `#[must_use = "the ingest report names the declaration-only graphs that were
  deliberately not interned"]`. Its `declarations_omitted` field names each
  omitted graph IRI, sorted and deduplicated. A caller cannot obtain a snapshot
  without receiving the receipt.
- `SnapshotBuilder::ingest_totals` accumulates the receipt across every `add_*`
  call on one builder, so the flat surfaces — whose frozen signatures cannot
  return a report — remain interrogable.
- The Python surface delivers the receipt in the same pass that mints the bytes:
  `compile_gts_with_report` returns the snapshot bytes and the totals dict —
  `declarations_omitted` included — from one ingestion. The frozen bare-bytes
  producers keep their signatures, and `gts_ingest_report` remains as the
  sources-addressed accessor for a caller that holds no bytes yet. The omission
  is therefore stated on that host too, rather than being a fact only a Rust
  caller can reach.

Two alternatives were rejected.

*Refusing a view that carries declaration-only graphs* would over-refuse. A
frozen `RdfDataset` with an empty declared graph is valid input that flat callers
have always been able to serialize; refusing it would break working callers in
the name of strictness while fixing nothing, since the wire bytes would be the
same either way.

*Adding a declarations slot to the payload* would change the frozen format. Every
existing `purrdf.gts` bundle and every third-party reader is written against the
current frame; a new slot is a format break, and a format break is not a thing a
carrier-side improvement gets to spend.

Callers that need an empty graph to survive the wire must give it a row. That is
a modelling decision the caller makes with full information, because the report
told them which graphs are at stake.

## 3. Terminal poison contract

A `SnapshotBuilder` interns terms and pushes rows incrementally. An ingestion
that fails partway through therefore leaves a half-ingested interior: some terms
minted, some rows pushed, and no way to distinguish that state from a complete
one by inspection.

Any ingestion error **poisons** the builder, terminally.

- The first `GtsIngestError` is stored on the builder. It is readable through
  `SnapshotBuilder::poison`.
- Every later `add_dataset`, `add_dataset_scoped`, `add_view` and `add_view_scoped`
  refuses with `GtsIngestError::Poisoned { cause }`, where `cause` renders the
  **earlier** failure rather than anything about the current call. A poisoned
  builder has nothing honest to say about the call being made; the only honest
  thing it can say is which ingestion left it this way.
- `emit_gts` refuses the same way, before it writes a byte.

The property this buys is stated as a negative, which is how it should be read:
**a partially accepted view can never publish.** There is no path by which a
builder that swallowed half of a source emits a bundle that is a prefix of the
requested data and calls it a snapshot. Recovery is to construct a fresh builder,
which is cheap and unambiguous, rather than to reason about which rows survived.

## 4. Digest identity and exact invalidation

`digest()` is unchanged. It is the same four-part SHA-256 fold it always was,
over, in a fixed order:

1. the canonical N-Quads hash of the RDF surface,
2. each lookaside resource's content digest, collected and sorted,
3. each blob's content digest in the store, sorted,
4. the provenance sidecar's runtime-id-free public projection.

The view carrier folds exactly those four sections, reading section 1 through
`try_canonicalize_view` instead of `canonicalize`. A composite view and the flat
dataset holding the same content canonicalize byte-identically, so the two folds
agree by construction rather than by coincidence: **equal content yields an equal
digest across the flat and view carriers.** The typed-handle lane contributes
nothing to the fold, so attaching or detaching a handle leaves the digest
byte-stable.

`graph_digest(g)` is IRI-addressed and per-graph: the canonical digest of exactly
the rows whose own graph slot is `<g>`. It is the same value, byte for byte, on
either carrier for equal content. Blank-node graph names have no per-graph leaf,
because a blank name is not stable across carriers — composition standardizes
blank scopes apart — and an unstable address cannot key a memo.

### The containment rule is what makes invalidation exact

`accumulate_named_graph` folds in **one** named graph, and it checks first, hard,
that every row of the contribution names that graph and nothing else. The check
covers all four layers, which is what `GraphLayer` enumerates:

| Layer | What it covers |
| --- | --- |
| `GraphLayer::Quad` | An ordinary quad |
| `GraphLayer::Reifier` | An RDF 1.2 reifier declaration row |
| `GraphLayer::Annotation` | An RDF 1.2 statement annotation row |
| `GraphLayer::Declaration` | A named-graph declaration, whether or not it holds rows |

A violation is `PipelineBundleError::GraphContainment`, carrying the intended
graph, the offending layer, and the graph the offending row actually names.
Checking the declaration layer as well as the three row tables is not
bookkeeping: a contribution that *declares* a second graph would widen the set of
graphs the carrier addresses, and therefore change the pipeline root, without
moving a single row.

On the view carrier the rule is doubly load-bearing. The contribution is placed
with `GraphPlacement::Named`, which **rewrites** every graph slot it is handed —
so an unchecked contribution would be silently relocated instead of refused, and
the two carriers would stop agreeing about identity.

Because containment is enforced, the append provably touched exactly one graph.
That is what licenses the exact invalidation:

- the bundle digest,
- the touched graph's own `graph_digest`,
- the pipeline root

are the only values invalidated. Every untouched graph's memoized digest stays
valid and is never recomputed. `BundleDigestWork` — non-semantic counters,
shared with a carrier's clones, never an identity input — is how a test proves
that: folding one graph in must not raise `graph_canonicalizations` for the
graphs it left alone.

### Reseating, not clearing

Accumulation **reseats** the digest-cache `Arc`: it installs a fresh map on the
carrier being accumulated into, seeded with every entry except the touched
graph's. It does not clear the shared map in place.

The distinction is a correctness one, not a tidiness one. Clearing a shared map
would let a clone taken *before* the accumulation read digests computed against
the *other* clone's content. That is a wrong answer, not a stale one. After a
reseat the two carriers own independent memos, each consistent with its own
content, and **a pre-accumulate clone keeps answering for its own content.**

### `stats_fingerprint` is not identity

`DatasetView::stats_fingerprint` is a cheap, deterministic size fingerprint for a
dataset-aware cache key, such as a join-order cache. It is a **cache
discriminator**. It is not a content digest, it is not a freshness signal, and it
is never folded into any identity. Its default is `0`, which is a legal answer
meaning "no discrimination" — which is exactly why nothing may infer sameness
from it.

## 5. Pipeline root

`pipeline_root()` is a second, independently versioned identity, domain-separated
by the bare token `PIPELINE_ROOT_DOMAIN = "purrdf.pipeline-root.v1"`, hashed as
the first bytes of every root. The token is deliberately not an IRI: nothing may
dereference it, assert with it, or mistake it for a vocabulary this project does
not publish. Its version suffix moves whenever the bytes a given carrier folds
would move — the leaf set, the leaf order, the residue rule, or the sidecar
sections — and does not move for a refactor that cannot change output.

The root is a SHA-256 fold over, in order:

1. the domain tag;
2. the carrier's IRI-named graphs — **every** graph it addresses, quad-bearing or
   declared empty — each as `(graph IRI bytes, per-graph canonical digest)`,
   sorted by IRI;
3. the **residue** leaf: the canonical digest of every row whose graph slot is
   the default graph or a blank-node graph name, with that slot preserved;
4. the same lookaside, blob and public-provenance sections `digest()` folds, byte
   for byte.

The residue exists because a per-graph leaf is addressed by IRI, so rows in the
default graph or under a blank graph name own no leaf. Folding them as one
residue is what keeps the root a function of the whole carrier rather than of its
IRI-addressable part. Preserving the slot is what makes the residue faithful: a
blank graph name is itself canonicalized, so two carriers holding the same rows
under differently-numbered blank scopes produce identical residue bytes, while
rows moved *between* the default graph and a blank-named one do not.

The root exists so a scheduler can see **which leaf moved** when a carrier
changes, which a single whole-dataset digest cannot express.

It is strictly more discriminating than `digest()`. A named graph that is
declared but holds no row contributes its IRI and the canonical digest of the
empty document; `digest()`'s canonical document has no place to record an empty
declaration at all. Two carriers can therefore share a `digest()` and differ in
their pipeline root.

It is **never a substitute** for `digest()`. The root addresses graphs by IRI, and
a declaration-only graph whose name is a blank node is invisible to both
identities for the same reason: it owns no row, and its name is not addressable.
A consumer pins the two together or pins neither.

### The stage-cache key contract

The pair `(input pipeline root, stage identity)` identifies a stage execution for
caching purposes. A scheduler that holds a cached product for that pair may reuse
it; a scheduler that does not must execute. The input root is what makes the key
sound, because it is a function of the carrier's per-graph leaves and its
sidecars and of nothing else — not of wall-clock time, not of allocation
addresses, not of the work counters, and not of `stats_fingerprint`.

## 6. Retained vs incremental and ledger scoping

Two accounting scopes exist and they answer different questions. Confusing them
produces a double count, which is why the types keep them in separate fields and
never sum them for you.

**Per view.** `ViewStats` is one view's own snapshot: what *this* view would
retain and construct if it were the only view in the process, checked against
*this* view's `ViewLimits`. It is deliberately unconditional — a base counted
twice by one composite is charged twice — because admission is a statement about
a single view's ceilings, not about process residency. **Admission is unchanged
and stays per view.** `PipelineViewBundle` re-checks its ceilings cumulatively as
each source is appended, so a carrier that has grown past its limits refuses the
append (`PipelineBundleError::AdmissionBreach`) rather than publishing a view it
was never admitted to hold.

**Across carriers.** `RetentionLedger` is the shared scope. It deduplicates
retained owners by **`Arc` pointer identity**: several carriers holding the same
`Arc` base retain one copy of it between them, so the ledger reports that base's
bytes once, under `RetainedCharge::of_dataset`. A second registration of an owner
already present adds a guard but not a second charge.

`RetentionGuard` is the RAII handle. It holds a strong, type-erased reference to
the owner it keys, so the allocation cannot be freed — and therefore cannot be
recycled under a second, unrelated owner — while any guard names it. **Release is
on last reader:** when the final guard for a key drops, the ledger erases that
key and every memo entry filed under it *before* releasing its strong handle, so
a later allocation that happens to land on the same address starts from an empty
slate. `OwnerKey::addr` exposes the raw address for diagnostics and stable test
ordering only; it is not an identifier a caller may persist or resolve back.

`OwnerMutability` gates memoization: only a `Frozen` owner — an `Arc<RdfDataset>`
is frozen by construction — may have a per-graph digest memoized under its key. A
`Mutable` owner is still accounted, but the memo refuses to store for it and
recomputes every call, which shows up as a miss that never becomes a hit.

`ViewAccountingReport` pairs the two scopes **without double counting**. It keeps
`retained` (deduplicated, ledger-wide) separate from
`incremental_auxiliary_bytes` and `incremental_work` (this view's own
construction charge: overlays, alias maps, suppression sets, rebound literal text
and caller-owned graph placement — genuinely private to that view, and therefore
additive across views). `ViewStats::retained_payload_bytes` and its siblings are
deliberately not carried over: they are that view's private restatement of base
bytes the ledger already reports once, and adding them is precisely the double
count the type exists to prevent. `total_accounted_bytes` sums retained payload,
memo bytes and incremental bytes with each byte counted exactly once.

Scoping follows ledger identity, not carrier identity: **carriers that share no
ledger account independently.** A ledger is a scope a caller chooses by passing
the same `Arc<RetentionLedger>`; two unrelated pipelines each see their own
totals, and neither can be made to under-report because the other happens to hold
the same base.

## 7. Fallible-view checkpoint contract

GTS ingestion is bounded on `FallibleDatasetView`, not on `DatasetView`.

That bound exists because an operationally fallible view preserves the infallible
iterator shape the evaluator requires: the first operational failure becomes
sticky and every iterator simply **stops yielding**. A view that faulted
mid-iteration does not error out of the loop; it ends the loop early. Without a
checkpoint, the builder would mint a `snapshot_content_id` over a truncation and
report success.

So the ingestion samples `operation_status()` twice:

- `IngestCheckpoint::BeforeRows`, before the first row is consumed;
- `IngestCheckpoint::AfterRows`, after every row — ordinary, reifier and
  annotation alike — has been consumed, and before any figure derived from them
  is published.

Publication is refused unless the status is `ViewOperationStatus::Ready` at
**both** checkpoints. A failure at either raises
`GtsIngestError::ViewNotReady { checkpoint, cause }`, naming which boundary
observed it and carrying the view's own sticky root cause: the rows read are a
truncation, not the view.

In-tree views are infallible and are always `Ready`. `RdfDataset` implements
`FallibleDatasetView` with `type Error = Infallible` and `type Evidence = ()`.
`Infallible` is the honest root-cause type — the `Failed` variant is uninhabited
here, not merely unused — and the unit is the honest evidence, because an
in-memory read consumes no request budget a boundary could meter. The impl exists
so that one ingestion path, bounded for the operational case, also accepts the
production view, instead of forcing callers to pick an entry point by backend.

### A claimed capability is not a row count

Ingestion reads the statement layer through the view's own accessors
(`reifier_quads()`, `annotation_quads()`) and ingests exactly what they
enumerate. A capability claim is **not** checked against enumeration at entry,
because emptiness is not evidence of anything: a view whose `capabilities()`
claims `reifiers` while `reifier_quads()` enumerates nothing is a legitimate
state — a delta can suppress the base's only reifier, and the honest result is
a claimed-but-empty layer. Refusing that shape rejects valid input.

RDF 1.2 reification and annotation are complete parts of the specification and
the snapshot frame carries both tables; what guards them is the trait's
snapshot-ingestion obligations (an accessor must enumerate every row the view
holds — a backend that under-enumerates violates its documented contract), not
a runtime heuristic that cannot distinguish "cannot enumerate" from "has
none".

## 8. Blank-node wire collision refusal

A scoped ingestion prefixes blank-node labels so that two equal labels in
different ingest scopes stay distinct terms. The wire encoding is
`"{scope}-{label}"`, with an unscoped label kept raw.

That encoding is **not injective** over `(scope, label)`. Taking a fixture pair:

```text
(Some("a"),   "b-c")  ->  "a-b-c"
(Some("a-b"), "c")    ->  "a-b-c"
```

Two distinct intern keys encode onto one wire value. Minting the second row would
leave two indistinguishable blank term rows whose relative order the stable
canonical sort takes from the **ingestion** order, so the emitted bytes would
stop being a pure function of the content — the exact property the whole GTS
pipeline exists to provide.

Changing the encoding would move every existing scoped caller's bytes, so the
encoding is frozen. The ambiguous case is refused instead, with a typed error:

```text
GtsIngestError::BlankWireCollision {
    wire_value, held_scope, held_label, incoming_scope, incoming_label,
}
```

The error names the wire value both keys encode onto, the key that already holds
it, and the key that collided with it, so a caller can see which of its scope
names to change.

The refusal is narrow by construction, and this is the part that matters. The
builder maintains a hash-consed index of blank term rows keyed by their **wire
value**, separately from the intern index keyed by `(scope, label)`. The
collision is detected at the moment a *new* intern key would mint a second row on
an *existing* wire value — not by rejecting scope names that merely contain a
hyphen, and not by rejecting a repeat of the same key, which is an ordinary
intern hit. Every non-colliding input, scoped or unscoped, hyphenated or not,
ingests **byte-identically to before**.

## 9. Composite vs materialize crossover

Two questions are separable and have different answers, so they are measured
separately.

**How to read this section.** The *mechanism counters* are the normative content.
They are what the code charges, they are what the tests assert, and they are a
function of the shape alone rather than of whatever ran it. Every wall-clock
figure below is one **observed sample** — one run, one machine, non-normative —
kept only because it is the one thing that answers "does the asymmetry actually
show up", and dated so a reader can weigh how old it is. A sample may not
reproduce under other hardware, other load or another allocator. Where a timing
and a counter appear to disagree, **the counter governs**.

### Carrier traversal: no observed crossover

Accumulating contributions onto a carrier and reading digests off it favours the
view carrier at every shape measured, and the gap widens with accumulation count
rather than closing. The mechanism is visible in the work counters, which is why
the benches report them alongside wall time.

Per accumulation, the two carriers charge:

| Carrier | row copies | dictionary freezes |
| --- | --- | --- |
| union (flat) | every row of both operands, replayed into a new dataset | one **whole-surface** refreeze |
| extend (view) | none | one **graph dictionary**, for the IRI-named placement |

That table is read down the column, not across a single accumulation. The flat
carrier replays the accumulated surface every time, so its row copies are
quadratic in the number of accumulations and each of its freezes covers
everything accumulated so far. The view carrier copies no source row at all and
charges one dictionary freeze per named append: `n` appends charge `n` freezes,
each over the contribution being appended rather than over the surface it is
appended to.

`extend`'s own aliasing cost deserves a separate statement, because the tempting
summary — that an append costs only what the new contribution costs — is not
true. Appending source `k+1` aliases that source's terms against the identity
space the `k` already-retained sources agreed on, so **one append is
`O(prefix × new-source terms)`**. What is saved relative to composing from
scratch is that the earlier source *pairs* are never re-aliased: composing `k`
sources afresh is `O(k² · terms)`, while extending an existing composite pays
only the new source against the prefix. `n` appends therefore remain quadratic in
`n` — with a much smaller constant than the flat carrier's row replay, which is
why the gap widens, but quadratic all the same.

Observed sample, 16 accumulations — one run, one machine, 2026-09,
non-normative:

| Carrier | wall |
| --- | --- |
| union (flat) | 56.5 ms |
| extend (view) | 1.29 ms |

There is no crossover to look for here. A caller accumulating onto a carrier
should use the view carrier.

### Through to emitted GTS bytes: measure your own shape past a few tens of sources

End to end — carrier construction, accumulation, ingestion and emitted
`purrdf.gts` bytes — view ingestion matches or beats materialize-then-flat at
moderate source counts. Observed sample — one run, one machine, 2026-09,
non-normative:

| Shape | view | materialize-then-flat |
| --- | --- | --- |
| Large shared base, few contributions folded one at a time | 3.13 ms | 3.47 ms |
| `k=8` composed sources over a small base | 0.87 ms | 1.00 ms |

At `k=32` composed sources the picture stops being clean: wall-clock measurement
puts the view path at 3.75 ms, while the criterion medians over the same shape
favour the flat path. **The two measurement methods disagree**, and a disagreement
is reported as a disagreement rather than resolved by picking the flattering
number. The mechanism does not break the tie either: by `k=32` the
`O(prefix × new-source terms)` aliasing has accumulated into a quadratic term of
its own, which is precisely the region where a materialize-then-flat path can
legitimately win.

The guidance follows from that honestly. A caller composing many tens of sources
before emitting should measure its own shape, and may legitimately choose the
explicit `materialize()` followed by flat ingestion. That path is fully
accounted — the freeze and the materialization are charged on the view's own
`ViewWork`, and the bytes are ledger-visible — so choosing it costs nothing in
observability.

What the library does **not** do is choose for the caller. There is no size
heuristic, no source-count threshold, and no hidden switch that materializes
behind the caller's back at some shape the caller cannot predict. A carrier that
sometimes copies and sometimes does not, on a rule the caller cannot see, is
worse than either behaviour consistently applied: it makes the accounting
unpredictable and the benchmark unreproducible. **The choice is the caller's, and
it is made by calling `materialize()`.**

## 10. Remaining materialization boundary

There is **no** remaining materialization of the composed surface on the
carrier → pin-validation → GTS path.

A view carrier can be constructed, accumulated into, digested per graph and as a
whole, have its pipeline root computed, have its typed handles pinned and
re-verified, be ingested through `add_view[_scoped]`, and emit `purrdf.gts` bytes,
without a single row being copied into a new dataset.

The integration proof asserts this directly rather than arguing it. It samples the
view's own `ViewWork` immediately before ingestion and again after emission, and
requires that `freezes`, `materializations` and `copied_rows` are **unchanged**
across the whole path. Publishing bytes costs zero copies. The proof is also
explicitly non-vacuous: it first asserts that the composite's `freezes` counter is
already non-zero at the sampling point. One dictionary freeze is charged per named
append — that is where `GraphPlacement::Named` derives the rewritten graph's
dictionary — so a carrier accumulated once, as this one is, reads `1`, and the
zero-movement claim is made against a counter demonstrated to be live rather than
against one that never increments. `materializations` is `0` in absolute terms throughout,
and a carrier over a preserved delta charges neither.

The one whole-surface materialization that remains is the caller-explicit
`PipelineViewBundle::materialize()`, plus `to_flat_bundle` / `into_flat_bundle`,
which are built on it. It is not a second freeze path: it delegates to
`CompositeDatasetView::materialize`, which drives the single pack reconstruction
loop, so the carrier cannot acquire a differently-accounted copy. It charges
`ViewWork::freezes` and `ViewWork::materializations` on the view's own counters,
and the resulting dataset's bytes are visible through the ledger like any other
retained base.

That is the whole boundary: one named method, charged where a reader can see it,
called only when a caller asks for it.

---

## Reading guide

| Concern | Where it is enforced |
| --- | --- |
| Carrier identity, containment, exact invalidation | `purrdf-core`, `ir::pipeline_bundle` |
| Composition, placement, blank scope binding, `extend` | `purrdf-core`, `ir::composite` |
| Retention scope, admission ceilings, accounting | `purrdf-core`, `ir::view_accounting` |
| Canonicalization entry points over views | `purrdf-core`, `ir::canon` |
| Checkpoints, ingestion, rollback, poison | `purrdf-rdf`, `gts_compose` |
| The frozen wire frame itself | `purrdf-gts`, and `docs/GTS-SPEC.md` |

The carrier types, the ledger family and the canonicalization entry points are
re-exported from the `purrdf_core` and `purrdf_rdf` crate roots; the ingestion
receipt and its refusals live in `purrdf_rdf::gts_compose`.

The measurement shapes quoted above are the ones the criterion benches build:
`crates/rdf-core/benches/shared_views.rs` for carrier traversal and
`crates/rdf/benches/gts_ingest.rs` for the path through to emitted bytes. Both
report the work counters beside the wall time, because a wall time without the
copy count does not say *why* a shape is fast — and because it is the counters,
not the sampled timings, that this document states normatively.
