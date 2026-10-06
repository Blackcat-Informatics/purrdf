// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Actual invocation counters retained through refused native phases.

use std::cell::Cell;
use std::ops::{Deref, DerefMut};

use purrdf_geo_kernel::{
    ExecutionPolicy, GeoError, MetricContext, MetricWorkObserver, PreparationBudget,
};

#[derive(Clone, Copy, Default)]
struct Receipt {
    work: u64,
    workspace: u64,
}

pub(super) struct Invocation {
    policy: ExecutionPolicy,
    receipt: Cell<Receipt>,
}

impl Invocation {
    pub(super) const fn new(policy: ExecutionPolicy) -> Self {
        Self {
            policy,
            receipt: Cell::new(Receipt {
                work: 0,
                workspace: 0,
            }),
        }
    }

    fn capture(&self, work: u64, workspace: u64) {
        let previous = self.receipt.get();
        self.receipt.set(Receipt {
            work: previous.work.max(work),
            workspace: previous.workspace.max(workspace),
        });
    }

    pub(super) fn budget(&self, budget: PreparationBudget) {
        self.capture_policy(
            budget.policy(),
            budget.work_items(),
            budget.workspace_peak(),
        );
    }

    fn capture_policy(&self, policy: ExecutionPolicy, work: u64, workspace: u64) {
        let original = self.policy.limits();
        let local = policy.limits();
        self.capture(
            original
                .max_work_items
                .saturating_sub(local.max_work_items)
                .saturating_add(work),
            original
                .max_workspace_bytes
                .saturating_sub(local.max_workspace_bytes)
                .saturating_add(workspace),
        );
    }

    pub(super) fn track_budget(&self, budget: PreparationBudget) -> Budget<'_> {
        self.budget(budget);
        Budget {
            budget,
            invocation: self,
        }
    }

    pub(super) fn context(&self, context: MetricContext) -> Context<'_> {
        Context {
            context,
            invocation: self,
        }
    }

    pub(super) fn phase(&self, budget: PreparationBudget) -> Phase<'_> {
        self.budget(budget);
        Phase {
            invocation: self,
            prefix: budget,
            work: 0,
            workspace: 0,
        }
    }

    pub(super) fn admit_refusal(&self, error: &super::GeoCallError) -> Result<(), GeoError> {
        let receipt = self.receipt.get();
        let mut budget = PreparationBudget::new(self.policy);
        // A refused worker has dropped its temporaries, but keeping its whole
        // peak here conservatively covers original input and error owners that
        // remain live beside the response. No new policy allowance is opened.
        budget.retain(receipt.work, receipt.workspace)?;
        let layout = super::output::refusal(error)?;
        super::output::admit(budget, layout)
    }
}

pub(super) struct Budget<'a> {
    budget: PreparationBudget,
    invocation: &'a Invocation,
}
impl Deref for Budget<'_> {
    type Target = PreparationBudget;
    fn deref(&self) -> &PreparationBudget {
        &self.budget
    }
}
impl DerefMut for Budget<'_> {
    fn deref_mut(&mut self) -> &mut PreparationBudget {
        &mut self.budget
    }
}
impl Drop for Budget<'_> {
    fn drop(&mut self) {
        self.invocation.budget(self.budget);
    }
}

pub(super) struct Context<'a> {
    context: MetricContext,
    invocation: &'a Invocation,
}
impl Deref for Context<'_> {
    type Target = MetricContext;
    fn deref(&self) -> &MetricContext {
        &self.context
    }
}
impl DerefMut for Context<'_> {
    fn deref_mut(&mut self) -> &mut MetricContext {
        &mut self.context
    }
}
impl Drop for Context<'_> {
    fn drop(&mut self) {
        self.invocation.capture_policy(
            self.context.policy(),
            self.context.work_items(),
            self.context.workspace_peak(),
        );
    }
}

