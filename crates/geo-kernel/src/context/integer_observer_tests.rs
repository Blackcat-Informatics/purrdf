// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use super::WorkProgress;
use crate::{GeoError, MetricContext, MetricWorkObserver};
#[cfg(target_arch = "aarch64")]
use purrdf_xsd::ieee::control::{Fpcr as FloatingControl, fpcr};
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
use purrdf_xsd::ieee::control::{Mxcsr as FloatingControl, mxcsr_available};
use purrdf_xsd::math::{CoordinateMath, MathError, MathLimits};

fn control_available() -> bool {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        mxcsr_available()
    }
    #[cfg(target_arch = "aarch64")]
    {
        true
    }
}

fn flush_to_zero() -> FloatingControl {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        FloatingControl::flush_to_zero()
    }
    #[cfg(target_arch = "aarch64")]
    {
        FloatingControl::load(fpcr() | (1 << 24))
    }
}

#[derive(Default)]
struct ChangedEnvironment {
    guard: Option<FloatingControl>,
    work: u64,
    workspace: u64,
}
impl MetricWorkObserver for ChangedEnvironment {
    fn charge_chunk(&mut self, work: u64, workspace: u64) -> Result<(), GeoError> {
        self.work += work;
        self.workspace += workspace;
        self.guard.get_or_insert_with(flush_to_zero);
        Ok(())
    }
}

#[test]
fn integer_callbacks_ignore_fenv_without_losing_cancellation_or_receipts() {
    if !control_available() {
        return;
    }
    let mut context = MetricContext::wgs84().unwrap();
    let mut observer = ChangedEnvironment::default();
    {
        let mut progress = WorkProgress::integer(Some(&mut observer));
        progress.initial().unwrap();
        context.begin_integer(1).unwrap();
        assert!(matches!(
            context.begin(1),
            Err(GeoError::FloatEnvironment(_))
        ));
        progress.charge_counts(7, 32).unwrap();
        progress.context_poll(&context).unwrap();
        context.cancel();
        assert_eq!(progress.context_poll(&context), Err(GeoError::Cancelled));
    }
    assert_eq!((observer.work, observer.workspace), (7, 32));
    // Dropping the test observer restores its entering control state.
    drop(observer);
    purrdf_xsd::ieee::environment::check().unwrap();
}

#[test]
fn numerical_chunks_still_revalidate_after_integer_observer_phases() {
    if !control_available() {
        return;
    }
    let context = MetricContext::wgs84().unwrap();
    let math = CoordinateMath::new(MathLimits {
        precision_bits: 96,
        max_work: 1024,
        max_workspace_bytes: 64 * 1024,
    })
    .unwrap();
    let mut observer = ChangedEnvironment::default();
    {
        let mut progress = WorkProgress::integer(Some(&mut observer));
        progress.initial().unwrap();
        progress.math_phase(&context);
        assert_eq!(progress.math_poll(&math), Err(MathError::Cancelled));
        assert!(matches!(
            progress.latched_error(),
            Some(GeoError::FloatEnvironment(_))
        ));
    }
    drop(observer);
    purrdf_xsd::ieee::environment::check().unwrap();
    let mut observer = ChangedEnvironment::default();
    assert!(matches!(
        WorkProgress::new(Some(&mut observer)).initial(),
        Err(GeoError::FloatEnvironment(_))
    ));
}

#[test]
fn numerical_output_refuses_changed_fenv_before_allocating_or_inspecting_values() {
    if !control_available() {
        return;
    }
    let mut context = MetricContext::wgs84().unwrap();
    let mut observer = ChangedEnvironment::default();
    let mut entered = false;
    let cost = purrdf_xsd::integer::ExactArithmeticCost::for_operation(
        purrdf_xsd::integer::ExactOperation::Linear,
        64,
        1,
    )
    .unwrap();
    let window = purrdf_alloc_probe::CurrentThreadWindow::open();
    let refusal = context.produce_output_metered(
        cost,
        37,
        || {
            entered = true;
            Ok(Vec::<u8>::with_capacity(37))
        },
        &mut observer,
    );
    let allocations = window.close();
    assert!(matches!(refusal, Err(GeoError::FloatEnvironment(_))));
    assert!(!entered);
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.requested_bytes, 0);
    assert_eq!(
        context.remaining_workspace(),
        context.policy().limits().max_workspace_bytes
    );
    drop(observer);
    context.checkpoint().unwrap();
}
