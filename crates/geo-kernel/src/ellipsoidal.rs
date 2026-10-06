// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified metrics for the source-linear geographic carrier.
//!
//! Longitude interpolates exactly as written. An edge from 170 to −170 traverses
//! 340 degrees; it is never replaced by a shortest geodesic. Taylor quadrature
//! uses derivative enclosures over complete subintervals and Taylor's theorem.

mod distance;
mod jet;
pub use distance::{distance, distance_metered, within_physical, within_physical_metered};

use purrdf_hash::Domain;
use purrdf_xsd::{
    BigInt,
    math::{CoordinateMath, FixedInterval, MathError, TaylorJet, TaylorWorkspace},
};

use crate::context::WorkProgress;
use crate::numerical::{fixed_from_rat, geo_math_error};
use crate::profile::hash_fields;
use crate::{
    GeoBindingId, GeoError, LonLat, Metres, MetricContext, MetricWorkObserver, PreparedCurve,
    PreparedEdge, PreparedGeometry, PreparedRegion, Rat, SemanticLawId, SquareMetres,
};

pub(crate) mod native;

const GEOMETRY_METRIC_LAW: Domain = Domain::new(b"purrdf-geo-kernel/geometry-metric-law/v1");
const GEOMETRY_METRIC_CERTIFICATE: Domain =
    Domain::new(b"purrdf-geo-kernel/geometry-metric-certificate/v1");

/// Dimension and frozen deterministic approximation law of a geographic metric.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GeometryMetricLaw {
    /// Globally certified minimum distance; total response error at most0.25mm.
    Distance,
    /// Length; total response error at most max(1mm,1e-14*value).
    Length,
    /// Areal boundary perimeter, otherwise curve length, with the length bound.
    Perimeter,
    /// Oriented atlas surface area; total error at most max(.1m²,1e-14*abs(value)).
    Area,
    /// Signed source-edge gauge contribution `-integral Q(phi)dlongitude`.
    AreaIntegral,
    /// Signed unrounded selected-geodesic quadrilateral gauge contribution.
    GeodesicAreaIntegral,
}

impl GeometryMetricLaw {
    /// Completed-output identity, excluding invocation proof tightness and limits.
    #[must_use]
    pub fn id(self) -> SemanticLawId {
        let descriptor: &[u8] = match self {
            Self::Distance => b"ellipsoid-general-distance;exact-atlas-intersections-and-certified-normal-curve-contacts;complete-closed-selected-areal-curve-and-isolated-point-support;original-selected-cut-families-retained;global-dyadic-lipschitz-boxes;source-linear-path-bound=ceil-micrometre-original-metric-variation-pi-upper;analytic-parallel-meridian-minima;certified-equatorial-meridian-reflection;sample=point-micrometre-v1;selected-estimate-parameter=RN24-original-contact-endpoints-then-dyadic-affine-sample;selected-full-cut-tail-speed-bound<=1um-per-point;physical-selected-cut-families-refine-with-proof-until-decisive;transformed-sample=unrounded-original-image-centre-with-radius<=1um;uncertainty=.000003m;stop-gap=.000125m;answer=minimum-sampled-quantum;half-even-metre6;total-bound=.00025m;certificate=v1",
            Self::Length => b"ellipsoid-carrier-length;greatest-actual-stratum-total;closed-selected-curve-strata-add-to-original-curves;physical-union-boundary;exact-noded-atlas-cuts-collinear-coalescing;native-selected-original-contact-fragments;native-parameter-RN24-stable-under-refined-proofs;endpoint-tail-speed-bound<=budget/16;core-budget15/16;arcs=exact-azimuth-length-or-certified-shortest;source-linear-Taylor8;partition=outward-fixed96-derivative8;transformed=complete-original-chain-speed-fixed96;exact-matched-native-TM-roundtrip=admitted-nonpolar-original-source-Taylor8;monotone-meridian=unrounded-endpoint-meridian-integrals;transformed-dyadic-panel-width<=budget*parameter-width;transformed-value=exact-dyadic-midpoint;dyadic;remainder-budget=.00025m;half-even-metre6;total-bound=max(.001m,1e-14*value);certificate=v1",
            Self::Perimeter => b"ellipsoid-carrier-perimeter;actual-areal-faces-select-physical-union-boundary-otherwise-all-selected-curve-strata-and-original-curves;exact-noded-atlas-cuts-collinear-coalescing;native-selected-original-contact-fragments;native-parameter-RN24-stable-under-refined-proofs;endpoint-tail-speed-bound<=budget/16;core-budget15/16;arcs=exact-azimuth-length-or-certified-shortest;source-linear-Taylor8;partition=outward-fixed96-derivative8;transformed=complete-original-chain-speed-fixed96;exact-matched-native-TM-roundtrip=admitted-nonpolar-original-source-Taylor8;monotone-meridian=unrounded-endpoint-meridian-integrals;transformed-dyadic-panel-width<=budget*parameter-width;transformed-value=exact-dyadic-midpoint;dyadic;remainder-budget=.00025m;half-even-metre6;total-bound=max(.001m,1e-14*value);certificate=v1",
            Self::Area => b"ellipsoid-carrier-area;nonnegative-union-area;oriented-longitude-and-normal-atlas;holes-complements-whole;exact-union-vertical-atlas-arrangement;written-source-linear=original-affine-atlas-strips-labelled-by-complete-physical-region-and-Taylor8-approximation-with-separate-.025m2-budget;native=correct-half-even-true-physical-area-RN2;two-gauges-AN=(Q1-Qz)dlambda-AS=-(Q1+Qz)dlambda;generated-J-series-tail-refines-with-precision;complete-original-fragment-parameter-enclosures-RN24-with-both-gauge-endpoint-tails;strict-north-south-panels;all-unresolved-equator-strips-retain-both-gauge-flux-and-whole-continuous-longitude-image;actual-equatorial-runs-directed-selected-left-gauge-and-inward-boundary-intervals;whole-panel-proved-monotone-lift-uses-enclosed-endpoints-otherwise-full-image;cut-angle-walls=directed-outward-contact-inward-boundary;equator-cut=2Q1*complete-eastward-Interior-angular-measure;unknown-measure-enclosed-not-guessed;Taylor8-or-complete-transformed-image-box-with-full-remainder;all-truncation-contact-cut-and-arithmetic-errors-refine-until-left-and-complement-RN2-decisive;96-output-enclosure-rechecked;unresolved-ties-refuse;half-even-metre2;total-bound=max(.1m2,1e-14*abs(value));certificate=v1",
            Self::AreaIntegral => b"ellipsoid-source-area-integral;gauge=-Q(phi)dlongitude;written-longitude;source-linear-Taylor8;partition=outward-fixed96-derivative8;dyadic;global-budget=.025m2;half-even-metre2;total-bound=max(.1m2,1e-14*abs(value));certificate=v1",
            Self::GeodesicAreaIntegral => b"ellipsoid-selected-geodesic-area-integral;gauge=-Q(phi)dlongitude;original-auxiliary-line-S12-with-internal-meridian-transitions;endpoint-pole-atlas-cuts-separate;original-area-equation-Cauchy-series-fixed96;half-even-metre2;total-bound=max(.1m2,1e-14*abs(value));certificate=v1",
        };
        if matches!(self, Self::Distance | Self::Length | Self::Perimeter) {
            // These algorithms consume every actual selected physical stratum.
            // Bind the one topology law, including v3's complete complement
            // sectors, rather than retaining obsolete internal walls/nodes.
            let selected = crate::atlas::topology_law_id().digest();
            SemanticLawId::from_digest(hash_fields(
                GEOMETRY_METRIC_LAW,
                [descriptor, selected.as_bytes()],
            ))
        } else {
            SemanticLawId::from_digest(hash_fields(GEOMETRY_METRIC_LAW, [descriptor]))
        }
    }
    const fn places(self) -> u32 {
        if self.is_area() { 2 } else { 6 }
    }
    const fn is_area(self) -> bool {
        matches!(
            self,
            Self::Area | Self::AreaIntegral | Self::GeodesicAreaIntegral
        )
    }
}

/// Exact quantized general metric with a fixed response certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryMetricEstimate {
    value: Rat,
    reported: u64,
    law: GeometryMetricLaw,
    binding: GeoBindingId,
}

