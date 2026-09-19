<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# The SHACL change path: what allocates, what does not, and where the rest of it lives

Validation in PurRDF sits at projection time, so it runs when data **changes**
rather than when data is queried. That makes its allocation profile multiply with
update traffic instead of with process starts, and it is why the change path —
`PreparedValidator::validate_focus_nodes` and its id-native twin
`validate_focus_node_ids` — is measured to a closed form rather than described.

This document records what that closed form is, how it was established, and the
one part of it that is **not** zero, because a reader who only saw the headline
would reasonably assume the whole surface is constant and it is not.

Allocation counts, not timings. They are deterministic and reproducible, so they
are a valid optimization signal without a quiet benchmark host, and every figure
below is regenerable by the command printed beside it.

## What is constant

Validating a **conforming** focus set costs a fixed number of allocations
regardless of how many focus nodes were supplied, for every constraint kind and
path form whose evaluation stays inside `purrdf-shapes`. The invariant is
asserted as an exact equality with no tolerance — `delta(2N) == delta(N)` — across
the constraint and path cases:

```
cargo test -p purrdf-shapes --test change_path_alloc
```

A violating population scales with the results it produces, not with the focus
count, which is the property that stops a cheap row from being a row that quietly
stopped validating.

Two residuals are excluded from those exact equalities and both are third-party:
`rayon`'s global injector queue, whose block boundary is periodic rather than
per-item, and the `regex` crate's thread-sharded cache pool. Neither is a
per-focus-node term. Each is named where it is excluded, and a compensating
single-threaded companion measures a slope of exactly zero for the same
constraint.

Coverage is not a matter of anyone remembering to add a case. A wildcard-free
match over the lowered constraint enum makes a newly added variant fail to
compile, and a companion check requires every kind to be answered by a measured
case, a named exclusion, or a documented entry pointing at the file that measures
it. A kind cannot be added without being measured or explicitly accounted for.

## What is not constant

The SPARQL-bearing surfaces carry a genuine, first-party per-focus-node term. The
closed form is `CHANGE_PATH_CONSTANT + per_focus_node * N`, and the file that
measures it asserts that form rather than the zero-growth one:

```
cargo test -p purrdf-shapes --test sparql_path_alloc -- --nocapture
```

| surface | allocations per focus node |
|---|---:|
| `sh:sparql` constraint | 96 |
| custom `sh:ask` component | 214 |
| custom `sh:select` component | 116 |
| `sh:expression` function call | 194 |

The term is flat in the size of the data graph, so it is the price of executing a
query once per focus node, not a scan. That distinction matters: a scan would be
a planning defect, and one was found and fixed on this path — a pre-bound variable
was being bound by joining a single-row `VALUES` onto the core pattern without
ever touching the triple patterns, so a pattern with a bound subject enumerated
every quad with that predicate and discarded all but one subject's.

## Where the term actually goes

The obvious hypothesis was that this cost is dominated by cloning the prepared
algebra once per focus node and by materializing pre-bound terms as strings. It
is measurably not.

The decomposition below was taken by inserting **one extra, discarded copy** of
each slice into the live path and differencing against the baseline, with the
evaluated algebra held byte-identical. Truncation was rejected as a method:
removing a slice changes which query runs, so the evaluation term moves with it
and the slopes cannot be differenced. Figures are against the baseline of
100 / 218 / 120 / 200 that held before the contained reductions described in the
next section.

| component | `sh:sparql` | `sh:ask` | `sh:select` | `sh:expression` |
|---|---:|---:|---:|---:|
| pre-binding rewrite, total | 39 | 112 | 51 | 56 |
| — the algebra clone | 6 | 8 | 6 | 8 |
| — term and string materialization | 11 | 48 | 18 | 30 |
| — pushdown, seed and expression walks | 22 | 56 | 27 | 18 |
| **evaluator per-query execution setup** | **53** | **86** | **58** | **92** |
| SHACL-side remainder | 8 | 20 | 11 | 52 |

