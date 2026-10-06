// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-defined normalized polynomials and regular in-memory bilinear grids.

use super::OperationCoordinates;

use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

use super::{OperationPoint, rational_fields};
use crate::context::WorkProgress;
use crate::numerical::{exact_bounds, fixed_from_bounds, fixed_from_rat};
use crate::{GeoError, Rat};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation};
use purrdf_xsd::math::RootJacobian2;

// Both typed models project their own original image/Jacobian body through
// the same image-only adapter; the mathematical model bodies remain distinct.
macro_rules! model_evaluation {
    ($type:ty) => {
        impl $type {
            pub(super) fn evaluate(
                &self,
                coordinates: &[FixedInterval],
                math: &mut CoordinateMath,
                progress: &mut WorkProgress<'_>,
            ) -> Result<OperationCoordinates, MathError> {
                self.evaluate_parts(coordinates, false, math, progress)
                    .map(|(image, _)| image)
            }
        }
    };
}
model_evaluation!(Polynomial2d);
model_evaluation!(BilinearGrid);

/// One explicit monomial coefficient pair in a two-dimensional polynomial.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PolynomialTerm {
    /// Power of normalized first coordinate.
    pub x_power: u16,
    /// Power of normalized second coordinate.
    pub y_power: u16,
    /// Coefficient of the first output coordinate, in metres.
    pub x_coefficient: Rat,
    /// Coefficient of the second output coordinate, in metres.
    pub y_coefficient: Rat,
}

/// Explicit monomial polynomial: `(x-origin_x)/scale_x`, `(y-origin_y)/scale_y`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Polynomial2d {
    degree: u16,
    origin: [Rat; 2],
    scale: [Rat; 2],
    terms: Vec<PolynomialTerm>,
}

impl Polynomial2d {
    /// Compile and canonicalize explicit monomial terms by `(x_power,y_power)`.
    ///
    /// # Errors
    /// Refuses repeated powers, terms above declared degree and nonpositive normalization.
    pub fn new(
        degree: u16,
        origin: [Rat; 2],
        scale: [Rat; 2],
        terms: Vec<PolynomialTerm>,
    ) -> Result<Self, GeoError> {
        Self::new_in_budget(
            degree,
            origin,
            scale,
            terms,
            &mut crate::PreparationBudget::new(crate::ExecutionPolicy::geometry()),
        )
    }

