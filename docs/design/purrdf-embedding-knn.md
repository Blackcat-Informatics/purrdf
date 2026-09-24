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

It would also disagree with the spec. `docs/PURREMB.md` states the arithmetic
contract normatively for the artifact's own folds: *"All intermediate operations are
IEEE-754 binary64, round-to-nearest ties-to-even, performed in the written order
without a fused multiply-add."* The L2 norm is one of those folds (§13.2), so the kNN
kernels do not compute a norm of their own: `knn::norm` is `purrdf_core`'s normative
`norm_fold`, the single copy of that order in the workspace. For the distance sums,
PURREMB §7.4 lets a kernel optimize evaluation as long as it preserves the metric and
the row-number tie-break, so their order is this crate's contract rather than the
format's, and it is pinned just as hard: every dot product and squared Euclidean sum is
`purrdf_core::distance::Exact`.

So there are precisely two residual ways a float kernel can diverge, and both are
closed structurally rather than hoped about:

| hazard | why it would diverge | what closes it |
|---|---|---|
| **reassociation** | float addition is not associative, so a sum depends on the order it was folded in | the order is part of the arithmetic's definition (`Exact`, identifier `binary64-lane16-tree-v1`): sixteen binary64 lanes, lane `l` summing the terms at indices `16·c + l` over the whole sixteen-element chunks in ascending `c`; the pairwise tree `(l, l+8)`, `(l, l+4)`, `(l, l+2)`, `(0, 1)`; then the remaining terms one at a time in ascending index. No accumulator is ever split across rayon workers. Every target and every dispatch path computes that one order |
| **fused multiply-add** | `a * b + c` as a single FMA rounds once where the written form rounds twice | every product is bound to a named local before it is added; Rust never contracts implicitly, no exact path enables `fma`, and PURREMB forbids the fusion |

Fixing the order as sixteen independent lanes is what lets the fold vectorize without
licensing the compiler to reorder anything: the lanes are sixteen separate add chains,
and LLVM packs them into whatever vector width the target has (SSE2 and AVX2 on x86-64,
NEON on aarch64, `f64x2` under wasm `+simd128`). On `x86_64` the exact scan dispatches
once per search between a portable compilation of the body and an AVX2 compilation of
the same body; `purrdf_core`'s tests hold every path the host can execute to a scalar
reference model of the lanes, the tree and the tail, bit for bit. A vector shorter than
one chunk folds exactly as the old ascending sequential loop did, since the tree of
sixteen zero lanes is zero.

The order is asserted, not merely stated. A test folds a vector chosen so the orders
genuinely disagree — `1e16`, `1` and `-1e16` in lanes 0, 1 and 8 against all-ones sums
to `1` under the lane tree and to `0` under both sequential orders — and pins which one
the kernel produces. A test that only checked "the same input gives the same output
twice" would pass on a kernel with no fixed order at all.

The arithmetic also assumes IEEE-754's default environment. A thread with flush-to-zero
or denormals-are-zero set, or another rounding direction, computes different bits from
the same code, so resolving the arithmetic proves the environment by behaviour on every
target — eight binary64 operations whose IEEE-754 results are known constants, after a
read of the control register where one is readable (MXCSR on x86-64, FPCR on aarch64) —
and refuses such a thread with `EvalError::FloatEnvironment`, at space
construction and again at every search, since an invocation may run on another thread.

The metric does not change with the arithmetic. `DistanceMetric` names *what* is
measured, and the family-contract digest is computed from it alone; the arithmetic is
recorded beside it, in the HNSW image header and profile, and in the ranked declaration
of either relation (below).

A second arithmetic sits beside the exact one under its own names:
`Kernel::distance_reassociated` and `Kernel::distance_bounded_reassociated` compute
the same metric under `purrdf_core::distance::Reassociated` (identifier
`binary64-reassociated-v1`). Inside each 64-element block the sum is folded with the
`algebraic_*` operations, so the compiler may reassociate it and contract multiplies
into fused multiply-adds; block sums are combined in ascending order, so the bounded
form's checkpoints are true prefixes of the full value. Its last bits depend on the
target, the build and the dispatch path (SSE2, AVX2+FMA or AVX-512F on x86-64, chosen
at run time; NEON on aarch64; `simd128` or scalar on wasm, as built), and the resolved
handle's evidence says so in words. Each path compiles the body once, out of line, so
every caller on one path gets the same bits for the same pair. The exact entry points
never run it, and it never stands in for them; `knn_wasm_reassociated` executes it on
both wasm32 builds and holds each result to the summation error bound of the exact one.

