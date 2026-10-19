# T16 independent review (Addendum A: G-04, G-06, G-08)

Scope: uncommitted changes since c694b0bf4 in crates/shapes (src/plan.rs, src/constraints.rs,
src/shapes.rs, src/shapes/parser/annotations.rs, src/product/mod.rs STAGE_ID,
tests/fixtures/prepared-shapes-core.product, tests/conformance_memo.rs, new
tests/deactivation_closure.rs) against plan Addendum A (A-G3, Task 16, A-R4, A-R6, A-R8),
gap-analysis-stage3 G-04/G-06/G-08 and plan-review-addendum-a BLOCKING-1. No source modified.

## Commands run
- `cargo test --locked -p purrdf-shapes --no-fail-fast`: 91 binaries, 2195 passed, 0 failed
  (includes deactivation_closure, conformance_memo, product_determinism, closed_property_shape,
  shape_deactivation_expression).
- `cargo clippy --locked -p purrdf-shapes --all-targets -- -D warnings`: clean.
- `python3 scripts/check-shared-helpers.py`: OK.
- Mutation (scratch copy in /tmp, removed afterwards): `ShapePlan::exhaustive()` forced to false.
  conformance_memo `an_or_stops_at_...` fails (0 vs 1 walks); deactivation_closure
  `a_deactivation_expression_fails_the_validation_whatever_the_member_order` fails first at
  `or [A,B] at shape` (validation succeeded where it must fail). The neighbour, closed and
  refusal tests pass under the mutant, as they should.

## Evidence by check
1. Closure flag (plan.rs). `deactivates` is the growth of `walk.deactivations` across
   `lower_shape`, which counts shape `deactivated_when`, shape constraint-annotation
   expressions, property `deactivated_when` and property constraint-annotation expressions
   (property reifiers are the `sh:property` edge annotations merged into the property).
   Every edge in A-G3 is lowered by a nested `lower_shape`/`lower_property` call inside the
   delta: Not, And/Or/Xone, Node, QualifiedValueShape (shape and siblings), MemberShape,
   SomeValue, nested properties, property `reifier_shapes`, expression-borne Filter/FindFirst/
   MatchAll/NodesMatching/ConformsToShape(Named). References resolved at evaluation
   (`index_position`: nodeByExpression and computed conformsToShape; `CustomCall` bodies)
   bump `opaque_references` and fall back to the graph-wide `any_deactivation` (total
   count > 0). Soundness: a body or index lowered once is counted in the first caller's delta
   and in the graph total, and later callers take the graph flag. Cycle safety: the walk is
   over a finite `Shape` tree (the parser replaces an in-flight cycle by an empty stub,
   shapes.rs:2744, which evaluates nothing, so the flag matches what is evaluated); body
   re-entry is guarded by `in_flight`. Property constraints run under the parent shape plan,
   whose closure includes the property, so the flag covers them.
2. Short-circuits. Made exhaustive inside the flag: AnyViolation (records, `Flow::Continue`),
   `sh:and`, `sh:or`, MemberShape, qualified siblings. `sh:xone` already asked every member;
   `sh:not`/`sh:node` have no member loop. `sh:someValue` still breaks at the first
   conforming value node, but a failure met earlier is discarded by that very conformance, so
   the outcome is order-independent (sound). Outside the flag every arm is unchanged
   (`exhaustive == false` reproduces the old `break`/`Flow::Stop` exactly).
   18/20 claim: shape x or/and caught by member order; node/property/reifier/edge caught by
   AnyViolation continue. The two uncaught (not x shape, xone x shape) are genuinely
   order-independent: `walk_shape` evaluates the shape's own `sh:deactivated` before any
   constraint, `not` has one member and `xone` asks every member. Mutation run confirms the
   others are caught (the test aborts at the first uncaught cell, so the per-cell count is not
   visible from the test itself).
   conformance_memo neighbour: counts the second member's constraint runs, 1 first, 0 plain/
   constant false/constant true, 1 with the expression arm. A real observation, and it fails
   under the mutant.
3. Order tests: positions shape/node/property/reifier/edge x or/and/xone/not, both orders,
   compared with the direct `sh:targetNode`-style validation of the member (failure text
   contains `sh:deactivated`), plus a well-formed focus node and constant neighbours. Adequate.
4. G-06. `annotate_property_edge` now returns the property with `deactivated = true` instead
   of dropping it; `sh:closed`'s permitted set is built from all `property_shapes` paths.
   Consumers checked: constraints.rs:1654 (validation skips), target_eval.rs:236
   (narrowing skips), schema_surface.rs:3237/3274, json_schema.rs (3880/3900 keeps the key as
   permitted, 4426/4457/4652/4747/6964/7028), rules.rs:1757, product/ast.rs (flag encoded;
   STAGE_ID moved), footprint/plan (lowered like an own-deactivated property: sound
   over-approximation). extension_usage.rs, function_resolution.rs and shapes/link.rs walk
   property shapes regardless of `deactivated`, as they already did for a property's own
   `sh:deactivated true`; they now also see the deactivated edge's target, which is the
   same syntactic census (no defect). The copy is per edge, so an unannotated reach of the
   same shape is unaffected. Tests: constant, always-true expression and own
   `sh:deactivated true` give equal closed results on node and property shapes, with an
   active control and a `false` reifier neighbour.
5. G-08. `valued_reifiers` counts reifiers stating `sh:deactivated`; more than one with any
   expression value is refused ("at most one reifier"). An identical expression node on two
   reifiers counts as two reifiers and is refused by the code, as the spec text ("at most one
   reifier with a value") requires. Neighbours: single reifier (expression or `false`),
   agreeing booleans still load, disagreeing booleans keep their own refusal, a second
   reifier stating only another annotation is accepted.
6. STAGE_ID/fixture. The fixture contains the new STAGE_ID bytes, 4744 -> 4744 so GOLDEN_LEN
   is unchanged, which is consistent (a digest swap, same length). product_determinism and the
   census `stage_id_matches_shipped_constant` pass.
7. Performance. For graphs without deactivation expressions `exhaustive()` is two bool loads,
   false; it is read once per probe and once per combinator constraint evaluation. No
   allocation, no new walk, no extra memo work; `LoweredShape` grows by two bools; lowering
   adds an O(annotations) sum per shape, once per bind.

## Findings
No BLOCKING findings.

ADVISORY-1. `shnex:findFirst` (expression.rs:2121) and `shnex:matchAll` (:2133) still return at
the first conforming/non-conforming input node, so a deactivation expression in their filter
shape on a later input node is skipped. Their shape is inside the flag but these loops do not
consult it. A-G3 does not list these two edges and the input is a sequence, so this is not a
member-order defect in the sense G-04 names; the doc on `exhaustive()` ("exactly as a full
validation walks it") slightly overstates. Record or follow up; not required by A-R4.

ADVISORY-2. The `(expr1, expr1)` refusal case uses two textual `[ ... ]` expressions, which
are two distinct blank nodes, so the "identical node on two reifiers" reading is covered by
code reading only. A shared labelled node or IRI-named expression in the test would pin it.

ADVISORY-3. `a_deactivation_expression_fails_the_validation_whatever_the_member_order` aborts at
the first failing cell, so a regression reports one cell, not all; acceptable but the 18/20
figure cannot be reproduced from the test output.

## VERDICT: PASS
