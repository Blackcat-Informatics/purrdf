// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Unrounded selected-line enclosures for certified arrangements and materialization.

use super::line::LineState;
use super::{
    ChunkObserver, InverseProofReceipt, PreparedGeodesic, ShortestBranchMultiplicity, SolveError,
};
use crate::context::WorkProgress;
use crate::numerical::{carrier_integer, fixed_from_bounds, fixed_from_rat, geo_math_error};
use crate::{
    GeoError, LonLat, Metres, MetricContext, MetricWorkObserver, Rat, ShortestGeodesicArc,
};
use purrdf_xsd::{
    BigInt,
    math::{CoordinateMath, FixedInterval, MathError, TaylorJet, TaylorWorkspace},
};

/// Source-defined selected geodesic with unrounded branch proof.
#[derive(Clone, Debug)]
pub struct ArcIntervalView {
    prepared: PreparedGeodesic,
    start: LonLat,
    end: Option<LonLat>,
    azimuth: (Rat, Rat),
    exact_azimuth: Option<Rat>,
    length: (Metres, Metres),
    proof_bits: u32,
}

/// Indexed meridian-pole events: sigma=(k+1/2)*pi for every admitted k.
/// The end indices conservatively include events touching enclosure endpoints.
#[derive(Clone, Debug)]
pub struct PoleEventRange {
    /// First potentially intersecting pole index, inclusive.
    pub first: BigInt,
    /// Last potentially intersecting pole index, inclusive.
    pub last: BigInt,
    /// Certified pi at the same fixed-point precision.
    pub pi: FixedInterval,
}

/// Complete parameter image and chart derivative enclosures, in radians.
#[derive(Clone, Debug)]
pub struct ArcIntervalEnclosure {
    /// Complete continuous longitude lift, in radians.
    pub longitude: FixedInterval,
    /// Complete geodetic latitude image, in radians.
    pub latitude: FixedInterval,
    /// Absent only across explicit meridian-pole chart events.
    pub derivative: Option<[FixedInterval; 2]>,
    /// Auxiliary-sphere phase of the exact source point.
    pub sigma_start: FixedInterval,
    /// Complete auxiliary-sphere phase image.
    pub sigma: FixedInterval,
    /// Auxiliary phase derivative with respect to the exact source parameter.
    pub sigma_derivative: FixedInterval,
    /// Conserved sine of the equatorial azimuth.
    pub sin_alpha0: FixedInterval,
    /// Nonnegative cosine of the equatorial azimuth.
    pub cos_alpha0: FixedInterval,
    /// Explicit pole-index enclosure for a meridian chart crossing.
    pub meridian_poles: Option<PoleEventRange>,
    /// Complete unit geodetic-normal image, smooth through coordinate poles.
    pub normal: [FixedInterval; 3],
    /// Unit geodetic-normal derivative with respect to the source parameter.
    pub normal_derivative: [FixedInterval; 3],
}

pub(super) struct ChartImage {
    pub(super) longitude: FixedInterval,
    pub(super) latitude: FixedInterval,
    pub(super) exact_longitude_degrees: Option<Rat>,
    derivative: Option<[FixedInterval; 2]>,
    sigma_derivative: FixedInterval,
    meridian_poles: Option<PoleEventRange>,
    correction: FixedInterval,
    sine: FixedInterval,
    cosine: FixedInterval,
    vertical: FixedInterval,
    horizontal: FixedInterval,
    h: FixedInterval,
}

/// Worker precision-specific line scratch; no floating guard or callback is retained.
#[derive(Clone, Debug)]
pub(crate) struct FixedArcLine {
    binding_id: crate::GeoBindingId,
    start: LonLat,
    end: Option<LonLat>,
    azimuth: FixedInterval,
    exact_azimuth: Option<Rat>,
    length: FixedInterval,
    line: LineState,
}

// The original meridian-plane lift. Pole longitude is immaterial: the
// validated pole azimuth frame supplies its meridian; ordinary sources keep
// their exact written longitude. No periodic reduction is applied here.
fn exact_meridian_base(start: &LonLat, azimuth: Option<&Rat>) -> Option<Rat> {
    if start.is_pole() {
        azimuth.map(|angle| {
            if start.latitude().signum() < 0 {
                angle.clone()
            } else {
                Rat::from_i64(180).sub(angle)
            }
        })
    } else {
        Some(start.longitude().clone())
    }
}

/// Retain the original exact meridian-plane lift of an azimuth-defined line.
/// A nonpolar source is a meridian exactly for normalized azimuth 0 or 180;
/// every pole azimuth has the original pole-frame meridian. `None` proves the
/// nonpolar line is not a meridian. This does not classify its travel length.
/// The caller drops the returned owner before releasing the added allowance.
pub(crate) fn meridian_plane_degrees_admitted(
    start: &LonLat,
    azimuth: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    retained: &mut u64,
) -> Result<Option<Rat>, GeoError> {
    use crate::numerical::ExactAdmission;
    use purrdf_xsd::integer::ExactOperation as Op;
    let mut admission = ExactAdmission::new(context, progress);
    let turn = Rat::from_i64(360);
    // Four original division/product/copy allowances cover integer-period
    // reduction and its live normalized result. Arithmetic stays in Rat's
    // original modulo worker, shared with the direct geodesic path.
    let angle = admission.rational_owner_counted(
        Op::RationalDivide,
        &[azimuth, &turn],
        4,
        retained,
        || super::direct::normalize(azimuth, 0),
    )?;
    let pole = admission.rational(Op::Linear, &[start.latitude()], 1, || Ok(start.is_pole()))?;
    let half = Rat::from_i64(180);
    if !pole
        && !admission.compare(&angle, &Rat::zero())?.is_eq()
        && !admission.compare(&angle, &half)?.is_eq()
    {
        return Ok(None);
    }
    admission.rational_owner(
        Op::RationalAdd,
        &[start.longitude(), start.latitude(), &angle, &half],
        retained,
        || exact_meridian_base(start, Some(&angle)),
    )
}

fn rational_limb_bytes(value: &Rat) -> u64 {
    value.allocated_bytes() as u64
}

fn shortest_view_workspace_bound(
    prepared: &PreparedGeodesic,
    start: &LonLat,
    end: &LonLat,
    proof: &InverseProofReceipt,
) -> u64 {
    let mut bytes = size_of::<ArcIntervalView>() as u64
        + prepared.retained_workspace_bytes()
        + prepared.reference.ellipsoid().retained_limb_bytes();
    for value in [
        start.longitude(),
        start.latitude(),
        end.longitude(),
        end.latitude(),
        proof.distance.lower.exact(),
        proof.distance.upper.exact(),
    ] {
        bytes = bytes.saturating_add(rational_limb_bytes(value));
    }
    if let Some((lower, upper)) = &proof.forward_azimuth {
        bytes = bytes
            .saturating_add(rational_limb_bytes(lower))
            .saturating_add(rational_limb_bytes(upper));
        if lower == upper {
            bytes = bytes.saturating_add(rational_limb_bytes(lower));
        }
    }
    bytes
}

impl PreparedGeodesic {
    /// Prepare an exact azimuth/length source law for interval evaluation.
    ///
    /// # Errors
    /// Refuses a negative travel distance.
    pub fn prepare_azimuth_arc_view(
        &self,
        start: &LonLat,
        azimuth: &Rat,
        length: &Metres,
    ) -> Result<ArcIntervalView, GeoError> {
        if length.exact() < &Rat::zero() {
            return Err(GeoError::domain("arc travel distance must be nonnegative"));
        }
        let angle = super::direct::normalize(azimuth, 0);
        Ok(ArcIntervalView {
            prepared: self.clone(),
            start: start.clone(),
            end: None,
            azimuth: (angle.clone(), angle.clone()),
            exact_azimuth: Some(angle),
            length: (length.clone(), length.clone()),
            proof_bits: 0,
        })
    }

