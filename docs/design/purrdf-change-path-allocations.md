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
| `sh:sparql` constraint | 52 |
| custom `sh:ask` component | 118 |
| custom `sh:select` component | 65 |
| `sh:expression` function call | 127 |

That table is the UNGOVERNED lane. The same file now also pins the GOVERNED one —
the lane an incremental host with a budget runs, reached through
`purrdf_shapes::engine::validate_change_with_governors`, which is a different
entry in the evaluator (`execute_governed_in_operation`, with its relation-identity
receipt and admission estimate per run, on the trip-aware channel) reading a
delta-backed view whose pattern probe is type-erased:

| surface | allocations per focus node, governed |
|---|---:|
| `sh:sparql` constraint, governed | 69 |
| custom `sh:ask` component, governed | 142 |
| custom `sh:select` component, governed | 82 |
| `sh:expression` function call, governed | 152 |

Until that second table existed the governed lane's per-focus-node term was
measured by nothing at all, so a regression in it was invisible to every pin in
the workspace. It is asserted in the same closed form and with the same absence of
tolerance, at three focus populations rather than two — three, because at two
sizes a per-doubling residual cannot be told apart from a one-off at the larger
one, and this path has one.

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
96 / 214 / 116 / 194 that held when the decomposition was taken; the reductions
described in the next section moved them to 70 / 144 / 82 / 164, by emptying part
of the term-materialization row and most of the rebuild cost inside the pushdown,
seed and expression-walk rows. Two further reductions, described at the end of the
same section, have since moved them again to 52 / 118 / 65 / 127.

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
receipt — is no longer a per-focus-node term on EITHER lane, and the two halves
of that sentence were settled separately.

`crates/shapes/src/sparql.rs` takes the governed lane only when an operation
installs governors, which is why the ungoverned figures above never carried it.
But an operation that installs governors is not hypothetical —
`validate_change_with_governors` is a production entry that does — so "not part of
this term" was a statement about which table the work landed in, not about whether
it ran. The governed table above is the measurement, and it shows the prelude is
not there either: a prepared execution re-admits only its REGISTRIES per run
(`check_prepared_registries_unchanged`), and the walks that depend on nothing but
the plan are paid once at preparation.

The two remaining pieces were removed rather than relocated, and the lane they
were removed from is the one a bare `&PreparedQuery` reaches —
`NativeSparqlEngine::query_prepared` and its governed twin, which must re-admit
per call because the same admitted plan may legitimately be handed to calls naming
different registries. The IRI re-validation stopped owning a copy of each IRI
(`purrdf_iri::is_absolute` runs the identical grammar over a borrow), and the
duplicated plan-depth walk was deleted rather than relocated: the same walk ran a
second time inside the evaluation that followed it, over the SUBSTITUTED tree,
which is the admitted plan plus a seed `VALUES` and the join onto it and so is the
copy that must stay. Exactly one depth walk per evaluation remains, refusing the
same trees with the same diagnostic — once instead of twice.

The relocation was TRIED first and is recorded here because it is the more
attractive of the two and it is wrong. Establishing the nesting fact at admission
reads as obviously right — admission is once, and the plan is immutable afterwards
— and it silently moved an acceptance boundary the crate states and tests:
preparation accepts the PARSER's envelope, and the evaluator's narrower depth limit
belongs to execution. A flat `OPTIONAL {} OPTIONAL {} …` spine sits inside the
parser's budget at two brace levels and lowers to a `LeftJoin` chain far past the
evaluator's limit, so preparing it must succeed and evaluating it must return a
typed diagnostic. With the guard at admission, preparing it became an error — as
did preparing one of this workspace's own generated corpus queries. Every test in
the module holding the changed code still passed. Measured
on `crates/sparql-eval/tests/prepared_execution.rs`'s `query_prepared` pin over a
three-IRI query: **65 → 62 → 59** allocations per call, three for the IRI copies
and three for the traversal stack, each half isolated by reverting one change and
re-measuring. Neither moves the tables above, and that is the honest reading
rather than a disappointment: the SHACL change path already reaches the evaluator
through a handle, and a handle does not re-admit.

The **evaluation context** allocates exactly once per execution, which the table
confirms: one for the two surfaces that run a single query per focus node, two
for the two that run a query per value node or per argument tuple. The context is
already minimal; every other field is lazy, borrowed, or `Copy`.

So the dominant term is not *setting up* an execution. It is **evaluating a tree
that was minted for this focus node and will be dropped at the end of it** — the
seed `VALUES` the rewrite just built, the join onto it, and the `VarSchema` every
`Project` and `Bgp` node rebuilds because the node it belongs to is a fresh heap
temporary — and it is larger than the entire pre-binding rewrite on three of the
four surfaces. Together with the rewrite that produced that tree, those two
account for the whole per-focus-node figure, to within the SHACL-side remainder
the last row names.

