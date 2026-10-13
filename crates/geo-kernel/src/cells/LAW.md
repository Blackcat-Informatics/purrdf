<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Native quadratic cube assignment

`CubeHilbertQ62V1` assigns exact geographic coordinates to a geodetic-normal
cube. Longitude is validated in the closed interval −180° through 180° and
latitude in −90° through 90°. Native WGS84 and CGCS2000 profiles have distinct
content identities. No datum transformation occurs during assignment.

## Frozen numerical sequence

All rounding below is half-even, symmetric about zero. Select the nearest
multiple of 90° in the original rational degree space, using half-even ties.
Subtract it exactly, obtaining a signed remainder in [−45°,45°]. Round the
absolute remainder to Q96. Independently generate `RN(π×2^192)` and round that
integer down to Q96 precision, again half-even. The angle is the single fused
rounding

```text
angle_Q96 = RN(degree_Q96 × pi_Q96 / (180 × 2^96)).
```

Square this Q96 angle with one half-even rounding. Evaluate degree-19 sine and
degree-18 cosine as Horner polynomials in that square. Each coefficient is the
half-even Q96 rendering of `(-1)^k/(2k+1)!` or `(-1)^k/(2k)!`, for k=0..9.
Every product is rounded once to Q96; fixed-point additions are exact. Restore
the remainder sign and quarter turn, then round the trig values to Q62.
Horizontal normal components are the Q62 products `cos(latitude)cos(longitude)`
and `cos(latitude)sin(longitude)`; the vertical component is `sin(latitude)`.
Exact poles bypass the polynomial and become `(0,0,±2^62)` for every longitude.

The integer generator `examples/gen_cube_constants.rs` uses no external data.
It encloses each arctangent between consecutive alternating partial sums:

```text
pi = 16 atan(1/5) − 4 atan(1/239)
atan(1/q) = Σ (-1)^k / ((2k+1)q^(2k+1)).
```

The identity follows from `tan(2 atan(1/5))=5/12`,
`tan(4 atan(1/5))=120/119`, and the tangent subtraction formula, which gives
`tan(4 atan(1/5)−atan(1/239))=1` on the first-quadrant branch. With 44 terms for
q=5 and 13 terms for q=239, the π enclosure has width below `2^-200` and both
endpoints round to

```text
RN(pi × 2^192) = 0x3243f6a8885a308d313198a2e03707344a4093822299f31d0.
```

These are 192 **fractional** bits; storage requires 194 bits. Coefficients are
generated from factorial equations rather than imported arrays.

## Complete component error

Let δ=`2^-97`, the half-unit for Q96. Exact degree reduction and the fused angle
conversion give angular error below 2δ. The rounded square is below 2/3.
Horner partials have magnitude below 3, and a product plus coefficient introduces
at most 5δ after allowing for the rounded square. The recurrence is bounded by
`5δ Σ(2/3)^j <15δ`. The final sine product and argument perturbation fit within
128δ. On the reduced interval, a conservative common trig error is

```text
E < (4/5)^20/20! + 128δ < 2^-67.
```

The sine remainder is smaller. The sharper truncation bounds at π/4 are about
`1.22615×10^-22` for sine and `3.27849×10^-21` for cosine. Let D=`2^-63`, the
half-unit for Q62. After trig rounding and horizontal product rounding:

```text
horizontal component error ≤ 3D + 2E + (D+E)^2
vertical component error   ≤ D + E
total L1 component error   ≤ 7D + 5E + 2(D+E)^2 < 2^-60.
```

This also bounds Euclidean component error. For the true unit normal n and
quantized vector q, `||q||≥1−epsilon`, with epsilon=`2^-60`, and their angular
separation obeys `sin(theta)≤epsilon/(1−epsilon)`. Because epsilon<1/4,
`theta≤2epsilon=2^-59` radians is a conservative footprint guard. A cover of
assigned physical coordinates must include that guard; ideal cube charts alone
do not describe the complete quantized assignment footprint. This also covers
dominant-face changes near chart seams.

## Faces, warp and ownership

Select the largest component in the order `x,y,z,−x,−y,−z`, retaining the first
on a tie. The raw face charts are

```text
+X=(1,u,v)   +Y=(-u,1,v)   +Z=(-u,-v,1)
-X=(-1,-v,-u)  -Y=(v,-1,-u)  -Z=(v,u,-1).
```

