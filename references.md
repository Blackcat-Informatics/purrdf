# Referenced artifacts — issue #563

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/rdf-core/src/ir/segmented/session.rs` | yes | repo | file | 2474 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `docs/assessment/evidence/graph-layers-2026-10-15.md` | NO | not here; exists in sibling(s): katamari | — | — | — |
| `crates/katamari-unlock/tests/idle_cost.rs` | NO | not here; exists in sibling(s): katamari | — | — | — |

### History: `crates/rdf-core/src/ir/segmented/session.rs`

- `225e078da Keep optional routing growth from evicting admitted payloads`
- `57a12dbd1 Run routing pressure regression through the portable public harness`
- `ee13e8327 Keep admitted dictionary reads usable when routing hints cannot grow`
- `2ac66bf40 Keep shared integration checks compatible with current Clippy`
- `702235c70 Retain authenticated dictionary routing across payload eviction`
- `4b031fbac fix(storage): pack segmented dictionaries to their byte bounds`
- `839018278 Persist certified source statistics for segmented queries`
- `f8cbba207 Restore keyed segmented statements and index block cache`
- `39cae7092 fix(core): plan cache growth with the pending block layout`
- `2def24ab9 fix(core): reclaim unused cache for reader buffer admission`

## Identifiers (files containing each at HEAD)

- `QuadIter::next`: 0
- `detach_cache_links`: 1
- `append_cache_links`: 1
- `fixed_records`: 3
- `perf`: 718
- `SegmentedSession::block`: 0
- `State::detach_cache_links`: 0
- `SegmentedSession::ready`: 0
- `SegmentedOperation::ready`: 0
- `SegmentedSession::summary`: 0
- `SegmentedStackQuery::intern_source_term`: 0
- `SegmentedReservation::resize`: 0
- `State::append_cache_links`: 0
- `Arc`: 930
- `the_restart`: 0

## Workspace packages named

None.

## URLs

None.
