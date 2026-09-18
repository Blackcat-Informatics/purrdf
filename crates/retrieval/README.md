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
  reported per term as an `UnservedTerm`, never dropped.
* `plan(request, registry, statistics)` — the pure planner. It matches request
  terms to producers by a lookup over the producers' declared capabilities,
  records selected and rejected producers with reasons, derives per-stratum
  depths from the registry's own row-bound declarations capped by statistics,
  and records the statistics snapshot and both registry identities. It opens no
  store, no file and no clock.
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
* `fuse(streams, profile, k)` / `search(…, k)` — the exact fixed-point
  reciprocal-rank fusion, bounded by the caller's `TopK` because fused
  enumeration is top-k by construction. The bound stops the reading as well as
  the returning: a producer still holding rows when it is reached is reported at
  the contribution it was read down to, never drained to make it declare
  exhaustion. The answer carries every applicable producer's own status in its
  trailer — including those that could not answer — and every request term that
  reached nothing.

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