impl GeometryMetricEstimate {
    /// Exact SI value; metres except for area in square metres.
    #[must_use]
    pub const fn exact(&self) -> &Rat {
        &self.value
    }
    /// Host binary64 conversion of the exact completed value.
    #[must_use]
    pub const fn reported_double(&self) -> f64 {
        f64::from_bits(self.reported)
    }
    /// Dimensioned distance/length, absent for area.
    #[must_use]
    pub fn metres(&self) -> Option<Metres> {
        (!self.law.is_area()).then(|| Metres::new(self.value.clone()))
    }
    /// Whether the completed quantity is a square-metre result.
    #[must_use]
    pub const fn is_area(&self) -> bool {
        self.law.is_area()
    }
    /// Dimensioned surface area, absent for other metrics.
    #[must_use]
    pub fn square_metres(&self) -> Option<SquareMetres> {
        self.law
            .is_area()
            .then(|| SquareMetres::new(self.value.clone()))
    }
    /// Fixed total numerical, quantization and conversion bound in this SI dimension.
    #[must_use]
    pub fn error_bound(&self) -> Rat {
        metric_error_bound(self.law, &self.value, None, &mut 0)
            .expect("unbounded exact certificate arithmetic")
    }
    /// Storage of one retained result, including its inline fields and complete
    /// detached shared limb allocations. A caller retaining results in a batch
    /// or another prepared owner admits this storage separately; aliases share
    /// each immutable allocation and count its heap once.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> u64 {
        (size_of::<Self>() + self.value.shared_owned_bytes()) as u64
    }
    /// The mathematical law of this completed approximation.
    #[must_use]
    pub fn law_id(&self) -> SemanticLawId {
        self.law.id()
    }
    /// Exact declared reference binding.
    #[must_use]
    pub const fn binding_id(&self) -> GeoBindingId {
        self.binding
    }
    /// Canonical certificate with fixed bound rather than a tighter invocation proof.
    #[must_use]
    pub fn certificate_bytes(&self) -> Vec<u8> {
        let law = self.law_id().digest();
        let binding = self.binding.digest();
        let value = self.value.to_decimal_string(self.law.places());
        let bound = self.error_bound();
        let numerator = bound.numerator().to_string();
        let denominator = bound.denominator().to_string();
        let reported = self.reported.to_be_bytes();
        crate::metric::framed_certificate([
            GEOMETRY_METRIC_CERTIFICATE.as_bytes(),
            law.as_bytes(),
            binding.as_bytes(),
            value.as_bytes(),
            numerator.as_bytes(),
            denominator.as_bytes(),
            reported.as_slice(),
        ])
    }
}

/// Physical length of one exact source-linear edge, including its written long path.
///
/// # Errors
/// Refuses unresolved completed rounding, environment/resource failure and cancellation.
pub fn source_linear_length(
    start: &LonLat,
    end: &LonLat,
    context: &mut MetricContext,
) -> Result<GeometryMetricEstimate, GeoError> {
    context.begin(1)?;
    linear_metric(start, end, GeometryMetricLaw::Length, context)
}

/// Signed contribution `-integral Q(phi) dlongitude` in the prepared longitude atlas.
///
/// # Errors
/// Uses the same strict numerical and operational contract as source-linear length.
pub fn source_linear_area_integral(
    start: &LonLat,
    end: &LonLat,
    context: &mut MetricContext,
) -> Result<GeometryMetricEstimate, GeoError> {
    context.begin(1)?;
    linear_metric(start, end, GeometryMetricLaw::AreaIntegral, context)
}

/// Signed contribution `-integral Q(phi) dlongitude` of the original selected
/// geodesic line. Internal meridian transitions are retained; a closed region
/// adds its explicit endpoint-pole atlas cuts and declared interior.
///
/// # Errors
/// Refuses a non-geodesic edge, reference mismatch, unresolved branch or
/// incomplete numerical admission. Quantized inverse areas are never inputs.
pub fn geodesic_area_integral(
    edge: &PreparedEdge,
    context: &mut MetricContext,
) -> Result<GeometryMetricEstimate, GeoError> {
    context.begin(1)?;
    let mut progress = WorkProgress::new(None);
    progress.context_poll(context)?;
    let mut child = context.remaining_child()?;
    let view = {
        let mut nested = progress.nested(
            context.work_items(),
            context
                .policy()
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace()),
            context.workspace_peak(),
        );
        edge.geodesic_view_metered(96, &mut child, &mut nested)
    };
    let view = progress
        .absorb_child_result(context, &child, view)?
        .ok_or_else(|| GeoError::domain("selected geodesic area requires a geodesic edge"))?;
    let retained = view.retained_workspace_bytes();
    context.admit_workspace(retained)?;
    let reference = context.reference().clone();
    let iterations = context.policy().limits().max_iterations;
    let result = complete_metric(
        GeometryMetricLaw::GeodesicAreaIntegral,
        context,
        &mut progress,
        |math, progress| {
            let line = view.prepare_in(math, progress)?;
            line.quadrilateral_area_in(
                &FixedInterval::from_i64(0, math)?,
                &FixedInterval::from_i64(1, math)?,
                &reference,
                iterations,
                math,
                progress,
            )?
            .neg(math)
        },
    );
    drop(view);
    context.release_workspace(retained)?;
    result
}

/// Greatest total length by topological dimension, including polygon boundaries.
///
/// # Errors
/// Refuses mixed references and any unresolved numerical or operational result.
pub fn length(
    geometry: &PreparedGeometry,
    context: &mut MetricContext,
) -> Result<GeometryMetricEstimate, GeoError> {
    prepared_length(geometry, GeometryMetricLaw::Length, context, None)
}

/// Areal ring perimeter when an areal component exists, otherwise curve length.
///
/// # Errors
/// Applies the same strict completion contract as geographic length.
pub fn perimeter(
    geometry: &PreparedGeometry,
    context: &mut MetricContext,
) -> Result<GeometryMetricEstimate, GeoError> {
    prepared_length(geometry, GeometryMetricLaw::Perimeter, context, None)
}

