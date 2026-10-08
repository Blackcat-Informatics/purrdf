# Shared host golden remediation

Status: FOCUSED REMEDIATION PASS. Original generators and independent byte/cause
audits completed; focused native/C ABI/WASM-native, actual Python and public
packaged WASM/Node checks passed. Root owns independent review, normal
hooks/commit/push and any justified
full-gate continuation. Initial full gate failed, root verified exit 2; its log
remains tasks/T6-logs/make-check.log. This is not completed delivery.

## Source ownership and regeneration

Base HEAD 9867eba224035eb86c128e3c4a8df97b2c986210. The stale shared fixture is
crates/validate/tests/fixtures/regime-boundary.vectors, compiled verbatim into
the shared validate checker used by C ABI and WASM, also consumed by Python.
The original deliberate ignored Rust generator is
regime::tests::regenerate_regime_golden_vectors. It regenerated nine cases
covering all seven regimes under the stated native limits. Its execution is
artifact regeneration, not counted runtime qualification.

Searching every old native hash in actual source/test hosts (excluding Git,
Stage and target artifacts) identified one more first-party rendered report:
crates/sparql-conformance/tests/goldens/entailment/disjointclasses-001.report.
Its original Rust regenerate_mechanism_golden generator updated that report.
No unrelated historical pin was mechanically replaced; no external GTS or
vendored conformance payload was edited.

The fixture now includes an original version-2 transition note. Its stale
268,435,456 native-work-limit prose was corrected to the actual unchanged
1,048,576 constant. The mechanism oracle's documentation now explicitly binds
versioned evaluator semantics and effective limits. These are the only three
tracked changed paths; diagnostic planner/source modifications were restored
byte-for-byte before focused settled qualification.

## Independent audit and actual cause

An original Rust line auditor independently compares the artifact against
git show HEAD, with strict equal line count and only contract-hash/work changes
admitted inside report sections. Before adding the original transition note it
proved exactly nine contract hashes and three join counts changed; every case,
regime, input, program, closure, tally, witness field and non-work-budget byte
remained identical. The mechanism report changed only one hash and one work
count; its refutation mechanism, rule counts, boundaries and other budgets are
unchanged. Audits and original/current/control copies are retained under T6-logs.

| Regime/case | Actual work delta |
| --- | --- |
| D supported datatypes | 36 -> 68, 32 premise-free datatype admissions |
| OWL transitive/symmetric/inverse | 1867 -> 1903, 43 admissions minus 7 impossible initial-prefix candidates |
| OWL class expressions/equality | 1778 -> 1809, 43 admissions minus 12 impossible initial-prefix candidates |
| Refutation mechanism report | 1555 -> 1598, 43 premise-free admissions |
| Remaining shared cases | Work unchanged |

OWL's 43 credits are 32 datatype, nine built-in annotation-property and two
class-identity admissions. A narrow connectivity-priority-only counterfactual
changed no byte/count for these nine cases: the initial connectivity explanation
was disproved and corrected. A second isolated counterfactual disabled only the
new full-delta empty-OldOnly-suffix skip and restored exactly seven/twelve OWL
credits, leaving D and mechanism unchanged. Both original generators were run
under those controls. Exact planner/seminaive and actual golden copies were
restored with EXIT traps and verified with cmp. No diagnostic source remains.

All seven regimes have updated contract identities (simple, owl-direct and rif
share the empty-calculus identity); actual nine-case checks verify them instead
of accepting a string substitution. Default limits and authored programs remain
unchanged.

## Actual focused results so far

All direct Cargo commands specify --jobs 8 with CARGO_BUILD_JOBS=8. Nested Make/
backend children inherit that eight-job cap; all heavyweight activity is serial.

- Shared-vector native selection: PASS, C ABI one, validate two and WASM-native
  one tests; every shared checker executes all nine cases. Settled rerun after
  diagnostic restoration/transition note also passed.
- Conformance mechanism oracle: PASS, one exact selected test on final report.
- Python lib has test=false by design because PyO3 requires an interpreter;
  it was not presented as Rust-library runtime qualification. A fresh isolated
  /opt/purrdf-479-9867-t6-python environment used locked uv project/backend build
  with actual CPython 3.13.12. Existing golden_vector pytest selection PASS,
  ten cases: artifact/all-regime coverage and all nine byte-exact closures/reports.
