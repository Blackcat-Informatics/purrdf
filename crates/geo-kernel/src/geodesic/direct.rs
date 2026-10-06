// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Direct auxiliary-sphere propagation with a fixed decimal-degree output grid.

use purrdf_hash::Domain;

const DIRECT_CERTIFICATE: Domain = Domain::new(b"purrdf/direct-geodesic-certificate/v1");

use purrdf_xsd::{
    BigInt,
    math::{CoordinateMath, FixedInterval, MathError, MathLimits},
};

use super::{AdmittedGeodesicReference, ChunkObserver, PreparedGeodesic, SolveError};
use crate::numerical::{carrier_integer, fixed_from_bounds, fixed_from_rat, geo_math_error};
use crate::{
    GeoBindingId, GeoError, Int, LonLat, Metres, MetricContext, MetricWorkObserver, PointLaw, Rat,
    SemanticLawId,
};

/// An explicitly selected half-even decimal-degree output grid.
///
/// Invocation precision refines the proof of this grid; it never changes its
/// quantum. Grids which cannot meet the complete one-micrometre physical bound
/// or the admitted numerical precision refuse before decimal allocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DirectOutputGrid {
    decimal_places: u32,
}

#[derive(Clone, Copy)]
enum DirectInvocation {
    Public(DirectOutputGrid),
    Admitted(GeoBindingId),
}

impl AdmittedGeodesicReference<'_> {
    pub(crate) fn direct_metered(
        &self,
        start: &LonLat,
        azimuth: &Rat,
        length: &Metres,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<DirectResult, GeoError> {
        context.begin(1)?;
        let mut chunks = ChunkObserver::new(Some(observer));
        chunks.initial().map_err(super::solve_external_error)?;
        self.prepared.compute_direct(
            start,
            azimuth,
            length,
            DirectInvocation::Admitted(self.binding),
            context,
            &mut chunks,
        )
    }
}

impl DirectOutputGrid {
    /// The standard frozen fifteen-place angular grid.
    pub const DEGREE15: Self = Self::new(15);

    /// Declare an angular grid independently of numerical execution limits.
    #[must_use]
    pub const fn new(decimal_places: u32) -> Self {
        Self { decimal_places }
    }

    /// Exact caller-declared decimal degree places.
    #[must_use]
    pub const fn decimal_places(self) -> u32 {
        self.decimal_places
    }

    /// Mathematical law bound to this exact output quantum.
    #[must_use]
    pub fn law_id(self) -> SemanticLawId {
        PointLaw::DirectDecimalDegreeV1 {
            decimal_places: self.decimal_places,
        }
        .id()
    }
}

/// Correctly quantized direct endpoint and forward final azimuth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectResult {
    endpoint: LonLat,
    final_azimuth: Rat,
    binding: GeoBindingId,
    grid: DirectOutputGrid,
}

impl DirectResult {
    /// Endpoint at the explicitly selected decimal degree grid.
    #[must_use]
    pub const fn endpoint(&self) -> &LonLat {
        &self.endpoint
    }

    /// Forward azimuth in `[0,360)`, at the selected decimal degree grid.
    #[must_use]
    pub const fn final_azimuth(&self) -> &Rat {
        &self.final_azimuth
    }

    /// The exact completed output grid, independent of proof precision.
    #[must_use]
    pub const fn output_grid(&self) -> DirectOutputGrid {
        self.grid
    }

    /// Fixed total numerical and quantization surface bound, in metres.
    #[must_use]
    pub fn surface_error_bound(&self) -> Metres {
        Metres::new(Rat::parse_decimal("0.000001").expect("frozen decimal"))
    }

    /// The fixed completed endpoint law.
    #[must_use]
    pub fn law_id(&self) -> SemanticLawId {
        self.grid.law_id()
    }

    /// Canonical version-1 certificate with fixed coordinate precision and bound.
    ///
    /// Fields use the workspace's canonical length framing: format, law, binding,
    /// degree quantum, longitude, latitude, final azimuth, metre surface bound.
    #[must_use]
    pub fn certificate_bytes(&self) -> Vec<u8> {
        let law = self.law_id().digest();
        let binding = self.binding.digest();
        let places = self.grid.decimal_places();
        let longitude = self.endpoint.longitude().to_decimal_string(places);
        let latitude = self.endpoint.latitude().to_decimal_string(places);
        let azimuth = self.final_azimuth.to_decimal_string(places);
        let quantum = Rat::from_decimal(Int::one(), places).to_decimal_string(places);
        crate::metric::framed_certificate([
            DIRECT_CERTIFICATE.as_bytes(),
            law.as_bytes(),
            binding.as_bytes(),
            quantum.as_bytes(),
            longitude.as_bytes(),
            latitude.as_bytes(),
            azimuth.as_bytes(),
            b"0.000001",
        ])
    }

    /// The exact reference used by this direct propagation.
    #[must_use]
    pub const fn binding_id(&self) -> GeoBindingId {
        self.binding
    }
}

impl PreparedGeodesic {
    /// Propagate an azimuth and nonnegative exact distance on the prepared ellipsoid.
    ///
    /// Azimuth is measured clockwise from north and reduced modulo a full turn.
    /// An exact starting pole uses longitude zero as its azimuth frame. The
    /// original starting coordinates remain intact.
    ///
    /// # Errors
    ///
    /// Refuses negative length, reference mismatch, unresolved output rounding,
    /// operational exhaustion or a grid whose physical bound exceeds one micrometre.
    pub fn direct(
        &self,
        start: &LonLat,
        azimuth: &Rat,
        length: &Metres,
        context: &mut MetricContext,
    ) -> Result<DirectResult, GeoError> {
        self.direct_with_grid(start, azimuth, length, DirectOutputGrid::DEGREE15, context)
    }

    /// Propagate using an explicitly selected half-even decimal-degree grid.
    ///
    /// # Errors
    /// Refuses a grid beyond admission or whose physical quantum exceeds the
    /// one-micrometre certificate, plus the errors of [`Self::direct`].
    pub fn direct_with_grid(
        &self,
        start: &LonLat,
        azimuth: &Rat,
        length: &Metres,
        grid: DirectOutputGrid,
        context: &mut MetricContext,
    ) -> Result<DirectResult, GeoError> {
        context.begin(1)?;
        self.compute_direct(
            start,
            azimuth,
            length,
            DirectInvocation::Public(grid),
            context,
            &mut ChunkObserver::new(None),
        )
    }

