# Old serial monolithic failure: retained computation

VERDICT: independent owning collector/admission defect, still present in40de8b040; NOT a consequence of the four ordinary-CI fixture failures. Source-only/read-only analysis; no build, source edit, rerun, forge/cancellation/dispatch or validator relaxation. 454 retains its lane. Original uploads/receipts/logs were read, never rewritten.

Exact run37790706268 attempt1 head639a72740c0df091216382f9996d6bcf03aaedbc, measured job113357254729. Before-monolithic step14:25:18–15:21:52Z, controller exit1; job endedFAILURE15:22:01Z. Retained profile.log1380 at15:21:51.9927469Z reports `child-time C library identity inventory missing`. Step/job failure is real, but the diagnostic must not be interpreted as failure of the measured native command or loss of its actual C runtime.

## Actual order and cause

`arm-before-monolithic/monolithic-cold-receipt.json` has completed `actual-native-command` success=true, exit_code0, elapsed3391663952891ns, then `cargo-telemetry-inventory` success=false with that exact error. No completed warm receipt exists in this failed case. Successful native-command timing is an observation of one failed-to-qualify cold arm, not campaign acceptance or an improvement claim.

The actual parent Cargo child `monolithic-cold-cargo/cargo-11584-1791469520074007723.json` records original `cargo test --workspace --locked`, success=true/exit0. Its real output includes full native_profile35PASS/0fail/0ignored (including all four old safety fixtures), and `native_lubm_seed_index_keep_u64_identity_before_acquisition ... ok`. Unlike ordinary integration-2's checkout scratch, the measured monolithic target is real `/opt`; those four old fixtures therefore pass here. The monolithic failure is independent of their ordinary-CI failure.

Thirty actual shim child receipts exist: one successful test, one metadata, twenty-two successful run, three failed run, one --version, one pkgid, one build. Exactly three lack context.capi_artifacts; all other27 have that context field. The three are the original first-party LUBM Make admission test's intentional invalid-configuration commands:

| Child receipt | Actual unchanged original/delegated command | Actual outcome |
|---|---|---|
| cargo-38186-1791471839957780953.json | cargo run --manifest-path SOURCE/Cargo.toml --locked --release --jobs8 -p purrdf-bench --bin lubm-check -- config 18446744073709551616 0 1 ... | exit1; invalid seed: number too large to fit in target type |
| cargo-38262-1791471840153704311.json | same command, config 0 18446744073709551615 1 ... | exit1; university count must be positive and the exclusive range end must fit u64 |
| cargo-38319-1791471840314040129.json | same command, config not-a-seed 0 1 ... | exit1; seed must be unsigned decimal |

All three retain their false child command phase, actual exit1/stdout/stderr. They contain no compiler-artifact JSON frame and no claimed C library. Their real release lubm-check command ran and correctly refused input; the original Make test asserts nonzero, exact actionable native-configuration message and no artifact acquisition, then passes. See `crates/bench/tests/make_bench_lanes.rs`1053–1097, especially the three invalid knobs1078–1081. These are intentional negative test subprocesses, not silently failed measured Cargo preparations or malformed library frames.

The actual `monolithic-cold-cargo/c-phases.json` separately has all16 phases success=true, real runtime library `/opt/purrdf-native-profile/37790706268-1/before-monolithic/target/debug/libpurrdf.so`,47216840bytes/BLAKE3 `10ca4aa0093f9d61f1324799157ad5f233be334bc0aaab5cc98504ce3b6f3840`. Two successful child-library captures are retained across the inventory; the actual parent workspace child has one. No missing-library or ambiguous-library diagnosis is supported by this failing computation.

## Current source applicability

`cargo_shim_at` records original/delegated arguments and the truthful native child phase. It captures capi_artifacts only when that command succeeds. Its existing measured-role classification recognizes build/test; these failed `run` commands are not telemetry-augmented and their delegated bytes equal originals.

`collect` indiscriminately visits all cargo-*.json and requires context.capi_artifacts before deciding any role. The first lexically sorted failed run child consequently produces the observed missing-inventory refusal. Merely writing an empty array on failure is NOT a complete correction: current `valid_receipt` also refuses every Cargo child containing any false phase, without separating a successful parent harness's intentional negative subprocess from required successful compiler/C proof. Their failures must remain visible; do not mark them successful or erase them from the global inventory.

Diff639a→40de changes profile.rs only at the configuration-absence fixture holder/comment; collect/shim/valid_receipt bodies are unchanged. Portability/provisioning repair does not repair this independent production collection/admission defect. New focused35 tests did not exercise a whole successful parent Cargo test containing a retained unsuccessful non-build Cargo run probe. Root must adjudicate a coherent typed child-role/provenance correction with a meaningful real original negative-probe production seam before dispatching another expensive full campaign. Preserve complete actual arguments/output/failure phase, required successful measured build/test identities/C capture strictness and parent native success; no failed required preparation may become accepted through a generic negative-child exemption.

Separate resource observation, not assigned as the missing-inventory cause: this old Request records jobs4/test_threads4, while these nested Make Cargo run probes explicitly request --jobs8 and delegate that exact argument unchanged. Source/proof of effective nested concurrency must be assessed against the declared campaign resource criterion before numerical acceptance. All values remain retained; this report neither normalizes them nor infers an effective peak from command arguments alone.

## Attributable retained evidence

Stage raw root `raw/T4-hosted-serial-37790706268-attempt1/`: profile-job.json, profile.log and actual download exit0; artifact11559958842.zip SHA256 `3db9b1bc81061c1453cf1bcf54ac9597af6aa465159a8ced20ca57f0e170a4ae`, extracted arm-before-monolithic; resources11559968915.zip SHA256 `1f6a79dd83fe6facf0c39987d9faa646816dadb230802964b415fb1e5b238e75`, extracted resources. Local archive digest readbacks match C's retained API digest identities. No ZIP/output was regenerated.

Actual hosted compiler resources: rustc1.101.0-nightly1d81eb4ad9cd207e3e638bd32b17ec4fce8412a6 (2026-10-07), LLVM23.1.3; Cargo1.101.0-nightlyf3865b2a4d1acc5276f6b3c67d0e057f4dab3928 (2026-09-29). This is not the local Sep14 portability compiler. Request binds ubuntu24 image20261004.327.1 and full hardware/prerequisite digest cohort; no heterogeneous substitution.

Compact exact parsed receipt inventory is `tasks/T4-old-monolithic-forensic-receipt.json`, including outer phase success/error/elapsed/exit, original+delegated negative-child argv/exit/stderr, parent actual35+Make-test PASS flags,16C-phase/library identity and archive SHA256 readbacks. It contains no replacement comparison or acceptance record. Complete original child/compiler/phase data remain at the immutable paths. Root owns old overall-run terminal and any subsequent repair/dispatch decision; fresh campaign/full Task5/numerical acceptance are NOT MET.

## Final old-attempt terminal (retained, not repaired)

Native captured terminal-run.json confirms run37790706268/attempt1/head639a72740 completed FAILURE, updated2026-10-08T15:27:56Z. comparison-job.json confirms job113389924146 completed FAILURE15:27:55Z. Its retained comparison refuses incomplete current-attempt evidence; a full rerun is required and prior-attempt arms cannot be borrowed. Comparison artifact11560656209.zip/API SHA256 `2d2c45a96f23b2d063740c3ecbac54f376f0401aa6513ed67b8a57ca40904955` remains preserved with restore-refusals.txt. This establishes a failed campaign, not performance acceptance. No old receipt was edited or accepted through the proposed correction.
