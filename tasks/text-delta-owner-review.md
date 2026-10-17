# Implementation assessment

This is the implementation owner's source assessment, not independent review.
Subagents are disabled. Hosted material review remains a delivery requirement.

The public `TextIndex::with_delta` is the actual production entry point. It
collects the next source and effective additions/removals through the existing
two-layer reader. Its reverse multiset join refuses an absent addition and
checks the reconstructed prior digest before changing tables. Both occurrences
of a literal present in asserted and annotation layers survive this proof.
Coverage is refreshed from the complete next source, including nonliteral and
reifier-only observations; the public portable case exercises the nonliteral
same-generation path.

Changed document identities include graph, subject and language. Only their next
rows reach the existing analysis/assembly homes. Complete changed documents are
reanalyzed, including their retained literal rows; unchanged documents are not.
The fixed-work counter observes this directly. The delta owns its buffers;
cloning a snapshot is explicit and copies them. An unchanged document's token
allocation survives canonical rebasing in the tested mixed update.

Old/new document maps are monotone. This preserves the retained posting order
by (partition, document), not document alone. Terms join by spelling; postings
for removed or replaced documents are filtered, empty dictionary entries are
removed and both token streams are rebased to the resulting dictionary.
Partition spans, subject lookup and occurrence traversal use the original
construction homes. Partition and field statistics sum retained document facts
through the same existing laws rather than introducing a second scorer or
statistical implementation.

Surface source/term maps preserve alignment, gram positions, span shapes and
pronunciation caches. The private bound-index path avoids hashing the full
standalone surface during rebasing/merging; the final TextIndex fingerprint is
computed and bound once. Public standalone operations still compute their own
content generation. Their sorted join is shared with the existing pronunciation
union; the old union body is removed. No runtime dependency or feature is added.

The initial native oracle cases pass for insert/replace/retract, wrong source,
missing additions, cross-layer multiplicity, annotations, graph-free storage,
empty transitions, snapshots, lexical/Han/surface-disabled modes and weighted
ranking. A 65-step population walk under three configurations provides 195
fresh-build comparisons of tables, streams, identities, statistics, ranking and
auxiliary evidence. Complete affected-package execution passes 353 tests in 33
suites. Its source-inventory assertion is updated for the two actual new files.
The subsequent nonliteral coverage assertion passes its focused native rerun.
Latest strict compilation passes. Actual WASM execution and current helper
census are running; neither is presumed successful. Full gate, hosted review,
publication and integration remain unverified.

Costs remain explicit: collection and canonical sorting/hash traversal still
visit retained source data; ordered merges visit retained tables and allocate
their final posting buffers. No elapsed-time benchmark, throughput claim,
constant-time update or shallow-copy snapshot claim is made.
