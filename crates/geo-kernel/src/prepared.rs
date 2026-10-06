// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable original-coordinate curves and explicitly selected geographic regions.

use purrdf_hash::{Domain, blake3::Hasher, frame::frame_le_into, hex::Digest32};
use std::sync::Arc;

use crate::context::WorkProgress;
use crate::geodesic::{
    ArcIntervalView, DirectResult, InverseProofReceipt, InverseResult, PreparedGeodesic,
    ShortestBranchMultiplicity,
};
use crate::{
    AxisOrder, Coord, GeoBindingId, GeoError, GeoProfile, GeographicReference, GeometryBody,
    GeometryLiteral, LonLat, Metres, MetricContext, MetricWorkObserver, OperationImagePoint, Rat,
};

const PREPARED_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/prepared-geographic-geometry/v1");
const SURFACE_VIEW_DOMAIN: Domain =
    Domain::new(b"purrdf-geo-kernel/exact-surface-reference-view/v1");
const SOURCE_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/exact-carrier-source/v1");
const SYMBOLIC_POINTS_DOMAIN: Domain = Domain::new(b"exact-symbolic-operation-image-points/v1");

/// Original geographic position and verbatim carrier ordinates. Z and M are
/// retained as data; surface metrics never invent a height from them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedCoordinate {
    point: LonLat,
    source: Coord,
    binding: GeoBindingId,
}
impl PreparedCoordinate {
    /// Validate original source axes under an explicit geographic reference.
    ///
    /// # Errors
    /// Refuses an original angular coordinate outside its valid range.
    pub fn new(source: Coord, reference: &GeographicReference) -> Result<Self, GeoError> {
        Self::with_binding(source, reference, reference.id())
    }
    /// Validate original coordinates with complete exact arithmetic admission.
    ///
    /// # Errors
    /// Refuses range/reference mismatch, limits or cancellation before arithmetic.
    pub fn new_in_context(
        source: Coord,
        reference: &GeographicReference,
        context: &mut MetricContext,
    ) -> Result<Self, GeoError> {
        Self::prepare_coordinate(source, reference, context, None)
    }
    /// Validate an owned coordinate with external governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::new_in_context`].
    pub fn new_in_context_metered(
        source: Coord,
        reference: &GeographicReference,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_coordinate(source, reference, context, Some(observer))
    }
    fn prepare_coordinate(
        source: Coord,
        reference: &GeographicReference,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        context.begin_integer(1)?;
        let mut progress = WorkProgress::integer(observer);
        progress.initial()?;
        Self::owned_with_progress(source, reference, context, &mut progress)
    }
    fn owned_with_progress(
        source: Coord,
        reference: &GeographicReference,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        let source_binding =
            crate::numerical::reference_identity(context, progress, Some(reference))?;
        let target = crate::numerical::reference_identity(context, progress, None)?;
        matching_binding(source_binding, target)?;
        let bytes = coordinate_bit_count(&source)
            .div_ceil(8)
            .saturating_mul(32)
            .saturating_add(size_of::<Self>() as u64)
            .saturating_add(4096);
        context.admit_workspace(bytes)?;
        let result = (|| {
            let cost = crate::numerical::rational_cost(
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &[source.x(), source.y()],
                4,
            )
            .ok_or(GeoError::ArithmeticOverflow("coordinate range admission"))?;
            progress.exact(context, cost, || Self::new(source, reference))
        })();
        context.release_workspace(bytes)?;
        if result.is_ok() {
            progress.context_poll(context)?;
        }
        result
    }
    /// Clone and validate a borrowed carrier only after admitting its exact
    /// limbs, including Z/M, and the original angular bound comparisons.
    ///
    /// # Errors
    /// Uses [`Self::new_in_context`]'s complete admission contract.
    pub fn from_source_in_context(
        source: &Coord,
        reference: &GeographicReference,
        context: &mut MetricContext,
    ) -> Result<Self, GeoError> {
        Self::prepare_borrowed_coordinate(source, reference, context, None)
    }
    /// Clone/validate a borrowed carrier with governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::from_source_in_context`].
    pub fn from_source_in_context_metered(
        source: &Coord,
        reference: &GeographicReference,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_borrowed_coordinate(source, reference, context, Some(observer))
    }
    fn prepare_borrowed_coordinate(
        source: &Coord,
        reference: &GeographicReference,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        context.begin_integer(1)?;
        let mut progress = WorkProgress::integer(observer);
        progress.initial()?;
        let operands = [Some(source.x()), Some(source.y()), source.z(), source.m()]
            .into_iter()
            .flatten()
            .collect::<purrdf_core::SmallVec<[&Rat; 4]>>();
        let mut retained = 0;
        let result = (|| {
            let source = crate::numerical::ExactAdmission::new(context, &mut progress)
                .rational_owner_counted(
                    purrdf_xsd::integer::ExactOperation::Linear,
                    &operands,
                    operands.len() as u64,
                    &mut retained,
                    || source.clone(),
                )?;
            Self::owned_with_progress(source, reference, context, &mut progress)
        })();
        // The original clone stays admitted during reference-ID/range work.
        // Its successful scalar owner transfers to the caller; a refusal drops
        // it before releasing this invocation's temporary allowance.
        context.release_workspace(retained)?;
        result
    }
    fn with_binding(
        source: Coord,
        reference: &GeographicReference,
        binding: GeoBindingId,
    ) -> Result<Self, GeoError> {
        let (lon, lat) = match reference.axes() {
            AxisOrder::LonLat => (source.x(), source.y()),
            AxisOrder::LatLon => (source.y(), source.x()),
        };
        Ok(Self {
            point: LonLat::new(lon.clone(), lat.clone())?,
            source,
            binding,
        })
    }
    /// Original validated longitude and latitude.
    #[must_use]
    pub const fn point(&self) -> &LonLat {
        &self.point
    }
    /// Original carrier axes, height and measure without relabeling.
    #[must_use]
    pub const fn source(&self) -> &Coord {
        &self.source
    }
    /// Retained object and exact source/normalized-coordinate limb capacities.
    /// This excludes allocator metadata and is independent of admission policy.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> u64 {
        [
            Some(self.source.x()),
            Some(self.source.y()),
            self.source.z(),
            self.source.m(),
            Some(self.point.longitude()),
            Some(self.point.latitude()),
        ]
        .into_iter()
        .flatten()
        .fold(size_of::<Self>() as u64, |bytes, value| {
            bytes
                .saturating_add(value.numerator().allocated_bytes() as u64)
                .saturating_add(value.denominator().allocated_bytes() as u64)
        })
    }
    /// Exact original geographic reference binding.
    #[must_use]
    pub const fn binding_id(&self) -> GeoBindingId {
        self.binding
    }
}

