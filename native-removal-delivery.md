Scheduled Datalog withdrawal currently rebuilds the surviving relation store,
copies its facts and remints internal handles. That prevents a small correction
from retaining the existing native source and execution coordinates. Native
point removal is the first dependency of full signed model maintenance.

Part of #493. This independently usable storage delivery leaves every remaining
maintenance requirement in that parent intact.

Deliver indexed `RelationStore::remove_ids` and lexical `remove_row`, and route
scheduled layer/finish withdrawal through them. Preserve unrelated original row
and term handles and the immutable base. Reinsertion must receive a fresh append
handle, while delta boundaries and row-indexed metadata use retained append
extent rather than live cardinality.

Acceptance:

- Triple and quad stores remove present rows and return the original handle;
  repeated/unknown removal is inert and never interns unknown surfaces. Native
  wide IDs need no lexical projection. Source failure refuses before mutation.
- Sealed batches, mutable tails, lazy object permutations, compaction and
  reinsertion preserve all surviving handles. Empty partitions disappear from
  live predicate/graph/partition enumeration while retaining their old lookup
  handles for reactivation. Empty probes do not build cold object indexes.
- Actual scheduled withdrawal no longer rebuilds survivors. Shapes incremental
  view cursors use append boundaries, and both seminaive and chase governors
  charge retained slots. Vacant log cells and interned terms are not reclaimed by
  point removal; no bounded long-lived maintenance-memory claim is made.
- Native recursive models and independently checked recorded proofs match cold
  evaluation after holes in the append log, in both scheduling and derivation
  modes. Mutation sequences match an independent reference set. The native
  removal plus warmed object-scan allocator control records zero allocations,
  requested bytes, retained bytes and peak working bytes.
- Execute actual portable public controls, affected Shapes rule-engine cases,
  strict compiler/docs/helper gates, normal signed commit/push hooks, independent
  review, complete hosted qualification and sequential integration.

This child delivery does not close parent issue 493. All original signed
insert/retract transactions, recursive/negative maintenance, ungrounded-cycle
removal, canonical current witnesses including unchanged-membership repair,
per-relation ChangeSource watermarks, governed certain/withheld resumption,
million-edge affected-part work and actual maintained refutation integration
remain required there. Parents 486, 488 and 501 likewise retain their complete
memory, continuation and reusable-proof contracts. Recorded-proof filtering and
SHACL projection can still scan retained structures; this is no global
affected-part work claim.
