// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified shortest distances on an oblate ellipsoid of revolution.
//!
//! The auxiliary-sphere equations follow Karney, *Algorithms for geodesics*,
//! doi:10.1007/s00190-012-0578-z, equations (5)--(12) and (44)--(46). Series
//! coefficients are generated here from binomial and reciprocal equations;
//! neither coefficient arrays nor implementation bodies are imported.

pub(crate) mod arc_view;
mod astroid;
pub(crate) use arc_view::meridian_plane_degrees_admitted;
pub use arc_view::{ArcIntervalEnclosure, ArcIntervalView, PoleEventRange};
mod coefficients;
mod integral_fallback;
mod line;
pub(crate) mod normal;

mod band;
mod direct;
mod inverse;
mod point;
mod preparation;
mod seed;
use band::DistanceGoal;
pub(crate) use direct::canonical_endpoint;
pub use direct::{DirectOutputGrid, DirectResult, direct};
pub use inverse::{InverseProofReceipt, InverseResult, ShortestBranchMultiplicity, inverse};
pub use point::PreparedGeodesicPoint;
pub(crate) use preparation::WorkerCoefficientCache;

use purrdf_core::SmallVec;
use purrdf_xsd::{
    BigInt,
    math::{
        CertifiedInterval, CoordinateMath, FixedInterval, FloatEnclosure, IntervalBound,
        IntervalContext, MathError, MathLimits, WordInterval,
    },
};
use std::sync::Arc;

use crate::context::WorkProgress;
use crate::numerical::{exact_bounds, exact_bounds_in, fixed_from_rat, geo_math_error};
use crate::{
    GeoError, GeographicReference, LonLat, Metres, MetricContext, MetricEstimate,
    MetricProofReceipt, MetricWorkObserver, Rat, XsdDoubleMetres,
};

/// Immutable reference preparation, shareable between independent worker contexts.
#[derive(Clone, Debug)]
pub struct PreparedGeodesic {
    reference: GeographicReference,
    preparation: Option<Arc<preparation::PreparedCoefficients>>,
}

/// One immutable reference validation for a logical internal invocation.
/// Numerical samples retain the original direct solver and grid. This holds
/// no numerical scratch or floating guard across callbacks.
pub(crate) struct AdmittedGeodesicReference<'a> {
    prepared: &'a PreparedGeodesic,
    binding: crate::GeoBindingId,
}

impl PartialEq for PreparedGeodesic {
    fn eq(&self, other: &Self) -> bool {
        self.reference == other.reference
    }
}
impl Eq for PreparedGeodesic {}

impl PreparedGeodesic {
    pub(crate) fn admitted_reference(
        &self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<AdmittedGeodesicReference<'_>, GeoError> {
        let binding =
            crate::numerical::reference_identity(context, progress, Some(&self.reference))?;
        let target = crate::numerical::reference_identity(context, progress, None)?;
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if binding != target {
            return Err(GeoError::config(
                "prepared geodesic and metric context references differ",
            ));
        }
        crate::numerical::validate_angular_output_grid(
            DirectOutputGrid::DEGREE15.decimal_places(),
            self.reference
                .ellipsoid()
                .normal_metric_bounds_ref()
                .1
                .exact(),
            None,
            context,
            progress,
        )?;
        Ok(AdmittedGeodesicReference {
            prepared: self,
            binding,
        })
    }

    /// Prepare an exact ellipsoid and declared datum/axis binding.
    #[must_use]
    pub const fn new(reference: GeographicReference) -> Self {
        Self {
            reference,
            preparation: None,
        }
    }

    /// The immutable declared reference.
    #[must_use]
    pub const fn reference(&self) -> &GeographicReference {
        &self.reference
    }

    /// Compute one certified shortest distance with caller-owned admission.
    ///
    /// # Errors
    ///
    /// Refuses a mismatched reference, numerical environment, resource exhaustion
    /// or an unresolved true-distance rounding boundary.
    pub fn distance(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
    ) -> Result<MetricEstimate, GeoError> {
        context.begin(1)?;
        self.compute(a, b, context, None, None)
            .map(|(answer, _)| answer)
    }

    /// Compute a distance with an external governor/cancellation capability.
    ///
    /// The observer receives bounded numerical work outside floating guards.
    /// Its refusal prevents another chunk and the result is never published.
    ///
    /// # Errors
    ///
    /// Adds the observer's refusals to the ordinary distance contract.
    pub fn distance_metered(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<MetricEstimate, GeoError> {
        context.begin(1)?;
        self.compute(a, b, context, None, Some(observer))
            .map(|(answer, _)| answer)
    }

    /// Return separate invocation proof evidence alongside the unchanged answer.
    ///
    /// # Errors
    ///
    /// Uses the same admission and rounding contract as [`Self::distance`].
    pub fn distance_with_proof(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
    ) -> Result<(MetricEstimate, MetricProofReceipt), GeoError> {
        context.begin(1)?;
        let (answer, proof) = self.compute(a, b, context, None, None)?;
        let mut progress = WorkProgress::new(None);
        let mut proof = proof.into_shared_owned(context, &mut progress)?;
        proof.work_items = context.work_items();
        Ok((answer, proof))
    }

    /// Return the unchanged answer and invocation enclosure with governed work.
    ///
    /// # Errors
    /// Adds observer refusals to [`Self::distance_with_proof`].
    pub fn distance_with_proof_metered(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(MetricEstimate, MetricProofReceipt), GeoError> {
        context.begin(1)?;
        let (answer, proof) = self.compute(a, b, context, None, Some(observer))?;
        let mut continuation = crate::MetricWorkContinuation::new(
            observer,
            context.work_items(),
            context.workspace_peak(),
        );
        let mut progress = WorkProgress::new(Some(&mut continuation));
        let mut proof = proof.into_shared_owned(context, &mut progress)?;
        proof.work_items = context.work_items();
        Ok((answer, proof))
    }

    /// Compute a batch into an equally sized caller-owned result buffer.
    ///
    /// The environment is validated at batch entry. Any refusal aborts the batch;
    /// the caller must disregard every output slot unless the call succeeds.
    ///
    /// # Errors
    ///
    /// Refuses shape, reference, admission and uncertified numerical results.
    pub fn distance_batch(
        &self,
        points: &[(LonLat, LonLat)],
        output: &mut [Option<MetricEstimate>],
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.distance_batch_borrowed(points.iter().map(|(a, b)| (a, b)), output, context)
    }

    /// Borrow complete original inputs without a coordinate-copy allocation.
    ///
    /// # Errors
    /// Refuses length/reference/environment/resource failures and clears every
    /// result slot on refusal, including an iterator that misreports its length.
    pub fn distance_batch_borrowed<'a>(
        &self,
        points: impl ExactSizeIterator<Item = (&'a LonLat, &'a LonLat)>,
        output: &mut [Option<MetricEstimate>],
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.distance_batch_observed(points, output, context, None)
    }

    /// Borrowed caller-buffer distances with bounded external work charging.
    ///
    /// # Errors
    /// Adds the observer's typed refusal to [`Self::distance_batch_borrowed`].
    pub fn distance_batch_borrowed_metered<'a>(
        &self,
        points: impl ExactSizeIterator<Item = (&'a LonLat, &'a LonLat)>,
        output: &mut [Option<MetricEstimate>],
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.distance_batch_observed(points, output, context, Some(observer))
    }

    fn distance_batch_observed<'a>(
        &self,
        points: impl ExactSizeIterator<Item = (&'a LonLat, &'a LonLat)>,
        output: &mut [Option<MetricEstimate>],
        context: &mut MetricContext,
        mut observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<(), GeoError> {
        batch(points, output, context, |(a, b), context| {
            let result = if let Some(observer) = observer.as_deref_mut() {
                self.compute(a, b, context, None, Some(observer))
            } else {
                self.compute(a, b, context, None, None)
            };
            result.map(|(answer, _)| answer)
        })
    }

    /// Compare the unrounded physical distance to an exact metre threshold.
    ///
    /// # Errors
    ///
    /// Refuses an unresolved inclusive comparison, reference mismatch or
    /// operational exhaustion. Negative thresholds return false.
    pub fn within_physical(
        &self,
        center: &LonLat,
        point: &LonLat,
        threshold: &Metres,
        context: &mut MetricContext,
    ) -> Result<bool, GeoError> {
        self.within_physical_observed(center, point, threshold, context, None, None)
    }