trait SourcePreparation {
    fn source_step(&mut self) -> Result<(), GeoError>;
    fn prepare_coordinate(
        &mut self,
        source: &Coord,
        reference: &GeographicReference,
        binding: GeoBindingId,
    ) -> Result<PreparedCoordinate, GeoError>;
    fn chart_coordinate(&mut self, point: &PreparedCoordinate) -> Result<Coord, GeoError>;
}
struct PureSourcePreparation<'a, F>(&'a mut F);
impl<F: FnMut() -> Result<(), GeoError>> SourcePreparation for PureSourcePreparation<'_, F> {
    fn source_step(&mut self) -> Result<(), GeoError> {
        (self.0)()
    }
    fn prepare_coordinate(
        &mut self,
        source: &Coord,
        reference: &GeographicReference,
        binding: GeoBindingId,
    ) -> Result<PreparedCoordinate, GeoError> {
        PreparedCoordinate::with_binding(source.clone(), reference, binding)
    }
    fn chart_coordinate(&mut self, point: &PreparedCoordinate) -> Result<Coord, GeoError> {
        Ok(Coord::xy(
            point.point.longitude().clone(),
            point.point.latitude().clone(),
        ))
    }
}
/// Written coordinate-linear edge; interpolation preserves the full signed
/// longitude difference, including a written 170 to -170 degree long path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceLinearEdge {
    start: PreparedCoordinate,
    end: PreparedCoordinate,
}
impl SourceLinearEdge {
    /// Recover an exact parameter on the complete written coordinate segment.
    /// This uses original longitude spelling; physical seam aliases are handled
    /// by the atlas before entering this coordinate-law helper.
    #[must_use]
    pub fn parameter_on(start: &LonLat, end: &LonLat, point: &LonLat) -> Option<Rat> {
        Self::parameter_on_inner(start, end, point, None)
            .expect("pure exact parameter has no admission refusal")
    }
    pub(crate) fn parameter_on_admitted(
        start: &LonLat,
        end: &LonLat,
        point: &LonLat,
        admission: &mut crate::numerical::ExactAdmission<'_, '_>,
    ) -> Result<Option<Rat>, GeoError> {
        Self::parameter_on_inner(start, end, point, Some(admission))
    }
    fn parameter_on_inner(
        start: &LonLat,
        end: &LonLat,
        point: &LonLat,
        mut admission: Option<&mut crate::numerical::ExactAdmission<'_, '_>>,
    ) -> Result<Option<Rat>, GeoError> {
        Self::parameter_on_xy_inner(
            [start.longitude(), start.latitude()],
            [end.longitude(), end.latitude()],
            [point.longitude(), point.latitude()],
            &mut admission,
        )
    }
    /// Recover an original planar carrier parameter without interpreting its
    /// units, axes, datum or coordinate range as geographic coordinates.
    pub(crate) fn parameter_on_coord_admitted(
        start: &Coord,
        end: &Coord,
        point: &Coord,
        admission: &mut crate::numerical::ExactAdmission<'_, '_>,
    ) -> Result<Option<Rat>, GeoError> {
        Self::parameter_on_xy_inner(
            [start.x(), start.y()],
            [end.x(), end.y()],
            [point.x(), point.y()],
            &mut Some(admission),
        )
    }
    /// The same original parameter equation with each returned arithmetic
    /// owner retained until its enclosing contact proof drops.
    pub(crate) fn parameter_on_coord_retained(
        start: &Coord,
        end: &Coord,
        point: &Coord,
        admission: &mut crate::numerical::ExactAdmission<'_, '_>,
        retained: &mut u64,
    ) -> Result<Option<Rat>, GeoError> {
        Self::parameter_on_xy_inner(
            [start.x(), start.y()],
            [end.x(), end.y()],
            [point.x(), point.y()],
            &mut RetainedCoordinateArithmetic {
                admission,
                retained,
            },
        )
    }
    fn parameter_on_xy_inner<A: CoordinateArithmetic<Error = GeoError>>(
        start: [&Rat; 2],
        end: [&Rat; 2],
        point: [&Rat; 2],
        arithmetic: &mut A,
    ) -> Result<Option<Rat>, GeoError> {
        use purrdf_xsd::integer::ExactOperation::{
            Linear, RationalAdd, RationalCompare, RationalDivide,
        };
        let equal = |value: [&Rat; 2], arithmetic: &mut A| {
            Ok::<_, GeoError>(
                arithmetic.exact(Linear, &[value[0], point[0]], || value[0] == point[0])?
                    && arithmetic.exact(Linear, &[value[1], point[1]], || value[1] == point[1])?,
            )
        };
        let endpoint = if equal(start, arithmetic)? {
            Some(Rat::zero())
        } else if equal(end, arithmetic)? {
            Some(Rat::one())
        } else {
            None
        };
        if endpoint.is_some() {
            return Ok(endpoint);
        }
        let longitude =
            arithmetic.exact(RationalAdd, &[end[0], start[0]], || end[0].sub(start[0]))?;
        let latitude =
            arithmetic.exact(RationalAdd, &[end[1], start[1]], || end[1].sub(start[1]))?;
        let parameter = if !longitude.is_zero() {
            let delta = arithmetic.exact(RationalAdd, &[point[0], start[0]], || {
                point[0].sub(start[0])
            })?;
            arithmetic.exact(RationalDivide, &[&delta, &longitude], || {
                delta.div(&longitude)
            })?
        } else if !latitude.is_zero() {
            let delta = arithmetic.exact(RationalAdd, &[point[1], start[1]], || {
                point[1].sub(start[1])
            })?;
            arithmetic.exact(RationalDivide, &[&delta, &latitude], || {
                delta.div(&latitude)
            })?
        } else {
            return Ok(None);
        };
        let Some(parameter) = parameter else {
            return Ok(None);
        };
        let at_least_zero =
            arithmetic.exact(RationalCompare, &[&parameter], || Rat::zero() <= parameter)?;
        let at_most_one =
            arithmetic.exact(RationalCompare, &[&parameter], || parameter <= Rat::one())?;
        let in_range = at_least_zero && at_most_one;
        if !in_range {
            return Ok(None);
        }
        let longitude = interpolate_ordinate_with(start[0], end[0], &parameter, arithmetic)?;
        let latitude = interpolate_ordinate_with(start[1], end[1], &parameter, arithmetic)?;
        // The original affine ordinates suffice for exact reconstruction.
        // Geographic endpoints and a unit parameter already prove every range
        // bound; metre carriers require no fabricated LonLat interpretation.
        let equal =
            arithmetic.exact(Linear, &[&longitude, &latitude, point[0], point[1]], || {
                longitude == *point[0] && latitude == *point[1]
            })?;
        Ok(equal.then_some(parameter))
    }
    /// Connect two original validated positions in source coordinate space.
    ///
    /// # Errors
    /// Refuses coordinates from different geographic reference bindings.
    pub fn new(start: PreparedCoordinate, end: PreparedCoordinate) -> Result<Self, GeoError> {
        matching_binding(start.binding, end.binding)?;
        Ok(Self { start, end })
    }
    /// Original edge start and retained carrier ordinates.
    #[must_use]
    pub const fn start(&self) -> &PreparedCoordinate {
        &self.start
    }
    /// Original edge end and retained carrier ordinates.
    #[must_use]
    pub const fn end(&self) -> &PreparedCoordinate {
        &self.end
    }
    /// Signed longitude change in the written source coordinates.
    #[must_use]
    pub fn delta_longitude(&self) -> Rat {
        self.end.point.longitude().sub(self.start.point.longitude())
    }
    /// Signed latitude change in the written source coordinates.
    #[must_use]
    pub fn delta_latitude(&self) -> Rat {
        self.end.point.latitude().sub(self.start.point.latitude())
    }
    /// Evaluate the complete source-linear edge at an exact unit parameter.
    ///
    /// # Errors
    /// Refuses a parameter outside the closed unit interval.
    pub fn at(&self, parameter: &Rat) -> Result<LonLat, GeoError> {
        Self::interpolate(&self.start.point, &self.end.point, parameter)
    }
    /// Evaluate a written source-linear segment without fabricating carrier
    /// coordinates. The signed longitude difference is retained exactly.
    ///
    /// # Errors
    /// Refuses a parameter outside the closed unit interval.
    pub fn interpolate(start: &LonLat, end: &LonLat, parameter: &Rat) -> Result<LonLat, GeoError> {
        if parameter < &Rat::zero() || parameter > &Rat::one() {
            return Err(GeoError::domain("edge parameter outside [0,1]"));
        }
        LonLat::new(
            interpolate_ordinate(start.longitude(), end.longitude(), parameter),
            interpolate_ordinate(start.latitude(), end.latitude(), parameter),
        )
    }
    /// Interpolate the complete written carrier coordinates without applying
    /// geographic range or axis interpretation. Optional ordinates are carried
    /// when both endpoints supply them; no missing height or measure is invented.
    /// The caller owns the parameter's admitted domain.
    #[must_use]
    pub fn interpolate_coord(start: &Coord, end: &Coord, parameter: &Rat) -> Coord {
        interpolate_coord_with(start, end, parameter, &mut None)
            .expect("pure interpolation has no admission refusal")
    }
    pub(crate) fn interpolate_coord_admitted(
        start: &Coord,
        end: &Coord,
        parameter: &Rat,
        admission: &mut crate::numerical::ExactAdmission<'_, '_>,
    ) -> Result<Coord, GeoError> {
        interpolate_coord_with(start, end, parameter, &mut Some(admission))
    }
    pub(crate) fn interpolate_coord_retained(
        start: &Coord,
        end: &Coord,
        parameter: &Rat,
        admission: &mut crate::numerical::ExactAdmission<'_, '_>,
        retained: &mut u64,
    ) -> Result<Coord, GeoError> {
        interpolate_coord_with(
            start,
            end,
            parameter,
            &mut RetainedCoordinateArithmetic {
                admission,
                retained,
            },
        )
    }
    pub(crate) fn interpolate_ordinate_admitted(
        start: &Rat,
        end: &Rat,
        parameter: &Rat,
        admission: &mut crate::numerical::ExactAdmission<'_, '_>,
    ) -> Result<Rat, GeoError> {
        interpolate_ordinate_inner(start, end, parameter, Some(admission))
    }
    pub(crate) fn interpolate_ordinate_retained(
        start: &Rat,
        end: &Rat,
        parameter: &Rat,
        admission: &mut crate::numerical::ExactAdmission<'_, '_>,
        retained: &mut u64,
    ) -> Result<Rat, GeoError> {
        interpolate_ordinate_with(
            start,
            end,
            parameter,
            &mut RetainedCoordinateArithmetic {
                admission,
                retained,
            },
        )
    }
    pub(crate) fn interpolate_ordinate_math(
        start: &Rat,
        end: &Rat,
        parameter: &Rat,
        math: &mut purrdf_xsd::math::CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Rat, purrdf_xsd::math::MathError> {
        interpolate_ordinate_with(
            start,
            end,
            parameter,
            &mut MathCoordinateArithmetic { math, progress },
        )
    }
    pub(crate) fn interpolate_coord_math(
        start: &Coord,
        end: &Coord,
        parameter: &Rat,
        math: &mut purrdf_xsd::math::CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Coord, purrdf_xsd::math::MathError> {
        interpolate_coord_with(
            start,
            end,
            parameter,
            &mut MathCoordinateArithmetic { math, progress },
        )
    }
    /// Certified complete path-length upper bound, in metres, from the inverse
    /// Gauss-map metric bound and the exact source-coordinate variation.
    ///
    /// # Errors
    /// Refuses a reference different from the original source binding.
    pub fn length_upper_bound(&self, reference: &GeographicReference) -> Result<Metres, GeoError> {
        matching_binding(self.start.binding, reference.id())?;
        Ok(Self::upper_bound(
            &self.start.point,
            &self.end.point,
            reference,
        ))
    }
    /// Certified path-length bound for the complete written segment, using the
    /// explicitly supplied ellipsoid. Endpoints have no implicit datum binding.
    #[must_use]
    pub fn upper_bound(start: &LonLat, end: &LonLat, reference: &GeographicReference) -> Metres {
        Self::upper_bound_with(start, end, reference, None)
            .expect("pure path bound has no admission refusal")
    }
    pub(crate) fn upper_bound_with(
        start: &LonLat,
        end: &LonLat,
        reference: &GeographicReference,
        mut admission: Option<&mut crate::numerical::ExactAdmission<'_, '_>>,
    ) -> Result<Metres, GeoError> {
        use purrdf_xsd::integer::ExactOperation::RationalCompare;
        let constant_pole = if let Some(admission) = admission.as_deref_mut() {
            admission.rational(
                RationalCompare,
                &[start.latitude(), end.latitude()],
                3,
                || Ok(start.is_pole() && start.latitude() == end.latitude()),
            )?
        } else {
            start.is_pole() && start.latitude() == end.latitude()
        };
        if constant_pole {
            return Ok(Metres::new(Rat::zero()));
        }
        let variation = Self::variation_with(start, end, admission.as_deref_mut())?;
        let (_, maximum) = reference.ellipsoid().normal_metric_bounds_ref();
        source_path_bound(
            &variation,
            maximum.exact(),
            &mut RationalPathScale { admission },
        )
        .map(Metres::new)
    }

    pub(crate) fn variation_with(
        start: &LonLat,
        end: &LonLat,
        mut admission: Option<&mut crate::numerical::ExactAdmission<'_, '_>>,
    ) -> Result<Rat, GeoError> {
        use crate::numerical::exact_rational;
        use purrdf_xsd::integer::ExactOperation::{Linear, RationalAdd};
        let longitude = exact_rational(
            admission.as_deref_mut(),
            RationalAdd,
            &[end.longitude(), start.longitude()],
            || end.longitude().sub(start.longitude()),
        )?;
        let latitude = exact_rational(
            admission.as_deref_mut(),
            RationalAdd,
            &[end.latitude(), start.latitude()],
            || end.latitude().sub(start.latitude()),
        )?;
        let longitude = exact_rational(admission.as_deref_mut(), Linear, &[&longitude], || {
            longitude.abs()
        })?;
        let latitude = exact_rational(admission.as_deref_mut(), Linear, &[&latitude], || {
            latitude.abs()
        })?;
        exact_rational(admission, RationalAdd, &[&longitude, &latitude], || {
            longitude.add(&latitude)
        })
    }

    /// Enclose the same original path-bound equation without constructing its
    /// compound rational products. Precision changes only this proof enclosure.
    pub(crate) fn enclose_upper_bound(
        variation: &Rat,
        reference: &GeographicReference,
        math: &mut purrdf_xsd::math::CoordinateMath,
    ) -> Result<purrdf_xsd::math::FixedInterval, purrdf_xsd::math::MathError> {
        let (_, maximum) = reference.ellipsoid().normal_metric_bounds_ref();
        source_path_bound(variation, maximum.exact(), &mut IntervalPathScale { math })
    }
}
/// One equation for the exact and outward-enclosure path-bound proofs.
trait PathScaleArithmetic {
    type Value;
    type Error;
    fn source(&mut self, value: &Rat) -> Result<Self::Value, Self::Error>;
    fn multiply(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, Self::Error>;
    fn divide(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, Self::Error>;
}
fn source_path_bound<A: PathScaleArithmetic>(
    variation: &Rat,
    maximum: &Rat,
    arithmetic: &mut A,
) -> Result<A::Value, A::Error> {
    let maximum = arithmetic.source(maximum)?;
    let variation = arithmetic.source(variation)?;
    let product = arithmetic.multiply(&maximum, &variation)?;
    let pi = arithmetic.source(&crate::numerical::frozen_decimal(
        "3.14159265358979323846264338327950288419716939937511",
    ))?;
    let distance = arithmetic.multiply(&product, &pi)?;
    let degrees = arithmetic.source(&Rat::from_i64(180))?;
    arithmetic.divide(&distance, &degrees)
}
struct RationalPathScale<'a, 'context, 'observer> {
    admission: Option<&'a mut crate::numerical::ExactAdmission<'context, 'observer>>,
}
impl PathScaleArithmetic for RationalPathScale<'_, '_, '_> {
    type Value = Rat;
    type Error = GeoError;
    fn source(&mut self, value: &Rat) -> Result<Rat, GeoError> {
        crate::numerical::exact_rational(
            self.admission.as_deref_mut(),
            purrdf_xsd::integer::ExactOperation::Linear,
            &[value],
            || value.clone(),
        )
    }
    fn multiply(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        crate::numerical::exact_rational(
            self.admission.as_deref_mut(),
            purrdf_xsd::integer::ExactOperation::RationalMultiply,
            &[a, b],
            || a.mul(b),
        )
    }
    fn divide(&mut self, a: &Rat, b: &Rat) -> Result<Rat, GeoError> {
        crate::numerical::exact_rational(
            self.admission.as_deref_mut(),
            purrdf_xsd::integer::ExactOperation::RationalDivide,
            &[a, b],
            || a.div(b).expect("positive path-bound divisor"),
        )
    }
}
struct IntervalPathScale<'a> {
    math: &'a mut purrdf_xsd::math::CoordinateMath,
}
impl PathScaleArithmetic for IntervalPathScale<'_> {
    type Value = purrdf_xsd::math::FixedInterval;
    type Error = purrdf_xsd::math::MathError;
    fn source(&mut self, value: &Rat) -> Result<Self::Value, Self::Error> {
        crate::numerical::fixed_from_rat(value, self.math)
    }
    fn multiply(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, Self::Error> {
        a.mul(b, self.math)
    }
    fn divide(&mut self, a: &Self::Value, b: &Self::Value) -> Result<Self::Value, Self::Error> {
        a.div(b, self.math)
    }
}

fn interpolate_ordinate(start: &Rat, end: &Rat, parameter: &Rat) -> Rat {
    interpolate_ordinate_inner(start, end, parameter, None)
        .expect("pure interpolation has no admission refusal")
}
fn interpolate_ordinate_inner(
    start: &Rat,
    end: &Rat,
    parameter: &Rat,
    mut admission: Option<&mut crate::numerical::ExactAdmission<'_, '_>>,
) -> Result<Rat, GeoError> {
    interpolate_ordinate_with(start, end, parameter, &mut admission)
}
trait CoordinateArithmetic {
    type Error;
    fn exact<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        evaluate: impl FnOnce() -> T,
    ) -> Result<T, Self::Error>;
}
impl CoordinateArithmetic for Option<&mut crate::numerical::ExactAdmission<'_, '_>> {
    type Error = GeoError;
    fn exact<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        evaluate: impl FnOnce() -> T,
    ) -> Result<T, Self::Error> {
        crate::numerical::exact_rational(self.as_deref_mut(), operation, operands, evaluate)
    }
}
struct MathCoordinateArithmetic<'a, 'observer> {
    math: &'a mut purrdf_xsd::math::CoordinateMath,
    progress: &'a mut WorkProgress<'observer>,
}
struct RetainedCoordinateArithmetic<'a, 'context, 'observer> {
    admission: &'a mut crate::numerical::ExactAdmission<'context, 'observer>,
    retained: &'a mut u64,
}
impl CoordinateArithmetic for RetainedCoordinateArithmetic<'_, '_, '_> {
    type Error = GeoError;
    fn exact<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        evaluate: impl FnOnce() -> T,
    ) -> Result<T, Self::Error> {
        self.admission
            .rational_owner(operation, operands, self.retained, evaluate)
    }
}
impl CoordinateArithmetic for MathCoordinateArithmetic<'_, '_> {
    type Error = purrdf_xsd::math::MathError;
    fn exact<T>(
        &mut self,
        operation: purrdf_xsd::integer::ExactOperation,
        operands: &[&Rat],
        evaluate: impl FnOnce() -> T,
    ) -> Result<T, Self::Error> {
        crate::numerical::math_exact_rational(
            operation,
            operands,
            self.math,
            self.progress,
            evaluate,
        )
    }
}
fn interpolate_ordinate_with<A: CoordinateArithmetic>(
    start: &Rat,
    end: &Rat,
    parameter: &Rat,
    arithmetic: &mut A,
) -> Result<Rat, A::Error> {
    use purrdf_xsd::integer::ExactOperation::{Linear, RationalAdd, RationalMultiply};
    // Endpoint and constant-ordinate identities preserve the original carrier
    // exactly. Admit their limb comparisons/copy before selecting the shortcut;
    // no product, GCD or denominator growth occurs on these paths.
    let endpoint = arithmetic.exact(Linear, &[start, end, parameter], || {
        if parameter.is_zero() || start == end {
            Some(start.clone())
        } else if parameter == &Rat::one() {
            Some(end.clone())
        } else {
            None
        }
    })?;
    if let Some(endpoint) = endpoint {
        return Ok(endpoint);
    }
    let delta = arithmetic.exact(RationalAdd, &[end, start], || end.sub(start))?;
    let part = arithmetic.exact(RationalMultiply, &[&delta, parameter], || {
        delta.mul(parameter)
    })?;
    arithmetic.exact(RationalAdd, &[start, &part], || start.add(&part))
}
fn interpolate_coord_with<A: CoordinateArithmetic>(
    start: &Coord,
    end: &Coord,
    parameter: &Rat,
    arithmetic: &mut A,
) -> Result<Coord, A::Error> {
    let x = interpolate_ordinate_with(start.x(), end.x(), parameter, arithmetic)?;
    let y = interpolate_ordinate_with(start.y(), end.y(), parameter, arithmetic)?;
    let mut optional = |a: Option<&Rat>, b: Option<&Rat>| {
        a.zip(b)
            .map(|(a, b)| interpolate_ordinate_with(a, b, parameter, arithmetic))
            .transpose()
    };
    let z = optional(start.z(), end.z())?;
    let m = optional(start.m(), end.m())?;
    Ok(Coord::new(x, y, z, m))
}

