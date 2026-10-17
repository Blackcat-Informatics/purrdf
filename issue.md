# Issue #588: Refresh shared cross-host proof goldens for current-support encoding

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

PR587 added shared current-support proof recording and v5 encoding but did not refresh the shared cross-host proof golden artifact consumed by purrdf-validate, C ABI, WASM and Python. The entailment unit/service pins were updated; that narrower check missed the boundary artifact.

The completed [wasm-package job117458981838](https://github.com/Blackcat-Informatics/purrdf/actions/runs/39054291844/job/117458981838) now fails in the real optimized WASM entry entailCheckProofGoldenVectors: case realize reports a proof mismatch. Its source is crates/validate/tests/fixtures/dl-proof.vectors, loaded by regime::check_dl_proof_golden_vectors. Full decoded logs are retained in the active GMEOW1843 Stage while this regression is repaired.

Regenerate the proof and replay-report bodies from the actual current native producer while retaining every case name, question, source document and expected answer. Review every changed byte-bearing field and trust classification. Preserve all shared-host consumers and their exact mismatch checks; do not skip, weaken, delete or silently accept this test. Provide an explicit Rust fixture refresh operation if the artifact has no owning producer, keeping normal tests read-only and failing on drift.

Acceptance: all shared proof vectors produce and verify natively and on actual WASM; affected Rust/host boundary checks and normal hooks pass; optimized package qualification reproduces the actual failed consumer or is honestly distinguished from focused portable evidence. Merge only with ghprsq, retain raw evidence, and inspect review/CI feedback. This is a regression of587, not completion of the broader GMEOW node-kind/native integration work.


## Comments (0)

