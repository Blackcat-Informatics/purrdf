# Issue #566: Unify native graph projection across default and named RDF worlds

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

The resident dataset exposes only `project_named_graph(&str)`. A consumer cannot
select the actual default world using the same native graph-faithful,
graph-erasing operation across base assertions, reifier declarations and
annotations. Reconstructing those tables downstream duplicates a kernel job.

This surfaced while benchmarking GMEOW's recorded production worlds: its full
object-world assembler lists 179,177 default-world assertions, while a shortcut
that reads the authored default before the assembler's meta-level filtering
produces 180,062. That shortcut is not the production world and must be removed.

Replace the named-only projection with one native `project_graph` operation
whose explicit selector addresses the default graph or a named RDF graph.
Keep one implementation for all three native tables. Remove the superseded
named-only API and migrate its repository callers and documentation. Preserve
existing named-graph isolation and support graph names without converting
them into fabricated IRI strings. Invalid graph names must fail explicitly.

Acceptance:

- Select the true default graph and erase only that selected graph slot.
- Preserve every selected base assertion, reifier declaration and annotation;
  never infer statement-layer membership from a reifier's use as a subject.
- Isolate default, IRI-named and blank-named graphs even when the same reifier
  identity occurs in more than one graph.
- Missing and declared-empty graphs yield an empty selected dataset without
  importing content from another world.
- Keep named-world canonicalization and bundle identity controls passing.
- Use first-party Rust, no dependency, legacy alias or fallback.

Downstream integration is part of GMEOW's engine responsibility split. Its
benchmark must select the default world after complete production assembly,
using this native operation, and remove the authored-default shortcut.


## Comments (4)

### paudley — 2026-10-17T03:50:36Z

# One native graph projection

The owning contract is the complete issue body and acceptance list. Repository
goals, baseline and AGENTS.md govern the change. The existing named projection
already preserves independent graph slots across all three RDF 1.2 tables; retain
that behavior and its canonicalization controls rather than copying it downstream.
The unrelated REWORK_LIST.md in the main checkout remains untouched.

## Task 1: Replace the named-only operation

Replace `project_named_graph(&str)` with `project_graph(Option<&RdfTerm>)`.
`None` selects the true default graph. `Some` selects exactly one IRI- or
blank-named graph; invalid name kinds fail before row traversal. Every selected
row is emitted in the default graph. Use one graph-slot equality predicate for
ordinary assertions, reifiers and annotations. Migrate every repository caller
and Rust documentation link; delete the old method instead of adding an alias.
No dependency, schema, compiler-profile or wire-format change.

## Task 2: Prove isolation through the native operation

Extend the existing graph-projection tests with default and blank-named worlds
sharing a reifier, including a nested RDF 1.2 term and graph-local annotations.
Assert exact output content and graph erasure. Test absent and declared-empty
worlds and invalid graph-name rejection. Run the existing named isolation,
canonicalization and graph identity controls, affected-target Clippy and required
normal hooks. Tests construct only tiny synthetic RDF, never the GMEOW corpus.

## Task 3: Review and integrate the complete kernel change

Commit and push normally, open the issue-linked PR and obtain CodeRabbit feedback
where available. Record actual qualification and any missing independent review
without fabricating it. Merge only through ghprsq with selected Stage evidence.
The downstream GMEOW consumer then pins the merged native change, migrates its
callers and selects every benchmark world from the full production assembler.
Remove its authored-default shortcut; rerun the real default-world operation and
keep any genuine OWL admission refusal visible rather than falling back.

The separate combined stop/proof and shared-store/delta boundaries remain part of
the complete GMEOW engine migration. This kernel issue does not claim to finish it.


### paudley — 2026-10-17T04:17:46Z

The workspace CI failure was stale source receipts in the logical-capacity
ledger after the graph API and its tests changed the pinned kernel files.
Commit `af8936e6f22c70dcbc3f3dc1224be887aba51cc5` renews the dataset authority
identity, constructor-closure identity and the moved permutation sink address.
No proof class, sink identity, reason, authority set, limit or checker was changed.

The complete `make helpers-hygiene` now passes: all 82 enforced ledger jobs,
118 distinct-body rows and all 133 registered hash domains. The initial local
attempt correctly failed because it used the function declaration's line
instead of the observed sink line; that was corrected before qualification.
The original 1459 passing kernel and consumer tests remain applicable because
this repair changes only ledger source receipts. Normal signed commit hooks
and the normal push succeeded. Hosted checks are refreshing on the new head.

The linked-issue review warning about reviewable downstream integration is still
being addressed. The GMEOW worktree deletes the authored-default shortcut,
projects every selected world from the complete object EDB and migrates its
named callers. Its consumer type-check passed against the new native API.
The dependency pin now includes this receipt repair; current strict Clippy and
the explicit optimized default-world benchmark are running before publication.
I will add the actual dependent commit/PR and execution result here; no full
canonical admission or complete engine delegation is claimed.


### paudley — 2026-10-17T04:23:39Z

