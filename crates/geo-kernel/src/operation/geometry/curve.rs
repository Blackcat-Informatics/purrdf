// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original source/chain curves. Export chords never become metric inputs.

use super::{OperationChain, OperationSolverLimits, PanelKind, source_delta};
use crate::context::WorkProgress;
use crate::numerical::{exact_bounds, fixed_from_rat, geo_math_error};
use crate::{
    Coord, ExecutionPolicy, GeoError, GeographicReference, Geometry, GeometryBody, MetricContext,
    MetricWorkObserver, Rat, SourceLinearEdge,
};
use purrdf_hash::{Domain, hex::Digest32};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};
use std::sync::Arc;

const SOURCE_DOMAIN: Domain = Domain::new(b"purrdf-geo-kernel/operation-image-curve/v1");

/// Immutable complete image of an original written coordinate-linear edge.
/// Shared workers retain the source and chain; each worker owns its scratch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationImageCurve {
    source: Arc<Geometry>,
    chain: Arc<OperationChain>,
    reference: GeographicReference,
    epoch: Option<Rat>,
    id: Digest32,
    retained_bytes: u64,
    preparation_work: u64,
    exact_source_linear: bool,
}

struct ImageSource {
    start: Coord,
    end: Option<Coord>,
}

mod selected;

/// Unrounded complete target image and parameter derivative, in angular degrees.
#[derive(Clone, Debug)]
pub struct OperationImageEnclosure {
    /// Continuous normalized longitude image; the atlas identifies its seams.
    pub longitude: FixedInterval,
    /// Complete geodetic latitude image.
    pub latitude: FixedInterval,
    /// Whole-panel derivatives with respect to the original parameter in `[0,1]`.
    /// Absence requires refinement or a separately certified singularity law.
    pub derivative: Option<[FixedInterval; 2]>,
}

