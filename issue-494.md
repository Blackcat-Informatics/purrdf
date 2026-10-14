# Issue #494: Evaluate at caller-supplied coordinates: host-supplied NOW(), as-of snapshots of a PagedStack, and range scans on annotation values

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Let a caller evaluate a query or a rule set **at explicit coordinates** and get the same answer every time it asks with the same coordinates over the same snapshot:

1. a host-supplied value for `NOW()` (and the `RAND()` / `UUID()` seed), instead of the wall clock;
2. an **as-of position** for layered snapshots: read a `PagedStack` as it was after its first *k* sealed generations, as one snapshot;
3. range predicates on ordered annotation values (valid-from / valid-until style) pushed into scans, so "valid at time *t*" is a range scan rather than a filter over every annotation.

## Current behaviour (`main` at `6273b6173`)

- `NOW()` always reads the host wall clock: `crates/sparql-eval/src/clock.rs:1-35` ("every target returns the correct live value — there is no caller-visible knob"); `engine.rs:1360` and `eval.rs:996` set it from `wall_clock_now()`. `EvalCtx::with_now` exists but is `#[cfg(test)]` only (`eval.rs:1135-1143`). The same holds for the entropy seed. (A minor point found on the way: `wall_clock_now` maps a pre-epoch clock to 1970 via `map_or(0, …)`.)
- `PagedStack` (contract G11, `docs/design/purrdf-backend-contract.md:569+`; `crates/rdf-core/src/ir/paged/stack.rs`) composes bases, chronological sealed delta generations and a head, and `snapshot()` (`stack.rs:335`) pins the **current** stack. There is no way to read the stack as of an earlier generation. `PageGeneration` (`paged/provider.rs:35`) is a consistency check (`StaleGeneration`), not a selectable point.
- `QueryOptions` (`engine.rs`, around 3790-3843) has no time or position input.
- Range `FILTER`s over annotation objects are evaluated after the scan; there is no ordered `(predicate, object)` index on annotations to range over (see #491).

## Why it matters

Katamari makes "current" a function of coordinates, never of the clock (ADR-0038 38.6, "no implicit now"): a view is evaluated at a **valid time** (an instant in the world the claims describe) and a **knowledge position** (which commits the evaluation may see), and every answer records the coordinates it was computed at, so "current" always means "current at these coordinates". Two consequences for purrdf:

- A query or rule containing `NOW()` must be reproducible. Katamari will pass the instant it means; the answer has to be computed at that instant, and re-running it must give the same result.
- "Known as of commit K" is a prefix of an append-ordered history. purrdf's layered stack already orders its generations chronologically; reading a prefix of it is the same idea with no extra storage.

The scale review behind this (Katamari `docs/design/QUERY-AT-SCALE.md`) found that version chains degrade (OSTRICH became unusable around version 1,100) and that what works is time as **columns**: a monotone position, so "as of K" is a prefix, and valid time filtered by zone maps first, with interval indexes only where measured. For a mail vault of about 300 million claims with valid-time annotations on each, "valid at t" as a post-scan filter touches every claim.

## Proposed direction (a sketch)

1. `QueryOptions::with_now(XsdDateTime)` and `with_entropy_seed(u64)` as public options (the test-only setters already exist); the evaluation identity records them when set. The same for rule evaluation that reads time.
2. `PagedStack::snapshot_at(generation_count)` (or by a generation id): a snapshot over the bases and the first *k* sealed generations, with their removals, excluding later layers and the head. Same G9 binding and G8 evidence as `snapshot()`.
3. With an ordered annotation index (#491), compile `FILTER(?v >= a && ?v < b)` on an annotation object bound to a known predicate into a range scan; for paged and segmented data, test the range against page/block min-max summaries before admission (multi-range summaries where one outlier would widen a single min-max).

## Acceptance criteria

- A query with `NOW()` run twice with the same `with_now` value returns identical results, and differs when the value differs; without the option, behaviour is unchanged.
- For a stack with generations G1..G5, `snapshot_at(3)` answers every query exactly as an eager seal of G1..G3 (with their removals) would.
- A valid-time range query over 10^7 reified claims reads only the pages whose summaries overlap the range (G8 evidence), with results identical to the post-filter plan.

## Related

#457 (layered stack), #491 (annotation indexes), #484 (statistics), #22 (`NOW()`).


## Comments (0)

