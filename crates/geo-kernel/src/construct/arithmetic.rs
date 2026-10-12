// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The original constructor arithmetic with optional native capabilities.

use crate::context::WorkProgress;
use crate::numerical::{ExactAdmission, ExactValues};
use crate::{Coord, GeoError, Int, MetricContext, Rat};
use core::{cmp::Ordering, ops::Deref};
use purrdf_xsd::integer::{ExactArithmeticCost, ExactOperation, LimbScratchError};
use std::cell::Cell;
use std::cell::RefCell;

pub(crate) struct Arithmetic<'a, 'context, 'observer> {
    context: Option<&'a RefCell<&'context mut MetricContext>>,
    progress: Option<&'a RefCell<WorkProgress<'observer>>>,
    output: Cell<usize>,
}

impl<'a, 'context, 'observer> Arithmetic<'a, 'context, 'observer> {
    pub(crate) const fn resident() -> Self {
        Self {
            context: None,
            progress: None,
            output: Cell::new(0),
        }
    }

    pub(crate) const fn admitted(
        context: &'a RefCell<&'context mut MetricContext>,
        progress: &'a RefCell<WorkProgress<'observer>>,
    ) -> Self {
        Self {
            context: Some(context),
            progress: Some(progress),
            output: Cell::new(0),
        }
    }

    pub(crate) fn output(&self, additional: usize) -> Result<(), GeoError> {
        let next = self
            .output
            .get()
            .checked_add(additional)
            .ok_or(GeoError::ArithmeticOverflow("constructed coordinate count"))?;
        if let Some(context) = self.context {
            let limit = context.borrow().policy().limits().max_output_elements;
            if next as u64 > limit {
                return Err(GeoError::OutputExhausted { limit });
            }
        }
        self.output.set(next);
        Ok(())
    }

    pub(crate) fn visit(&self) -> Result<(), GeoError> {
        if let (Some(context), Some(progress)) = (self.context, self.progress) {
            let mut context = context.borrow_mut();
            context.charge_work(1)?;
            progress.borrow_mut().context_poll(&context)?;
        }
        Ok(())
    }

    pub(crate) fn exact<T>(
        &self,
        operation: ExactOperation,
        operands: &[&Rat],
        count: u64,
        evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, LimbScratchError>,
    ) -> Result<T, GeoError> {
        match (self.context, self.progress) {
            (Some(context), Some(progress)) => {
                let mut context = context.borrow_mut();
                let mut progress = progress.borrow_mut();
                ExactAdmission::new(&mut context, &mut progress)
                    .expression(operation, operands, count, evaluate)
            }
            _ => evaluate(&ExactValues::resident()).map_err(GeoError::NumericalScratch),
        }
    }

