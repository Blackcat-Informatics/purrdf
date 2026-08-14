<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# `purrdf-sparql-governors` — SPARQL Execution Governor Profile

**Profile identifier:** `purrdf-sparql-governors` &nbsp;·&nbsp; **Profile version:** 6
&nbsp;·&nbsp; **Editor:** Patrick Audley, Blackcat Informatics® Inc.

Every value in this document is readable from the library rather than only from
here — `purrdf_sparql_eval::GOVERNOR_PROFILE_ID`, `GOVERNOR_PROFILE_VERSION`,
`GOVERNOR_PROFILE_DIGEST`, `GOVERNOR_CORPUS_DIGEST`, `CHARGE_SCHEDULE`,
`STOP_POLL_FUEL` — so a consumer can assert that the build it linked is the
profile it pinned rather than trusting prose.

## Abstract

This profile specifies what a **caller-supplied execution ceiling** costs, when it
trips, which ceiling wins when several trip at once, and what a caller may do with
the rows an execution had already reached when it did. It is the contract behind
`NativeSparqlEngine::query_governed`, `update_governed` and their view-taking
siblings, and behind the governor surfaces exposed to Python, JavaScript/wasm, C and
the `purrdf` CLI.

A budget is a number about *one* charge schedule. Pinning the identity of that
schedule is what makes the number mean the same thing to two parties, which is why
this profile is versioned and content-digested at all.

All example IRIs use `example.org`. PurRDF mints no vocabulary IRIs; the profile
identifier below is a bare token precisely so nothing can dereference it, assert
with it, or mistake it for an ontology.

## 1. Determinism: what is claimed, and what is not

Stated first, because it is the fact a consumer is most likely to over-read.

| Governed quantity | Deterministic? |
|---|---|
| `fuel` | **yes** |
| `answer-rows` (the answer cap) | **yes** |
| `intermediate-cells` (the cardinality / cell ceiling) | **yes** |
| `scratch-bytes` | yes — a function of the same fixed inputs |
| `remote-requests` | yes — a function of the same fixed inputs |
| `udf-depth` | yes — a fixed build ceiling, not caller-settable |
| **the wall deadline** | **NO. No determinism is claimed for it.** |

"Deterministic" here means: the same query over the same data under the same
ceilings consumes exactly the same amount and trips at exactly the same point,
independent of worker count, thread scheduling, hash seed, machine and build
profile. Parallel evaluation preserves it because charges are accumulated
chunk-locally — one record per input row, no atomics, nothing shared — and folded
in **source-item order**, so the reported trip point is a pure function of
`(query, data, ceilings)`. A chunk-granular fold would have made it a function of
`rayon::current_num_threads()` instead, which is the failure that fold exists to
prevent.

A **wall deadline is excluded by name**. Where elapsed time runs out is a property
of the machine, not of the query, so no trip point, row count or cost is
reproducible for it and none is published. The normative corpus of §11 separates
that reader from the stop-poll schedule: three injected-deadline cases use a
deterministic poll count and pin zero, boundary and over-bound receipts in full;
the wall-deadline smoke case pins only that a trip happened and named the deadline.
It has no expected rows or cost because a time-dependent trip point has none to
publish honestly.

A **cancellation is not a clock**, so cancellation cases are pinned in full.

## 2. A governor changes an outcome, never an answer

This is the invariant the whole profile rests on.

> A ceiling never changes the query's complete answer. It changes which certified
> interval, if any, can be returned before that answer is complete.

A ceiling therefore is not semantic optionality and is not a Cargo feature by
another name. It decides whether the caller receives the complete answer or one of
three typed interval outcomes: `Certain` contains only proven answers, `AtMost`
contains every answer but may also contain candidates not yet excluded, and
`Unknown` exposes no rows. Different ceilings may expose different intervals; none
changes the complete answer or labels an uncertified row as one.

Two structural rules keep that from decaying into "sometimes you get a wrong
answer":

* **A truncated result is never returned in the shape of a complete one.** A
  governed call returns `GovernedOutcome`, whose `BudgetExhausted` arm is a
  distinct type carrying its certificate. `SparqlResult` and `CompleteSparqlResult`
  still mean "this is the answer".
* **A genuine evaluation error outranks every governor** (§6). Reporting an
  exhausted budget for a query that could not have been answered at all would hand
  a caller a partial answer to a question that has none.

An **ungoverned** query and `QueryGovernors::UNBOUNDED` take a direct evaluator path
before any governor stop probe, counter, certificate ledger, or output re-wrap. The
report-only `governed_eval` Criterion benchmark measures that path against the
`UNBOUNDED` carrier; no timing equivalence is asserted. `QueryGovernors::METERED` is
the opposite trade — every counter engaged at a ceiling nothing can reach — and is
the intended way to *size* a budget: run under it, read the evidence off the
completed result, choose the real ceilings from the numbers.

