<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# The ranked-producer contract

What a ranked producer owes this layer, and who holds it to each obligation.

A ranked producer is host Rust: a
[`PropertyFunction`](purrdf_sparql_eval::PropertyFunction) registered through
[`register_ranked`](purrdf_sparql_eval::PropertyFunctionRegistry::register_ranked)
with a [`RankedDeclaration`](purrdf_sparql_eval::RankedDeclaration) describing
how its rows rank. The layer above it plans over those declarations, compiles
them to SPARQL, runs them, and fuses the results into one ordered answer. Every
number in that answer is a function of what the producers said about themselves.

## Two kinds of obligation, and why the difference is printed on every entry

Some of what follows is **checked**. The layer measures the rows as they arrive
and a breach is a named refusal — [`ProtocolError`],
[`AdmissionError`], [`ExecutionError`]
— that names the dimension that failed and the stratum that failed it. A
producer cannot make the layer unsound in these dimensions; it can only make its
own call fail.

The rest is **believed**. No consumer-side measure can see inside host code, and
a breach is a wrong answer rather than an error. Those entries say so plainly and
name the test in this repository that proves the shipped producers keep the
promise, where one exists. Where no such test exists, the entry says that too:
an unverifiable claim in a contract document is worse than no claim.

A third property cuts across both and is stated once here rather than sixteen
times. **A promise about rows nobody has pulled is verified exactly as far as the
rows actually pulled reach, and no further.** A false declaration that no pulled
row contradicts produces a wrong answer, and this layer does not dress that up as
a proof. The alternatives are reading a stream to its end, or *asking* it about
one candidate where its registration licensed the question
([A15](#a15--declare-candidate-domains-and-never-name-a-candidate-outside-them));
the first is the precise cost several of these declarations exist to avoid, and
the second is an observation, verified exactly as far as the rows that later
contradict it.

## The obligations

| # | Obligation | Enforced by |
|---|---|---|
| [A1](#a1--memory-is-odepth-never-ocorpus) | Memory is `O(depth)`, never `O(corpus)` | producer |
| [A2](#a2--filters-apply-during-selection-before-rank-assignment) | Filters apply during selection, before rank assignment | layer, per row |
| [A3](#a3--filters-express-eligibility-never-relevance) | Filters express eligibility, never relevance | producer |
| [A4](#a4--scoring-statistics-are-computed-over-the-indexs-declared-scope) | Scoring statistics come from the index's declared scope, never the filtered subset | producer |
| [A5](#a5--duplicate-fan-in-is-collapsed-inside-the-producer) | Duplicate fan-in is collapsed inside the producer, and the stream then declares uniqueness | both |
| [A6](#a6--only-an-o1-from-index-cardinality-may-be-projected) | Only an `O(1)`-from-index cardinality may be projected | producer |
| [A7](#a7--a-projected-cardinality-is-an-attribute-never-an-aggregate) | A projected cardinality is an attribute, never an aggregate | producer |
| [A8](#a8--capability-declarations-are-contracts-cardinality-declarations-are-estimates) | Capability declarations are contracts; cardinality declarations are estimates | layer for modes, producer for the bound |
| [A9](#a9--declare-the-honest-unfiltered-worst-case-for-the-row-bound) | Declare the honest unfiltered worst case for the row bound | producer |
| [A10](#a10--the-engine-pushed-ceiling-is-honoured-for-efficiency-only) | The engine-pushed ceiling is honoured for efficiency only | engine withholds, producer honours |
| [A11](#a11--volatility-must-be-true-of-the-snapshot-held) | Volatility must be true of the snapshot held, and the snapshot is pinned for the query | producer |
| [A12](#a12--bounds-narrow-they-never-zero) | Bounds narrow; they never zero | layer, three times; recorded once |
| [A13](#a13--attest-the-generation-of-the-snapshot-that-answered) | Attest the generation of the snapshot that answered | producer declares, layer carries |
| [A14](#a14--declare-incompleteness-rather-than-refusing-or-faking-exhaustion) | Declare incompleteness rather than refusing or faking exhaustion | producer declares, layer refuses an unrecordable one |
| [A15](#a15--declare-candidate-domains-and-never-name-a-candidate-outside-them) | Declare candidate domains, name each row's block, and never name a candidate outside them | both, per row |
| [A16](#a16--declare-the-fidelity-of-the-rows-you-can-name) | Declare the fidelity of the rows you can name, on both axes, with evidence for any loss | producer declares, registration refuses an empty disclosure |

[A9](#a9--declare-the-honest-unfiltered-worst-case-for-the-row-bound),
[A10](#a10--the-engine-pushed-ceiling-is-honoured-for-efficiency-only) and
[A14](#a14--declare-incompleteness-rather-than-refusing-or-faking-exhaustion)
are three instances of one doctrine, and
[reading them apart loses the argument](#a9-a10-and-a14-are-one-doctrine-stopping-early-and-running-out-are-the-same-empty-cursor).

---

## A1 — Memory is `O(depth)`, never `O(corpus)`

**The obligation.** An invocation's working set is bounded by the depth it was
asked for, not by the corpus it ranks. A top-k selection keeps a heap of at most
`depth + 1` entries — push, and pop the worst once the heap is over the bound —
and the heap grows only as candidates actually arrive.

**The failure it prevents.** A planning number becoming an allocation.
`with_capacity(depth)` is the exact point at which it happens: the depth is
arithmetic derived from declarations and statistics before a single row exists,
so a plan that legitimately asks for a million ranks reserves a million slots
whether the index holds a million documents or a hundred. It is forbidden.
Reserving against a count the producer has actually measured is a different act
and is fine.

**Who enforces it.** The producer. The layer sees only rows and cannot observe
an allocation inside host code.

The shipped lexical producer is the exemplar: its bounded selection sizes the
heap `keep.min(candidates.len()) + 1` — the `min` is what keeps the planner's
number from reaching the allocator, so a ten-row answer over a million-document
corpus holds eleven entries at a time and sorts ten at the end. The consumer half
of the same discipline is measured rather than asserted, in
`tests/fusion_frontier_alloc.rs`: `the_frontier_peak_tracks_the_profile_bound_and_not_the_stream_length`
and `a_unique_declaration_is_what_keeps_a_deep_answer_affordable`.

## A2 — Filters apply during selection, before rank assignment

**The obligation.** Every eligibility filter the producer owns is applied while
candidates are being selected, so ranks are assigned over the set that survives.
The rank law that follows is absolute and identical for every ranked stream there
is: **1-based, contiguous, ascending** — rank 1, then 2, then 3, with no gap, no
repeat and no step backwards. Rows the producer scores equally still take distinct
consecutive ranks under whatever total tie-break it applies.

**The failure it prevents.** Ranking the whole index and filtering afterwards
leaves gaps — rank 1, then 4, then 9. A gap is not a cosmetic defect: fusion
computes the contribution from the rank, so a gap either loses a contribution the
answer was owed or states a position the row does not occupy. Fusion refuses a
gap rather than closing it, because closing it would be the consumer inventing
ranks the producer never assigned.

**Who enforces it.** The layer, per row.
[`FusionStream`] holds the next rank it expects from each
stream and measures every arriving row against it: a lower rank is
[`ProtocolError::OutOfOrderRanks`], a
higher one is
[`ProtocolError::NonContiguousRanks`].
Both sides are pinned by `a_backwards_rank_and_a_skipped_rank_are_refused_while_their_valid_neighbours_fuse`
in `tests/fusion.rs` — the conforming neighbours fuse in the same test, so the
refusal is not over-broad.

The producer side of it is visible in the shipped lexical producer, which pushes
its language constraint and its bound-subject restriction into the partition
filter *before* ranking runs. That is sound for a stated reason rather than by
luck: ranks are per-partition, so dropping whole partitions cannot move the rank
of any row that survives.

### A bound output position is not one of these filters

This is the clarification the code forced, and it is easy to read the rule past.

"Filter" in A2 means the **producer's own eligibility filters** — the ones inside
selection that decide which rows exist at all. A **bound output position** is a
different thing entirely. Under the seam's generate-then-filter model a relation
is entitled to emit candidates and let the engine drop the rows that disagree with
the values it was handed (see [`PfRow`](purrdf_sparql_eval::PfRow)), so a caller
that constrains a score, a match flag or the candidate itself is applying an
**engine-side value filter**, not asking the producer to re-rank.

The shipped lexical producer shows the whole shape. Its `?doc`, `?score` and
`?matched` positions are invisible to the ranker, so a call binding any of them is
post-rank filtered, and three things follow:

* `open` **withholds the engine's ceiling** from the ranker for such a call
  (`select_ceiling` becomes `None` when any post-rank position is bound), and the
  ranking is computed in full. Handing the ceiling down instead would truncate the
  ranking first and let the cursor filter a prefix — it could then emit fewer than
  `k` rows and report exhaustion while matching rows sat beyond the truncation,
  which the engine reads as a complete answer.
* The cursor applies equality on **every** bound position and decrements the
  licence **only on rows it actually emits**, because a row it skips is one the
  engine would have dropped anyway and counting it would be the miscount the
  ceiling contract warns about.
* **Rows keep their search ranks.** Nothing is renumbered. A rank-two row bound at
  `?doc` still arrives as rank two under a ceiling of one, which is what
  `a_ceiling_with_a_bound_doc_still_emits_the_matching_row` pins.

A bound `?lang` or a bound `?rank` are the opposite case and are handed down
freely: the first drops whole partitions before ranking and the second selects one
per-partition position, and neither can drop a row the cursor would have emitted,
so the ceiling stays valid on those paths.

**The retrieval unit never binds such a position.** The ladder's compiled unit
projects the candidate from
[`RankedDeclaration::candidate_position`](purrdf_sparql_eval::RankedDeclaration)
as a free variable and renders only the placements the declaration names — and
`register_ranked` refuses a declaration whose candidate position is also a
placement or a depth target, because a position filled with a constant cannot
also be the projected candidate. Both shipped
producers' own declaration builders place their input facets and nothing else, so
a unit compiled from either leaves the post-rank output positions free and the
post-rank path is never taken from inside the ladder. It is reachable only by a
hand-written SPARQL call, which is exactly the caller the withholding logic above
exists for.

## A3 — Filters express eligibility, never relevance

**The obligation.** A filter answers *may this row be in the answer at all*. It
never answers *is this row less good*.

**The failure it prevents.** Reciprocal-rank fusion discards score magnitude
entirely. A row's contribution is a function of `(decay rule, K, weight, rank)`
and of nothing the producer supplies —
[`FusionStream`] recomputes it and refuses any other number
as
[`ProtocolError::ContributionMismatch`].
So a filter meaning "this row is less good" does not demote the row. It deletes
it, and every row behind it moves up one rank and collects a *larger*
contribution than it had earned. The distortion is invisible: the stream is
well-formed, the ranks are contiguous, and the answer is plausible.

**Who enforces it.** The producer. No test in this repository proves it, and none
can: the layer sees a contiguous stream either way and has no way to distinguish a
row that was ineligible from a row the producer merely thought poor. A relevance
judgement belongs in the score that decides the rank, where fusion's weights can
act on it.

## A4 — Scoring statistics are computed over the index's declared scope

**The obligation.** The corpus-shaped inputs a score depends on — the document
count, the average document length, a term's document frequency — are read from
stored index state for the scope the score is defined over. They are never
recomputed over the subset a request's filters happened to leave.

**The failure it prevents.** A document's score becoming a function of an
unrelated constraint. If the inverse document frequency were computed over the
filtered candidate set, adding a language restriction to a query would move the
score of a document the restriction never touched, and two rows of one answer
would stop being comparable — they would have been scored against different
vocabularies while being printed side by side.

**Who enforces it.** The producer.

The shipped lexical producer fixes the scope up front rather than letting a
request narrow it: the scope is a `PartitionKey`, the `(graph, language)` pair,
and every corpus-shaped BM25 input is read from that partition's stored
statistics — [`partition_stats`](purrdf_text::TextIndex::partition_stats),
[`document_frequency`](purrdf_text::TextIndex::document_frequency) — rather than
from the candidate set a filter produced. That the scope is the partition and not
a pooled corpus is pinned by `statistics_are_per_partition_not_pooled` in
`crates/text/tests/index.rs`, and that ranks follow the same scope by
`rank_is_per_partition_not_global` in `crates/text/tests/scoring.rs`. Pooling is
rejected for the mirror-image reason: it would make an English needle's inverse
document frequency a function of how much Japanese happened to sit beside it.

## A5 — Duplicate fan-in is collapsed inside the producer

**The obligation.** A producer whose internal structure can reach one entity by
several routes — one route per needle term, one per occurrence, one per matrix row
where two rows carry one vector — collapses those routes itself, and *then*
registers [`DuplicatePolicy::Unique`]. The collapse
carries a test rather than a comment.

**The failure it prevents.** The declaration is the one the layer actually
*spends*. A `Unique` stream is not charged a per-row identity set, and that set is
the one structure in fusion that would grow with the rows pulled rather than with
the disagreement window. The saving is real, which means the belief is real: until
something drives a producer over data built to break the promise, "Unique" is a
comment.

**And it is what buys the bounded read.** A memory saving is not the whole of what
this declaration spends. `Unique` is the promise that makes a count of ranks a
count of candidates, and a bounded request's per-stratum depth of `k` is derived
from exactly that identity: a candidate its stratum ranks past `k` is beaten by the
`k` **distinct** candidates above it, and "distinct" is this obligation. An
`Allowed` stream is de-duplicated by the consumer, so a repeat is validated,
charged to the producer and then discarded — a depth-`k` prefix of it carries `k`
rows and can carry fewer than `k` candidates, which is no longer a superset of the
top `k`. So the planner narrows a depth only where every surviving stratum
declared `Unique`, and an `Allowed` stratum keeps the full declared-or-measured
depth it has always read, answering exactly as it does for a request that states no
bound at all. A producer that collapses its fan-in and declares `Unique` therefore
buys the bounded read for the whole request; one that cannot loses nothing but
reading. Pinned by `the_unique_neighbour_keeps_its_bounded_read_and_the_answer_it_already_gave`
and `an_allowed_stratum_keeps_the_read_its_repeats_need_and_answers_as_the_undeclared_one_does`
in `tests/search.rs`, which assert the two answers row for row rather than
comparing depths alone.

**Who enforces it.** Both.

The layer refuses a breach as
[`ProtocolError::DuplicateItem`], naming the
entity **and** the stratum, because which stream broke its promise is the
actionable half. The refusal holds for the whole fusion and not merely for the
frontier: a candidate that leaves the frontier by certification leaves behind the
set of streams that named it, so a stream naming it again afterwards is still
refused. A stream that declared `Allowed` is unaffected — its repeat is dropped
before it becomes a head, which is what that declaration asks a consumer to do.

The shipped producers are proved over corpora built to tempt a repeat, in
`tests/real_producers.rs`:

* `the_text_producer_names_each_document_once_over_a_corpus_that_tempts_a_repeat`
  — a document holding every needle term twice over, so an implementation walking
  a term at a time would name it three times and one walking occurrences six. The
  corpus is *shown* to be tempting rather than assumed to be: each needle term is
  additionally run alone and reaches that document.
* `the_knn_producer_names_each_target_once_when_two_rows_share_one_vector` — two
  distinct terms carrying byte-identical vectors and a third a hair away, tying
  exactly and landing at adjacent ranks, so an implementation keying its emitted
  set on the vector or the distance rather than on the target would collapse or
  repeat them.

Both assertions are two-sided — the candidates must be distinct **and** the call
must have succeeded — because a refusal would also leave no duplicate in the
answer, and "kept the promise" must not be readable as "was caught". The
refusal side of the lexical producer's own guard is pinned by
`the_text_producer_declares_unique_only_where_one_subject_can_appear_once`: an
index of more than one partition, where one subject really can appear in several,
hands out no ranked declaration at all.

## A6 — Only an `O(1)`-from-index cardinality may be projected

**The obligation.** A count a producer puts in an output position must be one the
index already holds, readable without walking rows.

**The failure it prevents.** A walked count is truncated by exactly the bound that
made the read affordable. A count derived from the rows *this* invocation emitted
is a count of the rows the depth allowed, reported as though it were a property of
the data — and it shrinks when a caller lowers a limit, which no property of the
data does. A stored count is structurally immune, because no walk produced it:
there is nothing for a bound to cut.

**Who enforces it.** The producer. No test in this repository proves it, because
no shipped producer projects a corpus cardinality at all. The reads that would
qualify exist and are named:
[`document_frequency`](purrdf_text::TextIndex::document_frequency) is the length
of a stored posting list, read without walking it, and
[`partition_stats`](purrdf_text::TextIndex::partition_stats) is a stored record.

The layer's nearest analogue is
[`rows_per_invocation`](purrdf_sparql_eval::PropertyFunction::rows_per_invocation),
held to the same honesty contract as
[`cardinality_estimate`](purrdf_core::DatasetView::cardinality_estimate): an upper
bound the relation actually respects, not a guess, because a bound that
under-states reality turns an admission decision into a wrong one.

## A7 — A projected cardinality is an attribute, never an aggregate

**The obligation.** A count in an output position describes the row it sits on. It
is never an answer to a SPARQL count.

**The failure it prevents.** A SPARQL count is defined over **solutions** — after
the joins, filters and duplicate elimination that happen above the relation and
that the producer cannot see. A relation emitting "the count" would be answering a
question about a solution sequence it has no access to, and a caller writing an
aggregate over the same pattern would hold two different numbers with nothing in
either to say which was which.

**Who enforces it.** The producer. The engine has no way to tell an honest
attribute from a usurped aggregate; both are just an integer in a row.

The shipped lexical producer's `?matched` position is exactly the right shape:
*how many distinct needle terms **this document** holds* — an attribute of the
row, computed while that row is scored. It behaves like one, too. It binds
nothing, because arbitrarily many documents can share a matched-term count, and it
is filtered after ranking like any other post-rank position
(see [A2](#a-bound-output-position-is-not-one-of-these-filters)).

## A8 — Capability declarations are contracts; cardinality declarations are estimates

**The obligation.** [`modes`](purrdf_sparql_eval::PropertyFunction::modes) and
[`rows_per_invocation`](purrdf_sparql_eval::PropertyFunction::rows_per_invocation)
are declared one line apart in the same trait and are read completely differently.
A mode is a **contract**: an invocation whose binding pattern no declared mode
subsumes is not served, full stop. A row bound is an **estimate** in the sense
that nothing can check it against the index — but it is an *upper bound the
producer must actually respect*, consulted to order the call against the other
operators of its group and to admit it against a ceiling.

**The failure it prevents.** Reading a mode as an estimate means attempting a call
that cannot be computed. Reading a row bound as a contract means admitting a plan
against a number nothing holds anyone to.

**Who enforces it.** Split.

For modes, the layer.
[`admits`](purrdf_sparql_eval::PropertyFunction::admits) is *provided* rather than
required, because the subsumption rule is the binding lattice's and a relation
that could restate it could also restate it wrongly. The retrieval admission waist
then re-derives the planner's own matching and placement decisions over a plan it
treats as untrusted, so a hand-edited plan cannot bind a producer to a call its
declarations do not serve.

For the row bound, the producer. The layer checks the recorded depth *against* the
declaration — [`AdmissionError::DepthBoundViolation`],
pinned by `a_declared_row_bound_still_refuses_a_raised_depth` and
`a_ghost_stratum_is_refused_above_its_bound_and_refused_again_at_zero` in
`tests/admission_tests.rs` — and cannot ask your index what its real worst case
is, because nothing in the seam answers that question.

It can, however, catch the declaration being beaten where it was about to be
relied on. A plan whose depth sits at your declared bound is emitted with a probe
row one past it, so if the row your declaration ruled out shows up, the run is
refused (`ExecutionError::RowBoundBreached`) instead of reported exhausted; see
[A9](#a9--declare-the-honest-unfiltered-worst-case-for-the-row-bound). That is
narrower than checking the declaration — it says nothing about a bound never
planned to — and it is exactly the case where being wrong would have cost a
consumer a false completeness claim rather than a bad join order.

**On declaring many modes.** The subset direction is the useful one: a relation
that can serve object-bound/subject-free can also serve both-bound, by producing
the former's rows and letting the engine's equality filter discard the mismatches.
An index-informed producer can therefore declare many modes precisely because its
indices serve binding directions a scan cannot, and a relation that can serve
everything declares exactly one all-free mode that subsumes every pattern of its
arity. Neither shipped producer takes that latitude, and both are right not to:
the lexical search relation's general mode is `fbffff` and both nearest-neighbour
relations' is `fbbf`, because each has an input position it genuinely cannot
enumerate — it retrieves documents for a needle and cannot enumerate needles for
a document, and it retrieves neighbours for a seed and cannot enumerate seeds.
Declaring narrowly is the honest move when the index really is directional.
Declaring broadly is the honest move when it is not. Neither is a default.

The lexical search relation declares a *second* mode, `bbffff`, and it is worth
reading for what it is not. It widens nothing — `fbffff` already subsumes it, so
no call becomes feasible by its being there — and it is not a second spelling of
the same promise. It states a different fact about the same call: that binding
the document is a **point lookup**, with the row bound declared for it, which is
the pair `register_ranked` reads before it admits an exclusion basis — the
declared answer to *do you hold this candidate*, which fusion asks once per
frontier candidate and must therefore never turn into a scan. Declare such a
mode when the candidate-bound call really is a
lookup, and do not declare one when it is a scan the engine's equality filter
happens to narrow — see
[A9](#a9--declare-the-honest-unfiltered-worst-case-for-the-row-bound) for why
the bound beside it is the one you will be held to.

There is a second condition, and it is the one that decides whether a basis is
declarable at all: **the lookup must arrive in the mode that answers it.** An
exclusion lookup is your own call with the candidate bound as the parameter of a
prepared execution, and that execution is prepared once per stratum. The
candidate is therefore a *variable* in the text at the moment the call is
admitted, and whatever mode that admission selects is the mode your relation is
invoked in.

For the lexical relation this is settled by its general mode alone: `fbffff`
leaves every position but the needle free, so a candidate-bound call has one
meaning and `bbffff` is what it arrives in. For a producer that declares a
`DepthPlacement` it is not, and the difference is decisive. If the lookup were
rendered the way the ranked read is, the depth would be bound, and the pattern
reaching the relation would be exactly the pattern an ordinary ranked call
arrives in — with no signal by which to tell the two apart. The answer would then
be the answer to *is this candidate among your best n*, whose absences are **not**
exclusions: a candidate outside the best n is one the producer may still name at
rank n, and a consumer that read that absence as an exclusion would refuse the
fused read as `ExclusionContradicted` the first time one arrived.

Two things together keep that from happening, and a producer with a depth
placement should know both, because its basis rests on them.

**The exclusion unit renders no depth.** [`compile`] leaves the depth position a
free variable in the lookup's text — and *only* in the lookup's text; the
streaming unit still carries the depth the plan derived. A depth is an offer, and
a lookup is not asking for an offer to be filled.

**The candidate is declared to the prepare, not merely substituted.** The
consumer prepares the lookup through
[`prepare_execution`](purrdf_sparql_eval::NativeSparqlEngine::prepare_execution),
naming the candidate variable as the execution's parameter, and the feasibility
pass treats it as bound at the point the binding really lands. So the admission
pass sees the candidate-bound, depth-free pattern, and that is the mode your
relation receives: the bound candidate is written into your call's candidate
position, so each lookup is one point read rather than a scan a join narrows.
The declaration is **enforced**: an execution refuses to run, by name, while
its parameter is unbound — because running it with the position free would
invoke a relation in a mode nobody declared, which is the whole thing the
promise was traded for.

Both shipped vector relations sit on exactly that footing. Each declares the
count-free membership mode `bbff` beside its general `fbbf`, each answers it by a
binary search over its own term universe with no ranking, no vector read and no
graph traversal, and each declares `ExclusionBasis::Membership`. For the
approximate one that basis is admitted *despite* an unconditionally
`Completeness::Lossy` declaration, and deliberately so: a term the matrix holds
no row for is a term no beam reaches at any `ef`, so the verdict is a fact about
the matrix rather than about what the search found. `ExclusionBasis::Search` is
still refused from it, because there "not found" and "not present" really do
differ.

Declaring several modes also decides which row bound your read is held to, because that
bound is a function of the mode — see
[A9](#a9--declare-the-honest-unfiltered-worst-case-for-the-row-bound).

---

## A9, A10 and A14 are one doctrine: stopping early and running out are the same empty cursor

The seam has exactly one shape for "no more rows": the cursor returns `None`.
Nothing the engine can observe on the way out of a drained invocation says whether
the relation ran out of rows or stopped because it was allowed to. That single
fact is why the row ceiling is documented as a licence rather than a contract, and
it is why [`ServiceLevel`] has no `Whole` variant and never
will: a relation that stopped at the ceiling is not incomplete — it answered the
question it was licensed to answer, in full — and it equally could not certify
wholeness, because it never looked at the rows it was licensed to skip.

The distinction is worth real money to a consumer, so the layer buys it three
separate ways rather than asking the seam for it once. **The engine withholds a
ceiling it cannot account for. The executor reads one row past a bound it
recorded. The producer declares a shortfall only it can know.** Read as three
unrelated rules they look like three pieces of bookkeeping; read together they are
one answer to one question, bought at three different altitudes because no single
altitude can answer it.

The layer's own purchase — the middle one — is the depth probe.
[`compile`] emits `LIMIT depth + 1`, so a
unit whose producer still had rows past the planned depth hands back one more row
than its stratum may contribute. That row is a **probe**: never emitted onto the
stream, never ranked, present in no plan field and no identity, and counted in
exactly one number — the rows a read materialized, which asks what the read cost
rather than what the answer is made of, and which would understate the read by
exactly this row if it left it out. All the probe *decides* is
[`ProducerStatus::DepthReached`] versus
[`ProducerStatus::Exhausted`]. Without it an
executor could only ever say `Exhausted` — the one ending that names no
stopper — about a read the plan itself had cut short.

Your declared row bound does **not** cap that emitted bound, at any size. The
admission waist has already refused a recorded depth above your declaration, so
`min(depth, declared)` selects the depth for every unit that can be emitted, and
writing the `min` anyway would be a guard firing for no reachable plan. Its one
consequential case was a bug rather than a saving: at the depth that already equals
your declared bound — the depth an under-declaring producer lands a plan on —
capping the bound erased the slot, and `Exhausted` became a guess at exactly the
depth where it matters. A declared **zero** is the same bug one size smaller: the
planner floors that depth at one, and a bound capped to the declaration was then
`LIMIT 1` *equal* to the depth, so no row past it could arrive and the read was
certified exhausted however many rows the index turned out to hold. A zero
declaration is read rather than obeyed everywhere else in this layer
(see [A12](#a12--bounds-narrow-they-never-zero)), so it is read here too: the
emitted bound is `depth + 1` there as well, an empty index reports
`Exhausted { rows_emitted: 0 }` as a *verified* claim, and an index that turns out
to hold rows breaches its declaration by name exactly as a wrong declaration of any
other size does.

There is exactly one depth for which the slot could not be arranged — the top of the
32-bit rank range, where the row past the depth is not a number a `LIMIT` can hold
— and no unit is ever emitted there. Saturating would have emitted a bound equal to
the depth, so no probe could arrive and the read would be certified `Exhausted`
however many rows your relation still held: the `LIMIT 0` fault at the other end of
the range. The planner records a derived bound past that ceiling **at** the ceiling,
where the probe row still fits and `DepthReached` can still say the planned depth
stopped the read; it refuses a *request* bound past it, because that number is the
caller's own and asks for something unrepresentable
(`PlanError::ReadBoundBeyondDepthRange`); and the waist refuses such a depth in an
edited plan (`AdmissionError::DepthWithoutProbe`, the mirror of
`AdmissionError::ZeroDepth`), as does `StratumUnit::new` in a hand-built bundle
(`UnitError::DepthWithoutProbe`).

Declaring more rows per invocation than a read can be taken to is therefore **not**
a refusal, at any size up to and including the genuinely unbounded `u64::MAX`. It is
an honest description of a large index, the read the caller asked for may be a single
page of it, and the shapes that cannot narrow a depth to that page —
`DuplicatePolicy::Allowed`, or two or more strata none of which declares a block —
are ordinary rather than exotic. Refusing them would have left you two ways out and
both are dishonest: under-declare `rows_per_invocation` against
[A9](#a9--declare-the-honest-unfiltered-worst-case-for-the-row-bound), or invent a
cardinality statistic. Such a declaration is recorded at the deepest readable depth
with the probe row one past it, so the read ends `DepthReached`, which names the
planned depth as the stopper and claims nothing about the rows below it. Pinned by
`a_declared_row_bound_past_the_read_range_is_planned_at_the_ceiling`,
`a_small_top_k_over_an_oversized_declaration_plans_admits_and_runs`,
`a_bound_gives_an_unbounded_declaration_a_finite_depth_and_its_absence_reads_to_the_ceiling`
and `a_read_bound_no_rank_can_address_is_refused_and_the_addressable_ones_plan` in
`tests/compile_request.rs`, by
`an_unbounded_stratum_without_statistics_reads_to_the_ceiling` in
`tests/planner_golden.rs`, and by
`a_recorded_depth_that_cannot_carry_its_probe_row_is_refused_at_the_ceiling` in
`tests/admission_tests.rs`, each of which executes the depth one rank shallower and
requires it to plan, admit and compile with its probe row present.

The slot is not an over-refusal either, because it costs an honest producer
nothing. A producer that declared `n` and really holds `n` returns `n` rows into
an `n + 1`-row bound, the slot comes back empty, and `Exhausted` is *verified*
rather than believed. A row arriving in it is the producer yielding an `n + 1`-th
after promising there is none, and the executor refuses the run by name
(`ExecutionError::RowBoundBreached`, carrying the stratum, the declared bound, the
access mode that bound was read at and the count actually returned) rather than truncating to the depth and certifying
the remainder as completeness.

**If you declare a depth placement, the number you receive is bounded
differently.** A `LIMIT` is a ceiling the evaluator applies to a cursor you never
hear about, so probing one row past your declaration there costs you nothing. The
depth argument is a *request you read*, and a request for `declared + 1` asks you
to exceed the bound you registered — which a producer with a configured ceiling
should refuse, as the nearest-neighbour relation refuses a `k` above its guard.
So the argument carries `max(1, min(depth + 1, declared bound))`: the probe where
your declaration leaves room, your declaration itself where it does not. Your
unit's own `LIMIT` still sits one row past the declaration either way, so a
self-bounding producer that returns more rows than it declared is still caught —
you are simply never asked to produce them.

**That class of producer has an ending of its own, because at one depth it cannot
be probed at all.** Where the depth already sits on your declaration the two
numbers coincide: you are asked for exactly `depth` rows, you return at most
`depth` rows, and the slot past the depth can never be filled however many rows
your index holds. How that read ended is then **genuinely unobservable**, and
neither neighbouring ending is true of it — `DepthReached` would blame a planned
depth that cut nothing, and `Exhausted` would claim your rows ran out on the
strength of your own registration. So it is reported as
`ProducerStatus::RowBoundReached`, which names the stopper it really had: the row
bound you declared. It is reported **only** there. A depth below your declaration
still receives the probe and still ends `DepthReached` or `Exhausted`, and a read
that returned fewer rows than it was allowed is `Exhausted`, verified, because it
stopped before anything stopped it.

Raising the argument to go looking is not available and is not an oversight: it is
precisely the request a conforming relation must refuse. If you want that read taken
further, raise your declared row bound — re-planning deeper cannot help, because
the planner's `derived_bound` derives every depth downward from the declaration.
Pinned by
`the_probe_separates_a_cut_read_from_an_exhausted_one` and
`an_under_declared_row_bound_is_refused_and_an_honest_one_is_not` in
`tests/execute_dataset.rs`, by
`the_probe_row_changes_the_ending_and_nothing_else`,
`a_row_past_the_declaration_is_refused_and_an_honest_one_is_not` and
`a_self_bounding_producer_at_its_declaration_reports_the_bound_that_stopped_it`
in `src/execute.rs`, by
`a_self_bounding_producer_reports_the_bound_that_stopped_it_and_still_catches_a_wrong_one`
in `tests/compile_request.rs` — whose four arms hold the unobservable ending, the
wrong declaration the unit's own bound still catches, the short answer that really is
exhaustion, and the depth below the declaration where the probe returns — and by
`the_neighbour_count_stays_inside_the_guard_and_the_ending_says_which_bound_stopped_it`
in `tests/real_producers.rs`, which drives the shipped relation end to end.

`Exhausted` is the only ending that names no stopper. Every other one says who
stopped the read.

It is not, on its own, a completeness claim. It says the producer emitted every
row **its search produced**; whether those were every row that was **due** is
[A16](#a16--declare-the-fidelity-of-the-rows-you-can-name), reported beside it as
`FusionTrailer::fidelities`. For a producer that declared itself exhaustive the
two together do say the corpus ran out. For one that declared a lossy search they
say only that the search did.

## A9 — Declare the honest unfiltered worst case for the row bound

**The obligation.** [`rows_per_invocation`](purrdf_sparql_eval::PropertyFunction::rows_per_invocation)
reports the most rows one invocation can emit under that mode, measured against
the index with no request's filters applied. A genuinely unbounded generator
declares `u64::MAX`, and nothing asks it for a cardinality statistic to go with
that: the planner records such a stratum at the deepest depth a read can be taken
to, one row past which the probe still fits, so the read's own ending reports that
the planned depth stopped it.

**The failure it prevents.** The two directions fail differently, and that
asymmetry is the whole guidance.

Under-declaring **narrows your own reads**, and the waist refuses a recorded depth
above the declaration, so an under-declaration caps the read at a number your index
could have beaten. It is a configuration error a host will find.

What matters more is the depth that cap lands on. A plan whose depth lands *at* your
declared bound is admitted — it asks for no more than you promised — and for a
producer the evaluator bounds, the emitted probe row therefore sits one past your
declaration. If your index really does stop where you said, that row never
arrives and your stratum is reported
[`ProducerStatus::Exhausted`], now verified rather
than assumed. If it does not, the row arrives, and the run is refused by name with
your declared bound, the mode that bound was read at and the count actually returned
in the message (`ExecutionError::RowBoundBreached`). Either way the mistake is *said*, and nothing
is reported exhausted on the strength of a bound you got wrong.

**If you declare a depth placement, that probe is not available at this depth, and
the ending says so instead of guessing.** You are handed the depth rather than
bounded by it, and the number you are handed is never raised past your declaration —
so at a depth sitting on it you return exactly `depth` rows and no further row can be
asked for. A wrong bound there is not caught, because there is nothing to catch it
with; what the layer does instead is refuse to pretend. Your stratum is reported
`ProducerStatus::RowBoundReached`, which names your declared bound as the stopper and
claims nothing about what lies below it, and `Exhausted` is never minted from your
declaration. The obligation this places on you is the one A9 already states: measure
the bound from the index. The layer's promise is narrower and exact — it never
reports exhaustion on the strength of a bound you got wrong; it reports that it read
to your bound and stopped.

**If you declare several modes, the bound that binds is the tightest of the ones
serving the call.** This obligation is per mode, and A8 tells you to declare many, so
"your declaration" has to name one number. The layer reads the bound at the mode your
producer is invoked in: the tightest declared under any mode that subsumes the
invocation, which is the same lattice rule placement admitted the call by. A producer
serving through a subsuming mode emits at most that mode's rows, since the extra
bindings only filter, so every subsuming mode is a valid bound and the smallest is the
promise you actually made about this read. The number you are handed, the depth the
waist admits and the breach the executor refuses all use that one number, computed in
one place — never the widest mode you happen to have declared elsewhere.

So declaring `3` rows with a position free and `100` with it bound holds the bound call
to `3`: the free mode covers it, and the `100` over-declares a read its own coarser mode
already bounds. Because the binding number is therefore routinely declared at a mode the
call is *not* made under, both refusals that name a row bound
(`AdmissionError::DepthBoundViolation` and `ExecutionError::RowBoundBreached`) name the
mode they read it at, and say so when that mode is not the invoked one — so the figure
in the message always points at a declaration you wrote.

This is what makes A8's reading of `rows_per_invocation` hold on this path rather
than needing an exception carved out of it. The number remains an estimate in
A8's sense — nothing can check it against your index, and it may be wrong without
your producer being incorrect, buying you a bad join order and nothing worse. What
the probe adds is that where the layer would otherwise have had to *rely* on it to
know it had seen your last row, it no longer does: it goes and looks.

Over-declaring is **silent** and costs a worse join order, because the bound is
what orders a call against the other operators of its group: a call that emits at
most one row belongs before one that emits thousands, and a producer claiming
`u64::MAX` when it holds forty rows will be scheduled as though it were a
firehose.

**Who enforces it.** The producer. The shipped lexical producer measures its
bounds from the index once, at construction, rather than guessing them — which is
also why an index rebuild moves the bound (see
[A13](#a13--attest-the-generation-of-the-snapshot-that-answered) for the
consequence that has for a plan's identity).

## A10 — The engine-pushed ceiling is honoured for efficiency only

**The obligation.** The ceiling
[`open`](purrdf_sparql_eval::PropertyFunction::open) receives is permission to
stop early so that a generator can bound its own work. Correctness never depends
on it: the engine stops consuming at its own ceiling regardless, so a relation
that ignores the argument entirely is correct, merely less efficient. And it never
sizes an allocation — it is a number that arrived from a caller's `LIMIT`, which
is [A1](#a1--memory-is-odepth-never-ocorpus)'s forbidden input wearing a different
name.

A relation that *does* spend it counts **rows it emits that agree with the bound
positions it was handed**. That distinction is the whole of the obligation: a
candidate the relation can itself see is doomed must not be counted against the
licence, or a stop at `k` hands back fewer than `k` usable rows and the engine
reads the short bag as an exhausted one.

**The failure it prevents.** Precisely the one above — a truncated read
presenting itself as a complete one, which is the doctrine's failure in its purest
form.

**Who enforces it.** Both halves, at the two altitudes that can see them.

The **engine withholds what it cannot account for**: everything it filters on that
a relation could not see — a repeated variable across two free positions, a
partially-bound quoted triple — makes it withhold the ceiling entirely for that
call rather than ask a relation to account for something it was never told.

The **producer honours what it is given**. The shipped lexical producer does both
halves itself, because it can see a case the engine cannot: it withholds the
ceiling from its own ranker whenever a post-rank position is bound
(see [A2](#a-bound-output-position-is-not-one-of-these-filters)), and its cursor
decrements the licence only on rows it actually emits. All three properties carry
tests in `crates/text/src/relation.rs`:
`a_ceiling_yields_the_prefix_of_the_unbounded_answer`,
`a_ceiling_with_a_bound_doc_still_emits_the_matching_row` and
`the_ceiling_counts_emitted_rows_not_skipped_ones`.

---

## A11 — Volatility must be true of the snapshot held

**The obligation.** [`volatility`](purrdf_sparql_eval::PropertyFunction::volatility)
is a claim about this query's view of host state, not about the host state in
general. Declaring [`Volatility::Stable`](purrdf_sparql_eval::Volatility) means
the snapshot that answers the first invocation answers every invocation, so the
snapshot has to be **pinned when the invocation opens and held for the life of the
query**.

**The failure it prevents.** Only `Stable` may run across workers. A relation that
misdeclares itself diverges silently under parallel evaluation: two workers see
two states, and the answer becomes a function of the schedule rather than of the
data. No engine-side measure can repair that, because the engine's only evidence is
the rows.

**Who enforces it.** The producer. The layer reads the declaration through the
fork-join parallel gate and cannot check it. (It *is* contained: `volatility` is
host code exactly as `open` is, so a panic inside it is caught rather than
unwinding through the engine — but containment is not verification.)

Both shipped producers hold their index behind an `Arc` and pin it at `open`. The
pinning is indirectly **observable** through
[A13](#a13--attest-the-generation-of-the-snapshot-that-answered): the generation
is read immediately after `open` precisely because that is the instant the snapshot
is pinned, so a producer that rebuilt underneath a drain would attest a generation
that did not produce the rows it produced.

## A12 — Bounds narrow; they never zero

**The obligation.** A [`Statistics`] provider's cardinality and
selectivity **lower** a stratum's depth and never raise it, and a measurement of
zero is an honest report about the data rather than a licence to skip the read. An
unknown cardinality is not a zero one, which is why the provider returns an option
rather than a number. The same holds for a registry's own
`rows_per_invocation`: a declaration of zero rows reports what the producer holds
right now, and it too is read rather than obeyed.

**A provider's revision is a promise.** Admission decides whether a plan's
evidence is still in force by comparing the provider-minted `revision` string it
recorded against the one the environment reports now. That comparison is the only
thing standing between a stored plan and a silent replan, so a provider **must**
advance its revision whenever any answer it would give changes. A provider whose
numbers move under an unchanged revision is admitted, and the plan then reads at a
depth the current data does not support — with nothing anywhere reporting it.

**The failure it prevents.** A depth of zero compiles to `LIMIT 0`, which hands
back no row whatever the relation holds, and the trailer still reports the stratum
exhausted with zero rows — the one ending that names no stopper, made about the
bound rather than about the data, and identical in every trailer field to an
honest empty answer, so nothing downstream can tell them apart.
A tiny non-zero selectivity was already safe through ceiling division;
zero was the one input that escaped it, and it arrives by three roads: a provider
honestly reporting a selectivity of zero parts per million, a measured
cardinality of zero, which lands in the bound before the ratio is applied, and a
producer whose every declared access mode promises zero rows per invocation.

**Who enforces it.** The layer, in three places that are one rule, so that "this
stratum is empty" is always said by a producer's receipt against rows fusion
verified and never by a plan that declined to ask. Any one of the three missing
brings the `LIMIT 0` back.

1. **The derived depth is floored at one row**, whichever road the zero arrived
   by. Emptiness is then reported by the producer. Pinned by
   `a_zero_statistic_still_plans_one_row_and_the_producer_reports_the_emptiness`,
   `a_zero_selectivity_narrows_to_one_row_and_the_relation_is_still_invoked`,
   `a_cardinality_of_zero_is_floored_with_or_without_a_selectivity`,
   `a_mandatory_producer_under_a_zero_selectivity_plans_admits_and_runs`,
   `a_producer_declaring_no_rows_is_planned_at_one_row_and_still_serves_its_term`
   and `a_mandatory_producer_that_declares_no_rows_is_served_rather_than_missing`,
   all in `tests/compile_request.rs`.
2. **Admission admits that floored row against a declared bound of zero** and
   refuses every depth past it, while a stratum no ranked producer emits under is
   still refused at every depth — there is no producer there to hand a probing row
   to. Pinned by `a_declared_zero_admits_the_floored_row_and_refuses_the_one_past_it`
   in `tests/admission_tests.rs`, whose four arms hold the declared zero, the row
   past it, a recorded zero and the absent bound apart.
3. **A recorded depth of zero is refused at admission** as
   [`AdmissionError::ZeroDepth`], and the compiler's emitted bound is one row past
   the depth whatever the declaration says, so a declared zero can write neither
   `LIMIT 0` nor a `LIMIT` equal to its own depth. Given the two above,
   a recorded zero can only have come from an edited plan. A stratum that is to
   read nothing carries no entry at all. Pinned by
   `a_recorded_depth_of_zero_is_refused_over_a_stratum_the_registry_ranks_under`,
   with `a_stratum_no_surviving_producer_ranks_under_records_no_depth_at_all` as
   its valid neighbour.

"Declared no rows" and "declared nothing" stay distinct all the way down, and they
are distinct in a way worth stating precisely. A producer that declared **zero
rows** described its data: it is invocable, so it is selected, bound, planned at
the floored row and asked, and it answers with its own
`Exhausted { rows_emitted: 0 }`. A producer that declared **no access mode**
described nothing: it admits no invocation, so placement refuses it as
[`RejectionReason::UnsatisfiedConstraint`], its stratum bounds no depth, and
inventing a zero for it would refuse a plan the registry never spoke against.

The end-to-end case is the one a host actually meets — an index built before its
documents land, registered as the only producer — and it is driven over the
shipped `TextSearchRelation` by
`the_sole_text_producer_over_an_empty_corpus_reports_its_own_emptiness` in
`tests/real_producers.rs`, with
`the_sole_text_producer_over_one_document_still_returns_that_document` as its
valid neighbour.

**And the option travels into the record.** The three places above are one rule
about what a bound may *do*; this is the same distinction about what a plan
*says*. A plan records every input each depth was derived from — the declaration,
the cardinality, the applied selectivity with the request terms it aggregates
over, and the licensed prefix — and each statistic is absent there exactly where
the provider reported none. An unknown cardinality is not a zero one at the
provider boundary, and it is not a zero one in the record either: writing one
would put a measurement nobody took beside a depth, where
[`Plan::certify`] would then recompute the depth
from it and agree. The same holds for the declaration, for the reason the
paragraph above gives — inventing a zero for a producer that declared nothing
would refuse a plan the registry never spoke against — so the layer refuses that
state by name rather than defaulting it. Pinned by
`a_selectivity_only_stratum_is_recorded_and_narrows_the_depth`,
`a_consulted_but_silent_stratum_records_both_statistics_absent` and
`an_undeclared_row_bound_is_not_a_declaration_of_zero_rows`, with
`a_subject_the_provider_reports_but_nothing_consults_is_absent` as the valid
neighbour that keeps the recording from widening into everything the provider
knows.

## A13 — Attest the generation of the snapshot that answered

**The obligation.** Override
[`PfCursor::generation`](purrdf_sparql_eval::PfCursor::generation) and return your
own spelling of the version that is answering. It is read **once, immediately
after `open`** — the instant the snapshot is pinned, so the claim is true of every
row that follows.

**The failure it prevents.** Two runs of the same query, over the same dataset,
under the same registry, can legitimately return different rows because the index
behind a relation was rebuilt between them — and nothing the evaluator can observe
changes. The query text is the same, the dataset snapshot is the same, and the
registry fingerprint is the same, because a rebuild changes no declaration. The
cursor is the only party that knows, so the generation travels from there or not
at all.

**What a generation owes.** It must move exactly when the rows that can be
returned move — no sooner, and no later — and the producer must **say which kind of
value it is supplying**, because the engine records the string verbatim and never
parses, orders or interprets it. Nothing downstream can tell the two kinds apart
from the bytes:

* a **content-derived digest** is comparable across processes and machines: two
  hosts that built the same index from the same inputs attest the same value, so a
  caller comparing two answers' evidence is comparing the indexes;
* a **host-scoped label** — a counter, a build id, a path, a wall time — is
  comparable only within one host, and two processes comparing such labels are
  comparing nothing at all.

The engine will not mint one on a producer's behalf. A value invented here from a
clock or an RNG would differ on every run and on every machine, would make every
receipt disagree with every other one, and would be untrue besides — the engine
has no idea when the index was built.
[`IndexGeneration::Undeclared`] is the honest absence for a
relation that is not index-backed — an in-memory table, a computation over its
arguments, a walk of the dataset already being queried — and it stays an absence
all the way out, never a certificate that the index was current.

**Who enforces it.** The producer declares; the layer carries, digests and refuses
a record that cannot be one unit's. [`execute`] runs each unit
through the governed entry with every ceiling declined and reads the attestation
off the
[`RelationWitness`](purrdf_sparql_eval::RelationWitness) on the receipt that
already carries the run's evidence. A compiled unit binds one producer and calls
it with constant arguments, so a conforming run attests one generation and at most
one shortfall; anything else means the snapshot moved underneath the query, which
invalidates the run rather than one stratum, and is refused as
[`ExecutionError::InconsistentWitness`].
(The invocation count is deliberately not part of that rule: it varies with how the
evaluator chunks a forked expression, while the declarations do not.) From there
the value is tagged onto the stream, read by
[`FusionStream`] **before any row is pulled**, and reported in
[`FusionTrailer::attestations`] and digested
into [`EvidenceId`].

[`search`] reads each stratum **on demand** — one invocation held open and read a
row per pull — so the stream it announces is what the invocation attested *the
instant it opened*: the generation, and the service level asked at that instant
too. When the fusion stops, the stream settles to the witness the invocation
stands behind then, under the same rule, and is held to the announcement. A
relation whose cursor reports a different generation at the stop than at the
open, or a different service level, has its answer refused
([`ProtocolError::AttestationMoved`]) — every row was certified under the
announcement, so relabelling the trailer would keep rows ordered under a claim
the read did not end with. A relation over a snapshot pinned at `open`, which
answers both questions the same way at both instants, never meets this.

**Both shipped producers attest a content-derived generation**, and each value is
pinned against its own source rather than merely asserted non-empty.

The lexical producer attests its **index fingerprint**, which closes over the
documents, the term dictionary, every posting, the partition statistics, the
configured predicates and the ranking law. The two neighbouring digests were
rejected against the movement rule, and the rejections are themselves tests in
`crates/text/tests/search_property_function.rs`:
`the_same_rows_under_a_different_ranking_law_attest_a_different_generation` — the
digest of the source rows does not move when the same rows are re-ranked under a
different law, though every emitted score does — and
`two_builds_of_one_corpus_attest_one_generation_and_a_new_document_moves_it`,
which is the same rule read in the other direction.

The nearest-neighbour producer cannot declare its matrix alone. The map from a row
to the RDF term it stands for is a **host argument**, because the artifact format
deliberately allows a target to be disclosed by digest alone — so two spaces over
byte-identical artifacts with different bindings return a different term at every
position, and a matrix-only identity would call them the same generation. The
declared value folds the **projection** digest (rather than the matrix digest,
because a prefix policy lets two spaces share one stored matrix and differ in the
prefix taken), the family contract that names the metric, and every bound term in
row order. The bound on work is excluded: it decides how hard a search tries,
never which rows exist or how they rank.

Both values are computed once where the snapshot is pinned, out of any per-row
path. `both_real_producers_attest_the_generation_of_the_index_that_answered` in
`tests/real_producers.rs` checks each attested value against the producer's own
public surface — a constant, a counter or a digest of the wrong thing would pass a
non-emptiness assertion and fail this one — and
`two_runs_over_the_same_index_state_carry_one_evidence_id` checks the stability the
third identity rests on.

**One consequence worth stating, because it is easy to expect the opposite.** A
rebuild of the lexical index moves more than the evidence identity. That producer
measures its declared row bound *from the index* ([A9](#a9--declare-the-honest-unfiltered-worst-case-for-the-row-bound)),
the declared bound reaches the registry's content fingerprint, the plan records
that fingerprint and derives its depths from the same declaration — so the plan
identity moves too. [`EvidenceId`] is the identity that moves on
a change altering no declaration at all, which is exactly the case the other two
cannot see.

## A14 — Declare incompleteness rather than refusing or faking exhaustion

**The obligation.** A relation serving from an index that is not whole — a shard
that failed to load, a segment mid-rebuild, a replica that has not caught up —
declares it through
[`PfCursor::service_level`](purrdf_sparql_eval::PfCursor::service_level). It does
not fail the query, which overstates, and it does not return fewer rows in silence,
which is a false completeness claim.

**The two moments, and why they differ.**
[`generation`](purrdf_sparql_eval::PfCursor::generation) is read immediately after
`open`, where the snapshot is pinned.
[`service_level`](purrdf_sparql_eval::PfCursor::service_level) is read when the
invocation **ends** — after the cursor returned `None`, and equally when the engine
stopped pulling at its own ceiling, at a governor trip, or at a stop signal. The
end rather than the beginning, because a shard discovered missing on the
four-hundredth pull is exactly the case this channel exists for; a cursor that knew
at `open` simply answers the same way at both instants.

Only the **narrow** question is asked — *was your index NOT whole?* — for the
doctrine's own reason: a relation that stopped at the ceiling is not incomplete and
could not honestly certify wholeness either, so the seam asks for the one fact the
relation alone can know and can state without inspecting anything it skipped.
`Undeclared` is silence, and a reader must not upgrade it to "the index was whole".

**The consequence for ungoverned queries.** An entry point with nowhere to put that
evidence **refuses** a relation that declares itself incomplete —
`EvalError::RelationIncomplete` — rather than returning a short bag whose receipt
nobody can read, which is precisely what the hard-fail doctrine exists to prevent.

**Who enforces it.** The producer declares; the layer refuses the unrecordable
case and carries the recordable one. The record is owned per context and merged at
every fork join, with set union and a saturating count making the merge commutative
and associative, so the result is a function of what was attested rather than of
scheduling. Pinned in `crates/sparql-eval/tests/relation_witness.rs`, with both
sides of the refusal executed:

* `a_governed_entry_answers_and_records_the_incompleteness`,
* `an_ungoverned_entry_refuses_an_incomplete_relation`, and its valid neighbour
  `an_ungoverned_entry_still_answers_a_relation_that_declares_nothing_short`,
* `an_incompleteness_discovered_at_close_is_still_recorded`,
* `an_attestation_collected_on_a_row_loop_worker_reaches_the_receipt`,
* `an_update_over_an_incomplete_relation_refuses_and_writes_nothing`, with
  `the_same_update_over_a_relation_declaring_nothing_short_commits` beside it.

**What it does to a fused score, in both directions.** A stratum serving from a
short index omits whatever its missing shard held, so a candidate that shard would
have named is summed one contribution light. That much is obvious. What is not is
that fusion scores by **rank** and nothing else, so the omission also promotes
every row behind the missing one into a rank it did not earn — and each of those
collects a *larger* contribution than it was due. A candidate the short stratum
missed scores too low; one it named scores too high.

Labelling that "exact" would be a bound on the read becoming a value, and
labelling it a *lower bound* would be worse: right about the first direction and
wrong about the second, so a consumer acting on the name would be confidently
wrong in the one direction the name told it not to look.
[`FusionTrailer::exactness`] says which reading applies: [`ScoreExactness::Exact`],
or [`ScoreExactness::Estimated`] naming the responsible strata on each side.
`FusedRow::interval` carries the size of each for one row. The rows are returned
either way, because a short index still produced real rows in a real order.

## A15 — Declare candidate domains, and never name a candidate outside them

**The obligation.**
[`RankedDeclaration::domains`](purrdf_sparql_eval::RankedDeclaration) states which
blocks of the candidate universe this producer may name.
[`CandidateDomains::Unrestricted`] is the **wider** promise
— "this producer may name anything" — and is what every stream effectively said
before the term existed.
[`CandidateDomains::Within`] names blocks of a *partition*
of the candidate universe, so a candidate lies in exactly one block. It is never
empty: a promise to name nothing is not a narrow domain, it describes a producer
that should not be registered, and `register_ranked` refuses it where it is
written.

**And every row backs it.** A restricted declaration is not only a set other
declarations are compared against: each row says which block it was drawn from
([`RowBlock`] on [`RankedRow`]), because the axiom the arithmetic below rests on —
the tags partition the candidate universe — is a fact about the host's corpus that
no consumer can derive, and a row that names its block is what makes it checkable
at all. So the per-row duty, exactly:

* a [`CandidateDomains::Within`] producer owes a block on **every** row, and owes
  one its own declaration admits;
* a [`CandidateDomains::Unrestricted`] producer owes **none**. It restricts no
  consumer arithmetic — its head counts in every block's bound — so there is no
  promise for a row to back, and [`RowBlock::Undeclared`] is its honest answer. It
  may still name one, and a block it names is kept, because a block is evidence
  about the *candidate* rather than about the stream.

**Where the block comes from, for a producer registered through the seam.** Two
places, and never a guess:

* the **declaration**, when it names exactly one block. It has already said that
  every candidate this producer names lies in that block, so the per-row fact is
  entailed and `execute` reads it straight off the declaration. No host repeats
  itself per row, and this is the configuration the whole mechanism exists for —
  one producer per block, blocks that do not overlap;
* the producer's own **block column**, named by
  [`RankedDeclaration::block_position`](purrdf_sparql_eval::RankedDeclaration): the
  argument position each row carries its block in. `compile` projects it beside
  `?candidate` for exactly the producers that declare it, and `execute` reads it
  back by name. This is how a producer restricted to **several** blocks backs its
  promise, because a several-block declaration entails nothing about any one row
  and no consumer may choose on the host's behalf.

A several-block declaration with no column to back it is refused at the first row
([`ProtocolError::UnbackedDomainDeclaration`]) rather than quietly read as the
wider promise it did not make: fusion has already *used* the restriction by then.
A host in that position has two one-line exits — declare
[`CandidateDomains::Unrestricted`], which costs only the early certification the
narrower claim would have bought, or register one producer per block. Neither
shipped producer declares a block column: a text index answers with documents and
a vector index with neighbours, and neither holds any notion of a host's
partition, so both leave the position unset and take their domains from the host
unchanged.

**The failure it prevents.** With nothing declared and nothing askable, "could
this stream still name the candidate" is true of every open stream, so strata
whose candidate sets do not overlap are read to their ends however small the
caller's top-k — a top-ten over two million-row strata reads two million rows and
grows a frontier to match. Weakening the finality test was not available: against
a producer that will say nothing about its own candidates there is no random
access, so the only way to learn that a stream does *not* name a candidate is to
read it to its end, and certifying sooner while knowing nothing would emit a
score missing a contribution and call it exact. Exact scores and a bounded read
are jointly reachable only if fusion is told, or can ask, which candidates a
stream can name. This obligation is the telling; the exclusion basis in
[A8](#a8--capability-declarations-are-contracts-cardinality-declarations-are-estimates)
is the asking, and it reaches the configuration this one cannot — two producers
over one block whose results never overlap, where both declarations are true and
neither settles a thing.

**What it licenses, precisely.**
The finality test inside [`FusionStream`] stays a **membership** question — a
stream that could name a candidate and has not still blocks it, which is what keeps
a live head contributing nothing from being read as an absent one — and gains only
a **smaller quantifier**: the streams that provably cannot name the candidate are
skipped. Nothing about what a live stream owes a candidate changes. The threshold
follows the same shape: an item nobody has named yet lies in one block, so the bound
over unseen items is the **largest per-block sum of open heads**, which reduces
exactly to the sum of every head when nothing is declared. A fusion whose every
stream declares `Unrestricted` therefore computes precisely what it computed before
the term existed — the same rows, the same scores, the same provenance, the same
reading cost.

**And it licenses a narrower depth, conditioned on `Unique`, and it is decided per
stratum.** The declaration buys a second thing, one stage earlier and larger than
the quantifier: where a request states a bound of `k`, a surviving stratum `s`
declares a block set that meets no other surviving stratum's, **and `s` itself
declared [`DuplicatePolicy::Unique`]**, the planner records a depth of `k` for `s`
and the compiled unit is emitted at that `LIMIT`. The read itself is bounded, not
merely the walk over a stream that was materialized in full. The `Unique` condition
is not decoration: the merge argument counts ranks and reads the count as a count of
candidates, which is true only of a stream that names an item once — see
[A5](#a5--duplicate-fan-in-is-collapsed-inside-the-producer).

Read the conditions as being about `s`, because they are. A stratum that fails them
takes no other stratum's narrowing with it: two strata sharing a block cost each
other their prefixes and cost a third, disjoint stratum nothing, and a neighbour
declaring `Allowed` cannot repeat a candidate it is not allowed to name, so how
often it repeats its own is not a fact about `s`. The one declaration that still
costs *everyone* a prefix is `Unrestricted` among two or more strata, and that is
request-wide because the declaration is: "may name anything" is a statement about
every stratum's candidates. Wherever a condition fails, that stratum's depth is the
declared-or-measured bound it always was; no request is refused and no answer moves
either way.

**A single surviving stratum needs no domain declaration for that depth**, and the
reason is what disjointness is a statement about: pairs. With one stratum there is no
pair, so the premise the blocks supply is one the shape already has — every candidate
the answer can hold was named by that one stream, its rank order is the fused order,
and a `k`-row prefix of it is the top `k` under `Unique` alone. So a lone
`Unrestricted` producer narrows exactly as a lone `Within` producer does. Withholding
it asked the ordinary configuration to pay for a premise it did not need: neither
shipped ranked relation declares a domain, so a top-five over a declared
ten-billion-row index recorded a depth at the read ceiling and emitted
`LIMIT 4294967295` for five wanted rows, with the executor materializing every row
that came back. Register a second producer and the pair exists, `Unrestricted` is a
missing promise again, and the narrowing stops there rather than silently later — the
depth is re-derived per request from the registry in hand.

**Who enforces it.** Both.

The declaration is part of
[`canonical_description`](purrdf_sparql_eval::RankedDeclaration::canonical_description),
so it reaches the registry's content fingerprint and hence the plan identity, and
admission holds a plan to the fingerprint it was planned against rather than taking
the declaration on faith. Pinned by `a_declared_candidate_domain_moves_the_plan_identity`
in `tests/plan.rs`.

A stream that names a candidate its declaration cannot reach is refused as
[`ProtocolError::OutsideDeclaredDomain`],
carrying three fields because each answers a different question: the item says
*what*, the stratum says *who broke it*, and the third names the stratum whose own
declaration — already applied — put the candidate out of this one's reach. The
answer is not silently widened instead, because the declaration has already been
*used*: fusion certified candidates early on the strength of it, so an earlier row
may already have been emitted on the assumption this stream would never name it.
Pinned by `a_stream_naming_a_candidate_outside_its_declared_domains_is_refused` in
`tests/fusion.rs`, with the reading benefit measured rather than asserted in
`declared_domains_bound_the_reading_over_disjoint_strata` and the unchanged answer
in `the_differential_holds_under_declared_domains_over_many_configurations`.

The per-row duty is held to by three further refusals, each a distinct fact and
each paired in the tests with a valid neighbour that still answers:

| Refusal | The fact it reports | Pinned by |
|---|---|---|
| [`ProtocolError::UnbackedDomainDeclaration`] | a restricted stream's row names no block, so nothing backs the restriction | `a_several_block_declaration_no_row_backs_is_refused` (`tests/search.rs`) |
| [`ProtocolError::BlockOutsideDeclaredDomain`] | a row names a block its **own** declaration excludes — a stream contradicting itself, needing no second stream to witness it | `a_row_naming_a_block_outside_its_own_declaration_is_refused` (`tests/search.rs`) |
| [`ProtocolError::CandidateInTwoBlocks`] | two rows place one candidate in two blocks, so the axiom is false for that candidate | `two_streams_naming_one_candidate_from_two_blocks_are_refused` (`tests/search.rs`), `a_candidate_two_rows_place_in_two_blocks_is_refused_and_one_block_fuses` and `a_dropped_duplicate_may_not_place_its_candidate_in_a_second_block` (`tests/fusion.rs`) |

The third is the one the threshold depends on, and it is deliberately not
`OutsideDeclaredDomain`: that refusal compares *declarations*, so it cannot see two
overlapping declarations whose **rows** disagree — and that is exactly the case the
per-block bound under-counts, because the streams that reach one block and the
streams that reach the other are different sets. It names the item, both strata and
both blocks, because either producer may be the one that tagged wrongly and this
layer cannot know which. A row a permissive duplicate policy discards is checked
too: a repeat may be dropped, but its claim about the candidate may not be.

**The honest limit.** Verification reaches only as far as the rows actually pulled.
A false declaration that no pulled row contradicts yields a score that declaration
made wrong, and a candidate two streams would have placed in two blocks at ranks
this fusion never reached leaves no trace. What that leaves unverified is stated
rather than implied: the axiom holds over the rows read, and nothing is claimed
about the rows below them. This layer does not claim otherwise — it is the identical trust the
uniqueness declaration of [A5](#a5--duplicate-fan-in-is-collapsed-inside-the-producer)
already carries, whose breach is likewise detected only when the repeat is actually
read.

**And the depth this declaration buys shortens that reach**, which is the one place
the two halves of A15 pull against each other. A host that really has tagged one
entity into two blocks is caught by `CandidateInTwoBlocks` only if both naming rows
are pulled, and under a bounded request the read stops at `k` rows per stratum, so a
second naming row at a deeper rank is never read and the contradiction leaves no
trace. So the promise is exactly this and no more: **a mis-tagging is reported
wherever the rows that witness it are among the rows pulled, and a narrower read
pulls fewer of them.** Detection is strongest under
[`ReadBound::Complete`], which reads every stratum to its declared
or measured depth, and weakest under a small bound over strata that declare disjoint
blocks — the configuration the narrowing exists for. A host commissioning a new
partition therefore has somewhere to exercise it: run the corpus once complete, where
every row is measured against the declaration, rather than inferring from bounded
traffic that the tags are sound.

**Nothing is defaulted from a stratum or a graph.** The tags come from the host,
because the host is the only party that knows whether its text index and its vector
index name the same entities. A consumer deriving a tag per stratum would hand two
producers over one entity space a pair of tags it reads as disjoint, and there is no
safe way for that to be wrong: the pair either makes the consumer refuse a query
that was valid, or lets it certify a score missing a contribution the other stream
was about to make. A wrong guess here is a wrong answer, so there is no guess — and
that is also why neither shipped producer carries a domain declaration of its own.
Each takes one from the host at registration and passes it through unchanged.

## A16 — Declare the fidelity of the rows you can name

**The obligation.** State, where you register, whether your search names every
row that was due to it, and whether a row you *do* name arrives at a rank no
better than the one it earned. The two are independent axes and both are
required. If either is degraded, supply your own evidence for it — prose a
reader can act on, naming the measurement, its limit, and what an empty result
does not prove.

**The failure it prevents.** Every other ending in the vocabulary is
contradicted by a row that *arrives*: a repeat falsifies `DuplicatePolicy::Unique`,
an out-of-block candidate falsifies a domain declaration. This one cannot be, and
that is exactly why it must be declared. A stream that ran out of rows and a
stream whose beam stopped finding them are indistinguishable from the consumer's
side — both simply stop yielding, both leave contiguous ranks, both report
`Exhausted`. So an undeclared approximation is invisible here **by construction**,
and silence about it is read as completeness. Nothing downstream can recover the
fact; a consumer either receives it from the producer or never has it.

The second axis carries a fact the first cannot. Order faithfulness is the
precondition for any finite bound on the answer's error: if a named row's true
rank is at least its emitted rank, the contribution it collected bounds the
contribution it was due. A producer comparing *approximated* values — quantized
vectors, a sketched score — can rank a row it found better than it was due, and
then no bound exists at all. A consumer must be told that rather than handed a
number, and one flag could not carry both facts.

**Who enforces it.** The producer, and it cannot be otherwise. Registration
refuses a declared loss with empty evidence — a disclosure that discloses nothing
is the one failure visible from here — but nothing checks that an exhaustive
declaration is true, for the same reason nothing checks a `Unique` promise before
the repeat arrives. A producer declaring itself exhaustive while quietly missing
rows yields an answer that declaration made wrong, and this layer does not claim
otherwise.

**Where a shipped relation gets it from.** Every ranked producer in this
workspace takes its fidelity, or the half of it the relation cannot derive, as a
parameter at the point of declaration. `TextSearchRelation` and
`EmbeddingKnnRelation` take the whole `RankFidelity`, because both are
exhaustive over the data they hold and neither can see whether that data is the
whole of the host's corpus: BM25 scores every document holding a query term and
the kNN relation scans every row of its space, so `RankFidelity::EXACT` is a true
statement about each one's *search* and says nothing about its *coverage*. A
relation asserting it would be the failure above in its purest form — the
strongest claim in the lattice in the mouth of the one party that never spoke,
about the one fact nothing downstream can recover.

The HNSW relation is the exception, and takes only an `OrderFidelity`. Its
completeness axis is `Lossy` over every space on every request, derived from a
fact it genuinely holds — a beam offers what it reached and never certifies that
nothing else matched — so there is no silence there to be read as completeness,
and the profile's own evidence occupies the axis, which is where this contract
requires it to reach a consumer byte for byte. What the relation cannot see is
what the host did to the vectors *before* the build: quantize them and the
producer is order-perturbed, and no loss contract reachable from the index
records it. That is the parameter, and it composes with the derived value by
taking the worse of the two, so a host can degrade the axis and never upgrade it.

**This is not the attestation channel, and does not duplicate it.**
`ServiceLevel::Incomplete` answers a different question — was the index *version*
that served this invocation whole, given a shard that failed to load or a replica
that has not caught up — and it already degrades `ScoreExactness` on its own. It
is the wrong channel for a corpus a host deliberately sampled, twice over: it is
read per invocation from the cursor rather than stated at registration, and an
`Incomplete` reading is a *refusal* on every entry point that carries no witness,
so a host stating a permanent fact there would take out its ordinary SPARQL
queries along with its fused ones.

**What it does to a fused answer.** The declaration reaches
`FusionTrailer::fidelities` verbatim, under the same stratum key as the status,
and the two are read together: a status is how the read *ended*, a fidelity is
whether the rows that ended it were all the rows that were *due*. It also drives
`FusionTrailer::exactness` and each row's `FusedRow::interval`, exactly as a short
index does and for the identical reason — see
[A14](#a14--attest-the-generation-that-answered-and-say-when-it-was-short).

**It is part of a compiled plan's identity.** The declaration folds into the
registry's content fingerprint and so into the plan id: a plan drawn from
producers that approximate is not the plan drawn from producers that do not, and
the two answers differ in what they may be read to claim. **A consequence worth
knowing before you edit an evidence string:** changing that sentence changes
every plan that names the producer.

Pinned by `the_declaration_carries_the_profile_evidence_byte_for_byte`,
`the_order_axis_is_a_function_of_the_loss_contract_and_not_a_literal`,
`a_host_that_approximated_its_vectors_declares_a_perturbed_order`,
`a_host_that_transformed_nothing_gets_the_declaration_it_always_had` and
`the_composition_takes_the_worse_of_the_two_and_keeps_the_hosts_words` in
`crates/hnsw/tests/ranked_declaration.rs`; by
`the_ranked_declaration_carries_the_fidelity_the_caller_stated` in
`crates/sparql-eval/src/knn/tests.rs`; by
`a_vector_space_over_half_the_corpus_stops_the_answer_claiming_wholeness` and its
neighbour `a_whole_corpus_with_nothing_to_disclose_still_certifies_exact_scores`
in `tests/real_producers.rs`, which run the whole ladder over a real sealed
artifact holding half a corpus; and by the `T8`/`T9` groups in `tests/fusion.rs`.
