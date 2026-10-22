// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One transaction's cumulative allowance for retained native maintenance work.

use super::{EvalError, MaintenanceError, StopSignal, StorageError, is_stopped};

pub(super) struct Meter<'a, 's> {
    used: &'a mut u128,
    preparation: u128,
    limit: u128,
    stop: Option<&'s dyn StopSignal>,
    needed: Option<u128>,
    overflow: bool,
    stopped: bool,
}

impl<'a, 's> Meter<'a, 's> {
    pub(super) fn new(
        used: &'a mut u128,
        preparation: u128,
        limit: u128,
        stop: Option<&'s dyn StopSignal>,
    ) -> Self {
        Self {
            used,
            preparation,
            limit,
            stop,
            needed: None,
            overflow: false,
            stopped: false,
        }
    }

    fn total(&self) -> u128 {
        self.preparation
            .checked_add(*self.used)
            .expect("admitted transaction work is representable")
    }

    fn admit(&mut self, cost: u128) -> bool {
        let Some(next) = self.total().checked_add(cost) else {
            self.overflow = true;
            return false;
        };
        if next > self.limit {
            self.needed = Some(next);
            return false;
        }
        *self.used += cost;
        true
    }

    pub(super) fn allow(&mut self, cost: usize) -> bool {
        if is_stopped(self.stop) {
            self.stopped = true;
            return false;
        }
        self.admit(cost as u128)
    }

    pub(super) fn remaining_joins(&self) -> u64 {
        u64::try_from(self.limit.saturating_sub(self.total())).unwrap_or(u64::MAX)
    }

    pub(super) fn account_joins(&mut self, count: u64) {
        assert!(
            self.admit(u128::from(count)),
            "native joins stay within the original work grant"
        );
    }

    pub(super) fn refuse_join(&mut self) {
        assert!(
            !self.admit(1),
            "a work-limited native frame needs another unit"
        );
    }

    pub(super) fn stop(&self) -> Option<&'s dyn StopSignal> {
        self.stop
    }

    /// False is an owned work refusal; stops and overflow retain their native error.
    pub(super) fn paused<E>(
        &self,
        report: super::super::BudgetReport,
    ) -> Result<bool, EvalError<E>> {
        if self.overflow {
            Err(EvalError::Storage(StorageError::SizeOverflow))
        } else if self.stopped {
            Err(EvalError::Stopped { report })
        } else {
            assert!(
                self.needed.is_some(),
                "a retained frame stopped for a stated reason"
            );
            Ok(false)
        }
    }

    pub(super) fn refusal<E>(&self) -> MaintenanceError<E> {
        MaintenanceError::WorkLimit {
            limit: self.limit,
            consumed: self.total(),
            needed: self
                .needed
                .expect("a work refusal retains its first unadmitted unit"),
        }
    }
}
