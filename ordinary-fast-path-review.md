# Ordinary FILTER, GROUP and path delta review

Verdict: **PASS for the bounded source review** of the frozen changes against
`ccfac0a5e` in `expr.rs`, `vm/mod.rs`, `modifier.rs`, `path.rs` and
`tests/evidence_ingress.rs`. No concrete correctness defect or missing required
production branch was found in this delta. This is not runtime qualification or
performance acceptance.

- FILTER completes `ebv` before deciding whether evidence work can be elided.
  Hard expression failures therefore still propagate. The shortcut requires no
  computed VM evidence owner, no external evidence input and no physical row
  attachment. Native worker, resumed and sequential FILTER callers now use this
  same decision; computed gaps, original owner/rank attachments and declarations
  still use `filter_evidence` and its original combination/refusal path.
- GROUP establishes its decision from the completed physical input: external
  declarations, nonvirtual whole-search evidence, or any physical row attachment
  retain tracking. The proof is shared by bucketed and ordinary grouping, and
  by native workers, resumed groups and sequential groups. Aggregate value
  computation, worker witness absorption, expression barriers, resident transfer,
  output order and restoration of the original bag search remain in place.
  The separate contextual implementation is unchanged. When tracking is false,
  the previous `group_evidence` and `native_aggregate` also returned virtual
  evidence; the shortcut removes that work without removing an original record.
- Path shortcuts use empty physical metadata maps and, where a separate premise
  exists, its virtual identity. The current private path-machine ingress derives
  metadata from original edge declarations; producer row ranks remain on the
  surrounding solution/join carrier rather than entering endpoint sets. A
  nondefault edge declaration still enters the same admitted map, conjunction,
  alternative and closure fixpoint paths. The ordinary closure uses the original
  endpoint visited-set traversal; duplicates are still retained in the bag
  machine and deduplicated in repeated-path sets. Map/row/frontier allocations
  remain admitted, the shared first-error latch remains checked before output,
  and leaf source reads and governor checks are not bypassed.
- Added ordinary controls execute the same cyclic graph and queries as the
  declared controls for `p/q`, `p+`, `p{3,3}` and `p{67,67}`. They assert matching
  cardinality and ordinary evidence. Existing adjacent controls still assert
  declared polarity, complete found witnesses and refusal before publication.

No builds, tests, profiles or private inputs were run or read for this review.
The owner/root must still complete the queued ingress, prepared-path, library
path and strict-package checks on this source. Prior same-binary A/A instability
and the rejected normal performance panel remain unresolved performance
acceptance; this source review supplies no performance pass and waives no
physical-shape or allocation requirement.

## Frozen projection, aggregate ownership, VM and frame-request refresh

Verdict: **PASS for this additional bounded source review**. No concrete
production defect found in the requested material delta. Earlier judgments above
remain applicable to unchanged paths; runtime and performance acceptance remain
unqualified by this review.

- Projection's `None => projected` follows a traversal-wide check of every
  physical row attachment. Any owner, rank or retained declaration still selects
  `project_columns` and `with_evidence`; the shortcut cannot erase such an input
  attachment. Both ordinary and delivered projection use this macro. Native
  whole-search propagation remains at the enclosing `eval_evaluated_with` search
  frame, including empty output; the shortcut changes only per-row attachment.
- `AggregateSurvivors` retains actual term grants, not empty placeholders.
  Reviewed all three callers: contextual tuples, ordinary built-in survivors and
  custom aggregate tuples. Each calls `reserve_next` before the fallible term
  clone. Bounded execution pre-admits both arrays; a real foreign grant entering
  a resident destination is reserved before transfer and retained independently
  of the destination's capability. A failed reserve still drops the original
  `WorkspaceTerm` with its grant attached. Value arrays and payloads continue to
  die before the retained grant arrays, including `into_parts` custom-fold
  transfers. Omitting an ownerless grant changes neither live admission nor term
  lifetime. The `push` comment's phrase "sole caller" is inaccurate: there are
  three verified callers, all obeying its pre-reservation rule; this is a comment
  correction, not a behavioral gap.
