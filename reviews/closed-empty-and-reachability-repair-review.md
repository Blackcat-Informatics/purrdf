# Closed-empty polarity and plan reachability: source review

Read-only review of `evidence_exec.rs`, `eval.rs`, `plan/mod.rs`, the existing
numbered walk, VM/filter consumers and actual producer regressions. No builds,
tests, source edits or process changes. Compiler/runtime results remain separate.

## Material remaining finding

**Native positive EXISTS over unlicensed empty search loses unknown membership.**
The revised `QueryEvidence::empty_existential` selects Neither when search is
incomplete, unclosed or already undecided, but does not itself set undecided.
`native_existential` calls it directly. A valid producer search can carry only
`SearchCompleteness::Undetermined` or `with_unclosed_search(true)` while retaining
`undecided=false`, exactly as the new native producer fixtures do.

The positive EXISTS answer is false because there are no witnesses.
`expr::filter_evidence` retains false rows only if the computed record is
undecided. Consequently direct positive FILTER EXISTS drops such a row as
decided absent. Explicit-source execution does set undecided first through
`absence_evidence`; NOT EXISTS later sets it in `native_negative`. Thus the new
NOT EXISTS/nested-MINUS matrix cannot detect the missing positive native case.

At the sole empty-predicate constructor, retain undecided membership whenever
the checked closed-absence condition is false, keeping reasons/frame/bounds and
original closure flags. Add positive EXISTS to the actual producer matrix:
closed complete emptiness filters the row, incomplete/unclosed emptiness retains
the enumerable unknown row. Do not change top-level empty bag summaries or
mark a nonempty witnessed positive unknown merely because the wider search is
incomplete. This finding was sent to the implementation owner and remains open.

## Sound inspected changes

- `EvidenceInput::existential_evidence` now guards the actual empty search with
  `absence_evidence` before constructing the predicate. Native execution uses
  the same empty constructor, whose Opposed branch requires not-undecided,
  permitted closure and Complete search. This fixes the missing licensed negative
  proof without changing FOUR negation or treating arbitrary nonempty Neither
  claims as absence. Nonempty witness coalescing returns before this constructor.
- The existing admitted transform/combine retains owner, original search frame,
  reasons, aggregate certificates and closure state. Direct empty answer-bag
  summary code is untouched. Its Neither law is separate from a closed false
  existential proposition.
- The new integration matrix actually invokes `DeclaredRelation` and asserts
  its native owner was installed. It covers three native search declarations
  (complete closed, complete open, incomplete), three external scope modes
  (none, explicit closed, explicit open), and NOT EXISTS versus a nested MINUS:
  18 executions, not merely uninvoked-registry tests. It asserts supported
  completed negative rows and unknown membership for the negative neighbors.
  The positive empty case above is the omitted dimension.
- `PlanShape::has_property_functions` is computed as a boolean during the
  existing admitted numbered walk, without a new traversal or payload. That
  walk includes subquery children and EXISTS expression bodies. The existing
  shape-versus-walk assertion now verifies the boolean against the same complete
  pattern population. Actual native calls inside these bodies prevent the
  ordinary fast-path proof.
- `may_have_evidence` consults the prepared plan in constant time. Explicit
  sources and shape providers remain conservative regardless of the boolean;
  absent plan and separately stored bound UDF bodies are conservative too.
  A nonempty relation registry alone no longer changes a known ordinary plan
  with no property calls and no bound UDFs. This restores the old eligibility
  condition without deleting registrations or suppressing producer metadata.
- Prepared EXISTS reuse remains restricted to ordinary unbounded execution;
  evidence preparation stays query-local. The actual query installs its full
  plan handle before evaluation, and child query evaluation installs/restores
  the appropriate plan. Retained row attachments are still checked by row
  operators independently of the reachability preflight.

No additional source defect was established in the reachability change. Existing
Both/scoped/nonempty evidence controls and top-level empty summary controls
remain required, along with the unchanged W3C answer expectations and trace
goldens. These source observations are not a passing gate or throughput result.

## Positive native EXISTS follow-up

The owner repaired the finding before execution. The sole empty constructor now
computes `closed` once and publishes undecided membership as `!closed` after
the original search combination. Because `closed` already requires the search
not be undecided, this cannot clear an existing unknown. No nonempty witness
path or top-level bag summary changed.

The native unit now checks unknown membership for incomplete/open empty search.
The actual invoked-producer matrix adds positive FILTER EXISTS, increasing its
scope to 27 executions: closed positive emptiness drops, closed NOT EXISTS
retains Supported rows, closed nested MINUS excludes, and every incomplete/open
neighbor retains undecided rows. Source reread confirms the correction addresses
the identified missing path. No material source finding remains open in these
two bounded repairs; compiler, runtime, unchanged conformance/trace and performance
acceptance remain pending.