    /// Prepare the unique shortest endpoint-defined law at a minimum arithmetic
    /// precision. The retained unrounded enclosure remains explicit; use
    /// [`ArcIntervalView::refined`] to request tighter source-parameter proofs.
    /// Completed inverse grids are never inputs to this view.
    ///
    /// # Errors
    /// Refuses ambiguous shortest branches and numerical/resource exhaustion.
    pub fn prepare_shortest_arc_view(
        &self,
        start: &LonLat,
        end: &LonLat,
        minimum_bits: u32,
        context: &mut MetricContext,
        mut observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<ArcIntervalView, GeoError> {
        context.begin(1)?;
        if minimum_bits > context.policy().limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted {
                bits: context.policy().limits().max_precision_bits,
            });
        }
        let (_, proof) = match observer.as_mut() {
            Some(observer) => {
                self.compute_inverse_precision(start, end, context, Some(*observer), minimum_bits)?
            }
            None => self.compute_inverse_precision(start, end, context, None, minimum_bits)?,
        };
        let mut continuation = observer.map(|observer| {
            crate::MetricWorkContinuation::new(
                observer,
                context.work_items(),
                context.workspace_peak(),
            )
        });
        let mut progress = WorkProgress::new(
            continuation
                .as_mut()
                .map(|observer| observer as &mut dyn MetricWorkObserver),
        );
        self.shortest_view_owned_from_proof(start, end, &proof, context, &mut progress)
    }

    /// Reuse only a proof owned by the privately validated endpoint-arc type.
    /// Public mutable receipt records cannot enter this trusted preparation seam.
    pub(crate) fn prepare_shortest_arc_view_with_proof(
        &self,
        arc: &ShortestGeodesicArc,
        minimum_bits: u32,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<ArcIntervalView, GeoError> {
        if minimum_bits > context.policy().limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted {
                bits: context.policy().limits().max_precision_bits,
            });
        }
        context.begin(1)?;
        context.charge_work(1)?;
        let mut progress = WorkProgress::new(observer);
        let binding =
            crate::numerical::reference_identity(context, &mut progress, Some(&self.reference))?;
        if arc.start().binding_id() != binding
            || arc.end().binding_id() != binding
            || arc.inverse().binding_id() != binding
            || !crate::numerical::reference_matches(&self.reference, context, &mut progress)?
        {
            return Err(GeoError::config(
                "cached arc and numerical reference differ",
            ));
        }
        let start = arc.start().point();
        let end = arc.end().point();
        let proof = arc.proof();
        if proof.distance.precision_bits < minimum_bits && !start.same_location(end) {
            let mut child = context.remaining_child()?;
            let result = {
                let mut nested = progress.nested(
                    context.work_items(),
                    context.current_workspace_bytes(),
                    context.workspace_peak(),
                );
                self.prepare_shortest_arc_view(
                    start,
                    end,
                    minimum_bits,
                    &mut child,
                    Some(&mut nested),
                )
            };
            return progress.absorb_child_result(context, &child, result);
        }
        self.shortest_view_owned_from_proof(start, end, proof, context, &mut progress)
    }

    fn shortest_view_owned_from_proof(
        &self,
        start: &LonLat,
        end: &LonLat,
        proof: &InverseProofReceipt,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<ArcIntervalView, GeoError> {
        let values = [
            start.longitude(),
            start.latitude(),
            end.longitude(),
            end.latitude(),
            proof.distance.lower.exact(),
            proof.distance.upper.exact(),
        ];
        let azimuth = proof
            .forward_azimuth
            .iter()
            .flat_map(|(lower, upper)| [lower, upper]);
        let admission =
            crate::metric::ReceiptOwnership::for_operands(values.into_iter().chain(azimuth))?;
        let retained = shortest_view_workspace_bound(self, start, end, proof);
        context.admit_workspace(retained)?;
        let result = admission.prepare(context, progress, || {
            let mut view = self.shortest_view_from_proof(start, end, proof)?;
            view.azimuth = (view.azimuth.0.into_shared(), view.azimuth.1.into_shared());
            view.exact_azimuth = view.exact_azimuth.map(Rat::into_shared);
            view.length = (
                Metres::new(view.length.0.into_exact().into_shared()),
                Metres::new(view.length.1.into_exact().into_shared()),
            );
            Ok(view)
        });
        context.release_workspace(retained)?;
        result
    }

    fn shortest_view_from_proof(
        &self,
        start: &LonLat,
        end: &LonLat,
        proof: &InverseProofReceipt,
    ) -> Result<ArcIntervalView, GeoError> {
        if proof.branch_multiplicity != ShortestBranchMultiplicity::Unique {
            return Err(GeoError::AmbiguousGeodesic);
        }
        let azimuth = proof
            .forward_azimuth
            .clone()
            .unwrap_or_else(|| (Rat::zero(), Rat::zero()));
        let exact_azimuth = (azimuth.0 == azimuth.1).then(|| azimuth.0.clone());
        Ok(ArcIntervalView {
            prepared: self.clone(),
            start: start.clone(),
            end: Some(end.clone()),
            azimuth,
            exact_azimuth,
            length: (proof.distance.lower.clone(), proof.distance.upper.clone()),
            proof_bits: proof.distance.precision_bits,
        })
    }
}