- VM selects `advance::<true>` or `advance::<false>` only after creating and
  clearing the original evidence owner. Selection repeats on every resume.
  The tracked specialization retains the original opcode `begin`, `finish`,
  completion and suspension records, and dictionary failures propagate unchanged.
  The ordinary specialization removes only work that previously saw `None`;
  opcode computation, branching, source reads and operational refusals remain.
- `FrameRequest` forwards all eight constructor shapes with unchanged arguments.
  Moved seed/current sets retain their original buffers and owners; power requests
  retain the same shared memo owner and bit. The full frame array still uses the
  original admitted `PathVec<Frame>`, with unchanged growth and drop ownership.
  Constructors now run immediately after the step error check; a constructor
  failure remains latched, prevents frame publication and is returned at the next
  loop check. Linear and powered ranges, reverse direction and open tails retain
  their original transitions. Absent-source edge insertion calls the same
  default insertion kernel; a present source still executes `claim` and preserves
  its original error before publication.

Read-only inspection only; no builds, tests, profiles, private input access or
shipping-source changes. Root/owner validation applies to this final source.

## Physical attachment transport refresh

Verdict: **BLOCKED** on one remaining production erasure in the touched unscoped
ranked adapter. Earlier PASS judgments cover their earlier bounded deltas only.

`crates/retrieval/src/execute.rs`, `CallRead::pull`, refuses nonvirtual row
evidence only after destructuring `Some(row)`. Its separate original native
whole-search declaration is retained only when `self.retained` is present.
Consequently an unscoped read accepts empty `None` with nonvirtual search, and
accepts virtual emitted rows whose search is owned/default, incomplete or
unclosed. The strengthened row guard cannot see that search declaration.
`CallRead::settle` likewise calls `settled_search_evidence` only for scoped reads,
so unscoped settlement omits late original search declarations and any typed
failure raised by that search callback. `CallCursor::settle` only settles the
generation/service witness; it does not perform the missing search read.

Required repair: inspect original search after every successful pull, before
the `None` branch; retain it for scoped reads and refuse nonvirtual search on
the raw unscoped adapter. Read the original settled search for both modes,
preserve its typed operational failure, then retain or refuse it. Cover empty
and emitted ordinary rows with original search, plus late settlement and its
typed callback failure, through the actual unscoped ranked stream. The owner
has received this finding; no repair or runtime result is claimed here.

The rest of the requested source delta has no concrete defect found:

- `QueryEvidence::combine` and `transform` preserve a real input owner through
  semantic-default output after original ownership and dictionary checks;
  virtual inputs alone still avoid attaching an owner. `is_virtual` distinguishes
  owner/rank transport from semantic support. Attachment equality now includes
  owner and rank instead of treating all semantic-default records as equal.
- Path set, bag and pending/fixpoint storage now elide only virtual evidence.
  Original attachments survive optional set-to-bag and result publication;
  absent attachments use the original node arrays without metadata insertion.
  Conjunction, alternatives, error latching, governor checks and deterministic
  bag/set ordering remain on their original paths.
- `with_optional_evidence(None)` preserves the existing row. All seven actual
  FILTER/OPTIONAL-filter callers inspect the separate keep decision before
  publication. All three BIND schedules, OPTIONAL padding, BGP borrowed cells,
  property publication and shape publication preserve attached owners/ranks and
  original errors. Whole-search tracking remains separate from found-row
  completeness. The strengthened raw row guards correctly reject owned-default
  and ranked-default attachments.

Inspected the registered owner/default transformation and raw-row controls,
owned-default path controls, and true unscoped ranked-row refusal fixture.
They cover row attachments; they do not yet cover the search-only defect above.
No builds, tests, profiles or private inputs were used. The outstanding normal
performance rejection remains a failure; this review does not qualify runtime,
physical-shape scaling, allocation bounds or performance acceptance.

## Unscoped whole-search repair reread

Verdict: **PASS for the repaired bounded source surface**. This resolves the
specific BLOCKED source finding above; it does not qualify unrun execution.

`CallRead::pull` now examines the original cursor search before destructuring
`Some` versus `None`. Scoped execution retains the original handle; an unscoped
nonvirtual search produces the explicit whole-run producer refusal before a row
or ending can be published. The separate row-attachment refusal remains intact.
`CallRead::settle` now invokes `settled_search_evidence` in both modes, propagates
its original typed error through `ReadFault::Native`, and retains or refuses the
result according to whether the receipt exists. It does not rebuild completeness
or merge the search declaration into positive row evidence.

