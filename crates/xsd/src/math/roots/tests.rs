// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::*;
use crate::math::MathLimits;

struct PolynomialSystem {
    squared: bool,
    roots: bool,
    polls: usize,
    cancel_at: Option<usize>,
}

impl RootSystem2 for PolynomialSystem {
    fn image(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootBox2, MathError> {
        let x = if self.squared {
            domain[0].square(math)?
        } else {
            domain[0].clone()
        };
        let value = if self.roots {
            x.sub(&FixedInterval::from_i64(1, math)?, math)?
        } else {
            x.add(&FixedInterval::from_i64(1, math)?, math)?
        };
        Ok([value, domain[1].clone()])
    }
    fn jacobian(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootJacobian2, MathError> {
        let zero = FixedInterval::from_i64(0, math)?;
        let one = FixedInterval::from_i64(1, math)?;
        let dx = if self.squared {
            domain[0].mul(&FixedInterval::from_i64(2, math)?, math)?
        } else {
            one.clone()
        };
        Ok([[dx, zero.clone()], [zero, one]])
    }
    fn complete(
        &mut self,
        enclosure: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<bool, MathError> {
        for value in enclosure {
            let (lower, upper) = value.round_decimal(15, math)?;
            if lower != upper {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn poll(&mut self, _math: &CoordinateMath) -> Result<(), MathError> {
        self.polls += 1;
        if self.cancel_at.is_some_and(|at| self.polls >= at) {
            Err(MathError::Cancelled)
        } else {
            Ok(())
        }
    }
}

fn math() -> CoordinateMath {
    CoordinateMath::new(MathLimits::DEFAULT).expect("math")
}

#[test]
fn inverse_completion_refuses_negative_proof_widths_on_both_output_laws() {
    let mut math = math();
    let enclosure = FixedInterval::from_i64(1, &mut math).expect("exact enclosure");
    let zero = FixedInterval::from_i64(0, &mut math).expect("zero width");
    let negative = FixedInterval::from_i64(-1, &mut math).expect("invalid width");
    for quantize in [false, true] {
        for (input, evaluation) in [(&negative, &zero), (&zero, &negative)] {
            assert_eq!(
                inverse_coordinate_complete_with_error(
                    &enclosure, input, evaluation, 15, quantize, &mut math,
                ),
                Err(MathError::Domain("negative inverse conditioning width"))
            );
        }
    }
}

fn domain(lower: i128, upper: i128, math: &mut CoordinateMath) -> RootBox2 {
    let scale = BigInt::from_i128(1).mul_pow2(math.limits().precision_bits);
    [
        FixedInterval::from_bounds(
            BigInt::from_i128(lower).mul(&scale),
            BigInt::from_i128(upper).mul(&scale),
            math,
        )
        .expect("x"),
        FixedInterval::from_bounds(scale.negated(), scale, math).expect("y"),
    ]
}
fn system(squared: bool, roots: bool) -> PolynomialSystem {
    PolynomialSystem {
        squared,
        roots,
        polls: 0,
        cancel_at: None,
    }
}
const LIMITS: RootIsolationLimits = RootIsolationLimits {
    max_depth: 64,
    max_refinements: 128,
};

struct ExactBoundarySystem;
impl RootSystem2 for ExactBoundarySystem {
    fn image(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootBox2, MathError> {
        // This valid outward image hides the exact source-defined root at 0.
        let uncertainty =
            FixedInterval::from_bounds(BigInt::from_i128(-1), BigInt::from_i128(1), math)?;
        Ok([domain[0].add(&uncertainty, math)?, domain[1].clone()])
    }
    fn jacobian(
        &mut self,
        _domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootJacobian2, MathError> {
        let one = FixedInterval::from_i64(1, math)?;
        let zero = FixedInterval::from_i64(0, math)?;
        Ok([[one.clone(), zero.clone()], [zero, one]])
    }
    fn exact_root(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<Option<RootBox2>, MathError> {
        let zero = FixedInterval::from_i64(0, math)?;
        let witness = [zero.clone(), zero];
        Ok(subset(&witness, domain).then_some(witness))
    }
    fn complete(
        &mut self,
        enclosure: &RootBox2,
        _math: &mut CoordinateMath,
    ) -> Result<bool, MathError> {
        Ok(enclosure
            .iter()
            .all(|value| value.lower().is_zero() && value.upper().is_zero()))
    }
    fn poll(&mut self, _math: &CoordinateMath) -> Result<(), MathError> {
        Ok(())
    }
}

#[test]
fn exact_source_witness_proves_closed_boundary_despite_outward_residual() {
    let mut math = math();
    let zero = FixedInterval::from_i64(0, &mut math).expect("zero");
    let one = FixedInterval::from_i64(1, &mut math).expect("one");
    let interval = zero.hull(&one, &mut math).expect("closed axis");
    let roots = isolate_roots2(
        [interval.clone(), interval],
        LIMITS,
        &mut ExactBoundarySystem,
        &mut math,
    )
    .expect("source-defined zero");
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].enclosure(), &[zero.clone(), zero]);
}

#[test]
fn unrounded_inverse_family_completes_against_conditioning_not_decimal_grid() {
    let mut math = math();
    let one = FixedInterval::from_i64(1, &mut math).expect("one");
    let two = FixedInterval::from_i64(2, &mut math).expect("two");
    let family = one.hull(&two, &mut math).expect("family");
    assert!(!inverse_coordinate_complete(&family, &one, 15, true, &mut math).expect("scalar grid"));
    assert!(
        inverse_coordinate_complete(&family, &one, 15, false, &mut math)
            .expect("conditioned family")
    );
    let negative = FixedInterval::from_i64(-1, &mut math).expect("negative");
    assert!(matches!(
        inverse_coordinate_complete(&family, &negative, 15, false, &mut math),
        Err(MathError::Domain(_))
    ));
}

#[test]
fn inverse_family_arithmetic_floor_is_explicit_and_never_changes_final_rounding() {
    let mut math = math();
    let zero = FixedInterval::from_i64(0, &mut math).expect("zero");
    let floor =
        FixedInterval::from_bounds(BigInt::from_i128(1024), BigInt::from_i128(1024), &mut math)
            .expect("proved evaluation floor");
    let enclosure = FixedInterval::from_bounds(BigInt::zero(), BigInt::from_i128(2048), &mut math)
        .expect("inverse enclosure");
    assert!(!inverse_coordinate_complete(&enclosure, &zero, 15, false, &mut math).unwrap());
    assert!(
        inverse_coordinate_complete_with_error(&enclosure, &zero, &floor, 15, false, &mut math)
            .unwrap()
    );
    let one = FixedInterval::from_i64(1, &mut math).expect("one");
    let wide = zero.hull(&one, &mut math).expect("wide family");
    assert!(
        !inverse_coordinate_complete_with_error(&wide, &one, &one, 15, true, &mut math).unwrap()
    );
    let negative = floor.neg(&mut math).expect("negative floor");
    assert!(matches!(
        inverse_coordinate_complete_with_error(&enclosure, &zero, &negative, 15, false, &mut math),
        Err(MathError::Domain(_))
    ));
}

#[test]
fn complete_isolation_keeps_both_roots_and_proves_no_root() {
    let mut math = math();
    let domain = domain(-2, 2, &mut math);
    let before = math.workspace_reserved();
    let roots = isolate_roots2(domain.clone(), LIMITS, &mut system(true, true), &mut math)
        .expect("all roots");
    assert_eq!(roots.len(), 2);
    for (root, wanted) in roots.iter().zip([-1, 1]) {
        let (lower, upper) = root.enclosure()[0]
            .round_decimal(0, &mut math)
            .expect("rounded");
        assert_eq!(
            (lower, upper),
            (BigInt::from_i128(wanted), BigInt::from_i128(wanted))
        );
    }
    assert_eq!(math.workspace_reserved(), before);
    assert_eq!(
        isolate_roots2(domain, LIMITS, &mut system(true, false), &mut math).expect("no root"),
        [],
    );
}

#[test]
fn closed_original_boundary_is_retained() {
    let mut math = math();
    let domain = domain(1, 3, &mut math);
    let roots =
        isolate_roots2(domain, LIMITS, &mut system(false, true), &mut math).expect("boundary root");
    assert_eq!(roots.len(), 1);
    let (lower, upper) = roots[0].enclosure()[0]
        .round_decimal(0, &mut math)
        .expect("boundary");
    assert_eq!((lower, upper), (BigInt::from_i128(1), BigInt::from_i128(1)));
}

#[test]
fn cancellation_and_depth_refuse_without_partial_roots_and_release_workspace() {
    let mut math = math();
    let domain = domain(-2, 2, &mut math);
    let before = math.workspace_reserved();
    let mut cancelled = system(true, true);
    cancelled.cancel_at = Some(3);
    assert_eq!(
        isolate_roots2(domain.clone(), LIMITS, &mut cancelled, &mut math),
        Err(MathError::Cancelled)
    );
    assert_eq!(math.workspace_reserved(), before);
    let shallow = RootIsolationLimits {
        max_depth: 1,
        max_refinements: 1,
    };
    assert!(matches!(
        isolate_roots2(domain, shallow, &mut system(true, true), &mut math),
        Err(MathError::PrecisionExhausted | MathError::ConvergenceExhausted { .. })
    ));
    assert_eq!(math.workspace_reserved(), before);
}

#[test]
fn derivative_composition_and_actual_inverse_share_outward_arithmetic() {
    let mut math = math();
    let integer =
        |value, math: &mut CoordinateMath| FixedInterval::from_i64(value, math).expect("integer");
    let matrix = [
        [integer(2, &mut math), integer(1, &mut math)],
        [integer(3, &mut math), integer(4, &mut math)],
    ];
    let inverse = invert_jacobian2(&matrix, &mut math).expect("nonsingular");
    let identity = compose_jacobians2(&matrix, &inverse, &mut math).expect("composition");
    for (row, entries) in identity.iter().enumerate() {
        for (column, value) in entries.iter().enumerate() {
            let wanted = integer(i64::from(row == column), &mut math);
            assert!(value.lower() <= wanted.lower() && wanted.upper() <= value.upper());
        }
    }
    let singular = [
        [integer(1, &mut math), integer(2, &mut math)],
        [integer(2, &mut math), integer(4, &mut math)],
    ];
    assert_eq!(
        invert_jacobian2(&singular, &mut math),
        Err(MathError::PrecisionExhausted)
    );
}

#[test]
fn caller_root_storage_keeps_complete_order_and_clears_every_refusal() {
    let mut first = math();
    let expected = isolate_roots2(
        domain(-2, 2, &mut first),
        LIMITS,
        &mut system(true, true),
        &mut first,
    )
    .unwrap();
    let mut second = math();
    let mut output = Vec::new();
    isolate_roots2_into(
        domain(-2, 2, &mut second),
        LIMITS,
        &mut system(true, true),
        &mut second,
        &mut output,
    )
    .unwrap();
    assert_eq!(output, expected);
    let mut cancelled = system(true, true);
    cancelled.cancel_at = Some(1);
    let result = isolate_roots2_into(
        domain(-2, 2, &mut second),
        LIMITS,
        &mut cancelled,
        &mut second,
        &mut output,
    );
    assert!(matches!(result, Err(MathError::Cancelled)));
    assert_eq!(output.len(), 0);
}
