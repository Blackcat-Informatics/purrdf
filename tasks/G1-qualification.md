# Bounded feedback qualification

Status: affected local qualification PASS; heavy lane FREE. Actual sequential
qualification session99286 exited0 after all checks. No owned process remains.
Normal hooks/commit/push, feedback disposition, hosted acceptance and integration
remain root-owned; this report does not certify those surfaces.

Candidate HEAD b1833aeed387299546f883baaacb87f2206e169a plus the three-path
uncommitted delta captured in G1-logs/source-delta.patch. Read applicable AGENTS,
root .baseline/.goals, remediation, root review and current validation before
qualification. No Git/index/catalogue mutation or resource-heavy example main
execution occurred. Only owned source repair: replace three newly added scratch
macro calls with existing purrdf_testkit::TempDir::for_unit_test().

All Cargo commands used the same active SDK first on PATH:
/home/paudley/stage/packages/rustup/active-toolchain/bin, CARGO_BUILD_JOBS=8,
CARGO_BUILD_BUILD_DIR=/opt/purrdf-260-t3-full/settled-build and
CARGO_TARGET_DIR=/opt/purrdf-260-t3-full/settled-target. Existing Cargo-created
roots/tags were preserved; no manufactured TMPDIR compile variable, cache/tag
override or weaker test selection was used. Compiler: rustc1.100.0-nightly
4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM23.1.1; Cargo1.100.0-nightly
7941be6fb. Actual version outputs are retained in G1-logs.

## Actual results

Every command below exited0, with its named log under tasks/G1-logs.

* glossary-tests.log: cargo test --locked --jobs 8 -p helper-census --bin
  helper-census glossary::;12passed,0failed/ignored,58unrelatedfiltered. Includes
  K lone/mixed regex refusal, literal K keep/drop and non-K regex matching.
* example-controls-corrected.log: cargo test --locked --jobs 8 -p helper-census
  --example glossary_hook_probe;3passed,0failed/ignored. Exact restoration, normal
  failure refusal and original unwind payload preservation all executed in
  testkit-owned scratch. The intentional Is-a-directory diagnostic is retained.
* callers.log: cargo test --locked --jobs 8 -p helper-census --test
  glossary_callers;2passed,0failed/ignored. Real external inputs/renamed tracked
  Markdown and missing Make/CI coverage refusals remain active.
* clippy.log: cargo clippy --locked --jobs 8 -p helper-census --all-targets --
  -D warnings; warning-free.
* native-self-test.log: cargo run --quiet --locked --jobs 8 -p helper-census --
  --glossary-gate --self-test;69rows/43rejections/24Ktokens/1716controls.
* native-scan.log: same cargo run prefix, --glossary-gate; actual current scan
  69rows/43rejections/24Ktokens/4181units/3trackedtranslatedMarkdown/1716controls.
* parity-self-test.log and parity.log: python3 scripts/check-gate-parity.py
  --self-test and without arguments; both production caller contracts pass.
* helpers-hygiene.log: make helpers-hygiene; structural census, legacy wrapper
  self-tests and hash-domain hygiene pass. Registry table remains current.
* fmt.log: cargo fmt --all -- --check; whitespace.log: git diff --check.

Total selected Rust tests17passed,0failed/ignored. Terminal ledger retains exact
initial/corrected outcomes; the separate first glossary command actual exit0 is
recorded in its log and report.

## Preserved failure and correction

Initial explicit example compilation exited101 (example-controls.log): Cargo
does not define CARGO_TARGET_TMPDIR for this explicitly selected example harness,
and temp_dir! requires it at compile time. Existing testkit documents its
for_unit_test constructor for targets without that variable: it derives the
nearest real Cargo CACHEDIR.TAG ancestor from the executable, refusing absent
ownership. Three constructor replacements solved the owned fixture error.
Corrected three-control harness and all-target clippy actually pass. No invocation
workaround or successful-path restoration fallback was introduced.

## Evidence applicability and handoff

G1-logs/source-receipt.sha256 binds14source/caller/config/catalogue/render inputs;
source-readback.log verifies all14match with exit0 after qualification. Exact
patch and HEAD retained. No translations, catalogue, render implementation,
shipping dependencies/features, caller wiring or existing table entries changed.
Prior full make check and six-arm rendered book qualification at b1833aeed remain
historical evidence for unchanged scope. They are not relabeled as new execution.
Current affected parser, Drop/example, production scan and caller/hygiene paths
are qualified above. No blank full-suite/render repetition, real hook-poison
probe main, Git/forge action or commit bypass was performed. Root independently
reviews the final three-constructor correction before normal handoff.
