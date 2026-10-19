// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Caller-defined normalized polynomials and regular in-memory bilinear grids.

use super::OperationCoordinates;

use purrdf_xsd::math::{CoordinateMath, FixedInterval, MathError};

use super::{OperationPoint, ParameterSink};
use crate::context::WorkProgress;
use crate::context::retained::NativeDetached;
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

/// Exact container/control quotations of the native constructors below. The
/// stable sorter keeps two usize index arrays; cells remain in their original
/// caller buffer while every detached field uses the common native factory.
fn native_parameter_array_bytes<T: NativeDetached>(
    values: &Vec<T>,
    sort: bool,
    scalars: &[&[Rat; 2]],
    observer: Option<&dyn crate::MetricWorkObserver>,
) -> Result<u64, GeoError> {
    use core::alloc::Layout;
    use purrdf_lex::allocation::StorageError;
    let owner = observer
        .map_or_else(
            || WorkProgress::integer(None).retained_owner_layout(),
            super::super::context::MetricWorkObserver::retained_owner_layout,
        )
        .size();
    let mut bytes = 0_usize;
    if values.capacity() != 0 {
        bytes = Layout::array::<T>(values.capacity())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?
            .size()
            .checked_add(crate::PreparedValue::<Vec<T>>::allocation_layout().size())
            .and_then(|bytes| bytes.checked_add(owner))
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        for value in values {
            bytes = bytes
                .checked_add(value.layout_bytes().map_err(GeoError::Storage)?)
                .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
        }
    }
    if sort && values.len() > 1 {
        let indices = Layout::array::<usize>(values.len())
            .map_err(|_| GeoError::Storage(StorageError::SizeOverflow))?
            .size();
        bytes = bytes
            .checked_add(indices)
            .and_then(|bytes| bytes.checked_add(indices))
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
    }
    let mut scalar_bytes = 0_usize;
    for scalar in scalars {
        scalar_bytes = scalar_bytes
            .checked_add(scalar.layout_bytes().map_err(GeoError::Storage)?)
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
    }
    if scalar_bytes != 0 {
        bytes = bytes
            .checked_add(scalar_bytes)
            .and_then(|bytes| bytes.checked_add(owner))
            .ok_or(GeoError::Storage(StorageError::SizeOverflow))?;
    }
    u64::try_from(bytes).map_err(|_| GeoError::Storage(StorageError::SizeOverflow))
}

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
impl purrdf_hash::error::StructuredError for PolynomialTerm {
    purrdf_hash::error_visit! {
        @record self { x_power, y_power, x_coefficient, y_coefficient } => "polynomial-term" {
            "x_power" => Unsigned(u128::from(self.x_power)), "y_power" => Unsigned(u128::from(self.y_power)),
            "x_coefficient" => Record(&self.x_coefficient), "y_coefficient" => Record(&self.y_coefficient)
        }
    }
}

crate::context::retained::native_detached_fields!(PolynomialTerm {
    native: [x_coefficient, y_coefficient],
    copy: [x_power, y_power]
});

/// Explicit monomial polynomial: `(x-origin_x)/scale_x`, `(y-origin_y)/scale_y`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Polynomial2d {
    degree: u16,
    origin: [Rat; 2],
    scale: [Rat; 2],
    terms: crate::PreparedValues<PolynomialTerm>,
}
impl purrdf_hash::error::StructuredError for Polynomial2d {
    purrdf_hash::error_visit! {
        @record self { degree, origin, scale, terms } => "polynomial-2d" {
            "degree" => Unsigned(u128::from(self.degree)),
            "origin" => List(self.origin.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "scale" => List(self.scale.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "terms" => List(self.terms.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value)))
        }
    }
}

