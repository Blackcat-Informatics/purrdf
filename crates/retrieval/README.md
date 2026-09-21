<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# purrdf-retrieval

The composition layer over the ranked property-function producers.

PurRDF already answers ranked retrieval through several relations reached
through the evaluator's property-function seam. This crate is the layer above
them: it turns one request into one plan, then compiles, executes and fuses that
plan into one answer. It is a composition **outside the kernel**: nothing in
`purrdf-core` or the seam changes to admit it.

```text
plan(request, registry, statistics) -> Plan
compile(Plan, environment)          -> per-stratum SPARQL units
execute(units, registry, dataset)   -> per-stratum ranked streams
fuse(streams, profile, k)           -> the top k of one ordered answer
```

Every boundary is a supported place to stop *and* to start, and `search` is
defined as the composition of all four rather than as a second implementation of
it. A caller that stops at `execute` and later wants to fuse resumes through
`RankedStreamAdapter`, the same exported bridge `search` uses, so the two
compositions cannot drift.

The stages:

* `RetrievalRequest` / `RequestTerm` — the typed term lattice a request is
  expressed in (lexical, vector, spatial, temporal interval, numeric range,
  entity seed). The enum is closed and stays closed, so a caller matching
  exhaustively gets a compile error when a modality is added rather than a
  wildcard arm that swallows it. It deliberately carries modalities ahead of the
  producers that answer them — producers are caller-supplied configuration, not
  a bound on what may be asked — and a term this registry has nobody for is
  reported per term as an `UnservedTerm`, never dropped. A request also states
  how much of the answer it is for, as a `ReadBound` over the bounded and the
  complete case — a planning input rather than a trailing preference, because it
  is what each stratum's depth is derived from.
* `plan(request, registry, statistics)` — the pure planner. It matches request
  terms to producers by a lookup over the producers' declared capabilities,
  records selected and rejected producers with reasons, derives per-stratum
  depths from the registry's own row-bound declarations capped by statistics,
  and records **every input each depth was derived from** alongside the
  statistics snapshot and both registry identities. It opens no store, no file
  and no clock.
* `Plan::certify` / `depth_from` / `Plan::explain_depth` — a recorded depth is a
  checkable claim, not an asserted one. `depth_from` is the one arithmetic path
  the planner runs and a reader can re-run; `certify` recomputes every depth from
  the plan's own recorded inputs and refuses a plan the two disagree about;
  `explain_depth` names which input bound a depth, so a depth of one says whether
  it came from a declaration, a measurement, a selectivity or the floor. Cold
  paths: admission calls none of them.
* `Statistics` — the caller-supplied cardinality/selectivity input planning
  consults. There is no built-in provider. Both are exact integers — a
  selectivity is parts per million, never a float — and both **lower** a
  stratum's depth and never raise it: a cardinality bounds how many rows the
  stratum holds, a selectivity what fraction of them the request's terms can
  match. A provider that measured nothing narrows nothing.
* `Plan` — pure, inspectable, editable and serializable data: the request
  terms, per-producer bindings, selected/rejected producers with reasons, every
  request term that reached no producer at all and why (`UnservedTerm`), the
  per-stratum depths, the statistics snapshot it was planned against, and both
  registry identities. It records **no** stratum weights: the weights that fuse
  an answer belong to the `FusionProfile`, which is chosen later and is
  deliberately not a planning input, so a plan carrying a second set could only
  be a number no fusion reads.
* `PlanId` — a domain-separated BLAKE3 digest over a versioned, canonical,
  length-framed encoding of the plan. A version mismatch on decode refuses
  loudly rather than guessing.
* `compile(plan, environment)` — the semantic admission waist. Every plan is
  untrusted input, whether freshly planned, hand-built or deserialized, and it
  is checked here against what the registry declared before anything is emitted.
  An environment that also names the fusion profile the answer will be composed
  under is held to that law's own arithmetic: a per-stratum depth beyond the
  rank at which the profile's contributions stop being distinct is refused, and
  `search` always names the profile it is about to fuse under.
* `execute(units, registry, dataset)` — one run per stratum through
  `purrdf-sparql-eval` against the caller's own dataset. A stratum that cannot
  run becomes its own `ProducerStatus` while every other stratum streams on. A
  `RankedStream` is a reading order, **not** a cursor: the evaluator materializes
  a stratum's whole result before its first row is readable, so stopping here
  buys independent per-stratum receipts and no cross-stratum accounting — never
  a cheaper enumeration than reading the stratum costs.
