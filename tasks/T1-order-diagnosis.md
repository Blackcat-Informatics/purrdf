<!-- SPDX-License-Identifier: CC-BY-4.0 -->
# Task1 actual probe-order diagnosis

Status: ACTUAL FAILURE DIAGNOSED; correction/runtime recheck pending. Read-only review, no source edits/builds/forge actions.

Actual evidence: /opt/purrdf-401-qualification/logs/core-qualification.log and .exit (101), probe-order-actual.txt, and unchanged crates/rdf-core/tests/golden/probe-order.txt. The shared_views target reports58 passed/1 failed, specifically delta_composite_and_selection_probe_order_is_frozen. The earlier passed core/blank/CDT/graph/import/declaration controls are not erased by this ordering failure.

## Actual difference

The exact diff changes only two output pairs: the unbound delta/composite rows and the subject-bound late delta/composite rows. Expected late p0 o0 default BEFORE late p1 o1 g1; actual yields late p1 o1 g1 BEFORE late p0 o0 default. The demoted r p1 o1 g0 row itself stays in the expected position. Selection output is unchanged. Thus this failure is not evidence of missing rows or an r-segment relocation, and regenerating the golden would silently accept a real compatibility change.

## Owning computation

order_delta starts from order_source(1,true), removes its first ordinary row and reifier declaration, then inserts late/p0/o0/default, late/p1/o1/g1 and other rows. The old delta publication contains only caller-added ordinary records; its initial late row interns p0/o0 before p1/o1. Demoted base annotations are replayed separately by the old view's explicit base → demoted → delta stream.

Owner normalization now admits the base annotation r/p1/o1/g0 as a generated ordinary delta row before the caller inserts late. append_delta follows its earlier ordinal and interns p1/o1 before the caller's p0/o0. Builder freeze and its index use interned IDs for ordering, so both late rows remain present but their probe order reverses. This is observable through actual production DatasetView probes and propagates into composite views, exactly as the frozen fixture demonstrates.

## Small coherent correction boundary

Keep normalized physical membership/counts and the single classifier in the mutable owner. Preserve caller-added term admission order separately from generated base-role conversions when constructing the frozen delta: a base-derived conversion must not seed caller delta predicate/object IDs earlier than those callers did. Preserve the existing base-conversion replay segment using explicit known origin/target provenance when needed; that is replay metadata, not a second lazy classifier. Exclude any separately replayed target from the generic delta segment so counts/set semantics agree. Reuse the original immutable base handles/table order for base-derived replay rather than a new flattened freezer or broad term-table clone.

The writer must check the entire original golden through the existing target, not just the first late pair, because bound axes choose different index plans. Do not reorder all output globally, regenerate the frozen contract, or revert owner classification merely to recover its incidental term IDs. Separately retain the deterministic total-origin ordering finding: canonical conversion ties and exact ordered-vector undo/permutation neighbors are necessary, since set comparison cannot establish replay order.

## Related public metric finding

Current public added_len/suppressed_len still return physical reference-map lengths. With base Ordinary(q), ordinary insertion of a new declaration adds both the caller declaration and generated Annotation(q), and suppresses base Ordinary(q): new metrics2/1 versus old1/0. Removing a base reifier similarly counts generated demotion as a caller addition. This violates the accepted preservation of legacy value-key churn metrics even though physical effective counts are correct. Keep logical caller-mutation metrics distinct from owned classification effects, maintained without read-time allocating sets. Assert promotion/demotion and selective restoration neighbors against the old public values; semver signature checks alone cannot detect this behavioral change.

Both actual order computation and metric finding were sent to root and sole writer. Final Task1 acceptance remains open until corrected frozen source and actual owning results are independently read.

## Actual retry and proposed origin replay

The later complete shared_views retry98496 also exits101 (58PASS/1golden failure). Its exact probe-order-retry-actual.txt differs from the initial failure: caller-origin term priming fixes the late p0/p1 swap, but generated r/p1/o1 now appears after caller-added delta rows under multiple actual index plans. Thus priming alone is insufficient. Both outputs and original golden remain retained unchanged; filtered runs selecting zero integration controls are not golden acceptance.

Source/proposal assessment: retain a sparse typed set(target RecordKind, original base QuadIds) for generated conversions. Replay Ordinary targets from original base annotation cursor/table order and Annotation targets from original base ordinary indexes, with actual pattern axes/graph filters. This is membership replay of owner-normalized roles, not view-time classification. Exclude exactly those target rows from generic delta streams, preserving same-kind dedup and distinct cross-role membership. Retained base terms provide safe immutable handles and later snapshots remain independent.

Required ownership condition: source must be BASE, target must remain ADDED, and conversion must actually own the generated target (classification_created or equivalent). Merely sourceBASE/targetADDED also selects a conversion colliding with an independent caller-added row: lifting that row into the earlier base segment would change authored ordering despite the conversion never creating it. Restored visible-base targets stay in their native base stream and are not replayed again. A unique typed key set prevents duplicate replay when several origins converge.

Charge the extra sparse set in both DeltaAdmission and frozen view stats before successful publication, using the same formula; actual copied_index_bytes must record it too. Existing caller-origin term priming remains necessary to preserve unaffected caller index order, with admission checked for newly interned terms and no retained-unused-origin payload. Public caller metrics must continue to exclude classification-owned changes and resolve removal back to logical caller origins. Positive/collision/restored-base/undo neighbors plus the full unchanged probe-order golden are the meaningful final checks. No additional campaign or new validator is needed.
