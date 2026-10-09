# Retained-case fixture disk portability

Status: SOURCE ASSESSMENT COMPLETE; proposed repair and runtime checks UNRUN. No shipping source/index/ref was edited and no build/test/process/dispatch was started. 454 retains the local build lane. This proposal leaves all measured campaign evidence and production validators intact.

## Exact cause

Four tests in `tests/support/hosted.rs` call `retained_case`: `uploaded_case_reclamation_preserves_proof_and_all_siblings`, `incomplete_or_changed_case_refuses_before_reclaiming_any_cache`, `private_cache_boundaries_and_upload_outputs_are_mandatory`, and Unix `symlinked_case_or_cache_never_reclaims_foreign_bytes`. Each currently allocates `temp_dir!()`, placing real case paths beneath compile-time `CARGO_TARGET_TMPDIR`. `retained_case` writes real cold/warm/child files and calls `completed_case`, which calls the unchanged production `profile::completed_pair` → `validate_request`. That home explicitly requires an absolute `/opt` arm directory; normal GitHub Cargo scratch lives under its checkout in `/home/runner/work`. Therefore failure occurs before the intended positive/reclamation or negative-neighbour assertion. Our local `/opt` Cargo targets masked this defect. The macro is correct for the other fixtures and must not be globally altered.

The four fixtures do not run real Cargo or a C workload. Their child/proof bytes are existing bounded fixture construction, while the case/path/cache/sibling/symlink filesystem is real. The proposed change does not promote fixture receipts to campaign evidence.

## Smallest coherent repair

Replace ONLY those four holder constructors with `purrdf_testkit::TempDir::new_in("/opt/purrdf-native-profile-tests").unwrap()`. Existing testkit exclusive PID/counter/time child creation, automatic drop cleanup and error behaviour provide uniqueness and ownership; do not write another temporary-path helper. Each test owns only its unique child. Parent is separately provisioned, is not a measured Cargo artifact directory, and is never recursively removed or adopted by the test. Keep `retained_case`, request serialization, `completed_pair`, `valid_receipt`, canonical/symlink checks and all negative controls unchanged.

Provision the parent explicitly in ordinary CI **before integration-2 execution**:

```sh
sudo mkdir -p /opt/purrdf-native-profile-tests
sudo chown "$(id -u):$(id -g)" /opt/purrdf-native-profile-tests
```

Use `if: matrix.shard == 'integration-2'`; no need for unrelated five runners to change their filesystem. Add the same bounded parent provisioning to optional `native-profile` setup, before its first measured child, because its `/opt/purrdf-native-profile/<run>-<attempt>` ownership does not grant write access to the sibling fixture parent. No recursive chown, chmod777, root test execution, synthetic `/opt` strings, target-directory overrides, symlink aliases, cfg(test) validator relaxation or ignored test is needed. The new CI setup must be represented in workflow source qualification, including that integration-2 provisioning precedes the native test command and optional campaign provisioning precedes the first measured arm.

Local callers use this same declared `/opt` requirement. On the current admitted host the owned parent can be created/written directly; on a fresh developer host an administrator provisions that one parent once with the above ownership command. Do not silently fall back elsewhere or have tests invoke sudo. `TempDir::new_in` creates an absent parent only where permissions allow; failure is an actionable prerequisite failure. No Make command needs a hidden privilege action or whole target relocation. Document this narrow prerequisite beside the controller's local qualification instructions before claiming portable local full-gate acceptance.

## Every source-discovered execution environment

| Caller | Executes these four fixtures? | Required handling |
|---|---|---|
| `make check` workspace Cargo tests; `make test`; direct `cargo test --workspace`; direct `cargo test -p purrdf-capi`; focused `--test native_profile`; running a previously built native_profile binary | Yes | Local owned fixture parent must be writable, independent of Cargo target/build position. |
| Ordinary CI `test-shard`, `make test-shard SHARD=integration-2` | Yes: `native_profile` matches `[e-o]*` | Explicit parent provisioning on integration-2 runner before command. |
| Ordinary lib/doc/integration-1/3/4 shards | No | No fixture scratch requirement; preserve selectors. |
| Optional controller `before-monolithic` cold/warm | Yes: lane invokes `make test` | Same campaign runner must provision fixture parent before first measurement. Both original and current library-profile sources include this unchanged test target. |
| Optional `after-integration-2` cold/warm | Yes | Same provisioned parent, fresh unique fixture children per invocation. |
| Optional other ten cases, including C smoke and downstream consumer | No | Do not add measured work or change inventory for these arms. Shared setup provisioning remains visible as setup cost, not a claimed cold-cache reset. |
| Local controlled monolithic/integration-2 production controller requests | Yes | Parent provisioned before measurement and captured as prerequisite. No need for `/opt` Cargo target itself to make tests pass. |
| Building/checking/clippy for `native_ci_profile` example | No runtime fixture execution | Example has `test=false`; its `#[cfg(test)]` shared modules are not executed by normal workspace test selection. Explicit future example-test execution must meet same prerequisite. |
| Hooks/helper census/workflow contracts/shard checks, `make capi-check`, release workflow transport/slice tests, selected wasm/runtime suites | No execution of these four fixtures | Preserve existing behaviour. Compiling or scanning a shared test module is not fixture execution. |

This inventory is from current `Makefile` test/test-shard commands, `profile::lane_command`, `hosted::selection`, `scripts/test-shards.py`, current workflows and CAPI manifest. No other workflow contains a whole-workspace/native_profile runtime command. The dedicated CAPI target is a host-only controller test; this does not make `/opt` a release-library dependency or affect shipping wasm semantics.

## Alternatives and required real checks

Changing `CARGO_TARGET_DIR` for all native tests merely to move these four holders is much larger and changes the workload whose cost is being measured. Using a configurable fixture root still needs strict `/opt` validation and explicit provisioning, adds an unnecessary knob and cannot solve an unwritable `/opt` host. Using a synthetic request path while files live elsewhere breaks attributable containment. Loosening `validate_request` or splitting a permissive test validator defeats the actual production-path check. The exact four constructor changes plus two setup sites and honest local prerequisite are the smallest correction.

After root approves source editing and later admits the lane: run the existing full `native_profile` target (current source inventory35), strict affected CAPI all-target clippy, fmt/workflow/shard/profile/parity/helper checks as previously owned. Demonstrate the four tests with **Cargo-created target/build paths outside `/opt`**, while the fixture parent is genuinely under `/opt`; this is the missing valid neighbour and needs no new fake fixture or whole-workspace test. Inspect retained paths to verify they use real unique children, all owned children are cleaned, symlink negatives preserve sentinel bytes, and the parent/siblings survive. Keep any initial refusal. Before a fresh campaign, requalify exact corrected workflow setup and source identities; do not infer full12-arm or numerical acceptance from this focused repair.
