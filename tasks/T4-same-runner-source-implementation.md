# Same-runner source delivery and qualification handoff

Status: SOURCE IMPLEMENTED; RUNTIME UNRUN. No Cargo, compilation, test, hook, commit, push, or dispatch ran for this implementation. Direct active-SDK rustfmt and git diff --check both exited0. These establish formatting only. Root retains build admission while 454 prepares its cost lane.

## Integrated workload

HEAD remains f83095460c81d7cfbd93adfb2966098b4118da4c. The ordinary uncommitted merge now has MERGE_HEAD=df2cec88b0f8e05e05ef3d01d14714474be6bbb1: actual main includes native LUBM and merged402 conformance-kit/community-conformance, with real new Cargo libraries, binaries, integration tests, dependencies, workspace membership and Make consumers. Old f830/538 durations cannot qualify this composition.

Root authorized replacing the earlier pending2eb merge. T4-main-df2-sync retains four working preimages, exact binary repair patch, original incoming-index patch/inventory and refs. Restore-only-four-homes0, ordinary abort0, ordinary df2 merge0/no conflicts, patch reapplication0. Repository diff.noprefix=true caused initial default git apply to refuse stripped paths; its failure is retained. Correct git apply -p0 succeeded. All four preimages read back byte-identical; no unmerged entries or other unstaged tracked scope. Incoming composition remains staged; four repair homes remain unstaged. No reset/rebase/history rewrite/new branch/worktree occurred.

## Production changes

1. ci.yaml replaces twelve independent measured runners with one bounded360-minute job. Admission and strict restored-evidence comparison remain. The exact twelve cases each execute cold then unchanged warm with distinct private target/build directories. Shared prerequisites/controller stay outside measured trees. Each current-attempt always-upload precedes successful production hosted-reclaim, binding its actual artifact-id/artifact-digest outputs. Failed execution/upload/reclamation stops dependent execution; failure uploads remain. One always-upload resources artifact retains setup/compiler/disk and reclamation proof. Normal six-shard/C/downstream/mandatory jobs are unchanged.
2. native_ci_profile.rs adds only hosted-reclaim CASE dispatch/usage; original production cargo-filename shim and all modes remain.
3. profile.rs extracts unchanged-warm identity/request matching into one private home used by run and completed_pair. Exact directory/lane/warmth/previous pass the original typed Request/validate_request/valid_receipt homes. Strict source/config/compiler/runner/cache/coverage matching and C selector/typed artifact roles remain. Original receipt construction is reused under cfg(test), not copied.
4. hosted.rs closes workflow inventory over exact execution/upload/reclaim triplets, one bounded runner, no cache action, setup-before-case, retained globs and upload-output binding. Completed receipts integrity-validate before their file identities freeze. Reclamation requires canonical nonzero upload id/lowercase SHA-256 and real exact campaign/case/target/build directories; symlinks, aliases and replay refuse. Existing Recorder captures du/df, deletion and validation. Only target/build are removed after warm/upload; retained Cargo/C/timing evidence, source/controller/registry caches/siblings survive and revalidate. Boundary disk observations are explicitly not high-water marks.

Common setup has two ordered actual endpoints and identities for six endpoint/compiler/disk records. Case preparation starts separately; later cases never count earlier measurements as setup. Download and uncontrolled page caches are disclosed as shared sequentially; private Cargo artifacts remain cold-empty/warm-unchanged. Serial execution windows explicitly establish no parallel speedup.

## Fixtures and remaining checks

Source inventory has35 test attributes (16 profile/14 hosted/5 phases); actual discovery remains UNRUN. Five new tests cover setup missing/malformed/reversed/lost records; real deletion with retained proof and source/controller/sibling sentinels; failed/mismatched/tampered requests/receipts/children refusing before deletion; wrong-root/parent/missing-build/upload-output boundaries; and real Unix symlink refusal preserving foreign bytes. Existing workflow fixtures reject omitted/duplicate/unknown cases, matrix/split runners, fake recipes, cache restoration, wrong upload names/globs/bindings, absent retention/setup endpoint and timeout changes. No fixture is a claimed hosted measurement/upload.

After root admission use active raw SDK first in PATH, jobs8, scoped MemoryMax64GiB/MemorySwapMax0, RUST_TEST_THREADS=1 and existing Cargo-created managed roots. Retain complete logs/terminals and source/compiler/config bindings; preserve first failures.

```bash
export PATH=/home/paudley/stage/packages/rustup/active-toolchain/bin:$PATH
export CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=1
export CARGO_TARGET_DIR=/opt/.cargo/target/308-reduce-native-ci-test-compilation-and-c-95be0b90
export CARGO_BUILD_BUILD_DIR=/opt/.cargo/slots/95be0b9034a0e1f3/0/build
cargo test --locked --jobs 8 -p purrdf-capi --test native_profile -- --list
cargo test --locked --jobs 8 -p purrdf-capi --test native_profile
cargo clippy --locked --jobs 8 -p purrdf-capi --all-targets -- -D warnings
cargo build --locked --jobs 8 -p purrdf-capi --example native_ci_profile --profile test --message-format=json-render-diagnostics
cargo run --quiet --locked --jobs 8 -p helper-census -- --check
python3 scripts/check-shared-helpers.py --self-test
python3 scripts/check-shared-helpers.py
actionlint
python3 scripts/check-test-shards.py --self-test
python3 scripts/check-test-shards.py
python3 scripts/check-build-profiles.py --self-test
python3 scripts/check-build-profiles.py
python3 scripts/check-toolchain-pin.py
python3 scripts/check-gate-parity.py --self-test
python3 scripts/check-gate-parity.py
cargo fmt --all -- --check
git diff --check
git diff --cached --check
```

Select the current controller uniquely from its real Cargo compiler-artifact frame (purrdf-capi/example/native_ci_profile/executable), retaining the full frame/file identity. Owning ratchet/generated checks use their real production commands/normal gates. Expanded df2 workload requires eventual settled full qualification and hosted mandatory six-shard/C/downstream gates under Task5; focused tests do not claim these.

Independent review and normal signed hooks/commit/push precede fresh native_profile=true/simd_projection=false dispatch. Fresh acceptance requires every12case/24receipt, actual upload/reclaim/resources, strict native-partition and nested-profile comparisons for both warmth variants, artifact digest readback, and attributable measured reduction. Actual remote upload evidence cannot be replaced by locally supplied IDs. No PR/merge/completion/numerical improvement is claimed.

## Preserved evidence limits

Run37768038171/f830 COMPLETED/FAILURE: all12armsSUCCESS,24 supported self-validations0, strict runner_class refusal. Its14uploads/watch44188exit1/receipts remain. Historical execution217.241198 + maximum setup12.833333 + upload tails0.933333 =231.008minutes; plus60uncertainty/headroom=291.008, below360 by68.992. Heterogeneous old-source feasibility is not a guarantee for expanded df2. Current elapsed time/within-command disk peak remain unknown; failures are hard. Prior controlled invalidation/role seams retain their exact source limits. See T4-critical-path-evidence-protocol.md for scientific contract mapping.
