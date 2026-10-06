// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit, offline coordinate operations over exact source coordinates.
//!
//! A source and target identify actual reference realizations. An ellipsoid alone
//! never establishes a datum transformation. Models contain caller declarations;
//! nothing infers a zone, applicability rectangle, epoch, height or shift bundle.

mod admission;
mod affine;
mod forward;
mod gcj_inverse;
mod geometry;
pub(crate) use geometry::AdmittedImageSource;
mod grid;
mod inverse;
mod models;
mod models_inverse;
#[cfg(test)]
mod source_admission_tests;
mod transverse;
mod transverse_inverse;

pub use affine::{HelmertParameters, HelmertRates, RotationConvention, RotationLaw, Similarity2d};
pub(crate) use geometry::{AreaCellCertificate, ExactAreaCell};
pub use geometry::{
    GeometryImage, ImageMetric, OperationImageCurve, OperationImageEnclosure, OperationImagePoint,
};
pub use grid::TransformOutputGrid;
pub use models::{BilinearGrid, GridNode, Polynomial2d, PolynomialTerm};
pub use models_inverse::{BilinearGridInverse, MetricSourceDomain, Polynomial2dInverse};
pub use transverse::{Hemisphere, ProjectionDifferential, TransverseMercator, ZoneFamily};

use std::sync::Arc;

use purrdf_hash::{Domain, hex::Digest32};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

use crate::context::WorkProgress;
use crate::numerical::{fixed_from_rat, geo_math_error};
use crate::profile::hash_fields;
use crate::{
    GeoError, LonLat, MetricContext, MetricWorkObserver, PreparedEllipsoid, Rat, SemanticLawId,
};

// Coordinate operations have at most three numerical axes. Keep every lane
// inline while the variable-length operation chain remains separately owned.
pub(crate) type OperationCoordinates = purrdf_core::SmallVec<[FixedInterval; 3]>;
type QuantizedCoordinates = purrdf_core::SmallVec<[Rat; 3]>;

const OPERATION_BINDING: Domain = Domain::new(b"purrdf-geo-kernel/coordinate-operation/v1");
const OPERATION_LAW: Domain = Domain::new(b"purrdf-geo-kernel/coordinate-operation-law/v1");
const OPERATION_CHAIN: Domain = Domain::new(b"purrdf-geo-kernel/operation-chain/v1");
const TRANSVERSE_PARAMETER: Domain =
    Domain::new(b"purrdf-geo-kernel/transverse-mercator-parameters/v1");
const TRANSFORM_CERTIFICATE: Domain = Domain::new(b"purrdf-geo-kernel/transform-certificate/v1");

// Public entry boilerplate is generated once for both pure model shapes;
// the bounded application body is shared rather than reimplemented per facade.
macro_rules! coordinate_application {
    ($type:ty) => {
        impl $type {
            /// Transform exact input onto the fixed 15-place angular/6-place metre grid.
            /// # Errors
            /// Refuses domain/metadata gaps, unresolved output and resource exhaustion.
            pub fn apply(
                &self,
                point: &OperationPoint,
                context: &mut MetricContext,
            ) -> Result<TransformResult, GeoError> {
                self.apply_with_grid(point, TransformOutputGrid::DEFAULT, context)
            }
            /// Transform with bounded borrowed work and cancellation callbacks.
            /// # Errors
            /// Propagates every refusal without publishing partial coordinates.
            pub fn apply_metered(
                &self,
                point: &OperationPoint,
                context: &mut MetricContext,
                observer: &mut dyn MetricWorkObserver,
            ) -> Result<TransformResult, GeoError> {
                self.apply_with_grid_metered(point, TransformOutputGrid::DEFAULT, context, observer)
            }
            /// Transform to explicit correctly half-even rounded output quanta.
            /// # Errors
            /// Refuses any grid outside precision or the one-micrometre quantization certificate.
            pub fn apply_with_grid(
                &self,
                point: &OperationPoint,
                grid: TransformOutputGrid,
                context: &mut MetricContext,
            ) -> Result<TransformResult, GeoError> {
                apply_entry(point, context, None, |point, context, progress| {
                    self.apply_inner(point, grid, context, progress)
                })
            }
            /// Apply explicit output quanta with bounded work and cancellation callbacks.
            /// # Errors
            /// Carries the complete scalar refusal contract and observer errors.
            pub fn apply_with_grid_metered(
                &self,
                point: &OperationPoint,
                grid: TransformOutputGrid,
                context: &mut MetricContext,
                observer: &mut dyn MetricWorkObserver,
            ) -> Result<TransformResult, GeoError> {
                apply_entry(
                    point,
                    context,
                    Some(observer),
                    |point, context, progress| self.apply_inner(point, grid, context, progress),
                )
            }
            /// Transform a caller-buffer batch with one entry validation.
            /// # Errors
            /// Every refusal clears all slots, including earlier completed results.
            pub fn apply_batch(
                &self,
                points: &[OperationPoint],
                output: &mut [Option<TransformResult>],
                context: &mut MetricContext,
            ) -> Result<(), GeoError> {
                self.apply_batch_with_grid(points, output, TransformOutputGrid::DEFAULT, context)
            }
            /// Transform a batch with one borrowed work and cancellation observer.
            /// # Errors
            /// Every refusal clears all slots, including earlier completed results.
            pub fn apply_batch_metered(
                &self,
                points: &[OperationPoint],
                output: &mut [Option<TransformResult>],
                context: &mut MetricContext,
                observer: &mut dyn MetricWorkObserver,
            ) -> Result<(), GeoError> {
                self.apply_batch_with_grid_metered(
                    points,
                    output,
                    TransformOutputGrid::DEFAULT,
                    context,
                    observer,
                )
            }
            /// Transform every original point to the same declared output grid.
            /// # Errors
            /// Refuses the complete batch and clears every output slot on failure.
            pub fn apply_batch_with_grid(
                &self,
                points: &[OperationPoint],
                output: &mut [Option<TransformResult>],
                grid: TransformOutputGrid,
                context: &mut MetricContext,
            ) -> Result<(), GeoError> {
                apply_batch_entry(points, output, context, None, |point, context, progress| {
                    self.apply_inner(point, grid, context, progress)
                })
            }
            /// Transform an explicit-grid batch with one borrowed observer.
            /// # Errors
            /// Carries the scalar grid certificate and complete batch refusal contract.
            pub fn apply_batch_with_grid_metered(
                &self,
                points: &[OperationPoint],
                output: &mut [Option<TransformResult>],
                grid: TransformOutputGrid,
                context: &mut MetricContext,
                observer: &mut dyn MetricWorkObserver,
            ) -> Result<(), GeoError> {
                apply_batch_entry(
                    points,
                    output,
                    context,
                    Some(observer),
                    |point, context, progress| self.apply_inner(point, grid, context, progress),
                )
            }
        }
    };
}
coordinate_application!(CoordinateOperation);
coordinate_application!(OperationChain);

fn apply_entry(
    point: &OperationPoint,
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
    apply: impl FnOnce(
        &OperationPoint,
        &mut MetricContext,
        &mut WorkProgress<'_>,
    ) -> Result<TransformResult, GeoError>,
) -> Result<TransformResult, GeoError> {
    context.begin(1)?;
    let mut progress = WorkProgress::new(observer);
    progress.initial()?;
    with_application_source(point, context, &mut progress, apply)
}

fn with_application_source<T>(
    point: &OperationPoint,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    apply: impl FnOnce(
        &OperationPoint,
        &mut MetricContext,
        &mut WorkProgress<'_>,
    ) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    let bytes = point_retained_bytes(point)?;
    context.admit_workspace(bytes)?;
    let result = progress
        .context_poll(context)
        .and_then(|()| apply(point, context, progress));
    context.release_workspace(bytes)?;
    result.and_then(|value| progress.context_poll(context).map(|()| value))
}

fn point_operand_bits(point: &OperationPoint) -> u64 {
    [
        Some(&point.x),
        Some(&point.y),
        point.z.as_ref(),
        point.epoch.as_ref(),
    ]
    .into_iter()
    .flatten()
    .map(crate::numerical::rational_operand_bits)
    .max()
    .unwrap_or(0)
}

fn point_retained_bytes(point: &OperationPoint) -> Result<u64, GeoError> {
    [
        Some(&point.x),
        Some(&point.y),
        point.z.as_ref(),
        point.epoch.as_ref(),
    ]
    .into_iter()
    .flatten()
    .try_fold(size_of::<OperationPoint>() as u64, |bytes, value| {
        let limbs = value
            .numerator()
            .allocated_bytes()
            .checked_add(value.denominator().allocated_bytes())
            .and_then(|bytes| u64::try_from(bytes).ok())
            .ok_or(GeoError::ArithmeticOverflow("operation point storage"))?;
        bytes
            .checked_add(limbs)
            .ok_or(GeoError::ArithmeticOverflow("operation point storage"))
    })
}