/// Explicit azimuth/length geodesic law inputs. The direct endpoint is separately
/// certified materialization evidence and is never the mathematical arc endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AzimuthLengthArc {
    start: PreparedCoordinate,
    azimuth: Rat,
    length: Metres,
    endpoint: DirectResult,
}
impl AzimuthLengthArc {
    /// Prepare the explicitly selected branch and certify its materialized end.
    ///
    /// # Errors
    /// Propagates the direct solver's reference, domain and operational refusals.
    pub fn new(
        start: PreparedCoordinate,
        azimuth: Rat,
        length: Metres,
        context: &mut MetricContext,
    ) -> Result<Self, GeoError> {
        geodesic_entry(
            context,
            None,
            &[start.binding],
            |_, _, _| Ok(()),
            |(), geodesic, child, observer| {
                Self::solve(start, azimuth, length, geodesic, child, observer)
            },
        )
    }
    /// Clone exact branch parameters only after their complete limb-work and
    /// scratch admission. The direct endpoint uses the same original solver.
    ///
    /// # Errors
    /// Propagates branch/reference failures and refuses resource exhaustion
    /// before copying or evaluating the exact parameters.
    pub fn from_source_in_context(
        start: PreparedCoordinate,
        azimuth: &Rat,
        length: &Metres,
        context: &mut MetricContext,
    ) -> Result<Self, GeoError> {
        geodesic_entry(
            context,
            None,
            &[start.binding],
            |context, progress, retained| {
                let mut admission = crate::numerical::ExactAdmission::new(context, progress);
                let (azimuth, _) = admission.rational_retained(
                    purrdf_xsd::integer::ExactOperation::Linear,
                    &[azimuth],
                    retained,
                    || azimuth.clone(),
                )?;
                let (length, _) = admission.rational_retained(
                    purrdf_xsd::integer::ExactOperation::Linear,
                    &[length.exact()],
                    retained,
                    || length.exact().clone(),
                )?;
                Ok((azimuth, Metres::new(length)))
            },
            |(azimuth, length), geodesic, child, observer| {
                Self::solve(start, azimuth, length, geodesic, child, observer)
            },
        )
    }
    fn solve(
        start: PreparedCoordinate,
        azimuth: Rat,
        length: Metres,
        geodesic: &PreparedGeodesic,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        let endpoint =
            geodesic.direct_metered(start.point(), &azimuth, &length, context, observer)?;
        Ok(Self {
            start,
            azimuth,
            length,
            endpoint,
        })
    }
    /// Exact original start.
    #[must_use]
    pub const fn start(&self) -> &PreparedCoordinate {
        &self.start
    }
    /// Exact clockwise-from-north law input, before modular reduction.
    #[must_use]
    pub const fn azimuth(&self) -> &Rat {
        &self.azimuth
    }
    /// Exact selected branch length in metres.
    #[must_use]
    pub const fn length(&self) -> &Metres {
        &self.length
    }
    /// Separate fixed-grid endpoint materialization and certificate.
    #[must_use]
    pub const fn materialized_endpoint(&self) -> &DirectResult {
        &self.endpoint
    }
    /// Materialize an exact arc parameter without changing its law inputs.
    ///
    /// # Errors
    /// Refuses a parameter outside `[0,1]` or the direct solver's operational failure.
    pub fn at(
        &self,
        parameter: &Rat,
        context: &mut MetricContext,
    ) -> Result<DirectResult, GeoError> {
        geodesic_entry(
            context,
            None,
            &[self.start.binding],
            |context, progress, retained| {
                let mut admission = crate::numerical::ExactAdmission::new(context, progress);
                if admission.compare(parameter, &Rat::zero())?.is_lt()
                    || admission.compare(parameter, &Rat::one())?.is_gt()
                {
                    return Err(GeoError::domain("arc parameter outside [0,1]"));
                }
                admission
                    .rational_retained(
                        purrdf_xsd::integer::ExactOperation::RationalMultiply,
                        &[self.length.exact(), parameter],
                        retained,
                        || self.length.exact().mul(parameter),
                    )
                    .map(|(length, _)| Metres::new(length))
            },
            |length, geodesic, child, observer| {
                geodesic.direct_metered(self.start.point(), &self.azimuth, &length, child, observer)
            },
        )
    }
}

