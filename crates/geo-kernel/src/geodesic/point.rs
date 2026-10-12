// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Immutable exact endpoint preparation shared by repeated point trials.

use crate::context::retained::{NativeDetached, produce_persistent_numeric_fields};

use purrdf_xsd::math::{
    CertifiedInterval, FixedInterval, FloatEnclosure, IntervalContext, MathError, MathLimits,
    WordInterval,
};

use super::{ChunkObserver, PreparedGeodesic, SolveError, reduced_latitude_with};
use crate::numerical::{
    canonical_rational_scan_cost, fixed_from_rat, geo_math_error, math_exact_rational,
};
use crate::{
    GeoBindingId, GeoError, LonLat, Metres, MetricContext, MetricEstimate, MetricWorkObserver, Rat,
};

#[derive(Clone, Debug)]
pub(super) struct ReducedPoint<I> {
    pub(super) sine: I,
    pub(super) cosine: I,
    pub(super) height_factor: I,
}

#[derive(Clone, Debug)]
struct PointData {
    source: LonLat,
    binding: GeoBindingId,
    bits: u32,
    fixed: ReducedPoint<FixedInterval>,
    floating: ReducedPoint<FloatEnclosure>,
}

crate::context::retained::native_detached_fields!(ReducedPoint<FixedInterval> {
    native: [sine, cosine, height_factor], copy: []
});
crate::context::retained::native_detached_fields!(PointData {
    native: [source, fixed],
    copy: [binding, bits],
    shared: [floating]
});

/// Immutable reduced-latitude preparation retaining the original exact point.
///
/// Clones share heap-backed proof limbs while inline values remain inline,
/// without retaining thread-local arithmetic control
/// state. The reference and exact source define equality; proof precision and
/// execution admission do not change that mathematical identity.
#[derive(Clone, Debug)]
pub struct PreparedGeodesicPoint {
    data: PointData,
    retained_bytes: u64,
}

impl PartialEq for PreparedGeodesicPoint {
    fn eq(&self, other: &Self) -> bool {
        self.data.binding == other.data.binding && self.data.source == other.data.source
    }
}
impl Eq for PreparedGeodesicPoint {}

impl PreparedGeodesicPoint {
    /// Original validated coordinates, with their source longitude spelling.
    #[must_use]
    pub fn source(&self) -> &LonLat {
        &self.data.source
    }

    /// Declared geographic reference to which the prepared point belongs.
    #[must_use]
    pub fn binding_id(&self) -> GeoBindingId {
        self.data.binding
    }

    /// Actual retained source/proof limbs and their original shared owner.
    /// Inline source and proof values retain no heap allocation.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> u64 {
        self.retained_bytes
    }
}

