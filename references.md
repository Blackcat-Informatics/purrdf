# Referenced artifacts — issue #553

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/rdf-core/src/ir/segmented/query/view.rs` | yes | repo | file | 758 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/rdf-core/src/ir/segmented/session.rs:955-976` | yes | repo | file | 2171 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/sparql-eval/src/statement_layer.rs` | yes | repo | file | 179 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/rdf-core/src/dataset_view.rs` | yes | repo | file | 2465 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/rdf-core/src/ir/segmented/session.rs` | yes | repo | file | 2171 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |

### History: `crates/rdf-core/src/ir/segmented/query/view.rs`

- `47725646e Stop bounded statement scans and index segmented source probes`
- `5ec569c32 Consolidate typed capacity carriers through existing homes`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`

### History: `crates/rdf-core/src/ir/segmented/session.rs:955-976`

- `39cae7092 fix(core): plan cache growth with the pending block layout`
- `2def24ab9 fix(core): reclaim unused cache for reader buffer admission`
- `f7862dccc fix(core): refund failed growth and preserve the first read fault`
- `7c39fae37 fix(core): finish strict verification of reader bookkeeping evidence`
- `b12019ebe fix(core): distinguish retained reader bookkeeping from query owners`
- `47725646e Stop bounded statement scans and index segmented source probes`
- `b5db98eb7 Admit segmented request and cache buffers as they grow`
- `7a3f96ef7 Share admitted segmented request identities without losing their residency charge`
- `5ec569c32 Consolidate typed capacity carriers through existing homes`
- `15bc79027 Propagate typed capacity failures through workspace consumers`

### History: `crates/sparql-eval/src/statement_layer.rs`

- `47725646e Stop bounded statement scans and index segmented source probes`
- `9c53340b1 feat(license): offer MulanPSL-2.0 as a third license option (#352)`
- `eb03ab702 fix(331): narrow the RDF 1.2 side tables by graph before the scan`
- `8b9e30946 feat(sparql-eval,purrdf,cli,python)!: bind path witnesses as property functions over the closure (#224)`

### History: `crates/rdf-core/src/dataset_view.rs`

- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `1ef59cbf9 Address GRAPH <iri> only when the IRI names a graph`
- `d6f94f62a Preserve concrete blank identity across governed runtime allocation`
- `04e516a1a feat!: establish v3 storage and distribution contracts`
- `8d61324db refactor!: one home each for the triple-term and schema nesting bounds`
- `b8b661d2a refactor: one body per job across the foundation crates`
- `bf1282f00 refactor: one body per job across the query, reasoning and index engines`
- `f626418a6 refactor: close remaining duplicate walkers, escapes and stacks`

### History: `crates/rdf-core/src/ir/segmented/session.rs`

- `39cae7092 fix(core): plan cache growth with the pending block layout`
- `2def24ab9 fix(core): reclaim unused cache for reader buffer admission`
- `f7862dccc fix(core): refund failed growth and preserve the first read fault`
- `7c39fae37 fix(core): finish strict verification of reader bookkeeping evidence`
- `b12019ebe fix(core): distinguish retained reader bookkeeping from query owners`
- `47725646e Stop bounded statement scans and index segmented source probes`
- `b5db98eb7 Admit segmented request and cache buffers as they grow`
- `7a3f96ef7 Share admitted segmented request identities without losing their residency charge`
- `5ec569c32 Consolidate typed capacity carriers through existing homes`
- `15bc79027 Propagate typed capacity failures through workspace consumers`

## Identifiers (files containing each at HEAD)

- `c2c1c8c78`: 0
- `SegmentedStackQuery`: 6
- `reifier_quads_of`: 30
- `annotations_of_with_graph`: 31
- `SegmentedSession`: 23
- `SegmentedSession::block`: 0
- `Vec`: 1641
- `position`: 1286
- `remove`: 936
- `push`: 1458

## Workspace packages named

None.

## URLs

None.
