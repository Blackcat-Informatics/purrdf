// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact ellipsoidal physical scales, including the quantized footprint guard.

use crate::error::GeoError;
use crate::exact::{Int, Rat};
use crate::metric::Metres;
use crate::profile::PreparedEllipsoid;

use super::fixed::{IntegerAdmission, PureAssignment, integer, rational};
use super::{MAX_LEVEL, assignment_admission, validate_level};
use crate::{ExecutionPolicy, MetricWorkObserver};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};

/// Outward edge-scale bounds in metres, using the actual prepared axes.
///
/// Nominal edges are smooth chart edges. The guarded bounds also enclose their
/// endpoint extent under the discrete assignment error; they do not assert a
/// smooth perimeter for a quantized ownership set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellScaleBounds {
    nominal_lower: Metres,
    nominal_upper: Metres,
    footprint_guard: Metres,
    lower: Metres,
    upper: Metres,
}

impl CellScaleBounds {
    /// Outward lower nominal edge bound, with √2 rounded downward exactly.
    #[must_use]
    pub const fn nominal_lower(&self) -> &Metres {
        &self.nominal_lower
    }

    /// Exact rational upper nominal edge bound.
    #[must_use]
    pub const fn nominal_upper(&self) -> &Metres {
        &self.nominal_upper
    }

    /// Metre guard for the two endpoint angular errors, `R × 2^-58`.
    #[must_use]
    pub const fn footprint_guard(&self) -> &Metres {
        &self.footprint_guard
    }

    /// Nonnegative nominal lower bound minus the complete endpoint guard.
    #[must_use]
    pub const fn lower(&self) -> &Metres {
        &self.lower
    }

    /// Nominal upper bound plus the complete endpoint guard.
    #[must_use]
    pub const fn upper(&self) -> &Metres {
        &self.upper
    }

    /// Actual storage retained by this completed exact scale record.
    #[must_use]
    pub fn retained_workspace_bytes(&self) -> u64 {
        (size_of::<Self>() as u64).saturating_add(
            [
                &self.nominal_lower,
                &self.nominal_upper,
                &self.footprint_guard,
                &self.lower,
                &self.upper,
            ]
            .into_iter()
            .map(|value| value.exact().allocated_bytes() as u64)
            .sum::<u64>(),
        )
    }
}

struct ScaleFactors {
    lower: Rat,
    upper: Rat,
    guard: Rat,
}

impl ScaleFactors {
    fn new(
        ellipsoid: &PreparedEllipsoid,
        admission: &mut impl IntegerAdmission,
    ) -> Result<Self, GeoError> {
        let m = ellipsoid.normal_metric_bounds_ref().0.exact();
        // floor(sqrt(2 * 2^192))/2^96 is an outward lower enclosure.
        let radicand = integer(
            admission,
            ExactOperation::Linear,
            &[&Int::from_i64(2)],
            1,
            192,
            || Ok(Int::from_i64(2).shl(192)),
        )?;
        let sqrt_cost = ExactArithmeticCost::integer_square_root(radicand.bit_len())
            .ok_or(GeoError::ArithmeticOverflow("physical scale square root"))?;
        let sqrt2_floor = admission.exact(sqrt_cost, || {
            Ok(radicand.sqrt_floor().expect("positive radicand"))
        })?;
        let dyadic_cost = ExactArithmeticCost::dyadic_rational(sqrt2_floor.bit_len(), 96)
            .ok_or(GeoError::ArithmeticOverflow("physical scale dyadic bridge"))?;
        let sqrt2_lower = admission.exact(dyadic_cost, || {
            Ok(Rat::from_dyadic_integer(&sqrt2_floor, 96))
        })?;
        let lower = rational(
            admission,
            ExactOperation::RationalMultiply,
            &[m, &sqrt2_lower],
            1,
            || Ok(m.mul(&sqrt2_lower)),
        )?;
        let lower_factor = fraction(2, 3, admission)?;
        let lower = rational(
            admission,
            ExactOperation::RationalMultiply,
            &[&lower, &lower_factor],
            1,
            || Ok(lower.mul(&lower_factor)),
        )?;
        let (upper, guard) = upper_factors(ellipsoid, admission)?;
        Ok(Self {
            lower,
            upper,
            guard,
        })
    }

