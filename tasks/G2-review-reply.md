Addressed in bb1437bb0, committed with normal hooks and pushed.

Partial rounds now discover productive New factor matches first, through shared
binary/cyclic last-new-atom decomposition. They acquire OldOnly/Full contexts
only when productive anchors require them, preserving first-new-factor
projection, authored proofs and every partial-delta combination. The suggested
all-delta-only shortcut would lose these cases and was not adopted.

Actual current qualification passes346 Datalog tests,793 entailment tests,
59 SHACL and14 CLI cases, five actual WASM rule cases, native shared artifacts,
ten Python cases and three public Node tests, strict clippy and hygiene.
Exhaustive partial/cyclic/negative/saturated-frontier controls and workers1/4/32
include exact, zero and one-below typed governor boundaries.

The real default-limit recursive CLI campaign uses identical inputs and unchanged
governors: at n256 the immutable pre-fix binary refuses its1,048,576 join-step
ceiling; the correction completes32,385 inferred facts and every authored proof.
Independent completion review passes. Current-head hosted CI remains pending.

The PR description also clarifies the scope warning: additive BLAKE3 APIs belong
to the assessed main/base delta. Their qualification establishes the integration
interaction; this PR does not introduce those hash APIs. No scope expansion is
needed to address the rule-engine finding.