### The relation is generic over the arithmetic

`EmbeddingKnnRelation<A: Arithmetic = Exact>` carries the law as a type parameter.
`EmbeddingKnnRelation::new(space)` is the exact relation, unchanged and infallible: it
resolves the float environment and a dispatch path per search, and every exact path
returns the same bits. `EmbeddingKnnRelation::new_reassociated(space)` returns
`Result<EmbeddingKnnRelation<Reassociated>, EvalError>`: it resolves the reassociated
dispatch path once, refuses a flushing float environment with the named
`EvalError::FloatEnvironment`, and every search then runs `A`'s batch kernel on that
one path (the environment is still checked per search, on the calling thread). The
scan is the same scan in both, scoring every row, so the reassociated relation omits
nothing the exact one would name; what it can do is order two near-tied rows
differently.

That difference is declared, not hidden. `ranked_declaration` names the law in
`RankedDeclaration::arithmetic` as `RankArithmetic::FloatDistance`
(`binary64-lane16-tree-v1` or `binary64-reassociated-v1`), which
`canonical_description` folds, so the registry's
content fingerprint and every plan id drawn from it bind the arithmetic: an exact and a
reassociated producer over one space are two plans. The reassociated relation also
composes the host's order fidelity with `OrderFidelity::Perturbed`, carrying the
arithmetic's evidence for the resolved path verbatim, through
`composed_order_fidelity`, the single composition the HNSW relation also uses. A fused
answer carries that evidence in `FusionTrailer::fidelities` and reports the stratum as
having no finite score bound. The completeness axis stays as the host declares it.

The tests hold each half to an observation. `exact_scan_matches_kernel_bits` and
`reassociated_scan_matches_reassociated_kernel_bits` compare every distance each
relation emits with `Kernel::distance` and `Kernel::distance_reassociated` on the same
path, bit for bit, over a fixture whose crafted pair cancels exactly under every
unfused order and leaves zero only there, so on a fused path the reassociated relation
is seen to differ from the exact one. `reassociated_relation_declares_perturbed_order`
and its control `exact_relation_names_exact_arithmetic` pin the declaration, and
`fusion_trailer_names_reassociated_kernel` in `purrdf-retrieval` fuses both producers
over one space, with the exact one as the control row.

### The cross-target claim is executed, not argued

Everything above is a reason to *expect* agreement between x86-64 and
`wasm32-unknown-unknown`. Two runs on one target cannot check it: they cannot tell a
kernel that is target-independent from one that is merely self-consistent wherever it
was last compiled. `make wasm` has the same limit in the other direction — it proves
the release crates *build* for wasm32, never that they *answer* the same way there.

So `crates/sparql-eval/tests/knn_wasm_determinism.rs` carries test bodies with two
attributes each: an ordinary `#[test]` natively, a `#[wasm_bindgen_test]` on wasm32.
They run real SPARQL kNN queries over real PURREMB artifacts whose components are
deliberately *not* exactly representable in binary64 — every product, every partial sum
and both norms round — and assert five pinned `xsd:double` lexicals each, in order. One
fixture is six-dimensional, which reaches only the exact fold's sequential tail; the
other is seventy-dimensional, which fills all sixteen lanes and the 64-element bound
checkpoint and leaves a six-element tail, asked under cosine and under squared
Euclidean. `cargo test` executes them on the host; `make wasm-test` compiles them to
wasm32 twice — on the baseline target and with `+simd128`, where the lanes become
`f64x2` operations — and runs both in Node through `wasm-bindgen-test-runner` (which
ships in the wasm-bindgen archive the wasm lane already installs, so there is no second
pin to keep in step). CI's wasm job runs that lane. A target that computes a different last bit renders a different lexical
and fails there, rather than surfacing later as an unexplained reordering.

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

Exactness is checked where it could plausibly stop holding rather than only on the
three-point fixture the rest of the suite uses: a 192-point space is searched at five
values of `k` against a brute-force oracle written out longhand in the test, and both
halves are asserted — the count exactly (`k-1` neighbours reported as complete is the
failure this surface is most likely to hide) and the emitted distances row for row
against the oracle's prefix.

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
