<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# PurRDF retrieval composition: plan, compile, execute, fuse

PurRDF already answers ranked retrieval three ways: exact BM25 rows from
`purrdf-text`, kNN rows from the evaluator's `knn` module over PURREMB
artifacts, and the `geof:` family from `purrdf-geo` — each a relation
reached through the property-function seam, each returning its own ranked
list under caller-supplied IRIs. What does not exist yet is the layer that
turns "one request" into "one answer" across all of them: something has to
decide which producers a request reaches, run them, and combine their
lists into a single ordered result without destroying what each producer
knew about its own answer.

This document records the design for that layer before any of it is code,
in the same spirit as the other design records here: the decisions a
reader would otherwise take for oversights, and the limits of the
guarantees. It fixes **structure**, not a wire signature or an API name.
Like every extension in this family it is a composition **outside the
kernel**: nothing in `purrdf-core` or the property-function seam changes
to admit it, and PurRDF continues to mint no vocabulary — producers,
strata, and weights are caller-supplied configuration, `example.org`
throughout the fixtures.

---

## 1. Four stages, one executor

Retrieval is four composed stages:

```text
plan(request, registry, statistics) -> Plan
compile(Plan)                       -> SPARQL query
execute(query)                      -> per-stratum ranked streams
fuse(streams, profile)              -> one ordered answer

search(s) = fuse(execute(compile(plan(s, registry, statistics))), profile)
```

Only `execute` runs a query, and every higher-level entry point is
*defined as* the composition — not implemented a second time to behave
like it. The distinction is enforceable: the composition identity above is
a testable equality, and the test suite carries it as one. A convenience
entry point that re-implemented the pipeline could drift from it silently;
one that calls it cannot.

The registry is the injection point. A producer registers the IRI of its
relation, the shape of its rows, the stratum labels it emits under, and a
parse predicate saying which request terms it can consume. All of that is
caller configuration in the existing style — the composition layer ships
no producers of its own and no default registry.

## 2. The plan is a value, and that is exactly why it is untrusted

`plan` returns a value the caller can inspect, edit, serialize, and hand
back. This is deliberate and load-bearing: a caller who knows the data
better than any statistic does — and such callers exist — corrects the
plan term by term instead of abandoning planning wholesale, and every
decision the planner made is visible as the return value rather than as a
reporting obligation someone must remember to implement.

The same property makes every plan untrusted input. A value that can be
edited can be edited wrongly, and a value that can be deserialized can be
forged. So the pipeline **admits** plans at `compile`, and `compile` is
the narrow waist where one check covers all three origins — freshly
planned, caller-edited, and deserialized — because all three must pass
through it to execute.

Admission is not syntax checking, and being compilable is not being
admissible. The failure that matters is semantic: delete a producer from
an edited plan and the result still compiles to a perfectly well-formed
query that silently answers less than the registry promised. Admission
therefore checks the plan against the registry's declared invariants —
producers the registry marks as always-applicable are present and receive
what the registry says they must receive, per-stratum depths respect
declared bounds, weights refer to declared strata and are valid under §5
— and refuses with the exact violated dimension rather than executing a
plausible subset.

The composition layer itself takes no position on which producers must
exist. Whether some producer is mandatory, always-applicable, or must
receive the whole request is **registry policy, declared by the caller**;
admission enforces whatever the registry declared and adds nothing of its
own. Coverage doctrine belongs to the systems that configure a registry,
not to the mechanism that honors one.

## 3. `compile` emits SPARQL, and there is no second algebra

The compiled form of a plan is a SPARQL query over the registered
property functions — the language the evaluator already executes, not a
new plan algebra with its own evaluator.

Two properties follow, and both are the point. The compiled query is
**independently executable**: a caller can take the emitted text and run
it through `purrdf-sparql-eval` with no composition layer in the path,
which extends the repository's existing discipline — the conformance
suites already insist the evaluator is the single source of query truth —
out to a seam a caller can stand on. And the planner is **structurally
unable to hide a decision**: if a plan's executable content is query
text and the executor runs only query text, everything the planner chose
is visible in the emission, and nothing can live between planning and
execution.

