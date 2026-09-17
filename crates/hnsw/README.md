<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# `purrdf-hnsw` — A deterministic HNSW index

`purrdf-hnsw` is the approximate nearest-neighbour (ANN) index of the PurRDF
toolkit. It is a hierarchical navigable small-world graph — the Malkov &
Yashunin algorithm — with the random number generator **removed**: node levels
and the build schedule are pure functions of the input, so the index is a
function of the data rather than of an RNG state, an insertion order, or a
rayon worker count.

The crate fills the ANN slot PURREMB declares and external crates could not fill:
two mature Rust HNSW implementations were evaluated and rejected because their
level assignment draws from an entropy-seeded generator with no seed-injection
API, and the deterministic mode they did offer was slower than an exact scan at
4,096 dimensions.

## Determinism by construction

Every source of nondeterminism an HNSW implementation usually carries is
replaced with a fixed rule:

* **Levels** are `splitmix64(stable_row_index)` through a pinned integer formula
  (`purrdf_hnsw::level`). The row index is the PURREMB target-set row order,
  ascending `TargetId`, so a rebuild over the same artifact reproduces the same
  levels.
* **The build is round-structured.** The entry point is computed from the level
  assignment before any link exists and excluded from every batch, so no round
  reads an empty snapshot. Batches start at one row and double, capped at 2048;
  each round proposes against a frozen snapshot and the proposals are merged
  canonically, sorted by the shared rank order and truncated to the degree bound.
  The schedule is part of artifact identity, not a caller parameter.
* **Every row stays reachable.** The degree bound can drop a node's last inbound
  edge, and out-edges do not make a row findable, so a final serial pass gives
  every unreachable row an inbound edge and protects it from later eviction. A row
  nothing points at could never be returned — not even as its own nearest
  neighbour at distance zero — and a graph that strands rows is still perfectly
  deterministic, so no digest could catch it.
* **Neighbours are selected for diversity**, by the relative-neighbourhood
  condition rather than by keeping the nearest `M`. Nearest-`M` truncation spends
  every edge in one direction and leaves the graph unnavigable.
* **Distances** come from the exact path's kernels, consumed from
  `purrdf_sparql_eval::knn::{Kernel, Ranked}` rather than re-implemented.
* **The digest** committed by a guard is a hand-rolled FNV-1a fold of the
  canonical payload bytes, stable across toolchain bumps; it is never
  `DefaultHasher` (SipHash, unspecified) or a randomly seeded map.

The result is a **canonical byte image** that is identical at 1, 2, 4 and 8
rayon workers and identical across `wasm32-unknown-unknown`. `rayon` runs
inline-sequentially on wasm, so that build is slower but not different.

## The approximation contract, stated honestly

This is an **approximate** index, and the crate is explicit about what that
means:

* **Recall is measured against the exact path, and pinned.** The conformance
  harness compares every offered row to the exact scan over a deterministic
  fixture family and pins the hit count as an **exact equality** — 96 regimes of
  `(fixture, parameters, k)` — rather than as a floor, because recall is a pure
  function of the corpus, the parameters and the algorithm. Held-out queries, the
  case where the answer is not already a stored row, are pinned separately. This
  is the sentence the artifact itself carries as its approximation evidence.
* **Measured to 50,000 rows; unmeasured at 10^6.** See the table below. The regime
  that motivates an ANN index at all is a million rows, and this crate has no
  measurement there — that is a gap in the evidence, not a property of the index.
* **The exact path stays the oracle.** `purrdf-sparql-eval`'s kNN relation is
  not replaced or modified. An HNSW result is an **offer of candidates** and
  never a certification that no nearer row exists; an empty result is never a
  proof of absence.
* **Parameters are required, with no defaults.** `M`, `M0`, `ef_construction`
  and `ef_search` are the index identity. A build under different numbers is a
  different index, so there is no `Default`, no builder that fills in a value,
  and no query-time override.
* **The loss contract travels in the artifact.** The approximation evidence
  string is carried as the implementation identity's revision bytes, so a guard
  binds the statement of what the index does not promise along with the graph.

## Build cost

### Measured admission evidence

Recall and per-query work are **deterministic** — pure functions of the corpus, the
parameters and the algorithm — so they are reported as measurements. Wall-clock is a
single-host sample, disclosed and never a threshold.

Recall against the exact scan, 50,000 rows × 4,096 dimensions, `k = 10`:

| corpus | `ef` | recall@10 | rows visited / query |
|---|---|---|---|
| embedding-like | 16 | 0.9344 | 362 of 50,000 |
| embedding-like | 32 | 0.9828 | 539 |
| embedding-like | 128 | **0.9984** | **1,093 (2.2%)** |
| uniform (control) | 128 | 0.5656 | 3,577 |
| uniform (control) | 512 | 0.8047 | 9,853 |

The uniform row is a control, not a target: independent coordinates make distances
concentrate, so no index scores well there and the figure describes the generator. The gap
between the two columns at the same `ef` is the evidence that the corpus, not the index,
decides a recall number.

Build cost, same shape, single-host samples:

| rows | build | payload image |
|---|---|---|
| 5,000 | 2.7 s | 2.7 MiB |
| 50,000 | 135.2 s | 26.8 MiB |
| 200,000 | 501.2 s | 107.2 MiB |
| 1,000,000 | **not measured** | — |

**The million-row point has not been observed.** On the curve above it extrapolates to
roughly fifty minutes and about 33 GiB resident, but an extrapolation is not a measurement
and this contract does not present it as one. That is the honest state of the evidence: the
index is measured where it has been measured, and the regime that motivates an ANN index at
all is one order of magnitude beyond it.

Building at 4,096 dimensions is expensive and is disclosed rather than tuned
away. `cargo bench -p purrdf-hnsw --bench build` times the shipped build path
at 5,000, 50,000, 200,000 and 1,000,000 rows — every scale, with no opt-in,
because a default that skips the one scale the offer is about reports a missing
measurement as a completed run. The million-row rung needs about 30.5 GiB
resident for the matrix alone; `PURRDF_HNSW_BENCH_SCALES=5000,50000` narrows the
run on a host that cannot hold it, and can only take scales away. Every number
it prints is a wall-clock sample from
whatever host runs it, reported for disclosure and never as an acceptance
threshold. The cost that is deterministic and hardware-independent is distance
evaluations: `CacheState::evaluations` (`crates/hnsw/src/search.rs`) counts
every one a search performs, and `cargo bench -p purrdf-hnsw --bench recall`
reports the mean and maximum rows visited per query at each `ef_search`.

## Usage

```rust
use purrdf_hnsw::{build, Params, VectorMatrix};
use purrdf_core::DistanceMetric;

let matrix = VectorMatrix::from_rows(&[
    vec![1.0, 0.0],
    vec![0.0, 1.0],
    vec![0.9, 0.1],
])?;
let params = Params::new(2, 4, 8, 4)?; // M, M0, ef_construction, ef_search
let index = build(matrix, &DistanceMetric::SquaredEuclidean, params)?;
let nearest = index.search_rows(0, 2)?; // row 0's two nearest neighbours
```

The crate depends on `purrdf-core`, `purrdf-sparql-eval`, `purrdf-xsd` and
`rayon`.

## PURREMB integration

An HNSW index is a derived index over PURREMB's existing `INDEX_GUARDS` slot. The
crate is a **typed adapter** over `purrdf-core`'s `IndexGuardContract`,
`DerivedIndex`, `IndexPayloadStorage` and `IndexGuardView`; no parallel wire
format is invented and no `purrdf-core` source changes.

`guard::load` proves the guard's profile, coordinates and payload commitment
before any search runs. `verify_rebuild` recomputes the graph from the source
matrix and declared parameters and compares the canonical payload commitment,
so a stale or tampered payload is a construction-time failure rather than a
silent wrong answer.

The relation is exposed through the evaluator's property-function seam: the
consumer registers it under a **caller-supplied** predicate IRI with
`register_hnsw_relation`. PurRDF mints no vocabulary, and there is no fabricated
namespace. The call shape is the exact kNN relation's — `(?neighbour, ?query, k,
?distance)` with mode `fbbf` — so queries name the provider explicitly, and the
result's approximation is governed by the guard's loss contract, the
caller-supplied predicate, and the cursor contract that a result is never a
completeness claim.

## Verification

```sh
cargo test -p purrdf-hnsw                 # unit, invariant, guard and oracle suites
make wasm                                 # builds the crate for wasm32-unknown-unknown
make hnsw-determinism                     # proves native and wasm32 bytes are identical
cargo bench -p purrdf-hnsw --bench recall # recall/work/latency against the exact oracle
```