pub(super) struct Phase<'a> {
    invocation: &'a Invocation,
    prefix: PreparationBudget,
    pub(super) work: u64,
    pub(super) workspace: u64,
}
impl Phase<'_> {
    pub(super) fn retain_phase(
        &self,
        budget: &mut PreparationBudget,
        retained: u64,
    ) -> Result<(), GeoError> {
        let peak = budget
            .workspace_bytes()
            .checked_add(self.workspace)
            .ok_or(GeoError::ArithmeticOverflow("response phase workspace"))?;
        budget.retain_phase(self.work, retained, peak)?;
        self.invocation.budget(*budget);
        Ok(())
    }
}
impl MetricWorkObserver for Phase<'_> {
    fn charge_chunk(&mut self, work: u64, growth: u64) -> Result<(), GeoError> {
        self.work = self
            .work
            .checked_add(work)
            .ok_or(GeoError::ArithmeticOverflow("response numerical work"))?;
        self.workspace = self
            .workspace
            .checked_add(growth)
            .ok_or(GeoError::ArithmeticOverflow("response numerical workspace"))?;
        self.invocation.capture_policy(
            self.prefix.policy(),
            self.prefix.work_items().saturating_add(self.work),
            self.prefix.workspace_bytes().saturating_add(self.workspace),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_geo_kernel::{ExecutionLimits, Int, Rat};

    #[test]
    fn refused_output_uses_the_existing_phase_work_and_peak() {
        let error =
            super::super::GeoCallError::Engine(GeoError::NonPositiveEdgeLength(Rat::from_i64(-1)));
        let mut exact = PreparationBudget::new(ExecutionPolicy::geometry());
        super::super::output::reserve(&mut exact, super::super::output::refusal(&error).unwrap())
            .unwrap();
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: exact.work_items(),
            max_workspace_bytes: exact.workspace_peak(),
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        assert!(Invocation::new(policy).admit_refusal(&error).is_ok());
        let invocation = Invocation::new(policy);
        {
            let mut phase = invocation.phase(PreparationBudget::new(policy));
            phase.charge_chunk(1, 0).unwrap();
        }
        assert!(matches!(
            invocation.admit_refusal(&error),
            Err(GeoError::WorkExhausted { .. })
        ));
        let invocation = Invocation::new(policy);
        {
            let mut phase = invocation.phase(PreparationBudget::new(policy));
            phase.charge_chunk(0, 1).unwrap();
        }
        assert!(matches!(
            invocation.admit_refusal(&error),
            Err(GeoError::MemoryExhausted { .. })
        ));
    }

    #[test]
    fn remaining_child_context_reports_its_original_prefix_and_transient_peak() {
        let policy = ExecutionPolicy::geometry();
        let invocation = Invocation::new(policy);
        let mut prefix = PreparationBudget::new(policy);
        prefix.retain(41, 123).unwrap();
        let child_policy = prefix.remaining().unwrap();
        {
            let mut context = invocation.context(
                MetricContext::new(
                    purrdf_geo_kernel::binding::standard_reference().clone(),
                    child_policy,
                )
                .unwrap(),
            );
            context.charge_work(29).unwrap();
            context.admit_workspace(257).unwrap();
            context.release_workspace(257).unwrap();
        }
        let receipt = invocation.receipt.get();
        assert_eq!(receipt.work, 70);
        assert_eq!(receipt.workspace, 380);
        // A later successful budget update cannot erase a refused phase's peak.
        invocation.budget(prefix);
        assert_eq!(invocation.receipt.get().workspace, 380);
    }

    #[test]
    fn unadmitted_large_error_operands_do_not_allocate_to_render() {
        let error = super::super::GeoCallError::Engine(GeoError::NonPositiveEdgeLength(
            Rat::from_int(Int::one().shl(65_536).neg()),
        ));
        let policy = ExecutionPolicy::new(ExecutionLimits {
            max_work_items: 64,
            max_workspace_bytes: 2_048,
            ..ExecutionLimits::GEOMETRY
        })
        .unwrap();
        let invocation = Invocation::new(policy);
        let window = purrdf_alloc_probe::CurrentThreadWindow::open();
        let result = invocation.admit_refusal(&error);
        let stats = window.close();
        assert!(matches!(
            result,
            Err(GeoError::WorkExhausted { .. } | GeoError::MemoryExhausted { .. })
        ));
        assert_eq!(stats.allocations, 0);
        assert_eq!(stats.requested_bytes, 0);
    }
}