/// The unique shortest geodesic defined by two exact original endpoints.
/// Quantized inverse metadata is evidence and never replaces those endpoints
/// with an approximate azimuth/length law.
#[derive(Clone, Debug)]
pub struct ShortestGeodesicArc {
    start: PreparedCoordinate,
    end: PreparedCoordinate,
    inverse: InverseResult,
    proof: InverseProofReceipt,
}
impl PartialEq for ShortestGeodesicArc {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}
impl Eq for ShortestGeodesicArc {}
impl ShortestGeodesicArc {
    /// Certify that the endpoints select one shortest branch.
    ///
    /// # Errors
    /// Refuses reference mismatch, multiple shortest branches, and the inverse
    /// solver's numerical or operational failures. Unresolved multiplicity is
    /// a precision failure, not a successful approximate branch selection.
    pub fn new(
        start: PreparedCoordinate,
        end: PreparedCoordinate,
        context: &mut MetricContext,
    ) -> Result<Self, GeoError> {
        Self::prepare(start, end, context, None)
    }
    /// Certify an endpoint arc while charging bounded numerical chunks.
    ///
    /// # Errors
    /// Adds the observer's typed refusal to [`Self::new`].
    pub fn new_metered(
        start: PreparedCoordinate,
        end: PreparedCoordinate,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare(start, end, context, Some(observer))
    }
    fn prepare(
        start: PreparedCoordinate,
        end: PreparedCoordinate,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        geodesic_entry(
            context,
            observer,
            &[start.binding, end.binding],
            |_, _, _| Ok(()),
            |(), geodesic, child, observer| {
                let (inverse, proof) = geodesic.inverse_with_proof_metered(
                    start.point(),
                    end.point(),
                    child,
                    observer,
                )?;
                if inverse.branch_multiplicity() != ShortestBranchMultiplicity::Unique {
                    return Err(GeoError::AmbiguousGeodesic);
                }
                Ok(Self {
                    start,
                    end,
                    inverse,
                    proof,
                })
            },
        )
    }
    /// Exact original start, including retained carrier ordinates.
    #[must_use]
    pub const fn start(&self) -> &PreparedCoordinate {
        &self.start
    }
    /// Exact original end, including retained carrier ordinates.
    #[must_use]
    pub const fn end(&self) -> &PreparedCoordinate {
        &self.end
    }
    /// Certified metadata for the unique shortest branch. Rounded azimuth and
    /// length fields are response values rather than mathematical arc inputs.
    #[must_use]
    pub const fn inverse(&self) -> &InverseResult {
        &self.inverse
    }
    /// Invocation proof enclosures, excluded from mathematical arc equality.
    #[must_use]
    pub const fn proof(&self) -> &InverseProofReceipt {
        &self.proof
    }
    /// Materialize an exact parameter of the endpoint-defined shortest arc.
    ///
    /// # Errors
    /// Refuses reference mismatch, parameters outside `[0,1]`, or incomplete
    /// certified propagation and fixed-grid endpoint rounding.
    pub fn at(
        &self,
        parameter: &Rat,
        context: &mut MetricContext,
    ) -> Result<DirectResult, GeoError> {
        self.propagate(parameter, context, None)
    }
    /// Materialize an endpoint-defined arc with bounded governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::at`].
    pub fn at_metered(
        &self,
        parameter: &Rat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<DirectResult, GeoError> {
        self.propagate(parameter, context, Some(observer))
    }

    fn propagate(
        &self,
        parameter: &Rat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<DirectResult, GeoError> {
        geodesic_entry(
            context,
            observer,
            &[self.start.binding, self.end.binding],
            |_, _, _| Ok(()),
            |(), geodesic, child, observer| {
                geodesic.shortest_arc_at_metered(
                    self.start.point(),
                    self.end.point(),
                    parameter,
                    child,
                    observer,
                )
            },
        )
    }
}

/// One prepared curve edge with an explicit mathematical interpolation law.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparedEdge {
    /// Interpolate the original written geographic source coordinates.
    SourceLinear(Box<SourceLinearEdge>),
    /// Propagate exact original azimuth and length, retaining its branch.
    AzimuthLength(Box<AzimuthLengthArc>),
    /// Follow the unique shortest branch selected by exact original endpoints.
    ShortestGeodesic(Box<ShortestGeodesicArc>),
    /// Complete image of an exact written source edge through a declared chain.
    /// Its target positions remain symbolic; export chords are separate evidence.
    Transformed(Box<crate::operation::OperationImageCurve>),
}
impl PreparedEdge {
    /// Clone the complete original owner after its actual integer copies and
    /// containers are admitted. The caller drops the cloned edge before
    /// releasing the accumulated retained allowance.
    pub(crate) fn clone_admitted(
        &self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<Self, GeoError> {
        use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
        context.charge_work(1)?;
        progress.context_poll(context)?;
        let container = size_of::<Self>()
            + match self {
                Self::SourceLinear(_) => size_of::<SourceLinearEdge>(),
                Self::AzimuthLength(_) => size_of::<AzimuthLengthArc>(),
                Self::ShortestGeodesic(arc) => size_of::<ShortestGeodesicArc>()
                    .checked_add(
                        arc.inverse()
                            .pole_cuts()
                            .len()
                            .checked_mul(size_of::<(Rat, Rat, Rat)>())
                            .ok_or(GeoError::ArithmeticOverflow("edge clone pole storage"))?,
                    )
                    .ok_or(GeoError::ArithmeticOverflow("edge clone storage"))?,
                Self::Transformed(_) => size_of::<crate::operation::OperationImageCurve>(),
            };
        context.retain_workspace(container as u64, retained)?;
        self.visit_owned_integers(|integer| {
            let cost =
                ExactArithmeticCost::for_operation(ExactOperation::Linear, integer.bit_len(), 1)
                    .ok_or(GeoError::ArithmeticOverflow("edge clone integer admission"))?;
            context.retain_workspace(integer.allocated_bytes() as u64, retained)?;
            progress.exact(context, cost, || Ok(()))
        })?;
        progress.context_poll(context)?;
        Ok(self.clone())
    }

    /// The copied magnitudes have one inventory home. Shared source/mapping
    /// Arcs are retained by handle, while normalized positions and proof/result
    /// fields keep their independently owned integer capacity evidence.
    fn visit_owned_integers(
        &self,
        mut visit: impl FnMut(&crate::Int) -> Result<(), GeoError>,
    ) -> Result<(), GeoError> {
        let coordinate =
            |point: &PreparedCoordinate,
             visit: &mut dyn FnMut(&crate::Int) -> Result<(), GeoError>| {
                for value in [
                    Some(point.source.x()),
                    Some(point.source.y()),
                    point.source.z(),
                    point.source.m(),
                    Some(point.point.longitude()),
                    Some(point.point.latitude()),
                ]
                .into_iter()
                .flatten()
                .flat_map(Rat::integer_operands)
                {
                    visit(value)?;
                }
                Ok(())
            };
        match self {
            Self::SourceLinear(edge) => {
                coordinate(edge.start(), &mut visit)?;
                coordinate(edge.end(), &mut visit)?;
            }
            Self::AzimuthLength(arc) => {
                coordinate(arc.start(), &mut visit)?;
                for value in [
                    arc.azimuth(),
                    arc.length().exact(),
                    arc.endpoint.endpoint().longitude(),
                    arc.endpoint.endpoint().latitude(),
                    arc.endpoint.final_azimuth(),
                ]
                .into_iter()
                .flat_map(Rat::integer_operands)
                {
                    visit(value)?;
                }
            }
            Self::ShortestGeodesic(arc) => {
                coordinate(arc.start(), &mut visit)?;
                coordinate(arc.end(), &mut visit)?;
                for value in arc
                    .inverse()
                    .owned_integer_operands()
                    .chain(arc.proof().operands().flat_map(Rat::integer_operands))
                {
                    visit(value)?;
                }
            }
            Self::Transformed(image) => {
                let ellipsoid = image.reference().ellipsoid();
                let (minimum, maximum) = ellipsoid.normal_metric_bounds_ref();
                for value in [
                    Some(ellipsoid.semimajor()),
                    Some(ellipsoid.semiminor()),
                    Some(ellipsoid.inverse_flattening()),
                    Some(minimum.exact()),
                    Some(maximum.exact()),
                    image.exact_epoch(),
                ]
                .into_iter()
                .flatten()
                .flat_map(Rat::integer_operands)
                {
                    visit(value)?;
                }
            }
        }
        Ok(())
    }
    /// Exact geographic source start when the law has a geographic source.
    /// A transformed edge retains its source/chain endpoint identity instead.
    #[must_use]
    pub fn start(&self) -> Option<&PreparedCoordinate> {
        match self {
            Self::SourceLinear(edge) => Some(edge.start()),
            Self::AzimuthLength(arc) => Some(arc.start()),
            Self::ShortestGeodesic(arc) => Some(arc.start()),
            Self::Transformed(_) => None,
        }
    }
    /// Original source ordinates, including retained Z/M. For transformed
    /// sources these are not relabeled as target geographic coordinates.
    #[must_use]
    pub fn source_start(&self) -> &Coord {
        match self {
            Self::Transformed(curve) => curve.source_endpoints().0,
            _ => self.start().expect("geographic source variant").source(),
        }
    }
    pub(crate) fn original_coordinates(&self) -> impl Iterator<Item = &Coord> {
        let end = match self {
            Self::SourceLinear(edge) => Some(edge.end().source()),
            Self::ShortestGeodesic(arc) => Some(arc.end().source()),
            Self::Transformed(curve) => Some(curve.source_endpoints().1),
            Self::AzimuthLength(_) => None,
        };
        core::iter::once(self.source_start()).chain(end)
    }
    /// Largest original limb width needed by a complete curve image, including
    /// retained ordinates and operation parameters. Reading metadata allocates
    /// no coordinates and never substitutes a rounded output grid.
    pub(crate) fn max_original_operand_bits(&self) -> u64 {
        let coordinates = self
            .original_coordinates()
            .flat_map(|point| {
                [Some(point.x()), Some(point.y()), point.z(), point.m()]
                    .into_iter()
                    .flatten()
            })
            .map(crate::numerical::rational_operand_bits)
            .max()
            .unwrap_or(0);
        match self {
            Self::AzimuthLength(arc) => coordinates
                .max(crate::numerical::rational_operand_bits(arc.azimuth()))
                .max(crate::numerical::rational_operand_bits(
                    arc.length().exact(),
                )),
            Self::Transformed(curve) => coordinates.max(curve.max_original_operand_bits()),
            Self::SourceLinear(_) | Self::ShortestGeodesic(_) => coordinates,
        }
    }
    /// Actual geographic target binding used by metrics, topology and covers.
    #[must_use]
    pub fn binding_id(&self) -> GeoBindingId {
        match self {
            Self::Transformed(curve) => curve.reference().id(),
            _ => self
                .start()
                .expect("geographic source variant")
                .binding_id(),
        }
    }
    /// Prepare the complete unrounded image of a selected geodesic edge.
    /// Coordinate-linear and transformed edges retain their own complete image
    /// laws and return `None`. Quantized endpoints are never line inputs.
    ///
    /// # Errors
    /// Refuses a mismatched reference, unresolved unique branch proof, or
    /// incomplete numerical admission.
    pub fn geodesic_view(
        &self,
        minimum_bits: u32,
        context: &mut MetricContext,
    ) -> Result<Option<ArcIntervalView>, GeoError> {
        self.prepare_geodesic_view(minimum_bits, context, None)
    }
    /// Prepare an unrounded selected edge with bounded governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::geodesic_view`].
    pub fn geodesic_view_metered(
        &self,
        minimum_bits: u32,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Option<ArcIntervalView>, GeoError> {
        self.prepare_geodesic_view(minimum_bits, context, Some(observer))
    }
    fn prepare_geodesic_view(
        &self,
        minimum_bits: u32,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Option<ArcIntervalView>, GeoError> {
        match self {
            Self::SourceLinear(_) | Self::Transformed(_) => {
                context.begin(1)?;
                let mut progress = WorkProgress::new(observer);
                progress.initial()?;
                let source = if let Self::Transformed(curve) = self {
                    crate::numerical::reference_identity(
                        context,
                        &mut progress,
                        Some(curve.reference()),
                    )?
                } else {
                    self.binding_id()
                };
                let target = crate::numerical::reference_identity(context, &mut progress, None)?;
                matching_binding(source, target)?;
                Ok(None)
            }
            Self::AzimuthLength(arc) => geodesic_entry(
                context,
                observer,
                &[arc.start().binding_id()],
                |_, _, _| Ok(()),
                |(), prepared, child, observer| {
                    child.begin(1)?;
                    let mut progress = WorkProgress::new(Some(observer));
                    let cost = crate::numerical::rational_cost(
                        purrdf_xsd::integer::ExactOperation::RationalDivide,
                        &[
                            arc.start().point().longitude(),
                            arc.start().point().latitude(),
                            arc.azimuth(),
                            arc.length().exact(),
                        ],
                        8,
                    )
                    .ok_or(GeoError::ArithmeticOverflow("azimuth view admission"))?;
                    progress.exact(child, cost, || {
                        prepared
                            .prepare_azimuth_arc_view(
                                arc.start().point(),
                                arc.azimuth(),
                                arc.length(),
                            )
                            .map(Some)
                    })
                },
            ),
            Self::ShortestGeodesic(arc) => geodesic_entry(
                context,
                observer,
                &[arc.start().binding_id(), arc.end().binding_id()],
                |_, _, _| Ok(()),
                |(), prepared, child, observer| {
                    prepared
                        .prepare_shortest_arc_view_with_proof(
                            arc,
                            minimum_bits,
                            child,
                            Some(observer),
                        )
                        .map(Some)
                },
            ),
        }
    }
}

/// An immutable run of explicitly modeled edges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedCurve {
    edges: Arc<[PreparedEdge]>,
}
impl PreparedCurve {
    /// Retain ordered explicitly modeled edges. No shorter path is substituted.
    #[must_use]
    pub fn new(edges: Vec<PreparedEdge>) -> Self {
        Self {
            edges: edges.into(),
        }
    }
    /// Edges in source/branch order.
    #[must_use]
    pub fn edges(&self) -> &[PreparedEdge] {
        &self.edges
    }
    /// Prepare a coordinate-linear source curve, preserving Z/M and source axes.
    ///
    /// # Errors
    /// Refuses invalid original geographic coordinates.
    pub fn from_source(
        coordinates: &[Coord],
        reference: &GeographicReference,
    ) -> Result<Self, GeoError> {
        Self::from_source_with_work(coordinates, reference, &mut || Ok(()))
    }
    fn from_source_with_work(
        coordinates: &[Coord],
        reference: &GeographicReference,
        work: &mut impl FnMut() -> Result<(), GeoError>,
    ) -> Result<Self, GeoError> {
        Self::from_source_with_binding(
            coordinates,
            reference,
            reference.id(),
            &mut PureSourcePreparation(work),
        )
    }
    fn from_source_with_binding(
        coordinates: &[Coord],
        reference: &GeographicReference,
        binding: GeoBindingId,
        preparation: &mut impl SourcePreparation,
    ) -> Result<Self, GeoError> {
        let mut edges = Vec::with_capacity(coordinates.len().saturating_sub(1));
        for pair in coordinates.windows(2) {
            preparation.source_step()?;
            edges.push(PreparedEdge::SourceLinear(Box::new(SourceLinearEdge::new(
                preparation.prepare_coordinate(&pair[0], reference, binding)?,
                preparation.prepare_coordinate(&pair[1], reference, binding)?,
            )?)));
        }
        Ok(Self::new(edges))
    }
}

/// Explicit side of a written carrier region in the identified longitude atlas.
/// Reversing a ring never silently substitutes its smaller physical interior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegionInterior {
    /// Bounded written chart interior, with its holes removed.
    Written,
    /// Closed complementary side of the complete written region.
    Complement,
}

/// Explicit side of a certified native oriented ring intersection.
/// Exterior and hole traversals retain their declared orientation: the Left
/// base intersects the left sides of every supplied Jordan ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrientedInterior {
    /// Intersection of the selected Left sides, including their boundaries.
    Left,
    /// Closed complement of that complete intersection.
    Right,
}

/// Original areal rings, retaining written or explicitly oriented interiors.
#[derive(Clone, Debug)]
pub struct PreparedPolygon {
    rings: Arc<[PreparedCurve]>,
    chart: Option<crate::Geometry>,
    oriented: Option<OrientedInterior>,
    // A certified rank-lost closed areal image retains its complete selected
    // point/curve support without inventing an open face or explicit traversal.
    closed_support: bool,
    interior: RegionInterior,
    binding: GeoBindingId,
    coordinates: u64,
    coordinate_bits: u64,
    // A whole-cell operation proof may enclose the complete selected Written
    // image in one strict canonical, nonpolar chart. Proof tightness is neither
    // geometry semantics nor a source vertex and is excluded from identity.
    angular_bounds: Option<CertifiedAngularBounds>,
}

#[derive(Clone, Debug)]
struct CertifiedAngularBounds {
    walls: Arc<[Rat; 4]>,
    inner: Option<Arc<[Rat; 4]>>,
    retained_bytes: u64,
}

impl PartialEq for PreparedPolygon {
    fn eq(&self, other: &Self) -> bool {
        self.rings == other.rings
            && self.chart == other.chart
            && self.oriented == other.oriented
            && self.closed_support == other.closed_support
            && self.interior == other.interior
            && self.binding == other.binding
            && self.coordinates == other.coordinates
            && self.coordinate_bits == other.coordinate_bits
    }
}
impl Eq for PreparedPolygon {}
impl PreparedPolygon {
    /// Prepare written exterior and holes with an explicitly selected interior.
    ///
    /// # Errors
    /// Refuses malformed rings or invalid original angular coordinates.
    pub fn from_source(
        rings: &crate::Rings,
        reference: &GeographicReference,
        interior: RegionInterior,
    ) -> Result<Self, GeoError> {
        Self::from_source_with_work(rings, reference, interior, &mut || Ok(()))
    }
    fn from_source_with_work(
        rings: &crate::Rings,
        reference: &GeographicReference,
        interior: RegionInterior,
        work: &mut impl FnMut() -> Result<(), GeoError>,
    ) -> Result<Self, GeoError> {
        Self::from_source_with_binding(
            rings,
            reference,
            reference.id(),
            interior,
            &mut PureSourcePreparation(work),
        )
    }
    fn from_source_with_binding(
        rings: &crate::Rings,
        reference: &GeographicReference,
        binding: GeoBindingId,
        interior: RegionInterior,
        preparation: &mut impl SourcePreparation,
    ) -> Result<Self, GeoError> {
        let mut prepared = Vec::with_capacity(rings.len());
        let mut charts = Vec::with_capacity(rings.len());
        let mut coordinates = 0_u64;
        let mut coordinate_bits = 0_u64;
        for ring in rings {
            preparation.source_step()?;
            prepared.push(PreparedCurve::from_source_with_binding(
                ring,
                reference,
                binding,
                preparation,
            )?);
            let mut chart = Vec::with_capacity(ring.len());
            for coordinate in ring {
                preparation.source_step()?;
                coordinates = coordinates.saturating_add(1);
                coordinate_bits = coordinate_bits.saturating_add(coordinate_bit_count(coordinate));
                let point = preparation.prepare_coordinate(coordinate, reference, binding)?;
                chart.push(preparation.chart_coordinate(&point)?);
            }
            charts.push(chart);
        }
        let chart = crate::Geometry::new(crate::CoordDim::Xy, GeometryBody::Polygon(charts))?;
        Ok(Self {
            rings: prepared.into(),
            chart: Some(chart),
            oriented: None,
            closed_support: false,
            interior,
            binding,
            coordinates,
            coordinate_bits,
            angular_bounds: None,
        })
    }
    /// Prepare original native Jordan rings with an explicit oriented side.
    /// Each ring is certified before publication; source-linear and selected
    /// geodesic laws retain their exact original endpoints and branches.
    ///
    /// # Errors
    /// Refuses a reference mismatch, non-Jordan ring, uncertain closure or
    /// incomplete work, memory, precision or cancellation admission.
    pub fn from_curves(
        rings: Vec<PreparedCurve>,
        interior: OrientedInterior,
        context: &mut MetricContext,
    ) -> Result<Self, GeoError> {
        Self::prepare_curves(rings, interior, context, None)
    }
    /// Prepare native rings with bounded external governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::from_curves`].
    pub fn from_curves_metered(
        rings: Vec<PreparedCurve>,
        interior: OrientedInterior,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_curves(rings, interior, context, Some(observer))
    }
    fn prepare_curves(
        rings: Vec<PreparedCurve>,
        interior: OrientedInterior,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        context.begin(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.initial()?;
        let bytes = (rings.capacity() as u64)
            .saturating_mul(size_of::<PreparedCurve>() as u64)
            .saturating_add(crate::atlas::arcs::source_workspace_bound(
                rings.iter().flat_map(PreparedCurve::edges),
            ));
        context.admit_workspace(bytes)?;
        let result = (|| {
            for ring in &rings {
                crate::atlas::oriented::validate_ring(ring, context, &mut progress)?;
            }
            Self::from_validated_curves(rings, interior, context.reference())
        })();
        context.release_workspace(bytes)?;
        progress.context_poll(context)?;
        result
    }
    /// Publish already certified immutable rings under the same reference.
    /// The private ring witness carries the Jordan proof; no numerical samples
    /// or quantized output endpoints are admitted as a replacement.
    ///
    /// # Errors
    /// Refuses empty ring lists and mismatched reference bindings.
    pub fn from_oriented_rings(
        rings: Vec<crate::atlas::oriented::OrientedRing>,
        interior: OrientedInterior,
        reference: &GeographicReference,
    ) -> Result<Self, GeoError> {
        Self::from_validated_curves(
            rings
                .into_iter()
                .map(crate::atlas::oriented::OrientedRing::into_curve)
                .collect(),
            interior,
            reference,
        )
    }
    fn from_validated_curves(
        rings: Vec<PreparedCurve>,
        interior: OrientedInterior,
        reference: &GeographicReference,
    ) -> Result<Self, GeoError> {
        Self::from_native_support(rings, Some(interior), false, reference)
    }
    /// Retain a proved lower-dimensional closed image: an original areal
    /// boundary, or a complete source-cell image whose operation proof
    /// established rank loss. This component has no open areal face. The
    /// proof-producing caller admits the original source and curve owners.
    pub(crate) fn from_selected_support(
        rings: Vec<PreparedCurve>,
        reference: &GeographicReference,
    ) -> Result<Self, GeoError> {
        Self::from_native_support(rings, None, true, reference)
    }
    fn from_native_support(
        rings: Vec<PreparedCurve>,
        oriented: Option<OrientedInterior>,
        closed_support: bool,
        reference: &GeographicReference,
    ) -> Result<Self, GeoError> {
        if rings.is_empty() {
            return Err(GeoError::domain("empty native oriented polygon"));
        }
        let binding = reference.id();
        let mut coordinates = 0_u64;
        let mut coordinate_bits = 0_u64;
        for edge in rings.iter().flat_map(PreparedCurve::edges) {
            matching_binding(edge.binding_id(), binding)?;
            for coordinate in edge.original_coordinates() {
                coordinates = coordinates
                    .checked_add(1)
                    .ok_or(GeoError::ArithmeticOverflow(
                        "native oriented coordinate count",
                    ))?;
                coordinate_bits = coordinate_bits
                    .checked_add(coordinate_bit_count(coordinate))
                    .ok_or(GeoError::ArithmeticOverflow(
                        "native oriented coordinate bits",
                    ))?;
            }
        }
        Ok(Self {
            rings: rings.into(),
            chart: None,
            oriented,
            closed_support,
            interior: RegionInterior::Written,
            binding,
            coordinates,
            coordinate_bits,
            angular_bounds: None,
        })
    }

    /// Attach a complete selected-image enclosure proved by the operation-cell
    /// home. Its producer admits shared Rat owners, inline walls and the Arc
    /// wrapper before this immutable publication, retaining them until the
    /// prepared inventory takes over. Walls are west, south, east, north and
    /// strictly inside the canonical nonpolar chart; this never proves Inside.
    /// A separable, strictly monotone operation on a complete original
    /// rectangle may also prove a contained inner rectangle. Only strict
    /// inner inclusion decides Interior; each possible boundary still uses
    /// the original ring predicate. Both immutable owners are preadmitted.
    pub(crate) fn with_certified_angular_rectangle(
        mut self,
        walls: [Rat; 4],
        inner: Option<[Rat; 4]>,
        retained_bytes: u64,
    ) -> Self {
        self.angular_bounds = Some(CertifiedAngularBounds {
            walls: Arc::new(walls),
            inner: inner.map(Arc::new),
            retained_bytes,
        });
        self
    }

    pub(crate) fn certified_angular_bounds(&self) -> Option<(&[Rat; 4], Option<&[Rat; 4]>)> {
        self.angular_bounds
            .as_ref()
            .map(|bounds| (bounds.walls.as_ref(), bounds.inner.as_deref()))
    }
    /// Original rings followed by holes, preserving orientation and ring order.
    #[must_use]
    pub fn rings(&self) -> &[PreparedCurve] {
        &self.rings
    }
    /// Immutable native oriented base, absent for written carrier regions.
    #[must_use]
    pub const fn oriented_interior(&self) -> Option<OrientedInterior> {
        self.oriented
    }
    /// Whether an original areal image consists only of closed curve/point
    /// support, with no areal interior and no separate traversal multiplicity.
    pub(crate) const fn closed_support(&self) -> bool {
        self.closed_support
    }
    /// Explicit written or complementary physical interior.
    #[must_use]
    pub const fn interior(&self) -> RegionInterior {
        self.interior
    }
    /// Select the written or complementary side without reparsing or relabeling
    /// the immutable original rings.
    #[must_use]
    pub fn with_interior(mut self, interior: RegionInterior) -> Self {
        self.interior = interior;
        self
    }
    /// Original reference binding; no implicit datum equivalence is installed.
    #[must_use]
    pub const fn binding_id(&self) -> GeoBindingId {
        self.binding
    }
    /// Original areal vertex count, including each ring's written closure.
    #[must_use]
    pub const fn coordinate_count(&self) -> u64 {
        self.coordinates
    }
    pub(crate) const fn coordinate_bits(&self) -> u64 {
        self.coordinate_bits
    }
    /// Signed exact planar area of each written ring in the source-longitude
    /// chart. Its sign records orientation without choosing an interior side.
    #[must_use]
    pub fn signed_chart_ring_areas(&self) -> Option<Vec<Rat>> {
        match self.chart.as_ref()?.body() {
            GeometryBody::Polygon(rings) => rings
                .iter()
                .map(|ring| crate::measure::signed_ring_area(ring))
                .collect::<Vec<_>>()
                .into(),
            _ => unreachable!("prepared polygon chart"),
        }
    }
    /// Exact written source-longitude chart, absent for native curved regions.
    #[must_use]
    pub const fn chart(&self) -> Option<&crate::Geometry> {
        self.chart.as_ref()
    }
}
fn coordinate_bit_count(coordinate: &Coord) -> u64 {
    [coordinate.x(), coordinate.y()]
        .into_iter()
        .chain(coordinate.z())
        .chain(coordinate.m())
        .fold(0_u64, |sum, value| {
            sum.saturating_add(value.numerator().bit_len())
                .saturating_add(value.denominator().bit_len())
        })
}
fn matching_binding(source: GeoBindingId, target: GeoBindingId) -> Result<(), GeoError> {
    if source != target {
        return Err(GeoError::MissingOperation {
            source: source.digest().to_string(),
            target: target.digest().to_string(),
        });
    }
    Ok(())
}

/// One cumulative entry for endpoint and explicitly selected geodesic arcs.
/// The numerical child may reset its own invocation, while original identity
/// work and retained argument temporaries remain in the parent admission.
fn geodesic_entry<A, T>(
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
    bindings: &[GeoBindingId],
    prepare: impl FnOnce(&mut MetricContext, &mut WorkProgress<'_>, &mut u64) -> Result<A, GeoError>,
    solve: impl FnOnce(
        A,
        &PreparedGeodesic,
        &mut MetricContext,
        &mut dyn MetricWorkObserver,
    ) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    context.begin(1)?;
    let mut progress = WorkProgress::new(observer);
    progress.initial()?;
    let binding = crate::numerical::reference_identity(context, &mut progress, None)?;
    for &source in bindings {
        matching_binding(source, binding)?;
    }
    let mut retained = 0;
    let result = (|| {
        let arguments = prepare(context, &mut progress, &mut retained)?;
        let reference = crate::numerical::reference_clone(context, &mut progress)?;
        let prepared = PreparedGeodesic::new(reference);
        let mut child = context.remaining_child()?;
        let result = {
            let mut nested = progress.nested(
                context.work_items(),
                context.current_workspace_bytes(),
                context.workspace_peak(),
            );
            solve(arguments, &prepared, &mut child, &mut nested)
        };
        progress.absorb_child_result(context, &child, result)
    })();
    // Argument temporaries remain admitted throughout the complete child solve.
    // On success their ownership may transfer into the scalar returned object;
    // on refusal the solve closure drops them before releasing the allowance.
    context.release_workspace(retained)?;
    result
}

/// An explicitly selected closed region, including empty and the whole surface.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PreparedRegion {
    /// Empty region; no polygon interior is invented for line inputs.
    Empty,
    /// Entire identified geographic surface.
    Whole,
    /// Union of explicitly sided written polygons, including their holes.
    Polygons(Arc<[PreparedPolygon]>),
    /// Closed complement of the complete polygon union. This is distinct from
    /// taking the union of each individual polygon's complement.
    ComplementOfPolygons(Arc<[PreparedPolygon]>),
}
impl PreparedRegion {
    /// Retain explicitly sided polygons as a region union.
    #[must_use]
    pub fn polygons(polygons: Vec<PreparedPolygon>) -> Self {
        if polygons.is_empty() {
            Self::Empty
        } else {
            Self::Polygons(polygons.into())
        }
    }
    /// Toggle the complete region's selected side without recursive ownership.
    #[must_use]
    pub fn complement(self) -> Self {
        match self {
            Self::Empty => Self::Whole,
            Self::Whole => Self::Empty,
            Self::Polygons(polygons) => {
                if polygons.is_empty() {
                    Self::Whole
                } else {
                    Self::ComplementOfPolygons(polygons)
                }
            }
            Self::ComplementOfPolygons(polygons) => {
                if polygons.is_empty() {
                    Self::Empty
                } else {
                    Self::Polygons(polygons)
                }
            }
        }
    }
    pub(crate) fn check_reference_with(
        &self,
        reference: &GeographicReference,
        work: &mut impl FnMut(u64) -> Result<(), GeoError>,
    ) -> Result<(), GeoError> {
        self.check_binding_with(reference.id(), work)
    }

