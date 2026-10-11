<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Certified geodesic arithmetic

The geographic kernel retains exact source rationals and evaluates one original
auxiliary-sphere solver through XSD's controlled binary64 and bounded fixed-point
intervals. Both arithmetic paths enclose the same mathematical quantities.
Their coefficients are generated from equations. No external geodesic runtime,
provider coefficient table or platform transcendental implementation is used.

## Source values and arithmetic ownership

WKT and GeoJSON numbers retain their original decimal value. Finite binary64
input is decoded from its sign, significand and exponent into an exact dyadic
before range validation. Longitude and latitude are validated as original
rationals. Exact poles, zero coordinates and seam aliases therefore remain exact
decisions even when a nearby source rounds to the same host double.

`purrdf_xsd::integer` owns integer arithmetic;
`purrdf_xsd::rational::Rat` owns normalized rational arithmetic and conversion.
The geographic `Int` and `Rat` names reexport these types. XSD's IEEE layer owns
binary64 decoding, endpoint rounding and floating-environment control; its math
layer owns intervals, elementary functions, Taylor arithmetic and root isolation.
The geometry modules contain no second arithmetic backend.

Floating entries check rounding mode, gradual underflow, masked exceptions and
excess precision before arithmetic. Scoped control guards stay on their creating
thread and restore its state. Numerical chunks release their guards before a
governor callback and revalidate afterward. Integer-only cell assignment,
hierarchy and encoding do not require a floating environment.

## Original integral construction

Let `c=b/a`, `e²=1-c²`, `e′²=e²/c²`, and let `s=sin(alpha0)` and
`C=cos(alpha0)` be the conserved auxiliary-sphere line constants. Reduced
latitude satisfies `tan(beta)=c*tan(phi)`. Set

```text
q = e′² C²
h(sigma) = sqrt(1 + q sin²(sigma))
ds/dsigma = b h(sigma)
d(lambda-omega)/dsigma = -e² s / (1+c h(sigma))
```