    fn bounds(
        &self,
        level: u8,
        admission: &mut impl IntegerAdmission,
    ) -> Result<CellScaleBounds, GeoError> {
        let divisor = level_divisor(level, admission)?;
        let nominal_lower = rational(
            admission,
            ExactOperation::RationalDivide,
            &[&self.lower, &divisor],
            1,
            || Ok(self.lower.div(&divisor).expect("positive level scale")),
        )?;
        let lower = rational(
            admission,
            ExactOperation::RationalAdd,
            &[&nominal_lower, &self.guard],
            1,
            || Ok(nominal_lower.sub(&self.guard)),
        )?;
        let zero = Rat::zero();
        let negative = rational(
            admission,
            ExactOperation::RationalCompare,
            &[&lower, &zero],
            1,
            || Ok(lower < zero),
        )?;
        let lower = if negative { zero } else { lower };
        let (nominal_upper, upper) = upper_at(&self.upper, &self.guard, &divisor, admission)?;
        let guard = rational(admission, ExactOperation::Linear, &[&self.guard], 1, || {
            Ok(self.guard.clone())
        })?;
        Ok(CellScaleBounds {
            nominal_lower: Metres::new(nominal_lower),
            nominal_upper: Metres::new(nominal_upper),
            footprint_guard: Metres::new(guard),
            lower: Metres::new(lower),
            upper: Metres::new(upper),
        })
    }
}

fn upper_factors(
    ellipsoid: &PreparedEllipsoid,
    admission: &mut impl IntegerAdmission,
) -> Result<(Rat, Rat), GeoError> {
    let r = ellipsoid.normal_metric_bounds_ref().1.exact();
    let upper_factor = fraction(7, 4, admission)?;
    let upper = rational(
        admission,
        ExactOperation::RationalMultiply,
        &[r, &upper_factor],
        1,
        || Ok(r.mul(&upper_factor)),
    )?;
    let denominator = integer(
        admission,
        ExactOperation::Linear,
        &[&Int::one()],
        1,
        58,
        || Ok(Rat::from_int(Int::one().shl(58))),
    )?;
    let guard = rational(
        admission,
        ExactOperation::RationalDivide,
        &[r, &denominator],
        1,
        || Ok(r.div(&denominator).expect("positive denominator")),
    )?;
    Ok((upper, guard))
}

fn level_divisor(level: u8, admission: &mut impl IntegerAdmission) -> Result<Rat, GeoError> {
    integer(
        admission,
        ExactOperation::Linear,
        &[&Int::one()],
        1,
        u64::from(level),
        || Ok(Rat::from_int(Int::one().shl(u32::from(level)))),
    )
}

fn upper_at(
    nominal: &Rat,
    guard: &Rat,
    divisor: &Rat,
    admission: &mut impl IntegerAdmission,
) -> Result<(Rat, Rat), GeoError> {
    let nominal_upper = rational(
        admission,
        ExactOperation::RationalDivide,
        &[nominal, divisor],
        1,
        || Ok(nominal.div(divisor).expect("positive level scale")),
    )?;
    let upper = rational(
        admission,
        ExactOperation::RationalAdd,
        &[&nominal_upper, guard],
        1,
        || Ok(nominal_upper.add(guard)),
    )?;
    Ok((nominal_upper, upper))
}

fn fraction(
    numerator: i64,
    denominator: i64,
    admission: &mut impl IntegerAdmission,
) -> Result<Rat, GeoError> {
    integer(
        admission,
        ExactOperation::RationalReduce,
        &[&Int::from_i64(numerator), &Int::from_i64(denominator)],
        1,
        0,
        || {
            Ok(
                Rat::new(Int::from_i64(numerator), Int::from_i64(denominator))
                    .expect("positive denominator"),
            )
        },
    )
}

fn select_level(
    ellipsoid: &PreparedEllipsoid,
    target: &Metres,
    admission: &mut impl IntegerAdmission,
) -> Result<u8, GeoError> {
    positive_target(target, admission)?;
    let (nominal, guard) = upper_factors(ellipsoid, admission)?;
    select_level_with_factors(&nominal, &guard, target, admission)
}

fn positive_target(target: &Metres, admission: &mut impl IntegerAdmission) -> Result<(), GeoError> {
    let positive = rational(
        admission,
        ExactOperation::RationalCompare,
        &[target.exact(), &Rat::zero()],
        1,
        || Ok(target.exact() > &Rat::zero()),
    )?;
    if !positive {
        return rational(
            admission,
            ExactOperation::Linear,
            &[target.exact()],
            1,
            || Err(GeoError::NonPositiveEdgeLength(target.exact().clone())),
        );
    }
    Ok(())
}

