# Applicability of retained corpus evidence after planar matrix reduction

Scope: the production delta is confined to `geo-kernel/src/topology/relate.rs`
(`PointTargets`, support deduplication and witness order/stopping). This review
read production source, the public driver and the sanitized Stage driver
`owned-private-probe/main.rs.txt`. It did not read private acquisition bytes,
private raw results, or run any build/test/process action.

## Public corpus

`crates/geo-kernel/examples/geodesic_corpus.rs:135,189,226,292–319,356–367` calls
`PreparedGeodesic::prepare`, `inverse_with_proof`, `distance_with_proof`, `direct`
and `distance`. The distance mode and all-mode inverse/direct/witness paths run
the geodesic/numerical homes; they do not construct a planar matrix or call
`topology::relate`. The changed private matrix helpers are reachable only through
that matrix body. The completed public 500000-row correctness receipts therefore
remain applicable to these unchanged computations. This does not relabel them
as executions of a newly linked artifact or qualify new-artifact timing.

## Ongoing private driver

The sanitized driver's actual production calls are:

- Exact WKT spelling/reparse and `PreparedGeometry::from_literal_in_policy`
  (`852–884`), with identity comparison.
- Ellipsoidal length and perimeter of the full original MultiLineString
  (`885–902`), through `prepared_length` and geographic atlas boundary selection.
- Explicit coordinate-operation forward, inverse, axis and residual calls,
  plus `OperationChain::materialize_geometry` for original line segments
  (`790–834`, `953–1026`).
- Cell assignment/hierarchy, mixed box covering and membership, immutable
  `PointCellIndex`, and physical zero-radius search (`908–951`, `1035–1093`).

There is no planar relation/index query or `relate` invocation. Geographic atlas
boundary and operation code can use shared planar noding, segment orientation,
intersection and location helpers, but those are unchanged and do not call back
into `topology/relate.rs`. For example `atlas/boundary.rs:143,152,521` calls
`fragments_admitted`, `midpoint_admitted` and `events_admitted`; support dedup is
local to the matrix body, not inserted into those helpers. The private driver's
correctness scope therefore remains applicable across this bounded change.
There is no cause here to restart its valid ongoing binary. Its eventual receipt
must retain the captured artifact identity and actual completed population;
partial progress remains incomplete, and no current planar/timing acceptance
is inferred from it.

## Actual planar ownership/refusal coverage

`context/retained.rs::native_planar_matrix_prices_actual_arrays_and_limb_owners_until_destruction`
(`1639–1690`) calls `topology::relate_metered` on a collection containing a holed
polygon and a line against a polygon. It checks the complete matrix, positive
observed peak, allocator peak covered by admitted ownership, no retained bytes,
zero live owners and no remaining heap scratch. Its second call denies storage
at limit zero, requiring the original `AdmissionFailed`, zero allocations and
zero surviving owners. This covers the actual native matrix allocation route,
but the refusal occurs before the new support sort and the input cannot use
singleton saturation.

`topology/relate.rs::native_matrix_owner_preserves_every_original_golden_and_relation_family`
(`624–648`) calls `relate_metered` across the matrix corpus and compares all
relation families. The resident golden and new degenerate/parity tests protect
answers but are not by themselves physical/refusal tests. Similarly named
`atlas::relate_metered` tests concern the separate geographic matrix home.

The focused gap is the new singleton early-stop lifecycle: extend the existing
owner test's inputs with a point/polygon case that attains the ceilings and
verify the same released-array/peak/zero-retention law. A bounded refusal after
work has actually begun (work exhaustion or observer cancellation after support
ordering) should also establish that the first error and all admitted owners
survive/settle correctly on that changed path. Reuse the existing observer/owner
test machinery; no new independent accounting implementation is needed.

Planar kernel/query acceptance and planar determinism consumers are affected
and require current-source execution. In particular `determinism.rs:167` invokes
the changed matrix and `index.rs:967–969` dispatches actual planar query pairs
to it. The retained geodesic/private correctness evidence does not substitute
for those checks or the mandatory full gate.

## Proposed shared scalar mean: one-home and applicability follow-up

The matrix-local optimized mean and the existing segment midpoint's add/half
body implement the same exact rational job. No distinct numerical contract
justifies preserving both. The general one-home rule applies even though the
ledger has no named scalar-mean job. Prefer one scalar mean body at the existing
`topology/segment.rs` midpoint home with optional ExactAdmission; compose both
coordinate ordinates and the relation worker's typed mean adapters through it,
and delete the matrix-local arithmetic body. This is a source recommendation,
not an implementation or gate result.

Two further observed same-law bodies are
`topology/arrangement.rs::average:383` (add/divide-two) and
`atlas/arcs/arrangement.rs::midpoint:1776` (add/multiply-half). Their context/
progress arguments can use the same admitted scalar mean. Do not claim a single
mean home while retaining known equivalent arithmetic bodies. FixedInterval and
FloatInterval midpoint methods implement different rounding/enclosure contracts
and are outside this exact-rational consolidation.

Changing segment midpoint broadens affected callers, but the current sanitized
private driver's exact shapes still do not reach it. Its prepared carrier is
MultiLineString (`main.rs.txt:860`), handled as curves at `prepared.rs:2271` with
no polygon insertion; `PreparedRegion::from_admitted` produces Empty at
`2040`, stored at `2345`. Ellipsoidal `prepared_length` calls `region_boundary`,
whose Empty arm (`atlas/boundary.rs:31–42`) returns before the midpoint path at
152. The other segment midpoint caller is `atlas/relate.rs:266`, and the driver
executes no geographic relation. Its materialized inputs are LineStrings, and
the operation/cell code has no call to segment midpoint. The public driver uses
geodesic arithmetic, not this midpoint body. No private input or raw result was
read to establish these shape/control-flow facts.

