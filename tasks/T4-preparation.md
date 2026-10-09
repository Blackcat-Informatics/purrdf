# Task4 matched campaign preparation — 2026-10-08

Status: PREPARED, NOT RUN. No build, dispatch, source mutation, publication or
deletion was performed. Root must admit local execution separately from external
hosted dispatch. The glossary qualifier owns the local heavy lane presently.

## Source and actual dispatch availability

Current local and live remote branch HEAD both equal
`6641041e457cbfa105432cc62fa676d5275d7a5f`, branch
`paudley/308-reduce-native-ci-test-compilation-and-c`. Read the sole plan,
validation, Task2 controller report and Task3 implementation/correction/review,
actual controller request/policy parsers and hosted operator. `rg --files
.github/workflows` identifies **ci.yaml**, not ci.yml. Read-only live GitHub
queries confirm repository `Blackcat-Informatics/purrdf`, default branch `main`,
active CI workflow ID305611062, path `.github/workflows/ci.yaml`.

Live main already has this workflow's `workflow_dispatch` trigger, with the older
`simd_projection` input. The pushed selected branch adds `native_profile`; it
does not require opening a PR or merging the experiment first. GitHub's
[manual workflow documentation](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow)
describes the existing default-branch trigger and `--ref` branch selection.
After root admission and a fresh branch identity check, the exact prepared
dispatch is:

```sh
gh workflow run ci.yaml --repo Blackcat-Informatics/purrdf \
  --ref paudley/308-reduce-native-ci-test-compilation-and-c \
  -f native_profile=true -f simd_projection=false
```

This command has NOT been issued. Capture the resulting actual run ID, attempt,
head SHA, submitted inputs, workflow identity and every job/artifact URL. Normal
mandatory CI jobs also run; this is not a profiling-only shortcut. Avoid another
push/dispatch on this branch while measuring: workflow concurrency cancels older
non-main runs. A cancelled campaign is unqualified, not a timing sample.

## Hosted complete matrix and admission

| Arm family | Exact cases | Current production command |
| --- | --- | --- |
| Native BEFORE | before-monolithic, before-capi, before-downstream | `make test`, `make capi-check`, existing locked preserve-order consumer Cargo test |
| Native AFTER | after-lib, after-doc, after-integration-1, after-integration-2, after-integration-3, after-integration-4, after-capi, after-downstream | Existing `make test-shard SHARD=...`, `make capi-check`, same consumer |
| Nested-profile BEFORE | profile-before-capi | Same `make capi-check`, source clone with ONLY debug nested `test`→`dev` profile selection |

Twelve cases each execute cold then unchanged warm: 24 actual command executions.
AFTER C is reused for the nested-profile comparison. Native partition comparison
requires all three BEFORE lanes and all eight AFTER lanes; actual combined
compiled target/features coverage must match. The monolithic baseline is current
source, not September's code. No narrowed selection is admissible.

Prerequisites are floating nightly with exact admitted `rustc -vV` and `cargo
-Vv`, Node24, pinned Binaryen via the existing action, cargo-c0.10.23 installed
outside measured directories, Python3, actual CC and ordinary Make/Git tools.
The header generator is identified by actual `cargo-capi --version`, not generic
`cargo capi --version` help. All actual native prerequisite gates stay active;
XML conformance acquisition/cache state is separately retained. Native arms do
not require a wasm target just to execute this native selection. Installed
Python-extension tests retain their ordinary ignored status, not an invented pass.

All optional arms use ubuntu-latest, a 360-minute bound and fail-fast=false.
Compiler admission runs once; every arm requires its exact observed identities
before controller construction. Controller lives in campaign/controller-target
AND controller-build, outside arm target/build. The hosted operator records
image OS/version, OS/architecture, actual CPU model/flags/topology/logical count,
physical memory, available parallelism and prerequisite versions. It sets both
Cargo jobs and libtest threads to min(actual available parallelism,8), retaining
the observation and resolved configuration. Do not call configured workflow
jobs8 equivalent to an eight-core runner: actual effective values are authoritative.

Cold requires empty private target **and** build directories; the controller
refuses existing content and deletes nothing. Arms reside on verified disk under
`/opt/purrdf-native-profile/RUN_ID-ATTEMPT/CASE`. No artifact/compiler cache action
runs in the experiment. Registry/download and uncontrolled page caches remain
separate declarations/observations; no cold-page-cache claim is made. Setup,
tool/controller install costs, runner disk reclamation and acquisition state
remain separate from monotonic actual command phases.

Matching rejects hardware/image/prerequisite/compiler/CC/environment/configuration/
flags/target/concurrency/cache or source differences, except the exact admitted
nested-profile edit and native partition. Hardware heterogeneity on hosted pools
can therefore refuse a campaign. Do not relax matching or cherry-pick successful
arms. A refused campaign requires diagnosing its actual reason and a fresh full
matched attempt. The operator also rejects failed/missing phases, missing timing
reports, changed retained identities and lost coverage. Every C-bearing comparison
requires all16 original C phases, including real header generation, two C object
compiles, links, executions and output/assertion validation.