fn select_level_with_factors(
    nominal: &Rat,
    guard: &Rat,
    target: &Metres,
    admission: &mut impl IntegerAdmission,
) -> Result<u8, GeoError> {
    let comparison = LevelComparison::new(nominal, guard, target, admission)?;
    if comparison.fits(0, admission)? {
        return Ok(0);
    }
    if !comparison.fits(MAX_LEVEL, admission)? {
        let divisor = level_divisor(MAX_LEVEL, admission)?;
        let (_, minimum) = upper_at(nominal, guard, &divisor, admission)?;
        let target = rational(
            admission,
            ExactOperation::Linear,
            &[target.exact()],
            1,
            || Ok(target.exact().clone()),
        )?;
        return Err(GeoError::UnattainableEdgeLength {
            target,
            minimum: Box::new(minimum),
        });
    }
    // R>0 is a prepared-ellipsoid invariant. Therefore R*(7/4)*2^-L
    // plus the fixed positive guard is strictly decreasing. Keep one proved
    // failing level and one proved satisfying level until they are adjacent.
    let mut outside = 0;
    let mut inside = MAX_LEVEL;
    while inside - outside > 1 {
        let middle = outside + (inside - outside) / 2;
        if comparison.fits(middle, admission)? {
            inside = middle;
        } else {
            outside = middle;
        }
    }
    Ok(inside)
}

struct LevelComparison {
    left: Int,
    right: Int,
}

impl LevelComparison {
    fn new(
        nominal: &Rat,
        guard: &Rat,
        target: &Metres,
        admission: &mut impl IntegerAdmission,
    ) -> Result<Self, GeoError> {
        let gap = rational(
            admission,
            ExactOperation::RationalAdd,
            &[target.exact(), guard],
            1,
            || Ok(target.exact().sub(guard)),
        )?;
        let left = integer(
            admission,
            ExactOperation::Multiply,
            &[nominal.numerator(), gap.denominator()],
            1,
            0,
            || Ok(nominal.numerator().mul(gap.denominator())),
        )?;
        let right = integer(
            admission,
            ExactOperation::Multiply,
            &[gap.numerator(), nominal.denominator()],
            1,
            0,
            || Ok(gap.numerator().mul(nominal.denominator())),
        )?;
        Ok(Self { left, right })
    }

    fn fits(&self, level: u8, admission: &mut impl IntegerAdmission) -> Result<bool, GeoError> {
        // All denominators are positive. Cross multiplication proves
        // nominal/2^L+guard <= target iff left <= right*2^L,
        // including a nonpositive target-minus-guard and exact ties.
        let right = integer(
            admission,
            ExactOperation::Linear,
            &[&self.right],
            1,
            u64::from(level),
            || Ok(self.right.shl(u32::from(level))),
        )?;
        integer(
            admission,
            ExactOperation::Linear,
            &[&self.left, &right],
            1,
            0,
            || Ok(self.left <= right),
        )
    }
}

fn live_storage(ellipsoid: &PreparedEllipsoid, target: Option<&Metres>) -> Result<u64, GeoError> {
    let (m, r) = ellipsoid.normal_metric_bounds_ref();
    let bits = crate::numerical::rational_operand_bits(m.exact())
        .max(crate::numerical::rational_operand_bits(r.exact()))
        .max(target.map_or(0, |value| {
            crate::numerical::rational_operand_bits(value.exact())
        }))
        .checked_add(194)
        .ok_or(GeoError::ArithmeticOverflow("physical scale live storage"))?;
    ExactArithmeticCost::for_operation(ExactOperation::RationalMultiply, bits, 1)
        .and_then(|cost| cost.workspace_bytes.checked_mul(8))
        .and_then(|bytes| bytes.checked_add(65_536))
        .ok_or(GeoError::ArithmeticOverflow("physical scale live storage"))
}

