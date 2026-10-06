// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit affine rotation laws; inverse uses the actual matrix, including small angles.

use super::OperationCoordinates;

use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

use super::{OperationPoint, radians, rational_fields};
use crate::numerical::fixed_from_rat;
use crate::{GeoError, Rat};

/// The calibrated rotation law; choosing another law changes the mathematical model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RotationLaw {
    /// `I + [omega]x`, with no substitution of exact rotations.
    HelmertSmallAngleV1,
    /// Right-handed active matrix `Rz Ry Rx`.
    HelmertEulerRzRyRxV1,
}

/// Explicit active-vector or passive-frame convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RotationConvention {
    /// Active right-handed rotation of position vectors.
    PositionVector,
    /// Full matrix transpose of the position-vector law.
    CoordinateFrame,
}

/// Per exact decimal year rates, in metres, arcseconds and ppm respectively.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HelmertRates {
    /// Reference epoch as an exact decimal year.
    pub epoch: Rat,
    /// Translation rates in metres/year.
    pub translation: [Rat; 3],
    /// Rotation rates in arcseconds/year.
    pub rotation_arcseconds: [Rat; 3],
    /// Scale rate in ppm/year.
    pub scale_ppm: Rat,
}

/// Caller-supplied seven parameters with an explicit pivot, calibrated law and convention.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HelmertParameters {
    /// Translation in metres.
    pub translation: [Rat; 3],
    /// Rotations in arcseconds; no implicit unit conversion convention.
    pub rotation_arcseconds: [Rat; 3],
    /// Additive scale in parts per million.
    pub scale_ppm: Rat,
    /// Actual Cartesian pivot, in metres.
    pub pivot: [Rat; 3],
    /// Small-angle or exact Euler mathematical law.
    pub law: RotationLaw,
    /// Position vector or coordinate frame convention.
    pub convention: RotationConvention,
    /// Optional rates with mandatory reference and observation epochs.
    pub rates: Option<HelmertRates>,
    /// Invert the actual calibrated affine matrix when true.
    pub inverse: bool,
}

impl HelmertParameters {
    pub(super) fn validate(&self) -> Result<(), GeoError> {
        if self.scale_ppm <= Rat::from_i64(-1_000_000) {
            return Err(GeoError::config("Helmert scale factor must be positive"));
        }
        Ok(())
    }

    pub(super) fn validate_point(&self, point: &OperationPoint) -> Result<(), GeoError> {
        if point.z.is_none() {
            return Err(GeoError::MissingHeight);
        }
        if self.rates.is_some() && point.epoch.is_none() {
            return Err(GeoError::domain(
                "a rate-bearing Helmert requires an actual observation epoch",
            ));
        }
        if self.adjusted(point)?.2 <= Rat::from_i64(-1_000_000) {
            return Err(GeoError::domain(
                "epoch-adjusted Helmert scale factor must be positive",
            ));
        }
        Ok(())
    }

    fn adjusted(&self, point: &OperationPoint) -> Result<([Rat; 3], [Rat; 3], Rat), GeoError> {
        let mut translation = self.translation.clone();
        let mut rotation = self.rotation_arcseconds.clone();
        let mut scale = self.scale_ppm.clone();
        if let Some(rates) = &self.rates {
            let epoch = point
                .epoch
                .as_ref()
                .ok_or_else(|| GeoError::domain("a Helmert observation epoch is required"))?;
            let delta = epoch.sub(&rates.epoch);
            for index in 0..3 {
                translation[index] = translation[index].add(&rates.translation[index].mul(&delta));
                rotation[index] =
                    rotation[index].add(&rates.rotation_arcseconds[index].mul(&delta));
            }
            scale = scale.add(&rates.scale_ppm.mul(&delta));
        }
        Ok((translation, rotation, scale))
    }

    pub(super) fn parameters(&self, fields: &mut Vec<Vec<u8>>) {
        fields.push(vec![
            match self.law {
                RotationLaw::HelmertSmallAngleV1 => 0,
                RotationLaw::HelmertEulerRzRyRxV1 => 1,
            },
            u8::from(self.convention == RotationConvention::CoordinateFrame),
            u8::from(self.inverse),
            u8::from(self.rates.is_some()),
        ]);
        for value in self
            .translation
            .iter()
            .chain(self.rotation_arcseconds.iter())
            .chain([&self.scale_ppm])
            .chain(self.pivot.iter())
        {
            rational_fields(value, fields);
        }
        if let Some(rates) = &self.rates {
            for value in std::iter::once(&rates.epoch)
                .chain(rates.translation.iter())
                .chain(rates.rotation_arcseconds.iter())
                .chain([&rates.scale_ppm])
            {
                rational_fields(value, fields);
            }
        }
    }

    pub(super) fn evaluate(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        math: &mut CoordinateMath,
    ) -> Result<OperationCoordinates, MathError> {
        self.evaluate_with_matrix(point, coordinates, math)
            .map(|(image, _)| image)
    }

