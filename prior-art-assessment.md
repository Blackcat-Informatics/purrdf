<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Independent identity/storage intake and grouping assessment

Status: grouping/intake assessment only. This is not acceptance of a detailed
implementation plan, source review, successful qualification, issue closure or
implementation completion. All six full contracts remain in scope. No source,
Git state, forge state or qualification process was changed for this assessment.

The complete captured bodies/comments in issue.md and issue-{492,495,491,484,494}.md
were read, together with issue-analysis.md. Only495 has a captured comment; its
host identity addendum is part of the full contract, not optional prior art.
Current native source was inspected at the actual DatasetView, Datalog ID/store,
paged stack, segmented session and evaluator interfaces. No live fetch or
repeated issue discovery was needed. Historical source references in the issue
bodies do not establish the current implementation state.

## Recommended complete delivery boundaries

| Boundary | Complete contracts | Reason and real dependency |
| --- | --- | --- |
| A | 490 +495 | Settle one host term-ID/dictionary path, independently wide internal Datalog row/partition IDs, typed capacity ingress and global text addressing, then complete the real sparse segmented stack, publications, removals and local purge under that representation. |
| B | 491 +484 +494 | Share reverse side-table access and ordered range probes, native page/block summaries, snapshot-bound statistics, and production estimator/statement-layer/filter callers. Coordinates and prefix snapshots complete494 beside those access paths. |
| Independently closable492 | Full492; append to A only if actually ready | Graph-free physical rows and arity-three rules do not logically require sparse stacks or host IDs. Its Datalog row/atom migration overlaps A, so settle one ID/row-shape-capable representation before independent closure. Do not delay A for unready canonicalization/text identity work. |

A full-six publication couples a new sparse stack and host Datalog identity to
independent graph-free canonical/text/arity migrations and three separate
million/ten-million-row acceptance families. Shared crates alone do not create
a dependency. The recommended boundaries optimize fully merged contracts;
they do not permit partial issue closure. One source writer should reconcile
the overlapping native homes, with one consolidated qualification at each
complete delivery boundary rather than a gate per issue or helper.

## Existing native waist and actual missing work

`crates/rdf-core/src/dataset_view.rs` already defines the generic
`DatasetView::Id`, guarded dictionary access and `ViewTermId::JoinKeyAtom`.
Dataset and computed-term keys have disjoint encodings, including a wider
atom for wide host IDs. Preserve this evaluator operator path and the current
compact native TermId adapter. A host identity capability must certify value
order where it is used; unrelated raw ID order cannot stand for numeric or
temporal comparison.

`crates/datalog/src/id.rs` still brands NonZeroU32 handles with a panicking
mint constructor. `store.rs` interns lexical surfaces, uses its own TermId and
translates ground surfaces at probes. This is the substantive missing
host-dictionary/no-join-remapping path. Parameterize that same store/operator
body over the selected host identity/dictionary while preserving its native
adapter and canonical emission law. Host term IDs do not subsume internal
row/partition width; those independently need the required64-bit representation
and typed exhaustion. No second host evaluator or scalar engine is warranted.

`ir/paged/stack.rs::snapshot_with_head` currently builds a new GlobalDictionary
and traverses/reinterns source dictionaries. `ir/segmented/session.rs` already
provides sparse provider-backed guarded/fallible reads. Reuse that sparse
mechanism for495, including its typed failures and original owned reservations;
do not merely wrap the old whole-dictionary snapshot. Native shared allocation,
term-copy and read-failure homes landed through508 are current prerequisites,
not an invitation to copy their implementations.

Resident reverse reifier lookup remains a scan, and segmented side-table access
currently has graph-level enumeration.491's access API and production callers
are therefore real work. Its explicitly permitted honest scan defaults do not
excuse the required matching-proportional indexed resident/host examples.
484 must expose and consume snapshot-bound statistics, not publish unused
summary structures.494's range pushdown requires491's ordered access and
compatible page/block summaries; host clock options alone cannot close it.

Datalog currently hardcodes ATOM_ARITY4 and POSITION_GRAPH3; its partition key
includes graph.492 requires a real arity-three and graph-free representation,
not hidden all-default graph columns. Compact native32-bit TermId can still
produce the required12-byte SPO rows; host64-bit identity is not a prerequisite
for that evidence. A triple-only DatasetView's named-graph admission must be a
typed refusal, with graph-free side tables, permutations and documented
canonical/text identity laws all complete.

## Closure acceptance that each boundary must retain

- A/490: actual public term/string/quad/permutation/delta/merge/import/provenance
  and Datalog growth/mint paths return typed table/count errors at injected
  boundaries, with healthy neighbors and a structural no-capacity-panic census.
  The real global range constructor crosses four GiB synthetically with checked
  host addressing; the real Datalog path represents the u32 boundary.
- A/495: opening1000 certified segments decodes zero dictionary bytes;
  publication work stays independent of total distinct terms under tenfold
  growth; chronological layers/head/removals and RDF1.2 rows equal the eager
  effective-set oracle. Local replacement plus release of original segments
  removes the lexical bytes, retained old snapshots remain valid, and an
  instrumented host shows no ID translation/remapping inside joins. A stable
  interface alone does not satisfy495.
- B/491: real triple/component and annotation reverse directions reach the
  statement-layer/planner; ten-million reifiers with six annotations examine
  candidates proportional to matches. Every backend supplies the actual index
  or reports its scan path honestly in EXPLAIN, with unchanged RDF1.2 results.
- B/484: exact/bound/sketch status, predicate/class/distinct/multicolumn and
  statement-layer statistics reach SPARQL and Datalog plans. Host statistics
  change real plans; rare-class and paged reifier controls pass; million-claim
  estimates meet the twofold contract. Two-segment merges equal one combined
  segment, including distinct identities.
- B/494: caller NOW/entropy values reach every execution/rule context and
  evaluation provenance; absent inputs preserve existing behavior. Prefix-three
  snapshots of five generations include exactly their removals and exclude
  later layers/head. Actual ordered annotation predicates reject nonoverlapping
  pages before admission, including outlier/multirange cases, and ten-million
  claim answers equal literal post-filter results.
-492: twelve-byte SPO storage, graph-free side tables and three permutations,
  typed GRAPH refusal, unchanged default-graph SPARQL results, actual arity-three
  rules equivalent to all-default quad models, and executed RDFC/digest/text
  identity laws. No foundation-only credit substitutes for these contracts.

No duration, future benchmark result, plan acceptance or source completion is
inferred here. Existing issue-analysis.md remains the complete contract map.
The root owns the next source plan and this worktree after this single artifact
is saved; the independent assessor remains the writer of the separate geo tree.
