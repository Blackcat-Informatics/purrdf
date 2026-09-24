<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# `purrdf-hnsw` design

`purrdf-hnsw` is the deterministic approximate nearest-neighbour (ANN) index of
the PurRDF toolkit. This document records the decisions behind the surface a
reader would otherwise take for an oversight — the RNG that is not there, the
parameters that have no defaults, the payload layout that exists before a view
is added, the work count that is a real count — and states the guarantees the
crate does and does not make.

It is a **sibling crate**, not a kernel change. `purrdf-core` and
`purrdf-sparql-eval` do not name this crate; the dependency arrow is
one-directional, `purrdf-hnsw → {purrdf-core, purrdf-sparql-eval, purrdf-xsd,
rayon}`. The exact kNN path remains the oracle and is not modified.

All example IRIs use `example.org`. PurRDF mints no vocabulary; every predicate
a query calls this crate by is caller-supplied configuration.

---

## 1. Determinism is a construction rule, not a seed

The published HNSW algorithm assigns each inserted node a level drawn from a
random number generator. That single choice is why a graph built twice from the
same input is not the same graph, and why mature external crates without a
seed-injection API cannot meet a byte-determinism bar. This crate removes the
generator rather than seeding it.

### 1.1 The level formula

A node's level is a pure integer function of its stable row index:

```text
bits_per_level(m) = floor(log2(m))              // largest k with 2^k <= m
level(i)          = min( lz(splitmix64(i)) / bits_per_level(m), level_cap(n, m) )
```

`lz` is the leading-zero count of the 64-bit SplitMix64 finalizer applied to the
row index — the counter folded in, no stateful generator to advance. `P(level >=
l)` is `2^(-b*l)`: exactly `m^-l` for a power-of-two `m`, and the documented
power-of-two lower bound otherwise. `level_cap` is the smallest `L` with
`m^L >= n`, derived from the input, not a caller parameter; the index identity
stays exactly `M`, `M0`, `ef_construction`, `ef_search`.

The row index is the **PURREMB target-set row order**, ascending sorted
`TargetId` — so rebuilding the same artifact reproduces the same levels and the
same graph. There is no seed, no entropy, no wall clock, and no float on the
level path; a level is a property of the index data and not of the target's
libm.

### 1.2 The build state machine

A serial HNSW insertion lets each node see the links its predecessor just made,
which makes the graph a function of insertion order and makes a parallel build
impossible. The build here cuts the dependency with finitely many **rounds**,
each a pure function of the graph that existed before it:

1. **Freeze.** The graph from the previous round is the sole snapshot every
   proposal in this round may read. The entry point is computed from the level
   assignment **before any link exists** and is excluded from every batch, so the
   snapshot is never empty and no round needs a bootstrap special case.
2. **Propose (parallel).** Each node in the batch runs the standard insertion
   search against the snapshot only — greedy descent above the node's level
   when the snapshot has layers there, then an `ef_construction` beam at each
   layer from the node's top down to layer 0. Candidates are the shared
   `Ranked` total order `(distance, row)`; proposal links are emitted in both
   directions. Every distance computed is memoized.