fn admitted<T>(
    ellipsoid: &PreparedEllipsoid,
    target: Option<&Metres>,
    policy: ExecutionPolicy,
    observer: Option<&mut dyn MetricWorkObserver>,
    action: impl FnOnce(&mut super::cover::CoverBudget<'_>) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    let mut admission = assignment_admission(policy, 1, observer)?;
    let bytes = live_storage(ellipsoid, target)?;
    admission.reserve(bytes)?;
    let result = action(&mut admission);
    admission.poll()?;
    result
}

/// Bound native-grid physical edge scales using an exact prepared ellipsoid.
///
/// For axes a,b, `m=min(a,b)^2/max(a,b)` and `R=max(a,b)^2/min(a,b)`.
/// Nominal edges lie in `[m(2√2/3)2^-L, R(7/4)2^-L]`.
///
/// # Errors
///
/// Refuses levels outside zero through thirty.
pub fn physical_scale_bounds(
    ellipsoid: &PreparedEllipsoid,
    level: u8,
) -> Result<CellScaleBounds, GeoError> {
    validate_level(level)?;
    ScaleFactors::new(ellipsoid, &mut PureAssignment)?.bounds(level, &mut PureAssignment)
}

/// Select the smallest level meeting the complete guarded upper edge bound.
///
/// The comparison is exact in metre rationals. Limits and numerical proof
/// precision cannot change a selected level.
///
/// # Errors
///
/// Refuses a nonpositive target or a target below the level-thirty upper bound.
pub fn level_for_max_edge_length(
    ellipsoid: &PreparedEllipsoid,
    target: &Metres,
) -> Result<u8, GeoError> {
    select_level(ellipsoid, target, &mut PureAssignment)
}

/// Bound scales with complete integer work, memory and output admission.
/// # Errors
/// Adds resource refusal to the exact scale contract.
pub fn physical_scale_bounds_in_policy(
    ellipsoid: &PreparedEllipsoid,
    level: u8,
    policy: ExecutionPolicy,
) -> Result<CellScaleBounds, GeoError> {
    physical_scale_admission(ellipsoid, level, policy, None)
}

/// Meter scale construction through integer-only governor callbacks.
/// # Errors
/// Adds cancellation or observer refusal to policy admission.
pub fn physical_scale_bounds_in_policy_metered(
    ellipsoid: &PreparedEllipsoid,
    level: u8,
    policy: ExecutionPolicy,
    observer: &mut dyn MetricWorkObserver,
) -> Result<CellScaleBounds, GeoError> {
    physical_scale_admission(ellipsoid, level, policy, Some(observer))
}

fn physical_scale_admission(
    ellipsoid: &PreparedEllipsoid,
    level: u8,
    policy: ExecutionPolicy,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<CellScaleBounds, GeoError> {
    validate_level(level)?;
    admitted(ellipsoid, None, policy, observer, |admission| {
        ScaleFactors::new(ellipsoid, admission)?.bounds(level, admission)
    })
}

/// Select the identical exact level with original-target arithmetic admitted.
/// # Errors
/// Adds complete resource refusal to the physical target contract.
pub fn level_for_max_edge_length_in_policy(
    ellipsoid: &PreparedEllipsoid,
    target: &Metres,
    policy: ExecutionPolicy,
) -> Result<u8, GeoError> {
    level_admission(ellipsoid, target, policy, None)
}

/// Meter exact physical level selection without floating-environment dependence.
/// # Errors
/// Adds cancellation or observer refusal to policy admission.
pub fn level_for_max_edge_length_in_policy_metered(
    ellipsoid: &PreparedEllipsoid,
    target: &Metres,
    policy: ExecutionPolicy,
    observer: &mut dyn MetricWorkObserver,
) -> Result<u8, GeoError> {
    level_admission(ellipsoid, target, policy, Some(observer))
}

fn level_admission(
    ellipsoid: &PreparedEllipsoid,
    target: &Metres,
    policy: ExecutionPolicy,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<u8, GeoError> {
    admitted(ellipsoid, Some(target), policy, observer, |admission| {
        select_level(ellipsoid, target, admission)
    })
}

/// Construct exact bounds and optionally select a physical level in one admitted phase.
/// Both results reuse the same immutable upper factors and retain their pure values.
/// # Errors
/// Refuses invalid levels, targets, complete resource exhaustion or observer cancellation.
pub fn physical_scale_bounds_and_level_in_policy(
    ellipsoid: &PreparedEllipsoid,
    level: u8,
    target: Option<&Metres>,
    policy: ExecutionPolicy,
) -> Result<(CellScaleBounds, Option<u8>), GeoError> {
    bounds_and_level_admission(ellipsoid, level, target, policy, None)
}

/// Meter combined exact scale and level selection with integer-only callbacks.
/// # Errors
/// Preserves the complete admitted scale and smallest-level contracts.
pub fn physical_scale_bounds_and_level_in_policy_metered(
    ellipsoid: &PreparedEllipsoid,
    level: u8,
    target: Option<&Metres>,
    policy: ExecutionPolicy,
    observer: &mut dyn MetricWorkObserver,
) -> Result<(CellScaleBounds, Option<u8>), GeoError> {
    bounds_and_level_admission(ellipsoid, level, target, policy, Some(observer))
}

fn bounds_and_level_admission(
    ellipsoid: &PreparedEllipsoid,
    level: u8,
    target: Option<&Metres>,
    policy: ExecutionPolicy,
    observer: Option<&mut dyn MetricWorkObserver>,
) -> Result<(CellScaleBounds, Option<u8>), GeoError> {
    validate_level(level)?;
    admitted(ellipsoid, target, policy, observer, |admission| {
        if let Some(target) = target {
            positive_target(target, admission)?;
        }
        let factors = ScaleFactors::new(ellipsoid, admission)?;
        let bounds = factors.bounds(level, admission)?;
        let selected = target
            .map(|target| {
                select_level_with_factors(&factors.upper, &factors.guard, target, admission)
            })
            .transpose()?;
        Ok((bounds, selected))
    })
}