    pub(super) fn evaluate_with_matrix(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        math: &mut CoordinateMath,
    ) -> Result<(OperationCoordinates, purrdf_xsd::math::Matrix3), MathError> {
        let (translation, rotation, scale) = self
            .adjusted(point)
            .map_err(|_| MathError::Domain("missing epoch"))?;
        let arcseconds = FixedInterval::from_i64(3600, math)?;
        let mut angles = OperationCoordinates::default();
        for angle in &rotation {
            angles.push(radians(
                &fixed_from_rat(angle, math)?.div(&arcseconds, math)?,
                math,
            )?);
        }
        let mut matrix: purrdf_xsd::math::Matrix3 = match self.law {
            RotationLaw::HelmertSmallAngleV1 => [
                FixedInterval::from_i64(1, math)?,
                angles[2].neg(math)?,
                angles[1].clone(),
                angles[2].clone(),
                FixedInterval::from_i64(1, math)?,
                angles[0].neg(math)?,
                angles[1].neg(math)?,
                angles[0].clone(),
                FixedInterval::from_i64(1, math)?,
            ],
            RotationLaw::HelmertEulerRzRyRxV1 => {
                let (sx, cx) = angles[0].sin_cos(math)?;
                let (sy, cy) = angles[1].sin_cos(math)?;
                let (sz, cz) = angles[2].sin_cos(math)?;
                [
                    cy.mul(&cz, math)?,
                    sx.mul(&sy, math)?
                        .mul(&cz, math)?
                        .sub(&cx.mul(&sz, math)?, math)?,
                    cx.mul(&sy, math)?
                        .mul(&cz, math)?
                        .add(&sx.mul(&sz, math)?, math)?,
                    cy.mul(&sz, math)?,
                    sx.mul(&sy, math)?
                        .mul(&sz, math)?
                        .add(&cx.mul(&cz, math)?, math)?,
                    cx.mul(&sy, math)?
                        .mul(&sz, math)?
                        .sub(&sx.mul(&cz, math)?, math)?,
                    sy.neg(math)?,
                    sx.mul(&cy, math)?,
                    cx.mul(&cy, math)?,
                ]
            }
        };
        if self.convention == RotationConvention::CoordinateFrame {
            for (left, right) in [(1, 3), (2, 6), (5, 7)] {
                matrix.swap(left, right);
            }
        }
        let million = FixedInterval::from_i64(1_000_000, math)?;
        let factor = fixed_from_rat(&scale, math)?
            .div(&million, math)?
            .add(&FixedInterval::from_i64(1, math)?, math)?;
        for value in &mut matrix {
            *value = value.mul(&factor, math)?;
        }
        let mut vector = OperationCoordinates::default();
        for index in 0..3 {
            let pivot = fixed_from_rat(&self.pivot[index], math)?;
            let mut coordinate = coordinates[index].sub(&pivot, math)?;
            if self.inverse {
                coordinate = coordinate.sub(&fixed_from_rat(&translation[index], math)?, math)?;
            }
            vector.push(coordinate);
        }
        if self.inverse {
            matrix = purrdf_xsd::math::invert_matrix3(&matrix, math)?;
        }
        let mut output = OperationCoordinates::default();
        for row in 0..3 {
            let mut sum = FixedInterval::from_i64(0, math)?;
            for column in 0..3 {
                sum = sum.add(&matrix[3 * row + column].mul(&vector[column], math)?, math)?;
            }
            sum = sum.add(&fixed_from_rat(&self.pivot[row], math)?, math)?;
            if !self.inverse {
                sum = sum.add(&fixed_from_rat(&translation[row], math)?, math)?;
            }
            output.push(sum);
        }
        Ok((output, matrix))
    }
}

/// Four-parameter active counterclockwise similarity or its declared frame transpose.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Similarity2d {
    /// Translation in metres.
    pub translation: [Rat; 2],
    /// Strictly positive dimensionless scale factor.
    pub scale: Rat,
    /// Counterclockwise rotation angle in exact degrees.
    pub rotation_degrees: Rat,
    /// Position-vector or transposed frame convention.
    pub convention: RotationConvention,
    /// Invert the actual affine map when true.
    pub inverse: bool,
}

impl Similarity2d {
    pub(super) fn validate(&self) -> Result<(), GeoError> {
        if self.scale <= Rat::zero() {
            return Err(GeoError::config("similarity scale must be positive"));
        }
        Ok(())
    }
    pub(super) fn parameters(&self, fields: &mut Vec<Vec<u8>>) {
        fields.push(vec![
            u8::from(self.convention == RotationConvention::CoordinateFrame),
            u8::from(self.inverse),
        ]);
        for value in self
            .translation
            .iter()
            .chain([&self.scale, &self.rotation_degrees])
        {
            rational_fields(value, fields);
        }
    }
    pub(super) fn evaluate(
        &self,
        coordinates: &[FixedInterval],
        math: &mut CoordinateMath,
    ) -> Result<OperationCoordinates, MathError> {
        let angle = radians(&fixed_from_rat(&self.rotation_degrees, math)?, math)?;
        let (mut sine, cosine) = angle.sin_cos(math)?;
        if self.convention == RotationConvention::CoordinateFrame {
            sine = sine.neg(math)?;
        }
        let scale = fixed_from_rat(&self.scale, math)?;
        let mut x = coordinates[0].clone();
        let mut y = coordinates[1].clone();
        if self.inverse {
            x = x
                .sub(&fixed_from_rat(&self.translation[0], math)?, math)?
                .div(&scale, math)?;
            y = y
                .sub(&fixed_from_rat(&self.translation[1], math)?, math)?
                .div(&scale, math)?;
            sine = sine.neg(math)?;
        }
        let mut xx = x.mul(&cosine, math)?.sub(&y.mul(&sine, math)?, math)?;
        let mut yy = x.mul(&sine, math)?.add(&y.mul(&cosine, math)?, math)?;
        if !self.inverse {
            xx = xx
                .mul(&scale, math)?
                .add(&fixed_from_rat(&self.translation[0], math)?, math)?;
            yy = yy
                .mul(&scale, math)?
                .add(&fixed_from_rat(&self.translation[1], math)?, math)?;
        }
        let mut output: OperationCoordinates = purrdf_core::smallvec![xx, yy];
        if let Some(z) = coordinates.get(2) {
            output.push(z.clone());
        }
        Ok(output)
    }
}
