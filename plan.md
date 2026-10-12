# Restore compiler and numeric dispatch acceptance

This delivery completes the two production defects recorded in the captured
issue and its comment. It starts from current main dd79ef77e. Rejected packaging
work remains rejected: no tooling, workflow, compiler, cache, manifest, harness
or acceptance-policy change is included. Standard installed tools and directories,
normal hooks, protected main and final ghprsq integration remain mandatory.

## Design and prior work

Shapes passes a slice of BTreeSet adjacency entries into the shared SCC helper.
The helper has a higher-ranked IntoIterator bound. Make the already known
adjacency element type explicit at this production call; leave traversal,
resource limits, output and its one shared implementation unchanged. The donor
correction is the sole shapes hunk in 7ee086ffd; its unrelated numeric changes
are excluded. Raising MSRV or recursion limits would evade the requirement.

The existing FloatProductBackend already exposes numeric paths, and the original
certified test requires them. Register numeric in the shared production family
registry and document its actual home. Preserve unknown-family/path refusal,
architecture availability and the original arithmetic replay. The numeric owner
has preserved this exact delta in T3-536-numeric-registry.patch; transfer it
without transferring the broader numeric or stack work. No second dispatcher
or tolerant parser is warranted.

The captured prior-art search covers the issue/comment, ten linked issues,
fourteen related forge items and two hundred recent commits. It found no prior
issue-specific delivery, but the published numeric branch contains the shapes
correction and native acceptance for the staged registry change. Those receipts
are donor evidence, not qualification of this main-based delivery. Rejected
PR535 logs establish failures only. No governing ADR is cited or present in
this checkout. Repository AGENTS, root .baseline/.goals and current user
constraints apply. A combined independent review is proportionate to these two
bounded changes; it must audit both issue requirements and real consumers.

## Completeness contract

| Requirement | Production entry point and acceptance | Task |
|---|---|---|
| Shapes compiles at declared MSRV without weakening a gate | Explicit type at propagate_sets_with_limit; original SCC chain/cycle controls and affected native strict check; unchanged hosted msrv workspace check plus compiler-version assertion | 1, 3 |
| Numeric family belongs to actual dispatch contract | Registry parser valid portable/neon and unknown-path refusal control; real FloatProductBackend product/endpoint test | 1, 2 |
| Preserve real certified replay | Original certified_numerical target with numeric:portable locally; unchanged Arm numeric:portable,numeric:neon hosted execution | 2, 3 |
| Additional failing library/integration shards recover | Original forced_available_paths product/endpoint unit and original directed_endpoint_product_replay; hosted affected shards | 2, 3 |
| Complete delivery | Independent review, mandatory local qualification, normal commit/push, hosted checks, integration assessment, ghprsq merge, issue closure and owned checkout cleanup | 3 |

## Task 1: Transfer the two production corrections

Apply only the explicit shapes type and numeric registry/documentation/regression
delta. Inspect source and donor evidence, preserve all sibling work. Obtain
independent plan/source review and resolve material findings. Check the diff and
format; no heavy build while the live Python job owns the lane.

### Bounded geographic gate prerequisite

The current main area-scan Clippy failure recorded in issue533 also blocks this
delivery's mandatory workspace gate. Independent read-only review at the root
geo Stage, 533-area-scan-gate-source-review.md, establishes that all eight inputs
have distinct scan/ownership meanings and recommends one documented function-local
expectation, matching nearby decomposition precedent. Add only that source
attribute on scan_for_areas. Both callers, body, signatures, work/admission limits,
assertions and lint configuration remain unchanged. Strict geo all-target Clippy
must verify the expected lint and required full qualification must still pass.
This does not complete issue533, alter its private corpus or hold complete536
behind that unrelated long-running acceptance. Source review must cover this
third file before publication; the original two536 requirements remain intact.

### Bounded retrieval caller prerequisite

The first full gate stopped at four retrieval test-adapter type errors after the
native cursor API changed from a raw box to WorkspaceCursor. Adapt only the
existing CountingTextRelation/PrefixRefusalCursor caller: retain the actual inner
admitted cursor, box the outer wrapper through WorkspaceCursor::new, and convert
only the resident entry with into_resident. Preserve every original producer,
counter, refusal, evidence method, input and assertion. The same bounded adapter
already exists in S1; transfer no other S1 code. Original real_producers21/21 and
its strict target check now pass, with independent ownership/source review.
The exact unchanged helper census and full gate remain required; no syntax or
semantic change may be used merely to evade a duplicate-wrapper fingerprint.

The exact census rejects the identical admitted-to-resident open bridge in HNSW
and that adapter. Independent source design review approves declaring that
invariant bridge once at the existing property-function home through a documented
opt-in macro, consistent with the repository's single-body constructor macros.
Both invoking implementations already override open_admitted; opaque producers
keep their existing required open/default-admitted contract. Transfer the exact
resident workspace and checked extraction body, retaining HNSW method documents
and every native producer body. No checker, ledger or tooling change is allowed.
Verify unchanged census, affected strict checks and original HNSW/retrieval tests
before normal publication and the required full-gate rerun.

## Task 2: Verify real native consumers

After coordinated lane admission (32 GiB available, 16 GiB per job, eight Cargo
jobs/four tests), run strict affected hash/XSD/shapes checks and existing SCC
chain/cycle controls. Run original numeric library product/endpoint control and
the complete original certified_numerical target with numeric:portable. Keep
original expectations and neighbors. Run one settled full make check as required;
reuse only unaffected source-bound acceptance with an explicit applicability
assessment. Missing/failed checks remain open.

## Task 3: Publish, qualify hosted configurations and integrate

Commit normally with required hooks, push and create a reviewable PR with exact
local results and explicit pending hosted configurations. The host currently has
only provided nightly toolchains; do not install another compiler or claim that
native compilation proves MSRV. The unchanged hosted MSRV and actual Arm jobs
are required acceptance, including the issue comment's affected test shards.
Resolve source failures; never revive rejected tooling or change requirements.
One independent combined completion review adjudicates the final source and
actual hosted/local evidence. No merge readiness until every required criterion
passes. Integrate only through ghprsq, verify closure and remove the completed
delivery branch/worktree. Finish the whole issue; it is not a partial foundation.