fn apply_batch_entry(
    points: &[OperationPoint],
    output: &mut [Option<TransformResult>],
    context: &mut MetricContext,
    observer: Option<&mut dyn MetricWorkObserver>,
    mut apply: impl FnMut(
        &OperationPoint,
        &mut MetricContext,
        &mut WorkProgress<'_>,
    ) -> Result<TransformResult, GeoError>,
) -> Result<(), GeoError> {
    output.fill(None);
    if points.len() != output.len() {
        return Err(GeoError::InvalidOutputLength {
            expected: points.len(),
            actual: output.len(),
        });
    }
    context.begin(points.len())?;
    let mut progress = WorkProgress::new(observer);
    progress.initial()?;
    let mut retained_outputs = 0_u64;
    let result = (|| {
        for (point, slot) in points.iter().zip(output.iter_mut()) {
            let result = with_application_source(point, context, &mut progress, &mut apply)?;
            let bytes = point_retained_bytes(result.point())?
                .checked_add((size_of::<TransformResult>() - size_of::<OperationPoint>()) as u64)
                .ok_or(GeoError::ArithmeticOverflow(
                    "operation batch output storage",
                ))?;
            let next = retained_outputs
                .checked_add(bytes)
                .ok_or(GeoError::ArithmeticOverflow(
                    "operation batch output storage",
                ))?;
            context.admit_workspace(bytes)?;
            retained_outputs = next;
            *slot = Some(result);
            progress.context_poll(context)?;
        }
        Ok(())
    })();
    let result = result.and_then(|()| progress.context_poll(context));
    if result.is_err() {
        output.fill(None);
    }
    context.release_workspace(retained_outputs)?;
    result
}

/// Physical coordinate units of a declared operation reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CoordinateUnit {
    /// Geographic or provider longitude/latitude in degrees.
    Degrees,
    /// Cartesian easting/northing or geocentric XYZ in metres.
    Metres,
}

impl CoordinateUnit {
    /// The frozen scalar output grid: angular XY uses fifteen decimal places;
    /// metres and any height ordinate use six. Proof refinement never changes it.
    #[must_use]
    pub const fn output_decimal_places(self, ordinate: usize) -> u32 {
        if matches!(self, Self::Degrees) && ordinate < 2 {
            15
        } else {
            6
        }
    }
}

/// A reference realization and explicit coordinate axes at an operation boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OperationReference {
    /// Content identity supplied by the reference's owner.
    pub realization: Digest32,
    /// Degrees or metres; height is always metres.
    pub unit: CoordinateUnit,
    /// Swap the first two source/target axes when true.
    pub swapped_axes: bool,
}

/// Closed, exact rectangle supplied as a model's admitted applicability.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Applicability {
    /// Inclusive first-coordinate lower bound.
    pub west: Rat,
    /// Inclusive first-coordinate upper bound.
    pub east: Rat,
    /// Inclusive second-coordinate lower bound.
    pub south: Rat,
    /// Inclusive second-coordinate upper bound.
    pub north: Rat,
}

impl Applicability {
    /// Declare an exact, non-wrapping rectangle. There is no default rectangle.
    ///
    /// # Errors
    /// Refuses reversed or empty intervals.
    pub fn new(west: Rat, east: Rat, south: Rat, north: Rat) -> Result<Self, GeoError> {
        Self::new_in_budget(
            west,
            east,
            south,
            north,
            &mut crate::PreparationBudget::new(crate::ExecutionPolicy::geometry()),
        )
    }

    /// Validate a declared applicability rectangle within cumulative configuration work.
    /// # Errors
    /// Refuses invalid extents and exact arithmetic limits before comparison.
    pub fn new_in_budget(
        west: Rat,
        east: Rat,
        south: Rat,
        north: Rat,
        budget: &mut crate::PreparationBudget,
    ) -> Result<Self, GeoError> {
        let cost = crate::numerical::rational_cost(
            purrdf_xsd::integer::ExactOperation::RationalCompare,
            &[&west, &east, &south, &north],
            2,
        )
        .ok_or(GeoError::ArithmeticOverflow("configuration rational work"))?;
        budget.exact(cost, || Self::new_admitted(west, east, south, north))
    }

    fn new_admitted(west: Rat, east: Rat, south: Rat, north: Rat) -> Result<Self, GeoError> {
        if west >= east || south >= north {
            return Err(GeoError::config(
                "operation applicability must have positive extents",
            ));
        }
        Ok(Self {
            west,
            east,
            south,
            north,
        })
    }

    /// Whether an exact source point is in the declared closed applicability.
    #[must_use]
    pub fn contains(&self, x: &Rat, y: &Rat) -> bool {
        x >= &self.west && x <= &self.east && y >= &self.south && y <= &self.north
    }
}

/// Exact operation input, preserving a real height and explicit decimal-year epoch.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OperationPoint {
    /// First declared coordinate axis.
    pub x: Rat,
    /// Second declared coordinate axis.
    pub y: Rat,
    /// Actual height or third Cartesian coordinate, in metres.
    pub z: Option<Rat>,
    /// Explicit observation epoch as an exact decimal year.
    pub epoch: Option<Rat>,
}

impl OperationPoint {
    /// Geographic XY input without inventing height or observation time.
    #[must_use]
    pub fn from_lon_lat(point: &LonLat) -> Self {
        Self {
            x: point.longitude().clone(),
            y: point.latitude().clone(),
            z: None,
            epoch: None,
        }
    }
}

/// Original mathematical model; constants in interoperability laws are exact decimals.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OperationModel {
    /// Published rational/harmonic GCJ interoperability equations. Applicability is mandatory.
    GcjRationalHarmonicV1(Applicability),
    /// Globally isolated inverse on the caller-declared original source domain.
    GcjRationalHarmonicInverseV1(Applicability),
    /// Published Cartesian BD09LL interoperability law over GCJ coordinates.
    Bd09LlV1,
    /// Certified unique inverse of the published Cartesian BD09LL law.
    Bd09LlInverseV1,
    /// Analytic ellipsoidal Mercator with the declared provider-correspondence law.
    BaiduMercatorAnalyticV1,
    /// Explicit ellipsoidal Mercator without an inferred provider or datum.
    EllipsoidalMercator {
        /// Positive semimajor axis in metres.
        semimajor: Rat,
        /// Exact squared eccentricity in `[0,1)`.
        eccentricity_squared: Rat,
    },
    /// Spherical Web Mercator on its square domain, with an explicit metre radius.
    WebMercator {
        /// Explicit positive spherical radius in metres.
        radius: Rat,
    },
    /// Geographic degrees/actual metre height to geocentric XYZ.
    GeographicToGeocentric {
        /// Explicit source ellipsoid; it does not select a datum transformation.
        ellipsoid: PreparedEllipsoid,
    },
    /// Inverse spherical or ellipsoidal Mercator with a certified forward residual.
    MercatorToGeographic {
        /// Explicit positive metre radius/semimajor axis.
        radius: Rat,
        /// Exact squared eccentricity in `[0,1)`.
        eccentricity_squared: Rat,
        /// Enforce the Web Mercator square when true.
        square_domain: bool,
    },
    /// Geocentric XYZ to geographic degrees and actual metre height.
    GeocentricToGeographic {
        /// Explicit target ellipsoid, without a datum operation.
        ellipsoid: PreparedEllipsoid,
    },
    /// Native explicitly zoned Gauss–Krüger or UTM through original certified continuation.
    TransverseMercator(Box<TransverseMercator>),
    /// Complete certified inverse of an explicitly declared native zone.
    TransverseMercatorInverse(Box<TransverseMercator>),
    /// Caller-calibrated three-dimensional affine realization change.
    Helmert(Box<HelmertParameters>),
    /// Caller-calibrated two-dimensional rotation/scale/translation.
    Similarity2d(Similarity2d),
    /// Explicit monomial basis, normalization and ordered caller coefficients.
    Polynomial2d(Polynomial2d),
    /// In-memory bilinear displacement model with explicit extents and holes.
    BilinearGrid(BilinearGrid),
    /// All-root polynomial inverse on an explicit original metre rectangle.
    Polynomial2dInverse(Box<Polynomial2dInverse>),
    /// All-root inverse on complete closed grid patches in explicit metre bounds.
    BilinearGridInverse(Box<BilinearGridInverse>),
}

// These are admission limits, never operation or output-law parameters.
#[derive(Clone, Copy)]
pub(super) struct OperationSolverLimits {
    pub(super) iterations: u32,
    pub(super) subdivisions: u32,
    pub(super) quantize_inverse: bool,
}

/// Compiled immutable coordinate operation; no external resource is needed at execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoordinateOperation {
    source: OperationReference,
    target: OperationReference,
    model: Arc<OperationModel>,
    transverse_parameter_id: Option<Digest32>,
    id: Digest32,
    law: SemanticLawId,
    maximum_original_operand_bits: u64,
}

impl CoordinateOperation {
    /// Validate and compile a caller-declared source, target and model.
    ///
    /// # Errors
    /// Refuses inconsistent units, singular parameters and invalid declarations.
    pub fn compile(
        source: OperationReference,
        target: OperationReference,
        model: OperationModel,
    ) -> Result<Self, GeoError> {
        Self::compile_in_budget(
            source,
            target,
            model,
            &mut crate::PreparationBudget::new(crate::ExecutionPolicy::geometry()),
        )
    }

    /// Compile under one cumulative configuration budget before parameter
    /// validation, decimal rendering and content identity construction.
    ///
    /// # Errors
    /// Carries the model declaration refusals and complete work/storage limits.
    pub fn compile_in_budget(
        source: OperationReference,
        target: OperationReference,
        model: OperationModel,
        budget: &mut crate::PreparationBudget,
    ) -> Result<Self, GeoError> {
        let cost = model.admit_compile(budget)?;
        budget.exact(cost, || Self::compile_admitted(source, target, model))
    }