## 3. The governed dimensions

`purrdf_core::ResourceDimension`, in declaration order. The labels are a pinned
contract: the frozen corpus records them as case discriminants, so renaming one is
a breaking change rather than a cosmetic edit.

| Label | Bounds | Accumulation | Caller-settable |
|---|---|---|---|
| `fuel` | abstract execution steps, priced by §4 | sum | yes |
| `answer-rows` | what the query commits to its **answer sequence** | sum | yes |
| `intermediate-cells` | the largest single intermediate bag, in `rows × columns` | **max** | yes |
| `scratch-bytes` | bytes minted into the per-query scratch arena | sum | yes |
| `remote-requests` | requests issued to a federated endpoint | sum | yes |
| `udf-depth` | nesting depth of user-defined function invocation | **max** | **no** — fixed at 32 |
| `pages` | distinct pages a demand-paging operation may admit | sum | via `PagedQueryLimits` |
| `bytes` | provider-reported byte charges of admitted pages | sum | via `PagedQueryLimits` |

Two dimensions are maxima rather than totals, for the same reason in two shapes.
`intermediate-cells` bounds how large *one* operator instance's bag may get —
summing it would make a long cheap query indistinguishable from a single
catastrophic cross product, which is the failure it exists to stop. `udf-depth` is
a *depth*: a query calling a function a thousand times at depth one has reached
depth one, not depth one thousand.

`udf-depth` is deliberately not caller-settable. It is a stack-recursion guard, and
a caller-relaxable recursion bound is not a bound. It stays in force on every
execution, governed or not, and appears here only so its consumption is *reported*.

### 3.1 `answer-rows` is an operational cap, never `LIMIT`

`LIMIT` is query semantics and applies **before** the cap is tested. The cap
denominates the query form's own answer sequence, because that is what the caller
receives:

| Form | One unit is |
|---|---|
| `SELECT` | one solution row |
| `CONSTRUCT`, `DESCRIBE` | one **output statement** — an ordinary triple, an RDF 1.2 reifier binding, or an annotation |
| `ASK` | nothing; a boolean has no sequence to bound |

A graph form is not counted in solution rows. One `CONSTRUCT` row can instantiate a
whole template and one `DESCRIBE` row can pull in an entire concise bounded
description, so a caller who capped such a query at a thousand and received a
hundred thousand statements would have no governor at all.

### 3.2 Ceilings are inclusive, and zero is valid

Consumption **equal** to a ceiling is admitted; only consumption that would exceed
it trips. Zero is a valid ceiling and trips on the first charged unit of work. This
is the same reading `PagedQueryLimits` documents for the I/O tier, and it is why the
corpus's `boundary` band (ceiling = the metered cost) must **complete** while its
`over-bound` band (ceiling = the metered cost minus one) must trip.

Charging saturates at `u64::MAX` rather than overflowing: an arithmetic panic in a
resource meter would turn an exhausted budget into a crash.

The scratch ceiling is observed at each operator commit boundary, after that operator's
value construction has made its exact arena growth knowable. It is not a reservation
system: the operator that first crosses the ceiling remains in the certified partial
result, and reported consumption may exceed the ceiling by that one operator's growth.
This post-mint boundary is deliberate; pre-admitting `CONCAT`, `REPLACE`, aggregate, and
extension-function output would require guessing bytes before the value exists.

## 4. The charge schedule (normative)

`fuel` is a count of **observable evaluation events**. Every point costs one unit,
which is deliberate: a schedule whose costs are all `1` makes fuel a quantity a
caller can reason about and a corpus can pin, rather than a weighted score whose
units mean nothing outside this build.

| Charge point (`CHARGE_SCHEDULE` label) | Cost | Charged once per |
|---|---:|---|
| `algebra-node-entry` | 1 | algebra node entered during evaluation |
| `committed-output-row` | 1 | row committed to an operator's output |
| `bgp-candidate-quad` | 1 | candidate quad examined while matching a basic graph pattern |
| `path-frontier-expansion` | 1 | property-path frontier node expanded |
| `row-expression-evaluation` | 1 | expression evaluated over one row (per row, not per sub-expression, so the cost is stable across planner changes) |
| `user-function-invocation` | 1 | user-defined function invocation |
| `remote-request-issued` | 1 | request issued to a remote endpoint |
| `remote-row-ingested` | 1 | row ingested from a remote endpoint's response |
| `update-mutated-quad` | 1 | quad inserted into, or removed from, the store by SPARQL `UPDATE` |
| `property-function-invocation` | 1 | invocation of a host-supplied property-function relation (one `open`, over one driving row) |
| `property-function-row` | 1 | row a property-function relation emitted and this engine accepted |
| `aggregate-invocation` | 1 | `(group, aggregate expression)` pair folded — the fold's init/finish overhead |
| `aggregate-accumulation` | 1 | value folded into a group's running aggregate state |

