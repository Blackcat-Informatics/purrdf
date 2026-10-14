<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Definition completion extraction: independent source review

Reviewed 2026-10-12 against the uncommitted `expr.rs` delta after published
`538eb2f71`. This is bounded source review; the candidate has no compiler,
runtime or optimized-package stack acceptance yet.

The measured raw bench artifact retains a 3952-byte `exists_prepared` frame at
its actual definition child call. The proposed extraction moves only completion
after that child returns. The original collision and strategy decisions, memo
lookup/key construction, witness choice, ledger push, child evaluation and
ledger pop remain in the caller. Error propagation still occurs after the pop.
The helper preserves truncation/barrier precedence, answered and consumed-row
charges, evidence computation/publication and evidence/definition memo insertion
order. Existing admitted clones are unchanged; the extraction introduces no
allocation, clone or heap owner.

Initial parameter order would have reversed destruction of the inner result and
memo key on an early return. The owner corrected it: the caller retains explicit
`let inner = inner_result?`, then passes `memo_key` before `inner`; helper
parameter order preserves inner-before-key destruction. Child failure leaves
the key in its original caller scope. This matters for grant release even where
the boolean result is unchanged.

No material source defect remains in this inspected delta. Effectiveness is
unproven: a prior memoized-probe extraction passed native tests but did not reduce
the optimized recursive stack slope and was reverted. This candidate must pass
original native semantics/evidence/governor controls and actual optimized WASM
stack/floor controls before any performance or completion claim. Raw bench frame
reservations cannot be added together as exact optimized-package stack usage.

## Compiler-driven ownership correction

The first strict check rejects passing `Evaluated` by value as unnecessary.
Current source instead retains `inner` and the earlier mutable memo key in the
caller and passes references. The helper takes the memo key only at the original
successful definition-memo insertion. Inspection confirms early truncation,
charge refusal and evidence failure leave the same owners in the caller with
inner-before-key destruction; child failure still propagates after ledger pop.
No allowance or new allocation was introduced. The initial failed check remains
failed; retry/native/package results must be recorded separately.

Corrected source now passes strict evaluator compilation and the original EXISTS
library84/84, witness transport1/1 and SEP-0007 25/25 controls. The owner is
building a fresh optimized standard-directory WASM package for original stack
slope/floor measurements. Native correctness does not establish reduced stack
usage or package acceptance.

## Optimized measurement: rejected and reverted

The fresh standard-directory optimized package builds/links successfully with
final WASM SHA256 `ac7020d337737b995056d46831b4bf72b5da9cfc51fd105fa367eec9acddec6c`.
Original NOT EXISTS depths2/4/8/16/32 consume44748/58956/87372/144204/257868
stack bytes: the unchanged slope is7104 bytes per level. The original depth
control fails at133 levels. This falsifies the proposed recursive-stack benefit;
passing native semantics does not qualify this candidate.

The owner retained the exact patch and all receipts, then restored `expr.rs`.
Root verified the production diff no longer includes that file. The diagnostic
package is now stale relative to restored production source. No complete stack,
package or issue acceptance follows from this source review.
