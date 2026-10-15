# #492 public SRL triple seam — source review, 2026-10-13

**Current source verdict: PASS for this seam after the correction below; runtime and full #492 acceptance unproven.** The initial inspection found one quad error-order regression, which the owner corrected in the same source slice. This is a read-only review of Shapes source and registered tests. No tests or build were run for this review.

## Material finding

`srl/eval.rs::evaluate_with` now executes `data.core_arc()?` and `rules::base_triples` before entering `evaluate_shape::<4, RdfDataset>`. In the published quad implementation, the same call occurred **after** element-rule well-formedness, `empty_data`/NOW preparation, stratification or declared scheduling, and `compile_scheduled`. Each preceding operation can return an error. A failing data projection can therefore replace the prior rule/program error when both conditions exist; this is an observable first-error change to the existing quad API. Preserve the old ordering in the common evaluation path, with quad base acquisition at its original point. The triple entry's `TripleRead::checked_read` ingress and sticky first cause should remain before its own work. This finding has been sent to the source owner.

## Source paths that are coherent

- `srl::infer_triples(document, &TripleRead, options)` is a real public entry. `checked_read` observes the original read cause before and after the closure, and `SrlError::TripleSource` retains its type. `RuleSetDocument::rule_set` emits only Elements, no Producer and no expected predicates; `RuleSource::Triple` cannot enter the quad-only SHACL producer view in valid public use. Filter and Assign still use the original empty SPARQL expression context.
- `evaluate_shape::<3/4>` is one schedule, Datalog and guard body. `RuleShape<3>` uses Base authority and NoOutput disposition outside the three term coordinates. `RuleShape<4>` still spells the original internal base and empty-head fourth-slot markers, default graph surface, and output filtering, so the quad program identity is not intentionally changed by this seam.
- The triple entry drains asserted, reifier and annotation projections with explicit roles into `TripleDatasetBuilder`; it does not create a quad result. The builder's `push_reifier_with_memory` ensures the implicit `rdf:reifies` predicate, so `TripleRead::reifier_quads` can represent those rows. `Inference<TripleDataset>` and its inferred-only dataset publish into the compact builder. The existing classifier and term fold are shared; scoped blanks and nested quoted triples use the original bottom-up conversion.
- `Inference<D>` keeps the prior default `D = RdfDataset`; its manual Clone shares the `Arc<D>` and does not require `D: Clone`. The existing public SRL test function names are all retained in the testkit registry; two new controls cover compact recursion/Base/NoOutput, typed poisoned read and nonempty scoped statement roles. The rules-engine empty-CONSTRUCT test covers legal no-output versus a neighboring rule.

## Evidence still required

The live strict check is not a runtime result. Execute the registered SRL and rules-engine controls, prove legacy quad identity/first-error behavior after the ordering correction, and complete the carrier's physical/host/portable and full issue acceptance. This review does not qualify physical 12/8/12 bytes by execution, the distinct triple contract hash, or the final issue delivery.

## Narrow correction readback

The source owner subsequently moved quad `core_arc`/base extraction into the deferred `PreparedRuleSource` closure. The shared evaluator invokes it immediately after `compile_scheduled`, restoring the published quad first-error order. The final publication loop is now shared in `rules::publish_inference`. **The source finding above is resolved on current readback; runtime and full acceptance remain unproven by this review.**

## Directed public runtime readback

`tasks/public-triple-rl-runtime.log` is terminal 0 with 4/4 selected `srl_language` tests passing: both new public compact-carrier controls, the existing triple-term dependency control and the existing nested-term-limit control. The new controls execute public `infer_triples` over recursive/Base/NoOutput and scoped statement-role inputs and assert physical 12/8/12 row bytes on their recorded cases. This qualifies those selected behaviors on that source, not the full SRL suite, complete #492 contract, portable/host execution or delivery.

## Existing-fixture compact parity extension (source only)

The current test-only `srl_language.rs::infer` retains its original `parse_and_check` and quad `srl::infer` error propagation. After a successful original inference, it moves the single fixture source into `TripleDataset`, calls the public `infer_triples` entry on a fresh read, and compares inferred-only **flat canonical bytes** through the same `try_canonicalize_flat_view` home. It then returns the original quad `inferred()` rows for the existing assertions. Thus successful original fixtures gain a compact parity check without a copied evaluator or changed original assertion body; invalid-rule errors still return through the original `?` before compact conversion. The fixture's `Arc::try_unwrap` assertion is a test ownership precondition, not a production fallback. Scoped source verdict: PASS; the extended fixtures have not yet executed in this review, and corpus/portable/full issue acceptance remains open.

Owner batch 17768 later terminated 0 with the registered `srl_language` suite 25/25 PASS, including the successful-fixture compact parity extension and the existing assertions. Its checker seed/strict and affected core/SRL strict also passed. This discharges that narrow runtime-pending sentence; portable, full #492 gate and integration remain open.
