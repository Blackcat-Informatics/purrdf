<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# Embedding Nearest Neighbours

**What it replaces, and where it stops.** This is the surface that lets an RDF
project drop the pgvector it kept beside its triple store for nearest-neighbour
search: `?neighbour <space> ( ?seed k ?distance )` answered in-process, exact
top-k under the metric the artifact declares, and byte-identical natively and
on wasm32. It is an exact scan — every candidate scored, no pruning, no
approximate index — bounded by a caller-supplied `KnnGuard`, under three
metrics (cosine, negative dot, squared Euclidean), over a PURREMB embedding
space. PurRDF computes no embeddings and runs no ANN payload: the vectors
arrive in an artifact the caller produced.

The PURREMB layer in `purrdf-core` (see
[Deterministic embedding companions](../concepts/codecs.md#deterministic-embedding-companions)
and [`docs/PURREMB.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/PURREMB.md))
stores RDF-1.2-addressable vectors, a declared distance metric, and
tamper-evident guards binding third-party index payloads to the exact matrix
they were built over. It deliberately does not rank anything — ranking is a
query operation and PURREMB is an artifact format. The `knn` module of
`purrdf-sparql-eval` is that query operation, exposed through the
property-function seam.

## The call shape

```sparql
PREFIX knn: <https://example.org/space/>
PREFIX d:   <https://example.org/d/>

SELECT ?neighbour ?distance WHERE {
  ?neighbour knn:points ( d:a 3 ?distance )
}
```

Four flattened positions — one on the subject side, three on the object side:

| position | name | role |
|---|---|---|
| 0 | `?neighbour` | **out**: the RDF term whose vector was retrieved |
| 1 | `?query` | **in**: the term whose vector seeds the search |
| 2 | `k` | **in**: how many neighbours to retrieve |
| 3 | `?distance` | **out**: the distance, as `xsd:double` |

The one declared mode is `fbbf`: the seed and `k` are inputs the relation
cannot enumerate, while `?neighbour` and `?distance` may each be bound or free.
`k` accepts any integer-derived datatype (`xsd:integer`, `xsd:int`,
`xsd:long`, `xsd:nonNegativeInteger`, …); `k = 0` is a valid request for zero
neighbours. A seed with no vector in the space is an **empty answer**, not an
abort — otherwise every query ranging a seed over partly-embedded terms would
die.

## PurRDF mints no IRI for this

The predicate is the caller's, and it is how a query names a space: a host
builds one `EmbeddingSpace` per `(artifact, target set, vector space)` triple it
wants queryable and registers an `EmbeddingKnnRelation` over it under whatever
IRI its own vocabulary uses. There is no default IRI and no fallback space.
Registering one relation per space — rather than one relation taking a space
IRI as an argument — is what lets the planner read an honest row bound off the
space that will actually be searched.

```rust,ignore
use std::sync::Arc;
use purrdf::sparql::{EmbeddingKnnRelation, EmbeddingSpace, KnnGuard, PropertyFunctionRegistry};

// The guard is caller-supplied configuration with no default: the largest
// space one invocation may scan, and the largest `k` it may request.
let guard = KnnGuard::new(/* max_candidates */ 100_000, /* max_neighbours */ 64)?;

// `bindings` names the RDF term each PURREMB row stands for. A row with no
// term is a construction-time refusal, not a skipped row — a top-k over a
// silently smaller candidate set would be the top-k of a subset.
let space = EmbeddingSpace::from_artifact(
    &purremb_bytes, target_set_id, vector_space_id, bindings, guard,
)?;

let mut registry = PropertyFunctionRegistry::new();
registry.register(
    "https://example.org/space/points".to_owned(),
    Arc::new(EmbeddingKnnRelation::new(Arc::new(space))),
);
```

The registry then reaches the engine through `QueryOptions::property_functions`
exactly as on the [full-text](full-text.md) page. This is a Rust-host seam.

## Exact search, ordered under the declared metric

The search is **exact**: every candidate row is scored under the metric the
space's family contract declares (`FamilyView::metric()` — cosine or squared
Euclidean disagree on real data, and a surface using one kernel for both fails
the tests that pin them apart), and the `k` returned are the true `k` nearest.
There is no candidate pruning anywhere. That is what lets the engine's
row-ceiling pushdown be sound here — emission order *is* rank order, so the
first `n` rows are the `n` nearest for every `n ≤ k` — and what makes
"ordered correctly" a property the module is tested for rather than a property
of a tuning parameter.

Rank order is `(distance ASC, row ASC)`. Row numbers are distinct, so the order
is strict and total, and the tie-break is meaningful: a PURREMB target set is
sorted and deduplicated by `TargetId`, a digest of canonical identity, so
ascending row number *is* ascending canonical content order. Two artifacts
built from the same content in opposite insertion orders answer byte for byte,
serialized JSON included.

## Binary64, in a pinned order

The workspace's other ranked-retrieval surface forbids floating point entirely,
because BM25 needs a logarithm and IEEE-754 does not require `ln` to be
correctly rounded. That reason does not transfer. A kNN kernel needs no
transcendental — subtraction, multiplication, addition, division and square
root are all correctly rounded — so binary64 gives the *same* cross-target
guarantee here, and it is the arithmetic `docs/PURREMB.md` states normatively
("binary64, round-to-nearest ties-to-even, in the written order, without a
fused multiply-add"). Fixed point would have introduced a quantization step the
format does not have.

The two residual hazards are closed structurally: every fold runs over
ascending component index in one sequential loop, never split across workers
(a test folds `[1e16, -1e16, 1]`, which sums to `1` one way and `0` the other,
and pins which), and every product is bound to a named local before it is added
so no fused multiply-add can form. Distances are emitted as `xsd:double`, whose
canonical lexical round-trips the exact bits — a rounded decimal would print
two adjacent doubles alike and hide exactly the divergence this is about.

One limit, stated rather than hidden: cosine self-distance is not exactly zero
(`dot(v, v)` and `|v|·|v|` are two roundings of one real number), so a seed
ranks strictly ahead of every other direction rather than at a zero that is not
there.

## What a search costs

The governor's earlier charge points priced a host relation by invocations
driven and rows accepted. For a generator, neither is where the cost is: a scan
over a million vectors returning five rows would have been priced like a
six-row table scan. The relation therefore reports its internal work through
`PfCursor::take_work` after every pull, and the engine spends one unit of fuel
per reported unit — so the search charge follows the space size, not the rows
returned, and a `KnnGuard` is what keeps both the charge and the planner's row
bound honest.

The design record — the four decisions that were not obvious and what fixed
each — is
[`docs/design/purrdf-embedding-knn.md`](https://github.com/Blackcat-Informatics/purrdf/blob/main/docs/design/purrdf-embedding-knn.md).
