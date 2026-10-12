# Planar query default-work failure: source diagnosis

Scope: read-only production/source and existing receipt inspection. No build,
test, timing, process modification, private-input read or shipping-source edit.

The full gate receipt `recovery-resumed-make-check-prose-fixed.log:6280–6379`
records eight failing query-rewrite cases and one passing bound join. All eight
fail on the same unlimited `sfWithin` query with the unchanged 262144 work
allowance. These are explicitly custom-planar fixtures, not CRS84 dispatch.

The actual path is `sparql-eval/src/geo/relation.rs:268–291` →
`geo-kernel/src/index.rs:737–755` → `PlanarPairs::matrix:959–971` →
`topology/relate.rs:173`. One context starts before the complete pair invocation.
`with_metadata` releases temporary workspace, not charged work; there is no
per-matrix reset. The first refusal is propagated. No duplicate work charge or
accidental context reset was established in this inspection.

The fixture has seven spatial-object entries and four distinct geometries:
five point-bearing entries and two aliases of the square. The complete Cartesian
answer computation (`index.rs:809–815`, `holds_relation_with:654–674`) therefore
attempts 49 matrices: 25 point/point, 20 point/polygon in both orders and four
square/square. Feature/default-geometry aliases recompute identical matrices.
There is no relation-specific early predicate or shared result reuse here.

Every matrix takes the generic body at `topology/relate.rs:273–319`: collect
edges/seeds; intersect every pair of collected edges; sort/deduplicate events;
locate every event against both inputs; fragment every edge; construct and locate
its midpoint; scan remaining area targets. A point/polygon pair thus nodes and
classifies the entire polygon. A square/square alias pair supplies the same
boundary twice. The passing bound join avoids most Cartesian pairs. The original
main implementation also used complete pair enumeration, but its resident
topology did not execute this new admitted arithmetic protocol.

Existing axis repairs are present: orientation reduces to admitted ordinate
comparison, perpendicular intersections preserve original ordinates, and
vertical crossing abscissae use an admitted copy. Those are not missing fixes.
One remaining concrete redundant operation is `topology/segment.rs:394–406`:
midpoint always performs addition and multiplication by one half for each
ordinate, including an axis edge's unchanged ordinate. An admitted equality
check followed by the original copy factory would remove actual arithmetic for
that ordinate. The scan mean at `relate.rs:386–396` has the same arithmetic shape,
but its sorted distinct interval endpoints generally differ. This inspection
does not establish that midpoint elimination alone fits the default allowance.

The existing receipt lacks per-pair counters or the first refused arithmetic
call. The earlier `recovery-planar-final-cause-probe.log` concerns a different,
single collection/polygon matrix; its first-band refusal cannot be assigned to
this query. Before selecting the repair, the narrow focused diagnostic is to
record context work before/after each actual `PlanarPairs::matrix` invocation,
plus the first refused operation and requested charge, while running the exact
unchanged failing query. That distinguishes repeated full-matrix overhead from
a dominant single pair. Remove the diagnostic after the production correction.

Preserve all original query answer/prefix/dedup/join assertions, the cumulative
invocation budget, admitted comparison/copy/refusal ordering and all matrix
goldens. Calling `begin` per pair, raising the default, changing these fixtures
to an unbounded policy, or truncating computation at a row ceiling would conceal
the failure rather than repair the computation. Consumer qualification remains
failed until the original nine cases and affected kernel acceptance execute.

## Actual pair probe and bounded reduction design

`recovery-planar-pair-probe.log` now identifies the computation: 20 completed
matrices precede refusal in pair 21 (square/square), at cumulative 262143 plus
16 requested work in `noding::fragments_inner`'s admitted rational comparison.
Point/point costs 323; point/square costs about 17960–18674; square/square costs
54377. The whole fixture would require roughly 580000 at those costs. This is
repeated topology computation, not a small last-operation overrun. Midpoint
elimination alone is not established sufficient.