    pub(crate) fn check_binding_with(
        &self,
        binding: GeoBindingId,
        work: &mut impl FnMut(u64) -> Result<(), GeoError>,
    ) -> Result<(), GeoError> {
        self.visit_bindings(&mut |source| {
            work(1)?;
            matching_binding(source, binding)
        })
    }

    pub(crate) fn check_binding_admitted(
        &self,
        binding: GeoBindingId,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        self.visit_bindings(&mut |source| {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            if source != binding {
                return Err(crate::numerical::missing_operation_ids(
                    source, binding, context, progress,
                )?);
            }
            Ok(())
        })
    }

    fn visit_bindings(
        &self,
        visit: &mut impl FnMut(GeoBindingId) -> Result<(), GeoError>,
    ) -> Result<(), GeoError> {
        if let Self::Polygons(polygons) | Self::ComplementOfPolygons(polygons) = self {
            for polygon in polygons.iter() {
                visit(polygon.binding)?;
            }
        }
        Ok(())
    }
}

/// One immutable geographic inventory shared by metrics, topology and covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedGeometry {
    reference: GeographicReference,
    points: Arc<[PreparedCoordinate]>,
    symbolic_points: Arc<[OperationImagePoint]>,
    curves: Arc<[PreparedCurve]>,
    region: PreparedRegion,
    id: Digest32,
    preparation_work: u64,
    retained_workspace: u64,
}
impl PreparedGeometry {
    /// Prepare every original carrier coordinate using the single profile resolver.
    /// Collections are walked iteratively. Polygon boundaries are a separate areal
    /// inventory, so length/perimeter can account for both dimensions correctly.
    ///
    /// # Errors
    /// Refuses unresolved references, malformed/range inputs and complete limits.
    pub fn from_literal(literal: &GeometryLiteral, profile: &GeoProfile) -> Result<Self, GeoError> {
        Self::prepare_literal(literal, profile, profile.policy(), None, None)
    }
    /// Prepare original carrier geometry with bounded governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::from_literal`].
    pub fn from_literal_metered(
        literal: &GeometryLiteral,
        profile: &GeoProfile,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_literal(literal, profile, profile.policy(), Some(observer), None)
    }
    /// Prepare using the profile's reference resolver and separately supplied
    /// remaining admission, without cloning or changing reference registries.
    ///
    /// # Errors
    /// Uses [`Self::from_literal`]'s contract under the supplied admission.
    pub fn from_literal_in_policy(
        literal: &GeometryLiteral,
        profile: &GeoProfile,
        policy: crate::ExecutionPolicy,
    ) -> Result<Self, GeoError> {
        Self::prepare_literal(literal, profile, policy, None, None)
    }
    /// Meter carrier preparation under separately supplied remaining admission.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::from_literal_in_policy`].
    pub fn from_literal_in_policy_metered(
        literal: &GeometryLiteral,
        profile: &GeoProfile,
        policy: crate::ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_literal(literal, profile, policy, Some(observer), None)
    }
    /// Interpret original carrier axes on an explicitly proven equivalent surface.
    /// The original source ordinates and reference remain bound in the view identity.
    ///
    /// # Errors
    /// Refuses a different datum/ellipsoid or incomplete source admission.
    pub fn from_literal_surface_view(
        literal: &GeometryLiteral,
        profile: &GeoProfile,
        target: &GeographicReference,
        policy: crate::ExecutionPolicy,
    ) -> Result<Self, GeoError> {
        Self::prepare_literal(literal, profile, policy, None, Some(target))
    }
    /// Prepare an exact equivalent-surface view with bounded governor polling.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::from_literal_surface_view`].
    pub fn from_literal_surface_view_metered(
        literal: &GeometryLiteral,
        profile: &GeoProfile,
        target: &GeographicReference,
        policy: crate::ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_literal(literal, profile, policy, Some(observer), Some(target))
    }
    fn prepare_literal(
        literal: &GeometryLiteral,
        profile: &GeoProfile,
        limits: crate::ExecutionPolicy,
        observer: Option<&mut dyn MetricWorkObserver>,
        target: Option<&GeographicReference>,
    ) -> Result<Self, GeoError> {
        let source_reference = profile.reference(literal.crs())?;
        let reference = target.unwrap_or(source_reference);
        if !reference.same_surface(source_reference) {
            return Err(GeoError::MissingOperation {
                source: source_reference.id().digest().to_string(),
                target: reference.id().digest().to_string(),
            });
        }
        let binding = reference.id();
        let mut admission = admit_preparation(literal.geometry(), limits.limits(), observer)?;
        let mut points = Vec::new();
        let mut curves = Vec::new();
        let mut polygons = Vec::new();
        let mut pending =
            purrdf_lex::walk::WorkList::<&crate::Geometry, 16>::with(literal.geometry());
        while let Some(geometry) = pending.pop() {
            admission.step(1)?;
            match geometry.body() {
                GeometryBody::Point(point) => {
                    if let Some(point) = point {
                        admission.step(1)?;
                        points.push(admission.prepare_coordinate(
                            point,
                            source_reference,
                            binding,
                        )?);
                    }
                }
                GeometryBody::MultiPoint(members) => {
                    for point in members.iter().flatten() {
                        admission.step(1)?;
                        points.push(admission.prepare_coordinate(
                            point,
                            source_reference,
                            binding,
                        )?);
                    }
                }
                GeometryBody::LineString(line) => {
                    if !line.is_empty() {
                        curves.push(PreparedCurve::from_source_with_binding(
                            line,
                            source_reference,
                            binding,
                            &mut admission,
                        )?);
                    }
                }
                GeometryBody::MultiLineString(lines) => {
                    for line in lines {
                        admission.step(1)?;
                        if !line.is_empty() {
                            curves.push(PreparedCurve::from_source_with_binding(
                                line,
                                source_reference,
                                binding,
                                &mut admission,
                            )?);
                        }
                    }
                }
                GeometryBody::Polygon(rings) => {
                    if !rings.is_empty() {
                        polygons.push(PreparedPolygon::from_source_with_binding(
                            rings,
                            source_reference,
                            binding,
                            RegionInterior::Written,
                            &mut admission,
                        )?);
                    }
                }
                GeometryBody::MultiPolygon(members) => {
                    for rings in members {
                        admission.step(1)?;
                        if !rings.is_empty() {
                            polygons.push(PreparedPolygon::from_source_with_binding(
                                rings,
                                source_reference,
                                binding,
                                RegionInterior::Written,
                                &mut admission,
                            )?);
                        }
                    }
                }
                GeometryBody::GeometryCollection(members) => pending.extend(members.iter().rev()),
            }
        }
        let original_id = prepared_identity(literal, source_reference, &mut admission)?;
        let id = if reference.id() == source_reference.id() {
            original_id
        } else {
            admission.step(2)?;
            crate::profile::hash_fields(
                SURFACE_VIEW_DOMAIN,
                [
                    original_id.as_bytes().as_slice(),
                    binding.digest().as_bytes().as_slice(),
                ],
            )
        };
        admission.poll()?;
        Ok(Self {
            reference: reference.clone(),
            points: points.into(),
            symbolic_points: Arc::from([]),
            curves: curves.into(),
            region: PreparedRegion::polygons(polygons),
            id,
            preparation_work: admission.work,
            retained_workspace: admission.bytes,
        })
    }
    /// Prepare explicitly supplied native points, curve laws and selected regions.
    /// Original references and complete admission are validated before ownership
    /// is converted to shared immutable storage. No polygon is invented for curves.
    ///
    /// # Errors
    /// Refuses reference mismatch and incomplete work/output/workspace admission.
    pub fn from_parts(
        reference: GeographicReference,
        points: Vec<PreparedCoordinate>,
        curves: Vec<PreparedCurve>,
        region: PreparedRegion,
        policy: crate::ExecutionPolicy,
    ) -> Result<Self, GeoError> {
        Self::prepare_parts(reference, points, Vec::new(), curves, region, policy, None)
    }
    /// Prepare explicit native parts with bounded governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::from_parts`].
    pub fn from_parts_metered(
        reference: GeographicReference,
        points: Vec<PreparedCoordinate>,
        curves: Vec<PreparedCurve>,
        region: PreparedRegion,
        policy: crate::ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_parts(
            reference,
            points,
            Vec::new(),
            curves,
            region,
            policy,
            Some(observer),
        )
    }
    /// Prepare native parts including exact symbolic operation-image points.
    /// Symbolic points retain their original coordinate, operation chain and
    /// target reference; quantized endpoint samples are never substituted.
    ///
    /// # Errors
    /// Refuses binding mismatch or incomplete source/storage/work admission.
    pub fn from_parts_with_symbolic(
        reference: GeographicReference,
        points: Vec<PreparedCoordinate>,
        symbolic_points: Vec<OperationImagePoint>,
        curves: Vec<PreparedCurve>,
        region: PreparedRegion,
        policy: crate::ExecutionPolicy,
    ) -> Result<Self, GeoError> {
        Self::prepare_parts(
            reference,
            points,
            symbolic_points,
            curves,
            region,
            policy,
            None,
        )
    }
    /// Prepare symbolic native parts with bounded external governor charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::from_parts_with_symbolic`].
    pub fn from_parts_with_symbolic_metered(
        reference: GeographicReference,
        points: Vec<PreparedCoordinate>,
        symbolic_points: Vec<OperationImagePoint>,
        curves: Vec<PreparedCurve>,
        region: PreparedRegion,
        policy: crate::ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare_parts(
            reference,
            points,
            symbolic_points,
            curves,
            region,
            policy,
            Some(observer),
        )
    }
    fn prepare_parts(
        reference: GeographicReference,
        points: Vec<PreparedCoordinate>,
        symbolic_points: Vec<OperationImagePoint>,
        curves: Vec<PreparedCurve>,
        region: PreparedRegion,
        policy: crate::ExecutionPolicy,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let mut admission = PreparationAdmission::new(policy.limits(), observer)?;
        admission.storage::<PreparedCoordinate>(points.capacity())?;
        admission.storage::<OperationImagePoint>(symbolic_points.capacity())?;
        admission.storage::<PreparedCurve>(curves.capacity())?;
        let binding = reference.id();
        for point in &points {
            matching_binding(point.binding, binding)?;
            admission.coordinate(&point.source)?;
        }
        for point in &symbolic_points {
            matching_binding(point.reference().id(), binding)?;
            admission.coordinate(point.source())?;
            admission.bytes = admission
                .bytes
                .checked_add(point.retained_workspace_bytes())
                .ok_or(GeoError::ArithmeticOverflow(
                    "symbolic point retained storage",
                ))?;
            admission.memory()?;
        }
        for curve in &curves {
            admit_curve(curve, binding, &mut admission)?;
        }
        if let PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) =
            &region
        {
            for polygon in polygons.iter() {
                admission.structure(1)?;
                matching_binding(polygon.binding, binding)?;
                if let Some(bounds) = &polygon.angular_bounds {
                    admission.step(1)?;
                    admission.bytes = admission.bytes.checked_add(bounds.retained_bytes).ok_or(
                        GeoError::ArithmeticOverflow("prepared angular proof storage"),
                    )?;
                    admission.memory()?;
                }
                for ring in polygon.rings() {
                    admit_curve(ring, binding, &mut admission)?;
                }
            }
        }
        let id = parts_identity(
            &reference,
            &points,
            &symbolic_points,
            &curves,
            &region,
            &mut admission,
        )?;
        admission.poll()?;
        Ok(Self {
            reference,
            points: points.into(),
            symbolic_points: symbolic_points.into(),
            curves: curves.into(),
            region,
            id,
            preparation_work: admission.work,
            retained_workspace: admission.bytes,
        })
    }
    /// Exact reference binding shared by this complete geometry.
    #[must_use]
    pub const fn reference(&self) -> &GeographicReference {
        &self.reference
    }
    /// Original point components.
    #[must_use]
    pub fn points(&self) -> &[PreparedCoordinate] {
        &self.points
    }
    /// Exact dimension-zero operation images, with original source retained.
    #[must_use]
    pub fn symbolic_points(&self) -> &[OperationImagePoint] {
        &self.symbolic_points
    }
    /// Retain an aggregate receipt from already admitted outer preparation
    /// phases. Admission evidence is excluded from mathematical identity.
    pub(crate) fn with_preparation_receipt(mut self, work: u64, retained: u64) -> Self {
        self.preparation_work = self.preparation_work.max(work);
        self.retained_workspace = self.retained_workspace.max(retained);
        self
    }
    /// Replace a generated proof partition's parts identity with the original
    /// continuous source identity computed by the operation image home.
    pub(crate) fn with_image_source_identity(mut self, identity: Digest32) -> Self {
        self.id = identity;
        self
    }
    /// Curve components, excluding areal boundaries.
    #[must_use]
    pub fn curves(&self) -> &[PreparedCurve] {
        &self.curves
    }
    /// Explicit areal region union and its separate boundary ring inventory.
    #[must_use]
    pub const fn region(&self) -> &PreparedRegion {
        &self.region
    }
    /// Original content, reference and interpolation law identity.
    #[must_use]
    pub const fn id(&self) -> Digest32 {
        self.id
    }
    /// Identity-bound source admission for a following operation that retains
    /// this same immutable geometry. It is excluded from the source identity.
    #[must_use]
    pub const fn source_receipt(&self) -> crate::PreparedSourceReceipt {
        crate::PreparedSourceReceipt::new(self.id, self.preparation_work, self.retained_workspace)
    }

