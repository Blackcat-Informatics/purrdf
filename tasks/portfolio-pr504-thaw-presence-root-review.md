# Compiler context transport: root diagnosis

2026-10-08. Source/proof review; implementation qualification remains pending.

Actual continuation 74024 ran three passing and two failing contextual controls.
The independent-Join admission fixture was not a valid correlated consumer;
the corrected Lateral fixture passed in diagnostic session 52044. The actual
compiled VALUES/BIND/bound-only relation still refused with both positions free.
Its compiled AST is retained at
`/opt/purrdf-454-current-qualification-20261008/pr504-bound-diagnostic-1/feedback-compiler-diagnostic.log`.

The driver carries a certainly bound VALUES value through merge and BIND output
08. `Compiler::thaw` then transports that value through
`IF(marker18, value08, outer02)` into slot1b; Apply maps input02 from slot1b.
The marker is the disjunction of BOUND over the selected mapping. The fallback
outer02 may be absent. The existing certainly-bound expression law therefore
cannot prove slot1b present, despite the relational guarantee that a present
selected value implies the marker is true. This is a compiler representation
problem; relaxing generic IF certainty would be unsound.

For a selected variable `actual`, the equivalent transport is
`COALESCE(actual, IF(marker, COALESCE(), fallback))`:

* Present actual implies marker true and returns exactly actual.
* Absent actual with another selected value present leaves the result absent;
  the true-marker guard must prevent fallback.
* No selected value present makes marker false and returns the original fallback,
  including absence when that fallback is absent.

The outer initial-binding COALESCE retains its original precedence. Remembered
scope computes both selected values and the marker from the same filtered map.
The rewrite uses the existing COALESCE certainty law, not a new planner exception.
Absent names cannot gain certainty; retaining the original conditional for them
can avoid a redundant empty-argument evaluation.

Writer owns the compiler-local change, actual bound-only relation execution,
empty/partial/remembered/initial controls, operational-error/governor applicability,
probe removal, and focused checks. Root has admitted this design, not declared it
tested. Prior cursor/SERVICE passes and the unchanged production/Python compile
remain attributable; failed relation execution is not reclassified as success.
Independent final judgment and affected analytical cost assessment remain open.
