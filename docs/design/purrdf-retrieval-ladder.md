<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# PurRDF retrieval composition: plan, compile, execute, fuse

PurRDF already answers ranked retrieval two ways: exact BM25 rows from
`purrdf-text` and kNN rows from the evaluator's `knn` module over PURREMB
artifacts — each a relation reached through the property-function seam,
each returning its own ranked list under caller-supplied IRIs. The
`geof:` family from `purrdf-geo` is reached the same way but is not a
third: it computes a **set**, with a deterministic ordering and neither a
score nor a rank, so it composes as a constraint on candidates rather
than as a stratum of a fused ranking. What does not exist yet is the
layer that turns "one request" into "one answer" across the ranked ones:
something has to decide which producers a request reaches, run them, and
combine their lists into a single ordered result without destroying what
each producer knew about its own answer.

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
execute(query, dataset)             -> per-stratum ranked streams
fuse(streams, profile)              -> one ordered answer

search(s, d) = fuse(execute(compile(plan(s, registry, statistics)), d), profile)
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
producers the registry **declares** mandatory are present and receive
what the registry says they must receive, per-stratum depths respect
declared bounds — and refuses with the exact violated dimension rather
than executing a plausible subset.

Stratum weights are **not** among those invariants, and §8 is why: the
fusion profile is deliberately not a planning input, so a plan records no
weights for admission to check. The two things a weight dimension would
have asked are answered where the fusing weights live instead. A weight
unusable under §5 cannot exist, because `FusionProfile` construction
refuses a non-positive weight, an empty weight map, and a weight vector
whose admitted ceiling leaves the fixed-point range. And the mismatch
that can actually cost a caller rows — a stratum the plan reaches that the
profile does not weight — is *reported* per stratum on the answer rather
than refused, because a profile is a reusable law chosen without
reference to any one plan.

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
text and the executor runs only query text, nothing can live between
planning and execution — the text is the whole of what runs.

That is a claim about the *executable* content, and it is not the same
as "every decision is legible in the text". One decision is not, and it
is named rather than left implicit: a producer's declaration may accept
a request term's shape and place none of it, in which case the producer
is called with the term absent from its arguments and the emitted text
carries no trace of the request. Two different requests then compile to
the same query, which is exactly what that producer asked for. So the
decision lives in the plan instead, per term: such a term is bound to
no producer and is reported in the plan value's own unserved-term evidence,
which makes an empty evidence list mean the strong thing — every term
reached a producer *with its content*. Between the text and that list,
nothing the planner chose is unaccounted for.

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

**Where the weight enters is a declared rule, not an implementation
detail.** Evaluating `recip` first and applying the weight afterwards
rounds twice, and the inner rounding is a ceiling the weight cannot
lift: `trunc(S / D)` with `S = 10^12` stops strictly decreasing once
`D² > S`, so under that rule *every* weight at or above one shares one
monotone range ending near a million ranks, and a stratum that must be
read deeper than that cannot be. Folding the weight into the numerator
computes the same quantity with one exactly-rounded division —
`trunc(w_raw / D)`, where a weight's raw integer is already `w · S` —
and its monotone range runs to about `10^6 · sqrt(w)`. Both are §5 laws;
neither needs a transcendental; the profile names which one it runs
under and carries that choice in its identity, so the second rule adds a
capability without moving any number a previously issued profile ever
produced. The relation reads in both directions, and a profile author
needs both: a weight caps the depth that stays ordered, and a required
depth therefore sets a floor under the weight — `w ≳ (depth / 10^6)²`.
Weights are only ever compared with each other, so scaling the whole
vector buys depth without changing any fused order.

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

That last clause decides a configuration question, so it is stated as a
rule rather than left as an aside. One stratum carries one producer, and
the refusal at registration names two exits: producers whose scores are
already comparable merge inside one producer; producers that score by
different laws take a stratum each. **The first exit requires two things,
not one: comparable scores, *and* that the host's intended weighting can
ride inside the score.** Sharing a law buys only the first.