    /// Compare true distance while charging an external governor between
    /// floating chunks through the same certified physical-threshold solver.
    ///
    /// # Errors
    /// Adds the observer's typed refusals to [`Self::within_physical`].
    pub fn within_physical_metered(
        &self,
        center: &LonLat,
        point: &LonLat,
        threshold: &Metres,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<bool, GeoError> {
        self.within_physical_observed(center, point, threshold, context, Some(observer), None)
    }

    #[allow(clippy::too_many_arguments)] // Exact inputs and optional original-source cache/observer capabilities.
    fn within_physical_observed(
        &self,
        center: &LonLat,
        point: &LonLat,
        threshold: &Metres,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
        source: Option<&PreparedGeodesicPoint>,
    ) -> Result<bool, GeoError> {
        context.begin(1)?;
        if threshold.exact() <= &Rat::zero() {
            context.charge_work(1)?;
            let mut progress = WorkProgress::new(observer);
            progress.initial()?;
            progress.context_poll(context)?;
            if !crate::numerical::reference_matches(&self.reference, context, &mut progress)? {
                return Err(GeoError::config(
                    "prepared geodesic and worker references differ",
                ));
            }
            let binding = crate::numerical::reference_identity(
                context,
                &mut progress,
                Some(&self.reference),
            )?;
            if source.is_some_and(|source| source.binding_id() != binding) {
                return Err(GeoError::config(
                    "prepared point and geodesic references differ",
                ));
            }
            if threshold.exact().signum() < 0 {
                return Ok(false);
            }
            // A positive-axis ellipsoid is a metric: its true distance is zero
            // exactly for original physical aliases. No numerical lower bound
            // or output quantization may identify distinct tiny neighbors.
            return crate::numerical::same_location_admitted(center, point, context, &mut progress);
        }
        let proof = self.compute_goal_cached(
            center,
            point,
            context,
            DistanceGoal::Point(Some(threshold.exact())),
            observer,
            source,
        )?;
        Ok(proof.upper.exact() <= threshold.exact())
    }

    fn compute(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        physical_threshold: Option<&Rat>,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<(MetricEstimate, MetricProofReceipt), GeoError> {
        self.compute_point_cached(a, b, context, physical_threshold, observer, None)
    }

    #[allow(clippy::too_many_arguments)] // Exact endpoints, comparator, optional preparation and admission.
    fn compute_point_cached(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        physical_threshold: Option<&Rat>,
        observer: Option<&mut dyn MetricWorkObserver>,
        source: Option<&PreparedGeodesicPoint>,
    ) -> Result<(MetricEstimate, MetricProofReceipt), GeoError> {
        let metered = observer.is_some();
        let mut progress = WorkProgress::new(observer);
        let mut proof = {
            let mut nested = progress.nested(0, 0, context.workspace_peak());
            self.compute_goal_cached(
                a,
                b,
                context,
                DistanceGoal::Point(physical_threshold),
                if metered { Some(&mut nested) } else { None },
                source,
            )?
        };
        let result = crate::metric::complete_point_enclosure_observed(
            proof.lower.exact(),
            proof.upper.exact(),
            self.reference.id(),
            context,
            &mut progress,
        )?;
        proof.work_items = context.work_items();
        Ok((result, proof))
    }

    #[allow(clippy::too_many_arguments)] // Original source plus its optional immutable proof preparation.
    fn compute_goal_cached(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        goal: DistanceGoal<'_>,
        observer: Option<&mut dyn MetricWorkObserver>,
        source: Option<&PreparedGeodesicPoint>,
    ) -> Result<MetricProofReceipt, GeoError> {
        context.charge_work(1)?;
        let mut chunks = ChunkObserver::new(observer);
        chunks.initial().map_err(solve_external_error)?;
        if !chunks.reference_matches(&self.reference, context)? {
            return Err(GeoError::config(
                "prepared geodesic and metric context references differ",
            ));
        }
        let binding = chunks.reference_identity(Some(&self.reference), context)?;
        if source.is_some_and(|source| source.binding_id() != binding) {
            return Err(GeoError::config(
                "prepared endpoint and geodesic references differ",
            ));
        }
        if a.same_location(b) {
            let zero = Rat::zero();
            return Ok(MetricProofReceipt {
                lower: Metres::new(zero.clone()),
                upper: Metres::new(zero),
                precision_bits: 0,
                work_items: context.work_items(),
            });
        }
        let canonical = CanonicalPair::new(a, b);
        let short_upper = canonical.path_distance_upper(self.reference.ellipsoid());
        let half_micrometre = Rat::new(crate::Int::one(), crate::Int::from_i64(2_000_000))
            .expect("positive quantization denominator");
        if let Some(short_upper) = short_upper
            && short_upper <= half_micrometre
            && goal.exact_decisive(&Rat::zero(), &short_upper)
        {
            return Ok(MetricProofReceipt {
                lower: Metres::new(Rat::zero()),
                upper: Metres::new(short_upper),
                precision_bits: 0,
                work_items: context.work_items(),
            });
        }
        let policy = context.policy();
        let maximum = policy.limits().max_precision_bits;
        // Near-equatorial vertex geometry can lose many working bits in the
        // inverse square root. The same certified solver starts its fixed
        // proof at112 bits there, avoiding an unproductive80-bit attempt.
        // This selects proof effort; the output grid and completion test stay
        // independent of this proposal and of the caller's admission limits.
        let mut bits = maximum.min(80);
        let mut floating = maximum >= 64;
        let mut double_word = false;
        if bits < 16 {
            return Err(GeoError::PrecisionExhausted { bits: maximum });
        }
        loop {
            chunks.reset();
            if !floating {
                chunks.prepare_scratch(
                    context,
                    scratch_source_bits(
                        &[a.longitude(), a.latitude(), b.longitude(), b.latitude()],
                        self.reference.ellipsoid(),
                    ),
                )?;
            }
            let fixed_coefficients = if floating {
                None
            } else {
                self.worker_coefficients(
                    bits,
                    integral_order(self.reference.ellipsoid(), bits),
                    context,
                    &mut chunks,
                )?
            };
            chunks
                .context_entry(context)
                .map_err(solve_external_error)?;
            let remaining = policy
                .limits()
                .max_work_items
                .saturating_sub(context.work_items());
            let mut math = context
                .coordinate_math(MathLimits {
                    precision_bits: bits,
                    max_work: remaining,
                    max_workspace_bytes: usize::try_from(context.remaining_workspace()).map_err(
                        |_| GeoError::MemoryExhausted {
                            limit: policy.limits().max_workspace_bytes,
                        },
                    )?,
                })
                .map_err(|error| geo_math_error(&error, policy))?;
            let order = integral_order(self.reference.ellipsoid(), bits);
            let interval_bytes = u64::from(bits.div_ceil(8))
                .saturating_mul(8)
                .saturating_add(128);
            let retained_bytes = interval_bytes.saturating_mul(2 * (order as u64 + 1) + 64);
            let retained_bytes =
                usize::try_from(retained_bytes).map_err(|_| GeoError::MemoryExhausted {
                    limit: policy.limits().max_workspace_bytes,
                })?;
            math.reserve_workspace(retained_bytes)
                .map_err(|error| geo_math_error(&error, policy))?;
            let enclosure = if floating && double_word {
                // The double-word tier: about 106 bits from the same validated
                // binary64 operations, for enclosures binary64 cannot resolve.
                let mut arithmetic = IntervalContext::binary64(&mut math)
                    .map_err(|error| geo_math_error(&error, policy))?;
                solve_distance_with::<WordInterval>(
                    &InverseProblem {
                        pair: &canonical,
                        reference: &self.reference,
                        order,
                        iterations: policy.limits().max_iterations.min(16),
                        goal,
                        capture: false,
                        source,
                        prepared: self.preparation.as_ref().map(|prepared| &prepared.double),
                    },
                    &mut chunks,
                    &mut arithmetic,
                )
                .map_err(|error| {
                    if matches!(error, SolveError::Iterations) {
                        SolveError::Math(MathError::PrecisionExhausted)
                    } else {
                        error
                    }
                })
                .and_then(|solution| {
                    solution
                        .distance
                        .to_fixed(&mut arithmetic)
                        .map_err(SolveError::Math)
                })
            } else if floating {
                context
                    .install_arithmetic(&mut math)
                    .map_err(|error| geo_math_error(&error, policy))?;
                let mut arithmetic = IntervalContext::binary64(&mut math)
                    .map_err(|error| geo_math_error(&error, policy))?;
                solve_distance_with::<FloatEnclosure>(
                    &InverseProblem {
                        pair: &canonical,
                        reference: &self.reference,
                        order,
                        iterations: policy.limits().max_iterations.min(16),
                        goal,
                        capture: false,
                        source,
                        prepared: self.preparation.as_ref().map(|prepared| &prepared.floating),
                    },
                    &mut chunks,
                    &mut arithmetic,
                )
                .map_err(|error| {
                    if matches!(error, SolveError::Iterations)
                        && policy.limits().max_iterations > 16
                    {
                        // Binary64 is a bounded proof attempt. A residual
                        // rounding enclosure proceeds to the original exact
                        // fixed-point solver before consuming its admission.
                        SolveError::Math(MathError::PrecisionExhausted)
                    } else {
                        error
                    }
                })
                .and_then(|solution| {
                    solution
                        .distance
                        .to_fixed(&mut arithmetic)
                        .map_err(SolveError::Math)
                })
            } else {
                solve_distance_with::<FixedInterval>(
                    &InverseProblem {
                        pair: &canonical,
                        reference: &self.reference,
                        order,
                        iterations: policy.limits().max_iterations,
                        goal,
                        capture: false,
                        source,
                        prepared: fixed_coefficients.as_deref(),
                    },
                    &mut chunks,
                    &mut IntervalContext::fixed(&mut math),
                )
                .map(|solution| solution.distance)
            };
            let observer_result = chunks.finish(&math);
            math.release_workspace(retained_bytes)
                .map_err(|error| geo_math_error(&error, policy))?;
            context.charge_work(math.work_used())?;
            context.admit_workspace(math.workspace_peak() as u64)?;
            context.release_workspace(math.workspace_peak() as u64)?;
            observer_result.map_err(solve_external_error)?;
            match enclosure {
                Ok(enclosure) => {
                    let (lower, upper) = exact_bounds(&enclosure);
                    let proof_bits = match (floating, double_word) {
                        (true, false) => 53,
                        (true, true) => 106,
                        (false, _) => bits,
                    };
                    let lower = lower.max(Rat::zero());
                    if goal.exact_decisive(&lower, &upper) {
                        return Ok(MetricProofReceipt {
                            lower: Metres::new(lower),
                            upper: Metres::new(upper),
                            precision_bits: proof_bits,
                            work_items: context.work_items(),
                        });
                    }
                }
                Err(SolveError::Math(MathError::PrecisionExhausted | MathError::Domain(_))) => {}
                Err(SolveError::Iterations) => {
                    return Err(GeoError::ConvergenceExhausted {
                        iterations: policy.limits().max_iterations,
                    });
                }
                Err(SolveError::External(error)) => return Err(error),
                Err(SolveError::Math(error)) => return Err(geo_math_error(&error, policy)),
            }
            if floating && !double_word {
                double_word = true;
                continue;
            }
            if floating {
                // The double-word tier already carries about 100 working bits,
                // so the exact solver resumes beyond it rather than at 80.
                floating = false;
                bits = maximum.min(initial_inverse_precision(&canonical).max(112));
                continue;
            }
            if bits == maximum {
                return Err(GeoError::PrecisionExhausted { bits: maximum });
            }
            bits = next_precision(bits, maximum);
        }
    }
}

impl PreparedGeodesic {
    /// One certified binary64 enclosure of the true shortest distance from a
    /// prepared native endpoint, in metres, without micrometre rounding.
    ///
    /// Only the deterministic binary64 proof runs, under a fixed internal
    /// numerical budget independent of any caller policy, so the same inputs
    /// give the same enclosure, or `None`, on every host. `None` means that
    /// proof did not complete; it is never an approximation.
    ///
    /// # Errors
    /// Refuses an invalid floating environment or a prepared-reference mismatch.
    pub(crate) fn binary64_enclosure(
        &self,
        source: &PreparedGeodesicPoint,
        point: &LonLat,
        context: &mut MetricContext,
    ) -> Result<Option<(f64, f64)>, GeoError> {
        if source.source().same_location(point) {
            return Ok(Some((0.0, 0.0)));
        }
        let canonical = CanonicalPair::new(source.source(), point);
        let policy = context.policy();
        let order = integral_order(self.reference.ellipsoid(), 80);
        let mut math = context
            .coordinate_math(MathLimits {
                precision_bits: 80,
                max_work: ENCLOSURE_WORK,
                max_workspace_bytes: ENCLOSURE_WORKSPACE,
            })
            .map_err(|error| geo_math_error(&error, policy))?;
        context
            .install_arithmetic(&mut math)
            .map_err(|error| geo_math_error(&error, policy))?;
        let mut chunks = ChunkObserver::new(None);
        let solved = {
            let mut arithmetic = IntervalContext::binary64(&mut math)
                .map_err(|error| geo_math_error(&error, policy))?;
            solve_distance_with::<FloatEnclosure>(
                &InverseProblem {
                    pair: &canonical,
                    reference: &self.reference,
                    order,
                    iterations: 16,
                    goal: DistanceGoal::Enclosure,
                    capture: false,
                    source: Some(source),
                    prepared: self.preparation.as_ref().map(|prepared| &prepared.floating),
                },
                &mut chunks,
                &mut arithmetic,
            )
        };
        match solved {
            Ok(solution) => Ok(Some(solution.distance.bounds())),
            Err(SolveError::Math(_) | SolveError::Iterations) => Ok(None),
            Err(SolveError::External(error)) => Err(error),
        }
    }
}

/// Fixed numerical budget of one internal binary64 enclosure. It bounds the
/// proof's own effort; it is not caller admission and enters no identity.
const ENCLOSURE_WORK: u64 = 1 << 20;
const ENCLOSURE_WORKSPACE: usize = 1 << 20;

/// WGS84 shortest distance, correctly rounded half-even to one micrometre.
///
/// # Errors
///
/// Refuses operational exhaustion and unresolved rounding; antipodal points are
/// valid and retain the same true-shortest-distance contract.
#[allow(clippy::needless_pass_by_value)] // Owned convenience API; prepared methods borrow inputs.
pub fn distance(a: LonLat, b: LonLat) -> Result<MetricEstimate, GeoError> {
    let (prepared, mut context) = native_wgs84()?;
    prepared.distance(&a, &b, &mut context)
}

/// The process-wide immutable WGS84 preparation behind the plain functions.
///
/// Coefficient tables and binary64 constants depend only on the exact native
/// ellipsoid, so one preparation serves every call. Each call still receives a
/// fresh default-policy context: admission, cancellation and the floating
/// environment check stay per call, exactly as with a caller-built context.
fn native_wgs84() -> Result<(&'static PreparedGeodesic, MetricContext), GeoError> {
    let (prepared, arithmetic) = native(&GeographicReference::wgs84())?;
    let mut context = MetricContext::wgs84()?;
    context.set_prepared_arithmetic(arithmetic.clone());
    Ok((prepared, context))
}

type NativePreparation = (PreparedGeodesic, purrdf_xsd::math::PreparedBinary64);

/// The immutable preparation of a native WGS84 or CGCS2000 reference.
fn native(reference: &GeographicReference) -> Result<&'static NativePreparation, GeoError> {
    static WGS84: std::sync::OnceLock<NativePreparation> = std::sync::OnceLock::new();
    static CGCS2000: std::sync::OnceLock<NativePreparation> = std::sync::OnceLock::new();
    let cell = if *reference == GeographicReference::wgs84() {
        &WGS84
    } else if *reference == GeographicReference::cgcs2000() {
        &CGCS2000
    } else {
        return Err(GeoError::config("not a native geographic reference"));
    };
    if let Some(native) = cell.get() {
        return Ok(native);
    }
    let mut context = MetricContext::new(reference.clone(), crate::ExecutionPolicy::geometry())?;
    let arithmetic = context.prepare_arithmetic()?;
    let prepared = PreparedGeodesic::prepare(reference.clone(), &mut context)?;
    // A concurrent first call may have stored an equal preparation.
    let _ = cell.set((prepared, arithmetic));
    Ok(cell.get().expect("native preparation stored above"))
}

/// The shared immutable geodesic preparation of a native reference.
///
/// # Errors
/// Refuses a non-native reference or an invalid floating environment.
pub(crate) fn native_prepared(
    reference: &GeographicReference,
) -> Result<&'static PreparedGeodesic, GeoError> {
    native(reference).map(|native| &native.0)
}

/// The exact public reported-double comparison used by SPARQL distance filters.
///
/// # Errors
///
/// Finite negative thresholds return false. Other inputs use the distance law's
/// numerical and operational refusals.
#[allow(clippy::needless_pass_by_value)] // Owned convenience API; prepared methods borrow inputs.
pub fn within(center: LonLat, point: LonLat, threshold: XsdDoubleMetres) -> Result<bool, GeoError> {
    let (prepared, mut context) = native_wgs84()?;
    if threshold.is_negative() {
        return Ok(false);
    }
    prepared
        .distance(&center, &point, &mut context)
        .map(|estimate| estimate.within_reported(threshold))
}

/// Compare the true WGS84 shortest distance to an exact physical threshold.
///
/// # Errors
///
/// Refuses an undecidable threshold boundary and the ordinary numerical limits.
#[allow(clippy::needless_pass_by_value)] // Owned convenience API; prepared methods borrow inputs.
pub fn within_physical(center: LonLat, point: LonLat, threshold: Metres) -> Result<bool, GeoError> {
    let (prepared, mut context) = native_wgs84()?;
    prepared.within_physical(&center, &point, &threshold, &mut context)
}

struct CanonicalPair {
    latitude1: Rat,
    latitude2: Rat,
    longitude: Rat,
    swapped: bool,
    latitude_sign: i64,
    longitude_sign: i64,
}

impl CanonicalPair {
    fn new(a: &LonLat, b: &LonLat) -> Self {
        let (mut latitude1, mut latitude2) = (a.latitude().clone(), b.latitude().clone());
        let swapped = latitude1.abs() < latitude2.abs();
        if swapped {
            core::mem::swap(&mut latitude1, &mut latitude2);
        }
        let latitude_sign = if latitude1 > Rat::zero() { -1 } else { 1 };
        if latitude_sign < 0 {
            latitude1 = latitude1.neg();
            latitude2 = latitude2.neg();
        }
        let longitude1 = if a.is_pole() {
            Rat::zero()
        } else {
            a.longitude().clone()
        };
        let longitude2 = if b.is_pole() {
            Rat::zero()
        } else {
            b.longitude().clone()
        };
        let delta = if swapped {
            crate::geographic::longitude_difference(&longitude2, &longitude1)
        } else {
            crate::geographic::longitude_difference(&longitude1, &longitude2)
        };
        let longitude_sign = if delta < Rat::zero() { -1 } else { 1 };
        Self {
            latitude1,
            latitude2,
            longitude: delta.abs(),
            swapped,
            latitude_sign,
            longitude_sign,
        }
    }

