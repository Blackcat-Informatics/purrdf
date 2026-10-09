# Task1 public metric and probe-order remediation

Current status: PASS on the final owner-provenance replay plus two-role guard source. Final97654 actually exited0 with mutable55, complete shared_views59 and strict/fmt0. The unchanged golden passes. Full settled1373 core1230 plus six integration targets remains applicable; see T1-implementation.md and T1-review.md. The following chronology preserves the genuine earlier failures.

Internal role normalization previously changed the public value metrics: promotion of a base ordinary row added a derived annotation and suppressed its origin, incorrectly reporting two additions and one suppression for one caller-added declaration. The owner now maintains caller-origin role membership separately from physical role masks, with reference counts per quad value. Classification transitions never update public mutation metrics. Removing a derived target removes its caller origins before forgetting conversion provenance. Metric reads allocate nothing.

The new native regression checks base promotion (added/suppressed 1/0), base demotion (0/1), undo (0/0), and removal of a converted delta origin. Existing cross-role restoration controls still pass. Physical count, graph lifetime and snapshot role membership remain independently checked.

The original complete core qualification exited 101 solely on the probe-order golden, after 1228 library controls and five integration targets passed. Its exact actual output differed in four lines: two caller-added predicates swapped, in the delta and composite streams. Owner normalization moved a base annotation into the delta and minted its predicate before the first caller-added row. Native index ordering then changed. The golden remains unchanged.

Snapshot construction now primes only caller-added origin terms that actually have a delta record, in their authored ordinal order, before replaying normalized physical roles. It checks DeltaAdmission after every term. Origins whose derived target exists only in the base are excluded, so priming introduces no unused retained terms. Base-wide materialization and lazy reclassification are not used.

Actual session65517 exited0 with 54 mutable controls passing. Its integration filter selected zero controls, which proves nothing about the golden. Actual session58673 also exited0 with 54 mutable controls passing after the unused-term filter correction; its combined filter again selected zero integration controls. Both original commands and zero-match outputs are retained honestly. Session98496 executed the entire shared_views target without a filter and exited101: 58 controls passed, the unchanged probe-order golden failed. Clippy did not run. Caller priming fixes the late predicate order but changes the converted base row's placement. Explicit normalized base-origin replay ordering is required; selective term priming alone does not satisfy the contract.

Raw evidence is retained under /opt/purrdf-401-qualification/logs: core-qualification.log/.exit, probe-order-actual.txt, metric-order.log/.exit, metric-order-retry.log/.exit and order-strict.log plus per-command exit receipts. These remain external pending the final selected-Stage evidence copy.

Task1 final qualification and independent review remain incomplete; Task2 production transfers and Task3 whole gate/hosted qualification are separate and unrun.