3. **Merge (canonical).** Every touched node's new neighbourhood is `frozen
   neighbours ∪ inbound proposals`, sorted by `(distance, row)` and truncated to
   the per-layer degree bound (`M0` at layer 0, `M` above). A set union
   followed by a total sort is order-free, so the merge cannot observe which
   worker produced which proposal.
4. **Commit.** The merged adjacency replaces the frozen one. The entry point does
   not move: it was fixed before the first round and is the minimum row index at
   the maximum level, a function of the level assignment alone.
5. **Repair (serial).** The degree bound in step 3 can drop a node's last
   *inbound* edge, and out-edges do not make a row findable — a beam arrives only
   by following an inbound edge. A final serial pass gives every unreachable row
   an inbound edge from the nearest already-reachable node it points at, and
   protects that edge from later eviction. Protected edges are never removed and
   every pass converts at least one row, so the repair terminates in at most `n`
   passes.

The loop walks a finite partition of the rows, so termination is structural and a
node is proposed exactly once. There is no `while pending` and no level-based
re-queue.

The batch schedule is **an integer rule of the profile, not a caller parameter**,
and it is part of artifact identity: a different schedule is a different canonical
image. It starts at a single row and doubles, capped at `MAX_ROUND = 2048`.

Both halves of that rule are load-bearing, and each fixes a distinct way the graph
can lose rows. Rows within one round cannot link to each other — they all read the
same frozen snapshot — so a round is a window of mutual invisibility. Starting at
one keeps the early graph dense in links; a flat schedule of `isqrt(n)` rows would
instead leave an entire first batch reading an empty or near-empty snapshot, and
every row in it would end the build with no edges at all, unreachable forever.
Capping the doubling bounds the window at the other end: uncapped, the final round
is half the corpus, and half the corpus mutually invisible is a graph whose recall
collapses at scale.

The within-round isolation is load-bearing and is asserted rather than claimed:
`batch = 1` is a plain serial insertion and produces a **different** canonical
digest.

### 1.3 Why rayon does not break it

The round's proposals run through rayon, but each reads only the frozen
snapshot and the shared memo, and the merge is order-free. Worker count can
therefore change how long the build takes and cannot change its output. On
`wasm32-unknown-unknown` rayon runs inline-sequentially; the graph is
byte-identical, only slower.

---

## 2. The PURREMB profile

PURREMB stores opaque index payloads and refuses to interpret them. Everything a
reader needs to decide *which* opaque payload is this crate's — and to reject
one that claims to be but is not — is stated by the crate, in
`purrdf_hnsw::profile`, and carried in the guard. No parallel wire format is
invented; the adapter is `purrdf_hnsw::guard` over `purrdf-core`'s existing
`IndexGuardContract`, `DerivedIndex`, `IndexPayloadStorage` and
`IndexGuardView`.

### 2.1 Identity

| field | value |
|---|---|
| implementation identifier | `hnsw-v2` (`hnsw-reassociated-v2` for the reassociated index, §3.1) |
| implementation media type | `application/vnd.blackcatinformatics.purrdf.hnsw.profile-v1` |
| parameter encoding | `application/vnd.blackcatinformatics.purrdf.hnsw.parameters+tlv-v1` |
| payload media type | `application/vnd.blackcatinformatics.purrdf.hnsw` |
| `use_role` | `Generic` |

The implementation identity's digest is a domain-separated SHA-256 of a stable,
human-readable profile declaration (no host layout, no version-dependent
serialization). The declaration names the distance arithmetic
(`arithmetic=binary64-lane16-tree-v1`), so the digest binds the law every recorded
distance was folded under as well as the algorithm. Its revision bytes are the approximation evidence string, so
the guard digest commits **what the index does not promise**, not merely the
algorithm name.

### 2.2 The loss contract

The guard's `IndexLossContract` is approximate and non-transforming:
`transforms_vectors = false`, with no `loss_encoding` and no `loss_parameters`,
because an HNSW graph stores no vectors at all. The evidence string is exactly
`approximate: recall measured against the exact oracle on synthetic corpora up to
50,000 rows, and UNMEASURED at the 10^6 scale this index exists for; an offer of
candidates is never a proof of absence`;
`guard::validate_guard` refuses a guard whose revision says anything else, so a
host cannot bind an HNSW index without binding that statement — including the
half that says where the evidence stops.

That quote is checked against `profile::LOSS_EVIDENCE` itself by
`scripts/check-doc-claims.py`, not maintained by hand. It was maintained by hand,
and it went stale the moment the constant was corrected: this paragraph went on
publishing the superseded sentence — the one whose missing scale caveat was
exactly what the correction added — while the code, its test pin and the shipped
README all carried the honest one. A document that restates a constant is the
last place a reader checks and the first place a correction is forgotten.

### 2.3 Parameter TLV

Four `u64` fields in ascending tag order, little-endian, each padded to an
8-byte boundary — the canonical TLV form `purrdf-core` uses:

| tag | field |
|----:|---|
| 1 | `M` |
| 2 | `M0` |
| 3 | `ef_construction` |
| 4 | `ef_search` |

The block is the guard's `parameters` field, and `purrdf-core` commits its
SHA-256 as the guard's self-digest, so a changed parameter is a guard that fails
verification rather than an index that silently searches under a different
identity.

### 2.4 Canonical payload layout

The graph is a contiguous, little-endian, 8-byte-aligned image with no pointers
and no host-endian fields, so it is the same bytes on every target:

```text
header:
  magic      [u8; 8]   IMAGE_MAGIC
  version    u32       IMAGE_VERSION
  kernel     u32       0 cosine, 1 negative-dot, 2 squared-euclidean
  M          u64
  M0         u64
  ef_c       u64
  ef_search  u64
  node_count u64
  max_level  u32
  arithmetic u32       the distance arithmetic's image code: 1 = Exact
                       (binary64-lane16-tree-v1); 2..=7 = Reassociated
                       (binary64-reassociated-v1) on the dispatch path the
                       build ran; 0 is refused
  entry      u64       row, or u64::MAX for an empty graph
