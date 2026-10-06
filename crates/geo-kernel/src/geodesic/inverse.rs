// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Canonical inverse branch metadata from the shared auxiliary-sphere solver.

pub(crate) mod area;

use purrdf_hash::Domain;
use purrdf_xsd::math::{
    CertifiedInterval, CoordinateMath, FixedInterval, IntervalContext, MathError, MathLimits,
};

use super::{
    CanonicalPair, ChunkObserver, Hybrid, IntegralCoefficients, InverseProblem, PreparedGeodesic,
    SolveError, SpherePair, band::DistanceGoal, direct, solve_distance_with, solve_external_error,
};
use crate::numerical::{
    carrier_integer, exact_bounds, exact_bounds_in, fixed_from_rat, geo_math_error,
};
use crate::{
    GeoBindingId, GeoError, GeographicReference, Int, LonLat, Metres, MetricContext,
    MetricEstimate, MetricProofReceipt, MetricWorkObserver, PointLaw, Rat, SemanticLawId,
    SquareMetres,
};

const INVERSE_CERTIFICATE: Domain = Domain::new(b"purrdf/inverse-geodesic-certificate/v1");

/// Certified multiplicity of shortest endpoint geodesics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortestBranchMultiplicity {
    /// One shortest branch, including coalesced cut-locus endpoints.
    Unique,
    /// Two distinct equal shortest branches.
    Two,
    /// A continuum of shortest branches between opposite exact poles.
    Continuum,
}

/// Correctly quantized metadata for the canonically selected shortest branch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InverseResult {
    distance: MetricEstimate,
    forward_azimuth: Option<Rat>,
    final_azimuth: Option<Rat>,
    arc_degrees: Rat,
    reduced_length: Metres,
    scale12: Rat,
    scale21: Rat,
    geodesic_quadrilateral_area: SquareMetres,
    pole_cuts: Vec<(Rat, Rat, Rat)>,
    multiplicity: ShortestBranchMultiplicity,
}

impl InverseResult {
    /// Original integer owners copied by a completed inverse-result clone.
    pub(crate) fn owned_integer_operands(&self) -> impl Iterator<Item = &Int> {
        self.distance.owned_integer_operands().chain(
            self.forward_azimuth
                .iter()
                .chain(self.final_azimuth.iter())
                .chain([
                    &self.arc_degrees,
                    self.reduced_length.exact(),
                    &self.scale12,
                    &self.scale21,
                    self.geodesic_quadrilateral_area.exact(),
                ])
                .chain(
                    self.pole_cuts
                        .iter()
                        .flat_map(|(latitude, from, to)| [latitude, from, to]),
                )
                .flat_map(Rat::integer_operands),
        )
    }

    /// Correctly half-even rounded true shortest distance.
    #[must_use]
    pub const fn distance(&self) -> &MetricEstimate {
        &self.distance
    }
    /// Forward azimuth, in `[0,360)` degrees; absent at zero distance.
    #[must_use]
    pub const fn forward_azimuth(&self) -> Option<&Rat> {
        self.forward_azimuth.as_ref()
    }
    /// Forward final azimuth in the declared longitude-zero pole frame.
    #[must_use]
    pub const fn final_azimuth(&self) -> Option<&Rat> {
        self.final_azimuth.as_ref()
    }
    /// Positive auxiliary-sphere arc, in degrees.
    #[must_use]
    pub const fn arc_degrees(&self) -> &Rat {
        &self.arc_degrees
    }
    /// Signed reduced length, in metres.
    #[must_use]
    pub const fn reduced_length(&self) -> &Metres {
        &self.reduced_length
    }
    /// Jacobi scale from the first endpoint to the second.
    #[must_use]
    pub const fn scale12(&self) -> &Rat {
        &self.scale12
    }
    /// Jacobi scale from the second endpoint to the first.
    #[must_use]
    pub const fn scale21(&self) -> &Rat {
        &self.scale21
    }
    /// Karney quadrilateral `S12=+∫Q(phi)dλ`, in signed square metres.
    /// The oriented boundary-area adapter uses the negative of this quantity.
    #[must_use]
    pub const fn geodesic_quadrilateral_area(&self) -> &SquareMetres {
        &self.geodesic_quadrilateral_area
    }
    /// Explicit zero-length longitude cuts `(latitude, from, to)`, in degrees,
    /// at exact poles. Longitudes use the canonical cut `[-180,180)`.
    #[must_use]
    pub fn pole_cuts(&self) -> &[(Rat, Rat, Rat)] {
        &self.pole_cuts
    }
    /// Shortest-branch multiplicity certified before output quantization.
    #[must_use]
    pub const fn branch_multiplicity(&self) -> ShortestBranchMultiplicity {
        self.multiplicity
    }
    /// Completed inverse metadata law, independent of tighter proof evidence.
    #[must_use]
    pub fn law_id(&self) -> SemanticLawId {
        PointLaw::InverseMetadataV1.id()
    }
    /// Exact reference identity.
    #[must_use]
    pub const fn binding_id(&self) -> GeoBindingId {
        self.distance.binding_id()
    }
    /// Canonical framed response: distance certificate, fixed grids, angular,
    /// Jacobi and quadrilateral fields, then explicit pole cuts.
    #[must_use]
    pub fn certificate_bytes(&self) -> Vec<u8> {
        let law = self.law_id().digest();
        let distance = self.distance.certificate_bytes();
        let azimuth1 = self
            .forward_azimuth
            .as_ref()
            .map_or_else(String::new, |v| v.to_decimal_string(15));
        let azimuth2 = self
            .final_azimuth
            .as_ref()
            .map_or_else(String::new, |v| v.to_decimal_string(15));
        let arc = self.arc_degrees.to_decimal_string(15);
        let reduced = self.reduced_length.exact().to_decimal_string(6);
        let scale12 = self.scale12.to_decimal_string(15);
        let scale21 = self.scale21.to_decimal_string(15);
        let area = self
            .geodesic_quadrilateral_area
            .exact()
            .to_decimal_string(2);
        let multiplicity: &[u8] = match self.multiplicity {
            ShortestBranchMultiplicity::Unique => b"unique",
            ShortestBranchMultiplicity::Two => b"two",
            ShortestBranchMultiplicity::Continuum => b"continuum",
        };
        let mut cuts = Vec::new();
        for (latitude, from, to) in &self.pole_cuts {
            for value in [latitude, from, to] {
                purrdf_hash::frame::frame_le(&mut cuts, value.to_decimal_string(15).as_bytes());
            }
        }
        crate::metric::framed_certificate([
            INVERSE_CERTIFICATE.as_bytes(),
            law.as_bytes(),
            &distance,
            b"degree=0.000000000000001;metre=0.000001;scale=0.000000000000001;square-metre=0.01",
            azimuth1.as_bytes(),
            azimuth2.as_bytes(),
            arc.as_bytes(),
            reduced.as_bytes(),
            scale12.as_bytes(),
            scale21.as_bytes(),
            b"geodesic_quadrilateral_area=+integral-Q-dlongitude;cut=[-180,180)",
            area.as_bytes(),
            &cuts,
            multiplicity,
        ])
    }
}

