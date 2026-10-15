# Frozen lexical owner: capacity-proof authority boundary

The #492 extraction moves the original `find_term_hashed` reconstruction into
`ir/frozen_terms.rs`. The two `TermId::from_index` sinks retain the original
stored-index invariant and literal body; their file/symbol/line pins move.
`Interner::freeze_with_memory` transfers the same admitted term vector and
store-once index without remapping. The compact builder uses this same method.

The actual new visibility matters: `FrozenTerms` fields are `pub(super)` in
`ir`, and `TripleDataset::dictionary` has that same visibility. The existing
constructor closure discovers `from_parts` tokens and opaque expansions. That
does not by itself pin a later otherwise plain sibling function that writes a
visible dictionary field.

Proposed bounded enforcement: extend the existing literal-preserving owner
aggregate to the complete shipping IR module visibility domain using the
original source walker, including path-mounted descendants by module identity.
Keep existing from_parts/opaque expansion closure outside this domain. A new
sibling or changed sibling mutation must stale the aggregate even without a
constructor token. The existing exact sink/class/authority/unique-occurrence
checks remain; this is an audited source ratchet, not a compiler proof.

Add a seeded sibling-mutation rejection and preserve the existing outside-owner
constructor/alias/macro rejection controls. Capture the actual stale index
before reviewing and refreshing affected rows. The bounded source-walker change and both sibling/mounted-writer rejection
controls are now written in the original source/capacity rule homes. They are
uncompiled; no proof ledger refresh has been applied. Prior corpus runtime
18461 is terminal 0. The next checkpoint captures the stale index first.

## Independent review disposition

The first bounded review found an actual closure gap: the original source walk
skips excluded or outside-package paths, even when mounted by a shipping IR
module. Job50398 remains frozen through its terminal. The correction will reject
such authority declarations explicitly while preserving cfg(test) exclusions.
Review: reviews/frozen-owner-module-closure-review.md.

Actual owner seams for the pending row update:
- Interner::freeze_with_memory transfers its checked term vector, arena and
  store-once index intact; it neither remints nor rewrites IDs.
- RdfDataset::from_parts accepts that owner rather than separate lexical fields.
  All original crate-internal call references remain closure inputs.
- TripleDatasetBuilder uses the same interner and validator; its rows are compact
  and the lexical builder's quad tables stay empty.
- TripleDataset::try_from(RdfDataset) checks all named roles before allocating
  compact rows and moves lexical ownership and source locations unchanged.
- Both moved find_term_hashed reconstruction sinks retain the stored-index law.
  Their new exact file/symbol/line and literal fingerprints must come from the
  stale index, not guessed locations or new exemptions.

## Corrected checkpoint

Original50398 terminated0: the original reconstruction seed passed1/1 and the
index retained40 stale proof errors/40 matching sink literals. The index's zero
exit only means its JSON was produced; it is not a passing helper gate.

The excluded-path correction uses one capacity::frozen_owner_domain predicate
in the original walker. A shipping IR mount outside the package, or inside an
excluded directory, now emits an actionable source error before skipping it.
The existing seed covers outside-package, tests and generated mounts plus each
cfg(test) neighbour. Corrected99970 has passed that seed1/1; strict checking is
still running. H appended scoped source PASS to the original review. No ledger
refresh has been applied.
