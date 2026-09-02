<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# PurRDF embedding kNN: determinism, guards, and what a search costs

The PURREMB layer in `purrdf-core` stores RDF-1.2-addressable vectors, a declared
`DistanceMetric`, and tamper-evident guards binding third-party index payloads to
the exact matrix they were built over. It deliberately does not rank anything —
ranking is a query operation and PURREMB is an artifact format. `purrdf-sparql-eval`'s
`knn` module is that query operation, exposed through the property-function seam.

This document records the four decisions that were not obvious, and the reasoning
that fixed each one. It is a design record, not a tutorial; the module's own rustdoc
is the reference.

## 1. Binary64, not fixed point — and why that is not the weaker choice

PurRDF's other ranked-retrieval surface computes every score in exact integer
arithmetic and forbids floating point at the crate root. The reason is specific and
it does not transfer: BM25 needs a natural logarithm, and `ln` is one of the
operations IEEE-754 does **not** require to be correctly rounded. Every libm is
entitled to a different last bit and they take it, so the same score can come back
as two adjacent doubles on x86-64 and on `wasm32-unknown-unknown` — enough to swap
two nearly-tied documents and make one engine return rows in two different orders.

A kNN kernel needs no transcendental at all. Squared Euclidean distance is
subtraction, multiplication and addition. Cosine distance adds a division and a
square root. **All five of those operations are correctly rounded by IEEE-754**:
each has exactly one permissible result for a given pair of operands, on every
conforming target. Reaching for fixed point here would not buy a stronger guarantee;
it would buy the same guarantee at the cost of introducing a quantization step that
PURREMB's own format does not have, and of disagreeing with the artifact's
arithmetic.

It would also disagree with the spec. `docs/PURREMB.md` already states the
arithmetic contract normatively: *"All intermediate operations are IEEE-754 binary64,
round-to-nearest ties-to-even, performed in the written order without a fused
multiply-add."* The kNN kernels implement exactly that, and `purrdf-core`'s own
deterministic L2 fold is the code precedent, `#[allow(clippy::suboptimal_flops)]`
and all.

So there are precisely two residual ways a float kernel can diverge, and both are
closed structurally rather than hoped about:

| hazard | why it would diverge | what closes it |
|---|---|---|
| **reassociation** | float addition is not associative, so a sum depends on the order it was folded in | every fold runs over ascending component index, in one sequential loop; no accumulator is ever split across rayon workers or chunked |
| **fused multiply-add** | `a * b + c` as a single FMA rounds once where the written form rounds twice | every product is bound to a named local before it is added; Rust never contracts implicitly and PURREMB forbids the fusion |

The reassociation rule is asserted, not merely stated. A test folds a vector chosen
so the two directions genuinely disagree — `[1e16, -1e16, 1]` against all-ones sums
to `1` left-to-right and `0` right-to-left — and pins which one the kernel produces.
A test that only checked "the same input gives the same output twice" would pass on
a kernel with no fixed order at all.

### The limit of the claim, stated

Cosine self-distance is **not exactly zero**. `dot(v, v)` and `|v| · |v|` are two
different roundings of the same real number, so their quotient is one ULP off `1`
and `1 - quotient` is one ULP off `0` — in practice slightly negative. That is a
property of the definition PURREMB states, not a defect in the kernel, and the tests
assert what the surface actually needs (a vector ranks strictly ahead of every other
direction) rather than a zero that is not there.

Distances are emitted as `xsd:double`, whose canonical lexical form round-trips the
exact bits. A rounded `xsd:decimal` would make two adjacent doubles print alike and
hide exactly the divergence this section is about.

## 2. Ties are broken by content, and the tie-break is free

Rank order is `(distance ASC, row ASC)`. Row numbers are distinct, so no two
candidates ever compare equal: the order is **strict and total**, which is what makes
a bounded top-`k` heap and a full sort agree by construction rather than by
coincidence, and what makes `sort_unstable` canonical rather than merely faster.

The tie-break is meaningful because a PURREMB target set is *sorted and deduplicated
by `TargetId`* when it is built, and a `TargetId` is a domain-separated digest of the
target's canonical identity. Ascending row number is therefore ascending canonical
content order. Two hosts that build artifacts over the same targets in opposite
insertion orders number their rows identically and answer identically — asserted
directly, over two artifacts built forward and backward, down to the serialized JSON
bytes.

Ties in *distance* are real: two identical vectors are genuinely equidistant from
everything, and both are returned. Ties in *rank* are impossible.

## 3. The guard bounds work; it does not license approximation

PURREMB v1 stores derived-index payloads but does not interpret them. An
`IndexGuardView` binds an opaque third-party ANN payload to the exact
`(source, family, space, matrix, projection, prefix)` tuple it was built over and
declares its own loss contract — it is not an algorithm PurRDF can run. So this
surface does not pretend to run one.

**The search is exact.** Every candidate row is scored and the `k` returned are the
true `k` nearest. There is no candidate pruning anywhere. That is what lets
"results ordered correctly under the declared metric" be a property the module is
tested for rather than a property of a tuning parameter, and it is what makes the
engine's row-ceiling pushdown sound here: emission order *is* rank order, so the
first `n` rows are the `n` nearest for every `n ≤ k`.

`KnnGuard` is therefore an admission bound on work, caller-supplied with no default:

