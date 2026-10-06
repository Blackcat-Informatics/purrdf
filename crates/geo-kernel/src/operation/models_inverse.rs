// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Complete inverses on caller-declared metre rectangles. Smooth polynomial
//! domains and every complete closed grid patch use the same interval root law.
//! Exact affine reductions prove boundary roots without a dyadic boundary test.

use super::OperationCoordinates;

use super::{BilinearGrid, OperationPoint, OperationSolverLimits, Polynomial2d, rational_fields};
use crate::{
    GeoError, Rat,
    context::WorkProgress,
    numerical::{exact_bounds, fixed_from_bounds, fixed_from_rat, math_exact_rational as exact_op},
};
use purrdf_xsd::integer::ExactOperation as Op;
use purrdf_xsd::math::{
    CoordinateMath, FixedInterval, MathError, RootBox2, RootIsolationLimits, RootJacobian2,
    RootSystem2, isolate_roots2, root_enclosure_complete,
};

/// Explicit closed source rectangle for metre-coordinate models.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct MetricSourceDomain {
    lower: [Rat; 2],
    upper: [Rat; 2],
}
impl MetricSourceDomain {
    /// Declare finite exact bounds in each source metre axis.
    ///
    /// # Errors
    /// Refuses a rectangle with a zero or negative extent.
    pub fn new(lower: [Rat; 2], upper: [Rat; 2]) -> Result<Self, GeoError> {
        Self::new_in_budget(
            lower,
            upper,
            &mut crate::PreparationBudget::new(crate::ExecutionPolicy::geometry()),
        )
    }

    /// Validate exact metre extents within cumulative configuration admission.
    /// # Errors
    /// Refuses invalid extents and limits before exact comparisons.
    pub fn new_in_budget(
        lower: [Rat; 2],
        upper: [Rat; 2],
        budget: &mut crate::PreparationBudget,
    ) -> Result<Self, GeoError> {
        let cost = crate::numerical::rational_cost(
            Op::RationalCompare,
            &[&lower[0], &lower[1], &upper[0], &upper[1]],
            2,
        )
        .ok_or(GeoError::ArithmeticOverflow("configuration rational work"))?;
        budget.exact(cost, || Self::new_admitted(lower, upper))
    }