* `fuse(streams, profile, k)` / `search(request, …)` — the exact fixed-point
  reciprocal-rank fusion, bounded by the request's own `TopK` because fused
  enumeration is top-k by construction. The bound stops the reading as well as
  the returning: a producer still holding rows when it is reached is reported at
  the contribution it was read down to, never drained to make it declare
  exhaustion. The answer carries every applicable producer's own status in its
  trailer — including those that could not answer — and every request term that
  reached nothing.

  `search` reads the bound from the request rather than taking one of its own,
  because the planner already derived every depth from it. `fuse` is the
  lower-level entry and still takes one — a caller assembling its own streams has
  no request to read it from — and refuses a bound the streams were not planned
  for (`FusionError::ReadBoundMismatch`), so the two cannot drift.

## The bound is a read bound, not only a row bound

Over strata whose producers declare pairwise **disjoint** candidate blocks, each
candidate has exactly one naming stratum, so its fused score is one weighted
contribution that falls with rank and the global top `k` is a merge of per-stratum
prefixes: nothing below per-stratum rank `k` can enter it. The planner therefore
records a depth of `min(declared, statistics-narrowed, k)`, the compiled unit is
emitted at that depth plus its probe row, and the work a bounded search does is
flat in the corpus rather than linear in it. The proof, the tie case and the
neighbours it must not fire on are on `plan`'s own documentation.

Any overlap between two declarations, or any `Unrestricted` stratum, and the
declared-or-measured bound stands exactly as it did — scores sum across strata
there and the merge argument has no premise. The answer is identical either way;
only the reading moves.

## What a producer owes this layer

Every number in that answer is a function of what the producers said about
themselves, so what a producer owes is written down in one place:
[`PRODUCER-CONTRACT.md`](PRODUCER-CONTRACT.md), rendered in the API docs as the
`producer_contract` module. Fifteen obligations — memory bounded by the depth
rather than by the corpus, filters applied during selection and expressing
eligibility rather than relevance, scoring statistics drawn from the index's
declared scope, duplicate fan-in collapsed inside the producer, cardinality
projected only where the index already holds it, the difference between a
capability declaration and a cardinality declaration, an honest unfiltered row
bound, a ceiling honoured for efficiency and never for correctness, a pinned
snapshot the volatility declaration is true of, statistics that narrow without
ever zeroing, an attested generation, a declared shortfall, and declared
candidate domains.

Each entry states the obligation, the failure it prevents, and **who enforces
it**: the layer *checks* some of them, and a breach is a named refusal; it
*believes* the rest, and a breach is a wrong answer — those entries name the test
in this repository that proves the shipped producers keep the promise, or say
plainly that no test covers it. Read it before writing a `RankedDeclaration`.

## What a read ending says, and what the index attested

A producer's terminal status says **who stopped the read**, and there are six
spellings. `Exhausted` is the one ending that names no stopper — the producer
emitted every row *its search produced*. That is not, on its own, a claim that
everything matching was returned: a producer whose search does not find every
row that was due still runs out of the rows it found, and reports exactly this.
What the status has to be read beside is the stratum's declared fidelity, below.
`DepthReached` is the planned depth stopping a
producer that had more to give, stated in rank space. `RowBoundReached` is the
producer's own declared row bound stopping it: a producer that takes its depth as
an argument, read to the number it registered, so the row past it could not be
asked for and whether one exists was not observable. `CeilingReached` is a
contribution bound, written either by the producer or by a fused top-k that
stopped reading. `TermsRejected` is the producer declining the terms it was
handed, and `ExecutionFailed` is a run that could not happen. Neither of the last
two carries a row.

What the *index* attested is a separate axis, read from every stream **before a
single row is pulled**, because a generation is pinned when a cursor opens. The
ordering is load-bearing rather than tidy: held as a terminal status, an
incomplete index would be overwritten by a bounded stop — a stream a top-k
stopped never returns a receipt at all — and the fact would vanish in exactly the
runs where the bound mattered. So a stratum that was both stopped and short
reports both.

That axis reaches the answer three ways. `FusionTrailer::attestations` carries it
verbatim, per stratum: which generation answered, and the verbatim reason if that
index declared itself short. `FusionTrailer::exactness` says how to read a fused
score — `Exact` when no handed stream was degraded, or `Estimated` naming the
responsible strata on each side. Both sides, because a stratum serving from a
short index omits whatever its missing shard held *and*, since this layer scores
by rank alone, promotes every row behind the missing one into a rank it did not
earn: the candidate it missed is summed too low, the ones it named too high.
`FusedRow::interval` carries the size of each error for one row. And `EvidenceId`
digests the attestation map
into the third identity an answer carries: `PlanId` names the question,
`FusionProfileId` names the law, `EvidenceId` names the index generations that
answered. The third exists because the first two are derived from configuration,
and configuration is exactly what does not change when an index is rebuilt
underneath a running system. Two answers are comparable iff all three agree.

## What the producer declared about its own search

A third axis, and the one a status is most often mistaken for. An attestation is
about the **index** — which generation answered, and whether that generation was
whole. A `RankFidelity` is about the **search over it**, and a producer declares
its own on two axes that fail independently:

