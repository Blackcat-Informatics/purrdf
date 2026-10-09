# Native CI and C artifact phase evidence

## Authority, prior work and governing constraints

The user approved the portfolio and requested "Implement the plan." This is its
early independent CI delivery. Worktree/branch and generated evidence come from
open.json; main remains protected. AGENTS.md, main .baseline/.goals and Stage
rules govern. No cited ADR or constitution was found. Rust tooling, no new
semantic features or shipping dependencies, deterministic library behavior,
normal hooks/signing and ghprsq-only final integration remain binding.

Commit65e110cb9 already merged six native workspace shards, shared dependency
caching, removal of duplicate Csmoke execution and the nested dev-to-test profile
fix. Do not implement them again. Current Csmoke always asks Cargo to build and
selects its cdylib from Cargo JSON. The historical32m40s job/26m2s build/275.71s
Ctest numbers are not a controlled comparison against today's source/compiler.
The generated intake covers the body, zero comments,14related items and200
commits' trailers; one performance trailer is a bounded census, not exhaustive.
Independent issue/prior-art analyses identify missing comparable phase receipts.

## Design and complete acceptance contract

1. Instrument existing real C header/link/run paths in Rust. Split the two C
   programs' current combined compiler/link invocations into equivalent object
   compilation and linkage. Record each Cargo preparation, C compilation, link,
   runtime and output-validation phase with elapsed duration and exit status.
   Retain all original assertions, flags, examples, vectors and permission checks.
   A failed child or receipt write is a hard failure, not a success with missing
   telemetry. Add --locked to the nested Cargo invocation.
2. Keep Cargo as freshness authority on EVERY run. Retain the selected artifact's
   package/target/profile/features/filenames/fresh data, actual binary/header
   identities, Rust/Cargo/CC identities, lock/source state, target and relevant
   configuration. Artifact existence never authorizes skipping preparation.
   Negative fixtures reject unrelated/missing/malformed cdylib messages. Warm
   reruns record Cargo freshness; controlled task-owned source and codegen-config
   mutations demonstrate actual invalidation despite an existing library path.
3. Use one shared Rust receipt/process helper for Csmoke and a host-only profiling
   example in the same crate, using existing dev lexical/hash dependencies.
   Controller builds live outside measured arm target directories. Do not add a
   new shipping dependency or private alternate C test. The example invokes the
   existing Makefile/shard commands rather than reproducing target selection.
   If a Cargo shim is needed for timing/message capture, it delegates exact argv
   to the captured actual Cargo and changes only telemetry flags on supported
   build/test commands; metadata/header/lint commands retain their behavior.
4. Capture Cargo timing output and compiler-artifact inventories for real native
   commands. Rust unit time includes code generation/linking: label it combined,
   never infer pure link time by subtracting unrelated clocks. Separate C compile
   and link costs provide direct attribution. Preserve full six-shard inventory,
   dedicated C lane, downstream preserve-order consumer, strict lint/hygiene,
   O3, debug assertions and overflow checks. Run resolved profile and shard gates.
5. Comparable cold/warm BEFORE/AFTER uses the same current source and admitted
   toolchain/flags/target/runner class/effective Cargo and libtest concurrency,
   except precisely recorded already-merged optimization counterfactuals. Cold
   means empty task-owned Cargo build artifacts; warm reruns unchanged arm data.
   Record dependency, compiler-cache and page-cache state separately. No shared
   clean, shared cache deletion or RAM-backed target directories. Local jobs8
   results are bounded local evidence, not hosted-concurrency equivalence.
6. Qualify merged improvement rather than demand an unrelated new optimization:
   a task-owned comparison tree changes ONLY nested test to dev for the narrow
   repeated-compilation counterfactual. Also retain comparable native compilation
   receipts for monolithic current-source selection versus existing six-shard
   selection, with real full coverage and dedicated C accounting. Record exact
   counterfactual diffs; never commit legacy production behavior or compare old
   September code on today's compiler as though inputs matched. Distinguish C
   profile savings, native aggregate work and six-runner critical-path savings.