pub(super) trait PointInterval: CertifiedInterval {
    fn endpoint_cache(point: &PreparedGeodesicPoint, bits: u32) -> Option<&ReducedPoint<Self>>;
    /// A round-to-nearest midpoint proposal. Proposals only select certified
    /// evaluation points; they prove nothing.
    fn proposal(&self, math: &mut IntervalContext<'_>) -> Option<f64>;
    /// An upper bound of the greatest endpoint magnitude.
    fn magnitude(&self, math: &mut IntervalContext<'_>) -> Option<f64>;
    /// The exact enclosure of two finite binary64 endpoints.
    fn from_binary64_bounds(
        lower: f64,
        upper: f64,
        math: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError>;
}
impl PointInterval for FixedInterval {
    fn endpoint_cache(point: &PreparedGeodesicPoint, bits: u32) -> Option<&ReducedPoint<Self>> {
        (point.data.bits == bits).then_some(&point.data.fixed)
    }
    fn proposal(&self, _: &mut IntervalContext<'_>) -> Option<f64> {
        self.approximate_binary64()
    }
    fn magnitude(&self, _: &mut IntervalContext<'_>) -> Option<f64> {
        // Only sizes the first proof neighbourhood; a later widening retries.
        let middle = self.approximate_binary64()?.abs();
        Some(purrdf_xsd::ieee::f64_mul(middle, 1.000_001))
    }
    fn from_binary64_bounds(
        lower: f64,
        upper: f64,
        math: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError> {
        let lower = Self::from_binary64(lower, math.math())?;
        let upper = Self::from_binary64(upper, math.math())?;
        Self::from_bounds(lower.lower().clone(), upper.upper().clone(), math.math())
    }
}
impl PointInterval for WordInterval {
    fn endpoint_cache(_: &PreparedGeodesicPoint, _: u32) -> Option<&ReducedPoint<Self>> {
        None
    }
    fn proposal(&self, math: &mut IntervalContext<'_>) -> Option<f64> {
        let ops = math.binary64_ops()?;
        let middle = ops.add(
            ops.mul(self.lower().hi(), 0.5),
            ops.mul(self.upper().hi(), 0.5),
        );
        middle.is_finite().then_some(middle)
    }
    fn magnitude(&self, _: &mut IntervalContext<'_>) -> Option<f64> {
        // Each leading component is within half a unit of its word.
        Some(
            self.lower()
                .hi()
                .abs()
                .max(self.upper().hi().abs())
                .next_up(),
        )
    }
    fn from_binary64_bounds(
        lower: f64,
        upper: f64,
        math: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError> {
        math.math().admit_exact(1, 128)?;
        Self::from_binary64(lower, upper)
    }
}
impl PointInterval for FloatEnclosure {
    fn endpoint_cache(point: &PreparedGeodesicPoint, _: u32) -> Option<&ReducedPoint<Self>> {
        Some(&point.data.floating)
    }
    fn proposal(&self, math: &mut IntervalContext<'_>) -> Option<f64> {
        let ops = math.binary64_ops()?;
        let (lower, upper) = self.bounds();
        let middle = ops.add(ops.mul(lower, 0.5), ops.mul(upper, 0.5));
        middle.is_finite().then_some(middle)
    }
    fn magnitude(&self, _: &mut IntervalContext<'_>) -> Option<f64> {
        let (lower, upper) = self.bounds();
        Some(lower.abs().max(upper.abs()))
    }
    fn from_binary64_bounds(
        lower: f64,
        upper: f64,
        math: &mut IntervalContext<'_>,
    ) -> Result<Self, MathError> {
        math.math().admit_exact(1, 64)?;
        Self::from_binary64_bounds(lower, upper)
    }
}

pub(super) fn reduced_point<I: PointInterval>(
    latitude: &Rat,
    source: Option<&PreparedGeodesicPoint>,
    c: &I,
    radians_per_degree: &I,
    ep2: &I,
    math: &mut IntervalContext<'_>,
) -> Result<ReducedPoint<I>, MathError> {
    if let Some(source) = source
        && let Some(cached) = I::endpoint_cache(source, math.limits().precision_bits)
        && {
            let original = source.source().latitude();
            let cost = canonical_rational_scan_cost(&[latitude, original])
                .ok_or(MathError::WorkExhausted)?;
            math.math().admit_exact_cost(cost)?;
            // Canonical denominators are positive. Compare borrowed numerator
            // magnitudes rather than cloning either wide absolute coordinate.
            latitude
                .numerator()
                .cmp_abs(original.numerator().as_ref())
                .is_eq()
                && latitude.denominator() == original.denominator()
        }
    {
        let mut result = cached.clone();
        if latitude.signum() < 0 {
            result.sine = result.sine.neg(math)?;
        }
        return Ok(result);
    }
    let (sine, cosine) = reduced_latitude_with(latitude, c, radians_per_degree, math)?;
    let height_factor = I::from_i64(1, math)?
        .add(&ep2.mul(&sine.square(math)?, math)?, math)?
        .sqrt(math)?;
    Ok(ReducedPoint {
        sine,
        cosine,
        height_factor,
    })
}

impl PreparedGeodesic {
    /// Prepare one exact source endpoint for repeated distance or threshold trials.
    ///
    /// # Errors
    /// Refuses reference mismatch, cancellation and numerical/resource exhaustion.
    pub fn prepare_point(
        &self,
        point: &LonLat,
        context: &mut MetricContext,
    ) -> Result<PreparedGeodesicPoint, GeoError> {
        self.prepare_point_observed(point, context, None)
    }

    /// Prepare the same endpoint with governed work and cancellation.
    ///
    /// # Errors
    /// Adds observer refusals to [`Self::prepare_point`].
    pub fn prepare_point_metered(
        &self,
        point: &LonLat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<PreparedGeodesicPoint, GeoError> {
        self.prepare_point_observed(point, context, Some(observer))
    }

    fn prepare_point_observed(
        &self,
        source: &LonLat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<PreparedGeodesicPoint, GeoError> {
        context.begin(0)?;
        context.charge_work(1)?;
        let mut chunks = ChunkObserver::new(observer);
        chunks.initial().map_err(super::solve_external_error)?;
        if !chunks.reference_matches(&self.reference, context)? {
            return Err(GeoError::config(
                "prepared point and context references differ",
            ));
        }
        let binding = chunks.reference_identity(Some(&self.reference), context)?;
        let policy = context.policy();
        let bits = policy.limits().max_precision_bits.min(80);
        let mut math = context
            .coordinate_math(MathLimits {
                precision_bits: bits,
                max_work: policy
                    .limits()
                    .max_work_items
                    .saturating_sub(context.work_items()),
                max_workspace_bytes: usize::try_from(context.remaining_workspace()).map_err(
                    |_| GeoError::MemoryExhausted {
                        limit: policy.limits().max_workspace_bytes,
                    },
                )?,
            })
            .map_err(|error| geo_math_error(&error, policy))?;
        chunks
            .context_entry(context)
            .map_err(super::solve_external_error)?;
        let result = (|| {
            chunks.install_arithmetic(context, &mut math)?;
            let a = fixed_from_rat(self.reference.ellipsoid().semimajor(), &mut math)?;
            let b = fixed_from_rat(self.reference.ellipsoid().semiminor(), &mut math)?;
            let c = b.div(&a, &mut math)?;
            let ep2 = fixed_from_rat(
                &self.reference.ellipsoid().eccentricity_squared(),
                &mut math,
            )?
            .div(&c.square(&mut math)?, &mut math)?;
            let radians_per_degree = FixedInterval::pi(&mut math)?
                .div(&FixedInterval::from_i64(180, &mut math)?, &mut math)?;
            let latitude = math_exact_rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[source.latitude()],
                &mut math,
                chunks.progress_mut(),
                |values| values.absolute(source.latitude()),
            )?;
            let fixed = reduced_point::<FixedInterval>(
                &latitude,
                None,
                &c,
                &radians_per_degree,
                &ep2,
                &mut IntervalContext::fixed(&mut math),
            )?;
            let floating = ReducedPoint {
                sine: FloatEnclosure::from_fixed(&fixed.sine, &mut math)?,
                cosine: FloatEnclosure::from_fixed(&fixed.cosine, &mut math)?,
                height_factor: FloatEnclosure::from_fixed(&fixed.height_factor, &mut math)?,
            };
            chunks.finish(&math)?;
            let payload_bytes = [
                source.detached_required_bytes(),
                fixed.sine.detached_required_bytes(),
                fixed.cosine.detached_required_bytes(),
                fixed.height_factor.detached_required_bytes(),
            ]
            .into_iter()
            .try_fold(0usize, |bytes, field| {
                bytes
                    .checked_add(field?)
                    .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)
            })
            .map_err(|error| SolveError::External(GeoError::Storage(error)))?;
            let point =
                produce_persistent_numeric_fields(payload_bytes, chunks.progress_mut(), |memory| {
                    Ok(PointData {
                        source: source
                            .detached_with_memory(memory)
                            .map_err(GeoError::Storage)?,
                        binding,
                        bits,
                        fixed: fixed.prepare(memory).map_err(GeoError::Storage)?,
                        floating,
                    })
                })
                .map_err(SolveError::External)?;
            let retained_bytes = point.workspace_bytes();
            Ok::<_, SolveError>(PreparedGeodesicPoint {
                data: point.into_value(),
                retained_bytes,
            })
        })();
        let observer_result = if matches!(&result, Err(SolveError::External(_))) {
            Ok(())
        } else {
            chunks.finish(&math)
        };
        context.charge_work(math.work_used())?;
        context.admit_workspace(math.workspace_peak() as u64)?;
        context.release_workspace(math.workspace_peak() as u64)?;
        observer_result.map_err(super::solve_external_error)?;
        result.map_err(|error| match error {
            SolveError::Math(error) => geo_math_error(&error, policy),
            SolveError::External(error) => error,
            SolveError::Iterations => GeoError::ConvergenceExhausted { iterations: 0 },
        })
    }

    /// Correctly rounded distance from a prepared exact source endpoint.
    ///
    /// # Errors
    /// Uses [`Self::distance`]'s reference, numerical and admission contract.
    pub fn distance_from_prepared(
        &self,
        source: &PreparedGeodesicPoint,
        point: &LonLat,
        context: &mut MetricContext,
    ) -> Result<MetricEstimate, GeoError> {
        self.distance_from_point_observed(source, point, context, None)
    }

    /// The same point distance with external governor/cancellation charging.
    ///
    /// # Errors
    /// Adds observer refusals to [`Self::distance_from_prepared`].
    pub fn distance_from_prepared_metered(
        &self,
        source: &PreparedGeodesicPoint,
        point: &LonLat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<MetricEstimate, GeoError> {
        self.distance_from_point_observed(source, point, context, Some(observer))
    }

    fn distance_from_point_observed(
        &self,
        source: &PreparedGeodesicPoint,
        point: &LonLat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<MetricEstimate, GeoError> {
        context.begin(1)?;
        self.compute_point_cached(
            source.source(),
            point,
            context,
            None,
            observer,
            Some(source),
        )
        .map(|(answer, _)| answer)
    }

    /// Compare unrounded true distance from a prepared source to a physical threshold.
    ///
    /// # Errors
    /// Uses [`Self::within_physical`]'s exact inclusive comparator and refusals.
    pub fn within_physical_from_prepared(
        &self,
        source: &PreparedGeodesicPoint,
        point: &LonLat,
        threshold: &Metres,
        context: &mut MetricContext,
    ) -> Result<bool, GeoError> {
        self.physical_from_point_observed(source, point, threshold, context, None)
    }

    /// The same physical comparison with external governor charging.
    ///
    /// # Errors
    /// Adds observer refusals to [`Self::within_physical_from_prepared`].
    pub fn within_physical_from_prepared_metered(
        &self,
        source: &PreparedGeodesicPoint,
        point: &LonLat,
        threshold: &Metres,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<bool, GeoError> {
        self.physical_from_point_observed(source, point, threshold, context, Some(observer))
    }

    fn physical_from_point_observed(
        &self,
        source: &PreparedGeodesicPoint,
        point: &LonLat,
        threshold: &Metres,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<bool, GeoError> {
        self.within_physical_observed(
            source.source(),
            point,
            threshold,
            context,
            observer,
            Some(source),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::PreparedGeodesicPoint;
    use crate::{
        GeoError, GeographicReference, LonLat, MetricContext, MetricWorkObserver, PreparedGeodesic,
        Rat,
    };

    fn point(longitude: &str, latitude: &str) -> LonLat {
        LonLat::new(
            Rat::parse_decimal(longitude).unwrap(),
            Rat::parse_decimal(latitude).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn inline_prepared_endpoint_has_no_heap_wrapper_and_clone_survives_worker() {
        let mut context = MetricContext::wgs84().unwrap();
        let solver = PreparedGeodesic::prepare(GeographicReference::wgs84(), &mut context).unwrap();
        let original = point("116", "40");
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let prepared = solver.prepare_point(&original, &mut context).unwrap();
        assert_eq!(prepared.retained_workspace_bytes(), 0);
        let clone = prepared.clone();
        drop(prepared);
        assert_eq!(window.close().allocations, 0);
        drop(context);
        assert_eq!(clone.source(), &original);
        assert_eq!(clone.binding_id(), solver.reference().id());
        let mut worker = MetricContext::wgs84().unwrap();
        let target = point("116.1", "40.1");
        assert_eq!(
            solver
                .distance_from_prepared(&clone, &target, &mut worker)
                .unwrap(),
            solver.distance(&original, &target, &mut worker).unwrap()
        );
    }

    #[test]
    fn wide_prepared_endpoint_clone_keeps_original_limbs_after_worker_destruction() {
        for original in [
            point(
                "116.0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
                "40",
            ),
            point(
                "116",
                "40.0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
            ),
            point(
                "116",
                "-40.0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
            ),
        ] {
            let mut context = MetricContext::wgs84().unwrap();
            let solver =
                PreparedGeodesic::prepare(GeographicReference::wgs84(), &mut context).unwrap();
            let prepared = solver.prepare_point(&original, &mut context).unwrap();
            assert!(prepared.retained_workspace_bytes() > 0);
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let clone = prepared.clone();
            assert_eq!(window.close().allocations, 0);
            drop((prepared, context, solver));
            assert_eq!(clone.source(), &original);
            assert_eq!(clone.source().longitude(), original.longitude());
            assert_eq!(clone.source().latitude(), original.latitude());
        }
    }

    #[test]
    fn cached_source_preserves_true_answers_across_reflections_poles_and_cut_locus() {
        fn assert_shareable<T: Send + Sync>() {}
        assert_shareable::<PreparedGeodesicPoint>();
        let mut context = MetricContext::wgs84().unwrap();
        let solver = PreparedGeodesic::prepare(GeographicReference::wgs84(), &mut context).unwrap();
        for (a, b) in [
            (
                point("0", "36.530042355041"),
                point("5.762344694676510456", "-48.164270779097768864"),
            ),
            (
                point("0", "-36.530042355041"),
                point("-5.762344694676510456", "48.164270779097768864"),
            ),
            (
                point("0", "56.048002478584"),
                point("0.000002169207675262", "56.048000583672714595"),
            ),
            (point("42", "90"), point("-180", "-90")),
            (point("0", "0"), point("180", "0")),
            (
                point("0", "89.999999999999"),
                point("0.1", "89.999999999998"),
            ),
            (
                point("40", "89.999999999999999999999999999999"),
                point("-140", "89.999999999999999999999999999998"),
            ),
            (
                point("-140", "-89.999999999999999999999999999998"),
                point("40", "-89.999999999999999999999999999999"),
            ),
            (
                point("0", "36.530042355041000000000000000001"),
                point("2", "-36.530042355041000000000000000002"),
            ),
        ] {
            let source = solver.prepare_point(&a, &mut context).unwrap();
            assert_eq!(source.source(), &a);
            assert_eq!(source.binding_id(), solver.reference().id());
            let ordinary = solver.distance(&a, &b, &mut context).unwrap();
            let cached = solver
                .distance_from_prepared(&source, &b, &mut context)
                .unwrap();
            assert_eq!(ordinary, cached);
            assert_eq!(ordinary.certificate_bytes(), cached.certificate_bytes());
        }
    }

    #[derive(Default)]
    struct Charges {
        work: u64,
        bytes: u64,
    }
    impl MetricWorkObserver for Charges {
        fn charge_chunk(&mut self, work: u64, bytes: u64) -> Result<(), GeoError> {
            self.work += work;
            self.bytes += bytes;
            Ok(())
        }
    }

    #[test]
    fn point_preparation_and_cached_trial_charge_exact_cumulative_receipts() {
        let mut context = MetricContext::wgs84().unwrap();
        context.prepare_arithmetic().unwrap();
        let solver = PreparedGeodesic::prepare(GeographicReference::wgs84(), &mut context).unwrap();
        let a = point("0", "36.530042355041");
        let b = point("5.762344694676510456", "-48.164270779097768864");
        let mut charges = Charges::default();
        let source = solver
            .prepare_point_metered(&a, &mut context, &mut charges)
            .unwrap();
        assert_eq!(charges.work, context.work_items());
        assert_eq!(charges.bytes, context.workspace_peak());
        let mut charges = Charges::default();
        solver
            .distance_from_prepared_metered(&source, &b, &mut context, &mut charges)
            .unwrap();
        assert_eq!(charges.work, context.work_items());
        assert_eq!(charges.bytes, context.workspace_peak());
    }
}