Warp `w=2s−1` to `sign(w)(2|w|+w²)/3`. With N=`2^30` and boundary s=k/N,
let a=`2k−N`. The boundary is exactly `a(2N+|a|)/(3N²)`. Compare a Q62 chart
ratio n/d against it by comparing `n×3N²` and `d×a(2N+|a|)`. Their magnitudes
are at most `3×2^122`, below signed-128 capacity. Ownership selects the greatest
boundary not above the coordinate; only the final outer endpoint is clamped
to its final bin. There is no inverse warp or epsilon.

## Hilbert hierarchy and store encoding

Child order is LL, UL, UR, LR. Child-local-to-parent maps are

```text
T0=(y/2,x/2)                 T1=(x/2,(y+1)/2)
T2=((x+1)/2,(y+1)/2)       T3=(1-y/2,(1-x)/2).
```

Transpose the root on odd faces. Consecutive face curves meet at
`(1,−1,−1)`, `(1,1,−1)`, `(1,1,1)`, `(−1,1,1)`, `(−1,−1,1)`,
`(−1,−1,−1)`, then the initial vertex again. Their normalized spherical
directions therefore form a continuous cycle.

A key contains three face bits, two path bits per level, one sentinel, and
`60−2×level` trailing zero bits. Valid faces are 0..5, levels 0..30. A key's
least set bit must occupy an even position no greater than 60. Zero and a
face without a sentinel are invalid. All ancestors derive from the assigned
level-thirty key.

For parent level r and stored level L≥r:

```text
s = 1 << (60−2r); t = 1 << (60−2L)
min = key−s+t; max = key+s−t; stride = 2t
logical_count = 1 << (2(L−r)).
```

Unsigned numeric order and eight-byte big-endian order coincide. Disjoint
subtree intervals appear in Hilbert leaf order. Ancestor sentinels occupy
interval midpoints, so this is not ancestor-first preorder. External-store
buckets carry profile identity and stored level before the big-endian key.
Compressed ranges retain their complete logical-cell count.

## Identity

The grid profile preimage frames the registered domain, the complete discrete
assignment law, and reduced rational semimajor axis and inverse flattening.
Policy limits, compiler/backend receipts, proof tightness and display names
do not enter it. An equivalent proof preserves point keys and profile IDs;
a change to completed assignment outputs changes the assignment law and ID.
Cover classifiers have separate identities and cannot remint unchanged point
keys.

## Physical edge scales

For exact axes a,b, set `m=min(a,b)^2/max(a,b)` and
`R=max(a,b)^2/min(a,b)`. The ellipsoid's principal radii of curvature lie in
[m,R]. Consequently, the inverse Gauss map stretches any normal-sphere tangent
between those factors, and spherical distances give lower ground bounds while
lifted spherical paths give upper ground bounds.

Along a constant-v chart edge, put t=`|2s−1|`, u=`t(t+2)/3` and z=`v^2`.
The normal-sphere speed with respect to s is

```text
D = (4(1+t)/3) sqrt(1+z)/(1+u^2+z).
```

It obeys `2sqrt(2)/3 ≤ D ≤ 7/4`. For the lower bound, `u≤t`, and the squared
inequality is concave in z; its endpoint cases follow from
`1+t^2≤1+t` and `2+t^2≤2(1+t)`. For the upper bound, D decreases with z, so
z=0 is extremal. The remaining polynomial inequality is

```text
7t^4 + 28t^3 + 28t^2 − 48t + 15 ≥ 0,  0≤t≤1.
```

Its degree-four Bernstein coefficients on [0,1/2] are
`15,9,25/6,11/8,31/16`; on [1/2,1] they are
`31/16,5/2,77/12,15,30`. They are strictly positive. The same proof applies
to the other coordinate by symmetry. A constant-v edge lies in the plane
`z=vx` through the origin, so its normalized image is a great-circle segment
shorter than a half circle. Its integrated speed therefore equals the shortest
spherical distance between its endpoints. A level-L chart edge spans `2^-L`,
giving the nominal ground bounds

```text
[m(2sqrt(2)/3)2^-L, R(7/4)2^-L].
```

The lower √2 enclosure is `floor(sqrt(2×2^192))/2^96`; its adjacent integer
numerator gives the strict upper enclosure. Integer square root verifies both
directions. Two physical endpoints can each move by the angular assignment
guard `2^-59`, so guarded metre bounds subtract/add `R×2^-58`; the lower is
clamped at zero. They bound endpoint extent, rather than the perimeter of a
potentially irregular discrete ownership set.

Scale selection compares the exact guarded upper bound with the exact metre
target at levels zero through thirty, returning the first qualifying level.
Nonpositive or unattainably small targets refuse. On native WGS84, a guaranteed
200-metre maximum selects level 16, and a guaranteed 150-metre maximum selects
level 17. Adequate execution limits cannot change that selection.

## Conservative cover footprints and classification

