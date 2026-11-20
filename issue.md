# Issue #1398: v5.0: rename the additive multiset accounting check

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

`purrdf_hash::multiset::Multiset256::fingerprint` is a published v4 method. It hashes the population and additive sum, so it cannot identify a row set. #1345 requires removing the fingerprint name from this public accounting surface. The sorted row-set identity is already implemented by #1382.

## Required before 5.0

- Replace the public fingerprint method with an explicitly named accounting check. Preserve the current domain framing and bytes for accounting consumers. Migrate workspace callers and document the replacement and the distinction from `sorted_fingerprint`.
- Preserve the separately documented deprecated `purrdf_text::source_fingerprint` result through 5.0. #1388 owns its removal after 5.0.
- Verify the accounting byte contract and the existing distinct-row-set control. Keep the closed change-feed and text-delta work in #1376 and #1377 out of this change.


## Comments (1)

### paudley — 2026-11-20T02:10:36Z

## Plan

**Surface that breaks.** `purrdf_hash::multiset::Multiset256::fingerprint(&self, domain)` is removed. Its replacement is an explicitly named accounting check, for example `accounting_check`, with the same signature and the same bytes. `sorted_fingerprint` stays as the row-set identity from #1382. Callers inside the workspace move to the new name in one change. Consumers outside the workspace change one method name; the value they get is identical.

**Frozen bytes.** The output does not change. Verification: before the rename, record the current output for a pinned set of inputs as a golden in `crates/hash`, covering the empty set, a single member, repeated members, and two domains. The same golden must pass after the rename. This is the pinned-vector check you asked for.

**Callers.** `grep` finds 121 `.fingerprint(` sites in `crates`. Not all of them are `Multiset256`; I have not yet classified them by type. The first step is the compiler: rename, then let the build list every caller that breaks, so the migration list is exact and not a text guess.

**Migration note.** Name the replacement, state that the bytes are unchanged, and state the difference from `sorted_fingerprint` (additive accounting versus sorted row-set identity).

**Not touched.** `purrdf_text::source_fingerprint` (#1388) and the change-feed work (#1376, #1377).

**Lane.** Touches `crates/hash` and the callers in `rdf-core`, `text` and `shapes`. Independent of #1397 and #1399.

**Estimated size.** Small: a rename plus a golden. The caller count is the uncertain part.


