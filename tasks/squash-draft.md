Restore evaluator scopes after answer callback unwind

Closes #545

An answer callback could unwind after the evaluator installed local caps,
node cursors and evidence scopes. The entered boundary now stays inside the
existing resident guard through callback execution and completion. Unwind calls
the original abort implementation; normal completion disarms the guard only
after `finish` returns. The abort state is consumed before later evidence work
can fail, preserving the parent scope and original source-error precedence.

The existing copied-join fixture is shared by both tests. There is no additional
heap allocation, second guard destructor, new dependency or numeric-policy change.

Validation includes the actual callback panic/error and healthy-context reuse,
existing provider and nested-scope controls, native/scalar/SIMD physical-stack
execution, exact 40-byte ordinary rows, admission/refusal/last-owner release,
all 19 physical-shape controls, strict Clippy and the unchanged helper census.
The final local `make check` passed, including its nested release-WASM build.
The same-source complete hosted run also passed portable execution, library,
Python/conformance and optimized package acceptance. The original producer
passed 112 sites on seven assembly configurations; its document matches.

The current PR run has passed its package, cross-architecture, portable,
documentation, conformance and all seven assembly jobs. Library completion
remains pending, so these notes are a draft and do not authorize merge.

The current independent source review found no actionable code issue. Its
documentation percentage suggestion was declined publicly because this changes
no public API and the warning-denied repository documentation gates pass.
The owner assessed the complete remaining contract and actual receipts under
the zero-subagent workflow; no fresh independent completion audit is invented.

The clean predicted integration uses the tested main base without an additional
base delta. No conflicts were resolved. Historical receipt reuse is scoped in
the current acceptance index; no old assembly identity or count is relabeled.
The DISTINCT fixed-work estimate remains uncertain and establishes no win.
Cancelled timing campaigns remain excluded.

The authoritative plan is `.stage/complete-evaluator-acceptance/plan.md`.
Select `.stage/complete-evaluator-acceptance` for separate evidence archival.

Defect-Class: none
