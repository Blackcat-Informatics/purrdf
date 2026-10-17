The merged recorded-cancellation branch exposed a concrete production gate failure in GMEOW PR1842, workflow39051285193. Both completed jobs reject the same canonical constraint:

- [Generation A, job117456045300](https://github.com/Blackcat-Informatics/gmeow-ontology/actions/runs/39051285193/job/117456045300)
- [Fixture prefix, job117456045340](https://github.com/Blackcat-Informatics/gmeow-ontology/actions/runs/39051285193/job/117456045340)

The stage-conformance enactment gate reports that `gmeow:FlagshipScenarioFailureIriConstraint` carries `gmeow:enforcesFailureClass` but did not lower into a violation rule: `unsupported-consequent`. Generation A exits through `make check-sync`; fixture-prefix exits through `make produce-producer-independent-test-fixtures`. This is a deterministic semantic gate rejection, not a runner-shutdown diagnosis. Full decoded logs are retained with the current native prerequisite's Stage evidence.

Current canonical source is `slices/grounding/logic/module.ttl`: this `logic:PathNodeKindConstraint` requires every `gmeow:enforcesFailureClass` value on a `gmeow:FlagshipScenario` to be an IRI. The corresponding source invariant in `slices/grounding/logic/tests/structural.ttl` also forbids treating this annotation as a logical object role. `crates/logic-compile/src/frontend.rs` recognizes the node-kind sugar, while relational violation lowering reports `RcViolationGap::UnsupportedConsequent` in `relational_core.rs`.

Repair the canonical compiler/runtime path so the selected node-kind integrity operation produces an actual violation rule with supported term-kind semantics. Do not remove the failure-class declaration, skip the enactment gate, hand-author a shape, or turn annotation metadata into an object property/restriction merely to pass.

Acceptance:

1. Tiny independent source fixtures cover valid IRIs and invalid literal/blank values on the real node-kind constraint frontend, relational violation lowering, and actual violation execution. Retain the original declared node-kind distinctions, missing-path semantics and source identity.
2. Preserve existing comparison/required-single failure metadata and the structural annotation-versus-logical-role invariant; do not weaken source policy or enactment completeness.
3. Run affected managed Make checks and ordinary hooks; no test may regenerate the corpus. Hosted producer/generation and fixture-prefix lanes must qualify the production authored constraint. Local optimized producer/full GMEOW gate remains outside current authority.
4. Inspect complete review feedback, merge through ghprsq, and retain candid pending-hosted-CI status under the existing merge instruction.

Principles4,7,17: one canonical logic source, executable enforcement, generated validation surfaces. This follow-up is required work; it does not close the larger logic or native-engine integration backlog.
