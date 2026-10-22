# Signed maintenance acceptance

PR #681 is open at commit 55e3b0527, committed and pushed with normal hooks
(sessions 27126 and 67341 both exited 0). Parent issue #493 remains open.

Independent review confirmed stale seal preparation after refused insertion and
row removal. The public regression reproduced the exact pinned-shape panic
(session 89277, exit 101), then passed after correction (session 41212, exit 0).
The first attempt failed to compile because of test-fixture mistakes, corrected
before the actual reproducer executed.

Current correction qualification:
- raw/review-seal-retirement-native.log: session 49757, exit 0; exact frame
  retirement across paused and ready preparation, with absent removal retaining work.
- raw/review-seal-native.log: session 76012, exit 0; all 467 library and 49 runtime
  cases, including generated histories and every preparation/transaction work cut.
- raw/review-seal-public-wasm.log: session 24079, exit 0; all 49 runtime and
  16 public refutation cases through the actual shipped WASM runner.
- raw/review-seal-allocation.log: session 36105, exit 0; all five allocation cases.
- raw/review-seal-strict.log: session 87668, exit 0; affected production consumers,
  all targets, with warnings denied.
- raw/review-seal-helpers.log: session 83517, exit 0; all helper/domain checks.
  Formatting session 75551 also exited 0.

The test dictionary uses an atomic refusal flag so the public immutable
dictionary borrow can switch the test failure without widening production access.
Five native assembly jobs found only chase documentation drift. The existing
writer projected their actual CI observations at commit 9a695a072: x86_64 has
3 vector instructions, v3 has 12, v4 has 4, AArch64 has 1, and Neoverse has 1;
all have zero fused or relaxed instructions. All corresponding corrected-head
assembly jobs and their combined check have now passed.

Review response 6349554994 links the generated-program acceptance control.
review-feedback.md and pr-comments.md record complete paginated feedback and the
independent confirmation of the fix. The sole thread is resolved. Updated CI
has 51 successful checks and four configured skips, with no pending or failed
check. RISC-V multiworker retry 120060497589 passed at 2026-10-22T20:29:33Z;
the original attempt exhausted the existing 30 minute limit after lengthy setup.
No check was waived or weakened. ghprsq, archival and cleanup are pending.
The earlier qualification below is reused only within its documented scope.

All Cargo commands use the locked toolchain/profiles, CARGO_BUILD_JOBS=8,
RUST_TEST_THREADS=4 and RAYON_NUM_THREADS=4. One owned Cargo command runs at a
time, including hooks. No timing acceptance, hook bypass or profile weakening.

| Criterion | Executed receipt and scope |
|---|---|
| Native model, proof, source, governors and storage | raw/retained-scan-final-native.log, session 6895 exit 0: 466 Datalog library tests, 47 public runtime cases, 5 allocation cases, 818 entailment library tests (1 existing ignored), 16 actual public refutation cases. Includes current grouped native guards, hard source/guard poisoning, canonical witnesses, original row identities and all 26,945 preparation / 86,246 transaction work cuts. |
| Vacant physical row and partition progress | raw/retained-scan-regressions-native-fixed.log, session 61352 exit 0: 2 new tests. A 257-slot dense log leaves only original row 128 live; denial and every single-slot resumption preserve the cursor, source, final bytes and coordinates. A 257-retired-partition control charges each object-preparation slot before advancing with unchanged admitted bytes. All 466 library tests also execute both controls. |
| Public bootstrap ownership and portability | raw/retained-scan-public-first-slot-native.log, session 92276 exit 0: 1 new public case (47 filtered). It exercises new_in with 257 original rows/partitions, zero credit needing exactly 1 unit for the first vacant slot, every next needed allowance, withheld facts, equal uninterrupted/resumed work, original row 128/next 257 and no fabricated watermark or initial ModelDelta. Native public coverage is 47 above plus this case. |
| Actual WASM runtime | raw/retained-scan-final-public-wasm.log, session 14219 exit 0: all 48 Datalog runtime and 16 public refutation cases through the shipped harness=false Node/wasm-bindgen runner. Includes the new public bootstrap case, grouped native negation/filter proofs, original source failure and work/admission boundaries. |
| Strict affected consumers | raw/retained-scan-strict.log, session 28670 exit 0:  cargo clippy --locked -p purrdf-datalog -p purrdf-entail -p purrdf-shapes -p purrdf-validate --all-targets -- -D warnings. |
| Formatting | raw/retained-scan-format.log, session 32985 exit 0:  cargo fmt --all --check. |
| Current helper census | raw/retained-scan-helpers.log, session 34550 exit 0: 85 enforced jobs, 120 distinct groups, 2469 files, 145 unique prefix-free hash domains; all self-tests pass. |
| Full local gate and release portability | raw/retained-initial-make-check.log, session 89300 exit 0. Workspace fmt/strict/check/hygiene/generated/docs and workspace native tests plus standalone JSON-schema consumer passed before the final two scan corrections. Kernel hygiene and the complete release WASM build finished afterward on corrected production source. Current native/portable/strict checks above requalify the changed closure; this is not a blanket old native-suite pass on the changed tree. |
| Large source, compaction and churn | raw/retained-initial-scale-growth-native.log, session 84984 exit 0, all 3 explicitly selected ignored cases. At 1,000 / 1,000,000 edges insertion takes 3 joins/49 allocations/11908 requested/332 retained/3394 peak bytes; deletion 1 join/33 allocations/4768 requested/-56 retained/1758 peak bytes, identical across sizes. Four churn blocks retain 0. Forced 1133504-row compaction admits 99615793 work/9 allocations/46665200 requested and peak/1932 retained bytes, equal uninterrupted and 55 paused execution. |
| Delivery | Commit 55e3b0527 and normal push passed. All current hosted checks and independent fix review pass; paginated feedback is complete and the sole thread is resolved. ghprsq, archive verification and cleanup remain pending. |

