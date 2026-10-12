# Focused original-owner source review

## Complete index and geometry benchmark additions

Repair reread verdict: **PASS for this bounded source surface**. The blocked
buffer accounting finding below is resolved in current `covers.rs`: calibration
clones its expected literal outside measurement, captures the original receipt,
drops the last result owner and releases the receipt before the probe. The probe
and every timed iteration likewise capture, drop and release in that order, with
typed release failures unwrapped rather than hidden. Timed iterations include
the complete output lifecycle, and the buffer allocation window closes after
release. The cells index and polygon metric allocation windows remain live-output
snapshots; these are different declared boundaries and should be read that way.
No obsolete benchmark execution or new build/test/profile is claimed by this
reread. Runtime benchmark completion remains with the owner.

Bounded source verdict: **BLOCKED** on buffer-output accounting in the new
`geometry_metrics` benchmark. No benchmark execution was performed by this
reviewer; the owner's clippy result is not a runtime qualification.

`covers.rs` drops buffer `expected`, `output` and each timed materialization,
but never releases their `MaterializedOutputReceipt` through the reused
`MetricContext`. Production intentionally retains registered output allowance
across `begin` and after last result drop until
`release_materialized_output` is called. Consequently these repetitions accrue
dead output grants and receipt entries, changing the measured workload and
eventually risking a workspace refusal. Capture each output receipt, drop its
last result owner, then release that receipt in the original context. Apply this
to calibration and allocation probes as well as timed iterations; include the
timed result cleanup in the declared timed boundary. Allocation receipts should
state whether their window closes with the output live or after release.

No other concrete defect found in this bounded addition. Point-index candidate
counting uses the same positive-radius mixed cover, grid assignment and stored
level as production interval scanning. The production-adjusted work/memory
ceilings can only refuse this strict cover, not emit a different coarse answer;
disjoint subtrees make point membership count equivalent to scan candidates.
Both levels compare complete sorted accepted keys. Index construction is outside
the measured query, while cover construction, refinement and result destruction
are inside its timed query. Allocation receipts close with the result live and
report separately retained index/result bytes.

Polygon length includes its boundary under the production length law; perimeter
and complete region area call their actual APIs. Geometry preparation and the
reused context are outside the timed metric operation. Every new measured call
unwraps completion, so refusal cannot become a successful timing sample. The
offset keeps its complete prepared source under its original Arc and actual CRS.
The prior covers repair was not requalified by this review. No builds, tests,
profiles, private inputs or shipping-code edits were performed.

This is partial source evidence, not a complete408/409 acceptance verdict.
The complete requirements remain those in issue-analysis.md and plan.md.

The possible GeoSession control-admission gap is closed by current source.
compile_identity first calls admit_session_storage, which checked-adds the exact
Shared<GeoProfile> and Shared<Mutex<PreparedGeodesicCache>> allocation layouts and
retains them under the cumulative configuration budget before vocabulary and
identity compilation. try_new_in_budget allocates either control only after
that sequence succeeds. Shared owns each concrete control through final drop;
native profile payloads preserve their independently retained owners. No extra
source change or duplicate gate is required for this resolved question.

The GeometryN unmetered carrier dispatch is not the standard production path.
compute_standard's GeometryN branch uses member_metered, admitted derived
geometry writing and geographic_carrier_term. evaluate_standard routes native
source/storage admission and first-cause failures through the original observer.
The visible resident wkt_term call alone therefore does not establish a native
production admission defect.

Full mathematics, hierarchy/cover/index, public and private corpora, complete host
parity, portable/assembly, performance and mandatory gates remain unproven here.

## Scratch working-set retry refresh

Bounded source PASS for the reviewed working-tree scratch correction, including
the final inline-coefficient correction. This is not runtime qualification.
Reviewed context/numerical phase ownership, geodesic coefficient generation,
fixed distance/inverse/direct/interpolation and worker-preparation retries,
centered fallback, atlas line captures, and the original XSD scratch owner.
No shipping source was edited and no build or test was run by this reviewer.

