# Issue #563: Segmented scans fetch their block through the cache once per row

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## What happens

`QuadIter::next` (`crates/rdf-core/src/ir/segmented/session.rs`) returns one quad per call, and
each call starts over: it checks `session.ready()` (the state mutex), fetches the current page
again through `session.block(page)` (a cache lookup, then `detach_cache_links` and
`append_cache_links` to move the block to the front), and re-parses the block's record header
with `fixed_records`, before decoding the next row. A full scan therefore pays one locked cache
transaction per row, not per block.

## Measured

Katamari's persisted vault graph, 200,000 claims as two segmented layers (1.2 million rows),
reopened from disk, `SELECT (COUNT(*) AS ?n) WHERE { ?s ?p ?o }`: 8.9-19 s (the in-memory graph
over the same claims answers in about 9 s). `perf` over the queries, self time:

| symbol | share |
|---|---:|
| `SegmentedSession::block` | 11.1% |
| `State::detach_cache_links` | 10.6% |
| `SegmentedSession::ready` | 9.9% |
| `SegmentedOperation::ready` | 7.9% |
| `SegmentedSession::summary` | 6.3% |
| `SegmentedStackQuery::intern_source_term` | 5.9% |
| `QuadIter::next` | 5.4% |
| `SegmentedReservation::resize` | 3.3% |
| `State::append_cache_links` | 2.0% |

About 45% of the scan is the cache and readiness bookkeeping above, repeated per row.

## Request

Hold the current block (its guard or `Arc`) and its parsed records in the iterator until its rows
are exhausted, checking readiness once per block rather than once per row; the same for the
reifier and annotation iterators if they share the shape. A scan would then cost one cache
transaction per block.

## Evidence

Katamari `docs/assessment/evidence/graph-layers-2026-10-15.md` (`graph-seed-compacted`), and
`crates/katamari-unlock/tests/idle_cost.rs` `the_restart` with `KATAMARI_GRAPH_LAYERS=1`.


## Comments (0)

