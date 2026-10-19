# Current coordinate acceptance

The first historical-prefix unit is implemented. All 26 tests in the native
paged_stack target pass, including the three new prefix/sealing/ownership cases.
Strict core all-target compilation passes with warnings denied, and the core
library compiles for wasm32-unknown-unknown. Logs: tasks/historical-prefix-native.log,
historical-prefix-strict.log and historical-prefix-wasm-check.log. The WASM check
is compilation only, not a portable runtime claim. Normal hooks and HTTPS
publication pass at 2f3d8a0fc (tasks/historical-prefix-commit.log and
historical-prefix-push.log). The complete issue remains unfinished; no PR exists.

The existing snapshot construction now accepts a sealed-layer count and checks,
owns and translates only that prefix. Public snapshot_at excludes the mutable
head and later sources. A new non-exhaustive PagedStackSnapshotError retains
requested/available positions and original construction causes, preserving the
existing exhaustive error enum. Five files are changed, including public
re-exports and three tests at the existing paged_stack test home. A sixth file
documents the historical coordinate contract. Initial bases are always retained;
the selected position counts appended sealed deltas, matching the issue's
base-plus-generation direction. Zero selects the base snapshot.

Written controls compare all prefixes of five generations with eager seals,
cover zero-delta/removal-only/reinserted states, pending head/graph exclusion, exact
selected descriptors and qualified page evidence. A second control retains
multiple initial bases, advances positions through actual removal-only and
insertion head seals, and checks retained old snapshots. The third changes
an excluded provider generation, checks that it is neither consulted nor owned
by the prefix, then verifies selected-source drift remains a sticky refusal.
These controls execute in the passing native target. Runtime source is reviewed
in tasks/historical-prefix-owner-review.md; it is an owner assessment, not
independent approval.

Caller time/entropy, evaluation identity and real annotation range pushdown,
including the ten-million-claim evidence control, remain unimplemented. No
partial issue completion, PR or closure is claimed. Plan/prior-art presence
gates pass with clearly owner-authored assessments, not independent reviews.