`update-mutated-quad` is the only point outside the query evaluator and the only one
no algebra node raises. It exists because `CLEAR ALL`, `MOVE`, `COPY`, `ADD`, `LOAD`
and the `DATA` forms do work proportional to the **store**, not to the request text,
and none of that work enters the evaluator: without it a caller could set every
ceiling and still have `CLEAR ALL` run to completion over a hundred million quads
reporting zero consumption. It is charged **before** the quads are applied, per
operation, so an operation whose mutation would breach the ceiling makes no mutation
rather than a truncated one.

The two `property-function-*` points are the second half of the same argument for the
second producer whose bag size an outside party picks. A property-function call invokes
host code from predicate position and ingests whatever rows that relation chooses to
emit, so without them a call's whole cost would ride the generic per-node accounting —
one `algebra-node-entry` in and one `committed-output-row` per committed row out — which
prices a relation emitting a million rows from one invocation exactly as it prices one
emitting ten, and prices a thousand invocations that each emit nothing at all.

* `property-function-invocation` is charged **after** the call's admission refusals (an
  arity mismatch, an access pattern no declared mode serves) and immediately before the
  relation is entered — the same placement `user-function-invocation` uses, and for the
  same reason: a refused invocation evaluated nothing, and charging for work that never
  happened would make the schedule a description of the query text rather than of the
  execution.
* `property-function-row` is charged in emission order through the same governed ingest
  core `remote-row-ingested` is charged through: after the intermediate-cell ceiling is
  observed and before the row is interned. Charging in row order is what makes the
  ingested bag a positional prefix of the relation's output, which is what lets a
  truncation certificate describe it. A row the call *filters* — one disagreeing with a
  bound argument position, or with an earlier occurrence of a repeated variable — is
  never admitted and so never charged; it is an ordinary non-match, already accounted for
  by the invocation that asked.

Admission control prices a call too, and the composition rule is stated exactly because
the plan survey has no cross-node arithmetic at all — the predicted peak is a **maximum**
over per-node peaks, and a `JOIN` of two basic graph patterns composes to neither node's
product. A property-function node contributes `rows_per_invocation(mode) × driving_rows`
rows of `columns` = the relation's flattened arity, where `driving_rows` is the row count
predicted for the call's immediate left arm when that arm is a node the survey predicts
at all (a basic graph pattern, or a chain spine ending in another call), and `1`
otherwise. `rows_per_invocation` is read for the access pattern the call is *actually*
invoked in, which the feasibility-ordering pass has already fixed by the time the survey
runs. An over-estimate costs a caller an answer and can never hand them a wrong one; an
under-estimate changes nothing, because the live cell ceiling is still in force.

Where the fuel went is readable per algebra node: `explain_query` runs under
`METERED` and returns a `QueryExplanation` whose ledger decomposes the single fuel
total per node and per charge point, beside the cost planner's estimate for each
basic graph pattern.

The two `aggregate-*` points are the third half of the same argument, for the third
producer whose per-group work an outside party — the data, not the plan — sizes: an
aggregate, built-in or a registered custom aggregate alike, folds a group's rows into
one answer, and without a dedicated pair of points that fold rode the generic
per-node accounting exactly as an unpriced property-function call once did — one
`algebra-node-entry` in and one `committed-output-row` per group out, which prices a
`GROUP_CONCAT` folding a million rows into one group exactly as it prices one folding
ten.

* `aggregate-invocation` is charged once per `(group, aggregate expression)` pair,
  from the one dispatch site in the evaluator that decides whether a given aggregate
  expression folds through a built-in's internal state machine or through a
  registered custom aggregate's accumulator — so a caller's budget prices the two
  identically, and a `SELECT` naming several aggregate expressions over the same
  `GROUP BY` charges this point once per expression per group, independent of how
  many rows any group holds.
* `aggregate-accumulation` is charged once per value the fold path would pass to its
  `step` — a built-in's argument expression evaluating to a bound term, or every
  position of a custom aggregate's argument tuple being bound — charged **before**
  the `DISTINCT` deduplication check that may go on to discard it. Producing and
  inspecting the value is the work this point prices, and that work already happened
  by the time a duplicate is discarded; `COUNT(DISTINCT ?x)` folding a million
  duplicates of one value therefore charges this point a million times even though
  the underlying accumulator's `step` runs once. `COUNT(*)` takes no argument
  expression at all and so charges this point once per row unconditionally, rather
  than never.

