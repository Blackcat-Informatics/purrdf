// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Row-major controlled interval matrices. Adjugate inversion uses the actual
//! determinant, including nonorthogonal calibrated linear maps.

use super::{CoordinateMath, FixedInterval, MathError};

/// A row-major three-dimensional linear map on the context's dyadic grid.
pub type Matrix3 = [FixedInterval; 9];

/// Enclose the inverse of every nonsingular matrix in the supplied enclosure.
///
/// # Errors
/// Refuses a determinant enclosure containing zero, precision mismatch, and
/// numerical work/workspace exhaustion before an incomplete inverse is returned.
pub fn invert_matrix3(matrix: &Matrix3, math: &mut CoordinateMath) -> Result<Matrix3, MathError> {
    let bytes = matrix
        .iter()
        .try_fold(512_usize, |bytes, value| {
            bytes.checked_add(value.workspace_bytes().checked_mul(12)?)
        })
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(bytes)?;
    let result = (|| {
        let minor = |a: usize, b: usize, c: usize, d: usize, math: &mut CoordinateMath| {
            matrix[a]
                .mul(&matrix[b], math)?
                .sub(&matrix[c].mul(&matrix[d], math)?, math)
        };
        let cofactors = [
            minor(4, 8, 5, 7, math)?,
            minor(5, 6, 3, 8, math)?,
            minor(3, 7, 4, 6, math)?,
            minor(2, 7, 1, 8, math)?,
            minor(0, 8, 2, 6, math)?,
            minor(1, 6, 0, 7, math)?,
            minor(1, 5, 2, 4, math)?,
            minor(2, 3, 0, 5, math)?,
            minor(0, 4, 1, 3, math)?,
        ];
        let determinant = matrix[0]
            .mul(&cofactors[0], math)?
            .add(&matrix[1].mul(&cofactors[1], math)?, math)?
            .add(&matrix[2].mul(&cofactors[2], math)?, math)?;
        let [a, b, c, d, e, f, g, h, i] =
            [0, 3, 6, 1, 4, 7, 2, 5, 8].map(|index| cofactors[index].div(&determinant, math));
        Ok([a?, b?, c?, d?, e?, f?, g?, h?, i?])
    })();
    math.release_workspace(bytes)?;
    result
}

/// Multiply one row-major map by a complete three-coordinate enclosure.
/// The reduction order is column0, column1, column2 on every execution path.
///
/// # Errors
/// Refuses mismatched precision or exhausted numerical admission.
pub fn matrix_vector3(
    matrix: &Matrix3,
    vector: &[FixedInterval; 3],
    math: &mut CoordinateMath,
) -> Result<[FixedInterval; 3], MathError> {
    let row = |row: usize, math: &mut CoordinateMath| {
        matrix[3 * row]
            .mul(&vector[0], math)?
            .add(&matrix[3 * row + 1].mul(&vector[1], math)?, math)?
            .add(&matrix[3 * row + 2].mul(&vector[2], math)?, math)
    };
    Ok([row(0, math)?, row(1, math)?, row(2, math)?])
}

#[cfg(test)]
mod tests {
    use super::super::MathLimits;
    use super::*;
    use crate::BigInt;

    #[test]
    fn nonorthogonal_inverse_is_certified_and_singular_matrices_refuse() {
        let mut math = CoordinateMath::new(MathLimits::DEFAULT).unwrap();
        let rational = |n, d, math: &mut CoordinateMath| {
            FixedInterval::from_ratio(&BigInt::from_i128(n), &BigInt::from_i128(d), math).unwrap()
        };
        let matrix = [
            rational(1, 1, &mut math),
            rational(-3, 10, &mut math),
            rational(2, 10, &mut math),
            rational(3, 10, &mut math),
            rational(1, 1, &mut math),
            rational(-1, 10, &mut math),
            rational(-2, 10, &mut math),
            rational(1, 10, &mut math),
            rational(1, 1, &mut math),
        ];
        let inverse = invert_matrix3(&matrix, &mut math).unwrap();
        let one = FixedInterval::from_i64(1, &mut math).unwrap();
        let zero = FixedInterval::from_i64(0, &mut math).unwrap();
        assert!(inverse[0].upper() < one.lower());
        for column in 0..3 {
            let vector = [
                inverse[column].clone(),
                inverse[3 + column].clone(),
                inverse[6 + column].clone(),
            ];
            let result = matrix_vector3(&matrix, &vector, &mut math).unwrap();
            for (row, value) in result.iter().enumerate() {
                let expected = if row == column { &one } else { &zero };
                assert!(value.lower() <= expected.lower() && value.upper() >= expected.upper());
            }
        }
        let mut singular = matrix;
        for column in 0..3 {
            singular[3 + column] = singular[column].clone();
        }
        assert_eq!(
            invert_matrix3(&singular, &mut math),
            Err(MathError::PrecisionExhausted)
        );
    }
}