Two classes over one embedding space — headings and bodies — share a law
exactly, but if one is meant to outweigh the other and the shared score
is a bounded metric (a cosine distance in `[0, 2]`, lower-better), a
merge sorted by `d / w` gives the favoured class an edge of only
`(1 - s)(1 - w_low/w_high)`: `0.45` against a match at similarity `0.1`,
`0.0005` against one at `0.999`, and zero against a perfect match. The
weighting is largest where it matters least and vanishes where the top-k
contest is decided, so the merge silently drops it — and an unweighted
merge still returns a plausible ranking, which is why nothing looks
broken.

The margin is only half the result, and the other half is why this is
structural rather than a tuning problem. It is also **distribution-free**:
for a weighted threshold `s'`, the expected number of competitors
outranking a hit is `n · (1 − F(s'))` — *linear in corpus size* — so no
fixed weight ratio survives corpus growth, even away from the boundary
where the margin argument bites hardest. No choice of weight inside a
bounded score is a fix; the weight has to leave the score.

So such a pair takes **a stratum each**, where the fusion weight acts in
rank space — and rank space is what a stratum is. An unbounded score
carries a multiplicative weight at every magnitude (BM25F's field weights
are natively this), so that configuration belongs at the first exit. The
question a host can act on while reading the refusal is therefore: *do
you want these two weighted differently, and is the score bounded? Then
separate strata.*

**The input protocol is validated against what each producer declared,
not against one law applied to all of them.** A ranked producer states,
where it is registered, how its rows are ordered and whether an item may
repeat within one invocation. `fuse` is the consumer both declarations
were written for, so both reach it — carried from the registry through
the compiled unit and the executed stream rather than re-fetched at the
end — and each is honoured on its own terms. A producer that declares its
repeats are the consumer's to remove is **de-duplicated**: one
contribution per `(stratum, item)`, at the best rank the stream gave it,
never counted twice. A producer that declares an item appears at most
once is believed, and pays no per-stream identity set for the promise —
which matters, because that set is the one structure a fusion holds that
grows with the rows *pulled* rather than with the disagreement window, so
an honest uniqueness declaration is what makes a deep answer affordable.
Refusing the permissive declaration instead would be the mirror failure:
a policy whose own definition names the consumer's obligation, rejected by
the consumer for exercising it.

The ordering declaration reads the same way. Both spellings forbid a
contribution that rises with rank — the threshold over the stream heads
would otherwise not be an upper bound and certification would be unsound
— and they differ exactly on equality. A producer that declared its ranks
may tie is admitted when two adjacent ranks carry one contribution; one
that declared every rank unambiguous is refused, because that equality is
precisely the condition under which the fused sum can no longer separate
those ranks. It is the same claim the monotone-depth dimension above
refuses at admission, held one layer lower against a stream that reached
fusion without passing the waist.

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

A status per producer is not the same obligation as an *ending* per
producer, and reading the first as the second would quietly repeal §7.
A producer the top-k bound stopped mid-stream has a true status — read
down to this contribution and no further — and that is what the trailer
carries for it. Draining the stream instead, so that it can be made to
declare exhaustion, would spend the whole memory bound on the report and
would then assert a completeness the fusion never established. The
trailer reports the state each producer is in; it does not put producers
into a state so that it has something to report.

## 7. Fused is top-k by construction; unfused carries no cross-stratum accounting

The two rungs differ in kind, and the difference is algebraic rather than
an implementation budget. The discriminator is the **absence versus
presence of cross-stratum accounting**, and not a difference in how
results are produced.

Unfused enumeration is **materialized per stratum**. `execute` calls the
evaluator's `query_with_options_view`, receives a fully materialized
`SparqlResult::Solutions`, converts it to a `Vec<(u64, Term)>` and hands
back a `VecDeque` behind `RankedStreamImpl`. There is no windowed or
incremental execution to rest a claim on: the evaluator exposes no
cursor, stream or iterator surface at all, and is materialized at every
operator. So the unfused rung is bounded by what a stratum's own result
costs, not by the consumer's depth — a consumer that reads one row has
already paid for all of them. What the rung *does* give is that no
cross-stratum accounting exists: with no summation, per-stream properties
compose, each producer emits in its own rank order, and N streams are N
independent facts with N receipts. That, and not unboundedness, is the
property the rung is for. (Making enumeration incremental is separate,
larger work; this paragraph records what ships.)