- Shipped WASM package: first build FAILED, exit 2. Strict private-output capture
  refused the selected Cargo artifact. Corrected scoped same-nightly invocation
  PASS, exit 0, with private capture, Binaryen 130 optimization, validated poison/
  suspend/stack postlink and 138081 SIMD opcodes. Public Node package-root golden
  checks PASS, two tests, zero failures/skips, including all nine shared cases.
- Final affected strict all-target clippy PASS with warnings denied; frozen
  external corpus guard and git diff --check PASS. Final fmt check recorded
  separately in host-golden-fmt.log.

Actual logs: host-golden-regenerate.log, host-golden-head-audit.log,
host-priority-counterfactual.log and host-priority-delta-audit.log,
host-initial-counterfactual.log and host-initial-delta-audit.log,
mechanism-regenerate.log and mechanism-initial-counterfactual.log,
host-golden-focused.log and host-golden-focused-settled.log,
mechanism-focused.log, python-host-sync.log, python-host-golden.log and
wasm-host-package.log. No successful full gate, PR, hosted CI or merge is claimed.

## WASM private-output diagnosis

The selected Stage Cargo wrapper is /home/paudley/stage/scripts/cargo, exposed
through /home/paudley/stage/root/bin/cargo. It intentionally removes explicit
--target-dir/--build-dir arguments, clears caller directory environment and
forces leased build/target config. The private builder correctly passed its
private --target-dir and correctly refused the artifact outside that directory.
The failure is preserved in wasm-host-package.log; strict capture stays intact.

The wrapper documents toolchain-bin PATH precedence for direct Cargo. The
verified active-toolchain resolves to nightly-2026-09-14-x86_64-unknown-linux-gnu:
Cargo 1.100.0-nightly (7941be6fb 2026-09-11), rustc 1.100.0-nightly
(4b6d04e70 2026-09-13). Root has received the concrete scoped same-toolchain
invocation with PATH precedence, CARGO_BUILD_JOBS=8, private /opt scratch
CARGO_BUILD_BUILD_DIR and CARGO_TARGET_DIR, TMPDIR and BINARYEN_CORES=8. The
builder's explicit target directory still takes precedence over the scoped host
target directory. No Stage repository, global configuration or shared cache was
changed. Root admitted the scoped rerun and it completed with actual exit 0.

The exact corrected invocation was:

```sh
env PATH="/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH" \
  CARGO_BUILD_JOBS=8 \
  CARGO_BUILD_BUILD_DIR=/opt/purrdf-479-t6-wasm-scratch/build \
  CARGO_TARGET_DIR=/opt/purrdf-479-t6-wasm-scratch/host-target \
  TMPDIR=/opt/purrdf-479-t6-wasm-scratch BINARYEN_CORES=8 make wasm-pkg
node --test --test-name-pattern 'entailCheckGoldenVectors|every golden case' \
  crates/rdf-wasm/js/tests/entail.test.mjs
CARGO_BUILD_JOBS=8 cargo clippy -p purrdf-validate -p purrdf-capi \
  -p purrdf-wasm -p purrdf-sparql-conformance --all-targets --jobs 8 \
  --locked -- -D warnings
```

Registered private output was under tmp.Z2RkzOI70Q/compiler-target-lway23f4,
captured source module SHA256
44d55eea5947a19b81bad374f8b207cd7f8956f3e234993d89a81f2293125475,
cargo_fresh=false. The private temporary compiler directory was removed normally
after capture; its selected JSON record and copied module remain. Receipt
tasks/T6-logs/wasm-private-capture.json preserves strict path identity, and
tasks/T6-source-artifact-receipt.sha256 binds the settled source, carrier module
and optimized public package. Source HEAD is
9867eba224035eb86c128e3c4a8df97b2c986210 with only the three owned tracked deltas.

Settled logs: wasm-host-package-corrected.log, wasm-public-node-golden.log,
host-golden-clippy.log, host-golden-corpus-frozen.log and host-golden-fmt.log.
The original wasm-host-package.log and failed full make-check.log remain intact.
Focused remediation does not establish a successful complete gate, commit,
push, PR, hosted CI or integration.
