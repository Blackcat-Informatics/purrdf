# Newly reached HNSW cross-architecture failure

## Current correction evidence — 8360fff43

Scoped shared-fixture correction PASS. Readback confirms all three current HNSW
test files are byte-identical to their qualified final603423c17 versions (`git
diff` over those exact paths is empty). The published four-file commit combines
this exact fixture correction with the independently reviewed retrieval links.
shared-hnsw-native.log records the original sequential5/5 and Rayon1/1 controls
passing with no ignored or filtered cases. Existing qualified i686 evidence
below remains applicable to the unchanged fixture and production homes; no new
local i686 execution is claimed. Actual corrected542 hosted cross-architecture
completion still needs its own result, including its previously unreached
suffix. No material current-source issue was identified and no build was rerun.

Diagnosis: the current542 branch lacks the already qualified shared HNSW MXCSR
fixture correction. No production guard defect is established. Source-only
review; no compile, target installation or source mutation performed.

Actual ci-cross-arch-corrected.log reaches i686 HNSW after the repaired core
distance37/37, kNN84 and kNN portable-answer3+4 controls pass. HNSW library77,
conformance5 and determinism8 pass. Its sequential float_environment target
then fails all5 cases at named refusal assertions (build, Exact::resolve and
worker exact search_batch). The target exits101; the subsequent Rayon and
remaining cross-architecture suffix are not qualified on this run. Prior
i58639/39 evidence cannot be substituted for whole current542 cross-arch PASS.

The sequential fixture's is_ftz_refusal and the Rayon fixture's duplicated is_ftz
both branch on x86_64 and otherwise expect Probe evidence. Both actual test
targets are restricted to x86_64 or x86 with SSE2. The production XSD
environment::register under those same cfgs calls mxcsr_register before the
arithmetic probe, yielding the named MXCSR refusal. Therefore the obsolete
i686 expectation rejects correctly refused operations. The failure log itself
prints assertion labels rather than the actual error payload; the cause is
established by this source path and the prior actual corrected-fixture runs,
not invented error output.

The qualified final shared correction is603423c17 across exactly:

- crates/hnsw/tests/float_environment.rs
- crates/hnsw/tests/float_environment_rayon.rs
- crates/hnsw/tests/support/fixture.rs

It moves the existing refusal predicates into the original shared fixture home,
reuses them in both targets and requires MXCSR Register evidence on their
entire cfg. All original consumer calls, refusal assertions, restored/default
neighbours and worker bit-result controls remain. It adds no second predicate
implementation, production behavior change or bypass.

Important transfer detail: this542 sequential file predates an earlier
sequential-only correction already present in603423c17's parent. Applying only
the603 commit patch without that context can miss its old Probe branch or
conflict. Compare the current three files with the final603423c17 versions and
transfer that exact bounded final state, including the corrected sequential
module prose. The current-to-final diff contains only the shared fixture
correction; do not copy unrelated source or claim an earlier partial fix is
equivalent.

Relevant HNSW production, XSD environment and core binary64 homes have no diff
between the qualified603 source and current542. Actual5392d01 cross-arch log
records the final shared fixture on i686: sequential5/5 (lines1275–1284) and
Rayon1/1 (1286–1291), with the original controls intact. That receipt is retained
in539 Stage/tasks/hosted-2d01-cross-arch.log and qualifies reuse of this exact
correction. Normal publication and actual corrected542 hosted checks still
need their own outcome; this diagnosis is not a green current CI claim or a
waiver of the previously unreached suffix.
