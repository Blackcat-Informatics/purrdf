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
- Fetched from the `main` branch on **2026-06-26**.

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

Of the seven vendored cases, four pass outright (a simple `SERVICE` join, a
`SERVICE` with an `OPTIONAL SERVICE`, a `SERVICE SILENT` against an invalid
endpoint that swallows to the join identity, and `service7` whose upstream
expected-result file uses an empty `<binding>` element to denote an unbound
variable — an older producer convention that the reader now tolerates correctly).
The remaining three are recorded as explicit expected-failures (never silently
skipped) for these capability gaps:

- **nested `SERVICE`** — a `SERVICE` clause inside another `SERVICE`'s pattern is
  not yet evaluated against its inner endpoint (*unsupported-construct*).
- **trailing top-level `VALUES`** — a `VALUES` clause after the `WHERE` block is
  not yet accepted by the parser, only inline `VALUES` inside a group
  (*unsupported-construct*).
- **variable-endpoint `SERVICE ?var`** — needs the lateral binding seam to bind the
  endpoint from the surrounding solution before federating (*pending-service*).

Live federation over real HTTP endpoints is exercised separately by the maintainer
network-lane test (`crates/sparql-eval/tests/service_live.rs`), which drives the
real `HttpRemoteQuerySource`.
