# Task 3 attempt-isolation correction

Independent review in T3-review.md correctly blocked static artifact identities.
The earlier T3 implementation report and its 18-test evidence remain historical;
they do not qualify this changed source. Root owns Git, forge and dispatch.

Every admission, arm and comparison artifact now names GITHUB_RUN_ID plus
GITHUB_RUN_ATTEMPT. Admission download names only that exact attempt. Comparison
downloads only the current attempt's arm prefix, excluding admission/comparison
and every older attempt. The original Rust launcher now owns hosted-restore.
It validates the exact current prefix, all twelve admitted cases, directory
types and absence of every destination before moving any evidence. It never
overwrites restored or prior evidence and never substitutes an older attempt.
The existing native YAML reader checks the actual upload/download identities.

Missing current-attempt compiler admission produces an actionable ALL-jobs rerun
error. Missing, stale, mixed or partial arm collections require a FULL profiling
workflow rerun; failed-jobs-only reruns cannot borrow previous audit records.
No measured behavior, production gate or shipping dependency changed.

Meaningful source/runtime fixtures cover stale workflow admission/arm/comparison
identities, broad download selection, isolated attempts 1/2 with prior bytes
preserved, stale or mixed arm prefixes, wrong cases, partial collections and
existing destinations without any preceding move. Actionlint and whitespace
checks pass. An additional Unix fixture refuses aliases to prior evidence while
preserving every prior file. Original report/review files remain unchanged.

## Settled correction checks

All actual builds used CARGO_BUILD_JOBS=8 and --jobs 8 in the admitted root lane.

- cargo test -p purrdf-capi --test native_profile --locked --jobs 8: PASS 22
  tests; tasks/T3-correction-tests.log. Four restoration tests exercise real
  task-owned filesystem content, including unchanged prior attempt bytes, exact
  current-attempt completeness, wrong/stale/mixed prefixes or cases, missing
  arms, existing destinations and alias refusal before any evidence move.
  The real-workflow native YAML fixture rejects stale admission/arm/comparison
  identities and broad artifact selection.
- cargo clippy -p purrdf-capi --all-targets --locked --jobs 8 -- -D warnings:
  PASS; tasks/T3-correction-clippy.log.
- cargo build -p purrdf-capi --example native_ci_profile --profile test --locked
  --jobs 8 --message-format=json-render-diagnostics: PASS;
  tasks/T3-correction-controller-build.jsonl and .log. Actual selected executable
  remains O3, debug assertions/overflow checks enabled, test=false.
- Actual selected executable, GITHUB_RUN_ID=314/GITHUB_RUN_ATTEMPT=2,
  hosted-restore /opt/purrdf-native-profile/314-1: expected exit 1 refusal
  “restore path is not the current campaign”; tasks/T3-correction-operator-refusal.log.
  This is a synthetic identity refusal, not a hosted campaign or restored
  real GitHub artifact. It performs no evidence move.
- actionlint PASS; tasks/T3-correction-actionlint.log.
- Existing shard self-test and live inventory PASS: six feature-unified shards
  cover 42 members; tasks/T3-correction-shard-self-test.log and
  tasks/T3-correction-shard-inventory.log.
- Existing toolchain-pin and gate-parity checks PASS (floating nightly and all
  71 hygiene gates); tasks/T3-correction-toolchain.log and
  tasks/T3-correction-parity.log.
- Changed-surface rustfmt --check PASS; tasks/T3-correction-fmt.log.
  git diff --check PASS.

Status: implementation and focused verification complete, ready for independent
re-review. No campaign, dispatch, commit, push or full qualification was performed.
The earlier independent BLOCKED review is retained; only the reviewer may provide
the new disposition. Command wall windows remain scheduling observations, not
certified six-runner speedup evidence.
