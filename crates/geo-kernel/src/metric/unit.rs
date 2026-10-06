// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exact dimensional conversion shares the original bounded rational arithmetic.

use crate::context::WorkProgress;
use crate::numerical::ExactAdmission;
use crate::{GeoError, Metres, MetricContext, MetricWorkContinuation, MetricWorkObserver, Rat};
use purrdf_xsd::integer::ExactOperation::{RationalCompare, RationalDivide, RationalMultiply};

/// The physical dimension selects the declared linear factor or its square.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricDimension {
    /// A linear distance, curve length or perimeter.
    Length,
    /// An areal quantity, including signed selected-branch area integrals.
    Area,
}

#[derive(Clone, Copy)]
enum Direction {
    FromMetres,
    ToMetres,
}

/// Convert an exact SI metric into an explicitly declared positive output unit.
/// Existing context work remains charged; conversion never resets the numerical
/// phase which produced the metric.
///
/// # Errors
/// Refuses nonpositive factors and insufficient exact arithmetic admission.
pub fn convert_unit(
    value: &Rat,
    metres_per_unit: &Rat,
    dimension: MetricDimension,
    context: &mut MetricContext,
) -> Result<Rat, GeoError> {
    let mut progress = WorkProgress::new(None);
    convert(
        value,
        metres_per_unit,
        dimension,
        Direction::FromMetres,
        context,
        &mut progress,
    )
}

/// Convert with bounded governor charging of this phase's additional work.
///
/// # Errors
/// Adds observer refusal to [`convert_unit`].
pub fn convert_unit_metered(
    value: &Rat,
    metres_per_unit: &Rat,
    dimension: MetricDimension,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<Rat, GeoError> {
    let mut continuation =
        MetricWorkContinuation::new(observer, context.work_items(), context.workspace_peak());
    let mut progress = WorkProgress::new(Some(&mut continuation));
    convert(
        value,
        metres_per_unit,
        dimension,
        Direction::FromMetres,
        context,
        &mut progress,
    )
}

/// Convert an original exact linear quantity into physical metres.
/// # Errors
/// Refuses undeclared/nonpositive factors or insufficient exact admission.
pub fn metres_from_unit(
    value: &Rat,
    metres_per_unit: &Rat,
    context: &mut MetricContext,
) -> Result<Metres, GeoError> {
    let mut progress = WorkProgress::new(None);
    to_metres(value, metres_per_unit, context, &mut progress)
}

/// Convert the same original quantity with bounded governor/cancellation polls.
/// # Errors
/// Preserves conversion and observer refusals without resetting context work.
pub fn metres_from_unit_metered(
    value: &Rat,
    metres_per_unit: &Rat,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<Metres, GeoError> {
    let mut continuation =
        MetricWorkContinuation::new(observer, context.work_items(), context.workspace_peak());
    let mut progress = WorkProgress::new(Some(&mut continuation));
    to_metres(value, metres_per_unit, context, &mut progress)
}

pub(super) fn to_metres(
    value: &Rat,
    metres_per_unit: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Metres, GeoError> {
    convert(
        value,
        metres_per_unit,
        MetricDimension::Length,
        Direction::ToMetres,
        context,
        progress,
    )
    .map(Metres::new)
}

fn convert(
    value: &Rat,
    metres_per_unit: &Rat,
    dimension: MetricDimension,
    direction: Direction,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<Rat, GeoError> {
    progress.context_poll(context)?;
    let mut admission = ExactAdmission::new(context, progress);
    let positive = admission.rational(RationalCompare, &[metres_per_unit], 1, || {
        Ok(metres_per_unit.signum() > 0)
    })?;
    if !positive {
        return Err(GeoError::config(
            "metric output units require a positive linear metre factor",
        ));
    }
    let squared;
    let factor = match dimension {
        MetricDimension::Length => metres_per_unit,
        MetricDimension::Area => {
            squared = admission.rational(RationalMultiply, &[metres_per_unit], 1, || {
                Ok(metres_per_unit.mul(metres_per_unit))
            })?;
            &squared
        }
    };
    match direction {
        Direction::FromMetres => admission.rational(RationalDivide, &[value, factor], 1, || {
            value
                .div(factor)
                .ok_or_else(|| GeoError::config("positive metric unit factor required"))
        }),
        Direction::ToMetres => admission.rational(RationalMultiply, &[value, factor], 1, || {
            Ok(value.mul(factor))
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionLimits, ExecutionPolicy, GeographicReference};

    #[test]
    fn dimensions_square_the_declared_factor_without_resetting_work() {
        let mut context = MetricContext::wgs84().unwrap();
        context.charge_work(7).unwrap();
        let length = convert_unit(
            &Rat::from_i64(1_000),
            &Rat::from_i64(1_000),
            MetricDimension::Length,
            &mut context,
        )
        .unwrap();
        assert_eq!(length, Rat::one());
        let first_work = context.work_items();
        let area = convert_unit(
            &Rat::from_i64(1_000_000),
            &Rat::from_i64(1_000),
            MetricDimension::Area,
            &mut context,
        )
        .unwrap();
        assert_eq!(area, Rat::one());
        assert!(context.work_items() > first_work && first_work > 7);
    }

    #[test]
    fn sequential_conversion_reports_each_new_work_and_peak_once() {
        struct Events(Vec<(u64, u64)>);
        impl MetricWorkObserver for Events {
            fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
                self.0.push((work, growth));
                Ok(())
            }
        }
        let mut context = MetricContext::wgs84().unwrap();
        context.set_preparation_work(17).unwrap();
        context.set_retained_workspace(256).unwrap();
        let mut events = Events(vec![(17, 256)]);
        for dimension in [MetricDimension::Length, MetricDimension::Area] {
            convert_unit_metered(
                &Rat::from_i64(1_000_000),
                &Rat::from_i64(1_000),
                dimension,
                &mut context,
                &mut events,
            )
            .unwrap();
            assert_eq!(
                events.0.iter().map(|event| event.0).sum::<u64>(),
                context.work_items()
            );
            assert_eq!(
                events.0.iter().map(|event| event.1).sum::<u64>(),
                context.workspace_peak()
            );
        }
    }

    #[test]
    fn conversion_admits_before_large_rational_arithmetic() {
        let mut context = MetricContext::new(
            GeographicReference::wgs84(),
            ExecutionPolicy::new(ExecutionLimits {
                max_work_items: 1_024,
                ..ExecutionLimits::GEOMETRY
            })
            .unwrap(),
        )
        .unwrap();
        let value = Rat::new(crate::Int::one().shl(32_768), crate::Int::one()).unwrap();
        assert!(matches!(
            convert_unit(&value, &Rat::one(), MetricDimension::Length, &mut context),
            Err(GeoError::WorkExhausted { .. })
        ));
        assert!(context.work_items() > 0 && context.work_items() < 1_024);
    }
}
