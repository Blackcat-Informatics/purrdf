<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Parser scope and lookup ownership proposal

Selected Stage engineering support for issue #508. Shipping source is unchanged by
this support lane. The ordinary patch is
`parser-scope-table-owner-draft.patch`; selected readable postimages are
`parser-scope-table-postimage.rs` and `parser-scope-homes-postimage.rs`.
Apply the patch through the sole source writer. The postimages are proposals,
not replacements for whole shipping files.

The patch was refreshed against the writer's current four-lifetime
`Parser<'a,'o,'m,'s,RDFLIB>`, `self.memory`, `ParserAdmission`, and actual admitted
fork. It preserves the fork's token/base/prebound admission and the same original
Memory reborrow. No builds, tests, Git operations or forge actions were run.
Standard-diff counts/contexts were checked against captured source in memory;
that is mechanical artifact verification, not qualification.

## Concrete storage home and APIs

`parser/table.rs::ScopeTable<K,V>` is a private parser AVL lookup tree in one
`Vec<Slot<K,V>>`. Keys order lookup only. Array-index links, parent links,
heights and a free-list occupy the same precisely admitted native array.
There are no boxes per node, opaque std collection fees, new dependencies,
hash collision assumptions, fallback lookup scans or semantic capacity limits.

Empty construction and lookup allocate nothing. Insert/reserve use checked
capacity arithmetic and the existing `Memory::reserve`, which admits the exact
old-plus-new Vec layouts before the allocator. Refusal preserves every original
association. Replacing an existing value, removing a key, reusing a free slot,
rotating, and clearing logical entries allocate nothing. `clear` retains the
covered arena capacity; `release` destroys it before releasing that layout.

The shared private API is `get`, `get_by` (borrowed query-versus-key comparator),
`get_mut`, `insert`, `remove`, `clear`, `iter`, `try_clone` and `release`.
Only parser-native shallow key/value clones may use `try_clone`. Prefix tables
use immutable `OwnedText`; clone/extraction therefore keeps the original lexical
owner. Iteration is arena order; declaration/projection order remains in the
consumer's explicit sequence and is never inferred from tree order.

## Actual production migration

- `VarScope`: first-appearance `order` plus `ScopeTable<Variable,()>`.
  Reserve the output array before committing membership; no partial binding is
  published on any admission/allocator failure. Hidden variables remain excluded.
- `ExistsScopes`: retain the exact shared trail/frame law. The latest-position
  table is `ScopeTable<Variable,usize>`; parallel `previous` links restore an
  earlier occurrence when a boundary's own duplicate is popped. Isolated frames
  copy nothing, boundaries see only their own tail, and pop discards only that
  frame's introductions. All parallel destinations are reserved before mutation.
- Visibility census and conflict checks: the existing traversal bodies use
  admitted pending arrays, balanced membership and explicit scratch destruction.
  SELECT projection, Group visibility, hidden identities, UNFOLD declaration
  order, contextual MINUS visibility, sub-SELECT narrowing and first conflict
  order are preserved. Native projection narrowing builds its membership once.
- `machine.rs`: ordinary/deferred SELECT/GROUP scopes and projection-readability
  sets use the same VarScope home. Debug census arrays use the same original
  Memory and are released afterward. Group/sub-SELECT completion propagates
  physical refusal, and consumed group scopes release their metadata.
- Prefix/blank-BGP tables: `OwnedText` keeps admitted immutable spelling; borrowed
  `str` lookup allocates no key. Prefix rebinding remains last-value-wins.
  A blank-BGP key and its AST blank share the original newly admitted spelling.
- Cross-update blank labels: `ScopeTable<BlankNode,()>` shallowly retains parsed
  blank spellings. BlankNode's new lexical Ord follows its existing OwnedText
  Eq/Hash law. Native label walks and metadata are admitted, and both label
  tables release scratch before update publication. The reused-label rejection
  remains; when several labels are simultaneously invalid, the native arena
  deterministically selects the first encountered label rather than the old
  fixed-hash bucket order. Accepted update semantics/bytes are unchanged.

Every native scope/table error propagates as `ParseError::Storage`; it must use
the existing first-cause preservation at operational egress. These bodies do
not turn refusal into syntax/lexical invalidity or render a new owned failure
message after refusal.

## Required integration with the active parser producer unit

These are necessary issue #508 work in the writer's existing lane, not waived
criteria or accepted partial completion:

1. The active AST/token/error constructors and native publication must finish.
   In particular, deferred EXISTS still needs an admitted copy of its body and
   its local-scope/pending-check metadata before publication. The scope patch
   covers that metadata's consumption; the original AST producer owns the
   actual body clone.
2. Prefix input producers `expect_pname_ns`/`expect_iriref` and every current
   AST lexical producer need original preconstruction admission. Charging the
   immutable destination after an earlier raw String was copied is insufficient.
   The scope's `ParserAdmission::text` calls price its real owned spelling; they
   do not certify an earlier raw temporary in those producers.
3. Native parser success must destroy scope scratch before reading surviving
   Memory bytes: consume/release `prefixes`, `blank_label_bgps` and
   `exists_scope_stack` under the same Memory. Pop/clear retain actual capacity
   intentionally. The existing parser producer unit owns the overall cleanup of
   tokens/control stacks/AST temporaries; use the scope types' release methods.
   One suitable parser-local splice is:

   ```rust
   fn release_scope_storage(&mut self) -> Result<()> {
       std::mem::take(&mut self.prefixes).release(self.memory)?;
       std::mem::take(&mut self.blank_label_bgps).release(self.memory)?;
       std::mem::take(&mut self.exists_scope_stack).release(self.memory)?;
       Ok(())
   }
   ```

   Call once after the completed request no longer consults scope/prefix state,
   before extracting its surviving native layout. On refusal the original
   account must still cover parser destruction; do not discard/recreate that
   grant or report a post-census estimate.
4. `BlockSink::promote_path_blanks/promote_terms` is the remaining lookup table
   at an AST producer home: replace its current BTreeMap with
   `ScopeTable<BlankNode,Variable>`, cloning the already-owned blank key.
   The writer's BlockSink migration must admit its current pattern/pending
   worklists and use the existing hidden-blank identity home through its admitted
   leaf factory before insertion. This support lane does not concurrently
   overwrite that AST producer.

Native compilation/qualification must include both units. No current native
API refuses supported query constructs for absent scope accounting.

## Meaningful acceptance

The packet adds four table fixtures: differential associations against an
independent BTreeMap across ordered insertion/replacement/interior and root
deletions/recycled slots; refusal preserving live associations; allocation-free
borrowed lookup/clear; and measured comparisons over sorted identities at widths
1024/8192 to distinguish logarithmic lookup from quadratic degradation.

The existing nested boundary/isolated-frame fixture now runs through the same
native Memory body and verifies full release. An additional scope refusal case
checks that failure while reserving parallel destinations publishes no binding,
preserves enclosing lexical identity and previous links, leaves duplicate lookup
healthy without growth, and releases to zero.

The serial writer qualification must also run existing parser scope restrictions
and wide/deep cases through actual production entry points: SELECT star order,
BIND/UNFOLD collisions, nested EXISTS/NOT EXISTS and MINUS, sub-SELECT projection,
deferred projection/aggregate EXISTS, LATERAL, prefix rebinding, blank-BGP
reuse/cross-update rejection and contextual scope behavior. Actual bounded
parse/evaluate success/refusal, allocator peak versus receipt, and retained
AST/error-owner lifetimes remain the production proof; these draft unit fixtures
do not substitute for it.
