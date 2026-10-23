# Change-proportional text updates

The existing source walk accounts for nearly all reported update cost, but it is
also the current independent witness against an incomplete delta. Removing that
witness without replacing it would admit incorrect indexes. Canonical ordinal
rebasing and full metadata hashing are separate retained-index costs.

1. Replace the ordered source digest with a versioned multiset accumulator over
   independently domain-separated row hashes and an exact multiplicity count.
   Preserve graph, subject, predicate, datatype, lexical form, language and
   direction, including identical rows in asserted and annotation layers. Use
   the same accumulator for fresh builds and reverse-delta validation. Eliminate
   the reverse validation's corpus-sized temporary vectors. Pin add/remove,
   duplicate, ordering, graph-free and native/portable equality controls.
2. Introduce a source-owned, independently maintained projection summary contract.
   Native source construction and mutation must supply the observed summary;
   the index computes the expected next summary from its own retained state and
   the requested delta. Refuse unavailable authority, wrong configuration and
   mismatches. Never use stats_fingerprint, derive both sides from the supplied
   delta, or hide a source scan in update or subsequent binding verification.
3. Collect replacement literal rows by changed subject in both RDF layers using
   actual indexed source probes. Maintain coverage counts for configured
   predicates, including nonliteral rows, and source nonemptiness across all
   three physical RDF layers. Preserve multiplicity and source error refusal.
4. Remove canonical ordinal rebasing from unchanged physical entries using stable
   internal keys with ordered public projection. Preserve deterministic public
   order, ranking ties, fresh statistics, auxiliary indexes and complete content
   identity. Token buffers now use stable shared spellings and a persistent chunked
   positional table. The complete source identity, analyzer/ranking laws, Unicode
   versions and configuration determine all derived tables, so fingerprinting
   binds those inputs directly under new index domains without a corpus walk.
   Preserve the full fresh-table oracle independently of identity equality.
   Account separately for remaining metadata work; no total delta-work claim
   while any corpus-sized update or clone pass remains.
5. Validate against the full fresh-build oracle and long mutation walks, including
   omitted/wrong deltas, cross-layer duplicates, empty transitions, graph-free,
   surface and Han indexes. Record source reads, visited entries, allocations,
   retained/peak bytes and growth with fixed delta against growing sources.
   Execute native and scalar/SIMD WASM, strict affected all-target checks, required
   hygiene and normal hooks. Publish, review, finish hosted gates and land only
   through ghprsq, then verify closure/archive and clean the owned checkout.

This plan preserves the complete issue. The accumulator alone does not finish
it, and source preparation costs must not be displaced outside measurement.

## Current implementation and delivery sequence

Primary index and surface storage now use shared persistent ordered tables.
Canonical public ranks project from stable content/document entries. Changed
partitions, token totals, auxiliary memberships and source identities update
without rebuilding retained tables. Full fresh-table comparisons remain separate
from fingerprint equality.

The source observer now compares certified normalized directories across
multiple old/new images, keeps source-independent effective keys, and leaves
intermediate regrouped prefixes lazy in the current snapshot. Full/suffix
compaction, head resets/handoffs, successive sealing and multiple outputs have
focused controls. A reproduced combination of sealed regrouping and a changed
large resident head is fixed by selecting the sealed baseline independently
from the native head-operation comparison. Overlap, old/new removals, request
refusal and retry controls pass. No former aligned physical enumeration path
remains. Directory bags can still visit duplicate multiplicities; do not turn
canonical-partition controls into a universal performance claim.

Actual resource controls measure new batch preparation, snapshot clone and the
whole update separately at three retained populations for lexical and Han
indices. They compare all fresh tables from an independent reconstructed source
and account separately for payload and authentication-proof reads. Full current
qualification is now appropriate: strict all-target hash/core/text/shapes passes;
current core runtime is active, followed by text runtime, scalar/SIMD WASM,
helper/capacity proof review, required gates/hooks, review, PR/hosted checks and
sequential ghprsq delivery. validation.md owns exact commands/results and limits.

The delivery-control failure was repeated implementation changes invalidating
qualification while historical next actions accumulated. Reuse valid coverage,
finish concrete failures and required acceptance, and do not restart intake or
add speculative redesign. All original issue and portfolio contracts remain.