impl OperationImageCurve {
    /// A pointwise-equal original-parameter image obtained from a certified
    /// polynomial source reflection. This alias is private topology proof data.
    pub(crate) fn reflected_source_towards_in(
        &self,
        other: &Self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<Option<Self>, GeoError> {
        selected::reflected_source_towards(self, other, context, progress, retained)
    }

    pub(crate) fn joined_source_in(
        &self,
        other: &Self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<Option<Self>, GeoError> {
        selected::joined_source(self, other, context, progress, retained)
    }
    /// A complete exact original-source restriction for topology branch
    /// partitioning. The retained public image and scalar metric are untouched.
    pub(crate) fn restricted_in(
        &self,
        bounds: [&Rat; 2],
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<Self, GeoError> {
        selected::restricted(self, bounds[0], bounds[1], context, progress, retained)
    }
    /// These elementary models are real analytic on every domain admitted by
    /// the original complete panel evaluator. This is a regularity witness,
    /// not an injectivity claim: a component still needs a whole-panel weak
    /// derivative sign and an independently certified nonconstant derivative.
    pub(crate) fn analytic_in_parameter(
        &self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<bool, GeoError> {
        for operation in self.chain.operations() {
            context.charge_work(1)?;
            progress.context_poll(context)?;
            if !matches!(
                operation.model(),
                crate::operation::OperationModel::Polynomial2d(_)
                    | crate::operation::OperationModel::Similarity2d(_)
                    | crate::operation::OperationModel::Helmert(_)
                    | crate::operation::OperationModel::EllipsoidalMercator { .. }
                    | crate::operation::OperationModel::WebMercator { .. }
                    | crate::operation::OperationModel::MercatorToGeographic { .. }
            ) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    /// An independently certified equivalent selected source image. This is
    /// used by topology only; the public original parameter and metric inputs
    /// remain retained by `self`.
    pub(crate) fn selected_set_in(
        &self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<Option<Self>, GeoError> {
        if let Some(image) = selected::selected_range(self, context, progress, retained)? {
            Ok(Some(image))
        } else {
            selected::reflected_half(self, context, progress, retained)
        }
    }
    /// Retain original source ordinates, actual operation and target reference.
    /// Z and M remain exact; a model requiring height must receive actual Z.
    ///
    /// # Errors
    /// Refuses inconsistent endpoint dimensions, target realization/axes/units,
    /// missing required metadata, or complete preparation admission.
    pub fn new(
        start: Coord,
        end: Coord,
        chain: Arc<OperationChain>,
        reference: GeographicReference,
        epoch: Option<Rat>,
        policy: ExecutionPolicy,
    ) -> Result<Self, GeoError> {
        Self::prepare(
            ImageSource {
                start,
                end: Some(end),
            },
            chain,
            reference,
            epoch,
            policy,
            None,
        )
    }
    /// Retain an original image curve with bounded preparation and identity polling.
    ///
    /// # Errors
    /// Adds observer refusal to [`Self::new`].
    pub fn new_metered(
        start: Coord,
        end: Coord,
        chain: Arc<OperationChain>,
        reference: GeographicReference,
        epoch: Option<Rat>,
        policy: ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare(
            ImageSource {
                start,
                end: Some(end),
            },
            chain,
            reference,
            epoch,
            policy,
            Some(observer),
        )
    }
    pub(super) fn from_point_metered(
        source: Coord,
        chain: Arc<OperationChain>,
        reference: GeographicReference,
        epoch: Option<Rat>,
        policy: ExecutionPolicy,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::prepare(
            ImageSource {
                start: source,
                end: None,
            },
            chain,
            reference,
            epoch,
            policy,
            Some(observer),
        )
    }
    fn prepare(
        source: ImageSource,
        chain: Arc<OperationChain>,
        reference: GeographicReference,
        epoch: Option<Rat>,
        policy: ExecutionPolicy,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let target = chain
            .operations()
            .last()
            .expect("compiled nonempty chain")
            .target();
        if target.unit != super::super::CoordinateUnit::Degrees
            || target.realization != reference.id().digest()
            || target.swapped_axes != (reference.axes() == crate::AxisOrder::LatLon)
        {
            return Err(GeoError::config(
                "image curve requires its actual geographic target",
            ));
        }
        let mut context = MetricContext::new(reference.clone(), policy)?;
        context.begin(1)?;
        let mut progress = WorkProgress::new(observer);
        progress.context_poll(&context)?;
        let coordinates = [Some(&source.start), source.end.as_ref()];
        for coordinate in coordinates.into_iter().flatten() {
            chain.prepare_source_coordinate(
                coordinate,
                epoch.as_ref(),
                &mut context,
                &mut progress,
            )?;
        }
        let exact_source_linear = if chain.exact_tm_roundtrip().is_some() {
            let source_reference = chain.operations()[0].source();
            let end = source.end.as_ref().unwrap_or(&source.start);
            let latitudes = if source_reference.swapped_axes {
                [source.start.x(), end.x()]
            } else {
                [source.start.y(), end.y()]
            };
            let cost = crate::numerical::rational_cost(
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &latitudes,
                4,
            )
            .ok_or(GeoError::ArithmeticOverflow(
                "nonpolar inverse-pair curve proof",
            ))?;
            progress.exact(&mut context, cost, || {
                Ok(latitudes.into_iter().all(|latitude| {
                    latitude > &Rat::from_i64(-90) && latitude < &Rat::from_i64(90)
                }))
            })?
        } else {
            false
        };
        let (epoch_fields, epoch_field_bytes) =
            super::super::epoch_fields_admitted(epoch.as_ref(), &mut context, &mut progress)?;
        context.admit_workspace(epoch_field_bytes.saturating_add(1024))?;
        let source = Arc::new(match source.end {
            Some(end) => Geometry::new(
                source.start.dim(),
                GeometryBody::LineString(vec![source.start, end]),
            )?,
            None => Geometry::new(source.start.dim(), GeometryBody::Point(Some(source.start)))?,
        });
        let remaining = policy.remaining_after(context.work_items(), context.workspace_peak())?;
        let inventory = {
            let mut nested = progress.nested(context.work_items(), 0, context.workspace_peak());
            crate::prepared::source_inventory(&source, remaining.limits(), Some(&mut nested))?
        };
        context.charge_work(inventory.work_items)?;
        context.admit_workspace(inventory.workspace_bytes)?;
        let mut fields = vec![
            inventory.content_id.as_bytes().to_vec(),
            chain.id().as_bytes().to_vec(),
            reference.id().digest().as_bytes().to_vec(),
            vec![u8::from(epoch.is_some())],
        ];
        let epoch_bytes = epoch.as_ref().map_or(0, |value| {
            value
                .numerator()
                .allocated_bytes()
                .saturating_add(value.denominator().allocated_bytes())
        });
        let retained_bytes = inventory.workspace_bytes.saturating_add(epoch_bytes as u64);
        if retained_bytes > policy.limits().max_workspace_bytes {
            return Err(GeoError::MemoryExhausted {
                limit: policy.limits().max_workspace_bytes,
            });
        }
        fields.extend(epoch_fields);
        context.charge_work(fields.len() as u64)?;
        progress.context_poll(&context)?;
        let id = crate::profile::hash_fields(SOURCE_DOMAIN, fields.iter().map(Vec::as_slice));
        progress.context_poll(&context)?;
        Ok(Self {
            source,
            chain,
            reference,
            epoch,
            id,
            retained_bytes,
            preparation_work: context.work_items(),
            exact_source_linear,
        })
    }
    /// Exact original curve, operation and target identity; admission is excluded.
    #[must_use]
    pub const fn id(&self) -> Digest32 {
        self.id
    }
    /// Actual target geographic reference.
    #[must_use]
    pub const fn reference(&self) -> &GeographicReference {
        &self.reference
    }
    /// Whether two original source edges use exactly the same mathematical
    /// mapping. This proves symbolic ring junctions before coordinate rounding.
    #[must_use]
    pub fn same_mapping(&self, other: &Self) -> bool {
        self.chain.id() == other.chain.id()
            && self.reference == other.reference
            && self.epoch == other.epoch
    }
    /// Admit the original declaration and epoch scans before comparing the
    /// retained mappings. The immutable chain identity is a fixed-size key.
    pub(crate) fn same_mapping_admitted(
        &self,
        other: &Self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<bool, GeoError> {
        context.charge_work(1)?;
        progress.context_poll(context)?;
        if self.chain.id() != other.chain.id() {
            return Ok(false);
        }
        let mut cost = crate::numerical::reference_copy_cost(&self.reference)?
            .followed_by(crate::numerical::reference_copy_cost(&other.reference)?)
            .ok_or(GeoError::ArithmeticOverflow("mapping declaration scans"))?;
        let epochs = self
            .epoch
            .iter()
            .chain(other.epoch.iter())
            .collect::<purrdf_core::SmallVec<[&Rat; 2]>>();
        cost = cost
            .followed_by(
                crate::numerical::rational_cost(
                    purrdf_xsd::integer::ExactOperation::Linear,
                    &epochs,
                    1,
                )
                .ok_or(GeoError::ArithmeticOverflow("mapping epoch scan"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow("mapping declaration scans"))?;
        progress.exact(context, cost, || Ok(self.same_mapping(other)))
    }
    pub(crate) const fn exact_epoch(&self) -> Option<&Rat> {
        self.epoch.as_ref()
    }
    /// The complete operation chain mapping the original source to the image.
    pub(crate) fn chain(&self) -> &OperationChain {
        &self.chain
    }
    /// Original endpoint ordinates, including Z and M.
    #[must_use]
    pub fn source_endpoints(&self) -> (&Coord, &Coord) {
        match self.source.body() {
            GeometryBody::LineString(points) => (&points[0], &points[1]),
            GeometryBody::Point(Some(point)) => (point, point),
            _ => unreachable!("checked original edge or symbolic point"),
        }
    }
    /// An explicit matched TM roundtrip has the original affine image on its
    /// completely admitted nonpolar source rectangle. The operation IDs remain
    /// part of provenance; no exported or rounded projected chord is reused.
    pub(crate) fn exact_linear_endpoints_in(
        &self,
        range: Option<[&Rat; 2]>,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Option<[crate::LonLat; 2]>, MathError> {
        if !self.exact_source_linear {
            return Ok(None);
        }
        let (start, end) = self.source_endpoints();
        let axes = self.chain.operations()[0].source().swapped_axes;
        let (start, end) = if axes {
            ([start.y(), start.x()], [end.y(), end.x()])
        } else {
            ([start.x(), start.y()], [end.x(), end.y()])
        };
        let unit = [Rat::zero(), Rat::one()];
        let range = range.unwrap_or_else(|| unit.each_ref());
        let mut endpoint = |parameter: &Rat| {
            let longitude = SourceLinearEdge::interpolate_ordinate_math(
                start[0], end[0], parameter, math, progress,
            )?;
            let latitude = SourceLinearEdge::interpolate_ordinate_math(
                start[1], end[1], parameter, math, progress,
            )?;
            crate::numerical::math_exact_rational(
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &[&longitude, &latitude],
                math,
                progress,
                || crate::LonLat::new(longitude.clone(), latitude.clone()),
            )?
            .map_err(|_| {
                MathError::Domain("original inverse-pair fragment outside geographic range")
            })
        };
        Ok(Some([endpoint(range[0])?, endpoint(range[1])?]))
    }
    /// Maximum original numerical operand width across the complete retained
    /// source/mapping/epoch, read without copying coordinates or parameters.
    pub(crate) fn max_original_operand_bits(&self) -> u64 {
        let endpoints: [&Coord; 2] = self.source_endpoints().into();
        let source_bits = crate::numerical::scratch_source_bits_in(
            endpoints.into_iter().flat_map(|point| {
                [Some(point.x()), Some(point.y()), point.z(), point.m()]
                    .into_iter()
                    .flatten()
            }),
            self.reference.ellipsoid(),
        );
        source_bits.max(self.chain.max_original_operand_bits()).max(
            self.epoch
                .as_ref()
                .map_or(0, crate::numerical::rational_operand_bits),
        )
    }

    /// Complete preparation work, independent of subsequent refinement.
    #[must_use]
    pub const fn preparation_work(&self) -> u64 {
        self.preparation_work
    }
    /// Conservative live source retention for bounded preparation and queries.
    #[must_use]
    pub const fn retained_workspace_bytes(&self) -> u64 {
        self.retained_bytes
    }
    pub(super) fn materialize_source_point(
        &self,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<super::super::TransformResult, GeoError> {
        self.chain.apply_source_coordinate_admitted(
            self.source_endpoints().0,
            self.epoch.as_ref(),
            context,
            progress,
        )
    }
    /// Enclose the complete original image panel without intermediate or final
    /// coordinate rounding. Explicit precision affects proof, never source law.
    ///
    /// # Errors
    /// Refuses an invalid parameter panel, mismatched reference, domain crossing,
    /// precision/work/memory admission, or unresolved inverse families.
    pub fn enclosure(
        &self,
        lower: &Rat,
        upper: &Rat,
        precision_bits: u32,
        context: &mut MetricContext,
    ) -> Result<OperationImageEnclosure, GeoError> {
        self.enclosure_inner(lower, upper, precision_bits, context, None)
    }
    /// Enclose a complete panel with bounded work and cancellation callbacks.
    ///
    /// # Errors
    /// Preserves an observer refusal without a partial image.
    pub fn enclosure_metered(
        &self,
        lower: &Rat,
        upper: &Rat,
        precision_bits: u32,
        context: &mut MetricContext,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<OperationImageEnclosure, GeoError> {
        self.enclosure_inner(lower, upper, precision_bits, context, Some(observer))
    }
    fn enclosure_inner(
        &self,
        lower: &Rat,
        upper: &Rat,
        bits: u32,
        context: &mut MetricContext,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<OperationImageEnclosure, GeoError> {
        if context.reference() != &self.reference {
            return Err(GeoError::config("image curve/context reference mismatch"));
        }
        context.begin(1)?;
        let policy = context.policy();
        if bits > policy.limits().max_precision_bits {
            return Err(GeoError::PrecisionExhausted {
                bits: policy.limits().max_precision_bits,
            });
        }
        context.admit_workspace(self.retained_bytes)?;
        let result = (|| {
            let mut progress = WorkProgress::new(observer);
            progress.context_poll(context)?;
            let bytes = (bits.div_ceil(8) as usize * 8 + 128).saturating_mul(192);
            crate::numerical::with_math(context, &mut progress, bits, bytes, |math, progress| {
                self.enclosure_in(
                    lower,
                    upper,
                    OperationSolverLimits {
                        iterations: policy.limits().max_iterations,
                        subdivisions: policy.limits().max_subdivision_levels,
                        quantize_inverse: false,
                    },
                    math,
                    progress,
                )
                .map_err(|error| geo_math_error(&error, policy))
            })
        })();
        context.release_workspace(self.retained_bytes)?;
        result
    }
    pub(crate) fn enclosure_in(
        &self,
        lower: &Rat,
        upper: &Rat,
        solver: OperationSolverLimits,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<OperationImageEnclosure, MathError> {
        let panel = self.panel_in(lower, upper, solver, PanelKind::Directional, math, progress)?;
        let (south, north) = exact_bounds(&panel.coordinates[1]);
        if south < Rat::from_i64(-90) || north > Rat::from_i64(90) {
            return Err(MathError::Domain(
                "image curve leaves actual geographic latitude domain",
            ));
        }
        Ok(OperationImageEnclosure {
            longitude: panel.coordinates[0].clone(),
            latitude: panel.coordinates[1].clone(),
            derivative: panel.directional,
        })
    }

    /// Request the smooth Cartesian normal before a final geocentric inverse
    /// chooses angular coordinates. Other chain families retain their original
    /// angular evaluator; they do not synthesize a different mapping here.
    pub(crate) fn geocentric_normal_in(
        &self,
        lower: &Rat,
        upper: &Rat,
        solver: OperationSolverLimits,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Option<super::OperationNormalEnclosure>, MathError> {
        if !matches!(
            self.chain
                .operations()
                .last()
                .expect("compiled chain")
                .model(),
            super::super::OperationModel::GeocentricToGeographic { .. }
        ) {
            return Ok(None);
        }
        self.panel_in(lower, upper, solver, PanelKind::Normal, math, progress)
            .map(|panel| panel.normal)
    }

    fn panel_in(
        &self,
        lower: &Rat,
        upper: &Rat,
        solver: OperationSolverLimits,
        kind: PanelKind,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<super::PanelEnclosure, MathError> {
        self.panel_through_in(
            lower,
            upper,
            (solver, kind, self.chain.operations().len()),
            math,
            progress,
        )
    }

    pub(super) fn panel_through_in(
        &self,
        lower: &Rat,
        upper: &Rat,
        request: (OperationSolverLimits, PanelKind, usize),
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<super::PanelEnclosure, MathError> {
        let outside = crate::numerical::math_exact_rationals(
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            &[lower, upper],
            3,
            math,
            progress,
            || lower < &Rat::zero() || upper > &Rat::one() || lower > upper,
        )?;
        if outside {
            return Err(MathError::Domain("image curve parameter outside [0,1]"));
        }
        let (start, end) = self.source_endpoints();
        let a = SourceLinearEdge::interpolate_coord_math(start, end, lower, math, progress)?;
        let b = SourceLinearEdge::interpolate_coord_math(start, end, upper, math, progress)?;
        let delta = source_delta(start, end, math, progress)?;
        super::evaluate_panel_through(
            &self.chain,
            self.epoch.as_ref(),
            (&a, &b),
            Some(&delta),
            request,
            math,
            progress,
        )
    }
    pub(crate) fn speed_in(
        &self,
        lower: &Rat,
        upper: &Rat,
        solver: OperationSolverLimits,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<FixedInterval, MathError> {
        let image = self.enclosure_in(lower, upper, solver, math, progress)?;
        let derivative = image.derivative.ok_or(MathError::PrecisionExhausted)?;
        let factor = FixedInterval::pi(math)?.div(&FixedInterval::from_i64(180, math)?, math)?;
        let phi = image.latitude.mul(&factor, math)?;
        let (sine, cosine) = phi.sin_cos_range(math)?;
        let one = FixedInterval::from_i64(1, math)?;
        let ellipsoid = self.reference.ellipsoid();
        let e2 = fixed_from_rat(&ellipsoid.eccentricity_squared(), math)?;
        let w2 = one.sub(&e2.mul(&sine.square(math)?, math)?, math)?;
        let n = fixed_from_rat(ellipsoid.semimajor(), math)?.div(&w2.sqrt(math)?, math)?;
        let m = n.mul(&one.sub(&e2, math)?, math)?.div(&w2, math)?;
        let north = m.mul(&derivative[1].mul(&factor, math)?, math)?;
        let east = n
            .mul(&cosine, math)?
            .mul(&derivative[0].mul(&factor, math)?, math)?;
        north
            .square(math)?
            .add(&east.square(math)?, math)?
            .sqrt(math)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::{
        CoordinateOperation, CoordinateUnit, HelmertParameters, HelmertRates, OperationModel,
        OperationReference, RotationConvention, RotationLaw,
    };
    use purrdf_xsd::math::MathLimits;

    #[test]
    fn original_xyz_curve_retains_a_smooth_normal_through_both_polar_axes() {
        for sign in [-1, 1] {
            let reference = GeographicReference::wgs84();
            let operation = CoordinateOperation::compile(
                OperationReference {
                    realization: Digest32::new([94; 32]),
                    unit: CoordinateUnit::Metres,
                    swapped_axes: false,
                },
                OperationReference {
                    realization: reference.id().digest(),
                    unit: CoordinateUnit::Degrees,
                    swapped_axes: false,
                },
                OperationModel::GeocentricToGeographic {
                    ellipsoid: reference.ellipsoid().clone(),
                },
            )
            .unwrap();
            let chain = Arc::new(OperationChain::compile(vec![operation]).unwrap());
            let z = reference
                .ellipsoid()
                .semiminor()
                .add(&Rat::from_i64(12))
                .mul(&Rat::from_i64(sign));
            let curve = OperationImageCurve::new(
                Coord::new(Rat::from_i64(-1), Rat::zero(), Some(z.clone()), None),
                Coord::new(Rat::one(), Rat::zero(), Some(z), None),
                chain,
                reference,
                None,
                ExecutionPolicy::geometry(),
            )
            .unwrap();
            let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
            let limits = OperationSolverLimits {
                iterations: 128,
                subdivisions: 64,
                quantize_inverse: false,
            };
            let (normal, derivative) = curve
                .geocentric_normal_in(
                    &Rat::zero(),
                    &Rat::one(),
                    limits,
                    &mut math,
                    &mut WorkProgress::new(None),
                )
                .unwrap()
                .unwrap();
            assert!(normal[0].lower().is_negative());
            assert!(!normal[0].upper().is_negative());
            assert!(normal[1].is_exact_zero());
            assert!(derivative.unwrap()[0].lower() > &purrdf_xsd::BigInt::zero());
            let middle = Rat::one().div(&Rat::from_i64(2)).unwrap();
            let (normal, _) = curve
                .geocentric_normal_in(
                    &middle,
                    &middle,
                    limits,
                    &mut math,
                    &mut WorkProgress::new(None),
                )
                .unwrap()
                .unwrap();
            assert!(normal[..2].iter().all(FixedInterval::is_exact_zero));
            assert_eq!(
                exact_bounds(&normal[2]),
                (Rat::from_i64(sign), Rat::from_i64(sign))
            );
            let mut context = MetricContext::wgs84().unwrap();
            let edge = crate::PreparedEdge::Transformed(Box::new(curve));
            let pole = crate::LonLat::new(Rat::zero(), Rat::from_i64(sign * 90)).unwrap();
            assert!(crate::atlas::point::contact(&edge, &pole, &mut context).unwrap());
        }
    }

    fn curve(policy: ExecutionPolicy) -> OperationImageCurve {
        let reference = GeographicReference::wgs84();
        let operation = CoordinateOperation::compile(
            OperationReference {
                realization: Digest32::new([7; 32]),
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
        OperationImageCurve::new(
            Coord::xy(Rat::from_i64(-1_000_000), Rat::from_i64(-1_000_000)),
            Coord::xy(Rat::from_i64(1_000_000), Rat::from_i64(1_000_000)),
            Arc::new(OperationChain::compile(vec![operation]).unwrap()),
            reference,
            None,
            policy,
        )
        .unwrap()
    }

    #[test]
    fn inverse_family_encloses_original_curve_without_scalar_quantization() {
        let curve = curve(ExecutionPolicy::geometry());
        let mut context = MetricContext::wgs84().unwrap();
        let panel = curve
            .enclosure(&Rat::zero(), &Rat::one(), 96, &mut context)
            .unwrap();
        let (lo, hi) = exact_bounds(&panel.latitude);
        assert!(lo < Rat::from_i64(-8));
        assert!(hi > Rat::from_i64(8));
        let derivative = panel.derivative.expect("smooth original inverse");
        assert!(derivative.iter().all(|value| !value.lower().is_negative()));
        let half = Rat::one().div(&Rat::from_i64(2)).unwrap();
        let center = curve.enclosure(&half, &half, 128, &mut context).unwrap();
        assert!(center.longitude.lower().is_zero() && center.longitude.upper().is_zero());
        assert!(center.latitude.lower().is_zero() && center.latitude.upper().is_zero());
        let first = curve
            .enclosure(&Rat::zero(), &half, 96, &mut context)
            .unwrap();
        let second = curve
            .enclosure(&half, &Rat::one(), 96, &mut context)
            .unwrap();
        let whole_width = panel
            .latitude
            .width(
                &mut CoordinateMath::new(MathLimits {
                    precision_bits: 96,
                    max_work: 262_144,
                    max_workspace_bytes: 64 * 1024 * 1024,
                })
                .unwrap(),
            )
            .unwrap();
        assert!(first.latitude.upper().sub(first.latitude.lower()) < *whole_width.upper());
        assert!(second.latitude.upper().sub(second.latitude.lower()) < *whole_width.upper());
    }

    #[test]
    fn symbolic_mapping_identity_and_cancelled_panels_preserve_original_source() {
        let a = curve(ExecutionPolicy::geometry());
        let mut limits = crate::ExecutionLimits::GEOMETRY;
        limits.max_work_items *= 2;
        let b = curve(ExecutionPolicy::new(limits).unwrap());
        assert_eq!(a.id(), b.id());
        assert!(a.same_mapping(&b));
        struct Cancel;
        impl MetricWorkObserver for Cancel {
            fn charge_chunk(&mut self, _: u64, _: u64) -> Result<(), GeoError> {
                Err(GeoError::Cancelled)
            }
        }
        let mut context = MetricContext::wgs84().unwrap();
        assert!(matches!(
            a.enclosure_metered(&Rat::zero(), &Rat::one(), 96, &mut context, &mut Cancel),
            Err(GeoError::Cancelled)
        ));
        assert!(matches!(
            a.enclosure(&Rat::from_i64(-1), &Rat::one(), 96, &mut context),
            Err(GeoError::Domain(_))
        ));
    }

    fn height_epoch_chain(law: RotationLaw, convention: RotationConvention) -> Arc<OperationChain> {
        let geographic = GeographicReference::wgs84();
        let reference = |id, unit| OperationReference {
            realization: id,
            unit,
            swapped_axes: false,
        };
        let angular = reference(geographic.id().digest(), CoordinateUnit::Degrees);
        let cartesian = reference(Digest32::new([31; 32]), CoordinateUnit::Metres);
        let shifted = reference(Digest32::new([32; 32]), CoordinateUnit::Metres);
        let rat = |value| Rat::parse_decimal(value).unwrap();
        let helmert = HelmertParameters {
            translation: [rat("1"), rat("-2"), rat("3")],
            rotation_arcseconds: [rat("10"), rat("-20"), rat("30")],
            scale_ppm: rat("2"),
            pivot: [rat("500"), rat("600"), rat("700")],
            law,
            convention,
            rates: Some(HelmertRates {
                epoch: rat("2020"),
                translation: [rat("0.1"), rat("0.2"), rat("0.3")],
                rotation_arcseconds: [rat("0.01"), rat("-0.02"), rat("0.03")],
                scale_ppm: rat("0.1"),
            }),
            inverse: false,
        };
        let mut inverse = helmert.clone();
        inverse.inverse = true;
        Arc::new(
            OperationChain::compile(vec![
                CoordinateOperation::compile(
                    angular,
                    cartesian,
                    OperationModel::GeographicToGeocentric {
                        ellipsoid: geographic.ellipsoid().clone(),
                    },
                )
                .unwrap(),
                CoordinateOperation::compile(
                    cartesian,
                    shifted,
                    OperationModel::Helmert(Box::new(helmert)),
                )
                .unwrap(),
                CoordinateOperation::compile(
                    shifted,
                    cartesian,
                    OperationModel::Helmert(Box::new(inverse)),
                )
                .unwrap(),
                CoordinateOperation::compile(
                    cartesian,
                    angular,
                    OperationModel::GeocentricToGeographic {
                        ellipsoid: geographic.ellipsoid().clone(),
                    },
                )
                .unwrap(),
            ])
            .unwrap(),
        )
    }

    #[test]
    fn actual_height_and_epoch_differential_survives_the_calibrated_affine_inverse() {
        let rat = |value| Rat::parse_decimal(value).unwrap();
        for law in [
            RotationLaw::HelmertSmallAngleV1,
            RotationLaw::HelmertEulerRzRyRxV1,
        ] {
            for convention in [
                RotationConvention::PositionVector,
                RotationConvention::CoordinateFrame,
            ] {
                let start = Coord::new(rat("117.7"), rat("35"), Some(rat("10")), Some(rat("7")));
                let end = Coord::new(
                    rat("117.700001"),
                    rat("35.000002"),
                    Some(rat("12")),
                    Some(rat("9")),
                );
                let curve = OperationImageCurve::new(
                    start,
                    end,
                    height_epoch_chain(law, convention),
                    GeographicReference::wgs84(),
                    Some(rat("2025.25")),
                    ExecutionPolicy::geometry(),
                )
                .unwrap();
                let mut context = MetricContext::wgs84().unwrap();
                let half = rat("0.5");
                let image = curve.enclosure(&half, &half, 96, &mut context).unwrap();
                for (value, expected) in [
                    (&image.longitude, rat("117.7000005")),
                    (&image.latitude, rat("35.000001")),
                ] {
                    let (lower, upper) = exact_bounds(value);
                    assert!(lower <= expected && expected <= upper);
                    assert!(upper.sub(&lower) < rat("0.000000000001"));
                }
                let derivative = image.derivative.unwrap();
                for (value, expected) in derivative.iter().zip([rat("0.000001"), rat("0.000002")]) {
                    let (lower, upper) = exact_bounds(value);
                    assert!(lower <= expected && expected <= upper);
                    assert!(upper.sub(&lower) < rat("0.000000000001"));
                }
                assert!(
                    context.work_items() <= ExecutionPolicy::geometry().limits().max_work_items
                );
            }
        }
    }

    #[test]
    fn image_preparation_refuses_missing_actual_height_and_observation_epoch() {
        let chain = height_epoch_chain(
            RotationLaw::HelmertSmallAngleV1,
            RotationConvention::PositionVector,
        );
        let start = Coord::xy(Rat::from_i64(117), Rat::from_i64(35));
        assert_eq!(
            OperationImageCurve::new(
                start.clone(),
                start,
                chain.clone(),
                GeographicReference::wgs84(),
                Some(Rat::from_i64(2025)),
                ExecutionPolicy::geometry(),
            ),
            Err(GeoError::MissingHeight),
        );
        let start = Coord::new(
            Rat::from_i64(117),
            Rat::from_i64(35),
            Some(Rat::from_i64(10)),
            None,
        );
        assert!(matches!(
            OperationImageCurve::new(
                start.clone(),
                start,
                chain,
                GeographicReference::wgs84(),
                None,
                ExecutionPolicy::geometry()
            ),
            Err(GeoError::Domain(_)),
        ));
    }

    #[test]
    fn admitted_native_tm_pair_preserves_the_complete_original_ground_curve() {
        use crate::operation::{Hemisphere, TransverseMercator, ZoneFamily};
        use crate::{AxisOrder, PreparedCurve, PreparedEdge, PreparedGeometry, PreparedRegion};
        for axes in [AxisOrder::LonLat, AxisOrder::LatLon] {
            let reference = GeographicReference::cgcs2000().with_axes(axes);
            let angular = OperationReference {
                realization: reference.id().digest(),
                unit: CoordinateUnit::Degrees,
                swapped_axes: axes == AxisOrder::LatLon,
            };
            let projected = OperationReference {
                realization: Digest32::new([69; 32]),
                unit: CoordinateUnit::Metres,
                swapped_axes: true,
            };
            let parameters = TransverseMercator {
                ellipsoid: reference.ellipsoid().clone(),
                family: ZoneFamily::GaussKruger3,
                zone: 39,
                central_meridian: Rat::from_i64(117),
                scale: Rat::one(),
                false_easting: Rat::from_i64(500_000),
                false_northing: Rat::zero(),
                hemisphere: Hemisphere::North,
                zone_prefix: true,
            };
            let forward = CoordinateOperation::compile(
                angular,
                projected,
                OperationModel::TransverseMercator(Box::new(parameters)),
            )
            .unwrap();
            let inverse = forward.inverse().unwrap();
            let chain = Arc::new(OperationChain::compile(vec![forward, inverse]).unwrap());
            let start = crate::LonLat::new(
                Rat::parse_decimal("117.125").unwrap(),
                Rat::parse_decimal("35.25").unwrap(),
            )
            .unwrap();
            let end = crate::LonLat::new(
                Rat::parse_decimal("117.25").unwrap(),
                Rat::parse_decimal("35.375").unwrap(),
            )
            .unwrap();
            let coordinate = |point: &crate::LonLat| {
                if axes == AxisOrder::LatLon {
                    Coord::xy(point.latitude().clone(), point.longitude().clone())
                } else {
                    Coord::xy(point.longitude().clone(), point.latitude().clone())
                }
            };
            let policy = ExecutionPolicy::geometry();
            let curve = OperationImageCurve::new(
                coordinate(&start),
                coordinate(&end),
                chain.clone(),
                reference.clone(),
                None,
                policy,
            )
            .unwrap();
            assert!(curve.exact_source_linear);
            let mut context = MetricContext::new(reference.clone(), policy).unwrap();
            let panel = curve
                .enclosure(&Rat::zero(), &Rat::one(), 96, &mut context)
                .unwrap();
            assert_eq!(
                exact_bounds(&panel.longitude),
                (start.longitude().clone(), end.longitude().clone())
            );
            assert_eq!(
                exact_bounds(&panel.latitude),
                (start.latitude().clone(), end.latitude().clone())
            );
            let geometry = PreparedGeometry::from_parts(
                reference,
                Vec::new(),
                vec![PreparedCurve::new(vec![PreparedEdge::Transformed(
                    Box::new(curve),
                )])],
                PreparedRegion::Empty,
                policy,
            )
            .unwrap();
            let ground = crate::ellipsoidal::length(&geometry, &mut context).unwrap();
            let ground_work = context.work_items();
            let original =
                crate::ellipsoidal::source_linear_length(&start, &end, &mut context).unwrap();
            assert_eq!(ground.exact(), original.exact());
            assert!(ground_work <= policy.limits().max_work_items);
            assert!(matches!(
                OperationImageCurve::new(
                    coordinate(&crate::LonLat::new(Rat::from_i64(119), Rat::from_i64(35)).unwrap()),
                    coordinate(&end),
                    chain,
                    geometry.reference().clone(),
                    None,
                    policy,
                ),
                Err(GeoError::Domain(_))
            ));
        }
    }
}
