<!--
SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Vendored W3C SPARQL 1.1 conformance fixtures

This tree vendors the official W3C SPARQL 1.1 test suite: the full query-eval,
UPDATE-eval, entailment-regime, and **complete syntax** (query + update +
federation) groups verbatim at a pinned commit, plus a small PurRDF-curated
`aggregates`/`subquery`/`service` selector subset over the exotic-aggregation,
deep-subquery, and federated-`SERVICE` surface. It is consumed by the native
conformance harness (`crates/sparql-conformance`).

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
| bindings | 11 | 11 | — |
| cast | 6 | 3 | 3 upstream-erratum (`cast-decimal`, `cast-double`, `cast-float`) |
| construct | 7 | 7 | — |
| exists | 6 | 6 | — |
| functions | 75 | 73 | 2 upstream-erratum (`coalesce01`, `plus-1-corrected`) |
| grouping | 6 | 6 | — |
| negation | 12 | 12 | — |
| project-expression | 7 | 7 | — |
| property-path | 33 | 33 | — |

This table is derived from `crates/sparql-conformance/src/xfail.rs::XFAIL`, the
same registry `run_manifest` honors, not maintained separately from it: every
row's ledgered count is that registry's live count for the group, not a count
that can drift out from under it. The 5 ledgered cases above are the entire
`XFAIL` registry, and every one of them is `XfailReason::UpstreamErratum` — a
fixture whose expected lexical form the W3C manifest itself states
inconsistently (see the reasons recorded alongside each entry in `xfail.rs`),
not a native-engine gap. `construct`, `exists`, `grouping`, and `property-path`
each once ledgered several `unsupported-construct`/`property-path` gaps
(CONSTRUCT WHERE, EXISTS over a GRAPH variable, non-grouped-variable
rejection, inverse paths inside a negated property set); all of them are
implemented now and none is ledgered.

## Full W3C UPDATE-eval groups (commit `426c7df`)

Eleven update groups are vendored verbatim and run through the harness's
UPDATE-eval path (`SparqlEngine::update` → RDFC-1.0 canonical post-state diff).
All 102 UPDATE-eval cases pass outright; the ledger holds no `update-semantics`
entry.

| Group | Cases | Green | Ledgered (reason) |
|-------|------:|------:|-------------------|
| add | 8 | 8 | — |
| basic-update | 13 | 13 | — |
| clear | 4 | 4 | — |
| copy | 6 | 6 | — |
| delete | 19 | 19 | — |
| delete-data | 6 | 6 | — |
| delete-insert | 17 | 17 | — |
| delete-where | 6 | 6 | — |
| drop | 4 | 4 | — |
| move | 6 | 6 | — |
| update-silent | 13 | 13 | — |

The five `update-semantics` divergences these groups once ledgered (COPY/ADD
graph edge cases; blank-node scoping across separate INSERT operations) are
fixed: the ledger is asserted with XPASS discipline, so a case that starts
passing must leave it, and all five did.

## Full W3C syntax groups (commit `426c7df`)

The complete SPARQL 1.1 syntax surface is vendored verbatim and gated through the
parser only (`SparqlParser::parse_query` for query-syntax cases,
`parse_update` for update-syntax cases) — no dataset, no evaluation. Positive
cases must parse `Ok`, negatives must parse `Err`. Syntax tests are parsed with
the test file's own IRI as the in-scope `BASE` (§4.1.1.1), matching the W3C
convention, so relative IRI references resolve. **The entire surface passes —
zero ledgered residuals.**

| Group | Cases | Green | Ledgered (reason) |
|-------|------:|------:|-------------------|
| syntax-query | 94 | 94 | — |
| syntax-update-1 | 54 | 54 | — |
| syntax-update-2 | 1 | 1 | — |
| syntax-fed | 3 | 3 | — |

Five genuine parser gaps these groups surfaced are fixed rather than
ledgered: two relative-IRI positives (resolved via the per-file `BASE` above);
`SELECT *` in an aggregate query is now rejected (§11.1); a `BIND(… AS ?v)`
whose target is already in scope is rejected (§19.6); and reuse of a blank-node
label across two `INSERT DATA` operations is rejected (§4.1.1) — while the same
template label legitimately recurs across `INSERT … WHERE` operations.

## W3C entailment-regime group (commit `426c7df`)

The `entailment/` group's `sd:entailmentRegime` is read by the harness, which
answers each case under the regime the manifest names: forward materialization via
the native `purrdf-entail` reasoner for the RDF/RDFS/D/OWL-RL regimes, a
query-directed SHOIQ(D) tableau (`purrdf_entail::materialize_dl_reported`) for OWL-Direct, and
a Horn forward chase over the referenced RIF-in-XML rule documents
(`purrdf_entail::materialize_rif`) for RIF. **The entire group passes — 70 of 70,
with zero ledgered residuals**, which the harness prints as
`70 passed, 0 xfail, 0 unexpected-pass, 0 failed, 0 unmodeled`.

That covers every `rdf*`/`rdfs*`/`lang`/`plainLit`/`bind*` case, the
`parent*`/`simple*`/`owlds*` OWL-Direct cases, the full `sparqldl-*` /
`paper-sparqldl-Q*` OWL-DL query-answering set, and all four `rif*` cases;
`crates/sparql-conformance/src/xfail.rs` records the same fact on the ledger side,
where this group has no `Entailment` entry at all.

### RIF rule-document sub-corpus (distinct upstream)

The `rif*` entailment cases' `.ttl` fixtures reference RIF rule documents
(`<rif01.rif>`, `<Frames-premise.rif>`, `<Modeling_Brain_Anatomy-premise.rif>`,
`<RDF_Combination_Blank_Node-premise.rif>`) that are **absent from `w3c/rdf-tests`
at commit `426c7df`** — the upstream SPARQL suite ships only the `.ttl`/`.rq`/`.srx`
sides. Those referenced documents are the W3C **RIF Working Group** test-case files,
vendored here verbatim so the `rif*` cases are runnable end-to-end:

| File | Upstream source |
|------|-----------------|
| `rif01.rif` | `www.w3.org/2009/sparql/docs/tests/data-sparql11/entailment/rif01.rif` |
| `Frames-premise.rif` | `www.w3.org/2005/rules/test/repository/tc/Frames/` |
| `Modeling_Brain_Anatomy-premise.rif` (+ `-import001.rdf`) | `www.w3.org/2005/rules/test/repository/tc/Modeling_Brain_Anatomy/` |
| `RDF_Combination_Blank_Node-premise.rif` (+ `-import001`) | `www.w3.org/2005/rules/test/repository/tc/RDF_Combination_Blank_Node/` |

The two `-import001*` files are the RDF data pulled in by each premise's
`<directive><Import>` element (RDF/XML and N-Triples respectively). Each file
carries its own `LicenseRef-W3C-Test-Suite` `.license` sidecar recording its exact
source URL.

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