node records, in ascending row order:
  row        u64       must equal the record's position
  level      u32
  reserved   u32       0
  layer records, ascending layer 0..=level:
    layer      u32     must equal the record's position
    reserved   u32     0
    count      u64
    neighbours, strictly ascending by neighbour row:
      row      u64
      distance u64     f64 bits
```

The in-memory graph orders a node's neighbours by `(distance, row)`; the image
orders them by neighbour row, because the row set is the identity and the
distances are derived. Decoding re-sorts by rank, so the two are views of one
graph. The layout is fixed **before** any borrowed view is added precisely so a
zero-copy `HnswView<'a>` can be introduced later without a wire-format change.

`IMAGE_VERSION` (and `INDEX_VERSION`) is 2. Version 2 is the first whose recorded
distances are folded by `purrdf_core::distance::Exact` — sixteen binary64 lanes, the
pairwise tree `(l, l+8)`, `(l, l+4)`, `(l, l+2)`, `(0, 1)`, then a sequential tail —
and the first whose header records that arithmetic, in the `u32` version 1 reserved as
zero. A version-1 image's distances were folded sequentially, so its bits are not the
ones this build computes: `decode`, `guard::load` and `guard::verify_rebuild` refuse it
with the named `HnswError::VersionMismatch`, never with an `Ok(false)` that would read
as tampering. A header whose arithmetic field is not `1` is refused by the exact decoder with
`HnswError::ArithmeticMismatch`; the reassociated index records its path's code there
instead (§3.1). The exact canonical image is byte-identical across worker counts,
across the exact arithmetic's dispatch paths (portable and AVX2 on x86-64), and across
`wasm32-unknown-unknown` with and without `+simd128`; `make hnsw-determinism` executes
all three wasm-side and native digests against the one golden. A reassociated image is
byte-identical across worker counts and bound to its build and dispatch path.

Every build, rebuild, decode and search resolves the exact arithmetic on its own thread
first, which refuses a flush-to-zero or re-rounding float environment with
`HnswError::FloatEnvironment`, and a beam expands each node by one call to the batch
kernel over its unvisited neighbours.

### 2.5 What a binding proves before a search runs

`guard::load` is the strict entry point, in order: the guard declares this
crate's implementation, parameter encoding, media type and evidence revision;
the inline payload's length and SHA-256 match what the guard committed; and the
canonical image decodes over the matrix with embedded parameters that agree with
the guard's parameter block. Stale, substituted, zero-matching and
multiple-matching guards are construction-time errors naming the index
coordinate.

`guard::verify_rebuild` answers the question an opaque payload otherwise cannot:
*is this payload the canonical image of building the given matrix under the
given parameters?* It recomputes the graph and compares. The canonical image is
the same bytes the determinism digest folds, so rebuildability and determinism
are one claim rather than two.

---

## 3. The distance and tie law

Every distance comes from `purrdf_sparql_eval::knn::Kernel`, consumed through the
public re-export; no wrapper re-implements the arithmetic. Candidates are
ordered by the exact path's `Ranked` type `(distance, row)` — the same total
order the exact kNN relation uses — so there is no second comparator to drift
out of step. Non-finite kernel results are hard errors, never `partial_cmp`
panics; a zero-norm row under a norm-dividing kernel is rejected at
construction.

