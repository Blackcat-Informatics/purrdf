<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# `purrdf-sparql-governors` — normative vector corpus

The executable half of
[`docs/SPARQL-GOVERNOR-PROFILE.md`](../../docs/SPARQL-GOVERNOR-PROFILE.md). A fuel budget is a number
about *one* charge schedule, so a consumer that pins **(profile id, profile
version, profile digest, corpus digest)** needs a body of evidence saying what
those numbers buy. This corpus is that evidence: running it against a linked
build produces a **receipt** rather than a promise.

First-party, not vendored. Run by
`crates/sparql-conformance/tests/governor_corpus.rs`.

## Identity

The corpus is content-addressed. Its digest is the SHA-256 of its freeze
manifest:

```sh
sha256sum scripts/conformance-frozen/vectors-sparql-governors.sha256
```

That value is pinned in the library as
`purrdf_sparql_eval::GOVERNOR_CORPUS_DIGEST`, which **derives** it from the
freeze manifest and compares — `the_corpus_digest_is_a_well_formed_reproducible_sha256`
in `crates/sparql-eval/src/governor/mod.rs` — so the corpus cannot change
without the constant being re-pinned in the same commit.

The freeze manifest covers every payload byte under this directory
(`scripts/check-corpus-frozen.py`), so a silently edited expectation fails the
build rather than passing it. This `README.md` is deliberately **not** covered —
sidecars are excluded from freeze manifests so editing prose never requires
regenerating a digest.

## Layout

| Path | Role |
|---|---|
| `manifest.tsv` | the case list: inputs, source, governors (numeric ceilings, stop signals, or injected deadline-poll ceilings), band, outcome |
| `transport.tsv` | injected-transport wiring and exchange count, for federated cases |
| `relations.tsv` | scripted property-function wiring, invocation count and pull count, for relation cases |
| `cases/<name>.ttl` | input dataset, in Turtle 1.2 |
| `cases/<name>.rq` | input query |
| `cases/<name>.<endpoint>.srj` | a federated endpoint's pinned SPARQL-results JSON response |
| `expected/<case>.answer` | the certified rows and their certificate class |
| `expected/<case>.spend` | what the execution consumed, per dimension |
| `expected/<case>.metered` | what the same case costs under `QueryGovernors::METERED` |
| `expected/<case>.charges` | *relation and aggregate-isolating cases only* — the metered fuel decomposed per charge point |

A case's data syntax is taken from its **extension**, not from the manifest, so
a fixture cannot be listed under a syntax it is not written in. Endpoint
responses are found from the query fixture's stem (`cases/federated.rq` finds
`cases/federated.a.srj`), so a response cannot be attached to a case it does not
belong to.

Several cases share one `(data, query)` pair — that is the point of the band
matrix: the *only* thing that differs between `fuel-zero`, `fuel-boundary` and
`fuel-over-bound` is the ceiling.

## Three records, not one

Each case pins three facts, and keeping them apart is deliberate:

1. **the outcome discriminant** (`manifest.tsv`'s last column) — completed, or
   which governor stopped it, spelled in the kernel's own
   `TrippedGovernor::label` vocabulary;
2. **the certified rows and their certificate class** (`.answer`) — what the
   caller receives, and what those rows are licensed to be used as
   (`certain` / `at-most` / `unknown`, plus whether they are a positional
   prefix);
3. **what the execution spent** (`.spend`), recorded **independently of the
   rows**.

The third is the one easy to omit and impossible to reconstruct afterwards. A
corpus that pinned only rows could not detect a charge-schedule change that
happened to cut in the same place: the answer would be unchanged, the receipt
would not, and every budget a consumer sized against the old schedule would be
silently wrong while the corpus stayed green.

### A fourth, for the relation lane and the aggregate lane

Fuel is **one** number, and profile v5 put **two** new charge points inside it —
`property-function-invocation` and `property-function-row` — and v6 put **two**
more — `aggregate-invocation` and `aggregate-accumulation`. A band on the
aggregate is satisfied by any schedule whose total happens to land in the same
place, including one that doubled one point's cost and halved the other's. So
every relation case, and the two `aggregate-*-fuel-*` lanes, also carry
`expected/<case>.charges`: the metered run's fuel decomposed **per charge
point**, read off the public per-node ledger (`QueryExplanation::ledger`, taken
under the same `QueryGovernors::METERED` — for a relation case through
`NativeSparqlEngine::explain_query_with_property_functions`, and for an
aggregate case through plain `NativeSparqlEngine::explain_query`, since an
aggregate is ordinary algebra rather than an injected seam). The harness
re-derives it on every run and additionally checks that the thirteen counts add
back up to the `.metered` fuel — a decomposition that did not sum to the
quantity it claims to decompose would be a decomposition of something else.

Note that `.charges` and `relations.tsv` describe **different runs**: the
`.charges` sidecar decomposes the unconstrained METERED measurement, while the
`relations.tsv` invocation/pull counts record the banded (ceiling-constrained)
run. For a zero-fuel band the two legitimately disagree — the METERED
decomposition shows what the seam spends when it runs, the banded row shows the
trip firing before any relation resolves (`0/0`). The aggregate lane has no
equivalent side table — an aggregate fold is ordinary algebra, not a scripted
seam — so its `.charges` and `.answer`/`.spend` pair are read together instead:
both `aggregate-invocation-fuel-*` and `aggregate-accumulation-fuel-*` fold
their one group to completion well inside the boundary ceiling, so the
over-bound member (one fuel unit below it) trips on the trailing
`committed-output-row` charge for the group's own already-computed output row
rather than inside the fold — the row is reported, certified as a positional
prefix of itself, at both boundary and over-bound.

The `aggregate-custom-*` lanes name a registered **custom** aggregate instead
of a built-in, and carry no `.charges` sidecar at all — not by omission, but
because there is no seam to take one through. `explain_query_with_property_functions`
exists for the relation lane; there is no `explain_query_with_aggregates`
counterpart (`NativeSparqlEngine::explain_for` in the evaluator crate takes no
aggregate registry, so a query naming a custom `AGG(<iri>, …)` is refused at
its prepare-time admission check exactly as an unregistered one would be). The
`aggregate-custom-*` cases are therefore ordinary bands — `.answer`/`.spend`/
`.metered` only — and their fuel is compared against the built-in lane's
directly, case for case, rather than through a shared per-charge-point
decomposition.

## The boundary is measured, never guessed

For a numeric ceiling, `expected/<case>.metered` is the consumption vector of
the same case run under `QueryGovernors::METERED`, which engages every counter
and bounds nothing. For an injected deadline, the harness instead counts the
complete run's stop-signal polls with a never-firing deterministic signal. The
band columns are defined against the applicable measurement:

| Band | Ceiling | Must |
|---|---|---|
| `zero` | `0` | trip — a zero ceiling is valid and admits no charged work |
| `boundary` | exactly the metered cost or complete-run poll count | **complete** — the ceiling is inclusive |
| `over-bound` | the applicable measurement minus one | **trip** |

The harness re-measures and re-derives that relation on every run rather than
trusting the numbers in the file, so a charge- or stop-poll-schedule change
cannot leave a stale boundary behind looking authoritative: the measurement
moves, and the manifest's ceiling stops being the boundary it claims to be.

## What determinism is claimed for — and what is not

Charging is deterministic by construction on **fuel**, the **answer cap** and
the **intermediate-cell** peak, and the scratch-arena and remote-request
counters are functions of the same fixed inputs. Every case above pins rows and
a cost on that basis, and `the_corpus_is_reproducible_within_a_run` re-runs the
whole corpus to check it.

A caller-supplied stop signal can be deterministic even when its cause is
`deadline`. The `deadline-injected-*` cases fire solely after a fixed number of
polls, so their zero, boundary and over-bound outcomes, answers and receipts are
pinned in full. They grade the evaluator's poll schedule, not elapsed time.

A **wall deadline** is time-dependent and carries no such claim. The separate
`deadline-zero-budget` smoke case therefore pins exactly what is guaranteed —
that a trip happened, and that it named the deadline — and has no `.answer`,
`.spend` or `.metered` sidecar at all. Publishing bytes for it would be
publishing a promise this engine does not make.

A **cancellation** is not a clock, so cancellation cases are pinned in full.

## Case inventory

### The band matrix — one zero, one boundary, one over-bound per deterministic lane

| Lane | Inputs | Covers |
|---|---|---|
| `fuel-*` | `chain` | abstract execution steps |
| `property-function-fuel-*` | `property-function` | the two property-function charge points **together**: three driving rows, two emitted rows each |
| `property-function-invocation-fuel-*` | `property-function-miss` | the invocation point **alone** — twelve driving rows against a relation that emits nothing, so the band contains twelve invocation charges and zero row charges |
| `property-function-row-fuel-*` | `property-function-single` | the row point **alone** — one driving row against a relation that emits twelve, so the band contains one invocation charge and twelve row charges |
| `property-function-parallel-fuel-*` | `property-function-wide` | the same seam at 1200 driving rows, above the evaluator's fork threshold |
| `aggregate-invocation-fuel-*` | `aggregate-invocation` | the invocation point **alone** — twelve aggregate expressions folding one implicit group whose input matches nothing, so the band contains twelve invocation charges and zero accumulation charges |
| `aggregate-accumulation-fuel-*` | `aggregate-accumulation` | the accumulation point **alone** — one aggregate expression (`SUM`) folding one implicit group of twelve rows, so the band contains one invocation charge and twelve accumulation charges |
| `aggregate-custom-fuel-*` | `aggregate-accumulation` data, `aggregate-custom` query | the SAME group shape as `aggregate-accumulation-fuel-*`, folded through a registered custom aggregate instead of the built-in `SUM` — a direct fuel comparison between the two dispatch paths |
| `aggregate-custom-scratch-bytes-*` | `aggregate-custom-scratch` data, `aggregate-custom` query | 1200 rows in one implicit group, above the evaluator's within-group chunk threshold — the custom accumulator's declared `ScratchBytes` state bound, charged once per live chunk, exercising the ONE dimension no built-in aggregate ever charges at all |

The seven lanes above carry a `boundary` and an `over-bound` member but no
`zero`; the section on the relation seam says why that is the honest shape for
the property-function pair, and the aggregate section below says why the same
shape holds for the aggregate pair.
| `answer-rows-*` | `chain` | what the query commits to its answer sequence |
| `intermediate-cells-*` | `join` | the largest intermediate bag; the zero and over-bound cases are refused at **admission**, because the planner's estimate already exceeds the ceiling |
| `scratch-bytes-*` | `concat` | arena growth, which is independent of every row and cell count |
| `remote-requests-*` | `federated` | requests issued to a federated endpoint |
| `deadline-injected-*` | `chain` | deterministic stop-signal polling, independent of wall time |

### RDF 1.2 — the statement layer is inside the perimeter

| Case | Covers |
|---|---|
| `rdf12-reifier-fuel-boundary` / `-over-bound` | the reifier-layer expansion is charged, so a query over the virtual layer is bounded like any other |
| `rdf12-reifier-answer-boundary` / `-over-bound` | the answer cap over reifier solutions, and the certificate the truncation carries |
| `rdf12-construct-answer-zero` / `-boundary` / `-over-bound` | the answer cap denominating **output statements** over a CONSTRUCTed reification layer: three solutions become six statements, and a cap of five keeps five |

A reification layer is a whole encoding a query can be written around — reifier
and annotation rows live in side tables — so a governor that counted only quads
would report a satisfied ceiling for a query that read, or emitted, an unbounded
number of them.

### The `SERVICE` transport seam

`HttpRequest::stop` can only be honoured by a transport capable of abandoning a
call it is already inside, and nothing forces a host to write one.

| Case | Covers |
|---|---|
| `service-deaf-transport-request-ceiling` | a transport that never reads the signal, bounded anyway by the remote-request ceiling: exactly one exchange is performed |
| `service-deaf-transport-cancel-mid-exchange` | the host cancels while the evaluator is blocked inside a deaf transport |
| `service-honouring-transport-cancel-mid-exchange` | the same, through a transport that does poll the signal |

The last two reach the **same** outcome and the **same** exchange count, which
is the claim: ignoring `stop` degrades to per-request granularity, not to
unboundedness. The one thing that differs is pinned rather than smoothed over —
an abandoned exchange preserves the positional-prefix relation needed for
deterministic resumption, while a completed-then-discarded one withdraws it.

### The property-function relation seam

`PfCursor::next` is handed no signal it is obliged to read, and nothing forces a
host to write a cursor that stops early. The evaluator polls before opening a
cursor and between successive pulls, and every emitted row crosses the per-row
admission point on its way into the bag, so the seam is bounded per **row**,
not merely per invocation, whatever the relation does.

| Case | Covers |
|---|---|
| `property-function-deaf-relation-cancel-mid-invocation` | the host cancels during the relation's second pull, through a cursor that never reads the signal |
| `property-function-cooperating-relation-cancel-mid-invocation` | the same timeline, through a cursor that polls it and abandons the invocation |

The two answers are **byte-identical** — same rows, same certificate — and that
is a stronger statement than the `SERVICE` pair's. A relation's output is a row
stream rather than one atomic exchange, and every row of it crosses the engine's
per-row admission point on the way into the bag; that point is also a bounded
work checkpoint, so a fired signal is observed there whether or not the cursor
ever looked at it. The deaf cursor's extra row is therefore pulled and then
*refused*, never ingested: ignoring the signal buys the relation nothing and
costs the caller nothing.

`relations.tsv` records the invocation and pull counts for the same reason
`transport.tsv` records exchange counts: they are the only observations that
separate "the ceiling prevented the work" from "the work was done and its rows
discarded". Here they are what makes the paragraph above checkable — the deaf
case is pulled exactly once more than it delivers.

#### Separating the two charge points

| Case | Relation | Pinned | Isolates |
|---|---|---|---|
| `property-function-invocation-fuel-boundary` | emits **0** rows | 12 invocations, 12 pulls, 0 rows; fuel 40 | 12 `property-function-invocation` charges, **0** `property-function-row` charges |
| `property-function-invocation-fuel-over-bound` | emits **0** rows | 11 invocations, 11 pulls | one unit below the measurement stops the drive an invocation short — the invocation point is what cut |
| `property-function-row-fuel-boundary` | emits **12** rows | 1 invocation, 13 pulls, 12 rows; fuel 43 | **1** `property-function-invocation` charge, 12 `property-function-row` charges |
| `property-function-row-fuel-over-bound` | emits **12** rows | 1 invocation, 13 pulls | one unit below the measurement trips at the final charge, rows intact |

A cost change at either point moves exactly one of these lanes, which the joint
`property-function-fuel-*` lane cannot tell you. Neither lane has a `zero`
member, deliberately: a zero fuel ceiling trips at the first algebra-node entry,
before any relation is resolved, so a `property-function-*-fuel-zero` case pins
0 invocations and 0 pulls — a band in which the seam never fires, and therefore
one that says nothing about the seam. (The joint lane's `zero` member is kept: it
pins that a zero budget admits *no* charged work at all, which is a statement
about the ceiling rather than about the relation.)

#### The seam between an invocation and its first row

| Case | Ceiling | Pinned |
|---|---|---|
| `property-function-first-row-fuel-ceiling` | `fuel=7` | 1 invocation, **1** pull, 0 rows, `budget-exhausted fuel` |

The narrowest window the seam has: host code was entered, a cursor was opened,
one row was produced — and the budget refused that row at the engine's admission
point, before it reached the bag. The pull count is what separates this from "the
ceiling prevented the call". The ceiling is **authored**, exactly as
`service-deaf-transport-request-ceiling`'s is, because it names a seam rather
than a band; authored is not unchecked, though — the harness re-derives that 7 is
the *unique* such value by re-running the same case at 8 and finding exactly one
more pull and exactly one admitted row.

The certificate is `certain` with `positional-prefix=false`, and that is pinned
rather than smoothed over. The call is driven laterally, so its own emission
order is not the joined output's order: the empty bag bounds the answer from
below (vacuously, but soundly) while licensing no resumption point. A
*standalone* call keeps the licence, and `property_fn_eval`'s own truncation test
pins that half.

#### The parallel drive

| Case | Ceiling | Pinned |
|---|---|---|
| `property-function-parallel-fuel-boundary` | `fuel=4804` | 1200 invocations, 1200 pulls, `complete` |
| `property-function-parallel-fuel-over-bound` | `fuel=4803` | 1199 invocations, `budget-exhausted fuel` |

`property-function-wide` drives 1200 rows through a two-pattern BGP, so the stage
that feeds the call folds above the evaluator's fork threshold. The band **is**
the parity statement, and it needs no test-only seam to arrange one:

* `QueryGovernors::METERED` engages the intermediate-cell counter, and a BGP
  stage carrying a cell ceiling folds through the **sequential** cell-bounded
  driver — a parallel chunk cannot enforce a ceiling whose running total it
  cannot see. So the `.metered` measurement is a sequential run.
* a band case sets **fuel alone**, leaving the cell counter disengaged, so the
  same stage folds through the **parallel** chunk driver.

The boundary therefore completes under a ceiling that is exactly what a
sequential run cost, and the over-bound member — one unit below it — must trip.
Equal totals alone would be a strong statement; the over-bound member makes it
the sharper one, because a ceiling one below the measurement can only be crossed
at all if the parallel fold charged the same items in the same order.

### The aggregate fold

`GROUP BY`'s aggregate fold is the evaluator's third producer whose per-group
work an outside party — the data, not the plan — sizes, alongside the federated
transport and the property-function relation. Unlike those two it needs no
injected seam: an aggregate is ordinary algebra, built-in or a registered
custom aggregate alike, so its two charge points are read straight off
`explain_query`'s ledger rather than off a scripted side table.

#### Separating the two charge points

| Case | Fixture | Pinned |
|---|---|---|
| `aggregate-invocation-fuel-boundary` | `aggregate-invocation` | fuel 53; `.charges`: 12 `aggregate-invocation`, 0 `aggregate-accumulation`; `complete`, 1 row |
| `aggregate-invocation-fuel-over-bound` | `aggregate-invocation` | fuel 52; `budget-exhausted fuel`, 1 row (the group's one row is already fully computed by the time the trailing `committed-output-row` charge trips) |
| `aggregate-accumulation-fuel-boundary` | `aggregate-accumulation` | fuel 45; `.charges`: 1 `aggregate-invocation`, 12 `aggregate-accumulation`; `complete`, 1 row |
| `aggregate-accumulation-fuel-over-bound` | `aggregate-accumulation` | fuel 44; `budget-exhausted fuel`, 1 row |

`aggregate-invocation.rq` names twelve aggregate expressions — `(COUNT(*) AS
?c0)` through `?c11` — over a `WHERE` clause that matches nothing, so the
special "no `GROUP BY`, empty input" rule forms exactly one implicit group with
zero rows: every expression's fold still runs (twelve `aggregate-invocation`
charges) but never folds a single value in (zero `aggregate-accumulation`
charges), and `COUNT(*)`'s empty-group answer is `0`, not unbound, so the query
completes with one row of twelve zeroes.

`aggregate-accumulation.rq` is the mirror image: one `SUM(?val)` over twelve
matching rows and no `GROUP BY`, so one implicit group folds twelve values
(twelve `aggregate-accumulation` charges) through exactly one fold (one
`aggregate-invocation` charge).

A cost change at either point moves exactly one lane's `.charges` decomposition,
which a single joint fuel number cannot tell you. Neither lane carries a `zero`
member, for the same reason the property-function lanes do not: a zero fuel
ceiling trips at the first algebra-node entry, before the `Group` node — let
alone any aggregate expression — is ever reached.

Both fixtures fold their one group to completion well inside the boundary
ceiling: the twelve invocation, or twelve accumulation, charges are a small
fraction of the lane's total cost, which also pays the generic per-node
accounting every algebra node pays. So the over-bound ceiling lands on the
TRAILING `committed-output-row` charge for the group's own already-computed
row rather than inside the fold itself — unlike the property-function
invocation lane, whose relation emits nothing at all and so has no trailing
row-commit charge to land on instead. The two lanes' `.answer` records say so
directly: the same one row, `certain` with `positional-prefix=true`, at both
boundary and over-bound — a caller paging by exactly one more unit of fuel
gets the identical row back, now reported as `complete`.

#### The custom-aggregate path

Every case above names the built-in `SUM`. The custom-aggregate seam —
`AGG(<iri>, …)` resolved against a host-registered `AggregateRegistry`, wired
through `QueryOptions::aggregates` — is a different dispatch path through the
SAME evaluator call site, and until this pair existed the corpus pinned no
case that took it at all.

| Case | Fixture | Pinned |
|---|---|---|
| `aggregate-custom-fuel-boundary` | `aggregate-accumulation` data, `aggregate-custom` query | fuel 45 — identical to `aggregate-accumulation-fuel-boundary`'s; `complete`, 1 row |
| `aggregate-custom-fuel-over-bound` | `aggregate-accumulation` data, `aggregate-custom` query | fuel 44; `budget-exhausted fuel`, 1 row |
| `aggregate-custom-scratch-bytes-boundary` | `aggregate-custom-scratch` data, `aggregate-custom` query | scratch-bytes 2112; `complete`, 1 row |
| `aggregate-custom-scratch-bytes-over-bound` | `aggregate-custom-scratch` data, `aggregate-custom` query | scratch-bytes 2111; `budget-exhausted scratch-bytes`, 1 row |

`aggregate-custom-fuel-*` drives the identical twelve-row, no-`GROUP BY` shape
`aggregate-accumulation-fuel-*` does, through a registered custom aggregate
instead of `SUM` — `AggregateInvocation`/`AggregateAccumulation` are charged
from the ONE call site that dispatches either kind, so the two lanes' fuel
must be, and is, identical: 45 at the boundary, both times.
`crates/sparql-conformance/tests/governor_corpus.rs`'s
`a_custom_aggregate_costs_the_same_fuel_as_a_built_in_over_the_same_group_shape`
checks this directly against the frozen numbers above, not merely against an
in-crate unit test that could drift from what the corpus itself pins.

`aggregate-custom-scratch-bytes-*` is the lane no built-in aggregate can ever
exercise: `ScratchBytes` is charged only against a custom aggregate's declared
[`CustomAggregate::state_bound`](../../crates/sparql-eval/src/agg_fn.rs), and
only once the group is large enough to cross the evaluator's within-group
chunk threshold (1200 rows, one implicit group, well above
`PARALLEL_MIN_ROWS`). The registered fixture aggregate declares a state bound
of 64 bytes; the fold plans 33 chunks for 1200 rows, so the pinned charge is
`64 × 33 = 2112` — the admission charge for the first accumulator plus the
extra 32 the chunked fold actually allocates. That chunk count, and hence this
charge, is a pure function of the row count alone: it does not depend on
`rayon::current_num_threads()`, so the SAME query/data/ceiling triple this
pair pins is admitted or refused identically on every host, which is exactly
the property `purrdf_sparql_eval::parallel::aggregate_chunk_size_for`'s own
doc comment claims and this frozen pair now checks on every run.

### Deadlines

| Case | Covers |
|---|---|
| `deadline-injected-zero` | a deadline signal firing on its first poll stops before output |
| `deadline-injected-boundary` | a ceiling equal to the complete run's measured poll count completes |
| `deadline-injected-over-bound` | one poll fewer trips, exposing a moved or missing terminal poll |
| `deadline-zero-budget` | a zero-budget wall deadline trips and names the deadline — and nothing further is pinned |

## Regenerating

```sh
PURRDF_UPDATE_GOVERNOR_CORPUS=1 \
  cargo test -p purrdf-sparql-conformance --test governor_corpus
python3 scripts/check-corpus-frozen.py --update
# then re-pin GOVERNOR_CORPUS_DIGEST from the sha256sum above
```

Deliberately three steps, not one. A regeneration that moved a charge is a
`GOVERNOR_PROFILE_VERSION` change, and the friction is what keeps an accidental
refresh from being mistaken for a no-op.