/// Invocation enclosures, kept separate from completed metadata certificates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InverseProofReceipt {
    /// Unrounded shortest distance enclosure and work/precision evidence.
    pub distance: MetricProofReceipt,
    /// Exact shortest-branch multiplicity, independent of rounded azimuths.
    pub branch_multiplicity: ShortestBranchMultiplicity,
    /// Inclusive unwrapped forward azimuth degree bounds, absent at zero.
    pub forward_azimuth: Option<(Rat, Rat)>,
    /// Inclusive unwrapped forward final azimuth degree bounds, absent at zero.
    pub final_azimuth: Option<(Rat, Rat)>,
    /// Auxiliary-sphere arc degree enclosure.
    pub arc_degrees: (Rat, Rat),
    /// Signed reduced-length metre enclosure.
    pub reduced_length: (Metres, Metres),
    /// Jacobi scale enclosure from endpoint one to two.
    pub scale12: (Rat, Rat),
    /// Jacobi scale enclosure from endpoint two to one.
    pub scale21: (Rat, Rat),
    /// Quadrilateral area enclosure, in signed square metres.
    pub geodesic_quadrilateral_area: (SquareMetres, SquareMetres),
}

impl InverseProofReceipt {
    pub(crate) fn operands(&self) -> impl Iterator<Item = &Rat> {
        [self.forward_azimuth.as_ref(), self.final_azimuth.as_ref()]
            .into_iter()
            .flatten()
            .flat_map(|(lower, upper)| [lower, upper])
            .chain([
                self.distance.lower.exact(),
                self.distance.upper.exact(),
                &self.arc_degrees.0,
                &self.arc_degrees.1,
                self.reduced_length.0.exact(),
                self.reduced_length.1.exact(),
                &self.scale12.0,
                &self.scale12.1,
                &self.scale21.0,
                &self.scale21.1,
                self.geodesic_quadrilateral_area.0.exact(),
                self.geodesic_quadrilateral_area.1.exact(),
            ])
    }

    pub(crate) fn into_shared_owned(
        mut self,
        context: &mut MetricContext,
        progress: &mut crate::context::WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        let admission = crate::metric::ReceiptOwnership::for_operands(self.operands())?;
        admission.prepare(context, progress, || {
            let pair = |(lower, upper): (Rat, Rat)| (lower.into_shared(), upper.into_shared());
            self.distance.lower = Metres::new(self.distance.lower.into_exact().into_shared());
            self.distance.upper = Metres::new(self.distance.upper.into_exact().into_shared());
            self.forward_azimuth = self.forward_azimuth.map(pair);
            self.final_azimuth = self.final_azimuth.map(pair);
            self.arc_degrees = pair(self.arc_degrees);
            self.reduced_length = (
                Metres::new(self.reduced_length.0.into_exact().into_shared()),
                Metres::new(self.reduced_length.1.into_exact().into_shared()),
            );
            self.scale12 = pair(self.scale12);
            self.scale21 = pair(self.scale21);
            self.geodesic_quadrilateral_area = (
                SquareMetres::new(
                    self.geodesic_quadrilateral_area
                        .0
                        .into_exact()
                        .into_shared(),
                ),
                SquareMetres::new(
                    self.geodesic_quadrilateral_area
                        .1
                        .into_exact()
                        .into_shared(),
                ),
            );
            Ok(self)
        })
    }
}

pub(super) struct InverseSolution<I> {
    pub(super) distance: I,
    pub(super) metadata: Option<Metadata>,
}
pub(super) struct AuxiliaryArc<I> {
    pub(super) azimuth1: I,
    pub(super) azimuth2_y: I,
    pub(super) azimuth2_x: I,
    pub(super) sigma1: I,
    pub(super) sigma2: I,
    pub(super) sin0: I,
    pub(super) cos0_squared: I,
    pub(super) q: I,
    pub(super) jacobi: I,
}

pub(super) struct Metadata {
    forward: Rat,
    final_azimuth: Rat,
    arc: Rat,
    reduced: Rat,
    scale12: Rat,
    scale21: Rat,
    area: Rat,
    raw: RawMetadata,
}
#[derive(Clone)]
struct RawMetadata {
    multiplicity: ShortestBranchMultiplicity,
    forward: FixedInterval,
    final_azimuth: FixedInterval,
    arc: FixedInterval,
    reduced: FixedInterval,
    scale12: FixedInterval,
    scale21: FixedInterval,
    area: FixedInterval,
}