Consequently existing public correctness receipts and the private ongoing binary
remain applicable if the change is the described shared scalar mean. Geographic
polygon metric/boundary and atlas relation checks do reach the changed segment
helper and need current-source refresh; the earlier matrix-only applicability
statement must not be used to preserve those affected checks. Additional mean
caller migrations require their own concrete call-path readback before widening
this reuse statement.

## Concrete reachability of the two additional mean migrations

Further source trace confirms neither additional exact-mean body is reached by
the current sanitized private driver:

- `topology::arrangement::average` has its two call sites inside the slab
  decomposition (`arrangement.rs:198,252`). Its production callers are
  `atlas::arrangement::decompose_inner:168` and
  `operation::geometry::critical::visit_source_cells_inner:311`.
  The atlas decomposition returns immediately for PreparedRegion::Empty
  (`atlas/arrangement.rs:97–99`); the driver constructs multiline carriers with
  precisely that region and requests length/perimeter, not area. The operation
  route is `folded_area → visit_source_cells`, dispatched only for Polygon or
  MultiPolygon (`operation/geometry.rs:759–775`), or `visit_exact_area_cells`,
  called by `binding/prepared_image.rs::cell_polygon:657`. The driver's
  materialized sources are LineString segments and dispatch to `self.line`
  (`operation/geometry.rs:754`), never either areal route.
- `atlas::arcs::arrangement::midpoint` has one call in
  `relation::witness_graph:1504`. That graph is entered through
  `witnesses_with`, either from `relation_matrix` or the selected-strata output
  for native boundary construction (`relation.rs:287,570,1437–1448`). The
  driver's empty areal region exits `region_boundary` before native boundary
  construction, and the driver executes no prepared-geometry distance or
  relation query. Its physical point-index search holds a `PreparedGeodesic`
  (`cells/index.rs:170`, prepared at `321–323`) and refines point distance there;
  it does not invoke the geometry relation graph.

These shape gates derive from the sanitized program itself, independent of
private coordinate values or acquisition contents. Therefore consolidating the
two identified means into the shared segment home does not invalidate the
ongoing private driver's exercised correctness computations, nor the completed
public geodesic corpus. Keep their original binary identities and completion
status. Affected polygon decomposition, transformed areal-image, native boundary
and geographic relation tests do require current-source acceptance. No build,
test, private-input/raw-result read or process action was used in this review.

## Implemented consolidation readback

The actual formatted delta now matches the traced proposal: segment scalar
`mean_inner` owns the exact arithmetic; segment coordinate midpoint, relation
workers, planar arrangement average and atlas cut midpoint call it. The two
arrangement adapters retain the original context/progress and exact operands.
No additional consumer or algorithmic home was introduced. Consequently the
public geodesic/private-driver applicability conclusion above survives this
actual delta, while polygon decomposition, transformed areal images, native
boundary and geographic relation acceptance must be refreshed against current
source. Existing partial private completion remains partial; source reachability
does not promote it to a pass. Runtime checks underway in the parent were not
run or claimed by this review.

## Additional atlas relation witness adapter and scope correction

The parent identified `atlas/relate.rs::average:461–476`, still implementing
exact add/divide-two at readback. Its two calls are the open-face height and
longitude representatives in `scan_faces:599,624`; the caller at line 246 is
the prepared-geometry relation computation. Both operands are ordinary exact
Rat chart coordinates. This is the same scalar mean law and the same
context/progress ExactAdmission contract as the touched witness adapters.
Routing it through `topology::mean_inner` preserves its representative exactly,
including negative or zero longitudes. This is not a circular angular midpoint
or a rounded interval midpoint; the existing chart already supplies the ordered
open-face bounds. The new shared body must retain actual operation admission,
as reviewed above. At this readback the replacement was planned, not executed.

The sanitized private driver executes no prepared-geometry relation, so this
additional adapter does not change the prior private/public applicability
conclusion. Geographic relation acceptance remains affected and requires
current-source verification.

Scope clarification: the consolidation verdict covers the touched planar and
atlas relation/arrangement witness adapters and coordinate midpoint only. It
does not claim that every arithmetic mean in the kernel now has this home.
The bounded search also encountered unchanged inline subdivision, certification,
centroid and other arithmetic with MathContext or retained-owner contracts;
those are outside this planar-query repair. No migration of those paths is
recommended by this bounded review. In particular the sanitized private driver
does call LineString materialization, so the earlier preservation argument must
not be extended to a hypothetical operation-subdivision change. Search stopped
at the parent's explicit scope boundary. No builds, private-input/raw-result
reads or process actions were performed.

## Final adapter consolidation readback

The final source now implements the additional atlas relation migration.
`atlas/relate.rs:13` and `topology/arrangement.rs:9` import the common
`mean_admitted` as `average`; neither retains its former wrapper/arithmetic
body. `segment.rs:402` owns the single context/progress adapter, constructing
ExactAdmission and delegating to the unchanged optional-admission arithmetic
at `mean_inner:412`. The admitted relation worker calls that adapter directly;
the atlas cut midpoint adapts only its two Cut endpoint fields. Coordinate
midpoint and the resident relation worker keep the same arithmetic home.
No duplicate witness-mean adapter remains among these inspected paths, and
no ownership, operand, or error-propagation defect was found in the final delta.

The scoped applicability conclusion is unchanged. The parent reports 714
kernel cases passed the predecessor arithmetic, but that receipt precedes
the atlas average migration and is not a pass for this final source. Current
topology/atlas checks remain pending the parent's next run. This final readback
performed no tests/builds and did not broaden the review scope.
