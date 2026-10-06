// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original interval evaluation of published coordinate equations.

use super::OperationCoordinates;

use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError, RootJacobian2};

use super::{
    CoordinateUnit, OperationModel, OperationPoint, OperationSolverLimits, decimal, degrees,
    radians, rational_fields,
};
use crate::context::WorkProgress;
use crate::numerical::{exact_bounds, fixed_from_rat};
use crate::{GeoError, LonLat, PreparedEllipsoid, Rat};

impl OperationModel {
    /// These original equations are injective on their admitted areal domains.
    /// Their actual whole-panel differential also establishes analyticity on
    /// the panel used by separated-coordinate proofs; in particular BD09's
    /// continuous origin is not an analytic point and cannot provide that
    /// differential. Other models use critical-cell isolation and subdivision.
    pub(crate) fn registered_injective_area(&self) -> bool {
        matches!(
            self,
            Self::Similarity2d(_)
                | Self::WebMercator { .. }
                | Self::EllipsoidalMercator { .. }
                | Self::BaiduMercatorAnalyticV1
                | Self::MercatorToGeographic { .. }
                | Self::TransverseMercator(_)
                | Self::TransverseMercatorInverse(_)
                | Self::Bd09LlV1
                | Self::Bd09LlInverseV1
        )
    }
    pub(super) fn descriptor(&self) -> &'static [u8] {
        match self {
            Self::GcjRationalHarmonicInverseV1(_) => b"InverseGcjRationalHarmonicV1;explicit-closed-source-applicability;all-cusp-chart-roots;multiple=refuse;angular-half-even15;forward-residual<=1e-12degree;certificate=v1",
            Self::GcjRationalHarmonicV1(_) => b"GcjRationalHarmonicV1;exact-decimal-constants;explicit-closed-applicability;outside=refuse;angular-half-even15;metre-half-even6;certificate=v1",
            Self::Bd09LlInverseV1 => b"InverseBd09LLCartesianV1;global-source-angular-rectangle;contraction-L2<=.04;all-roots=unique;angular-half-even15;forward-residual<=1e-12degree;certificate=v1",
            Self::EllipsoidalMercator { .. } => b"ExplicitEllipsoidalMercatorV1;declared-axis-eccentricity;open-poles;metre-half-even6;certificate=v1",
            Self::Bd09LlV1 => b"Bd09LLCartesianV1;k=3000*pi/180;origin=(.0065,.006);angular-half-even15;metre-half-even6;certificate=v1",
            Self::BaiduMercatorAnalyticV1 => b"BaiduMercatorAnalyticV1;a=6378206.4;b=6356583.8;open-poles;metre-half-even6;certificate=v1",
            Self::WebMercator { .. } => b"WebMercatorSquareV1;abs(phi)<=atan(sinh(pi));metre-half-even6;certificate=v1",
            Self::GeographicToGeocentric { .. } => b"GeographicGeocentricV1;actual-height;angular-half-even15;metre-half-even6;certificate=v1",
            Self::MercatorToGeographic { .. } => b"InverseMercatorV1;monotone-certified-root;angular-half-even15;metre-half-even6;forward-residual<=.000001m;certificate=v1",
            Self::GeocentricToGeographic { .. } => b"InverseGeocentricV1;certified-normal-branch;angular-half-even15;metre-half-even6;forward-residual<=.000001m;certificate=v1",
            Self::TransverseMercatorInverse(_) => b"InverseTransverseMercatorTaylorV1;explicit-zone;complete-Krawczyk-isolation;shared-original-isometric-ODE-and-differentiated-Cauchy-tail;angular-half-even15;forward-residual<=.000001m;certificate=v1",
            Self::TransverseMercator(_) => b"TransverseMercatorTaylorV1;explicit-zone-scale-origins-hemisphere-prefix;original-isometric-ODE;certified-Cauchy-tail;metre-half-even6;certificate=v1",
            Self::Helmert(parameters) => match parameters.law {
                super::RotationLaw::HelmertSmallAngleV1 => b"HelmertSmallAngleV1;I+cross-product;frame=full-transpose;actual-nonorthogonal-affine-inverse;exact-epoch-rates;positive-1+ppm/1e6;pivot;metre-half-even6;certificate=v1",
                super::RotationLaw::HelmertEulerRzRyRxV1 => b"HelmertEulerRzRyRxV1;right-handed-active-RzRyRx;frame=full-transpose;actual-affine-inverse;exact-epoch-rates;positive-1+ppm/1e6;pivot;metre-half-even6;certificate=v1",
            },
            Self::Similarity2d(_) => b"Similarity2dV1;active-counterclockwise-or-frame-transpose;positive-scale;actual-affine-inverse;metre-half-even6;certificate=v1",
            Self::Polynomial2d(_) => b"PolynomialMonomialV1;explicit-degree;canonical-powers;explicit-normalization;metre-half-even6;certificate=v1",
            Self::BilinearGrid(_) => b"BilinearDisplacementClosedPatchesV1;regular-explicit-extent;row-major;domain=closed-union-of-complete-patches;holes-refuse;no-extrapolation;metre-half-even6;certificate=v1",
            Self::Polynomial2dInverse(_) => b"InversePolynomialMonomialV1;explicit-closed-metre-source-rectangle;all-roots;multiple=refuse;metre-half-even6;forward-residual<=.000001m;certificate=v1",
            Self::BilinearGridInverse(_) => b"InverseBilinearClosedPatchesV1;explicit-closed-metre-source-rectangle;domain=closed-union-of-complete-patches;all-roots;multiple=refuse;no-extrapolation;metre-half-even6;forward-residual<=.000001m;certificate=v1",
        }
    }
    pub(super) const fn units(&self) -> (CoordinateUnit, CoordinateUnit) {
        match self {
            Self::GcjRationalHarmonicV1(_)
            | Self::GcjRationalHarmonicInverseV1(_)
            | Self::Bd09LlV1
            | Self::Bd09LlInverseV1 => (CoordinateUnit::Degrees, CoordinateUnit::Degrees),
            Self::BaiduMercatorAnalyticV1
            | Self::EllipsoidalMercator { .. }
            | Self::WebMercator { .. }
            | Self::GeographicToGeocentric { .. }
            | Self::TransverseMercator(_) => (CoordinateUnit::Degrees, CoordinateUnit::Metres),
            Self::MercatorToGeographic { .. }
            | Self::GeocentricToGeographic { .. }
            | Self::TransverseMercatorInverse(_) => {
                (CoordinateUnit::Metres, CoordinateUnit::Degrees)
            }
            Self::Helmert(_)
            | Self::Similarity2d(_)
            | Self::Polynomial2d(_)
            | Self::BilinearGrid(_)
            | Self::Polynomial2dInverse(_)
            | Self::BilinearGridInverse(_) => (CoordinateUnit::Metres, CoordinateUnit::Metres),
        }
    }
    pub(super) fn validate(&self) -> Result<(), GeoError> {
        match self {
            Self::GcjRationalHarmonicV1(domain) | Self::GcjRationalHarmonicInverseV1(domain) => {
                super::Applicability::new_admitted(
                    domain.west.clone(),
                    domain.east.clone(),
                    domain.south.clone(),
                    domain.north.clone(),
                )?;
                LonLat::new(domain.west.clone(), domain.south.clone())?;
                LonLat::new(domain.east.clone(), domain.north.clone())?;
                if domain.south <= Rat::from_i64(-90) || domain.north >= Rat::from_i64(90) {
                    return Err(GeoError::config(
                        "GCJ applicability must exclude both poles",
                    ));
                }
                Ok(())
            }
            Self::WebMercator { radius } if radius <= &Rat::zero() => {
                Err(GeoError::config("Web Mercator radius must be positive"))
            }
            Self::MercatorToGeographic {
                radius,
                eccentricity_squared,
                ..
            }
            | Self::EllipsoidalMercator {
                semimajor: radius,
                eccentricity_squared,
            } => {
                if radius <= &Rat::zero()
                    || eccentricity_squared < &Rat::zero()
                    || eccentricity_squared >= &Rat::one()
                {
                    return Err(GeoError::config(
                        "inverse Mercator requires positive radius and squared eccentricity in [0,1)",
                    ));
                }
                Ok(())
            }
            Self::TransverseMercator(parameters) | Self::TransverseMercatorInverse(parameters) => {
                parameters.validate()
            }
            Self::BilinearGridInverse(parameters) => parameters.validate(),
            Self::Helmert(parameters) => parameters.validate(),
            Self::Similarity2d(parameters) => parameters.validate(),
            _ => Ok(()),
        }
    }
    pub(super) fn validate_metadata(&self, point: &OperationPoint) -> Result<(), GeoError> {
        match self {
            Self::GeographicToGeocentric { .. } | Self::GeocentricToGeographic { .. }
                if point.z.is_none() =>
            {
                Err(GeoError::MissingHeight)
            }
            Self::Helmert(parameters) => parameters.validate_point(point),
            _ => Ok(()),
        }
    }
    pub(crate) fn validate_presence(
        &self,
        height: Option<&Rat>,
        epoch: Option<&Rat>,
    ) -> Result<(), GeoError> {
        if matches!(
            self,
            Self::GeographicToGeocentric { .. }
                | Self::GeocentricToGeographic { .. }
                | Self::Helmert(_)
        ) && height.is_none()
        {
            return Err(GeoError::MissingHeight);
        }
        if matches!(self, Self::Helmert(parameters) if parameters.rates.is_some())
            && epoch.is_none()
        {
            return Err(GeoError::domain(
                "a rate-bearing Helmert requires an actual observation epoch",
            ));
        }
        Ok(())
    }

    /// Complete exact validation bound, derived from the original source/model
    /// operands before their first clone, comparison or normalization.
    pub(super) fn validation_cost(
        &self,
        source: [&Rat; 2],
        height: Option<&Rat>,
        epoch: Option<&Rat>,
        full: bool,
    ) -> Option<ExactArithmeticCost> {
        let mut bits = 20; // Includes range constants and the Helmert ppm bound.
        let mut include = |value: &Rat| {
            bits = bits
                .max(value.numerator().bit_len())
                .max(value.denominator().bit_len());
        };
        for value in source.into_iter().chain(height).chain(epoch) {
            include(value);
        }
        match self {
            Self::GcjRationalHarmonicV1(domain) => {
                for value in [&domain.west, &domain.east, &domain.south, &domain.north] {
                    include(value);
                }
            }
            Self::TransverseMercator(parameters) => include(&parameters.central_meridian),
            Self::Helmert(parameters) => {
                // XYZ is not an operand of epoch-adjusted parameter validation.
                // Bounding it here would compound unrelated coordinate widths.
                bits = parameters
                    .translation
                    .iter()
                    .chain(&parameters.rotation_arcseconds)
                    .chain(core::iter::once(&parameters.scale_ppm))
                    .chain(epoch)
                    .fold(1, |bits, value| {
                        bits.max(value.numerator().bit_len())
                            .max(value.denominator().bit_len())
                    });
                if let Some(rates) = &parameters.rates {
                    bits = rates
                        .translation
                        .iter()
                        .chain(&rates.rotation_arcseconds)
                        .chain([&rates.scale_ppm, &rates.epoch])
                        .fold(bits, |bits, value| {
                            bits.max(value.numerator().bit_len())
                                .max(value.denominator().bit_len())
                        });
                }
            }
            _ => {}
        }
        let empty = ExactArithmeticCost::for_operation(ExactOperation::Linear, bits, 0)?;
        if !full && !matches!(self, Self::Helmert(_)) {
            return Some(empty);
        }
        if full && let Self::BilinearGrid(grid) = self {
            return grid.location_cost(source[0], source[1], true);
        }
        let comparisons = match self {
            Self::GcjRationalHarmonicV1(_) => 8,
            Self::Bd09LlV1
            | Self::Bd09LlInverseV1
            | Self::GcjRationalHarmonicInverseV1(_)
            | Self::GeographicToGeocentric { .. } => 4,
            Self::BaiduMercatorAnalyticV1
            | Self::EllipsoidalMercator { .. }
            | Self::WebMercator { .. } => 6,
            Self::TransverseMercator(_) => 10,
            Self::Helmert(_) => 1,
            _ => 0,
        };
        let mut cost = ExactArithmeticCost::for_operation(
            ExactOperation::RationalCompare,
            bits.max(20),
            comparisons,
        )?;
        if matches!(self, Self::TransverseMercator(_)) {
            let difference =
                ExactArithmeticCost::for_operation(ExactOperation::RationalAdd, bits, 1)?;
            let comparison = ExactArithmeticCost::for_operation(
                ExactOperation::RationalCompare,
                difference.output_bits,
                1,
            )?;
            cost = cost.followed_by(difference)?.followed_by(comparison)?;
        }
        if matches!(self, Self::Helmert(_)) {
            cost = cost.followed_by(ExactArithmeticCost::for_operation(
                ExactOperation::Linear,
                bits,
                14,
            )?)?;
        }
        if let Self::Helmert(parameters) = self
            && parameters.rates.is_some()
        {
            let declared = parameters.rates.as_ref().expect("checked rate parameters");
            let observation = epoch?;
            let delta = crate::numerical::rational_cost(
                ExactOperation::RationalAdd,
                &[observation, &declared.epoch],
                1,
            )?;
            let rate_bits = declared
                .translation
                .iter()
                .chain(&declared.rotation_arcseconds)
                .chain(core::iter::once(&declared.scale_ppm))
                .fold(delta.output_bits, |bits, value| {
                    bits.max(value.numerator().bit_len())
                        .max(value.denominator().bit_len())
                });
            let rates =
                ExactArithmeticCost::for_operation(ExactOperation::RationalMultiply, rate_bits, 7)?;
            let adjusted = ExactArithmeticCost::for_operation(
                ExactOperation::RationalAdd,
                rates.output_bits.max(bits),
                7,
            )?;
            let compare = ExactArithmeticCost::for_operation(
                ExactOperation::RationalCompare,
                adjusted.output_bits,
                1,
            )?;
            cost = cost
                .followed_by(delta)?
                .followed_by(rates)?
                .followed_by(adjusted)?
                .followed_by(compare)?;
        }
        Some(cost)
    }
    pub(super) fn validate_point(&self, point: &OperationPoint) -> Result<(), GeoError> {
        match self {
            Self::GcjRationalHarmonicV1(domain) => {
                LonLat::new(point.x.clone(), point.y.clone())?;
                if !domain.contains(&point.x, &point.y) {
                    return Err(GeoError::domain(
                        "GCJ point is outside the explicitly declared applicability",
                    ));
                }
            }
            Self::Bd09LlV1 | Self::Bd09LlInverseV1 | Self::GcjRationalHarmonicInverseV1(_) => {
                LonLat::new(point.x.clone(), point.y.clone())?;
            }
            Self::BaiduMercatorAnalyticV1
            | Self::EllipsoidalMercator { .. }
            | Self::WebMercator { .. } => {
                let coordinate = LonLat::new(point.x.clone(), point.y.clone())?;
                if coordinate.is_pole() {
                    return Err(GeoError::domain("Mercator is undefined at a pole"));
                }
            }
            Self::GeographicToGeocentric { .. } => {
                LonLat::new(point.x.clone(), point.y.clone())?;
                if point.z.is_none() {
                    return Err(GeoError::MissingHeight);
                }
            }
            Self::GeocentricToGeographic { .. } => {
                if point.z.is_none() {
                    return Err(GeoError::MissingHeight);
                }
            }
            Self::MercatorToGeographic { .. } | Self::TransverseMercatorInverse(_) => {}
            Self::TransverseMercator(parameters) => parameters.validate_point(point)?,
            Self::Helmert(parameters) => parameters.validate_point(point)?,
            Self::BilinearGrid(grid) => grid.validate_point(point)?,
            Self::Similarity2d(_)
            | Self::Polynomial2d(_)
            | Self::Polynomial2dInverse(_)
            | Self::BilinearGridInverse(_) => {}
        }
        Ok(())
    }
    pub(super) fn parameters(&self, fields: &mut Vec<Vec<u8>>) {
        match self {
            Self::GcjRationalHarmonicV1(domain) | Self::GcjRationalHarmonicInverseV1(domain) => {
                for value in [&domain.west, &domain.east, &domain.south, &domain.north] {
                    rational_fields(value, fields);
                }
            }
            Self::WebMercator { radius } => rational_fields(radius, fields),
            Self::EllipsoidalMercator {
                semimajor,
                eccentricity_squared,
            } => {
                rational_fields(semimajor, fields);
                rational_fields(eccentricity_squared, fields);
            }
            Self::MercatorToGeographic {
                radius,
                eccentricity_squared,
                square_domain,
            } => {
                rational_fields(radius, fields);
                rational_fields(eccentricity_squared, fields);
                fields.push(vec![u8::from(*square_domain)]);
            }
            Self::GeographicToGeocentric { ellipsoid }
            | Self::GeocentricToGeographic { ellipsoid } => {
                rational_fields(ellipsoid.semimajor(), fields);
                rational_fields(ellipsoid.inverse_flattening(), fields);
            }
            Self::TransverseMercator(parameters) | Self::TransverseMercatorInverse(parameters) => {
                parameters.parameters(fields);
            }
            Self::Helmert(parameters) => parameters.parameters(fields),
            Self::Similarity2d(parameters) => parameters.parameters(fields),
            Self::Polynomial2d(polynomial) => polynomial.parameters(fields),
            Self::BilinearGrid(grid) => grid.parameters(fields),
            Self::Polynomial2dInverse(parameters) => parameters.parameters(fields),
            Self::BilinearGridInverse(parameters) => parameters.parameters(fields),
            Self::Bd09LlV1 | Self::Bd09LlInverseV1 | Self::BaiduMercatorAnalyticV1 => {}
        }
    }

    /// Complete borrowed parameter walk for original-operand numerical
    /// destination and output preparation. Includes derived immutable ellipsoid
    /// bounds as well as declared parameters; no serialization or clone occurs.
    pub fn visit_original_operands(&self, visit: &mut impl FnMut(&Rat)) {
        let polynomial = |value: &super::Polynomial2d, visit: &mut dyn FnMut(&Rat)| {
            for value in value.origin().iter().chain(value.scale()) {
                visit(value);
            }
            for term in value.terms() {
                visit(&term.x_coefficient);
                visit(&term.y_coefficient);
            }
        };
        let grid = |value: &super::BilinearGrid, visit: &mut dyn FnMut(&Rat)| {
            for value in value.origin().iter().chain(value.spacing()) {
                visit(value);
            }
            for node in value.nodes().iter().flatten() {
                visit(&node.x);
                visit(&node.y);
            }
        };
        let ellipsoid = |value: &PreparedEllipsoid, visit: &mut dyn FnMut(&Rat)| {
            let (lower, upper) = value.normal_metric_bounds_ref();
            for value in [
                value.semimajor(),
                value.semiminor(),
                value.inverse_flattening(),
                lower.exact(),
                upper.exact(),
            ] {
                visit(value);
            }
        };
        match self {
            Self::GcjRationalHarmonicV1(domain) | Self::GcjRationalHarmonicInverseV1(domain) => {
                for value in [&domain.west, &domain.east, &domain.south, &domain.north] {
                    visit(value);
                }
            }
            Self::WebMercator { radius } => visit(radius),
            Self::EllipsoidalMercator {
                semimajor,
                eccentricity_squared,
            } => {
                visit(semimajor);
                visit(eccentricity_squared);
            }
            Self::MercatorToGeographic {
                radius,
                eccentricity_squared,
                ..
            } => {
                visit(radius);
                visit(eccentricity_squared);
            }
            Self::GeographicToGeocentric { ellipsoid: value }
            | Self::GeocentricToGeographic { ellipsoid: value } => ellipsoid(value, visit),
            Self::TransverseMercator(parameters) | Self::TransverseMercatorInverse(parameters) => {
                ellipsoid(&parameters.ellipsoid, visit);
                for value in [
                    &parameters.central_meridian,
                    &parameters.scale,
                    &parameters.false_easting,
                    &parameters.false_northing,
                ] {
                    visit(value);
                }
            }
            Self::Helmert(parameters) => {
                for value in parameters
                    .translation
                    .iter()
                    .chain(&parameters.rotation_arcseconds)
                    .chain(&parameters.pivot)
                    .chain(core::iter::once(&parameters.scale_ppm))
                {
                    visit(value);
                }
                if let Some(rates) = &parameters.rates {
                    for value in rates
                        .translation
                        .iter()
                        .chain(&rates.rotation_arcseconds)
                        .chain([&rates.epoch, &rates.scale_ppm])
                    {
                        visit(value);
                    }
                }
            }
            Self::Similarity2d(parameters) => {
                for value in parameters
                    .translation
                    .iter()
                    .chain([&parameters.scale, &parameters.rotation_degrees])
                {
                    visit(value);
                }
            }
            Self::Polynomial2d(value) => polynomial(value, visit),
            Self::BilinearGrid(value) => grid(value, visit),
            Self::Polynomial2dInverse(parameters) => {
                polynomial(&parameters.polynomial, visit);
                for value in parameters
                    .source_domain
                    .lower()
                    .iter()
                    .chain(parameters.source_domain.upper())
                {
                    visit(value);
                }
            }
            Self::BilinearGridInverse(parameters) => {
                grid(&parameters.grid, visit);
                for value in parameters
                    .source_domain
                    .lower()
                    .iter()
                    .chain(parameters.source_domain.upper())
                {
                    visit(value);
                }
            }
            Self::Bd09LlV1 | Self::Bd09LlInverseV1 | Self::BaiduMercatorAnalyticV1 => {}
        }
    }
    pub(super) fn certify_quantized_fixed(
        &self,
        input: &[FixedInterval],
        output: &[Rat],
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), MathError> {
        match self {
            Self::GcjRationalHarmonicInverseV1(domain) => super::gcj_inverse::certify_quantized(
                domain,
                &[input[0].clone(), input[1].clone()],
                output,
                math,
            ),
            Self::Bd09LlInverseV1 => super::inverse::bd09_residual(input, output, math),
            Self::MercatorToGeographic {
                radius,
                eccentricity_squared,
                ..
            } => super::inverse::projected_residual(
                input,
                output,
                radius,
                eccentricity_squared,
                math,
            ),
            Self::GeocentricToGeographic { ellipsoid } => {
                super::inverse::geocentric_residual(input, output, ellipsoid, math)
            }
            Self::TransverseMercatorInverse(parameters) => {
                super::transverse_inverse::residual(parameters, input, output, math, progress)
            }
            Self::Polynomial2dInverse(parameters) => {
                parameters.certify(input, output, math, progress)
            }
            Self::BilinearGridInverse(parameters) => {
                parameters.certify(input, output, math, progress)
            }
            _ => Ok(()),
        }
    }

    /// Complete unrounded image and an optional whole-panel XY derivative.
    /// A missing derivative denotes an actual singularity or a model whose
    /// derivative is supplied by its dedicated original implementation.
    pub(super) fn differential(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        limits: OperationSolverLimits,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(OperationCoordinates, Option<RootJacobian2>), MathError> {
        let image = self.evaluate_fixed(point, coordinates, false, limits, math, progress)?;
        let jacobian = match self {
            Self::GcjRationalHarmonicV1(_) => {
                super::gcj_inverse::forward_jacobian(coordinates, math)?
            }
            Self::GcjRationalHarmonicInverseV1(_) => {
                super::gcj_inverse::forward_jacobian(&image, math)?
                    .map(|jacobian| purrdf_xsd::math::invert_jacobian2(&jacobian, math))
                    .transpose()?
            }
            Self::Bd09LlV1 => bd09_parts(coordinates, true, math)?.1,
            Self::Bd09LlInverseV1 => bd09_parts(&image, true, math)?
                .1
                .map(|jacobian| purrdf_xsd::math::invert_jacobian2(&jacobian, math))
                .transpose()?,
            Self::Polynomial2d(polynomial) => {
                Some(polynomial.differential(coordinates, math, progress)?.1)
            }
            Self::BilinearGrid(grid) => Some(grid.differential(coordinates, math, progress)?.1),
            Self::Polynomial2dInverse(parameters) => Some(purrdf_xsd::math::invert_jacobian2(
                &parameters
                    .polynomial
                    .differential(&image, math, progress)?
                    .1,
                math,
            )?),
            Self::BilinearGridInverse(parameters) => Some(purrdf_xsd::math::invert_jacobian2(
                &parameters.grid.differential(&image, math, progress)?.1,
                math,
            )?),
            _ => None,
        };
        progress.math_poll(math)?;
        Ok((image, jacobian))
    }

    pub(super) fn evaluate(
        &self,
        point: &OperationPoint,
        limits: OperationSolverLimits,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<super::TransformEnclosure, MathError> {
        let mut coordinates: OperationCoordinates = purrdf_core::smallvec![
            fixed_from_rat(&point.x, math)?,
            fixed_from_rat(&point.y, math)?,
        ];
        if let Some(z) = &point.z {
            coordinates.push(fixed_from_rat(z, math)?);
        }
        let output = self.evaluate_fixed(point, &coordinates, true, limits, math, progress)?;
        Ok(super::TransformEnclosure {
            output,
            certification_input: coordinates,
        })
    }
    pub(super) fn evaluate_fixed(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        source_is_exact: bool,
        limits: OperationSolverLimits,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<OperationCoordinates, MathError> {
        if matches!(
            self,
            Self::Helmert(_)
                | Self::GeographicToGeocentric { .. }
                | Self::GeocentricToGeographic { .. }
        ) && coordinates.len() < 3
        {
            return Err(MathError::Domain("an actual third coordinate is required"));
        }
        if !source_is_exact && self.units().0 == CoordinateUnit::Degrees {
            for (axis, bound) in [(0, 180), (1, 90)] {
                let (lower, upper) = exact_bounds(&coordinates[axis]);
                let maximum = Rat::from_i64(bound);
                let minimum = maximum.neg();
                if lower > maximum || upper < minimum {
                    return Err(MathError::Domain(
                        "intermediate angular coordinate outside its range",
                    ));
                }
                if lower < minimum || upper > maximum {
                    return Err(MathError::PrecisionExhausted);
                }
            }
        }
        if !source_is_exact
            && matches!(
                self,
                Self::BaiduMercatorAnalyticV1
                    | Self::EllipsoidalMercator { .. }
                    | Self::WebMercator { .. }
            )
        {
            let (latitude_lower, latitude_upper) = exact_bounds(&coordinates[1]);
            if latitude_lower >= Rat::from_i64(90) || latitude_upper <= Rat::from_i64(-90) {
                return Err(MathError::Domain("Mercator is undefined at a pole"));
            }
            if latitude_lower <= Rat::from_i64(-90) || latitude_upper >= Rat::from_i64(90) {
                return Err(MathError::PrecisionExhausted);
            }
        }
        progress.math_poll(math)?;
        let output = match self {
            Self::GcjRationalHarmonicV1(domain) => {
                let (xlo, xhi) = exact_bounds(&coordinates[0]);
                let (ylo, yhi) = exact_bounds(&coordinates[1]);
                if !source_is_exact
                    && (!domain.contains(&xlo, &ylo) || !domain.contains(&xhi, &yhi))
                {
                    return Err(MathError::PrecisionExhausted);
                }
                gcj(coordinates, math)?
            }
            Self::GcjRationalHarmonicInverseV1(domain) => {
                super::gcj_inverse::evaluate(domain, coordinates, limits, math, progress)?
            }
            Self::Bd09LlV1 => bd09(coordinates, math)?,
            Self::Bd09LlInverseV1 => super::inverse::bd09(
                coordinates,
                limits.iterations,
                limits.quantize_inverse,
                math,
                progress,
            )?,
            Self::EllipsoidalMercator {
                semimajor,
                eccentricity_squared,
            } => mercator(coordinates, semimajor, eccentricity_squared, false, math)?,
            Self::BaiduMercatorAnalyticV1 => {
                math.admit_exact_cost(
                    super::baidu_mercator_parameters_cost().ok_or(MathError::PrecisionExhausted)?,
                )?;
                progress.math_poll(math)?;
                let (a, e2) = super::baidu_mercator_parameters();
                mercator(coordinates, &a, &e2, false, math)?
            }
            Self::WebMercator { radius } => {
                mercator(coordinates, radius, &Rat::zero(), true, math)?
            }
            Self::GeographicToGeocentric { ellipsoid } => geocentric(coordinates, ellipsoid, math)?,
            Self::MercatorToGeographic {
                radius,
                eccentricity_squared,
                square_domain,
            } => super::inverse::mercator(
                coordinates,
                (radius, eccentricity_squared),
                *square_domain,
                limits.iterations,
                limits.quantize_inverse,
                math,
                progress,
            )?,
            Self::GeocentricToGeographic { ellipsoid } => super::inverse::geocentric(
                coordinates,
                ellipsoid,
                limits.iterations,
                limits.quantize_inverse,
                math,
                progress,
            )?,
            Self::TransverseMercator(parameters) => {
                parameters.evaluate(point, coordinates, source_is_exact, math, progress)?
            }
            Self::TransverseMercatorInverse(parameters) => super::transverse_inverse::inverse(
                parameters,
                point,
                coordinates,
                limits,
                math,
                progress,
            )?,
            Self::Helmert(parameters) => parameters.evaluate(point, coordinates, math)?,
            Self::Similarity2d(parameters) => parameters.evaluate(coordinates, math)?,
            Self::Polynomial2d(polynomial) => polynomial.evaluate(coordinates, math, progress)?,
            Self::BilinearGrid(grid) => {
                grid.evaluate_declared(point, coordinates, source_is_exact, math, progress)?
            }
            Self::Polynomial2dInverse(parameters) => {
                parameters.evaluate(point, coordinates, source_is_exact, limits, math, progress)?
            }
            Self::BilinearGridInverse(parameters) => {
                parameters.evaluate(point, coordinates, source_is_exact, limits, math, progress)?
            }
        };
        progress.math_poll(math)?;
        Ok(output)
    }
}

fn harmonic(
    value: &FixedInterval,
    frequency: &str,
    amplitude: i64,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let angle = value
        .mul(&FixedInterval::pi(math)?, math)?
        .mul(&decimal(frequency, math)?, math)?;
    angle
        .sin_cos_range(math)?
        .0
        .mul(&FixedInterval::from_i64(amplitude, math)?, math)
}

pub(super) fn gcj(
    coordinates: &[FixedInterval],
    math: &mut CoordinateMath,
) -> Result<OperationCoordinates, MathError> {
    let x = coordinates[0].sub(&FixedInterval::from_i64(105, math)?, math)?;
    let root = x.abs(math)?.sqrt(math)?;
    gcj_with_root(coordinates, &x, &root, math)
}

// Cusp charts supply the exact identity sqrt(|longitude-105|)=u. The
// polynomial, harmonic and metric-factor bodies remain shared with forward GCJ.
pub(super) fn gcj_with_root(
    coordinates: &[FixedInterval],
    x: &FixedInterval,
    root: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<OperationCoordinates, MathError> {
    let [longitude, latitude] = gcj_corrections(x, &coordinates[1], root, math)?;
    let [longitude_scale, latitude_scale] = gcj_scales(&coordinates[1], math)?;
    let mut output: OperationCoordinates = purrdf_core::smallvec![
        coordinates[0].add(&longitude.mul(&longitude_scale, math)?, math)?,
        coordinates[1].add(&latitude.mul(&latitude_scale, math)?, math)?,
    ];
    if let Some(z) = coordinates.get(2) {
        output.push(z.clone());
    }
    Ok(output)
}

pub(super) fn gcj_corrections(
    x: &FixedInterval,
    latitude_degrees: &FixedInterval,
    root: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<[FixedInterval; 2], MathError> {
    let y = latitude_degrees.sub(&FixedInterval::from_i64(35, math)?, math)?;
    let xy = x.mul(&y, math)?;
    let two = FixedInterval::from_i64(2, math)?;
    let third = two.div(&FixedInterval::from_i64(3, math)?, math)?;
    let mut latitude = FixedInterval::from_i64(-100, math)?
        .add(&x.mul(&two, math)?, math)?
        .add(&y.mul(&FixedInterval::from_i64(3, math)?, math)?, math)?
        .add(&y.square(math)?.mul(&decimal("0.2", math)?, math)?, math)?
        .add(&xy.mul(&decimal("0.1", math)?, math)?, math)?
        .add(&root.mul(&decimal("0.2", math)?, math)?, math)?;
    let mut longitude = FixedInterval::from_i64(300, math)?
        .add(x, math)?
        .add(&y.mul(&two, math)?, math)?
        .add(&x.square(math)?.mul(&decimal("0.1", math)?, math)?, math)?
        .add(&xy.mul(&decimal("0.1", math)?, math)?, math)?
        .add(&root.mul(&decimal("0.1", math)?, math)?, math)?;
    let shared = harmonic(x, "6", 20, math)?
        .add(&harmonic(x, "2", 20, math)?, math)?
        .mul(&third, math)?;
    latitude = latitude.add(&shared, math)?;
    longitude = longitude.add(&shared, math)?;
    // Rational frequencies are constructed directly, never fitted coefficients.
    for (coordinate, target, pairs) in [
        (
            &y,
            &mut latitude,
            [(1, 1, 20), (1, 3, 40), (1, 12, 160), (1, 30, 320)],
        ),
        (
            x,
            &mut longitude,
            [(1, 1, 20), (1, 3, 40), (1, 12, 150), (1, 30, 300)],
        ),
    ] {
        let mut sum = FixedInterval::from_i64(0, math)?;
        for (numerator, denominator, amplitude) in pairs {
            let frequency = Rat::from_i64(numerator)
                .div(&Rat::from_i64(denominator))
                .expect("positive");
            let angle = coordinate
                .mul(&FixedInterval::pi(math)?, math)?
                .mul(&fixed_from_rat(&frequency, math)?, math)?;
            sum = sum.add(
                &angle
                    .sin_cos(math)?
                    .0
                    .mul(&FixedInterval::from_i64(amplitude, math)?, math)?,
                math,
            )?;
        }
        *target = target.add(&sum.mul(&third, math)?, math)?;
    }
    Ok([longitude, latitude])
}

pub(super) fn gcj_scales(
    latitude_degrees: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<[FixedInterval; 2], MathError> {
    let phi = radians(latitude_degrees, math)?;
    let (sine, cosine) = phi.sin_cos_range(math)?;
    let e2 = decimal("0.00669342162296594323", math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let w = one
        .sub(&e2.mul(&sine.square(math)?, math)?, math)?
        .sqrt(math)?;
    let a = FixedInterval::from_i64(6_378_245, math)?;
    let n = a.div(&w, math)?;
    let m = a
        .mul(&one.sub(&e2, math)?, math)?
        .div(&w.square(math)?.mul(&w, math)?, math)?;
    Ok([
        degrees(&one.div(&n.mul(&cosine, math)?, math)?, math)?,
        degrees(&one.div(&m, math)?, math)?,
    ])
}

pub(super) fn bd09(
    coordinates: &[FixedInterval],
    math: &mut CoordinateMath,
) -> Result<OperationCoordinates, MathError> {
    let offset = bd09_offset(coordinates, math)?;
    bd09_image(coordinates, &offset, math)
}

fn bd09_image(
    coordinates: &[FixedInterval],
    offset: &[FixedInterval; 2],
    math: &mut CoordinateMath,
) -> Result<OperationCoordinates, MathError> {
    let mut output: OperationCoordinates = purrdf_core::smallvec![
        coordinates[0]
            .add(&offset[0], math)?
            .add(&decimal("0.0065", math)?, math)?,
        coordinates[1]
            .add(&offset[1], math)?
            .add(&decimal("0.006", math)?, math)?,
    ];
    if let Some(z) = coordinates.get(2) {
        output.push(z.clone());
    }
    Ok(output)
}

// F(p)-p without subtracting dependent interval copies of p. The original
// Cartesian rotation and radial displacement are evaluated in one home.
pub(super) fn bd09_offset(
    coordinates: &[FixedInterval],
    math: &mut CoordinateMath,
) -> Result<[FixedInterval; 2], MathError> {
    Ok(bd09_parts(coordinates, false, math)?.0)
}

// The original Cartesian recurrence evaluates both the offset and its exact
// analytic derivative. Derivative callers share the same constants and trig
// reductions; the origin enclosure has no invented differentiability claim.
fn bd09_parts(
    coordinates: &[FixedInterval],
    derivative: bool,
    math: &mut CoordinateMath,
) -> Result<([FixedInterval; 2], Option<RootJacobian2>), MathError> {
    let x = &coordinates[0];
    let y = &coordinates[1];
    if x.lower().is_zero() && x.upper().is_zero() && y.lower().is_zero() && y.upper().is_zero() {
        return Ok((
            [
                FixedInterval::from_i64(0, math)?,
                FixedInterval::from_i64(0, math)?,
            ],
            None,
        ));
    }
    let k = FixedInterval::pi(math)?
        .mul(&FixedInterval::from_i64(3000, math)?, math)?
        .div(&FixedInterval::from_i64(180, math)?, math)?;
    let radius = x.square(math)?.add(&y.square(math)?, math)?.sqrt(math)?;
    let amplitude = decimal("0.00002", math)?;
    let (ky_sine, ky_cosine) = k.mul(y, math)?.sin_cos_range(math)?;
    let d = ky_sine.mul(&amplitude, math)?;
    let ratio = if radius.lower() > &purrdf_xsd::BigInt::zero() {
        d.div(&radius, math)?
    } else {
        // |sin(k*y)| <= k*|y| <= k*r also encloses the continuous origin.
        let bound = amplitude.mul(&k, math)?;
        FixedInterval::from_bounds(bound.upper().negated(), bound.upper().clone(), math)?
    };
    let (kx_sine, kx_cosine) = k.mul(x, math)?.sin_cos_range(math)?;
    let rotation_amplitude = decimal("0.000003", math)?;
    let angle = kx_cosine.mul(&rotation_amplitude, math)?;
    let (sine, cosine) = angle.sin_cos_range(math)?;
    let cosine_minus_one = cosine.sub(&FixedInterval::from_i64(1, math)?, math)?;
    let rotated_x = x.mul(&cosine, math)?.sub(&y.mul(&sine, math)?, math)?;
    let rotated_y = x.mul(&sine, math)?.add(&y.mul(&cosine, math)?, math)?;
    let offset = [
        x.mul(&cosine_minus_one, math)?
            .sub(&y.mul(&sine, math)?, math)?
            .add(&ratio.mul(&rotated_x, math)?, math)?,
        x.mul(&sine, math)?
            .add(&y.mul(&cosine_minus_one, math)?, math)?
            .add(&ratio.mul(&rotated_y, math)?, math)?,
    ];
    let jacobian = if derivative && radius.lower() > &purrdf_xsd::BigInt::zero() {
        let one = FixedInterval::from_i64(1, math)?;
        let radial_scale = one.add(&ratio, math)?;
        let inverse_radius_cubed = one.div(&radius.square(math)?.mul(&radius, math)?, math)?;
        let radial_x = d
            .mul(x, math)?
            .mul(&inverse_radius_cubed, math)?
            .neg(math)?;
        let radial_y = amplitude
            .mul(&k, math)?
            .mul(&ky_cosine, math)?
            .div(&radius, math)?
            .sub(&d.mul(y, math)?.mul(&inverse_radius_cubed, math)?, math)?;
        let angle_x = rotation_amplitude
            .mul(&k, math)?
            .mul(&kx_sine, math)?
            .neg(math)?;
        let rotated_x_x = cosine.sub(&angle_x.mul(&rotated_y, math)?, math)?;
        let rotated_y_x = sine.add(&angle_x.mul(&rotated_x, math)?, math)?;
        Some([
            [
                radial_x
                    .mul(&rotated_x, math)?
                    .add(&radial_scale.mul(&rotated_x_x, math)?, math)?,
                radial_y
                    .mul(&rotated_x, math)?
                    .sub(&radial_scale.mul(&sine, math)?, math)?,
            ],
            [
                radial_x
                    .mul(&rotated_y, math)?
                    .add(&radial_scale.mul(&rotated_y_x, math)?, math)?,
                radial_y
                    .mul(&rotated_y, math)?
                    .add(&radial_scale.mul(&cosine, math)?, math)?,
            ],
        ])
    } else {
        None
    };
    Ok((offset, jacobian))
}

pub(super) fn mercator(
    coordinates: &[FixedInterval],
    a: &Rat,
    e2: &Rat,
    square: bool,
    math: &mut CoordinateMath,
) -> Result<OperationCoordinates, MathError> {
    let lambda = radians(&coordinates[0], math)?;
    let phi = radians(&coordinates[1], math)?;
    let (sine, cosine) = phi.sin_cos(math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let mut psi = one.add(&sine, math)?.div(&cosine, math)?.log(math)?;
    if !e2.is_zero() {
        let e = fixed_from_rat(e2, math)?.sqrt(math)?;
        let es = e.mul(&sine, math)?;
        let correction = one
            .sub(&es, math)?
            .div(&one.add(&es, math)?, math)?
            .log(math)?
            .mul(&e, math)?
            .div(&FixedInterval::from_i64(2, math)?, math)?;
        psi = psi.add(&correction, math)?;
    }
    if square {
        let pi = FixedInterval::pi(math)?;
        let absolute = psi.abs(math)?;
        if absolute.lower() > pi.upper() {
            return Err(MathError::Domain(
                "Web Mercator latitude outside the square domain",
            ));
        }
        if absolute.upper() > pi.lower() {
            return Err(MathError::PrecisionExhausted);
        }
    }
    let radius = fixed_from_rat(a, math)?;
    let mut output: OperationCoordinates =
        purrdf_core::smallvec![lambda.mul(&radius, math)?, psi.mul(&radius, math)?];
    if let Some(z) = coordinates.get(2) {
        output.push(z.clone());
    }
    Ok(output)
}

pub(super) fn geocentric(
    coordinates: &[FixedInterval],
    ellipsoid: &PreparedEllipsoid,
    math: &mut CoordinateMath,
) -> Result<OperationCoordinates, MathError> {
    let (sin_lon, cos_lon, sine, cosine) = geocentric_angles(coordinates, math)?;
    let one = FixedInterval::from_i64(1, math)?;
    let e2 = fixed_from_rat(&ellipsoid.eccentricity_squared(), math)?;
    let n = fixed_from_rat(ellipsoid.semimajor(), math)?.div(
        &one.sub(&e2.mul(&sine.square(math)?, math)?, math)?
            .sqrt(math)?,
        math,
    )?;
    let height = coordinates
        .get(2)
        .ok_or(MathError::Domain("actual height is required"))?;
    let radius = n.add(height, math)?.mul(&cosine, math)?;
    Ok(purrdf_core::smallvec![
        radius.mul(&cos_lon, math)?,
        radius.mul(&sin_lon, math)?,
        n.mul(&one.sub(&e2, math)?, math)?
            .add(height, math)?
            .mul(&sine, math)?,
    ])
}

/// Whole angular image shared by Cartesian positions and their differentials.
/// Exact pole decisions precede trigonometry and retain the written frame.
pub(super) fn geocentric_angles(
    coordinates: &[FixedInterval],
    math: &mut CoordinateMath,
) -> Result<(FixedInterval, FixedInterval, FixedInterval, FixedInterval), MathError> {
    let lambda = radians(&coordinates[0], math)?;
    let phi = radians(&coordinates[1], math)?;
    let (mut sine, mut cosine) = phi.sin_cos_range(math)?;
    let (lower, upper) = exact_bounds(&coordinates[1]);
    if lower == upper && lower.abs() == Rat::from_i64(90) {
        sine = FixedInterval::from_i64(i64::from(lower.signum()), math)?;
        cosine = FixedInterval::from_i64(0, math)?;
    }
    let (sin_lon, cos_lon) = lambda.sin_cos_range(math)?;
    Ok((sin_lon, cos_lon, sine, cosine))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_xsd::math::MathLimits;

    #[test]
    fn bd_cartesian_derivative_encloses_independent_endpoint_secants() {
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).expect("math");
        let lower = decimal("116.397", &mut math).expect("lower");
        let upper = decimal("116.3970001", &mut math).expect("upper");
        let latitude = decimal("39.908", &mut math).expect("latitude");
        let panel = [
            lower.hull(&upper, &mut math).expect("panel"),
            latitude.clone(),
        ];
        let jacobian = bd09_parts(&panel, true, &mut math)
            .expect("differential")
            .1
            .expect("smooth away from origin");
        let first = bd09(&[lower.clone(), latitude.clone()], &mut math).expect("first");
        let last = bd09(&[upper.clone(), latitude], &mut math).expect("last");
        let step = upper.sub(&lower, &mut math).expect("step");
        for row in 0..2 {
            let secant = last[row]
                .sub(&first[row], &mut math)
                .expect("difference")
                .div(&step, &mut math)
                .expect("secant");
            assert!(
                jacobian[row][0]
                    .intersection(&secant, &mut math)
                    .expect("comparison")
                    .is_some()
            );
        }
        let zero = FixedInterval::from_i64(0, &mut math).expect("origin");
        assert_eq!(
            bd09_parts(&[zero.clone(), zero], true, &mut math)
                .expect("origin law")
                .1,
            None
        );
    }

    #[test]
    fn gcj_derivative_keeps_cusp_singularity_explicit() {
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).expect("math");
        let west = decimal("104.999", &mut math).expect("west");
        let east = decimal("105.001", &mut math).expect("east");
        let latitude = decimal("35", &mut math).expect("latitude");
        let cusp = [
            west.hull(&east, &mut math).expect("cusp panel"),
            latitude.clone(),
        ];
        assert_eq!(
            super::super::gcj_inverse::forward_jacobian(&cusp, &mut math)
                .expect("singularity classified"),
            None
        );
        let lower = decimal("116.397", &mut math).expect("lower");
        let upper = decimal("116.3970001", &mut math).expect("upper");
        let panel = [
            lower.hull(&upper, &mut math).expect("panel"),
            latitude.clone(),
        ];
        let jacobian = super::super::gcj_inverse::forward_jacobian(&panel, &mut math)
            .expect("differential")
            .expect("smooth chart");
        let first = gcj(&[lower.clone(), latitude.clone()], &mut math).expect("first");
        let last = gcj(&[upper.clone(), latitude], &mut math).expect("last");
        let step = upper.sub(&lower, &mut math).expect("step");
        for row in 0..2 {
            let secant = last[row]
                .sub(&first[row], &mut math)
                .expect("difference")
                .div(&step, &mut math)
                .expect("secant");
            assert!(
                jacobian[row][0]
                    .intersection(&secant, &mut math)
                    .expect("comparison")
                    .is_some()
            );
        }
    }
}