The validation matrix is fail-closed at construction: `M >= 2`, `M0 >= M`,
`ef_construction >= M`, `ef_search >= 1`, non-zero rows and dimensions, checked
count arithmetic, and a 32-bit address-space admission check on wasm32. There is
no `Default` and no query-time override, because the four numbers are the index
identity rather than tuning.

### 3.1 The reassociated index

The index is generic over its distance arithmetic: `HnswIndex<A: Arithmetic = Exact>`,
with `HnswSpace<A>` and `HnswRelation<A>` over it. `HnswIndex::build` and
`HnswIndex::decode` (and `hnsw::build`) are unchanged and exact.
`HnswIndex::build_reassociated` (and `hnsw::build_reassociated`) builds the same
algorithm under `purrdf_core::distance::Reassociated`, whose sums may be reassociated
and contracted to fused multiply-add along the dispatch path the build resolves, the
widest this process runs;
`HnswIndex::decode_reassociated`, `guard::load_reassociated` and
`HnswSpace::from_artifact_reassociated` read it back. It is a second type, not a mode:
neither index ever computes a distance under the other's law, and there is no runtime
branch per distance.

**One distance per pair.** Build, neighbour selection, repair and search all reach a
pair's distance through the reassociated kernels, which exist exactly once per
dispatch path and stored-width pair, out of line. So on one path every call site
computes one pair's distance with the same compiled copy and gets the same bits; the
bounded form a selection uses returns the full value's bits whenever it returns one,
because its checkpoints are true prefixes. The graph a reassociated build produces is
therefore still a pure function of its input *on that path*, and byte-identical across
worker counts.

**What it gives up.** Its distances may differ in the last bits from the exact index's,
and between dispatch paths and builds, so wherever two candidates nearly tie the graph
may link or rank them differently. Its recall is graded against the exact oracle
exactly as the exact index's is, and on the conformance family it meets the exact
index's pinned recall on every regime.

**Where the choice is recorded.**

| record | exact index | reassociated index |
|---|---|---|
| image header `arithmetic` field | `1` | the code of the dispatch path the build resolved: `2` sse2, `3` avx2+fma, `4` avx512f, `5` neon, `6` wasm-simd128, `7` wasm-scalar, `8` portable (every target other than x86-64, aarch64 and wasm) |
| implementation identifier | `hnsw-v2` (`hnsw-reassociated-v2` for the reassociated index, §3.1) | `hnsw-reassociated-v2` |
| evidence revision | `LOSS_EVIDENCE` | `LOSS_EVIDENCE`, `"; "`, the reassociated evidence for the path, and "Its canonical image is reproducible only by a build running the same dispatch path." (`profile::loss_evidence_reassociated`) |
| profile declaration | `arithmetic=binary64-lane16-tree-v1` | `arithmetic=binary64-reassociated-v1`, and the path's revision |
| `IndexLossContract` | `transforms_vectors: false` | `transforms_vectors: false` |
| ranked declaration | `RankArithmetic::float_distance::<Exact>()` | `RankArithmetic::float_distance::<Reassociated>()` |
| relation fidelity | `Lossy` with `LOSS_EVIDENCE`; order faithful | `Lossy` with the reassociated revision; order `Perturbed` with the arithmetic's evidence |

The loss contract does not change, deliberately: an arithmetic decides how a distance
is rounded and transforms no stored vector, and `purrdf-core` requires
`loss_encoding: None` for a non-transforming index. The image field, the
implementation and its revision carry the choice instead. The guard's legal
(identifier, revision, image codes) rows are derived from `Exact` and `Reassociated`
themselves: the exact identifier with a reassociated revision, the reassociated
identifier with the exact revision, and a reassociated revision naming another path
than the payload records are each a `GuardProfile` failure.