    fn new_admitted(lower: [Rat; 2], upper: [Rat; 2]) -> Result<Self, GeoError> {
        if (0..2).any(|axis| lower[axis] >= upper[axis]) {
            return Err(GeoError::config(
                "inverse source rectangle requires positive metre extents",
            ));
        }
        Ok(Self { lower, upper })
    }
    /// Inclusive lower metre coordinates.
    #[must_use]
    pub const fn lower(&self) -> &[Rat; 2] {
        &self.lower
    }
    /// Inclusive upper metre coordinates.
    #[must_use]
    pub const fn upper(&self) -> &[Rat; 2] {
        &self.upper
    }
    /// Exact membership, independent of numerical execution precision.
    #[must_use]
    pub fn contains(&self, point: &[Rat; 2]) -> bool {
        self.contains_pair([&point[0], &point[1]])
    }
    fn contains_pair(&self, point: [&Rat; 2]) -> bool {
        (0..2).all(|axis| &self.lower[axis] <= point[axis] && point[axis] <= &self.upper[axis])
    }
    fn contains_math(
        &self,
        point: [&Rat; 2],
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<bool, MathError> {
        crate::numerical::math_exact_rationals(
            Op::RationalCompare,
            &[
                point[0],
                point[1],
                &self.lower[0],
                &self.lower[1],
                &self.upper[0],
                &self.upper[1],
            ],
            4,
            math,
            progress,
            || self.contains_pair(point),
        )
    }
    fn parameters(&self, fields: &mut Vec<Vec<u8>>) {
        for value in self.lower.iter().chain(&self.upper) {
            rational_fields(value, fields);
        }
    }
    fn enclosure(&self, math: &mut CoordinateMath) -> Result<RootBox2, MathError> {
        Ok([
            fixed_from_bounds(&self.lower[0], &self.upper[0], math)?,
            fixed_from_bounds(&self.lower[1], &self.upper[1], math)?,
        ])
    }
}

/// A polynomial inverse with an explicit complete original source domain.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Polynomial2dInverse {
    /// Original normalized polynomial and exact coefficients.
    pub polynomial: Polynomial2d,
    /// Every admissible source root is sought in this closed metre rectangle.
    pub source_domain: MetricSourceDomain,
}
impl Polynomial2dInverse {
    pub(super) fn parameters(&self, fields: &mut Vec<Vec<u8>>) {
        self.polynomial.parameters(fields);
        self.source_domain.parameters(fields);
    }
    pub(super) fn evaluate(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        exact: bool,
        limits: OperationSolverLimits,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<OperationCoordinates, MathError> {
        let target = [coordinates[0].clone(), coordinates[1].clone()];
        if self.polynomial.degree() <= 1
            && let Some(target) = exact_target(point, &target, exact)
        {
            admit_rationals(
                self.polynomial
                    .terms()
                    .iter()
                    .flat_map(|term| [&term.x_coefficient, &term.y_coefficient])
                    .chain(self.polynomial.origin())
                    .chain(self.polynomial.scale())
                    .chain(target.iter()),
                math,
                progress,
            )?;
            let mut constant = [Rat::zero(), Rat::zero()];
            let mut matrix = [[Rat::zero(), Rat::zero()], [Rat::zero(), Rat::zero()]];
            for term in self.polynomial.terms() {
                let coefficient = [term.x_coefficient.clone(), term.y_coefficient.clone()];
                if (term.x_power, term.y_power) == (0, 0) {
                    constant = coefficient;
                } else {
                    let column = usize::from(term.y_power == 1);
                    for row in 0..2 {
                        matrix[row][column] = coefficient[row].clone();
                    }
                }
            }
            if let Some([normalized_x, normalized_y]) =
                solve_affine(&matrix, &constant, &target, math, progress)?
            {
                let normalized = [normalized_x, normalized_y];
                let mut root = [Rat::zero(), Rat::zero()];
                for axis in 0..2 {
                    let scale = &self.polynomial.scale()[axis];
                    let origin = &self.polynomial.origin()[axis];
                    let scaled = exact_op(
                        Op::RationalMultiply,
                        &[scale, &normalized[axis]],
                        math,
                        progress,
                        || scale.mul(&normalized[axis]),
                    )?;
                    root[axis] =
                        exact_op(Op::RationalAdd, &[origin, &scaled], math, progress, || {
                            origin.add(&scaled)
                        })?;
                }
                let operands = [
                    &self.source_domain.lower[0],
                    &self.source_domain.lower[1],
                    &self.source_domain.upper[0],
                    &self.source_domain.upper[1],
                    &root[0],
                    &root[1],
                ];
                if !crate::numerical::math_exact_rationals(
                    Op::RationalCompare,
                    &operands,
                    4,
                    math,
                    progress,
                    || self.source_domain.contains(&root),
                )? {
                    return Err(MathError::Domain("polynomial inverse has no source root"));
                }
                return image_with_height(&root, coordinates, math);
            }
        }
        let mut system = ModelSystem {
            model: SmoothModel::Polynomial(&self.polynomial),
            target: &target,
            source: &self.source_domain,
            quantize: limits.quantize_inverse,
            progress,
        };
        let roots = isolate_roots2(
            self.source_domain.enclosure(math)?,
            isolation_limits(limits),
            &mut system,
            math,
        )?;
        finish_roots(
            roots
                .into_iter()
                .map(purrdf_xsd::math::RootIsolation2::into_enclosure),
            coordinates,
        )
    }
    pub(super) fn certify(
        &self,
        target: &[FixedInterval],
        output: &[Rat],
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), MathError> {
        let inside = self
            .source_domain
            .contains_math([&output[0], &output[1]], math, progress)?;
        certify(
            inside,
            target,
            output,
            math,
            progress,
            |coordinates, math, progress| self.polynomial.evaluate(coordinates, math, progress),
        )
    }
}

/// A grid inverse on the closed union of complete patches inside exact bounds.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BilinearGridInverse {
    /// Original displacement grid, including its node holes.
    pub grid: BilinearGrid,
    /// Explicit original source rectangle; no root outside it is admissible.
    pub source_domain: MetricSourceDomain,
}
impl BilinearGridInverse {
    pub(super) fn validate(&self) -> Result<(), GeoError> {
        let (columns, rows) = self.grid.dimensions();
        let (_, upper) = self.grid.patch_bounds(columns - 2, rows - 2);
        if (0..2).any(|axis| {
            self.source_domain.lower[axis] < self.grid.origin()[axis]
                || self.source_domain.upper[axis] > upper[axis]
        }) {
            return Err(GeoError::config(
                "grid inverse source rectangle extends beyond declared grid",
            ));
        }
        Ok(())
    }
    pub(super) fn parameters(&self, fields: &mut Vec<Vec<u8>>) {
        self.grid.parameters(fields);
        self.source_domain.parameters(fields);
    }
    pub(super) fn evaluate(
        &self,
        point: &OperationPoint,
        coordinates: &[FixedInterval],
        exact: bool,
        limits: OperationSolverLimits,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<OperationCoordinates, MathError> {
        let target = [coordinates[0].clone(), coordinates[1].clone()];
        let target_exact = exact_target(point, &target, exact);
        admit_rationals(
            self.grid
                .origin()
                .iter()
                .chain(self.grid.spacing())
                .chain(self.source_domain.lower())
                .chain(self.source_domain.upper()),
            math,
            progress,
        )?;
        let (columns, rows) = self.grid.dimensions();
        let mut roots: Vec<RootBox2> = Vec::new();
        let mut exact_roots: Vec<[Rat; 2]> = Vec::new();
        let mut reserved = 0usize;
        let result = (|| {
            for row in 0..rows - 1 {
                for column in 0..columns - 1 {
                    math.admit_exact(1, math.limits().precision_bits as usize)?;
                    progress.math_poll(math)?;
                    let Ok(corners) = self.grid.corners(column, row) else {
                        continue;
                    };
                    let (lower, upper) =
                        self.grid.patch_bounds_math(column, row, math, progress)?;
                    let operands = [
                        &lower[0],
                        &lower[1],
                        &upper[0],
                        &upper[1],
                        &self.source_domain.lower[0],
                        &self.source_domain.lower[1],
                        &self.source_domain.upper[0],
                        &self.source_domain.upper[1],
                    ];
                    let (lower, upper, outside) = crate::numerical::math_exact_rationals(
                        Op::RationalCompare,
                        &operands,
                        8,
                        math,
                        progress,
                        || {
                            let lower = [
                                lower[0].clone().max(self.source_domain.lower[0].clone()),
                                lower[1].clone().max(self.source_domain.lower[1].clone()),
                            ];
                            let upper = [
                                upper[0].clone().min(self.source_domain.upper[0].clone()),
                                upper[1].clone().min(self.source_domain.upper[1].clone()),
                            ];
                            let outside = (0..2).any(|axis| lower[axis] > upper[axis]);
                            (lower, upper, outside)
                        },
                    )?;
                    if outside {
                        continue;
                    }
                    if let Some(target) = &target_exact {
                        admit_rationals(
                            corners
                                .iter()
                                .flat_map(|node| [&node.x, &node.y])
                                .chain(self.grid.origin())
                                .chain(self.grid.spacing())
                                .chain(target.iter()),
                            math,
                            progress,
                        )?;
                        let (origin, _) =
                            self.grid.patch_bounds_math(column, row, math, progress)?;
                        if let Some((matrix, constant)) =
                            grid_affine(&corners, &origin, self.grid.spacing(), math, progress)?
                            && let Some(root) =
                                solve_affine(&matrix, &constant, target, math, progress)?
                        {
                            let operands = [
                                &lower[0], &lower[1], &upper[0], &upper[1], &root[0], &root[1],
                            ];
                            let inside = crate::numerical::math_exact_rationals(
                                Op::RationalCompare,
                                &operands,
                                4,
                                math,
                                progress,
                                || {
                                    (0..2).all(|axis| {
                                        lower[axis] <= root[axis] && root[axis] <= upper[axis]
                                    })
                                },
                            )?;
                            if inside && !exact_roots.contains(&root) {
                                retain_bytes(&root, &mut reserved, math)?;
                                exact_roots
                                    .try_reserve_exact(1)
                                    .map_err(|_| MathError::WorkspaceExhausted)?;
                                exact_roots.push(root);
                            }
                            continue;
                        }
                    }
                    let domain = [
                        fixed_from_bounds(&lower[0], &upper[0], math)?,
                        fixed_from_bounds(&lower[1], &upper[1], math)?,
                    ];
                    let source_patch = MetricSourceDomain { lower, upper };
                    let mut system = ModelSystem {
                        model: SmoothModel::Grid(&self.grid, column, row),
                        target: &target,
                        source: &source_patch,
                        quantize: limits.quantize_inverse,
                        progress: &mut *progress,
                    };
                    let isolated =
                        isolate_roots2(domain, isolation_limits(limits), &mut system, math)?;
                    let bytes = isolated
                        .iter()
                        .try_fold(
                            isolated
                                .capacity()
                                .checked_mul(size_of::<purrdf_xsd::math::RootIsolation2>())
                                .ok_or(MathError::WorkspaceExhausted)?,
                            |bytes, root| {
                                root.enclosure()
                                    .iter()
                                    .chain(root.uniqueness_box())
                                    .try_fold(bytes, |bytes, interval| {
                                        bytes.checked_add(interval.workspace_bytes())
                                    })
                            },
                        )
                        .ok_or(MathError::WorkspaceExhausted)?;
                    math.reserve_workspace(bytes)?;
                    reserved = reserved
                        .checked_add(bytes)
                        .ok_or(MathError::WorkspaceExhausted)?;
                    for root in isolated {
                        retain_interval(root.enclosure(), &mut roots, &mut reserved, math)?;
                    }
                    math.release_workspace(bytes)?;
                    reserved -= bytes;
                }
            }
            if !roots.is_empty() && !exact_roots.is_empty() {
                return Err(MathError::PrecisionExhausted);
            }
            if !exact_roots.is_empty() {
                if exact_roots.len() > 1 {
                    return Err(MathError::AmbiguousRoots {
                        roots: exact_roots.len(),
                    });
                }
                return image_with_height(&exact_roots[0], coordinates, math);
            }
            finish_roots(roots.into_iter(), coordinates)
        })();
        math.release_workspace(reserved)?;
        result
    }
    pub(super) fn certify(
        &self,
        target: &[FixedInterval],
        output: &[Rat],
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), MathError> {
        let point = OperationPoint {
            x: output[0].clone(),
            y: output[1].clone(),
            z: None,
            epoch: None,
        };
        let inside = self
            .source_domain
            .contains_math([&point.x, &point.y], math, progress)?
            && match self
                .grid
                .location_math(&point.x, &point.y, true, math, progress)
            {
                Ok(_) => true,
                Err(MathError::Domain(_)) => false,
                Err(error) => return Err(error),
            };
        certify(
            inside,
            target,
            output,
            math,
            progress,
            |coordinates, math, progress| {
                self.grid
                    .evaluate_declared(&point, coordinates, true, math, progress)
            },
        )
    }
}

fn isolation_limits(limits: OperationSolverLimits) -> RootIsolationLimits {
    RootIsolationLimits {
        max_depth: limits.subdivisions,
        max_refinements: limits.iterations,
    }
}
fn exact_target(point: &OperationPoint, target: &RootBox2, exact: bool) -> Option<[Rat; 2]> {
    if exact {
        return Some([point.x.clone(), point.y.clone()]);
    }
    if target.iter().all(|value| value.lower() == value.upper()) {
        Some([exact_bounds(&target[0]).0, exact_bounds(&target[1]).0])
    } else {
        None
    }
}
fn admit_rationals<'a>(
    values: impl Iterator<Item = &'a Rat>,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<(), MathError> {
    for value in values {
        exact_op(Op::Linear, &[value], math, progress, || ())?;
    }
    Ok(())
}
fn binary(
    operation: Op,
    a: &Rat,
    b: &Rat,
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    evaluate: fn(&Rat, &Rat) -> Rat,
) -> Result<Rat, MathError> {
    exact_op(operation, &[a, b], math, progress, || evaluate(a, b))
}
fn positive_quotient(a: &Rat, b: &Rat) -> Rat {
    a.div(b).expect("validated positive grid spacing")
}
type AffineModel2 = ([[Rat; 2]; 2], [Rat; 2]);