## Coverage and reuse

Initialization retains metadata, asserted weights/reference capture, directory,
object permutations and native execution; no host watermark is invented.
Compiled dependency analysis, levels and program identity are borrowed. Existing
sealed buffers retain their owners; supplemental buffers use the supplied grant.
Sealing retains collection/sort/merge/permutation work before the incoming
dictionary transaction and pre-admits its atomic publication suffix. Resource
refusal retains progress; source/guard failures hard-poison the original owner.

The independent checker re-decides native filters and grouped negation, rejecting
a complete blocking extension. Caller callback meaning remains an explicit
GuardedRule refusal by this existing lexical checker; never an assumed success.

The final production diff after the broad gate started only corrects bootstrap
dense-slot copying and partition/batch header progress. Current library/runtime,
portable and strict receipts above cover those changes. The subsequent public
test addition changes no production input to the completed release build.

Scale reuse is bounded to its actual measured update/compaction/churn paths.
maintained_scale completes initialization before opening update allocation
windows. Forced compaction measures the final update after initialization and 63
acknowledged inputs. Those kernels/windows are unchanged by the scan correction.
New controls prove bootstrap cursor/work and final metadata equivalence. The
earlier 3-case scale receipt is reused, not represented as a fresh execution.
Compaction is governed large work, not constant-cost work.

## Failed receipts and dispositions

retained-initial-library.log exposed the cold delta caller missing the new seal
preparation, accidental resident-test ownership conversion and pause-only index
retirement. The caller now uses the same seal kernel, resident test setup retains
its owner and retirement flushes at the same phase boundary. Current 466/48
native/public coverage (split as above) and 64 portable cases qualify these paths.

The first scale run used a wrong power-of-two compaction fixture and its
million-edge case was explicitly interrupted after source/profiling diagnosed
exact-capacity event spill growth. Overall exit 101 is not a pass. The corrected
fixture uses the original twice-span boundary; task reservation uses the shared
geometric growth law. The complete 3-case scale run and the 4096-event library
control pass. The verified lesson is retained in repository bank purrdf,
document native-maintenance-initial-event-growth-20261022, completed.

retained-scan-regressions-native.log exited 101: two new triple fixtures used the
quad-only new() constructor. Both use the existing with_dictionary home now;
the corrected 2-case and complete 466-case runs pass.

retained-scan-regressions-wasm.log, session 17001 exit 101, is an unqualified
ordinary-libtest invocation. The shipped runner explicitly requires harness=false
targets; it did not apply the requested library filter and trapped outside a
registered case. No runner workaround, production fallback or exclusion was
introduced. The new public bootstrap case is registered in the supported target;
all 48+16 portable cases execute and pass. Internal library controls remain native
coverage. Earlier compile/style/grouped-proof failures remain raw and are
superseded only by the matching successful receipts, never relabelled passes.