Two compatible reductions at the existing matrix home have a source proof:

1. **Complete-matrix saturation for a singleton point operand.** If A is a
   nonempty single Point, locate its coordinate in B once. Its I row has exactly
   one Zero and its B row is entirely Empty; E/E is Two. Only E/I and E/B remain.
   Conservative dimension ceilings come from B's leaf kinds: interior at most
   Two for polygons, One for curves, Zero for points; boundary at most One for
   polygons, Zero for curves, Empty for points. Collections take the maximum;
   their boundary-parity combination cannot exceed these ceilings. Obtain the
   remaining entries from the original admitted witnesses and stop only when
   they actually attain their ceilings. Apply the transposed law when B is the
   singleton. No polygon-validity or positive-area assumption is licensed.

   The current area scan runs after all fragment classifications. To save those
   classifications, the point case must scan area after complete noding but
   before the remaining event/fragment witnesses, then check saturation as
   witnesses arrive. Matrix raises are maxima, so order is immaterial to the
   answer. Keep the singleton coordinate among original noding seeds: this
   prevents a fragment midpoint from being the singleton and spuriously raising
   its row to One. If any ceiling is unattainable (empty/degenerate/self-crossing
   geometry or parity cancellation), execute the original remaining witnesses;
   absence of a maximum is not proof of an empty cell. This is a complete-matrix
   stopping proof, not a relation-only shortcut.

2. **Deduplicate identical undirected XY edge supports in the matrix's witness
   array.** `relate_inner` already seeds every leaf coordinate, including all edge
   endpoints. Removing duplicate/reversed supports therefore removes duplicate
   intersections only: coincident overlap endpoints remain original seeds;
   intersections with a third edge are still produced once. Existing event
   sorting/dedup yields the same event set, midpoint witnesses repeat the same
   support, and scan crossings already deduplicate abscissae. Canonical endpoint
   order/equality must use admitted XY comparisons, with array lifetime under
   the existing owner. Do not deduplicate original leaves, rings or geometries:
   those retain the multiplicity consulted by point-location boundary parity.
   Keep this at `relate_inner`'s witness boundary unless another noder caller
   proves the same complete-endpoint seed contract.

The second reduction addresses four costly square/square alias computations;
the first targets the twenty point/polygon matrices. Neither requires an index
representation change, new memo table, quota reset or raised allowance. This is
design review only: no implementation or sufficient measured saving is claimed.
The actual patch must retain full matrix differentials, invalid/degenerate and
collection controls, original nine query cases, and admitted owner/refusal laws.

## Actual topology patch source review

Reviewed the written `unique_edge_supports` and `PointTargets` changes in
`crates/geo-kernel/src/topology/relate.rs`, without building or executing them.
No material complete-matrix or ownership defect found in this bounded patch.

`PointTargets` initializes only the proven singleton row/column using the actual
location result. It takes conservative maxima over the opposite leaves; empty
and degenerate members retain loose ceilings and therefore cannot authorize an
incorrect early return. All original coordinates remain noding seeds. The same
admitted area scan runs before classification for this case, and events/fragments
stop only when both remaining cells attain their upper bounds. The transposed
case reads the corresponding opposite cells. A point/point equality, collapsed
line, empty polygon or collection parity cancellation that cannot attain its
ceiling simply executes the remaining original witnesses. Nonpoint cases retain
their original classification/area order.

`unique_edge_supports` canonicalizes borrowed endpoints by admitted XY comparison,
sorts and partitions the original retained edge buffer, and truncates references
without creating another allocation owner. Reversed or equal-XY supports are
equivalent even when Z/M differ, since every topology computation projects XY.
Original leaves/rings remain untouched for location and mod-2 boundary semantics.
The endpoint-seed proof above applies at this actual call site before noding.
Working events, fragments and edges still release through their existing owners,
including fragment release after saturation. No budget reset or increased limit
appears in this patch.

