Track the failing acceptance exposed by the SHACL identity-catalog integration on `paudley/569-design-around-recursive-shacl-validation`. The integration PR is being published at the user's explicit request with its scope frozen. This issue does not waive required checks or close the complete recursive-validation contract in #569.

Observed native qualification:

- `cargo test -p purrdf-core -p purrdf-shapes --lib --no-fail-fast`: core **1398 passed**; shapes **947 passed, 41 failed**.
- Strict core/shapes/validate/Python all-target Clippy passed with warnings denied before the final test-only pin adjustments.
- All 18 incremental-footprint differential controls and all 10 carried-analysis controls passed.
- Actual admission fell to 230 allocations from the old 293 baseline. Its exact pin has been lowered and needs re-execution. Static-root preparation is 18 allocations / 3106 requested / 0 retained / 1674 peak bytes at 1/8/32 retained definitions; its ceilings were tightened and need re-execution.

Required investigation and remediation:

1. Restore schema-import logical round-trip parity: `schema_import::tests::emitted_logical_shapes_round_trip_byte_exactly` reports `schema-applicator-dropped` losses after helper-definition references are emitted. Do not remove the lossless/byte-equality assertions or silently drop applicators.
2. Preserve compatibility query-admission applicability for retained but unreachable definitions: `shapes::parser::admission::tests::rec_audits_unreached_constraint_declarations_but_not_unrelated_selects` now rejects the legacy unattached constraint. Keep the dated graph-wide and execution-qualified laws distinct; no swallowed query refusal.
3. Investigate the reifier-message override failure in `engine::complete_reports::rec_core_messages_ignore_draft_reifier_overrides_and_legacy_keeps_them`. The test currently annotates a root reference handle after parsing; establish whether the annotation must instead be authored on the actual definition, without weakening per-profile message/severity assertions.
4. Adapt model-inspection fixtures to original definition/property views while preserving their assertions. Failures include direct parser constraint/path/list inspection, rule conditions and empty rule-head expressions, box-role collection, retained SPARQL provenance, and range-bound inspection. Runtime parity must be checked separately; do not assume all failures are obsolete fixtures.
5. Reconcile source-occurrence admission's native catalog retention with `dated_product_sources_reconstruct_without_legacy_readmission_or_model_relinking`. Preserve the complete product-source/provenance contract and the prohibition on silently incomplete catalogs.
6. Complete the already-known explicit recursive mode/solver prerequisite in #569 for the bundled recursive SHACL meta-schema. Three lint-import tests currently fail because that schema is refused without selected recursion semantics; this is an existing unfinished requirement, not newly delivered behavior.
7. Refresh the stale preparation stage from the actual census and regenerate only the mutable current-stage product through its supported generator. The frozen format-epoch artifact must remain unchanged. Re-run affected census/product checks.
8. Execute the revised new shared-DAG allocation test at depths 1/8/32. Its initial adjacent-ratio requested-byte assertion incorrectly assumed equal hash-table capacity utilization (112077 bytes at depth 32 versus 27597 at depth 8); the revised guard uses the depth-one linear envelope (3553 bytes per depth). Preserve bounded expansion and the disconnected untouched target.

The complete failing test inventory is recorded in the integration PR's local Stage log `shared-catalog-core-shapes-units.log`; the groups above cover all 41 failures. Acceptance requires the relevant original controls to pass and an explicit mapping for any remaining failure. No ignored tests, raised original allocation ceilings, hook bypasses, or unsupported success claims.
