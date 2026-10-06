Paged readers need to retain small sealed updates without rebuilding the entire
base. Add chronological paged generations and a resident mutable batch whose
retained snapshots expose one guarded effective RDF 1.2 view.

Closes #457.

- Resolve independent source dictionaries by value; apply ordered removals,
  reinsertion and graph-scoped RDF 1.2 classification through indexed paging.
- Pin every source descriptor and resident batch behind shared limits, caching,
  qualified receipts and sticky operational failures.
- Fold into canonically partitioned native pages with actual eager carrier-byte
  identity, and expose the API through the supported RDF/umbrella facades.

Checked literal registration shares the existing embedded-blank home. Graph-scoped
range probes bound native postings before summary work, and raw ID-row filtering
uses the shared failure latch while retaining complete descriptor checkpoints at
materialization and publication boundaries. Real consumers refuse delayed source
generation and page-count drift, including cached rows and canonical folding.

Validation executed: 18 final paging unit tests and 85 core integrations, plus a
strengthened buffered-side regression; 42 mutable and 24 global unit tests; 33
evaluator integrations; the root facade smoke; and the
runnable consumer demonstrating retained/current answers and eager carrier parity.
All-target clippy with warnings denied and release wasm library compilation passed
for core, evaluator, RDF and umbrella packages. Helper/effective-profile hygiene,
formatting and whitespace passed with recorded source applicability. Independent
Task 1/Task 2/G1/G2 reviews passed; signed commits passed their normal hooks.

The CI workflow now retains failed-shard assembly diagnostics and supports an
explicit full seven-configuration projection using the existing measurement gate.
Static workflow/schema/reader checks passed. Actual hosted v3/v4 assembly confirms
the selected eager-seal mapping loops are scalar under the unchanged counting law.
The full hosted generator and separate checked report passed all 110 sites on all
seven configurations, followed by the unchanged aggregate and compiler-stability
check. The exact generated document was independently reviewed and committed as
d48be7c960adfeaba44d642630cde4a66d7e86e7. Final normal CI remains pending.

Wasm library compilation is established; local wasm runtime execution was not run.
Storage/log policy, depth scheduling and atomic publication remain caller-owned as
specified by the issue. Retained dictionaries have documented linear snapshot and
potential cumulative quadratic costs; this change makes no throughput claim.
