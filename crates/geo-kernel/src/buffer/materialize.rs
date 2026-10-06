// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original certified secant bounds for ellipsoidal metric-circle materialization.
//!
//! With m=b²/a, principal radii >=m, K<=1/m², |grad K|<=4/m³.
//! Jacobi comparison gives J(s)<=s and |J'(s)|<=1+s²/(2m²). Differentiating
//! J''+KJ=0 and bounding its Green function by s-t gives |J_theta|<=s⁴/(3m³).
//! Thus covariant circle acceleration <=s(1+s²/(2m²))+s⁴/(3m³). Explicit
//! Christoffel bounds below enclose complete coordinate second derivatives;
//! a carrier secant deviates by at most R*A*delta_theta²/8.
//!
//! Conjugate radius >=pi*m. Closed geodesics have ambient curvature <=1/m;
//! Fenchel bounds their length below by2*pi*m. Klingenberg's estimate therefore
//! bounds injectivity radius below bypi*m. Before that bound the disk has winding
//! one. A homotopy moving its boundary <=0.02m at radius r+0.05m stays outside
//! B_r and inside B_(r+0.1m), proving both required set inclusions.
//! The pole atlas uses rho<m<pi*m/2, hence strong convexity. Original meridian
//! distances bound its boundary's pole clearance; Christoffel and physical
//! longitude bounds retain their cosine factors in that chart.
//! Mathematical references, not implementation sources:
//! <https://www.numdam.org/item/CM_1974__29_2_151_0.pdf>
//! <https://math.ou.edu/~shankar/papers/skso.pdf>
//! <https://arxiv.org/pdf/1704.03269> (global convexity-radius estimate (1.6))

use super::OffsetRegion;
use crate::carrier::MaterializationStorage;
use crate::context::WorkProgress;
use crate::numerical::frozen_decimal as decimal;
use crate::{
    AxisOrder, Coord, CoordDim, Crs, GeoError, GeoProfile, Geometry, GeometryBody, GeometryLiteral,
    Int, LonLat, Metres, MetricContext, MetricWorkObserver, PreparedGeodesic, PreparedRegion, Rat,
    SemanticLawId,
};
use purrdf_hash::{Domain, hex::Digest32};
use std::borrow::Cow;
use std::sync::Arc;

mod covering;
mod cut;
mod global;

const LAW: Domain = Domain::new(b"purrdf-geo-kernel/offset-materialization-law/v1");
// Materialization coalesces only maximal adjacent ordinate pairs already
// proved Interior in the same exact slab. This preserves the closed union but
// can remove artificial collinear carrier vertices, so the completed algorithm
// is bound separately from the unchanged circle and physical-band equations.
const UNION_DESCRIPTION: &[u8] = b"exact-source-atlas-union;all-original-adjacent-ordinate-pairs-classified;maximal-consecutive-proven-Interior-runs-coalesced-per-event-slab;Exterior-and-Boundary-gaps-retained;metric-subdivision-unchanged;original-exact-wall-noding-and-angular-successor-law;v1";
const DESCRIPTION:&[u8]=b"physical-point-union;ellipsoid-Jacobi-Christoffel-bound;injectivity-pi-m;outward-radius0.05m;complete-secant-error0.019m;degree15-direct-error1um;symbolic-centre-degree15-surface-error1um;all-centres-meridian-diameter-whole-band0.099998m;dyadic-circle-N>=8;latitude-and-lift-degree-grid-guard1e-15;continuous-centred-longitude-lift;exact-two-crossing-affine-meridian-cuts;closed-chart-pieces;exact-polar-meridian-caps-and-whole-surface;uncut-single-certified-circle-preserves-original-direct-ring;cut-single-certified-circle-retains-disjoint-closed-chart-pieces;multiple-components-complete-labelled-source-atlas-union;exact-noding-internal-wall-cancellation-angular-successors;degree15-union-vertex-error1um;CCW-outers-CW-holes-lex-ring-start;sorted-components-and-holes;negative-empty;zero-exact-carrier-closure;outer-band0.1m;v1";
const GLOBAL_DESCRIPTION: &[u8] = b"physical-point-union;original-local-circle-law-unchanged;exact-original-rho=r+0.05m;local-rho<3m;global-rho>=3m;pole-atlas-rho<m;meridian-clearance>=2um;cos-lower2delta-over-piR-minus-degree15-guard;cos-uppermin1-nearest-pole-distance-plus-rho-over-m-plus-gridguard;Jacobi-Christoffel-coordinate-secant-weighted-cos-upper;sequential-continuous-lift;strong-convex-pole-winding-and-two-crossing-atlas-cuts;global-angular-CubeHilbertQ62V1-partition;symbolic-centre-grid-guard1um-inside-and-outside;complete-assigned-footprint-outward15-carrier-box;entire-carrier-axis-radius-H=ceil-micrometre-normal-metric-route;physical-D-axis-closed-source;outside-iff-D>r+H;inside-iff-D<=r-H;otherwise-refine-until2H<=0.099998m;six-roots-Hilbert-depth-first;closed-boxes-labelled-exact-atlas-union;axis-aligned15-grid-intersections-exact;CCW-outers-CW-holes-lex-ring-start;sorted-components-and-holes;negative-empty;zero-exact-carrier-closure;outer-band0.1m;v1";
// H determines emitted cells, so its exact rational formula is part of both
// completed global laws rather than an interchangeable proof receipt.
const GLOBAL_CARRIER_RADIUS_DESCRIPTION: &[u8] = b"p=3.14159265358979323846264338327950288419716939937511;axis=shared-footprint-normal-midpoint-RN15;walls=shared-complete-assigned-footprint-Down15-Up15;dphi=max(abs(south-axis_phi),abs(north-axis_phi));dlambda=max-wedges(min(180,min-j-in{-1,0,1}(max(abs(west-axis_lambda-360j),abs(east-axis_lambda-360j)))));c=min(1,p*(90-abs(axis_phi))/180);U=min(dphi+c*dlambda,180-axis_phi-south,180+axis_phi+north);H=ceil-6(R*p*U/180);R=a*a/b";
const GLOBAL_REGION_DESCRIPTION: &[u8] = b"complete-original-physical-source-union;complete-selected-areal-curve-and-isolated-point-strata;original-exact-cut-domains;covered-disks-share-pole-clearance-atlas-law;native-geodesic-and-operation-image-interiors;holes-and-complements;global-angular-CubeHilbertQ62V1-partition;complete-assigned-footprint-outward15-carrier-box;entire-carrier-axis-radius-H=ceil-micrometre-normal-metric-route;physical-D-axis-closed-source;outside-iff-D>r+H;inside-iff-D<=r-H;otherwise-refine-until2H<=0.099998m;covered-boundary-disk-global-band0.080m-plus-source-inflation0.014002m;six-roots-Hilbert-depth-first;closed-boxes-labelled-exact-atlas-union;axis-aligned15-grid-intersections-exact;CCW-outers-CW-holes-lex-ring-start;sorted-components-and-holes;negative-empty;zero-exact-carrier-closure;outer-band0.1m;v1";
const REGION_DESCRIPTION: &[u8] = b"complete-physical-source-union;whole-and-negative-empty;all-original-images-validated-before-meridian-diameter-whole-band0.099998m;zero-exact-written-closure-includes-selected-curve-and-isolated-point-strata-unrepresentable-native-closure-refuses;original-closed-areal-atlas;regular-written-boundary-rings-preserved;native-selected-areal-and-curve-fragments-cover-only-original-exact-cut-domains;selected-isolated-points-retained;implicit-native-support-uses-complete-global-sublevel-law;fixed80-complete-normal-panels;dyadic-L1-chord-diameter-times-R-pi-over2<=0.014m;source-linear-and-image=actual-original-midpoint-degree15-grid<=1um;geodesic=dyadic-endpoints-unrounded-arclength-upper;nonnegative-curvature-distance-square-comparison;h-square<=8r-times0.014+4times0.014-square;actual-original-endpoint-degree15-grid<=1um;radius-inflation0.014002m;continuous-longitude-lift-exact-meridian-cuts;shared-point-circle-law-actual-outer-band<=0.069002m;closed-component-union;total-outward-band0.1m;no-incomplete-cover;v1";

