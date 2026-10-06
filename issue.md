# Issue #457: Paged tier: an LSM-style stack of sealed delta generations with removals, read as one snapshot

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## The gap

The paged tier is built from sealed, quad-disjoint pages under one durable dictionary generation, and G2 compaction rewrites a whole generation. That fits a corpus that is sealed once. It does not fit a consumer that ingests continuously and must answer queries between compactions: re-sealing a generation per commit is not an option, and G6 states that the reference `PagedDataset` is a demonstrator whose eager seal does not build incrementally at ingest, leaving incremental ingest to the consumer.

The pieces that exist today are close but not composed:

- **C4** `DatasetMut` overlays **one** frozen base with **one** in-memory delta, mutating by value with tagged `Base`/`Delta` handles so no single id space is implied (C0.8).
- **G-clauses** give sealed pages, exact page evidence (G7, G8), snapshot binding (G9) and sealed-summary admission (G10) — for one sealed generation.
- **G5** puts durable tiers outside purrdf, which is right: the request below is for the mechanism, not for persistence.

## Requested shape: an LSM-style stack read as one snapshot

1. **Base:** one or more sealed paged generations, as today.
2. **Deltas:** a stack of small sealed delta generations above the base, each produced by sealing a batch of commits. Sealed, so they participate in G8 evidence and G10 admission like any page.
3. **Head:** one in-memory mutable delta at the top (C4's shape), which the consumer can rebuild from its own log at open, so it carries no durability obligation.
4. **Removals:** a removal set over quads (by value, per C4) applied at read time across every layer below the one that removed it, and dropped at compaction.
5. **One snapshot:** a query pins the whole stack — base, deltas, head and removal set — as one G9 snapshot, and its G7/G8 evidence accounts for pages read in every layer.
6. **Compaction:** folds deltas and removals into a new base generation, renumbering by canonical value per G2. The consumer publishes the result atomically; purrdf provides the fold.

## Acceptance identities

Both are testable without trusting either side's implementation:

- **Read equivalence:** for any query, results over `base + deltas + head - removals` are identical to results over an eager seal of the effective quad set.
- **Compaction equivalence:** compacting the stack produces pages byte-identical to an eager seal of the same effective set.

## Questions for the design

- **Term identity across layers.** A delta can introduce terms absent from the base dictionary generation. Do sealed deltas extend the dictionary as an append-only generation, or carry their own interner with C4-style tagged handles until compaction? C0.8 rules out a numeric id range meaning "delta"; which of the remaining shapes keeps G10's local-id-space summaries exact?
- **Removal of a quad that a lower layer holds** is the common case (a retraction of something already sealed). Should the removal set be per layer, or one set per snapshot, and how does G10 admission account for pages whose quads are partly removed — still exact, or labelled?
- **Depth.** Is there a bound on stack depth the read path should enforce, or is that the consumer's compaction policy, with depth reported in G8 evidence?

## Out of scope

Durable storage of any layer (G5), the consumer's commit log, and when to compact. Those stay with the consumer.


## Comments (0)