A level-L cell has exact dyadic chart coordinates i/2^L through (i+1)/2^L,
with the same range for j. Inverting its frozen Hilbert maps and undoing the
odd-face transpose recovers i,j. Applying the exact quadratic warp gives a
closed rectangle `[u0,u1] × [v0,v1]`. Half-open ownership only removes points
from this rectangle; closing it is conservative.

Let q(u,v) be its raw three-dimensional face chart and let n=q/|q|. Each chart
is an orthogonal permutation/sign change of (1,u,v). Along any straight chart
path, `|dn/dt|=|(I−nnᵀ)dq/dt|/|q|≤|dq/dt|` because |q|≥1. Therefore spherical
angular distance from the normalized chart midpoint to any rectangle point is
at most

```text
h = ((u1−u0)+(v1−v0))/2.
```

This bounds the complete curved chart footprint by integrating along the line
to each point; it does not rely on corner sampling. The bound shrinks uniformly
under subdivision. The Q62 assigned normal lies in that closed ideal rectangle,
even at face ties. The physical original normal is within 2^-59 radians of it
by the assignment proof above. Thus the complete physical footprint has angular
radius `h+2^-59`, including face changes, seams and poles.

The closed box classifier materializes this raw midpoint axis on the frozen
half-even 15-place degree grid. Latitude and longitude errors each contribute
at most half a degree quantum to a spherical path, so an additional
`pi_upper×10^-15/180` radians bounds its total movement, also at a pole. The
two pi endpoints written in the classifier are exact decimal neighbours at 50
decimal places; the independently generated Machin enclosure proves the true
pi is strictly between them.

## Physical-disk classification (cover law version 2)

For exact ellipsoid axes define `m=min(a,b)^2/max(a,b)` and
`R=max(a,b)^2/min(a,b)`. The inverse Gauss map's two principal radii lie in
[m,R]. Every surface curve therefore has length at least m times its
normal-sphere curve length; lifting a shortest normal-sphere arc gives a path
of length at most R times its angle. Shortest distance is a metric, so for a
centre C and any two surface points M and P,
`|d(C,P) - d(C,M)| <= d(M,P) <= R*angle(n_M, n_P)`.

The disk classifier encloses a cell's complete footprint by a cap about `u`,
the normalized chart midpoint. Its corners are normalized raw chart vectors,
and constant-chart-coordinate edges lie on great circles, so the closed chart
rectangle is a spherically convex quadrilateral on the normal sphere. Angle
from `u` is quasi-convex on it (its sublevel sets are caps narrower than a
hemisphere), so its maximum over the rectangle is attained at a corner. The
cap radius is the largest corner angle plus the `2^-59` assignment guard,
which covers every assigned physical normal including face ties.

For unit vectors with chord c, `c <= theta = 2 asin(c/2) <= c + c^3/4` on
[0,2]: differentiating the difference with `t=c^2/4` gives the sign of
`t(5+3t-9t^2)`, which has one positive root in (0,1), so the difference rises
and then falls between its endpoint values 0 and `4-pi>0`. The upper bound is
clamped to pi.

Two certified bounds then classify the whole footprint against the closed disk
of exact radius r:

* The normal-sphere bound: with `theta` the angle between the centre's
  geodetic normal and `u`, every footprint distance lies in
  `[m*max(0, theta-cap), R*min(pi, theta+cap)]`.
* The metric bound: `M` is the exact surface point whose longitude and
  latitude are the round-to-nearest binary64 midpoints of `u`'s enclosed
  longitude and latitude. The angle from `u` to `M` is at most the sum of
  those two radian enclosures' widths (cos(latitude) <= 1), so `M`'s cap is the
  footprint cap widened by that sum. A certified binary64 enclosure of the true
  shortest distance `d(C,M)` then gives every footprint distance within
  `R*cap_M` of it. The enclosure comes from the geodesic solver's binary64
  proof alone, under a fixed internal budget independent of caller limits.

The metric bound runs only when the normal-sphere bound leaves the cell
undecided and `2*R*cap_M` is narrower than the normal-sphere band. A cell is
Outside when a lower bound exceeds r, Inside when an upper bound is at most r,
and Straddling otherwise; an unresolved enclosure leaves the cell Straddling,
so it can only refine. A radius above `2^31` metres exceeds every native
shortest distance and classifies every cell Inside without conversion.

Every quantity is an outward binary64 interval built from correctly rounded
basic operations and the shared deterministic transcendental kernels inside a
validated floating chunk, and the geodesic proof is deterministic in the same
way. The classification of a cell, and therefore the completed cover, is the
same on every host; neither caller limits nor proof precision enter it. The
cover law identity frames this classifier description together with the box
classifier's, so a change to either remints every cover law.