The downstream source is now reviewable in
[GMEOW PR 1837](https://github.com/Blackcat-Informatics/gmeow-ontology/pull/1837),
commit `7db11af84`.

Its [recorded consumer](https://github.com/Blackcat-Informatics/gmeow-ontology/blob/7db11af84/crates/pipeline/examples/recorded-class-consistency.rs)
unconditionally calls `assemble_object_level_edb` before selecting a world.
Selecting `default` produces `None`, which is passed to the native
`project_graph` operation. The authored-default helper and shortcut are deleted.
The full EDB is released after selection and before native reasoning; its
certificate and question-bound proof replay remain native. All named graph
callers and the canonical pipeline description are migrated as well.

The consumer type-check passed against the new API. The final dependency pin is
`af8936e6f22c70dcbc3f3dc1224be887aba51cc5`, including the reviewed source-receipt
renewal. Normal signed commit hooks and the normal push passed. Strict workspace
Clippy and the explicit optimized default-world operation are still running on
that final pin. No current default-world execution result or complete canonical
admission is claimed before that operation finishes.

This provides the concrete source evidence requested by the linked-issue
warning. The dependent GMEOW PR will be merged after this native PR; its complete
engine responsibility issue remains open.


### paudley — 2026-10-17T04:57:26Z

feat(core): project default and named RDF graphs through one operation

Advances #566. The paired GMEOW consumer is published in
Blackcat-Informatics/gmeow-ontology#1837 at 7db11af84 and pins af8936e6f.
It assembles the complete object EDB before default selection and deletes its
authored-default shortcut. Its strict prebuild Clippy passes. Actual recorded
execution and hosted completion remain pending, so issue 566 stays open for
that paired acceptance. This native dependency lands first. No complete-canon
admission or full class-engine delegation is claimed.

## What changed

Consumers need the assembled default world, not a broader authored-input
shortcut. Replace project_named_graph with project_graph(Option<&RdfTerm>): None
selects the true default; Some selects an IRI- or blank-named graph. One slot
predicate selects assertions, reifiers and annotations independently, then erases
only the selected graph slot. Invalid name kinds hard-fail. Missing and declared
empty worlds produce an empty selected dataset. Remove the old API and migrate
every repository caller and documentation link without an alias or fallback.

Existing capacity proofs are rebound to reviewed full-file/constructor source
identities and the moved permutation sink; proof classes, reasons, sinks,
authority sets, limits and the checker remain unchanged. No runtime dependency,
compiler profile, wire format or semantic Cargo feature was added.

Validation: 1373 core unit tests, 60 shared-view controls, 18 RDF determinism
controls and eight SHACL graph-view controls pass. One existing large compaction
case remains ignored by its ordinary inventory. Six focused projection controls
pass, covering independent scopes, nested RDF 1.2 triple terms, shared reifiers,
empty selections and invalid selectors. Affected all-target Clippy passes with
warnings denied. Complete helper hygiene passes: 82 enforced jobs, 118 distinct
rows and 133 hash domains. Normal signed commits and pushes passed.

All 49 final hosted checks pass, including selected compiler, conformance,
WASM, cross-architecture, Miri, SIMD, language binding and native test lanes.
Four configured experimental lanes are skipped. CodeRabbit's final reviewed
head is af8936e6f, with no actionable code finding and minimal assessed risk.
Its downstream-source warning is addressed by the linked published consumer;
that consumer's remaining execution is kept visible under the open issue.
No inline or unresolved review thread remains. Review-debt and validation
indexes distinguish actual independent review from owner assessment; no distinct
completion-auditor tool is available and none is impersonated.

The complete plan and current acceptance index are under the selected Stage:
/home/paudley/Active/purrdf/.worktrees/566-unify-native-graph-projection-across/.stage/unify-native-graph-projection-across
See plan.md, validation.md, review-debt.md, integration-assessment.md and
capacity-receipt-review.md with their attributable raw logs and review evidence.
No test produced or rebuilt a GMEOW corpus. No unrelated dirty work or sibling
worktree is included or removed by this integration. Failed initial test fixtures,
the first hosted stale-receipt failure and the corrected sink-address attempt
remain archived and are not counted as passes.

## Commits squashed (2)

- `af8936e6f` fix(core): renew graph projection capacity source receipts
- `03902c2f5` feat(core): project default and named RDF graphs through one operation

## Files

```
crates/rdf-core/src/ir/canon.rs                 |  21 +++-
 crates/rdf-core/src/ir/dataset.rs               | 152 ++++++++++++++++++++----
 crates/rdf-core/src/ir/pipeline_bundle.rs       |   6 +-
 crates/rdf-core/tests/shared_views.rs           |  14 ++-
 crates/rdf/tests/content_id_byte_determinism.rs |   4 +-
 crates/shapes/benches/shared_views.rs           |   4 +-
 crates/shapes/tests/named_graph_view.rs         |  16 ++-
 helpers-ledger.toml                             | 150 +++++++++++------------
 8 files changed, 255 insertions(+), 112 deletions(-)
```

## Conflicts

None. The actual origin/main base has not advanced. The merge-tree prediction
is clean at 1e3215c6674a788b45798fb5efdafbead55c5f96; no synchronization is
needed. Existing execution evidence applies to unchanged Rust source across
the final ledger-only repair.

---

Defect-Class: none