    fn path_distance_upper(&self, ellipsoid: &crate::PreparedEllipsoid) -> Option<Rat> {
        // Coordinate meridians/parallels and either two-meridian pole route
        // are admissible surface paths. The Gauss-map metric is bounded above
        // by R=a²/b, and pi<22/7 is a strict rational enclosure. Therefore the
        // shortest distance is at most this exact bound even at chart poles.
        let angular_span = self
            .latitude2
            .sub(&self.latitude1)
            .abs()
            .add(&self.longitude);
        let polar_span = Rat::from_i64(180).sub(&self.latitude1.add(&self.latitude2).abs());
        let span = angular_span.min(polar_span);
        if span
            > Rat::new(crate::Int::one(), crate::Int::from_i64(1_000_000_000)).expect("positive")
        {
            return None;
        }
        let (_, radius) = ellipsoid.normal_metric_bounds();
        Some(radius.exact().mul(&span).mul(
            &Rat::new(crate::Int::from_i64(22), crate::Int::from_i64(1260)).expect("positive"),
        ))
    }
}

// These independent exact symmetry and conditioning predicates select
// mathematical reductions; none represents an operational state machine.
#[allow(clippy::struct_excessive_bools)]
struct SpherePair<I: CertifiedInterval = FixedInterval> {
    sin1: I,
    cos1: I,
    sin2: I,
    cos2: I,
    h1: I,
    h2: I,
    same_latitude: bool,
    opposite_latitude: bool,
    short: bool,
    polar: bool,
}

#[derive(Debug)]
enum SolveError {
    Math(MathError),
    Iterations,
    External(GeoError),
}

impl From<MathError> for SolveError {
    fn from(error: MathError) -> Self {
        Self::Math(error)
    }
}

enum ChunkProgress<'a> {
    Owned(WorkProgress<'a>),
    Borrowed(&'a mut dyn FnMut(&CoordinateMath) -> Result<(), MathError>),
}
struct ChunkObserver<'a> {
    progress: ChunkProgress<'a>,
    metered: bool,
    entry_work: u64,
    entry_workspace: u64,
    first_phase: bool,
}

impl<'a> ChunkObserver<'a> {
    fn new(observer: Option<&'a mut dyn MetricWorkObserver>) -> Self {
        let metered = observer.is_some();
        Self {
            progress: ChunkProgress::Owned(WorkProgress::new(observer)),
            metered,
            entry_work: 0,
            entry_workspace: 0,
            first_phase: true,
        }
    }
    fn borrowed(poll: &'a mut dyn FnMut(&CoordinateMath) -> Result<(), MathError>) -> Self {
        Self {
            progress: ChunkProgress::Borrowed(poll),
            metered: true,
            entry_work: 0,
            entry_workspace: 0,
            first_phase: true,
        }
    }
    fn into_owned_progress(self) -> Option<WorkProgress<'a>> {
        match self.progress {
            ChunkProgress::Owned(progress) => Some(progress),
            ChunkProgress::Borrowed(_) => None,
        }
    }
    fn initial(&mut self) -> Result<(), SolveError> {
        match &mut self.progress {
            ChunkProgress::Owned(progress) => {
                self.entry_work = 1;
                progress.initial().map_err(SolveError::External)
            }
            ChunkProgress::Borrowed(_) => Ok(()),
        }
    }
    fn context_entry(&mut self, context: &MetricContext) -> Result<(), SolveError> {
        if let ChunkProgress::Owned(progress) = &mut self.progress {
            progress
                .context_poll(context)
                .map_err(SolveError::External)?;
        }
        // Caller-buffer rows share one context and progress. Offset a fresh
        // numerical phase by its cumulative entry, including exact preflight;
        // every row and every retained/scratch peak is charged exactly once.
        self.entry_work = context.work_items();
        self.entry_workspace = context
            .policy()
            .limits()
            .max_workspace_bytes
            .saturating_sub(context.remaining_workspace());
        self.first_phase = true;
        Ok(())
    }
    // Native public entries own their original progress. Mathematical borrowed
    // chunks enter through the caller's already admitted reference phase.
    fn reference_identity(
        &mut self,
        reference: Option<&GeographicReference>,
        context: &mut MetricContext,
    ) -> Result<crate::GeoBindingId, GeoError> {
        let ChunkProgress::Owned(progress) = &mut self.progress else {
            return Err(GeoError::config("native entry requires its owned observer"));
        };
        crate::numerical::reference_identity(context, progress, reference)
    }
    fn reference_matches(
        &mut self,
        reference: &GeographicReference,
        context: &mut MetricContext,
    ) -> Result<bool, GeoError> {
        let ChunkProgress::Owned(progress) = &mut self.progress else {
            return Err(GeoError::config("native entry requires its owned observer"));
        };
        crate::numerical::reference_matches(reference, context, progress)
    }
    fn prepare_scratch(
        &mut self,
        context: &mut MetricContext,
        source_bits: u64,
    ) -> Result<(), GeoError> {
        match &mut self.progress {
            ChunkProgress::Owned(progress) => {
                context.prepare_integer_scratch_for_observed(source_bits, progress)
            }
            ChunkProgress::Borrowed(_) => context.prepare_integer_scratch_for(source_bits),
        }
    }
    fn finish_interval(&mut self, math: &mut IntervalContext<'_>) -> Result<(), SolveError> {
        if self.metered {
            self.finish(math.pause())?;
            math.resume()?;
        } else {
            self.finish(math.math())?;
        }
        Ok(())
    }
    fn reset(&mut self) {
        // The first phase retains its already observed native entry charge.
        // Later precision attempts start fresh numerical counters and must not
        // charge that entry again. Workspace peaks remain cumulative.
        if self.first_phase {
            self.first_phase = false;
        } else if let ChunkProgress::Owned(progress) = &mut self.progress {
            progress.reset();
            self.entry_work = 0;
        }
    }
    fn finish(&mut self, math: &CoordinateMath) -> Result<(), SolveError> {
        match &mut self.progress {
            ChunkProgress::Owned(progress) => progress
                .charge_counts(
                    self.entry_work.saturating_add(math.work_used()),
                    self.entry_workspace
                        .saturating_add(math.workspace_peak() as u64),
                )
                .map_err(SolveError::External),
            ChunkProgress::Borrowed(poll) => poll(math).map_err(SolveError::Math),
        }
    }
}

fn solve_external_error(error: SolveError) -> GeoError {
    match error {
        SolveError::External(error) => error,
        SolveError::Math(_) | SolveError::Iterations => {
            GeoError::ArithmeticOverflow("invalid observer failure")
        }
    }
}

pub(super) use crate::numerical::scratch_source_bits;

/// Select proof degree from an exact coefficient-tail bound. Native axes
/// satisfy e'²<=1/128, so q^(N+1)/(1-q) contributes at least seven bits per
/// term. No completed output precision or mathematical law depends on this
/// implementation choice; the evaluator still checks its full enclosure.
fn next_precision(bits: u32, maximum: u32) -> u32 {
    if bits < 112 {
        112.min(maximum)
    } else {
        bits.saturating_mul(2).min(maximum)
    }
}

fn integral_order(ellipsoid: &crate::PreparedEllipsoid, bits: u32) -> usize {
    if bits <= 80 {
        8
    } else if ellipsoid.eccentricity_squared().mul(&Rat::from_i64(129)) <= Rat::one() {
        bits.div_ceil(7) as usize
    } else {
        bits.div_ceil(4) as usize
    }
}

fn initial_inverse_precision(pair: &CanonicalPair) -> u32 {
    // Cut-locus neighbors can have very small reduced length: their inverse
    // azimuth is ill-conditioned although shortest distance remains regular.
    // Exact source coordinates select a more precise first proof, avoiding
    // work on coarse enclosures which cannot contract the inverse bracket.
    let latitude_sum = pair.latitude1.add(&pair.latitude2).abs();
    if pair.longitude > Rat::from_i64(170)
        && !latitude_sum.is_zero()
        && latitude_sum
            < Rat::one()
                .div(&Rat::from_i64(1_000_000_000_000))
                .expect("positive")
    {
        448
    } else if pair.longitude > Rat::from_i64(170)
        && latitude_sum < Rat::one().div(&Rat::from_i64(1_000_000)).expect("positive")
    {
        224
    } else if pair.latitude1.abs() <= Rat::from_i64(1) {
        112
    } else {
        80
    }
}

/// Signed equator-to-latitude meridian arc using the shared certified integrals.
#[cfg(test)]
pub(crate) fn meridian_arc(
    latitude: &Rat,
    ellipsoid: &crate::PreparedEllipsoid,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let mut chunks = ChunkObserver::new(None);
    meridian_arc_with(latitude, ellipsoid, None, &mut chunks, math).map_err(meridian_math_error)
}

/// The same integral using an explicit worker's immutable coefficient cache.
/// New preparation is observed before allocation and retained by its phase.
pub(crate) fn meridian_arc_observed(
    latitude: &Rat,
    ellipsoid: &crate::PreparedEllipsoid,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    let coefficients = prepare_integral_coefficients_observed(ellipsoid, math, progress)?;
    let mut poll = |math: &CoordinateMath| progress.math_poll(math);
    let mut chunks = ChunkObserver::borrowed(&mut poll);
    meridian_arc_with(
        latitude,
        ellipsoid,
        Some(coefficients.as_ref()),
        &mut chunks,
        math,
    )
    .map_err(meridian_math_error)
}