There is an irony worth recording. The pre-binding path round-trips an identity
through a string and back to the same identity: a term id becomes an owned term
value, becomes a ground term in the algebra, is rewritten into the pattern, and
is then resolved by the compiler back to the same term id it started as. That
round trip is real and it is wasteful. It is also a minority of the cost, which
is why removing it was not the fix it appeared to be.

## Why an id-native pre-binding does not reach zero

Carrying an identity through the pre-binding interface, taken to its theoretical
maximum, removes the whole pre-binding row. That leaves 59, 104, 67 and 142
allocations per focus node. The goal of no growth term is unreachable from that
path — not narrowly, but by a factor of one and a half to two and a half on the
residual alone, because nothing in the pre-binding path, taken alone, can reach
the evaluator's per-execution cost. That bounds identity-carrying pre-binding
specifically; it does not bound every mechanism that could stand in for it, and
the paragraph below is where a different one reached past it.

At the time this section was first written, the contained version of that change
did not exist either. The pushdown's boundary is a function of the query and the
**names** of the pre-bound variables, both constant across focus nodes, so the
shape could in principle be decided once and the values bound later. But "decided
once, bound later" needs a placeholder in the algebra, and the algebra's ground
term is deliberately dataset-independent — correctly so, since it is the cell type
of a `VALUES` block and has 181 references across four crates. The two spellings
then known both re-derive where the boundary sits: either the
right-arm-of-`OPTIONAL`-and-`MINUS` rule in two further places, which is the
change most likely to introduce a silent soundness difference, or a
sentinel-marking scheme plus a second plan cache plus a side table threaded
through four subsystems — for a ceiling that is still 59 / 104 / 67 / 142.

A third spelling avoids re-deriving the boundary at all, which is why it carries
neither hazard the paragraph above named. `crates/sparql-eval/src/prebind_memo.rs`
is that spelling, and the eighth reduction below is its numbers: it decides the
substituted shape once by **observing** the real rewrite rather than
reimplementing its rule, and never believes the observation — every hit is
replayed through the ordinary rewrite in debug builds and compared node for node,
so a memo that would ever answer differently from the rewrite it stands in for is
discarded, and that run takes the ordinary path instead of a silently wrong one.
Because it retains the whole substituted tree rather than only the pre-binding
row, it is not bounded by this section's ceiling at all — that ceiling assumed the
evaluator still mints a fresh tree every focus node, which a retained tree does
not do. `sh:sparql`, `sh:select` and `sh:expression` now sit at or below
59 / 104 / 67 / 142, one of the three exactly at it, and only `sh:ask` remains
above, by 18 allocations, which the next section's numbers explain.

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

Six more have since been taken against the decomposition above.

The first: the SHACL rewrite grounded every pre-bound value **twice**: once into the `GroundTerm` the `VALUES`
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

The second: **the rewrite rebuilt the algebra in order to rewrite it.** Both
halves were `GraphPattern -> GraphPattern` by value, and every recursive child of
a `GraphPattern` is a `Box`, so moving a child out and putting the rewritten node
back allocated a fresh `Box` for every node the walk visited — the pushdown for
each node on its path, and the expression walk for every node in the query. Both
now take `&mut` and mutate a clone of the prepared plan in place; a visited node
costs nothing. The algebra-side primitives converted with them, and the by-value
`map_core_pattern` and `substitute_variable` are now thin wrappers over the
in-place forms rather than second implementations of the same descent — two walks
carrying the same rule about where the core begins would mean a pattern variant
added later has to be handled twice, and a walk that missed it would still compile
and still return an answer. That took the term to 83 / 172 / 98 / 190.

The third: the **empty variable schema** is a constant, and it is reached on every
execution — an empty BGP is the identity table `Z`, and the unit sequence standing
for it seeds every group pattern that starts from nothing. It is now a
process-wide shared `Arc`, which is sound because nothing in the crate reaches a
`VarSchema` through `Arc::make_mut` or `Arc::get_mut`. That took the term to
83 / 170 / 98 / 188.

The fourth: a `VarSchema` carried a hash index for what is almost always a
handful of columns, and every `Project`, `Values` and `Bgp` node builds one on
every execution. Below nine columns the ordinal is now found by scanning the
column vector, and a `DetHashMap` that never takes an insert never allocates a
table — the same argument `crate::substitute`'s `ExprSubs` already makes one layer
up. Above the threshold the index is still built and is still authoritative, so a
wide schema keeps its O(1) lookup; nothing reads a column ordinal inside a per-row
loop, so the scan is not on a quadratic path. That took the term to
78 / 164 / 92 / 180.

