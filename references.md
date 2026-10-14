# Referenced artifacts — issue #542

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/rdf-core/src/ir/mutable.rs` | yes | repo | file | 4065 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |

### History: `crates/rdf-core/src/ir/mutable.rs`

- `c3171c015 Admit complete operational SPARQL storage before allocation`
- `be3fdeb84 Preserve independent record ordinals during classification`
- `20e2e637c Preserve typed graph transfers and fresh LOAD identities`
- `b0fc9e889 Preserve physical RDF record roles in mutable datasets`
- `dd9109995 core: layer sealed paged generations with pinned mutable snapshots`
- `5231acd17 fix(core): withdraw a populated remembered slot the same from either origin`
- `cb9a5b379 docs(core): name the mutable dataset, not a branch, in graph-mode rustdoc`
- `1305804df docs: document named graph lifetime and its expected v4.0 default`
- `b050f7848 fix(sparql): checkpoint declaration work and final UPDATE publication`
- `e0343ae16 Avoid allocating the named graph registry for capability reads`

## Identifiers (files containing each at HEAD)

- `MutableDataset::freeze`: 6
- `ViewLimits`: 44
- `snapshot_view_with_limits`: 3
- `UNBOUNDED`: 106
- `freeze`: 628

## Workspace packages named

None.

## URLs

None.