A row whose argument expression is honestly unbound is not a value either point
counts beyond the invocation that already ran: no expression evaluated to nothing,
so no value existed for `aggregate-accumulation` to price.

## 5. The stop signal, and the one clock read

A deadline and a cancellation are the same primitive — `StopSignal`, a host-supplied
`poll() -> Option<StopCause>` — and differ only in the cause reported.

**Implementations MUST latch.** Once `poll` returns `Some(cause)` it must return the
same cause forever. A signal that can un-trip is not a governor: a query that
observed its deadline at one charge point and a clear signal at the next would
resume past that deadline, and the caller who set it would be told the query
completed inside a budget it had already blown through.

Two implementations ship:

* `CancellationFlag` — a shareable monotone bit a host can flip from any thread.
  Latching by construction; build a new flag per query rather than resetting one.
* `WallDeadline` — the profile's **one clock reader**. It is shipped so that
  obtaining a deadline is not a caller obligation: making every caller hand-roll a
  signal would leave two callers with two different notions of "expired", which is
  the optionality this profile refuses everywhere else.

`WallDeadline` is target-split — `std::time::Instant` natively, `js_sys::Date::now()`
on `wasm32-unknown-unknown` — so the wasm target remains supported. The wasm half is
demonstrated by an executed Node round-trip against a real module rather than by a
green cross-compile.

It latches on **either** `now >= deadline` **or** `now < start`. The second
disjunct is the *rewind rule*: on wasm the clock is a wall clock and is
NTP-steppable, and a backwards step would make a naive `now >= deadline` test
un-trippable — a silent failure in which a query outlives a budget the caller
believes is enforced. An observed rewind is treated as expiry, never as recovered
budget, because the engine cannot distinguish a small clock correction from a large
one. The rule is enforced on both targets from one predicate.

The signal is observed on algebra-node entry and exit, at every logical charge point
when fuel is not engaged, every **`STOP_POLL_FUEL` = 4093** units when fuel is
engaged, at a ceiling crossing, and around a federated request. The exit checkpoint
is load-bearing: it prevents a signal that fires inside the final operator from being
reported as complete merely because no next node exists. The interval is a build
constant, never a knob — a caller-tunable poll interval would make the exact charge
point at which an injected deadline is observed part of caller configuration. It is
prime, is not a power of two and is not a round decimal, so it shares no factor with
the ceilings callers actually pick and the stop poll never lands on a fuel trip
systematically. `STOP_POLL_FUEL` is part of the pinned identity (§10) because it
bounds the charged-work interval between periodic polls.

Reminder, because it is the point of §1: the wall-clock measurement in this section
is non-deterministic. The polling schedule itself is deterministic and is frozen by
the injected-deadline matrix.

## 6. Precedence (normative)

When several governors are true at the **same charge point**, the winner is fixed by
`TrippedGovernor::precedence_rank`, lower winning. The order lives beside the kernel
type so every tier resolves a simultaneous trip identically, and the match that
assigns ranks is wildcard-free so a new governor cannot be added without being
ranked.

1. A genuine `EvalError` — **outranks every governor**, and is compared where
   evaluation results are combined rather than inside the rank function.
2. `Stopped { cause: Cancelled }` — an explicit cancellation is a decision.
3. `Stopped { cause: Deadline }` — a deadline signal fired; the wall implementation
   is elapsed-time based, while a caller may inject a deterministic signal.
4. Every `Refused { .. }`, in this dimension order: `IntermediateCells`, `Fuel`,
   `AnswerRows`, `ScratchBytes`, `RemoteRequests`, `UdfDepth`, `Pages`, `Bytes`. An
   **admission refusal** is decided before the first charge, so it is the earliest
   verdict any numeric governor can reach and the one that explains why nothing ran.
5. Every `Budget { .. }`, in the same dimension order. `IntermediateCells` leads
   because it defends against unrecoverable allocation failure; `Pages` and `Bytes`
   belong to the demand-paging tier and normally surface as that tier's own typed
   error, but remain ranked here so the shared order is total.

Because the stop signal is polled every `STOP_POLL_FUEL` units, fuel can cross its
ceiling *between* polls. The rule is therefore: at each charge point, resolve in
precedence order over the conditions **already true at that point**, never over
conditions that might become true later.

### 6.1 `Refused` is a distinct verdict from `Budget`

`TrippedGovernor::Refused` reports a ceiling refused at **admission**, because the
planner's estimate for an operation already exceeded it — so nothing ran and nothing
was consumed. It is a separate variant on purpose: folding it into `Budget` would put
an *estimate* in the `consumed` slot of an evidence vector every other dimension
fills with measurements, i.e. a receipt saying a query spent what it was merely
predicted to spend.

