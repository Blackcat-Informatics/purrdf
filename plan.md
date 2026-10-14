# #365: asynchronous host and nested representation follow-ups

Issue: https://github.com/Blackcat-Informatics/purrdf/issues/365. Base:
`origin/main`. Branch: `paudley/365-post-v3-0-asynchronous-wasm-lane-and`.
This is one complete issue contract; no native-only or JS-only closure. The
current captured body and comment are in `issue.md`, mapped in
`issue-analysis.md`. `prior-art-assessment.md` distinguishes current source
from historical proposals. No `.goals`, `.baseline`, `CONSTITUTION.md` or
governing ADR exists in this checkout; `AGENTS.md` applies. Main is protected.

The user's portfolio instruction removes **exclusive timing work**. Use fixed
work instruction counts and nonexclusive elapsed/resource observations with
conditions stated. Never call a timing result exclusive. Hooks are never
bypassed. A complete issue requires final native, optimized WASM-package,
Cloudflare, scale, review, hosted-check, and `ghprsq` integration evidence.

## Prior art and design decisions

Current `purrdf_jspi.mjs::expiry` already uses a referenced timer and clears it;
the child-process hung/success/failure/cancel fixture protects both effect
kinds. Current `QuotedTerm` shares a flat `Arc<Vec<QuotedNode>>`; its Rust and
packaged JS deep-term fixtures supersede the older patch quoted in the issue
comment. Verify both on a rebuilt package; do not replace them merely because
the issue predates their landing. Source inspection still finds an uncoded
poison error, lane-dependent stack budget and message, evaluator-stack HTTP
500, a per-poll deep test, eager nested star projections, and repeated GTS
identity expansion. The Stage trailer census counts two performance and two
semantic-identity-loss classifications in its last 200 commits, not four
recurrences of #365; issue-specific recurrence is unknown.

Choose the issue's permitted HTTP alternative: map typed evaluation-stack
refusal to 400 at the shared protocol/Cloudflare error boundary. Keep the
evaluator's typed diagnostic, and preserve 500 for real execution failures.
For `SELECT *`, retain semantics and either represent each star compactly or
admit projected size *before* constructing repeated lists; decide from the
parser's real scope and allocation paths. For GTS, reuse the existing canonical
CBOR key grammar; memoization cannot change cycle/missing-ID keys or sort order.
Declined: a second deadline watchdog, the historical boxed-term patch, relaxing
stack assertions, rewriting frozen GTS vectors, or a new external dependency.

## Completeness contract

| Required behavior | Task | Executable acceptance on shipped entry |
| --- | --- | --- |
| Hung SERVICE/LOAD deadline keeps idle Node alive, releases timers, shared siblings survive | 6 | Rebuilt `make wasm-pkg` artifact; `async-term-effects.test.mjs` child fixture for both effects and all four outcomes plus siblings exits 0 with zero timers. |
| Every package poison error carries stable `error.code` | 1 | Actual panic/trap child fixtures assert code on faulting/in-flight and later async/ready calls; later sync calls retain poison gate. |
| Sync/async deepest answer, first refusal and wording agree from shared operation entry | 2 | Package bisection for OPTIONAL, EXISTS, GRAPH, MINUS, LATERAL, SERVICE yields identical endpoints, code and statement on both lanes; a later shallow query answers; fixed-work synchronous cost comparison recorded. |
| Deep evaluator-stack refusal receives HTTP 400 | 3 | Cloudflare `fetch` for all six shapes: deep neighbor gets 400 and typed code, shallow neighbor answers; genuine evaluation fault remains 500. README/types agree. |
| Deep async no-poll-frame proof is materially cheaper | 2 | Keep actual synchronous/async deep answer, region high-water and no-poison assertions; lower timer-yield count by at least 10× on the same required proof, recording old/new counts and nonexclusive elapsed. |
| Nested `SELECT *` avoids quadratic materialization | 4 | Public native parser and optimized WASM query entry on fresh-variable nested controls through 8,000 levels; accepted neighbors preserve variable order and round-trip/evaluator answers. Count projection variable visits/copies and allocated bytes at doubling depths: accepted growth must stay below 3× per doubling, or a typed projected-size refusal must happen before repeated lists, with pre-refusal visits/copies at most 8× input variable occurrences plus a fixed documented setup bound. Record cap and peak bytes. |
| GTS canonical identity avoids repeated deep expansion/path scan | 5 | Actual remap/write on growing chains and shared DAGs: stored identity nodes/bytes linear in source terms, edges and flat payload; no linear path-membership scan; counted component visits and identity bytes grow below 3× per doubling. Cycle/missing-ID parity, frozen vectors and readback remain byte-identical. |
| 20,000-deep remote SERVICE term crosses WASM/JS boundary safely | 6 | Rebuilt package `deep-service-term-child.mjs` both lanes, getters/clone/equality/drop and later query; Rust 100,000-level small-stack and malformed predicate/scope controls. |