The algebra clone is four to six percent of the term. The string round trip is
eleven to twenty-two percent. The largest single component — thirty-nine to
fifty-three percent, and larger than the entire pre-binding rewrite on three of
the four surfaces — is the evaluator's per-query execution setup: the plan-cache
key, the evaluation context, the solution schema, and what a query allocates
simply by running once.

There is an irony worth recording. The pre-binding path round-trips an identity
through a string and back to the same identity: a term id becomes an owned term
value, becomes a ground term in the algebra, is rewritten into the pattern, and
is then resolved by the compiler back to the same term id it started as. That
round trip is real and it is wasteful. It is also a minority of the cost, which
is why removing it was not the fix it appeared to be.

## Why an id-native pre-binding does not reach zero

Carrying an identity through the pre-binding interface, taken to its theoretical
maximum, removes the whole pre-binding row. That leaves 61, 106, 69 and 144
allocations per focus node. The goal of no growth term is unreachable from that
path — not narrowly, but by a factor of one and a half to two and a half on the
residual alone, because nothing in the pre-binding path can reach the evaluator's
per-execution cost.

The contained version of that change also does not exist. The pushdown's boundary
is a function of the query and the **names** of the pre-bound variables, both
constant across focus nodes, so the shape could in principle be decided once and
the values bound later. But "decided once, bound later" needs a placeholder in the
algebra, and the algebra's ground term is deliberately dataset-independent —
correctly so, since it is the cell type of a `VALUES` block and has 181 references
across four crates. Every remaining spelling either re-derives the
right-arm-of-`OPTIONAL`-and-`MINUS` boundary in two further places, which is the
change most likely to introduce a silent soundness difference, or needs a
sentinel-marking scheme plus a second plan cache plus a side table threaded
through four subsystems — for a ceiling that is still 61 / 106 / 69 / 144.

## What was taken, and what is left

Three contained reductions were taken, each isolated by reverting one change at a
time and re-measuring, and they are exactly additive:

- the BGP compiler minted an owned term value for the reifies IRI on every call,
  purely to hand a lookup a borrow; it is a constant and now lives in a once-cell;
- the plan cache built a fresh key vector on every prepare, although its map is
  keyed by a shared slice and its probe was already generic over a borrow, so only
  the construction allocated; it now builds into a scratch buffer and probes with
  a slice, and only a miss allocates, which is right because a miss is already
  parsing and planning;
- the substitution rewrite walked the core pattern twice, once to push constants
  into the triple patterns and once to join the seed; the two walks are now one,
  with the seed built around the pushdown's result so it still cannot come between
  the core and the peephole that is stated about it.

One candidate was declined rather than taken. The single allocation left in
constructing an evaluation context is the expression barrier's shared cell, and it
is load-bearing: workers forked for a parallel evaluation clone it, and the
barrier latches write-once, so reusing a context across focus nodes would let a
trip recorded for one focus node withhold the output of the next. Making it
optional and allocating only when governed would convert a documented, deliberate
ungoverned fallback into a silently dropped truncation. One allocation per query
is the cheaper side of that trade.

What remains is the evaluator's per-query execution setup, which is where the
measurement says the value now is. It is a larger piece of work than the pre-binding
path it was mistaken for, and it belongs to `purrdf-sparql-eval` rather than to the
validator: the plan-cache probe, the evaluation context, the variable schema, the
solution sequence, and the intermediates a query allocates by running once.

## The rule that produced all of this

Every claim above obliges a test, and every number above is the reading of one.
Where a claim turned out to be false it was the claim that changed, not the
measurement: the bounded-allocation guarantee on the id-native entry points once
named only the two third-party residuals and closed by asserting the guarantee was
about this crate's own traffic, while the suite beside it had been asserting a
first-party per-focus-node term all along. A benchmark in this workspace reports
and never asserts a speedup, so an allocation count in a test is the only
optimization signal that can fail a build — which is precisely why the qualified
claim, not the flattering one, is the one that ships.