crate::context::retained::native_detached_fields!(Polynomial2d {
    native: [origin, scale, terms],
    copy: [degree]
});

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
        Self::new_in_budget_observed(degree, origin, scale, terms, budget, None)
    }

    /// Compile the original monomial model under one native storage owner.
    /// Canonical sorting uses the shared admitted stable sorter; coefficients
    /// and immutable tables retain their original lease through every clone.
    ///
    /// # Errors
    /// Preserves declaration, native work, cancellation and physical refusal.
    pub fn new_in_budget_metered(
        degree: u16,
        origin: [Rat; 2],
        scale: [Rat; 2],
        terms: Vec<PolynomialTerm>,
        budget: &mut crate::PreparationBudget,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::new_in_budget_observed(degree, origin, scale, terms, budget, Some(observer))
    }

    fn new_in_budget_observed(
        degree: u16,
        origin: [Rat; 2],
        scale: [Rat; 2],
        terms: Vec<PolynomialTerm>,
        budget: &mut crate::PreparationBudget,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let count = u64::try_from(terms.len())
            .map_err(|_| GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow))?;
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
        let source = [&origin, &scale];
        let destination_bits = cost.output_bits;
        cost = cost
            .followed_by(
                crate::numerical::rational_copy_cost(
                    origin.iter().chain(&scale).chain(
                        terms
                            .iter()
                            .flat_map(|term| [&term.x_coefficient, &term.y_coefficient]),
                    ),
                )
                .ok_or(GeoError::ArithmeticOverflow("polynomial native copy work"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow("polynomial native copy work"))?;
        let bytes = native_parameter_array_bytes(&terms, true, &source, observer.as_deref())?;
        cost.workspace_bytes = cost
            .workspace_bytes
            .checked_add(bytes)
            .ok_or(GeoError::Storage(
                purrdf_lex::allocation::StorageError::SizeOverflow,
            ))?;
        budget.exact_values_with_bits_observed(
            cost,
            destination_bits,
            |values, progress| Self::new_admitted(degree, origin, scale, terms, values, progress),
            observer,
        )
    }

    fn new_admitted(
        degree: u16,
        origin: [Rat; 2],
        scale: [Rat; 2],
        terms: Vec<PolynomialTerm>,
        values: &crate::numerical::ExactValues<'_>,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        for (axis, value) in scale.iter().enumerate() {
            if !values
                .compare(value, &Rat::zero())
                .map_err(GeoError::NumericalScratch)?
                .is_gt()
            {
                return Err(crate::PolynomialInputError::normalization(
                    degree,
                    origin,
                    scale,
                    axis,
                    terms.len(),
                    progress,
                ));
            }
        }
        let terms = crate::PreparedValues::from_vec_with(
            terms,
            progress,
            |terms, memory| {
                memory
                    .sort_by(terms, |a, b| {
                        (a.x_power, a.y_power).cmp(&(b.x_power, b.y_power))
                    })
                    .map_err(GeoError::Storage)?;
                for (index, term) in terms.iter().enumerate() {
                    if u32::from(term.x_power) + u32::from(term.y_power) > u32::from(degree) {
                        return Err(crate::PolynomialInputError::monomial_with_memory(
                            degree,
                            index,
                            terms.len(),
                            term,
                            None,
                            memory,
                        )
                        .map_err(GeoError::Storage)?);
                    }
                }
                if let Some(index) = terms.windows(2).position(|pair| {
                    (pair[0].x_power, pair[0].y_power) == (pair[1].x_power, pair[1].y_power)
                }) {
                    return Err(crate::PolynomialInputError::monomial_with_memory(
                        degree,
                        index,
                        terms.len(),
                        &terms[index],
                        Some(&terms[index + 1]),
                        memory,
                    )
                    .map_err(GeoError::Storage)?);
                }
                for term in terms.iter_mut() {
                    *term = term.prepare(memory).map_err(GeoError::Storage)?;
                }
                Ok(())
            },
            |terms, original| {
                for term in terms {
                    term.publish(original).map_err(GeoError::Storage)?;
                }
                Ok(())
            },
        )?;
        let [origin, scale] =
            crate::context::retained::produce_persistent_numeric([origin, scale], progress)?
                .into_value();
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
                        |values| values.original(&(self.origin[axis])),
                    )
                    .map(Some);
            };
            let two = Rat::from_i64(2);
            let divisor = admission.rational_owner(
                ExactOperation::RationalMultiply,
                &[quadratic, &two],
                retained,
                |values| values.multiply(quadratic, &two),
            )?;
            let relative = admission.rational_owner(
                ExactOperation::RationalDivide,
                &[linear, &divisor],
                retained,
                |values| Ok(values.divide(linear, &divisor)?.expect("nonzero quadratic")),
            )?;
            let offset = admission.rational_owner(
                ExactOperation::RationalMultiply,
                &[&self.scale[axis], &relative],
                retained,
                |values| values.multiply(&(self.scale[axis]), &relative),
            )?;
            return admission
                .rational_owner(
                    ExactOperation::RationalAdd,
                    &[&self.origin[axis], &offset],
                    retained,
                    |values| values.subtract(&(self.origin[axis]), &offset),
                )
                .map(Some);
        }
        Ok(None)
    }
    pub(super) fn container_bytes(&self) -> Option<u64> {
        u64::try_from(self.terms.admitted_bytes()).ok()
    }

    pub(super) fn parameters(
        &self,
        fields: &mut (impl ParameterSink + ?Sized),
    ) -> Result<(), GeoError> {
        fields.bytes(&self.degree.to_be_bytes())?;
        for value in self.origin.iter().chain(self.scale.iter()) {
            fields.rational(value)?;
        }
        for term in &self.terms {
            fields.bytes(&term.x_power.to_be_bytes())?;
            fields.bytes(&term.y_power.to_be_bytes())?;
            fields.rational(&term.x_coefficient)?;
            fields.rational(&term.y_coefficient)?;
        }
        Ok(())
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
impl purrdf_hash::error::StructuredError for GridNode {
    purrdf_hash::error_visit! {
        @record self { x, y } => "grid-node" { "x" => Record(&self.x), "y" => Record(&self.y) }
    }
}

/// The first absent node in the original ordered four-corner lookup.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridMissingNode {
    /// Original patch column.
    pub column: u32,
    /// Original patch row.
    pub row: u32,
    /// Original missing node column.
    pub node_column: u32,
    /// Original missing node row.
    pub node_row: u32,
    /// Original row-major lookup index.
    pub index: usize,
}
impl purrdf_hash::error::StructuredError for GridMissingNode {
    purrdf_hash::error_visit! {
        @record self { column, row, node_column, node_row, index } => "grid-missing-node" {
            "column" => Unsigned(u128::from(self.column)), "row" => Unsigned(u128::from(self.row)),
            "node_column" => Unsigned(u128::from(self.node_column)), "node_row" => Unsigned(u128::from(self.node_row)), "index" => Unsigned(self.index as u128)
        }
    }
}
impl core::fmt::Display for GridMissingNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("bilinear grid cell touches a missing node")
    }
}
impl std::error::Error for GridMissingNode {}

