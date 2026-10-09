# PR507 ordinal finding: independent adjudication

VERDICT: FINDINGS-OPEN

The sole CodeRabbit finding4226494288 is **valid** on published20e2e637c. This is a focused source adjudication, not a new execution or a repeated whole implementation audit. Root remains sole remediation writer.

`MutableDataset::classify_record` captures the source's optional added ordinal, removes its row, and calls `insert_record_rows(target, ordinal)`. That home already supplies the source ordinal when a conversion genuinely creates its target. If the target exists independently it returns false without changing membership or ordinal. The unconditional following `min(old,current)` therefore changes an independently owned target's insertion order without creating it.

Concrete reachable sequence on an empty mutable base:

1. Ordinary `insert(q)` adds a delta ordinary origin with ordinalN.
2. Typed annotation insertion adds a different annotation first, then the annotation of q with ordinalM>N.
3. Ordinary insertion of the matching rdf:reifies declaration classifies q. The independent q annotation remains caller-owned, but the unconditional min changes its ordinalM toN, moving it before the earlier independent annotation.
4. Removing the declaration restores q's ordinary source with N and retains the independent annotation because classification_created never owned it. Its changed ordinalN remains, so the restored source and independent target now tie in added_ord.

`added_in_order` sorts only by ordinal with sort_unstable_by_key. The tie therefore exposes map iteration as replay ordering; independently of which tie order appears in a particular run, the target has lost its caller-authored order. The failure is not repaired by deterministic hashing or a generic typed-key tie-break: those would conceal the lost independent insertion provenance.

The existing `normalization_does_not_reorder_an_independently_added_target` test starts q in the BASE builder. Its source ordinal is None, so the offending `(Some(old),Some(current))` branch never runs. Its earlier pass and the frozen probe-order golden remain legitimate coverage of that base-origin case; they do not refute this distinct delta-origin defect.

## Smallest coherent correction and failing-first control

Remove only the unconditional min rewrite after insert_record_rows. Retain generated-target ownership, recorded source ordinal, the existing insert_record_rows supplied-ordinal behavior and undo logic. No extra order fallback, classifier, snapshot priming or fixture golden regeneration is required.

Extend the existing owning regression with the empty-base DELTA sequence above. Assert the independent target's original ordinal before classification, while classified and after undo; assert the interleaved annotation value stream remains caller-ordered. After undo, assert both original role membership and uniqueness of live insertion ordinals, and the exact ordinary/annotation snapshot order. These observations exercise real mutable insertion/classification/declaration undo, rather than mirroring the conditional being removed. Keep the base-origin scenario and immutable shared_views probe-order golden unchanged.

The change affects normalization order only; exact role ownership, logical counters, graph cardinality, admission and LOAD remapping should remain unchanged. Existing T1/T3 evidence can be reused for untouched contracts, with affected mutable/order controls and strict/fmt qualification supplied by the writer. The actual failing-first and corrected runtime receipts have not been executed by this reviewer and remain required before closing the finding. PR feedback and current-head integration acceptance must retain this finding OPEN until that correction is demonstrated.

## Postfix fixture-order clarification

Root's failing-first execution T4-ordinal-before.log exits101 at target ordinal0 versus expected2, demonstrating the actual defect. After removing the rewrite, the target ordinal assertion passes. The subsequent `[z,r]` public annotation stream expectation is a fixture error: it is not the native frozen annotation ordering contract.

The empty-base source ordinary r was inserted before both independent annotations. append_delta primes terms from caller_added ordinal order, so r receives its dictionary identity before z even while r's annotation was added later. RdfDatasetBuilder freezes annotations with `annotations.sort_unstable()`, and the dataset annotation cursor documents sorted native term-ID order. DeltaDatasetView exposes that frozen delta annotation cursor. Therefore its public annotation value stream is `[r,z]` even BEFORE classification; `[z,r]` cannot be demanded from typed annotation insertion order alone. The original base-origin fixture differs because the delta builder does not prime the base source r as a caller-added delta record.

Correct the regression by capturing its actual independent annotation value stream before declaration insertion and asserting that exact stream remains unchanged during classification and after undo. Keep the direct target ordinal equality, complete added_in_order equality after undo, original ordinal vector and unique live ordinal assertions. These are non-tautological transition invariants: the original code still fails target ordinal2→0 before any stream expectation, and the corrected test preserves the established immutable sorting/probe contract rather than changing production to fit a new ordering law. Preserve exact role membership checks and the existing base-origin regression/golden. No reviewer execution or source edit was performed for this clarification.
