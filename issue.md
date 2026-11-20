# Issue #1343: Take the content identity off the query path

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: performance

## Body

## Problem

The first query on a dataset computes a content identity over every quad before returning a result. On a large frozen dataset that hash is most of the cold first run: a query that reads a few rows pays for hashing all of them. A caller that never asks for a receipt or an identity still pays, and "the first user pays for later users" does not hold when there is no later user.

The cost sits on the query path because the snapshot identity is a required method of the dataset view and a field of the public query coordinates, so every lane that keeps the coordinates must produce it. An earlier change could only skip the hash on lanes that discard the coordinates; the governed prepared lane keeps them and still hashes.

## Required behaviour

1. Evaluating a query never computes a content identity unless the caller asks for one. No lane hashes the dataset as a side effect of producing results.
2. Content identity is a separate capability of a view, not a required method of every dataset view. Views that cannot provide one do not pretend to.
3. The execution identity of a query run is a function the caller applies when it wants a receipt, taking the identity as an explicit input. A receipt cannot be produced without one and that is a typed error, not a silent hash.
4. The value of the content identity for a given dataset does not change in this work; only when it is computed.
5. Views that compose others (composite, delta, paged, segmented) compute their identity only on request.
6. Callers that key cached or prepared state by content identity move to a key that does not require hashing the dataset.

## Acceptance

- Retired instructions for the reporter-shape first query on a frozen dataset, cold, with no receipt requested, contain no content-hash work; measured with the existing frozen evaluation benches against the old reference commit and reported as a table, cold and warm.
- A caller that requests a receipt gets the same identity value as before.
- Requesting a receipt from a view without the capability is a typed refusal; a neighbouring view with the capability succeeds.
- No time measurements; instructions and allocations only.


## Comments (1)

### paudley — 2026-11-20T02:16:17Z

## Verification of the landed receipt migration on main `d2be07c47`

Read against the four items the issue asks the migration guide to state, and the release-candidate evidence. Verified by reading the code and the text on main; no test was run for this comment.

**Stated and present**

- The replacement for `DatasetView::snapshot_identity`: `ContentIdentified::content_identity` (`crates/rdf-core/src/dataset_view.rs:1891`). The changelog states the migration path, including that a view which cannot certify its content returns `Err`, not `Ok(None)`.
- The explicit `ContentIdentified` capability: the trait is separate from `DatasetView`. Views that never provide it do not implement it.
- The new receipt fields and versions: `run_identity` (`execution-run/v1`), `execution_identity` (`execution-coordinates/v2`) and the rule form (`rule-execution/v2`) are in the changelog. The migration guide's receipts row names the moved values.
- Refusal: `ReceiptError::NotStarted` and `RuleReceiptError` are returned from the receipt paths (`crates/cli/src/query.rs`, `crates/shapes/src/sparql/coordinates.rs`).
- Tests present: `crates/sparql-eval/tests/query_coordinates.rs` (replay and change of the execution identity), `crates/shapes/tests/rule_coordinates.rs`, `crates/rdf-core/tests/content_identity.rs`.

**Not found, so open**

1. **A typed refusal test for a view without the capability.** The issue asks that requesting a receipt from such a view is a typed refusal, with a neighbouring view that has the capability succeeding. I did not find a test that does this. The compile-time absence of the trait is a guarantee, not the typed refusal the issue describes.
2. **Cold and warm evidence on the release candidate.** The changelog records the governed first-query instruction counts at 22,000 quads (`q1` 224.2 to 62.7 million, `q3` 262.3 to 97.1, `q4` 212.1 to 50.3, the two-pattern scan 209.7 to 47.2). Those are from the change, not from the release candidate, and they are not a cold and warm table against the old reference commit as the issue's acceptance asks. This needs a re-run on the release candidate.
3. **The typed receipt refusal on the release candidate.** Item 1 applies here too.

**Recommendation.** Item 1 is a small test and can be added now. Items 2 and 3 are measurements on the release candidate and wait for it. The 1343 closing comment should wait until those three are in.