    pub(crate) fn cost<T>(
        &self,
        cost: ExactArithmeticCost,
        evaluate: impl FnOnce(&ExactValues<'_>) -> Result<T, LimbScratchError>,
    ) -> Result<T, GeoError> {
        match (self.context, self.progress) {
            (Some(context), Some(progress)) => {
                let mut context = context.borrow_mut();
                let mut progress = progress.borrow_mut();
                ExactAdmission::new(&mut context, &mut progress).expression_cost(cost, |values| {
                    evaluate(values).map_err(GeoError::NumericalScratch)
                })
            }
            _ => evaluate(&ExactValues::resident()).map_err(GeoError::NumericalScratch),
        }
    }

    pub(crate) fn compare(&self, left: &Rat, right: &Rat) -> Result<Ordering, GeoError> {
        self.exact(
            ExactOperation::RationalCompare,
            &[left, right],
            1,
            |values| values.compare(left, right),
        )
    }

    pub(crate) fn compare_coord(&self, left: &Coord, right: &Coord) -> Result<Ordering, GeoError> {
        self.exact(
            ExactOperation::RationalCompare,
            &[left.x(), left.y(), right.x(), right.y()],
            2,
            |values| values.compare_coordinate(left, right),
        )
    }

    pub(crate) fn plane(&self, coord: &Coord) -> Result<Coord, GeoError> {
        self.exact(
            ExactOperation::Linear,
            &[coord.x(), coord.y()],
            2,
            |values| values.planar_coordinate(coord),
        )
    }

    pub(crate) fn coordinate(&self, coord: &Coord) -> Result<Coord, GeoError> {
        self.output(1)?;
        let mut ordinates = [coord.x(), coord.y(), coord.x(), coord.x()];
        let mut length = 2;
        if let Some(z) = coord.z() {
            ordinates[length] = z;
            length += 1;
        }
        if let Some(m) = coord.m() {
            ordinates[length] = m;
            length += 1;
        }
        self.exact(
            ExactOperation::Linear,
            &ordinates[..length],
            length as u64,
            |values| values.coordinate(coord),
        )
    }

    pub(crate) fn original(&self, value: &Rat) -> Result<Rat, GeoError> {
        self.exact(ExactOperation::Linear, &[value], 1, |values| {
            values.original(value)
        })
    }

    pub(crate) fn add(&self, left: &Rat, right: &Rat) -> Result<Rat, GeoError> {
        self.exact(ExactOperation::RationalAdd, &[left, right], 1, |values| {
            values.add(left, right)
        })
    }

    pub(crate) fn subtract(&self, left: &Rat, right: &Rat) -> Result<Rat, GeoError> {
        self.exact(ExactOperation::RationalAdd, &[left, right], 1, |values| {
            values.subtract(left, right)
        })
    }

    pub(crate) fn multiply(&self, left: &Rat, right: &Rat) -> Result<Rat, GeoError> {
        self.exact(
            ExactOperation::RationalMultiply,
            &[left, right],
            1,
            |values| values.multiply(left, right),
        )
    }

    pub(crate) fn divide(&self, left: &Rat, right: &Rat) -> Result<Rat, GeoError> {
        self.exact(
            ExactOperation::RationalDivide,
            &[left, right],
            1,
            |values| {
                Ok(values
                    .divide(left, right)?
                    .expect("the original constructor proves a nonzero divisor"))
            },
        )
    }

    pub(crate) fn orientation(&self, a: &Coord, b: &Coord, c: &Coord) -> Result<i32, GeoError> {
        match (self.context, self.progress) {
            (Some(context), Some(progress)) => {
                let mut context = context.borrow_mut();
                let mut progress = progress.borrow_mut();
                crate::topology::orientation_admitted(
                    a,
                    b,
                    c,
                    &mut ExactAdmission::new(&mut context, &mut progress),
                )
            }
            _ => Ok(crate::topology::orientation(a, b, c)),
        }
    }

    pub(crate) fn geometry(
        &self,
        dim: crate::CoordDim,
        body: crate::GeometryBody,
    ) -> Result<crate::Geometry, GeoError> {
        match self.progress {
            Some(progress) => {
                crate::Geometry::new_with_progress(dim, body, &mut progress.borrow_mut())
            }
            None => crate::Geometry::new(dim, body),
        }
    }

    pub(crate) fn ring_area(&self, ring: &[Coord]) -> Result<ScopedArea<'a, 'context>, GeoError> {
        match (self.context, self.progress) {
            (Some(context), Some(progress)) => {
                let mut bytes = 0;
                let mut context_borrow = context.borrow_mut();
                let mut progress = progress.borrow_mut();
                let area = crate::measure::signed_ring_area_retained(
                    ring,
                    &mut ExactAdmission::new(&mut context_borrow, &mut progress),
                    &mut bytes,
                )?;
                Ok(ScopedArea {
                    area: Some(area),
                    context: Some(context),
                    bytes,
                })
            }
            _ => Ok(ScopedArea {
                area: Some(crate::measure::signed_ring_area(ring)),
                context: None,
                bytes: 0,
            }),
        }
    }

    pub(crate) fn integer_add(&self, left: &Int, right: &Int) -> Result<Int, GeoError> {
        let bits = left.bit_len().max(right.bit_len());
        let cost = ExactArithmeticCost::for_operation(ExactOperation::Linear, bits, 1)
            .ok_or(GeoError::ArithmeticOverflow("constructor integer sum"))?;
        self.cost(cost, |values| values.integer_add(left, right))
    }
}

/// The exact ring result dies before its original live-result allowance is
/// refunded. Its pooled limbs independently retain their physical arena owner.
pub(crate) struct ScopedArea<'a, 'context> {
    area: Option<Rat>,
    context: Option<&'a RefCell<&'context mut MetricContext>>,
    bytes: u64,
}

impl Deref for ScopedArea<'_, '_> {
    type Target = Rat;
    fn deref(&self) -> &Rat {
        self.area.as_ref().expect("live original ring area")
    }
}

impl Drop for ScopedArea<'_, '_> {
    fn drop(&mut self) {
        drop(self.area.take());
        if let Some(context) = self.context {
            context
                .borrow_mut()
                .release_workspace(self.bytes)
                .expect("original ring result allowance remains in its context");
        }
    }
}