The nominal edge-scale bounds follow directly from the warp and normal map.
On a chart edge write `w=|2s−1|`, `t=1+w`, and `u=w(2+w)/3`, holding the other
chart coordinate v fixed. Its normal-sphere speed per unit s is
`(4/3)t sqrt(1+v²)/(1+u²+v²)`. Since |u|,|v|≤1, the speed decreases as v²
increases. Its lower bound occurs at |v|=1; `u²≤w≤2w` therefore gives the
bound `2 sqrt(2)/3`. Its upper bound occurs at v=0 and is
`12t/(t^4−2t²+10)`. The assertion that this is at most 7/4 is equivalent to
`P(t)=7t^4−14t²−48t+70≥0` for 1≤t≤2. P's degree-four Bernstein coefficients
on [1,3/2] are `[15,9,25/6,11/8,31/16]`, and on [3/2,2] they are
`[31/16,5/2,77/12,15,30]`. Every coefficient is positive, proving that bound
on both intervals. Constant-chart-coordinate edges lie on great circles;
integrating over width 2^-L and applying m and R proves the two nominal
physical bounds. Each endpoint's assigned footprint differs angularly from
the nominal chart by at most 2^-59, so the triangle inequality adds/subtracts
the distinct physical guard `R×2^-58`. Nominal and guarded bounds remain
separate API values. The outward sqrt(2) lower endpoint is the exact integer
floor of sqrt(2×2^192), divided by 2^96; no host square root selects a level.

A closed box classifier first encloses the cap's latitude by axis latitude plus
or minus its angular radius converted using pi's strict lower bound. A band
reaching a pole admits every longitude. Otherwise, along the cap's shortest
spherical paths the absolute latitude never exceeds this band, and concavity
of cos on [0,pi/2] gives `cos(phi)≥1−|latitude|/90`. Dividing the angular path
length by that exact positive lower bound gives a conservative unwrapped
longitude band. Split bands at the antimeridian, retain their closed endpoints,
and compare exact closed intervals. Exact poles belong to every longitude
wedge. Meridian boxes and seam aliases are therefore retained without epsilon
or a one-cell shortcut.

Traversal starts at all six roots, discards only proved outside cells, refines
intersections below min, emits proved inside cells from min onward, and refines
straddlers through max. The shared bounded work list holds at most
`6+3×30=96` entries. Completed four-sibling unions coalesce only down to min.
One work item admits each root's classification, one each refinement (the
bounded classification of a cell's four children) and one each emitted cell,
so the work limit tracks the cover's structure rather than its leaf count.
The final cells are sorted disjoint subtrees. Cell admission is checked after
this canonicalization: mixed limits count final emitted cells, fixed levels
count logical cells, and compressed descendant ranges retain their complete
logical counts. Work, retained capacity and temporary allocation overlap are
checked before allocation/work. Successful geometry and its cover law do not
bind those limits or proof receipts.

Reported-threshold candidate covers pad the exact promoted finite threshold
by one micrometre plus its binary64 binade half-ULP. A qualifying reported
quantized distance cannot differ from truth by more than the fixed half
micrometre plus the conversion half-ULP; the larger padding is conservative.
Index refinement uses the exact corresponding reported comparator. Physical
index refinement instead resolves the unrounded inclusive distance comparison.
The immutable point index binds grid, storage level, declared native reference,
original normalized rational target coordinates sorted by caller key, and any
explicit conversion binding identity. It never infers a provider reference or
datum operation. Admission limits do not enter that source-content identity.

Admitted index assignment executes the same integer body as plain assignment.
Before each original-coordinate product, shift, quotient, comparison or clone,
the shared XSD exact-operation cost bounds its actual integer operand widths.
The remaining fixed-width products, face selection, dyadic bin trials and
Hilbert digits have a bounded 160-item reservation per assigned point.
Original integer decimal rendering is likewise admitted before framing index
content. Scratch is reserved before execution and released on success or
failure; observer refusal cannot publish a partial index. Plain assignment
keeps its integer-only, policy-free contract. Native profile digests are cached
after their identical canonical framing, so warmed caller-buffer batches do not
allocate profile parameter strings. Cache initialization, compiler identities
and admission limits do not enter the grid or index laws.

Disk entry admits the conversion of each original radius and centre rational
before its outward binary64 conversion, linear in the operand widths. Box entry
reserves original source storage before cloning exact box intervals. Box
footprint comparisons admit the actual source and generated bound widths before
products, using the same XSD rational operand-shape costs as geographic
predicates, and numerical cap phases use the shared scoped numerical admission
home, including failure work/peak accounting and observer polls between fixed
primitives. Unusually wide original rationals may therefore exhaust admission
even when their physical extent is small. Increasing adequate limits preserves
completed cells and does not enter the cover law.

