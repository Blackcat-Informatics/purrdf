# Contract 2 / Task4 production freshness coverage

VERDICT: BLOCKED — actual controlled source and codegen invalidation witnesses remain unverified.

This is a read-only coverage assessment and executable witness design, not a failed implementation finding or a new execution receipt. No Cargo, test, process-control, source, Git or forge action was performed. Applicable worktree AGENTS and main `.baseline`/`.goals`, the sole plan contracts1–8 and task reports govern. No governing ADR is cited by the authoritative plan. The local production doc/capi-test/capi-dev sequence70910 was still active when assessed; its eventual outcome must be recorded by its owner, never inferred here. External repository submissions remain excluded.

## Authoritative evidence and exact remaining requirement

| Requirement | Actual attributable evidence | Disposition |
|---|---|---|
| Cargo prepares the real selected cdylib on every smoke invocation, even when a library exists | `c_smoke.rs:28–46,66–126`: unconditional actual pkgid/build, exact package/cdylib selection, byte identity; historical actual `T1-c-smoke-receipt.json` and `T2-c-smoke-receipt.json` complete real C runs | Implemented and historically demonstrated. These do not establish current invalidation trials. |
| Valid package/target/profile/features/filenames/fresh metadata, hard refusal of malformed selection | `support/phases.rs:53–134`; Task1 actual four fixture controls and current28-test qualification | Demonstrated selection/refusal behavior; fixture metadata is not a production rebuild. |
| Actual unchanged preparation can be fresh while still running real C callers | Task1 receipt retained exact actual `fresh=true`, O3/assertions/overflow and16 successful phases; Task2 real smoke and delegation demonstration | Historical actual evidence. A current unchanged neighbor is still needed to bind the proposed trials. |
| Source change invalidates existing actual library | `plan.md` contract2 and Task4; `T4-preparation.md:199–229` explicitly prepared, not run; current `validation.md` explicitly lists source/config trials pending | **UNVERIFIED.** No retained actual before/edit/after Cargo preparation pair demonstrates this. Source-identity comparison fixtures refuse mismatches; they do not prove rebuild. |
| Effective codegen change invalidates existing actual library | Same authoritative acceptance; current actual effective configuration queries and fixture refusals | **UNVERIFIED.** Config query/parser tests and changed-pair refusal do not execute a changed build with an old library present. |
| Real header, object compilation, links, both C runtimes, projection validation remain successful after each change | `c_smoke.rs:129–260` retains actual programs, frozen inputs and owner-only output check | Source-confirmed implementation. Actual post-mutation execution remains required. |
| Matching full native campaign and attributable reduction | Earlier hosted37738739777/37746157677 were incomplete/failed; current focused28-test/profile/parity evidence is bounded | Separate Task4 acceptance, not cleared by these freshness witnesses. Task5 full qualification remains separate. |

Read reports: `T1-implementation.md` explicitly says no invalidation campaign ran; `T2-implementation.md` makes the same distinction; `T4-current-focused-qualification.md` reports the actual28 tests, selected controller and current11-input readback, but requires production seams and a fresh matched campaign. `T4-hosted-monitor.md` explicitly keeps invalidation pending. Current live three-lane qualification tests collector correctness and nested-profile counterfactual, not controlled source/codegen invalidation unless its owner separately executes and retains those trials. No existing accepted witness was located in these authoritative artifacts.

## Smallest reliable production witness

Use ONE new task-owned current-source Git clone/snapshot and ONE retained disk target/build pair under `/opt`, prepared by root after70910 settles and the heavy lane is admitted. Export the exact settled staged tree with the existing snapshot procedure in `T4-current-qualification-handoff.md`; retain real Git context and all source bytes. Do not use a historical committed checkout when the settled staged source differs. Keep controller/shim outside measured targets. Preserve existing scopes, eight Cargo jobs, compiler/wrapper/cache policy and actual SDK identities.

Build/select the **existing** `c_smoke` integration executable through actual Cargo JSON:

```sh
# cwd: owned current-source clone; all variables below resolved by root
CARGO_BUILD_JOBS=8 CARGO_TARGET_DIR="$task_target" \
CARGO_BUILD_BUILD_DIR="$task_build" \
"$task_real_cargo" test --locked --jobs 8 -p purrdf-capi \
  --test c_smoke --no-run --message-format=json-render-diagnostics
```

