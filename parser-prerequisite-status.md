<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Shipping parser prerequisite

The release-only no-op scope-consultation helper unnecessarily required a mutable receiver. The required shipping lint diagnosed this on unchanged main. The fix changes only that private release receiver to `&self`; the debug mutable counter remains intact. Independent source review covers it in the cumulative governor review and task records. Focused release evaluator and CLI clippy receipts pass with the fix.

Committed normally with signing as `f35709b0e`, `fix(sparql-algebra): keep release scope observation immutable`. All configured pre-commit hooks completed successfully; the staged non-Rust ratchet reports zero changed non-Rust paths. Only `crates/sparql-algebra/src/parser.rs` was staged. Normal push succeeded to `paudley/478-sparql-governor-governed-parallel-row`.

This is a prerequisite commit, not completion of either governor issue. The common cell-layout/allocation repair is still being implemented. Required final-source numerical acceptance, including metered/plain <=1.2 at both 4 and 32 workers, remains open. No PR, merge or full-workflow qualification is claimed.