Admission, twelve arm uploads and comparison uploads bind RUN_ID+ATTEMPT. Exact
admission download and arm-only current-attempt download cannot use earlier
attempts. `hosted-restore` validates the complete admitted case inventory, prefix,
directory types and absence of every destination before moving anything; it
preserves older attempts and never overwrites. Failed-jobs-only reruns without
current admission/arms require an actionable **FULL workflow rerun**. Uploads
retain failures, JSON receipts, HTML timings, identities and exact counterfactual
diffs; they intentionally omit target/build caches and launcher binaries.

The comparison controller runs `hosted-restore CAMPAIGN`, then `hosted-compare
CAMPAIGN`, producing native-partition and nested-profile policies/comparisons for
both cold and warm. Hash-bound paths are restored at their original absolute
location. Preserve all actual artifacts before their 30-day expiry. Comparison
success requires every actual arm and compiler admission job to succeed.

## Local bounded C attribution and freshness witnesses

Local jobs8 evidence supplements hosted full coverage; it cannot establish hosted
concurrency or six-runner critical path. Root should admit the focused local two-C-
arm comparison and subsequent invalidations first, not launch a competing local
12-arm workspace replication. Use a fresh owned disk root, e.g.
`/opt/purrdf-308-t4-20261008.UNIQUE`, with current and counterfactual source clones
at the exact campaign SHA. Root owns their creation. Never edit main or this
branch for a counterfactual. Preserve only the exact one-line profile diff in the
legacy clone; no unrelated tracked/untracked source may enter either clone.

Build/select the controller outside BOTH measured arm directories:

Important local admission correction: `command -v cargo` currently names
`/home/paudley/stage/root/bin/cargo`. Read-only inspection confirms this wrapper
unsets requested target/build paths, strips directory arguments, forces managed
slot roots and kache. It is **inadmissible** as the captured real Cargo for a
private cold experiment. Use the same installed SDK's actual active-toolchain
bin first on PATH, for controller construction, capture and all experiment
children, preserving normal hook policy unchanged:

```sh
task_sdk_bin=/home/paudley/stage/packages/rustup/active-toolchain/bin
export PATH="$task_sdk_bin:$PATH"
task_real_cargo="$task_sdk_bin/cargo"
```

The active symlink currently resolves to nightly-2026-09-14; read-only identities
are Cargo1.100.0-nightly commit7941be6fb and rustc1.100.0-nightly commit4b6d04e70,
LLVM23.1.1. This is an observed installed identity, not a new global dated pin.
Refresh/freeze resolved identities for the campaign. Requests must use this raw
Cargo invocation path, not the managed wrapper or a different `rustup which`
installation. Capture existing compiler-wrapper/cache configuration truthfully
and equally; raw Cargo does not itself prove absence of compiler caches. Verify
resolved effective target/build roots equal the requested private paths before
any measured work, and check cold emptiness there. A managed-slot redirection
invalidates the trial even if the requested empty directory still exists.
Hosted plain GitHub Cargo is unaffected by this local Stage wrapper.

```sh
CARGO_BUILD_JOBS=8 CARGO_TARGET_DIR="$campaign/controller-target" \
 CARGO_BUILD_BUILD_DIR="$campaign/controller-build" \
 "$task_real_cargo" build --locked --jobs 8 -p purrdf-capi --example native_ci_profile \
 --profile test --message-format=json-render-diagnostics
```

Select the executable from this actual Cargo JSON output, freeze/capture its
identity and captured real Cargo proxy invocation path. Do not choose an artifact
merely because a guessed filename exists. Record compiler/CC/Node/Python/Binaryen/
cargo-c versions, source/lock/config identities, filesystem, memory/swap/load,
available cores and cache/wrapper observations before admission. Use the same
environment and runner identity across paired requests.

The exact native request schema (no alternative selector) is:

```json
{
  "root": "/opt/OWNED/current-source",
  "directory": "/opt/OWNED/after-capi",
  "cargo": "/ABSOLUTE/CAPTURED/cargo",
  "lane": "capi",
  "warmth": "cold",
  "jobs": 8,
  "test_threads": 8,
  "runner_class": "LOCAL_OBSERVED_IMAGE_CPU_MEMORY_CORES_IDENTITY",
  "dependency_cache": "ACTUAL_IDENTICAL_REGISTRY_DOWNLOAD_OBSERVATION",
  "compiler_cache": "ACTUAL_IDENTICAL_WRAPPER_CACHE_OBSERVATION",
  "page_cache": "uncontrolled local OS page cache; artifact-cold only"
}
```

Instantiate absolute paths and actual observations, not the uppercase placeholders.
Run `CONTROLLER run after-cold-request.json`. Warm request changes ONLY warmth to
`warm` and adds `previous` pointing to
`after-capi/capi-cold-receipt.json`; invoke `CONTROLLER run after-warm-request.json`.
Use the corresponding legacy clone/private `profile-before-capi` directory for
before cold/warm. Capture explicit jobs8 in environment and effective build
configuration; controller sets both child Cargo jobs and libtest threads to8.
Reject overridden effective settings rather than inventing equivalence.

