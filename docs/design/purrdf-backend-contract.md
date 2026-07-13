<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# PurRDF Backend Contract

This document is the authoritative statement of the invariants that any PurRDF
dataset backend — and the pluggable read seam layered over it — must honor. It
consolidates the normative contract that previously lived only as scattered
doc-comments in `purrdf-core`, and it adds the **G-clauses** governing the paged
storage layer.

The contract has two families of clauses:

- **C-clauses** describe the term-identity model and the read/write traits over a
  single dataset (`DatasetView`, `DatasetMut`, `RdfStoreCapabilities`).
- **G-clauses** describe the paged storage layer: a global, durable dictionary
  generation and the deterministic, value-boundary rules by which quad-disjoint
  pages compose into one logical dataset.

All example IRIs use `example.org`. PurRDF mints no vocabulary IRIs; every
vocabulary term is caller-supplied configuration.

---

## C-clauses — term identity and the single-dataset seam

### C0 — Term identity is defined by the IR

Literal identity is defined by the IR, not by any backend (C0.1): the datatype is
always expanded (`xsd:string` for a plain literal, `rdf:langString` for a
language-tagged literal), the language tag is lowercased for the interning key,
base direction participates in identity, and the lexical spelling is preserved
verbatim. Blank-node scope participates in the interning key (C0.2): two blank
nodes from different scopes are distinct even with the same label, and two blank
nodes in the same scope with the same label are the same node. Triple terms are
identified structurally by their resolved `(s, p, o)` (C0.3).

#### C0.8 — `TermId` is dataset-local (the ring-fence the whole design honors)

`TermId` is an opaque term identity that is **local to exactly one frozen
`RdfDataset`**. It is deliberately:

- **not** `Serialize` / `Deserialize`,
- **not** merge-stable,
- **not** meaningful across datasets.

Any consumer that needs a durable identifier MUST resolve the `TermId` to its RDF
term value and retain the value — never the id. This is the ring-fence that the
entire backend design is built to honor: an id names a position in one dataset's
term table and nothing more. Even the low-level `TermId::index` /
`TermId::from_index` kernel API, which exposes the dense table index so the
sibling `purrdf` adapters (the canonical Turtle serializer in particular) can
address terms by position, is only meaningful against the single dataset whose
table holds that index.

#### The `Option<TermId>` niche (load-bearing P3a invariant)

`TermId` wraps a `NonZeroU32` holding `dense_index + 1`, so the all-zero bit
pattern is free for the `Option` niche. This makes `Option<TermId>` **4 bytes**,
not 8, which shrinks the quad row from 20 to 16 bytes — roughly 20% off the quad
table — because the absent-graph slot (`g: Option<TermId>`) needs no extra
discriminant word. Two compile-time assertions enforce it and fail the build if
the niche ever regresses:

```text
size_of::<TermId>()         == 4
size_of::<Option<TermId>>() == 4
```

The `+1` storage offset is confined to `index` / `from_index`; it never leaks
into `Hash` (which hashes the 0-based dense index, byte-identical to a plain
`TermId(u32)` derive) and never into `Ord`. The niche is therefore a pure memory
optimization with no observable effect on iteration order or sort order — a perf
change must not silently reorder any hash-iteration-dependent output.

### C1 — Frozen, validated dataset; single compile-time backend via static generics

The production read view is the immutable, value-interned `RdfDataset`: a frozen,
validated dataset over which every read method is infallible. Backend selection is
**compile-time and static**. This is realized as **static generics**
(`impl DatasetView`, monomorphized), NOT dynamic dispatch: the `DatasetView`
methods return position-independent RPITIT iterators and are therefore
non-object-safe **by design**. Because a backend is resolved at compile time at
each query site, the trait carries no object-safety obligation and there is no
erased `&mut dyn` layer.

`DatasetView` is **open**: any crate may implement it, so an external
demand-paged or store-backed dataset can serve the query evaluator directly rather
than being materialized into a scratch `RdfDataset` first. Openness does not weaken
C0.8 — each implementer carries its **own** associated id type, and an id is only
ever meaningful within the view that minted it. `RdfDataset`'s id type is the
dataset-local `TermId`; the paged layer's is `GlobalTermId` (G0). The evaluator is
generic over the view's associated id, so the `RdfDataset` case monomorphizes to
exactly the concrete `TermId` code path.

### C2 / C3 / C6 — the `DatasetView` read contract

`DatasetView` is the static, allocation-free, id-based read view over an RDF
dataset. It yields `Copy` quad-id rows and borrowed, resolved quad references with
no per-quad allocation and no term-string clones. Its surface:

- `quads` — iterate every quad as `Copy` quad-id rows in dataset-local `TermId`s.
- `quad_refs` — iterate every quad as a borrowed, resolved quad reference (no
  allocation).
