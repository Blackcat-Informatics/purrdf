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
  consults. There is no built-in provider.
* `Plan` — pure, inspectable, editable and serializable data: the request
  terms, per-producer bindings, selected/rejected producers with reasons, every
  request term that reached no producer at all and why (`UnservedTerm`), the
  per-stratum depths and weights, the statistics snapshot it was planned
  against, and both registry identities.
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
  run becomes its own `ProducerStatus` while every other stratum streams on.
* `fuse(streams, profile, k)` / `search(…, k)` — the exact fixed-point
  reciprocal-rank fusion, bounded by the caller's `TopK` because fused
  enumeration is top-k by construction. The answer carries every applicable
  producer's own status in its trailer — including those that could not answer —
  and every request term that reached nothing.

Nothing here mints a vocabulary. Producers, strata and weights are
caller-supplied configuration; the fixtures use `example.org`. There is no
default registry and no built-in producer.
