<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Segmented query ownership review

Read-only production review of dirty `segmented/query.rs`, the original session
budget/reservation home and `term_walk.rs` at HEAD
`dd79ef77e94212314adc9db5af42fdd72ec18222`. This is not full stack qualification.

The query identity code retains source-qualified representatives and aliases,
materializes only touched values through `term_value_with_memory`, and keeps
comparison copies alive through equality. Its admitted hashing reuses the
original identity encoding; successful traversal scratch is refunded at the
shared walk home. Public imports and point reads latch failures in the original
operation. Final readiness checks every certified descriptor, including skipped
sources. Current production compile receipt is terminal 0. The initial strict
runtime launcher stopped on four lints, terminal 101; it proves no runtime pass.

Material finding: escaped `SegmentedStackTerm` contains a `QueryStorage` whose
reservation retains `Arc<Budget>` but not the operation's original base owner.
After query and session destruction, the base reservation can be released while
that budget allocation survives through the returned term. Existing immutable
term guards and evidence retain `_base` for precisely that lifetime. Retain the
original base handle in the escaped query storage, without a second admission or
whole-dictionary retention. Verify live charge conservation while the term
outlives the query, then final release; allocator final-zero alone does not prove
the intermediate charge.

## Current checkpoint: 2026-10-12 04:46 MDT

The escaped touched-value finding is resolved in the reviewed source slice.
`QueryStorage` retains `operation.base_owner()` alongside its original payload
reservation. The returned value drops before its storage, and the payload grant
drops before the base pin. No extra base admission or historical dictionary
retention was introduced.

Root read `recovery-segmented-query-owner-final-runtime.log`, terminal 0:
seven integration controls and the escaped-base unit pass. The latter checks
live charge equals base plus payload after query destruction, then verifies
the final base owner is released after the returned value drops. The integration
controls include actual 1,000-source query-session opening, unchanged shallow
snapshot allocation, scoped value identities, allocation-free memo hits,
admitted/refunded identity spills, untouched descriptor drift and resident-head
refusal with healthy retry. Strict production Clippy passed in the preceding
owner receipt; failed earlier fixtures remain failed historical receipts.

Resident publications and provider backing remain caller-owned. Copies of
touched resident values and every query-created buffer use the shared operation
admission. This is not a certificate that all caller-owned backing is charged
as query-created storage.

Adjacent material finding confirmed while integrating evaluator workspace:
`SegmentedSession::reserve_owned_workspace` returns a boxed reservation retaining
Budget without the base owner. The same live-charge defect can survive session
destruction through that returned workspace. The implementation owner is fixing
the shared reservation home with a base pin for returned workspaces and no pin
in the base reservation itself. Its source and conservation test remain to be
reviewed and run; this passing query-value checkpoint does not discharge it.

Verdict: scoped touched-value ownership checkpoint passes. Actual effective-set
DatasetView/FallibleDatasetView, chronological classification, escaped evaluator
workspace ownership and persisted purge remain unfinished.

## Shared workspace lifetime correction

Root read the final shared `SegmentedReservation` home and
`recovery-segmented-shared-workspace-drop-runtime.log`, terminal 0. Returned
concrete/boxed workspace reservations hold `base: Some(original Arc)`;
base and cache-block reservations hold None, and the cache association remains
weak. Drop refunds payload/control charge before releasing that pin. QueryStorage
uses this same reservation home instead of holding a redundant separate base Arc.
The box control layout follows the changed concrete reservation size.

Both escaped-owner units pass with this final layout, alongside 13 existing
segmented tests and seven stack/query integration controls. The direct workspace
test destroys the session, checks base plus both payloads plus box charge,
resizes both payloads to zero while retaining box charge, then proves final base
release. Root found no owner cycle or uncharged Budget lifetime in these reviewed
paths. The preceding scoped pass is superseded by this current-layout replay.

Verdict: both reviewed lifetime defects are resolved for this source slice.
Chronological effective-set DatasetView/FallibleDatasetView and persisted purge
are still unfinished; this verdict is not full issue acceptance.