/// Original exact locator data; trial panels may use it only to refine precision.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GridLocationRefusal {
    /// The original normalized coordinate is outside the closed grid extent.
    OutsideExtent {
        /// First failed zero-based coordinate axis.
        axis: usize,
        /// Original computed normalized coordinate.
        relative: Rat,
        /// Original greatest node index on that axis.
        last_node: u32,
    },
    /// No complete incident patch contains the original point.
    NoCompletePatch {
        /// Original located patch column.
        column: u32,
        /// Original located patch row.
        row: u32,
        /// First incident column actually searched.
        first_column: u32,
        /// First incident row actually searched.
        first_row: u32,
    },
}
impl purrdf_hash::error::StructuredError for GridLocationRefusal {
    purrdf_hash::error_visit! {
        OutsideExtent { axis, relative, last_node } => "grid-location-outside-extent" {
            "axis" => Unsigned(*axis as u128), "relative" => Record(relative), "last_node" => Unsigned(u128::from(*last_node))
        },
        NoCompletePatch { column, row, first_column, first_row } => "grid-location-no-complete-patch" {
            "column" => Unsigned(u128::from(*column)), "row" => Unsigned(u128::from(*row)),
            "first_column" => Unsigned(u128::from(*first_column)), "first_row" => Unsigned(u128::from(*first_row))
        },
    }
}
crate::context::retained::native_detached_enum!(GridLocationRefusal {
    units: [],
    tuples: [],
    fields: [
        OutsideExtent {
            native: [relative],
            copy: [axis, last_node]
        },
        NoCompletePatch {
            native: [],
            copy: [column, row, first_column, first_row]
        }
    ]
});
impl core::fmt::Display for GridLocationRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::OutsideExtent { .. } => "bilinear grid does not extrapolate outside its extent",
            Self::NoCompletePatch { .. } => {
                "bilinear grid point is outside all complete closed patches"
            }
        })
    }
}
impl std::error::Error for GridLocationRefusal {}