The initial arena now uses the ordinary working set rather than multiplying
unbounded policy ceilings. Wider attempts preserve both existing arena maxima.
Coefficient generation checks available destinations before allocating its two
arrays; its request includes live destinations, four endpoints per new order,
and 64 temporary slots. The convolution retains no order-sized temporary array.
Centered fallback therefore counts the still-live original coefficients rather
than assuming that only the inner expansion exists.

One concrete review finding was corrected during this refresh: the first version
counted inline coefficient endpoints as pooled destinations. Current preflight
runs only at precision >=192. The retained coefficients are bounded by one, so
below that threshold their scaled endpoints fit three inline limbs; actual wide
arithmetic temporaries retain their existing checkout admission. Current generic
instantiations are FixedInterval and the two floating implementations; floating
contexts skip this fixed-arena preflight.

The retry signal remains LimbScratchError::Destinations through MathError and
GeoError. It does not enter the meridian/arc External-error adapters. Generic
phase retries first settle work, peak and observations, drain phase coefficient
state, drop math, and grow under the original slot/memory ceilings. Output and
cache transfer remain success-only. A latched observer failure prevents retry.
Other scratch, cancellation, storage and arithmetic failures are not classified
as growth requests. Arena replacement still refuses held destinations and
admits the new allocation while the old allocation remains charged.

The direct, inverse, distance and worker-preparation loops settle attempted work
and observations before growth. Their retained coefficient records are detached
from the scratch arena. The generic numerical boundary covers meridian,
projection and prepared-arc consumers beyond those top-level solver loops.

The original 36 phase-call sites and their production forwarding callbacks were
checked for retry captures. Attempt-local builders, sums and retained counters
are reconstructed on each call. The previously consuming atlas views inventory
now remains borrowed; transformed lines share the original PreparedValue owner.
The atlas forwarding family and CoverBudget phase callback now propagate FnMut.
No additional partially published output or captured-counter rollback defect
was found within this scope.

The new 640-bit adaptive regression retains outer order-128 arrays, requests
inner actual-order growth, compares with a pre-admitted arena, and checks a hard
slot ceiling and cumulative work. It is UNRUN at this review. The new scratch
preflight tests and all affected native/portable/consumer gates are likewise not
claimed here. Earlier tests and corpus artifacts do not qualify this new retry
implementation.

## Cover shared-width correction

The subsequent complete native collection invalidated any inference that the
scratch work was runtime-complete: `recovery-scratch-complete-kernel-xsd.log`
records 703 kernel passes and nine failures, all NumericalScratch::Retained in
cover chart/scale paths, plus 414 XSD passes. The earlier bounded source review
did not establish the lifetime-spanning width requirement at this exact-cover
caller.

Reviewed the corrective delta in cells/cover.rs::with_exact_scratch and
cells/tests.rs. Bounded source PASS: the first pooled exact result now admits
the shared initial numerical working width, using the policy precision capped
at the ordinary GEOMETRY precision, and takes the maximum with the exact cost's
required destination width. It no longer sizes the arena for only the first
scalar while returning pooled values needed by later expressions. Destination
count remains capped to the ordinary initial working set, so UNBOUNDED policy
does not request maximum allocation. Existing held-destination rejection and
old/new allocation overlap admission remain intact.

The expanded physical-scale regression compares exact results and observed work
and peak for both native grid profiles at levels 0 through 30 under ordinary and
UNBOUNDED policies. No new runtime pass is claimed: the focused log
`recovery-cover-shared-working-width.log` currently ends with a warnings-as-errors
compile refusal for unnecessary `crate::` qualification on ExecutionLimits in
cover.rs. The owner was informed of the exact correction. This reviewer ran no
builds and edited only this existing review record. Full geo acceptance remains
unfinished.