Select only the actual `compiler-artifact` for package `purrdf-capi`, test target `c_smoke`, non-null executable, admitted effective profile. Capture its complete frame, executable identity and compiled manifest root. This executable's `env!(CARGO_MANIFEST_DIR)` must point to that SAME mutable owned clone. Do not borrow the live-worktree harness or guess a hash-suffixed executable path.

Invoke that actual existing executable directly for each witness:

```sh
# SAME clone cwd; same target/build/env for baseline and source trial
CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=8 \
CARGO_TARGET_DIR="$task_target" CARGO_BUILD_BUILD_DIR="$task_build" \
PURRDF_PROFILE_CARGO="$task_real_cargo" \
PURRDF_C_PHASE_RECEIPT="$trial_receipt" \
"$task_c_smoke_executable" --exact c_abi_smoke --nocapture
```

This is the shipped existing integration harness, not a private alternate test: `c_smoke.rs:28–118` performs its actual nested Cargo build and `129–260` performs both complete C program paths. It isolates the freshness authority. Re-running outer `cargo test` after a mutation may rebuild the library before the harness starts; then nested `fresh=true` is legitimate and cannot alone prove this specific boundary. If outer Cargo is used, retain its real relevant `fresh=false` frame and prove the nested selected bytes belong to that preceding actual rebuild. The direct harness avoids that ambiguity without production changes.

1. **Baseline + unchanged neighbor:** run the real harness to create/select the library, then run it unchanged with a distinct receipt. Record old library path, actual bytes/identity and presence immediately before the second run. Require nested actual `fresh=true`, same library identity, both Cargo phases actually executed, all16 phases success, actual test exit0. No cache flushing/deletion.
2. **Source trial:** add exactly one documentation-comment line to owned `crates/rdf-capi/src/lib.rs`; save original bytes, exact diff and new identity. Keep the existing library at its captured path. Invoke the SAME existing harness, with a fresh receipt and otherwise unchanged environment. Require the nested actual CAPI cdylib frame `fresh=false`, same logical selected filename and correct package/profile/features, all16 phases success and test exit0. A byte-identical resulting library is valid for a comment-only change; identity change alone is not the rebuild proof. Run one unchanged source-mutated neighbor and require nested `fresh=true`. Restore only that owned line to original byte identity and capture restoration before the codegen trial.
3. **Codegen trial:** first run restored-source baseline to settle original configuration. Capture the effective baseline codegen-units. Change ONLY effective codegen-units to8, or16 if baseline already8. Prefer `CARGO_PROFILE_TEST_CODEGEN_UNITS=8` for the actual nested `--profile test` build: this variable is captured by `phases::context` and does not conflict with encoded-Rustflags precedence. Validate the actual resolved profile/unit graph under precisely this environment before drawing conclusions. Alternatively the previously prepared `RUSTFLAGS` suffix is admissible ONLY if actual flag precedence is established, including `CARGO_ENCODED_RUSTFLAGS` and target-specific configuration; never silently mask them or append a flag that Cargo ignores. Preserve O3, assertions, overflow and every unrelated compiler flag. Keep the old library path present; invoke the SAME harness with a fresh receipt and capture actual nested `fresh=false` plus actual selected library identity/profile. Require all16 phases/test exit0. Unchanged mutated-config neighbor requires `fresh=true`; restore original environment, settle original configuration, then final unchanged original neighbor requires `fresh=true`.

Codegen-units is a genuine compiler configuration change, unlike source comments. Cargo artifact profile JSON does not itself report codegen-units: retain actual effective unit graph/profile/config evidence in addition to the receipt environment and fresh metadata. Do not require a changed library digest as a proxy for changed codegen; optimization may produce equal bytes.

## Capture, preservation and acceptance

For EVERY baseline/change/neighbor/restoration invocation retain exact cwd/argv/environment, actual terminal status, stdout/stderr, distinct phase receipt, selected original Cargo frame, effective profile/config and source/lock/header/tool identities. Record old library presence and identity before each changed call and selected new bytes afterward. The receipt contains nested Cargo stdout, so rebuild frames remain attributable to the actual `cargo-cdylib-preparation` child (`phases.rs:235–271`). Require the16 original successful phases and actual C program/projection outputs; no ignored/skipped real smoke. Retain codegen/source changes outside unchanged-arm comparison inputs.

Optional timings use the existing actual controller executable named `cargo`, with `PURRDF_PROFILE_REAL_CARGO` and `PURRDF_PROFILE_TELEMETRY` bound to fresh owned directories (`profile.rs:490–568`). This shim only adds supported telemetry flags and retains actual delegated output; no synthetic library/frame/freshness receipt. Timings are not required to prove these two invalidation laws, and are not a measured reduction claim.

