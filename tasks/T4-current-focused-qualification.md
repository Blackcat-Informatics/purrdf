# Current merged composition: first focused qualification

Status: first bounded focused phase PASS on settled source. Actual list session
92181 exited0; initial test/clippy session19517 exited101; corrected sequential
session68055 exited0. All owned children are terminal. Real doc/CAPI/test-to-dev
production collector arms and owned snapshot have NOT STARTED; they await root
admission. No timing/performance or complete hosted acceptance is claimed.

## Source and execution boundary

Candidate HEAD221b1ace8bd30e010dba6b9d732cfc9ff576a0ed with MERGE_HEAD
5384882d65750ee22bddfeeac473095ab9c3db03. Settled staged tree
`ff72f20806a30bb7d90ea5886713a9994e7100eb` supersedes initial8b9 after one
root-authorized fix: `artifact_metadata(&message)?` becomes
`artifact_metadata(message)?` in the already-borrowed parsed-frame loop.
Only support/profile.rs was edited/staged by this worker, as expressly admitted;
no merge state, refs, other tracked files, commit/push/forge state changed.
Initial three-path source patch remains preserved, and settled patch is retained
separately in `tasks/T4-current-logs/source-settled.patch`.

Every workload ran in a user systemd scope with `MemoryMax=64G` and
`MemorySwapMax=0`, Cargo jobs8, CARGO_BUILD_JOBS=8 and RUST_TEST_THREADS=1.
Actual cgroup values68719476736 and0 are retained in resource-limits.txt.
The first scope invocation's incompatible --scope/--pipe combination was refused
before any child/Cargo start; scope-initial-refusal.log preserves it. Corrected
scope invocation omitted --pipe; no resource or verification escape was used.

Same active raw SDK first PATH:
`/home/paudley/stage/packages/rustup/active-toolchain/bin`.
Actual rustc1.100.0-nightly4b6d04e706108ccfeafe2547fbe857dfe8972bad,
LLVM23.1.1, with complete rustc.txt/cargo.txt outputs retained. Existing
Cargo-created target `/opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90`
and build `/opt/.cargo/slots/95be0b9034a0e1f3/0/build` reused. No synthetic tags,
artifact-admission override, HOME/config/cache rewrite or sibling workload change.

## Actual settled results

Exact sequential commands are retained in focused-commands.sh and commands.txt;
every actual terminal is in terminals.txt. All paths below are under
`tasks/T4-current-logs/`:

- test-list.log: actual28tests/0benchmarks, command exit0. tests.log: all28
  passed,0failed/ignored/filtered, command exit0. The three new telemetry fixtures
  execute mixed Cargo/libtest frames, live library capture then owned cleanup,
  exact C-smoke frame binding, malformed/missing/ambiguous/tampered refusal and
  full phase/receipt validation. Existing real SDK configuration queries and
  source/config/coverage/counterfactual/restoration controls also pass.
- clippy.log: strict purrdf-capi all-target clippy with -Dwarnings exit0.
  Initial clippy-initial-101.log is preserved: the sole extra-borrow warning
  appeared through both shared example/test inclusions. The root-authorized
  one-token correction was followed by rerunning all28tests and strictclippy.
  tests-before-clippy-fix.log retains the original28test pass, not current proof.
- controller-build.log: actual Cargo JSON example build under profiletest,
  jobs8, exit0. Exactly one selected example artifact from the actual manifest
  has `target.test=false`, opt_level3, debug_assertions/overflow_checks true,
  fresh=false and empty features. Complete frame/identity is retained in
  controller-artifact.json. Selection/identity readback exited0; no guessed or
  historical executable was substituted.
- census.log plus shared-helper-self-test.log/shared-helper-gate.log: all0;
  current81enforcedjobs,23reasonedvariants,91distinctrows,1916files, no outside-
  home/unsanctioned groups or escaped path includes.
- actionlint.log, shard-self-test.log/shard-inventory.log: all0, six feature-
  unified shards cover42workspace members on current metadata.
- profile-self-test.log/profile-gate.log: all0, current1017units across2gate
  invocations build O3 with assertions and overflow checks on.
- toolchain.log, parity-self-test.log/parity.log: all0,71local/hosted gates match.
- fmt.log and scoped whitespace.log: exit0. The scoped staged whitespace check
  covers resolved manifest/telemetry/Make/workflow paths; it does not relabel
  incoming verbatim vendor license whitespace as a broad diff-check pass.

Selected actual controller:

`/opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90/debug/examples/native_ci_profile`

bytes1610056; BLAKE3
`c23f0bd194a0f0ef7c9306406ea356ce4e05c88cd4c2dc6ae59ef371564ccadf`.
controller.path and controller-readback.blake3 match this selected artifact.
source-settled.sha256/source-readback.log verify all11captured source/caller/
manifest/config inputs. git-context.txt preserves actual HEAD/MERGE_HEAD;
unstaged-tracked.txt is empty. Initial source/patch receipts remain separate.

## Remaining admission

The actual production doc lane, capi-test lane and owned same-source capi-dev
counterfactual are still required to close the observed live collector failures.
Use T4-current-qualification-handoff.md with the NEW staged tree ff72 above,
not its historical8b9 snapshot instruction. Root will prepare/admit those
three original production seams; no attempt was started opportunistically.

Earlier hosted37738739777 and37746157677 remain failures. This focused local
PASS cannot convert incomplete historical arms into complete comparisons or
prove reduction on the current merged composition. Following production seam
qualification, root owns independent acceptance, normal hooks/commit/push and
a fresh complete twelve-arm cold/warm hosted campaign. Numerical/performance
acceptance remains NOT MET. No blanket full suite ran or is claimed.
