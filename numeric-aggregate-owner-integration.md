<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Numeric aggregate owner integration

Apply the Stage core, caller, cleanup and regression drafts together. These are proposed changes prepared on the read-only support lane; the sole shipping writer integrates them and owns validation. No check, build or test was run here.

- `numeric-aggregate-owner-core-draft.patch`: `NumericFold` and `NumericSummary` retain immutable `ParsedValue` magnitudes. The existing native parser and numeric arithmetic/promotion kernels compute under `NumericFrame`; the exact-merge proof, magnitude-bound proof and source-order IEEE replay remain one algorithm. Duration months/seconds stay raw until final validation, and duration AVG keeps its existing default seconds division and month rounding law.
- The same core exposes `expr::xsd_workspace_value(&XsdValue,&WorkspaceCapability)->Result<WorkspaceTerm,EvalError>`, sharing the actual canonical numeric/nonnumeric writers with expressions. Final native text destination sizes are checked and charged before allocation; results keep their original grants. Only the explicitly resident `fold_values` boundary and resident association-oracle fixture extract raw terms.
- Numeric tails use `AdmittedVec<Option<ParsedValue>>`. The existing parallel fold body gains `par_chunk_reduce_init_admitted`: it checks `Layout::array::<Result<S,EvalError>>` for the actual indexed chunk count, charges before fallible allocation and writes into that capacity with indexed `collect_into_vec`. The array grant outlives its iterator/reduction on success and every error. The resident wrapper calls the same body. Other bounded callers must select this admitted entry when they use it.
- `numeric-aggregate-owner-callers-draft.patch`: only contextual Numeric step/finish and ordinary SUM/AVG arms change. The writer's settled `AggregateFinished::Admitted` publication routes the resulting `WorkspaceTerm` into `intern_workspace_term`; no original result grant is discarded before interning.
- `numeric-aggregate-owner-cleanup-draft.patch`: deletes the now-unused raw canonical result helper; its remaining typed-literal fixture helper is test-only.
- `numeric-aggregate-owner-regressions-draft.patch`: actual operational prepared queries check frozen large exact answers, result-only clone/extraction lifetime, actual allocation peaks against storage/workspace evidence, IEEE chain and raw-duration correctness, the exact non-terminating AVG error code, calibrated capacity refusal and a sufficient-capacity rerun. The large refusal fixture chooses its lexical length from the measured small run's ceiling so each final payload alone must exceed that ceiling; this is a test input, not an implementation bound. Existing exact differential/property and parallel association-oracle coverage remains required.

Physical refusal propagates as `EvalError`; it is never converted to poisoned SUM/AVG, an unbound value, an exact-merge fallback/replay, or a successful partial. Semantic numeric refusals preserve the prior aggregate-error behavior and code absorption. Incoming payloads and surviving output/control metadata have distinct live owners.

Nonnumeric/custom host accumulator ownership and enclosing governor/report/EXPLAIN controls are separate actual production owners still being implemented by their assigned lanes. These drafts do not establish issue 508 completion or replace complete acceptance/normal mandatory gates.

