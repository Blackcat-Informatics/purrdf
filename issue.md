# Issue #542: rdf-core: MutableDataset::freeze uses fixed default ViewLimits; allow caller limits

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

`MutableDataset::freeze` (`crates/rdf-core/src/ir/mutable.rs`) calls `snapshot_view()` with the default `ViewLimits`, so a dataset whose auxiliary bytes pass 512 MiB cannot be frozen at all: it is refused with `view-retention-limit`. `snapshot_view_with_limits` exists for snapshots, but there is no way to freeze under caller-chosen limits.

Katamari's query projection publishes every view with `UNBOUNDED` limits and compacts its mutable dataset through `freeze`. Loading a synthetic corpus of about 950k claims into one dataset is refused at that compaction step, though the process stayed far below the machine's memory.

## Ask

A `freeze_with_limits(&self, limits: ViewLimits)` (or `freeze` honouring limits the dataset was built with), so a caller can freeze under the same limits it already publishes snapshots with.


## Comments (0)

