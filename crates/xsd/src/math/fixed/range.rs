// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete real interval trigonometric ranges, including multiple periods.

use super::{CoordinateMath, FixedInterval, MathError};
use crate::BigInt;

impl FixedInterval {
    /// Enclose sine and cosine over a complete real argument interval. A wide
    /// interval uses the global derivative bound one around an exact dyadic
    /// midpoint, clamped to the exact unit range. This is a proven enclosure;
    /// it does not infer extrema from sampled values.
    ///
    /// # Errors
    /// Refuses mismatched precision, insufficient angular reduction precision,
    /// or exhausted numerical resources.
    pub fn sin_cos_range(&self, math: &mut CoordinateMath) -> Result<(Self, Self), MathError> {
        // A precision mismatch must not be mistaken for a wide-angle
        // reduction refusal and reinterpreted on a different dyadic grid.
        self.admit(None, math, 1)?;
        match self.sin_cos(math) {
            Ok(value) => return Ok(value),
            Err(MathError::PrecisionExhausted) => {}
            Err(error) => return Err(error),
        }
        let two = BigInt::from_i128(2);
        let midpoint = math
            .integer_div_rem(&math.integer_add(self.lower(), self.upper())?, &two)?
            .ok_or(MathError::Domain("zero range midpoint divisor"))?
            .0;
        let radius = math
            .integer_sub(self.upper(), &midpoint)?
            .abs()
            .max(math.integer_sub(self.lower(), &midpoint)?.abs());
        let one = Self::from_i64(1, math)?.upper().clone();
        if radius >= one {
            let full = Self::from_bounds(one.negated(), one, math)?;
            return Ok((full.clone(), full));
        }
        let point = Self::from_bounds(midpoint.clone(), midpoint, math)?;
        let (sine, cosine) = point.sin_cos(math)?;
        let expand = |value: &Self, math: &mut CoordinateMath| {
            Self::from_bounds(
                math.integer_sub(value.lower(), &radius)?.max(one.negated()),
                math.integer_add(value.upper(), &radius)?.min(one.clone()),
                math,
            )
        };
        Ok((expand(&sine, math)?, expand(&cosine, math)?))
    }
}
