#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Stage-only command driver; invoke only after explicit root lane admission.
set -euo pipefail
stage402_root="/home/paudley/Active/purrdf/.worktrees/402-shacl-profiles-reports/.stage/shacl-profiles-reports"
qualification402_logs=$(cat "${stage402_root}/tasks/current-native-log-root.txt")
cd /home/paudley/Active/purrdf/.worktrees/402-shacl-profiles-reports
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8
run402() {
  local name="$1" result
  shift
  [[ ! -e "${qualification402_logs}/${name}.log" ]] || {
    echo "Refusing to overwrite ${name} evidence; use owning failure discussion" >&2
    return 2
  }
  printf '%q ' "$@" >"${qualification402_logs}/${name}.command"
  printf '\n' >>"${qualification402_logs}/${name}.command"
  if "$@" >"${qualification402_logs}/${name}.log" 2>&1; then result=0; else result=$?; fi
  printf '%s\n' "${result}" >"${qualification402_logs}/${name}.exit"
  printf '%s actual exit %s\n' "${name}" "${result}"
  [[ "${result}" == 0 ]] || { tail -n 50 "${qualification402_logs}/${name}.log"; return "${result}"; }
}
packages402=(-p purrdf-shapes -p purrdf-validate -p purrdf-sparql-conformance -p purrdf-conformance-kit)
case "${1:-}" in
  native|native-resume)
    if [[ "$1" == native ]]; then
    run402 rustc rustc -vV
    run402 cargo cargo --version
    run402 fmt cargo fmt --all --check
    run402 native-packages cargo test --locked --jobs 8 "${packages402[@]}"
    run402 invocation-admission cargo test --locked --jobs 8 -p purrdf-sparql-eval --lib user_fn::tests::invocation_admission
    rg -q 'test result: ok\. [1-9][0-9]* passed;' "${qualification402_logs}/invocation-admission.log"
    run402 allocation cargo test --locked --jobs 8 -p purrdf-shapes --test complete_reports warm_complete_core_allocation_count_has_no_per_focus_growth -- --exact --nocapture
    rg -q 'complete Core warm focus allocations: N=16 [0-9]+, N=32 [0-9]+' "${qualification402_logs}/allocation.log"
    rg -q 'test result: ok\. 1 passed;' "${qualification402_logs}/allocation.log"
    [[ ! -e "${qualification402_logs}/community-output" ]]
    run402 community-cli cargo run --locked --jobs 8 -p purrdf-sparql-conformance --bin community-conformance -- corpora/community "${qualification402_logs}/community-output"
    else
      # Resume only the unrun tail after the documented initial scoreboard-label
      # assertion error. No previously successful execution is repeated.
      for prior402 in rustc cargo fmt native-packages invocation-admission allocation community-cli; do
        [[ $(cat "${qualification402_logs}/${prior402}.exit") == 0 ]]
      done
      [[ $(cat "${qualification402_logs}/native-controller.exit") == 1 ]]
    fi
    rg -q '^COMMUNITY SHACL TOTAL: cases 64 executions 115 passed 115 failed 0 unsupported 0 observations 535 passedObservations 535$' "${qualification402_logs}/community-output/scoreboard.txt"
    rg -q '^COMMUNITY SHACL PROFILE shacl-20170720: executions 57 passed 57 failed 0 unsupported 0$' "${qualification402_logs}/community-output/scoreboard.txt"
    rg -q '^COMMUNITY SHACL PROFILE shacl12-20260918: executions 58 passed 58 failed 0 unsupported 0$' "${qualification402_logs}/community-output/scoreboard.txt"
    test -s "${qualification402_logs}/community-output/records.json"
    test -s "${qualification402_logs}/community-output/earl.nt"
    run402 bench-inventory cargo bench --locked --jobs 8 -p purrdf-shapes --bench shared_views -- shacl_complete_reports --list
    [[ $(rg -c '^shacl_complete_reports/' "${qualification402_logs}/bench-inventory.log") == 8 ]]
    run402 bench-correctness cargo bench --locked --jobs 8 -p purrdf-shapes --bench shared_views -- shacl_complete_reports --test
    run402 clippy cargo clippy --locked --jobs 8 "${packages402[@]}" --all-targets -- -D warnings
    run402 rustdoc env RUSTDOCFLAGS=-Dwarnings cargo doc --locked --jobs 8 "${packages402[@]}" --no-deps
    run402 owning-fastgates make helpers-hygiene layer-hygiene terminal-hygiene thread-local-hygiene build-profile-hygiene
    run402 no-features cargo run --locked --jobs 8 -q -p helper-census -- --no-features
    run402 non-rust-ratchet cargo run --locked --jobs 8 -q -p helper-census -- --non-rust-ratchet --merge-base-with origin/main --target worktree
    run402 parser-drop-selftest python3 scripts/check-shapes-parser-drops.py --self-test
    run402 parser-drop python3 scripts/check-shapes-parser-drops.py
    run402 generated bash scripts/check-generated.sh
    run402 wasm-interface python3 scripts/check-wasm-js-exports.py
    run402 python-interface python3 scripts/check-python-stub-parity.py
    printf 'NATIVE CORRECTNESS/GATES COMPLETE; semver requires separate root lane admission\n'
    ;;
  semver)
    run402 semver-tool cargo semver-checks --version
    run402 semver-shapes cargo semver-checks check-release -p purrdf-shapes --baseline-rev rust-v3.0.1 --release-type minor
    run402 semver-validate cargo semver-checks check-release -p purrdf-validate --baseline-rev rust-v3.0.1 --release-type minor
    printf 'NATIVE PREPARED BATCH COMPLETE; host/render/timing/hosted gates remain separate\n'
    ;;
  report-timing)
    # Separately admitted only after the remote308 performance campaign terminates.
    export PURRDF_BENCH_HOME="${qualification402_logs}/bench-estimates"
    run402 complete-report-timing cargo bench --locked --jobs 8 -p purrdf-shapes --bench shared_views -- shacl_complete_reports
    ;;
  *) echo 'usage: current-native-qualification.sh native|native-resume|semver|report-timing (explicit root admission required)' >&2; exit 2 ;;
esac