Cold/warm policies passed to `CONTROLLER compare POLICY.json` are:

```json
{
  "before": ["/opt/OWNED/profile-before-capi/capi-cold-receipt.json"],
  "after": ["/opt/OWNED/after-capi/capi-cold-receipt.json"],
  "change": "nested-profile",
  "output": "/opt/OWNED/nested-profile-cold-comparison.json"
}
```

For warm, substitute both warm receipt paths and a fresh output name. The
controller validates the exact c_smoke debug profile replacement and all other
source bytes, tools/config/coverage/runner/cache equivalence before reporting.

After these unchanged pairs, run freshness witnesses in the task-owned current
clone and retained AFTER target/build directories. They are invalidation trials,
**not** unchanged-warm comparison inputs. Keep the existing cdylib path present
and record its identity before every trial. Always execute the existing real
`c_smoke` harness, whose nested Cargo build remains freshness authority:

```sh
CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 \
 CARGO_TARGET_DIR="$campaign/after-capi/target" \
 CARGO_BUILD_BUILD_DIR="$campaign/after-capi/build" \
 PURRDF_C_PHASE_RECEIPT="$campaign/invalidation-source-c-phases.json" \
 "$task_real_cargo" test --locked --jobs 8 -p purrdf-capi --test c_smoke \
 --message-format=json-render-diagnostics -- --exact c_abi_smoke
```

Execute this from the owned current source clone after an explicitly captured
documentation-comment-only edit to `crates/rdf-capi/src/lib.rs`. It changes source
freshness without changing API/semantics. Preserve the exact diff and old/new
source identities; require relevant Cargo compiler-artifact fresh=false despite
the old library path. A byte-identical resulting library is allowed for a comment
edit, not evidence of a skipped Cargo build. Retain all16 successful C phases.
Restore that exact owned edit, then run a separate codegen invalidation with the
same command and a fresh config receipt, setting `RUSTFLAGS` to the captured
baseline plus `-Ccodegen-units=8` (if already8 choose16 and record why). Preserve
O3/assertions/overflow and every other flag. Capture effective configuration,
fresh=false/rebuild messages and fresh Cargo-selected artifact identity; restore
the environment afterward. Do not route changed source/config through the warm
operation, which correctly refuses them. For full Cargo timings use the already
qualified controller-as-cargo telemetry shim with captured real Cargo and a fresh
private telemetry directory; never bypass the actual C harness with a private
library-only freshness check. A final unchanged rerun may establish fresh=true
reuse but still must invoke Cargo and all C phases.

## Resource admission and numerical completion

Read-only host snapshot: /opt disk has322GiB free; RAM124GiB total/107GiB available;
swap153GiB total/57GiB used. These are transient, not reservations. Refresh before
execution. Propose one local eight-job process tree with a64GiB memory ceiling and
root-admitted disk/time budget. Use actual first-arm peak target/build sizes and
peak memory to admit the second arm; compilation size is not known from telemetry
fixtures. Do not assert all12 local full-workspace arms fit322GiB. Reserve headroom,
stop admission before exhaustion, preserve failed artifacts and never solve it
with shared cache deletion/RAM-backed targets. Hosted arms get separate runners;
their actual disk/memory admission can still fail and must remain visible. No
runtime duration estimate is justified by the historical32m40s/275.71s figures.

Required numerical report, all bound to matched actual receipts:

1. Cold and warm nested-profile BEFORE/AFTER actual-command and nested cdylib
   preparation nanoseconds, Cargo freshness/compiler-unit inventories/timing
   reports, with direct C object/link/runtime/validation phases shown separately.
   Demonstrate removed repeated compilation attributable to the dev→test fix.
2. Cold and warm native partition aggregate command work over complete BEFORE
   and AFTER coverage. Aggregate work can increase under partitioning: do not
   label that a reduction merely because individual shards finish sooner.
3. Per-lane durations, setup/cache observations and scheduling windows. Hosted
   first-start/last-finish spans include scheduling and uncertified cross-runner
   clock alignment; the controller explicitly forbids inferring a six-runner
   speedup from them. A sum or max of lane clocks is not certified wall critical
   path. State this limitation rather than fabricating a six-runner speed proof.
4. Actual source/config invalidation witnesses and mandatory real C/header/output
   acceptance. Failed/missing/cancelled/unmatched trials are not usable numbers.

Closure needs an actual measured reduction attributable to the merged profile
fix or a newly evidenced/qualified optimization; telemetry or historical timings
alone cannot satisfy criterion8. No arbitrary percentage is required. Report both
cold and warm values honestly; a noisy/non-reducing arm is not relabeled as saved
work. If no matched reduction is observed, inspect actual redundant work, implement
the smallest supported remedy and qualify it before completion. Full local gate,
hosted mandatory gates and independent completion/Stage integration remain later
requirements distinct from this prepared campaign. Root owns all dispatch,
commits, publication and final acceptance decisions.