One invocation retains its largest admitted source-aware integer arena across
sequential numerical phases. Growth is prepared through the shared context
home, including simultaneous old and new heap, observer admission before
allocation, and refusal if an old destination is still held. The new arena
then replaces the old owner and is borrowed by subsequent workers. Numerical
loops cannot grow it; the actual retained receipt remains separate from grid
and cover identities. A wider generated cap operand therefore causes one
admitted preparation rather than repeated per-cell arena allocation.

The complete coordinate-linear atlas carrier uses these same footprint caps.
Its lower latitude/longitude walls are rounded downward to 15 decimal places;
upper walls are rounded upward, before bounding the resulting carrier. The
clamped exact pole and seam walls remain exactly ±90 and ±180 degrees. This
outward materialization can enlarge a carrier but cannot remove an assigned
point or create a gap at an identified seam.

For axis latitude phi and one closed longitude interval, choose the nearest
of the three written axis aliases lambda−360, lambda, lambda+360. The largest
absolute endpoint difference for that alias bounds every interval point;
180 degrees is also a universal shortest-longitude bound. Take the largest
such bound over all intervals, denoted delta_lambda. Let delta_phi be the
largest absolute latitude difference from the axis to either wall. A path
along the axis parallel followed by a meridian has normal-sphere length at
most `delta_phi + c_upper*delta_lambda`, where
`c_upper=min(1,(90−abs(phi))*pi_upper/180)` bounds cos(phi) by sin(x)≤x.
The paths through the north and south poles have respective degree bounds
`180−phi−south` and `180+phi+north`. The minimum of these three complete path
bounds, converted with outward pi and multiplied by the actual ellipsoid's
exact normal metric upper factor R, encloses the entire carrier. It includes
all points between walls, rather than only corners or sampled assigned keys.
The resulting physical radius is rounded upward to the frozen micrometre
grid. This adds less than one micrometre while keeping later original-law
distance thresholds compact. The complete carrier is still enclosed.
The polar path prevents a full longitude interval from causing a fixed-width
bound near an exact pole. All walls, paths and comparisons are admitted through
the shared XSD arithmetic costs; the returned carrier's actual owned storage
is retained independently of the point-key and cover identities.

The original chart-cap equations also supply the closed-box and prepared-region
classifiers. Box containment and intersection evaluate those equations through
outward integer intervals. Only a proved comparison selects a status; unresolved
comparisons increase arithmetic precision or refuse. No wall quantization enters
the box classification law.

For a prepared region, each exact chart wall lies between two dyadic endpoints.
The outer rectangle uses the lower west/south and upper east/north bounds; the
inner rectangle uses the opposite endpoints. Complete selected-boundary exclusion
from the outer rectangle proves exclusion from the exact rectangle. A certified
intersection with the inner rectangle proves intersection with the exact
rectangle. If neither implication decides, the walls refine. This preserves the
same closed tangency, seam, pole and selected-union-boundary law while avoiding
compound rational denominators in numerical chart construction. An unresolved
gap never selects a successful straddling cell or a coarser level. All held
dyadic walls have separately admitted storage and are dropped before refinement.

Physical scale construction and level selection have pure and governed entries
into the same exact equation. The governed entries admit original rational
operand widths before each comparison, product, quotient, reduction or copy,
and admit the integer square root before constructing the outward lower square
root of two. Shared XSD arithmetic costs cover these operations; no floating
environment is required. Level selection evaluates only the guarded upper
bound, so it avoids the unused lower-bound square root. Since the prepared
ellipsoid has R>0, the nominal upper factor times `2^-L` plus the fixed guard is
strictly decreasing. Exact root and leaf tests either decide the extremal result
or bracket a failing and satisfying level. Binary search preserves that bracket
until its levels are adjacent, proving the smallest satisfying level, including
equality. To avoid repeated rational normalization, set `gap=target-guard`
once and compare the exact integers `nominal_n*gap_d` and
`gap_n*nominal_d*2^L`. All denominators are positive, so this is equivalent
to the original guarded-bound inequality even when the gap is zero or negative.
The combined bounds-and-selection entry constructs the original scale factors
once and shares them between both results. It returns the exact level-thirty
upper bound as evidence when a
positive target is unattainable.
Resource policies and proof workspace do not enter the completed scale values,
selected level, point keys or cover identity. The scale record exposes the actual
storage of its five retained quantities for cumulative host admission.
