<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #531 stack floors versus #470 acceptance: bounded classification

VERDICT: The **three failing floor assertions predate #470's exact-only checkpoint and are present on a main-based optimized package**. They are established failures of the unchanged full Node package gate, not established #470 numeric regressions. This does **not** turn `npm run check` into a pass or waive its required status in the #470 Stage plan. No build or source edit was made for this review.

## Source and artifact comparison

- Retained #470 issue body requires exact-only public numerics, exact-or-18-half-even default, exact temporal seconds, CLI/Rust/Python/C/WASM coverage, governor version/vectors, and documentation. It does not set the recursive stack floors. The Stage plan's Task 3 separately requires optimized WASM package tests and the repository gate; their failed results must remain visible.
- PR #535's head `96f8c5f6` was based on merged-main `dd79ef77e`, before #470 exact-only commit `fff66a72e`. Its [hosted `wasm-package` result](https://github.com/Blackcat-Informatics/purrdf/issues/531#issuecomment-6117980784) was a fresh optimized package build and reported **446 passes, the same three floor failures**: sync 135 against 182, right-nested LATERAL 236 against 282, and NOT EXISTS 134 against 181. PR #535 changed packaging, not the failing evaluator and test sources. The separate [#365 optimized package measurement](https://github.com/Blackcat-Informatics/purrdf/issues/531#issuecomment-6118368906) also failed LATERAL and NOT EXISTS floors at 248/142. These independent pre-#470 observations establish the existing failure mode. A local artifact made via the temporary-path packaging route would not by itself establish source attribution, and the varying depths caution against assigning exact per-level bytes to one source change.
- `git diff dd79ef77e..fff66a72e` shows no changes to `crates/sparql-eval/src/binop.rs`, `crates/rdf-wasm/js/tests/query.test.mjs`, or `crates/rdf-wasm/js/tests/async-concurrency.test.mjs`. The only `eval.rs` change in that range is division-policy documentation. #470 does alter XSD values and evaluator numeric/cost paths, so source comparison alone cannot prove identical optimized stack frames.
- The retained #470 optimized package from `fff66a72e` (WASM SHA-256 `744d7f5a...`) failed full `npm run check` with sync/job **134/134 against >=181**, LATERAL **234 against >=281**, and NOT EXISTS **133 against >=180**. The deep async test passed; selected exact numeric/default-policy package tests passed **13/13**, and standalone installed-package smoke passed. These selected successes establish the tested #470 host behavior only. The full command exited **1** (`validation.md`, final-source WASM section).

## Acceptance consequence and unknowns

The main-based observation above is the retained hosted optimized package result,
not an attribution derived from the old mismatched local temporary-path artifact.
The rejected tooling proposal is not reinstated or rerun by citing its receipt.

The #531 debt is a legitimate separate production owner: the original stack tests still fail on main-based and #470 packages. It is **not evidence of a newly introduced #470 numeric defect** and does not by itself require the #470 owner to change evaluator recursion. Nonetheless, Task 3's full optimized package check remains **FAILED**, not completed or accepted with a caveat; any required hosted check with the same failure will also block integration under the normal gate. No assertion, budget, or acceptance criterion is changed here.

**UNKNOWN:** Whether #470's altered value layouts or optimizer effects account for the small 1–2-level difference between the PR #535 and #470 measurements. The receipts do not use a matched compiler/package build of main and #470, so non-regression of exact stack depth is not proven. The current uncommitted `binop.rs` #531 LATERAL helper has native tests only; it has not passed optimized-package floors and cannot be credited as a correction. A queued fresh default-directory package remains pending. This review does not approve full #470 issue completion or its other open repository/portable gates.
