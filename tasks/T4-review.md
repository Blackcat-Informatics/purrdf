# T4 independent review: sh:closed on property shapes

## Scope
Uncommitted diff over 215b8bf45: `crates/shapes/src/constraints.rs` (`eval_closed` now takes
`source_roles` and uses `source.id` as sourceShape; the `PlannedConstraint::Closed` arm of
`eval_constraint` evaluates it per value node when `path.is_some()`),
`crates/shapes/tests/closed_property_shape.rs` (new, 5 tests), pin 374->375 in
`product_corpus_equivalence.rs`, `docs/CONFORMANCE.md` 578->579. No source modified by this review.

## Evidence
- Core 1.2 §7.9.1: "each value node has values only for those properties ... enumerated via the
  property shapes specified for the shape via sh:property"; "a validation result for each triple that has
  a value node as its subject and a predicate that is not in P"; P = sh:property/sh:path "from the current
  shape". Nothing restricts it to node shapes; sh:closed is a shape parameter. For a node shape the value
  node is the focus node; for a property shape it is each node the path reaches. Reading is correct, and
  the approved test core/property/closed-001 requires it. "Current shape" = the shape carrying sh:closed,
  so P comes from that property shape's own sh:property/sh:path; lowering does this, and the test with a
  nested `sh:property [ sh:path ex:zip ]` conforms.
- 2017 REC §4.8 (ClosedConstraintComponent): identical "value node as subject" and "current shape ...
  property shapes declared via sh:property at the current shape" wording; no node-shape restriction. So the
  REC also applies closed to property-shape value nodes. No profile gating exists for this arm
  (`report_profile` only selects severity/message precedence).
- Result shape: focus = the property shape's focus node (Stamped sink re-homes it; test shows ex:Bob),
  resultPath = offending predicate (Stamped::apply keeps a pre-set path, the spec's closed exception),
  value = object, sourceShape = property shape id (`source.id`), component = ClosedConstraintComponent.
  Verified by tests.
- sh:ignoredProperties and sh:ByTypes on property shapes: both work (permitted set and by_types go through
  the same lowering; ByTypes collects via the value node's rdf:type; `true` reports rdf:type, ByTypes
  does not). Covered by tests.
- Node shapes: unchanged path (`walk_shape` still calls `eval_closed`, with `&shape.box_roles` and
  `source.id` equal to `shape.id`); the new arm is gated on `path.is_some()`, so no double evaluation.
  w3c_conformance (2017 core/node/closed-001/002 included) passes, 2 passed.
- Runs: w3c12_conformance: `passed 579, non-canonical-expected-decimal 6, xfailed 0`, failing only the four
  rules-entailment-ruleset-* and stratification test_13/14 (the test binary exits red as expected).
  closed_property_shape: 5 passed. `cargo test -p purrdf-shapes --no-fail-fast`: every suite ok except the
  two w3c12_conformance tests above (same known six). `cargo clippy -p purrdf-shapes --all-targets`: clean.
  No closed-on-property-shape case exists in corpora/community or crates/shapes/corpus.

## Findings (non-blocking)
1. Behaviour change for existing users: shapes graphs that put sh:closed on a property shape (any
   profile, including 2017) previously had it silently ignored and now report violations. This is what the
   REC text requires; note it in release notes/changelog.
2. Box roles: the closed arm passes `&[]` as source roles and relies on `PropertyStamp::apply`, which
   overwrites the roles with the property shape's merged source roles and the property PATH's path roles.
   So path_box_roles describes the property's path (e.g. ex:address), not the offending predicate (ex:zip)
   that the node-shape closed result uses. Source roles are right; this only affects the box-role
   annotation of path roles, is untested, and has no spec dimension. The doc comment on eval_closed
   ("caller's stamp supplies its own focus node and roles") is accurate; the `&[]` is harmless.
3. Test gap: no test pins the 2017 profile or box roles of a property-shape closed result. Optional.

## Verdict
The change matches both specifications, is minimal, and has no regression in node-shape closed.

VERDICT: PASS