The clone itself stays. The substituted query must not poison the shared
plan-cache entry, so each execution rewrites its own copy; what is gone is
rebuilding that copy a second time in order to change it.

The fifth: a `Variable` for a pre-binding NAME was rebuilt from a borrow on every
focus node. `Variable::new` takes `impl Into<String>`, so building one from a `&str`
allocates a `String` and then the `Arc<str>` it converts into — twice per pre-bound
variable, per focus node, for shape text that is the same on every focus node in the
run. `Prebinding::variable` is a `&str` precisely to avoid owning that text, and the
rewrite was re-owning it one layer down. Names are now interned per worker in a table
keyed by `Box<str>` and probed by `&str`, so a hit hashes the borrowed name and
allocates nothing and only a first sighting owns a copy — the same borrowed-probe
shape the plan cache's key buffer uses. That took the term to 74 / 148 / 86 / 172.

The sixth: a `VarSchema` is a pure function of its variable list, and that list is
a plan constant — but the node it belongs to is a fresh heap temporary on every
execution, so every `Project` and `VALUES` rebuilt the layout from scratch each
time. A memo keyed by node address would read an address a later allocation can
reuse, which is the hazard this crate documents for its other node-keyed caches.
Keying by CONTENT sidesteps it: what the layout for a given column list is has the
same answer forever, whoever asks and from whichever node. Layouts are now interned
per worker in a `hashbrown::HashTable` probed by hashing the caller's borrowed
slice, so a hit is an `Arc` clone and owning a key to look one up — the allocation
this removes — never happens. That took the term to 70 / 144 / 82 / 164.

The stored layout is compared against the column list it was BUILT FROM, not
against its own columns, because `from_vars` drops later duplicates: `SELECT ?s ?s`
is legal, the crate has a test for it, and a one-column layout does not equal the
two-column request it answered. A debug assertion written on the assumption that
projected lists are duplicate-free found that test within one run.

The seventh: a prepared execution stopped reaching the evaluator through query
**text**. Two algebra soundness walks (`validate`, a graph-pattern depth check)
and a replanning walk against the run's registries were keyed to the *run* rather
than to the *plan*; a prepared execution — parsed and admitted once, checked out
of a per-worker table keyed by query text and parameter list — now pays them once
at preparation, the replanning walk returns immediately once neither side supplies
a registry, and `$PATH` substitution now returns `Cow` so two pass-through cases —
a node shape, and a property shape whose query does not mention the placeholder —
stop copying the whole query text per focus node. `sh:ask` is unchanged by this
one: its validator had already hoisted its pre-binding list out of the value-node
loop, so what this removed there is matched by what it added, a parameter-name
list per focus node. The other three move. That took the term to
68 / 144 / 81 / 151.

The eighth: a prepared execution still cloned the admitted algebra and rewrote it
on every run — the whole pre-binding rewrite this section has been reducing, paid
in full on every focus node regardless, because caching the PLAN is not caching
the SUBSTITUTED plan. `crates/sparql-eval/src/prebind_memo.rs` is the artifact
named in the section above, "Why an id-native pre-binding does not reach zero": a
`PrebindMemo` holds the rewritten query and the positions in it a caller-supplied
value occupies, so a run whose values have shapes the memo has already seen
writes those values into a tree built once, at a handful of refcount bumps,
instead of cloning and rewriting the admitted algebra again. It is never believed
on trust — built by observing which cells a handful of differencing rewrites
actually move, and every hit checked, in debug builds, by replaying the run
through the ordinary rewrite and comparing the two trees node for node, so a memo
that would ever answer differently from the rewrite it stands in for is discarded
and that run takes the ordinary path instead. That took the term to
54 / 122 / 67 / 131.

The ninth: the SCRATCH interner grew from empty on every focus node. A prepared
execution's handle outlives its runs, so the tables a run grows can be kept and
EMPTIED between runs rather than dropped and regrown — capacity retained, contents
not. That is worth exactly two allocations per evaluation context (a `Vec` and a
`HashTable`, each growing once), so it is two on the surfaces that run one query
per focus node and four on the two that run one per value node or per argument
tuple: 52 / 118 / 65 / 127 ungoverned, 69 / 142 / 82 / 152 governed, which is
where both stand now.

