# Native graph projection acceptance

The current implementation replaces the named-only method; it does not retain
an alias. `project_graph(None)` selects default and `Some(&RdfTerm)` selects an
IRI- or blank-named world. One slot predicate governs all three RDF 1.2 tables.
All repository callers and documentation links have been migrated. No runtime
dependency or wire-format change was made.

## Results

| Criterion | Evidence | Current verdict |
| --- | --- | --- |
| Default and named isolation, graph erasure, nested RDF 1.2 terms, shared reifier identity | `graph-projection-tests-final.log` | Six focused controls passed |
| Same blank graph label in different scopes | Final scoped-blank control selects both independently | Passed |
| Missing and declared-empty graphs, invalid name kinds | Focused controls check all native tables and typed diagnostic code | Passed |
| Complete core unit inventory, including graph canonicalization and bundle identities | `core-qualification.log` | 1373 passed, none ignored |
| Native composite/shared-view controls | Same log | 60 passed, one existing large compaction case ignored by its ordinary inventory |
| Kernel, RDF and SHACL all-target Clippy | `affected-clippy.log` | Passed with warnings denied; production implementation and all migrated callers covered |
| RDF byte determinism and SHACL graph-view execution | `consumer-projection-tests.log` | All 18 RDF and eight SHACL controls passed |
| Normal signed commit hooks | `commit.log` | Passed; committed as `03902c2f5` |
| Hosted full workspace qualification and independent PR review | Final PR 567 head `af8936e6f` | All 49 checks pass; CodeRabbit reports no actionable code finding and minimal risk |

The normal push succeeded. Stagectl created PR 567 but returned a JSON parse
error after publication; the existing PR was resolved through `stagectl pr`
without retrying creation. Complete native review submissions, inline comments
and GraphQL thread pagination were captured and refreshed after final review.
No inline comment or unresolved thread exists; pagination is exhausted.

## Receipt repair and current head

The first hosted workspace failure was stale capacity source receipts. The
review in capacity-receipt-review.md establishes unchanged admission and mint
invariants and the unchanged builder/freeze path. The repair is committed and
pushed as af8936e6f. capacity-receipt-hygiene-repaired.log passes the complete
helpers-hygiene gate: 82 enforced jobs, 118 distinct rows and 133 hash domains.
The initial failed local address renewal is retained separately. Original Rust
execution checks remain applicable to this ledger-only delta; normal commit
hooks passed. The current hosted workspace check now passes, confirming the
original CI failure is resolved on af8936e6f. The final rollup below supersedes
the earlier pending snapshots retained in raw logs and review responses.

## Native delivery qualification

The final refreshed pr-checks.txt reports PASS: all 49 hosted checks succeeded,
including workspace, unit/integration/doc tests, conformance, Miri, cross-arch,
WASM, Python, C API, compiler/MSRV and every selected SIMD configuration.
Four unselected experimental jobs have their configured skipped state.
CodeRabbit's final reviewed-head marker is af8936e6f, with no actionable code
finding and minimal assessed risk. The complete review/inline/thread surfaces
were refreshed; no inline or unresolved thread exists, and pagination is exhausted.

The library's actual public operation is executed by the passing projection,
canonicalization, shared-view, determinism and SHACL controls. Together with
the unchanged-base integration assessment this qualifies the native dependency
delivery. The paired consumer source and strict Clippy are published in GMEOW
1837. Its actual recorded default operation and hosted completion remain open;
issue 566 is deliberately retained until that paired acceptance completes.
No current-canon or complete class-engine migration claim is made.

CodeRabbit completed its code review of 03902c2f5 with no actionable code
comments and minimal assessed risk. Its linked-issue check requested reviewable
GMEOW integration evidence, now supplied through PR 1837 and the posted source
response. Actual consumer execution and combined issue closure remain pending
in that owning work; they are not waived or called complete here.

Clippy's production and caller checks remain applicable after the additional
test fixture row selecting a second scoped blank graph. The complete core unit
run covers that final fixture. Earlier logs retain the initial test-source
spelling failures and the invalid nested-subject fixture; those failures were
repaired using the actual native RDF 1.2 model and are not counted as passes.

No test produced a GMEOW corpus. The recorded GMEOW default shortcut is deleted
in its published paired PR; downstream execution remains unqualified until
the exact assembled world operation finishes and its evidence is recorded.
This issue does not establish the complete class-engine migration.
