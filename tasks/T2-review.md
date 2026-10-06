# Task 2 independent final review

VERDICT: PASS

Task 2's assigned consumer implementation and local qualification meet the plan.
No unresolved source defect or assigned acceptance gap was established. This is
an independent source review with inspection of attributable implementation-run
receipts; the reviewer did not independently execute Cargo, tests or the example.
It does not declare the whole issue complete, hosted CI successful, or any commit,
push, publication or merge performed.

## Qualified identity and authority

Issue: 457. Branch: `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Worktree: `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Parent/current HEAD: `331c44e93aa6bcf8434dd027c4e0aae2b5b939b9`.
Final staged candidate tree: `13496228db0e5ec79074acad9020cac5aab7556e`.
Plan SHA-256: `ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`.
Final five-file patch `raw/T2-source.patch` SHA-256:
`040958eab3e246bd0b1d15f3ba51e39bf07d0f8d6a2cc49aefb86c572ab42a84`.

Independently checked every entry of `raw/T2-source-sha256.txt`, the patch digest,
current HEAD, and staged-index equality to the candidate tree using
`git diff --cached --quiet 13496228db0e5ec79074acad9020cac5aab7556e` (exit 0).
The candidate contains exactly the five Task 2 changed/new files below.

| Source | Final SHA-256 |
| --- | --- |
| crates/sparql-eval/tests/paged_stack_query.rs | e362206826c0ac977be6ec2269caefb5bff27a09f7452e088c3303ba2095ebce |
| crates/sparql-eval/examples/paged_stack.rs | 765580136161854f9586d3314c3ccb7455eb2f2b80530cfc4aa4eb53d3d80ff0 |
| crates/sparql-eval/examples/support/paged_stack.rs | e5df7a0504d4f2c517fc308dca5402f0fac0fd8724e7ab73f8f30488726837f7 |
| crates/rdf/src/lib.rs | 1b0f9cad6ac0fb83724c5272db77dd7d556d235fd7f19ca49d92e9587fc6eefc |
| crates/purrdf/src/lib.rs | 7035d22faa1e23ab170f1a4b5d429395736e35cdae7667383fc31b07d025e703 |

Applied the already-read repository laws, deficiency notice, backend contract,
issue/analysis/prior-art assessment, reviewed plan, validation index and Task 2
consumer contract. Reused `tasks/T1-review.md` only for unchanged core inputs;
the Task 1 manifest still matches after base synchronization, with applicability
receipt in `raw/T2-core-applicability.log`. No Task 1 runtime receipt substitutes
for the Task 2 evaluator checks. No source, Cargo configuration, dependency,
feature, forge or history mutation occurred in this review. Only this final
report was written; `tasks/T2-review-draft.md` remains historical evidence.

## Caller and oracle judgment

The tests call the shipped native engine's `query_fallible_view` and
`query_prepared_fallible_view` directly on views of retained stack snapshots.
Independent ordinary values are maintained in a `BTreeSet` and reconstructed
through the native builder. RDF 1.2 expected tables are explicitly built with
typed reifier/annotation methods. Neither oracle drains the stack or substitutes
an eager scratch dataset for the stack caller. Numeric ordinal-zero disagreement
between independently numbered bases remains an asserted witness.

The 19-text chronological matrix exercises joins/FILTER, OPTIONAL/unbound cells,
UNION bags and DISTINCT, MINUS/NOT EXISTS, grouping/subquery, nonempty paths,
named/default graphs, empty graph membership, ASK, CONSTRUCT and ordered slices.
SELECT equality includes variables and exact ordered `Option<TermValue>` cells,
so duplicate solutions, order and unbound values are observable. CONSTRUCT uses
actual deterministic graph-carrier bytes. The effective four-edge witness guards
against vacuous agreement. Retractions, reinsertion, resident head and sealing
states are compared, and earlier snapshots are revisited after later mutations.

The generated sequence checks 32 value mutations against independent values,
five query families and retained readers at five sealing boundaries. Its anchor
self-cycle has an asserted exact nonempty closure at every step; this demonstrates
preserved closure amid edits, while the chronological fixture supplies changing
closure evidence. Fully withdrawn input produces empty consumers and zero fold
pages. Ordered compacted per-page `PackBuilder` carriers equal independently
constructed eager effective inputs at multiple row bounds and physical histories.
Sharing the canonical partitioner does not replace the independently maintained
effective typed inputs; the assertion observes actual emitted page bytes.

The typed fixture preserves separate blank scopes with the same label, nested
triples, directional literals and named graph identities. It observes annotation
demotion after the last declaration and promotion of an older physical ordinary
row by a later declaration in another graph. Explicit expected typed counts,
both query entry points and carrier bytes qualify head and sealed states. The
native ordinary/virtual-reifier overlap test preserves the native per-stream
contract and compares both SELECT bags and COUNT to a separate native input.

## Preliminary findings resolved