/// Complete outward buffer carrier and a canonical mathematical certificate.
#[derive(Clone, Debug)]
pub struct BufferMaterialization {
    literal: Arc<GeometryLiteral>,
    source: Digest32,
    law: SemanticLawId,
    output_receipt: crate::MaterializedOutputReceipt,
}
impl PartialEq for BufferMaterialization {
    fn eq(&self, other: &Self) -> bool {
        self.literal == other.literal && self.source == other.source && self.law == other.law
    }
}
impl Eq for BufferMaterialization {}
impl BufferMaterialization {
    /// Frozen completed point-buffer polygon law, available before execution.
    /// Query provenance binds this identity separately from the source set.
    #[must_use]
    pub fn output_law_id() -> SemanticLawId {
        SemanticLawId::from_digest(crate::profile::hash_fields(
            LAW,
            [DESCRIPTION, UNION_DESCRIPTION],
        ))
    }
    /// Completed complete-source buffer law. The unchanged point-only law has
    /// its own identity; proofs and execution limits occur in neither identity.
    #[must_use]
    pub fn region_output_law_id() -> SemanticLawId {
        let selected = crate::atlas::topology_law_id().digest();
        SemanticLawId::from_digest(crate::profile::hash_fields(
            LAW,
            [REGION_DESCRIPTION, UNION_DESCRIPTION, selected.as_bytes()],
        ))
    }
    /// Completed global physical sublevel point-buffer law.
    #[must_use]
    pub fn global_output_law_id() -> SemanticLawId {
        SemanticLawId::from_digest(crate::profile::hash_fields(
            LAW,
            [
                GLOBAL_DESCRIPTION,
                GLOBAL_CARRIER_RADIUS_DESCRIPTION,
                UNION_DESCRIPTION,
            ],
        ))
    }
    /// Completed global physical sublevel complete-source buffer law.
    #[must_use]
    pub fn global_region_output_law_id() -> SemanticLawId {
        let selected = crate::atlas::topology_law_id().digest();
        SemanticLawId::from_digest(crate::profile::hash_fields(
            LAW,
            [
                GLOBAL_REGION_DESCRIPTION,
                GLOBAL_CARRIER_RADIUS_DESCRIPTION,
                UNION_DESCRIPTION,
                selected.as_bytes(),
            ],
        ))
    }
    /// Complete union carrier under its actual caller-declared reference.
    #[must_use]
    pub fn literal(&self) -> &GeometryLiteral {
        &self.literal
    }
    /// Exact source/radius offset identity.
    #[must_use]
    pub const fn source_id(&self) -> Digest32 {
        self.source
    }
    /// Completed output law, independent of admission and tighter proof receipts.
    #[must_use]
    pub const fn law_id(&self) -> SemanticLawId {
        self.law
    }
    /// Guaranteed extra physical radius beyond the requested exact buffer.
    #[must_use]
    pub fn outward_error(&self) -> Metres {
        Metres::new(decimal("0.1"))
    }
    crate::metric::source_bound_certificate!(b"B_r subset output subset B_(r+0.1m)");
}
impl OffsetRegion {
    /// Materialize the complete physical offset under its actual reference.
    ///
    /// # Errors
    /// Refuses an uncertified continuous source image or incomplete precision,
    /// work, workspace or output admission without returning partial geometry.
    pub fn materialize(
        &self,
        profile: &GeoProfile,
        target: &Crs,
        context: &mut MetricContext,
    ) -> Result<BufferMaterialization, GeoError> {
        self.materialize_points_inner(profile, target, None, context, None, false)
    }
    /// Materialize a complete offset with bounded cancellation/work polling.
    ///
    /// # Errors
    /// Preserves all materialization errors and the observer's original refusal.
    pub fn materialize_metered(
        &self,
        profile: &GeoProfile,
        target: &Crs,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<BufferMaterialization, GeoError> {
        self.materialize_points_inner(profile, target, None, context, Some(observer), false)
    }
    /// Continue a materialization with its original source already retained.
    ///
    /// # Errors
    /// Also refuses a different source or absent retained preparation receipt.
    pub fn materialize_prepared(
        &self,
        profile: &GeoProfile,
        target: &Crs,
        receipt: &crate::context::PreparedSourceReceipt,
        context: &mut MetricContext,
    ) -> Result<BufferMaterialization, GeoError> {
        self.materialize_points_inner(profile, target, Some(receipt), context, None, false)
    }
    /// Continue the same original invocation with bounded governor polling.
    ///
    /// # Errors
    /// Preserves receipt validation, materialization and observer errors.
    pub fn materialize_prepared_metered(
        &self,
        profile: &GeoProfile,
        target: &Crs,
        receipt: &crate::context::PreparedSourceReceipt,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<BufferMaterialization, GeoError> {
        self.materialize_points_inner(
            profile,
            target,
            Some(receipt),
            context,
            Some(observer),
            false,
        )
    }
    /// Materialize the physical buffer of a complete point collection.
    ///
    /// # Errors
    /// Refuses nonpoint sources, uncertified chart/cut-locus cases and incomplete
    /// output/work/workspace/precision admission; never publishes a partial union.
    pub fn materialize_points(
        &self,
        profile: &GeoProfile,
        target: &Crs,
        context: &mut MetricContext,
    ) -> Result<BufferMaterialization, GeoError> {
        self.materialize_points_inner(profile, target, None, context, None, true)
    }
    /// Materialize point buffers with bounded borrowed cancellation/work polls.
    ///
    /// # Errors
    /// Also preserves the observer's original refusal without partial geometry.
    pub fn materialize_points_metered(
        &self,
        profile: &GeoProfile,
        target: &Crs,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<BufferMaterialization, GeoError> {
        self.materialize_points_inner(profile, target, None, context, Some(observer), true)
    }

    /// Continue an invocation whose exact source receipt is already retained.
    /// The receipt must name this complete source and its unchanged storage;
    /// constructor work and source memory remain in the original governor.
    ///
    /// # Errors
    /// Refuses a different or absent retained source and all materialization errors.
    pub fn materialize_points_prepared(
        &self,
        profile: &GeoProfile,
        target: &Crs,
        receipt: &crate::context::PreparedSourceReceipt,
        context: &mut MetricContext,
    ) -> Result<BufferMaterialization, GeoError> {
        self.materialize_points_inner(profile, target, Some(receipt), context, None, true)
    }

    /// Continue the same retained-source materialization with governor polling.
    ///
    /// # Errors
    /// Also preserves the observer's original refusal.
    pub fn materialize_points_prepared_metered(
        &self,
        profile: &GeoProfile,
        target: &Crs,
        receipt: &crate::context::PreparedSourceReceipt,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<BufferMaterialization, GeoError> {
        self.materialize_points_inner(
            profile,
            target,
            Some(receipt),
            context,
            Some(observer),
            true,
        )
    }
    fn materialize_points_inner(
        &self,
        profile: &GeoProfile,
        target: &Crs,
        receipt: Option<&crate::context::PreparedSourceReceipt>,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
        point_only: bool,
    ) -> Result<BufferMaterialization, GeoError> {
        if profile.reference(target)? != self.source.reference()
            || context.reference() != self.source.reference()
        {
            return Err(GeoError::config(
                "buffer materialization requires its original actual geographic reference",
            ));
        }
        if let Some(receipt) = receipt {
            if receipt.source_id() != self.source.id()
                || receipt.workspace_bytes() != self.source.retained_workspace_bytes()
                || receipt.work_items() != self.source.preparation_work_items()
            {
                return Err(GeoError::config(
                    "buffer receipt names a different original source",
                ));
            }
            receipt.validate_context(context)?;
        }
        context.begin(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.context_poll(context)?;
        let general = !self.source.curves().is_empty()
            || !matches!(self.source.region(), PreparedRegion::Empty);
        if point_only && general {
            return Err(GeoError::domain(
                "point materialization requires a complete point collection",
            ));
        }
        let source_bytes = if receipt.is_some() {
            0
        } else {
            self.source.retained_workspace_bytes()
        };
        context.admit_workspace(source_bytes)?;
        let mut retained = MaterializationStorage::admitted_transient(source_bytes);
        let mut global_output = false;
        let result = (|| {
            let output_bytes = (size_of::<BufferMaterialization>() as u64)
                .checked_add((size_of::<GeometryLiteral>() + 2 * size_of::<usize>()) as u64)
                .and_then(|bytes| bytes.checked_add(target.retained_text_bytes() as u64))
                .ok_or(GeoError::ArithmeticOverflow("buffer carrier containers"))?;
            retained.admit_output(output_bytes, context)?;
            let body_output_start = retained.output_bytes();
            let mut total = 0;
            let body = if general {
                covering::complete_body(
                    self,
                    (profile, target),
                    context,
                    &mut progress,
                    &mut total,
                    &mut retained,
                    &mut global_output,
                )?
            } else if self.radius.exact().numerator().is_negative()
                || (self.source.points().is_empty() && self.source.symbolic_points().is_empty())
            {
                GeometryBody::GeometryCollection(Vec::new())
            } else if self.radius.exact().is_zero() {
                if !self.source.symbolic_points().is_empty() {
                    // A rounded export cannot represent the exact closure of an
                    // irrational image point. Native OffsetRegion membership
                    // continues to evaluate its unrounded source law.
                    return Err(GeoError::PrecisionExhausted {
                        bits: context.policy().limits().max_precision_bits,
                    });
                }
                admit_vertices(
                    self.source.points().len() as u64,
                    &mut total,
                    context,
                    &mut retained,
                )?;
                let mut points = Vec::with_capacity(self.source.points().len());
                for point in self.source.points() {
                    points.push(Some(carrier_admitted(
                        point.point(),
                        context,
                        &mut progress,
                        &mut retained,
                    )?));
                }
                GeometryBody::MultiPoint(points)
            } else {
                let prepared = prepare_disks(context, &mut progress, &mut retained)?;
                // Validate every symbolic operation image even when the
                // global diameter makes its location immaterial to the disk.
                // A shortcut must not turn an inadmissible source into Whole.
                let centers = point_centers(&self.source, context, &mut progress, &mut retained)?;
                point_polygons(
                    DiskRequest {
                        centers: &centers,
                        radius: self.radius.exact(),
                        prepared: &prepared,
                        profile,
                        target,
                        global_band: &decimal("0.099998"),
                    },
                    context,
                    &mut progress,
                    &mut total,
                    &mut retained,
                    &mut global_output,
                )?
            };
            let literal = GeometryLiteral::new(target.clone(), Geometry::new(CoordDim::Xy, body)?);
            // The local secant certificate and injectivity proof already
            // establish a single circle's complete simple ring. An arrangement
            // is needed only to union multiple original disk components.
            let single_center = self
                .source
                .points()
                .len()
                .saturating_add(self.source.symbolic_points().len())
                == 1;
            let literal = if !single_center
                && matches!(literal.geometry().body(), GeometryBody::MultiPolygon(polygons) if polygons.len() > 1)
            {
                let remaining = context.policy().remaining_after(
                    context.work_items(),
                    context
                        .policy()
                        .limits()
                        .max_workspace_bytes
                        .saturating_sub(context.remaining_workspace()),
                )?;
                let preparation = {
                    let mut nested = progress.nested(
                        context.work_items(),
                        context
                            .policy()
                            .limits()
                            .max_workspace_bytes
                            .saturating_sub(context.remaining_workspace()),
                        context.workspace_peak(),
                    );
                    crate::PreparedGeometry::from_literal_in_policy_metered(
                        &literal,
                        profile,
                        remaining,
                        &mut nested,
                    )
                };
                let prepared = progress.absorb_nested(context, preparation)?;
                let bytes = prepared.retained_workspace_bytes();
                context.admit_workspace(bytes)?;
                let original_output = retained.output_bytes() - body_output_start;
                let result = crate::atlas::union::materialize_with_output(
                    prepared.region(),
                    context,
                    &mut progress,
                    |geometry, available, context, progress| {
                        retained.adopt_geometry_output(
                            geometry,
                            available,
                            context,
                            progress,
                            |count, context| {
                                let mut replacement = 0;
                                admit_vertex_count(count, &mut replacement, context)?;
                                total = replacement;
                                Ok(())
                            },
                        )
                    },
                );
                drop(prepared);
                context.release_workspace(bytes)?;
                let mut area = result?;
                if context.reference().axes() == AxisOrder::LatLon {
                    area = covering::swap_area_axes(area, context, &mut progress)?;
                }
                drop(literal);
                retained.release_output(original_output, context)?;
                GeometryLiteral::new(target.clone(), area)
            } else {
                literal
            };
            Ok(literal)
        })();
        let receipt = retained.finish(result.is_ok(), context, &mut progress)?;
        Ok(BufferMaterialization {
            literal: Arc::new(result?),
            source: self.id(),
            output_receipt: receipt.expect("successful complete buffer owns a receipt"),
            law: match (general, global_output) {
                (false, false) => BufferMaterialization::output_law_id(),
                (true, false) => BufferMaterialization::region_output_law_id(),
                (false, true) => BufferMaterialization::global_output_law_id(),
                (true, true) => BufferMaterialization::global_region_output_law_id(),
            },
        })
    }
}

fn prepare_disks(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut MaterializationStorage,
) -> Result<PreparedGeodesic, GeoError> {
    let reference = crate::numerical::reference_clone(context, progress)?;
    // Admit and observe the complete reusable arena before the preparation
    // entry can render a cold binding or allocate immutable tables. The child
    // borrows this same parent-owned arena through its original context seam.
    context.prepare_integer_scratch_for_observed(
        crate::numerical::scratch_source_bits(&[], reference.ellipsoid()),
        progress,
    )?;
    let mut child = context.remaining_child()?;
    let prepared = {
        let mut observer = progress.nested(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
            context.workspace_peak(),
        );
        PreparedGeodesic::prepare_metered(reference, &mut child, &mut observer)
    };
    let prepared = progress.absorb_child_result(context, &child, prepared)?;
    if let Some(arithmetic) = child.prepared_arithmetic() {
        context.set_prepared_arithmetic(arithmetic);
    }
    let bytes = prepared.retained_workspace_bytes();
    retained.admit_transient(bytes, context)?;
    progress.context_poll(context)?;
    Ok(prepared)
}

#[derive(Clone, Copy)]
struct DiskRequest<'call, 'source> {
    centers: &'call [Cow<'source, LonLat>],
    radius: &'call Rat,
    prepared: &'call PreparedGeodesic,
    profile: &'call GeoProfile,
    target: &'call Crs,
    global_band: &'call Rat,
}

fn point_polygons(
    request: DiskRequest<'_, '_>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    total: &mut u64,
    retained: &mut MaterializationStorage,
    global_output: &mut bool,
) -> Result<GeometryBody, GeoError> {
    let DiskRequest {
        centers,
        radius,
        prepared,
        profile,
        target,
        global_band,
    } = request;
    Ok(if whole_disk(radius, prepared, context, progress)? {
        admit_vertices(5, total, context, retained)?;
        GeometryBody::MultiPolygon(vec![whole_surface_ring(context.reference().axes())])
    } else {
        use purrdf_xsd::integer::ExactOperation::{RationalAdd, RationalDivide, RationalMultiply};
        let guard = decimal("0.05");
        let rho = crate::numerical::ExactAdmission::new(context, progress).rational(
            RationalAdd,
            &[radius, &guard],
            1,
            || Ok(Metres::new(radius.add(&guard))),
        )?;
        let admitted = prepared.admitted_reference(context, progress)?;
        let mut polygons = Vec::new();
        for point in centers {
            if point.is_pole() {
                admit_vertices(5, total, context, retained)?;
                let rings = polar_disk(point, radius, prepared, context, progress)?;
                polygons.push(rings);
                continue;
            }
            let (count, atlas_chart) = match circle_count(point, radius, context, progress)? {
                CircleCount::Local(count) => (count, None),
                CircleCount::Atlas { count, pole } => {
                    *global_output = true;
                    (count, Some(pole))
                }
                CircleCount::Global => {
                    // An original symbolic center may be within 1um of its
                    // degree15 export. Cover that uncertainty on the inside;
                    // the 0.099998m cell band leaves the same 1um outside.
                    let padding = decimal("0.000001");
                    let radius = crate::numerical::ExactAdmission::new(context, progress)
                        .rational(RationalAdd, &[radius, &padding], 1, || {
                            Ok(radius.add(&padding))
                        })?;
                    let body = global::materialize(
                        &global::Request {
                            source: global::Source::Point(point, prepared),
                            radius: &radius,
                            profile,
                            target,
                            band: global_band,
                        },
                        context,
                        progress,
                        total,
                        retained,
                    )?;
                    let GeometryBody::MultiPolygon(pieces) = body else {
                        return Ok(body);
                    };
                    polygons.extend(pieces);
                    *global_output = true;
                    continue;
                }
                CircleCount::OutputLimit => {
                    return Err(GeoError::OutputExhausted {
                        limit: context.policy().limits().max_output_elements,
                    });
                }
            };
            let ring_vertices = count
                + if atlas_chart.flatten().is_some() {
                    4
                } else {
                    1
                };
            let ring_bytes = ring_vertices
                .checked_mul(2048)
                .ok_or(GeoError::ArithmeticOverflow("temporary buffer circle"))?;
            retained.admit_transient(ring_bytes, context)?;
            let capacity =
                usize::try_from(ring_vertices).map_err(|_| GeoError::OutputExhausted {
                    limit: context.policy().limits().max_output_elements,
                })?;
            let mut ring: Vec<Coord> = Vec::with_capacity(capacity);
            let turn = Rat::from_i64(360);
            let denominator = Rat::from_int(Int::from_u64(count));
            for i in 0..count {
                let index = Rat::from_int(Int::from_u64(i));
                let mut admission = crate::numerical::ExactAdmission::new(context, progress);
                let numerator =
                    admission.rational(RationalMultiply, &[&turn, &index], 1, || {
                        Ok(turn.mul(&index))
                    })?;
                let angle =
                    admission.rational(RationalDivide, &[&numerator, &denominator], 1, || {
                        Ok(numerator.div(&denominator).expect("positive circle count"))
                    })?;
                let mut child = context.remaining_child()?;
                let output = {
                    let mut observer = progress.nested(
                        context.work_items(),
                        context
                            .policy()
                            .limits()
                            .max_workspace_bytes
                            .saturating_sub(context.remaining_workspace()),
                        context.workspace_peak(),
                    );
                    admitted.direct_metered(point, &angle, &rho, &mut child, &mut observer)
                };
                let output = progress.absorb_child_result(context, &child, output)?;
                let lifted = if atlas_chart.is_some() && !ring.is_empty() {
                    cut::lifted_from_longitude(
                        output.endpoint(),
                        ring.last().expect("nonempty").x(),
                        context,
                        progress,
                    )?
                } else {
                    cut::lifted(output.endpoint(), point, context, progress)?
                };
                ring.push(lifted);
            }
            if let Some(Some(pole)) = atlas_chart {
                // Increasing azimuth winds clockwise: longitude decreases once
                // about North, and increases once about South. The endpoints of
                // the lifted open chart denote the same physical source point.
                let winding = Rat::from_i64(-360 * i64::from(pole));
                let longitude = crate::numerical::ExactAdmission::new(context, progress).rational(
                    RationalAdd,
                    &[ring[0].x(), &winding],
                    1,
                    || Ok(ring[0].x().add(&winding)),
                )?;
                let latitude = crate::numerical::copy_rat(ring[0].y(), context, progress)?;
                ring.push(Coord::xy(longitude, latitude));
            }
            ring.reverse();
            if let Some(Some(pole)) = atlas_chart {
                let latitude = Rat::from_i64(90 * i64::from(pole));
                let last = crate::numerical::copy_rat(
                    ring.last().expect("boundary").x(),
                    context,
                    progress,
                )?;
                let first = crate::numerical::copy_rat(ring[0].x(), context, progress)?;
                ring.push(Coord::xy(last, latitude.clone()));
                ring.push(Coord::xy(first, latitude));
            }
            let closing = crate::numerical::ExactAdmission::new(context, progress).rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[ring[0].x(), ring[0].y()],
                2,
                || Ok(ring[0].clone()),
            )?;
            ring.push(closing);
            let pieces = if atlas_chart.is_some() {
                cut::atlas_polygons(ring, context, progress, total, retained)?
            } else {
                cut::polygons(ring, context, progress, total, retained)?
            };
            retained.release_transient(ring_bytes, context)?;
            polygons.extend(pieces);
        }
        purrdf_lex::walk::try_sort_unstable_by(&mut polygons, |a, b| {
            Ok(coordinate_order(&a[0][0], &b[0][0], context, progress)?
                .then_with(|| a[0].len().cmp(&b[0].len())))
        })?;
        GeometryBody::MultiPolygon(polygons)
    })
}

/// Export centers only for the certified polygon approximation. The complete
/// symbolic sources remain in OffsetRegion and every exact metric inventory.
fn point_centers<'source>(
    source: &'source crate::PreparedGeometry,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut MaterializationStorage,
) -> Result<Vec<Cow<'source, LonLat>>, GeoError> {
    let count = source
        .points()
        .len()
        .checked_add(source.symbolic_points().len())
        .ok_or(GeoError::ArithmeticOverflow("buffer center count"))?;
    let bytes = (count as u64)
        .checked_mul((size_of::<Cow<'_, LonLat>>() + size_of::<LonLat>()) as u64)
        .ok_or(GeoError::ArithmeticOverflow("buffer center storage"))?;
    retained.admit_transient(bytes, context)?;
    let mut centers = Vec::with_capacity(count);
    centers.extend(
        source
            .points()
            .iter()
            .map(|point| Cow::Borrowed(point.point())),
    );
    if !source.symbolic_points().is_empty() {
        certify_center_grid(context, progress)?;
    }
    for point in source.symbolic_points() {
        let mut child = context.remaining_child()?;
        let output = {
            let mut observer = progress.nested(
                context.work_items(),
                context
                    .policy()
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace()),
                context.workspace_peak(),
            );
            point.materialize_metered(&mut child, &mut observer)
        };
        let output = progress.absorb_child_result(context, &child, output)?;
        let crate::operation::OperationPoint { x, y, .. } = output.into_point();
        let bytes = (x.allocated_bytes() + y.allocated_bytes()) as u64;
        retained.admit_transient(bytes, context)?;
        let (longitude, latitude) = match context.reference().axes() {
            AxisOrder::LonLat => (x, y),
            AxisOrder::LatLon => (y, x),
        };
        centers.push(Cow::Owned(LonLat::new(longitude, latitude)?));
    }
    Ok(centers)
}

fn certify_center_grid(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(), GeoError> {
    let reference = crate::numerical::reference_clone(context, progress)?;
    let policy = context.policy();
    crate::numerical::with_math(context, progress, 96, 2048, |math, _| {
        use purrdf_xsd::math::{FixedInterval, MathError};
        (|| {
            let (_, radius) = reference.ellipsoid().normal_metric_bounds_ref();
            // Each angular ordinate is correctly quantized at fifteen places.
            // A meridian/parallel path bounds their summed half-grid displacement.
            let error = crate::numerical::fixed_from_rat(radius.exact(), math)?
                .mul(&FixedInterval::pi(math)?, math)?
                .mul(
                    &crate::numerical::fixed_from_rat(&decimal("0.000000000000001"), math)?,
                    math,
                )?
                .div(&FixedInterval::from_i64(180, math)?, math)?;
            let limit = crate::numerical::fixed_from_rat(&decimal("0.000001"), math)?;
            if error.upper() <= limit.lower() {
                Ok(())
            } else {
                Err(MathError::PrecisionExhausted)
            }
        })()
        .map_err(|error| crate::numerical::geo_math_error(&error, policy))
    })
}

pub(super) fn carrier(point: &LonLat, axes: AxisOrder) -> Coord {
    match axes {
        AxisOrder::LonLat => Coord::xy(point.longitude().clone(), point.latitude().clone()),
        AxisOrder::LatLon => Coord::xy(point.latitude().clone(), point.longitude().clone()),
    }
}
fn carrier_admitted(
    point: &LonLat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut MaterializationStorage,
) -> Result<Coord, GeoError> {
    // Exact zero closure may carry arbitrarily wide original decimal operands.
    // Its copied heap storage is additional to the ordinary 2048-byte output
    // vertex allowance; admit it before the original coordinate is cloned.
    let bytes = point
        .longitude()
        .allocated_bytes()
        .checked_add(point.latitude().allocated_bytes())
        .and_then(|bytes| bytes.checked_mul(2))
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or(GeoError::ArithmeticOverflow("exact buffer closure storage"))?;
    retained.admit_output(bytes, context)?;
    let axes = context.reference().axes();
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::Linear,
        &[point.longitude(), point.latitude()],
        2,
        || Ok(carrier(point, axes)),
    )
}
fn coordinate_order(
    a: &Coord,
    b: &Coord,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<core::cmp::Ordering, GeoError> {
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[a.x(), a.y(), b.x(), b.y()],
        2,
        || Ok(crate::topology::cmp_xy(a, b)),
    )
}
fn admit_vertices(
    count: u64,
    total: &mut u64,
    context: &mut MetricContext,
    retained: &mut MaterializationStorage,
) -> Result<(), GeoError> {
    admit_vertex_count(count, total, context)?;
    let bytes = count
        .checked_mul(2048)
        .ok_or(GeoError::ArithmeticOverflow("buffer output storage"))?;
    retained.admit_output(bytes, context)?;
    Ok(())
}

fn admit_vertex_count(
    count: u64,
    total: &mut u64,
    context: &MetricContext,
) -> Result<(), GeoError> {
    let complete = total
        .checked_add(count)
        .ok_or(GeoError::ArithmeticOverflow("buffer output vertices"))?;
    if complete > context.policy().limits().max_output_elements {
        return Err(GeoError::OutputExhausted {
            limit: context.policy().limits().max_output_elements,
        });
    }
    *total = complete;
    Ok(())
}
/// Every pair of points is connected by the two broken meridian paths
/// through the north and south poles. Their lengths sum to twice the complete
/// meridian half-circuit D, so the shorter is at most D. Also ds >= M|dphi|
/// proves the pole-to-pole distance is exactly D. Thus this is an ellipsoidal
/// diameter upper bound for every original or symbolic center, including its
/// entire cut locus, without a center-coordinate approximation.
fn whole_disk(
    radius: &Rat,
    prepared: &PreparedGeodesic,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<bool, GeoError> {
    let padded = padded_whole_radius(radius, context, progress)?;
    let reference = crate::numerical::reference_clone(context, progress)?;
    let minimum = reference.ellipsoid().normal_metric_bounds_ref().0.exact();
    let three_m = crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalMultiply,
        &[minimum, &Rat::from_i64(3)],
        1,
        || Ok(minimum.mul(&Rat::from_i64(3))),
    )?;
    // D >= pi*m > 3*m rules out the global case before the inverse kernel.
    let local = crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[&padded, &three_m],
        1,
        || Ok(padded < three_m),
    )?;
    if local {
        return Ok(false);
    }
    let diameter = meridian_diameter(prepared, context, progress)?;
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[diameter.upper.exact(), &padded],
        1,
        || Ok(diameter.upper.exact() <= &padded),
    )
}

