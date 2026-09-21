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
| `sh:sparql` constraint | 90 |
| custom `sh:ask` component | 184 |
| custom `sh:select` component | 105 |
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
96 / 214 / 116 / 194 that held when the decomposition was taken; the reduction
described in the next section has since moved three of them to 90 / 184 / 105,
by removing part of the term-materialization row.

Slices that nest are differenced against each other rather than summed, so no
allocation is counted twice: an extra `apply_shacl_prebinding` contains an extra
`apply_substitutions`, which contains an extra algebra clone and an extra probe
build; and the evaluation root contains every node beneath it.

| component | `sh:sparql` | `sh:ask` | `sh:select` | `sh:expression` |
|---|---:|---:|---:|---:|
| pre-binding rewrite, total | 37 | 110 | 49 | 52 |
| — the algebra clone | 6 | 8 | 6 | 8 |
| — term and string materialization | 11 | 48 | 18 | 30 |
| — pushdown and seed descent | 8 | 12 | 8 | 14 |
| — expression walk and the second ground-term conversion | 12 | 42 | 17 | — |
| evaluation context construction | 1 | 2 | 1 | 2 |
| query-context preparation | 4 | 8 | 4 | 8 |
| **evaluating the freshly minted tree** | **46** | **74** | **51** | **80** |
| — of which the seed `VALUES` node | 6 | 22 | 8 | 16 |
| — of which the seed join and its operands | 26 | 38 | 29 | 32 |
| — of which the `Bgp` | 12 | 4 | 12 | 4 |
| — of which per-node `VarSchema` construction | 6 | — | 6 | 6 |
| SHACL-side remainder | 8 | 20 | 11 | 52 |

Three of the components the earlier reading named are **not on the measured path
at all**, and each was measured at zero rather than assumed away.

The **plan-cache probe** is already free. A hit returns without allocating, which
is what the key-scratch buffer described in the next section was for; doubling
the probe moves no figure. The first bullet of the per-query-execution-setup
list is therefore already discharged.

The **governed prelude** — the algebra re-validation, its dropped IRI `String`
per IRI in the query, the duplicated plan-depth walk, and the relation-identity
receipt — never runs here. `crates/shapes/src/sparql.rs` takes the governed lane
only when an operation installs governors, and validating a focus set does not.
That work is real and worth removing, but it is not part of this term and must
not be credited against these figures.

The **evaluation context** allocates exactly once per execution, which the table
confirms: one for the two surfaces that run a single query per focus node, two
for the two that run a query per value node or per argument tuple. The context is
already minimal; every other field is lazy, borrowed, or `Copy`.

So the dominant term is not *setting up* an execution. It is **evaluating a tree
that was minted for this focus node and will be dropped at the end of it** — the
seed `VALUES` the rewrite just built, the join onto it, and the `VarSchema` every
`Project` and `Bgp` node rebuilds because the node it belongs to is a fresh heap
temporary. Together with the rewrite that produced that tree, those two account
for the whole per-focus-node figure, to within the SHACL-side remainder the last
row names.

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

A fourth has since been taken against the decomposition above. The SHACL rewrite
grounded every pre-bound value **twice**: once into the `GroundTerm` the `VALUES`
seed carries, and again into the constant its expression-position walk writes into
the pattern. The second conversion allocated, once per pre-bound value, per focus
node — and it was unnecessary, because the algebra's `NamedNode` and `Literal` are
both `Arc<str>`-backed, so lifting an already-grounded term into an expression is a
refcount bump. The rewrite now grounds each value once and shares it, which took
the term to 90 / 184 / 105 / 194. The same change gives the rewrite the early
return it was missing for an empty pre-binding list, where it had been paying for a
full walk-and-rebuild of the algebra to change nothing in it. `sh:expression` is
unmoved because it reaches the plain substitution lane, which has no
expression-position walk to feed.

One candidate was declined rather than taken. The single allocation left in
constructing an evaluation context is the expression barrier's shared cell, and it
is load-bearing: workers forked for a parallel evaluation clone it, and the
barrier latches write-once, so reusing a context across focus nodes would let a
trip recorded for one focus node withhold the output of the next. Making it
optional and allocating only when governed would convert a documented, deliberate
ungoverned fallback into a silently dropped truncation. One allocation per query
is the cheaper side of that trade.

What remains belongs to `purrdf-sparql-eval` rather than to the validator, and the
decomposition above says which part. Not the plan-cache probe, which is already
free on a hit, and not the evaluation context, which allocates exactly once. What
is left is the **tree minted for one focus node and dropped at the end of it**: the
rewrite that builds it, the seed `VALUES` node it plants, the join onto that node,
and the `VarSchema` every `Project` and `Bgp` rebuilds because the node it belongs
to is a fresh heap temporary rather than a stable one. Those are one problem, not
five, and the shape that answers it is a reusable execution artifact — the
substituted shape decided once per query and per pre-bound variable name, with only
the values written per focus node.

## The instrument's blind spot

An allocation count cannot see work that was skipped if the skipped work allocates
nothing. That sounds obvious stated plainly and it is not obvious in practice,
because the better this path gets the larger the blind spot grows.

The conformance memo is the worked example. It lets a shape named at two sites
reuse the first site's answer instead of traversing the inner shape twice, and a
test was written to prove it saves work by comparing allocation counts for a named
and an inlined spelling of the same constraint. The test could not fail: it
measured parsing and preparation alongside validation, and the two spellings are
different documents, so the parse alone satisfied the comparison.

Excluding the parse made it read equal — and the tempting conclusion, that the
memo never fires and is a dark feature to delete, was wrong. Instrumenting the
memo directly shows it hitting on that same fixture. What it saves is a Core
traversal, and this work made the conforming Core traversal allocation-free, which
is the result the change-path suite pins. The memo was skipping work the counter
was structurally unable to price.

The fix is to measure it against a constraint whose evaluation has a real
per-focus-node cost, which on this surface means a SPARQL-backed one. So the
general rule: an allocation count proves a fast path only where the slow path
allocates. Anywhere the slow path is already free, a saving has to be demonstrated
some other way, and a comparison that reads equal is evidence about the
instrument before it is evidence about the code.

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
