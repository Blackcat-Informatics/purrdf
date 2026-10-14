# #365 qualification readback while execution is paused

Read-only reconciliation of retained receipts on 2026-10-13. No source, build,
package, or test was changed or rerun for this review.

## Source and complete local gate

Branch `paudley/365-post-v3-0-asynchronous-wasm-lane-and` is at
`a73408ddf0adedddd704bbe989dcb98cfe16d3f2`; local HEAD and the recorded
origin branch agree, and the tracked worktree is clean. Remote `main` readback
is `5144f5c1a7dc824f32165eca91d3c2a6df6226fb`, the branch's existing
integration base.

The normal `make check` launched on that exact head with `CARGO_BUILD_JOBS=8`
and `RUST_TEST_THREADS=4` through the existing 16 GiB/no-swap scope returned
**terminal exit 0** from retained exec session `81733`. Its complete log is
`task4-final-make-check-a73408ddf.log`: format, strict all-target workspace
Clippy, generated/manual claim checks, hygiene, all workspace tests and
doctests, the external JSON Schema consumer, ring-fence and final `make wasm`
ran. The log ends `Finished release profile [optimized] target(s) in 11m 21s`,
which is the final recipe step; a search found no `test result: FAILED`,
`error:`, `failures:`, or `make: ***` markers. This is a **complete repository
gate pass**, not a complete optimized JavaScript package-suite pass. The
earlier corrected gate's two failed `Project`-only evaluator test targets are
preserved in `task4-final-make-check-corrected.log`; both were adapted to
accept the public `ProjectStar` variant in normally hooked, pushed commit
`a73408ddf`, then passed 10/10 and 16/16 in
`task4-projectstar-consumer-rerun2.log` before the complete gate replay.

The original high-u64 WASM identity fixture also passed on `a73408ddf` using
Cargo's default compiler directories and the script's original test-profile
flags, without invoking its private-directory launcher. The unique
`purrdf_wasm` `profile.test` artifact was verified as WASM; bindgen Node output,
TypeScript's original strict flags, and the unchanged Node fixture all exited
0. `task4-high-u64-{cargo.jsonl,build.log,path.txt,source-sha256.txt,bindgen.log,tsc.log,node.log,bindgen-sha256.txt}` retain the evidence. The Node
fixture proves dataset/generation identities above 2^53, adjacent exchange IDs,
invalid wrapping inputs and MAX exhaustion.

## Remaining complete-issue acceptance

The optimized package built from production source at `728d2d98d` (before
the test-only `a73408ddf` change) passes the named deadline, poison, deep
SERVICE term, stack-parity, Cloudflare, installed-package, typecheck and
Task 4 public-query controls recorded in `validation.md` and the task reviews.
Its *complete* Node suite is **450 passed, 2 failed, 0 skipped** in
`task4-package-full-node.log`: LATERAL answered through depth 248 against
the unchanged required floor 281 (historical build observed 282), and FILTER
NOT EXISTS through 142 against required 180 (historical 181). These are actual
answer/refusal changes in the optimized evaluator, not merely assertions that
refer to the old projection enum. The source cause and ownership relative to
`ProjectStar` have not been established; #531 owns that shared stack work and
has no qualified correction in these receipts. Accordingly the optimized
package acceptance and full #365 delivery remain **NOT MET** despite the
repository gate passing.

`ProjectStar` is a new public `GraphPattern` variant. The in-repository
production consumers and the two missed integration-test assertions now handle
both forms, but external exhaustive matches require adaptation. The issue
body describes its follow-ups as nonbreaking; that source-compatibility impact
needs an explicit final integration/review disposition rather than a claim of
unchanged downstream source compatibility. The scoped material review at
`reviews/task4-current-root-review.md` found no further projection defect at
`728d2d98d`, but it predates the test-only commit and is not a whole-issue
review. Generated and manual *mechanical* checks passed in `make check`;
final material claim review and current hosted checks remain unverified.

There is no #365 PR, `ghprsq` merge, issue closure, mirror readback or delivery
cleanup. With execution paused, the branch remains parked. On resumption, the
shortest complete path is to qualify a correction for the two real depth
floors at their shared source (or establish a valid issue-contract disposition),
rerun the invalidated optimized-package checks, settle the public enum impact
and final review, then publish, satisfy hosted checks, merge through `ghprsq`,
verify closure/mirror and remove the completed branch. No exclusive timing
work is required by the user's current instruction.