SPARQL's reach ends at that rung in both directions. Below: `plan` takes
free text, which is not a graph pattern, and no one should hand-write a
query to obtain what planning exists to derive. Above: rank fusion is
ranking policy, not a graph-pattern operation, and expressing §5 as an
extension function would embed policy inside the query language where
neither the conformance corpora nor other engines could follow it.

## 4. Seams are stopping points; there are no mode flags

Every boundary between stages is a supported place to stop. A caller who
wants only the plan calls `plan` and stops. A caller who wants the query
text calls through `compile` and stops. A caller who wants per-producer
ranked streams — because it intends to enumerate a million rows, or to
apply its own combination — calls through `execute` and stops. None of
these is a degraded use of the surface, and none is requested by passing
an option that asks the full pipeline for less.

Every boundary is equally a place to **start**. A caller who has already
done its own planning — one that knows exactly which producer terms it
wants, scoped exactly how, with nothing inferred — does not need a
planner mode that defers to it; it hands the plan it built to `compile`,
where admission covers it on the same terms as anything the planner
produced, since §2's three origins already include the hand-built case.
A caller holding compiled query text likewise runs it through the
evaluator directly. Stopping early and starting late are one property
read in both directions, and the design carries one concept — the seam —
instead of a mode selector per knowing-caller shape.

The rule is what keeps the surface from accumulating options: a flag that
changes what a stage produces is a second behavior the first must be kept
in agreement with, while a seam is the same behavior observed earlier.
When a capability appears to need a flag, the design question is which
stage boundary it actually belongs at.

## 5. Fusion is stratified reciprocal-rank with exact arithmetic

`fuse` combines per-stratum ranked lists. Within its stratum an item has
rank `r` (1-based); its contribution is

```text
contribution(stratum, r) = w_stratum * recip(K + r)
```

where `recip` is the reciprocal evaluated in fixed point at a declared
scale with a declared rounding, `w_stratum` is the stratum's declared
weight, and `K` is the profile's smoothing constant. An item appearing in
several strata **sums** its contributions, and the final order is by
fused score descending with a declared, total tie-break.

The arithmetic is exact for the same reason `purrdf-text`'s is, and the
reasoning transfers where the kNN module's deliberately does not. The kNN
kernels stay in binary64 because every operation they need is correctly
rounded and their artifact format is already binary64; BM25 left floating
point because `ln` is not correctly rounded and last-bit disagreement
reorders near-ties. Rank fusion needs no transcendental — a reciprocal at
a declared scale is one exactly-rounded division — and near-ties are not
an edge case here but the workload: fusing ranked lists *is* deciding
between items whose scores differ by almost nothing, so a representation
in which two sums can differ by an unrepresentable amount, or in which
addition order changes the total, converts the mechanism's core case into
target-dependent output. Fixed point with checked additions closes both:
sums are associative when they do not overflow, overflow refuses loudly
rather than wrapping, and the emitted order is byte-identical on every
target with no fold-order caveats to assert.

Summation, not selection, is the cross-stratum rule, and the reason is
recorded here because the rule will otherwise be re-litigated by someone
reasonable. There is no optimal fusion rule to defend: the impossibility
results for rank aggregation forbid one in general, and exact aggregation
(Kemeny-style) needs the complete rankings that bounded per-stratum depth
deliberately denies. So the decidable question is narrower — sum versus
discard — and there summation dominates. An item surfacing in two strata
is two independent observations supporting it, and under any
evidence-combining account two observations for one candidate combine to
at least as much as either alone; a "best-stratum-wins" rule discards a
confirming observation, which is the same silent loss §6 forbids for
status, applied to rank mass. (A caller who wants selection can have it — by
stopping before `fuse` and selecting from the unfused streams itself.
What the fused rung does is sum; no arrangement of weights reproduces
selection, and this surface does not pretend otherwise.)