fn grid_affine(
    corners: &[&super::GridNode; 4],
    origin: &[Rat; 2],
    spacing: &[Rat; 2],
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<AffineModel2>, MathError> {
    let mut matrix = [[Rat::one(), Rat::zero()], [Rat::zero(), Rat::one()]];
    let mut constant = [Rat::zero(), Rat::zero()];
    for axis in 0..2 {
        let values = corners.map(|node| if axis == 0 { &node.x } else { &node.y });
        let left = binary(
            Op::RationalAdd,
            values[3],
            values[1],
            math,
            progress,
            Rat::sub,
        )?;
        let left = binary(Op::RationalAdd, &left, values[2], math, progress, Rat::sub)?;
        let mixed = binary(Op::RationalAdd, &left, values[0], math, progress, Rat::add)?;
        if mixed != Rat::zero() {
            return Ok(None);
        }
        let dx = binary(
            Op::RationalAdd,
            values[1],
            values[0],
            math,
            progress,
            Rat::sub,
        )?;
        let dx = binary(
            Op::RationalDivide,
            &dx,
            &spacing[0],
            math,
            progress,
            positive_quotient,
        )?;
        let dy = binary(
            Op::RationalAdd,
            values[2],
            values[0],
            math,
            progress,
            Rat::sub,
        )?;
        let dy = binary(
            Op::RationalDivide,
            &dy,
            &spacing[1],
            math,
            progress,
            positive_quotient,
        )?;
        matrix[axis][0] = binary(
            Op::RationalAdd,
            &matrix[axis][0],
            &dx,
            math,
            progress,
            Rat::add,
        )?;
        matrix[axis][1] = binary(
            Op::RationalAdd,
            &matrix[axis][1],
            &dy,
            math,
            progress,
            Rat::add,
        )?;
        let x = binary(
            Op::RationalMultiply,
            &dx,
            &origin[0],
            math,
            progress,
            Rat::mul,
        )?;
        let y = binary(
            Op::RationalMultiply,
            &dy,
            &origin[1],
            math,
            progress,
            Rat::mul,
        )?;
        let c = binary(Op::RationalAdd, values[0], &x, math, progress, Rat::sub)?;
        constant[axis] = binary(Op::RationalAdd, &c, &y, math, progress, Rat::sub)?;
    }
    Ok(Some((matrix, constant)))
}
fn solve_affine(
    matrix: &[[Rat; 2]; 2],
    constant: &[Rat; 2],
    target: &[Rat; 2],
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
) -> Result<Option<[Rat; 2]>, MathError> {
    let left = exact_op(
        Op::RationalMultiply,
        &[&matrix[0][0], &matrix[1][1]],
        math,
        progress,
        || matrix[0][0].mul(&matrix[1][1]),
    )?;
    let right = exact_op(
        Op::RationalMultiply,
        &[&matrix[0][1], &matrix[1][0]],
        math,
        progress,
        || matrix[0][1].mul(&matrix[1][0]),
    )?;
    let determinant = exact_op(Op::RationalAdd, &[&left, &right], math, progress, || {
        left.sub(&right)
    })?;
    if determinant == Rat::zero() {
        return Ok(None);
    }
    let mut residual = [Rat::zero(), Rat::zero()];
    for (axis, value) in residual.iter_mut().enumerate() {
        *value = exact_op(
            Op::RationalAdd,
            &[&target[axis], &constant[axis]],
            math,
            progress,
            || target[axis].sub(&constant[axis]),
        )?;
    }
    let mut root = [Rat::zero(), Rat::zero()];
    for (axis, coordinate) in root.iter_mut().enumerate() {
        let (a, b, c, d) = if axis == 0 {
            (&residual[0], &matrix[1][1], &matrix[0][1], &residual[1])
        } else {
            (&matrix[0][0], &residual[1], &residual[0], &matrix[1][0])
        };
        let left = exact_op(Op::RationalMultiply, &[a, b], math, progress, || a.mul(b))?;
        let right = exact_op(Op::RationalMultiply, &[c, d], math, progress, || c.mul(d))?;
        let numerator = exact_op(Op::RationalAdd, &[&left, &right], math, progress, || {
            left.sub(&right)
        })?;
        *coordinate = exact_op(
            Op::RationalDivide,
            &[&numerator, &determinant],
            math,
            progress,
            || numerator.div(&determinant).expect("nonzero determinant"),
        )?;
    }
    Ok(Some(root))
}

fn image_with_height(
    root: &[Rat; 2],
    coordinates: &[FixedInterval],
    math: &mut CoordinateMath,
) -> Result<OperationCoordinates, MathError> {
    let mut image: OperationCoordinates = purrdf_core::smallvec![
        fixed_from_rat(&root[0], math)?,
        fixed_from_rat(&root[1], math)?,
    ];
    if let Some(z) = coordinates.get(2) {
        image.push(z.clone());
    }
    Ok(image)
}
fn finish_roots(
    mut roots: impl ExactSizeIterator<Item = RootBox2>,
    coordinates: &[FixedInterval],
) -> Result<OperationCoordinates, MathError> {
    if roots.len() > 1 {
        return Err(MathError::AmbiguousRoots { roots: roots.len() });
    }
    let root = roots.next().ok_or(MathError::Domain(
        "inverse model has no root in its explicit source domain",
    ))?;
    let mut image = OperationCoordinates::from_array(root);
    if let Some(z) = coordinates.get(2) {
        image.push(z.clone());
    }
    Ok(image)
}
fn retain_bytes(
    root: &[Rat; 2],
    reserved: &mut usize,
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    let bytes = root
        .iter()
        .try_fold(size_of::<[Rat; 2]>() + 64, |bytes, value| {
            bytes
                .checked_add(value.numerator().allocated_bytes())
                .and_then(|bytes| bytes.checked_add(value.denominator().allocated_bytes()))
        })
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(bytes)?;
    *reserved = reserved
        .checked_add(bytes)
        .ok_or(MathError::WorkspaceExhausted)?;
    Ok(())
}
fn retain_interval(
    root: &RootBox2,
    roots: &mut Vec<RootBox2>,
    reserved: &mut usize,
    math: &mut CoordinateMath,
) -> Result<(), MathError> {
    for previous in roots.iter() {
        if (0..2).all(|axis| {
            previous[axis].lower() == previous[axis].upper()
                && root[axis].lower() == root[axis].upper()
                && previous[axis] == root[axis]
        }) {
            return Ok(());
        }
        if (0..2).all(|axis| {
            previous[axis].lower() <= root[axis].upper()
                && root[axis].lower() <= previous[axis].upper()
        }) {
            return Err(MathError::PrecisionExhausted);
        }
    }
    let bytes = root
        .iter()
        .try_fold(size_of::<RootBox2>() + 64, |bytes, interval| {
            bytes.checked_add(interval.workspace_bytes())
        })
        .ok_or(MathError::WorkspaceExhausted)?;
    math.reserve_workspace(bytes)?;
    *reserved = reserved
        .checked_add(bytes)
        .ok_or(MathError::WorkspaceExhausted)?;
    roots
        .try_reserve_exact(1)
        .map_err(|_| MathError::WorkspaceExhausted)?;
    roots.push(root.clone());
    Ok(())
}
#[derive(Clone, Copy)]
enum SmoothModel<'a> {
    Polynomial(&'a Polynomial2d),
    Grid(&'a BilinearGrid, u32, u32),
}
struct ModelSystem<'a, 'observer> {
    model: SmoothModel<'a>,
    target: &'a RootBox2,
    source: &'a MetricSourceDomain,
    quantize: bool,
    progress: &'a mut WorkProgress<'observer>,
}
impl RootSystem2 for ModelSystem<'_, '_> {
    fn image(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootBox2, MathError> {
        let image = match self.model {
            SmoothModel::Polynomial(model) => model.evaluate(domain, math, self.progress)?,
            SmoothModel::Grid(model, column, row) => {
                model.patch_image(domain, column, row, math, self.progress)?
            }
        };
        Ok([
            image[0].sub(&self.target[0], math)?,
            image[1].sub(&self.target[1], math)?,
        ])
    }
    fn jacobian(
        &mut self,
        domain: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<RootJacobian2, MathError> {
        match self.model {
            SmoothModel::Polynomial(model) => model
                .differential(domain, math, self.progress)
                .map(|(_, jacobian)| jacobian),
            SmoothModel::Grid(model, column, row) => model
                .patch_differential(domain, column, row, math, self.progress)
                .map(|(_, jacobian)| jacobian),
        }
    }
    fn complete(
        &mut self,
        enclosure: &RootBox2,
        math: &mut CoordinateMath,
    ) -> Result<bool, MathError> {
        for (axis, coordinate) in enclosure.iter().enumerate() {
            let (lower, upper) = exact_bounds(coordinate);
            let outside = crate::numerical::math_exact_rationals(
                Op::RationalCompare,
                &[
                    &lower,
                    &upper,
                    &self.source.lower[axis],
                    &self.source.upper[axis],
                ],
                2,
                math,
                self.progress,
                || lower < self.source.lower[axis] || upper > self.source.upper[axis],
            )?;
            if outside {
                return Ok(false);
            }
        }
        if self.quantize {
            for coordinate in enclosure {
                let (lower, upper) = coordinate.round_decimal(6, math)?;
                if lower != upper {
                    return Ok(false);
                }
            }
            Ok(true)
        } else {
            root_enclosure_complete(
                enclosure,
                self.target,
                &self.jacobian(enclosure, math)?,
                6,
                false,
                math,
            )
        }
    }
    fn poll(&mut self, math: &CoordinateMath) -> Result<(), MathError> {
        self.progress.math_poll(math)
    }
}

fn certify(
    inside: bool,
    target: &[FixedInterval],
    output: &[Rat],
    math: &mut CoordinateMath,
    progress: &mut WorkProgress<'_>,
    mut forward: impl FnMut(
        &[FixedInterval],
        &mut CoordinateMath,
        &mut WorkProgress<'_>,
    ) -> Result<OperationCoordinates, MathError>,
) -> Result<(), MathError> {
    if !inside {
        return Err(MathError::PrecisionExhausted);
    }
    let source = [
        fixed_from_rat(&output[0], math)?,
        fixed_from_rat(&output[1], math)?,
    ];
    let image = forward(&source, math, progress)?;
    let tolerance = fixed_from_rat(
        &Rat::parse_decimal("0.000001").expect("exact decimal"),
        math,
    )?;
    let residual = image
        .iter()
        .zip(target)
        .take(2)
        .try_fold(
            FixedInterval::from_i64(0, math)?,
            |sum, (actual, target)| sum.add(&actual.sub(target, math)?.square(math)?, math),
        )?
        .sqrt(math)?;
    if residual.upper() > tolerance.lower() {
        return Err(MathError::PrecisionExhausted);
    }
    progress.math_poll(math)
}

#[cfg(test)]
mod tests;