7. Hosted capture extends the existing dispatchable CI workflow with an optional
   profiling input and failure-preserving uploads; normal mandatory jobs remain.
   All compared jobs record actual effective concurrency and compiler identity;
   comparison rejects mismatched inputs. No global dated toolchain pin is added.
   All cold/warm arms complete actual C header/link/runtime and native target
   checks. Setup/cache restore costs are retained separately from measured phases.
8. Select further build/artifact optimization only after attribution identifies
   redundant work. A closure claim requires a measured reduction attributable to
   the merged fix or an evidenced additional fix; telemetry alone is insufficient.
   No fixed arbitrary speedup percentage, compiler/lint weakening or target loss.

The one performance campaign is serialized by the portfolio owner. Host data is
kept in task-owned disk directories under/opt with bounded build concurrency;
hosted runs are explicitly distinguished. No measurements have run yet.

## Task 1: Shared receipts and truthful C phases

Implement the shared host/test helper and instrument real Csmoke with explicit
object compilation/link/run phases and strict Cargo artifact selection/metadata.
Preserve actual header/vector/projection/permission checks. Add parser/failure
fixtures and run focused Ctest/helper checks and package warning checks. Independent
task review, normal commit/push and issue update. No broad discovery gate.

## Task 2: Reusable profiling controller and comparison validity

Add the Rust host example/controller over existing commands. Capture monotonic
phase receipts, identities, config/concurrency and Cargo timings. Typed comparison
validation rejects mismatched identities/caches/phase selections and failed
children; it never emits a successful speedup for missing data. Controller itself
is built outside cold arm targets. Add focused fixtures, independent review,
normal commit/push/update.

## Task 3: Hosted capture with full gate parity

Wire optional existing CI dispatch input, all native/C phase artifacts (including
failure records), comparison arms and complete target coverage. Normal jobs and
strict compiler/profile/hygiene contracts remain. Run shard/profile gate self-tests
and live inventory; independent review, normal commit/push/update. Dispatch uses
the existing workflow; no PR is opened before required acceptance is met.

## Task 4: Execute comparable campaign and evidence-led reduction

Run cold/warm arms under captured matching runner concurrency, retaining exact
source/counterfactual identities and artifacts. Execute genuine C header/link/run,
source/config invalidation witnesses and full native selections. Produce numerical
phase comparisons, aggregate work and critical path with limits stated. If data
does not demonstrate the existing improvement, inspect actual redundant work and
implement/qualify the smallest supported fix; no instrumentation-only closure.
Independent performance/task review, normal commit/push/update as appropriate.

## Task 5: Settled qualification, completion review and PR

One settled full local qualification plus qualifying hosted gates, independent
completion audit against every criterion and current receipts. Missing behavior
or unmatched measurements block PR. Publish the plan/current results and create
the Stage PR only when the full contract is met, then Stage2/3 feedback/integration
through ghprsq. Preserve selected Stage evidence and clean only this delivery
after verified merge. No release or registry publication.

## Review and progress

Independent plan review PASS (plan-review.md). Task1 passed focused checks and
independent review, normal hooks, commit/push3d0f398e1. Task2 controller completed:
13focused tests/real C smoke/clippy/shard gate/executable seam passed; independent
review PASS and normal hooks passed,eb875cb8c committed/pushed. Task3 hosted
wiring and current-attempt correction passed22 focused tests/clippy and independent
re-review; normal hooks passed and6641041e4 committed/pushed. Task4 actual
matched campaign and attributable reduction is next; no campaign has run.
The controller example is explicitly test=false, preserving the existing target
gate. Effective concurrency includes captured Cargo configuration overrides.
The monolithic baseline receives admitted time/disk bounds; cancellation cannot
qualify a comparison. Partition/runner-count differences are declared experimental
changes and require identical combined target coverage and per-runner settings.