    /// Logical preparation work counted by the shared checked admission walk.
    /// This receipt is excluded from the original geometry content identity.
    #[must_use]
    pub const fn preparation_work_items(&self) -> u64 {
        self.preparation_work
    }
    /// Certified upper bound for retained source/prepared storage, including
    /// the preparation home's conservative temporary allowance. This is an
    /// admission receipt, not a measurement of allocator traffic or RSS.
    #[must_use]
    pub const fn retained_workspace_bytes(&self) -> u64 {
        self.retained_workspace
    }
}
impl SourcePreparation for PreparationAdmission<'_, '_> {
    fn source_step(&mut self) -> Result<(), GeoError> {
        self.step(1)
    }
    fn prepare_coordinate(
        &mut self,
        source: &Coord,
        reference: &GeographicReference,
        binding: GeoBindingId,
    ) -> Result<PreparedCoordinate, GeoError> {
        use purrdf_xsd::integer::ExactOperation::{Linear, RationalCompare};
        let operands = [Some(source.x()), Some(source.y()), source.z(), source.m()]
            .into_iter()
            .flatten()
            .collect::<purrdf_core::SmallVec<[&Rat; 4]>>();
        let copies = crate::numerical::rational_cost(Linear, &operands, operands.len() as u64 + 3)
            .ok_or(GeoError::ArithmeticOverflow(
                "original coordinate clone admission",
            ))?;
        let range = crate::numerical::rational_cost(RationalCompare, &[source.x(), source.y()], 4)
            .ok_or(GeoError::ArithmeticOverflow(
                "original coordinate range admission",
            ))?;
        let cost = copies
            .followed_by(range)
            .ok_or(GeoError::ArithmeticOverflow(
                "original coordinate preparation admission",
            ))?;
        self.exact(cost, || {
            PreparedCoordinate::with_binding(source.clone(), reference, binding)
        })
    }
    fn chart_coordinate(&mut self, point: &PreparedCoordinate) -> Result<Coord, GeoError> {
        let cost = crate::numerical::rational_cost(
            purrdf_xsd::integer::ExactOperation::Linear,
            &[point.point.longitude(), point.point.latitude()],
            2,
        )
        .ok_or(GeoError::ArithmeticOverflow(
            "original chart coordinate clone admission",
        ))?;
        self.exact(cost, || {
            Ok(Coord::xy(
                point.point.longitude().clone(),
                point.point.latitude().clone(),
            ))
        })
    }
}
struct PreparationAdmission<'a, 'observer> {
    limits: &'a crate::ExecutionLimits,
    work: u64,
    vertices: u64,
    bytes: u64,
    nesting_depth: u64,
    maximum_original_operand_bits: u64,
    output_structure_bytes: u64,
    progress: WorkProgress<'observer>,
    polled_work: u64,
}
impl<'a, 'observer> PreparationAdmission<'a, 'observer> {
    fn new(
        limits: &'a crate::ExecutionLimits,
        observer: Option<&'observer mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let mut progress = WorkProgress::new(observer);
        progress.initial()?;
        let mut admission = Self {
            limits,
            work: 0,
            vertices: 0,
            bytes: 65_536,
            nesting_depth: 0,
            maximum_original_operand_bits: 0,
            output_structure_bytes: 0,
            progress,
            polled_work: 0,
        };
        admission.memory()?;
        admission.poll()?;
        Ok(admission)
    }
    fn exact<T>(
        &mut self,
        cost: purrdf_xsd::integer::ExactArithmeticCost,
        evaluate: impl FnOnce() -> Result<T, GeoError>,
    ) -> Result<T, GeoError> {
        let peak =
            self.bytes
                .checked_add(cost.workspace_bytes)
                .ok_or(GeoError::ArithmeticOverflow(
                    "source preparation arithmetic workspace",
                ))?;
        if peak > self.limits.max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: self.limits.max_workspace_bytes,
            });
        }
        self.step(cost.work_items)?;
        self.progress.charge_counts(self.work, peak)?;
        let result = evaluate();
        self.poll()?;
        result
    }
    fn step(&mut self, count: u64) -> Result<(), GeoError> {
        self.work = self.work.saturating_add(count);
        if self.work > self.limits.max_work_items {
            return Err(GeoError::WorkExhausted {
                limit: self.limits.max_work_items,
            });
        }
        if self.work.saturating_sub(self.polled_work) >= 64 {
            self.poll()?;
        }
        Ok(())
    }
    fn poll(&mut self) -> Result<(), GeoError> {
        self.progress.charge_counts(self.work, self.bytes)?;
        self.polled_work = self.work;
        Ok(())
    }
    fn structure(&mut self, count: usize) -> Result<(), GeoError> {
        let count = u64::try_from(count).map_err(|_| GeoError::WorkExhausted {
            limit: self.limits.max_work_items,
        })?;
        self.step(count)?;
        self.structure_storage(count)
    }
    fn structure_storage(&mut self, count: u64) -> Result<(), GeoError> {
        self.bytes = self.bytes.saturating_add(count.saturating_mul(256));
        self.memory()
    }
    fn memory(&self) -> Result<(), GeoError> {
        if self.bytes > self.limits.max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: self.limits.max_workspace_bytes,
            });
        }
        Ok(())
    }
    fn coordinate(&mut self, coordinate: &Coord) -> Result<(), GeoError> {
        self.structure(1)?;
        self.vertices = self.vertices.saturating_add(1);
        if self.vertices > self.limits.max_output_elements {
            return Err(GeoError::OutputExhausted {
                limit: self.limits.max_output_elements,
            });
        }
        self.bytes = self.bytes.saturating_add(4096);
        for value in [coordinate.x(), coordinate.y()]
            .into_iter()
            .chain(coordinate.z())
            .chain(coordinate.m())
        {
            self.maximum_original_operand_bits = self
                .maximum_original_operand_bits
                .max(crate::numerical::rational_operand_bits(value));
            let arithmetic = value
                .numerator()
                .bit_len()
                .saturating_add(value.denominator().bit_len())
                .div_ceil(8)
                .saturating_mul(32);
            let retained = u64::try_from(value.numerator().allocated_bytes())
                .unwrap_or(u64::MAX)
                .saturating_add(
                    u64::try_from(value.denominator().allocated_bytes()).unwrap_or(u64::MAX),
                );
            self.bytes = self.bytes.saturating_add(arithmetic.max(retained));
        }
        self.memory()
    }
    fn storage<T>(&mut self, capacity: usize) -> Result<(), GeoError> {
        let bytes = u64::try_from(capacity)
            .ok()
            .and_then(|capacity| capacity.checked_mul(size_of::<T>() as u64))
            .ok_or(GeoError::ArithmeticOverflow("carrier container storage"))?;
        self.output_structure_bytes =
            self.output_structure_bytes
                .checked_add(bytes)
                .ok_or(GeoError::ArithmeticOverflow(
                    "materialized carrier containers",
                ))?;
        self.bytes = self.bytes.saturating_add(bytes);
        self.memory()
    }
}