    /// Canonicalize original monomials under cumulative configuration admission.
    /// # Errors
    /// Refuses normalization, duplicate powers, declared degree and work/storage limits.
    pub fn new_in_budget(
        degree: u16,
        origin: [Rat; 2],
        scale: [Rat; 2],
        terms: Vec<PolynomialTerm>,
        budget: &mut crate::PreparationBudget,
    ) -> Result<Self, GeoError> {
        let count = terms.len() as u64;
        let sort_work = count
            .checked_mul(u64::from(count.bit_width()) + 1)
            .and_then(|work| work.checked_mul(16))
            .and_then(|work| work.checked_add(8))
            .ok_or(GeoError::ArithmeticOverflow(
                "polynomial canonicalization work",
            ))?;
        let mut cost = ExactArithmeticCost::for_operation(ExactOperation::Linear, 1, sort_work)
            .ok_or(GeoError::ArithmeticOverflow(
                "polynomial canonicalization work",
            ))?;
        cost = cost
            .followed_by(
                crate::numerical::rational_cost(
                    ExactOperation::RationalCompare,
                    &[&scale[0], &scale[1]],
                    2,
                )
                .ok_or(GeoError::ArithmeticOverflow("configuration rational work"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow(
                "polynomial canonicalization work",
            ))?;
        cost.workspace_bytes = cost
            .workspace_bytes
            .checked_add(
                count
                    .checked_mul(size_of::<PolynomialTerm>() as u64)
                    .ok_or(GeoError::ArithmeticOverflow("polynomial sort scratch"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow("polynomial sort scratch"))?;
        budget.exact(cost, || Self::new_admitted(degree, origin, scale, terms))
    }

    fn new_admitted(
        degree: u16,
        origin: [Rat; 2],
        scale: [Rat; 2],
        mut terms: Vec<PolynomialTerm>,
    ) -> Result<Self, GeoError> {
        if scale.iter().any(|value| value <= &Rat::zero()) {
            return Err(GeoError::config(
                "polynomial normalization scales must be positive",
            ));
        }
        terms.sort_by_key(|term| (term.x_power, term.y_power));
        for term in &terms {
            if u32::from(term.x_power) + u32::from(term.y_power) > u32::from(degree) {
                return Err(GeoError::config("polynomial term exceeds declared degree"));
            }
        }
        if terms
            .windows(2)
            .any(|pair| (pair[0].x_power, pair[0].y_power) == (pair[1].x_power, pair[1].y_power))
        {
            return Err(GeoError::config("duplicate polynomial monomial"));
        }
        Ok(Self {
            degree,
            origin,
            scale,
            terms,
        })
    }
    /// Declared total degree.
    #[must_use]
    pub const fn degree(&self) -> u16 {
        self.degree
    }
    /// Explicit metre normalization origins.
    #[must_use]
    pub const fn origin(&self) -> &[Rat; 2] {
        &self.origin
    }
    /// Explicit positive metre normalization scales.
    #[must_use]
    pub const fn scale(&self) -> &[Rat; 2] {
        &self.scale
    }
    /// Canonical ordered monomial coefficient pairs.
    #[must_use]
    pub fn terms(&self) -> &[PolynomialTerm] {
        &self.terms
    }
    /// Isolate an exact rational stationary coordinate of an original
    /// separated quadratic row. This is a source partition, not an image
    /// approximation or an injectivity assertion; every resulting cell still
    /// requires the original whole-panel proof.
    pub(super) fn axis_quadratic_critical(
        &self,
        axis: usize,
        context: &mut crate::MetricContext,
        progress: &mut WorkProgress<'_>,
        retained: &mut u64,
    ) -> Result<Option<Rat>, GeoError> {
        context.charge_work(
            (self.terms.len() as u64)
                .checked_mul(2)
                .ok_or(GeoError::ArithmeticOverflow("critical polynomial metadata"))?,
        )?;
        progress.context_poll(context)?;
        for row in 0..2 {
            let mut linear = None;
            let mut quadratic = None;
            let mut separated = true;
            for term in &self.terms {
                let coefficient = if row == 0 {
                    &term.x_coefficient
                } else {
                    &term.y_coefficient
                };
                if coefficient.is_zero() {
                    continue;
                }
                let powers = [term.x_power, term.y_power];
                if powers[1 - axis] != 0 || powers[axis] > 2 {
                    separated = false;
                    break;
                }
                match powers[axis] {
                    1 => linear = Some(coefficient),
                    2 => quadratic = Some(coefficient),
                    _ => {}
                }
            }
            if !separated {
                continue;
            }
            let Some(quadratic) = quadratic else {
                continue;
            };
            let mut admission = crate::numerical::ExactAdmission::new(context, progress);
            let Some(linear) = linear else {
                return admission
                    .rational_owner(
                        ExactOperation::Linear,
                        &[&self.origin[axis]],
                        retained,
                        || self.origin[axis].clone(),
                    )
                    .map(Some);
            };
            let two = Rat::from_i64(2);
            let divisor = admission.rational_owner(
                ExactOperation::RationalMultiply,
                &[quadratic, &two],
                retained,
                || quadratic.mul(&two),
            )?;
            let relative = admission.rational_owner(
                ExactOperation::RationalDivide,
                &[linear, &divisor],
                retained,
                || linear.div(&divisor).expect("nonzero quadratic"),
            )?;
            let offset = admission.rational_owner(
                ExactOperation::RationalMultiply,
                &[&self.scale[axis], &relative],
                retained,
                || self.scale[axis].mul(&relative),
            )?;
            return admission
                .rational_owner(
                    ExactOperation::RationalAdd,
                    &[&self.origin[axis], &offset],
                    retained,
                    || self.origin[axis].sub(&offset),
                )
                .map(Some);
        }
        Ok(None)
    }
    pub(super) fn container_bytes(&self) -> Option<u64> {
        (self.terms.capacity() as u64).checked_mul(size_of::<PolynomialTerm>() as u64)
    }

    pub(super) fn parameters(&self, fields: &mut Vec<Vec<u8>>) {
        fields.push(self.degree.to_be_bytes().to_vec());
        for value in self.origin.iter().chain(self.scale.iter()) {
            rational_fields(value, fields);
        }
        for term in &self.terms {
            fields.push(term.x_power.to_be_bytes().to_vec());
            fields.push(term.y_power.to_be_bytes().to_vec());
            rational_fields(&term.x_coefficient, fields);
            rational_fields(&term.y_coefficient, fields);
        }
    }

    pub(super) fn differential(
        &self,
        coordinates: &[FixedInterval],
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(OperationCoordinates, RootJacobian2), MathError> {
        self.evaluate_parts(coordinates, true, math, progress)
    }

    fn evaluate_parts(
        &self,
        coordinates: &[FixedInterval],
        derivatives: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(OperationCoordinates, RootJacobian2), MathError> {
        let scale = [
            fixed_from_rat(&self.scale[0], math)?,
            fixed_from_rat(&self.scale[1], math)?,
        ];
        let x = coordinates[0]
            .sub(&fixed_from_rat(&self.origin[0], math)?, math)?
            .div(&scale[0], math)?;
        let y = coordinates[1]
            .sub(&fixed_from_rat(&self.origin[1], math)?, math)?
            .div(&scale[1], math)?;
        let zero = FixedInterval::from_i64(0, math)?;
        let mut output: OperationCoordinates = purrdf_core::smallvec![zero.clone(), zero.clone()];
        let mut jacobian = [[zero.clone(), zero.clone()], [zero.clone(), zero.clone()]];
        for (index, term) in self.terms.iter().enumerate() {
            if index % 8 == 0 {
                progress.math_poll(math)?;
            }
            let xp = power(&x, term.x_power, math)?;
            let yp = power(&y, term.y_power, math)?;
            let monomial = xp.mul(&yp, math)?;
            let derivative = if derivatives {
                [
                    monomial_derivative(&x, term.x_power, &yp, &scale[0], math)?,
                    monomial_derivative(&y, term.y_power, &xp, &scale[1], math)?,
                ]
            } else {
                [zero.clone(), zero.clone()]
            };
            for (row, coefficient) in [&term.x_coefficient, &term.y_coefficient]
                .into_iter()
                .enumerate()
            {
                let coefficient = fixed_from_rat(coefficient, math)?;
                output[row] = output[row].add(&monomial.mul(&coefficient, math)?, math)?;
                if derivatives {
                    for (column, derivative) in derivative.iter().enumerate() {
                        jacobian[row][column] = jacobian[row][column]
                            .add(&derivative.mul(&coefficient, math)?, math)?;
                    }
                }
            }
        }
        progress.math_poll(math)?;
        if let Some(z) = coordinates.get(2) {
            output.push(z.clone());
        }
        Ok((output, jacobian))
    }
}

fn monomial_derivative(
    value: &FixedInterval,
    exponent: u16,
    other: &FixedInterval,
    scale: &FixedInterval,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    if exponent == 0 {
        return FixedInterval::from_i64(0, math);
    }
    power(value, exponent - 1, math)?
        .mul(other, math)?
        .mul(&FixedInterval::from_i64(i64::from(exponent), math)?, math)?
        .div(scale, math)
}

fn power(
    value: &FixedInterval,
    mut exponent: u16,
    math: &mut CoordinateMath,
) -> Result<FixedInterval, MathError> {
    let mut result = FixedInterval::from_i64(1, math)?;
    let mut factor = value.clone();
    while exponent != 0 {
        if exponent & 1 != 0 {
            result = result.mul(&factor, math)?;
        }
        exponent >>= 1;
        if exponent != 0 {
            factor = factor.square(math)?;
        }
    }
    Ok(result)
}

/// Exact two-dimensional displacement at one grid node, in metres.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GridNode {
    /// Displacement along the first metre axis.
    pub x: Rat,
    /// Displacement along the second metre axis.
    pub y: Rat,
}

/// A regular bilinear displacement grid; a missing node makes its incident cells holes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BilinearGrid {
    origin: [Rat; 2],
    spacing: [Rat; 2],
    columns: u32,
    rows: u32,
    nodes: Vec<Option<GridNode>>,
}

impl BilinearGrid {
    /// Declare regular extents and row-major nodes, with explicit holes.
    ///
    /// # Errors
    /// Refuses invalid extents, overflow and a node count inconsistent with dimensions.
    pub fn new(
        origin: [Rat; 2],
        spacing: [Rat; 2],
        columns: u32,
        rows: u32,
        nodes: Vec<Option<GridNode>>,
    ) -> Result<Self, GeoError> {
        Self::new_in_budget(
            origin,
            spacing,
            columns,
            rows,
            nodes,
            &mut crate::PreparationBudget::new(crate::ExecutionPolicy::geometry()),
        )
    }

    /// Validate declared grid dimensions and spacing under cumulative admission.
    /// # Errors
    /// Refuses shape, exact comparisons and arithmetic limits before validation.
    pub fn new_in_budget(
        origin: [Rat; 2],
        spacing: [Rat; 2],
        columns: u32,
        rows: u32,
        nodes: Vec<Option<GridNode>>,
        budget: &mut crate::PreparationBudget,
    ) -> Result<Self, GeoError> {
        let cost = crate::numerical::rational_cost(
            ExactOperation::RationalCompare,
            &[&spacing[0], &spacing[1]],
            2,
        )
        .ok_or(GeoError::ArithmeticOverflow("configuration rational work"))?;
        budget.exact(cost, || {
            Self::new_admitted(origin, spacing, columns, rows, nodes)
        })
    }

    fn new_admitted(
        origin: [Rat; 2],
        spacing: [Rat; 2],
        columns: u32,
        rows: u32,
        nodes: Vec<Option<GridNode>>,
    ) -> Result<Self, GeoError> {
        if columns < 2 || rows < 2 || spacing.iter().any(|value| value <= &Rat::zero()) {
            return Err(GeoError::config(
                "bilinear grid needs positive spacing and at least two nodes per axis",
            ));
        }
        let expected = usize::try_from(u64::from(columns) * u64::from(rows))
            .map_err(|_| GeoError::ArithmeticOverflow("grid dimensions"))?;
        if nodes.len() != expected {
            return Err(GeoError::InvalidOutputLength {
                expected,
                actual: nodes.len(),
            });
        }
        Ok(Self {
            origin,
            spacing,
            columns,
            rows,
            nodes,
        })
    }
    /// Exact first node coordinates.
    #[must_use]
    pub const fn origin(&self) -> &[Rat; 2] {
        &self.origin
    }
    /// Exact positive regular node spacing.
    #[must_use]
    pub const fn spacing(&self) -> &[Rat; 2] {
        &self.spacing
    }
    /// Number of grid columns and rows.
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.columns, self.rows)
    }
    /// Row-major exact displacements and explicit node holes.
    #[must_use]
    pub fn nodes(&self) -> &[Option<GridNode>] {
        &self.nodes
    }
    pub(super) fn container_bytes(&self) -> Option<u64> {
        (self.nodes.capacity() as u64).checked_mul(size_of::<Option<GridNode>>() as u64)
    }

    pub(super) fn parameters(&self, fields: &mut Vec<Vec<u8>>) {
        for value in self.origin.iter().chain(self.spacing.iter()) {
            rational_fields(value, fields);
        }
        fields.push(self.columns.to_be_bytes().to_vec());
        fields.push(self.rows.to_be_bytes().to_vec());
        for node in &self.nodes {
            fields.push(vec![u8::from(node.is_some())]);
            if let Some(node) = node {
                rational_fields(&node.x, fields);
                rational_fields(&node.y, fields);
            }
        }
    }
    pub(super) fn cell(&self, x: &Rat, y: &Rat) -> Result<(u32, u32), GeoError> {
        let coordinate = |value: &Rat, axis: usize, count: u32| -> Result<u32, GeoError> {
            let relative = value
                .sub(&self.origin[axis])
                .div(&self.spacing[axis])
                .expect("positive spacing");
            if relative < Rat::zero() || relative > Rat::from_i64(i64::from(count - 1)) {
                return Err(GeoError::domain(
                    "bilinear grid does not extrapolate outside its extent",
                ));
            }
            let index = relative
                .numerator()
                .div_rem(relative.denominator())
                .expect("positive denominator")
                .0
                .to_i128()
                .ok_or(GeoError::ArithmeticOverflow("grid cell index"))?;
            Ok(u32::try_from(index)
                .map_err(|_| GeoError::ArithmeticOverflow("grid cell index"))?
                .min(count - 2))
        };
        Ok((
            coordinate(x, 0, self.columns)?,
            coordinate(y, 1, self.rows)?,
        ))
    }

    /// Admit the original locator's two rational normalizations, comparisons,
    /// integer quotients and optional exact incident-patch boundary checks.
    pub(super) fn location_cost(
        &self,
        x: &Rat,
        y: &Rat,
        complete: bool,
    ) -> Option<ExactArithmeticCost> {
        let extent = Rat::from_i64(i64::from(self.columns.max(self.rows)));
        let operands = [
            x,
            y,
            &self.origin[0],
            &self.origin[1],
            &self.spacing[0],
            &self.spacing[1],
            &extent,
        ];
        let subtract = crate::numerical::rational_cost(ExactOperation::RationalAdd, &operands, 2)?;
        let quotient = ExactArithmeticCost::for_operation(
            ExactOperation::RationalDivide,
            subtract.output_bits,
            2,
        )?;
        let compare = ExactArithmeticCost::for_operation(
            ExactOperation::RationalCompare,
            quotient.output_bits,
            4,
        )?;
        let divide =
            ExactArithmeticCost::for_operation(ExactOperation::Divide, quotient.output_bits, 2)?;
        let mut cost = subtract
            .followed_by(quotient)?
            .followed_by(compare)?
            .followed_by(divide)?;
        if complete {
            let boundary_product =
                crate::numerical::rational_cost(ExactOperation::RationalMultiply, &operands, 2)?;
            let boundary_sum = ExactArithmeticCost::for_operation(
                ExactOperation::RationalAdd,
                boundary_product.output_bits,
                2,
            )?;
            // Rat equality compares normalized limbs; this larger cross-product
            // comparison bound also admits those scans.
            let boundary_compare = ExactArithmeticCost::for_operation(
                ExactOperation::RationalCompare,
                boundary_sum.output_bits,
                2,
            )?;
            cost = cost
                .followed_by(boundary_product)?
                .followed_by(boundary_sum)?
                .followed_by(boundary_compare)?;
        }
        Some(cost)
    }

    pub(super) fn location_math(
        &self,
        x: &Rat,
        y: &Rat,
        complete: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(u32, u32), MathError> {
        let cost = self
            .location_cost(x, y, complete)
            .ok_or(MathError::WorkExhausted)?;
        math.admit_exact_cost(cost)?;
        progress.math_poll(math)?;
        let location = if complete {
            self.complete_patch(x, y)
        } else {
            self.cell(x, y)
        };
        progress.math_poll(math)?;
        location.map_err(|_| MathError::Domain("bilinear grid point is outside complete patches"))
    }
    pub(super) fn corners(&self, column: u32, row: u32) -> Result<[&GridNode; 4], GeoError> {
        let get = |x: u32, y: u32| {
            self.nodes[(u64::from(y) * u64::from(self.columns) + u64::from(x)) as usize]
                .as_ref()
                .ok_or_else(|| GeoError::domain("bilinear grid cell touches a missing node"))
        };
        Ok([
            get(column, row)?,
            get(column + 1, row)?,
            get(column, row + 1)?,
            get(column + 1, row + 1)?,
        ])
    }
    pub(super) fn validate_point(&self, point: &OperationPoint) -> Result<(), GeoError> {
        self.complete_patch(&point.x, &point.y)?;
        Ok(())
    }

    // The domain is the closed union of complete patches. A missing node does
    // not remove the boundary of an adjacent complete patch.
    pub(super) fn complete_patch(&self, x: &Rat, y: &Rat) -> Result<(u32, u32), GeoError> {
        let (column, row) = self.cell(x, y)?;
        let previous = |value: &Rat, axis: usize, index: u32| {
            index > 0
                && value
                    == &self.origin[axis]
                        .add(&self.spacing[axis].mul(&Rat::from_i64(i64::from(index))))
        };
        let first_column = column - u32::from(previous(x, 0, column));
        let first_row = row - u32::from(previous(y, 1, row));
        for row in first_row..=row {
            for column in first_column..=column {
                if self.corners(column, row).is_ok() {
                    return Ok((column, row));
                }
            }
        }
        Err(GeoError::domain(
            "bilinear grid point is outside all complete closed patches",
        ))
    }
    pub(super) fn patch_bounds(&self, column: u32, row: u32) -> ([Rat; 2], [Rat; 2]) {
        let lower = [
            self.origin[0].add(&self.spacing[0].mul(&Rat::from_i64(i64::from(column)))),
            self.origin[1].add(&self.spacing[1].mul(&Rat::from_i64(i64::from(row)))),
        ];
        let upper = [
            lower[0].add(&self.spacing[0]),
            lower[1].add(&self.spacing[1]),
        ];
        (lower, upper)
    }

    pub(super) fn patch_bounds_math(
        &self,
        column: u32,
        row: u32,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<([Rat; 2], [Rat; 2]), MathError> {
        let indices = [
            Rat::from_i64(i64::from(column)),
            Rat::from_i64(i64::from(row)),
        ];
        let operands = [
            &self.origin[0],
            &self.origin[1],
            &self.spacing[0],
            &self.spacing[1],
            &indices[0],
            &indices[1],
        ];
        let product =
            crate::numerical::rational_cost(ExactOperation::RationalMultiply, &operands, 1)
                .ok_or(MathError::WorkExhausted)?;
        let first_sum =
            ExactArithmeticCost::for_operation(ExactOperation::RationalAdd, product.output_bits, 1)
                .ok_or(MathError::WorkExhausted)?;
        let complete = ExactArithmeticCost::for_operation(
            ExactOperation::RationalAdd,
            first_sum.output_bits,
            8,
        )
        .ok_or(MathError::WorkExhausted)?;
        math.admit_exact_cost(complete)?;
        progress.math_poll(math)?;
        let result = self.patch_bounds(column, row);
        progress.math_poll(math)?;
        Ok(result)
    }

    pub(super) fn evaluate_declared(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        source_is_exact: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<OperationCoordinates, MathError> {
        if source_is_exact {
            let (column, row) = self.location_math(&point.x, &point.y, true, math, progress)?;
            self.patch_image(coordinates, column, row, math, progress)
        } else {
            self.evaluate(coordinates, math, progress)
        }
    }

    pub(super) fn differential(
        &self,
        coordinates: &[FixedInterval],
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(OperationCoordinates, RootJacobian2), MathError> {
        self.evaluate_parts(coordinates, true, math, progress)
    }

    fn evaluate_parts(
        &self,
        coordinates: &[FixedInterval],
        derivatives: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(OperationCoordinates, RootJacobian2), MathError> {
        let (xmin, xmax) = exact_bounds(&coordinates[0]);
        let (ymin, ymax) = exact_bounds(&coordinates[1]);
        let mut first = self
            .location_math(&xmin, &ymin, false, math, progress)
            .map_err(|error| match error {
                MathError::Domain(_) => MathError::PrecisionExhausted,
                other => other,
            })?;
        let last = self
            .location_math(&xmax, &ymax, false, math, progress)
            .map_err(|error| match error {
                MathError::Domain(_) => MathError::PrecisionExhausted,
                other => other,
            })?;
        for (axis, value) in [&xmin, &ymin].into_iter().enumerate() {
            let (column, row) = first;
            let index = if axis == 0 {
                &mut first.0
            } else {
                &mut first.1
            };
            let previous = if *index > 0 {
                let (_, upper) = self.patch_bounds_math(
                    column.saturating_sub(u32::from(axis == 0)),
                    row.saturating_sub(u32::from(axis == 1)),
                    math,
                    progress,
                )?;
                crate::numerical::math_exact_rational(
                    ExactOperation::RationalCompare,
                    &[value, &upper[axis]],
                    math,
                    progress,
                    || value == &upper[axis],
                )?
            } else {
                false
            };
            if previous {
                *index -= 1;
            }
        }
        let mut hull: Option<(OperationCoordinates, RootJacobian2)> = None;
        for row in first.1..=last.1 {
            for column in first.0..=last.0 {
                math.admit_exact(1, math.limits().precision_bits as usize)?;
                progress.math_poll(math)?;
                let (lower, upper) = self.patch_bounds_math(column, row, math, progress)?;
                let (clipped_lower, clipped_upper, outside) =
                    crate::numerical::math_exact_rationals(
                        ExactOperation::RationalCompare,
                        &[
                            &xmin, &xmax, &ymin, &ymax, &lower[0], &lower[1], &upper[0], &upper[1],
                        ],
                        8,
                        math,
                        progress,
                        || {
                            let clipped_lower = [
                                xmin.clone().max(lower[0].clone()),
                                ymin.clone().max(lower[1].clone()),
                            ];
                            let clipped_upper = [
                                xmax.clone().min(upper[0].clone()),
                                ymax.clone().min(upper[1].clone()),
                            ];
                            let outside =
                                (0..2).any(|axis| clipped_lower[axis] > clipped_upper[axis]);
                            (clipped_lower, clipped_upper, outside)
                        },
                    )?;
                if outside {
                    continue;
                }
                if self.corners(column, row).is_err() {
                    // A hole excludes its relative interior. Its boundary can
                    // belong to a complete neighboring patch, so only a proven
                    // interior intersection makes the complete image undefined.
                    let interior = crate::numerical::math_exact_rationals(
                        ExactOperation::RationalCompare,
                        &[
                            &clipped_lower[0],
                            &clipped_lower[1],
                            &clipped_upper[0],
                            &clipped_upper[1],
                            &lower[0],
                            &lower[1],
                            &upper[0],
                            &upper[1],
                        ],
                        6,
                        math,
                        progress,
                        || {
                            (0..2).all(|axis| {
                                clipped_lower[axis] < clipped_upper[axis]
                                    || (clipped_lower[axis] > lower[axis]
                                        && clipped_lower[axis] < upper[axis])
                            })
                        },
                    )?;
                    if interior {
                        return Err(MathError::Domain("bilinear grid image crosses a hole"));
                    }
                    continue;
                }
                let mut clipped: OperationCoordinates = purrdf_core::smallvec![
                    fixed_from_bounds(&clipped_lower[0], &clipped_upper[0], math)?,
                    fixed_from_bounds(&clipped_lower[1], &clipped_upper[1], math)?,
                ];
                if let Some(z) = coordinates.get(2) {
                    clipped.push(z.clone());
                }
                let next =
                    self.patch_evaluate(&clipped, column, row, derivatives, math, progress)?;
                if let Some((image, jacobian)) = &mut hull {
                    for (old, next) in image.iter_mut().zip(next.0) {
                        *old = old.hull(&next, math)?;
                    }
                    if derivatives {
                        for (old, next) in jacobian
                            .iter_mut()
                            .flatten()
                            .zip(next.1.into_iter().flatten())
                        {
                            *old = old.hull(&next, math)?;
                        }
                    }
                } else {
                    hull = Some(next);
                }
            }
        }
        hull.ok_or(MathError::PrecisionExhausted)
    }

    pub(super) fn patch_differential(
        &self,
        coordinates: &[FixedInterval],
        column: u32,
        row: u32,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(OperationCoordinates, RootJacobian2), MathError> {
        self.patch_evaluate(coordinates, column, row, true, math, progress)
    }

    pub(super) fn patch_image(
        &self,
        coordinates: &[FixedInterval],
        column: u32,
        row: u32,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<OperationCoordinates, MathError> {
        self.patch_evaluate(coordinates, column, row, false, math, progress)
            .map(|(image, _)| image)
    }

    fn patch_evaluate(
        &self,
        coordinates: &[FixedInterval],
        column: u32,
        row: u32,
        derivatives: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(OperationCoordinates, RootJacobian2), MathError> {
        progress.math_poll(math)?;
        let corners = self
            .corners(column, row)
            .map_err(|_| MathError::Domain("bilinear grid hole"))?;
        let (origin, _) = self.patch_bounds_math(column, row, math, progress)?;
        let scale = [
            fixed_from_rat(&self.spacing[0], math)?,
            fixed_from_rat(&self.spacing[1], math)?,
        ];
        let u = coordinates[0]
            .sub(&fixed_from_rat(&origin[0], math)?, math)?
            .div(&scale[0], math)?;
        let v = coordinates[1]
            .sub(&fixed_from_rat(&origin[1], math)?, math)?
            .div(&scale[1], math)?;
        let one = FixedInterval::from_i64(1, math)?;
        let zero = FixedInterval::from_i64(0, math)?;
        let weights = [
            one.sub(&u, math)?.mul(&one.sub(&v, math)?, math)?,
            u.mul(&one.sub(&v, math)?, math)?,
            one.sub(&u, math)?.mul(&v, math)?,
            u.mul(&v, math)?,
        ];
        let mut output: OperationCoordinates = coordinates.iter().cloned().collect();
        let mut jacobian = [[one.clone(), zero.clone()], [zero, one.clone()]];
        for axis in 0..2 {
            let values = corners.map(|corner| if axis == 0 { &corner.x } else { &corner.y });
            let values = [
                fixed_from_rat(values[0], math)?,
                fixed_from_rat(values[1], math)?,
                fixed_from_rat(values[2], math)?,
                fixed_from_rat(values[3], math)?,
            ];
            let mut correction = FixedInterval::from_i64(0, math)?;
            for (value, weight) in values.iter().zip(&weights) {
                correction = correction.add(&value.mul(weight, math)?, math)?;
            }
            output[axis] = output[axis].add(&correction, math)?;
            if derivatives {
                let dx = values[1]
                    .sub(&values[0], math)?
                    .mul(&one.sub(&v, math)?, math)?
                    .add(&values[3].sub(&values[2], math)?.mul(&v, math)?, math)?
                    .div(&scale[0], math)?;
                let dy = values[2]
                    .sub(&values[0], math)?
                    .mul(&one.sub(&u, math)?, math)?
                    .add(&values[3].sub(&values[1], math)?.mul(&u, math)?, math)?
                    .div(&scale[1], math)?;
                jacobian[axis][0] = jacobian[axis][0].add(&dx, math)?;
                jacobian[axis][1] = jacobian[axis][1].add(&dy, math)?;
            }
        }
        progress.math_poll(math)?;
        Ok((output, jacobian))
    }
}