impl PreparedGeodesic {
    /// Compute canonical inverse metadata using the same shortest-distance body.
    ///
    /// # Errors
    /// Refuses uncertified branch comparisons/rounding and operational limits.
    pub fn inverse(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
    ) -> Result<InverseResult, GeoError> {
        context.begin(1)?;
        self.compute_inverse(a, b, context, None)
            .map(|(answer, _)| answer)
    }
    /// Return separate first-variation/Jacobi enclosures alongside the response.
    ///
    /// # Errors
    /// Uses the same branch, admission and rounding law as [`Self::inverse`].
    pub fn inverse_with_proof(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
    ) -> Result<(InverseResult, InverseProofReceipt), GeoError> {
        context.begin(1)?;
        let (answer, proof) = self.compute_inverse(a, b, context, None)?;
        let mut progress = crate::context::WorkProgress::new(None);
        let mut proof = proof.into_shared_owned(context, &mut progress)?;
        proof.distance.work_items = context.work_items();
        Ok((answer, proof))
    }
    /// Compute an inverse while charging a governor outside numerical chunks.
    ///
    /// # Errors
    /// Adds the observer's typed refusals to [`Self::inverse`].
    pub fn inverse_metered(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<InverseResult, GeoError> {
        context.begin(1)?;
        self.compute_inverse(a, b, context, Some(observer))
            .map(|(answer, _)| answer)
    }
    /// Return invocation enclosures with governed work/cancellation.
    ///
    /// # Errors
    /// Adds the observer's refusals to [`Self::inverse_with_proof`].
    pub fn inverse_with_proof_metered(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(InverseResult, InverseProofReceipt), GeoError> {
        context.begin(1)?;
        let (answer, proof) = self.compute_inverse(a, b, context, Some(observer))?;
        let mut continuation = crate::MetricWorkContinuation::new(
            observer,
            context.work_items(),
            context.workspace_peak(),
        );
        let mut progress = crate::context::WorkProgress::new(Some(&mut continuation));
        let mut proof = proof.into_shared_owned(context, &mut progress)?;
        proof.distance.work_items = context.work_items();
        Ok((answer, proof))
    }
    /// Complete a caller-buffer batch, clearing every slot on a refusal.
    ///
    /// # Errors
    /// Refuses shape, reference, environment, numerical and operational errors.
    pub fn inverse_batch(
        &self,
        inputs: &[(LonLat, LonLat)],
        output: &mut [Option<InverseResult>],
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.inverse_batch_borrowed(inputs.iter().map(|(a, b)| (a, b)), output, context)
    }
    /// Governed inverse batch through the same scalar body.
    ///
    /// # Errors
    /// Adds the observer's typed refusals to [`Self::inverse_batch`].
    pub fn inverse_batch_metered(
        &self,
        inputs: &[(LonLat, LonLat)],
        output: &mut [Option<InverseResult>],
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.inverse_batch_borrowed_metered(
            inputs.iter().map(|(a, b)| (a, b)),
            output,
            context,
            observer,
        )
    }
    /// Complete borrowed inverse inputs with no original-coordinate copies.
    ///
    /// # Errors
    /// Uses the same complete admission and refusal law as [`Self::inverse_batch`].
    pub fn inverse_batch_borrowed<'a>(
        &self,
        inputs: impl ExactSizeIterator<Item = (&'a LonLat, &'a LonLat)>,
        output: &mut [Option<InverseResult>],
        context: &mut MetricContext,
    ) -> Result<(), GeoError> {
        self.inverse_batch_observed(inputs, output, context, None)
    }

    /// Borrowed inverse batch with bounded external charging and cancellation.
    ///
    /// # Errors
    /// Preserves the observer's typed refusal without any partial output.
    pub fn inverse_batch_borrowed_metered<'a>(
        &self,
        inputs: impl ExactSizeIterator<Item = (&'a LonLat, &'a LonLat)>,
        output: &mut [Option<InverseResult>],
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<(), GeoError> {
        self.inverse_batch_observed(inputs, output, context, Some(observer))
    }

    fn inverse_batch_observed<'a>(
        &self,
        inputs: impl ExactSizeIterator<Item = (&'a LonLat, &'a LonLat)>,
        output: &mut [Option<InverseResult>],
        context: &mut MetricContext,
        mut observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<(), GeoError> {
        super::batch(inputs, output, context, |(a, b), context| {
            let result = if let Some(observer) = observer.as_deref_mut() {
                self.compute_inverse(a, b, context, Some(observer))
            } else {
                self.compute_inverse(a, b, context, None)
            };
            result.map(|(answer, _)| answer)
        })
    }
    pub(super) fn compute_inverse(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<(InverseResult, InverseProofReceipt), GeoError> {
        self.compute_inverse_precision(a, b, context, observer, 112)
    }

    pub(super) fn compute_inverse_precision(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
        minimum_bits: u32,
    ) -> Result<(InverseResult, InverseProofReceipt), GeoError> {
        self.compute_inverse_goal(
            a,
            b,
            context,
            observer,
            minimum_bits,
            DistanceGoal::Point(None),
        )
    }

    pub(super) fn compute_arc_inverse_precision(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
        minimum_bits: u32,
    ) -> Result<(InverseResult, InverseProofReceipt), GeoError> {
        if minimum_bits > context.policy().limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted {
                bits: context.policy().limits().max_precision_bits,
            });
        }
        self.compute_inverse_goal(
            a,
            b,
            context,
            observer,
            minimum_bits,
            DistanceGoal::ArcProof(minimum_bits / 2),
        )
    }

    #[allow(clippy::too_many_arguments)] // Exact endpoints, worker/observer and separate arithmetic/proof goals.
    fn compute_inverse_goal(
        &self,
        a: &LonLat,
        b: &LonLat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
        minimum_bits: u32,
        goal: DistanceGoal<'_>,
    ) -> Result<(InverseResult, InverseProofReceipt), GeoError> {
        context.charge_work(1)?;
        let mut chunks = ChunkObserver::new(observer);
        chunks.initial().map_err(solve_external_error)?;
        if !chunks.reference_matches(&self.reference, context)? {
            return Err(GeoError::config(
                "prepared geodesic and metric context references differ",
            ));
        }
        let binding = chunks.reference_identity(Some(&self.reference), context)?;
        if a.same_location(b) {
            let mut progress = chunks
                .into_owned_progress()
                .expect("native inverse owns its observer");
            let distance = crate::metric::complete_point_enclosure_observed(
                &Rat::zero(),
                &Rat::zero(),
                binding,
                context,
                &mut progress,
            )?;
            return Ok(zero_result(distance, context.work_items()));
        }
        chunks.prepare_scratch(
            context,
            super::scratch_source_bits(
                &[a.longitude(), a.latitude(), b.longitude(), b.latitude()],
                self.reference.ellipsoid(),
            ),
        )?;
        let pair = CanonicalPair::new(a, b);
        let policy = context.policy();
        let maximum = policy.limits().max_precision_bits;
        let mut bits = maximum.min(minimum_bits.max(super::initial_inverse_precision(&pair)));
        loop {
            if bits < 16 {
                return Err(GeoError::PrecisionExhausted { bits: maximum });
            }
            chunks.reset();
            let order = super::integral_order(self.reference.ellipsoid(), bits);
            let coefficients = self.worker_coefficients(bits, order, context, &mut chunks)?;
            chunks
                .context_entry(context)
                .map_err(solve_external_error)?;
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
                u64::from(bits.div_ceil(8))
                    .saturating_mul(8)
                    .saturating_add(128)
                    .saturating_mul(2 * (order as u64 + 1) + 128),
            )
            .map_err(|_| GeoError::MemoryExhausted {
                limit: policy.limits().max_workspace_bytes,
            })?;
            math.reserve_workspace(reservation)
                .map_err(|error| geo_math_error(&error, policy))?;
            let solution = solve_distance_with(
                &InverseProblem {
                    pair: &pair,
                    reference: &self.reference,
                    order,
                    iterations: policy.limits().max_iterations,
                    goal,
                    capture: true,
                    source: None,
                    prepared: coefficients.as_deref(),
                },
                &mut chunks,
                &mut IntervalContext::fixed(&mut math),
            );
            let solution = solution.and_then(|solution| {
                let proof = exact_proof(&solution, bits, &mut math)?;
                Ok((solution, proof))
            });
            let observed = chunks.finish(&math);
            math.release_workspace(reservation)
                .map_err(|error| geo_math_error(&error, policy))?;
            context.charge_work(math.work_used())?;
            context.admit_workspace(math.workspace_peak() as u64)?;
            context.release_workspace(math.workspace_peak() as u64)?;
            observed.map_err(solve_external_error)?;
            match solution {
                Ok((solution, mut proof)) => {
                    let metadata = solution
                        .metadata
                        .ok_or_else(|| GeoError::domain("inverse metadata capture missing"))?;
                    let mut progress = chunks
                        .into_owned_progress()
                        .expect("native inverse owns its observer");
                    let distance = crate::metric::complete_point_enclosure_observed(
                        proof.distance.lower.exact(),
                        proof.distance.upper.exact(),
                        binding,
                        context,
                        &mut progress,
                    )?;
                    proof.distance.work_items = context.work_items();
                    let pole_cuts = pole_cuts(a, b, &metadata.forward, &metadata.final_azimuth);
                    return Ok((
                        InverseResult {
                            distance,
                            forward_azimuth: Some(metadata.forward),
                            final_azimuth: Some(metadata.final_azimuth),
                            arc_degrees: metadata.arc,
                            reduced_length: Metres::new(metadata.reduced),
                            scale12: metadata.scale12,
                            scale21: metadata.scale21,
                            geodesic_quadrilateral_area: SquareMetres::new(metadata.area),
                            multiplicity: metadata.raw.multiplicity,
                            pole_cuts,
                        },
                        proof,
                    ));
                }
                Err(SolveError::Math(MathError::PrecisionExhausted | MathError::Domain(_))) => {}
                Err(SolveError::Math(error)) => return Err(geo_math_error(&error, policy)),
                Err(SolveError::External(error)) => return Err(error),
                Err(SolveError::Iterations) => {
                    return Err(GeoError::ConvergenceExhausted {
                        iterations: policy.limits().max_iterations,
                    });
                }
            }
            if bits == maximum {
                return Err(GeoError::PrecisionExhausted { bits: maximum });
            }
            bits = super::next_precision(bits, maximum);
        }
    }
}

