# Optimized-package stack boundary

Hosted job 116264778255 on 49295bd27 runs 458 optimized-package tests: 457 pass.
The sole failure is the deep OPTIONAL boundary control, which expects host
algebra-height admission specifically but receives the evaluator's typed stack
refusal. The original bisection already accepts either typed refusal while
finding the synchronous boundary; the final expectation contradicted that law.

The correction changes the existing test only. It captures the actual synchronous
typed stack refusal and requires the asynchronous lane to produce the identical
message. The deepest accepted query must still answer on both lanes, twice as
jobs, with original high-water bounds and shadow-stack restoration. The no-poll
expression-frame controls, healthy neighbor, repeated resumptions and canary
checks remain unchanged. No admission limit, stack region, production evaluator,
error mapper or expected result is weakened. Stale hard-coded frame measurements
are removed from the test comment, together with an unrelated duration diagnostic;
the poll/yield counts remain.

Standard make wasm-pkg completes successfully, including release compilation,
wasm-bindgen, the pinned optimizer, post-link region/poison validation and SIMD
opcode verification. On that actual artifact the focused named test passes once:
673128 polls and 457 yields. Durations printed by Node are not performance
acceptance. No broad unchanged package suite is repeated locally; hosted checks
will assess publication separately. The earlier hosted failure remains actual
red evidence; the focused local result is not represented as the whole package.

Receipts: review-stack-wasm-package.log, review-stack-package-build.log and
review-stack-package-focused.log. Normal commit/push results are separate.

The seven-configuration producer 38687235725 remains live on the unchanged Rust
and manifest source. This test-only correction does not invalidate its compiler
measurements. Terminal ARM, v3 and Neoverse shards fail only stale document
cells; their actual logs are retained. Final complete projection and replay,
current hosted checks, material review, integration and merge remain outstanding.