    fn compile_admitted(
        source: OperationReference,
        target: OperationReference,
        model: OperationModel,
    ) -> Result<Self, GeoError> {
        model.validate()?;
        let (input, output) = model.units();
        if source.unit != input || target.unit != output {
            return Err(GeoError::config(
                "operation units disagree with model input/output units",
            ));
        }
        let law = SemanticLawId::from_digest(hash_fields(OPERATION_LAW, [model.descriptor()]));
        let mut fields = vec![
            law.digest().as_bytes().to_vec(),
            source.realization.as_bytes().to_vec(),
            target.realization.as_bytes().to_vec(),
            vec![u8::from(source.swapped_axes), u8::from(target.swapped_axes)],
        ];
        model.parameters(&mut fields);
        // Both directions share the original complete parameter preimage,
        // rendered once under compilation admission. Panel consumers compare
        // this fixed-width fingerprint instead of deeply comparing rational
        // axes, origins and projection parameters on every evaluation.
        let transverse_parameter_id = matches!(
            model,
            OperationModel::TransverseMercator(_) | OperationModel::TransverseMercatorInverse(_)
        )
        .then(|| hash_fields(TRANSVERSE_PARAMETER, fields[4..].iter().map(Vec::as_slice)));
        let id = hash_fields(OPERATION_BINDING, fields.iter().map(Vec::as_slice));
        let mut maximum_original_operand_bits = 0;
        model.visit_original_operands(&mut |value| {
            maximum_original_operand_bits =
                maximum_original_operand_bits.max(crate::numerical::rational_operand_bits(value));
        });
        Ok(Self {
            source,
            target,
            model: Arc::new(model),
            transverse_parameter_id,
            id,
            law,
            maximum_original_operand_bits,
        })
    }

    /// Exact source realization, units and axis order.
    #[must_use]
    pub const fn source(&self) -> OperationReference {
        self.source
    }
    /// Exact target realization, units and axis order.
    #[must_use]
    pub const fn target(&self) -> OperationReference {
        self.target
    }
    /// Model and parameter content identity, independent of admission limits.
    #[must_use]
    pub const fn id(&self) -> Digest32 {
        self.id
    }
    /// Largest original rational magnitude in the complete compiled model,
    /// including inverse domains, projection axes, calibrated rates and grids.
    /// Computed once from integer metadata; it is not a mathematical ID field.
    #[must_use]
    pub const fn max_original_operand_bits(&self) -> u64 {
        self.maximum_original_operand_bits
    }
    /// Completed transform law, independent of parameters and tighter proofs.
    #[must_use]
    pub const fn law_id(&self) -> SemanticLawId {
        self.law
    }
    /// The compiled immutable mathematical model.
    #[must_use]
    pub fn model(&self) -> &OperationModel {
        &self.model
    }

    /// Compile the corresponding inverse with the exact source/target bindings
    /// exchanged. Affine inverses invert the calibrated matrix of the stated law.
    ///
    /// # Errors
    /// Refuses a model without a certified inverse or invalid inverse parameters.
    pub fn inverse(&self) -> Result<Self, GeoError> {
        self.inverse_in_budget(&mut crate::PreparationBudget::new(
            crate::ExecutionPolicy::geometry(),
        ))
    }

    /// Derive the explicitly declared inverse within cumulative configuration
    /// admission, before cloning original parameter bundles or grids.
    /// # Errors
    /// Refuses unsupported inverse domains and exact work/storage exhaustion.
    pub fn inverse_in_budget(
        &self,
        budget: &mut crate::PreparationBudget,
    ) -> Result<Self, GeoError> {
        use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
        budget.retain(
            self.model
                .original_operand_count()
                .ok_or(GeoError::ArithmeticOverflow("inverse parameter metadata"))?,
            0,
        )?;
        let bytes = self.retained_workspace_bytes()?;
        let bits = bytes
            .checked_mul(8)
            .ok_or(GeoError::ArithmeticOverflow("inverse parameter width"))?;
        let mut cost = ExactArithmeticCost::for_operation(ExactOperation::Linear, bits, 8)
            .ok_or(GeoError::ArithmeticOverflow("inverse parameter copy"))?;
        if matches!(self.model(), OperationModel::BaiduMercatorAnalyticV1) {
            cost = cost
                .followed_by(
                    baidu_mercator_parameters_cost()
                        .ok_or(GeoError::ArithmeticOverflow("analytic inverse constants"))?,
                )
                .ok_or(GeoError::ArithmeticOverflow("analytic inverse constants"))?;
        }
        budget.with_workspace(bytes, |phase| {
            let model = phase.exact(cost, || self.inverse_model())?;
            Self::compile_in_budget(self.target, self.source, model, phase)
        })
    }

    fn inverse_model(&self) -> Result<OperationModel, GeoError> {
        let model = match self.model() {
            OperationModel::Bd09LlV1 => OperationModel::Bd09LlInverseV1,
            OperationModel::Bd09LlInverseV1 => OperationModel::Bd09LlV1,
            OperationModel::GcjRationalHarmonicV1(domain) => {
                OperationModel::GcjRationalHarmonicInverseV1(domain.clone())
            }
            OperationModel::GcjRationalHarmonicInverseV1(domain) => {
                OperationModel::GcjRationalHarmonicV1(domain.clone())
            }
            OperationModel::BaiduMercatorAnalyticV1 => {
                let (radius, eccentricity_squared) = baidu_mercator_parameters();
                OperationModel::MercatorToGeographic {
                    eccentricity_squared,
                    radius,
                    square_domain: false,
                }
            }
            OperationModel::WebMercator { radius } => OperationModel::MercatorToGeographic {
                radius: radius.clone(),
                eccentricity_squared: Rat::zero(),
                square_domain: true,
            },
            OperationModel::EllipsoidalMercator {
                semimajor,
                eccentricity_squared,
            } => OperationModel::MercatorToGeographic {
                radius: semimajor.clone(),
                eccentricity_squared: eccentricity_squared.clone(),
                square_domain: false,
            },
            OperationModel::MercatorToGeographic {
                radius,
                eccentricity_squared,
                square_domain,
            } => {
                if *square_domain && eccentricity_squared.is_zero() {
                    OperationModel::WebMercator {
                        radius: radius.clone(),
                    }
                } else {
                    OperationModel::EllipsoidalMercator {
                        semimajor: radius.clone(),
                        eccentricity_squared: eccentricity_squared.clone(),
                    }
                }
            }
            OperationModel::GeographicToGeocentric { ellipsoid } => {
                OperationModel::GeocentricToGeographic {
                    ellipsoid: ellipsoid.clone(),
                }
            }
            OperationModel::GeocentricToGeographic { ellipsoid } => {
                OperationModel::GeographicToGeocentric {
                    ellipsoid: ellipsoid.clone(),
                }
            }
            OperationModel::TransverseMercator(parameters) => {
                OperationModel::TransverseMercatorInverse(parameters.clone())
            }
            OperationModel::TransverseMercatorInverse(parameters) => {
                OperationModel::TransverseMercator(parameters.clone())
            }
            OperationModel::Helmert(parameters) => {
                let mut parameters = parameters.clone();
                parameters.inverse = !parameters.inverse;
                OperationModel::Helmert(parameters)
            }
            OperationModel::Similarity2d(parameters) => {
                let mut parameters = parameters.clone();
                parameters.inverse = !parameters.inverse;
                OperationModel::Similarity2d(parameters)
            }
            _ => {
                return Err(GeoError::config(
                    "model requires an explicitly certified inverse domain",
                ));
            }
        };
        Ok(model)
    }

    fn apply_inner(
        &self,
        point: &OperationPoint,
        grid: TransformOutputGrid,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<TransformResult, GeoError> {
        let input = prepare_source_point(
            &self.model,
            [&point.x, &point.y],
            point.z.as_ref(),
            point.epoch.as_ref(),
            self.source.swapped_axes,
            context,
            progress,
        )?;
        with_application_source(&input, context, progress, |input, context, progress| {
            grid.validate(self.target.unit, point.z.is_some(), context, progress)?;
            let limits = *context.policy().limits();
            let solver_limits = OperationSolverLimits {
                iterations: limits.max_iterations,
                subdivisions: limits.max_subdivision_levels,
                quantize_inverse: grid == TransformOutputGrid::DEFAULT,
            };
            complete_transform(
                TransformTarget {
                    reference: self.target,
                    operation: self.id,
                    law: self.law,
                    grid,
                    original_operand_bits: point_operand_bits(input)
                        .max(self.maximum_original_operand_bits),
                    exact_output: None,
                },
                point.epoch.as_ref(),
                context,
                progress,
                |math, progress| self.model.evaluate(input, solver_limits, math, progress),
                |certification_input, values, math, progress| {
                    self.model
                        .certify_quantized_fixed(certification_input, values, math, progress)
                },
            )
        })
    }
}

/// Fixed-grid scalar transform result with a canonical completed certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransformResult {
    point: OperationPoint,
    operation: Digest32,
    law: SemanticLawId,
    angular: bool,
    grid: TransformOutputGrid,
}

impl TransformResult {
    /// The quantized output, with the supplied observation epoch preserved.
    #[must_use]
    pub const fn point(&self) -> &OperationPoint {
        &self.point
    }