impl crate::carrier::GeometryStorageVisitor for PreparationAdmission<'_, '_> {
    fn geometry(&mut self, depth: u64) -> Result<(), GeoError> {
        self.step(1)?;
        self.nesting_depth = self.nesting_depth.max(depth);
        Ok(())
    }
    fn structure(&mut self) -> Result<(), GeoError> {
        PreparationAdmission::structure(self, 1)
    }
    fn coordinate(&mut self, coordinate: &Coord) -> Result<(), GeoError> {
        PreparationAdmission::coordinate(self, coordinate)
    }
    fn storage<T>(&mut self, capacity: usize) -> Result<(), GeoError> {
        PreparationAdmission::storage::<T>(self, capacity)
    }
    fn collection(&mut self, children: usize) -> Result<(), GeoError> {
        self.structure_storage(
            u64::try_from(children).map_err(|_| GeoError::MemoryExhausted {
                limit: self.limits.max_workspace_bytes,
            })?,
        )
    }
    fn pending_child(&mut self) -> Result<(), GeoError> {
        self.step(1)
    }
}

/// Admission evidence for the complete original carrier tree. This walk makes
/// no geographic range, reference or axis interpretation and counts actual
/// owned vector capacities before another tree or coordinate image is built.
pub(crate) struct SourceInventory {
    pub(crate) content_id: Digest32,
    pub(crate) work_items: u64,
    pub(crate) workspace_bytes: u64,
    pub(crate) vertices: u64,
    pub(crate) nesting_depth: u64,
    maximum_original_operand_bits: u64,
    output_structure_bytes: u64,
}

