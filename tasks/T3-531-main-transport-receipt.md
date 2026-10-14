# #531 evaluator source transport

- Donor: `470-v4-0-exact-tower-only-and-an-exact-or` at `330c55f8c`, with only its 11 dirty `crates/sparql-eval/src` paths exported relative to HEAD.
- Retained donor patch: `/home/paudley/Active/purrdf/.worktrees/470-v4-0-exact-tower-only-and-an-exact-or/.stage/v4-0-exact-tower-only-and-an-exact-or/tasks/T3-531-main-transport.patch`; SHA-256 `ad6f6133a233e2f05d8ec783a6cdf3a15864af2048846927ca279cfceb8f8383`.
- Recipient: main-based #531 checkout at `5144f5c1a7dc824f32165eca91d3c2a6df6226fb`. `git apply --check -p0`, application, and `git diff --check` passed. Only the same 11 evaluator source paths became dirty.
- Recipient compile: `CARGO_BUILD_JOBS=1 cargo check --locked -p purrdf-sparql-eval --lib`, terminal 0, `tasks/T3-531-main-transport-lib-check.log`; `Finished dev profile` in 38.87 s.
- Donor-only affected runtime before transport: `cargo test --locked -p purrdf-sparql-eval --lib exists_paths_agree`, 13/13 passed, donor `tasks/T3-531-exists-parity-runtime.log`. This is not recipient runtime or stack-depth qualification.
- Recipient affected runtime before shared prerequisites: the same command, 13/13 passed, `tasks/T3-531-main-exists-parity-runtime.log`, terminal 0. The test binary compiled from the main-based recipient in 11m11s.
- After that source-frozen test finished, `tasks/known-shared-prerequisites.patch` passed `git apply --check -p0`, applied, and left `git diff --check` clean. Its separate source/fixture corrections require their applicable current-head checks; the preceding 13/13 result does not cover them.
- Complete suspended EXISTS/LATERAL child driver, current-main runtime, original three optimized floors, full gates, commit and publication remain open.