/// Acquire equivalent immutable distance/longitude tables from the worker's
/// one coefficient cache. Their equations depend only on the ellipsoid and
/// precision, so all prepared lines in a phase share this detached ownership.
pub(super) fn prepare_integral_coefficients_observed(
    ellipsoid: &crate::PreparedEllipsoid,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Arc<IntegralCoefficients>, MathError> {
    let bits = math.limits().precision_bits;
    let order = integral_order(ellipsoid, bits);
    let previous = progress.prepared_coefficients().cloned();
    let mut coefficients = if let Some(cache) = &previous {
        let cost = cache
            .lookup_cost(ellipsoid.inverse_flattening())
            .ok_or(MathError::WorkExhausted)?;
        math.admit_exact_cost(cost)?;
        progress.math_poll(math)?;
        cache
            .find(
                ellipsoid.inverse_flattening(),
                bits,
                order,
                preparation::CoefficientUse::Meridian,
            )
            .cloned()
    } else {
        None
    };
    if coefficients.is_none() {
        let bytes = meridian_workspace_bytes(bits, order)?;
        math.reserve_workspace(bytes)?;
        let prepared = {
            let mut poll = |math: &CoordinateMath| progress.math_poll(math);
            let mut chunks = ChunkObserver::borrowed(&mut poll);
            preparation::prepare_worker_coefficients(
                ellipsoid,
                bits,
                order,
                preparation::CoefficientUse::Meridian,
                previous,
                &mut chunks,
                math,
            )
        };
        math.release_workspace(bytes)?;
        let (cache, prepared, retained) = prepared.map_err(meridian_math_error)?;
        math.reserve_workspace(
            usize::try_from(retained).map_err(|_| MathError::WorkspaceExhausted)?,
        )?;
        progress.retain_prepared_coefficients(cache, retained)?;
        coefficients = Some(prepared);
    }
    coefficients.ok_or(MathError::PrecisionExhausted)
}

fn meridian_workspace_bytes(bits: u32, order: usize) -> Result<usize, MathError> {
    let intervals = order
        .checked_add(1)
        .and_then(|value| value.checked_mul(2))
        .and_then(|value| value.checked_add(64))
        .ok_or(MathError::WorkspaceExhausted)?;
    usize::try_from(bits.div_ceil(8))
        .ok()
        .and_then(|value| value.checked_mul(8))
        .and_then(|value| value.checked_add(128))
        .and_then(|value| value.checked_mul(intervals))
        .ok_or(MathError::WorkspaceExhausted)
}

fn meridian_math_error(error: SolveError) -> MathError {
    match error {
        SolveError::Math(error) => error,
        SolveError::Iterations => MathError::ConvergenceExhausted { iterations: 0 },
        SolveError::External(_) => MathError::Domain("unexpected external meridian observer"),
    }
}

fn meridian_arc_with(
    latitude: &Rat,
    ellipsoid: &crate::PreparedEllipsoid,
    prepared: Option<&IntegralCoefficients>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, SolveError> {
    let order = integral_order(ellipsoid, math.limits().precision_bits);
    let bytes = meridian_workspace_bytes(math.limits().precision_bits, order)?;
    math.reserve_workspace(bytes)?;
    let result = (|| -> Result<FixedInterval, SolveError> {
        let a = fixed_from_rat(ellipsoid.semimajor(), math)?;
        let b = fixed_from_rat(ellipsoid.semiminor(), math)?;
        let c = b.div(&a, math)?;
        let ep2 =
            fixed_from_rat(&ellipsoid.eccentricity_squared(), math)?.div(&c.square(math)?, math)?;
        let radians_per_degree =
            FixedInterval::pi(math)?.div(&FixedInterval::from_i64(180, math)?, math)?;
        let (sin, cos) = reduced_latitude(latitude, &c, &radians_per_degree, math)?;
        let sigma = FixedInterval::atan2(&sin, &cos, math)?;
        let zero = FixedInterval::from_i64(0, math)?;
        let generated;
        let coefficients = if let Some(prepared) = prepared {
            prepared
        } else {
            generated = IntegralCoefficients::new(&c, order, chunks, math)?;
            &generated
        };
        Ok(coefficients
            .evaluate(&zero, &sigma, &ep2, chunks, math)?
            .distance
            .mul(&b, math)?)
    })();
    math.release_workspace(bytes)?;
    result
}

/// Monotone signed meridian image of an unquantized degree enclosure.
/// Exact dyadic endpoints feed the same original integral as scalar callers.
pub(crate) fn meridian_arc_interval_observed(
    latitude_degrees: &FixedInterval,
    ellipsoid: &crate::PreparedEllipsoid,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<FixedInterval, MathError> {
    let bits = latitude_degrees
        .lower()
        .bits_upper_bound()
        .max(latitude_degrees.upper().bits_upper_bound())
        .saturating_add(latitude_degrees.precision_bits() as usize)
        .saturating_mul(4);
    math.admit_exact(16, bits)?;
    progress.math_poll(math)?;
    let (lower, upper) = exact_bounds_in(latitude_degrees, math)?;
    if lower < Rat::from_i64(-90) || upper > Rat::from_i64(90) {
        return Err(MathError::Domain("meridian latitude outside [-90,90]"));
    }
    let first = meridian_arc_observed(&lower, ellipsoid, math, progress)?;
    if lower == upper {
        return Ok(first);
    }
    let last = meridian_arc_observed(&upper, ellipsoid, math, progress)?;
    FixedInterval::from_bounds(first.lower().clone(), last.upper().clone(), math)
}

fn interval_from_rat<I: CertifiedInterval>(
    value: &Rat,
    math: &mut IntervalContext<'_>,
) -> Result<I, MathError> {
    I::from_ratio(
        &BigInt::from(value.numerator().clone()),
        &BigInt::from(value.denominator().clone()),
        math,
    )
}

// The one complete caller-buffer traversal shared by distance, inverse and
// direct propagation. It owns shape checks, entry validation, all-output refusal
// and verifies an external ExactSizeIterator cannot publish a truncated batch.
fn batch<Input, Output: Clone>(
    mut inputs: impl ExactSizeIterator<Item = Input>,
    output: &mut [Option<Output>],
    context: &mut MetricContext,
    mut compute: impl FnMut(Input, &mut MetricContext) -> Result<Output, GeoError>,
) -> Result<(), GeoError> {
    output.fill(None);
    let expected = inputs.len();
    if expected != output.len() {
        return Err(GeoError::InvalidOutputLength {
            expected,
            actual: output.len(),
        });
    }
    context.begin(expected)?;
    let result = (|| {
        for (index, slot) in output.iter_mut().enumerate() {
            let input = inputs.next().ok_or(GeoError::InvalidOutputLength {
                expected,
                actual: index,
            })?;
            // No numerical guard crosses the iterator callback.
            context.checkpoint()?;
            *slot = Some(compute(input, context)?);
        }
        let extra = inputs.next();
        context.checkpoint()?;
        if extra.is_some() {
            return Err(GeoError::InvalidOutputLength {
                expected: expected
                    .checked_add(1)
                    .ok_or(GeoError::ArithmeticOverflow("batch iterator length"))?,
                actual: expected,
            });
        }
        Ok(())
    })();
    if result.is_err() {
        output.fill(None);
    }
    result
}

#[cfg(test)]
fn point_decisive(
    distance: &FixedInterval,
    threshold: Option<&FixedInterval>,
    math: &mut CoordinateMath,
) -> Result<bool, MathError> {
    point_decisive_with(distance, threshold, &mut IntervalContext::fixed(math))
}

fn reduced_latitude(
    latitude: &Rat,
    c: &FixedInterval,
    radians_per_degree: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<(FixedInterval, FixedInterval), MathError> {
    reduced_latitude_with(
        latitude,
        c,
        radians_per_degree,
        &mut IntervalContext::fixed(math),
    )
}

impl IntegralCoefficients<FixedInterval> {
    fn new(
        c: &FixedInterval,
        order: usize,
        chunks: &mut ChunkObserver<'_>,
        math: &mut CoordinateMath,
    ) -> Result<Self, SolveError> {
        Self::new_with(c, order, chunks, &mut IntervalContext::fixed(math))
    }
    fn evaluate(
        &self,
        sigma1: &FixedInterval,
        sigma2: &FixedInterval,
        q: &FixedInterval,
        chunks: &mut ChunkObserver<'_>,
        math: &mut CoordinateMath,
    ) -> Result<Integrals, SolveError> {
        self.evaluate_with(sigma1, sigma2, q, chunks, &mut IntervalContext::fixed(math))
    }
}

struct InverseProblem<'a, I: CertifiedInterval> {
    pair: &'a CanonicalPair,
    reference: &'a GeographicReference,
    order: usize,
    iterations: u32,
    goal: DistanceGoal<'a>,
    prepared: Option<&'a IntegralCoefficients<I>>,
    capture: bool,
    source: Option<&'a PreparedGeodesicPoint>,
}

fn solve_distance_with<I: point::PointInterval>(
    problem: &InverseProblem<'_, I>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<inverse::InverseSolution<I>, SolveError> {
    let latitude_sum = problem.pair.latitude1.add(&problem.pair.latitude2);
    if !problem.capture
        && math.limits().precision_bits <= 224
        && !latitude_sum.is_zero()
        && problem.pair.longitude > Rat::from_i64(170)
        && latitude_sum.abs()
            < Rat::one()
                .div(&Rat::from_i64(10_000_000_000))
                .expect("positive")
    {
        // Shortest distance is Lipschitz in each endpoint. Projecting only
        // the second latitude to the exact opposite-latitude branch changes
        // it by at most R*|delta_phi|, even at a conjugate point. Its azimuth
        // need not be resolved to certify the original point distance.
        let pair = CanonicalPair {
            latitude1: problem.pair.latitude1.clone(),
            latitude2: problem.pair.latitude1.neg(),
            longitude: problem.pair.longitude.clone(),
            swapped: problem.pair.swapped,
            latitude_sign: problem.pair.latitude_sign,
            longitude_sign: problem.pair.longitude_sign,
        };
        let symmetric = InverseProblem {
            pair: &pair,
            ..*problem
        };
        match solve_source_distance_with(&symmetric, chunks, math) {
            Ok(solution) => {
                let (_, radius) = problem.reference.ellipsoid().normal_metric_bounds();
                let allowance = interval_from_rat::<I>(&latitude_sum.abs(), math)?
                    .mul(&I::pi(math)?, math)?
                    .div(&I::from_i64(180, math)?, math)?
                    .mul(&interval_from_rat::<I>(radius.exact(), math)?, math)?;
                let lower = solution.distance.sub(&allowance, math)?;
                let upper = solution.distance.add(&allowance, math)?;
                let distance = I::from_bounds(
                    lower.lower().max(I::from_i64(0, math)?.lower()).clone(),
                    upper.upper().clone(),
                    math,
                )?;
                let physical = problem
                    .goal
                    .physical_threshold()
                    .map(|threshold| interval_from_rat::<I>(threshold, math))
                    .transpose()?;
                if problem.goal.decisive(&distance, physical.as_ref(), math)? {
                    return Ok(inverse::InverseSolution {
                        distance,
                        metadata: None,
                    });
                }
            }
            Err(SolveError::Math(MathError::PrecisionExhausted | MathError::Domain(_))) => {}
            Err(error) => return Err(error),
        }
    }
    solve_source_distance_with(problem, chunks, math)
}

fn solve_source_distance_with<I: point::PointInterval>(
    problem: &InverseProblem<'_, I>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<inverse::InverseSolution<I>, SolveError> {
    let InverseProblem {
        pair,
        reference,
        order,
        iterations,
        goal,
        prepared,
        capture,
        source,
    } = *problem;
    let pi = I::pi(math)?;
    let a = interval_from_rat::<I>(reference.ellipsoid().semimajor(), math)?;
    let b = interval_from_rat::<I>(reference.ellipsoid().semiminor(), math)?;
    let c = b.div(&a, math)?;
    let e2 = interval_from_rat::<I>(&reference.ellipsoid().eccentricity_squared(), math)?;
    let ep2 = e2.div(&c.square(math)?, math)?;
    let degrees = I::from_i64(180, math)?;
    let radians_per_degree = pi.div(&degrees, math)?;
    let longitude =
        interval_from_rat::<I>(&pair.longitude, math)?.mul(&radians_per_degree, math)?;
    let physical: Option<I> = goal
        .physical_threshold()
        .map(|threshold| interval_from_rat::<I>(threshold, math))
        .transpose()?;
    let endpoint1 =
        point::reduced_point::<I>(&pair.latitude1, source, &c, &radians_per_degree, &ep2, math)?;
    let endpoint2 =
        point::reduced_point::<I>(&pair.latitude2, source, &c, &radians_per_degree, &ep2, math)?;
    let one = I::from_i64(1, math)?;
    let sphere = SpherePair {
        sin1: endpoint1.sine,
        cos1: endpoint1.cosine,
        sin2: endpoint2.sine,
        cos2: endpoint2.cosine,
        h1: endpoint1.height_factor,
        h2: endpoint2.height_factor,
        polar: pair.latitude1.abs() > Rat::from_i64(45),
        same_latitude: pair.latitude1 == pair.latitude2,
        opposite_latitude: pair.latitude1 == pair.latitude2.neg(),
        short: pair.longitude
            < Rat::new(crate::Int::one(), crate::Int::from_i64(100)).expect("positive")
            && pair.latitude2.sub(&pair.latitude1).abs()
                < Rat::new(crate::Int::one(), crate::Int::from_i64(100)).expect("positive"),
    };
    let generated;
    let coefficients = if let Some(prepared) = prepared {
        prepared
    } else {
        generated = IntegralCoefficients::new_with(&c, order, chunks, math)?;
        &generated
    };
    chunks.finish_interval(math)?;

    if pair.latitude1.abs() == Rat::from_i64(90)
        || pair.longitude.is_zero()
        || pair.longitude == Rat::from_i64(180)
    {
        let beta1 = I::atan2(&sphere.sin1, &sphere.cos1, math)?;
        let beta2 = I::atan2(&sphere.sin2, &sphere.cos2, math)?;
        let sigma1 = beta1;
        let sigma2 = if pair.longitude.is_zero() || pair.latitude1.abs() == Rat::from_i64(90) {
            beta2
        } else {
            pi.neg(math)?.sub(&beta2, math)?
        };
        let integral = coefficients
            .evaluate_with(&sigma1, &sigma2, &ep2, chunks, math)?
            .distance
            .abs(math)?;
        let distance = integral.mul(&b, math)?;
        return if goal.decisive(&distance, physical.as_ref(), math)? {
            inverse::special_solution(
                distance,
                pair,
                reference,
                &sphere,
                &c,
                &ep2,
                coefficients,
                capture,
                chunks,
                math,
            )
        } else {
            Err(MathError::PrecisionExhausted.into())
        };
    }
    let equatorial_limit = Rat::from_i64(180).mul(
        &reference
            .ellipsoid()
            .semiminor()
            .div(reference.ellipsoid().semimajor())
            .expect("positive"),
    );
    if pair.latitude1.is_zero() && pair.latitude2.is_zero() && pair.longitude <= equatorial_limit {
        let distance = a.mul(&longitude, math)?;
        return if goal.decisive(&distance, physical.as_ref(), math)? {
            inverse::equatorial_solution(distance, &longitude, pair, reference, &c, capture, math)
        } else {
            Err(MathError::PrecisionExhausted.into())
        };
    }

    let zero = I::from_i64(0, math)?;
    let two = I::from_i64(2, math)?;
    let half_pi = pi.div(&two, math)?;
    let mut lower = if sphere.same_latitude {
        half_pi.clone()
    } else {
        zero.clone()
    };
    let mut upper = pi;
    let mut root_before_vertex = false;
    let antipodal_seed = astroid::seed(
        &sphere,
        &longitude,
        &c,
        &e2,
        &ep2,
        coefficients,
        chunks,
        math,
    )?;
    if antipodal_seed.is_some() && !sphere.same_latitude {
        // The exact right-angle sample partitions the canonical global branch
        // without subtracting a near-conjugate square root or guessed epsilon.
        let critical = hybrid_with_trig(
            &sphere,
            &half_pi,
            &one,
            &zero,
            &ep2,
            &e2,
            false,
            coefficients,
            chunks,
            math,
        )?;
        if !capture {
            // At fixed latitude, moving the endpoint longitude has path length
            // at most R*|dλ|. The triangle inequality bounds shortest distance
            // around this certified ellipsoidal sample even at a conjugate point.
            let (_, radius) = reference.ellipsoid().normal_metric_bounds();
            let allowance = interval_from_rat::<I>(radius.exact(), math)?
                .mul(&longitude.sub(&critical.longitude, math)?.abs(math)?, math)?;
            let middle = critical.distance.mul(&b, math)?;
            let low = middle.sub(&allowance, math)?;
            let high = middle.add(&allowance, math)?;
            let enclosure = I::from_bounds(
                low.lower().max(zero.lower()).clone(),
                high.upper().clone(),
                math,
            )?;
            if goal.decisive(&enclosure, physical.as_ref(), math)? {
                return Ok(inverse::InverseSolution {
                    distance: enclosure,
                    metadata: None,
                });
            }
        }
        if critical.longitude.upper() < longitude.lower() {
            if sphere.opposite_latitude
                && let Some(solution) = symmetric_vertex_solution(
                    pair,
                    reference,
                    &sphere,
                    &longitude,
                    &critical.longitude,
                    &b,
                    &e2,
                    &ep2,
                    coefficients,
                    iterations,
                    goal,
                    physical.as_ref(),
                    capture,
                    chunks,
                    math,
                )?
            {
                return Ok(solution);
            }
            lower = half_pi.clone();
        } else if critical.longitude.lower() > longitude.upper() {
            upper = half_pi.clone();
            root_before_vertex = true;
        }
        chunks.finish_interval(math)?;
    }

    let antipodal = antipodal_seed.is_some();
    let local = antipodal
        || (pair.longitude < Rat::from_i64(5)
            && pair.latitude2.sub(&pair.latitude1).abs() < Rat::from_i64(5));
    // The mean-radius spherical approximation (Karney equation 48) supplies
    // a useful short-range sample without subtracting an ellipsoidal answer.
    let mean_cosine = sphere.cos1.add(&sphere.cos2, math)?.div(&two, math)?;
    let mean_scale = one
        .sub(&e2.mul(&mean_cosine.square(math)?, math)?, math)?
        .sqrt(math)?;
    let approximate_omega = longitude.div(&mean_scale, math)?;
    let (sin_longitude, cos_longitude) = approximate_omega.sin_cos(math)?;
    let spherical_y = sphere.cos2.mul(&sin_longitude, math)?;
    let spherical_x = sphere.cos1.mul(&sphere.sin2, math)?.sub(
        &sphere
            .sin1
            .mul(&sphere.cos2, math)?
            .mul(&cos_longitude, math)?,
        math,
    )?;
    // Astroid and spherical solutions are proposals only. Both local probes
    // use the complete ellipsoidal law before contracting the global bracket.
    let mut proposed = if antipodal_seed.is_some() {
        antipodal_seed
    } else {
        match I::atan2(&spherical_y, &spherical_x, math) {
            Ok(value) => Some(value),
            Err(MathError::PrecisionExhausted | MathError::Domain(_)) => None,
            Err(error) => return Err(error.into()),
        }
    };
    if let Some(solution) = seeded_regular(
        &SeededProblem {
            sphere: &sphere,
            longitude: &longitude,
            lower: &lower,
            upper: &upper,
            proposal: proposed.as_ref(),
            ep2: &ep2,
            e2: &e2,
            a: &a,
            b: &b,
            c: &c,
            coefficients,
            goal,
            physical: physical.as_ref(),
            pair,
            reference,
            capture,
            root_before_vertex,
            antipodal,
        },
        chunks,
        math,
    )? {
        return Ok(solution);
    }
    if local && let Some(seed) = proposed.as_ref() {
        let coarse_radius = I::pi(math)?.div(&I::from_i64(128, math)?, math)?;
        let vertex_radius = seed
            .sub(&half_pi, math)?
            .abs(math)?
            .mul(&I::from_i64(4, math)?, math)?
            .add(&seed.width(math)?.mul(&I::from_i64(32, math)?, math)?, math)?;
        // An astroid proposal close to the vertex suggests a much smaller
        // initial neighborhood. This radius supplies no proof: each endpoint
        // still changes the global bracket only after a complete sign test.
        let radius = if antipodal
            && vertex_radius.lower() > zero.upper()
            && vertex_radius.upper() < coarse_radius.lower()
        {
            vertex_radius
        } else {
            coarse_radius
        };
        for probe in [seed.sub(&radius, math)?, seed.add(&radius, math)?] {
            if probe.lower() <= lower.upper() || probe.upper() >= upper.lower() {
                continue;
            }
            match hybrid(
                &sphere,
                &probe,
                &ep2,
                &e2,
                root_before_vertex,
                coefficients,
                chunks,
                math,
            ) {
                Ok(value) => {
                    if value.longitude.upper() < longitude.lower() {
                        lower = probe;
                    } else if value.longitude.lower() > longitude.upper() {
                        upper = probe;
                    }
                }
                Err(SolveError::Math(MathError::PrecisionExhausted | MathError::Domain(_))) => {}
                Err(error) => return Err(error),
            }
            chunks.finish_interval(math)?;
        }
    }
    for iteration in 0..iterations {
        let midpoint = lower.add(&upper, math)?.div(&two, math)?;
        let alpha = proposed
            .take()
            .filter(|seed| seed.lower() > lower.upper() && seed.upper() < upper.lower())
            .unwrap_or(midpoint);
        let sample = hybrid(
            &sphere,
            &alpha,
            &ep2,
            &e2,
            root_before_vertex,
            coefficients,
            chunks,
            math,
        )?;
        chunks.finish_interval(math)?;
        let classified = if sample.longitude.upper() < longitude.lower() {
            lower = alpha.clone();
            true
        } else if sample.longitude.lower() > longitude.upper() {
            upper = alpha.clone();
            true
        } else {
            false
        };
        let mut newton_stalled = false;
        if iteration >= 2 || !classified {
            let bracket = I::from_bounds(lower.lower().clone(), upper.upper().clone(), math)?;
            match hybrid(
                &sphere,
                &bracket,
                &ep2,
                &e2,
                root_before_vertex,
                coefficients,
                chunks,
                math,
            ) {
                Ok(candidate) => {
                    let distance = candidate.distance.mul(&b, math)?;
                    if goal.decisive(&distance, physical.as_ref(), math)?
                        && let Some(solution) = inverse::candidate_solution(
                            distance,
                            &candidate,
                            pair,
                            reference,
                            &c,
                            &ep2,
                            coefficients,
                            capture,
                            goal,
                            chunks,
                            math,
                        )?
                    {
                        return Ok(solution);
                    }
                    if let Some(derivative) = candidate
                        .derivative
                        .filter(|value| !value.lower().is_zero() && !value.lower().is_negative())
                    {
                        // The mean-value theorem puts every root in this
                        // interval Newton image. Intersecting it with the
                        // existing global bracket cannot discard a solution.
                        let image = alpha.sub(
                            &sample
                                .longitude
                                .sub(&longitude, math)?
                                .div(&derivative, math)?,
                            math,
                        )?;
                        let next_lower = image.lower().max(bracket.lower()).clone();
                        let next_upper = image.upper().min(bracket.upper()).clone();
                        if next_lower > next_upper {
                            return Err(MathError::PrecisionExhausted.into());
                        }
                        if next_lower > *bracket.lower() || next_upper < *bracket.upper() {
                            lower = I::from_bounds(next_lower.clone(), next_lower, math)?;
                            upper = I::from_bounds(next_upper.clone(), next_upper, math)?;
                            chunks.finish_interval(math)?;
                            continue;
                        }
                        newton_stalled = true;
                    }
                }
                Err(SolveError::Math(MathError::PrecisionExhausted | MathError::Domain(_))) => {}
                Err(error) => return Err(error),
            }
        }
        if !classified {
            if newton_stalled {
                // A root enclosure remains, but the certified image cannot
                // contract it at this arithmetic/series precision. Refine
                // precision rather than exhausting iterations on this proof.
                return Err(MathError::PrecisionExhausted.into());
            }
            // A locally accurate proposal can straddle the root while the
            // opposite global endpoint remains far away. Probe a symmetric
            // neighborhood; only certified sign-separated samples become
            // new endpoints. This is a bracket search, never a tolerance
            // substitute for the final enclosure and half-even proof.
            let old_lower = lower.lower().clone();
            let old_upper = upper.upper().clone();
            let radius = upper
                .sub(&lower, math)?
                .div(&I::from_i64(1024, math)?, math)?;
            let local_radius = if let Some(derivative) = sample
                .derivative
                .as_ref()
                .filter(|value| !value.lower().is_negative() && !value.lower().is_zero())
            {
                let uncertainty = sample
                    .longitude
                    .width(math)?
                    .add(&longitude.width(math)?, math)?;
                let proposed_radius = uncertainty
                    .div(derivative, math)?
                    .add(&alpha.width(math)?, math)?
                    .mul(&I::from_i64(32, math)?, math)?;
                (proposed_radius.upper() < radius.lower()).then_some(proposed_radius)
            } else {
                None
            };
            // A local derivative chooses only the sample neighborhood. Every
            // new endpoint still needs a complete sign proof; a proposal
            // which cannot supply both signs retries the coarse search.
            for radius in [local_radius, Some(radius)].into_iter().flatten() {
                for probe in [alpha.sub(&radius, math)?, alpha.add(&radius, math)?] {
                    if probe.lower() <= lower.upper() || probe.upper() >= upper.lower() {
                        continue;
                    }
                    let value = hybrid(
                        &sphere,
                        &probe,
                        &ep2,
                        &e2,
                        root_before_vertex,
                        coefficients,
                        chunks,
                        math,
                    )?;
                    if value.longitude.upper() < longitude.lower() {
                        lower = probe;
                    } else if value.longitude.lower() > longitude.upper() {
                        upper = probe;
                    }
                    chunks.finish_interval(math)?;
                }
                if lower.lower() > &old_lower && upper.upper() < &old_upper {
                    break;
                }
            }
            if lower.lower() == &old_lower && upper.upper() == &old_upper {
                // Repeating the identical midpoint and neighborhood at this
                // precision cannot add a sign proof. Preserve the certified
                // bracket and request finer arithmetic immediately.
                return Err(MathError::PrecisionExhausted.into());
            }
            chunks.finish_interval(math)?;
            continue;
        }
        // A local Newton proposal accelerates the next sample without being
        // used as an enclosure. Invalid or out-of-bracket proposals simply
        // select the globally convergent midpoint above.
        if let Some(derivative) = sample
            .derivative
            .filter(|value| !value.lower().is_zero() && !value.lower().is_negative())
        {
            proposed = Some(
                alpha.sub(
                    &sample
                        .longitude
                        .sub(&longitude, math)?
                        .div(&derivative, math)?,
                    math,
                )?,
            );
        }
        chunks.finish_interval(math)?;
    }
    Err(SolveError::Iterations)
}

// On the exact opposite-latitude branch with alpha>=pi/2, u=cos²(alpha)
// removes the vanishing azimuth derivative at the cut-locus vertex. The
// complete pi-period auxiliary integral is independent of its starting phase.
#[allow(clippy::too_many_arguments)] // One exact symmetry reduction of the shared inverse problem.
fn symmetric_vertex_solution<I: CertifiedInterval>(
    pair: &CanonicalPair,
    reference: &GeographicReference,
    sphere: &SpherePair<I>,
    longitude: &I,
    critical_longitude: &I,
    b: &I,
    e2: &I,
    ep2: &I,
    coefficients: &IntegralCoefficients<I>,
    iterations: u32,
    goal: DistanceGoal<'_>,
    physical: Option<&I>,
    capture: bool,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<Option<inverse::InverseSolution<I>>, SolveError> {
    let zero = I::from_i64(0, math)?;
    let one = I::from_i64(1, math)?;
    let two = I::from_i64(2, math)?;
    let four = I::from_i64(4, math)?;
    let pi = I::pi(math)?;
    let scale = e2.mul(&sphere.cos1, math)?;
    let derivative_lower = scale.mul(&pi, math)?.div(&four, math)?;
    if derivative_lower.lower() <= zero.upper() {
        return Ok(None);
    }
    let mut lower = zero;
    let allowance = longitude
        .sub(critical_longitude, math)?
        .div(&derivative_lower, math)?;
    if allowance.upper() >= one.lower() {
        return Ok(None);
    }
    let mut upper = I::from_bounds(allowance.upper().clone(), allowance.upper().clone(), math)?;
    let cosine2 = sphere.cos1.square(math)?;
    let sine2 = sphere.sin1.square(math)?;
    let one_plus_c = one.add(&coefficients.c, math)?;
    // |I3_q|<=c*pi/[4(1+c)²] follows by integrating sin² over its
    // pi-period; I3 lies in [pi/2,pi/(1+c)] for the native oblate ellipsoid.
    let correction_bound = ep2
        .mul(&cosine2, math)?
        .mul(&coefficients.c, math)?
        .mul(&pi, math)?
        .div(&four.mul(&one_plus_c.square(math)?, math)?, math)?;
    for _ in 0..iterations {
        let bracket = I::from_bounds(lower.lower().clone(), upper.upper().clone(), math)?;
        let q = ep2.mul(&sine2.add(&cosine2.mul(&bracket, math)?, math)?, math)?;
        let integrals = coefficients.evaluate_period_with(&q, chunks, math)?;
        let distance = integrals.distance.mul(b, math)?;
        if goal.decisive(&distance, physical, math)? {
            if !capture {
                return Ok(Some(inverse::InverseSolution {
                    distance,
                    metadata: None,
                }));
            }
            let sin_alpha = one.sub(&bracket, math)?.sqrt(math)?;
            let cos_alpha = bracket.sqrt(math)?.neg(math)?;
            let alpha = I::atan2(&sin_alpha, &cos_alpha, math)?;
            let candidate = hybrid_with_trig(
                sphere,
                &alpha,
                &sin_alpha,
                &cos_alpha,
                ep2,
                e2,
                false,
                coefficients,
                chunks,
                math,
            )?;
            if let Some(solution) = inverse::candidate_solution(
                distance,
                &candidate,
                pair,
                reference,
                &coefficients.c,
                ep2,
                coefficients,
                true,
                goal,
                chunks,
                math,
            )? {
                return Ok(Some(solution));
            }
        }
        let midpoint = lower.add(&upper, math)?.div(&two, math)?;
        let q_mid = ep2.mul(&sine2.add(&cosine2.mul(&midpoint, math)?, math)?, math)?;
        let integral_mid = coefficients.evaluate_period_with(&q_mid, chunks, math)?;
        let sine_mid = one.sub(&midpoint, math)?.sqrt(math)?;
        let sample = pi.sub(
            &scale
                .mul(&sine_mid, math)?
                .mul(&integral_mid.longitude, math)?,
            math,
        )?;
        let sine_minimum = one.sub(&upper, math)?.sqrt(math)?;
        let derivative_upper = scale.mul(
            &pi.div(&two.mul(&one_plus_c, math)?.mul(&sine_minimum, math)?, math)?
                .add(&correction_bound, math)?,
            math,
        )?;
        let derivative = I::from_bounds(
            derivative_lower.lower().clone(),
            derivative_upper.upper().clone(),
            math,
        )?;
        let image = midpoint.sub(&sample.sub(longitude, math)?.div(&derivative, math)?, math)?;
        let next_lower = image.lower().max(bracket.lower()).clone();
        let next_upper = image.upper().min(bracket.upper()).clone();
        if next_lower > next_upper {
            return Err(MathError::PrecisionExhausted.into());
        }
        if next_lower == *bracket.lower() && next_upper == *bracket.upper() {
            return Err(MathError::PrecisionExhausted.into());
        }
        lower = I::from_bounds(next_lower.clone(), next_lower, math)?;
        upper = I::from_bounds(next_upper.clone(), next_upper, math)?;
        chunks.finish_interval(math)?;
    }
    Err(SolveError::Iterations)
}

/// The ordinary regular branch of one canonical inverse problem.
struct SeededProblem<'a, I: CertifiedInterval> {
    sphere: &'a SpherePair<I>,
    longitude: &'a I,
    lower: &'a I,
    upper: &'a I,
    proposal: Option<&'a I>,
    ep2: &'a I,
    e2: &'a I,
    a: &'a I,
    b: &'a I,
    c: &'a I,
    coefficients: &'a IntegralCoefficients<I>,
    goal: DistanceGoal<'a>,
    physical: Option<&'a I>,
    pair: &'a CanonicalPair,
    reference: &'a GeographicReference,
    capture: bool,
    root_before_vertex: bool,
    antipodal: bool,
}

/// Certify a binary64 Newton proposal by an outward interval Newton inclusion.
///
/// On the global bracket the certified longitude law is increasing, so it has
/// one root. For a proposal `t` inside a neighbourhood `A` of that bracket, the
/// interval Newton image `N = t - F(t)/F'(A)` with `F'(A) > 0` and `N` strictly
/// inside `A` proves the root exists in `A` and lies in `N`. The distance law
/// evaluated over `A`, or over the narrower `N`, then encloses the true
/// shortest distance exactly as the bracketing iteration's final enclosure
/// does. Any failure returns `None` and the unchanged bracketing iteration
/// runs; a proposal never substitutes for an enclosure.
fn seeded_regular<I: point::PointInterval>(
    problem: &SeededProblem<'_, I>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<Option<inverse::InverseSolution<I>>, SolveError> {
    // Near the antipodal cut the longitude derivative approaches zero and a
    // local inclusion rarely contracts. The astroid-bracketing iteration owns
    // those problems in exact arithmetic, where a failed attempt is costly.
    if problem.antipodal && math.binary64_ops().is_none() {
        return Ok(None);
    }
    let pi = I::pi(math)?;
    let Some(proposal) = seed_proposal(problem, &pi, math) else {
        return Ok(None);
    };
    let soft = |error: SolveError| match error {
        SolveError::Math(MathError::PrecisionExhausted | MathError::Domain(_)) => Ok(None),
        error => Err(error),
    };
    let thin = I::from_binary64_bounds(proposal.azimuth, proposal.azimuth, math)?;
    let center = match hybrid(
        problem.sphere,
        &thin,
        problem.ep2,
        problem.e2,
        problem.root_before_vertex,
        problem.coefficients,
        chunks,
        math,
    ) {
        Ok(value) => value,
        Err(error) => return soft(error),
    };
    let residual = center.longitude.sub(problem.longitude, math)?;
    // The first neighbourhood is a few final Newton steps or a few units in the
    // last place; each retry widens it. Width only selects the proof attempt.
    for widening in [1.0, 64.0, 4096.0] {
        // The Newton image of the thin residual predicts the root offset.
        let Some(magnitude) = residual.magnitude(math) else {
            return Ok(None);
        };
        let floating = math.binary64_ops().is_some();
        let neighbourhood = {
            let Ok(chunk) = math.math().enter_chunk() else {
                return Ok(None);
            };
            let ops = chunk.ops();
            let floor = ops.mul(proposal.azimuth.abs().max(1.0), proposal_floor(floating));
            let predicted = ops.div(magnitude, proposal.derivative);
            let radius = ops.mul(
                ops.mul(proposal.step, 16.0)
                    .max(ops.mul(predicted, 4.0))
                    .max(floor),
                widening,
            );
            (
                ops.sub(proposal.azimuth, radius),
                ops.add(proposal.azimuth, radius),
            )
        };
        let wide = I::from_binary64_bounds(neighbourhood.0, neighbourhood.1, math)?;
        if wide.lower() <= problem.lower.upper() || wide.upper() >= problem.upper.lower() {
            return Ok(None);
        }
        let sample = match hybrid(
            problem.sphere,
            &wide,
            problem.ep2,
            problem.e2,
            problem.root_before_vertex,
            problem.coefficients,
            chunks,
            math,
        ) {
            Ok(value) => value,
            Err(error) => return soft(error),
        };
        chunks.finish_interval(math)?;
        let Some(derivative) = sample
            .derivative
            .as_ref()
            .filter(|value| !value.lower().is_zero() && !value.lower().is_negative())
        else {
            return Ok(None);
        };
        let image = match residual.div(derivative, math) {
            Ok(quotient) => thin.sub(&quotient, math)?,
            Err(MathError::PrecisionExhausted | MathError::Domain(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        if image.lower() > wide.lower() && image.upper() < wide.upper() {
            // The root t* lies in the image. Along the branch, the first
            // variation of length gives s'(alpha) = a sin(alpha0) F'(alpha),
            // so with F' > 0 on A the integral mean value theorem places
            // s(t*) - s(t) = -a sin(alpha0)(xi) F(t) for some xi in A.
            let correction = problem
                .a
                .mul(&sample.arc.sin0, math)?
                .mul(&residual, math)?;
            let distance = center
                .distance
                .mul(problem.b, math)?
                .sub(&correction, math)?;
            let zero = I::from_i64(0, math)?;
            let distance = I::from_bounds(
                distance.lower().max(zero.lower()).clone(),
                distance.upper().clone(),
                math,
            )?;
            if problem.goal.decisive(&distance, problem.physical, math)? {
                return inverse::candidate_solution(
                    distance,
                    &sample,
                    problem.pair,
                    problem.reference,
                    problem.c,
                    problem.ep2,
                    problem.coefficients,
                    problem.capture,
                    problem.goal,
                    chunks,
                    math,
                );
            }
            return Ok(None);
        }
    }
    Ok(None)
}

/// The binary64 Newton proposal for one regular canonical problem.
fn seed_proposal<I: point::PointInterval>(
    problem: &SeededProblem<'_, I>,
    pi: &I,
    math: &mut IntervalContext<'_>,
) -> Option<seed::Proposal> {
    let sphere = problem.sphere;
    let coefficients = problem.coefficients;
    // Terms past q^8 are below 2^-58 of the series and cannot move a binary64
    // proposal; wider certified arithmetic still evaluates its full order.
    let order = coefficients
        .square_root
        .len()
        .min(coefficients.reciprocal.len())
        .checked_sub(1)?
        .min(8);
    let mut square_root = [0.0_f64; 9];
    let mut reciprocal = [0.0_f64; 9];
    for n in 0..=order {
        square_root[n] = coefficients.square_root[n].proposal(math)?;
        reciprocal[n] = coefficients.reciprocal[n].proposal(math)?;
    }
    let c = coefficients.c.proposal(math)?;
    let ep2 = problem.ep2.proposal(math)?;
    let e2 = problem.e2.proposal(math)?;
    let pi = pi.proposal(math)?;
    let approximate = seed::Sphere {
        sin1: sphere.sin1.proposal(math)?,
        cos1: sphere.cos1.proposal(math)?,
        sin2: sphere.sin2.proposal(math)?,
        cos2: sphere.cos2.proposal(math)?,
        h1: sphere.h1.proposal(math)?,
        h2: sphere.h2.proposal(math)?,
        same_latitude: sphere.same_latitude,
        opposite_latitude: sphere.opposite_latitude,
        short: sphere.short,
        polar: sphere.polar,
    };
    let lower = problem.lower.proposal(math)?;
    let upper = problem.upper.proposal(math)?;
    let start = problem.proposal.and_then(|value| value.proposal(math));
    let target = problem.longitude.proposal(math)?;
    // A validated chunk of its own: the integer-only solver has none, and the
    // proposal never needs the caller's floating scope to remain live.
    let chunk = math.math().enter_chunk().ok()?;
    let ops = chunk.ops();
    let series = seed::Series {
        c,
        ep2,
        e2,
        square_root,
        reciprocal,
        order,
        tables: seed::Tables::new(pi, ops),
    };
    let start = start.unwrap_or_else(|| ops.mul(ops.add(lower, upper), 0.5));
    seed::newton(&approximate, &series, target, lower, upper, start, ops)
}

/// The smallest first proof neighbourhood, relative to the azimuth: a few
/// units in the last place of the arithmetic that will certify it.
fn proposal_floor(floating: bool) -> f64 {
    if floating {
        1.776_356_839_400_250_5e-15
    } else {
        // Wider arithmetic certifies tighter neighbourhoods; the binary64
        // proposal itself is good to a few units in its last place.
        4.440_892_098_500_626e-16
    }
}

fn point_decisive_with<I: CertifiedInterval>(
    distance: &I,
    threshold: Option<&I>,
    math: &mut IntervalContext<'_>,
) -> Result<bool, MathError> {
    let maximum_width = I::from_ratio(&BigInt::from_i128(1), &BigInt::from_i128(2_000_000), math)?;
    if distance.width(math)?.upper() > maximum_width.lower() {
        return Ok(false);
    }
    let (rounded_lower, rounded_upper) = distance.round_decimal(6, math)?;
    Ok(rounded_lower == rounded_upper
        && threshold.is_none_or(|limit| {
            distance.upper() <= limit.lower() || distance.lower() > limit.upper()
        }))
}

fn reduced_latitude_with<I: CertifiedInterval>(
    latitude: &Rat,
    c: &I,
    radians_per_degree: &I,
    math: &mut IntervalContext<'_>,
) -> Result<(I, I), MathError> {
    if latitude.is_zero() {
        return Ok((I::from_i64(0, math)?, I::from_i64(1, math)?));
    }
    if latitude.abs() == Rat::from_i64(90) {
        return Ok((
            I::from_i64(i64::from(latitude.signum()), math)?,
            I::from_i64(0, math)?,
        ));
    }
    let (sin, cos) = if latitude.abs() > Rat::from_i64(45) {
        // Exact co-latitude reduction retains relative accuracy at pole
        // neighbors instead of subtracting two binary64 angles near pi/2.
        let complement = Rat::from_i64(90).sub(&latitude.abs());
        let angle = interval_from_rat::<I>(&complement, math)?.mul(radians_per_degree, math)?;
        let (cosine, mut sine) = angle.sin_cos(math)?;
        if latitude.signum() < 0 {
            sine = sine.neg(math)?;
        }
        (sine, cosine)
    } else {
        let angle = interval_from_rat::<I>(latitude, math)?.mul(radians_per_degree, math)?;
        angle.sin_cos(math)?
    };
    let y = sin.mul(c, math)?;
    let norm = y.square(math)?.add(&cos.square(math)?, math)?.sqrt(math)?;
    Ok((y.div(&norm, math)?, cos.div(&norm, math)?))
}

struct Hybrid<I: CertifiedInterval> {
    longitude: I,
    distance: I,
    derivative: Option<I>,
    arc: inverse::AuxiliaryArc<I>,
}

// Keep the analytic parameters and the separate observer/scratch capabilities
// explicit at this private mathematical boundary.
#[allow(clippy::too_many_arguments)]
fn hybrid<I: CertifiedInterval>(
    sphere: &SpherePair<I>,
    alpha: &I,
    ep2: &I,
    e2: &I,
    root_before_vertex: bool,
    coefficients: &IntegralCoefficients<I>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<Hybrid<I>, SolveError> {
    let (sin_alpha, cos_alpha) = alpha.sin_cos(math)?;
    hybrid_with_trig(
        sphere,
        alpha,
        &sin_alpha,
        &cos_alpha,
        ep2,
        e2,
        root_before_vertex,
        coefficients,
        chunks,
        math,
    )
}

#[allow(clippy::too_many_arguments)] // Original line formula with either exact cardinal or enclosed trigonometry.
fn hybrid_with_trig<I: CertifiedInterval>(
    sphere: &SpherePair<I>,
    alpha: &I,
    sin_alpha: &I,
    cos_alpha: &I,
    ep2: &I,
    e2: &I,
    root_before_vertex: bool,
    coefficients: &IntegralCoefficients<I>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<Hybrid<I>, SolveError> {
    let sin0 = sin_alpha.mul(&sphere.cos1, math)?;
    let x1 = cos_alpha.mul(&sphere.cos1, math)?;
    let cos0_squared = x1.square(math)?.add(&sphere.sin1.square(math)?, math)?;
    let discriminant = if sphere.same_latitude || sphere.opposite_latitude {
        x1.square(math)?
    } else {
        let delta = if sphere.polar {
            sphere
                .cos2
                .sub(&sphere.cos1, math)?
                .mul(&sphere.cos2.add(&sphere.cos1, math)?, math)?
        } else {
            sphere
                .sin1
                .sub(&sphere.sin2, math)?
                .mul(&sphere.sin1.add(&sphere.sin2, math)?, math)?
        };
        nonnegative(&x1.square(math)?.add(&delta, math)?, math)?
    };
    let x2 = if sphere.same_latitude || sphere.opposite_latitude {
        x1.abs(math)?
    } else {
        discriminant.sqrt(math)?
    };
    let sigma1 = if sphere.sin1.lower().is_zero()
        && sphere.sin1.upper().is_zero()
        && x1.upper().is_negative()
    {
        I::pi(math)?.neg(math)?
    } else {
        I::atan2(&sphere.sin1, &x1, math)?
    };
    let mut sigma2 = I::atan2(&sphere.sin2, &x2, math)?;
    // Factoring the small endpoint differences avoids subtracting two O(1)
    // expressions when the geodesic is only centimetres or metres long.
    let sine_difference = sphere.sin2.sub(&sphere.sin1, math)?;
    let cross = if sphere.short && !x1.lower().is_negative() && !x1.lower().is_zero() {
        let x_difference = sine_difference
            .neg(math)?
            .mul(&sphere.sin1.add(&sphere.sin2, math)?, math)?
            .div(&x1.add(&x2, math)?, math)?;
        sine_difference
            .mul(&x1, math)?
            .sub(&x_difference.mul(&sphere.sin1, math)?, math)?
    } else {
        sphere
            .sin2
            .mul(&x1, math)?
            .sub(&x2.mul(&sphere.sin1, math)?, math)?
    };
    let special_opposite =
        sphere.opposite_latitude && cos_alpha.upper() <= I::from_i64(0, math)?.lower();
    let omega = if special_opposite {
        let pi = I::pi(math)?;
        sigma2 = sigma1.add(&pi, math)?;
        pi
    } else {
        let y = nonnegative(&sin0.mul(&cross, math)?, math)?;
        let x = x1.mul(&x2, math)?.add(
            &sin0
                .square(math)?
                .mul(&sphere.sin1, math)?
                .mul(&sphere.sin2, math)?,
            math,
        )?;
        I::atan2(&y, &x, math)?
    };
    // With x_i = cos(alpha0) cos(sigma_i) and sin(beta_i) = cos(alpha0)
    // sin(sigma_i), x1 x2 + sin1 sin2 = cos^2(alpha0) cos(sigma12) and the
    // cross term is cos^2(alpha0) sin(sigma12). Certified positive cross and
    // dot terms place sigma12 in (0, pi/2), where one arctangent recovers it
    // without subtracting two independently enclosed endpoint angles.
    let arc_delta = if special_opposite {
        I::pi(math)?
    } else if sphere.short {
        I::atan2(
            &nonnegative(&cross, math)?,
            &x1.mul(&x2, math)?
                .add(&sphere.sin1.mul(&sphere.sin2, math)?, math)?,
            math,
        )?
    } else if !cross.lower().is_negative() && !cross.lower().is_zero() {
        let dot = x1
            .mul(&x2, math)?
            .add(&sphere.sin1.mul(&sphere.sin2, math)?, math)?;
        // Only certified sigma12 < pi/2 uses the single arctangent: there it
        // avoids cancellation, while longer arcs keep the endpoint difference.
        if dot.lower().is_negative() || dot.lower().is_zero() {
            sigma2.sub(&sigma1, math)?
        } else {
            match I::atan2(&cross, &dot, math) {
                Ok(delta) => delta,
                Err(MathError::PrecisionExhausted | MathError::Domain(_)) => {
                    sigma2.sub(&sigma1, math)?
                }
                Err(error) => return Err(error.into()),
            }
        }
    } else {
        sigma2.sub(&sigma1, math)?
    };
    let q = ep2.mul(&cos0_squared, math)?;
    let integrals =
        coefficients.evaluate_delta_with(&sigma1, &sigma2, &arc_delta, &q, chunks, math)?;
    let correction = e2.mul(&sin0, math)?.mul(&integrals.longitude, math)?;
    // Helmert/Karney reduced length and the derivative at fixed latitudes.
    // h_i depends only on beta_i because q sin²(sigma_i)=e'² sin²(beta_i).
    // x_i=cos(alpha_i)cos(beta_i), and cos²(alpha0) is the common
    // normalization of both auxiliary-sphere endpoint vectors.
    let reduced = if sphere.short {
        let h_difference = ep2
            .mul(&sine_difference, math)?
            .mul(&sphere.sin1.add(&sphere.sin2, math)?, math)?
            .div(&sphere.h1.add(&sphere.h2, math)?, math)?;
        sphere
            .h2
            .mul(&cross, math)?
            .add(&h_difference.mul(&sphere.sin1, math)?.mul(&x2, math)?, math)?
            .sub(&x1.mul(&x2, math)?.mul(&integrals.jacobi, math)?, math)?
            .div(&cos0_squared, math)?
    } else {
        sphere
            .h2
            .mul(&x1, math)?
            .mul(&sphere.sin2, math)?
            .sub(&sphere.h1.mul(&sphere.sin1, math)?.mul(&x2, math)?, math)?
            .sub(&x1.mul(&x2, math)?.mul(&integrals.jacobi, math)?, math)?
            .div(&cos0_squared, math)?
    };
    let derivative = if sphere.opposite_latitude && root_before_vertex {
        // On the independently sign-certified alpha<=pi/2 branch, x2=x1,
        // sin(beta2)=-sin(beta1), and h2=h1. Cancel x1 before dividing the
        // reduced length by x2. The resulting derivative remains finite and
        // positive at the vertex, allowing interval Newton to contract a
        // bracket whose endpoint is exactly pi/2 without a 0/0 enclosure.
        Some(
            coefficients.c.mul(
                &sphere
                    .h1
                    .mul(&sphere.sin1, math)?
                    .mul(&I::from_i64(-2, math)?, math)?
                    .sub(&x1.mul(&integrals.jacobi, math)?, math)?
                    .div(&cos0_squared, math)?,
                math,
            )?,
        )
    } else if x2.lower().is_zero() || x2.lower().is_negative() {
        None
    } else {
        Some(coefficients.c.mul(&reduced, math)?.div(&x2, math)?)
    };
    Ok(Hybrid {
        longitude: omega.sub(&correction, math)?,
        distance: integrals.distance,
        derivative,
        arc: inverse::AuxiliaryArc {
            azimuth1: alpha.clone(),
            azimuth2_y: sin0.clone(),
            azimuth2_x: x2,
            sigma1,
            sigma2,
            sin0,
            cos0_squared,
            q,
            jacobi: integrals.jacobi,
        },
    })
}

/// Restrict an enclosure using a separately proven nonnegative analytic domain.
fn nonnegative<I: CertifiedInterval>(
    value: &I,
    math: &mut IntervalContext<'_>,
) -> Result<I, MathError> {
    if value.upper().is_negative() {
        return Err(MathError::PrecisionExhausted);
    }
    I::from_bounds(
        value.lower().max(I::from_i64(0, math)?.lower()).clone(),
        value.upper().clone(),
        math,
    )
}

#[derive(Clone, Debug)]
pub(crate) struct IntegralCoefficients<I: CertifiedInterval = FixedInterval> {
    c: I,
    square_root: SmallVec<[I; 9]>,
    reciprocal: SmallVec<[I; 9]>,
    area: Option<Arc<inverse::area::PreparedArea>>,
}
struct Integrals<I: CertifiedInterval = FixedInterval> {
    distance: I,
    longitude: I,
    jacobi: I,
}

impl<I: CertifiedInterval> IntegralCoefficients<I> {
    fn new_with(
        c: &I,
        order: usize,
        chunks: &mut ChunkObserver<'_>,
        math: &mut IntervalContext<'_>,
    ) -> Result<Self, SolveError> {
        let (square_root, reciprocal) = coefficients::generate(c, order, chunks, math)?;
        Ok(Self {
            c: c.clone(),
            square_root,
            reciprocal,
            area: None,
        })
    }

    fn evaluate_with(
        &self,
        sigma1: &I,
        sigma2: &I,
        q: &I,
        chunks: &mut ChunkObserver<'_>,
        math: &mut IntervalContext<'_>,
    ) -> Result<Integrals<I>, SolveError> {
        let delta = sigma2.sub(sigma1, math)?;
        self.evaluate_delta_with(sigma1, sigma2, &delta, q, chunks, math)
    }

    // Endpoint positions and their correlated difference are separate proof
    // inputs to this one sine-power integral recurrence.
    #[allow(clippy::too_many_arguments)]
    fn evaluate_delta_with(
        &self,
        sigma1: &I,
        sigma2: &I,
        delta: &I,
        q: &I,
        chunks: &mut ChunkObserver<'_>,
        math: &mut IntervalContext<'_>,
    ) -> Result<Integrals<I>, SolveError> {
        self.evaluate_trig_delta_with(sigma1, sigma2, delta, q, None, chunks, math)
    }

    fn evaluate_period_with(
        &self,
        q: &I,
        chunks: &mut ChunkObserver<'_>,
        math: &mut IntervalContext<'_>,
    ) -> Result<Integrals<I>, SolveError> {
        let pi = I::pi(math)?;
        let half_pi = pi.div(&I::from_i64(2, math)?, math)?;
        let minus_half_pi = half_pi.neg(math)?;
        let one = I::from_i64(1, math)?;
        let minus_one = one.neg(math)?;
        let zero = I::from_i64(0, math)?;
        self.evaluate_trig_delta_with(
            &minus_half_pi,
            &half_pi,
            &pi,
            q,
            Some((&minus_one, &zero, &one, &zero)),
            chunks,
            math,
        )
    }

    #[allow(clippy::too_many_arguments)] // One recurrence with either certified or exact endpoint trigonometry.
    fn evaluate_trig_delta_with(
        &self,
        sigma1: &I,
        sigma2: &I,
        delta: &I,
        q: &I,
        known_trig: Option<(&I, &I, &I, &I)>,
        chunks: &mut ChunkObserver<'_>,
        math: &mut IntervalContext<'_>,
    ) -> Result<Integrals<I>, SolveError> {
        if q.lower().is_zero() && q.upper().is_zero() {
            // Exact auxiliary-sphere integrands at q=0, including the native
            // equator: ds/b=dσ, dI3=dσ/(1+c), and the Jacobi correction is0.
            return Ok(Integrals {
                distance: delta.clone(),
                longitude: delta.mul(&self.reciprocal[0], math)?,
                jacobi: I::from_i64(0, math)?,
            });
        }
        let one = I::from_i64(1, math)?;
        let two = I::from_i64(2, math)?;
        let twice_q = q.mul(&two, math)?;
        if twice_q.upper() >= one.lower() {
            let (distance, longitude, jacobi) = integral_fallback::evaluate_with(
                sigma1,
                sigma2,
                q,
                &self.c,
                self.square_root.len() - 1,
                chunks,
                math,
            )?;
            return Ok(Integrals {
                distance,
                longitude,
                jacobi,
            });
        }
        let (sin1, cos1, sin2, cos2) = if let Some((sin1, cos1, sin2, cos2)) = known_trig {
            (sin1.clone(), cos1.clone(), sin2.clone(), cos2.clone())
        } else {
            let (sin1, cos1) = sigma1.sin_cos(math)?;
            let (sin2, cos2) = sigma2.sin_cos_range(math)?;
            (sin1, cos1, sin2, cos2)
        };
        let sin1_sq = sin1.square(math)?;
        let sin2_sq = sin2.square(math)?;
        let endpoint1 = sin1.mul(&cos1, math)?;
        let mut endpoint2 = sin2.mul(&cos2, math)?;
        let short = delta.abs(math)?.upper() < I::power_of_two(-6, math)?.lower();
        let (mut endpoint_difference, square_difference) = if short {
            // cos(s1)sin(s1)-cos(s2)sin(s2)=-sin(delta)cos(s1+s2)
            // sin²(s1)-sin²(s2)=-sin(delta)sin(s1+s2).
            let (sin_delta, _) = delta.sin_cos(math)?;
            let negative_sine = sin_delta.neg(math)?;
            let cosine_sum = cos1.mul(&cos2, math)?.sub(&sin1.mul(&sin2, math)?, math)?;
            let sine_sum = sin1.mul(&cos2, math)?.add(&cos1.mul(&sin2, math)?, math)?;
            (
                negative_sine.mul(&cosine_sum, math)?,
                negative_sine.mul(&sine_sum, math)?,
            )
        } else {
            (
                endpoint1.sub(&endpoint2, math)?,
                sin1_sq.sub(&sin2_sq, math)?,
            )
        };
        let mut integral = delta.clone();
        let mut q_power = one.clone();
        let mut distance = delta.clone();
        let mut longitude = delta.mul(&self.reciprocal[0], math)?;
        let mut jacobi = I::from_i64(0, math)?;
        for n in 1..self.square_root.len() {
            q_power = q_power.mul(q, math)?;
            let odd = I::from_i64(2 * n as i64 - 1, math)?;
            let even = I::from_i64(2 * n as i64, math)?;
            integral = integral
                .mul(&odd, math)?
                .add(&endpoint_difference, math)?
                .div(&even, math)?;
            let term = integral.mul(&q_power, math)?;
            distance = distance.add(&term.mul(&self.square_root[n], math)?, math)?;
            // sqrt(1+z)-1/sqrt(1+z) has coefficient 2n*binom(1/2,n).
            jacobi = jacobi.add(
                &term.mul(&self.square_root[n], math)?.mul(&even, math)?,
                math,
            )?;
            longitude = longitude.add(&term.mul(&self.reciprocal[n], math)?, math)?;
            // D_(n+1)=D_n*sin²(s1)+endpoint2_n*(sin²(s1)-sin²(s2)).
            endpoint_difference = endpoint_difference
                .mul(&sin1_sq, math)?
                .add(&endpoint2.mul(&square_difference, math)?, math)?;
            endpoint2 = endpoint2.mul(&sin2_sq, math)?;
            if n.is_multiple_of(64) {
                chunks.finish_interval(math)?;
            }
        }
        // Binomial sqrt coefficients have magnitude <=1. The Jacobi
        // coefficients 2n*binomial(1/2,n) also have magnitude <=1:
        // their successive absolute ratio is (2n-1)/(2n). For c>=0 the
        // reciprocal is holomorphic on |z|<1 with modulus <=1 because
        // Re sqrt(1+z)>0; taking Cauchy radii up to one gives the same
        // coefficient bound. Therefore every omitted integral tail is
        // bounded by q^(N+1)/(1-q), without a fitted error estimate.
        let mut tail_power = one.clone();
        for n in 0..self.square_root.len() {
            tail_power = tail_power.mul(q, math)?;
            if n.is_multiple_of(64) {
                chunks.finish_interval(math)?;
            }
        }
        let tail = tail_power
            .div(&one.sub(q, math)?, math)?
            .mul(&delta.abs(math)?, math)?;
        let padding = I::from_bounds(tail.upper().negated(), tail.upper().clone(), math)?;
        Ok(Integrals {
            distance: distance.add(&padding, math)?,
            longitude: longitude.add(&padding, math)?,
            jacobi: jacobi.add(&padding, math)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{PreparedGeodesic, distance};
    use crate::{
        ExecutionLimits, ExecutionPolicy, GeoError, GeographicReference, LonLat, MetricContext, Rat,
    };

    fn point(longitude: &str, latitude: &str) -> LonLat {
        LonLat::new(
            Rat::parse_decimal(longitude).unwrap(),
            Rat::parse_decimal(latitude).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn directly_cold_reference_entries_refuse_before_identity_allocation() {
        struct Receipt {
            work: u64,
            peak: u64,
        }
        impl crate::MetricWorkObserver for Receipt {
            fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
                self.work += work;
                self.peak += growth;
                Ok(())
            }
        }
        let adequate = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 1_000_000_000,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut preparation = crate::PreparationBudget::new(adequate);
        let ellipsoid = crate::PreparedEllipsoid::new_in_budget(
            Rat::from_int(crate::Int::one().shl(4096).add(&crate::Int::one())),
            Rat::from_i64(300),
            &mut preparation,
        )
        .unwrap();
        let origin = point("0", "0");
        let azimuth = Rat::from_i64(90);
        let zero = crate::Metres::new(Rat::zero());
        for entry in 0..6 {
            for memory in [false, true] {
                let reference = GeographicReference::new(
                    ellipsoid.clone(),
                    purrdf_hash::hex::Digest32::new([81; 32]),
                    crate::AxisOrder::LonLat,
                );
                let prepared = PreparedGeodesic::new(reference.clone());
                let policy = ExecutionPolicy::new(ExecutionLimits {
                    max_work_items: if memory { 1_000_000_000 } else { 1 },
                    max_workspace_bytes: if memory { 1 } else { 64 * 1024 * 1024 },
                    ..ExecutionLimits::GEOMETRY
                })
                .unwrap();
                let mut context = MetricContext::new(reference.clone(), policy).unwrap();
                let mut receipt = Receipt { work: 0, peak: 0 };
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let result = match entry {
                    0 => prepared
                        .distance_metered(&origin, &origin, &mut context, &mut receipt)
                        .map(|_| ()),
                    1 => prepared
                        .inverse_metered(&origin, &origin, &mut context, &mut receipt)
                        .map(|_| ()),
                    2 => prepared
                        .direct_metered(&origin, &azimuth, &zero, &mut context, &mut receipt)
                        .map(|_| ()),
                    3 => prepared
                        .prepare_point_metered(&origin, &mut context, &mut receipt)
                        .map(|_| ()),
                    4 => prepared
                        .within_physical_metered(
                            &origin,
                            &origin,
                            &zero,
                            &mut context,
                            &mut receipt,
                        )
                        .map(|_| ()),
                    _ => PreparedGeodesic::prepare_metered(reference, &mut context, &mut receipt)
                        .map(|_| ()),
                };
                let measured = window.close();
                if memory {
                    assert!(matches!(
                        result,
                        Err(GeoError::MemoryExhausted { limit: 1 })
                    ));
                } else {
                    assert!(matches!(result, Err(GeoError::WorkExhausted { limit: 1 })));
                }
                assert_eq!(measured.allocations, 0, "entry {entry}, memory {memory}");
                assert_eq!(measured.requested_bytes, 0);
                assert_eq!(context.current_workspace_bytes(), 0);
                assert_eq!(context.work_items(), receipt.work);
                assert_eq!(context.workspace_peak(), receipt.peak);
                // A separate actual cold render is the positive allocator
                // control; it cannot have happened during the refused entry.
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let _binding = prepared.reference().id();
                let rendered = window.close();
                assert!(rendered.allocations > 0);
                assert!(rendered.requested_bytes > 0);
            }
        }
    }

    #[test]
    fn borrowed_batches_refuse_a_false_iterator_length_without_partial_outputs() {
        struct WrongLength {
            values: std::array::IntoIter<u8, 2>,
            advertised: usize,
        }
        impl Iterator for WrongLength {
            type Item = u8;
            fn next(&mut self) -> Option<Self::Item> {
                self.values.next()
            }
            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.advertised, Some(self.advertised))
            }
        }
        impl ExactSizeIterator for WrongLength {}
        let mut context = MetricContext::wgs84().unwrap();
        for advertised in [1, 3] {
            let mut output = vec![Some(99); advertised];
            let result = super::batch(
                WrongLength {
                    values: [1, 2].into_iter(),
                    advertised,
                },
                &mut output,
                &mut context,
                |value, _| Ok(value),
            );
            assert!(matches!(result, Err(GeoError::InvalidOutputLength { .. })));
            assert_eq!(output, vec![None; advertised]);
        }
    }

    #[test]
    fn forced_arithmetic_paths_preserve_completed_distance_inverse_direct_and_batches() {
        use purrdf_hash::Backend as _;
        use purrdf_xsd::math::FloatProductBackend;
        let pairs = [
            (
                point("0", "36.530042355041"),
                point("5.762344694676510456", "-48.164270779097768864"),
            ),
            (
                point("0", "56.048002478584"),
                point("0.000002169207675262", "56.048000583672714595"),
            ),
            (
                point("0", "70.262058960871"),
                point("179.795729562201047750", "-70.262058960870992417"),
            ),
            (point("73", "90"), point("-17", "-90")),
            (point("0", "0"), point("180", "0")),
        ];
        for reference in [
            GeographicReference::wgs84(),
            GeographicReference::cgcs2000(),
        ] {
            let mut scalar =
                MetricContext::new(reference.clone(), ExecutionPolicy::geometry()).unwrap();
            scalar
                .set_binary64_backend(FloatProductBackend::Portable)
                .unwrap();
            scalar.prepare_arithmetic().unwrap();
            let prepared = PreparedGeodesic::prepare(reference.clone(), &mut scalar).unwrap();
            let expected = pairs
                .iter()
                .map(|(a, b)| {
                    let distance = prepared.distance(a, b, &mut scalar).unwrap();
                    let inverse = prepared.inverse(a, b, &mut scalar).unwrap();
                    let direct = prepared
                        .direct(a, &Rat::from_i64(37), distance.value(), &mut scalar)
                        .unwrap();
                    (distance, inverse, direct)
                })
                .collect::<Vec<_>>();
            for path in FloatProductBackend::all_available() {
                let mut worker =
                    MetricContext::new(reference.clone(), ExecutionPolicy::geometry()).unwrap();
                worker.set_binary64_backend(path).unwrap();
                worker.set_prepared_arithmetic(scalar.prepared_arithmetic().unwrap());
                for ((a, b), (distance, inverse, direct)) in pairs.iter().zip(&expected) {
                    let source = prepared.prepare_point(a, &mut worker).unwrap();
                    let got = prepared
                        .distance_from_prepared(&source, b, &mut worker)
                        .unwrap();
                    assert_eq!(
                        got.certificate_bytes(),
                        distance.certificate_bytes(),
                        "{}",
                        path.name()
                    );
                    assert_eq!(
                        prepared
                            .inverse(a, b, &mut worker)
                            .unwrap()
                            .certificate_bytes(),
                        inverse.certificate_bytes()
                    );
                    assert_eq!(
                        prepared
                            .direct(a, &Rat::from_i64(37), distance.value(), &mut worker)
                            .unwrap()
                            .certificate_bytes(),
                        direct.certificate_bytes()
                    );
                    assert!(got.within_reported(
                        crate::XsdDoubleMetres::new(distance.reported_double()).unwrap()
                    ));
                }
                let mut output = vec![None; pairs.len()];
                prepared
                    .distance_batch(&pairs, &mut output, &mut worker)
                    .unwrap();
                for (got, (distance, _, _)) in output.iter().zip(&expected) {
                    assert_eq!(
                        got.as_ref().unwrap().certificate_bytes(),
                        distance.certificate_bytes()
                    );
                }
            }
        }
    }
    #[test]
    fn physical_zero_uses_original_metric_identity_for_subnormal_neighbors() {
        let reference = GeographicReference::wgs84();
        let prepared = PreparedGeodesic::new(reference);
        let mut context = MetricContext::wgs84().unwrap();
        let center = point("0", "0");
        let tiny = Rat::from_binary64(f64::from_bits(1)).unwrap();
        let longitude = LonLat::new(tiny.clone(), Rat::zero()).unwrap();
        let latitude = LonLat::new(Rat::zero(), tiny).unwrap();
        let zero = crate::Metres::new(Rat::zero());
        for neighbor in [&longitude, &latitude] {
            assert!(
                !prepared
                    .within_physical(&center, neighbor, &zero, &mut context)
                    .unwrap()
            );
            assert!(
                prepared
                    .distance(&center, neighbor, &mut context)
                    .unwrap()
                    .within_reported(crate::XsdDoubleMetres::new(0.0).unwrap())
            );
        }
        assert!(
            prepared
                .within_physical(&point("180", "3"), &point("-180", "3"), &zero, &mut context)
                .unwrap()
        );
        assert!(
            prepared
                .within_physical(
                    &point("45", "90"),
                    &point("-120", "90"),
                    &zero,
                    &mut context
                )
                .unwrap()
        );
        let source = prepared.prepare_point(&center, &mut context).unwrap();
        assert!(
            !prepared
                .within_physical_from_prepared(&source, &longitude, &zero, &mut context)
                .unwrap()
        );
        struct Cancel;
        impl crate::MetricWorkObserver for Cancel {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        assert_eq!(
            prepared.within_physical_metered(&center, &longitude, &zero, &mut context, &mut Cancel),
            Err(GeoError::Cancelled)
        );
        assert_eq!(
            prepared.within_physical_from_prepared_metered(
                &source,
                &longitude,
                &zero,
                &mut context,
                &mut Cancel
            ),
            Err(GeoError::Cancelled)
        );
    }

    #[test]
    fn exact_equator_and_meridian_reference_quantization() {
        for (a, b, expected) in [
            (point("0", "0"), point("1", "0"), "111319490793"),
            (point("0", "0"), point("0.000000001", "0"), "111"),
            (point("0", "0"), point("0", "90"), "10001965729313"),
            (point("0", "0"), point("180", "0"), "20003931458625"),
        ] {
            let answer = distance(a, b).expect("certified analytic/reference path");
            assert_eq!(answer.quantized_micrometres().to_string(), expected);
        }
    }

    // CC0 GeodTest rows 151552 and 160841 exercise short oblique geodesics.
    #[test]
    fn public_submetre_and_metre_rows_complete_under_default_limits() {
        let tolerance = Rat::parse_decimal("0.0000006").unwrap();
        for (latitude1, longitude2, latitude2, expected) in [
            (
                "56.048002478584",
                "0.000002169207675262",
                "56.048000583672714595",
                "0.2505729",
            ),
            (
                "8.731720592203",
                "0.000009585960285958",
                "8.731724087115001044",
                "1.1234106",
            ),
        ] {
            let a = point("0", latitude1);
            let b = point(longitude2, latitude2);
            let result = distance(a.clone(), b.clone()).expect("short certified default inverse");
            assert!(
                result
                    .value()
                    .exact()
                    .sub(&Rat::parse_decimal(expected).unwrap())
                    .abs()
                    <= tolerance
            );
            assert_eq!(
                result,
                distance(b, a).expect("symmetric certified default inverse")
            );
        }
    }

    #[test]
    fn pole_and_seam_aliases_have_exact_zero_answers() {
        for (a, b) in [
            (point("180", "0"), point("-180", "0")),
            (point("-147", "90"), point("93", "90")),
            (point("42", "-90"), point("-83", "-90")),
        ] {
            assert!(distance(a, b).unwrap().value().exact().is_zero());
        }
    }

    // CC0 data: C. F. F. Karney, GeodTest, Zenodo record 32156.
    // The finite corpus distances carry decimal rounding uncertainty; this
    // agreement test does not treat the printed values as infinitely precise.
    #[test]
    fn public_inverse_reference_rows_agree_with_certified_quantization() {
        let tolerance = Rat::parse_decimal("0.0000006").unwrap();
        for (lat1, lon2, lat2, metres) in [
            (
                "36.530042355041",
                "5.762344694676510456",
                "-48.164270779097768864",
                "9398502.0434687",
            ),
            (
                "62.912967770911",
                "68.567247430960081525",
                "-23.186512533703375483",
                "11254667.2007643",
            ),
            (
                "2.881248229541",
                "44.520619105667619923",
                "53.997072295385487038",
                "6958264.1576889",
            ),
            (
                "15.543058059808",
                "166.82808321068892367",
                "-49.416672064705806337",
                "16063336.69802",
            ),
        ] {
            let answer = distance(point("0", lat1), point(lon2, lat2)).expect("certified inverse");
            let expected = Rat::parse_decimal(metres).unwrap();
            assert!(
                answer.value().exact().sub(&expected).abs() <= tolerance,
                "reference {metres}, got {}",
                answer.value().exact().to_decimal_string(6)
            );
        }
    }

    #[test]
    fn insufficient_precision_refuses_even_an_ordinary_point_pair() {
        let reference = GeographicReference::wgs84();
        let prepared = PreparedGeodesic::new(reference.clone());
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_precision_bits: 1,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut context = MetricContext::new(reference, policy).unwrap();
        assert!(matches!(
            prepared.distance(&point("0", "0"), &point("1", "0"), &mut context),
            Err(GeoError::PrecisionExhausted { bits: 1 })
        ));
    }
}