- `resolve(id)` — resolve a dataset-local `TermId` to its borrowed term reference.
- `quads_for_pattern(s, p, o, g)` — quads matching an optional `(s, p, o)` id
  pattern plus a three-way `GraphMatch` (any graph / the default graph / one named
  graph). The default implementation is an id-equality **linear scan** with no
  string resolution; a backend carrying access-pattern indexes overrides it with
  an indexed lookup that is byte-identical to the scan. Callers resolve term
  *values* to ids first via `term_id_by_value`.
- `term_id_by_value(value)` — resolve a term **value** to its dataset-local
  `TermId` **without minting**. A value interned nowhere in the view yields `None`:
  it names no term, so a structural walk keyed on a not-present IRI simply finds
  nothing. Absence is an empty match, never an error.
- `capabilities()` — the capability probe for this view's backing data (see C7).
- `len_hint()` — an optional quad-count hint.

The graph slot is stored as `g: Option<TermId>` where `None` is the default
graph. Because `Option<TermId>` alone cannot distinguish *any graph* from *the
default graph*, the read view uses the dedicated three-way `GraphMatch` enum, which
is deliberately exhaustive (a quad's graph is either the default or exactly one
named graph). All `DatasetView` methods are infallible for a frozen, validated
dataset.

### C4 — `DatasetMut` mutates by value, not by id

`DatasetMut` is the write companion to `DatasetView` — the mutation surface a
copy-on-write or store-backed dataset exposes. Where `DatasetView` reads in
dataset-local `TermId`s, `DatasetMut` mutates by **value**: its `Quad` associated
type is an owned, dataset-independent quad, each component a term value.