A refusal never claims completeness. An over-estimate therefore costs a caller an
answer they could have had; it can never cost them a wrong one.

## 7. The answer-completeness interval

A truncated execution returns `BudgetExhausted { tripped, evidence, partial }`. The
`partial` field is `PartialAnswers`, a **three-way interval** rather than a yes/no —
the two bounds are genuinely different licences, and collapsing them would either
forbid a sound use or permit an unsound one.

| Arm | What is proven | What it licenses |
|---|---|---|
| `Certain(rows)` | a certified **lower** bound: every row here is an answer | admitting the rows as answers; the query may have more |
| `AtMost(rows)` | a certified **upper** bound: every answer is here, but some rows here may not be answers | **only the negative reading** — a row absent from this result is definitively not an answer |
| `Unknown(barrier)` | neither bound survived to the root | nothing; **no row crosses at all**. The `NonMonotoneBarrier` names the operator that withheld them, which is what tells a caller whether a larger budget or a different query is the way forward |

The certificate is computed by a static analysis over the algebra, in terms of
**prefix**-monotonicity rather than subset-monotonicity. That distinction is
load-bearing: `Slice(0, 1)` is not subset-monotone, so a subset certificate would
license emitting `b` for a query whose only answer is `a`.

### 7.1 The positional-prefix bit

`PartialSparqlResult::is_positional_prefix()` carries the further fact the rows
themselves do not: whether they are the true output's **first** rows, in order. This
is a relation to the complete output, not by itself a timing promise. Under a
deterministic governor, re-running the same query and snapshot with a larger ceiling
returns these rows first, so a caller can page by raising that ceiling. A wall
deadline is not deterministic: even a longer duration may stop sooner on another
run, which must therefore be treated as fresh.

When it does not, the rows are a sound sub-bag (or super-bag) whose *positions* mean
nothing. `ORDER BY` costs it; so does truncating the **right** input of a left-major
hash join, or the **left** arm of a `UNION`, both of which remove rows from the
middle of the output rather than from its end.

### 7.2 The schema of a partial result

A complete result reports the columns the query produces. A partial result reports
the columns of the operator arms that were **actually evaluated**, and for four
operators those differ: when the **left** arm of `JOIN`, `OPTIONAL`, `LATERAL` or
`UNION` truncates, only the left arm's columns are reported. The right arm is
deliberately never evaluated — starting a fresh subtree after the budget is spent is
unbounded work a governor must not license — and this engine chooses column *order*
during evaluation, so the right arm's columns are not derivable without doing the
work that was just refused. No row is affected; only the reported column list is
narrower, and a caller that diffs column lists across the complete and partial paths
must expect that.

### 7.3 `UPDATE` has no partial arm, structurally

`GovernedUpdateOutcome` is deliberately not `GovernedOutcome`. A partial *mutation*
is not a certifiable thing: an `INSERT`/`DELETE` that landed halfway and was reported
as "budget exhausted" is not an incomplete result, it is a corrupt store, and the
corruption is silent because every later query answers confidently from it.

So the trip arm carries the governor and the evidence and **structurally nothing
else** — there is no field partial mutations could be read out of, because there are
none. A tripped request leaves the caller's dataset handle exactly as it found it.

### 7.4 Entailment regimes are governed in two halves

Phase two — the SPARQL evaluation over the materialized closure — is governed
completely, and a trip there is an ordinary `BudgetExhausted` with certified partial
answers. Phase one — materializing the closure — honours the **stop signal and
nothing else**, and that is a semantic boundary rather than an omission: a numeric
ceiling on a reasoning run is a charge schedule, and a caller-settable one would mean
two callers materializing the same regime over the same data get different closures.
A stop signal has no such property — the closure either finishes bit-for-bit as it
would have with no signal attached, or the run ends with `ClosureStopped` and nothing
at all. See `purrdf_datalog::stop` for the full argument.

## 8. The federated transport seam

`HttpRequest::stop` can only be honoured by a transport capable of abandoning a call
it is already inside, and nothing forces a host to write one. Ignoring it is a
supported implementation, and the contract is stated for both cases:

* A transport that **ignores** `stop` is still bounded at **per-request
  granularity**, never unboundedness: the evaluator polls the signal and charges the
  request before dispatch, and inspects the outcome the moment the call returns.
* What a deaf transport loses is the **positional-prefix claim**. An honouring
  transport *abandons* the exchange, so the rows in hand are still the true output's
  first rows and retain the relation needed for deterministic resumption. A deaf
  transport *completes* the exchange and has its response discarded, so rows are
  missing from the middle of the answer rather than from its end, and the claim is
  withdrawn.
* The multiset bound survives either way — the certificate is `Certain` in both — so
  the loss is resumability, never soundness.

