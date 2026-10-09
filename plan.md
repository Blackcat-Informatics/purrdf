# Typed RDF transfer and document-local LOAD identity

Authorized portfolio delivery: additive foundation for issue 401, followed promptly
by the separate v4 remembered-graph default adaptation (471). Do not combine the
breaking default with the additive semver qualification. Entire portfolio remains
authorized; LargeRDFBench and all external-repository submissions are excluded.

Base is current main aab23cbf2. Existing opt-in remembered graph, typed importer,
term folds, CDT blank rewriting, governor and blank minting homes are retained.
The prior three public-path defect witnesses remain preparation, not current-head
correctness evidence. Inspect current source and turn them into failing-first tests.
No governing ADR or constitution was discovered in the repository file inventory;
AGENTS.md, .baseline, .goals and helpers-ledger.toml govern this work.

Detailed design and concrete acceptance fixtures are in
tasks/typed-transfer-design.md. Its additive 401 phase is part of this plan;
its later v4 phase describes sequencing, not a cut from 401 requirements.
The pre-change source used flat QuadKey added/suppressed sets and LOAD obtained
QuadValues from a mutable wrapper. Task1 is committed and pushed at b0fc9e889;
its role-aware storage and probe-order preservation are independently reviewed.
Task2 is committed and pushed at 20e2e637c on the same branch, wiring typed
transfers and document-local LOAD identities through the production engine.

## Completeness contract

| Requirement | Task | Executable acceptance |
|---|---|---|
| Additive typed physical record API and exact role preservation | 1 | Public builder/mutable insertion, role-specific removal/restoration, equal-value cross-role fixtures; snapshot and freeze tables match exact kinds and counts after each transition. |
| Preserve untyped mutation/classification and graph lifetime | 1 | Existing mutable/delta/import/graph-existence tests plus base+delta, promotion/demotion, same-kind dedup, cross-role collision, clone/retained snapshot and net-zero graph transitions in both explicit modes. |
| ADD/COPY/MOVE preserve ordinary/reifier/annotation roles | 2 | NativeSparqlEngine::update on default/named source/destination, orphan annotations and overlapping physical rows; exact typed frozen tables, source blanks unchanged, self/missing/empty neighbours. On error/trip the production engine leaves the caller Arc unchanged; internal mutable operation ordering remains intact. |
| Every successful LOAD gets fresh blank identity | 2 | Same cached Arc and IRI loaded twice and across requests; same label at different source scopes, bare/nested-triple/CDT List/Map co-reference within a document and separation between documents; suppressed/unused/CDT-only destination identities cannot collide. |
| Errors, cancellation and governed physical attempts remain truthful | 2 | Existing LOAD SILENT/resolver/stop tests and new valid/refused neighbours; charge original physical rows before dedup, successful empty named LOAD remembers destination, failed/canceled resolution publishes no rows or declaration. |
| Public API purely additive | 3 | Actual cargo semver-checks against pre-change base for affected public crates, without weakening exclusions or changing v4 default in this delivery. |
| Full required native and WASM qualification | 3 | One settled make check and make wasm qualification (reuse actual equivalent hosted gates only where issue/repository permit); meaningful actual native update execution and affected WASM runtime parity. |
| Normal publication, independent acceptance, merge and cleanup | 4 | Normal hooks/commit/push, full PR feedback dispositions, current required CI, integration assessment, completion audit, ghprsq archived evidence, remote merged state and own-worktree cleanup. |

## Task 1: Role-aware mutable storage and typed snapshots

Add RecordKind/RecordValues next to QuadValues, retain existing DatasetMut APIs.
Represent physical membership and suppression by kind+value; maintain exact row
counts, graph live counts, ordinal ordering and delta view admission.
Preserve public added_len/suppressed_len value-key metrics for existing callers:
cross-role equal-value suppressions still count as one value, with physical role
cardinality maintained separately for effective rows and retained-view accounting.
Test overlapping base-role removal/restoration explicitly; do not replace these
metrics with physical record lengths silently. Maintain counts in the mutation
owner rather than allocating a set each time a public metric is read.
Reuse builder validation and import traversal, normalize ordinary classification in its owner,
and remove lazy duplicate classification from delta publication. Typed ingress is
exact and must never infer a different role. Preserve source locations/sidecars and
freeze through the existing importer. Implement failing-first semantic regressions
and meaningful state-machine/property neighbours, review independently, then normal
commit/push and issue update for this coherent unit.