impl ArcIntervalView {
    /// Actual retained object/limb bytes, including shared numerical preparation.
    /// Callers that retain several views may account one shared preparation once.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> u64 {
        let mut bytes = size_of::<Self>() as u64 + self.prepared.retained_workspace_bytes();
        for value in [
            self.start.longitude(),
            self.start.latitude(),
            &self.azimuth.0,
            &self.azimuth.1,
            self.length.0.exact(),
            self.length.1.exact(),
        ] {
            bytes = bytes.saturating_add(rational_limb_bytes(value));
        }
        if let Some(end) = &self.end {
            bytes = bytes
                .saturating_add(rational_limb_bytes(end.longitude()))
                .saturating_add(rational_limb_bytes(end.latitude()));
        }
        if let Some(angle) = &self.exact_azimuth {
            bytes = bytes.saturating_add(rational_limb_bytes(angle));
        }
        bytes.saturating_add(self.prepared.reference.ellipsoid().retained_limb_bytes())
    }
    /// Original validated source point, retaining its longitude spelling.
    #[must_use]
    pub const fn start(&self) -> &LonLat {
        &self.start
    }

    /// Inclusive unrounded source travel-length bounds from this view's proof.
    pub(crate) fn length_bounds(&self) -> (&Metres, &Metres) {
        (&self.length.0, &self.length.1)
    }

    /// Whether every parameter denotes the exact source point.
    #[must_use]
    pub fn is_constant(&self) -> bool {
        self.length.0.exact().is_zero() && self.length.1.exact().is_zero()
    }

    /// Original endpoint for the implicit shortest law.
    #[must_use]
    pub const fn end(&self) -> Option<&LonLat> {
        self.end.as_ref()
    }

    /// Refine a shortest branch from its original endpoints. The retained distance
    /// in metres and forward azimuth in degrees each have width at most
    /// `2^(-bits/2)`, independently of the fixed public inverse grids. Exact
    /// azimuth/length laws retain exact parameters and are shared unchanged.
    ///
    /// # Errors
    /// Refuses an uncertifiable branch, numerical exhaustion or reference mismatch.
    pub fn refined(
        &self,
        bits: u32,
        context: &mut MetricContext,
        mut observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        context.begin(1)?;
        if let Some(end) = &self.end {
            let (_, proof) = match observer.as_mut() {
                Some(observer) => self.prepared.compute_arc_inverse_precision(
                    &self.start,
                    end,
                    context,
                    Some(*observer),
                    bits,
                )?,
                None => self.prepared.compute_arc_inverse_precision(
                    &self.start,
                    end,
                    context,
                    None,
                    bits,
                )?,
            };
            let mut continuation = observer.map(|observer| {
                crate::MetricWorkContinuation::new(
                    observer,
                    context.work_items(),
                    context.workspace_peak(),
                )
            });
            let mut progress = WorkProgress::new(
                continuation
                    .as_mut()
                    .map(|observer| observer as &mut dyn MetricWorkObserver),
            );
            self.prepared.shortest_view_owned_from_proof(
                &self.start,
                end,
                &proof,
                context,
                &mut progress,
            )
        } else {
            context.charge_work(1)?;
            let mut progress = WorkProgress::new(observer);
            progress.initial()?;
            progress.context_poll(context)?;
            if !crate::numerical::reference_matches(
                &self.prepared.reference,
                context,
                &mut progress,
            )? {
                return Err(GeoError::config("arc and context references differ"));
            }
            Ok(self.clone())
        }
    }

    /// Precision of the retained shortest-branch proof; zero for exact azimuth/length.
    #[must_use]
    pub const fn proof_bits(&self) -> u32 {
        self.proof_bits
    }

    /// Admit and prepare the one direct line in an enclosing numerical phase.
    pub(crate) fn prepare_in(
        &self,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<FixedArcLine, MathError> {
        let binding_id =
            crate::numerical::math_reference_identity(&self.prepared.reference, math, progress)?;
        let azimuth = fixed_from_bounds(&self.azimuth.0, &self.azimuth.1, math)?;
        let length = fixed_from_bounds(self.length.0.exact(), self.length.1.exact(), math)?;
        let order = super::integral_order(
            self.prepared.reference.ellipsoid(),
            math.limits().precision_bits,
        );
        let coefficients = super::prepare_integral_coefficients_observed(
            self.prepared.reference.ellipsoid(),
            math,
            progress,
        )?;
        let mut poll = |math: &CoordinateMath| progress.math_poll(math);
        let mut chunks = ChunkObserver::borrowed(&mut poll);
        let line = LineState::new_with_coefficients(
            &self.start,
            &azimuth,
            self.exact_azimuth.as_ref(),
            &self.prepared,
            Some(&coefficients),
            order,
            &mut chunks,
            math,
        )
        .map_err(|error| math_error(error, 0))?;
        chunks.finish(math).map_err(|error| math_error(error, 0))?;
        Ok(FixedArcLine {
            binding_id,
            start: self.start.clone(),
            end: self.end.clone(),
            azimuth,
            exact_azimuth: self.exact_azimuth.clone(),
            length,
            line,
        })
    }

    /// Enclose a complete closed parameter interval on this source-defined line.
    /// Longitude is a continuous lift in radians; meridian pole events are explicit.
    ///
    /// # Errors
    /// Refuses invalid parameters, unresolved singularities and numerical/resource exhaustion.
    pub fn parameter_enclosure(
        &self,
        lower: &Rat,
        upper: &Rat,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<ArcIntervalEnclosure, GeoError> {
        context.begin(1)?;
        context.charge_work(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.initial()?;
        progress.context_poll(context)?;
        if !crate::numerical::reference_matches(&self.prepared.reference, context, &mut progress)? {
            return Err(GeoError::config("arc and context references differ"));
        }
        let mut admission = crate::numerical::ExactAdmission::new(context, &mut progress);
        if admission.compare(lower, &Rat::zero())?.is_lt()
            || admission.compare(upper, &Rat::one())?.is_gt()
            || admission.compare(lower, upper)?.is_gt()
        {
            return Err(GeoError::domain("arc parameter interval must be in [0,1]"));
        }
        let policy = context.policy();
        let bits = policy
            .limits()
            .max_precision_bits
            .min(self.proof_bits.max(80));
        let reservation = (bits.div_ceil(8) as usize + 64).saturating_mul(256);
        let sources = [
            lower,
            upper,
            self.start.longitude(),
            self.start.latitude(),
            &self.azimuth.0,
            &self.azimuth.1,
            self.length.0.exact(),
            self.length.1.exact(),
        ];
        let source_bits = crate::numerical::scratch_source_bits_in(
            sources
                .into_iter()
                .chain(
                    self.end
                        .iter()
                        .flat_map(|end| [end.longitude(), end.latitude()]),
                )
                .chain(self.exact_azimuth.iter()),
            self.prepared.reference.ellipsoid(),
        );
        context.prepare_integer_scratch_for_observed(source_bits, &mut progress)?;
        crate::numerical::with_math_for_sources(
            context,
            &mut progress,
            bits,
            reservation,
            &sources,
            |math, progress| {
                (|| {
                    let line = self.prepare_in(math, progress)?;
                    if line.workspace_bytes() > reservation {
                        return Err(MathError::WorkspaceExhausted);
                    }
                    let parameters = fixed_from_bounds(lower, upper, math)?;
                    line.parameter_enclosure_in(
                        &parameters,
                        policy.limits().max_iterations,
                        math,
                        progress,
                    )
                })()
                .map_err(|error| geo_math_error(&error, policy))
            },
        )
    }
}

impl FixedArcLine {
    /// Unrounded source travel length at this worker's admitted arithmetic grid.
    pub(crate) const fn travel_length(&self) -> &FixedInterval {
        &self.length
    }

    /// The conserved equatorial azimuth proves a meridian, including pole events.
    pub(crate) fn is_meridian(&self) -> bool {
        self.line.sin0.is_exact_zero()
    }

    /// The conserved equatorial azimuth proves an exact equatorial line.
    pub(crate) fn is_equatorial(&self) -> bool {
        self.line.cos0.is_exact_zero()
    }

    pub(super) fn from_state(
        binding_id: crate::GeoBindingId,
        start: LonLat,
        azimuth: FixedInterval,
        exact_azimuth: Option<Rat>,
        length: FixedInterval,
        line: LineState,
    ) -> Self {
        Self {
            binding_id,
            start,
            end: None,
            azimuth,
            exact_azimuth,
            length,
            line,
        }
    }
    pub(super) const fn state(&self) -> &LineState {
        &self.line
    }

    /// Retained scratch accounting, including every allocated endpoint limb.
    pub(crate) fn workspace_bytes(&self) -> usize {
        let mut bytes = size_of::<Self>();
        for value in [
            &self.azimuth,
            &self.length,
            &self.line.a,
            &self.line.b,
            &self.line.c,
            &self.line.e2,
            &self.line.q,
            &self.line.radians_per_degree,
            &self.line.sin0,
            &self.line.cos0,
            &self.line.sigma1,
            &self.line.coefficients.c,
        ] {
            bytes = bytes.saturating_add(value.workspace_bytes() - size_of::<FixedInterval>());
        }
        if let Some(omega) = &self.line.omega_start {
            bytes = bytes.saturating_add(omega.workspace_bytes() - size_of::<FixedInterval>());
        }
        for coefficients in [
            &self.line.coefficients.square_root,
            &self.line.coefficients.reciprocal,
        ] {
            if coefficients.spilled() {
                bytes = bytes.saturating_add(
                    coefficients
                        .capacity()
                        .saturating_mul(size_of::<FixedInterval>()),
                );
            }
        }
        for value in self
            .line
            .coefficients
            .square_root
            .iter()
            .chain(self.line.coefficients.reciprocal.iter())
        {
            bytes = bytes.saturating_add(value.workspace_bytes() - size_of::<FixedInterval>());
        }
        for value in [self.start.longitude(), self.start.latitude()] {
            bytes = bytes.saturating_add(value.allocated_bytes());
        }
        if let Some(end) = &self.end {
            for value in [end.longitude(), end.latitude()] {
                bytes = bytes.saturating_add(value.allocated_bytes());
            }
        }
        if let Some(angle) = &self.exact_azimuth {
            bytes = bytes.saturating_add(angle.allocated_bytes());
        }
        bytes
    }

    /// Evaluate the complete source-defined parameter image in borrowed scratch.
    /// The caller admits retained line/image workspace before keeping these values.
    pub(crate) fn parameter_enclosure_in(
        &self,
        parameter: &FixedInterval,
        iterations: u32,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<ArcIntervalEnclosure, MathError> {
        let zero = FixedInterval::from_i64(0, math)?;
        let one = FixedInterval::from_i64(1, math)?;
        if parameter.lower() < zero.lower() || parameter.upper() > one.upper() {
            return Err(MathError::Domain("arc parameter outside [0,1]"));
        }
        self.continuation_enclosure_in(parameter, iterations, math, progress)
    }

    /// Certified continuation around closed source endpoints. This internal
    /// capability shares all source law and equations; no boundary epsilon is used.
    pub(crate) fn continuation_enclosure_in(
        &self,
        parameter: &FixedInterval,
        iterations: u32,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<ArcIntervalEnclosure, MathError> {
        let distance = self.length.mul(parameter, math)?;
        let mut poll = |math: &CoordinateMath| progress.math_poll(math);
        let mut chunks = ChunkObserver::borrowed(&mut poll);
        let sigma = self
            .line
            .sigma_enclosure(&distance, iterations, &mut chunks, math)
            .map_err(|error| math_error(error, iterations))?;
        let chart = self.image(&sigma, true, &mut chunks, math)?;
        let (normal, normal_derivative) = self.normal_image(&chart, math)?;
        chunks.finish(math).map_err(|error| math_error(error, 0))?;
        Ok(ArcIntervalEnclosure {
            longitude: chart.longitude,
            latitude: chart.latitude,
            derivative: chart.derivative,
            sigma_start: self.line.sigma1.clone(),
            sigma,
            sigma_derivative: chart.sigma_derivative,
            sin_alpha0: self.line.sin0.clone(),
            cos_alpha0: self.line.cos0.clone(),
            meridian_poles: chart.meridian_poles,
            normal,
            normal_derivative,
        })
    }

    pub(super) fn image(
        &self,
        sigma: &FixedInterval,
        derivatives: bool,
        chunks: &mut ChunkObserver<'_>,
        math: &mut CoordinateMath,
    ) -> Result<ChartImage, MathError> {
        let line = &self.line;
        let (sin, cos) = sigma.sin_cos_range(math)?;
        let vertical = line.cos0.mul(&sin, math)?;
        let horizontal = line.cos0.mul(&cos, math)?;
        let cos_beta_squared = horizontal
            .square(math)?
            .add(&line.sin0.square(math)?, math)?;
        let cos_beta = cos_beta_squared.sqrt(math)?;
        let latitude = if cos_beta.lower().is_zero()
            && vertical.lower() <= &BigInt::from_i128(0)
            && vertical.upper() >= &BigInt::from_i128(0)
        {
            reduced_sine_latitude(&vertical, &line.c, math)?
        } else {
            FixedInterval::atan2(&vertical, &cos_beta.mul(&line.c, math)?, math)?
        };
        let meridian = line.sin0.lower().is_zero() && line.sin0.upper().is_zero();
        let correction = self.longitude_correction(sigma, chunks, math)?;
        let (longitude, poles, exact_longitude_degrees) = if meridian {
            self.meridian_longitude(&cos, sigma, math)?
        } else {
            let omega1 = line
                .omega_start
                .as_ref()
                .ok_or(MathError::PrecisionExhausted)?;
            let omega2 = omega_lift(sigma, &line.sin0, math)?;
            (
                fixed_from_rat(self.start.longitude(), math)?
                    .mul(&line.radians_per_degree, math)?
                    .add(&omega2.sub(omega1, math)?.sub(&correction, math)?, math)?,
                None,
                None,
            )
        };
        let h = FixedInterval::from_i64(1, math)?
            .add(&line.q.mul(&sin.square(math)?, math)?, math)?
            .sqrt(math)?;
        let dsigma = self.length.div(&line.b.mul(&h, math)?, math)?;
        let derivative = if poles.is_some() || !derivatives {
            None
        } else {
            let dlatitude = line
                .c
                .mul(&horizontal, math)?
                .div(
                    &cos_beta.mul(
                        &line
                            .c
                            .square(math)?
                            .mul(&cos_beta_squared, math)?
                            .add(&vertical.square(math)?, math)?,
                        math,
                    )?,
                    math,
                )?
                .mul(&dsigma, math)?;
            let dlongitude = if meridian {
                FixedInterval::from_i64(0, math)?
            } else {
                line.sin0
                    .mul(&self.length, math)?
                    .div(&line.a.mul(&cos_beta_squared, math)?, math)?
            };
            Some([dlongitude, dlatitude])
        };
        Ok(ChartImage {
            longitude,
            latitude,
            exact_longitude_degrees,
            derivative,
            sigma_derivative: dsigma,
            meridian_poles: poles,
            correction,
            sine: sin,
            cosine: cos,
            vertical,
            horizontal,
            h,
        })
    }

    fn phase_start(&self, math: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
        let line = &self.line;
        if let Some(omega) = &line.omega_start {
            fixed_from_rat(self.start.longitude(), math)?
                .mul(&line.radians_per_degree, math)?
                .sub(omega, math)
        } else if self.start.is_pole() {
            let angle = self.azimuth.mul(&line.radians_per_degree, math)?;
            if self.start.latitude() < &Rat::zero() {
                Ok(angle)
            } else {
                angle.neg(math)
            }
        } else {
            let (_, cos) = line.sigma1.sin_cos(math)?;
            let longitude = fixed_from_rat(self.start.longitude(), math)?
                .mul(&line.radians_per_degree, math)?;
            if cos.lower() > &BigInt::from_i128(0) {
                Ok(longitude)
            } else if cos.upper() < &BigInt::from_i128(0) {
                longitude.sub(&FixedInterval::pi(math)?, math)
            } else {
                Err(MathError::PrecisionExhausted)
            }
        }
    }

    fn normal_image(
        &self,
        image: &ChartImage,
        math: &mut CoordinateMath,
    ) -> Result<([FixedInterval; 3], [FixedInterval; 3]), MathError> {
        let args = super::normal::NormalArguments {
            c: self.line.c.clone(),
            e2: self.line.e2.clone(),
            sin0: self.line.sin0.clone(),
            phase: self.phase_start(math)?.sub(&image.correction, math)?,
            sine: image.sine.clone(),
            cosine: image.cosine.clone(),
            vertical: image.vertical.clone(),
            horizontal: image.horizontal.clone(),
            h: image.h.clone(),
            dsigma: image.sigma_derivative.clone(),
        };
        super::normal::evaluate(&args, &mut super::normal::FixedArithmetic(math))
    }

    /// Complete normalized derivatives of the smooth normal and its parameter
    /// derivative. A caller supplies at least96 free jets at the desired order.
    /// Coefficient n encloses the derivative divided by n! at every center in
    /// the parameter interval, so the highest coefficient bounds Taylor's
    /// remainder. The autonomous line IVP determines each successive derivative
    /// by induction; no sampled derivative or quadrature difference is used.
    pub(crate) fn normal_taylor_in<'scope>(
        &self,
        parameter: &FixedInterval,
        iterations: u32,
        workspace: &mut TaylorWorkspace<'scope>,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<([TaylorJet<'scope>; 3], [TaylorJet<'scope>; 3]), MathError> {
        use super::normal::TaylorArithmetic;
        let distance = self.length.mul(parameter, math)?;
        let mut poll = |math: &CoordinateMath| progress.math_poll(math);
        let mut chunks = ChunkObserver::borrowed(&mut poll);
        let sigma0 = self
            .line
            .sigma_enclosure(&distance, iterations, &mut chunks, math)
            .map_err(|error| math_error(error, iterations))?;
        let correction = self.longitude_correction(&sigma0, &mut chunks, math)?;
        let phase0 = self.phase_start(math)?.sub(&correction, math)?;
        chunks.finish(math).map_err(|error| math_error(error, 0))?;
        let sigma = workspace.constant(sigma0.clone(), math)?;
        let phase = workspace.constant(phase0.clone(), math)?;
        let c = workspace.constant(self.line.c.clone(), math)?;
        let e2 = workspace.constant(self.line.e2.clone(), math)?;
        let sin0 = workspace.constant(self.line.sin0.clone(), math)?;
        let cos0 = workspace.constant(self.line.cos0.clone(), math)?;
        let q = workspace.constant(self.line.q.clone(), math)?;
        let b = workspace.constant(self.line.b.clone(), math)?;
        let length = workspace.constant(self.length.clone(), math)?;
        let one = workspace.constant(FixedInterval::from_i64(1, math)?, math)?;
        let retained = workspace.checkpoint();
        for _ in 0..workspace.order() {
            let (sine, _) = workspace.sin_cos(sigma, math)?;
            let sine2 = workspace.mul(sine, sine, math)?;
            let q_sine2 = workspace.mul(q, sine2, math)?;
            let h2 = workspace.add(one, q_sine2, math)?;
            let h = workspace.sqrt(h2, math)?;
            let b_h = workspace.mul(b, h, math)?;
            let dsigma = workspace.div(length, b_h, math)?;
            let dphase = super::normal::phase_speed(
                &c,
                &e2,
                &sin0,
                &h,
                &dsigma,
                &mut TaylorArithmetic { workspace, math },
            )?;
            let next_sigma = workspace.integral(dsigma, sigma0.clone(), math)?;
            let next_phase = workspace.integral(dphase, phase0.clone(), math)?;
            workspace.assign(sigma, next_sigma, math)?;
            workspace.assign(phase, next_phase, math)?;
            workspace.rewind(retained)?;
            progress.math_poll(math)?;
        }
        let (sine, cosine) = workspace.sin_cos(sigma, math)?;
        let sine2 = workspace.mul(sine, sine, math)?;
        let q_sine2 = workspace.mul(q, sine2, math)?;
        let h2 = workspace.add(one, q_sine2, math)?;
        let h = workspace.sqrt(h2, math)?;
        let b_h = workspace.mul(b, h, math)?;
        let dsigma = workspace.div(length, b_h, math)?;
        let vertical = workspace.mul(cos0, sine, math)?;
        let horizontal = workspace.mul(cos0, cosine, math)?;
        let args = super::normal::NormalArguments {
            c,
            e2,
            sin0,
            phase,
            sine,
            cosine,
            vertical,
            horizontal,
            h,
            dsigma,
        };
        let result = super::normal::evaluate(&args, &mut TaylorArithmetic { workspace, math })?;
        progress.math_poll(math)?;
        Ok(result)
    }

    fn longitude_correction(
        &self,
        sigma: &FixedInterval,
        chunks: &mut ChunkObserver<'_>,
        math: &mut CoordinateMath,
    ) -> Result<FixedInterval, MathError> {
        let line = &self.line;
        if line.sin0.lower().is_zero() && line.sin0.upper().is_zero() {
            FixedInterval::from_i64(0, math)
        } else {
            line.e2.mul(&line.sin0, math)?.mul(
                &line
                    .coefficients
                    .evaluate(&line.sigma1, sigma, &line.q, chunks, math)
                    .map_err(|error| math_error(error, 0))?
                    .longitude,
                math,
            )
        }
    }

    /// Unquantized Karney quadrilateral S12=+integral Q(phi)dlongitude on
    /// the selected continuous line lift. Internal meridian pole jumps are
    /// included. Atlas cuts from endpoint pole frames remain separate terms.
    /// Both parameter endpoint enclosures must be ordered on this source line.
    #[allow(clippy::too_many_arguments)] // Endpoint panel, reference and distinct pure scratch/governor capabilities.
    pub(crate) fn quadrilateral_area_in(
        &self,
        lower: &FixedInterval,
        upper: &FixedInterval,
        reference: &crate::GeographicReference,
        iterations: u32,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<FixedInterval, MathError> {
        if self.binding_id != crate::numerical::math_reference_identity(reference, math, progress)?
        {
            return Err(MathError::Domain("geodesic area reference mismatch"));
        }
        if lower.upper() > upper.lower() {
            return Err(MathError::Domain("unordered geodesic area endpoints"));
        }
        if lower.lower() == upper.upper()
            || (self.length.lower().is_zero() && self.length.upper().is_zero())
        {
            return FixedInterval::from_i64(0, math);
        }
        let mut poll = |math: &CoordinateMath| progress.math_poll(math);
        let mut chunks = ChunkObserver::borrowed(&mut poll);
        let evaluate_sigma = |parameter: &FixedInterval,
                              chunks: &mut ChunkObserver<'_>,
                              math: &mut CoordinateMath| {
            self.line
                .sigma_enclosure(&self.length.mul(parameter, math)?, iterations, chunks, math)
                .map_err(|error| math_error(error, iterations))
        };
        let sigma1 = evaluate_sigma(lower, &mut chunks, math)?;
        let sigma2 = evaluate_sigma(upper, &mut chunks, math)?;
        let alpha1 = self.area_endpoint_azimuth(lower, &sigma1, math)?;
        let alpha2 = self.area_endpoint_azimuth(upper, &sigma2, math)?;
        let ep2 = self.line.e2.div(&self.line.c.square(math)?, math)?;
        let area = super::inverse::area::quadrilateral(
            &alpha1,
            &alpha2,
            &sigma1,
            &sigma2,
            &self.line.sin0,
            &self.line.cos0.square(math)?,
            &self.line.q,
            &self.line.c,
            &ep2,
            reference,
            self.line.coefficients.area.as_deref(),
            &mut chunks,
            math,
        )
        .map_err(|error| math_error(error, iterations))?;
        chunks.finish(math).map_err(|error| math_error(error, 0))?;
        Ok(area)
    }

    fn area_endpoint_azimuth(
        &self,
        parameter: &FixedInterval,
        sigma: &FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<FixedInterval, MathError> {
        if parameter.lower().is_zero() && parameter.upper().is_zero() && self.start.is_pole() {
            return self.azimuth.mul(&self.line.radians_per_degree, math);
        }
        let (sine, cosine) = sigma.sin_cos_range(math)?;
        if self.line.sin0.lower().is_zero() && self.line.sin0.upper().is_zero() {
            let one = FixedInterval::from_i64(1, math)?;
            let exact_end_pole =
                parameter == &one && self.end.as_ref().is_some_and(LonLat::is_pole);
            if cosine.lower() > &BigInt::zero()
                || (exact_end_pole && sine.lower() > &BigInt::zero())
            {
                FixedInterval::from_i64(0, math)
            } else if cosine.upper() < &BigInt::zero()
                || (exact_end_pole && sine.upper() < &BigInt::zero())
            {
                FixedInterval::pi(math)
            } else {
                Err(MathError::PrecisionExhausted)
            }
        } else {
            FixedInterval::atan2(&self.line.sin0, &self.line.cos0.mul(&cosine, math)?, math)
        }
    }

    /// Exact indexed pole events strictly inside the complete selected line.
    /// Even indices identify North, odd indices South. Source/end pole frames
    /// exclude endpoint events independently of a rounded phase enclosure.
    pub(crate) fn interior_meridian_poles_in(
        &self,
        iterations: u32,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Option<PoleEventRange>, MathError> {
        if self.length.is_exact_zero() {
            return Ok(None);
        }
        if !self.line.sin0.is_exact_zero() {
            if self.line.sin0.lower() > &BigInt::zero() || self.line.sin0.upper() < &BigInt::zero()
            {
                return Ok(None);
            }
            return Err(MathError::PrecisionExhausted);
        }
        let first = if self.start.is_pole() {
            BigInt::from_i128(i128::from(self.start.latitude().signum() > 0))
        } else {
            let sides = meridian_side_interval(&self.line.sigma1, math)?;
            let (lower, upper) = sides.floor_bounds(math)?;
            if lower != upper {
                return Err(MathError::PrecisionExhausted);
            }
            lower
        };
        let mut poll = |math: &CoordinateMath| progress.math_poll(math);
        let mut chunks = ChunkObserver::borrowed(&mut poll);
        let end_sigma = self
            .line
            .sigma_enclosure(&self.length, iterations, &mut chunks, math)
            .map_err(|error| math_error(error, iterations))?;
        let sides = meridian_side_interval(&end_sigma, math)?;
        let last = if let Some(endpoint) = self.end.as_ref().filter(|end| end.is_pole()) {
            // The original endpoint proves its phase is an integer. A common
            // nearest-even integer result isolates which lifted pole it is;
            // this is phase identification, not approximate pole ownership.
            let (lower, upper) = sides.round_decimal(0, math)?;
            if lower != upper {
                return Err(MathError::PrecisionExhausted);
            }
            let parity = lower.div_rem_u64(2).expect("nonzero divisor").1;
            if parity != u64::from(endpoint.latitude().signum() > 0) {
                return Err(MathError::PrecisionExhausted);
            }
            lower.sub(&BigInt::from_i128(2))
        } else {
            let (lower, upper) = sides.floor_bounds(math)?;
            if lower != upper {
                return Err(MathError::PrecisionExhausted);
            }
            if self.end.is_none() {
                let boundary = FixedInterval::from_ratio(&lower, &BigInt::from_i128(1), math)?;
                if sides.lower() <= boundary.upper() {
                    return Err(MathError::PrecisionExhausted);
                }
            }
            lower.sub(&BigInt::from_i128(1))
        };
        chunks.finish(math).map_err(|error| math_error(error, 0))?;
        if first > last {
            return Ok(None);
        }
        Ok(Some(PoleEventRange {
            first,
            last,
            pi: FixedInterval::pi(math)?,
        }))
    }

    fn meridian_longitude(
        &self,
        cosine: &FixedInterval,
        sigma: &FixedInterval,
        math: &mut CoordinateMath,
    ) -> Result<(FixedInterval, Option<PoleEventRange>, Option<Rat>), MathError> {
        let pi = FixedInterval::pi(math)?;
        let (first_side, last_side) = meridian_side_interval(sigma, math)?.floor_bounds(math)?;
        let (base, initial_side) = if self.start.is_pole() {
            if self.start.latitude() < &Rat::zero() {
                (self.azimuth.clone(), BigInt::from_i128(0))
            } else {
                (
                    FixedInterval::from_i64(180, math)?.sub(&self.azimuth, math)?,
                    BigInt::from_i128(1),
                )
            }
        } else {
            let (first, last) =
                meridian_side_interval(&self.line.sigma1, math)?.floor_bounds(math)?;
            if first != last {
                return Err(MathError::PrecisionExhausted);
            }
            (fixed_from_rat(self.start.longitude(), math)?, first)
        };
        let offset =
            FixedInterval::from_ratio(&first_side.sub(&initial_side), &BigInt::from_i128(1), math)?
                .hull(
                    &FixedInterval::from_ratio(
                        &last_side.sub(&initial_side),
                        &BigInt::from_i128(1),
                        math,
                    )?,
                    math,
                )?
                .mul(&pi, math)?;
        let mut longitude = base
            .mul(&self.line.radians_per_degree, math)?
            .add(&offset, math)?;
        let zero = BigInt::from_i128(0);
        let poles = if cosine.lower() > &zero || cosine.upper() < &zero {
            None
        } else {
            let half =
                FixedInterval::from_ratio(&BigInt::from_i128(1), &BigInt::from_i128(2), math)?;
            let (first, last) = sigma.div(&pi, math)?.sub(&half, math)?.floor_bounds(math)?;
            if self.start.is_pole() && sigma.lower() <= self.line.sigma1.upper() {
                longitude = longitude.hull(&FixedInterval::from_i64(0, math)?, math)?;
            }
            Some(PoleEventRange {
                first,
                last: last.add(&BigInt::from_i128(1)),
                pi,
            })
        };
        let exact_longitude = if first_side == last_side && poles.is_none() {
            let source_bits = [self.start.longitude(), self.start.latitude()]
                .into_iter()
                .chain(self.exact_azimuth.iter())
                .map(crate::numerical::rational_operand_bits)
                .max()
                .unwrap_or(0);
            // Admit the original frame copies/subtraction before constructing
            // the exact base, and the subsequent lifted side offset together.
            let bits = source_bits
                .saturating_mul(2)
                .saturating_add(first_side.bits_upper_bound() as u64)
                .saturating_add(16);
            math.admit_exact(
                16,
                usize::try_from(bits).map_err(|_| MathError::WorkspaceExhausted)?,
            )?;
            let base = exact_meridian_base(&self.start, self.exact_azimuth.as_ref());
            base.map(|base| {
                base.add(
                    &Rat::from_int(carrier_integer(&first_side.sub(&initial_side)))
                        .mul(&Rat::from_i64(180)),
                )
            })
        } else {
            None
        };
        Ok((longitude, poles, exact_longitude))
    }
}

/// Continuous auxiliary longitude lift. Folding by pi keeps atan2's cosine
/// nonnegative, so its negative-axis cut never decides the branch.
pub(super) fn omega_lift(
    sigma: &FixedInterval,
    sin0: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let zero = BigInt::from_i128(0);
    let sign = if sin0.lower() > &zero {
        1
    } else if sin0.upper() < &zero {
        -1
    } else {
        return Err(MathError::PrecisionExhausted);
    };
    let one = FixedInterval::from_i64(1, math)?;
    if sin0.lower() == one.lower() && sin0.upper() == one.upper() {
        return Ok(sigma.clone());
    }
    let minus_one = one.neg(math)?;
    if sin0.lower() == minus_one.lower() && sin0.upper() == minus_one.upper() {
        return sigma.neg(math);
    }
    if sigma.upper().sub(sigma.lower()) < *one.lower() {
        return omega_point(sigma, sin0, sign, math);
    }
    let lower = FixedInterval::from_bounds(sigma.lower().clone(), sigma.lower().clone(), math)?;
    let upper = FixedInterval::from_bounds(sigma.upper().clone(), sigma.upper().clone(), math)?;
    let first = omega_point(&lower, sin0, sign, math)?;
    let last = omega_point(&upper, sin0, sign, math)?;
    first.hull(&last, math)
}

// Meridian chart side j changes at sigma=(j-1/2)*pi. This one original
// phase expression serves longitude lifts and strict interior pole counts.
fn meridian_side_interval(
    sigma: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let pi = FixedInterval::pi(math)?;
    sigma
        .add(&pi.div(&FixedInterval::from_i64(2, math)?, math)?, math)?
        .div(&pi, math)
}

fn omega_point(
    sigma: &FixedInterval,
    sin0: &FixedInterval,
    sign: i64,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let pi = FixedInterval::pi(math)?;
    let half_pi = pi.div(&FixedInterval::from_i64(2, math)?, math)?;
    let (mut k, last) = sigma
        .add(&half_pi, math)?
        .div(&pi, math)?
        .floor_bounds(math)?;
    if last.sub(&k) > BigInt::from_i128(2) {
        return Err(MathError::PrecisionExhausted);
    }
    let mut enclosure: Option<FixedInterval> = None;
    while k <= last {
        let factor = FixedInterval::from_ratio(&k, &BigInt::from_i128(1), math)?;
        let reduced = sigma.sub(&factor.mul(&pi, math)?, math)?;
        let chart =
            FixedInterval::from_bounds(half_pi.upper().negated(), half_pi.upper().clone(), math)?;
        if let Some(reduced) = reduced.intersection(&chart, math)? {
            let (sin, cos) = reduced.sin_cos_range(math)?;
            let cosine = FixedInterval::from_bounds(
                cos.lower().max(&BigInt::from_i128(0)).clone(),
                cos.upper().clone(),
                math,
            )?;
            let omega = FixedInterval::atan2(&sin0.mul(&sin, math)?, &cosine, math)?.add(
                &factor
                    .mul(&pi, math)?
                    .mul(&FixedInterval::from_i64(sign, math)?, math)?,
                math,
            )?;
            enclosure = Some(if let Some(old) = enclosure {
                old.hull(&omega, math)?
            } else {
                omega
            });
        }
        k = k.add(&BigInt::from_i128(1));
    }
    enclosure.ok_or(MathError::PrecisionExhausted)
}

fn math_error(error: SolveError, iterations: u32) -> MathError {
    match error {
        SolveError::Math(error) => error,
        SolveError::Iterations => MathError::ConvergenceExhausted { iterations },
        SolveError::External(_) => MathError::Cancelled,
    }
}

// Geodetic latitude is strictly increasing in sin(beta). Endpoint evaluation
// removes the artificial atan2 origin in a broad whole-line rectangle.
fn reduced_sine_latitude(
    sine: &FixedInterval,
    c: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let one = FixedInterval::from_i64(1, math)?;
    let negative_one = one.lower().negated();
    let low = sine.lower().max(&negative_one).clone();
    let high = sine.upper().min(one.upper()).clone();
    if low > high {
        return Err(MathError::PrecisionExhausted);
    }
    let mut values = purrdf_core::SmallVec::<[FixedInterval; 2]>::default();
    for bound in [low, high] {
        let value = FixedInterval::from_bounds(bound.clone(), bound, math)?;
        let square = one.sub(&value.square(math)?, math)?;
        let square = FixedInterval::from_bounds(
            square.lower().max(&BigInt::from_i128(0)).clone(),
            square.upper().clone(),
            math,
        )?;
        values.push(FixedInterval::atan2(
            &value,
            &c.mul(&square.sqrt(math)?, math)?,
            math,
        )?);
    }
    FixedInterval::from_bounds(values[0].lower().clone(), values[1].upper().clone(), math)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeographicReference, Int};
    use purrdf_xsd::math::MathLimits;
    fn rat(value: &str) -> Rat {
        Rat::parse_decimal(value).unwrap()
    }
    fn point(lon: &str, lat: &str) -> LonLat {
        LonLat::new(rat(lon), rat(lat)).unwrap()
    }
    fn contains(value: &FixedInterval, exact: &Rat) {
        let scale = Int::one().shl(value.precision_bits());
        assert!(
            value.lower().as_integer().mul(exact.denominator()) <= exact.numerator().mul(&scale)
        );
        assert!(
            value.upper().as_integer().mul(exact.denominator()) >= exact.numerator().mul(&scale)
        );
    }
    #[test]
    fn admitted_meridian_frame_keeps_pole_convention_and_exact_nonmeridians() {
        for (start, azimuth, expected) in [
            (point("73", "90"), rat("47"), Some(rat("133"))),
            (point("-81", "90"), rat("407"), Some(rat("133"))),
            (point("73", "-90"), rat("-1"), Some(rat("359"))),
            (point("12.125", "23"), rat("720"), Some(rat("12.125"))),
            (point("12.125", "23"), rat("-180"), Some(rat("12.125"))),
            (point("12.125", "23"), rat("90"), None),
            (point("12.125", "23"), rat("180.0000000000000000001"), None),
        ] {
            let mut context = MetricContext::wgs84().unwrap();
            context.begin(0).unwrap();
            let mut progress = WorkProgress::new(None);
            let mut retained = 0;
            let actual = meridian_plane_degrees_admitted(
                &start,
                &azimuth,
                &mut context,
                &mut progress,
                &mut retained,
            )
            .unwrap();
            assert_eq!(actual, expected);
            assert!(retained > 0);
            assert_eq!(context.current_workspace_bytes(), retained);
            drop(actual);
            context.release_workspace(retained).unwrap();
            assert_eq!(context.current_workspace_bytes(), 0);
        }
        let start = point("73", "90");
        let azimuth = Rat::from_int(Int::one().shl(4096));
        for memory in [false, true] {
            let limits = crate::ExecutionLimits {
                max_work_items: if memory { 1_000_000_000 } else { 1 },
                max_workspace_bytes: if memory { 1 } else { 64 * 1024 * 1024 },
                ..crate::ExecutionLimits::GEOMETRY
            };
            let mut context = MetricContext::new(
                GeographicReference::wgs84(),
                crate::ExecutionPolicy::new(limits).unwrap(),
            )
            .unwrap();
            context.begin(0).unwrap();
            let mut retained = 0;
            let window = purrdf_alloc_probe::CurrentThreadWindow::open();
            let result = meridian_plane_degrees_admitted(
                &start,
                &azimuth,
                &mut context,
                &mut WorkProgress::new(None),
                &mut retained,
            );
            let measured = window.close();
            assert!(matches!(
                result,
                Err(GeoError::WorkExhausted { limit: 1 } | GeoError::MemoryExhausted { limit: 1 })
            ));
            assert_eq!(measured.allocations, 0);
            assert_eq!(measured.requested_bytes, 0);
            context.release_workspace(retained).unwrap();
            assert_eq!(context.current_workspace_bytes(), 0);
        }
    }
    #[test]
    fn exact_equatorial_source_preserves_zero_latitude_and_origin_image() {
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        let view = prepared
            .prepare_azimuth_arc_view(&point("0", "0"), &rat("90"), &Metres::new(rat("100")))
            .unwrap();
        for parameter in [Rat::zero(), rat("0.0625"), Rat::one()] {
            let image = view
                .parameter_enclosure(&parameter, &parameter, &mut context, None)
                .unwrap();
            assert!(image.latitude.is_exact_zero());
            assert!(image.normal[2].is_exact_zero());
            if parameter.is_zero() {
                assert!(image.longitude.is_exact_zero());
            }
        }
    }

    #[test]
    fn retained_arc_proofs_release_worker_destinations_before_arena_growth() {
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        let start = point("0", "30");
        let end = point("60", "40");
        let original = prepared
            .prepare_shortest_arc_view(&start, &end, 224, &mut context, None)
            .unwrap();
        let refined = original.refined(448, &mut context, None).unwrap();
        let scratch = context.integer_scratch().unwrap();
        assert_eq!(scratch.available(), scratch.destination_capacity());
        context.prepare_integer_scratch_for(8192).unwrap();
        drop(context);
        assert_eq!(original.start(), &start);
        assert_eq!(refined.end(), Some(&end));
        assert!(refined.length.0.exact() <= refined.length.1.exact());
        assert!(refined.azimuth.0 <= refined.azimuth.1);
    }

    #[test]
    fn a_cached_constant_arc_cannot_bypass_the_invocation_precision_policy() {
        let reference = GeographicReference::wgs84();
        let prepared = PreparedGeodesic::new(reference.clone());
        let mut context = MetricContext::wgs84().unwrap();
        let source =
            crate::PreparedCoordinate::new(crate::Coord::xy(rat("17"), rat("42")), &reference)
                .unwrap();
        let arc = ShortestGeodesicArc::new(source.clone(), source, &mut context).unwrap();
        assert_eq!(
            prepared
                .prepare_shortest_arc_view_with_proof(&arc, 513, &mut context, None)
                .unwrap_err(),
            GeoError::PrecisionExhausted { bits: 512 }
        );
        let view = prepared
            .prepare_shortest_arc_view_with_proof(&arc, 80, &mut context, None)
            .unwrap();
        assert!(view.is_constant());
    }

    #[test]
    fn requested_arc_precision_tightens_unrounded_source_parameters() {
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        let start = point("0", "30");
        let end = point("60", "40");
        let original = prepared.inverse(&start, &end, &mut context).unwrap();
        let ordinary = prepared
            .prepare_shortest_arc_view(&start, &end, 80, &mut context, None)
            .unwrap();
        let view = ordinary.refined(96, &mut context, None).unwrap();
        let fine = view.refined(128, &mut context, None).unwrap();
        for (bits, view) in [(96, &view), (128, &fine)] {
            let quantum = Rat::new(Int::one(), Int::one().shl(bits / 2)).unwrap();
            assert!(view.length.1.exact().sub(view.length.0.exact()) <= quantum);
            assert!(view.azimuth.1.sub(&view.azimuth.0) <= quantum);
            assert_eq!(view.start(), &start);
            assert_eq!(view.end(), Some(&end));
        }
        assert_eq!(
            prepared.inverse(&start, &end, &mut context).unwrap(),
            original
        );
        assert_eq!(
            view.refined(513, &mut context, None).unwrap_err(),
            GeoError::PrecisionExhausted { bits: 512 },
        );
    }
    #[test]
    fn complete_equatorial_image_and_tangent_keep_longitude_lift() {
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        let view = prepared
            .prepare_azimuth_arc_view(
                &point("170", "0"),
                &rat("90"),
                &Metres::new(rat("10000000")),
            )
            .unwrap();
        let image = view
            .parameter_enclosure(&Rat::zero(), &Rat::one(), &mut context, None)
            .unwrap();
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        // Compare physical derivatives in their exact rational units, independently
        // of any emitted decimal-degree endpoint.
        let radians_per_degree = FixedInterval::pi(&mut math)
            .unwrap()
            .div(&FixedInterval::from_i64(180, &mut math).unwrap(), &mut math)
            .unwrap();
        let (lonlo, lonhi) = crate::numerical::exact_bounds(&image.longitude);
        let (anglelo, _) = crate::numerical::exact_bounds(&radians_per_degree);
        assert!(lonlo <= anglelo.mul(&rat("170")));
        assert!(lonhi > anglelo.mul(&rat("200")));
        contains(&image.latitude, &Rat::zero());
        let derivative = image.derivative.as_ref().unwrap();
        contains(
            &derivative[0],
            &rat("10000000").div(&rat("6378137")).unwrap(),
        );
        contains(&derivative[1], &Rat::zero());
        assert!(image.sigma_derivative.lower() > &BigInt::from_i128(0));
        assert!(image.meridian_poles.is_none());
        assert_eq!(view.start(), &point("170", "0"));
        assert_eq!(view.end(), None);
    }
    #[test]
    fn complete_meridian_images_name_poles_and_support_signed_continuation() {
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        let view = prepared
            .prepare_azimuth_arc_view(
                &point("0", "0"),
                &Rat::zero(),
                &Metres::new(rat("30000000")),
            )
            .unwrap();
        let image = view
            .parameter_enclosure(&Rat::zero(), &Rat::one(), &mut context, None)
            .unwrap();
        assert!(image.derivative.is_none());
        let poles = image.meridian_poles.as_ref().unwrap();
        assert!(poles.first <= poles.last);
        assert!(image.sigma_derivative.lower() > &BigInt::from_i128(0));
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        let mut progress = WorkProgress::new(None);
        progress.initial().unwrap();
        let line = view.prepare_in(&mut math, &mut progress).unwrap();
        let poles = line
            .interior_meridian_poles_in(128, &mut math, &mut progress)
            .unwrap()
            .unwrap();
        assert_eq!(poles.first, BigInt::zero());
        assert_eq!(poles.last, BigInt::zero());
        let parameter = fixed_from_bounds(&rat("-0.000001"), &rat("0.000001"), &mut math).unwrap();
        assert!(
            line.parameter_enclosure_in(&parameter, 128, &mut math, &mut progress)
                .is_err()
        );
        let continuation = line
            .continuation_enclosure_in(&parameter, 128, &mut math, &mut progress)
            .unwrap();
        contains(&continuation.latitude, &Rat::zero());
        contains(&continuation.longitude, &Rat::zero());
        assert!(continuation.derivative.is_some());
    }
    #[test]
    fn shortest_view_retains_endpoints_and_metered_receipts() {
        #[derive(Default)]
        struct Counts(u64, u64);
        impl MetricWorkObserver for Counts {
            fn charge_chunk(&mut self, work: u64, bytes: u64) -> Result<(), GeoError> {
                self.0 += work;
                self.1 += bytes;
                Ok(())
            }
        }
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        let a = point("0", "0");
        let b = point("2", "0");
        let view = prepared
            .prepare_shortest_arc_view(&a, &b, 80, &mut context, None)
            .unwrap();
        assert_eq!(view.start(), &a);
        assert_eq!(view.end(), Some(&b));
        let refined = view.refined(112, &mut context, None).unwrap();
        assert!(refined.proof_bits() >= 112);
        let mut counts = Counts::default();
        let image = refined
            .parameter_enclosure(&rat("0.5"), &rat("0.5"), &mut context, Some(&mut counts))
            .unwrap();
        assert_eq!(counts.0, context.work_items());
        assert_eq!(counts.1, context.workspace_peak());
        contains(&image.latitude, &Rat::zero());
    }
    #[test]
    fn zero_views_and_normal_vectors_stay_defined_at_coordinate_poles() {
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let mut context = MetricContext::wgs84().unwrap();
        for source in [point("37", "42"), point("179", "90"), point("-170", "-90")] {
            let view = prepared
                .prepare_shortest_arc_view(&source, &source, 80, &mut context, None)
                .unwrap();
            assert!(view.is_constant());
            let image = view
                .parameter_enclosure(&Rat::zero(), &Rat::one(), &mut context, None)
                .unwrap();
            for derivative in &image.normal_derivative {
                contains(derivative, &Rat::zero());
            }
            if source.is_pole() {
                contains(&image.normal[0], &Rat::zero());
                contains(&image.normal[1], &Rat::zero());
                contains(
                    &image.normal[2],
                    &Rat::from_i64(i64::from(source.latitude().signum())),
                );
            }
        }
        let view = prepared
            .prepare_azimuth_arc_view(
                &point("0", "0"),
                &Rat::zero(),
                &Metres::new(rat("11000000")),
            )
            .unwrap();
        let image = view
            .parameter_enclosure(&rat("0.9"), &rat("0.92"), &mut context, None)
            .unwrap();
        assert!(image.meridian_poles.is_some());
        contains(&image.normal[0], &Rat::zero());
        assert!(
            image.normal[2].upper()
                >= &BigInt::from_i128(1).mul_pow2(image.normal[2].precision_bits())
        );
        for derivative in &image.normal_derivative {
            assert!(derivative.lower() <= derivative.upper());
        }
    }

    #[test]
    fn normal_taylor_equator_coefficients_follow_the_exact_geodesic_equations() {
        use purrdf_xsd::math::with_taylor_workspace;
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let view = prepared
            .prepare_azimuth_arc_view(&point("0", "0"), &rat("90"), &Metres::new(rat("6378137")))
            .unwrap();
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        let mut progress = WorkProgress::new(None);
        progress.initial().unwrap();
        let line = view.prepare_in(&mut math, &mut progress).unwrap();
        let parameter = FixedInterval::from_i64(0, &mut math).unwrap();
        with_taylor_workspace(8, 96, &mut math, |workspace, math| {
            let (normal, derivative) =
                line.normal_taylor_in(&parameter, 128, workspace, math, &mut progress)?;
            let mut factorial = 1_i64;
            for n in 0..=8 {
                if n != 0 {
                    factorial *= n as i64;
                }
                let sign = if (n / 2) % 2 == 0 { 1 } else { -1 };
                let x = if n % 2 == 0 { sign } else { 0 };
                let y = if n % 2 == 1 { sign } else { 0 };
                for (value, numerator) in [(normal[0], x), (normal[1], y), (normal[2], 0)] {
                    contains(
                        workspace.coefficient(value, n)?,
                        &Rat::from_i64(numerator)
                            .div(&Rat::from_i64(factorial))
                            .unwrap(),
                    );
                }
                let next_sign = if n.div_ceil(2).is_multiple_of(2) {
                    1
                } else {
                    -1
                };
                let dx = if n % 2 == 1 { next_sign } else { 0 };
                let dy = if n % 2 == 0 { next_sign } else { 0 };
                for (value, numerator) in
                    [(derivative[0], dx), (derivative[1], dy), (derivative[2], 0)]
                {
                    contains(
                        workspace.coefficient(value, n)?,
                        &Rat::from_i64(numerator)
                            .div(&Rat::from_i64(factorial))
                            .unwrap(),
                    );
                }
            }
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn unrounded_area_keeps_meridian_pole_jumps_and_additive_panels() {
        let reference = GeographicReference::wgs84();
        let prepared = PreparedGeodesic::new(reference.clone());
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        let mut progress = WorkProgress::new(None);
        progress.initial().unwrap();
        let zero = FixedInterval::from_i64(0, &mut math).unwrap();
        let one = FixedInterval::from_i64(1, &mut math).unwrap();
        let view = prepared
            .prepare_azimuth_arc_view(&point("0", "0"), &rat("90"), &Metres::new(rat("6378137")))
            .unwrap();
        let line = view.prepare_in(&mut math, &mut progress).unwrap();
        assert!(
            line.quadrilateral_area_in(&zero, &one, &reference, 128, &mut math, &mut progress)
                .unwrap()
                .is_exact_zero()
        );
        assert!(matches!(
            line.quadrilateral_area_in(
                &zero,
                &one,
                &GeographicReference::cgcs2000(),
                128,
                &mut math,
                &mut progress,
            ),
            Err(MathError::Domain("geodesic area reference mismatch"))
        ));

        let view = prepared
            .prepare_azimuth_arc_view(
                &point("0", "0"),
                &Rat::zero(),
                &Metres::new(rat("30000000")),
            )
            .unwrap();
        let line = view.prepare_in(&mut math, &mut progress).unwrap();
        let middle = fixed_from_rat(&rat("0.2"), &mut math).unwrap();
        let end = fixed_from_rat(&rat("0.6"), &mut math).unwrap();
        let whole = line
            .quadrilateral_area_in(&zero, &end, &reference, 128, &mut math, &mut progress)
            .unwrap();
        let first = line
            .quadrilateral_area_in(&zero, &middle, &reference, 128, &mut math, &mut progress)
            .unwrap();
        let last = line
            .quadrilateral_area_in(&middle, &end, &reference, 128, &mut math, &mut progress)
            .unwrap();
        assert!(first.is_exact_zero());
        let sum = first.add(&last, &mut math).unwrap();
        assert!(whole.intersection(&sum, &mut math).unwrap().is_some());
        let half_hemisphere =
            super::super::inverse::area::authalic_radius_squared(&reference, &mut math)
                .unwrap()
                .mul(&FixedInterval::pi(&mut math).unwrap(), &mut math)
                .unwrap();
        assert!(
            whole
                .intersection(&half_hemisphere, &mut math)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn north_wedge_area_leaves_canonical_endpoint_pole_cut_explicit() {
        let reference = GeographicReference::wgs84();
        let prepared = PreparedGeodesic::new(reference.clone());
        let north = point("0", "90");
        let equator_zero = point("0", "0");
        let equator_sixty = point("60", "0");
        let mut context = MetricContext::wgs84().unwrap();
        for (start, end) in [
            (&north, &equator_zero),
            (&equator_zero, &equator_sixty),
            (&equator_sixty, &north),
        ] {
            let view = prepared
                .prepare_shortest_arc_view(start, end, 80, &mut context, None)
                .unwrap();
            let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
            let mut progress = WorkProgress::new(None);
            progress.initial().unwrap();
            let line = view.prepare_in(&mut math, &mut progress).unwrap();
            assert!(
                line.interior_meridian_poles_in(128, &mut math, &mut progress)
                    .unwrap()
                    .is_none()
            );
            let zero = FixedInterval::from_i64(0, &mut math).unwrap();
            let one = FixedInterval::from_i64(1, &mut math).unwrap();
            let area = line
                .quadrilateral_area_in(&zero, &one, &reference, 128, &mut math, &mut progress)
                .unwrap();
            contains(&area, &Rat::zero());
        }
        // The remaining North cut is longitude 60 -> canonical longitude 0.
        // Its -Qnorth*dlongitude is the positive area of this 60-degree wedge.
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        let authalic =
            super::super::inverse::area::authalic_radius_squared(&reference, &mut math).unwrap();
        let wedge = authalic
            .mul(&FixedInterval::pi(&mut math).unwrap(), &mut math)
            .unwrap()
            .div(&FixedInterval::from_i64(3, &mut math).unwrap(), &mut math)
            .unwrap();
        assert!(wedge.lower() > &BigInt::zero());
    }

    #[test]
    fn south_wedge_exact_poles_are_excluded_from_interior_event_counts() {
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        let south = point("0", "-90");
        let zero = point("0", "0");
        let sixty = point("60", "0");
        let mut context = MetricContext::wgs84().unwrap();
        for (start, end) in [(&south, &sixty), (&sixty, &zero), (&zero, &south)] {
            let view = prepared
                .prepare_shortest_arc_view(start, end, 96, &mut context, None)
                .unwrap();
            let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
            let mut progress = WorkProgress::new(None);
            let line = view
                .prepare_in(&mut math, &mut progress)
                .unwrap_or_else(|error| panic!("prepare {start:?} -> {end:?}: {error:?}"));
            assert!(
                line.interior_meridian_poles_in(128, &mut math, &mut progress)
                    .unwrap_or_else(|error| panic!("{start:?} -> {end:?}: {error:?}"))
                    .is_none()
            );
        }
    }
    #[test]
    fn strict_meridian_pole_indices_exclude_source_and_keep_internal_south() {
        let prepared = PreparedGeodesic::new(GeographicReference::wgs84());
        for (length, expected) in [("1000000", None), ("30000000", Some(1_i128))] {
            let view = prepared
                .prepare_azimuth_arc_view(
                    &point("73", "90"),
                    &Rat::zero(),
                    &Metres::new(rat(length)),
                )
                .unwrap();
            let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
            let mut progress = WorkProgress::new(None);
            progress.initial().unwrap();
            let line = view.prepare_in(&mut math, &mut progress).unwrap();
            let actual = line
                .interior_meridian_poles_in(128, &mut math, &mut progress)
                .unwrap();
            if let Some(index) = expected {
                let actual = actual.unwrap();
                assert_eq!(actual.first, BigInt::from_i128(index));
                assert_eq!(actual.last, BigInt::from_i128(index));
            } else {
                assert!(actual.is_none());
            }
        }
    }
}
