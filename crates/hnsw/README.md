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
* **The build is round-structured.** Rows are processed in batches of
  `max(1, isqrt(n))`; each round proposes against a frozen snapshot of the graph
  and the proposals are merged canonically, sorted by the shared rank order and
  truncated to the degree bound. The graph does not depend on insertion order.
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

* **Recall is unmeasured on realistic corpora.** No recall percentage is
  promised. The conformance harness measures recall@k against the exact path on
  deterministic fixtures and reports it; that evidence is what an admission
  decision uses, not a claim in this file.
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

Building at 4,096 dimensions is expensive and is disclosed rather than tuned
away. `cargo bench -p purrdf-hnsw --bench build` times the shipped build path
at 5,000, 50,000 and 200,000 rows, with 1,000,000 rows behind
`PURRDF_HNSW_BENCH_1M=1`; every number it prints is a wall-clock sample from
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