/// Canonically selected WGS84 inverse geodesic and certified metadata.
///
/// # Errors
/// Refuses operational exhaustion and unresolved exact branch/rounding decisions.
#[allow(clippy::needless_pass_by_value)] // Owned convenience surface; preparation borrows.
pub fn inverse(a: LonLat, b: LonLat) -> Result<InverseResult, GeoError> {
    let mut context = MetricContext::wgs84()?;
    PreparedGeodesic::new(context.reference().clone()).inverse(&a, &b, &mut context)
}

fn exact_proof(
    solution: &InverseSolution<FixedInterval>,
    bits: u32,
    math: &mut CoordinateMath,
) -> Result<InverseProofReceipt, MathError> {
    let metadata = solution
        .metadata
        .as_ref()
        .ok_or(MathError::Domain("inverse metadata capture missing"))?;
    let (lower, upper) = exact_bounds_in(&solution.distance, math)?;
    let (reduced_lower, reduced_upper) = exact_bounds_in(&metadata.raw.reduced, math)?;
    let (area_lower, area_upper) = exact_bounds_in(&metadata.raw.area, math)?;
    Ok(InverseProofReceipt {
        branch_multiplicity: metadata.raw.multiplicity,
        distance: MetricProofReceipt {
            lower: Metres::new(lower.max(Rat::zero())),
            upper: Metres::new(upper),
            precision_bits: bits,
            work_items: 0,
        },
        forward_azimuth: Some(exact_bounds_in(&metadata.raw.forward, math)?),
        final_azimuth: Some(exact_bounds_in(&metadata.raw.final_azimuth, math)?),
        arc_degrees: exact_bounds_in(&metadata.raw.arc, math)?,
        reduced_length: (Metres::new(reduced_lower), Metres::new(reduced_upper)),
        scale12: exact_bounds_in(&metadata.raw.scale12, math)?,
        scale21: exact_bounds_in(&metadata.raw.scale21, math)?,
        geodesic_quadrilateral_area: (SquareMetres::new(area_lower), SquareMetres::new(area_upper)),
    })
}

fn zero_result(distance: MetricEstimate, work: u64) -> (InverseResult, InverseProofReceipt) {
    let zero = Rat::zero();
    let one = Rat::one();
    (
        InverseResult {
            distance,
            forward_azimuth: None,
            final_azimuth: None,
            arc_degrees: zero.clone(),
            reduced_length: Metres::new(zero.clone()),
            scale12: one.clone(),
            scale21: one.clone(),
            geodesic_quadrilateral_area: SquareMetres::new(zero.clone()),
            pole_cuts: Vec::new(),
            multiplicity: ShortestBranchMultiplicity::Unique,
        },
        InverseProofReceipt {
            branch_multiplicity: ShortestBranchMultiplicity::Unique,
            distance: MetricProofReceipt {
                lower: Metres::new(zero.clone()),
                upper: Metres::new(zero.clone()),
                precision_bits: 0,
                work_items: work,
            },
            forward_azimuth: None,
            final_azimuth: None,
            arc_degrees: (zero.clone(), zero.clone()),
            reduced_length: (Metres::new(zero.clone()), Metres::new(zero.clone())),
            scale12: (one.clone(), one.clone()),
            scale21: (one.clone(), one),
            geodesic_quadrilateral_area: (SquareMetres::new(zero.clone()), SquareMetres::new(zero)),
        },
    )
}

// Scalar metadata quantization uses the shared exact endpoint half-even home.
fn quantize(
    value: &FixedInterval,
    places: u32,
    math: &mut CoordinateMath,
) -> Result<Rat, MathError> {
    let (lower, upper) = value.round_decimal(places, math)?;
    if lower != upper {
        return Err(MathError::PrecisionExhausted);
    }
    Ok(Rat::from_decimal(carrier_integer(&lower), places))
}

#[allow(clippy::too_many_arguments)] // Solver state, declared reference and numerical capabilities.
pub(super) fn candidate_solution<I: CertifiedInterval>(
    distance: I,
    candidate: &Hybrid<I>,
    pair: &CanonicalPair,
    reference: &GeographicReference,
    c: &I,
    ep2: &I,
    coefficients: &IntegralCoefficients<I>,
    capture: bool,
    goal: DistanceGoal<'_>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<Option<InverseSolution<I>>, SolveError> {
    if !capture {
        return Ok(Some(InverseSolution {
            distance,
            metadata: None,
        }));
    }
    match metadata(
        &candidate.arc,
        pair,
        reference,
        c,
        ep2,
        coefficients,
        chunks,
        math,
    )
    .and_then(|raw| {
        if !goal.enclosure_decisive(&raw.forward, math)? {
            return Err(MathError::PrecisionExhausted.into());
        }
        complete_metadata(raw, math.math()).map_err(SolveError::Math)
    }) {
        Ok(metadata) => Ok(Some(InverseSolution {
            distance,
            metadata: Some(metadata),
        })),
        Err(SolveError::Math(MathError::PrecisionExhausted | MathError::Domain(_))) => Ok(None),
        Err(error) => Err(error),
    }
}