The last expression follows by rationalizing the difference of the ellipsoidal
and spherical longitude derivatives. Its denominator stays positive on an
oblate ellipsoid, including meridian-pole limits. This avoids separately
integrating two singular longitude terms. The auxiliary-sphere relations and
Jacobi quantities follow the mathematical construction in
[Karney, *Algorithms for geodesics*](https://arxiv.org/pdf/1109.4448).

For the native small-eccentricity path, expand `sqrt(1+z)=sum(u_n z^n)` and
`1/(1+c sqrt(1+z))=sum(v_n z^n)`. The implemented generators use

```text
u_0 = 1
u_n = u_(n-1) (3-2n)/(2n)
v_0 = 1/(1+c)
v_n = -c/(1+c) sum_(k=1..n) u_k v_(n-k)
```

The integral of each `sin^(2n)(sigma)` term follows integration by parts from
the previous term and endpoint sine/cosine products. Close endpoints use
factored angle-difference identities, preserving correlated small differences
instead of subtracting independent large endpoint values.

For `0<=q<1`, the square-root, reciprocal and Jacobi coefficient magnitudes are
bounded by one. The reciprocal bound follows from its holomorphy and modulus
bound on the open unit disk, taking Cauchy radii toward one. Consequently the
omitted integral after degree `N` is bounded by
`abs(delta_sigma)*q^(N+1)/(1-q)`. Arithmetic rounding is enclosed separately.

The large-eccentricity evaluator rewrites the integrand with
`A=1+q/2`, `k=q/(2+q)` and `sqrt(A)*sqrt(1-k*cos(2*sigma))`.
Here `k<1` for every finite nonnegative `q`. A Cauchy circle strictly between
`k` and one gives the remainder; the evaluator increases the admitted order
until that bound meets its proof target. It does not estimate error from the
difference between successive quadrature results.

Area coefficients are likewise original: the analytic function
`t(z)=z+sqrt(1+z)*asinh(sqrt(z))/sqrt(z)` has removable value one at zero and
satisfies `2z(1+z)t′=1+4z+2z²-t`. Ordinary or positively centered coefficient
recurrences follow this equation, with a separate Cauchy remainder.

## Elementary functions and solver decisions

Pi is enclosed by Machin's identity using bounded alternating rational series.
Sine and cosine use exact quarter-turn reduction and factorial recurrences;
Taylor remainders cover the complete reduced interval. Square root uses exact
integer roots. Arctangent uses monotone endpoint evaluation, reciprocal and
half-angle reduction; logarithm and exponential use admitted series with
independently bounded tails. Broad trigonometric intervals use the global
derivative bound and clamp to `[-1,1]`. Exact zero returns exact sine zero and
cosine one.

Inverse initialization uses spherical, short-range and antipodal constructions.
These supply trial values to a safeguarded bracket and certified interval Newton
contraction. A local iterate never constitutes a completed answer. Unresolved
branch comparisons, singular limits or output rounding refine the original
sources through higher precision, or return a typed operational refusal.

Monotone coordinate inverses also distinguish source-family width from their
arithmetic evaluation floor. For a proved contraction `T` with derivative bound
`L<1`, the map enclosure at one exact dyadic midpoint supplies an independently
computed evaluation width. Division by `1-L` transports that width through the
inverse conditioning bound. The solver can retain a certified enclosure when
this floor prevents further contraction; final coordinate rounding and forward
residual certification still decide completion. A fixed number of arithmetic
grid steps cannot require a nonzero evaluation enclosure to disappear. This
bound uses the map equations and a global derivative bound, rather than a
difference between successive approximations.

Direct propagation contracts its monotone distance equation. Its derivative lies
in `[b,a]`; for `q=0` it is identically `b`, so the phase is exactly
`sigma1+distance/b`. A selected arc retains the original endpoint or
azimuth/length law. Its unrounded interval view supplies lifted longitude,
latitude, smooth geodetic normals and derivatives; it does not reinterpret
rounded inverse outputs as the exact source arc. Every monotone initial bracket
and outward Newton intersection encloses all retained source parameters. If
uncertainty in the inverse azimuth or initial phase prevents further contraction,
the interval view keeps that proved bracket. Its caller then proves the required
geometric decision or requests tighter source evidence; a distance-only width
target cannot discard a valid source image.

## Cartesian normal through a transformed pole

The geocentric inverse solves its unique latitude and height family before
choosing a longitude chart. For Cartesian radius `r`, the global fixed-point
derivative bound is `cr/r`, with `cr=e²*a/(1-e²)^(3/2)`. A complete source box
must prove `r>cr`. Sign-certain latitude families retain odd reflection; a family
crossing the equator uses the same signed map on `[-pi/2,pi/2]`. The original
height equation `h=p*cos(phi)+z*sin(phi)-a*sqrt(1-e²*sin²(phi))` remains regular
at both poles. A geographic coordinate response then selects its canonical
longitude, while a physical normal request omits that chart operation.

For `nz=sin(phi)`, set `w²=1-e²*nz²`, `N=a/sqrt(w²)` and
`M=N*(1-e²)/w²`. The unique normal is
`n=(x/(N+h), y/(N+h), nz)`. Its Cartesian differential is

```text
P = I - n*nᵀ
w = (-ny, nx, 0)
C = -N*e² / (w²*(N+h)*(M+h))
Dn = P/(M+h) + C*w*wᵀ.
```

Away from a pole this follows from the principal meridian and parallel
derivatives `1/(M+h)` and `1/(N+h)`. Their difference is
`-N*e²*(1-nz²)/(w²*(N+h)*(M+h))`; it cancels the denominator of the east unit
projection exactly. At a pole `w=0` and `M=N`, leaving the finite isotropic
horizontal derivative. The same original chain traversal propagates its actual
Cartesian source derivative through this matrix. No sampled longitude or
rounded geographic carrier supplies a normal, and no longitude `atan2` is
evaluated for this final inverse-geocentric normal goal. All family, matrix and
source operations use the existing admitted interval and limb homes.

## Completed values, proof receipts and branches

Point distance completes only when both endpoints of the certified true-shortest
distance enclosure round to the same micrometre by half-even rounding. Its
public error is the fixed half-micrometre bound, with the separately stated
half-ULP host-conversion bound. An acceptable approximate distance alone cannot
complete an unresolved rounding decision.

Finite printed reference data supplies regression evidence; it does not decide
an exact rounding boundary. When a corpus is constructed from exact forward
inputs, its printed endpoint coordinates still have finite uncertainty. Near
conjugate points, tiny endpoint changes can produce large changes in inverse
azimuth and quadrilateral area. Endpoint propagation checks therefore accompany
nominal metadata comparisons, and the certified original-input enclosure remains
the numerical contract. The
[author's test-data description](https://geographiclib.sourceforge.io/2009-03/geodesic.html)
explains this conditioning problem for inverse azimuths.

Direct coordinates default to fifteen decimal degree places. Explicit grids
must satisfy the same physical error certificate or refuse. Proof precision
never chooses an output grid. Canonical longitude is `[-180,180)` and is zero
at an exact pole; original source coordinate values remain in the retained source.

Inverse azimuths use `[0,360)`. Zero distance has absent azimuths and zero arc,
reduced length and area. Tied shortest branches select the smallest forward
azimuth and then final azimuth. Separate multiplicity evidence prevents an
endpoint-only arc from treating that canonical choice as a unique branch.
The signed geodesic quadrilateral area is `+integral(Q(phi) d(lambda))` in its
declared continuous lift and endpoint-pole frame. Oriented region boundaries
apply their own atlas cut and winding law.

Completed certificates bind the mathematical output law and actual reference.
Execution limits have a separate policy identity. Tighter invocation enclosures
belong in proof receipts; an equivalent backend or stronger proof does not
remint an unchanged completed law.

For a direct angular quantum `q=10^-places` degrees, the original normal-metric
upper radius `R` bounds the two coordinate half-quantum errors by
`R*(22/7)/180*q` metres. Every public direct grid, including fifteen places,
must prove this is at most one micrometre on its actual ellipsoid. The grid
never changes during proof refinement. A private prepared invocation can retain
this proof together with both admitted reference bindings, then verify the
initialized binding before each sample of the unchanged direct body.

## Scratch lifetime and admission

Immutable prepared coefficients and arithmetic tables can be shared between
workers. Each worker owns its context and reusable limb arena. Work, retained
tables, arena heap, destination capacity and output storage are checked before
construction; external governors observe the admitted count before allocation.
Meridian integrals reuse these same generated tables. Their cache key includes
the exact normalized inverse flattening, precision and expansion order; overlap
between numerical enclosures cannot make different ellipsoids share a key.
Dimensionless tables can serve different semimajor axes with the same flattening,
while each integral still uses its declared axis for the metre scale. Meridian-only
preparation omits unrelated area tables; full inverse metadata separately admits
its area witness. Sequential children retain newly prepared immutable table heads
only when their allocation lineage extends the parent's current shared tail.
Those bytes are promoted from the child's already observed peak, preserving live
source reservations and counting inherited tables once.

Arithmetic destinations return to the arena when temporary values die. A held
destination produces a typed scratch-capacity refusal rather than an allocation
fallback. Replacing an arena with retained destinations refuses explicitly.
Public immutable proofs and selected-arc preparations detach their retained
endpoints once under output-preparation admission. They remain valid after the
worker context dies and do not pin its reusable arithmetic destinations.

Taylor coefficient storage also belongs to the explicit worker context. A
branded workspace checks out reusable buffers and returns cleared storage on
success, refusal or unwinding. Handles cannot escape their workspace. A
simultaneous checkout refuses with a typed scratch error, and a larger shape
prepares a separate admitted pool before replacing its owner. Sequential
children borrow the parent's pool without counting its heap twice. Observed
setup exposes the complete work and allocation admission before allocation;
no pool lock survives a numerical chunk or external callback. Mathematical
coefficients and their derivative remainders use the same equations on warmed
and newly prepared storage.

Caller-buffer batches share the same scalar bodies and validate their complete
entry once. A refusal clears every output slot; no successfully returned batch
contains a partial prefix. Allocation receipts distinguish warmed complete
kernels from the separately admitted immutable-output preparation boundary.

## Continuous matched transverse-Mercator roundtrips

An explicitly matching forward/inverse pair acts on the original continuous
source. No projected carrier is rounded between stages. Equality requires every
declared ellipsoid, family, zone, scale, false origin, hemisphere, prefix and
source/target binding to match, including reversed axes and realizations. The
complete source rectangle must lie in the declared zone and hemisphere and be
strictly away from both poles. Exported projected coordinates use the ordinary
inverse solver and cannot acquire this identity proof.

Write the original holomorphic map as `F(q+iλ)=m(φ(q+iλ))`. Differentiating its
defining equations gives `D=F′=a cosφ/sqrt(1−e²sin²φ)` and the exact cancellation
`(log D)′=−sinφ`. For the admitted `e²≤0.01`, the complex latitude ODE
`φ′=cosφ(1−e²sin²φ)/(1−e²)` obeys
`|φ′|≤cosh(3/4)(1+0.01cosh²(3/4))/0.99<3/2` while
`|φ−φ₀|<3/4`. Continuation on a radius-1/2 disc about any finite real `q`
therefore remains strictly inside that latitude ball. In particular
`|Imφ|<3/4` throughout the native longitude strip. Consequently
`|arg D(q+iλ)|≤cosh(3/4)|λ|<(4/3)(π/60)=π/45`, so `Re D>0`. On the convex
nonpolar strip, the integral of `D` along any segment has positive real part;
`F(z₂)−F(z₁)=(z₂−z₁)∫D` cannot vanish for distinct endpoints. Real isometric
latitude is strictly monotone, and positive scale and declared false origins
preserve injectivity. The matching all-root inverse therefore returns the
original source everywhere on the admitted panel.

This proof returns original coordinate and differential enclosures directly.
Ground length reuses the same source-linear Taylor integral, including original
parameter fragments. Operation-chain identity remains in provenance. Changed
materialization partitions or completed metric approximations bind their own
output laws; admission limits and a tighter injectivity proof do not choose a
different mathematical result.

## Prepared continuous images

Exact areal images retain the complete original source boundary and a union of
certified source-cell images. A whole-cell Jacobian proof establishes
injectivity and orientation before publishing an oriented ring. A proved
annihilated source direction, or an affine rank loss, instead retains the
complete closed curve or point support without inventing an areal interior.
Unresolved critical cells are subdivided under the original admission policy;
an approximate materialization box cannot satisfy exact preparation.

Whole-cell angular enclosures can exclude a query only when their complete
image lies strictly inside one canonical nonpolar chart. Tighter enclosures do
not change geometry equality, source identities, output laws or vertex counts.
Their immutable owners remain admitted through publication and subsequent
prepared use. Seam-crossing and pole-reaching cells use the original atlas
predicates.

An original axis-aligned rectangle has a stronger proof when the whole-cell
Jacobian has exact zero cross derivatives and each coordinate is strictly
monotone. Its complete image is a rectangle in the physical angular chart.
Unrounded endpoint enclosures give outward walls and a contained inner box:
the upper bound of each lower endpoint and the lower bound of each upper
endpoint delimit that inner box. Strict inner inclusion proves Interior;
strict outer exclusion proves Exterior. Every wall equality or unresolved
candidate continues through the original curve predicate. The shared exact
rectangle proof requires four alternating nonzero coordinate-aligned edges;
it does not infer a rectangle from an endpoint bounding box alone.

Several contacts in a union proved to contain only closed curve and point
support remain ambient Boundary, since there is no open areal face that could
remove an internal wall. Relative-dimensional relations and metrics still use
the complete selected fragment and node graph. Closed complements remove only
the selected open areal side, including complete sector proofs at internal
walls and vertices.

## Pole charts and global physical sublevels

The ordinary point-circle output retains its original secant law. Write
`rho=r+0.05 m`, `m=b²/a` and `R=a²/b`. Exact original rational comparisons
select `rho<3m`, `rho>=3m` and the stronger `rho<m` domain. Equalities at
these boundaries therefore do not depend on an interval's current width.
All remaining numerical inequalities refine until decisive or refuse; neither
resource limits nor an unresolved proof selects a coarser successful output.

Positive curvature gives `K<=1/m²` and `|grad K|<=4/m³`. Along a radial
geodesic, the transverse Jacobi field satisfies `J<=rho` and
`|J′|<=1+rho²/(2m²)`. Differentiating its equation in the initial azimuth,
using the Green-function bound `G(rho,s)<=rho-s`, gives
`|J_theta|<=rho⁴/(3m³)`. The covariant circle acceleration is consequently
bounded by `rho(1+rho²/(2m²))+rho⁴/(3m³)`. The original Christoffel equations
turn this into bounds on both coordinate second derivatives. A coordinate
secant's physical error is at most its second-derivative bound times the square
of its azimuth interval divided by eight.

The same lower principal-radius bound gives injectivity radius at least
`pi*m`. Together with the curvature bound, the global convexity-radius
estimate gives strong convexity for `rho<m`. These are geometric estimates,
not spherical replacements for the ellipsoidal metric. Original meridian
integrals compute both center-to-pole distances. The reverse triangle inequality
then gives boundary clearance `delta=min(|D_north-rho|,|D_south-rho|)`. When
`delta>=2 micrometres`, the chart uses the proved cosine lower bound
`2*delta/(pi*R)` minus the angular output-grid guard. Its cosine upper bound
is the lesser of one and `(D_nearest_pole+rho)/m` plus that guard. Retaining
this upper bound in both the Christoffel longitude term and the physical
longitude displacement avoids treating a short polar circle as a large
Cartesian longitude excursion.

A pole inside the circle contributes one full turn to the continuous longitude
lift: increasing azimuth winds westward around North and eastward around South.
Closing the chart through the included pole adds collapsed pole edges and
oppositely identified meridian walls. Strong convexity proves the meridian
intersection count used by the exact cuts. The same carrier-secant homotopy
proves `B_r subset output subset B_(r+0.1 m)` after direct-output quantization.
A boundary without the required pole clearance uses the global sublevel law.

The global law reuses the six-face cube hierarchy as an angular partition under
the actual declared ellipsoid. For each cell, the shared chart proof returns
closed coordinate boxes on the outward fifteen-place grid, including the complete
assigned-footprint guard. It bounds **every point of those actual boxes** within
`H` of their axis `q`, using normal-metric meridian/parallel routes and both
pole routes. `H` is rounded upward to a micrometre; corner samples do not supply
this proof. This partition does not perform a datum transformation or substitute
a latitude/longitude grid.

The completed global law freezes the actual bound, rather than accepting any
sufficiently tight interchangeable proof. In degrees, let `dphi` be the maximum
latitude-wall separation from the axis. For each longitude wedge, take the
minimum of 180 and its maximum wall separation over each of the three axis
aliases; `dlambda` is the greatest resulting wedge bound. With the frozen
upward rational `p=3.14159265358979323846264338327950288419716939937511`, set
`c=min(1,p*(90-|axis_phi|)/180)` and
`U=min(dphi+c*dlambda,180-axis_phi-south,180+axis_phi+north)`.
The emitted-cell classifier uses exactly `H=ceil_micrometre(R*p*U/180)`.
Changing this bound can change a completed union and therefore changes its
global output-law identity; unchanged point keys and existing cover laws keep
their own identities.

Distance to a closed source is one-Lipschitz. The original physical predicate
therefore discards a cell only when `D(q,S)>r+H`, and emits a proved inside box
only when `D(q,S)<=r-H`. Every other cell is subdivided until `2H` fits the
frozen outward band. If the outside predicate is false, its emitted box satisfies
`D(x,S)<=D(q,S)+H<=r+2H` at every point. The six roots cover the surface, so
these decisions also retain every point in `B_r`. Holes, complements and complete
continuous source images use the same original source-membership and distance
law in these comparisons.

Exact labelled atlas unions cancel the internal rectangle walls. All walls and
their intersections already lie on the output grid, so union serialization
introduces no additional movement of an external boundary. A materialized
symbolic center receives its separately proved one-micrometre inner padding and
outer allowance. Boundary-cover disks retain their separate source-inflation
bound. These allowances are included in their respective frozen output laws;
they are not inferred from a tighter invocation proof.

For materialization, each exact event slab retains only maximal consecutive
ordinate pairs proved Interior. Coalescing removes their shared interior walls;
an Exterior or Boundary pair always ends a run. Metrics keep their original
individual strips and quadrature subdivisions. Removing these artificial walls
can remove collinear carrier vertices, so the materialization law binds this
algorithm separately from the unchanged metric, circle and band equations.

Traversal, generated chart proofs, nested metric work, retained boxes, carrier
vertices, union preparation and final immutable carrier storage all use the
same cumulative context and external governor. Counts and memory are admitted
before allocation. A refusal releases producer storage and publishes no partial
polygon. A successful completed union has its own law identity; stronger proofs
and adequately raised execution limits preserve that union's completed bytes.

Sequential numerical children settle their original work, transient peak and
new immutable coefficient ownership in that same accounting home. An external
observer's first refusal stays latched through settlement and cleanup, and no
later callback runs. A row-scoped argument error cannot hide a fatal work or
storage refusal discovered while settling the child. The shared geometry error
classification supplies this distinction at the evaluator boundary.

The convexity estimate is [Xu, *Local convexity and focal radius*, equation
(1.6)](https://arxiv.org/pdf/1704.03269). The injectivity estimate uses the
positive-curvature closed-geodesic alternative in [Ehrlich, *Continuity
properties of the injectivity radius function*](https://www.numdam.org/item/CM_1974__29_2_151_0.pdf).
All coefficients and chart bounds in this engine follow the equations above.

### Complete native area at poles and equatorial tangencies

For geodetic normal height `z = sin(latitude)`, the exact physical density is
`J(z) = a²(1 − e²)/(1 − e²z²)²`. Let `Q(z) = integral(0,z,J)` and
`Q1 = Q(1)`. The north regular potential is `(Q1 − Q(z)) dλ`; its south
regular partner is `−(Q1 + Q(z)) dλ`. Each has the same exterior derivative,
and their difference is exactly `2 Q1 dλ`. Splitting the selected surface at
the equator therefore gives the sum of the original northern and southern
boundary integrals plus `2 Q1` times the eastward equatorial Interior measure.
Whole surface has equatorial measure `2π`, so this gives `4π Q1`. A directed
equatorial Boundary run uses the north potential when its selected left side
is north, the south potential when its selected left side is south, and is
excluded from the additional Interior measure.

The implementation partitions complete original parameter enclosures. A
strictly positive or negative height enclosure selects its regular gauge.
Every unresolved strip near zero retains both possible gauge fluxes: their
difference is bounded by `2 Q1 integral(abs(dλ/dt))`. Its entire continuous
longitude image enters the cut inventory, with periodic aliases split before
union. This retains folds and tangencies without using a transverse-root test
as an absence proof. Actual equatorial runs require an exact zero-height and
zero-height-derivative proof; their directed speed must be proved or subdivided.
A derivative sign proved over the whole panel allows its continuous longitude
lift to be bounded by enclosed endpoints; unresolved or folded lifts retain the
whole image. At `bits` arithmetic precision, cut walls convert to directed
degrees with `max(24, floor(3*bits/10))` decimal places: outward for possible
contacts, inward for actual Boundary runs. Certified multiplication by `180/pi`
includes the conversion error; directed rounding adds less than one decimal
unit per wall. Thus each outer wall contains the whole angular image, and each
inner wall stays inside the proved Boundary run. Their uncertain strips remain
inside the area enclosure and refine with precision. This internal proof grid
never selects an area result quantum and does not change the separately frozen
RN24 original contact-parameter law. Only an inward enclosure
of a guaranteed selected parameter interval can be labelled Boundary. Outward endpoint and contact strips remain uncertain.

Every cut component outside that complete inventory is boundary free. Its
membership is obtained from the original region. Unknown components contribute
an interval from zero to their complete angular measure, rather than a guessed
point classification. Sufficiently narrow components can use the same complete
measure enclosure, avoiding an unnecessary membership query arbitrarily close
to a boundary. Overlapping longitude bands are partitioned before summation, so
no unknown portion is lost or counted negatively.

The native general area law completes only when the full true-area enclosure
rounds half even to one hundredth of a square metre. Generated density tails,
Taylor remainders, endpoint tails, both gauge fluxes, uncertain cut measure and
arithmetic are all inside that enclosure and refine together. Left and its
complement must both be decisive. Transferring a tighter proof to the common
96-bit result enclosure rechecks both rounding decisions. Unresolved ties or
incomplete admission produce typed refusal. The written source-linear area
branch retains its separately declared polynomial approximation and truncation
certificate; it does not inherit the stronger native completion contract.