/// A domain probe remains separate from an actual operational failure. Only
/// domain uncertainty in a trial enclosure may request greater precision.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum GridLocationFailure {
    Domain(GridLocationRefusal),
    IndexOverflow,
}
impl purrdf_hash::error::StructuredError for GridLocationFailure {
    purrdf_hash::error_visit! {
        Domain(cause) => "grid-location-domain" { "cause" => Cause(cause) },
        IndexOverflow => "grid-location-index-overflow" {},
    }
}
impl core::fmt::Display for GridLocationFailure {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Domain(cause) => core::fmt::Display::fmt(cause, f),
            Self::IndexOverflow => {
                core::fmt::Display::fmt(&GeoError::ArithmeticOverflow("grid cell index"), f)
            }
        }
    }
}
impl std::error::Error for GridLocationFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Domain(cause) => Some(cause),
            Self::IndexOverflow => None,
        }
    }
}
impl GridLocationFailure {
    fn into_geo(
        self,
        grid: &BilinearGrid,
        point: [&Rat; 2],
        progress: &mut WorkProgress<'_>,
    ) -> GeoError {
        match self {
            Self::Domain(cause) => crate::ModelInputError::location(grid, point, &cause, progress),
            Self::IndexOverflow => GeoError::ArithmeticOverflow("grid cell index"),
        }
    }
}

crate::context::retained::native_detached_fields!(GridNode {
    native: [x, y],
    copy: []
});