## Task 2: Wire typed transfers and fresh LOAD through production Update

Use an owned typed source snapshot before destination mutation. Retain ADD/COPY/MOVE
admission ordering, physical governor charging, destination clearing, source removal
and declaration semantics in both modes. LOAD shares the request blank counter and
destination prefix, uses a fresh pair-keyed map for each successful resolution,
rewrites triple/CDT identities through existing iterative homes, and prepares the
translated document before publication. Preserve SILENT and cancellation laws.
Run the full affected native update/core/CDT/governor controls and valid neighbours;
independent review, normal commit/push and issue update.

At the internal operation layer, COPY/MOVE can modify their private branch before
a later charge fails. The production engine publishes only after complete success
and drops the branch on every error/trip, leaving the caller's original Arc intact.
Preserve that actual boundary; internal prefix state is never a public partial success.

## Task 3: Qualify the complete additive foundation

Run actual additive semver checks and required settled native/WASM gates; retain
failures honestly, repair owning defects and rerun only invalidated coverage.
Record one full workflow qualification in validation.md. Use managed nightly SDK,
production profiles, bounded jobs/memory and private output directories; no sibling
process or cache mutation. Complete independent implementation audit against every
row and literal issue body. No implementation requirement may be deferred, stubbed,
silently omitted or counted satisfied by refusal.

## Task 4: Publish and integrate without branch accumulation

Open one focused PR for the fully qualified 401 foundation; post plan and evidence,
resolve actual feedback, assess the fresh merge-tree candidate against main, and
merge with ghprsq when required gates are green. Preserve selected Stage evidence
separately from source and remove only this merged worktree/branch. Then adapt 471
on the landed foundation; preserved dirty sibling files remain untouched until
their owning adaptation. No external-repository submission is authorized.

## Design choices and costs

Explicit record kinds increase membership/index width but are necessary to preserve
three physical RDF surfaces. Keep indexed role probes rather than full base scans.
LOAD needs one per-document blank map and translated snapshot; charge and bound
physical work at existing governor homes. Do not create a second classifier,
blank-token scanner, flattened freezer, recursive mapper or speculative compatibility
engine. Do not globally switch graph defaults during additive qualification.

Current status: Tasks1/2 are implemented, independently reviewed, committed and
pushed. Actual additive API comparison and all ten portable WASM runtime cases
pass. The first full CI=1 make check terminated2 under controller86259: three
helper-census temporary-directory controls failed because the private Cargo
output root lacks CACHEDIR.TAG. The original failure is retained. Owning setup
correction and focused all-six policy proof completed. Mandatory retry38424
now terminates0, including full native workspace/consumer/kernel gates and
actual nested all-release make wasm. Original failure remains retained;
independent Task3 closure is PASS against those actual receipts. Task4 is active:
PR507 is OPEN with reviewed ordinal repair be3fdeb84 normally committed/pushed.
Affected core1232/views59/native10/WASM10/final strict/fmt passed; independent
focused gap/completion PASS. Functional review thread is resolved, advisory
dispositions posted. Fresh hosted CI37883191140 is IN_PROGRESS, clean merge-tree
prediction against unchanged main assessed. Current CI37883191140 now SUCCESS,
all required PR checks pass, and final complete feedback/candidate refresh retains
the same resolved thread and clean integration tree. Final debt/gates/merge/archive/
cleanup remain pending. A single dependent471 branch was deliberately opened from
published be3fdeb84 during CI; the independently reviewed sequencing refinement
requires401 to land before471 integration and avoids a third implementation branch.
The current acceptance index is validation.md; historical defect witnesses remain
failing-behavior evidence, not current-head correctness evidence.
