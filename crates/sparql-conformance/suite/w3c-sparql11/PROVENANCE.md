<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Vendored W3C SPARQL 1.1 conformance fixtures

This tree vendors a **curated subset** of the official W3C SPARQL 1.1 test suite,
exercising the exotic-aggregation, deep-subquery, and federated-`SERVICE` surface.
It is consumed by the native conformance harness (`crates/sparql-conformance`).

## Source

- Upstream: **W3C `rdf-tests`** — <https://github.com/w3c/rdf-tests>,
  path `sparql/sparql11/`.
- Mirror of the W3C DAWG/SPARQL-WG test suite at
  <https://www.w3.org/2009/sparql/docs/tests/>.
- The curated `aggregates`/`subquery`/`service` subset was fetched from the
  `main` branch on **2026-06-26**.
- The full query-eval groups (see below) are vendored **verbatim** at the pinned
  commit **`426c7df4b5d5d292e3ba09dc22e622ea301f230a`** — every file, `manifest.ttl`
  included, carries its own `LicenseRef-W3C-Test-Suite` `.license` sidecar.

## Full W3C query-eval groups (commit `426c7df`)

Ten groups are vendored verbatim and discovered automatically by the harness
(one nextest case per `manifest.ttl`). Unlike the curated subset, these ship the
**upstream** `manifest.ttl` verbatim (sidecar'd), so the whole group runs. Every
non-passing case is recorded in `crates/sparql-conformance/src/xfail.rs` with a
typed reason — nothing is silently skipped.

| Group | Cases | Green | Ledgered (reason) |
|-------|------:|------:|-------------------|
| bind | 10 | 10 | — |
| bindings | 11 | 10 | 1 result-format (Turtle `rs:ResultSet`) |
| cast | 6 | 0 | 6 value-mismatch (XSD cast lexical/datatype) |
| construct | 7 | 2 | 1 parse-unsupported, 4 unsupported-construct (CONSTRUCT WHERE) |
| exists | 6 | 5 | 1 unsupported-construct (EXISTS over GRAPH var) |
| functions | 75 | 59 | 14 value-mismatch, 2 non-deterministic (BNODE labels) |
| grouping | 6 | 4 | 2 unsupported-construct (missing non-grouped-var rejection) |
| negation | 12 | 12 | — |
| project-expression | 7 | 7 | — |
| property-path | 33 | 24 | 9 property-path (inverse-in-NPS, `*`/`?` over sets) |

The ledgered gaps are genuine (the curated subset simply never exercised these
surfaces). The trailing-`VALUES` parser fix (§18.2.4.3) cleared the 9 `bindings`
VALUES cases and the `service` group's `service4a`; `value-mismatch` marks real
evaluation-correctness gaps still to close.

## Full W3C UPDATE-eval groups (commit `426c7df`)

Eleven update groups are vendored verbatim and run through the harness's
UPDATE-eval path (`SparqlEngine::update` → RDFC-1.0 canonical post-state diff).
The engine's UPDATE implementation is strong: 97 of 102 cases pass outright.

| Group | Cases | Green | Ledgered (reason) |
|-------|------:|------:|-------------------|
| add | 8 | 7 | 1 update-semantics |
| basic-update | 13 | 11 | 2 update-semantics (cross-op bnode scoping) |
| clear | 4 | 4 | — |
| copy | 6 | 4 | 2 update-semantics |
| delete | 19 | 19 | — |
| delete-data | 6 | 6 | — |
| delete-insert | 17 | 17 | — |
| delete-where | 6 | 6 | — |
| drop | 4 | 4 | — |
| move | 6 | 6 | — |
| update-silent | 13 | 13 | — |

The 5 `update-semantics` residuals are genuine post-state divergences (COPY/ADD
graph edge cases; blank-node scoping across separate INSERT operations).

## W3C entailment-regime group (commit `426c7df`)

The `entailment/` group's `sd:entailmentRegime` is read by the harness, which
materializes the dataset's closure via the native `purrdf-entail` reasoner
(RDFS + OWL-RL-shaped) before the query runs. **40 of 70 cases pass** — every
`rdf*`/`rdfs*`/`lang`/`plainLit`/`bind*` case and many OWL cases. The 30
residuals are ledgered `Entailment`: OWL-Direct-only (`parent*`, `simple*`) and
OWL-DL query answering (`sparqldl-*`, `paper-sparqldl-Q*`) — full DL
is not a materialize-and-match affair; RIF-rule entailment (`rif*`); and RDF
axiomatic-triple entailment under the bare RDF regime (`rdf01`). These are
spec-inherent boundaries of a forward-materialization reasoner, not silent skips.

## License

The W3C test files are published under the **W3C Test Suite License** / **W3C
Software and Document License** — see
<https://www.w3.org/Consortium/Legal/2015/copyright-software-and-document>.
They are vendored verbatim (query + data) and are **not** relicensed; each carries
a `.license` SPDX sidecar (`SPDX-License-Identifier: LicenseRef-W3C-Test-Suite`).
The selector `manifest.ttl` files and this document are PurRDF-authored
(MIT OR Apache-2.0).

## Vendored files & fidelity

| Group | Query / Data | Fidelity |
|-------|--------------|----------|
| aggregates | `agg-numeric.ttl`, `agg-group-builtin.rq`, `agg-sum-01.rq`, `agg-multiple-having.rq` | **verbatim** from `sparql/sparql11/aggregates/` |
| subquery | `sq13.rq`, `sq13.ttl` | **verbatim** from `sparql/sparql11/subquery/` |
| service | `service0{1,2,3,4a,5,6,7}.rq`, `service0{1..7}.srx`, `data*.ttl` (default-graph + per-endpoint) | **verbatim** from `sparql/sparql11/service/` |

The expected-result files (`*.srx`) for the `aggregates` and `subquery` groups are
**reconstructed to a semantically equivalent** SPARQL Results XML document: the
harness compares SELECT results as a W3C *solution-set multiset* (via the native
`from_xml` reader), so the exact bytes of those upstream `.srx` are immaterial —
only the solution content is, and that is reproduced faithfully from the upstream
expected results. The `service` group's `.srx` files are vendored **verbatim** to
exercise the reader against the upstream fixtures as-published (see the
upstream-erratum note below).

## Curation rationale

- `agg-group-builtin` — `GROUP BY (DATATYPE(?o) AS ?d)` directly exercises the
  expression-valued `GROUP BY`.
- `agg-multiple-having` — `HAVING (COUNT(*) > 1) (COUNT(*) < 3)` exercises
  multi-condition `HAVING`.
- `agg-sum-01` — `SUM` over the XSD decimal value space.
- `subquery13` ("Subqueries don't inject bindings") — a nested `SELECT` whose
  inner variable scope is independent of the outer query; it also exercises
  blank-node property lists (`[ rdfs:label ?L ]`).

## The W3C federated `service` group runs offline

The W3C `sparql11/service` tests bundle **each remote endpoint's data in the
manifest** via `qt:serviceData [ qt:endpoint <ep> ; qt:data <file> ]`. The harness
resolves every endpoint through an in-memory source (`LocalRemoteQuerySource`),
which dog-foods the native engine — no socket, no live HTTP, fully deterministic.
The whole group therefore runs offline alongside the rest of the suite.

All seven vendored cases now pass. The last three capability gaps were closed:

- **nested `SERVICE`** (`service3`) — a `SERVICE` inside another `SERVICE`'s
  pattern now resolves: the in-memory source threads itself into the forwarded
  evaluation, so the inner endpoint is resolved against the same sources.
- **trailing top-level `VALUES`** (`service4a`) — the parser now accepts a
  `VALUES DataBlock` after the WHERE/solution-modifiers (§18.2.4.3), joined with
  the group graph pattern.
- **variable-endpoint `SERVICE ?var`** (`service5`) — evaluated via the LATERAL
  seam: the endpoint variable is bound from the enclosing solution per row and
  substituted into a concrete `SERVICE <iri>` before federating.

Live federation over real HTTP endpoints is exercised separately by the maintainer
network-lane test (`crates/sparql-eval/tests/service_live.rs`), which drives the
real `HttpRemoteQuerySource`.