Inventory the owned snapshot, target/build, library and all receipts before/after; preserve failures verbatim. Restore source byte-for-byte and environment before final readback. Leave live worktree/index, sibling workloads, global caches, compiler wrappers and existing campaign roots untouched. Do not delete the existing library or clean target/build between mutation and preparation: that would prove cold rebuild, not invalidation despite existence. The task-owned clone can be retained for archive and removed only after evidence ownership/capture permits it.

No changed-source/config request may pass the controller's unchanged-warm admission (`profile.rs:365–376`), and no comparison policy is weakened. These are separate production freshness trials, followed by unchanged positive neighbors. Their successful execution would discharge these two specific contract2/Task4 gaps; it would not supply the separate twelve-arm numerical comparison, full native coverage/reduction, settled whole gate or PR publication.

## Updated applicability: reuse the newly settled shipping arm

The owner subsequently supplied actual70910 terminal evidence: production doc/capi-test/capi-dev each exit0, their individual validators each exit0, **nested-profile paired validation exit1**. `T4-current-logs/production-nested-profile-inventory-delta.json` contains exactly `/build_directory`: `/opt/.cargo/55/66b80cb52bee1e` versus `/opt/.cargo/d4/e25fed445f959c`; the retained paired log says `comparison mismatch /context/identity/target_inventory`. Current `profile.rs:338–347` queries metadata with cwd but lacks the jobs/target/build overrides supplied to native children and the configuration query at310–313. This is an actual collection identity defect; the strict comparison at1058–1066 correctly refuses it. Removing `build_directory` or weakening the matcher would hide the defect.

After the owner fixes and qualifies metadata's actual child environment, root can materialize fresh shipping and exact nested-dev source clones from the **updated** settled staged tree and execute the two actual C arms. The unchanged doc collector result may be reused with an explicit affected-source applicability judgment; it does not become a corrected paired C result. Original70910 receipts/failure remain untouched history.

**A third clone or extra no-run compilation is unnecessary** if the new shipping C arm already compiled the actual `c_smoke` test-profile executable from that shipping clone. Select its existing exact package/test target/executable from the retained actual Cargo inventory and verify byte identity/readback and compiled manifest root. The prior direct-harness commands then use that shipping clone, its already populated owned target/build and that SAME executable. Do not use the nested-dev harness: its changed nested profile would test a different production boundary. Do not use a live-worktree harness merely by changing cwd.

Reuse is valid only under these preservation conditions:

1. Settle and validate the exact new shipping/dev comparison **before** mutations. Preserve the immutable source tree/diff, source inventories, both complete receipts, original child receipts, copied timing HTML and library/header identities. Keep both comparison source snapshots attributable to their captured state; the dev clone remains untouched.
2. Store invalidation receipts/logs/telemetry in distinct new paths. The direct harness must not overwrite the shipping arm's original `c-phases.json`, child receipt or copied timing evidence. Do not invoke a controller warm request with changed source/config. Existing Cargo target/build contents and runtime library may evolve because that is the phenomenon tested; record old library presence and actual identity immediately before each change.
3. Source changes remain exactly one harmless owned shipping CAPI documentation comment. Restore exact source bytes and verify the complete baseline source inventory between trials and finally. Config changes are invocation-scoped, proven effective, and restored without editing global/user configuration or originals. Preserve original effective-profile evidence separately from changed-profile evidence.
4. Keep baseline artifacts' content-hash inventory unchanged. `profile.rs:895–966` rechecks retained child receipts, copied timing artifacts and original C phase receipt bytes; it validates captured child-time library identities rather than pretending the live target library must remain unchanged forever. A trial may overwrite Cargo's live library/timing-latest path, but never the baseline copies referenced by the comparison. If any referenced retained timing path is a mutable Cargo path, freeze it through the existing collector **before** running trials, or use separate targets; never rewrite receipt identities to accommodate changed evidence.
5. Re-run the unchanged existing strict comparison validator on its original policy after final restoration and preserve actual terminal status, plus readbacks of all original evidence hashes. This checks preservation; it is not a new timed comparison. A fresh source inventory and final unchanged positive neighbor must match the captured shipping source/config. Failures stay explicit.

This reuse directly satisfies contract2's existing-path requirement with fewer builds while preserving the original comparison proof. All witness trials are still **NOT RUN** at this review's close; no forthcoming metadata correction, paired comparison or restoration is presumed successful.
