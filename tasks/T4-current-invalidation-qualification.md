# Current production and invalidation qualification

Status: PASS for the admitted local repair and real source/codegen invalidation acceptance. Fresh matched twelve-arm hosted campaign and full qualification remain NOT MET. No local speed claim is made.

Source is staged tree `a68fb1b716c5937af20b38b9930a69191d4cd6d4`, HEAD `221b1ace8`, ordinary main synchronization `5384882d6` still uncommitted. Metadata correction focused qualification passed29 tests, strict all-target CAPI clippy, controller rebuild and owning hygiene gates as recorded in T4-metadata-correction-qualification.md. Live source/index/refs were not modified during this campaign; final11-path readback and empty unstaged tracked diff pass.

Current controller is the actual Cargo-selected native_ci_profile, BLAKE3 `bbce03510c8bc4685979680e99e2ff905d91a3a37f8849a457c3db4f12dcf836`; full frame is T4-metadata-logs/controller-artifact.json. The root-prepared shipping/dev snapshots and exact requests are bound in T4-metadata-snapshots.md. Sequential production session77405 actually exited0: shipping0, shipping self-policy0, dev0, dev self-policy0, strict nested-profile pair0. Both C arms completed all16 real phases, including temporary header-child capture and exact runtime library selection. Self-policy change=none validates integrity and is not a performance comparison.

Before controlled mutations,18 baseline receipt/child/phase/copied-timing files were independently copied and hashed. Their original and frozen copies all remain byte-identical after the trials: T4-metadata-logs/baseline-final-readback.log. The inventory is `/opt/purrdf-308-current-telemetry.591np1/metadata-current/baseline-freeze/retained-evidence-inventory.json`. Original strict pair was rerun after restoration through the production comparator, actual exit0: policy-restored-nested-profile.json, restored-nested-profile-validation.json/.log. Original failure evidence and earlier ff72 snapshots remain preserved.

The shipping c_smoke executable was selected from the actual shipping arm Cargo frame, not recompiled or selected by filename guessing. Its manifest/source root is the exact shipping clone; full receipt T4-metadata-logs/shipping-harness-artifact.json, executable BLAKE3 `5f58eb6164c92574ae30560867180c4d402365a84c87cf957ca06cbe14df1e35`. Every trial directly invokes this same existing harness with --exact c_abi_smoke --nocapture, jobs/testthreads8, same owned Cargo target/build, verbose output and64GiB/Swap0 scoped resource limits. The existing library is present and hashed before each trial; no cache cleanup or outer Cargo prebuild conceals invalidation.

Actual neighbors session50808, source session43358, and codegen session15289 each exited0. All nine child calls exited0 with all16 C phases passing:

| Trial | CAPI artifact fresh |
|---|---|
| 01-baseline | true |
| 02-unchanged | true |
| 03-source-changed | false |
| 04-source-unchanged | true |
| 05-source-restored | false |
| 06-codegen-changed | false |
| 07-codegen-unchanged | true |
| 08-codegen-restored | true |
| 09-final-unchanged | true |

The only source mutation appends one harmless comment to owned clone crates/rdf-capi/src/lib.rs. Exact captured bytes were restored before codegen trials; full tracked source diff is empty. Trials06/07 set CARGO_PROFILE_TEST_CODEGEN_UNITS=8;08/09 restore original unset environment. Fresh=true on restoration is valid reuse of the original configuration artifact, rather than a required rebuild.

Effective codegen is supported by the real unit graph and actual nested rustc argv, not the profile field alone. unit-graph-baseline.json and unit-graph-codegen8.json retain77 units with CAPI O3/assertions/overflow unchanged, baseline nonincremental/no explicit codegen override versus override8. Active unit-graph schema omits rustflags; actual captured configuration/environment and nested commands were inspected instead. The exact actual CAPI cdylib invocation is retained at invalidations/06-codegen-changed-capi-rustc-argv.txt and contains exactly one -C codegen-units=8, with no later override. The broader argv file contains both umbrella and CAPI crates named purrdf; a read-only overly broad audit assertion was corrected by selecting CAPI source/cdylib, without changing or rerunning the trial. Source-changed/restored actual CAPI commands have no explicit codegen-units flag;08/09 are fresh and correctly have no recompilation argv. Original library identities recur at08/09.

Final audit logs are T4-metadata-logs/freshness-final-audit.log, restoration-final.log, baseline-final-readback.log and batch-terminal-receipt.txt. Complete trial commands/env/library-before identities/phases/stdout/stderr/diffs live in `/opt/purrdf-308-current-telemetry.591np1/metadata-current/invalidations`; actual effective argv03/05/06 and config-env06–09 are retained there. Earlier hosted37738739777 and37746157677 failures, initial strict-clippy failure and ff72 metadata comparison refusal remain failures. Prior actual doc production scope is reused because the metadata correction did not change the doc caller/collector; no blanket full suite or extra campaign was run.

All admitted workload children are terminal. Local build lane FREE. Root owns normal hook publication and a new matched hosted campaign; this local PASS does not establish numerical improvement, hosted acceptance, full qualification, or completed delivery.