Both readings are pinned by the corpus's `service-*-transport-*` cases, which record
the exact number of exchanges the transport was asked to perform. That count is the
observation separating "the request was prevented" from "the request was made and its
answer discarded", which no governor evidence can distinguish.

### 8.1 The property-function relation seam

`PfCursor::next` is likewise handed no signal it is obliged to read, and a relation that
blocks forever inside one `next` degrades the stop-check granularity to one call. The
contract is stronger here than for a transport, because a relation's output is a row
**stream** rather than one atomic exchange:

* The evaluator polls the stop signal before opening a cursor and between successive
  pulls, and every emitted row crosses the per-row admission point on its way into the
  bag. That point is itself a bounded-work checkpoint, so a fired signal is observed
  there whether or not the relation ever looked at it.
* A deaf relation is therefore bounded **per row**, not merely per invocation: its extra
  row is pulled and then refused at admission rather than ingested.
* The consequence is pinned by the corpus's `property-function-*-relation-cancel-mid-invocation`
  pair, which fires the caller's flag at the identical pull through a cursor that abandons
  the invocation and one that ignores the signal entirely: the two answers are
  **byte-identical**, certificate included. Reading the signal inside an invocation is an
  optimisation, never the thing that makes the query bounded.

Both cases in that pair carry `positional-prefix=false`, and that is not the deaf/
cooperating distinction leaking through — the corpus drives the call laterally
(`?s ex:p ?m . ?s ex:pf:emit ?x`, the driving pattern joined to the call through
`LATERAL`), so the call's own emission order is not the joined output's order. A
truncation that originates inside a laterally-driven call therefore bounds the answer
from below without being able to say where in the answer the cut fell, and the licence
is withdrawn for that structural reason, independent of whether the cursor was deaf or
cooperating. The standalone shape — where the call IS the node directly under the
projection, so its own order and the output's order coincide — keeps the licence; see
the `property-function-first-row-fuel-ceiling` case and `property-fn-eval`'s own
truncation test for that half.

`rows_per_invocation` is a separate obligation and it is an **honesty contract**, not a
hint: admission control refuses a plan whose declared bound already breaches the cell
ceiling, so a bound that under-states reality turns an admission decision into a wrong
one. A genuinely unbounded generator declares `u64::MAX`.

## 9. Evidence

`GovernorEvidence` is returned on the **complete** path as well as the exhausted one,
because "completed, cost N fuel, peak M cells" is how a caller sizes a budget in the
first place. It carries consumption per dimension, the ceilings in force (echoed, so
the evidence is self-describing), and the governor that stopped the execution or
`None`.

Governor state is **operation-local**. Build one per execution: consumption is
cumulative, so a state shared across queries would drain one query's budget into the
next and produce an intermittent, essentially undiscoverable bug.

## 10. Profile identity (normative)

| Constant | Value / how to read it |
|---|---|
| `GOVERNOR_PROFILE_ID` | `purrdf-sparql-governors` |
| `GOVERNOR_PROFILE_VERSION` | `6` |
| `GOVERNOR_PROFILE_DIGEST` | derived — see below |
| `STOP_POLL_FUEL` | `4093` |

The digest is **derived, not declared**. A hand-maintained "remember to bump it when
costs move" convention is a rule enforced by discipline, which is to say not
enforced: the one time it is forgotten, a consumer's pinned identity silently
describes a schedule that no longer exists and every budget sized against it is
wrong. Hashing the table means a schedule change *cannot* keep the old identity.

It is the lowercase-hex SHA-256 of a line-oriented preimage: the profile identifier,
then the profile version, then one `label\tcost` line per `CHARGE_SCHEDULE` entry in
table order, every line `\n`-terminated. Tab and newline cannot occur in a label, so
no entry encodes two ways and no two distinct schedules encode alike. A consumer can
therefore recompute it from this document alone:

```sh
{ printf 'purrdf-sparql-governors\n6\n'
  printf '%s\t1\n' algebra-node-entry committed-output-row bgp-candidate-quad \
    path-frontier-expansion row-expression-evaluation user-function-invocation \
    remote-request-issued remote-row-ingested update-mutated-quad \
    property-function-invocation property-function-row \
    aggregate-invocation aggregate-accumulation
} | sha256sum
# 8857d03631fc533881ccad603cdf7f82786c581a8bf0ee9f1637d227fb36290a
```

SHA-256 through the `sha2` crate, which is pure software with no entropy source, so
the derivation is `wasm32-unknown-unknown`-clean and reproducible on every target.

**What pinning the identity means.** The id and version say *which* schedule was
agreed; the digest says *exactly what that schedule is*. Together they make a fuel
number portable: two parties holding the same triple are talking about the same
events priced the same way. They say nothing about which cases were run to
demonstrate it — that is §11's job.

