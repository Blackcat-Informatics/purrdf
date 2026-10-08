# Paired outer inliner diagnostic review

2026-10-08. **Status: FAILED to demonstrate the requested exact inliner decision.** Actual4629 terminal0/all six commands and driver0/readbacks0, frozen579 source, same native profiles/compiler and original-output32 comparisons0. Compilation and diagnostic artifact generation succeeded; exact Project bulk-edge decision coverage did not. No source/build/forge action by reviewer.

Raw logs are `/opt/purrdf-454-current-qualification-20261008/project-outer-inline-diagnostic-1/{baseline,candidate}-inline-emit.log`. Demangled complete logs, full actual candidate callee and call inventory are retained under Stage `raw/project-outer-inline-diagnostic-review/`.

## Actual findings

Both roles' remarks report ordinary `eval_project<RdfDataset>` into `eval_node<RdfDataset>` as unavailable-definition, baseline5745/candidate5877. Candidate107 additionally reports `eval_project_with<RdfDataset, ()>` into `eval_node` success from its always-inline attribute. These are dispatcher edges, not the requested `SpecExtend` into Project edge. No remark mentions the Project helper closure identity or exact Project SpecExtend decision. Unrelated `TermValue`/range-generator SpecExtend remarks contain cost/threshold values; their different monomorphizations cannot diagnose this failure.

Actual verbose command78 compiles `purrdf_sparql_eval` as `rlib`, linker-plugin-lto and codegen1 without `-Cremark=inline`. Command80 compiles the final qualification binary with the diagnostic flag. This captures final-crate decisions but does not demonstrate the owning evaluator backend's inner edge. Final optimized bodies cannot recover missing rejection cost/hotness/pass history.

The actual diagnostic candidate full SpecExtend body is263LLVM lines and equals the original97434 body after compiler metadata/attribute/content-addressed global suffix normalization; the retained diff is empty. Actual full-module use inventory still has exactly two invocation sites: ordinary Project and contextual yield-transform, sharing `eval_project_sequence<TermId>::closure#1`. The paired df2 baseline has no living Project outer SpecFromIter/SpecExtend definition/call (metadata references are not calls). Baseline source places its projection closure inside ordinary Project. This establishes exact source closure/use-count difference and unchanged failed candidate emission, but does not establish that last-call bonus, hotness, phase ordering or a particular cost threshold caused rejection.

## Required next diagnostic scope

No concrete source repair is justified by these absent decisions. The next bounded diagnostic must actually request and retain inlining remarks from the owning evaluator codegen/backend, plus the final ThinLTO invocation, with identical frozen source/profile/compiler and unchanged thresholds. Its admission must verify the evaluator invocation received the remark request and that the exact current call edge is represented; a final-crate-only successful log is insufficient. The smallest implementation choice for that capture belongs to the root/qualifier; this review does not claim an unexecuted flag route will work.

After actual edge remarks exist, correlate complete call-use inventory with actual success/miss cost, threshold, pass and any recorded hotness/last-use decision. Report absent fields as absent. Do not introduce a mode/type phantom, private unsafe/partial-init guard, copied standard-library body or threshold override from a hypothesis. Whole cost acceptance remains FAILED as settled in `portfolio-final-project-outer-bulk-cost-body-review.md`; this diagnostic does not relabel it.