**Bound to its build and its path.** A reassociated image is reproducible only on the
dispatch path that built it, so `decode_reassociated`, `verify_rebuild` and every search
resolve *the recorded path* through `Arithmetic::resolve_recorded`, not the widest one
this process runs. A processor runs every compilation its binary holds whose features it
reports: an image built on `x86_64`'s SSE2 or AVX2+FMA path is decoded, searched and
rebuilt on that path by an AVX-512F processor, and the decoded index re-encodes its own
code. Only a path this process cannot run -- its compilation belongs to another target,
or the processor does not report a feature it needs -- is refused, with
`HnswError::ArithmeticPathUnavailable { recorded, available }`, where `available` is the
widest path this process runs (the one a new build here would record). The refusal is
named, never an `Ok(false)` that would read as tampering, and never a search run with
bits from another compilation. A header naming a code that is not one of the index type's
own is `HnswError::ArithmeticMismatch`, in both directions.

**Compiled in this crate.** The search traversal and the graph build are held by the
index as walks its non-generic constructors instantiate, so both are compiled in
`purrdf-hnsw` for both arithmetics -- the copies the asm evidence gate measures
(`hnsw.search-build` in `docs/design/purrdf-simd.md`) -- and every search, batch and
rebuild verification runs those copies, whichever crate calls the generic method.

---

## 4. The digest doctrine

The crate's central claim — the canonical image is a pure function of the input
— is turned into evidence by `crates/hnsw/src/determinism.rs`: it builds a fixed
5,000-row × 4,096-dimension corpus generated from a seeded SplitMix64 stream
(no committed binary, no entropy at test time), serializes the canonical
payload, and folds the **bytes** with a hand-rolled FNV-1a `u64`.

FNV-1a is chosen because it is six lines of integer arithmetic with published
constants and no state. `DefaultHasher` is SipHash with an unspecified
per-process key and `ahash` is seed- and version-sensitive by design; either
would make the digest move for reasons unrelated to the graph, which is exactly
the false signal the harness exists to remove. A golden that moves under a
toolchain bump is therefore a serialization defect, never a hasher change.

Two proofs share the one constant:

* `crates/hnsw/tests/determinism.rs` pins the golden at 1, 2, 4 and 8 rayon
  workers and asserts a serial-insert build produces a different digest;
* `scripts/check-hnsw-determinism.sh` builds the workspace-excluded cdylib in
  `crates/hnsw/determinism` for `wasm32-unknown-unknown`, runs it under Node,
  and fails unless the wasm digest equals the same native golden over the same
  corpus length.

`make hnsw-determinism` runs the gate. It needs the wasm32 target and Node, so
it is **not** part of `make check`; CI runs it in the wasm job where both are
present, and hard-fails there if the target is absent.

---

## 5. The approximation contract

This index is approximate, and the contract is declared rather than implied.
Four governed channels carry it:

1. **The artifact.** The guard's `IndexLossContract` and the evidence string are
   bound into the implementation identity and checked at bind time.
2. **The query surface.** The relation is registered under a caller-supplied
   predicate IRI, so the query text names the provider and no namespace is
   fabricated.
3. **The cursor contract.** A result is an **offer of candidates**, never a
   certification of absence. The cursor can say "here are `k` candidates"; it
   can never say "no row is nearer". `crates/hnsw/tests/oracle_contract.rs`
   asserts that even when a query misses the exact oracle's nearest row, the
   empty and non-empty answers are ordinary cursor results and never a
   completeness claim.
