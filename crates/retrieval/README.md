<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: CC-BY-4.0
-->

# purrdf-retrieval

The composition layer over the ranked property-function producers.

PurRDF already answers ranked retrieval through several relations reached
through the evaluator's property-function seam. This crate is the layer above
them: it turns one request into one plan, and — once the later stages land —
compiles, executes and fuses that plan. It is a composition **outside the
kernel**: nothing in `purrdf-core` or the seam changes to admit it.

This crate currently ships the plan value itself:

* `RetrievalRequest` / `RequestTerm` — the typed term lattice a request is
  expressed in (lexical, vector, spatial, entity seed).
* `Plan` — pure, inspectable, editable and serializable data: the request
  terms, per-producer bindings, selected/rejected producers with reasons,
  per-stratum depths and weights, the statistics snapshot it was planned
  against, and both registry identities.
* `PlanId` — a domain-separated BLAKE3 digest over a versioned, canonical,
  length-framed encoding of the plan. A version mismatch on decode refuses
  loudly rather than guessing.

Nothing here mints a vocabulary. Producers, strata and weights are
caller-supplied configuration; the fixtures use `example.org`. There is no
default registry and no built-in producer.