    /// Consume the completed response and move its final exact carrier coordinates.
    #[must_use]
    pub fn into_point(self) -> OperationPoint {
        self.point
    }
    /// Exact model/source/target parameter identity.
    #[must_use]
    pub const fn operation_id(&self) -> Digest32 {
        self.operation
    }
    /// Fixed output-law identity.
    #[must_use]
    pub const fn law_id(&self) -> SemanticLawId {
        self.law
    }
    /// Effective declared output quanta; inactive units retain their defaults.
    #[must_use]
    pub const fn output_grid(&self) -> TransformOutputGrid {
        self.grid
    }
    /// Canonical certificate; proof precision never changes these completed bytes.
    #[must_use]
    pub fn certificate_bytes(&self) -> Vec<u8> {
        let mut fields = vec![
            TRANSFORM_CERTIFICATE.as_bytes().to_vec(),
            self.law.digest().as_bytes().to_vec(),
            self.operation.as_bytes().to_vec(),
            vec![
                u8::from(self.point.z.is_some()),
                u8::from(self.point.epoch.is_some()),
            ],
            if self.angular {
                format!(
                    "degrees:{};metres:{}",
                    self.grid.angular_decimal_places(),
                    self.grid.metric_decimal_places()
                )
                .into_bytes()
            } else {
                format!("metres:{}", self.grid.metric_decimal_places()).into_bytes()
            },
        ];
        for value in [&self.point.x, &self.point.y]
            .into_iter()
            .chain(self.point.z.iter())
            .chain(self.point.epoch.iter())
        {
            rational_fields(value, &mut fields);
        }
        crate::metric::framed_certificate(fields.iter().map(Vec::as_slice))
    }
}

/// A compiled chain with explicit, equal adjacent reference boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationChain {
    operations: Arc<[CoordinateOperation]>,
    id: Digest32,
    law: SemanticLawId,
    maximum_original_operand_bits: u64,
}

impl OperationChain {
    /// Compile a nonempty chain; all adjacent realization/unit/axis bindings must match.
    ///
    /// # Errors
    /// Refuses a gap rather than inserting an identity/datum operation.
    pub fn compile(operations: Vec<CoordinateOperation>) -> Result<Self, GeoError> {
        Self::compile_in_budget(
            operations,
            &mut crate::PreparationBudget::new(crate::ExecutionPolicy::geometry()),
        )
    }

    /// Compile adjacent bindings and their canonical chain identities under
    /// the same original configuration admission as component compilation.
    ///
    /// # Errors
    /// Refuses declaration gaps and work/storage exhaustion before allocating.
    pub fn compile_in_budget(
        operations: Vec<CoordinateOperation>,
        budget: &mut crate::PreparationBudget,
    ) -> Result<Self, GeoError> {
        Self::compile_in_budget_observed(operations, budget, None)
    }

    /// Compile the same immutable chain with observed admission before hashing
    /// and container allocation under the cumulative configuration policy.
    ///
    /// # Errors
    /// Adds the observer's original refusal to [`Self::compile_in_budget`].
    pub fn compile_in_budget_metered(
        operations: Vec<CoordinateOperation>,
        budget: &mut crate::PreparationBudget,
        observer: &mut dyn MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::compile_in_budget_observed(operations, budget, Some(observer))
    }