/// A regular bilinear displacement grid; a missing node makes its incident cells holes.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BilinearGrid {
    origin: [Rat; 2],
    spacing: [Rat; 2],
    columns: u32,
    rows: u32,
    nodes: crate::PreparedValues<Option<GridNode>>,
}
impl purrdf_hash::error::StructuredError for BilinearGrid {
    purrdf_hash::error_visit! {
        @record self { origin, spacing, columns, rows, nodes } => "bilinear-grid" {
            "origin" => List(self.origin.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "spacing" => List(self.spacing.iter().map(|value| purrdf_hash::error::ErrorValue::Record(value))),
            "columns" => Unsigned(u128::from(self.columns)), "rows" => Unsigned(u128::from(self.rows)),
            "nodes" => List(self.nodes.iter().map(|value| match value {
                Some(node) => purrdf_hash::error::ErrorValue::Record(node), None => purrdf_hash::error::ErrorValue::Absent,
            }))
        }
    }
}

crate::context::retained::native_detached_fields!(BilinearGrid {
    native: [origin, spacing, nodes],
    copy: [columns, rows]
});

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
        Self::new_in_budget_observed(origin, spacing, columns, rows, nodes, budget, None)
    }

    /// Validate and publish the complete declared grid through the native
    /// immutable array/scalar factories under the caller's original account.
    ///
    /// # Errors
    /// Preserves shape, work, cancellation and physical allocation failures.
    pub fn new_in_budget_metered(
        origin: [Rat; 2],
        spacing: [Rat; 2],
        columns: u32,
        rows: u32,
        nodes: Vec<Option<GridNode>>,
        budget: &mut crate::PreparationBudget,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::new_in_budget_observed(
            origin,
            spacing,
            columns,
            rows,
            nodes,
            budget,
            Some(observer),
        )
    }

    fn new_in_budget_observed(
        origin: [Rat; 2],
        spacing: [Rat; 2],
        columns: u32,
        rows: u32,
        nodes: Vec<Option<GridNode>>,
        budget: &mut crate::PreparationBudget,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        let mut cost = crate::numerical::rational_cost(
            ExactOperation::RationalCompare,
            &[&spacing[0], &spacing[1]],
            2,
        )
        .ok_or(GeoError::ArithmeticOverflow("configuration rational work"))?;
        let source = [&origin, &spacing];
        let destination_bits = cost.output_bits;
        cost = cost
            .followed_by(
                crate::numerical::rational_copy_cost(
                    origin
                        .iter()
                        .chain(&spacing)
                        .chain(nodes.iter().flatten().flat_map(|node| [&node.x, &node.y])),
                )
                .ok_or(GeoError::ArithmeticOverflow("grid native copy work"))?,
            )
            .ok_or(GeoError::ArithmeticOverflow("grid native copy work"))?;
        cost.workspace_bytes = cost
            .workspace_bytes
            .checked_add(native_parameter_array_bytes(
                &nodes,
                false,
                &source,
                observer.as_deref(),
            )?)
            .ok_or(GeoError::Storage(
                purrdf_lex::allocation::StorageError::SizeOverflow,
            ))?;
        budget.exact_values_with_bits_observed(
            cost,
            destination_bits,
            |values, progress| {
                Self::new_admitted(origin, spacing, columns, rows, nodes, values, progress)
            },
            observer,
        )
    }

    fn new_admitted(
        origin: [Rat; 2],
        spacing: [Rat; 2],
        columns: u32,
        rows: u32,
        nodes: Vec<Option<GridNode>>,
        values: &crate::numerical::ExactValues<'_>,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Self, GeoError> {
        let mut reason = if columns < 2 {
            Some(crate::GridConfigurationReason::Columns)
        } else if rows < 2 {
            Some(crate::GridConfigurationReason::Rows)
        } else {
            None
        };
        if reason.is_none() {
            for (axis, value) in spacing.iter().enumerate() {
                if !values
                    .compare(value, &Rat::zero())
                    .map_err(GeoError::NumericalScratch)?
                    .is_gt()
                {
                    reason = Some(crate::GridConfigurationReason::Spacing { axis });
                    break;
                }
            }
        }
        if let Some(reason) = reason {
            return Err(crate::ModelInputError::grid_configuration(
                origin,
                spacing,
                (columns, rows),
                nodes.len(),
                reason,
                progress,
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
        let nodes = crate::PreparedValues::from_vec_with(
            nodes,
            progress,
            |nodes, memory| {
                for node in nodes {
                    *node = node.prepare(memory).map_err(GeoError::Storage)?;
                }
                Ok(())
            },
            |nodes, original| {
                for node in nodes {
                    node.publish(original).map_err(GeoError::Storage)?;
                }
                Ok(())
            },
        )?;
        let [origin, spacing] =
            crate::context::retained::produce_persistent_numeric([origin, spacing], progress)?
                .into_value();
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
        u64::try_from(self.nodes.admitted_bytes()).ok()
    }

    pub(super) fn parameters(
        &self,
        fields: &mut (impl ParameterSink + ?Sized),
    ) -> Result<(), GeoError> {
        for value in self.origin.iter().chain(self.spacing.iter()) {
            fields.rational(value)?;
        }
        fields.bytes(&self.columns.to_be_bytes())?;
        fields.bytes(&self.rows.to_be_bytes())?;
        for node in &self.nodes {
            fields.bytes(&[u8::from(node.is_some())])?;
            if let Some(node) = node {
                fields.rational(&node.x)?;
                fields.rational(&node.y)?;
            }
        }
        Ok(())
    }
    #[allow(
        clippy::result_large_err,
        reason = "The original exact normalized coordinate stays inline until its caller admits the retained diagnostic; boxing the trial refusal would allocate outside that admission"
    )]
    pub(super) fn cell(&self, x: &Rat, y: &Rat) -> Result<(u32, u32), GridLocationFailure> {
        let coordinate =
            |value: &Rat, axis: usize, count: u32| -> Result<u32, GridLocationFailure> {
                let relative = value
                    .sub(&self.origin[axis])
                    .div(&self.spacing[axis])
                    .expect("positive spacing");
                if relative < Rat::zero() || relative > Rat::from_i64(i64::from(count - 1)) {
                    return Err(GridLocationFailure::Domain(
                        GridLocationRefusal::OutsideExtent {
                            axis,
                            relative,
                            last_node: count - 1,
                        },
                    ));
                }
                let index = relative
                    .numerator()
                    .div_rem(relative.denominator().as_ref())
                    .expect("positive denominator")
                    .0
                    .to_i128()
                    .ok_or(GridLocationFailure::IndexOverflow)?;
                Ok(u32::try_from(index)
                    .map_err(|_| GridLocationFailure::IndexOverflow)?
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
        match self.location_result_math(x, y, complete, math, progress)? {
            Ok(location) => Ok(location),
            // Trial-panel uncertainty requests refinement, not a latched failure.
            Err(GridLocationFailure::Domain(_)) if !complete => Err(MathError::PrecisionExhausted),
            Err(error) => Err(self.location_error_math(error, x, y, progress)),
        }
    }

    /// Certification may reject a trial point without rejecting the operation.
    /// Cost admission and both polling boundaries are the declared path's.
    pub(super) fn contains_complete_patch_math(
        &self,
        x: &Rat,
        y: &Rat,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<bool, MathError> {
        match self.location_result_math(x, y, true, math, progress)? {
            Ok(_) => Ok(true),
            Err(GridLocationFailure::Domain(_)) => Ok(false),
            Err(error) => Err(self.location_error_math(error, x, y, progress)),
        }
    }

    fn location_error_math(
        &self,
        error: GridLocationFailure,
        x: &Rat,
        y: &Rat,
        progress: &mut WorkProgress<'_>,
    ) -> MathError {
        let error = error.into_geo(self, [x, y], progress);
        GeoError::math_validation_failure(
            error,
            crate::OperationValidationContext::GridLocation,
            progress,
        )
    }

    fn location_result_math(
        &self,
        x: &Rat,
        y: &Rat,
        complete: bool,
        math: &mut CoordinateMath,
        progress: &mut WorkProgress<'_>,
    ) -> Result<Result<(u32, u32), GridLocationFailure>, MathError> {
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
        Ok(location)
    }
    pub(super) fn corners(&self, column: u32, row: u32) -> Result<[&GridNode; 4], GridMissingNode> {
        let get = |x: u32, y: u32| {
            let index = (u64::from(y) * u64::from(self.columns) + u64::from(x)) as usize;
            self.nodes[index].as_ref().ok_or(GridMissingNode {
                column,
                row,
                node_column: x,
                node_row: y,
                index,
            })
        };
        Ok([
            get(column, row)?,
            get(column + 1, row)?,
            get(column, row + 1)?,
            get(column + 1, row + 1)?,
        ])
    }
    pub(super) fn validate_point(
        &self,
        point: &OperationPoint,
        progress: &mut WorkProgress<'_>,
    ) -> Result<(), GeoError> {
        self.complete_patch(&point.x, &point.y)
            .map_err(|error| error.into_geo(self, [&point.x, &point.y], progress))?;
        Ok(())
    }

    // The domain is the closed union of complete patches. A missing node does
    // not remove the boundary of an adjacent complete patch.
    #[allow(
        clippy::result_large_err,
        reason = "The original exact normalized coordinate stays inline until its caller admits the retained diagnostic; boxing the trial refusal would allocate outside that admission"
    )]
    pub(super) fn complete_patch(
        &self,
        x: &Rat,
        y: &Rat,
    ) -> Result<(u32, u32), GridLocationFailure> {
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
        Err(GridLocationFailure::Domain(
            GridLocationRefusal::NoCompletePatch {
                column,
                row,
                first_column,
                first_row,
            },
        ))
    }

    pub(super) fn patch_bounds_with(
        &self,
        column: u32,
        row: u32,
        values: &crate::numerical::ExactValues<'_>,
    ) -> Result<([Rat; 2], [Rat; 2]), purrdf_xsd::integer::LimbScratchError> {
        let lower = [
            values.add(
                &self.origin[0],
                &values.multiply(&self.spacing[0], &Rat::from_i64(i64::from(column)))?,
            )?,
            values.add(
                &self.origin[1],
                &values.multiply(&self.spacing[1], &Rat::from_i64(i64::from(row)))?,
            )?,
        ];
        let upper = [
            values.add(&lower[0], &self.spacing[0])?,
            values.add(&lower[1], &self.spacing[1])?,
        ];
        Ok((lower, upper))
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
        let result = self
            .patch_bounds_with(column, row, &crate::numerical::ExactValues::for_math(math))
            .map_err(MathError::LimbScratch)?;
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
        let mut first = self.location_math(&xmin, &ymin, false, math, progress)?;
        let last = self.location_math(&xmax, &ymax, false, math, progress)?;
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
                    |_| Ok(value == &upper[axis]),
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
                        |values| {
                            let clipped_lower = [
                                values.original(values.extreme(&xmin, &lower[0], true)?)?,
                                values.original(values.extreme(&ymin, &lower[1], true)?)?,
                            ];
                            let clipped_upper = [
                                values.original(values.extreme(&xmax, &upper[0], false)?)?,
                                values.original(values.extreme(&ymax, &upper[1], false)?)?,
                            ];
                            let outside = values
                                .compare(&clipped_lower[0], &clipped_upper[0])?
                                .is_gt()
                                || values
                                    .compare(&clipped_lower[1], &clipped_upper[1])?
                                    .is_gt();
                            Ok((clipped_lower, clipped_upper, outside))
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
                        |values| {
                            for axis in 0..2 {
                                if !(values
                                    .compare(&clipped_lower[axis], &clipped_upper[axis])?
                                    .is_lt()
                                    || (values
                                        .compare(&clipped_lower[axis], &lower[axis])?
                                        .is_gt()
                                        && values
                                            .compare(&clipped_lower[axis], &upper[axis])?
                                            .is_lt()))
                                {
                                    return Ok(false);
                                }
                            }
                            Ok(true)
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
        let corners = self.corners(column, row).map_err(|cause| {
            let error = crate::ModelInputError::missing_node(self, cause, progress);
            GeoError::math_validation_failure(
                error,
                crate::OperationValidationContext::GridHole,
                progress,
            )
        })?;
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