fn padded_whole_radius(
    radius: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    let guard = decimal("0.099998");
    crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalAdd,
        &[radius, &guard],
        1,
        || Ok(radius.add(&guard)),
    )
}

fn meridian_diameter(
    prepared: &PreparedGeodesic,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<crate::MetricProofReceipt, GeoError> {
    let north = LonLat::new(Rat::zero(), Rat::from_i64(90)).expect("exact north pole");
    let south = LonLat::new(Rat::zero(), Rat::from_i64(-90)).expect("exact south pole");
    let mut child = context.remaining_child()?;
    let diameter = {
        let mut observer = progress.nested(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
            context.workspace_peak(),
        );
        prepared.distance_with_proof_metered(&north, &south, &mut child, &mut observer)
    };
    progress
        .absorb_child_result(context, &child, diameter)
        .map(|(_, proof)| proof)
}

fn whole_surface_ring(axes: AxisOrder) -> Vec<Vec<Coord>> {
    vec![
        [(-180, -90), (180, -90), (180, 90), (-180, 90), (-180, -90)]
            .into_iter()
            .map(|(longitude, latitude)| match axes {
                AxisOrder::LonLat => Coord::xy(Rat::from_i64(longitude), Rat::from_i64(latitude)),
                AxisOrder::LatLon => Coord::xy(Rat::from_i64(latitude), Rat::from_i64(longitude)),
            })
            .collect(),
    ]
}

fn polar_disk(
    center: &LonLat,
    radius: &Rat,
    prepared: &PreparedGeodesic,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Vec<Vec<Coord>>, GeoError> {
    let diameter = meridian_diameter(prepared, context, progress)?;
    let padded = padded_whole_radius(radius, context, progress)?;
    let whole = crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[diameter.upper.exact(), &padded],
        1,
        || Ok(diameter.upper.exact() <= &padded),
    )?;
    if whole {
        return Ok(whole_surface_ring(context.reference().axes()));
    }
    let guard = decimal("0.05");
    let rho = crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalAdd,
        &[radius, &guard],
        1,
        || Ok(radius.add(&guard)),
    )?;
    let unresolved = crate::numerical::ExactAdmission::new(context, progress).rational(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        &[&rho, diameter.lower.exact()],
        1,
        || Ok(&rho >= diameter.lower.exact()),
    )?;
    if unresolved {
        return Err(GeoError::PrecisionExhausted {
            bits: context.policy().limits().max_precision_bits,
        });
    }
    let mut child = context.remaining_child()?;
    let boundary = {
        let mut observer = progress.nested(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
            context.workspace_peak(),
        );
        prepared.direct_metered(
            center,
            &Rat::zero(),
            &Metres::new(rho),
            &mut child,
            &mut observer,
        )
    };
    let boundary = progress.absorb_child_result(context, &child, boundary)?;
    let latitude = boundary.endpoint().latitude();
    let mut ring = [
        LonLat::new(Rat::from_i64(-180), latitude.clone())?,
        LonLat::new(Rat::from_i64(180), latitude.clone())?,
        LonLat::new(Rat::from_i64(180), center.latitude().clone())?,
        LonLat::new(Rat::from_i64(-180), center.latitude().clone())?,
    ];
    if center.latitude() < &Rat::zero() {
        ring.reverse();
    }
    let mut ring: Vec<_> = ring
        .iter()
        .map(|point| carrier(point, context.reference().axes()))
        .collect();
    ring.push(ring[0].clone());
    Ok(vec![ring])
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CircleCount {
    Local(u64),
    Atlas { count: u64, pole: Option<i8> },
    Global,
    OutputLimit,
}

/// Exact original radius comparisons select the geometric proof family.
/// In particular, equality with m or 3m must not remain an interval tie whose
/// success changes with arithmetic precision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CircleDomain {
    StronglyConvex,
    BeforeCutLocus,
    Global,
}

