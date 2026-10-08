# Independent OPTIONAL fork-witness diagnosis/design

VERDICT: PASS

This PASS admits the proposed test correction design, not implementation or runtime acceptance. Actual failed hosted log raw/main500-pr448-integration2.log, current numeric_parallel_determinism.rs:40–145, production binop.rs:1697–1840, eval.rs:1396/1671/1808/2059 and parallel.rs:359/1207/1304–1399 inspected read-only. No production/source change or execution by reviewer.

Actual hosted9/1 failure is metered/2 mask≤1 despite plain/2 mask2; mask records callback worker identity and yield_now merely yields the OS thread. Indexed Rayon chunk/block iteration is allowed to execute its runnable work on one worker. The observed mask does not prove that production never dispatched parallel work. The actual gates support the expected fork:1500 rows exceed1024; stable native callback is safe, ordinary options do not force sequential, METERED has no reachable allocation-layout cell ceiling and no finite non-fuel/scratch caller ceiling. The finite4500-cell case deliberately retains its serial path. No source evidence contradicts these gate conditions.

Use a test-owned Condvar rendezvous in the real registered callback, activated only for fork-expected cases with threads>1. The first callback records its worker and waits, releasing the mutex. A callback on a distinct worker notifies completion and returns; other callbacks must not all wait at a barrier. The first waiter loops against spurious wakeups with one absolute bounded deadline. Reset witness state before each actual query after the preceding synchronous query has joined. A real serial path then produces an actionable bounded hard failure rather than success, while an already-dispatched Rayon sibling has a concrete opportunity to run instead of being raced by a short callback.

Retain existing exact1500 visits, multiworker mask, one-worker/finite-cell serial neighbours, output and governor/error equality, finite-overflow neighbour. Do not force production gates, add dummy work, retry until success, sleep, weaken counts, silently time out or change shipping parallel code. Tests remain native-only; no unsupported wasm threading assertion is introduced. The successful rendezvous demonstrates actual concurrent callback execution, not merely an outer pool or correct answer.

Current correction still needs source review of the implemented deadline/reset/refusal logic and actual affected tests/lint before qualification. Hosted failure remains recorded and unresolved until that verified fix is pushed and current required checks pass.
