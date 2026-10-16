# Chronological stack statistics owner review

Owner review, not independent approval. Whole issue acceptance is incomplete.

Both actual stack views expose SnapshotStatistics through one common provider
implementation. The segmented adapter translates bound values through the
existing lazy identity owner; the paged adapter uses its existing common-ID
physical metadata. Single uncut sources retain exact counts and NDVs. Layered
counts are physical upper bounds; local NDVs remain unknown rather than being
summed. Ordinary/annotation bounds combine both physical populations because
reifier declarations can change classification. Complete patterns count each
physical stream once. Checked addition leaves overflow unknown.

Review found two important corner cases before publication: a removal-only
paged head has no physical head page, and metadata reads must check original
source descriptors even before an operational checkpoint. Both paths are
corrected and covered by meaningful regressions. Removing the final declaration,
reinsertion, overlapping sources and snapshot isolation retain actual row parity.
Source-local sealed metadata was not forwarded as exact effective-set metadata.

All 37 affected stack controls pass: 23 paged and 14 segmented. They include the
new bound/exactness/provenance checks, original thousand-header-only open,
term-independent shallow historical snapshots, chronological annotation controls
and original refusal/lifetime cases. The shared comparison verifies against
actual view streams and independently asserted fixture answers. Unbound
segmented stream metadata mints no terms, and reading does not change revision.
Strict affected library and two test targets pass with warnings denied; the
initial test-helper visibility and tuple-array lint failures are retained.

Revisions include pinned source order and immutable head/removal metadata.
Resident revisions are computed once on publication without allocating a new
term carrier, so historical snapshot/open does not rehash dictionaries. No
physical row or term representation changes. Existing capacity proofs retain
unchanged reasons, classes, authorities and sink coordinates except the changed
paged consuming source identity; constructor closure identities are refreshed.
No exemption, checker or toolchain changes are added.

Native helper verification passes. The actual persisted reifier LIMIT/count
regression passes its original 2,000/200,000-claim and block-layout controls,
including touched-value bounds, keyed rows and full-count parity. All eight
public paged-stack consumer cases pass, retaining eager answer bags, source
faults and exact page/admission receipts through actual engine entry points.
Standard wasm32 library compilation passes; portable runtime execution, final
profile and whole-issue conformance/hosted qualification remain required. No
timing evidence or independent approval is claimed.
