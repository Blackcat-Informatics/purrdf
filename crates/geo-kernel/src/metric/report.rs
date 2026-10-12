// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Observed exact IEEE rounding after dimensional unit conversion.

use crate::context::WorkProgress;
use crate::{GeoError, MetricContext, MetricWorkContinuation, MetricWorkObserver, Rat};
use purrdf_xsd::integer::ExactArithmeticCost;

/// Convert a complete exact metric through the original IEEE half-even home.
/// Earlier context work and owned data remain live; no phase is reset.
/// # Errors
/// Refuses complete rounding admission or nonfinite converted results.
pub fn reported_double(value: &Rat, context: &mut MetricContext) -> Result<f64, GeoError> {
    reported_double_with_progress(value, context, &mut WorkProgress::integer(None))
}

/// Report the same result after charging only this phase's new work and peak.
/// An aggregate observer must already have charged the context's entering
/// work and workspace peak, which this phase does not charge again.
/// # Errors
/// Adds the original observer refusal to [`reported_double`].
///
/// The observer must already have observed `context.work_items()` and
/// `context.workspace_peak()`. This output continuation reports only added
/// work and growth, preserving the previously observed preparation prefix.
pub fn reported_double_metered(
    value: &Rat,
    context: &mut MetricContext,
    observer: &mut dyn MetricWorkObserver,
) -> Result<f64, GeoError> {
    let mut continuation =
        MetricWorkContinuation::new(observer, context.work_items(), context.workspace_peak());
    reported_double_with_progress(
        value,
        context,
        &mut WorkProgress::integer(Some(&mut continuation)),
    )
}

pub(crate) fn reported_double_with_progress(
    value: &Rat,
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
) -> Result<f64, GeoError> {
    let cost = ExactArithmeticCost::rational_binary64(
        value.numerator().bit_len(),
        value.denominator().bit_len(),
    )
    .ok_or(GeoError::ArithmeticOverflow(
        "reported metric rounding admission",
    ))?;
    progress.exact(context, cost, || {
        // Rat owns the established canonical-positive-zero postprocessing;
        // preserve it instead of reproducing the raw IEEE adapter's -0 rule.
        let reported = value.to_f64();
        if !reported.is_finite() {
            return Err(GeoError::ArithmeticOverflow(
                "finite converted geographic metric",
            ));
        }
        Ok(reported)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionLimits, ExecutionPolicy, GeographicReference, Int};

    #[test]
    fn rounding_ledger_preserves_ieee_ties_and_canonical_carrier_zero() {
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
        for value in [
            Rat::new(Int::one().shl(53).add(&Int::one()), Int::one().shl(53)).unwrap(),
            Rat::new(
                Int::one().shl(53).add(&Int::from_i64(3)),
                Int::one().shl(53),
            )
            .unwrap(),
            Rat::new(Int::from_i64(-1), Int::one().shl(1075)).unwrap(),
        ] {
            assert_eq!(
                reported_double_metered(&value, &mut context, &mut events)
                    .unwrap()
                    .to_bits(),
                value.to_f64().to_bits()
            );
            assert_eq!(
                events.0.iter().map(|event| event.0).sum::<u64>(),
                context.work_items()
            );
            assert_eq!(
                events.0.iter().map(|event| event.1).sum::<u64>(),
                context.workspace_peak()
            );
        }
        assert_eq!(
            context.remaining_workspace(),
            context.policy().limits().max_workspace_bytes - 256
        );
    }

    #[test]
    fn original_large_rounding_refuses_before_allocating_shifted_operands() {
        let value = Rat::new(
            Int::one().shl(32768).add(&Int::one()),
            Int::one().shl(32768),
        )
        .unwrap();
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 64,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let mut context = MetricContext::new(GeographicReference::wgs84(), policy).unwrap();
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let refusal = reported_double(&value, &mut context);
        let allocations = window.close();
        assert!(matches!(
            refusal,
            Err(GeoError::WorkExhausted { limit: 64 })
        ));
        assert_eq!(allocations.allocations, 0);
        assert_eq!(allocations.requested_bytes, 0);
        assert_eq!(context.work_items(), 0);
        assert_eq!(
            context.remaining_workspace(),
            policy.limits().max_workspace_bytes
        );
    }
}