## Task 1: Stable poison code on the real package error

Assign one code at `purrdf_jspi.mjs::poisonError`, not a second wrapper. Extend
existing child-process panic/trap tests to check that code on each exposed
package error and gate behavior. Run focused Node tests on the optimized package
and affected Rust/JS checks; independently review the actual callers. Commit
and push this coherent fix, then post its exact result to #365.

## Task 2: One stack endpoint and affordable deep proof

Diagnose frame use at `Operation::execute`, synchronous and job dispatch, and
evaluator guards with actual bisections. Implement a common reference for both
lanes and position-independent wording without weakening refusal. Change the
deep async test's yield quantum or representative depth while preserving a
frame that never polls and its region/no-poison proof. Require at least 10×
fewer timer yields on that proof. Compare fixed-work synchronous instructions
before/after, and record nonexclusive test poll/yield and elapsed counts. Run
both lanes on rebuilt package and review. Commit, push,
and report the task.

## Task 3: HTTP classification of deep typed stack refusal

Map `native-sparql-evaluation-stack-exhausted` to 400 at the shared error
classification consumed by Cloudflare, leaving other evaluation errors at 500.
Add real `fetch` controls for six named shapes and adjacent successful depths,
including both stack codes and a true 500 neighbor. Update package documentation
and declarations. Run focused protocol/Cloudflare checks and review. Commit,
push, and report the task.

## Task 4: Bound nested star projection work

Inspect current `visible_variables_with_memory` and `GraphPattern::Project`
consumers. Implement compact star projection or a typed size admission before
each growing vector is built, preserving scope and first-appearance order.
Exercise actual public parse and optimized WASM entry paths on the issue's
nested fresh variables, accepted neighbors, and an 8,000-level control.
Instrument projection variable visits/copies and allocations at doubling
depths. Accepted work must grow below 3× per doubling; if the 8,000-level
case refuses, do it before repeated vector construction with pre-refusal
visits/copies at most 8× the input variable occurrences plus a fixed,
documented setup bound. Record the chosen cap, actual count/byte bounds and
peak memory; preserve parser/serializer round trips and affected evaluator
queries. Review, commit, push, and report.

## Task 5: Reuse GTS term identities

Replace repeated full nested-key expansion with shared structural identity
nodes or an equivalent representation; remove linear path membership on
acyclic chains and DAGs. Preserve canonical CBOR key ordering/bytes, cycle and
missing-ID markers, and sort tie law. Count source term/edge visits,
path-membership probes, and retained identity bytes on actual remap/write at
several doubling depths and DAG widths. Retained identity state must be linear
in source terms/edges/flat payload, membership lookup constant per visit, and
visits plus identity bytes must grow below 3× per doubling; compare the old
failure's counts. Run malformed cycles/missing IDs, frozen vectors and writer
readback byte checks. Review, commit, push, and report.

## Task 6: Requalify already-landed host behavior and the complete package

Rebuild the optimized WASM package from final applicable source. Execute the
idle-Node deadline and deep-term child reproductions, relevant native unit
tests, host checks and the issue's real Cloudflare paths. Reconcile generated
and manual claims. Run the mandatory complete repository gate once when fixes
settle; record any source/CI invalidation honestly. Resolve material independent
review findings. Complete hosted checks and integration assessment.

## Task 7: Publish and integrate the complete issue

Only after all eight completeness rows are demonstrated, create the PR with
`Closes #365`, publish the plan and exact validation, address hosted feedback,
and merge only through `/home/paudley/stage/root/bin/ghprsq`. Verify the issue
closed and required mirror readback, then remove the delivery worktree/branch
with Stage cleanup after its evidence is archived. A prior task commit or a
green local suite is not an issue closure.
