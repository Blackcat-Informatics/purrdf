# Issue #408: geo: deterministic geodesic distance in metres for CRS84 (geof:metric*)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement, complexity-hard

## Body

`purrdf-geo` registers the ellipsoidal geodesic needed by the `geof:metric*` family
but hard-errors on it by name, and `measure::distance` is an exact planar distance
in the coordinate reference system's own units. A consumer that ranks or filters
points on WGS84 by distance in metres therefore has to carry its own great-circle
computation beside purrdf.

## Ask

A deterministic geodesic distance in metres between two points in OGC CRS84
(longitude, latitude), exposed both as a Rust function and through the
`geof:metric*` functions that currently hard-error:

- the WGS84 ellipsoid geodesic (Karney's algorithm), with a declared tolerance; and/or
- a spherical great-circle (haversine) variant on a declared mean radius, under its
  own name so the two are never confused;
- out-of-range latitude or longitude refused with a typed error rather than
  wrapped or clamped;
- a stated rounding contract, so two hosts given the same coordinates return the
  same value.

## Acceptance

- Conformance vectors against published geodesic test data (for example Karney's
  GeodTest set) within the declared tolerance.
- Antipodal and near-antipodal pairs, the poles, and the antimeridian are covered.
- The functions that hard-error today return metres instead.


## Comments (2)

### paudley — 2026-10-04T15:55:14Z

# Deterministic geodesy, Chinese coordinate operations, and spatial cells — issues #408 and #409

## 1. Summary and review gate

Deliver an original first-party Rust geographic engine: complete ellipsoidal metrics and buffers, WGS84 and CGCS2000, standard coordinate-linear geometry plus explicit geodesic arcs, offline Chinese coordinate operations, and a deterministic native cell hierarchy with conservative covers and point indexing. Expose the same engine through Rust, SPARQL, Python, C, wasm, and the CLI.

The current code already supports metrics for explicitly declared projected metre systems. Geographic degree inputs refuse, metricBuffer is unsupported, and length omits polygon boundaries. Preserve the projected planar law and fix the length defect.

**Publication and execution order:** this comment publishes the full plan for the requested external review. Acceptance of that review precedes Task 0 and implementation. Independent plan, compliance, and enhancement reviews have passed for this design; implementation qualification is still required.

## 2. Baseline defaults

Apply .baseline: adversarial gap analysis; complete every accepted requirement; remedy discovered defects; keep issue numbers and development-process text in GitHub rather than repository documentation. Preserve unrelated work and protected main. The emergency ledger was checked and is clean.

## 3. Standing constraints

Apply .goals: greenfield SOTA design, Rust-first implementation with a Python surface, maximal utility/performance/portability, hard failures, and RDF 1.2 throughout. CONSTITUTION.md: None.

- Add zero third-party runtime dependencies and no semantic Cargo features. Independently implement published mathematics; exclude imported codebases, implementation source, and upstream/provider coefficient tables.
- Preserve geo's unsafe-code prohibition and float-arithmetic lint. Shared numerical work has one home in purrdf_xsd::math/ieee/wide; geo calls its safe primitives. Reuse existing hash, framing, dispatch, JSON, allocation, test, and benchmark homes.
- Keep ingest and carrier coordinates exact. Floating and fixed-point scratch values have declared error; they do not replace source coordinates or decide topology by an epsilon.
- First-party source retains MIT OR Apache-2.0 OR MulanPSL-2.0 headers. Imported test data retains its own grant and notices. Mathematical implementation and test-data provenance are recorded separately.
- Private licensed standards/data are read in place only. No copied contents, identifying excerpts, private document names, or private paths enter the repository, public logs, issues, or PRs. Public evidence contains aggregate results.
- All release crates remain wasm-compatible. No ambient network, filesystem, clock, random source, or C toolchain enters the numerical engine.

## 4. Completeness contract

| Requirement or acceptance criterion | Tasks |
|---|---|
| WGS84 Karney inverse/direct; declared error and host-identical rounding | 1–2, 7–8 |
| Full published geodesic corpus; antipodes, near-antipodes, poles, antimeridian | 2, 8 |
| Original rational range checks; typed refusal without wrapping/clamping | 1–2, 5–6, 8 |
| Geographic metricDistance, metricLength, metricPerimeter, metricArea, metricBuffer; corresponding unit functions | 3, 6, 8 |
| All geometry classes, holes/collections, coordinate-linear and geodesic-arc models, complements/full surface | 3, 8 |
| Native CGCS2000 measurements, grid, and explicit datum/axis/epoch identity | 1–5, 8 |
| Gauss–Krüger 3°/6°, UTM, Web Mercator; explicit zones/units and bounded inverses | 4, 6, 8 |
| WGS84 ↔ GCJ02 ↔ BD09LL ↔ BD09MC, original named laws and qualified provider agreement | 4, 6, 8 |
| Caller-parameter Helmert/similarity/polynomial/grid operations; height, epochs, inverse ambiguity | 4, 8 |
| Pure deterministic assignment, parents/children, levels 0–30, stable validated IDs | 1, 5, 7–8 |
| Fixed-resolution disk/box covers, finite limits, typed refusal, no false negatives | 5, 8 |
| Versioned grid parameters remint derived index identities; candidate lookup plus refinement | 1, 5, 8 |
| Frozen hierarchy/cover vectors, seam/pole/extreme levels, exhaustive small-radius checks | 5, 8 |
| Real Rust/SPARQL/Python/C/wasm/CLI exposure and error/rounding parity | 6, 8 |
| Measured portable/batch/SIMD paths, actual vector instructions, zero inner-kernel allocations | 1, 7–8 |
| Source licensing, mathematical derivations, public corpus receipts, private aggregate validation | 1–4, 8 |
| User-directed H3 v4.0 compatibility architecture, without current H3 execution claims | 5, 8 |
| External-review hold, isolated worktree, verified commits/pushes/updates, qualified non-draft PR closing both issues | 0–9 |

## 5. Enhancement audit and closed design choices

Adopt the transformational structure: **certified prepared curves, regions, references, and operation chains**, shared by measurement, topology, conversion, buffers, and indexing. This is an L-cost change with decisive payoff: individual point formulas cannot establish correct geometry or cover behavior.

Also adopt the native cubed-sphere hierarchy and enclosure proof (L); prepared operation chains, SoA batches, bounded cell ranges, and reusable point indexes (M); typed dimensions, error evidence, and complete provenance receipts (S). Reuse shared arithmetic/qualification homes and add singular/ambiguous inverse and global materialization checks.

Decline spherical approximation as the ellipsoidal answer, a latitude/longitude grid with poor polar cell quality, runtime CRS databases, imported geospatial implementations, copied provider tables, and automatic datum/zone selection. Decline the independently fitted GCJ model because improved fixture agreement does not establish its behavior outside that dataset. A separate public mesh engine adds a different abstraction without strengthening these requirements.

The explicit **v4.0 H3 compatibility design** is a design deliverable here. It defines a family boundary for qualified IDs, variable-count hierarchy operations, logical versus geometric containment, icosahedral Class II/III rotations and pentagons, independently derived ID encoding, and compatibility vectors. Current execution implements the native family; it exposes no H3 compatibility claim or unsupported H3 API variant.

## 6. Task 0 — isolated execution baseline

After external-review acceptance, refresh issues/comments, origin/main, working-tree state, sibling worktrees, instructions, and the emergency ledger. Create branch issue-408-geodesy-spatial-index in .worktrees/408-geodesy-spatial-index from fresh origin/main. Preserve all unrelated worktrees and main.

Establish focused and full baseline validation in that worktree, ensure ordinary hooks are installed, and capture the source/toolchain identity. A failing baseline must be diagnosed and remedied; it is not a passed check.

## 7. Implementation tasks

### Task 1 — numerical foundation, profiles, and errors

Build the original deterministic binary64/double-double math pipeline with interval enclosures and bounded higher-precision fallback using existing first-party BigInt/wide arithmetic. Freeze operation sequences, range reduction, coefficients, branches, and rounding; use no platform libm, FMA contraction, approximate reciprocal, or reassociated reduction.

Move the existing float-environment checker into the shared IEEE home and preserve its core public reexports. A thread-bound MetricContext checks rounding, gradual underflow, and excess precision once per call/batch. Prepared data is shareable; the live numerical context is not transferable between threads.

Validate original Rat longitude/latitude before conversion. Add an exact finite-binary64/dyadic bridge and propagate conversion error. Exact pole detection uses the original coordinate: a valid value just inside a pole must refine rather than become a false projection-domain rejection.

Expose validated LonLat/GeographicReference, PreparedEllipsoid/PreparedGeometry/CoordinateOperation, dimensioned metre and square-metre quantities, and MetricEstimate carrying value, certified error, and profile identity. Scalar and batch APIs accept caller-owned outputs.

Use canonical framed descriptors and existing BLAKE3/Domain/Digest32 for profile, operation, cache, and new index identities. Bind datum/realization/epoch, axes/units, mathematical and edge laws, parameters, coefficient identity, rounding, and applicability. GeoVocab explicitly registers these against caller-supplied IRIs; names or angular units alone never imply WGS84.

Add structured range/domain/ambiguity errors and operational precision, work, allocation, cancellation, convergence, and environment errors. Operational failures remain fatal through SPARQL FILTER/BIND; genuine argument errors retain established expression behavior. Outer query-governor trips retain the existing incomplete-outcome/evidence contract.

Register shared jobs/layer edges and capture portable benchmark baselines before optimization.

### Task 2 — original ellipsoidal point and line kernels

Independently derive Karney's auxiliary-sphere inverse/direct, reduced-length/Jacobi, and area kernels: safeguarded Newton/bracketing, antipodal astroid initialization, and order-eight fast-path coefficients generated from integral equations by our exact generator. Add a separately certified integral evaluator and precision fallback; published implementation performance is not our proof. [Karney's mathematical derivation](https://arxiv.org/pdf/1109.4448)

Provide native WGS84 (a=6378137 m, inverse flattening=298.257223563) and CGCS2000 (a=6378137 m, inverse flattening=298.257222101), without equating datum realizations. [NGA WGS84 definition](https://earth-info.nga.mil/index.php?action=wgs84&dir=wgs84), [BeiDou coordinate-system definition](https://www.cnsa.gov.cn/n6758823/n6758839/c6796160/part/6776242.pdf)

| Geographic result | Certified acceptance target | Completed-result rounding |
|---|---|---|
| Point inverse/direct | ≤1 µm surface error, including coordinate/distance rounding; internal target ≤0.25 µm | Distance: half-even, 6 decimal metre places; direct coordinates adapt to the certificate |
| General geometry distance | ≤0.25 mm absolute | Half-even, 6 decimal metre places |
| Length/perimeter | max(1 mm, 1e-14 × value) | Half-even, 6 decimal metre places |
| Area | max(0.1 m², 1e-14 × value) | Half-even, 2 decimal square-metre places |

Round once after completion. The existing correctly rounded xsd:double conversion adds at most half an ULP, which is included in host-facing evidence and search padding. Preserve existing projected planar numerical contracts except documented conformance fixes.

Expose geodesic::inverse/direct and scalar/batch line operations through the prepared context. Antipodal point distance works. Direct results use canonical output longitude; input coordinates are never normalized into validity.

### Task 3 — complete geometry, topology, and buffers

Standard WKT/GeoJSON edges retain source-coordinate interpolation, including the written long route from 170° to −170°. Add explicit shortest-geodesic and azimuth/length arc constructors with a retained branch; ambiguous endpoint-only construction refuses. The typed region model represents empty, whole surface, oriented interiors, holes, and complements without choosing the smaller area. [RFC 7946 edge definition](https://www.rfc-editor.org/rfc/rfc7946.html#section-3.1.1)

Use one exact longitude-cut atlas with seam/pole identification. Reuse exact planar predicates within charts; independently derive certified geodesic intersection/overlap classification. Wire this same geographic law into scalar spatial relations, relation/property-function indexes, broad-phase bounds, membership, and metrics. Remint identities/caches for changed geographic relation laws; preserve explicitly planar laws.

For coordinate-linear curves, integrate the ellipsoid surface metric and area potential using deterministic Taylor-model interval quadrature with certified remainders. Explicit arcs use geodesic integrals. Ground measurement of projected/encoded inputs evaluates continuously mapped source edges; it does not measure a coarsely serialized approximation.

Find general minimum distance through topology followed by globally certified point/edge and edge/edge minimization: prepared bounds, deterministic branch-and-bound, stationary-root isolation, and endpoints. Local optimization seeds the search but cannot exclude candidates.

Implement all five metric functions and their unit counterparts. Include polygon boundaries in length; sum within each topological dimension and return the greatest dimension-total for mixed length. Perimeter measures areal boundaries and falls back to length for non-areal inputs. Area subtracts holes; area conversion squares unit factors. XY measurement retains Z/M carrier data without inventing height-dependent distance. [GeoSPARQL metric definitions](https://docs.ogc.org/is/22-047r1/22-047r1.html)

Provide a typed OffsetRegion predicate and certified polygon materialization. Negative radius gives empty; zero gives source closure. Positive buffer satisfies B_r ⊆ output ⊆ B_(r+ε), with ε=0.1 m including coordinate quantization. Use offset normals/end circles, certified arrangement/union and face labeling, plus atlas fallback for poles/cut loci/global regions. Union overlapping collection buffers; encode whole-surface carrier results through verified coordinate-linear atlas polygons.

Normalize output deterministically by orientation, cyclic start, hole/component ordering, and antimeridian cuts. Keep exact offset membership distinct from its finite polygon representation.

Expose metric::distance/length/perimeter/area/buffer over prepared geometry and its registered profile. Default geometry/solver budgets: 131072 output vertices, 262144 admitted work items, 64 MiB retained workspace, 128 iterations, 64 subdivision levels, and 512 precision bits. Explicit overrides use checked counts and memory accounting. Exhaustion returns structured evidence, never weakened accuracy or successful partial output.

### Task 4 — offline Chinese coordinate operations

Expose CoordinateOperation::compile/apply/apply_batch and implement these operations once per profile:

- CGCS2000 geographic/geocentric conversion; native transverse Mercator with 3°/6° Gauss–Krüger and UTM. Declare zones, meridians, hemisphere, scale, false origins, units, axes, and zone-prefix conventions explicitly.
- Web Mercator's standard square-domain profile: |latitude|≤atan(sinh π), approximately 85.0511287798066°. Refuse outside it without clipping.
- WGS84 ↔ GCJ02 under GcjRationalHarmonicV1, derived from published polynomial/harmonic equations; GCJ02 ↔ BD09LL under the original Cartesian radial/angular perturbation implementation.
- BD09LL ↔ BD09MC under BaiduMercatorAnalyticV1: independently implemented ellipsoidal Mercator with Clarke 1866 axes a=6378206.4 m and b=6356583.8 m. Its correspondence to provider outputs is an empirical inference. Mathematical domain: longitude [-180°,180°], latitude strictly between poles, finite representable output.
- Caller-parameter 7-parameter geocentric Helmert and 4-parameter planar similarity; polynomial and in-memory bilinear-grid transformations. Polynomial basis/degree/order, grids/extents/holes, units, domain, pivot, rotation convention, epochs/rates, and parameter/content digests are explicit. Require actual height for 3D Helmert. Bundle no regional datum-shift parameters and assume no WGS84↔CGCS2000 identity.

Use published mathematical definitions only. [Transverse Mercator research](https://geographiclib.sourceforge.io/tm.html), [Mercator mathematics](https://proj.org/en/stable/operations/projections/merc.html), [Wu's original GCJ research](https://wuyongzheng.github.io/china-map-deviation/paper.html), [tool-publisher conversion equations](https://go-tools.org/tools/coordinate-converter)

GCJ applicability requires a caller-declared exact source domain. A commonly used interoperability rectangle is not a national boundary; there is no automatic identity-outside-domain branch. BD09 origin and all projection singularities have explicit mathematical treatment.

Certify inverses with forward residual ≤1e-12 degree for geographic offsets and ≤1e-6 m for projections, including rounding. Use conditioning bounds for returned coordinate error. Isolate GCJ roots on both smooth cusp branches longitude=105±u²; isolate/exclude every admissible branch. Multiple roots give AmbiguousTransform unless caller restrictions certify uniqueness. Polynomial/grid inverses use bounded root solving, not sign reversal.

Whole-geometry transform means the image of the complete source point set. Materialization has a certified ≤0.1 m Hausdorff bound including quantization. Preserve topology for certified injective maps; reconstruct image sets at folds using critical-curve isolation, subdivision, arrangements, and preimage classification. Unresolved reconstruction exhausts/refuses explicitly. Never silently join across an applicability discontinuity.

Standard geoJSONLiteral always remains WGS84 longitude/latitude. Other targets return WKT; asGeoJSON requires an actual registered conversion to CRS84 plus certified materialization/cutting. No CGCS/projected/provider coordinates are relabelled GeoJSON.

Qualify provider agreement separately from numerical error:

| Model | Fixture acceptance only |
|---|---|
| GcjRationalHarmonicV1 | ≤1.0 m |
| BD09LL perturbation | ≤0.02 m |
| BaiduMercatorAnalyticV1 | ≤0.20 m |

These bounds concern all 664 pinned rows, representing 332 unique sites. BD09MC evidence spans latitude 16.8380363°–52.3412419°; it establishes no global provider or surveying accuracy. [Pinned fixture](https://github.com/hujiulong/gcoord/blob/fa43556eaa0a55a25895d4e7b0aca4d2c2f4320e/test/fixtures/china-cities.json)

### Task 5 — native cells, conservative covers, and point indexes

Ship CubeHilbertQ62V1: six cubed-sphere faces, quadratic warp, Hilbert hierarchy, levels 0–30. Use geodetic unit normals and distinct WGS84/CGCS2000 profile identities.

Freeze face order +X,+Y,+Z,−X,−Y,−Z, dominant-component ties in that order, and right-handed charts:
+X=(1,u,v); +Y=(-u,1,v); +Z=(-u,-v,1); −X=(-1,-v,-u); −Y=(v,-1,-u); −Z=(v,u,-1).
Warp w=2s−1 to sign(w)(2|w|+|w|²)/3. Dyadic ownership is half-open except the final outer endpoint. Hilbert recursion uses lower-left/upper-left/upper-right/lower-right, swap/up/diagonal/reflected-swap child transforms, and odd-face root transposition. Verify face continuity independently.

The fixed transform uses exact quarter-turn reduction in degrees, Rat→Q96 half-even, an independently generated 192-bit π, degree-19 sine/degree-18 cosine Taylor Horner on [0,π/4] with Q96 coefficients/products, then Q62 half-even components/products. Prove total component error ≤2^-60, including source quantization; freeze the actual law. Exact poles and ±180° have specified overrides; original coordinates remain untouched.

Expose Grid::assign/assign_batch/parent/ancestor/children/cover_disk/cover_box. Cell keys are validated u64: three face bits, two Hilbert-child bits per level, a sentinel, and 60−2×level trailing zeros. Compute deepest ownership once and derive ancestors by prefix. Public CellId pairs GridProfileId with the local key. Use opaque IDs and bounded child iterators; parent/root and children/leaf behavior is explicit.

Disk cover proof: ellipsoidal distance ≥(b²/a)×geodetic-normal spherical separation. Traverse from six roots using certified corner-cap enclosures, Q62 error inflation, and outward dot/chord bounds; prune only proved separation. Include the 1 µm point-result allowance and host double-rounding allowance. Uncertain cells survive.

Box covers use closed latitude and longitude wedges: west>east crosses the antimeridian; west==east is a meridian; [-180°,180°] is full longitude. Closed poles/boundaries cannot disappear. Return sorted unique completed cells and checked ranges at the requested resolution.

Defaults: 65536 logical output cells, 262144 work items, 64 MiB workspace. Compressed ranges count every represented cell. Check u64 counts/bytes before allocation; no truncation, resolution reduction, or successful partial cover.

Build compact PointCellIndex sorted by cell and stable caller point key. Identity binds grid law/version, resolution, source content, and conversion profile. Search cover→key ranges→refinement. Offer distinct certified physical-radius and reported-metric-threshold predicates; the latter matches rounded Rust/SPARQL comparison. Physical-boundary uncertainty refines or raises typed precision exhaustion. Native indexing of provider coordinates requires an explicit conversion first.

Complete the H3 compatibility architecture described in section 5 as a GitHub design deliverable.

### Task 6 — real host and governed-query integration

Put one strict version-1 profile/request/response codec and pure string adapter in purrdf_validate::geo, using existing JSON record machinery. Mirror the typed Rust operations; use decimal strings for exact quantities and fixed-width hexadecimal strings for IDs/digests. Reject unknown/duplicate fields. Success and error evidence retain profile identities and numerical certificates.

Provide prepared GeoProfile/GeoSession access to every kernel operation, including cell/index construction and queries. Python gets native wrappers and geo= query configuration decoded before releasing the GIL; wasm gets the same sessions and QueryEngine profile configuration; C gets an opaque create/call/free context and ABI-compatible context-taking query variants; CLI gets --geo-profile and purrdf geo request/response access.

Bind one prepared function registry through QueryOptions into synchronous, asynchronous, governed, and entailment query lanes. Add a reusable context-aware metered native-function seam while preserving existing callback APIs, so inner work/stop checks participate in the existing governor. A stopped operation publishes no partial cover/geometry, and a stopped query keeps its established incomplete-outcome evidence.

Add only declared first-party layer edges. No binding reimplements mathematics, vocabulary selection, profile parsing, or rounding.

### Task 7 — measured optimization and backend determinism

Prepare ellipsoid/projection coefficients and compact SoA data once. Use caller buffers, masked independent solver lanes, scalar tails, bounded traversal, exact/dyadic completed sums, and zero allocations inside numerical kernels.

Use existing Backend/vector_backend dispatch for useful shared numeric/fixed-point/dot kernels on portable, SSE2/AVX2/AVX512, NEON, and wasm simd128 paths. Domain-specific Hilbert/traversal remains safe Rust. Add actual hot sites to the shared dispatch registry and assembly manifest.

Compare scalar, prepared, and batch sizes 1/4/16/256/4096; measure complete metrics, transformation chains, buffers, covers, candidate inflation, memory, and allocations with existing statistics/probes. Require identical completed bytes on every available path. Select an optimized path only when measurement supports it; report actual gains without a fabricated target percentage.

### Task 8 — conformance, provenance, documentation, and final qualification

Use the complete CC0 GeodTest corpus: 500000 rows, 86558916 bytes; SHA-256 c1cabdddbcd7d5cfc6e6111db4608fa55be292b15ba2c5bcd6372a178848c692. Acquire digest-checked public data into the test cache; run full native inverse/direct acceptance, plus representative frozen cases on every host/backend. [Official dataset](https://zenodo.org/records/32156)

Pin the public Chinese fixture to commit fa43556eaa0a55a25895d4e7b0aca4d2c2f4320e, blob b824f8bac7126ccfdbe02e70435fb079816fc398, SHA-256 d31a61f8e9185977c6a3f771e146a4040ab099ea153cf63a3ac08ae35cf366ab. Retain Jiulong Hu's MIT copyright/permission notice for any copied data; acquire no implementation-source files. [Pinned licence](https://github.com/hujiulong/gcoord/blob/fa43556eaa0a55a25895d4e7b0aca4d2c2f4320e/LICENSE)

Document independently generated constants/coefficients, mathematical sources, profile encoding, certificates, domains, rounding, and compatible versus mathematical-model claims. Regenerate metadata/assembly documentation through their generators. Repository documentation describes behavior without development-process references.

Run the private aggregate-validation probe against all 31 available CGCS2000 administrative geometry bundles in place: source-CRS validation, prepared geometry, conversion, cover/refinement consistency, and performance. Stream data in memory to the Rust runner; retain no copied private inputs. Treat it as workload evidence, not WGS84 ground truth or official certification.

## 8. Validation gates

- **Numeric:** analytic equator/meridian/pole cases, all geodesic rows, near-antipodes, exact/tiny range neighbors, signed zero, exact conversion/rounding thresholds, bad float environments, and valid near-singular inputs. Verify certificates including final rounding.
- **Geometry:** source-linear versus arc long routes; selected antipodal branches; interior minima; tangencies/overlaps/holes; mixed dimensions/units/Z/M; complements/full surface; seam/pole aliases across metrics, scalar relations, and property-function indexes.
- **Materialization:** negative/zero/positive/global buffers, containment bands, deterministic ring order and round trips; transformed continuous edges, noninjective images, discontinuities, and certified topology rules.
- **Conversions:** projection examples and independent analytic/inverse checks; zones/hemispheres/prefixes/axes/epochs/heights; GCJ cusp/root ambiguity; every fixture row; grid holes/boundaries, non-injective polynomials, singularities, and valid refusal neighbors.
- **Grid/index:** exhaustive small-level codecs/hierarchy, every face boundary and resolution extreme, proved Q62/cap bounds, independent finite-point radius/box oracles, physical and rounded thresholds, zero-radius neighbors, seams/poles, and identity reminting.
- **Limits/errors:** exact-limit and limit-plus-one cases; count/byte overflow, allocation, precision, iterations, depth, workspace, and cancellation. Exercise fatal intrinsic failures in FILTER/BIND and correct outer-governor outcomes. Assert no complete partial operation.
- **Hosts/performance:** real Python/C/wasm/CLI calls and queries, frozen response/error parity, all forced available backends, actual vector assembly, counting allocator, and measured baselines.
- **Full qualification:** focused geo/xsd/shared-adapter tests; make check, make wasm, make geo-determinism, make simd-asm, make conformance, generated-artifact checks, and required dependency/feature/layer/helper/ring-fence/MSRV checks. Native x86-64/Arm64/i686/i586-x87 and wasm portable/SIMD evidence is tied to source/toolchain identities. Skipped or unavailable checks are not passes.

Run independent wiring review on shared changes and final adversarial integration review. Resolve every accepted finding before completion.

## 9. Commit, push, issue updates, and CI

For each logical Task 1–8: validate the concrete change, perform required wiring review, fix failures, make a normal hook-verified signed commit, push the issue branch, and comment on issue #408 with the task result, commit SHA, and precise evidence. Never bypass a hook or substitute an ad-hoc test for its gate.

Run the mechanical deferral scan against the plan, branch diff, and PR text; resolve every finding or document its evidenced false positive. Recheck the emergency ledger and worktree cleanliness.

Dispatch the existing CI workflow on the final pushed branch before PR creation. Verify the run's head SHA equals that commit and every required matrix job completes successfully. If any code changes, repeat the affected qualification and dispatch for the new final SHA. Report signing/authentication/hooks/push/qualification failures precisely and leave the work incomplete.

## 10. Task 9 — qualified pull request

After every mapped task is committed, pushed, and qualified, create a non-draft PR to main with a plain project title, Closes #408 and Closes #409. Describe final behavior, public interfaces, numerical/model bounds, licensing, benchmark evidence, and local/hosted validation separately.

Attach this complete implementation plan to the PR, post the PR/evidence link to issue #408, and report the branch, commit identities, and validation status. This Stage-1 endpoint is PR creation; merging and release publication require their own authorized workflow.

### paudley — 2026-10-04T16:01:07Z

Review of the combined #408/#409 plan: approve with changes

The plan meets or exceeds both original asks on accuracy, range refusal, conformance data, hierarchy, no-false-negative covers, and versioned identity. Declining the spherical variant is within the "and/or" of #408. The points below are what a consumer that ranks points by distance and keeps a cell-keyed index in an external ordered key-value store would need to drop its local workarounds. Items 1-3 should become acceptance criteria; 4-5 are clarifications.

### 1. Covers across several levels, with the budget counting what is emitted

As written, covers are produced only "at the requested resolution" and "compressed ranges count every represented cell". A fixed-resolution disk grows with the square of the radius. At a resolution suited to street-scale search (cells of about 150 m), a 400 km disk is millions of cells, so it refuses under the 65536 default, and raising the limit only moves the quadratic cost onto the caller. The hierarchical prefix-key design already makes a coarse cell a contiguous range of descendant keys, so the fix fits the plan.

Requested:

- A cover operation that takes a minimum level, a maximum level, and a maximum number of emitted cells (or ranges). It returns a canonically sorted, non-overlapping set of cells at mixed levels within those bounds whose union is a proved superset of the disk or box. The budget counts emitted cells or ranges, not the leaf cells they stand for. Exceeding it is a typed refusal, never a coarser or truncated cover chosen silently.
- `CellId::range_min()` / `range_max()` (or an equivalent closed range of keys at a stated level) so a caller can turn any cover cell into one ordered scan over keys stored at a finer level.
- A documented guarantee that numeric u64 order on cell keys equals big-endian byte order equals the hierarchy's order, and that every descendant of a cell falls inside that cell's range.
- Acceptance: for random disks from 1 m to 20000 km and random boxes, including ones crossing the poles and the antimeridian, every leaf cell holding a point within the radius lies inside some returned range (exhaustive on small radii, as already planned), and the emitted count never exceeds the cap.

### 2. State exactly what is deterministic, and when identity changes

"Round once after completion" with frozen operations makes the result the same on every host. Please state which of these is the contract:

- (a) the completed distance is the correct rounding (half-even, 1e-6 m) of the true ellipsoidal distance, refining the enclosure until the rounding is decided; or
- (b) the frozen algorithm defines the value.

Under (a), a later algorithm change that keeps correct rounding need not re-mint anything. Under (b), any change to the operation sequence must change the profile identity. Either is acceptable if it is written down. Please also state the behaviour when the enclosure cannot be resolved within the precision budget: a typed precision error, not a value that is merely within tolerance.

Acceptance: a test showing that a profile identity changes if and only if some completed output can change, and frozen distance vectors pinned to the profile identity.

### 3. Default CRS84 behaviour in `geof:metric*`

The OGC CRS84 IRI, and GeoSPARQL WKT with no prefix (whose default CRS is CRS84), are WGS84 longitude/latitude by definition, not by inference from a name. #408's acceptance is that the functions that hard-error today return metres. That should hold for a default-configured query engine on unprefixed or CRS84 WKT points, without caller-side profile registration. Explicit registration is still right for every other CRS IRI.

Please also state the EPSG:4326 rule: its axis order is latitude, longitude. The literal must either be refused or read through the registered axis order; it must never be silently read as longitude, latitude.

Acceptance: on a default engine, `geof:metricDistance` over two unprefixed POINT literals returns the same metres as the Rust function. An EPSG:4326-prefixed point either refuses with a typed error or measures with the axes swapped correctly, and the test pins which.

### 4. A plain function, and the cost of the context

#408 asked for a Rust function. The plan routes everything through a `MetricContext` that cannot move between threads. That is fine for batches, but please also provide `geodesic::distance(a: LonLat, b: LonLat) -> Result<MetricEstimate, _>` (and the reported-threshold predicate, `within(center, point, radius)`, as a standalone function, not only inside `PointCellIndex`). Please state the per-call cost of building the context, so callers on worker pools know whether to cache one per thread.

Please also expose the physical size of each level (at least min/max edge length in metres or min/max area, per grid profile) so a caller can pick index and cover levels from a target spatial precision.

### 5. Report cost as acceptance evidence

No target percentage is needed, but certified interval arithmetic with a bigint fallback can be orders of magnitude slower than binary64, and distance refinement runs once per candidate in interactive search. Please record as acceptance evidence:

- ns/op for a scalar point inverse distance on the common path, and the fraction of GeodTest rows that take the higher-precision fallback;
- elapsed time and emitted count for disk covers of 1 km, 30 km and 400 km at a street-scale maximum level, using the mixed-level cover from item 1.

### Optional

The prepared-region model already exists, so a conservative cover for a prepared polygon or multipolygon (same superset proof and budget rules) would let a consumer index non-point geometry with the same keys. It wasn't part of either ask, but it is cheap given the existing machinery.

Note on sequencing: Tasks 1, 2 and 5 cover both original asks. The coordinate-operation and buffer tasks are much larger. If they turn out to dominate the schedule, consider landing the distance and cell work first rather than holding it to the end.


