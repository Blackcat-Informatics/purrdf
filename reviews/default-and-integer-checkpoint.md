# Independent review: default division and integer-reader checkpoint

VERDICT: PASS for committing this logical partial checkpoint after the three review blockers were corrected. This verdict covers only the exact-or-18-half-even default, affected hosts, governor profile v17/corpus, and the exact public integer readers. It does not assess completion of either numeric issue.

1. **Resolved: checked result-set index conversion.** `crates/sparql-conformance/src/rs_resultset.rs` now narrows the exact integer through `as_i128()` and `usize::try_from`, preserving refusal of negative or oversized indexes. The owner reports that the package's production check passed; I inspected the correction but did not rerun that check.

2. **Resolved: independent default vector oracle.** `crates/xsd/examples/gen_exact_numeric_vectors.rs` now uses the testkit rational oracle's exact terminating projection, and uses 18-place half-even rounding only when the quotient has no finite decimal expansion. The 7,164-row frozen file was regenerated; its `div 1 1.000000000000000000000000000000000000001` answer at line 239 is now `1`, and the `1 / 2^127` row retains its full terminating scale. The owner reports focused replay 3/3. I did not rerun native/WASM replay and this checkpoint does not prove complete cross-target qualification.

3. **Resolved: affected policy descriptions.** The shipped Python README, XSD module example, and vector generator now describe the exact-terminating/rounded-repeating default. The remaining `truncating div_rem` phrase describes integer quotient/remainder, while `N` still correctly denotes an explicit toward-zero scale policy. `git diff --check` passes.

The reviewed production path dispatches exact pairs through tower division, preserves explicit `18:toward-zero`, charges the default tower path, and treats a typed non-terminating result as the sole fallback trigger. Profile v17 and the new 52-case governor corpus are internally aligned by the recorded focused evidence. I did not run broad gates or independently reproduce the recorded validation commands. Full exact-only values/temporal, #443 performance and algorithms, and full #470 acceptance remain pending.