Fused enumeration is inherently top-k, because §5 sums across strata: no
item can be emitted until it is known not to reappear in another
stratum's stream and raise its total. Contributions fall monotonically
with rank, so a threshold over the stream heads bounds how deep the
fusion must look to certify its next emission — certification is bounded,
and the frontier is the memory. *Complete* fused enumeration stays
excluded for its own reason, which survives any change to how streams are
produced: complete enumeration may not discard, so it must retain, and
retention is linear wherever it is put — emitted-set, frontier or spill.
This surface does not offer it as though it were free. A caller who wants
to walk everything wants the unfused rung, whose cost is one stratum's
materialized result.

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

## Open questions, and the answers the implementation records

This document deliberately left three questions to the implementation. All
three are now answered, and the answers are recorded here rather than left
as questions a shipped design record still asks.

1. **Where the composition layer lives — one crate.** The extension
   precedent said a sibling crate over the seam with `purrdf-sparql-eval`
   untouched; what it did not fix was whether that sibling is one crate or
   a `fuse` primitive plus a thin planner. It is **one crate**,
   `purrdf-retrieval`. Splitting it would put the admission waist on a
   crate boundary: `compile` re-derives the planner's own matching and
   placement decisions over a plan it must treat as untrusted, and it can
   only do that by running the *same* rule the planner ran. Two crates
   would be two copies of that rule, and a latent divergence between them
   is invisible to any test on either side — the exact failure the waist
   exists to prevent. The `fuse` primitive is still separable *as a
   surface*: it takes streams and a profile and knows nothing about
   planning, and a caller that wants only that rung calls only that rung.
2. **The reciprocal's scale, rounding and overflow bound — decided.** The
   scale is `purrdf-text`'s own, twelve fractional digits, reusing that
   crate's `Fixed` rather than defining a second base-10 type: two
   fixed-point types in one workspace would be two conventions for what a
   rounded reciprocal means. The rounding direction is **truncation toward
   zero**, applied to the reciprocal once, at the declared scale, before
   the weight is applied. The overflow bound is the profile's own admitted
   ceiling — the largest declared weight times the admitted contribution
   count — checked at profile construction, and every intermediate is
   checked at the point it is formed, so an intermediate that leaves the
   range is a loud refusal rather than a wrapped score. All three are part
   of the profile's canonical bytes, so a change to any of them is a new
   profile identity.

   The consequence that must be stated with them is the coupling between
   the weight, that scale, and the per-stratum depth. Truncating the
   reciprocal *before* weighting makes the reciprocal's own resolution a
   ceiling no weight can lift: at any weight of one or more, two adjacent
   ranks stop producing distinct contributions just above rank
   `sqrt(10^12) − K`, whatever the weight is, and below one the weight
   binds earlier. Beyond that depth nothing errors and nothing becomes
   nondeterministic — the declared tie-break is total — but the fused
   score stops separating ranks, so the sum across strata stops being
   rank-weighted. The bound is therefore an admitted dimension rather than
   a footnote: a per-stratum depth beyond it is refused at the admission
   waist whenever the environment names the profile the answer will be
   fused under.
3. **The request lattice is closed, and stays closed.** The modalities a
   request can name are not bounded by the producers that happen to exist
   in-tree — producers are caller-supplied configuration, and a registry
   that has nobody for a modality is a fact about that registry, not about
   the lattice. So the lattice carries the modalities the layer means to
   support, including ones no shipped producer accepts, and a term that
   reaches nothing is reported per term with a typed reason (§6's obligation
   applied to the request side).

   Given that, the enum is **closed** rather than `#[non_exhaustive]`, and
   the choice is deliberate. A closed enum gives a caller matching
   exhaustively a compile error when a modality is added, which is exactly
   the signal a caller routing terms to its own producers wants;
   `#[non_exhaustive]` replaces that signal with a wildcard arm that
   silently swallows the new modality, and charges every caller a wildcard
   arm forever in exchange. The semver relief it buys is worth less than
   the signal it costs, and it is cheapest to buy nothing: adding variants
   before first publication is free, so the modalities that are foreseeable
   are added now rather than deferred into a later major version.