    /// The same correctly quantized direct result with governed work/cancellation.
    ///
    /// # Errors
    /// Adds the external observer's refusals to [`Self::direct`].
    pub fn direct_metered(
        &self,
        start: &LonLat,
        azimuth: &Rat,
        length: &Metres,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<DirectResult, GeoError> {
        self.direct_with_grid_metered(
            start,
            azimuth,
            length,
            DirectOutputGrid::DEGREE15,
            context,
            observer,
        )
    }

    /// The explicit-grid direct law with governed work and cancellation.
    ///
    /// # Errors
    /// Adds external refusal to [`Self::direct_with_grid`].
    pub fn direct_with_grid_metered(
        &self,
        start: &LonLat,
        azimuth: &Rat,
        length: &Metres,
        grid: DirectOutputGrid,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<DirectResult, GeoError> {
        context.begin(1)?;
        let mut chunks = ChunkObserver::new(Some(observer));
        chunks.initial().map_err(super::solve_external_error)?;
        self.compute_direct(
            start,
            azimuth,
            length,
            DirectInvocation::Public(grid),
            context,
            &mut chunks,
        )
    }

    /// Compute direct propagations into an equally sized caller-owned buffer.
    /// A refusal clears every slot; only a complete successful batch is usable.
    ///
    /// # Errors
    /// Refuses shape, reference, numerical environment and resource/rounding errors.
    pub fn direct_batch(
        &self,
        inputs: &[(LonLat, Rat, Metres)],
        output: &mut [Option<DirectResult>],
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.direct_batch_with_grid(inputs, output, DirectOutputGrid::DEGREE15, context)
    }

    /// The same direct batch with bounded governor/cancellation callbacks.
    ///
    /// # Errors
    /// Adds the observer's refusals to [`Self::direct_batch`].
    pub fn direct_batch_metered(
        &self,
        inputs: &[(LonLat, Rat, Metres)],
        output: &mut [Option<DirectResult>],
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.direct_batch_with_grid_metered(
            inputs,
            output,
            DirectOutputGrid::DEGREE15,
            context,
            observer,
        )
    }

    /// Complete direct batch on one caller-declared angular grid.
    ///
    /// # Errors
    /// Refuses the same grid/shape/resources as scalar explicit propagation;
    /// every output slot is cleared on refusal.
    pub fn direct_batch_with_grid(
        &self,
        inputs: &[(LonLat, Rat, Metres)],
        output: &mut [Option<DirectResult>],
        grid: DirectOutputGrid,
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.direct_batch_borrowed(
            inputs
                .iter()
                .map(|(start, azimuth, length)| (start, azimuth, length)),
            output,
            grid,
            context,
        )
    }

    /// Complete explicit-grid batch with bounded external charging.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::direct_batch_with_grid`].
    pub fn direct_batch_with_grid_metered(
        &self,
        inputs: &[(LonLat, Rat, Metres)],
        output: &mut [Option<DirectResult>],
        grid: DirectOutputGrid,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.direct_batch_borrowed_metered(
            inputs
                .iter()
                .map(|(start, azimuth, length)| (start, azimuth, length)),
            output,
            grid,
            context,
            observer,
        )
    }

    /// Complete explicit-grid borrowed batch without original-coordinate copies.
    ///
    /// # Errors
    /// Refuses shape, grid, environment and operational failures with no partial output.
    pub fn direct_batch_borrowed<'a>(
        &self,
        inputs: impl ExactSizeIterator<Item = (&'a LonLat, &'a Rat, &'a Metres)>,
        output: &mut [Option<DirectResult>],
        grid: DirectOutputGrid,
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.direct_batch_observed(inputs, output, grid, context, None)
    }

    /// Borrowed explicit-grid batch with bounded external work/cancellation polls.
    ///
    /// # Errors
    /// Adds the observer's refusal to [`Self::direct_batch_borrowed`].
    pub fn direct_batch_borrowed_metered<'a>(
        &self,
        inputs: impl ExactSizeIterator<Item = (&'a LonLat, &'a Rat, &'a Metres)>,
        output: &mut [Option<DirectResult>],
        grid: DirectOutputGrid,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.direct_batch_observed(inputs, output, grid, context, Some(observer))
    }

    fn direct_batch_observed<'a>(
        &self,
        inputs: impl ExactSizeIterator<Item = (&'a LonLat, &'a Rat, &'a Metres)>,
        output: &mut [Option<DirectResult>],
        grid: DirectOutputGrid,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<(), GeoError> {
        let mut chunks = ChunkObserver::new(observer);
        super::batch(
            inputs,
            output,
            context,
            |(start, azimuth, length), context| {
                self.compute_direct(
                    start,
                    azimuth,
                    length,
                    DirectInvocation::Public(grid),
                    context,
                    &mut chunks,
                )
            },
        )
    }

    fn compute_direct(
        &self,
        start: &LonLat,
        azimuth: &Rat,
        length: &Metres,
        invocation: DirectInvocation,
        context: &mut MetricContext,
        chunks: &mut ChunkObserver<'_>,
    ) -> Result<DirectResult, GeoError> {
        context.charge_work(1)?;
        chunks
            .context_entry(context)
            .map_err(super::solve_external_error)?;
        if matches!(invocation, DirectInvocation::Public(_))
            && !chunks.reference_matches(&self.reference, context)?
        {
            return Err(GeoError::config(
                "prepared geodesic and metric context references differ",
            ));
        }
        let grid = match invocation {
            DirectInvocation::Public(grid) => grid,
            DirectInvocation::Admitted(_) => DirectOutputGrid::DEGREE15,
        };
        if length.exact() < &Rat::zero() {
            return Err(GeoError::domain("direct distance must be nonnegative"));
        }
        if matches!(invocation, DirectInvocation::Public(_)) {
            validate_grid(grid, self, context, chunks)?;
        }
        // Invalid output declarations return before any certificate identity
        // is needed. Every completed direct result still admits its cold or
        // cached binding through the original shared identity worker.
        let binding = match invocation {
            DirectInvocation::Public(_) => {
                chunks.reference_identity(Some(&self.reference), context)?
            }
            DirectInvocation::Admitted(binding) => {
                context.charge_work(1)?;
                chunks
                    .context_entry(context)
                    .map_err(super::solve_external_error)?;
                let target = match context.reference().cached_identity() {
                    Some(target) => target,
                    None => chunks.reference_identity(None, context)?,
                };
                if target != binding {
                    return Err(GeoError::config(
                        "admitted geodesic and metric context references differ",
                    ));
                }
                binding
            }
        };
        let azimuth = normalize(azimuth, 0);
        if length.exact().is_zero() {
            let longitude = if start.is_pole() {
                Rat::zero()
            } else {
                normalize(
                    &Rat::from_decimal(
                        start.longitude().round_to_scale(grid.decimal_places()),
                        grid.decimal_places(),
                    ),
                    -180,
                )
            };
            let latitude = Rat::from_decimal(
                start.latitude().round_to_scale(grid.decimal_places()),
                grid.decimal_places(),
            );
            return Ok(DirectResult {
                endpoint: canonical_endpoint(longitude, latitude)?,
                final_azimuth: normalize(
                    &Rat::from_decimal(
                        azimuth.round_to_scale(grid.decimal_places()),
                        grid.decimal_places(),
                    ),
                    0,
                ),
                binding,
                grid,
            });
        }
        let policy = context.policy();
        let maximum = policy.limits().max_precision_bits;
        chunks.prepare_scratch(
            context,
            super::scratch_source_bits(
                &[
                    start.longitude(),
                    start.latitude(),
                    &azimuth,
                    length.exact(),
                ],
                self.reference.ellipsoid(),
            ),
        )?;
        let mut bits = maximum.min(80);
        if bits < 16 {
            return Err(GeoError::PrecisionExhausted { bits: maximum });
        }
        loop {
            chunks.reset();
            let order = super::integral_order(self.reference.ellipsoid(), bits);
            let coefficients = self.worker_coefficients(bits, order, context, chunks)?;
            chunks
                .context_entry(context)
                .map_err(super::solve_external_error)?;
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
            let reservation =
                (u64::from(bits.div_ceil(8)) * 8 + 128).saturating_mul(2 * (order as u64 + 1) + 64);
            let reservation =
                usize::try_from(reservation).map_err(|_| GeoError::MemoryExhausted {
                    limit: policy.limits().max_workspace_bytes,
                })?;
            math.reserve_workspace(reservation)
                .map_err(|error| geo_math_error(&error, policy))?;
            let result = propagate(
                start,
                &azimuth,
                length,
                grid,
                self,
                coefficients.as_ref(),
                order,
                policy.limits().max_iterations,
                chunks,
                &mut math,
            );
            let observer_result = if matches!(&result, Err(SolveError::External(_))) {
                Ok(())
            } else {
                chunks.finish(&math)
            };
            math.release_workspace(reservation)
                .map_err(|error| geo_math_error(&error, policy))?;
            context.charge_work(math.work_used())?;
            context.admit_workspace(math.workspace_peak() as u64)?;
            context.release_workspace(math.workspace_peak() as u64)?;
            observer_result.map_err(super::solve_external_error)?;
            match result {
                Ok((longitude, latitude, final_azimuth)) => {
                    return Ok(DirectResult {
                        endpoint: canonical_endpoint(longitude, latitude)?,
                        final_azimuth,
                        binding,
                        grid,
                    });
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
            if bits == maximum {
                return Err(GeoError::PrecisionExhausted { bits: maximum });
            }
            bits = super::next_precision(bits, maximum);
        }
    }
}

fn validate_grid(
    grid: DirectOutputGrid,
    prepared: &PreparedGeodesic,
    context: &mut MetricContext,
    chunks: &mut ChunkObserver<'_>,
) -> Result<(), GeoError> {
    let mut pure = crate::context::WorkProgress::new(None);
    let progress = match &mut chunks.progress {
        super::ChunkProgress::Owned(progress) => progress,
        super::ChunkProgress::Borrowed(_) => &mut pure,
    };
    let result = crate::numerical::validate_angular_output_grid(
        grid.decimal_places(),
        prepared
            .reference
            .ellipsoid()
            .normal_metric_bounds_ref()
            .1
            .exact(),
        None,
        context,
        progress,
    );
    // The first numerical phase includes the already observed exact grid
    // admission, so subsequent chunk receipts charge only its new work.
    chunks.entry_work = context.work_items();
    chunks.first_phase = true;
    result
}

/// WGS84 direct endpoint at the frozen fifteen-place angular grid.
///
/// # Errors
///
/// Uses [`PreparedGeodesic::direct`]'s domain, numerical and resource contract.
#[allow(clippy::needless_pass_by_value)] // Owned convenience API; prepared methods borrow inputs.
pub fn direct(start: LonLat, azimuth: Rat, length: Metres) -> Result<DirectResult, GeoError> {
    let mut context = MetricContext::wgs84()?;
    PreparedGeodesic::new(context.reference().clone()).direct(
        &start,
        &azimuth,
        &length,
        &mut context,
    )
}

#[allow(clippy::too_many_arguments)] // Original line inputs and separate restricted observer/scratch capabilities.
fn propagate(
    start: &LonLat,
    azimuth: &Rat,
    length: &Metres,
    grid: DirectOutputGrid,
    prepared: &PreparedGeodesic,
    coefficients: Option<&std::sync::Arc<super::IntegralCoefficients>>,
    order: usize,
    iterations: u32,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<(Rat, Rat, Rat), SolveError> {
    let azimuth_interval = fixed_from_rat(azimuth, math)?;
    let distance = fixed_from_rat(length.exact(), math)?;
    propagate_enclosed(
        start,
        &azimuth_interval,
        Some(azimuth),
        &distance,
        grid,
        prepared,
        coefficients,
        order,
        iterations,
        chunks,
        math,
    )
}

#[allow(clippy::too_many_arguments)] // Certified line inputs and separate restricted scratch/observer capabilities.
fn propagate_enclosed(
    start: &LonLat,
    azimuth: &FixedInterval,
    exact_azimuth: Option<&Rat>,
    distance: &FixedInterval,
    grid: DirectOutputGrid,
    prepared: &PreparedGeodesic,
    coefficients: Option<&std::sync::Arc<super::IntegralCoefficients>>,
    order: usize,
    iterations: u32,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<(Rat, Rat, Rat), SolveError> {
    let line = super::line::LineState::new_with_coefficients(
        start,
        azimuth,
        exact_azimuth,
        prepared,
        coefficients,
        order,
        chunks,
        math,
    )?;
    let a = &line.a;
    let b = &line.b;
    let radians_per_degree = &line.radians_per_degree;
    // Correct rounding limits each angular error to half the declared
    // quantum. The complete L1 surface bound is R*(pi/180)*quantum.
    let grid_quantum = fixed_from_rat(&Rat::from_decimal(Int::one(), grid.decimal_places()), math)?;
    let radius_bound = a.square(math)?.div(b, math)?;
    let grid_error = radius_bound
        .mul(radians_per_degree, math)?
        .mul(&grid_quantum, math)?;
    let maximum_error = fixed_from_rat(&Rat::parse_decimal("0.000001").expect("frozen"), math)?;
    if grid_error.upper() > maximum_error.lower() {
        return Err(MathError::PrecisionExhausted.into());
    }

    if start.latitude().is_zero()
        && exact_azimuth
            .is_some_and(|angle| angle == &Rat::from_i64(90) || angle == &Rat::from_i64(270))
    {
        let mut delta = distance.div(a, math)?.div(radians_per_degree, math)?;
        if exact_azimuth == Some(&Rat::from_i64(270)) {
            delta = delta.neg(math)?;
        }
        let longitude = fixed_from_rat(start.longitude(), math)?.add(&delta, math)?;
        return Ok((
            quantize_angle_grid(&longitude, -180, grid, math)?,
            Rat::zero(),
            exact_azimuth.expect("equatorial exact angle").clone(),
        ));
    }
    let arc = super::arc_view::FixedArcLine::from_state(
        prepared.reference.id(),
        start.clone(),
        azimuth.clone(),
        exact_azimuth.cloned(),
        distance.clone(),
        line,
    );
    let (mut lower, mut upper) = arc.state().initial_sigma(distance, math)?;
    for _ in 0..iterations {
        let contracted = arc
            .state()
            .contract_sigma(distance, &mut lower, &mut upper, chunks, math)?;
        let sigma = FixedInterval::from_bounds(lower.lower().clone(), upper.upper().clone(), math)?;
        match endpoint(&arc, &sigma, grid, chunks, math) {
            Ok(result) => return Ok(result),
            Err(SolveError::Math(MathError::PrecisionExhausted | MathError::Domain(_))) => {
                if !contracted {
                    return Err(MathError::PrecisionExhausted.into());
                }
            }
            Err(error) => return Err(error),
        }
    }
    Err(SolveError::Iterations)
}

fn endpoint(
    arc: &super::arc_view::FixedArcLine,
    sigma: &FixedInterval,
    grid: DirectOutputGrid,
    chunks: &mut ChunkObserver<'_>,
    math: &mut CoordinateMath,
) -> Result<(Rat, Rat, Rat), SolveError> {
    let image = arc.image(sigma, false, chunks, math)?;
    let line = arc.state();
    let latitude = quantize_decimal(
        &image.latitude.div(&line.radians_per_degree, math)?,
        grid,
        math,
    )?;
    let (_, cos) = sigma.sin_cos(math)?;
    let final_azimuth = if line.sin0.lower().is_zero() && line.sin0.upper().is_zero() {
        if cos.lower() > &BigInt::from_i128(0) {
            Rat::zero()
        } else if cos.upper().is_negative() {
            Rat::from_i64(180)
        } else {
            return Err(MathError::PrecisionExhausted.into());
        }
    } else {
        quantize_angle_grid(
            &FixedInterval::atan2(&line.sin0, &line.cos0.mul(&cos, math)?, math)?
                .div(&line.radians_per_degree, math)?,
            0,
            grid,
            math,
        )?
    };
    let longitude = if let Some(exact) = image.exact_longitude_degrees {
        normalize(
            &Rat::from_decimal(
                exact.round_to_scale(grid.decimal_places()),
                grid.decimal_places(),
            ),
            -180,
        )
    } else {
        quantize_angle_grid(
            &image.longitude.div(&line.radians_per_degree, math)?,
            -180,
            grid,
            math,
        )?
    };
    Ok((longitude, latitude, final_azimuth))
}

pub(crate) fn canonical_endpoint(longitude: Rat, latitude: Rat) -> Result<LonLat, GeoError> {
    // A rounded pole has only one surface location. Replacing its longitude
    // by zero cannot enlarge the latitude quantization displacement.
    LonLat::new(
        if latitude.abs() == Rat::from_i64(90) {
            Rat::zero()
        } else {
            longitude
        },
        latitude,
    )
}

fn quantize_decimal(
    value: &FixedInterval,
    grid: DirectOutputGrid,
    math: &mut CoordinateMath,
) -> Result<Rat, MathError> {
    let (lower, upper) = value.round_decimal(grid.decimal_places(), math)?;
    if lower != upper {
        return Err(MathError::PrecisionExhausted);
    }
    Ok(Rat::from_decimal(
        carrier_integer(&lower),
        grid.decimal_places(),
    ))
}

pub(super) fn quantize_angle(
    value: &FixedInterval,
    origin: i64,
    math: &mut CoordinateMath,
) -> Result<Rat, MathError> {
    quantize_angle_grid(value, origin, DirectOutputGrid::DEGREE15, math)
}

fn quantize_angle_grid(
    value: &FixedInterval,
    origin: i64,
    grid: DirectOutputGrid,
    math: &mut CoordinateMath,
) -> Result<Rat, MathError> {
    let bits = value
        .lower()
        .bits_upper_bound()
        .max(value.upper().bits_upper_bound())
        .saturating_add(value.precision_bits() as usize);
    let reservation = bits
        .div_ceil(8)
        .checked_mul(32)
        .and_then(|bytes| bytes.checked_add(512))
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(reservation)?;
    let result = quantize_angle_bounds(value, origin, grid, math);
    math.release_workspace(reservation)?;
    result
}

fn quantize_angle_bounds(
    value: &FixedInterval,
    origin: i64,
    grid: DirectOutputGrid,
    math: &mut CoordinateMath,
) -> Result<Rat, MathError> {
    let (lower, upper) = crate::numerical::exact_bounds_in(value, math)?;
    let bits = lower
        .numerator()
        .bit_len()
        .max(upper.numerator().bit_len())
        .max(lower.denominator().bit_len())
        .max(upper.denominator().bit_len());
    math.admit_exact(
        16,
        usize::try_from(bits.saturating_add(65)).map_err(|_| MathError::WorkspaceExhausted)?,
    )?;
    let (lower, upper) = if let Some(scratch) = math.limb_scratch() {
        (
            lower
                .modulo_integer_in(origin, 360, scratch)
                .map_err(MathError::LimbScratch)?
                .expect("positive period"),
            upper
                .modulo_integer_in(origin, 360, scratch)
                .map_err(MathError::LimbScratch)?
                .expect("positive period"),
        )
    } else {
        (normalize(&lower, origin), normalize(&upper, origin))
    };
    // Both values must inhabit one continuous output chart. An enclosure that
    // straddles its cut is refined rather than choosing an arbitrary spelling.
    let compare_cost = purrdf_xsd::integer::ExactArithmeticCost::for_rational_operands(
        purrdf_xsd::integer::ExactOperation::RationalCompare,
        [&lower, &upper].map(|value| (value.numerator().bit_len(), value.denominator().bit_len())),
        1,
    )
    .ok_or(MathError::WorkExhausted)?;
    math.admit_exact_cost(compare_cost)?;
    let ordering = match math.limb_scratch() {
        Some(scratch) => lower
            .compare_in(&upper, scratch)
            .map_err(MathError::LimbScratch)?,
        None => lower.cmp(&upper),
    };
    if ordering.is_gt() {
        return Err(MathError::PrecisionExhausted);
    }
    let (lower, upper) = if let Some(scratch) = math.limb_scratch() {
        (
            lower
                .round_to_scale_in(grid.decimal_places(), scratch)
                .map_err(MathError::LimbScratch)?,
            upper
                .round_to_scale_in(grid.decimal_places(), scratch)
                .map_err(MathError::LimbScratch)?,
        )
    } else {
        (
            lower.round_to_scale(grid.decimal_places()),
            upper.round_to_scale(grid.decimal_places()),
        )
    };
    if lower != upper {
        return Err(MathError::PrecisionExhausted);
    }
    Ok(normalize(
        &Rat::from_decimal(lower, grid.decimal_places()),
        origin,
    ))
}

/// Canonical degree reduction uses exact integer remainder, with no epsilon.
pub(super) use crate::geographic::normalize_degrees as normalize;

impl PreparedGeodesic {
    /// Evaluate the unique shortest endpoint-defined arc at exact `t∈[0,1]`.
    /// The unrounded certified inverse azimuth and distance feed the shared
    /// direct enclosure solver; rounded inverse response fields are never inputs.
    ///
    /// # Errors
    /// Refuses nonunique endpoint branches, parameter range, unresolved rounding
    /// and the same numerical/resource failures as direct propagation.
    pub fn shortest_arc_at(
        &self,
        a: &LonLat,
        b: &LonLat,
        t: &Rat,
        context: &mut MetricContext,
    ) -> Result<DirectResult, GeoError> {
        self.shortest_arc_at_observed(a, b, t, context, None)
    }

    /// Evaluate the same implicit arc while polling an external governor.
    ///
    /// # Errors
    /// Adds the observer's typed failures to [`Self::shortest_arc_at`].
    pub fn shortest_arc_at_metered(
        &self,
        a: &LonLat,
        b: &LonLat,
        t: &Rat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<DirectResult, GeoError> {
        self.shortest_arc_at_observed(a, b, t, context, Some(observer))
    }

    fn shortest_arc_at_observed(
        &self,
        a: &LonLat,
        b: &LonLat,
        t: &Rat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<DirectResult, GeoError> {
        if t < &Rat::zero() || t > &Rat::one() {
            return Err(GeoError::domain("geodesic arc parameter must be in [0,1]"));
        }
        context.begin(1)?;
        let metered = observer.is_some();
        let mut progress = crate::context::WorkProgress::new(observer);
        let policy = context.policy();
        let maximum = policy.limits().max_precision_bits;
        let mut bits = maximum.min(80);
        loop {
            let (inverse, proof) = {
                let workspace = policy
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace());
                let previous_work = context.work_items();
                let mut nested =
                    progress.nested(previous_work, workspace, context.workspace_peak());
                let mut continuation =
                    crate::MetricWorkContinuation::new(&mut nested, previous_work, workspace);
                self.compute_inverse_precision(
                    a,
                    b,
                    context,
                    if metered {
                        Some(&mut continuation)
                    } else {
                        None
                    },
                    bits,
                )?
            };
            if inverse.branch_multiplicity() != super::ShortestBranchMultiplicity::Unique {
                return Err(GeoError::AmbiguousGeodesic);
            }
            if t.is_zero() || t == &Rat::one() || a.same_location(b) {
                let endpoint = if t == &Rat::one() { b } else { a };
                let longitude = if endpoint.is_pole() {
                    Rat::zero()
                } else {
                    normalize(
                        &Rat::from_decimal(endpoint.longitude().round_to_scale(15), 15),
                        -180,
                    )
                };
                let latitude = Rat::from_decimal(endpoint.latitude().round_to_scale(15), 15);
                let final_azimuth = if t == &Rat::one() {
                    inverse.final_azimuth()
                } else {
                    inverse.forward_azimuth()
                }
                .cloned()
                .unwrap_or_else(Rat::zero);
                return Ok(DirectResult {
                    endpoint: canonical_endpoint(longitude, latitude)?,
                    final_azimuth,
                    binding: self.reference.id(),
                    grid: DirectOutputGrid::DEGREE15,
                });
            }
            bits = proof.distance.precision_bits;
            let phase_base = context.work_items();
            let workspace = policy
                .limits()
                .max_workspace_bytes
                .saturating_sub(context.remaining_workspace());
            let mut nested = progress.nested(phase_base, workspace, context.workspace_peak());
            context.charge_work(1)?;
            let mut chunks = ChunkObserver::new(if metered { Some(&mut nested) } else { None });
            chunks.initial().map_err(super::solve_external_error)?;
            let order = super::integral_order(self.reference.ellipsoid(), bits);
            let coefficients = self.worker_coefficients(bits, order, context, &mut chunks)?;
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
            let reservation = usize::try_from(
                (u64::from(bits.div_ceil(8)) * 8 + 128).saturating_mul(2 * (order as u64 + 1) + 96),
            )
            .map_err(|_| GeoError::MemoryExhausted {
                limit: policy.limits().max_workspace_bytes,
            })?;
            math.reserve_workspace(reservation)
                .map_err(|error| geo_math_error(&error, policy))?;
            let result = (|| -> Result<(Rat, Rat, Rat), SolveError> {
                let (lower, upper) = proof.forward_azimuth.as_ref().ok_or(MathError::Domain(
                    "nonzero shortest arc lacks azimuth enclosure",
                ))?;
                let azimuth = fixed_from_bounds(lower, upper, &mut math)?;
                let exact_azimuth = (lower == upper).then_some(lower);
                let distance = fixed_from_bounds(
                    proof.distance.lower.exact(),
                    proof.distance.upper.exact(),
                    &mut math,
                )?
                .mul(&fixed_from_rat(t, &mut math)?, &mut math)?;
                propagate_enclosed(
                    a,
                    &azimuth,
                    exact_azimuth,
                    &distance,
                    DirectOutputGrid::DEGREE15,
                    self,
                    coefficients.as_ref(),
                    order,
                    policy.limits().max_iterations,
                    &mut chunks,
                    &mut math,
                )
            })();
            let observed = if matches!(&result, Err(SolveError::External(_))) {
                Ok(())
            } else {
                chunks.finish(&math)
            };
            math.release_workspace(reservation)
                .map_err(|error| geo_math_error(&error, policy))?;
            context.charge_work(math.work_used())?;
            context.admit_workspace(math.workspace_peak() as u64)?;
            context.release_workspace(math.workspace_peak() as u64)?;
            observed.map_err(super::solve_external_error)?;
            match result {
                Ok((longitude, latitude, final_azimuth)) => {
                    return Ok(DirectResult {
                        endpoint: canonical_endpoint(longitude, latitude)?,
                        final_azimuth,
                        binding: self.reference.id(),
                        grid: DirectOutputGrid::DEGREE15,
                    });
                }
                Err(SolveError::Math(MathError::PrecisionExhausted | MathError::Domain(_))) => {}
                Err(SolveError::Math(error)) => return Err(geo_math_error(&error, policy)),
                Err(SolveError::Iterations) => {
                    return Err(GeoError::ConvergenceExhausted {
                        iterations: policy.limits().max_iterations,
                    });
                }
                Err(SolveError::External(error)) => return Err(error),
            }
            if bits == maximum {
                return Err(GeoError::PrecisionExhausted { bits: maximum });
            }
            bits = super::next_precision(bits, maximum);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::direct;
    use crate::{GeoError, LonLat, Metres, MetricWorkObserver, Rat};

    struct Ledger(Vec<(u64, u64)>);
    impl MetricWorkObserver for Ledger {
        fn charge_chunk(&mut self, work: u64, bytes: u64) -> Result<(), GeoError> {
            self.0.push((work, bytes));
            Ok(())
        }
    }

    fn decimal(value: &str) -> Rat {
        Rat::parse_decimal(value).unwrap()
    }

    #[test]
    fn admitted_reference_refuses_cold_and_warm_wide_sources_before_copying() {
        use crate::context::WorkProgress;
        use crate::{
            AxisOrder, ExecutionLimits, ExecutionPolicy, GeographicReference, Int, MetricContext,
            PreparationBudget, PreparedEllipsoid, PreparedGeodesic,
        };
        let adequate = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 1_000_000_000,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut preparation = PreparationBudget::new(adequate);
        let ellipsoid = PreparedEllipsoid::new_in_budget(
            Rat::from_int(Int::one().shl(4096).add(&Int::one())),
            Rat::from_i64(300),
            &mut preparation,
        )
        .unwrap();
        for warm in [false, true] {
            for memory in [false, true] {
                let reference = GeographicReference::new(
                    ellipsoid.clone(),
                    purrdf_hash::hex::Digest32::new([97; 32]),
                    AxisOrder::LonLat,
                );
                let prepared = PreparedGeodesic::new(reference.clone());
                let policy = ExecutionPolicy::new(ExecutionLimits {
                    max_work_items: if memory { 1_000_000_000 } else { 1 },
                    max_workspace_bytes: if memory { 1 } else { 64 * 1024 * 1024 },
                    ..ExecutionLimits::GEOMETRY
                })
                .unwrap();
                let mut context = MetricContext::new(reference, policy).unwrap();
                if warm {
                    assert_eq!(prepared.reference().id(), context.reference().id());
                }
                context.begin(1).unwrap();
                let mut ledger = Ledger(Vec::new());
                let mut progress = WorkProgress::new(Some(&mut ledger));
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let result = prepared.admitted_reference(&mut context, &mut progress);
                let measured = window.close();
                assert!(if memory {
                    matches!(result, Err(GeoError::MemoryExhausted { limit: 1 }))
                } else {
                    matches!(result, Err(GeoError::WorkExhausted { limit: 1 }))
                });
                assert_eq!(measured.allocations, 0, "warm {warm}, memory {memory}");
                assert_eq!(measured.requested_bytes, 0);
                assert_eq!(context.current_workspace_bytes(), 0);
                assert_eq!(context.workspace_peak(), 0);
                assert_eq!(context.work_items(), 0);
                assert_eq!(ledger.0, [] as [(u64, u64); 0]);
                assert_eq!(prepared.reference().cached_identity().is_some(), warm);
                assert_eq!(context.reference().cached_identity().is_some(), warm);
                if !warm {
                    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                    std::hint::black_box(prepared.reference().id());
                    assert!(window.close().allocations > 0);
                }
            }
        }
    }

    #[test]
    fn admitted_direct_samples_keep_public_certificates_and_check_each_context() {
        use crate::context::WorkProgress;
        use crate::{
            AxisOrder, ExecutionPolicy, GeographicReference, MetricContext, PreparedGeodesic,
        };
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        context.begin(1).unwrap();
        let mut admission = Ledger(Vec::new());
        let witness = prepared
            .admitted_reference(&mut context, &mut WorkProgress::new(Some(&mut admission)))
            .unwrap();
        assert_eq!(
            admission.0.iter().map(|&(work, _)| work).sum::<u64>(),
            context.work_items()
        );
        assert_eq!(
            admission.0.iter().map(|&(_, bytes)| bytes).sum::<u64>(),
            context.workspace_peak()
        );
        let start = LonLat::new(decimal("12"), decimal("23")).unwrap();
        let length = Metres::new(decimal("1000"));
        // A fresh matching context exercises the admitted cold-cache path;
        // the second sample reuses its initialized immutable binding.
        let mut samples = MetricContext::wgs84().unwrap();
        for azimuth in [decimal("47"), decimal("227")] {
            let expected = prepared
                .direct(&start, &azimuth, &length, &mut context)
                .unwrap();
            let mut ledger = Ledger(Vec::new());
            let result = witness
                .direct_metered(&start, &azimuth, &length, &mut samples, &mut ledger)
                .unwrap();
            assert_eq!(result.certificate_bytes(), expected.certificate_bytes());
            assert_eq!(
                ledger.0.iter().map(|&(work, _)| work).sum::<u64>(),
                samples.work_items()
            );
            assert_eq!(
                ledger.0.iter().map(|&(_, bytes)| bytes).sum::<u64>(),
                samples.workspace_peak()
            );
        }
        let foreign = GeographicReference::new(
            prepared.reference().ellipsoid().clone(),
            purrdf_hash::hex::Digest32::new([98; 32]),
            AxisOrder::LonLat,
        );
        let mut wrong = MetricContext::new(foreign, ExecutionPolicy::geometry()).unwrap();
        wrong.begin(1).unwrap();
        assert!(matches!(
            prepared.admitted_reference(&mut wrong, &mut WorkProgress::new(None)),
            Err(GeoError::Config(_))
        ));
        for _ in 0..2 {
            let mut ledger = Ledger(Vec::new());
            assert!(matches!(
                witness.direct_metered(&start, &decimal("47"), &length, &mut wrong, &mut ledger,),
                Err(GeoError::Config(_))
            ));
            assert_eq!(wrong.current_workspace_bytes(), 0);
            assert_eq!(
                ledger.0.iter().map(|&(work, _)| work).sum::<u64>(),
                wrong.work_items()
            );
            assert_eq!(
                ledger.0.iter().map(|&(_, bytes)| bytes).sum::<u64>(),
                wrong.workspace_peak()
            );
        }
    }

    #[test]
    fn original_rational_zero_distance_uses_half_even_and_canonical_seam() {
        let start = LonLat::new(decimal("180"), decimal("0.0000000000000005")).unwrap();
        let result = direct(start, decimal("360"), Metres::new(Rat::zero())).unwrap();
        assert_eq!(result.endpoint().longitude(), &decimal("-180"));
        assert!(result.endpoint().latitude().is_zero());
        assert!(result.final_azimuth().is_zero());
    }

    #[test]
    fn every_direct_grid_proves_its_bound_on_the_original_huge_ellipsoid() {
        use super::DirectOutputGrid;
        use crate::context::WorkProgress;
        use crate::{
            AxisOrder, ExecutionPolicy, GeographicReference, MetricContext, PreparedEllipsoid,
            PreparedGeodesic,
        };
        let ellipsoid =
            PreparedEllipsoid::new(decimal("1000000000000"), Rat::from_i64(300)).unwrap();
        let reference = GeographicReference::new(
            ellipsoid,
            purrdf_hash::hex::Digest32::new([99; 32]),
            AxisOrder::LonLat,
        );
        let prepared = PreparedGeodesic::new(reference.clone());
        let mut context = MetricContext::new(reference, ExecutionPolicy::geometry()).unwrap();
        let start = LonLat::new(decimal("12"), decimal("23")).unwrap();
        let azimuth = decimal("47");
        for length in [Metres::new(Rat::zero()), Metres::new(decimal("1000"))] {
            for grid in [DirectOutputGrid::DEGREE15, DirectOutputGrid::new(12)] {
                let mut ledger = Ledger(Vec::new());
                let cached = prepared.reference().cached_identity();
                assert!(matches!(
                    prepared.direct_with_grid_metered(
                        &start,
                        &azimuth,
                        &length,
                        grid,
                        &mut context,
                        &mut ledger,
                    ),
                    Err(GeoError::PrecisionExhausted { .. })
                ));
                assert_eq!(
                    context.current_workspace_bytes(),
                    context.retained_workspace_bytes()
                );
                assert_eq!(
                    ledger.0.iter().map(|&(work, _)| work).sum::<u64>(),
                    context.work_items()
                );
                assert_eq!(
                    ledger.0.iter().map(|&(_, bytes)| bytes).sum::<u64>(),
                    context.workspace_peak()
                );
                assert_eq!(prepared.reference().cached_identity(), cached);
            }
            let result = prepared
                .direct_with_grid(
                    &start,
                    &azimuth,
                    &length,
                    DirectOutputGrid::new(18),
                    &mut context,
                )
                .unwrap();
            assert_eq!(result.output_grid(), DirectOutputGrid::new(18));
            assert_eq!(result.surface_error_bound().exact(), &decimal("0.000001"));
        }
        context.begin(1).unwrap();
        assert!(matches!(
            prepared.admitted_reference(&mut context, &mut WorkProgress::new(None)),
            Err(GeoError::PrecisionExhausted { .. })
        ));
        assert_eq!(
            context.current_workspace_bytes(),
            context.retained_workspace_bytes()
        );
    }

    #[test]
    fn direct_equator_and_public_example_use_frozen_grid() {
        let result = direct(
            LonLat::new(Rat::zero(), Rat::zero()).unwrap(),
            decimal("90"),
            Metres::new(decimal("111319.49079327357")),
        )
        .unwrap();
        assert_eq!(result.endpoint().longitude(), &decimal("1"));
        assert!(result.endpoint().latitude().is_zero());
        // Karney's published mathematical direct example (Table 2).
        let result = direct(
            LonLat::new(Rat::zero(), decimal("40")).unwrap(),
            decimal("30"),
            Metres::new(decimal("10000000")),
        )
        .unwrap();
        assert!(
            result
                .endpoint()
                .latitude()
                .sub(&decimal("41.793310205056"))
                .abs()
                < decimal("0.000000000001")
        );
        assert!(
            result
                .endpoint()
                .longitude()
                .sub(&decimal("137.84490004377"))
                .abs()
                < decimal("0.00000000001")
        );
    }

    #[test]
    fn exact_pole_meridians_round_the_source_proved_longitude_cut() {
        for (latitude, azimuth, longitude) in [
            ("90", "0", "-180"),
            ("90", "180", "0"),
            ("-90", "0", "0"),
            ("-90", "180", "-180"),
        ] {
            let result = direct(
                LonLat::new(decimal("73"), decimal(latitude)).unwrap(),
                decimal(azimuth),
                Metres::new(decimal("1000.05")),
            )
            .unwrap();
            assert_eq!(result.endpoint().longitude(), &decimal(longitude));
            assert!(result.endpoint().latitude().abs() < decimal("90"));
        }
        let result = direct(
            LonLat::new(decimal("180"), decimal("30")).unwrap(),
            decimal("0"),
            Metres::new(decimal("1000.05")),
        )
        .unwrap();
        assert_eq!(result.endpoint().longitude(), &decimal("-180"));
    }
    #[test]
    fn implicit_arc_retains_true_endpoint_law_and_governor_totals() {
        use crate::{
            GeoError, GeographicReference, MetricContext, MetricWorkObserver, PreparedGeodesic,
        };
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        let a = LonLat::new(decimal("0"), decimal("0")).unwrap();
        let b = LonLat::new(decimal("2"), decimal("0")).unwrap();
        let midpoint = prepared
            .shortest_arc_at(&a, &b, &decimal("0.5"), &mut context)
            .unwrap();
        assert_eq!(
            midpoint.endpoint(),
            &LonLat::new(decimal("1"), decimal("0")).unwrap()
        );
        let a = LonLat::new(decimal("-70"), decimal("40")).unwrap();
        let b = LonLat::new(decimal("80"), decimal("-20")).unwrap();
        let first = prepared
            .shortest_arc_at(&a, &b, &decimal("0.37"), &mut context)
            .unwrap();
        let reversed = prepared
            .shortest_arc_at(&b, &a, &decimal("0.63"), &mut context)
            .unwrap();
        assert_eq!(first.endpoint(), reversed.endpoint());
        #[derive(Default)]
        struct Counts {
            work: u64,
            workspace: u64,
        }
        impl MetricWorkObserver for Counts {
            fn charge_chunk(&mut self, work: u64, workspace: u64) -> Result<(), GeoError> {
                self.work += work;
                self.workspace += workspace;
                Ok(())
            }
        }
        let mut counts = Counts::default();
        let metered = prepared
            .shortest_arc_at_metered(&a, &b, &decimal("0.37"), &mut context, &mut counts)
            .unwrap();
        assert_eq!(first.certificate_bytes(), metered.certificate_bytes());
        assert_eq!(counts.work, context.work_items());
        assert_eq!(counts.workspace, context.workspace_peak());
        let antipode = LonLat::new(decimal("180"), decimal("0")).unwrap();
        assert_eq!(
            prepared.shortest_arc_at(
                &LonLat::new(Rat::zero(), Rat::zero()).unwrap(),
                &antipode,
                &decimal("0.5"),
                &mut context
            ),
            Err(GeoError::AmbiguousGeodesic)
        );
    }

    #[test]
    fn explicit_direct_grids_preserve_default_and_prove_the_declared_quantum() {
        use super::DirectOutputGrid;
        use crate::{GeographicReference, MetricContext, PreparedGeodesic};
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        let start = LonLat::new(Rat::zero(), Rat::zero()).unwrap();
        let azimuth = decimal("90");
        let length = Metres::new(decimal("1000"));
        let standard = prepared
            .direct(&start, &azimuth, &length, &mut context)
            .unwrap();
        let explicit = prepared
            .direct_with_grid(
                &start,
                &azimuth,
                &length,
                DirectOutputGrid::new(15),
                &mut context,
            )
            .unwrap();
        assert_eq!(standard.certificate_bytes(), explicit.certificate_bytes());
        for (places, expected) in [(12, "0.008983152841"), (18, "0.008983152841195214")] {
            let result = prepared
                .direct_with_grid(
                    &start,
                    &azimuth,
                    &length,
                    DirectOutputGrid::new(places),
                    &mut context,
                )
                .unwrap();
            assert_eq!(result.endpoint().longitude(), &decimal(expected));
            assert_eq!(result.endpoint().latitude(), &Rat::zero());
            assert_eq!(result.output_grid().decimal_places(), places);
            assert_ne!(result.law_id(), standard.law_id());
            assert_eq!(result.surface_error_bound().exact(), &decimal("0.000001"));
        }
        let halfway = LonLat::new(decimal("1.2345678901235"), decimal("-2.0000000000005")).unwrap();
        let rounded = prepared
            .direct_with_grid(
                &halfway,
                &Rat::zero(),
                &Metres::new(Rat::zero()),
                DirectOutputGrid::new(12),
                &mut context,
            )
            .unwrap();
        assert_eq!(rounded.endpoint().longitude(), &decimal("1.234567890124"));
        assert_eq!(rounded.endpoint().latitude(), &decimal("-2"));
    }

    #[test]
    fn alternate_grid_admission_is_metered_and_never_returns_partial_batches() {
        use super::DirectOutputGrid;
        use crate::{GeographicReference, MetricContext, PreparedGeodesic};
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        let start = LonLat::new(decimal("12"), decimal("23")).unwrap();
        let azimuth = decimal("47");
        let length = Metres::new(decimal("1000"));
        let mut ledger = Ledger(Vec::new());
        let completed = prepared
            .direct_with_grid_metered(
                &start,
                &azimuth,
                &length,
                DirectOutputGrid::new(12),
                &mut context,
                &mut ledger,
            )
            .unwrap();
        assert_eq!(
            ledger.0.iter().map(|&(work, _)| work).sum::<u64>(),
            context.work_items()
        );
        assert_eq!(
            ledger.0.iter().map(|&(_, bytes)| bytes).sum::<u64>(),
            context.workspace_peak()
        );
        let mut outputs = [Some(completed)];
        assert!(matches!(
            prepared.direct_batch_with_grid(
                &[(start.clone(), azimuth.clone(), length.clone())],
                &mut outputs,
                DirectOutputGrid::new(11),
                &mut context
            ),
            Err(GeoError::PrecisionExhausted { .. })
        ));
        assert_eq!(outputs, [None]);
        assert!(matches!(
            prepared.direct_with_grid(
                &start,
                &azimuth,
                &length,
                DirectOutputGrid::new(u32::MAX),
                &mut context
            ),
            Err(GeoError::PrecisionExhausted { .. })
        ));
        let declaration_work = crate::numerical::reference_copy_cost(prepared.reference())
            .unwrap()
            .followed_by(crate::numerical::reference_copy_cost(context.reference()).unwrap())
            .unwrap()
            .work_items;
        assert_eq!(context.work_items(), 1 + declaration_work);
        assert_eq!(context.workspace_peak(), context.retained_workspace_bytes());
    }

    #[test]
    fn borrowed_direct_batches_charge_every_entry_and_retained_plus_scratch_peak() {
        use super::DirectOutputGrid;
        use crate::{GeographicReference, MetricContext, PreparedGeodesic};
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let start = LonLat::new(decimal("12"), decimal("23")).unwrap();
        let inputs = [
            (start.clone(), decimal("47"), Metres::new(decimal("1000"))),
            (start.clone(), Rat::zero(), Metres::new(Rat::zero())),
            (start, decimal("90"), Metres::new(decimal("1"))),
        ];
        for grid in [DirectOutputGrid::DEGREE15, DirectOutputGrid::new(12)] {
            let mut context = MetricContext::wgs84().unwrap();
            context.set_retained_workspace(256).unwrap();
            let mut ledger = Ledger(Vec::new());
            let mut output = [None, None, None];
            prepared
                .direct_batch_borrowed_metered(
                    inputs
                        .iter()
                        .map(|(start, azimuth, length)| (start, azimuth, length)),
                    &mut output,
                    grid,
                    &mut context,
                    &mut ledger,
                )
                .unwrap();
            assert!(output.iter().all(Option::is_some));
            assert_eq!(
                ledger.0.iter().map(|&(work, _)| work).sum::<u64>(),
                context.work_items()
            );
            assert_eq!(
                ledger.0.iter().map(|&(_, bytes)| bytes).sum::<u64>(),
                context.workspace_peak()
            );
        }
    }
}