Emptying is the entire safety argument, and it is compiler-enforced. A
`SolutionTerm::Computed` id is an index into that interner, so a table carried
forward uncleared answers one focus node's id with another focus node's value —
a silently wrong answer, not a visible failure. `ScratchInterner::clear` is
therefore written as a destructuring `let` naming every field with no rest
pattern, so a field added later and not cleared does not compile; the field that
argument was actually needed for is `minted_bytes`, which looks like "only a
counter" and is in fact what the `ScratchBytes` governor charges the difference of,
so carrying it would move where a ceiling trips. Retained capacity is retained
memory, so the workspace charges it to `PlanMemoryObserver` the way the per-worker
interners do, and recharges only when it moves.

The seven OTHER lazy per-evaluation tables on the evaluation context — the
`BNODE(strExpr)` memo, three `EXISTS` caches, the regex cache, the constant-atom
cache and the XSD parse cache — are deliberately NOT retained, and that is a
measurement rather than a preference. Each is a `HashMap::default()`, which builds
no table until its first insert, and none of them takes an insert on a query that
does not use the feature it memoizes: pre-reserving all seven ADDS exactly seven
allocations per evaluation context to the prepared-execution pin and ten to the
per-focus-node figures above — seven for the tables and three more where a forked
`FILTER` worker clones the three `EXISTS` caches, which is free while they are
empty and is one allocation each once they are not. There is no capacity there to
keep. A later change that gives one of them a per-run cost belongs in the
workspace, and the exhaustive clear is what makes adding it without clearing it a
compile error.

One candidate was declined rather than taken. The single allocation left in
constructing an evaluation context is the expression barrier's shared cell, and it
is load-bearing: workers forked for a parallel evaluation clone it, and the
barrier latches write-once, so reusing a context across focus nodes would let a
trip recorded for one focus node withhold the output of the next. Making it
optional and allocating only when governed would convert a documented, deliberate
ungoverned fallback into a silently dropped truncation. One allocation per query
is the cheaper side of that trade.

What was left after the sixth belonged to `purrdf-sparql-eval` rather than to the
validator, and the decomposition above said which part. Not the plan-cache probe,
already free on a hit, and not the evaluation context, which allocates exactly
once. What remained was the **tree minted for one focus node and dropped at the
end of it**: the rewrite that builds it, the seed `VALUES` node it plants, the
join onto that node, and the `VarSchema` every `Project` and `Bgp` rebuilds
because the node it belongs to is a fresh heap temporary rather than a stable one.
Those were one problem, not five, and the eighth reduction above is the shape
that answers it: a reusable execution artifact, the substituted shape decided once
per query and per pre-bound variable-name set, with only the values written per
focus node.

## The sibling walk does not carry the same scan, and its leaves stay untouched

The scan this path fixed — a pre-bound variable bound only by a `VALUES` join, so a
leaf with a bound position enumerated every statement with that predicate — has an
obvious suspect one module over. `crate::expr`'s per-row `Replace` walk, which an
expression-correlated `EXISTS` and a `LATERAL` right arm both use, clones its `Bgp`
patterns UNCHANGED and joins a single-row `VALUES` onto them, and `eval_join`
evaluates both of its operands in full before hash-joining. That is the same shape,
at row scale instead of focus-node scale.

It was measured rather than read, and it is not there. Two slopes are needed,
because one alone cannot tell a per-row pass over the graph from a once-per-query
one:

| shape | cost slope in data-graph size | marginal cost per additional outer row |
|---|---:|---:|
| correlated `FILTER EXISTS` | 25x over a 32x graph, ONCE | 6.4 |
| correlated `LATERAL` right arm | 1.1x over a 32x graph | 41 |

`EXISTS` answers from a memoized probe: it evaluates its inner once and answers
every outer row from that, so the pass over the graph is amortized across rows and
each additional row costs six allocations. Measured against a single outer row the
one-time pass is indistinguishable from a per-row scan, which is a fact about that
fixture and not about the code. Neither shape re-scans per row.

Pushing constants into the leaf there was tried anyway, since it would have made
`LATERAL`'s per-row cost exactly flat in the graph. It is wrong, and the crate's
own tests say so in words: *"the leaf itself must be untouched — Values Insertion
joins a row onto it rather than rewriting its terms."* The reason is SPARQL
§18.5. Both sides of a `MINUS` must keep a shared variable as a real schema column
for the domain-disjointness test to read the truth, and rewriting a term out of a
leaf takes that column away. Values Insertion is also total over every RDF 1.2 term
kind, including blank nodes, which a term rewrite is not. The two walks diverge
here deliberately; `crate::enf`'s "The SHACL pre-binding fork" is the statement of
it, and this is the measurement that says the divergence costs nothing worth
reclaiming.

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