4. **The composed answer.** The three channels above all address a caller
   reading this relation directly, and none of them survives composition. Fuse
   these rows with an exhaustive producer's and the answer reports one terminal
   status per stratum — `Exhausted` for both — with nothing to say which one
   offered candidates and which returned everything it had. So the promise is
   also declared where a consumer of *composed* rows reads it, through
   `HnswRelation::ranked_declaration`, and carried verbatim into
   `FusionTrailer::fidelities`.

   The completeness axis is this profile's own evidence string. The order axis
   is **computed from a loss contract** rather than typed as a literal: an index
   whose guard says `transforms_vectors` compares approximated values and can
   rank a row it found *better* than that row was due, which removes the one
   inequality every score bound rests on. This profile does not transform
   vectors, so it is lossy but order-faithful, and a fused answer's error stays
   bounded.

   The contract it is computed from is the build's own, not one decoded out of
   the artifact, and the two cannot differ: a guard publishing a different
   evidence revision or a different loss contract is refused at bind time, so an
   artifact a space could have been built from cannot disagree with the
   constant. It has to be the build's own in any case, because a space built
   from an in-process graph has no artifact to decode a contract out of, and a
   field derived one way on one construction path and another way on the other
   is the modal optionality this workspace refuses.

   What that contract cannot describe is what the host did to the vectors
   **before** the build saw them. Quantize the embeddings, build a graph over
   the codes, and the producer is genuinely order-perturbed while every contract
   reachable from the index still reads `transforms_vectors: false` — the
   contract describes the build, not its input. So `ranked_declaration` takes
   that disclosure as an `OrderFidelity` parameter and composes it with the
   derived one by taking the worse of the two: a host can degrade the axis and
   can never upgrade it, and a host with nothing to disclose spells
   `Faithful` rather than being defaulted into the top of the axis.

   The completeness axis stays the relation's, and is not a parameter beside it.
   It is `Lossy` over every space on every request, so there is no silence there
   for a consumer to read as completeness — the failure a declared fidelity
   exists to prevent cannot occur on that axis — and the profile's own evidence
   occupies it, which is where the approximation contract requires that string
   to reach a consumer byte for byte.

   A relation registered *without* a ranked declaration is not silently fused:
   it forms no stratum, contributes no trailer entry, and the request reports
   its term as unserved. Reaching for the unranked registration by mistake is a
   planner that says so, never a stream that fuses while claiming to be
   exhaustive.

The call shape matches the exact kNN relation, so the two are interchangeable in
query text:

| position | name | role |
|---|---|---|
| 0 | `?neighbour` | out: the RDF term whose vector was retrieved |
| 1 | `?query` | in: the term whose vector seeds the search |
| 2 | `k` | in: how many neighbours to retrieve |
| 3 | `?distance` | out: the distance, as `xsd:double` |

The one declared mode is `fbbf`. Rows are emitted in rank order under the shared
`Ranked` order.

---

## 6. Work accounting

`HnswCursor` reports one unit per candidate distance **actually evaluated**,
through `PfCursor::take_work`. The search is lazy — it runs on the first pull,
not in `PropertyFunction::open` — so an invocation whose ceiling is already
exhausted performs no work and is charged none, and the count resets only when
the engine takes it. A distance already in the round memo is not charged twice.
The rows a search returns are `k`, and `k` says nothing about the size of the
graph they were selected from; the count is the observable work.

Tests cover eager and lazy construction, `k = 0`, bound and free output filters,
a rejected payload, budget exhaustion mid-traversal, and an empty index.

---

## 7. Deliberately absent

These are decisions, not omissions:

* **No `extendCandidates` heuristic extension.** Malkov & Yashunin's optional
  candidate-list extension (walking neighbours-of-neighbours before pruning)
  raises build cost for uncertain recall gain on the target corpus family, so
  it is not shipped — no dead code and no retention test. Neighbour selection
  itself is the plain relative-neighbourhood condition
  (`crates/hnsw/src/select.rs`).
* **No predicate-filtered traversal.** No consumer exists yet, and a filter
  parameter would design against an absent caller; filter pushdown needs
  algebra/evaluator work outside this crate.
* **No full-width rerank after a prefix retrieval.** The PURREMB use role IS read
  off the projection — an index over a projection shorter than the matrix it
  projects declares `CoarsePrefixRetrieval`, and one over the whole stored width
  declares `Generic` — and `IndexCoordinates.prefix_dimension` is bound to the
  projection's own effective dimension rather than to the full width. What is
  absent is the second stage: this profile retrieves and does not rerank, so the
  distances it emits are the prefix's own, exactly as the exact sibling emits them
  over the same projection. A caller wanting a full-width rerank composes it.
  Routing over a prefix does **not** change the distance law: it changes which
  candidates are evaluated, and they are evaluated with the same kernels.
* **No domain-salted level hashing.** The row index pins the level; level
  collisions across distinct vector spaces are harmless because each graph is a
  per-artifact build.
* **No feature-flagged parallelism.** `rayon` is unconditional; the round
  structure is what makes the output independent of workers.
* **No silent fallback to an exact scan.** A requested-but-invalid index is a
  construction-time data-integrity failure naming the coordinate; the exact kNN
  relation remains a separately registered, explicit relation.
