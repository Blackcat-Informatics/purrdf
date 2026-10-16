# Referenced artifacts — issue #560

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/rdf-core/src/ir/segmented/format.rs` | yes | repo | file | 486 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/katamari-query-purrdf/src/layers.rs` | NO | not here; exists in sibling(s): katamari | — | — | — |

### History: `crates/rdf-core/src/ir/segmented/format.rs`

- `58c1fa987 Enforce logical capacity refusal at native producer boundaries`
- `80ec82890 Make native capacity refusal typed and compose sparse dataset views`
- `04e516a1a feat!: establish v3 storage and distribution contracts`

## Identifiers (files containing each at HEAD)

- `SegmentedBuildLimits`: 22
- `block_bytes`: 8
- `records_per_block`: 5
- `ConstructionLimit`: 5
- `SegmentedBuilder::new`: 17
- `value_bound`: 3
- `build_limits`: 0
- `seal_image`: 0

## Workspace packages named

- `purrdf`

## URLs

None.
