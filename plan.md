# Shared immutable ownership storage

The actual main CI failure is an isomorphic owned/borrowed dereference in prepared proof contexts and retrieval profiles. The layer tool selects purrdf-core as their closest existing common home. Preserve inline ownership, zero-copy borrowing, non-Clone payloads, immutable lifetimes and exact drop behavior. No helper exemption, gate weakening, boxing, dependency, feature or source filtering is permitted. The larger causal-support branch stays separate.

## Task 1: Centralize both actual callers

Add one generic OwnedOrBorrowed in the core ownership module, with a shared immutable Deref and no Clone/ToOwned requirement. Replace both private enums and their impls, including private proof test patterns. Register the one enforced helper home. Use synthetic non-Clone/drop/pointer controls and actual prepared-context and retrieval-profile tests. Reject std Cow because it requires ToOwned; no boxed workaround is needed. Local reviews are self-reviews because no independent dispatcher exists.

## Task 2: Qualify the real failed gate

Run strict affected all-target Clippy, the full helpers-hygiene target, focused core ownership tests, actual proof ownership/checker cases and actual retrieval profile behavior. Required normal commit/push hooks run fully. Preserve failures and source fixes. Source-independent proof semantics and the larger incomplete causal graph remain distinct.

## Task 3: Review and integrate

Open a complete issue-linked PR, inspect all review/comment/thread surfaces and current CI, incorporate actionable CodeRabbit feedback where available, assess the actual integration candidate and merge only through ghprsq with selected Stage evidence. Do not infer CI or registry publication from local results. Clean up only this owned worktree after durable archive verification.