The remaining reduction uncertainty is concrete: for point/polygon, the patch
still classifies every event before its first fragment because Zero witnesses
cannot saturate the polygon boundary's One ceiling. Since the singleton row is
already exact, processing fragments before events in this point case could avoid
those classifications after saturation; otherwise events can run afterward as
fallback. That is an optional additional computation reduction if the measured
current patch still fails, not a correctness requirement or a sufficient-saving
claim. Tests were still being added at review time. Complete original matrix
goldens, duplicate/reversed supports with retained collection parity, invalid and
degenerate point cases in both orders, physical/refusal checks and the nine
unchanged query-rewrite cases remain runtime acceptance.

## Final singleton-dimension delta: adversarial source readback

Reviewed the current written `relate.rs` and the expanded existing owner test
after fragment-before-events, outside-band removal and singleton-dimension
reduction. The last singleton reduction is not covered by the previously reported
713-kernel pass. No build or execution was performed by this review.

No material proof or ownership defect found in the current source. The newer
proof deliberately supersedes the earlier requirement to keep the singleton in
the noding seed set: that requirement applied while midpoint/area classification
used both literal point locations. The current `locate_positive` explicitly
assigns the singleton side Exterior for One/Two witnesses. Removing a singleton
from a nonempty open fragment or face cannot lower its positive dimension, even
when the chosen representative equals that singleton. The other operand's
location is still evaluated normally. Its complete vertices, isolated points
and self-intersections remain noded, so its location is constant on each open
fragment/face before that one-point deletion. This applies equally to invalid
rings, collapsed components and the original collection parity semantics.

Initial exact point location fixes the singleton interior row/column. Its
boundary is empty. `classify_events` retains both exact locations, so an isolated
other-operand event equal to the singleton never contributes spuriously to the
singleton's exterior. Both singleton operands choose the first operand for this
reduction; the other point is still an event and equality/distinctness remains
correct. Loose dimension ceilings cannot authorize a premature stop on
degenerate/empty or parity-cancelled cells.

The fragment-first order changes only maxima accumulation; events remain a
fallback whenever ceilings are not attained. Outside ordinate sentinels were
removed at `band_boundaries`: every finite segment lies between its endpoint
ordinates, and polygon interior cannot occur outside that complete ordinate
range. Such outside bands can only supply E/E=Two, already initialized. Fewer
than two distinct ordinates cannot contain a two-dimensional face. The same
argument holds after singleton removal because deleting one point changes no
area. Retained bands are returned directly; insufficient-band and all normal
array paths retain the original release protocol.

The direct budget control `all_small_map_pairs_complete_in_one_default_work_budget`
uses the seven spatial entries with multiplicities 2/2/2/1, all 49 ordered pairs,
one unchanged default MetricContext, and no per-pair begin/reset. Its explicit
matrix strings cover equal/distinct points, inner/outer point versus square,
their transposes and square self-relation; the strings agree with those set
intersections. The bare center point also equals the first square area sample.
This is the correct direct computation control, but does not include index
reference preparation, row charges or other query plumbing; the original nine
query cases remain necessary after it passes.

The new line-midpoint golden has singleton (2,0) on line (0,0)–(4,0), directly
exercising a positive-dimensional representative equal to the singleton. The
additional collapsed polygon, collinear polygon, bow-tie interior/crossing and
reversed duplicate-curve cases check explicit matrices through both resident
and admitted doors in both operand orders. The existing retained-owner test now
adds both point/square orders, with peak/live/retention accounting, zero-capacity
refusal before allocation, and completed-work-minus-one exhaustion after real
work, followed by zero retained bytes/live owners. These additions close the
previously identified focused coverage gaps at source; execution is pending.

No temporary PHASE, PLANAR_REMAINING, probe_work, PLANAR_PAIR or PLANAR_REFUSAL
marker remains in the inspected matrix/index/context sources. Sufficient work
reduction is still a measurement, not established by this source verdict.

## Local mean simplification after the direct budget refusal

