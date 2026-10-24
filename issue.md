# Issue #694: Verify and preserve original-model identity in SHACL conformance memoization

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

The current conformance memo in `crates/shapes/src/constraints.rs` keys answers by interned focus node, re-entry depth and shape IRI. Its comment assumes that equal IRIs imply the same shape body. The current `ModelOwner`/exported-shape API preserves distinct original model domains, including declarations with equal IRIs. Source inspection exposes a possible cross-domain cache collision; an actual public-entry reproducer is required before claiming a runtime defect.

This is a bounded native correctness delivery under #569 and #690. Their complete recursive mode/solver and host contracts remain open.

Acceptance:

- Reproduce or rule out the collision using actual parsed/exported shapes from two original owners with the same declaration IRI and different conformance. Compare individual results with combined logical constraints, both encounter orders, interned focus nodes and complete report observations. Preserve owner lifetime after the factories are dropped.
- If reproduced, key the one production conformance memo by the original definition/model identity relevant to evaluation. Never substitute an IRI-only identity or serialized identity allocation per lookup. Preserve re-entry-depth behavior and foreign-focus recomputation semantics.
- Preserve useful sharing when repeated edges reach the same original definition. Include named and blank-node declarations where reachable; preserve actual reader/allocation controls and original ceilings.
- Reuse the existing evaluator, plan and model owners. No second evaluator, semantic Cargo feature or wire-format change.
- Run focused native regressions, relevant existing conformance/report/allocation controls, strict real-caller compilation, normal hooks and hosted review/checks. General semantic tests stay native under repository policy.
- Land a complete verified fix through ghprsq and clean up its owned checkout. If the public scenario cannot reach the suspected collision, document the exact invariant and evidence instead of manufacturing a fix.

Target: at most four hours of individual issue work; split any separately discovered complete behavior contract while retaining every parent requirement.


## Comments (0)

