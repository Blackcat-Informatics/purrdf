# Referenced artifacts — issue #551

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/rdf-core/src/ir/segmented/query.rs:228-232` | yes | repo | file | 436 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/lex/src/allocation.rs:503-526` | yes | repo | file | 1229 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/katamari-unlock/tests/idle_cost.rs` | NO | not here; exists in sibling(s): katamari | — | — | — |
| `crates/rdf-core/src/ir/segmented/query.rs` | yes | repo | file | 436 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/lex/src/allocation.rs` | yes | repo | file | 1229 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |

### History: `crates/rdf-core/src/ir/segmented/query.rs:228-232`

- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`

### History: `crates/lex/src/allocation.rs:503-526`

- `78d32abe0 Keep ordinary evaluator child continuations on admitted worklists`
- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `675df1658 Implement native geodesic kernel and physical spatial ownership`
- `2ce33c668 feat(entail): reason over regular OWL role chains`
- `c3171c015 Admit complete operational SPARQL storage before allocation`

### History: `crates/rdf-core/src/ir/segmented/query.rs`

- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`

### History: `crates/lex/src/allocation.rs`

- `78d32abe0 Keep ordinary evaluator child continuations on admitted worklists`
- `15bc79027 Propagate typed capacity failures through workspace consumers`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `f49c2f1a5 Carry query evidence and scoped ranked retrieval through physical shapes`
- `675df1658 Implement native geodesic kernel and physical spatial ownership`
- `2ce33c668 feat(entail): reason over regular OWL role chains`
- `c3171c015 Admit complete operational SPARQL storage before allocation`

## Identifiers (files containing each at HEAD)

- `main`: 2068
- `RdfDatasetBuilder`: 505
- `PackBuilder::build_view_bytes`: 9
- `SegmentedImage::from_pack_v1`: 3
- `SegmentedReceipt::from_authenticated_bytes`: 3
- `SegmentedProvider`: 11
- `read_exact_at`: 0
- `SegmentedLayer`: 8
- `SegmentedStack::publish`: 0
- `NativeSparqlEngine::query_fallible_view`: 0
- `SegmentedSession::block`: 0
- `Memory::reserve`: 1
- `Vec`: 1641
- `SegmentedReadLimits`: 23
- `the_segmented_graph`: 0
- `reserve_for_push`: 13
- `growth_capacity`: 6
- `dc9741fcc`: 0
- `b5db98eb7`: 0

## Workspace packages named

- `purrdf`

## URLs

None.