Weights and `K` live in a **fusion profile**, and the profile is an
identity, not a knob: two answers fused under different profiles are
answers to different questions, so a changed weight or a changed `K` is a
new profile identity, never a runtime parameter drifting silently under a
stable name. Rank space also makes the weights honest at the top of the
list — a weight applied to ranks preserves its declared ratio at rank 1,
where contests are decided, instead of decaying to nothing as matches
approach perfection the way a multiplier on a bounded distance does.

## 6. Producer status survives fusion — the only place it can die

Every producer's stream already names its own completeness: which
producer it is, whether it answered fully, and if not, why. Below `fuse`,
honesty is therefore free — N streams carry N statuses and nothing merges
them. Fusion is the single stage at which that information *can* be
destroyed, so the obligation binds there: a fused answer carries the
status of every producer that contributed and of every applicable
producer that could not, and never reduces them to one aggregate flag.
"Two producers answered and one could not" and "all three answered" are
different answers and must remain distinguishable.

The streaming corollary is the easy one to get wrong. A status written at
the head of a stream can be falsified by a failure later in the same
stream, so completeness is asserted by a **trailer**: a consumer that
reads a prefix and stops holds evidence that the answer is incomplete,
and only the trailer can say otherwise. Nothing readable mid-stream
entitles a consumer to a completeness claim.

## 7. Fused is top-k by construction; unfused streams without bound

The two rungs differ in kind, and the difference is algebraic rather than
an implementation budget.

Unfused enumeration is unbounded: each producer emits in its own rank
order, no cross-producer state exists, and the answer is N independent
streams of any length under the evaluator's ordinary windowed execution.

Fused enumeration is inherently top-k, because §5 sums across strata: no
item can be emitted until it is known not to reappear in another
stratum's stream and raise its total. Contributions fall monotonically
with rank, so a threshold over the stream heads bounds how deep the
fusion must look to certify its next emission — top-k terminates with
memory proportional to the frontier. *Complete* fused enumeration, by
contrast, would need to remember everything already emitted, and this
surface does not offer it as though it were free. A caller who wants to
walk everything wants the unfused rung, which is built for exactly that.

## 8. `plan` is a pure function, and a plan knows what it assumed

Planning consults statistics — cardinalities, selectivities, whatever the
registry's producers publish — because that is what makes it planning.
Those statistics are an **explicit input**, not something `plan` reaches
into the store for, and the emitted plan records what it was planned
against.

Purity buys three things at once. Planning is golden-testable as text in,
text out — `(request, registry, statistics) -> plan` with no store, no
corpus, and no clock in the fixture, so a planner change is a reviewable
diff in the goldens, in the same way the serializers are held
byte-deterministic today. Planning is reproducible across processes and
targets for the same reason. And a plan is **pinnable**, with a
precisely bounded guarantee: a captured plan reproduces the candidate
set and the per-stratum ranks, because those are decided by what the
plan records. It does not by itself reproduce the final order, because
the fusion profile enters at `fuse` and is deliberately not a plan input
— planning legitimately happens without knowing how the result will be
fused, and forcing that choice early would couple two stages this design
keeps apart. A reproducible **answer** is therefore a pair of
identities, the pinned plan and the fusion profile in force, and
anything reporting a fused order names both. Replaying a plan against
statistics that have moved is a detectable condition with a declared
behavior at admission — never a silent replan and never a silent
pretense that nothing changed.

## Open questions

1. **Where the composition layer lives.** The extension precedent says a
   sibling crate over the seam, with `purrdf-sparql-eval` untouched;
   whether it is one crate or a `fuse` primitive plus a thin planner is
   an implementation choice this document does not fix.
2. **The reciprocal's scale and rounding.** Fixed by the fusion profile
   identity in §5; the concrete scale, the rounding direction, and the
   overflow bound belong to the implementation record with its tests,
   alongside the existing exact-arithmetic precedents.