/// Geographic length with bounded external work/cancellation callbacks.
///
/// # Errors
/// Includes the observer's latched refusal in the ordinary metric contract.
pub fn length_metered(
    geometry: &PreparedGeometry,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<GeometryMetricEstimate, GeoError> {
    prepared_length(geometry, GeometryMetricLaw::Length, context, Some(observer))
}

/// Geographic perimeter with bounded external work/cancellation callbacks.
///
/// # Errors
/// Includes the observer's latched refusal in the ordinary metric contract.
pub fn perimeter_metered(
    geometry: &PreparedGeometry,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<GeometryMetricEstimate, GeoError> {
    prepared_length(
        geometry,
        GeometryMetricLaw::Perimeter,
        context,
        Some(observer),
    )
}

/// Surface area of the complete region union, including holes and complements.
///
/// Source-linear ring intersections are decomposed exactly before integration;
/// overlapping polygons are measured once and no smaller interior is selected.
///
/// # Errors
/// Refuses a missing reference operation and every incomplete numerical result.
pub fn area(
    geometry: &PreparedGeometry,
    context: &mut MetricContext,
) -> Result<GeometryMetricEstimate, GeoError> {
    area_observed(geometry, context, None)
}

/// Complete region area with bounded external work/cancellation callbacks.
///
/// # Errors
/// Refuses every observer, reference, arrangement or numerical failure.
pub fn area_metered(
    geometry: &PreparedGeometry,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<GeometryMetricEstimate, GeoError> {
    area_observed(geometry, context, Some(observer))
}

fn area_observed(
    geometry: &PreparedGeometry,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<GeometryMetricEstimate, GeoError> {
    if geometry.reference() != context.reference() {
        context.begin(0)?;
        return Err(crate::numerical::missing_operation(
            geometry.reference(),
            context,
            &mut WorkProgress::new(observer),
        )?);
    }
    region_area_inner(geometry.region(), context, observer)
}

/// Area of an explicitly selected empty, written, complementary or whole region.
///
/// # Errors
/// Refuses a missing reference operation or incomplete arrangement/integration.
pub fn region_area(
    region: &PreparedRegion,
    context: &mut MetricContext,
) -> Result<GeometryMetricEstimate, GeoError> {
    region_area_inner(region, context, None)
}

fn region_area_inner(
    region: &PreparedRegion,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<GeometryMetricEstimate, GeoError> {
    context.begin(0)?;
    let mut progress = WorkProgress::new(observer);
    progress.context_poll(context)?;
    let source_linear_native = if let PreparedRegion::Polygons(polygons)
    | PreparedRegion::ComplementOfPolygons(polygons) = region
        && polygons.iter().any(|polygon| polygon.chart().is_none())
    {
        let mut source_linear = true;
        for polygon in polygons.iter() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            source_linear &= !polygon.closed_support();
            for edge in polygon.rings().iter().flat_map(PreparedCurve::edges) {
                context.charge_work(1)?;
                progress.context_poll(context)?;
                source_linear &= matches!(edge, PreparedEdge::SourceLinear(_));
            }
        }
        source_linear
    } else {
        false
    };
    if let PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) =
        region
        && !source_linear_native
        && polygons.len() == 1
        && !polygons[0].closed_support()
        && let Some(side) = polygons[0].oriented_interior()
        && polygons[0].rings().len() == 1
    {
        let ring = crate::atlas::area::ring_area(
            &polygons[0].rings()[0],
            &metric_budget(GeometryMetricLaw::Area),
            context,
            &mut progress,
        )?;
        let complementary = (side == crate::OrientedInterior::Right)
            ^ (polygons[0].interior() == crate::RegionInterior::Complement)
            ^ matches!(region, PreparedRegion::ComplementOfPolygons(_));
        let storage = ring.storage;
        let result = complete_metric(
            GeometryMetricLaw::Area,
            context,
            &mut progress,
            |math, _| {
                if complementary {
                    ring.surface.sub(&ring.left, math)
                } else {
                    Ok(ring.left.clone())
                }
            },
        );
        drop(ring);
        context.release_workspace(storage)?;
        return result;
    }
    if !source_linear_native
        && matches!(region, PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) if polygons.iter().any(|polygon| polygon.chart().is_none()))
    {
        return selected_native_area(region, context, &mut progress);
    }
    if !source_linear_native
        && let Some(boundary) =
            crate::atlas::native::select_boundary(region, context, &mut progress)?
    {
        let ellipsoid = context.reference().ellipsoid().clone();
        let budget = metric_budget(GeometryMetricLaw::Area)
            .div(&Rat::from_int(crate::Int::from_u64(
                boundary.rings.len().max(1) as u64,
            )))
            .expect("positive native ring count");
        let mut retained = boundary.workspace_bytes;
        let result = (|| {
            let bytes = u64::try_from(boundary.rings.len())
                .ok()
                .and_then(|count| {
                    count
                        .checked_mul(size_of::<(crate::atlas::area::RingArea, bool, bool)>() as u64)
                })
                .ok_or(GeoError::ArithmeticOverflow(
                    "native area contribution storage",
                ))?;
            context.retain_workspace(bytes, &mut retained)?;
            progress.context_poll(context)?;
            let mut contributions = Vec::with_capacity(boundary.rings.len());
            for ring in &boundary.rings {
                let area =
                    crate::atlas::area::ring_area(&ring.curve, &budget, context, &mut progress)?;
                let Some(total) = retained.checked_add(area.storage) else {
                    let storage = area.storage;
                    drop(area);
                    context.release_workspace(storage)?;
                    return Err(GeoError::ArithmeticOverflow(
                        "native area contribution storage",
                    ));
                };
                retained = total;
                contributions.push((area, ring.reversed, ring.south_inside));
            }
            let result = complete_metric(
                GeometryMetricLaw::Area,
                context,
                &mut progress,
                |math, progress| {
                    let mut total = FixedInterval::from_i64(0, math)?;
                    let surface = crate::atlas::area::surface_area_in(&ellipsoid, math, progress)?;
                    for (ring, reversed, south_inside) in &contributions {
                        let integral = if *south_inside {
                            ring.left.sub(&ring.surface, math)?
                        } else {
                            ring.left.clone()
                        };
                        total = if *reversed {
                            total.sub(&integral, math)?
                        } else {
                            total.add(&integral, math)?
                        };
                    }
                    if boundary.south_inside {
                        total = total.add(&surface, math)?;
                    }
                    Ok(total)
                },
            );
            drop(contributions);
            result
        })();
        drop(boundary);
        context.release_workspace(retained)?;
        return result;
    }
    let arrangement = crate::atlas::arrangement::area_arrangement(region, context, &mut progress)?;
    let ellipsoid = context.reference().ellipsoid().clone();
    let max_depth = context.policy().limits().max_subdivision_levels;
    let budget = metric_budget(GeometryMetricLaw::Area)
        .div(&Rat::from_int(crate::Int::from_u64(
            arrangement.strips.len().saturating_mul(2).max(1) as u64,
        )))
        .expect("positive strip count");
    let result = (|| {
        let bits = crate::numerical::scratch_source_bits_in(
            arrangement
                .strips
                .iter()
                .flat_map(|strip| {
                    strip
                        .lower
                        .iter()
                        .chain(&strip.upper)
                        .flat_map(|point| [point.longitude(), point.latitude()])
                })
                .chain(core::iter::once(&budget)),
            &ellipsoid,
        );
        context.prepare_integer_scratch_for(bits)?;
        progress.context_poll(context)?;
        complete_metric(
            GeometryMetricLaw::Area,
            context,
            &mut progress,
            |math, progress| {
                let mut total = FixedInterval::from_i64(0, math)?;
                for strip in &arrangement.strips {
                    let bottom = integrate_linear(
                        (&strip.lower[0], &strip.lower[1]),
                        GeometryMetricLaw::Area,
                        &ellipsoid,
                        max_depth,
                        &budget,
                        math,
                        progress,
                    )?;
                    let top = integrate_linear(
                        (&strip.upper[1], &strip.upper[0]),
                        GeometryMetricLaw::Area,
                        &ellipsoid,
                        max_depth,
                        &budget,
                        math,
                        progress,
                    )?;
                    total = total.add(&bottom, math)?.add(&top, math)?;
                }
                Ok(total)
            },
        )
    })();
    context.release_workspace(arrangement.workspace_bytes)?;
    result
}

fn selected_native_area(
    region: &PreparedRegion,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<GeometryMetricEstimate, GeoError> {
    let boundary = crate::atlas::arcs::arrangement::native_boundary_in(region, context, progress)?;
    let mut retained = boundary.retained_workspace_bytes();
    context.admit_workspace(retained)?;
    let result = (|| {
        if !boundary.has_areal_faces() {
            return complete_metric(GeometryMetricLaw::Area, context, progress, |math, _| {
                FixedInterval::from_i64(0, math)
            });
        }
        let reference = crate::numerical::reference_clone(context, progress)?;
        let count = Rat::from_int(crate::Int::from_u64(
            boundary.fragments().len().max(1) as u64
        ));
        let total_budget = metric_budget(GeometryMetricLaw::Area);
        let budget = crate::numerical::ExactAdmission::new(context, progress).rational(
            purrdf_xsd::integer::ExactOperation::RationalDivide,
            &[&total_budget, &count],
            1,
            || Ok(total_budget.div(&count).expect("nonzero fragment count")),
        )?;
        let mut bits = crate::numerical::scratch_source_bits(&[&budget], reference.ellipsoid());
        for fragment in boundary.fragments() {
            bits = bits.max(edge_operand_bits(
                fragment.original_edge(),
                reference.ellipsoid(),
            ));
            for parameter in fragment.parameters() {
                bits = bits.max(crate::numerical::scratch_source_bits(
                    &<[_; 2]>::from(parameter.bounds()),
                    reference.ellipsoid(),
                ));
            }
        }
        context.prepare_integer_scratch_for_observed(bits, progress)?;
        let parameters = native::prepare(&boundary, context, progress, &mut retained)?;
        let area =
            crate::atlas::area::selected_area(&boundary, &parameters, &budget, context, progress)?;
        let storage = area.storage;
        let result = complete_metric(GeometryMetricLaw::Area, context, progress, |_, _| {
            Ok(area.left.clone())
        });
        drop(area);
        context.release_workspace(storage)?;
        result
    })();
    drop(boundary);
    context.release_workspace(retained)?;
    result
}

fn prepared_length(
    geometry: &PreparedGeometry,
    law: GeometryMetricLaw,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<GeometryMetricEstimate, GeoError> {
    if geometry.reference() != context.reference() {
        context.begin(0)?;
        return Err(crate::numerical::missing_operation(
            geometry.reference(),
            context,
            &mut WorkProgress::new(observer),
        )?);
    }
    let areal_count = match geometry.region() {
        PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) => {
            polygons
                .iter()
                .flat_map(crate::PreparedPolygon::rings)
                .map(|ring| ring.edges().len())
                .sum::<usize>()
        }
        PreparedRegion::Empty | PreparedRegion::Whole => 0,
    };
    let linear_count = geometry
        .curves()
        .iter()
        .map(|curve| curve.edges().len())
        .sum::<usize>();
    let count = areal_count
        .checked_add(linear_count)
        .ok_or(GeoError::ArithmeticOverflow("geographic edge count"))?;
    context.begin(count)?;
    let mut progress = WorkProgress::new(observer);
    progress.context_poll(context)?;
    let boundary =
        crate::atlas::boundary::region_boundary(geometry.region(), context, &mut progress)?;
    let ellipsoid = context.reference().ellipsoid().clone();
    let max_depth = context.policy().limits().max_subdivision_levels;
    let max_iterations = context.policy().limits().max_iterations;
    let budget = metric_budget(law)
        .div(&Rat::from_int(crate::Int::from_u64(
            boundary
                .edges
                .len()
                .saturating_add(
                    boundary
                        .curves
                        .iter()
                        .map(|curve| curve.edges().len())
                        .sum::<usize>(),
                )
                .saturating_add(linear_count)
                .saturating_add(
                    boundary
                        .native
                        .as_ref()
                        .map_or(0, |boundary| boundary.fragments().len()),
                )
                .max(1) as u64,
        )))
        .expect("nonzero edge count");
    let mut retained = boundary.workspace_bytes;
    let result = (|| {
        let mut bits = crate::numerical::scratch_source_bits_in(
            boundary
                .edges
                .iter()
                .flatten()
                .flat_map(|point| [point.longitude(), point.latitude()])
                .chain(core::iter::once(&budget)),
            &ellipsoid,
        );
        for edge in geometry
            .curves()
            .iter()
            .chain(&boundary.curves)
            .flat_map(PreparedCurve::edges)
        {
            bits = bits.max(edge_operand_bits(edge, &ellipsoid));
        }
        if let Some(native) = &boundary.native {
            for fragment in native.fragments() {
                bits = bits.max(edge_operand_bits(fragment.original_edge(), &ellipsoid));
                for parameter in fragment.parameters() {
                    let (lower, upper) = parameter.bounds();
                    bits = bits.max(crate::numerical::scratch_source_bits(
                        &<[_; 2]>::from((lower, upper)),
                        &ellipsoid,
                    ));
                }
            }
        }
        context.prepare_integer_scratch_for_observed(bits, &mut progress)?;
        progress.context_poll(context)?;
        let native_parameters = boundary
            .native
            .as_ref()
            .map(|boundary| native::prepare(boundary, context, &mut progress, &mut retained))
            .transpose()?;
        complete_metric(law, context, &mut progress, |math, progress| {
            let mut linear = FixedInterval::from_i64(0, math)?;
            for curve in geometry.curves() {
                linear = linear.add(
                    &integrate_curve(
                        curve,
                        &ellipsoid,
                        (max_depth, max_iterations),
                        &budget,
                        math,
                        progress,
                    )?,
                    math,
                )?;
            }
            let mut areal = FixedInterval::from_i64(0, math)?;
            for edge in &boundary.edges {
                areal = areal.add(
                    &integrate_linear(
                        (&edge[0], &edge[1]),
                        GeometryMetricLaw::Length,
                        &ellipsoid,
                        max_depth,
                        &budget,
                        math,
                        progress,
                    )?,
                    math,
                )?;
                progress.math_poll(math)?;
            }
            for curve in &boundary.curves {
                areal = areal.add(
                    &integrate_curve(
                        curve,
                        &ellipsoid,
                        (max_depth, max_iterations),
                        &budget,
                        math,
                        progress,
                    )?,
                    math,
                )?;
            }
            if let Some((native, parameters)) =
                boundary.native.as_ref().zip(native_parameters.as_ref())
            {
                for (fragment, parameters) in native.fragments().iter().zip(parameters) {
                    let contribution = native::length(
                        fragment.original_edge(),
                        parameters,
                        &ellipsoid,
                        (max_depth, max_iterations),
                        &budget,
                        math,
                        progress,
                    )?;
                    match fragment.stratum() {
                        crate::atlas::arcs::arrangement::SelectedFragmentStratum::CurveInterior => {
                            linear = linear.add(&contribution, math)?;
                        }
                        crate::atlas::arcs::arrangement::SelectedFragmentStratum::ArealBoundary => {
                            areal = areal.add(&contribution, math)?;
                        }
                    }
                }
            }
            if law == GeometryMetricLaw::Perimeter && boundary.has_areal_faces() {
                Ok(areal)
            } else {
                FixedInterval::from_bounds(
                    linear.lower().clone().max(areal.lower().clone()),
                    linear.upper().clone().max(areal.upper().clone()),
                    math,
                )
            }
        })
    })();
    drop(boundary);
    drop(budget);
    drop(ellipsoid);
    context.release_workspace(retained)?;
    result
}

/// Read complete original operand widths without building per-vertex copies.
/// Actual intermediate panels remain checked by the one numerical destination
/// arena; this preparation admits every source magnitude before conversion.
fn edge_operand_bits(edge: &PreparedEdge, ellipsoid: &crate::PreparedEllipsoid) -> u64 {
    let coordinates = edge
        .original_coordinates()
        .flat_map(|point| [point.x(), point.y()]);
    let mut bits = crate::numerical::scratch_source_bits_in(coordinates, ellipsoid);
    let parameters = match edge {
        PreparedEdge::AzimuthLength(arc) => Some([arc.azimuth(), arc.length().exact()]),
        PreparedEdge::ShortestGeodesic(arc) => Some([
            arc.proof().distance.lower.exact(),
            arc.proof().distance.upper.exact(),
        ]),
        PreparedEdge::Transformed(image) => {
            bits = bits.max(image.max_original_operand_bits());
            None
        }
        PreparedEdge::SourceLinear(_) => None,
    };
    if let Some(parameters) = parameters {
        bits = bits.max(crate::numerical::scratch_source_bits(
            &parameters,
            ellipsoid,
        ));
    }
    bits
}

fn integrate_curve(
    curve: &PreparedCurve,
    ellipsoid: &crate::PreparedEllipsoid,
    (max_depth, max_iterations): (u32, u32),
    budget: &Rat,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    let mut total = FixedInterval::from_i64(0, math)?;
    for edge in curve.edges() {
        let length = match edge {
            PreparedEdge::SourceLinear(edge) => integrate_linear(
                (edge.start().point(), edge.end().point()),
                GeometryMetricLaw::Length,
                ellipsoid,
                max_depth,
                budget,
                math,
                progress,
            )?,
            PreparedEdge::AzimuthLength(arc) => fixed_from_rat(arc.length().exact(), math)?,
            PreparedEdge::ShortestGeodesic(arc) => crate::numerical::fixed_from_bounds(
                arc.proof().distance.lower.exact(),
                arc.proof().distance.upper.exact(),
                math,
            )?,
            PreparedEdge::Transformed(image) => {
                integrate_image(image, (max_depth, max_iterations), budget, math, progress)?
            }
        };
        total = total.add(&length, math)?;
        progress.math_poll(math)?;
    }
    Ok(total)
}

fn integrate_image(
    image: &crate::operation::OperationImageCurve,
    (max_depth, max_iterations): (u32, u32),
    budget: &Rat,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    integrate_image_on(
        image,
        (max_depth, max_iterations),
        budget,
        None,
        math,
        progress,
    )
}

fn integrate_image_on(
    image: &crate::operation::OperationImageCurve,
    (max_depth, max_iterations): (u32, u32),
    budget: &Rat,
    range: Option<[&Rat; 2]>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    if let Some([start, end]) = image.exact_linear_endpoints_in(range, math, progress)? {
        return integrate_linear(
            (&start, &end),
            GeometryMetricLaw::Length,
            image.reference().ellipsoid(),
            max_depth,
            budget,
            math,
            progress,
        );
    }
    let bytes = (max_depth.min(63) as usize + 2).saturating_mul(4096);
    math.reserve_workspace(bytes)?;
    let result = (|| {
        let solver = crate::operation::OperationSolverLimits {
            iterations: max_iterations,
            subdivisions: max_depth,
            quantize_inverse: false,
        };
        let zero = Rat::zero();
        let one = Rat::one();
        let [start, end] = range.unwrap_or([&zero, &one]);
        let complete = match image.enclosure_in(start, end, solver, math, progress) {
            Ok(complete) => Some(complete),
            // A large initial box can straddle an inverse singularity although
            // each complete source subpanel has a certifiable image. The same
            // deterministic subdivision below decides that case.
            Err(MathError::PrecisionExhausted) => None,
            Err(error) => return Err(error),
        };
        if let Some(derivative) = complete
            .as_ref()
            .and_then(|image| image.derivative.as_ref())
            && derivative[0].lower().is_zero()
            && derivative[0].upper().is_zero()
            && (derivative[1].lower() >= &BigInt::zero()
                || derivative[1].upper() <= &BigInt::zero())
        {
            // Constant longitude and a whole-domain derivative sign prove the
            // complete image is a monotone meridian. Its physical integral is
            // exactly the difference of the original endpoint meridian arcs.
            let first = image.enclosure_in(start, start, solver, math, progress)?;
            let last = image.enclosure_in(end, end, solver, math, progress)?;
            let ellipsoid = image.reference().ellipsoid();
            let first = crate::geodesic::meridian_arc_interval_observed(
                &first.latitude,
                ellipsoid,
                math,
                progress,
            )?;
            let last = crate::geodesic::meridian_arc_interval_observed(
                &last.latitude,
                ellipsoid,
                math,
                progress,
            )?;
            progress.math_poll(math)?;
            return last.sub(&first, math)?.abs(math);
        }
        let budget = fixed_from_rat(budget, math)?;
        let jets = jet::MercatorPolynomial::new(image, [start, end], math, progress)?;
        let mut pending =
            purrdf_lex::walk::WorkList::<Panel, 8>::with(Panel { level: 0, index: 0 });
        let mut sum = FixedInterval::from_i64(0, math)?;
        while let Some(panel) = pending.pop() {
            progress.math_poll(math)?;
            let denominator = crate::Int::one().shl(panel.level);
            let lower = Rat::new(crate::Int::from_u64(panel.index), denominator.clone())
                .expect("positive dyadic");
            let upper = Rat::new(crate::Int::from_u64(panel.index + 1), denominator)
                .expect("positive dyadic");
            let (lower, upper) = if range.is_some() {
                (
                    crate::SourceLinearEdge::interpolate_ordinate_math(
                        start, end, &lower, math, progress,
                    )?,
                    crate::SourceLinearEdge::interpolate_ordinate_math(
                        start, end, &upper, math, progress,
                    )?,
                )
            } else {
                (lower, upper)
            };
            let width = fixed_from_rat(&upper.sub(&lower), math)?;
            // An analytic panel uses its Taylor model; a panel where that model
            // cannot certify (a vanishing speed) keeps the first-order bound.
            let modelled = match &jets {
                Some(jets) => match jets.panel(panel.level, panel.index, math, progress) {
                    Ok(value) => Some(value),
                    Err(MathError::PrecisionExhausted) => None,
                    Err(error) => return Err(error),
                },
                None => None,
            };
            let contribution = match modelled {
                Some(value) => Ok(value),
                None => image
                    .speed_in(&lower, &upper, solver, math, progress)
                    .and_then(|speed| speed.mul(&width, math)),
            };
            let accepted = match &contribution {
                Ok(value) => value.width(math)?.upper() <= budget.mul(&width, math)?.lower(),
                Err(MathError::PrecisionExhausted) => false,
                Err(error) => return Err(error.clone()),
            };
            if accepted {
                let value = contribution?;
                // This frozen interval quadrature selects its exact dyadic
                // midpoint. The complete integral differs by at most budget/2;
                // no heuristic successive-quadrature comparison is used.
                sum = sum.add(&value.midpoint(math)?, math)?;
            } else {
                if panel.level >= max_depth || panel.level >= 63 {
                    return Err(MathError::ConvergenceExhausted {
                        iterations: max_depth,
                    });
                }
                let next = panel.index.checked_mul(2).ok_or(MathError::WorkExhausted)?;
                pending.push(Panel {
                    level: panel.level + 1,
                    index: next + 1,
                });
                pending.push(Panel {
                    level: panel.level + 1,
                    index: next,
                });
            }
        }
        Ok(sum)
    })();
    math.release_workspace(bytes)?;
    result
}

fn metric_budget(law: GeometryMetricLaw) -> Rat {
    Rat::parse_decimal(if law.is_area() { "0.025" } else { "0.00025" }).expect("frozen decimal")
}

fn metric_error_bound(
    law: GeometryMetricLaw,
    value: &Rat,
    mut admission: Option<&mut crate::numerical::ExactAdmission<'_, '_>>,
    retained: &mut u64,
) -> Result<Rat, GeoError> {
    use purrdf_xsd::integer::ExactOperation;
    let absolute = match law {
        GeometryMetricLaw::Distance => "0.00025",
        GeometryMetricLaw::Length | GeometryMetricLaw::Perimeter => "0.001",
        GeometryMetricLaw::Area
        | GeometryMetricLaw::AreaIntegral
        | GeometryMetricLaw::GeodesicAreaIntegral => "0.1",
    };
    let bound = crate::numerical::frozen_decimal(absolute);
    if law == GeometryMetricLaw::Distance {
        return Ok(bound);
    }
    let (magnitude, _) = crate::numerical::exact_retained_rational(
        admission.as_deref_mut(),
        ExactOperation::Linear,
        &[value],
        retained,
        || value.abs(),
    )?;
    let factor = crate::numerical::frozen_decimal("0.00000000000001");
    let (relative, _) = crate::numerical::exact_retained_rational(
        admission.as_deref_mut(),
        ExactOperation::RationalMultiply,
        &[&magnitude, &factor],
        retained,
        || magnitude.mul(&factor),
    )?;
    let order = if let Some(admission) = admission {
        admission.compare(&bound, &relative)?
    } else {
        bound.cmp(&relative)
    };
    Ok(if order.is_lt() { relative } else { bound })
}

fn linear_metric(
    start: &LonLat,
    end: &LonLat,
    law: GeometryMetricLaw,
    context: &mut MetricContext,
) -> Result<GeometryMetricEstimate, GeoError> {
    let ellipsoid = context.reference().ellipsoid().clone();
    let max_depth = context.policy().limits().max_subdivision_levels;
    let mut progress = WorkProgress::new(None);
    let bits = crate::numerical::scratch_source_bits(
        &[
            start.longitude(),
            start.latitude(),
            end.longitude(),
            end.latitude(),
        ],
        &ellipsoid,
    );
    context.prepare_integer_scratch_for(bits)?;
    progress.context_poll(context)?;
    complete_metric(law, context, &mut progress, |math, progress| {
        integrate_linear(
            (start, end),
            law,
            &ellipsoid,
            max_depth,
            &metric_budget(law),
            math,
            progress,
        )
    })
}

fn complete_metric(
    law: GeometryMetricLaw,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    evaluate: impl FnOnce(
        &mut CoordinateMath,
        &mut WorkProgress<'_>,
    ) -> Result<FixedInterval, MathError>,
) -> Result<GeometryMetricEstimate, GeoError> {
    if let Some(error) = progress.latched_error() {
        return Err(error);
    }
    // Every completed scalar must pass round_decimal_with's interval
    // admission and scaled-quotient admission, each charging at least one
    // work item. Refuse an impossible completion before preparing its arena.
    // The actual original kernels still charge their full operand-shaped cost.
    context.next_work(2)?;
    let policy = context.policy();
    let bytes = 256_u64 * (96_u64.div_ceil(8) * 8 + 128);
    let (value, output_bytes) = crate::numerical::with_math_for_sources_retained(
        context,
        progress,
        96,
        bytes as usize,
        &[],
        |math, progress| {
            evaluate(math, progress)
                .and_then(|interval| {
                    let interval = if law == GeometryMetricLaw::Area {
                        FixedInterval::from_bounds(
                            interval.lower().clone().max(BigInt::zero()),
                            interval.upper().clone().max(BigInt::zero()),
                            math,
                        )?
                    } else {
                        interval
                    };
                    let (lower, upper) = interval.round_decimal(law.places(), math)?;
                    if lower != upper {
                        return Err(MathError::PrecisionExhausted);
                    }
                    let value = crate::numerical::math_quantized_decimal(
                        &lower,
                        law.places(),
                        math,
                        progress,
                    )?;
                    let ([value], bytes) =
                        crate::numerical::math_share_rationals([value], math, progress)?;
                    Ok((value, bytes))
                })
                .map_err(|error| geo_math_error(&error, policy))
        },
    )?;
    let mut temporary_bytes = 0;
    let result = (|| {
        let reported = crate::metric::reported_double_with_progress(&value, context, progress)?;
        // The chosen polynomial's complete enclosure rounds to this value. The
        // Taylor remainder, decimal half-quantum and reported conversion therefore
        // bound the entire response, independently of a tighter invocation proof.
        let remainder = metric_budget(law);
        let quantum = Rat::from_decimal(crate::Int::one(), law.places());
        // Compare the exact half-ulp with the remaining certificate allowance.
        // This is the same sum inequality, without constructing a generic reduced
        // rational whose denominator at zero would contain 2^1075.
        let half_ulp_cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            purrdf_xsd::integer::ExactOperation::Linear,
            1076,
            4,
        )
        .ok_or(GeoError::ArithmeticOverflow(
            "metric conversion bound admission",
        ))?;
        // Every finite binary64 half-ulp is a single power of two with numerator
        // or denominator at most 1076 bits. Its original construction uses only
        // a shift, canonical dyadic cancellation and linear copies, with no GCD.
        context.retain_workspace(half_ulp_cost.workspace_bytes, &mut temporary_bytes)?;
        let conversion = progress.exact(context, half_ulp_cost, || {
            Ok(crate::metric::binary64_half_ulp(reported.to_bits()))
        })?;
        let mut admission = crate::numerical::ExactAdmission::new(context, progress);
        let allowed = metric_error_bound(law, &value, Some(&mut admission), &mut temporary_bytes)?;
        let two = Rat::from_i64(2);
        let (half_quantum, _) = admission.rational_retained(
            purrdf_xsd::integer::ExactOperation::RationalDivide,
            &[&quantum, &two],
            &mut temporary_bytes,
            || quantum.div(&two).expect("positive divisor"),
        )?;
        let (after_remainder, _) = admission.rational_retained(
            purrdf_xsd::integer::ExactOperation::RationalAdd,
            &[&allowed, &remainder],
            &mut temporary_bytes,
            || allowed.sub(&remainder),
        )?;
        let (remaining, _) = admission.rational_retained(
            purrdf_xsd::integer::ExactOperation::RationalAdd,
            &[&after_remainder, &half_quantum],
            &mut temporary_bytes,
            || after_remainder.sub(&half_quantum),
        )?;
        if admission.compare(&conversion, &remaining)?.is_gt() {
            return Err(GeoError::PrecisionExhausted {
                bits: policy.limits().max_precision_bits,
            });
        }
        let binding = crate::numerical::reference_identity(context, progress, None)?;
        Ok(GeometryMetricEstimate {
            value,
            reported: reported.to_bits(),
            law,
            binding,
        })
    })();
    // Temporary bound owners have been dropped by the closure. The detached
    // scalar follows the same returned-owner contract as MetricEstimate; its
    // storage stayed admitted through all checks, and its caller can census it.
    context.release_workspace(temporary_bytes)?;
    context.release_workspace(output_bytes)?;
    result
}

#[derive(Clone, Copy)]
struct Panel {
    level: u32,
    index: u64,
}

struct LinearOperands {
    latitude: FixedInterval,
    latitude_delta: FixedInterval,
    longitude_delta: FixedInterval,
    a_squared: FixedInterval,
    e_squared: FixedInterval,
}

pub(crate) fn integrate_linear(
    edge: (&LonLat, &LonLat),
    law: GeometryMetricLaw,
    ellipsoid: &crate::PreparedEllipsoid,
    max_depth: u32,
    budget: &Rat,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    let (start, end) = edge;
    let zero = FixedInterval::from_i64(0, math)?;
    if (!law.is_area()
        && ((start.longitude() == end.longitude() && start.latitude() == end.latitude())
            || (start.is_pole() && start.latitude() == end.latitude())))
        || (law.is_area() && start.longitude() == end.longitude())
    {
        return Ok(zero);
    }
    let pi = FixedInterval::pi(math)?;
    let factor = pi.div(&FixedInterval::from_i64(180, math)?, math)?;
    let latitude = fixed_from_rat(start.latitude(), math)?.mul(&factor, math)?;
    let latitude_delta =
        fixed_from_rat(&end.latitude().sub(start.latitude()), math)?.mul(&factor, math)?;
    let longitude_delta =
        fixed_from_rat(&end.longitude().sub(start.longitude()), math)?.mul(&factor, math)?;
    if !law.is_area() && start.longitude() == end.longitude() {
        return crate::geodesic::meridian_arc_observed(end.latitude(), ellipsoid, math, progress)?
            .sub(
                &crate::geodesic::meridian_arc_observed(
                    start.latitude(),
                    ellipsoid,
                    math,
                    progress,
                )?,
                math,
            )?
            .abs(math);
    }
    let operands = LinearOperands {
        latitude,
        latitude_delta,
        longitude_delta,
        a_squared: fixed_from_rat(ellipsoid.semimajor(), math)?.square(math)?,
        e_squared: fixed_from_rat(&ellipsoid.eccentricity_squared(), math)?,
    };
    let budget = fixed_from_rat(budget, math)?;
    let mut stack = Vec::with_capacity(max_depth.min(63) as usize + 1);
    stack.push(Panel { level: 0, index: 0 });
    let mut sum = FixedInterval::from_i64(0, math)?;
    while let Some(panel) = stack.pop() {
        progress.math_poll(math)?;
        let denominator = BigInt::from_i128(1).mul_pow2(panel.level);
        let lower = FixedInterval::from_ratio(
            &BigInt::from_i128(i128::from(panel.index)),
            &denominator,
            math,
        )?;
        let upper = FixedInterval::from_ratio(
            &BigInt::from_i128(i128::from(panel.index) + 1),
            &denominator,
            math,
        )?;
        let interval =
            FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), math)?;
        let two = FixedInterval::from_i64(2, math)?;
        let middle = lower.add(&upper, math)?.div(&two, math)?;
        let half = upper.sub(&lower, math)?.div(&two, math)?;
        let width = upper.sub(&lower, math)?;
        let (polynomial, remainder) =
            panel_integral(&operands, law, &middle, &interval, &half, math)?;
        progress.math_poll(math)?;
        if remainder.upper() <= budget.mul(&width, math)?.lower() {
            sum = sum.add(&polynomial, math)?;
        } else {
            if panel.level >= max_depth || panel.level >= 63 {
                return Err(MathError::ConvergenceExhausted {
                    iterations: max_depth,
                });
            }
            let next = panel.index.checked_mul(2).ok_or(MathError::WorkExhausted)?;
            stack.push(Panel {
                level: panel.level + 1,
                index: next + 1,
            });
            stack.push(Panel {
                level: panel.level + 1,
                index: next,
            });
        }
    }
    Ok(sum)
}