The reason is C0.8. A mutable dataset that straddles a frozen base and an
in-memory delta has **no single id space** in which a caller could name a
brand-new term, so a term value is the only well-defined mutation identity. The
implementer resolves each value to its internal handle — a base hit, or a freshly
minted delta id. This is the precedent the paged layer's G-clauses follow: the
mutable dataset never widens `TermId`; it works in an internal tagged `MutTermId`
enum of `Base(TermId)` (an id into the frozen base) and `Delta(...)` (an index
into the delta's own small interner). A quad's term is bound to `Base` when the
base's `term_id_by_value` hits and to `Delta` when it misses. These tagged handles
are strictly internal; the outside world only ever sees frozen base `TermId`s
(pre-mutation) or post-freeze dense `TermId`s. A plain two-tier **numeric**
`TermId` — a single integer whose high range meant "delta" — would violate C0.8 by
implying one id space spanning two datasets, which is exactly why the segregation
is a tagged enum and never a numeric threshold.

`DatasetMut` operates on the **effective** set:

- `insert(quad)` / `remove(&quad)` return whether the effective set actually
  changed (a no-op returns `false`).
- `contains(&quad)` reflects the effective set after any sequence of mutations.
- `quads_for_pattern(s, p, o, g)` returns owned value-quads (the mutable view has
  no stable id space to borrow into across the base/delta boundary). Its graph
  filter is **value-based** (`GraphMatchValue`, not the read side's `TermId`-based
  `GraphMatch`), so a delta-only named graph — one introduced after branching, with
  no base `TermId` — is still expressible, consistent with the value-based
  `s`/`p`/`o` slots. The implementer resolves a filter value to its internal handle
  without minting; a value interned in neither base nor delta matches nothing.

### C7 — capabilities

`RdfStoreCapabilities` is the capability probe a view exposes for its backing
data. Each field is an independent yes/no feature probe, not an encoded state
machine:

- `named_graphs` — quads outside the default graph are representable.
- `quoted_triples` — RDF 1.2 triple terms (quoted triples) are representable.
- `reifiers` — RDF 1.2 reifier bindings are representable.
- `annotations` — RDF 1.2 statement annotations are representable.
- `source_locations` — source/location context is preserved.
- `loss_records` — conversion-loss records are preserved.
- `lookaside` — structured non-triple lookaside material is preserved.

The plain-RDF baseline has every flag off.

---

## G-clauses — the paged storage layer

The paged layer composes many independently frozen **pages** into one logical
dataset. Each page is itself a frozen dataset with its own dataset-local `TermId`
space (C0.8 still holds inside a page). The G-clauses define the global identity
generation and the deterministic value-boundary rules by which pages compose.

### G0 — `GlobalTermId` is global and durable within one dictionary generation

`GlobalTermId` is a **global, durable** term identity within one dictionary
generation. It is distinct from `TermId`: where a `TermId` names a position in one
frozen dataset's local term table (C0.8), a `GlobalTermId` names a term in the
paged layer's shared dictionary and is stable across every page that participates
in the same generation. A `GlobalTermId` is only meaningful within the generation
that minted it; a compaction (G2) begins a new generation.

### G1 — no numeric cross-space translation; the map is built by value

There is **no numeric translation** between a page's local `TermId` space and the
`GlobalTermId` space. A page's local `TermId` maps to a `GlobalTermId` only through
a `PageTranslation` built by re-interning each of the page's term **values** into
the shared dictionary — the same by-value boundary that union, `push_dataset`, and
ingest remaps already use when they re-intern a foreign dataset's terms into a
fresh interner. `GlobalTermId` **never widens `TermId`**: it is a separate space,
not a superset id, so nothing is ever reinterpreted by bit-pattern or by numeric
range across the two spaces. The value is the only bridge, exactly as in C4.

### G2 — compaction renumbers survivors deterministically in canonical value order

Compaction renumbers surviving terms **deterministically in canonical `TermValue`
sort order**. The new `GlobalTermId` assignment is a **pure function of the set of
live term values** — it depends only on which term values survive and their
canonical order, and is independent of ingest history, page arrival order, or the
prior generation's numbering. Two compactions of the same live term-value set
produce byte-identical renumberings.

### G3 — pages are quad-disjoint in `GlobalTermId` space; freeze refuses overlap

Pages must be **quad-disjoint** in `GlobalTermId` space: no quad may appear in more
than one page once terms are mapped to their global ids. Freeze **refuses** with a
hard error if the pages are not disjoint. It **never silently dedups** an
overlapping quad — silent dedup would hide a construction error and make the
composed dataset depend on page ordering, so overlap is a rejected input, not a
condition the layer papers over.

Disjointness is enforced on **all three composed streams**, not only the base quads:
the primary quads and both RDF 1.2 side tables — the reifier bindings and the
annotation triples — are each concatenated across pages with no cross-page dedup, so
two pages that share a reifier binding or an annotation triple are refused exactly as
two that share a base quad. The refusal names which stream the duplicate is in.

### G4 — a `PageProvider` is deterministic and thread-safe

A `PageProvider` supplies page contents to the paged layer on demand. It must be
**deterministic**: the same `PageId` always yields byte-identical quads. It must
also be `Send + Sync`, so the paged layer can be shared and read concurrently.
Determinism at this seam is what lets the composed dataset's egress order and
serialization be reproducible.

### G5 — the shipped `PageProvider` is in-memory; durable tiers are external

The reference `PageProvider` shipped in `purrdf-core` is **in-memory**. A durable
disk-backed or memory-mapped tier belongs to the **external consumer**, not to any
published crate, because every published crate must stay
`wasm32-unknown-unknown`-clean: no filesystem, no threads, no wall-clock, no RNG.
The in-memory provider satisfies the G4 contract on every target, including
wasm32; a consumer that needs persistence implements the same deterministic,
`Send + Sync` `PageProvider` seam against its own storage tier.

### G6 — two construction paths: eager seal vs warm restart

`PagedDataset` offers two constructors with different cost and different guarantees:

- **`from_provider` (eager).** Materializes every page ONCE to fold its terms into the
  shared dictionary by value (G1) and to verify quad-disjointness (G3). This is the
  checked path and the reference way to build a paged dataset from raw pages — but its
  construction cost is `O(all pages)`, which for a very large store is exactly the scan
  the paged design exists to defer.
- **`from_parts` (warm restart).** Reconstitutes a dataset from a pre-built
  `GlobalDictionary` and per-page `PagePart`s (`to_parts` is the inverse) WITHOUT
  materializing any page — construction is `O(page count)`, not `O(all content)`. A
  store that has already sealed once and persisted its dictionary and translations
  reloads through this path. It does NOT re-verify G3 (it cannot without reading the
  pages); the caller warrants the parts came from a previously-disjoint seal.

The reference `PagedDataset` is a **demonstrator** backend: `from_provider`'s eager
seal means it does not itself build a dictionary incrementally at ingest. A consumer
whose working set exceeds what a single eager scan can afford builds its index
incrementally at ingest and reaches the evaluator either through `from_parts` (a warm
restart from that persisted index) or by implementing `DatasetView` directly (C1) —
the id-agnostic read seam serves the evaluator over any backend, not only this one.

---

## Determinism & wasm

Determinism in PurRDF comes from **id-sorting and BTree egress** and from applying
**canonical ordering before serialization** — never from hash iteration order. The
interners use fixed-key ahash rather than a randomized hasher (getrandom is absent
on wasm32, and ids are insertion-ordered), so hash-map iteration order is never a
source of observable order: every serializer and the GTS writer sort or fold
through ordered structures first. The `TermId` niche, `GraphMatch`, the by-value
mutation boundary (C4), and the by-value page translation (G1) all preserve this:
none of them lets a hasher's iteration order leak into output. Compaction's
canonical `TermValue` ordering (G2) is the paged-layer instance of the same rule.

Everything in `purrdf-core` — including the reference in-memory `PageProvider`
(G5) — stays `wasm32-unknown-unknown`-clean: no filesystem, threads, wall-clock, or
RNG. Storage tiers that need any of those live in the external consumer, behind the
same deterministic `PageProvider` seam.