* `max_candidates` — the largest space one invocation may scan. A larger space is
  refused **at construction**, not truncated at query time. Truncating would be far
  worse than refusing: a top-`k` computed over an arbitrary prefix of the space is a
  wrong answer that looks exactly like a right one.
* `max_neighbours` — the largest `k` one invocation may request. A larger request is
  refused, naming both numbers, rather than clamped.

Both bounds are inclusive and both are exercised in both directions: a value *at* the
bound is admitted, a value one past it is not. A refusal tested only on the rejecting
side is indistinguishable from a refusal that rejects everything.

`max_neighbours` exists for a second, structural reason. `k` is a per-call argument
that `PropertyFunction::rows_per_invocation` cannot see, so without a configured
ceiling on it the only honest declaration would be the whole space — and the
planner's admission check, which prices a property-function node from its declared
bound against the intermediate-cell ceiling, would refuse calls that produce three
rows. Over-refusal from a cost estimator is a live defect class in this repository,
and a configured bound is what keeps the declaration tight enough to avoid it. There
is a test asserting that a modest cell ceiling **admits** this relation.

What the guard *is* used for, beyond bounding: every derived-index guard in the
artifact that names this target set and vector space is checked against the matrix
actually being scanned. A guard naming this space while pointing at a different
matrix or projection is stale or substituted — a statement about this space that is
no longer true — and it is a construction-time failure rather than silent agreement.

## 4. What a search costs, and the seam change that made it sayable

Before this work the evaluator could price a host relation by exactly two quantities:
how many times it was asked (`property-function-invocation`) and how many rows it
handed back (`property-function-row`). For a *generator* relation neither is where
the cost is. A nearest-neighbour search that examines a million vectors to return the
five closest charges one invocation and five rows: six units of fuel for a million
distance computations, priced identically to a six-row table scan. A caller's budget
was then a bound on the answer's size rather than on the execution, which is the one
thing a governor exists to prevent.

Charge-schedule **v8** adds `property-function-work`, fed by a new provided method on
`PfCursor`:

```rust
fn take_work(&mut self) -> u64 { 0 }
```

It reports internal work performed since the previous call and *takes* it, so
successive reads partition the work rather than re-charging it. The engine reads it
after every pull — the terminating one included, so a cursor that searches lazily on
first `next` and one that searched eagerly in `open` are charged the same total — and
spends one unit per reported unit.

Three properties keep this from being a budget-evasion channel or a fabrication one:

1. The count is **spent**, not merely recorded, so a relation that inflates it
   exhausts its own caller's budget. The incentive points the right way.
2. Under-reporting (and the default, zero) makes a query cheaper than it should be,
   but every other ceiling stays in force unchanged — the invocation point, the row
   point, the intermediate-cell peak, the answer cap, the wall deadline. It can cost
   a caller precision in a receipt; it cannot cost them soundness. No engine-side
   measure can see inside host code to do better.
3. It defaults to zero, so every relation written against the v5 seam charges
   nothing and a budget sized against v7 buys the same execution under v8. The
   regenerated governor corpus shows this directly: **every pinned spend figure is
   unchanged**, and the only diff is one `property-function-work 0` line added to
   each per-charge-point decomposition.

For this surface the unit is **one candidate examined** — one distance computation
against one row of the space. The acceptance test holds the returned row count fixed
at one across two spaces of different sizes: the row point cannot tell them apart,
and the work point reports 3 against 8.

The search runs lazily, on the first `next` rather than in `open`, precisely so that
"no rows were wanted" and "no work was done" are the same statement. A call whose
ceiling is already spent never pulls, so it never searches and never charges.

## 5. Refusal versus empty answer

The dichotomy is explicit, and every entry has a test on both sides.

**Refusals** (the query aborts; contributing zero rows would be indistinguishable
from an honest empty answer):

| condition | why |
|---|---|
| the seed or `k` is free | this relation retrieves neighbours *for* a seed; it cannot enumerate seeds, nor invent how many to return |
| `k` is not an integer literal, or is negative | there is no such request |
| `k` exceeds `max_neighbours` | returning fewer would be a short answer reported as a complete one |
| the space exceeds `max_candidates` | construction-time; see §3 |
| the family declares an extension metric | its parameters are opaque bytes this engine cannot evaluate, so ranking by it would mean ranking by a rule nobody in the process knows |
| a stored vector has zero norm under cosine | PURREMB: *"undefined for a zero-norm operand and hard-fails rather than inventing a score"* |
| a row of the target set has no caller-supplied term | an unnamed row would be searched and be unreportable, so the top-`k` would silently be the top-`k` of a subset |
| a distance leaves the finite binary64 range | an infinity still sorts, and would sort last — a confidently ranked answer computed from a number that overflowed |

**Empty answers** (well-formed questions the data does not answer):

| condition | why |
|---|---|
| the seed is not in the space | exactly as an unmatched triple pattern is. Refusing would abort any query ranging a seed over terms only some of which are embedded — which is the ordinary way this relation is used |
| `k = 0` | a request for zero neighbours, honoured with zero rows and zero work. A boundary a clamp-or-refuse rule gets wrong in both directions |

The zero-norm refusal has an explicit control: the same artifact opens fine under
the two metrics that never divide by a norm, so the refusal is demonstrably about
the *metric* and not about the vector. The `k` datatype check has one too — `xsd:int`,
`xsd:long`, `xsd:unsignedByte` and `xsd:nonNegativeInteger` are all accepted, because
a check that admitted only the literal `xsd:integer` would refuse well-formed queries
while every other test still passed.