fn circle_domain(
    radius: &Rat,
    ellipsoid: &crate::PreparedEllipsoid,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<CircleDomain, GeoError> {
    use purrdf_xsd::integer::ExactOperation::{RationalAdd, RationalCompare, RationalMultiply};
    let guard = decimal("0.05");
    let mut admission = crate::numerical::ExactAdmission::new(context, progress);
    let rho = admission.rational(RationalAdd, &[radius, &guard], 1, || Ok(radius.add(&guard)))?;
    let minimum = ellipsoid.normal_metric_bounds_ref().0.exact();
    if admission.rational(RationalCompare, &[&rho, minimum], 1, || Ok(&rho < minimum))? {
        return Ok(CircleDomain::StronglyConvex);
    }
    let three = Rat::from_i64(3);
    let cutoff = admission.rational(RationalMultiply, &[minimum, &three], 1, || {
        Ok(minimum.mul(&three))
    })?;
    if admission.rational(RationalCompare, &[&rho, &cutoff], 1, || Ok(rho < cutoff))? {
        Ok(CircleDomain::BeforeCutLocus)
    } else {
        Ok(CircleDomain::Global)
    }
}

fn circle_count(
    center: &LonLat,
    radius: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<CircleCount, GeoError> {
    let reference = crate::numerical::reference_clone(context, progress)?;
    let domain = circle_domain(radius, reference.ellipsoid(), context, progress)?;
    if domain == CircleDomain::Global {
        return Ok(CircleCount::Global);
    }
    let policy = context.policy();
    let mut bits = 96.min(policy.limits().max_precision_bits);
    loop {
        let result = crate::numerical::with_math_for_sources(
            context,
            progress,
            bits,
            8192,
            &[center.longitude(), center.latitude(), radius],
            |math, progress| {
                circle_count_attempt(center, radius, &reference, domain, policy, math, progress)
                    .map_err(|error| crate::numerical::geo_math_error(&error, policy))
            },
        );
        match result {
            Err(GeoError::PrecisionExhausted { .. })
                if bits < policy.limits().max_precision_bits =>
            {
                bits = bits
                    .saturating_mul(2)
                    .min(policy.limits().max_precision_bits);
            }
            result => return result,
        }
    }
}

/// Prove each decision of the exact original circle-bound expression. An
/// unresolved comparison refines arithmetic, never selects a different circle
/// count. This avoids constructing large compound rational denominators solely
/// to establish an inequality, while preserving the smallest admitted dyadic N.
fn circle_count_attempt(
    center: &LonLat,
    radius: &Rat,
    reference: &crate::GeographicReference,
    domain: CircleDomain,
    policy: crate::ExecutionPolicy,
    math: &mut purrdf_xsd::math::CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<CircleCount, purrdf_xsd::math::MathError> {
    use crate::numerical::fixed_from_rat;
    use purrdf_xsd::math::{FixedInterval, MathError};
    let one = FixedInterval::from_i64(1, math)?;
    let two = FixedInterval::from_i64(2, math)?;
    let three = FixedInterval::from_i64(3, math)?;
    let sixty = FixedInterval::from_i64(60, math)?;
    let ninety = FixedInterval::from_i64(90, math)?;
    let ellipsoid = reference.ellipsoid();
    let (minimum, maximum) = ellipsoid.normal_metric_bounds_ref();
    let m = fixed_from_rat(minimum.exact(), math)?;
    let upper = fixed_from_rat(maximum.exact(), math)?;
    let rho = fixed_from_rat(radius, math)?.add(&fixed_from_rat(&decimal("0.05"), math)?, math)?;
    let t = rho.div(&m, math)?;
    let t2 = t.square(math)?;
    // pi>3 gives the same conservative degree excursions as the exact law.
    let latitude = fixed_from_rat(center.latitude(), math)?
        .abs(math)?
        .add(&t.mul(&sixty, math)?, math)?
        .add(&fixed_from_rat(&decimal("0.000000000000001"), math)?, math)?;
    let mut cosine = one.sub(&latitude.div(&ninety, math)?, math)?;
    let mut atlas_chart = None;
    let mut geometric_fallback = cosine.upper().is_negative() || cosine.upper().is_zero();
    if !geometric_fallback && (cosine.lower().is_negative() || cosine.lower().is_zero()) {
        return Err(MathError::PrecisionExhausted);
    }
    if !geometric_fallback {
        // Preserve the original local expression and its degree-grid guard.
        let excursion = t
            .div(&cosine, math)?
            .mul(&sixty, math)?
            .add(&fixed_from_rat(&decimal("0.000000000000001"), math)?, math)?;
        let half_turn = FixedInterval::from_i64(180, math)?;
        if excursion.lower() >= half_turn.upper() {
            geometric_fallback = true;
        } else if excursion.upper() >= half_turn.lower() {
            return Err(MathError::PrecisionExhausted);
        }
    }
    if geometric_fallback {
        let Some(chart) = pole_clearance_chart(center, &rho, ellipsoid, domain, math, progress)?
        else {
            return Ok(CircleCount::Global);
        };
        cosine = chart.minimum_cosine;
        atlas_chart = Some((chart.maximum_cosine, chart.pole));
    }
    let reciprocal = one.div(&cosine, math)?;
    let axis_ratio = fixed_from_rat(ellipsoid.semiminor(), math)?
        .div(&fixed_from_rat(ellipsoid.semimajor(), math)?, math)?;
    let e2 = one.sub(&axis_ratio.square(math)?, math)?;
    let eccentric = e2.div(&one.sub(&e2, math)?, math)?;
    let acceleration = t
        .mul(&one.add(&t2.div(&two, math)?, math)?, math)?
        .add(&t2.square(math)?.div(&three, math)?, math)?;
    let g1 = eccentric.mul(&three, math)?;
    let g2 = upper
        .div(&m, math)?
        .square(math)?
        .mul(&one.add(&eccentric, math)?, math)?;
    let g3 = eccentric.add(&reciprocal, math)?;
    let connection = g1
        .add(&g2.mul(&reciprocal.square(math)?, math)?, math)?
        .add(&g3.mul(&reciprocal, math)?.mul(&two, math)?, math)?;
    let bound = if let Some((maximum_cosine, _)) = &atlas_chart {
        // |Gamma^phi_lambda_lambda| <= g2*cos(phi), not merely g2.
        // The physical longitude secant is also multiplied by cos(phi).
        let latitude_acceleration = acceleration.add(
            &t2.mul(
                &g1.add(
                    &g2.mul(maximum_cosine, math)?
                        .mul(&reciprocal.square(math)?, math)?,
                    math,
                )?,
                math,
            )?,
            math,
        )?;
        let longitude_acceleration = acceleration.mul(&reciprocal, math)?.add(
            &t2.mul(&g3, math)?.mul(&reciprocal, math)?.mul(&two, math)?,
            math,
        )?;
        upper.mul(
            &latitude_acceleration
                .add(&maximum_cosine.mul(&longitude_acceleration, math)?, math)?,
            math,
        )?
    } else {
        upper.mul(
            &acceleration
                .mul(&one.add(&reciprocal, math)?, math)?
                .add(&t2.mul(&connection, math)?, math)?,
            math,
        )?
    };
    let full_turn = fixed_from_rat(
        &decimal("6.28318530717958647692528676655900576839433879875022"),
        math,
    )?;
    let numerator = bound.mul(&full_turn.square(math)?, math)?;
    let limit = fixed_from_rat(&decimal("0.019"), math)?;
    let eight = FixedInterval::from_i64(8, math)?;
    let mut count = 8_u64;
    loop {
        let denominator = fixed_from_rat(&Rat::from_int(Int::from_u64(count)), math)?
            .square(math)?
            .mul(&eight, math)?;
        let allowed = limit.mul(&denominator, math)?;
        if numerator.upper() <= allowed.lower() {
            return Ok(atlas_chart.map_or(CircleCount::Local(count), |(_, pole)| {
                CircleCount::Atlas { count, pole }
            }));
        }
        if numerator.lower() <= allowed.upper() {
            return Err(MathError::PrecisionExhausted);
        }
        count = count.checked_mul(2).ok_or(MathError::Binary64Range)?;
        if count.saturating_add(
            if atlas_chart.as_ref().is_some_and(|(_, pole)| pole.is_some()) {
                4
            } else {
                1
            },
        ) > policy.limits().max_output_elements
        {
            // Every smaller dyadic count was proved insufficient. Propagate
            // an output refusal after the numerical phase charges its work;
            // no partial circle or weaker secant certificate is returned.
            return Ok(CircleCount::OutputLimit);
        }
    }
}

struct PoleClearanceChart {
    minimum_cosine: purrdf_xsd::math::FixedInterval,
    maximum_cosine: purrdf_xsd::math::FixedInterval,
    pole: Option<i8>,
}

/// The exact original meridian integral certifies clearance from both poles.
/// For rho<m the ball is strongly convex: focal and injectivity radii are
/// bounded below by pi*m/2 and pi*m. Each meridian ray therefore meets its
/// boundary at most once when its pole is inside, twice when outside.
fn pole_clearance_chart(
    center: &LonLat,
    rho: &purrdf_xsd::math::FixedInterval,
    ellipsoid: &crate::PreparedEllipsoid,
    domain: CircleDomain,
    math: &mut purrdf_xsd::math::CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<PoleClearanceChart>, purrdf_xsd::math::MathError> {
    use crate::numerical::fixed_from_rat;
    use purrdf_xsd::math::{FixedInterval, MathError};
    let (minimum, maximum) = ellipsoid.normal_metric_bounds_ref();
    let m = fixed_from_rat(minimum.exact(), math)?;
    if domain != CircleDomain::StronglyConvex {
        return Ok(None);
    }
    let quarter =
        crate::geodesic::meridian_arc_observed(&Rat::from_i64(90), ellipsoid, math, progress)?;
    let center_meridian =
        crate::geodesic::meridian_arc_observed(center.latitude(), ellipsoid, math, progress)?;
    let north = quarter.sub(&center_meridian, math)?;
    let south = quarter.add(&center_meridian, math)?;
    let north_clearance = north.sub(rho, math)?.abs(math)?;
    let south_clearance = south.sub(rho, math)?.abs(math)?;
    let clearance = if north_clearance.upper() <= south_clearance.lower() {
        north_clearance
    } else if south_clearance.upper() <= north_clearance.lower() {
        south_clearance
    } else {
        // Only lower bounds are needed; the minimum itself need not select a
        // pole. Both full intervals independently prove this exact lower bound.
        FixedInterval::from_bounds(
            north_clearance.lower().min(south_clearance.lower()).clone(),
            north_clearance.upper().min(south_clearance.upper()).clone(),
            math,
        )?
    };
    let guard = fixed_from_rat(&decimal("0.000002"), math)?;
    if clearance.upper() < guard.lower() {
        return Ok(None);
    }
    if clearance.lower() < guard.upper() {
        return Err(MathError::PrecisionExhausted);
    }
    let pi = FixedInterval::pi(math)?;
    let upper = fixed_from_rat(maximum.exact(), math)?;
    let grid = pi
        .mul(&fixed_from_rat(&decimal("0.000000000000001"), math)?, math)?
        .div(&FixedInterval::from_i64(180, math)?, math)?;
    let minimum_cosine = clearance
        .mul(&FixedInterval::from_i64(2, math)?, math)?
        .div(&pi.mul(&upper, math)?, math)?
        .sub(&grid, math)?;
    if minimum_cosine.lower().is_negative() || minimum_cosine.lower().is_zero() {
        return Err(MathError::PrecisionExhausted);
    }
    // The nearest pole bounds colatitude above: m*colatitude <= D(pole,x)
    // <= D(pole,center)+rho. This also covers every carrier secant latitude.
    let nearest = FixedInterval::from_bounds(
        north.lower().min(south.lower()).clone(),
        north.upper().min(south.upper()).clone(),
        math,
    )?;
    let maximum_cosine = nearest.add(rho, math)?.div(&m, math)?.add(&grid, math)?;
    let one = FixedInterval::from_i64(1, math)?;
    // min is monotone in both arguments. Enclose the frozen expression
    // directly, including an exact tie, instead of choosing a successful
    // geometric branch from the current enclosure's width.
    let maximum_cosine = FixedInterval::from_bounds(
        maximum_cosine.lower().min(one.lower()).clone(),
        maximum_cosine.upper().min(one.upper()).clone(),
        math,
    )?;
    let pole = if north.upper() < rho.lower() {
        Some(1)
    } else if south.upper() < rho.lower() {
        Some(-1)
    } else {
        None
    };
    Ok(Some(PoleClearanceChart {
        minimum_cosine,
        maximum_cosine,
        pole,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numerical::fixed_from_rat;
    use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathLimits};

    #[test]
    fn disk_preparation_observes_large_memory_growth_before_allocating() {
        struct RejectArena {
            observed: u64,
        }
        impl MetricWorkObserver for RejectArena {
            fn charge_chunk(&mut self, _: u64, growth: u64) -> Result<(), GeoError> {
                self.observed = self.observed.saturating_add(growth);
                if self.observed > 262_144 {
                    Err(GeoError::MemoryExhausted { limit: 262_144 })
                } else {
                    Ok(())
                }
            }
        }
        let mut context = MetricContext::wgs84().unwrap();
        context.begin(1).unwrap();
        let mut observer = RejectArena { observed: 0 };
        let mut progress = WorkProgress::new(Some(&mut observer));
        let mut storage = MaterializationStorage::admitted_transient(0);
        let instrument = purrdf_alloc_probe::CurrentThreadWindow::open();
        std::hint::black_box(Vec::<u8>::with_capacity(512));
        assert_eq!(instrument.close().allocations, 1);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = prepare_disks(&mut context, &mut progress, &mut storage);
        let measured = window.close();
        assert_eq!(
            result.unwrap_err(),
            GeoError::MemoryExhausted { limit: 262_144 }
        );
        assert_eq!(measured.allocations, 0);
        assert!(observer.observed > 262_144);
        assert!(context.integer_scratch().is_none());
        assert_eq!(context.retained_workspace_bytes(), 0);
        assert_eq!(
            context.remaining_workspace(),
            context.policy().limits().max_workspace_bytes
        );
    }

    #[test]
    fn radius_proof_domains_resolve_exact_original_cutoffs() {
        let ellipsoid =
            crate::PreparedEllipsoid::new(Rat::one(), decimal("298.257223563")).unwrap();
        let reference = crate::GeographicReference::new(
            ellipsoid.clone(),
            Digest32::new([85; 32]),
            AxisOrder::LonLat,
        );
        let minimum = ellipsoid.normal_metric_bounds_ref().0.exact();
        let third = minimum.mul(&Rat::from_i64(3));
        let epsilon = decimal("0.000000000000000001");
        let guard = decimal("0.05");
        let cases = [
            (minimum.sub(&epsilon), CircleDomain::StronglyConvex),
            (minimum.clone(), CircleDomain::BeforeCutLocus),
            (minimum.add(&epsilon), CircleDomain::BeforeCutLocus),
            (third.sub(&epsilon), CircleDomain::BeforeCutLocus),
            (third.clone(), CircleDomain::Global),
            (third.add(&epsilon), CircleDomain::Global),
        ];
        for (rho, expected) in cases {
            let mut context =
                MetricContext::new(reference.clone(), crate::ExecutionPolicy::geometry()).unwrap();
            context.begin(1).unwrap();
            let radius = rho.sub(&guard);
            let mut progress = WorkProgress::new(None);
            assert_eq!(
                circle_domain(&radius, &ellipsoid, &mut context, &mut progress).unwrap(),
                expected,
            );
            if expected == CircleDomain::Global {
                let center = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
                assert_eq!(
                    circle_count(&center, &radius, &mut context, &mut progress).unwrap(),
                    CircleCount::Global,
                );
            }
        }
    }

    #[test]
    fn pole_containing_circle_uses_complete_winding_and_original_physical_band() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        for hemisphere in [-1, 1] {
            let latitude = decimal("89.999999").mul(&Rat::from_i64(hemisphere));
            let center = LonLat::new(Rat::from_i64(19), latitude.clone()).unwrap();
            let literal = GeometryLiteral::new(
                crs.clone(),
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::Point(Some(Coord::xy(Rat::from_i64(19), latitude))),
                )
                .unwrap(),
            );
            let source =
                Arc::new(crate::PreparedGeometry::from_literal(&literal, &profile).unwrap());
            let offset = OffsetRegion::new(
                source,
                Metres::new(decimal("0.2")),
                crate::ExecutionPolicy::geometry(),
            )
            .unwrap();
            let mut limits = crate::ExecutionLimits::GEOMETRY;
            limits.max_work_items = 2_000_000;
            let policy = crate::ExecutionPolicy::new(limits).unwrap();
            let mut context =
                MetricContext::new(crate::GeographicReference::wgs84(), policy).unwrap();
            context.begin(1).unwrap();
            let mut progress = WorkProgress::new(None);
            assert!(
                matches!(circle_count(&center, &decimal("0.2"), &mut context, &mut progress).unwrap(),
                CircleCount::Atlas { pole: Some(pole), .. } if i64::from(pole) == hemisphere)
            );
            let output = offset.materialize(&profile, &crs, &mut context).unwrap();
            assert_eq!(
                output.law_id(),
                BufferMaterialization::global_output_law_id()
            );
            let carrier =
                crate::PreparedGeometry::from_literal(output.literal(), &profile).unwrap();
            let geodesic = PreparedGeodesic::new(crate::GeographicReference::wgs84());
            let pole = LonLat::new(Rat::zero(), Rat::from_i64(90 * hemisphere)).unwrap();
            assert_ne!(
                crate::atlas::locate(&pole, carrier.region(), &mut context).unwrap(),
                crate::Set::Exterior
            );
            for azimuth in (0..360).step_by(30) {
                for (radius, inside) in [("0.2", true), ("0.301", false)] {
                    let point = geodesic
                        .direct(
                            &center,
                            &Rat::from_i64(azimuth),
                            &Metres::new(decimal(radius)),
                            &mut context,
                        )
                        .unwrap()
                        .endpoint()
                        .clone();
                    let location =
                        crate::atlas::locate(&point, carrier.region(), &mut context).unwrap();
                    if inside {
                        assert_ne!(location, crate::Set::Exterior);
                    } else {
                        assert_eq!(location, crate::Set::Exterior);
                    }
                }
            }
        }
    }

    #[test]
    fn exact_zero_buffer_keeps_original_heap_and_carrier_storage_admitted() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let longitude = Rat::from_decimal(Int::one(), 240);
        let literal = GeometryLiteral::new(
            crs.clone(),
            Geometry::new(
                CoordDim::Xy,
                GeometryBody::Point(Some(Coord::xy(longitude, Rat::zero()))),
            )
            .unwrap(),
        );
        let source = Arc::new(crate::PreparedGeometry::from_literal(&literal, &profile).unwrap());
        let offset = OffsetRegion::new(
            source,
            Metres::new(Rat::zero()),
            crate::ExecutionPolicy::geometry(),
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let output = offset.materialize(&profile, &crs, &mut context).unwrap();
        let bytes = output.retained_workspace_bytes();
        assert!(bytes > 2048 + size_of::<BufferMaterialization>() as u64);
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            context.policy().limits().max_workspace_bytes
        );
        let registered = context.retained_workspace_bytes();
        context.begin(1).unwrap();
        assert_eq!(context.retained_workspace_bytes(), registered);
        let remaining = context.remaining_workspace();
        context.admit_workspace(remaining).unwrap();
        assert!(matches!(
            crate::wkt::write_exact_in_context(output.literal(), &mut context),
            Err(GeoError::MemoryExhausted { .. })
        ));
        assert_eq!(context.remaining_workspace(), 0);
        context.release_workspace(remaining).unwrap();
        let text = crate::wkt::write_exact_in_context(output.literal(), &mut context).unwrap();
        let restored = crate::wkt::parse(&text, &crs).unwrap();
        assert_eq!(
            restored.geometry().coords().next(),
            literal.geometry().coords().next()
        );
        let text_bytes = text.capacity() as u64;
        drop(text);
        context.release_workspace(text_bytes).unwrap();
        let shared = output.clone();
        assert!(core::ptr::eq(output.literal(), shared.literal()));
        let receipt = output.output_receipt();
        drop(output);
        assert!(matches!(
            context.release_materialized_output(receipt.clone()),
            Err(GeoError::Config(_))
        ));
        drop(shared);
        context.release_materialized_output(receipt).unwrap();
        assert_eq!(
            context.remaining_workspace() + context.retained_workspace_bytes(),
            context.policy().limits().max_workspace_bytes
        );
    }

    #[test]
    fn selected_native_curve_and_point_buffers_preserve_only_the_closed_support() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let reference = crate::GeographicReference::wgs84();
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items = 8_000_000;
        let policy = crate::ExecutionPolicy::new(limits).unwrap();
        let tiny = decimal("0.00000001");
        let mut context = MetricContext::new(reference.clone(), policy).unwrap();
        for isolated in [false, true] {
            let rings = [
                (0, 1, Rat::zero(), tiny.clone()),
                (
                    1,
                    2,
                    if isolated { tiny.clone() } else { Rat::zero() },
                    if isolated {
                        tiny.add(&tiny)
                    } else {
                        tiny.clone()
                    },
                ),
            ]
            .map(|(west, east, south, north)| {
                let coordinates = [
                    (Rat::from_i64(west), south.clone()),
                    (Rat::from_i64(east), south.clone()),
                    (Rat::from_i64(east), north.clone()),
                    (Rat::from_i64(west), north),
                    (Rat::from_i64(west), south),
                ]
                .map(|(x, y)| Coord::xy(x, y));
                crate::PreparedCurve::from_source(&coordinates, &reference).unwrap()
            });
            let polygon = crate::PreparedPolygon::from_curves(
                Vec::from(rings),
                crate::OrientedInterior::Left,
                &mut context,
            )
            .unwrap();
            let source = Arc::new(
                crate::PreparedGeometry::from_parts(
                    reference.clone(),
                    Vec::new(),
                    Vec::new(),
                    PreparedRegion::polygons(vec![polygon]),
                    policy,
                )
                .unwrap(),
            );
            let on_source = LonLat::new(
                Rat::one(),
                if isolated {
                    tiny.clone()
                } else {
                    tiny.div(&Rat::from_i64(2)).unwrap()
                },
            )
            .unwrap();
            let old_unselected_wall =
                LonLat::new(Rat::zero(), tiny.div(&Rat::from_i64(2)).unwrap()).unwrap();
            let zero = OffsetRegion::new(source.clone(), Metres::new(Rat::zero()), policy).unwrap();
            assert!(zero.contains(&on_source, &mut context).unwrap());
            assert!(!zero.contains(&old_unselected_wall, &mut context).unwrap());
            let output = zero.materialize(&profile, &crs, &mut context).unwrap();
            let output_source =
                crate::PreparedGeometry::from_literal(output.literal(), &profile).unwrap();
            let expected = crate::wkt::parse(
                if isolated {
                    "POINT(1 0.00000001)"
                } else {
                    "LINESTRING(1 0,1 0.00000001)"
                },
                &crs,
            )
            .unwrap();
            let expected = crate::PreparedGeometry::from_literal(&expected, &profile).unwrap();
            assert!(crate::SpatialRelation::SfEquals.holds(
                &crate::atlas::relate(&output_source, &expected, &mut context).unwrap(),
                0,
                0
            ));
            let receipt = output.output_receipt();
            drop(output);
            context.release_materialized_output(receipt).unwrap();
            let offset = OffsetRegion::new(source, Metres::new(decimal("0.1")), policy).unwrap();
            assert!(offset.contains(&on_source, &mut context).unwrap());
            assert!(!offset.contains(&old_unselected_wall, &mut context).unwrap());
            let output = offset.materialize(&profile, &crs, &mut context).unwrap();
            let carrier =
                crate::PreparedGeometry::from_literal(output.literal(), &profile).unwrap();
            assert_ne!(
                crate::atlas::locate(&on_source, carrier.region(), &mut context).unwrap(),
                crate::Set::Exterior
            );
            assert_eq!(
                crate::atlas::locate(&old_unselected_wall, carrier.region(), &mut context).unwrap(),
                crate::Set::Exterior
            );
            let geodesic = PreparedGeodesic::new(reference.clone());
            // Outer witnesses refer to an actual selected source point, never
            // to a discarded input wall.
            for coordinate in output.literal().geometry().coords() {
                let point = LonLat::new(coordinate.x().clone(), coordinate.y().clone()).unwrap();
                assert!(
                    geodesic
                        .within_physical(
                            &on_source,
                            &point,
                            &Metres::new(decimal("0.2")),
                            &mut context
                        )
                        .unwrap()
                );
            }
            for azimuth in [0, 45, 90, 135, 180, 225, 270, 315] {
                let witness = geodesic
                    .direct(
                        &on_source,
                        &Rat::from_i64(azimuth),
                        &Metres::new(decimal("0.1")),
                        &mut context,
                    )
                    .unwrap();
                assert_ne!(
                    crate::atlas::locate(witness.endpoint(), carrier.region(), &mut context)
                        .unwrap(),
                    crate::Set::Exterior
                );
            }
            assert_eq!(
                output.law_id(),
                BufferMaterialization::region_output_law_id()
            );
            let receipt = output.output_receipt();
            drop(output);
            context.release_materialized_output(receipt).unwrap();
            assert_eq!(
                context.current_workspace_bytes(),
                context.retained_workspace_bytes()
            );
        }
    }

    #[test]
    fn complete_curve_buffer_proves_containment_under_default_admission() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse("LINESTRING(0 0,0.00000001 0)", &crs).unwrap();
        let source = Arc::new(crate::PreparedGeometry::from_literal(&literal, &profile).unwrap());
        let offset = OffsetRegion::new(
            source,
            Metres::new(decimal("0.1")),
            crate::ExecutionPolicy::geometry(),
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let output = offset
            .materialize(&profile, &crs, &mut context)
            .expect("complete original millimetre curve at unchanged default limits");
        assert_eq!(
            output.law_id(),
            BufferMaterialization::region_output_law_id()
        );
        let covered = crate::PreparedGeometry::from_literal(output.literal(), &profile).unwrap();
        let prepared = PreparedGeodesic::new(crate::GeographicReference::wgs84());
        for longitude in [
            "0",
            "0.0000000025",
            "0.000000005",
            "0.0000000075",
            "0.00000001",
        ] {
            let source_point = LonLat::new(decimal(longitude), Rat::zero()).unwrap();
            for azimuth in [0, 45, 90, 135, 180, 225, 270, 315] {
                let point = prepared
                    .direct(
                        &source_point,
                        &Rat::from_i64(azimuth),
                        &Metres::new(decimal("0.1")),
                        &mut context,
                    )
                    .unwrap();
                assert_ne!(
                    crate::atlas::locate(point.endpoint(), covered.region(), &mut context).unwrap(),
                    crate::Set::Exterior
                );
            }
        }
        let middle = LonLat::new(decimal("0.000000005"), Rat::zero()).unwrap();
        for point in output.literal().geometry().coords() {
            let point = LonLat::new(point.x().clone(), point.y().clone()).unwrap();
            assert!(
                prepared
                    .distance(&middle, &point, &mut context)
                    .unwrap()
                    .value()
                    .exact()
                    < &decimal("0.2")
            );
        }
        assert!(matches!(
            offset.materialize_points(&profile, &crs, &mut context),
            Err(GeoError::Domain(_))
        ));
    }

    #[test]
    fn dateline_disks_are_cut_without_a_long_carrier_chord() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let center = LonLat::new(Rat::from_i64(180), Rat::zero()).unwrap();
        let literal = crate::wkt::parse("POINT(180 0)", &crs).unwrap();
        let source = Arc::new(crate::PreparedGeometry::from_literal(&literal, &profile).unwrap());
        let offset = OffsetRegion::new(
            source,
            Metres::new(decimal("0.1")),
            crate::ExecutionPolicy::geometry(),
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let output = offset
            .materialize_points(&profile, &crs, &mut context)
            .unwrap();
        let region = crate::PreparedGeometry::from_literal(output.literal(), &profile).unwrap();
        for azimuth in [0, 45, 90, 135, 180, 225, 270, 315] {
            let point = PreparedGeodesic::new(context.reference().clone())
                .direct(
                    &center,
                    &Rat::from_i64(azimuth),
                    &Metres::new(decimal("0.1")),
                    &mut context,
                )
                .unwrap()
                .endpoint()
                .clone();
            assert_ne!(
                crate::atlas::locate(&point, region.region(), &mut context).unwrap(),
                crate::Set::Exterior
            );
        }
        assert_eq!(
            crate::atlas::locate(
                &LonLat::new(Rat::zero(), Rat::zero()).unwrap(),
                region.region(),
                &mut context
            )
            .unwrap(),
            crate::Set::Exterior
        );
        for point in output.literal().geometry().coords() {
            assert!(point.x().abs() > Rat::from_i64(179));
        }
        let reparsed = crate::wkt::parse(&crate::wkt::write(output.literal(), 15), &crs).unwrap();
        let reparsed = crate::PreparedGeometry::from_literal(&reparsed, &profile).unwrap();
        assert_ne!(
            crate::atlas::locate(&center, reparsed.region(), &mut context).unwrap(),
            crate::Set::Exterior
        );
    }

    #[test]
    fn zero_curve_buffer_preserves_the_exact_written_source_closure() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse("LINESTRING(170 3,-170 4)", &crs).unwrap();
        let source = Arc::new(crate::PreparedGeometry::from_literal(&literal, &profile).unwrap());
        let offset = OffsetRegion::new(
            source,
            Metres::new(Rat::zero()),
            crate::ExecutionPolicy::geometry(),
        )
        .unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let output = offset.materialize(&profile, &crs, &mut context).unwrap();
        assert_eq!(
            output.literal().geometry().coords().collect::<Vec<_>>(),
            literal.geometry().coords().collect::<Vec<_>>()
        );
        let GeometryBody::GeometryCollection(members) = output.literal().geometry().body() else {
            panic!("complete closed source carrier");
        };
        assert_eq!(members.len(), 1);
        assert_eq!(members[0], *literal.geometry());
    }

    #[test]
    fn complete_buffer_negative_whole_and_output_refusals_are_explicit() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let literal = crate::wkt::parse("LINESTRING(0 0,1 1)", &crs).unwrap();
        let source = Arc::new(crate::PreparedGeometry::from_literal(&literal, &profile).unwrap());
        let mut context = MetricContext::wgs84().unwrap();
        for radius in ["-1", "21000000"] {
            let offset = OffsetRegion::new(
                source.clone(),
                Metres::new(decimal(radius)),
                crate::ExecutionPolicy::geometry(),
            )
            .unwrap();
            let output = offset.materialize(&profile, &crs, &mut context).unwrap();
            if radius == "-1" {
                assert!(
                    matches!(output.literal().geometry().body(), GeometryBody::GeometryCollection(v) if v.is_empty())
                );
            } else {
                let surface =
                    crate::PreparedGeometry::from_literal(output.literal(), &profile).unwrap();
                for (longitude, latitude) in [
                    (-180, 0),
                    (180, 0),
                    (0, -90),
                    (0, 90),
                    (179, 89),
                    (-70, -32),
                ] {
                    let point =
                        LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude)).unwrap();
                    assert_ne!(
                        crate::atlas::locate(&point, surface.region(), &mut context).unwrap(),
                        crate::Set::Exterior
                    );
                }
            }
        }
        let offset = OffsetRegion::new(
            source,
            Metres::new(decimal("1")),
            crate::ExecutionPolicy::geometry(),
        )
        .unwrap();
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_output_elements = 8;
        let mut limited = MetricContext::new(
            crate::GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            offset.materialize(&profile, &crs, &mut limited),
            Err(GeoError::OutputExhausted { limit: 8 })
        ));
        assert_eq!(
            limited
                .remaining_workspace()
                .checked_add(limited.retained_workspace_bytes()),
            Some(limits.max_workspace_bytes)
        );
        let first_refusal_storage = limited.retained_workspace_bytes();
        assert!(matches!(
            offset.materialize(&profile, &crs, &mut limited),
            Err(GeoError::OutputExhausted { limit: 8 })
        ));
        assert_eq!(limited.retained_workspace_bytes(), first_refusal_storage);
    }

    #[test]
    fn circle_turn_bound_is_above_independent_machin_enclosure() {
        for precision_bits in [192, 256] {
            let mut math = CoordinateMath::new(MathLimits {
                precision_bits,
                max_work: 262_144,
                max_workspace_bytes: 64 * 1024 * 1024,
            })
            .unwrap();
            let upper = fixed_from_rat(
                &decimal("6.28318530717958647692528676655900576839433879875022"),
                &mut math,
            )
            .unwrap();
            let turn = FixedInterval::pi(&mut math)
                .unwrap()
                .mul(&FixedInterval::from_i64(2, &mut math).unwrap(), &mut math)
                .unwrap();
            assert!(turn.upper() < upper.lower());
        }
    }

    #[test]
    fn global_disks_from_nonpolar_and_multiple_centres_cover_the_complete_surface() {
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        for text in ["POINT(45 12)", "MULTIPOINT((179.9 89),(-179.9 -89),(73 0))"] {
            let literal = crate::wkt::parse(text, &crs).unwrap();
            let prepared =
                Arc::new(crate::PreparedGeometry::from_literal(&literal, &profile).unwrap());
            let offset = OffsetRegion::new(
                prepared,
                Metres::new(decimal("21000000")),
                crate::ExecutionPolicy::geometry(),
            )
            .unwrap();
            let mut context = MetricContext::wgs84().unwrap();
            let image = offset
                .materialize_points(&profile, &crs, &mut context)
                .unwrap();
            let result = crate::PreparedGeometry::from_literal(image.literal(), &profile).unwrap();
            for (longitude, latitude) in [(-180, -90), (180, 90), (0, 0), (-95, 40), (73, -18)] {
                let point = LonLat::new(Rat::from_i64(longitude), Rat::from_i64(latitude)).unwrap();
                assert_eq!(
                    crate::atlas::locate(&point, result.region(), &mut context).unwrap(),
                    crate::Set::Interior
                );
            }
            assert!(
                crate::ellipsoidal::perimeter(&result, &mut context)
                    .unwrap()
                    .exact()
                    .is_zero()
            );
            let mut limits = crate::ExecutionLimits::GEOMETRY;
            limits.max_output_elements = 4;
            let mut limited = MetricContext::new(
                crate::GeographicReference::wgs84(),
                crate::ExecutionPolicy::new(limits).unwrap(),
            )
            .unwrap();
            assert!(matches!(
                offset.materialize_points(&profile, &crs, &mut limited),
                Err(GeoError::OutputExhausted { limit: 4 })
            ));
        }
    }

    #[test]
    fn polar_materialization_is_a_complete_cap_and_whole_surface() {
        let profile = GeoProfile::default();
        let policy = crate::ExecutionPolicy::geometry();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        for (latitude, radius) in [(90, "1000"), (-90, "1000"), (90, "21000000")] {
            let literal = GeometryLiteral::new(
                crs.clone(),
                Geometry::new(
                    CoordDim::Xy,
                    GeometryBody::Point(Some(Coord::xy(
                        Rat::from_i64(73),
                        Rat::from_i64(latitude),
                    ))),
                )
                .unwrap(),
            );
            let prepared =
                Arc::new(crate::PreparedGeometry::from_literal(&literal, &profile).unwrap());
            let offset = OffsetRegion::new(prepared, Metres::new(decimal(radius)), policy).unwrap();
            let mut context = MetricContext::wgs84().unwrap();
            let image = offset
                .materialize_points(&profile, &crs, &mut context)
                .unwrap();
            let prepared =
                crate::PreparedGeometry::from_literal(image.literal(), &profile).unwrap();
            let membership = |point: &LonLat, context: &mut MetricContext| {
                crate::atlas::locate(point, prepared.region(), context).unwrap()
                    != crate::Set::Exterior
            };
            assert!(membership(
                &LonLat::new(Rat::from_i64(-95), Rat::from_i64(latitude)).unwrap(),
                &mut context
            ));
            if radius == "1000" {
                assert!(!membership(
                    &LonLat::new(Rat::zero(), Rat::zero()).unwrap(),
                    &mut context
                ));
            } else {
                assert!(membership(
                    &LonLat::new(Rat::from_i64(170), Rat::from_i64(-90)).unwrap(),
                    &mut context
                ));
            }
        }
    }

    #[test]
    fn symbolic_point_buffer_encloses_the_original_image_without_replacing_it() {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationChain, OperationImagePoint,
            OperationModel, OperationReference,
        };
        let reference = crate::GeographicReference::wgs84();
        let chain = Arc::new(
            OperationChain::compile(vec![
                CoordinateOperation::compile(
                    OperationReference {
                        realization: Digest32::new([19; 32]),
                        unit: CoordinateUnit::Metres,
                        swapped_axes: false,
                    },
                    OperationReference {
                        realization: reference.id().digest(),
                        unit: CoordinateUnit::Degrees,
                        swapped_axes: false,
                    },
                    OperationModel::MercatorToGeographic {
                        radius: Rat::from_i64(6_378_137),
                        eccentricity_squared: Rat::zero(),
                        square_domain: true,
                    },
                )
                .unwrap(),
            ])
            .unwrap(),
        );
        let policy = crate::ExecutionPolicy::geometry();
        let symbolic = OperationImagePoint::new(
            Coord::xy(Rat::one(), Rat::zero()),
            chain,
            reference.clone(),
            None,
            policy,
        )
        .unwrap();
        let symbolic_id = symbolic.id();
        let source = Arc::new(
            crate::PreparedGeometry::from_parts_with_symbolic(
                reference,
                Vec::new(),
                vec![symbolic],
                Vec::new(),
                PreparedRegion::Empty,
                policy,
            )
            .unwrap(),
        );
        let profile = GeoProfile::standard();
        let crs = Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let mut context = MetricContext::wgs84().unwrap();
        let offset = OffsetRegion::new(source.clone(), Metres::new(Rat::one()), policy).unwrap();
        let output = offset
            .materialize_points(&profile, &crs, &mut context)
            .unwrap();
        assert!(context.work_items() <= policy.limits().max_work_items);
        let rounded = source.symbolic_points()[0]
            .materialize(&mut context)
            .unwrap()
            .into_point();
        let rounded_source = Arc::new(
            crate::PreparedGeometry::from_parts(
                source.reference().clone(),
                vec![
                    crate::PreparedCoordinate::new(
                        Coord::xy(rounded.x, rounded.y),
                        source.reference(),
                    )
                    .unwrap(),
                ],
                Vec::new(),
                PreparedRegion::Empty,
                policy,
            )
            .unwrap(),
        );
        let mut rounded_context = MetricContext::wgs84().unwrap();
        let rounded_output = OffsetRegion::new(rounded_source, Metres::new(Rat::one()), policy)
            .unwrap()
            .materialize_points(&profile, &crs, &mut rounded_context)
            .unwrap();
        assert!(rounded_context.work_items() <= policy.limits().max_work_items);
        assert_eq!(output.literal(), rounded_output.literal());
        assert_eq!(output.law_id(), rounded_output.law_id());
        let prepared = crate::PreparedGeometry::from_literal(output.literal(), &profile).unwrap();
        let origin = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        assert_ne!(
            crate::atlas::locate(&origin, prepared.region(), &mut context).unwrap(),
            crate::Set::Exterior
        );
        assert_eq!(source.symbolic_points()[0].id(), symbolic_id);
        assert_eq!(source.points(), []);
        let zero = OffsetRegion::new(source.clone(), Metres::new(Rat::zero()), policy).unwrap();
        assert!(matches!(
            zero.materialize_points(&profile, &crs, &mut context),
            Err(GeoError::PrecisionExhausted { .. })
        ));
        let negative = OffsetRegion::new(source, Metres::new(Rat::from_i64(-1)), policy).unwrap();
        assert!(
            matches!(negative.materialize_points(&profile, &crs, &mut context).unwrap().literal().geometry().body(), GeometryBody::GeometryCollection(values) if values.is_empty())
        );
    }
}