    fn compile_in_budget_observed(
        operations: Vec<CoordinateOperation>,
        budget: &mut crate::PreparationBudget,
        observer: Option<&mut dyn MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let count = operations.len() as u64;
        budget.retain(count, 0)?;
        let mut cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
            purrdf_xsd::integer::ExactOperation::Linear,
            256,
            count
                .checked_mul(8)
                .ok_or(GeoError::ArithmeticOverflow("chain compilation work"))?,
        )
        .ok_or(GeoError::ArithmeticOverflow("chain compilation work"))?;
        cost.workspace_bytes = cost
            .workspace_bytes
            .checked_add(
                (operations.capacity() as u64)
                    .checked_mul(size_of::<CoordinateOperation>() as u64)
                    .ok_or(GeoError::ArithmeticOverflow("chain compilation storage"))?,
            )
            .and_then(|bytes| {
                bytes.checked_add(count.checked_mul(64 + size_of::<CoordinateOperation>() as u64)?)
            })
            .ok_or(GeoError::ArithmeticOverflow("chain compilation storage"))?;
        if let Some(observer) = observer {
            budget.exact_metered(cost, || Self::compile_admitted(operations), observer)
        } else {
            budget.exact(cost, || Self::compile_admitted(operations))
        }
    }

    fn compile_admitted(operations: Vec<CoordinateOperation>) -> Result<Self, GeoError> {
        if operations.is_empty() {
            return Err(GeoError::config("coordinate operation chain is empty"));
        }
        for pair in operations.windows(2) {
            if pair[0].target != pair[1].source {
                return Err(GeoError::config(
                    "coordinate operation chain has unmatched reference boundaries",
                ));
            }
        }
        let ids: Vec<_> = operations.iter().map(CoordinateOperation::id).collect();
        let id = hash_fields(
            OPERATION_CHAIN,
            ids.iter().map(Digest32::as_bytes).map(<[u8; 32]>::as_slice),
        );
        let laws: Vec<_> = operations
            .iter()
            .map(|operation| operation.law.digest())
            .collect();
        let law = SemanticLawId::from_digest(hash_fields(
            OPERATION_LAW,
            std::iter::once(
                b"continuous-operation-chain;final-angular15-metre6;half-even;certificate=v1"
                    .as_slice(),
            )
            .chain(laws.iter().map(|id| id.as_bytes().as_slice())),
        ));
        let maximum_original_operand_bits = operations
            .iter()
            .map(CoordinateOperation::max_original_operand_bits)
            .max()
            .unwrap_or(0);
        Ok(Self {
            operations: operations.into(),
            id,
            law,
            maximum_original_operand_bits,
        })
    }
    /// Ordered model and binding content identity, independent of admission.
    #[must_use]
    pub const fn id(&self) -> Digest32 {
        self.id
    }
    /// Largest original rational model/domain magnitude in the complete chain.
    /// Caller coordinates and observation epochs are additional source operands.
    #[must_use]
    pub const fn max_original_operand_bits(&self) -> u64 {
        self.maximum_original_operand_bits
    }
    /// Full declared operation-chain binding, suitable for converted-index provenance.
    #[must_use]
    pub const fn binding_id(&self) -> crate::GeoBindingId {
        crate::GeoBindingId::from_digest(self.id)
    }
    /// The explicitly declared chain components.
    #[must_use]
    pub fn operations(&self) -> &[CoordinateOperation] {
        &self.operations
    }

    /// Recognize only a complete original forward-TM / matching inverse-TM
    /// chain, with exact reversed realization, unit and axis bindings.
    /// This metadata witness does not validate a source panel: consumers must
    /// prove its entire original image lies in the declared nonpolar zone and
    /// hemisphere before applying an identity simplification. It never applies
    /// to a materialized projected carrier or to an inverse-first chain.
    #[must_use]
    pub(crate) fn exact_tm_roundtrip(&self) -> Option<&TransverseMercator> {
        let [forward, inverse] = self.operations.as_ref() else {
            return None;
        };
        let OperationModel::TransverseMercator(parameters) = forward.model() else {
            return None;
        };
        if !matches!(
            inverse.model(),
            OperationModel::TransverseMercatorInverse(_)
        ) || forward.transverse_parameter_id != inverse.transverse_parameter_id
            || forward.source != inverse.target
            || forward.target != inverse.source
        {
            return None;
        }
        Some(parameters)
    }

    /// Apply the one scalar engine to a borrowed original carrier coordinate.
    /// Source cloning and metadata are admitted before entering the child phase.
    pub(super) fn apply_source_coordinate_admitted(
        &self,
        coordinate: &crate::Coord,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<TransformResult, GeoError> {
        let mut point = self.prepare_source_coordinate(coordinate, epoch, context, progress)?;
        // The scalar public entry interprets the original source axes. Undo
        // only the helper's permutation by moving the already admitted values.
        if self
            .operations
            .first()
            .expect("compiled chain")
            .source
            .swapped_axes
        {
            std::mem::swap(&mut point.x, &mut point.y);
        }
        let mut child = context.remaining_child()?;
        let result = {
            let mut nested = progress.nested(
                context.work_items(),
                context
                    .policy()
                    .limits()
                    .max_workspace_bytes
                    .saturating_sub(context.remaining_workspace()),
                context.workspace_peak(),
            );
            self.apply_metered(&point, &mut child, &mut nested)
        };
        progress.absorb_child_result(context, &child, result)
    }

    /// Admit a borrowed carrier endpoint's original source and all required
    /// chain metadata before cloning coordinates or adjusting epoch parameters.
    pub(super) fn prepare_source_coordinate(
        &self,
        coordinate: &crate::Coord,
        epoch: Option<&Rat>,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<OperationPoint, GeoError> {
        for operation in self.operations.iter() {
            operation.model.validate_presence(coordinate.z(), epoch)?;
        }
        let first = self
            .operations
            .first()
            .ok_or_else(|| GeoError::config("empty compiled operation chain"))?;
        let input = prepare_source_point(
            &first.model,
            [coordinate.x(), coordinate.y()],
            coordinate.z(),
            epoch,
            first.source.swapped_axes,
            context,
            progress,
        )?;
        self.validate_metadata_admitted(&input, context, progress)?;
        Ok(input)
    }

    pub(super) fn prepare_source_coordinate_math(
        &self,
        coordinate: &crate::Coord,
        epoch: Option<&Rat>,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<OperationPoint, MathError> {
        for operation in self.operations.iter() {
            operation
                .model
                .validate_presence(coordinate.z(), epoch)
                .map_err(|_| MathError::Domain("source image panel lacks actual model metadata"))?;
        }
        let first = self.operations.first().expect("compiled nonempty chain");
        let source = if first.source.swapped_axes {
            [coordinate.y(), coordinate.x()]
        } else {
            [coordinate.x(), coordinate.y()]
        };
        let cost = source_preparation_cost(&first.model, source, coordinate.z(), epoch)
            .ok_or(MathError::WorkExhausted)?;
        math.admit_exact_cost(cost)?;
        progress.math_poll(math)?;
        let point = make_source_point(&first.model, source, coordinate.z(), epoch)
            .map_err(|_| MathError::Domain("source image panel outside declared model domain"))?;
        progress.math_poll(math)?;
        for operation in self.operations.iter() {
            let cost = operation
                .model
                .validation_cost(
                    [&point.x, &point.y],
                    point.z.as_ref(),
                    point.epoch.as_ref(),
                    false,
                )
                .ok_or(MathError::WorkExhausted)?;
            math.admit_exact_cost(cost)?;
            progress.math_poll(math)?;
            operation
                .model
                .validate_metadata(&point)
                .map_err(|_| MathError::Domain("source image panel lacks actual model metadata"))?;
            progress.math_poll(math)?;
        }
        Ok(point)
    }

    pub(super) fn validate_metadata_admitted(
        &self,
        input: &OperationPoint,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        for operation in self.operations.iter() {
            operation
                .model
                .validate_presence(input.z.as_ref(), input.epoch.as_ref())?;
            let cost = operation
                .model
                .validation_cost(
                    [&input.x, &input.y],
                    input.z.as_ref(),
                    input.epoch.as_ref(),
                    false,
                )
                .ok_or(GeoError::ArithmeticOverflow(
                    "operation metadata validation cost",
                ))?;
            progress.exact(context, cost, || operation.model.validate_metadata(input))?;
        }
        Ok(())
    }
    fn apply_inner(
        &self,
        point: &OperationPoint,
        grid: TransformOutputGrid,
        context: &mut MetricContext,
        progress: &mut WorkProgress<'_>,
    ) -> Result<TransformResult, GeoError> {
        for operation in self.operations.iter() {
            operation
                .model
                .validate_presence(point.z.as_ref(), point.epoch.as_ref())?;
        }
        let first = self
            .operations
            .first()
            .ok_or_else(|| GeoError::config("empty compiled operation chain"))?;
        let last = self
            .operations
            .last()
            .ok_or_else(|| GeoError::config("empty compiled operation chain"))?;
        let input = prepare_source_point(
            &first.model,
            [&point.x, &point.y],
            point.z.as_ref(),
            point.epoch.as_ref(),
            first.source.swapped_axes,
            context,
            progress,
        )?;
        with_application_source(&input, context, progress, |input, context, progress| {
            self.validate_metadata_admitted(input, context, progress)?;
            let exact_roundtrip = self.exact_tm_roundtrip().is_some()
                && crate::numerical::exact_rational_counted(
                    Some(&mut crate::numerical::ExactAdmission::new(
                        context, progress,
                    )),
                    purrdf_xsd::integer::ExactOperation::RationalCompare,
                    &[&input.y, &Rat::from_i64(90)],
                    1,
                    || input.y.abs() != Rat::from_i64(90),
                )?;
            // No intermediate coordinate is rounded. Each complete image enclosure
            // becomes the next input; only the final declared carrier is quantized.
            grid.validate(last.target.unit, point.z.is_some(), context, progress)?;
            let limits = *context.policy().limits();
            let solver_limits = OperationSolverLimits {
                iterations: limits.max_iterations,
                subdivisions: limits.max_subdivision_levels,
                quantize_inverse: false,
            };
            complete_transform(
                TransformTarget {
                    reference: last.target,
                    operation: self.id,
                    law: self.law,
                    grid,
                    original_operand_bits: point_operand_bits(input)
                        .max(self.maximum_original_operand_bits),
                    exact_output: exact_roundtrip.then_some(input),
                },
                point.epoch.as_ref(),
                context,
                progress,
                |math, progress| {
                    let mut coordinates: OperationCoordinates = purrdf_core::smallvec![
                        fixed_from_rat(&input.x, math)?,
                        fixed_from_rat(&input.y, math)?,
                    ];
                    if let Some(z) = &input.z {
                        coordinates.push(fixed_from_rat(z, math)?);
                    }
                    if exact_roundtrip {
                        // The compiled complete native TM pair is injective on
                        // its declared nonpolar strip (Re F' > 0). Source
                        // preparation proved this exact original point belongs
                        // to the zone/hemisphere. Preserve the original output
                        // and evaluate F once for the mandatory inverse forward
                        // residual at the final quantized coordinate. A rounded
                        // projected carrier never enters this simplification.
                        let certification_input = first.model.evaluate_fixed(
                            input,
                            &coordinates,
                            true,
                            solver_limits,
                            math,
                            progress,
                        )?;
                        progress.math_poll(math)?;
                        return Ok(TransformEnclosure {
                            output: coordinates,
                            certification_input,
                        });
                    }
                    let mut certification_input = OperationCoordinates::default();
                    for (index, operation) in self.operations.iter().enumerate() {
                        if index + 1 == self.operations.len() {
                            certification_input.clone_from(&coordinates);
                        }
                        // Equal adjacent axis declarations cancel their two swaps.
                        coordinates = operation.model.evaluate_fixed(
                            input,
                            &coordinates,
                            index == 0,
                            solver_limits,
                            math,
                            progress,
                        )?;
                        progress.math_poll(math)?;
                    }
                    Ok(TransformEnclosure {
                        output: coordinates,
                        certification_input,
                    })
                },
                |certification_input, values, math, progress| {
                    last.model
                        .certify_quantized_fixed(certification_input, values, math, progress)
                },
            )
        })
    }
}

#[derive(Clone, Copy)]
struct TransformTarget<'a> {
    reference: OperationReference,
    operation: Digest32,
    law: SemanticLawId,
    grid: TransformOutputGrid,
    original_operand_bits: u64,
    exact_output: Option<&'a OperationPoint>,
}

struct TransformEnclosure {
    output: OperationCoordinates,
    certification_input: OperationCoordinates,
}

fn complete_transform(
    specification: TransformTarget<'_>,
    epoch: Option<&Rat>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    mut evaluate: impl FnMut(
        &mut CoordinateMath,
        &mut WorkProgress<'_>,
    ) -> Result<TransformEnclosure, MathError>,
    mut certify: impl FnMut(
        &[FixedInterval],
        &[Rat],
        &mut CoordinateMath,
        &mut WorkProgress<'_>,
    ) -> Result<(), MathError>,
) -> Result<TransformResult, GeoError> {
    let TransformTarget {
        reference: target,
        operation,
        law,
        grid,
        original_operand_bits,
        exact_output: _,
    } = specification;
    context.checkpoint()?;
    let policy = context.policy();
    let maximum = policy.limits().max_precision_bits;
    let mut precision = maximum.min(96);
    if precision < 16 {
        return Err(GeoError::PrecisionExhausted { bits: maximum });
    }
    context.prepare_integer_scratch_for_observed(original_operand_bits, progress)?;
    loop {
        let retained = (precision as usize)
            .div_ceil(8)
            .saturating_mul(8)
            .saturating_add(128)
            .saturating_mul(192);
        let result = crate::numerical::with_math_for_sources(
            context,
            progress,
            precision,
            retained,
            &[],
            |math, progress| {
                let enclosure =
                    evaluate(math, progress).map_err(|error| geo_math_error(&error, policy))?;
                quantize_transform_enclosure(
                    &enclosure,
                    specification,
                    epoch,
                    math,
                    progress,
                    &mut certify,
                )
                .map_err(|error| geo_math_error(&error, policy))
            },
        );
        match result {
            Ok((mut coordinates, epoch)) => {
                if target.swapped_axes {
                    coordinates.swap(0, 1);
                }
                let grid = grid.normalized(target.unit, coordinates.len() > 2);
                return Ok(TransformResult {
                    point: OperationPoint {
                        x: coordinates.remove(0),
                        y: coordinates.remove(0),
                        z: coordinates.pop(),
                        epoch,
                    },
                    operation,
                    law: grid.law(law),
                    angular: target.unit == CoordinateUnit::Degrees,
                    grid,
                });
            }
            Err(GeoError::PrecisionExhausted { .. }) if precision < maximum => {
                precision = precision.saturating_mul(2).min(maximum);
            }
            Err(error) => return Err(error),
        }
    }
}

fn quantize_transform_enclosure(
    output: &TransformEnclosure,
    target: TransformTarget<'_>,
    epoch: Option<&Rat>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    certify: &mut impl FnMut(
        &[FixedInterval],
        &[Rat],
        &mut CoordinateMath,
        &mut WorkProgress<'_>,
    ) -> Result<(), MathError>,
) -> Result<(QuantizedCoordinates, Option<Rat>), MathError> {
    let vector_bytes = output
        .output
        .len()
        .checked_mul(size_of::<Rat>())
        .and_then(|bytes| bytes.checked_add(size_of::<TransformResult>()))
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(vector_bytes)?;
    let mut values = QuantizedCoordinates::default();
    for (index, interval) in output.output.iter().enumerate() {
        let places = target.grid.decimal_places(target.reference.unit, index);
        let lower = if let Some(source) = target.exact_output {
            let original = match index {
                0 => &source.x,
                1 => &source.y,
                _ => source.z.as_ref().expect("existing source ordinate"),
            };
            crate::numerical::math_round_decimal(original, places, math, progress)?
        } else {
            let (lower, upper) = interval.round_decimal(places, math)?;
            if lower != upper {
                return Err(MathError::PrecisionExhausted);
            }
            lower
        };
        let storage = purrdf_xsd::integer::ExactArithmeticCost::decimal_rational(
            lower.as_integer().bit_len(),
            places,
        )
        .and_then(|cost| usize::try_from(cost.workspace_bytes).ok())
        .ok_or(MathError::WorkspaceExhausted)?;
        math.reserve_workspace(storage)?;
        values.push(crate::numerical::math_quantized_decimal(
            &lower, places, math, progress,
        )?);
    }
    if target.reference.unit == CoordinateUnit::Degrees {
        for (index, bound) in [(0, 180), (1, 90)] {
            let bound = Rat::from_i64(bound);
            let outside = crate::numerical::math_exact_rational(
                purrdf_xsd::integer::ExactOperation::RationalCompare,
                &[&values[index], &bound],
                math,
                progress,
                || values[index].abs() > bound,
            )?;
            if outside {
                return Err(MathError::Domain(
                    "transformed angular coordinate outside its declared geographic range",
                ));
            }
        }
    }
    certify(&output.certification_input, &values, math, progress)?;
    let epoch = epoch
        .map(|epoch| {
            let storage = epoch
                .numerator()
                .allocated_bytes()
                .checked_add(epoch.denominator().allocated_bytes())
                .and_then(|bytes| bytes.checked_add(size_of::<Rat>()))
                .ok_or(MathError::WorkspaceExhausted)?;
            math.reserve_workspace(storage)?;
            crate::numerical::math_exact_rational(
                purrdf_xsd::integer::ExactOperation::Linear,
                &[epoch],
                math,
                progress,
                || epoch.clone(),
            )
        })
        .transpose()?;
    Ok((values, epoch))
}

pub(super) fn rational_fields(value: &Rat, fields: &mut Vec<Vec<u8>>) {
    fields.push(value.numerator().to_string().into_bytes());
    fields.push(value.denominator().to_string().into_bytes());
}

/// Original normalized epoch numerator/denominator, admitted before decimal
/// rendering. The receipt is actual retained vector/string capacity.
pub(super) fn epoch_fields_admitted(
    epoch: Option<&Rat>,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Vec<Vec<u8>>, u64), GeoError> {
    let Some(value) = epoch else {
        return Ok((Vec::new(), 0));
    };
    rational_fields_admitted(value, context, progress)
}

/// The one admitted normalized numerator/denominator decimal encoder.
pub(crate) fn rational_fields_admitted(
    value: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<(Vec<Vec<u8>>, u64), GeoError> {
    let bits = value
        .numerator()
        .bit_len()
        .max(value.denominator().bit_len())
        .max(1);
    let mut cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
        purrdf_xsd::integer::ExactOperation::DecimalRender,
        bits,
        2,
    )
    .ok_or(GeoError::ArithmeticOverflow(
        "operation epoch rendering cost",
    ))?;
    let source_bytes = value
        .numerator()
        .allocated_bytes()
        .checked_add(value.denominator().allocated_bytes())
        .and_then(|bytes| u64::try_from(bytes).ok())
        .ok_or(GeoError::ArithmeticOverflow(
            "operation epoch source storage",
        ))?;
    let rendered = bits
        .div_ceil(63)
        .checked_mul(19)
        .and_then(|bytes| bytes.checked_add(1))
        .and_then(|bytes| bytes.checked_mul(4))
        .and_then(|bytes| bytes.checked_add(128))
        .ok_or(GeoError::ArithmeticOverflow(
            "operation epoch rendered storage",
        ))?;
    cost.workspace_bytes = cost
        .workspace_bytes
        .checked_add(source_bytes)
        .and_then(|bytes| bytes.checked_add(rendered))
        .ok_or(GeoError::ArithmeticOverflow(
            "operation epoch rendering storage",
        ))?;
    progress.exact(context, cost, || {
        let mut fields = Vec::with_capacity(2);
        rational_fields(value, &mut fields);
        let bytes = fields.iter().try_fold(
            fields.capacity().checked_mul(size_of::<Vec<u8>>()).ok_or(
                GeoError::ArithmeticOverflow("operation epoch fields storage"),
            )?,
            |bytes, field| {
                bytes
                    .checked_add(field.capacity())
                    .ok_or(GeoError::ArithmeticOverflow("operation epoch bytes"))
            },
        )?;
        Ok((
            fields,
            u64::try_from(bytes)
                .map_err(|_| GeoError::ArithmeticOverflow("operation epoch retained storage"))?,
        ))
    })
}

/// Prepare and validate a borrowed original source before cloning its limbs.
pub(super) fn prepare_source_point(
    model: &OperationModel,
    source: [&Rat; 2],
    height: Option<&Rat>,
    epoch: Option<&Rat>,
    swapped_axes: bool,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<OperationPoint, GeoError> {
    model.validate_presence(height, epoch)?;
    let normalized = if swapped_axes {
        [source[1], source[0]]
    } else {
        source
    };
    let cost = source_preparation_cost(model, normalized, height, epoch).ok_or(
        GeoError::ArithmeticOverflow("operation source preparation cost"),
    )?;
    progress.exact(context, cost, || {
        make_source_point(model, normalized, height, epoch)
    })
}

fn source_preparation_cost(
    model: &OperationModel,
    source: [&Rat; 2],
    height: Option<&Rat>,
    epoch: Option<&Rat>,
) -> Option<purrdf_xsd::integer::ExactArithmeticCost> {
    let operands = [Some(source[0]), Some(source[1]), height, epoch];
    let bits = operands.into_iter().flatten().fold(1, |bits, value| {
        bits.max(value.numerator().bit_len())
            .max(value.denominator().bit_len())
    });
    let copy = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
        purrdf_xsd::integer::ExactOperation::Linear,
        bits,
        8,
    )?;
    let validation = model.validation_cost(source, height, epoch, true)?;
    copy.followed_by(validation)
}

fn make_source_point(
    model: &OperationModel,
    source: [&Rat; 2],
    height: Option<&Rat>,
    epoch: Option<&Rat>,
) -> Result<OperationPoint, GeoError> {
    let point = OperationPoint {
        x: source[0].clone(),
        y: source[1].clone(),
        z: height.cloned(),
        epoch: epoch.cloned(),
    };
    model.validate_point(&point)?;
    Ok(point)
}

pub(super) fn decimal(text: &str, math: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
    fixed_from_rat(
        &Rat::parse_decimal(text).expect("frozen model decimal"),
        math,
    )
}

pub(super) fn radians(
    degrees: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let pi = FixedInterval::pi(math)?;
    let denominator = FixedInterval::from_i64(180, math)?;
    degrees.mul(&pi, math)?.div(&denominator, math)
}

pub(super) fn degrees(
    radians: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let factor = FixedInterval::from_i64(180, math)?;
    let pi = FixedInterval::pi(math)?;
    radians.mul(&factor, math)?.div(&pi, math)
}

/// Exact frozen analytic Mercator axes and eccentricity; shared by forward,
/// inverse compilation and continuous-image derivative evaluation.
const BAIDU_MERCATOR_AXES: [&str; 2] = ["6378206.4", "6356583.8"];

/// Evaluate the original fixed-axis equation after its caller admits the cost.
pub(super) fn baidu_mercator_parameters() -> (Rat, Rat) {
    let [a, b] = BAIDU_MERCATOR_AXES.map(|text| Rat::parse_decimal(text).expect("frozen decimal"));
    let eccentricity_squared = Rat::one().sub(&b.mul(&b).div(&a.mul(&a)).expect("positive axis"));
    (a, eccentricity_squared)
}

/// Bound the actual frozen parsing and exact `1-b²/a²` body before evaluation.
pub(super) fn baidu_mercator_parameters_cost() -> Option<purrdf_xsd::integer::ExactArithmeticCost> {
    use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
    let mut cost = ExactArithmeticCost::for_operation(ExactOperation::Linear, 1, 0)?;
    for text in BAIDU_MERCATOR_AXES {
        let (digits, power) = Rat::decimal_construction(text)?;
        cost = cost.followed_by(ExactArithmeticCost::decimal(digits, power)?)?;
    }
    // Both canonical axes have a 25-bit numerator and denominator five (3
    // bits). Squaring gives at most 50/6 bits, and their quotient at most
    // 56/56 bits before cancellation. These bounds include all actual input
    // cross-GCDs, quotients and checked-profile output coprimality assertions.
    for (operation, operands, count) in [
        (ExactOperation::RationalMultiply, [(25, 3), (25, 3)], 2),
        (ExactOperation::RationalDivide, [(50, 6), (50, 6)], 1),
        (ExactOperation::RationalAdd, [(1, 1), (56, 56)], 1),
    ] {
        cost = cost.followed_by(ExactArithmeticCost::for_rational_operands(
            operation, operands, count,
        )?)?;
    }
    Some(cost)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionLimits, ExecutionPolicy, GeographicReference, geodesic};

    fn rat(value: &str) -> Rat {
        Rat::parse_decimal(value).unwrap()
    }
    pub(super) fn reference(number: u8, unit: CoordinateUnit) -> OperationReference {
        OperationReference {
            realization: Digest32::new([number; 32]),
            unit,
            swapped_axes: false,
        }
    }
    fn point(x: &str, y: &str) -> OperationPoint {
        OperationPoint {
            x: rat(x),
            y: rat(y),
            z: None,
            epoch: None,
        }
    }
    fn compile(
        model: OperationModel,
        input: CoordinateUnit,
        output: CoordinateUnit,
    ) -> CoordinateOperation {
        CoordinateOperation::compile(reference(1, input), reference(2, output), model).unwrap()
    }
    fn context() -> MetricContext {
        MetricContext::wgs84().unwrap()
    }
    fn angular_disagreement(output: &OperationPoint, expected: &OperationPoint) -> Rat {
        geodesic::distance(
            LonLat::new(output.x.clone(), output.y.clone()).unwrap(),
            LonLat::new(expected.x.clone(), expected.y.clone()).unwrap(),
        )
        .unwrap()
        .value()
        .exact()
        .clone()
    }

    #[test]
    fn compiled_original_width_includes_large_coefficients_and_inverse_domains() {
        let coefficient = rat(
            "1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001",
        );
        let polynomial = Polynomial2d::new(
            1,
            [Rat::zero(), Rat::zero()],
            [Rat::one(), Rat::one()],
            vec![PolynomialTerm {
                x_power: 1,
                y_power: 0,
                x_coefficient: coefficient.clone(),
                y_coefficient: Rat::zero(),
            }],
        )
        .unwrap();
        let operation = compile(
            OperationModel::Polynomial2d(polynomial.clone()),
            CoordinateUnit::Metres,
            CoordinateUnit::Metres,
        );
        assert_eq!(
            operation.max_original_operand_bits(),
            coefficient.numerator().bit_len()
        );
        let domain = rat("1e200");
        let inverse = compile(
            OperationModel::Polynomial2dInverse(Box::new(Polynomial2dInverse {
                polynomial,
                source_domain: MetricSourceDomain::new(
                    [Rat::zero(), Rat::zero()],
                    [domain.clone(), Rat::one()],
                )
                .unwrap(),
            })),
            CoordinateUnit::Metres,
            CoordinateUnit::Metres,
        );
        assert_eq!(
            inverse.max_original_operand_bits(),
            domain.numerator().bit_len()
        );
        let source = operation.source();
        let target = operation.target();
        let next =
            CoordinateOperation::compile(target, source, inverse.model.as_ref().clone()).unwrap();
        let chain = OperationChain::compile(vec![operation, next]).unwrap();
        assert_eq!(
            chain.max_original_operand_bits(),
            domain.numerator().bit_len()
        );
    }

    #[test]
    fn published_data_fields_are_independent_empirical_comparisons() {
        let applicability = Applicability::new(
            rat("72.004"),
            rat("137.8347"),
            rat("0.8293"),
            rat("55.8271"),
        )
        .unwrap();
        let gcj = compile(
            OperationModel::GcjRationalHarmonicV1(applicability),
            CoordinateUnit::Degrees,
            CoordinateUnit::Degrees,
        );
        let mut context = context();
        let result = gcj
            .apply(&point("114.0164322", "33.0133556"), &mut context)
            .unwrap();
        assert!(
            angular_disagreement(result.point(), &point("114.022306", "33.0115324")) < rat("1.0")
        );
        // Each corresponding fixture field is tested independently: errors in a
        // preceding model are not propagated into a different model's agreement.
        let bd = compile(
            OperationModel::Bd09LlV1,
            CoordinateUnit::Degrees,
            CoordinateUnit::Degrees,
        );
        let result = bd
            .apply(&point("114.022306", "33.0115324"), &mut context)
            .unwrap();
        assert!(
            angular_disagreement(result.point(), &point("114.0287781", "33.0176697")) < rat("0.02")
        );
        let mercator = compile(
            OperationModel::BaiduMercatorAnalyticV1,
            CoordinateUnit::Degrees,
            CoordinateUnit::Metres,
        );
        let result = mercator
            .apply(&point("114.0287781", "33.0176697"), &mut context)
            .unwrap();
        let dx = result.point().x.sub(&rat("12693763.634"));
        let dy = result.point().y.sub(&rat("3874151.929"));
        assert!(dx.mul(&dx).add(&dy.mul(&dy)) < rat("0.04"));
        assert!(gcj.apply(&point("0", "0"), &mut context).is_err());
    }

    #[test]
    fn height_epoch_axis_and_square_domains_are_explicit() {
        let geocentric = compile(
            OperationModel::GeographicToGeocentric {
                ellipsoid: PreparedEllipsoid::cgcs2000(),
            },
            CoordinateUnit::Degrees,
            CoordinateUnit::Metres,
        );
        let mut context = context();
        assert_eq!(
            geocentric.apply(&point("0", "0"), &mut context),
            Err(GeoError::MissingHeight)
        );
        let mut source = point("0", "0");
        source.z = Some(rat("10"));
        let result = geocentric.apply(&source, &mut context).unwrap();
        assert_eq!(result.point().x, rat("6378147"));
        assert!(result.point().y.is_zero());
        assert!(result.point().z.as_ref().unwrap().is_zero());
        let web = compile(
            OperationModel::WebMercator {
                radius: rat("6378137"),
            },
            CoordinateUnit::Degrees,
            CoordinateUnit::Metres,
        );
        assert!(web.apply(&point("0", "86"), &mut context).is_err());
        let result = web.apply(&point("0", "0"), &mut context).unwrap();
        assert!(result.point().x.is_zero());
        assert!(result.point().y.is_zero());
    }

    #[test]
    fn bd09_inverse_proves_uniqueness_origin_and_quantized_residual() {
        let forward = compile(
            OperationModel::Bd09LlV1,
            CoordinateUnit::Degrees,
            CoordinateUnit::Degrees,
        );
        let inverse = forward.inverse().unwrap();
        assert_eq!(inverse.source(), forward.target());
        assert_eq!(inverse.target(), forward.source());
        let mut context = context();
        let origin = inverse
            .apply(&point("0.0065", "0.006"), &mut context)
            .unwrap();
        assert!(origin.point().x.is_zero() && origin.point().y.is_zero());
        for source in [
            point("114.022306", "33.0115324"),
            point("0.00001", "-0.00001"),
            point("-179", "80"),
        ] {
            let image = forward.apply(&source, &mut context).unwrap();
            let restored = inverse.apply(image.point(), &mut context).unwrap();
            assert!(restored.point().x.sub(&source.x).abs() <= rat("0.000000000000002"));
            assert!(restored.point().y.sub(&source.y).abs() <= rat("0.000000000000002"));
            let reprojected = forward.apply(restored.point(), &mut context).unwrap();
            assert!(reprojected.point().x.sub(&image.point().x).abs() <= rat("0.000000000001"));
            assert!(reprojected.point().y.sub(&image.point().y).abs() <= rat("0.000000000001"));
        }
    }

    #[test]
    fn matching_transverse_roundtrip_uses_all_parameters_and_reversed_bindings() {
        let parameters = TransverseMercator {
            ellipsoid: PreparedEllipsoid::cgcs2000(),
            family: ZoneFamily::GaussKruger6,
            zone: 20,
            central_meridian: rat("117"),
            scale: Rat::one(),
            false_easting: rat("500000"),
            false_northing: Rat::zero(),
            hemisphere: Hemisphere::North,
            zone_prefix: false,
        };
        let forward = compile(
            OperationModel::TransverseMercator(Box::new(parameters.clone())),
            CoordinateUnit::Degrees,
            CoordinateUnit::Metres,
        );
        let inverse = forward.inverse().unwrap();
        let pair = OperationChain::compile(vec![forward.clone(), inverse.clone()]).unwrap();
        assert_eq!(pair.exact_tm_roundtrip(), Some(&parameters));
        assert!(
            OperationChain::compile(vec![inverse, forward.clone()])
                .unwrap()
                .exact_tm_roundtrip()
                .is_none()
        );
        assert!(
            OperationChain::compile(vec![forward.clone()])
                .unwrap()
                .exact_tm_roundtrip()
                .is_none()
        );
        for field in 0..6 {
            let mut different = parameters.clone();
            match field {
                0 => different.scale = rat("0.9999"),
                1 => different.false_easting = rat("500001"),
                2 => different.false_northing = Rat::one(),
                3 => different.hemisphere = Hemisphere::South,
                4 => different.zone_prefix = true,
                _ => different.ellipsoid = PreparedEllipsoid::wgs84(),
            }
            let other = CoordinateOperation::compile(
                forward.target(),
                forward.source(),
                OperationModel::TransverseMercatorInverse(Box::new(different)),
            )
            .unwrap();
            assert!(
                OperationChain::compile(vec![forward.clone(), other])
                    .unwrap()
                    .exact_tm_roundtrip()
                    .is_none()
            );
        }
        let other = CoordinateOperation::compile(
            forward.target(),
            reference(3, CoordinateUnit::Degrees),
            OperationModel::TransverseMercatorInverse(Box::new(parameters.clone())),
        )
        .unwrap();
        assert!(
            OperationChain::compile(vec![forward.clone(), other])
                .unwrap()
                .exact_tm_roundtrip()
                .is_none()
        );
        let mut axis = forward.source();
        axis.swapped_axes = true;
        let other = CoordinateOperation::compile(
            forward.target(),
            axis,
            OperationModel::TransverseMercatorInverse(Box::new(parameters)),
        )
        .unwrap();
        assert!(
            OperationChain::compile(vec![forward, other])
                .unwrap()
                .exact_tm_roundtrip()
                .is_none()
        );
    }

    #[test]
    fn matching_transverse_scalar_preserves_exact_ties_and_domain() {
        let forward = compile(
            OperationModel::TransverseMercator(Box::new(TransverseMercator {
                ellipsoid: PreparedEllipsoid::cgcs2000(),
                family: ZoneFamily::GaussKruger6,
                zone: 20,
                central_meridian: rat("117"),
                scale: Rat::one(),
                false_easting: rat("500000"),
                false_northing: Rat::zero(),
                hemisphere: Hemisphere::North,
                zone_prefix: false,
            })),
            CoordinateUnit::Degrees,
            CoordinateUnit::Metres,
        );
        let chain =
            OperationChain::compile(vec![forward.clone(), forward.inverse().unwrap()]).unwrap();
        let mut context = context();
        for (source, expected) in [
            (point("117.25", "31.23"), point("117.25", "31.23")),
            (
                point("117.0000000000000005", "31.2300000000000015"),
                point("117", "31.230000000000002"),
            ),
        ] {
            let output = chain.apply(&source, &mut context).unwrap();
            assert_eq!(output.point(), &expected);
            assert_eq!(output.operation_id(), chain.id());
            assert_eq!(output.law_id(), chain.law);
            assert!(context.work_items() < ExecutionLimits::GEOMETRY.max_work_items);
        }
        for source in [point("121", "31.23"), point("117", "-1")] {
            assert!(matches!(
                chain.apply(&source, &mut context),
                Err(GeoError::Domain { .. })
            ));
        }
        for source_axes in [false, true] {
            for projected_axes in [false, true] {
                let mut source_reference = forward.source;
                source_reference.swapped_axes = source_axes;
                let mut projected_reference = forward.target;
                projected_reference.swapped_axes = projected_axes;
                let selected = CoordinateOperation::compile(
                    source_reference,
                    projected_reference,
                    forward.model().clone(),
                )
                .unwrap();
                let declared =
                    OperationChain::compile(vec![selected.clone(), selected.inverse().unwrap()])
                        .unwrap();
                let (x, y) = if source_axes {
                    ("31.2300000000000015", "117.0000000000000005")
                } else {
                    ("117.0000000000000005", "31.2300000000000015")
                };
                let mut source = point(x, y);
                source.z = Some(rat("10.0000005"));
                source.epoch = Some(rat("2026.75"));
                let mut expected = if source_axes {
                    point("31.230000000000002", "117")
                } else {
                    point("117", "31.230000000000002")
                };
                expected.z = Some(rat("10"));
                expected.epoch = Some(rat("2026.75"));
                let output = declared.apply(&source, &mut context).unwrap();
                assert_eq!(output.point(), &expected);
                assert_eq!(output.operation_id(), declared.id());
                assert_eq!(output.law_id(), declared.law);
            }
        }
        // Inverse-first input is an actual projected carrier, not the original
        // source to which the complete forward/inverse identity proof applies.
        let projected = OperationChain::compile(vec![forward.inverse().unwrap(), forward]).unwrap();
        assert!(projected.exact_tm_roundtrip().is_none());
    }

    #[test]
    fn declared_transverse_zones_inverse_prefix_and_hemisphere() {
        for (family, zone, central, latitude, prefix) in [
            (ZoneFamily::Utm, 50, "117", "35", false),
            (ZoneFamily::Utm, 56, "153", "-35", false),
            (ZoneFamily::GaussKruger3, 39, "117", "35", true),
            (ZoneFamily::GaussKruger6, 20, "117", "35", false),
        ] {
            let parameters = TransverseMercator {
                ellipsoid: PreparedEllipsoid::cgcs2000(),
                family,
                zone,
                central_meridian: rat(central),
                scale: if family == ZoneFamily::Utm {
                    rat("0.9996")
                } else {
                    Rat::one()
                },
                false_easting: rat("500000"),
                false_northing: if latitude.starts_with('-') {
                    rat("10000000")
                } else {
                    Rat::zero()
                },
                hemisphere: if latitude.starts_with('-') {
                    Hemisphere::South
                } else {
                    Hemisphere::North
                },
                zone_prefix: prefix,
            };
            let forward = compile(
                OperationModel::TransverseMercator(Box::new(parameters)),
                CoordinateUnit::Degrees,
                CoordinateUnit::Metres,
            );
            let inverse = forward.inverse().unwrap();
            let source = OperationPoint {
                x: rat(central).add(&rat("0.7")),
                y: rat(latitude),
                z: Some(rat("12.5")),
                epoch: None,
            };
            let mut context = context();
            let image = forward.apply(&source, &mut context).unwrap();
            let restored = inverse.apply(image.point(), &mut context).unwrap();
            assert!(restored.point().x.sub(&source.x).abs() < rat("0.00000000002"));
            assert!(restored.point().y.sub(&source.y).abs() < rat("0.00000000002"));
            assert_eq!(restored.point().z, source.z);
            let reprojected = forward.apply(restored.point(), &mut context).unwrap();
            let dx = reprojected.point().x.sub(&image.point().x);
            let dy = reprojected.point().y.sub(&image.point().y);
            assert!(dx.mul(&dx).add(&dy.mul(&dy)) <= rat("0.000000000001"));
            assert!(
                forward
                    .apply(
                        &OperationPoint {
                            y: source.y.neg(),
                            ..source
                        },
                        &mut context
                    )
                    .is_err()
            );
        }
    }

    #[test]
    fn actual_small_angle_affine_inverse_and_epoch_rates() {
        let mut parameters = HelmertParameters {
            translation: [rat("7"), rat("-2"), rat("1")],
            rotation_arcseconds: [rat("360000"), rat("-720000"), rat("180000")],
            scale_ppm: rat("4"),
            pivot: [rat("10"), rat("20"), rat("30")],
            law: RotationLaw::HelmertSmallAngleV1,
            convention: RotationConvention::CoordinateFrame,
            rates: None,
            inverse: false,
        };
        let forward = compile(
            OperationModel::Helmert(Box::new(parameters.clone())),
            CoordinateUnit::Metres,
            CoordinateUnit::Metres,
        );
        parameters.inverse = true;
        let inverse = CoordinateOperation::compile(
            forward.target(),
            forward.source(),
            OperationModel::Helmert(Box::new(parameters)),
        )
        .unwrap();
        let mut source = point("1234", "-4567");
        source.z = Some(rat("8901"));
        let mut context = context();
        let image = forward.apply(&source, &mut context).unwrap();
        let restored = inverse.apply(image.point(), &mut context).unwrap();
        for (actual, expected) in [
            &restored.point().x,
            &restored.point().y,
            restored.point().z.as_ref().unwrap(),
        ]
        .into_iter()
        .zip([&source.x, &source.y, source.z.as_ref().unwrap()])
        {
            assert!(actual.sub(expected).abs() <= rat("0.000002"));
        }
        let rates = HelmertRates {
            epoch: rat("2000"),
            translation: [rat("2"), rat("0"), rat("0")],
            rotation_arcseconds: [Rat::zero(), Rat::zero(), Rat::zero()],
            scale_ppm: Rat::zero(),
        };
        let parameters = HelmertParameters {
            translation: [Rat::zero(), Rat::zero(), Rat::zero()],
            rotation_arcseconds: [Rat::zero(), Rat::zero(), Rat::zero()],
            scale_ppm: Rat::zero(),
            pivot: [Rat::zero(), Rat::zero(), Rat::zero()],
            law: RotationLaw::HelmertEulerRzRyRxV1,
            convention: RotationConvention::PositionVector,
            rates: Some(rates),
            inverse: false,
        };
        let operation = compile(
            OperationModel::Helmert(Box::new(parameters)),
            CoordinateUnit::Metres,
            CoordinateUnit::Metres,
        );
        assert!(operation.apply(&source, &mut context).is_err());
        source.epoch = Some(rat("2100"));
        assert_eq!(
            operation.apply(&source, &mut context).unwrap().point().x,
            source.x.add(&rat("200"))
        );
    }

    #[test]
    fn chain_quantizes_only_final_image_and_certificates_ignore_proof_tightness() {
        let identity = Similarity2d {
            translation: [rat("0.00000025"), Rat::zero()],
            scale: Rat::one(),
            rotation_degrees: Rat::zero(),
            convention: RotationConvention::PositionVector,
            inverse: false,
        };
        let first = compile(
            OperationModel::Similarity2d(identity.clone()),
            CoordinateUnit::Metres,
            CoordinateUnit::Metres,
        );
        let second = CoordinateOperation::compile(
            first.target(),
            reference(3, CoordinateUnit::Metres),
            OperationModel::Similarity2d(identity),
        )
        .unwrap();
        let chain = OperationChain::compile(vec![first, second]).unwrap();
        let source = point("0.00000025", "0");
        let mut context = context();
        let result = chain.apply(&source, &mut context).unwrap();
        assert_eq!(result.point().x, rat("0.000001"));
        let mut limits = ExecutionLimits::GEOMETRY;
        limits.max_precision_bits = 192;
        let mut other = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(limits).unwrap(),
        )
        .unwrap();
        assert_eq!(
            result.certificate_bytes(),
            chain
                .apply(&source, &mut other)
                .unwrap()
                .certificate_bytes()
        );
    }
}