## 11. Normative vector corpus

The corpus is `vectors/sparql-governors/` and is **frozen**: every payload byte is
covered by a SHA-256 manifest checked on every build (`scripts/check-corpus-frozen.py`),
so a silently edited expectation fails rather than passes. It is run by
`crates/sparql-conformance/tests/governor_corpus.rs` and appears on the scoreboard in
[`CONFORMANCE.md`](./CONFORMANCE.md). See the corpus's own `README.md` for the case
inventory and file formats.

Each reproducible case pins **three** records, and keeping them apart is deliberate:

1. the **outcome discriminant**, in the kernel's own `TrippedGovernor::label`
   vocabulary;
2. the **certified rows and their certificate class** (`certain` / `at-most` /
   `unknown`, plus the positional-prefix bit);
3. **what the execution spent**, recorded independently of the rows.

A property-function or aggregate case pins a **fourth**: the metered fuel decomposed
*per charge point*, read off the per-node ledger. Fuel is one number and v5 put two
charge points inside it for the property-function seam, and v6 put two more inside it
for the aggregate fold, so a band alone cannot tell a schedule that moved one point in
a pair from one that moved the other.

The third is the one easy to omit and impossible to reconstruct afterwards: a corpus
pinning only rows could not detect a charge-schedule change that happened to cut in
the same place — the answer would be unchanged, the receipt would not, and every
budget sized against the old schedule would be silently wrong while the corpus stayed
green.

The wall-deadline smoke case is the deliberate exception: it has only the outcome
discriminant because rows and spend depend on elapsed time. Across the corpus there
are 45 cases total, of which 38 form zero, boundary, or over-bound lanes and the
remaining seven are transport, relation, charge-seam, and wall-clock cases.

Boundaries are **measured, never authored**. For each caller-settable dimension the
corpus carries a `zero` ceiling (must trip), a ceiling equal to the metered cost (must
**complete**, because ceilings are inclusive) and one below it (must trip); the
harness re-measures under `METERED` and re-derives that relation on every run rather
than trusting the numbers in the file.

Three of the band lanes exist to separate the two property-function charge points from
each other rather than to cover a dimension: one drives many invocations of a relation
that emits nothing (so its band carries zero row charges), one drives a single
invocation emitting many rows, and one drives 1200 rows — above the evaluator's fork
threshold. That last one is where the parallel-invariance claim of §1 becomes a
frozen vector rather than prose: `METERED` engages the intermediate-cell counter, which
routes the *measuring* run through the sequential chunk driver, while a band case sets
fuel alone and takes the parallel one. A boundary that is exactly the sequential
measurement, and an over-bound one unit below it that must trip, is the statement that
the two drivers charge the same items in the same order.

Two more band lanes separate the two aggregate charge points from each other the same
way: `aggregate-invocation-fuel-*` folds twelve aggregate expressions over one implicit
group whose input matches nothing (so its band carries zero accumulation charges), and
`aggregate-accumulation-fuel-*` folds one aggregate expression over one implicit group
holding twelve rows (so its band carries exactly one invocation charge). Both fixtures
fold their single group to completion well inside the boundary ceiling, so the
over-bound ceiling — one unit below it — lands on the generic per-node
`committed-output-row` charge for the group's own already-computed row rather than
inside the fold itself: the row is reported at both boundary and over-bound, `complete`
at the former and a certified positional-prefix `budget-exhausted` at the latter.

### 11.1 The corpus digest, and how to pin it

```text
GOVERNOR_CORPUS_DIGEST = 2f7daf6abb6ac960a76e260b59e79d76c8d1ab2be6fa17efaf50d0acf6ee7282
```

It is the SHA-256 of the corpus freeze manifest, which in turn covers every payload
byte. Defining it over the manifest rather than over a bespoke traversal means a
consumer reproduces it with one command and without running any of this crate's code
— a digest only its author can compute is not one anybody can independently check:

```sh
sha256sum scripts/conformance-frozen/vectors-sparql-governors.sha256
```

The library derives the same value and the harness compares them, so the corpus
cannot change without the constant being re-pinned in the same commit.

**What this digest does and does not certify.** It pins the *evidence*, not the
behaviour: two builds agreeing on it agree about which cases and which expected
numbers were on the table, which is what makes a disagreement about behaviour
legible. Behaviour itself is pinned by `GOVERNOR_PROFILE_DIGEST`, and the corpus is
what demonstrates that a build implements it.

## 12. Versioning

`GOVERNOR_PROFILE_VERSION` is incremented by any change that could **move the point
at which a caller's budget trips**:

* a change to `CHARGE_SCHEDULE` — a point added, removed, renamed, or repriced;
* a change to the precedence order of §6;
* a change to the inclusive-boundary rule of §3.2;
* a change to how many charged events a given query performs, even when the schedule
  itself is byte-identical.

A change that cannot move a charge — a refactor, a clearer diagnostic — does not
increment it. That restraint is what makes the number worth pinning.

| Version | What moved |
|---|---|
| 1 | the initial schedule |
| 2 | schedule byte-identical; an expression-embedded `EXISTS` stopped being re-evaluated once per rayon chunk, so its fuel stopped depending on the machine's thread count |
| 3 | schedule byte-identical; the answer cap and `LIMIT` are pushed down the certified prefix-monotone spine, so a leaf under a row ceiling stops scanning at the ceiling instead of materialising its whole output. Also adds `Refused` and applies the answer cap to `CONSTRUCT`/`DESCRIBE` output statements |
| 4 | the first version whose schedule is **not** byte-identical: `update-mutated-quad` is appended, because SPARQL `UPDATE` became governable and a mutation is work a budget must be able to bound. No *query* charges it |
| 5 | `property-function-invocation` and `property-function-row` are appended, because the evaluator gained a second producer whose bag size an outside party picks: a host-supplied relation invoked from predicate position. Admission control also learns to price a call from the relation's declared row bound. No query without a registered property function charges either point |
| **6** | `aggregate-invocation` and `aggregate-accumulation` are appended, because the evaluator's third such producer — an aggregate, built-in or a registered custom aggregate alike — folds a group's rows into one answer, and that fold's init/finish and per-value work rode the generic per-node accounting until now. Both points are charged from the one dispatch site that decides which kind of fold a given aggregate expression names, so a built-in and a custom aggregate over the same group shape cost the same fuel. No query without an aggregate charges either point |

### 12.1 What a consumer must re-verify when the version moves

A version bump is not a drop-in upgrade, and the list is short because each item is
a thing a pinned number can silently stop meaning:

1. **Re-read `GOVERNOR_PROFILE_DIGEST`** and confirm it matches the schedule you
   intend to price against. If the digest moved but the version did not, the build is
   lying and must be rejected rather than reconciled.
2. **Re-measure every fuel ceiling** under `QueryGovernors::METERED`, against your own
   representative queries. A ceiling sized against the previous version was sized
   against work this build may no longer do (v3) or may now do (v4, v5). Do not scale
   the old number.
3. **Re-check ceilings you sized at or near a boundary.** Ceilings are inclusive, so a
   ceiling that was exactly the metered cost completed; after a bump it may be one
   short.
4. **Re-read `GOVERNOR_CORPUS_DIGEST`** and re-run the corpus against the linked
   build. A profile version without corpus evidence is a claim, not a receipt.
5. **Re-check any simultaneous-trip handling** if §6 moved: which governor a caller
   sees for a query that breaches two ceilings at once is part of this contract.
6. **Nothing needs re-verifying for the wall deadline**, because nothing was
   guaranteed about it in the first place (§1). Do not treat a changed deadline trip
   point as a regression.

`answer-rows`, `intermediate-cells`, `scratch-bytes` and `remote-requests` ceilings
are *usually* unaffected by a version bump, because they denominate results rather
than events — but "usually" is not a guarantee, and step 2's re-measurement covers
them at no extra cost.

## 13. What a consumer pins

| Field | Source |
|---|---|
| profile id | `purrdf_sparql_eval::GOVERNOR_PROFILE_ID` → `purrdf-sparql-governors` |
| profile version | `purrdf_sparql_eval::GOVERNOR_PROFILE_VERSION` → `5` |
| profile digest | `purrdf_sparql_eval::GOVERNOR_PROFILE_DIGEST` (§10) |
| stop-poll interval | `purrdf_sparql_eval::STOP_POLL_FUEL` → `4093` |
| corpus digest | `purrdf_sparql_eval::GOVERNOR_CORPUS_DIGEST` (§11.1) |
| release | the tagged release the above were read from |

The profile id, version, schedule digest and stop-poll interval travel together on
every `QueryExplanation` as a `ProfileIdentity`, so two builds that disagree about
what a query costs cannot produce explanations that look comparable. The corpus
digest is read separately from `GOVERNOR_CORPUS_DIGEST`: it identifies the external
evidence set, not the execution that produced one explanation. The release identifies
the published artifact carrying both.

Running the corpus of §11 against a linked build turns that pin into a receipt. What
it does **not** turn into a receipt is the wall deadline — deliberately, and for the
last time: determinism is claimed for fuel, the answer cap, the cardinality/cell
ceiling, and injected poll-count signals; it is **not** claimed for elapsed time.