The parent reports the direct 49-pair control still refuses at pair 3/2 in its
final bare-point row, with 262134 work consumed. This is not a passing control;
no dependent query success is inferred. Read-only review of the proposed next
correction found its arithmetic/admission law sound:

- Route fragment X and Y means through the existing matrix-local `mean`, removing
  the now-obsolete trait midpoint methods and imports. The shared segment helper
  and its other callers remain untouched.
- An admitted comparison proving `a == b` permits `(a+b)/2 = a`. Produce that
  value with the existing admitted Linear/original-copy factory; do not borrow or
  clone around the owner/admission path.
- Canonical rational zero is a shape check, not a hidden limb traversal:
  `GeometricRational::is_zero` delegates to the canonical numerator predicate.
  If one operand is zero, multiply only the other by the exact half constant.
  Supply both actual operands to RationalMultiply admission. The absent addition
  is an eliminated computation, not uncharged arithmetic.
- Unequal nonzero operands retain the original admitted addition followed by
  multiplication. Each surviving comparison/copy/arithmetic operation retains
  pre-body refusal and native ownership. The resulting XY sample is exactly the
  original midpoint, including negative, rational, zero and equal coordinates.

This review concerns the proposal; the source at read time still had the old
trait midpoint and unconditional mean. No compiled result or sufficient saving
is claimed. Keep the direct cumulative control as the next computation check
before rebuilding dependent query targets. Confinement to `relate.rs` preserves
the earlier public/private corpus applicability result.

## Implemented shared mean: bounded source verdict

The final implementation supersedes the matrix-local proposal above. Source
readback found no material defect in the consolidation: `segment.rs:402`
`mean_inner` is the sole inspected exact scalar arithmetic body;
`midpoint_inner`, both `RelateWork::mean` adapters, planar arrangement `average`
and atlas cut-interval `midpoint` route through it. The old equivalent bodies
and obsolete relation midpoint methods are removed. `topology/mod.rs` exposes
only the crate-private seam needed by these callers.

The equal branch performs the admitted rational comparison, then the existing
Linear/original operation. `ExactValues::original` uses scratch `copy_in` on
the admitted path; it does not smuggle an unpriced clone or borrowed result
through admission. For unequal operands, a canonical zero avoids only the
addition; the multiplication descriptor includes the actual borrowed operand
and exact half. Otherwise the previous addition/multiplication computation
remains. All rational signs and widths preserve `(a+b)/2`; coordinate callers
still discard Z/M. Replacing planar arrangement division by two with
multiplication by an exact half preserves that exact value while correctly
pricing the operation now performed. Owned intermediate sums remain alive
through their admitted multiplication and are released on return/refusal.

The re-read singleton reduction still distinguishes positive-dimensional
witnesses (force singleton Exterior, locate the other operand) from event
witnesses (locate both). Original location semantics retain collection parity
and invalid/degenerate handling, with unattained conservative ceilings falling
back to complete traversal. The midpoint-equals-singleton and area-sample-
equals-singleton controls, explicit matrices for all 49 ordered fixture pairs,
and retained-owner/refusal checks continue to cover the identified hazards.
No new proof gap was found in this delta.

This is a source verdict, not a passing work-budget or runtime result. The
parent is executing the direct cumulative budget, topology, ownership and
determinism checks. No test/build/process action was performed by this review.
The original query acceptance remains necessary even if the direct matrix
control passes. The broadened polygon/atlas acceptance implications are
recorded in `recovery-planar-corpus-applicability.md`.

Final adapter follow-up: the atlas relation scan now shares the same
`segment::mean_admitted` context/progress adapter as planar arrangement.
Both use imports rather than duplicate wrapper bodies. The admitted relation
worker and atlas cut-field adapter call that same home; resident/coordinate
callers retain `mean_inner`. Source readback found no material defect. The
reported predecessor 714-case kernel pass does not qualify this final adapter
delta; current topology/atlas execution is pending. See the applicability
review for the precise preserved public/private scope.