fn panel_integral(
    operands: &LinearOperands,
    law: GeometryMetricLaw,
    middle: &FixedInterval,
    interval: &FixedInterval,
    half: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<(FixedInterval, FixedInterval), MathError> {
    let panel = purrdf_xsd::math::SymmetricTaylorPanel::new(middle, interval, half, math)?;
    purrdf_xsd::math::integrate_taylor_panel(8, 64, panel, math, |parameter, workspace, math| {
        evaluate_jet(operands, law, parameter, workspace, math)
    })
}

fn evaluate_jet<'scope>(
    operands: &LinearOperands,
    law: GeometryMetricLaw,
    t: &FixedInterval,
    workspace: &mut TaylorWorkspace<'scope>,
    math: &mut CoordinateMath,
) -> Result<TaylorJet<'scope>, MathError> {
    let one_value = FixedInterval::from_i64(1, math)?;
    let one = workspace.constant(one_value.clone(), math)?;
    let phi = workspace.argument(
        operands
            .latitude
            .add(&operands.latitude_delta.mul(t, math)?, math)?,
        operands.latitude_delta.clone(),
        math,
    )?;
    let (sine, cosine) = workspace.sin_cos(phi, math)?;
    let squared_sine = workspace.mul(sine, sine, math)?;
    let es_squared = workspace.scale(squared_sine, &operands.e_squared, math)?;
    let denominator = workspace.sub(one, es_squared, math)?;
    if law.is_area() {
        let first = workspace.div(sine, denominator, math)?;
        let eccentricity = operands.e_squared.sqrt(math)?;
        let es = workspace.scale(sine, &eccentricity, math)?;
        let positive = workspace.add(one, es, math)?;
        let negative = workspace.sub(one, es, math)?;
        let ratio = workspace.div(positive, negative, math)?;
        let logarithm = workspace.log(ratio, math)?;
        let atanh_factor = one_value.div(
            &eccentricity.mul(&FixedInterval::from_i64(2, math)?, math)?,
            math,
        )?;
        let second = workspace.scale(logarithm, &atanh_factor, math)?;
        let total = workspace.add(first, second, math)?;
        let factor = operands
            .a_squared
            .mul(&one_value.sub(&operands.e_squared, math)?, math)?
            .div(&FixedInterval::from_i64(2, math)?, math)?
            .mul(&operands.longitude_delta, math)?
            .neg(math)?;
        workspace.scale(total, &factor, math)
    } else {
        let cube = workspace.mul(denominator, denominator, math)?;
        let cube = workspace.mul(cube, denominator, math)?;
        let meridian_squared = one_value
            .sub(&operands.e_squared, math)?
            .square(math)?
            .mul(&operands.latitude_delta.square(math)?, math)?;
        let numerator = workspace.constant(meridian_squared, math)?;
        let first = workspace.div(numerator, cube, math)?;
        let cos_squared = workspace.mul(cosine, cosine, math)?;
        let second = workspace.div(cos_squared, denominator, math)?;
        let second = workspace.scale(second, &operands.longitude_delta.square(math)?, math)?;
        let squared_speed = workspace.add(first, second, math)?;
        let squared_speed = workspace.scale(squared_speed, &operands.a_squared, math)?;
        workspace.sqrt(squared_speed, math)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_wide_scalar_keeps_detached_ownership_and_refusal_receipts() {
        struct Receipt {
            work: u64,
            peak: u64,
            calls: u32,
            stop: u32,
        }
        impl MetricWorkObserver for Receipt {
            fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
                self.work += work;
                self.peak += growth;
                self.calls += 1;
                if self.calls == self.stop {
                    Err(GeoError::Cancelled)
                } else {
                    Ok(())
                }
            }
        }
        let value = BigInt::from(crate::Int::one().shl(200));
        let expected = Rat::from_int(value.as_integer().clone());
        let mut context = MetricContext::wgs84().unwrap();
        context.begin(1).unwrap();
        let mut receipt = Receipt {
            work: 0,
            peak: 0,
            calls: 0,
            stop: u32::MAX,
        };
        let result = complete_metric(
            GeometryMetricLaw::Length,
            &mut context,
            &mut WorkProgress::new(Some(&mut receipt)),
            |math, _| FixedInterval::from_ratio(&value, &BigInt::from_i128(1), math),
        )
        .unwrap();
        assert_eq!(result.exact(), &expected);
        assert!(result.retained_workspace_bytes() > size_of::<GeometryMetricEstimate>() as u64);
        assert_eq!(context.work_items(), receipt.work);
        assert_eq!(context.workspace_peak(), receipt.peak);
        assert_eq!(
            context.current_workspace_bytes(),
            context.retained_workspace_bytes()
        );
        let successful_calls = receipt.calls;
        let certificate = result.certificate_bytes();
        // Reusable arithmetic destinations cannot remain in a returned scalar.
        assert_eq!(
            context.integer_scratch().unwrap().available(),
            context.integer_scratch().unwrap().destination_capacity()
        );
        drop(context);
        assert_eq!(result.exact(), &expected);
        assert_eq!(result.certificate_bytes(), certificate);
        for stop in [1, successful_calls / 2, successful_calls] {
            let mut context = MetricContext::wgs84().unwrap();
            context.begin(1).unwrap();
            let mut receipt = Receipt {
                work: 0,
                peak: 0,
                calls: 0,
                stop,
            };
            let refused = complete_metric(
                GeometryMetricLaw::Length,
                &mut context,
                &mut WorkProgress::new(Some(&mut receipt)),
                |math, _| FixedInterval::from_ratio(&value, &BigInt::from_i128(1), math),
            );
            assert_eq!(refused.unwrap_err(), GeoError::Cancelled);
            assert_eq!(receipt.calls, stop);
            assert_eq!(context.work_items(), receipt.work);
            assert_eq!(context.workspace_peak(), receipt.peak);
            assert_eq!(
                context.current_workspace_bytes(),
                context.retained_workspace_bytes()
            );
            context.begin(1).unwrap();
        }
        for memory in [false, true] {
            let policy = crate::ExecutionPolicy::new(crate::ExecutionLimits {
                max_work_items: if memory { 1_000_000 } else { 1 },
                max_workspace_bytes: if memory { 1 } else { 64 * 1024 * 1024 },
                ..crate::ExecutionLimits::GEOMETRY
            })
            .unwrap();
            let mut context =
                MetricContext::new(crate::GeographicReference::wgs84(), policy).unwrap();
            context.begin(1).unwrap();
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let refused = complete_metric(
                GeometryMetricLaw::Length,
                &mut context,
                &mut WorkProgress::new(None),
                |math, _| FixedInterval::from_ratio(&value, &BigInt::from_i128(1), math),
            );
            let measured = window.close();
            assert!(matches!(
                refused,
                Err(GeoError::WorkExhausted { limit: 1 } | GeoError::MemoryExhausted { limit: 1 })
            ));
            assert_eq!(measured.allocations, 0);
            assert_eq!(measured.requested_bytes, 0);
            assert_eq!(context.current_workspace_bytes(), 0);
        }
    }

    #[test]
    fn collapsed_native_region_and_curve_use_the_same_one_dimensional_total() {
        let reference = crate::GeographicReference::wgs84();
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items = 8_000_000;
        let policy = crate::ExecutionPolicy::new(limits).unwrap();
        let mut context = MetricContext::new(reference.clone(), policy).unwrap();
        let rectangle = |west, east| {
            let coordinates = [(west, 0), (east, 0), (east, 1), (west, 1), (west, 0)]
                .map(|(x, y)| crate::Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
            PreparedCurve::from_source(&coordinates, &reference).unwrap()
        };
        // Both original Jordan left interiors meet only on the meridian wall.
        // Their closed intersection is a curve, with no selected surface face.
        let collapsed = crate::PreparedPolygon::from_curves(
            vec![rectangle(0, 1), rectangle(1, 2)],
            crate::OrientedInterior::Left,
            &mut context,
        )
        .unwrap();
        let curve = |coordinates: [(i64, i64); 2]| {
            let coordinates =
                coordinates.map(|(x, y)| crate::Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
            PreparedCurve::from_source(&coordinates, &reference).unwrap()
        };
        let independent = curve([(3, 0), (4, 0)]);
        let actual = PreparedGeometry::from_parts(
            reference.clone(),
            Vec::new(),
            vec![independent.clone()],
            PreparedRegion::polygons(vec![collapsed]),
            policy,
        )
        .unwrap();
        let expected = PreparedGeometry::from_parts(
            reference.clone(),
            Vec::new(),
            vec![curve([(1, 0), (1, 1)]), independent],
            PreparedRegion::Empty,
            policy,
        )
        .unwrap();
        let expected_length = length(&expected, &mut context).unwrap();
        let actual_length = length(&actual, &mut context).unwrap();
        assert_eq!(actual_length, expected_length);
        assert_eq!(
            perimeter(&actual, &mut context).unwrap(),
            perimeter(&expected, &mut context).unwrap()
        );
        assert_eq!(area(&actual, &mut context).unwrap().exact(), &Rat::zero());
        let contact = prepared("POINT(1 0.5)");
        assert_eq!(
            distance(&actual, &contact, &mut context).unwrap().exact(),
            &Rat::zero()
        );
        assert!(
            within_physical(&actual, &contact, &Metres::new(Rat::zero()), &mut context).unwrap()
        );
        let separated = prepared("POINT(1 -1)");
        assert_eq!(
            distance(&actual, &separated, &mut context).unwrap(),
            distance(&expected, &separated, &mut context).unwrap()
        );
    }

    #[test]
    fn native_union_perimeter_uses_original_selected_contact_fragments() {
        let profile = crate::GeoProfile::standard();
        let crs = crate::Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap();
        let reference = crate::GeographicReference::wgs84();
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items = 8_000_000;
        let policy = crate::ExecutionPolicy::new(limits).unwrap();
        let mut context = MetricContext::new(reference.clone(), policy).unwrap();
        let mut rectangle = |west: i64, east: i64| {
            let coordinates = [(west, 0), (east, 0), (east, 4), (west, 4), (west, 0)]
                .map(|(x, y)| crate::Coord::xy(Rat::from_i64(x), Rat::from_i64(y)));
            let curve = PreparedCurve::from_source(&coordinates, &reference).unwrap();
            crate::PreparedPolygon::from_curves(
                vec![curve],
                crate::OrientedInterior::Left,
                &mut context,
            )
            .unwrap()
        };
        let region = PreparedRegion::polygons(vec![rectangle(0, 4), rectangle(2, 6)]);
        let native = PreparedGeometry::from_parts(
            reference.clone(),
            Vec::new(),
            Vec::new(),
            region.clone(),
            policy,
        )
        .unwrap();
        let actual = perimeter(&native, &mut context).unwrap();
        let literal = crate::wkt::parse("POLYGON((0 0,6 0,6 4,0 4,0 0))", &crs).unwrap();
        let written = PreparedGeometry::from_literal(&literal, &profile).unwrap();
        let expected = perimeter(&written, &mut context).unwrap();
        assert!(actual.exact().sub(expected.exact()).abs() < Rat::parse_decimal("0.001").unwrap());
        let complement = PreparedGeometry::from_parts(
            reference.clone(),
            Vec::new(),
            Vec::new(),
            region.complement(),
            policy,
        )
        .unwrap();
        assert_eq!(perimeter(&complement, &mut context).unwrap(), actual);
        limits.max_work_items *= 2;
        let mut raised =
            MetricContext::new(reference, crate::ExecutionPolicy::new(limits).unwrap()).unwrap();
        assert_eq!(perimeter(&native, &mut raised).unwrap(), actual);
        let actual_area = area(&native, &mut context).unwrap();
        let expected_area = area(&written, &mut context).unwrap();
        assert!(
            actual_area.exact().sub(expected_area.exact()).abs()
                < Rat::parse_decimal("0.1").unwrap()
        );
        assert_eq!(area(&native, &mut raised).unwrap(), actual_area);
    }

    fn point(longitude: &str, latitude: &str) -> LonLat {
        LonLat::new(rat(longitude), rat(latitude)).unwrap()
    }
    fn rat(text: &str) -> Rat {
        Rat::parse_decimal(text).unwrap()
    }

    #[test]
    fn transformed_meridian_measures_the_complete_ground_curve() {
        use crate::operation::{
            CoordinateOperation, CoordinateUnit, OperationChain, OperationImageCurve,
            OperationModel, OperationReference,
        };
        use crate::{Coord, ExecutionPolicy, GeographicReference};
        use purrdf_hash::hex::Digest32;
        use std::sync::Arc;
        let reference = GeographicReference::wgs84();
        let policy = ExecutionPolicy::geometry();
        let operation = CoordinateOperation::compile(
            OperationReference {
                realization: Digest32::new([9; 32]),
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
        .unwrap();
        let image = OperationImageCurve::new(
            Coord::xy(Rat::zero(), Rat::from_i64(-1_000_000)),
            Coord::xy(Rat::zero(), Rat::from_i64(1_000_000)),
            Arc::new(OperationChain::compile(vec![operation]).unwrap()),
            reference.clone(),
            None,
            policy,
        )
        .unwrap();
        let mut context = MetricContext::new(reference.clone(), policy).unwrap();
        let endpoint = |t: &Rat, context: &mut MetricContext| {
            let enclosure = image.enclosure(t, t, 128, context).unwrap();
            let (lower, upper) = crate::numerical::exact_bounds(&enclosure.latitude);
            LonLat::new(
                Rat::zero(),
                lower.add(&upper).div(&Rat::from_i64(2)).unwrap(),
            )
            .unwrap()
        };
        let start = endpoint(&Rat::zero(), &mut context);
        let end = endpoint(&Rat::one(), &mut context);
        let oracle = crate::PreparedGeodesic::new(reference.clone())
            .distance(&start, &end, &mut context)
            .unwrap();
        let geometry = PreparedGeometry::from_parts(
            reference,
            Vec::new(),
            vec![PreparedCurve::new(vec![PreparedEdge::Transformed(
                Box::new(image),
            )])],
            PreparedRegion::Empty,
            policy,
        )
        .unwrap();
        let result = length(&geometry, &mut context).unwrap();
        assert!(result.exact().sub(oracle.value().exact()).abs() <= rat("0.000001"));
        assert!(Rat::from_i64(2_000_000).sub(result.exact()) > Rat::from_i64(10_000));
        assert!(context.work_items() <= policy.limits().max_work_items);
    }

    #[test]
    fn written_equator_seam_is_a_complete_circuit() {
        let mut context = MetricContext::wgs84().unwrap();
        let result =
            source_linear_length(&point("-180", "0"), &point("180", "0"), &mut context).unwrap();
        // A bounded independent decimal pi gives an analytic equatorial oracle.
        let pi = rat("3.14159265358979323846264338327950288419716939937510");
        let analytic = pi.mul(&rat("12756274"));
        assert!(result.exact().sub(&analytic).abs() < rat("0.000001"));
        assert!(result.exact() > &rat("40000000"));
        let long =
            source_linear_length(&point("170", "0"), &point("-170", "0"), &mut context).unwrap();
        let analytic = analytic.mul(&rat("340")).div(&rat("360")).unwrap();
        assert!(long.exact().sub(&analytic).abs() < rat("0.000001"));
    }

    #[test]
    fn collapsed_pole_and_zero_area_meridian_are_exact() {
        let mut context = MetricContext::wgs84().unwrap();
        let result =
            source_linear_length(&point("-180", "90"), &point("180", "90"), &mut context).unwrap();
        assert!(result.exact().is_zero());
        let result =
            source_linear_area_integral(&point("12", "-90"), &point("12", "90"), &mut context)
                .unwrap();
        assert!(result.exact().is_zero());
    }

    #[test]
    fn meridian_carrier_matches_analytic_inverse_without_shortening_longitude() {
        let mut context = MetricContext::wgs84().unwrap();
        let start = point("0", "-20");
        let end = point("0", "40");
        let result = source_linear_length(&start, &end, &mut context).unwrap();
        let inverse = crate::geodesic::distance(start, end).unwrap();
        assert_eq!(result.exact(), inverse.value().exact());
        assert!(context.work_items() > 0);
        assert!(context.workspace_peak() > 0);
    }

    #[test]
    fn response_is_stable_under_adequate_admission_and_refuses_low_precision() {
        let start = point("1", "10");
        let end = point("2", "11");
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items = 4_000_000;
        let mut context = MetricContext::new(
            crate::GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let first = source_linear_length(&start, &end, &mut context).unwrap();
        limits.max_precision_bits = 192;
        let mut other = MetricContext::new(
            crate::GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        let second = source_linear_length(&start, &end, &mut other).unwrap();
        assert_eq!(first.certificate_bytes(), second.certificate_bytes());
        limits.max_precision_bits = 80;
        let mut other = MetricContext::new(
            crate::GeographicReference::wgs84(),
            crate::ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert!(matches!(
            source_linear_length(&start, &end, &mut other),
            Err(GeoError::PrecisionExhausted { bits: 80 })
        ));
    }
    fn prepared(text: &str) -> PreparedGeometry {
        let profile = crate::GeoProfile::standard();
        let literal = crate::wkt::parse(
            text,
            &crate::Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap(),
        )
        .unwrap();
        PreparedGeometry::from_literal(&literal, &profile).unwrap()
    }

    #[test]
    fn geometry_length_sums_dimensions_and_includes_holes_once() {
        let polygon =
            prepared("POLYGON((0 0,2 0,2 2,0 2,0 0),(0.5 0.5,0.5 1.5,1.5 1.5,1.5 0.5,0.5 0.5))");
        let collection = prepared(
            "GEOMETRYCOLLECTION(POLYGON((0 0,2 0,2 2,0 2,0 0),(0.5 0.5,0.5 1.5,1.5 1.5,1.5 0.5,0.5 0.5)),LINESTRING(-180 0,180 0),POINT(0 0))",
        );
        let mut context = MetricContext::wgs84().unwrap();
        let boundary = perimeter(&polygon, &mut context).unwrap();
        assert!(boundary.exact() > &rat("1300000"));
        assert_eq!(
            perimeter(&collection, &mut context).unwrap().exact(),
            boundary.exact()
        );
        assert!(length(&collection, &mut context).unwrap().exact() > &rat("40000000"));
    }

    #[test]
    fn perimeter_uses_physical_union_boundary_and_discards_global_cuts() {
        let single = prepared("POLYGON((0 0,2 0,2 2,0 2,0 0))");
        let duplicate = prepared("MULTIPOLYGON(((0 0,2 0,2 2,0 2,0 0)),((0 0,2 0,2 2,0 2,0 0)))");
        let split = prepared("MULTIPOLYGON(((0 0,1 0,1 2,0 2,0 0)),((1 0,2 0,2 2,1 2,1 0)))");
        let whole = prepared("POLYGON((-180 -90,180 -90,180 90,-180 90,-180 -90))");
        let mut context = MetricContext::wgs84().unwrap();
        let expected = perimeter(&single, &mut context).unwrap();
        assert_eq!(
            perimeter(&duplicate, &mut context).unwrap().exact(),
            expected.exact()
        );
        assert_eq!(
            perimeter(&split, &mut context).unwrap().exact(),
            expected.exact()
        );
        assert!(perimeter(&whole, &mut context).unwrap().exact().is_zero());
        assert!(length(&whole, &mut context).unwrap().exact().is_zero());
    }

    #[test]
    fn region_area_is_a_union_with_holes_complements_and_whole_surface() {
        let single = prepared("POLYGON((0 0,2 0,2 2,0 2,0 0))");
        let duplicate = prepared("MULTIPOLYGON(((0 0,2 0,2 2,0 2,0 0)),((0 0,2 0,2 2,0 2,0 0)))");
        let split = prepared("MULTIPOLYGON(((0 0,1 0,1 2,0 2,0 0)),((1 0,2 0,2 2,1 2,1 0)))");
        let hole =
            prepared("POLYGON((0 0,2 0,2 2,0 2,0 0),(0.5 0.5,0.5 1.5,1.5 1.5,1.5 0.5,0.5 0.5))");
        let mut context = MetricContext::wgs84().unwrap();
        let expected = area(&single, &mut context).unwrap();
        assert_eq!(
            area(&duplicate, &mut context).unwrap().exact(),
            expected.exact()
        );
        assert_eq!(
            area(&split, &mut context).unwrap().exact(),
            expected.exact()
        );
        let holed = area(&hole, &mut context).unwrap();
        assert!(holed.exact() < expected.exact());
        let whole = region_area(&PreparedRegion::Whole, &mut context).unwrap();
        assert!(whole.exact() > &rat("510000000000000"));
        assert!(whole.exact() < &rat("511000000000000"));
        assert!(
            region_area(&PreparedRegion::Empty, &mut context)
                .unwrap()
                .exact()
                .is_zero()
        );
        let crate::GeometryBody::Polygon(rings) = crate::wkt::parse(
            "POLYGON((0 0,2 0,2 2,0 2,0 0))",
            &crate::Crs::new(purrdf_iri::vocab::ogc::CRS84).unwrap(),
        )
        .unwrap()
        .into_geometry()
        .body()
        .clone() else {
            unreachable!()
        };
        let complement = PreparedRegion::polygons(vec![
            crate::PreparedPolygon::from_source(
                &rings,
                context.reference(),
                crate::RegionInterior::Complement,
            )
            .unwrap(),
        ]);
        let complement = region_area(&complement, &mut context).unwrap();
        assert!(
            complement
                .exact()
                .add(expected.exact())
                .sub(whole.exact())
                .abs()
                <= rat("0.01")
        );
    }
}