The existing registered `real_producers` suffix case now exercises the actual
`execute_within(..., OnDemand)` stream for owned-default and ranked-default row
refusal; opening owned search with emitted ordinary rows; empty owned search;
empty Undetermined/unclosed search; and late owned search or typed search fault
enabled after one successful row. The late cases call the public stream's
`settle`, which directly calls `CallRead::settle`, so they cover the repaired
settlement branch rather than accidentally testing another pull. The fault case
asserts the original `AllocationFailed` construct inside `NativeFailure`.

No further defect found in this correction. No tests, builds, profiles or source
edits were performed; only this existing review artifact was updated. Focused
qualification and the outstanding performance acceptance remain required.

## Native owner binding refresh

Verdict: **PASS for this bounded source delta**; no concrete defect found.
Runtime qualification remains pending.

`EvidenceInput::bind_native_evidence` leaves virtual rows on the allocation-free
path. It adopts the producer's original owner when no input dictionary exists;
otherwise it imports through `QueryEvidence::import` after releasing the input
owner mutex. Existing dictionary ownership checks remain strict. Import preserves
reason identities/descriptions and the original record, then restores physical
ownership for owned semantic-default input plus rank, membership and closure.
It does not silently reinterpret a foreign id as a local id.

Materialized calls bind driving and emitted rows before their conjunction and
bind native invocation search before merging it into the answer search.
CallCursor binds its opening search and retains the selected binding owner for
subsequent emitted rows and settled searches. Shape publication and search
aggregation bind native evidence to the same answer input. The expanded
occurrence fast path's unchanged direct combination is not an external-input
bypass: its offer is disabled when an evidence source is installed. Cross-owner
acceptance refusal is unchanged; publication now supplies the intended owner.

The expanded registered controls exercise native owned-default/ranked rows
through acceptance, a nondefault external claim combined with the actual native
producer, import of an owned-default record, and actual scoped ranked reads
with retained physical attachments. The fixture extracts its owner clone before
using it outside the mutex guard. No test execution is claimed by this review.
No builds, tests, profiles, private inputs or shipping edits were performed.

## Final projection specialization reread

Verdict: **PASS for the bounded control-flow/admission delta**. The unchanged
whole-bag physical attachment proof selects the single authored row-loop macro
with a literal tracking mode. The ordinary branch calls `RowsBuilder::push_cells`,
whose existing body uses the same `RetainedRow::from_cells` and `push_row` homes,
workspace, width and iterator as the prior path. Admission order and fallible
publication therefore remain unchanged. The tracked branch still calls
`project_columns` before construction and preserves the exact attachment through
`with_evidence`; empty output, variable order and absent columns are unchanged.
Both bulk and delivered schedules enter the same proof and specialization.

The `RetainedRow` dereference, equality and hashing delta adds inline hints only;
their cell identity, owner and destruction bodies are unchanged. No concrete
defect found. No tests, builds or profiles were run, and no performance or full
qualification pass follows from this source judgment.

## Bag attachment-summary reread

Verdict: **PASS for the bounded summary delta**. No concrete defect found.
The summary uses physical tracked storage, preserving owned-default and ranked
attachments. `push_row` and `combine_evidence` update it after successful original
publication; failed attachment admission leaves the original row unchanged.
`push_cells` and plain-cell constructors cannot create attachments. `finish`
copies the proof, `clone` preserves it, ordering preserves or rebuilds it, and
truncation conservatively keeps true. Search evidence remains independent.

Neither immutable bag nor builder exposes mutable retained rows. Consuming
resident extraction destroys its old summary; re-entry through `from_storage`
recomputes from actual physical rows, including arbitrary caller edits. All ten
replaced scans use the summary only as an absence proof or conservative choice
of the existing tracked path. The existing unit's added controls cover owned
default/rank combination, clone/order/truncate, extraction/edit/re-entry, and
independent empty/nonempty search declarations.

Existing row, vector and shared-owner admission/destruction bodies are unchanged;
the new inline booleans add no payload allocation. No tests, builds or profiles
were run, and runtime/performance qualification remains with root.
