# Native CI compilation and C smoke: independent issue analysis

Evidence baseline: `6273b6173f3b3d98f49629a162ba88da8bc26477`. Read the generated issue, all comments (none), prior-art coverage, main-checkout `.baseline`, `.goals`, supplied AGENTS contract, current workflow, Makefile, profiles, C smoke and shard/profile gates. No forge retrieval, build, timing experiment, source edit, or performance claim was performed by this analysis.

## What already exists

The concrete redundant C build described in the issue is already repaired. Commit `65e110cb9654c8ad98978e6f6e47eed49bf2660b` changed the assertion-enabled C harness's nested build from `dev` to `test`, retaining optimized codegen while avoiding a second debug-info configuration. Current `crates/rdf-capi/tests/c_smoke.rs::c_abi_smoke` always invokes Cargo, takes the platform-specific cdylib path from that invocation's compiler-artifact messages, and does not authorize reuse on file existence. Cargo remains responsible for source/configuration freshness. The harness still compiles and links the committed header, executes the C smoke and projection example, checks nonempty output and Unix owner-only permissions, and uses the shared frozen entailment vectors.

The same commit split the native Rust suite into six parallel jobs: lib/bins, docs/examples, and four integration-target ranges. The C smoke alone is skipped in integration-1 and run by the dedicated capi job through `make capi-check`; it was relocated, not removed. `scripts/check-test-shards.py` checks target coverage and wiring. Every shard selects the workspace rather than an arbitrary package subset. The lib shard's excluded Python library has `test = false`; the target gate refuses a newly test-enabled excluded library. Dependencies are cached; first-party builds are not deliberately cached by the workflow's dependency cache.

The configured dev profile is O3 with debug assertions and overflow checks on; test inherits all three and changes debug info only. `make build-profile-hygiene` checks Cargo's resolved units, not just manifest spelling. The workspace job retains strict clippy, formatting and hygiene. These are existing constraints to preserve, not proposed reductions.

## Completeness contract

| Requirement | Present evidence | Required execution evidence |
|---|---|---|
| Profile before selecting a change | Historical issue contains one aggregate old build and C smoke duration; it explicitly lacks subphase attribution | Capture current-base cold/warm Cargo timing reports and distinguish compiler/build/link preparation from test execution; expose C nested-build, C compiler/link and C execution phases individually |
| Comparable cold/warm before and after | No paired records or phase artifact upload in current native/capi workflow | Same actual runner class, admitted compiler identity, flags, configured Cargo jobs, test concurrency, target selections and cache definition for each arm; retain source/tree and Cargo.lock identities; report every shard and capi, plus six-way wall critical path and aggregate work |
| Preserve complete Rust suite | Six-way partition and coverage gate already exist | Gate self-test and live target inventory; execute all six native shards (including doctests/examples), dedicated C smoke, downstream preserve-order consumer, strict lint/hygiene and resolved profile verification |
| Preserve O3/assertions/overflow | Inherited manifest values and resolved-unit gate exist | Resolved-unit profile check on both measured arms, including build overrides; no flags/profile settings that weaken checks to buy a timing win |
| Real C header/link/runtime | `make capi-check` checks generated header then runs `c_abi_smoke`; real C callers and projection example exist | Run actual capi entry point; capture nested Cargo outcome and cdylib identity, C compile/link outcome, both runtime outcomes and required output checks on both arms |
| Safe artifact reuse | Nested Cargo build plus compiler-artifact path exists | Preserve that freshness authority; warm rerun must report Cargo freshness, and a controlled changed-source/config test must demonstrate rebuilding or selecting a different valid artifact, rather than trusting a preexisting path |

## Smallest coherent remaining design

Instrument the existing entry points first, in Rust. A phase collector/driver should invoke the exact existing commands, record monotonic elapsed durations and exit statuses, retain Cargo timing output and compiler-artifact freshness metadata, and emit a fixed typed schema. It must not invent a parallel test-selection implementation or replace the real C caller with Rust-only calls. Add durable timing artifact uploads, including failure records, so the measurements are reviewable. C smoke instrumentation can time its existing nested Cargo invocation, each C compilation/link invocation and each execution without changing their correctness checks.

Cargo's usual aggregate timing report does not by itself prove link-only time. If a record combines rustc/codegen/linking, label it as such; obtain a supported finer-grained measurement before claiming a separately reduced link phase. Similarly the C compiler currently compiles and links in one command; either retain a truthful combined phase or explicitly split to object compilation and linkage while preserving flags and both statuses.

Define cold precisely: an empty task-owned artifact/build directory with the same preprovisioned dependencies and runner setup in both arms. Define warm precisely: rerun against that arm's unchanged sources/configuration without deleting its outputs. Distinguish dependency-cache restoration, Cargo artifact warmth, compiler-cache warmth and OS page-cache state. Never call a shared local `/opt` target cache a cold runner. Local eight-job results cannot be treated as comparable to hosted Cargo-default concurrency without an explicit identical setting in both hosted arms.

Only select additional build/profile/partition/artifact changes after this current-base attribution. The old dev-to-test fix and six-way split should not be implemented again. If historical improvement is part of the closing claim, compare the old issue head and repaired implementation with identical admitted tooling/harness and document the harness-only adaptation; the September aggregate and today's floating-nightly run are not a controlled before/after pair. If current instrumentation reveals no avoidable work, the issue's required reduction remains unmet until the owner accepts evidence that merged prior work already satisfies it; do not silently change the issue into instrumentation-only completion.

## Named acceptance entry points

- `make test-shard SHARD=lib`, `doc`, `integration-1`, `integration-2`, `integration-3`, `integration-4`: actual full Rust partition; retain configured SIMD-path checks and Node/Binaryen prerequisites.
- `make capi-check`: authoritative committed-header verification and `cargo test -p purrdf-capi --test c_smoke --locked`; nested `cargo build -p purrdf-capi --profile test --message-format=json-render-diagnostics` and actual C compiler/link/loader execution.
- `cargo test --manifest-path crates/jsonschema/tests/preserve_order_consumer/Cargo.toml --locked`: existing lib-job downstream unification check.
- `make build-profile-hygiene`, shard-gate self-test/live inventory, and settled `make check`: preserve complete local gate; do not run broad baseline discovery or duplicate full gates per task.
- Timing driver fixture tests: child failure must stay failure with recorded phase; malformed/missing artifact identity must fail; warm/fresh labels require recorded Cargo evidence; invalid comparison identities must be refused, not summarized as a speedup.

## Limits and decisions for the plan

The workflow does not set an explicit Cargo jobs value in either native or capi jobs, so configured runner concurrency currently means Cargo's effective default on the runner. Capture it or explicitly pin it consistently before paired measurement. Six jobs are six separate runners: sum of shard durations and elapsed critical-path wall time answer different questions. Preserve both. Header generation and cargo-c installation are capi setup phases that can dominate the job independently of C smoke, and need separate attribution.

No timing receipt is present in generated intake evidence. No current run was measured. Neither the issue's cold/warm acceptance nor a quantitative speedup is established by source inspection. The one performance campaign must be scheduled by the portfolio owner, with bounded task-owned directories and configured resource limits, after instrumentation is settled; this analysis has started none.
