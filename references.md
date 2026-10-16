# Referenced artifacts — issue #484

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/sparql-eval/src/bgp.rs:878-882` | yes | repo | file | 6810 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `rdf-core/src/ir/dataset.rs:1252-1261` | NO | repo | — | — | — |
| `rdf-core/src/ir/paged/summary.rs:421-460` | NO | repo | — | — | — |
| `paged/admission.rs:171-190` | NO | repo | — | — | — |
| `admission.rs:182-183` | NO | repo | — | — | — |
| `ir/segmented/session.rs:1242-1296` | NO | repo | — | — | — |
| `rdf-core/src/dataset_view.rs:679-689` | NO | repo | — | — | — |
| `bgp.rs:503-517` | NO | repo | — | — | — |
| `bgp.rs:968-972` | NO | repo | — | — | — |
| `ir/mutable/delta_view.rs:705-722` | NO | repo | — | — | — |
| `retrieval/src/statistics.rs:81-156` | NO | repo | — | — | — |
| `1/T` | NO | repo | — | — | — |
| `paudley/484-planner-statistics-per-predicate-per` | NO | repo | — | — | — |
| `crates/sparql-eval/src/bgp.rs` | yes | repo | file | 6810 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `rdf-core/src/ir/dataset.rs` | NO | repo | — | — | — |
| `rdf-core/src/ir/paged/summary.rs` | NO | repo | — | — | — |
| `paged/admission.rs` | NO | repo | — | — | — |
| `ir/segmented/session.rs` | NO | repo | — | — | — |
| `rdf-core/src/dataset_view.rs` | NO | repo | — | — | — |
| `ir/mutable/delta_view.rs` | NO | repo | — | — | — |
| `retrieval/src/statistics.rs` | NO | repo | — | — | — |

### History: `crates/sparql-eval/src/bgp.rs:878-882`

- `2c13dbe4d Fix merged estimates and qualify backend explanation receipts`
- `d0f544f49 perf(rdf): index structural reifiers and annotation values`
- `47725646e Stop bounded statement scans and index segmented source probes`
- `ae502bfc1 Keep nested SELECT star projections persistent across consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `5e1f3658a fix(eval): retain query evidence ownership and closed absence`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `e72660e87 fix(sparql): compile contextual mappings through native algebra`
- `af0019173 Price GRAPH over a name that addresses no graph as an empty scope`

### History: `crates/sparql-eval/src/bgp.rs`

- `2c13dbe4d Fix merged estimates and qualify backend explanation receipts`
- `d0f544f49 perf(rdf): index structural reifiers and annotation values`
- `47725646e Stop bounded statement scans and index segmented source probes`
- `ae502bfc1 Keep nested SELECT star projections persistent across consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `5e1f3658a fix(eval): retain query evidence ownership and closed absence`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `e72660e87 fix(sparql): compile contextual mappings through native algebra`
- `af0019173 Price GRAPH over a name that addresses no graph as an empty scope`

## Identifiers (files containing each at HEAD)

- `main`: 2073
- `DatasetView::cardinality_estimate`: 6
- `RdfDataset`: 744
- `PageSummary`: 8
- `PageStream::Base`: 5
- `emit_virtual_candidates`: 4
- `Statistics`: 48
- `DatasetView`: 303
- `a84f28cdd`: 0
- `de36c4840`: 0

## Workspace packages named

- `purrdf`

## URLs

None.
