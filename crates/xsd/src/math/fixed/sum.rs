// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact endpoint sums for retained certified quadrature panels.

use super::FixedInterval;
use crate::{
    BigInt,
    math::{CoordinateMath, MathError},
};

/// A fixed-grid sum of certified panels without arithmetic widening.
///
/// Each retained panel contributes its lower endpoint to the lower sum and its
/// upper endpoint to the upper sum. Removing a panel subtracts those same
/// endpoints; it does not perform interval subtraction. The caller must remove
/// only a panel previously added and not already removed. Panel membership is
/// owned by the caller's traversal, so this accumulator retains no second list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedIntervalSum {
    lower: BigInt,
    upper: BigInt,
    precision_bits: u32,
    panels: u64,
}
impl FixedIntervalSum {
    /// Begin an empty sum on the context's fixed grid.
    ///
    /// # Errors
    /// Refuses exhausted numerical work or workspace.
    pub fn new(context: &mut CoordinateMath) -> Result<Self, MathError> {
        context.admit_exact(1, context.limits().precision_bits as usize)?;
        Ok(Self {
            lower: BigInt::zero(),
            upper: BigInt::zero(),
            precision_bits: context.limits().precision_bits,
            panels: 0,
        })
    }
    /// Add one retained panel's certified endpoints exactly.
    ///
    /// # Errors
    /// Refuses a different fixed grid, panel-count overflow, or numerical limits
    /// before allocating endpoint sums. The previous sum survives a refusal.
    pub fn add(
        &mut self,
        panel: &FixedInterval,
        context: &mut CoordinateMath,
    ) -> Result<(), MathError> {
        self.update(panel, false, context)
    }
    /// Remove the same panel endpoints previously added by the caller.
    ///
    /// # Errors
    /// Adds empty-sum or inconsistent-endpoint refusal to [`Self::add`].
    pub fn remove(
        &mut self,
        panel: &FixedInterval,
        context: &mut CoordinateMath,
    ) -> Result<(), MathError> {
        self.update(panel, true, context)
    }
    fn update(
        &mut self,
        panel: &FixedInterval,
        remove: bool,
        context: &mut CoordinateMath,
    ) -> Result<(), MathError> {
        if self.precision_bits != context.limits().precision_bits
            || self.precision_bits != panel.precision_bits()
        {
            return Err(MathError::Domain("different fixed grids in interval sum"));
        }
        let panels = if remove {
            self.panels
                .checked_sub(1)
                .ok_or(MathError::Domain("remove from empty interval sum"))?
        } else {
            self.panels
                .checked_add(1)
                .ok_or(MathError::Domain("interval panel count overflow"))?
        };
        let bits = [&self.lower, &self.upper, panel.lower(), panel.upper()]
            .into_iter()
            .map(BigInt::bits_upper_bound)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        context.admit_exact(2, bits)?;
        let lower = if remove {
            self.lower.sub(panel.lower())
        } else {
            self.lower.add(panel.lower())
        };
        let upper = if remove {
            self.upper.sub(panel.upper())
        } else {
            self.upper.add(panel.upper())
        };
        if lower > upper || (panels == 0 && (!lower.is_zero() || !upper.is_zero())) {
            return Err(MathError::Domain("inconsistent retained interval panels"));
        }
        self.lower = lower;
        self.upper = upper;
        self.panels = panels;
        Ok(())
    }
    /// Complete retained-panel count, independent of the sum's sign.
    #[must_use]
    pub const fn panel_count(&self) -> u64 {
        self.panels
    }
    /// The complete certified sum enclosure on the same fixed grid.
    ///
    /// # Errors
    /// Refuses an empty sum, a different context grid, or numerical limits.
    pub fn enclosure(&self, context: &mut CoordinateMath) -> Result<FixedInterval, MathError> {
        if self.panels == 0 {
            return Err(MathError::Domain("empty interval sum has no enclosure"));
        }
        if self.precision_bits != context.limits().precision_bits {
            return Err(MathError::Domain("different fixed grids in interval sum"));
        }
        context.admit_exact(
            1,
            self.lower
                .bits_upper_bound()
                .max(self.upper.bits_upper_bound()),
        )?;
        FixedInterval::from_bounds(self.lower.clone(), self.upper.clone(), context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::MathLimits;

    #[test]
    fn replacing_retained_panels_preserves_exact_complete_bounds() {
        let mut math = CoordinateMath::new(MathLimits::default()).expect("context");
        let make = |lo, hi, math: &mut CoordinateMath| {
            FixedInterval::from_bounds(BigInt::from_i128(lo), BigInt::from_i128(hi), math)
                .expect("ordered interval")
        };
        let original = make(-19, 23, &mut math);
        let left = make(-11, -7, &mut math);
        let right = make(2, 8, &mut math);
        let other = make(-100, 31, &mut math);
        let mut sum = FixedIntervalSum::new(&mut math).expect("sum");
        assert!(sum.enclosure(&mut math).is_err());
        sum.add(&original, &mut math).expect("first panel");
        sum.add(&other, &mut math).expect("other panel");
        sum.remove(&original, &mut math)
            .expect("replace retained parent");
        sum.add(&left, &mut math).expect("left child");
        sum.add(&right, &mut math).expect("right child");
        assert_eq!(sum.panel_count(), 3);
        assert_eq!(
            sum.enclosure(&mut math).expect("sum"),
            left.add(&right, &mut math)
                .expect("children")
                .add(&other, &mut math)
                .expect("complete sum")
        );
        sum.remove(&left, &mut math).expect("remove left");
        sum.remove(&right, &mut math).expect("remove right");
        sum.remove(&other, &mut math).expect("remove other");
        assert_eq!(sum.panel_count(), 0);
        assert!(sum.remove(&other, &mut math).is_err());
    }

    #[test]
    fn refusal_preserves_accumulator_and_exact_grid() {
        let mut math = CoordinateMath::new(MathLimits::default()).expect("context");
        let panel = FixedInterval::from_i64(1, &mut math).expect("panel");
        let mut sum = FixedIntervalSum::new(&mut math).expect("sum");
        sum.add(&panel, &mut math).expect("panel retained");
        let before = sum.clone();
        let mut exhausted = CoordinateMath::new(MathLimits {
            max_work: 1,
            ..MathLimits::default()
        })
        .expect("admitted empty budget");
        assert_eq!(
            sum.add(&panel, &mut exhausted),
            Err(MathError::WorkExhausted)
        );
        assert_eq!(sum, before);
        let mut other = CoordinateMath::new(MathLimits {
            precision_bits: 160,
            ..MathLimits::default()
        })
        .expect("other grid");
        assert!(
            sum.add(
                &FixedInterval::from_i64(1, &mut other).expect("other panel"),
                &mut math
            )
            .is_err()
        );
        assert_eq!(sum, before);
    }
}