impl SourceInventory {
    /// Existing carrier container capacities, admitted before an image copies
    /// its tree, including empty rings and empty nested collection members.
    pub(crate) const fn output_structure_bytes(&self) -> u64 {
        self.output_structure_bytes
    }
    /// Maximum original XY/Z/M numerator or denominator width, read during
    /// the shared admission walk without copying limbs or changing identity.
    pub(crate) const fn maximum_original_operand_bits(&self) -> u64 {
        self.maximum_original_operand_bits
    }
}

pub(crate) fn source_inventory(
    geometry: &crate::Geometry,
    limits: &crate::ExecutionLimits,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<SourceInventory, GeoError> {
    let mut admission = admit_preparation(geometry, limits, observer)?;
    let mut hash = Hasher::new();
    frame_le_into(&mut hash, SOURCE_DOMAIN.as_bytes());
    frame_geometry(&mut hash, geometry, &mut admission)?;
    let content_id = Digest32::new(*hash.finalize().as_bytes());
    admission.poll()?;
    Ok(SourceInventory {
        content_id,
        work_items: admission.work,
        workspace_bytes: admission.bytes,
        vertices: admission.vertices,
        nesting_depth: admission.nesting_depth,
        maximum_original_operand_bits: admission.maximum_original_operand_bits,
        output_structure_bytes: admission.output_structure_bytes,
    })
}

/// Borrowed two-endpoint preflight before a named operation image copies them.
/// Uses the original coordinate/storage admission; it neither interprets axes
/// nor manufactures a source identity before the complete curve constructor.
pub(crate) fn coordinate_source_admission(
    coordinates: [&Coord; 2],
    limits: &crate::ExecutionLimits,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<(u64, u64), GeoError> {
    let mut admission = PreparationAdmission::new(limits, observer)?;
    admission.structure(1)?;
    admission.storage::<Coord>(coordinates.len())?;
    for coordinate in coordinates {
        admission.coordinate(coordinate)?;
    }
    admission.poll()?;
    Ok((admission.work, admission.bytes))
}

fn admit_preparation<'a, 'observer>(
    geometry: &crate::Geometry,
    limits: &'a crate::ExecutionLimits,
    observer: Option<&'observer mut dyn MetricWorkObserver>,
) -> Result<PreparationAdmission<'a, 'observer>, GeoError> {
    let mut admission = PreparationAdmission::new(limits, observer)?;
    admission.structure_storage(1)?;
    crate::carrier::visit_geometry_storage(geometry, &mut admission)?;
    Ok(admission)
}

fn frame_value(
    hash: &mut Hasher,
    value: &Rat,
    admission: &mut PreparationAdmission<'_, '_>,
) -> Result<(), GeoError> {
    let cost = crate::numerical::rational_cost(
        purrdf_xsd::integer::ExactOperation::DecimalRender,
        &[value],
        2,
    )
    .ok_or(GeoError::ArithmeticOverflow(
        "original scalar identity rendering",
    ))?;
    admission.exact(cost, || {
        frame_le_into(hash, value.numerator().to_string().as_bytes());
        frame_le_into(hash, value.denominator().to_string().as_bytes());
        Ok(())
    })
}
fn frame_coordinate(
    hash: &mut Hasher,
    coordinate: &Coord,
    admission: &mut PreparationAdmission<'_, '_>,
) -> Result<(), GeoError> {
    admission.step(1)?;
    frame_le_into(
        hash,
        &[
            u8::from(coordinate.z().is_some()),
            u8::from(coordinate.m().is_some()),
        ],
    );
    for value in [coordinate.x(), coordinate.y()]
        .into_iter()
        .chain(coordinate.z())
        .chain(coordinate.m())
    {
        frame_value(hash, value, admission)?;
    }
    Ok(())
}

fn frame_geometry(
    hash: &mut Hasher,
    geometry: &crate::Geometry,
    admission: &mut PreparationAdmission<'_, '_>,
) -> Result<(), GeoError> {
    fn line(
        hash: &mut Hasher,
        coordinates: &[Coord],
        admission: &mut PreparationAdmission<'_, '_>,
    ) -> Result<(), GeoError> {
        admission.step(1)?;
        frame_le_into(hash, &(coordinates.len() as u64).to_be_bytes());
        for value in coordinates {
            frame_coordinate(hash, value, admission)?;
        }
        Ok(())
    }
    fn polygon(
        hash: &mut Hasher,
        rings: &crate::Rings,
        admission: &mut PreparationAdmission<'_, '_>,
    ) -> Result<(), GeoError> {
        admission.step(1)?;
        frame_le_into(hash, &(rings.len() as u64).to_be_bytes());
        for ring in rings {
            line(hash, ring, admission)?;
        }
        Ok(())
    }
    let mut pending = purrdf_lex::walk::WorkList::<&crate::Geometry, 16>::with(geometry);
    while let Some(geometry) = pending.pop() {
        admission.step(1)?;
        frame_le_into(hash, geometry.kind().wkt_keyword().as_bytes());
        frame_le_into(hash, geometry.dim().name().as_bytes());
        match geometry.body() {
            GeometryBody::Point(point) => {
                frame_le_into(hash, &[u8::from(point.is_some())]);
                if let Some(point) = point {
                    frame_coordinate(hash, point, admission)?;
                }
            }
            GeometryBody::LineString(points) => line(hash, points, admission)?,
            GeometryBody::Polygon(rings) => polygon(hash, rings, admission)?,
            GeometryBody::MultiPoint(points) => {
                frame_le_into(hash, &(points.len() as u64).to_be_bytes());
                for point in points {
                    admission.step(1)?;
                    frame_le_into(hash, &[u8::from(point.is_some())]);
                    if let Some(point) = point {
                        frame_coordinate(hash, point, admission)?;
                    }
                }
            }
            GeometryBody::MultiLineString(lines) => {
                frame_le_into(hash, &(lines.len() as u64).to_be_bytes());
                for points in lines {
                    line(hash, points, admission)?;
                }
            }
            GeometryBody::MultiPolygon(polygons) => {
                frame_le_into(hash, &(polygons.len() as u64).to_be_bytes());
                for rings in polygons {
                    polygon(hash, rings, admission)?;
                }
            }
            GeometryBody::GeometryCollection(members) => {
                frame_le_into(hash, &(members.len() as u64).to_be_bytes());
                for member in members.iter().rev() {
                    admission.step(1)?;
                    pending.push(member);
                }
            }
        }
    }
    Ok(())
}

fn prepared_identity(
    literal: &GeometryLiteral,
    reference: &GeographicReference,
    admission: &mut PreparationAdmission<'_, '_>,
) -> Result<Digest32, GeoError> {
    let mut hash = Hasher::new();
    for field in [
        PREPARED_DOMAIN.as_bytes(),
        b"source-linear-original-coordinates;identified-atlas;written-rings;v1",
        reference.id().digest().as_bytes(),
    ] {
        frame_le_into(&mut hash, field);
    }
    frame_geometry(&mut hash, literal.geometry(), admission)?;
    Ok(Digest32::new(*hash.finalize().as_bytes()))
}

fn admit_curve(
    curve: &PreparedCurve,
    binding: GeoBindingId,
    admission: &mut PreparationAdmission<'_, '_>,
) -> Result<(), GeoError> {
    admission.structure(1)?;
    for edge in curve.edges() {
        matching_binding(edge.binding_id(), binding)?;
        admission.structure(1)?;
        admission.coordinate(edge.source_start())?;
        match edge {
            PreparedEdge::SourceLinear(edge) => {
                matching_binding(edge.end.binding, binding)?;
                admission.coordinate(&edge.end.source)?;
            }
            PreparedEdge::AzimuthLength(arc) => {
                for value in [&arc.azimuth, arc.length.exact()] {
                    admission.bytes = admission.bytes.saturating_add(
                        value
                            .numerator()
                            .bit_len()
                            .saturating_add(value.denominator().bit_len())
                            .div_ceil(8)
                            .saturating_mul(32),
                    );
                    admission.memory()?;
                }
            }
            PreparedEdge::ShortestGeodesic(arc) => {
                matching_binding(arc.end.binding, binding)?;
                admission.coordinate(&arc.end.source)?;
            }
            PreparedEdge::Transformed(image) => {
                admission.coordinate(image.source_endpoints().1)?;
                admission.bytes = admission
                    .bytes
                    .saturating_add(image.retained_workspace_bytes());
                admission.memory()?;
            }
        }
    }
    Ok(())
}
fn parts_identity(
    reference: &GeographicReference,
    points: &[PreparedCoordinate],
    symbolic_points: &[OperationImagePoint],
    curves: &[PreparedCurve],
    region: &PreparedRegion,
    admission: &mut PreparationAdmission<'_, '_>,
) -> Result<Digest32, GeoError> {
    fn curve(
        hash: &mut Hasher,
        curve: &PreparedCurve,
        admission: &mut PreparationAdmission<'_, '_>,
    ) -> Result<(), GeoError> {
        admission.step(1)?;
        frame_le_into(hash, &(curve.edges().len() as u64).to_be_bytes());
        for edge in curve.edges() {
            admission.step(1)?;
            frame_le_into(
                hash,
                &[match edge {
                    PreparedEdge::SourceLinear(_) => 0,
                    PreparedEdge::AzimuthLength(_) => 1,
                    PreparedEdge::ShortestGeodesic(_) => 2,
                    PreparedEdge::Transformed(_) => 3,
                }],
            );
            frame_coordinate(hash, edge.source_start(), admission)?;
            match edge {
                PreparedEdge::SourceLinear(edge) => {
                    frame_coordinate(hash, edge.end().source(), admission)?;
                }
                PreparedEdge::AzimuthLength(arc) => {
                    for value in [arc.azimuth(), arc.length().exact()] {
                        admission.step(1)?;
                        frame_value(hash, value, admission)?;
                    }
                }
                PreparedEdge::ShortestGeodesic(arc) => {
                    frame_coordinate(hash, arc.end().source(), admission)?;
                }
                PreparedEdge::Transformed(image) => {
                    frame_le_into(hash, image.id().as_bytes());
                }
            }
        }
        Ok(())
    }
    let mut hash = Hasher::new();
    for field in [PREPARED_DOMAIN.as_bytes(),b"native-original-law-parts;source-linear;azimuth-length;identified-atlas;explicit-region-side;v1",reference.id().digest().as_bytes()] {frame_le_into(&mut hash,field);}
    frame_le_into(&mut hash, &(points.len() as u64).to_be_bytes());
    for point in points {
        frame_coordinate(&mut hash, point.source(), admission)?;
    }
    frame_le_into(&mut hash, &(curves.len() as u64).to_be_bytes());
    for value in curves {
        curve(&mut hash, value, admission)?;
    }
    frame_le_into(
        &mut hash,
        &[match region {
            PreparedRegion::Empty => 0,
            PreparedRegion::Whole => 1,
            PreparedRegion::Polygons(_) => 2,
            PreparedRegion::ComplementOfPolygons(_) => 3,
        }],
    );
    if let PreparedRegion::Polygons(polygons) | PreparedRegion::ComplementOfPolygons(polygons) =
        region
    {
        frame_le_into(&mut hash, &(polygons.len() as u64).to_be_bytes());
        for polygon in polygons.iter() {
            admission.step(1)?;
            frame_le_into(
                &mut hash,
                &[u8::from(polygon.interior() == RegionInterior::Complement)],
            );
            frame_le_into(
                &mut hash,
                &[if polygon.closed_support() {
                    3
                } else {
                    match polygon.oriented_interior() {
                        None => 0,
                        Some(OrientedInterior::Left) => 1,
                        Some(OrientedInterior::Right) => 2,
                    }
                }],
            );
            frame_le_into(&mut hash, &(polygon.rings().len() as u64).to_be_bytes());
            for ring in polygon.rings() {
                curve(&mut hash, ring, admission)?;
            }
        }
    }
    if !symbolic_points.is_empty() {
        frame_le_into(&mut hash, SYMBOLIC_POINTS_DOMAIN.as_bytes());
        frame_le_into(&mut hash, &(symbolic_points.len() as u64).to_be_bytes());
        for point in symbolic_points {
            admission.step(1)?;
            frame_le_into(&mut hash, point.id().as_bytes());
        }
    }
    Ok(Digest32::new(*hash.finalize().as_bytes()))
}

#[cfg(test)]
mod tests;