#[allow(clippy::too_many_arguments)] // Special exact branch and the shared numerical capabilities.
pub(super) fn special_solution<I: CertifiedInterval>(
    distance: I,
    pair: &CanonicalPair,
    reference: &GeographicReference,
    sphere: &SpherePair<I>,
    c: &I,
    ep2: &I,
    coefficients: &IntegralCoefficients<I>,
    capture: bool,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<InverseSolution<I>, SolveError> {
    if !capture {
        return Ok(InverseSolution {
            distance,
            metadata: None,
        });
    }
    let pi = I::pi(math)?;
    let beta1 = I::atan2(&sphere.sin1, &sphere.cos1, math)?;
    let beta2 = I::atan2(&sphere.sin2, &sphere.cos2, math)?;
    let pole = pair.latitude1 == Rat::from_i64(-90);
    let southward = !pole && pair.longitude == Rat::from_i64(180);
    let sigma1 = if southward {
        pi.neg(math)?.sub(&beta1, math)?
    } else {
        beta1
    };
    let sigma2 = beta2;
    let azimuth1 = if pole {
        super::interval_from_rat::<I>(&pair.longitude, math)?
            .mul(&pi, math)?
            .div(&I::from_i64(180, math)?, math)?
    } else if southward {
        pi.clone()
    } else {
        I::from_i64(0, math)?
    };
    let jacobi = coefficients
        .evaluate_with(&sigma1, &sigma2, ep2, chunks, math)?
        .jacobi;
    let arc = AuxiliaryArc {
        azimuth1,
        azimuth2_y: I::from_i64(0, math)?,
        azimuth2_x: I::from_i64(1, math)?,
        sigma1,
        sigma2,
        sin0: I::from_i64(0, math)?,
        cos0_squared: I::from_i64(1, math)?,
        q: ep2.clone(),
        jacobi,
    };
    let mut raw = metadata(&arc, pair, reference, c, ep2, coefficients, chunks, math)?;
    let first = if pole {
        pair.longitude.clone()
    } else if southward {
        Rat::from_i64(180)
    } else {
        Rat::zero()
    };
    let (mut forward, mut final_azimuth) = restore_exact_angles(first, Rat::zero(), pair);
    if southward && pair.latitude1 == pair.latitude2.neg() {
        raw.multiplicity = ShortestBranchMultiplicity::Two;
        let alternate = restore_exact_angles(Rat::zero(), Rat::from_i64(180), pair);
        if alternate.0 < forward || (alternate.0 == forward && alternate.1 < final_azimuth) {
            (forward, final_azimuth) = alternate;
            raw.area = raw.area.neg(math.math())?;
            core::mem::swap(&mut raw.scale12, &mut raw.scale21);
        }
    }
    raw.forward = fixed_from_rat(&forward, math.math())?;
    raw.final_azimuth = fixed_from_rat(&final_azimuth, math.math())?;
    if pair.latitude1.abs() == Rat::from_i64(90) && pair.latitude2.abs() == Rat::from_i64(90) {
        raw.multiplicity = ShortestBranchMultiplicity::Continuum;
        // Opposite poles have a continuum of shortest branches. The frozen
        // longitude-zero frame selects azimuth zero at both endpoints.

        let fixed = math.math();
        raw.forward = FixedInterval::from_i64(0, fixed)?;
        raw.final_azimuth = FixedInterval::from_i64(0, fixed)?;
        let authalic = area::authalic_radius_squared(reference, fixed)?;
        raw.area = if pair.latitude_sign < 0 {
            authalic
                .mul(&FixedInterval::pi(fixed)?, fixed)?
                .mul(&FixedInterval::from_i64(-2, fixed)?, fixed)?
        } else {
            FixedInterval::from_i64(0, fixed)?
        };
    }
    Ok(InverseSolution {
        distance,
        metadata: Some(complete_metadata(raw, math.math())?),
    })
}

pub(super) fn equatorial_solution<I: CertifiedInterval>(
    distance: I,
    longitude: &I,
    pair: &CanonicalPair,
    reference: &GeographicReference,
    c: &I,
    capture: bool,
    math: &mut IntervalContext<'_>,
) -> Result<InverseSolution<I>, SolveError> {
    if !capture {
        return Ok(InverseSolution {
            distance,
            metadata: None,
        });
    }
    let sigma = longitude.div(c, math)?.to_fixed(math)?;
    let fixed = math.math();
    let degrees = FixedInterval::from_i64(180, fixed)?.div(&FixedInterval::pi(fixed)?, fixed)?;
    let ninety = FixedInterval::from_i64(90 * pair.longitude_sign, fixed)?;
    let b = fixed_from_rat(reference.ellipsoid().semiminor(), fixed)?;
    let (sin, cos) = sigma.sin_cos(fixed)?;
    let raw = RawMetadata {
        multiplicity: ShortestBranchMultiplicity::Unique,
        forward: ninety.clone(),
        final_azimuth: ninety,
        arc: sigma.mul(&degrees, fixed)?,
        reduced: sin.mul(&b, fixed)?,
        scale12: cos.clone(),
        scale21: cos,
        area: FixedInterval::from_i64(0, fixed)?,
    };
    let metadata = complete_metadata(raw, fixed)?;
    Ok(InverseSolution {
        distance,
        metadata: Some(metadata),
    })
}

#[allow(clippy::too_many_arguments)] // One branch state, exact symmetries and arithmetic capability.
fn metadata<I: CertifiedInterval>(
    arc: &AuxiliaryArc<I>,
    pair: &CanonicalPair,
    reference: &GeographicReference,
    c: &I,
    ep2: &I,
    coefficients: &IntegralCoefficients<I>,
    chunks: &mut ChunkObserver<'_>,
    math: &mut IntervalContext<'_>,
) -> Result<RawMetadata, SolveError> {
    let alpha2 = I::atan2(&arc.azimuth2_y, &arc.azimuth2_x, math)?.to_fixed(math)?;
    let alpha1 = arc.azimuth1.to_fixed(math)?;
    let sigma1 = arc.sigma1.to_fixed(math)?;
    let sigma2 = arc.sigma2.to_fixed(math)?;
    let sin0 = arc.sin0.to_fixed(math)?;
    let cos0_squared = arc.cos0_squared.to_fixed(math)?;
    let q = arc.q.to_fixed(math)?;
    let jacobi = arc.jacobi.to_fixed(math)?;
    let c = c.to_fixed(math)?;
    let ep2 = ep2.to_fixed(math)?;
    let fixed = math.math();
    let one = FixedInterval::from_i64(1, fixed)?;
    let pi = FixedInterval::pi(fixed)?;
    let degrees = FixedInterval::from_i64(180, fixed)?.div(&pi, fixed)?;
    // Integer grid shifts and sign reversals commute with half-even rounding.
    // Prove the three angular grids before generating the more expensive area
    // correction. No invocation-dependent proof width selects output precision.
    direct::quantize_angle(&alpha1.mul(&degrees, fixed)?, 0, fixed)?;
    direct::quantize_angle(&alpha2.mul(&degrees, fixed)?, 0, fixed)?;
    quantize(
        &sigma2.sub(&sigma1, fixed)?.mul(&degrees, fixed)?,
        15,
        fixed,
    )?;
    let (sin1, cos1) = sigma1.sin_cos(fixed)?;
    let (sin2, cos2) = sigma2.sin_cos(fixed)?;
    let h1 = one
        .add(&q.mul(&sin1.square(fixed)?, fixed)?, fixed)?
        .sqrt(fixed)?;
    let h2 = one
        .add(&q.mul(&sin2.square(fixed)?, fixed)?, fixed)?
        .sqrt(fixed)?;
    let reduced = h2
        .mul(&cos1, fixed)?
        .mul(&sin2, fixed)?
        .sub(&h1.mul(&sin1, fixed)?.mul(&cos2, fixed)?, fixed)?
        .sub(&cos1.mul(&cos2, fixed)?.mul(&jacobi, fixed)?, fixed)?
        .mul(
            &fixed_from_rat(reference.ellipsoid().semiminor(), fixed)?,
            fixed,
        )?;
    let common = cos1.mul(&cos2, fixed)?;
    let scale12 = common
        .add(
            &h2.div(&h1, fixed)?.mul(&sin1, fixed)?.mul(&sin2, fixed)?,
            fixed,
        )?
        .sub(
            &sin1
                .mul(&cos2, fixed)?
                .mul(&jacobi, fixed)?
                .div(&h1, fixed)?,
            fixed,
        )?;
    let scale21 = common
        .add(
            &h1.div(&h2, fixed)?.mul(&sin1, fixed)?.mul(&sin2, fixed)?,
            fixed,
        )?
        .add(
            &sin2
                .mul(&cos1, fixed)?
                .mul(&jacobi, fixed)?
                .div(&h2, fixed)?,
            fixed,
        )?;
    let area = area::quadrilateral(
        &alpha1,
        &alpha2,
        &sigma1,
        &sigma2,
        &sin0,
        &cos0_squared,
        &q,
        &c,
        &ep2,
        reference,
        coefficients.area.as_deref(),
        chunks,
        fixed,
    )?;
    let raw = RawMetadata {
        multiplicity: ShortestBranchMultiplicity::Unique,
        forward: alpha1,
        final_azimuth: alpha2,
        arc: sigma2.sub(&sigma1, fixed)?.mul(&degrees, fixed)?,
        reduced,
        scale12,
        scale21,
        area,
    };
    let mut selected = restore(raw.clone(), pair, &pi, &degrees, fixed)?;
    if pair.latitude1 == pair.latitude2.neg()
        && pair.latitude1.abs() != Rat::from_i64(90)
        && !pair.longitude.is_zero()
        && pair.longitude != Rat::from_i64(180)
    {
        let half_pi = pi.div(&FixedInterval::from_i64(2, fixed)?, fixed)?;
        if raw.forward.lower() > half_pi.upper() {
            selected.multiplicity = ShortestBranchMultiplicity::Two;
            let alternate = RawMetadata {
                multiplicity: ShortestBranchMultiplicity::Two,
                forward: raw.final_azimuth.clone(),
                final_azimuth: raw.forward.clone(),
                arc: raw.arc.clone(),
                reduced: raw.reduced.clone(),
                scale12: raw.scale21.clone(),
                scale21: raw.scale12.clone(),
                area: raw.area.neg(fixed)?,
            };
            let alternate = restore(alternate, pair, &pi, &degrees, fixed)?;
            if compare_angles(&alternate.forward, &selected.forward)? == core::cmp::Ordering::Less {
                selected = alternate;
            }
        } else if raw.forward.upper() >= half_pi.lower() {
            return Err(MathError::PrecisionExhausted.into());
        }
    }
    Ok(selected)
}

fn compare_angles(a: &FixedInterval, b: &FixedInterval) -> Result<core::cmp::Ordering, MathError> {
    let (al, au) = exact_bounds(a);
    let (bl, bu) = exact_bounds(b);
    let (al, au) = (direct::normalize(&al, 0), direct::normalize(&au, 0));
    let (bl, bu) = (direct::normalize(&bl, 0), direct::normalize(&bu, 0));
    if al > au || bl > bu {
        return Err(MathError::PrecisionExhausted);
    }
    if au < bl {
        Ok(core::cmp::Ordering::Less)
    } else if al > bu {
        Ok(core::cmp::Ordering::Greater)
    } else if al == au && bl == bu && al == bl {
        Ok(core::cmp::Ordering::Equal)
    } else {
        Err(MathError::PrecisionExhausted)
    }
}

fn restore_exact_angles(mut first: Rat, mut last: Rat, pair: &CanonicalPair) -> (Rat, Rat) {
    if pair.latitude_sign < 0 {
        first = Rat::from_i64(180).sub(&first);
        last = Rat::from_i64(180).sub(&last);
    }
    if pair.longitude_sign < 0 {
        first = first.neg();
        last = last.neg();
    }
    if pair.swapped {
        core::mem::swap(&mut first, &mut last);
        first = first.add(&Rat::from_i64(180));
        last = last.add(&Rat::from_i64(180));
    }
    (direct::normalize(&first, 0), direct::normalize(&last, 0))
}

fn restore(
    mut raw: RawMetadata,
    pair: &CanonicalPair,
    pi: &FixedInterval,
    degrees: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<RawMetadata, MathError> {
    for angle in [&mut raw.forward, &mut raw.final_azimuth] {
        if pair.latitude_sign < 0 {
            *angle = pi.sub(angle, math)?;
        }
        if pair.longitude_sign < 0 {
            *angle = angle.neg(math)?;
        }
    }
    if pair.swapped {
        core::mem::swap(&mut raw.forward, &mut raw.final_azimuth);
        raw.forward = raw.forward.add(pi, math)?;
        raw.final_azimuth = raw.final_azimuth.add(pi, math)?;
        core::mem::swap(&mut raw.scale12, &mut raw.scale21);
    }
    raw.forward = raw.forward.mul(degrees, math)?;
    raw.final_azimuth = raw.final_azimuth.mul(degrees, math)?;
    if pair.latitude_sign * pair.longitude_sign * if pair.swapped { -1 } else { 1 } < 0 {
        raw.area = raw.area.neg(math)?;
    }
    Ok(raw)
}

fn complete_metadata(raw: RawMetadata, math: &mut CoordinateMath) -> Result<Metadata, MathError> {
    Ok(Metadata {
        forward: direct::quantize_angle(&raw.forward, 0, math)?,
        final_azimuth: direct::quantize_angle(&raw.final_azimuth, 0, math)?,
        arc: quantize(&raw.arc, 15, math)?,
        reduced: quantize(&raw.reduced, 6, math)?,
        scale12: quantize(&raw.scale12, 15, math)?,
        scale21: quantize(&raw.scale21, 15, math)?,
        area: quantize(&raw.area, 2, math)?,
        raw,
    })
}

fn pole_cuts(a: &LonLat, b: &LonLat, forward: &Rat, final_azimuth: &Rat) -> Vec<(Rat, Rat, Rat)> {
    let mut cuts = Vec::new();
    if a.is_pole() {
        let meridian = if a.latitude() > &Rat::zero() {
            direct::normalize(&Rat::from_i64(180).sub(forward), -180)
        } else {
            direct::normalize(forward, -180)
        };
        if !meridian.is_zero() {
            cuts.push((a.latitude().clone(), Rat::zero(), meridian));
        }
    }
    if b.is_pole() {
        let meridian = if b.latitude() > &Rat::zero() {
            direct::normalize(&final_azimuth.neg(), -180)
        } else {
            direct::normalize(&final_azimuth.sub(&Rat::from_i64(180)), -180)
        };
        if !meridian.is_zero() {
            cuts.push((b.latitude().clone(), meridian, Rat::zero()));
        }
    }
    cuts
}

#[cfg(test)]
mod tests {
    use super::{ShortestBranchMultiplicity, inverse};
    use crate::{
        GeoError, GeographicReference, LonLat, Metres, MetricContext, MetricWorkObserver,
        PreparedGeodesic, Rat,
    };

    fn point(longitude: &str, latitude: &str) -> LonLat {
        LonLat::new(
            Rat::parse_decimal(longitude).unwrap(),
            Rat::parse_decimal(latitude).unwrap(),
        )
        .unwrap()
    }
    fn decimal(value: &str) -> Rat {
        Rat::parse_decimal(value).unwrap()
    }

    #[test]
    fn public_quadrilateral_jacobi_and_azimuth_fields_agree_independently() {
        // CC0: C. F. F. Karney, GeodTest, Zenodo record 32156. Each printed
        // field is finite precision; comparisons include its stated quantum.
        for (latitude1, latitude2, longitude2, azimuth1, azimuth2, arc, reduced, area) in [
            (
                "36.530042355041",
                "-48.164270779097768864",
                "5.762344694676510456",
                "176.125875162171",
                "175.334308316285410561",
                "84.663858149358862201",
                "6333544.7732452481809",
                "-559418252332.321555",
            ),
            (
                "62.912967770911",
                "-23.186512533703375483",
                "68.567247430960081525",
                "119.074519410562",
                "154.287114123871723086",
                "101.34948410954772013",
                "6243126.4334319566678",
                "24901551594881.448587",
            ),
            (
                "2.881248229541",
                "53.997072295385487038",
                "44.520619105667619923",
                "27.763592972746",
                "52.159486739947740473",
                "62.658669064582460176",
                "5654035.862840240277",
                "17238566899063.427934",
            ),
            (
                "15.543058059808",
                "-49.416672064705806337",
                "166.82808321068892367",
                "165.300914097354",
                "22.033030848248991914",
                "144.535997930981394845",
                "3730158.2076640481146",
                "-101426349516912.422958",
            ),
        ] {
            let result = inverse(point("0", latitude1), point(longitude2, latitude2))
                .expect("certified inverse metadata");
            assert_eq!(
                result.branch_multiplicity(),
                ShortestBranchMultiplicity::Unique
            );
            for (actual, expected) in [
                (result.forward_azimuth().unwrap(), azimuth1),
                (result.final_azimuth().unwrap(), azimuth2),
                (result.arc_degrees(), arc),
            ] {
                assert!(
                    actual.sub(&decimal(expected)).abs() <= decimal("0.000000000001"),
                    "angular field {expected}: {}",
                    actual.to_decimal_string(15)
                );
            }
            assert!(
                result.reduced_length().exact().sub(&decimal(reduced)).abs()
                    <= decimal("0.0000006")
            );
            assert!(
                result
                    .geodesic_quadrilateral_area()
                    .exact()
                    .sub(&decimal(area))
                    .abs()
                    <= decimal("0.02"),
                "quadrilateral {area}: {}",
                result
                    .geodesic_quadrilateral_area()
                    .exact()
                    .to_decimal_string(2)
            );
        }
    }

    #[test]
    fn exact_zero_equator_poles_and_antipodal_branch_laws() {
        let zero = inverse(point("180", "30"), point("-180", "30")).unwrap();
        assert!(zero.forward_azimuth().is_none());
        assert!(zero.final_azimuth().is_none());
        assert!(zero.arc_degrees().is_zero());
        assert!(zero.reduced_length().exact().is_zero());
        assert!(zero.geodesic_quadrilateral_area().exact().is_zero());
        assert_eq!(zero.scale12(), &Rat::one());
        assert_eq!(zero.scale21(), &Rat::one());
        let equator = inverse(point("0", "0"), point("1", "0")).unwrap();
        assert_eq!(equator.forward_azimuth(), Some(&Rat::from_i64(90)));
        assert_eq!(equator.final_azimuth(), Some(&Rat::from_i64(90)));
        assert!(equator.geodesic_quadrilateral_area().exact().is_zero());
        let antipode = inverse(point("0", "0"), point("180", "0")).unwrap();
        assert_eq!(
            antipode.branch_multiplicity(),
            ShortestBranchMultiplicity::Two
        );
        assert_eq!(antipode.forward_azimuth(), Some(&Rat::zero()));
        assert_eq!(antipode.final_azimuth(), Some(&Rat::from_i64(180)));
        assert_eq!(antipode.arc_degrees(), &Rat::from_i64(180));
        let north_south = inverse(point("41", "90"), point("-82", "-90")).unwrap();
        assert_eq!(
            north_south.branch_multiplicity(),
            ShortestBranchMultiplicity::Continuum
        );
        assert_eq!(north_south.forward_azimuth(), Some(&Rat::zero()));
        assert_eq!(north_south.final_azimuth(), Some(&Rat::zero()));
        assert!(north_south.geodesic_quadrilateral_area().exact() < &Rat::zero());
        assert_eq!(
            north_south.pole_cuts(),
            &[
                (Rat::from_i64(90), Rat::zero(), Rat::from_i64(-180)),
                (Rat::from_i64(-90), Rat::from_i64(-180), Rat::zero())
            ]
        );
        let south_north = inverse(point("-82", "-90"), point("41", "90")).unwrap();
        assert!(south_north.geodesic_quadrilateral_area().exact().is_zero());
        assert_eq!(south_north.pole_cuts(), []);
    }

    #[test]
    fn public_vertex_and_near_vertex_inverses_complete_under_default_admission() {
        // CC0 GeodTest rows100125,400001,450001,499999. The printed
        // distance uncertainty is included; no printed angle is treated as
        // the exact inverse of separately rounded endpoint coordinates.
        let mut context = MetricContext::wgs84().unwrap();
        let prepared =
            PreparedGeodesic::prepare(context.reference().clone(), &mut context).unwrap();
        for (latitude1, latitude2, longitude2, metres) in [
            (
                "4.19841765451",
                "-4.198416896099783242",
                "179.398024428225540172",
                "19970496.5132933",
            ),
            (
                "4.199535552987",
                "-4.199535552987",
                "179.398106343454992238",
                "19970505.608097404994",
            ),
            (
                "21.004101257892",
                "-21.004101257877442853",
                "179.43641220015808594",
                "19974623.3063849",
            ),
            (
                "32.716460827649",
                "-32.716460827645147496",
                "179.491863901399494732",
                "19980105.2410422",
            ),
        ] {
            let (answer, proof) = prepared
                .inverse_with_proof(
                    &point("0", latitude1),
                    &point(longitude2, latitude2),
                    &mut context,
                )
                .expect("complete certified default inverse at a conjugate neighbor");
            assert!(
                answer
                    .distance()
                    .value()
                    .exact()
                    .sub(&decimal(metres))
                    .abs()
                    <= decimal("0.0000006")
            );
            assert!(
                proof
                    .distance
                    .upper
                    .exact()
                    .sub(proof.distance.lower.exact())
                    <= decimal("0.0000005")
            );
            assert!(answer.forward_azimuth().is_some());
            assert!(answer.final_azimuth().is_some());
        }
    }

    #[test]
    fn near_conjugate_opposite_latitude_neighbor_retains_the_exact_shortest_law() {
        let reference = GeographicReference::wgs84();
        let mut context =
            MetricContext::new(reference.clone(), crate::ExecutionPolicy::geometry()).unwrap();
        let prepared = PreparedGeodesic::prepare(reference, &mut context).unwrap();
        let answer = prepared
            .inverse(
                &point("0", "54.709681492824"),
                &point("179.650757612091536687", "-54.709681492824217636"),
                &mut context,
            )
            .unwrap();
        assert!(
            answer
                .distance()
                .value()
                .exact()
                .sub(&decimal("19992674.6040408"))
                .abs()
                <= decimal("0.0000006")
        );
    }

    #[test]
    fn tiny_asymmetry_at_conjugate_vertices_completes_with_default_admission() {
        // CC0 GeodTest rows457455,470839,474828,481146,492903. The printed
        // distance is a finite corpus witness, not infinitely precise truth.
        let reference = GeographicReference::wgs84();
        let mut context = MetricContext::wgs84().unwrap();
        let prepared = PreparedGeodesic::prepare(reference, &mut context).unwrap();
        for (first, last, longitude, distance) in [
            (
                "7.326494029601",
                "-7.326494029600942956",
                "179.401396884582420845",
                "19970869.9355725",
            ),
            (
                "0.118891154294",
                "-0.118891154293994723",
                "179.396495370294585451",
                "19970326.5147189",
            ),
            (
                "0.223156798835",
                "-0.223156798834922388",
                "179.396498612331853533",
                "19970326.8756191",
            ),
            (
                "70.262058960871",
                "-70.262058960870992417",
                "179.795729562201047750",
                "20000080.0966750",
            ),
            (
                "0.047259856813",
                "-0.047259856813079878",
                "179.396494263473585099",
                "19970326.3915083",
            ),
        ] {
            let first = point("0", first);
            let last = point(longitude, last);
            let inverse = prepared.inverse(&first, &last, &mut context).unwrap();
            let point_distance = prepared.distance(&first, &last, &mut context).unwrap();
            assert_eq!(inverse.distance(), &point_distance);
            assert!(
                point_distance.value().exact().sub(&decimal(distance)).abs()
                    <= decimal("0.0000006")
            );
        }
    }

    struct Refuse;
    impl MetricWorkObserver for Refuse {
        fn charge_chunk(&mut self, _work: u64, _workspace: u64) -> Result<(), GeoError> {
            Err(GeoError::Cancelled)
        }
    }

    #[test]
    fn metered_direct_and_inverse_batches_clear_every_slot_on_refusal() {
        let reference = GeographicReference::wgs84();
        let prepared = PreparedGeodesic::new(reference);
        let mut context = MetricContext::wgs84().unwrap();
        let a = point("0", "0");
        let b = point("1", "1");
        let completed = prepared.inverse(&a, &b, &mut context).unwrap();
        let mut output = [Some(completed.clone()), Some(completed)];
        let inputs = [(a.clone(), b.clone()), (b.clone(), a.clone())];
        assert_eq!(
            prepared.inverse_batch_metered(&inputs, &mut output, &mut context, &mut Refuse),
            Err(GeoError::Cancelled)
        );
        assert!(output.iter().all(Option::is_none));
        let length = Metres::new(Rat::from_i64(1000));
        let azimuth = Rat::from_i64(30);
        let direct = prepared
            .direct(&a, &azimuth, &length, &mut context)
            .unwrap();
        let mut output = [Some(direct.clone()), Some(direct)];
        let inputs = [
            (a.clone(), azimuth.clone(), length.clone()),
            (b, azimuth, length),
        ];
        assert_eq!(
            prepared.direct_batch_metered(&inputs, &mut output, &mut context, &mut Refuse),
            Err(GeoError::Cancelled)
        );
        assert!(output.iter().all(Option::is_none));
        assert_eq!(
            prepared.direct_metered(
                &a,
                &Rat::zero(),
                &Metres::new(Rat::one()),
                &mut context,
                &mut Refuse
            ),
            Err(GeoError::Cancelled)
        );
    }
}