* `Completeness` — whether the search names every row that was due. A sampled,
  partitioned or stale index is `Lossy`, carrying the producer's own words for
  what it does not promise, verbatim.
* `OrderFidelity` — whether a row it *does* name arrives at a rank no better than
  it earned. Only a producer comparing approximated values is `Perturbed`, and
  that is the axis that breaks every score bound, because every bound here rests
  on the inequality a perturbed order violates.

`FusionTrailer::fidelities` reports it per stratum, populated before a row is
pulled, so a stratum is distinguishable as approximate without consulting the
registry. It is declared rather than observed because a consumer cannot tell the
difference: a stream that ran out of rows and a stream whose search merely
stopped finding them both simply stop yielding. There is no default — `EXACT` is
the top of the lattice, and defaulting to it would put the strongest claim in the
mouth of a producer that said nothing.

Read `fidelities` **with** `statuses`, never instead of them. `Exhausted` beside a
`Lossy` declaration is neither a contradiction nor a completeness claim: the
producer emitted every row its search produced, and the declaration says that
search does not produce every row there was. `FusionTrailer::certain_prefix` is
what a caller with a completeness obligation can still act on — how many leading
rows keep their places whatever the degraded strata did or did not find. It
claims membership and never absence.

That claim needs a term no row carries, because the candidate that could take an
emitted row's place is the one a lossy search never named — and it is therefore
not among the rows to compare against. `FusionTrailer::unemitted_ceiling` is that
term: the highest score anything outside the answer could have, over both the
candidates no stream named and the ones a bounded read left uncertified. A row is
certain of its place only when its own floor clears it.

Nothing here mints a vocabulary. Producers, strata and weights are
caller-supplied configuration; the fixtures use `example.org`. There is no
default registry and no built-in producer.

**One stratum, one producer.** A rank is meaningful only inside the list that
assigned it, so merging two ranked lists needs either a comparable score — which
a rank is not — or a fusion rule, and this crate *is* the fusion rule. Two
producers under one stratum have neither, and their rows could only be
concatenated: the second producer's best row would surface below the whole of the
first's output and decay as though it had lost to rows it never competed with. So
the configuration is refused where it is committed, in `register_ranked`, and the
refusal names the two ways to express it instead. Producers whose scores are
already comparable — shards, per-language segments, a partitioned index — and
between which no weight is meant to stand, merge inside **one** producer, which
owns that comparability. Producers that score by different laws take **a stratum
each**, where the weighted sum across strata is the design. The first exit is not
interchangeable with the second: each stratum is a summand, so shards recast as
strata would give a candidate they both hold two contributions where the host
meant one family's worth.

The first exit takes **two** things, not one: comparable scores, *and* a
weighting that can ride inside the score. Sharing a law buys only the first. Two
classes over one embedding space — headings and bodies, say — share a law
exactly, but if one is meant to **outweigh** the other and the shared score is a
*bounded* metric (a cosine distance in `[0, 2]`, lower-better), the merge cannot
carry the weight: sorting merged by `d / w`, the favoured class wins only past a
similarity edge of `(1 - s)(1 - w_low/w_high)`, which at a weight ratio of one
half is `0.45` against a match at similarity `0.1` and `0.0005` against one at
`0.999` — largest for the worst matches, and zero for a perfect one, which is
where the top-k contest is decided. That pair takes **a stratum each** despite
the shared law, because such a weight can act only in rank space, and rank space
is what a stratum is. An unbounded score — BM25F's field weights are natively one
— does carry a differential weight through a merge, and belongs at the first
exit. So the question to ask over a shared law is: *do you want these two
weighted differently, and is the score bounded?*

A profile's weights are read as **ratios only**, so the constructor that built
them matters and nothing can refuse the wrong one: `Fixed::ONE` and
`Fixed::from_integer(1)` are the number one, while `Fixed::from_raw(1)` is one
raw unit of `10^-12`. A weight map mixing the two spellings runs, refuses
nothing, and ranks as though the smaller stratum were absent. How many
contributions a candidate may receive is not a knob at all — it is the number of
strata the profile weights, because a candidate surfaces at most once in each.

## Run it

The whole ladder, end to end, over real data and two real ranked producers:

```sh
cargo run -p purrdf-retrieval --example fused_search
```

`examples/fused_search.rs` indexes one small corpus twice over — once lexically
with a real BM25 index, once geometrically with a real embedding space over a
sealed PURREMB artifact — registers each as its own ranked producer under its
own stratum, and runs `search` over a request that reaches both. The two
producers rank the same four documents in nearly opposite orders, which is the
case fusion exists for, and the printed answer gives each row's per-stratum
provenance, every producer's terminal status, and the one request term nothing
in that registry accepts. It is documentation that executes.

Reached from the umbrella crate as `purrdf::retrieval`, so a consumer that
registers a ranked relation composes the answer without a second dependency.

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