- R1: delta/head-only graph coverage is present. A populated new graph and an
  explicit empty declaration are queried with constant/variable GRAPH and ASK
  membership before/after seal. Last-row withdrawal removes the implicit graph;
  independent eager graph inputs, retained snapshots and actual fold bytes are
  compared. This repairs the former consumer coverage gap.
- R2: the generated path uses its real seeded anchor, with an exact nonempty
  answer at every generated step. No changing-anchor claim is made.
- R3: the external later reifier promotes the existing ordinary annotation
  candidate in `elsewhere`; the formerly demoted note in `g` stays ordinary.
  Explicit eager annotation/reifier inputs, stream counts, head/sealed queries,
  page bytes and retained demoted readers cover the repaired path.

No further concrete source finding remained after these bounded rechecks.
Historical findings and pre-final identities remain in the draft report.

## Operational boundaries and public usability

Controlled providers are armed after successful sealing. Tests observe exact
source-qualified page-zero addresses, newest-first first-request order, head
charges, global pages/bytes, inclusive limits, zero/below-limit refusal, cache
rereads and provider materialization counts. Repeated ordinary/prepared calls
on the same view preserve the full receipt and read counters. Refused requests
retain their evidence without materializing the source.

Provider refusal, cancellation, deadline, explicit invalid data and changed
content retain distinct typed operational causes. Assertions require no
diagnostic substitution or partial-answer payload. Failure after admitted head
data latches through metadata, later constants/prepared calls and compaction.
Generation/page-count drift targets source ordinal 1 in a two-source vector,
including a zero-page source; skipped/constant/head-only queries through both
entry points fail with zero content requests and charges. These are meaningful
untrusted-provider hard-error checks rather than success-only query parity.

The executable uses real public stack construction, edits, sealing, retained
readers, ordinary/prepared joins and folding. Its executed assertions establish
old Bob (2 layers, 2 pages, 202 bytes) versus current Robert (3 layers, 4 pages,
1088 bytes), qualified origins, identical cached receipts and independent eager
ordered carrier equality for one compacted page. Shared helpers only construct
inputs/carriers and live once within the evaluator crate. RDF selective exports
expose all nine new stack/canonical symbols; the umbrella smoke constructs,
mutates, snapshots, performs guarded ASK with Head evidence and folds through
the supported root facade. No added runtime dependencies, semantic features,
configuration, helper exemption or competing production path was found.

## Attributable validation and applicability

Read the final `tasks/T2-implementation.md` command/exit handoff and corresponding
receipts. All exits below are recorded as 0. Cargo invocations use the assigned
worktree and `CARGO_BUILD_JOBS=2`; compiler capture is `raw/T2-toolchain.txt`:
rustc 1.100.0-nightly, commit `4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1.

| Check | Observed result and receipt |
| --- | --- |
| `cargo test --locked -p purrdf-sparql-eval --test paged_stack_query --test fallible_query --test paged_query_e2e --test delta_view` | 32 pass: new 7, fallible 17, paged 7, delta 1; `raw/T2-evaluator-qualified.log` |
| `cargo test --locked -p purrdf-sparql-eval --test paged_stack_query` | Final spelling: 7 pass, none failed/ignored/filtered; `raw/T2-new-final.log` |
| `cargo run --locked -p purrdf-sparql-eval --example paged_stack` | Concrete asserted observations above; `raw/T2-example.log` |
| `cargo test --locked -p purrdf --lib facade_exposes_the_completed_umbrella` | Intended test passes; 43 unrelated tests filtered; `raw/T2-facade.log` |
| `cargo clippy --locked -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf --all-targets -- -D warnings` | Affected all-target check completed without warnings; `raw/T2-clippy-qualified.log` |
| `cargo build --locked --release --target wasm32-unknown-unknown --lib -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf` | Release compilation completed for the selected libraries and transitive facade dependencies; `raw/T2-wasm-qualified.log` |
| `make helpers-hygiene` | 81 enforced jobs, 23 reasoned variants, no open copies, 91 distinct rows, 1833 files and 80 prefix-free domains; `raw/T2-helpers-qualified.log` |
| `make build-profile-hygiene` | 984 resolved units across two gate invocations, 42 members, opt-level 3 with assertions/overflow checks; `raw/T2-profiles-qualified.log` |
| Final rustfmt, working/staged whitespace checks | Clean; `raw/T2-format.log`, `raw/T2-whitespace.log`, `raw/T2-staged-whitespace.log` |

The final seven-test run follows mechanical clippy repairs to the new target:
exact typed-empty-array equality and moving a blank value after its last use.
These preserve the reviewed assertions. The other 25 tests, example and facade
have unchanged source and production inputs, so their earlier executions remain
applicable. Historical dictionary-witness failure and first clippy findings were
repaired without weakening the witness or bypassing a gate; their logs remain
historical and do not qualify the final source.

No assigned Task 2 implementation or local qualification remains open. Wasm
evidence proves compilation only; no wasm runtime execution is inferred. This
verdict does not establish a full-workspace/conformance run, hosted CI, hook or
signing execution, final issue-completion review, publication or integration.
