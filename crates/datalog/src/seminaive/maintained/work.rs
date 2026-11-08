// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One transaction's cumulative allowance for retained native maintenance work.

use super::{EvalError, MaintenanceError, StopSignal, StorageError, is_stopped};
use core::cell::Cell;
use purrdf_lex::allocation::{Admission, Memory};

pub(super) struct Meter<'a, 's> {
    used: &'a Cell<u128>,
    preparation: u128,
    limit: u128,
    stop: Option<&'s dyn StopSignal>,
    needed: Cell<Option<u128>>,
    overflow: Cell<bool>,
    stopped: Cell<bool>,
}

impl<'a, 's> Meter<'a, 's> {
    pub(super) fn new(
        used: &'a Cell<u128>,
        preparation: u128,
        limit: u128,
        stop: Option<&'s dyn StopSignal>,
    ) -> Self {
        Self {
            used,
            preparation,
            limit,
            stop,
            needed: Cell::new(None),
            overflow: Cell::new(false),
            stopped: Cell::new(false),
        }
    }

    fn total(&self) -> u128 {
        self.preparation
            .checked_add(self.used.get())
            .expect("admitted transaction work is representable")
    }

    fn permits(&self, cost: u128) -> bool {
        let Some(next) = self.total().checked_add(cost) else {
            self.overflow.set(true);
            return false;
        };
        if next > self.limit {
            self.needed.set(Some(next));
            return false;
        }
        true
    }

    fn admit(&self, cost: u128) -> bool {
        if !self.permits(cost) {
            return false;
        }
        self.used.set(self.used.get() + cost);
        true
    }

    pub(super) fn allow(&self, cost: usize) -> bool {
        if is_stopped(self.stop) {
            self.stopped.set(true);
            return false;
        }
        self.admit(cost as u128)
    }

    pub(super) fn remaining_joins(&self) -> u64 {
        u64::try_from(self.limit.saturating_sub(self.total())).unwrap_or(u64::MAX)
    }

    pub(super) fn account_joins(&self, count: u64) {
        assert!(
            self.admit(u128::from(count)),
            "native joins stay within the original work grant"
        );
    }

    pub(super) fn refuse_join(&self) {
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
        if self.overflow.get() {
            Err(EvalError::Storage(StorageError::SizeOverflow))
        } else if self.stopped.get() {
            Err(EvalError::Stopped { report })
        } else {
            assert!(
                self.needed.get().is_some(),
                "a retained frame stopped for a stated reason"
            );
            Ok(false)
        }
    }

    pub(super) fn refused(&self) -> bool {
        self.needed.get().is_some() || self.overflow.get() || self.stopped.get()
    }

    pub(super) fn preparation_pause(&self) -> Result<super::PreparationPause, StorageError> {
        if self.overflow.get() {
            Err(StorageError::SizeOverflow)
        } else if self.stopped.get() {
            Ok(super::PreparationPause::Stopped)
        } else {
            Ok(super::PreparationPause::WorkLimit {
                limit: self.limit,
                needed: self
                    .needed
                    .get()
                    .expect("a retained preparation work refusal"),
            })
        }
    }

    pub(super) fn with_memory<A: Admission + ?Sized, T>(
        &self,
        memory: &mut Memory<'_, A>,
        body: impl FnOnce(&mut Memory<'_, FactoryAdmission<'_, '_, '_, '_, A>>) -> T,
    ) -> T {
        memory.with_account(|original, live| {
            let mut admission = FactoryAdmission::new(original, live, self);
            let mut memory = Memory::resume(&mut admission, live);
            let result = body(&mut memory);
            (result, memory.admitted_bytes())
        })
    }

    pub(super) fn refusal<E>(&self) -> MaintenanceError<E> {
        MaintenanceError::WorkLimit {
            limit: self.limit,
            consumed: self.total(),
            needed: self
                .needed
                .get()
                .expect("a work refusal retains its first unadmitted unit"),
        }
    }
}

/// Lend the same original grant while admitting actual replacement layout bytes
/// before its factory runs. Shrink/refund never needs additional work credit.
pub(super) struct FactoryAdmission<'a, 'm, 'u, 's, A: Admission + ?Sized> {
    original: &'a mut A,
    live: usize,
    meter: &'m Meter<'u, 's>,
}

impl<'a, 'm, 'u, 's, A: Admission + ?Sized> FactoryAdmission<'a, 'm, 'u, 's, A> {
    pub(super) fn new(original: &'a mut A, live: usize, meter: &'m Meter<'u, 's>) -> Self {
        Self {
            original,
            live,
            meter,
        }
    }
}

impl<A: Admission + ?Sized> Admission for FactoryAdmission<'_, '_, '_, '_, A> {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        let bytes = live.saturating_sub(self.live) as u128;
        if bytes != 0 {
            if is_stopped(self.meter.stop) {
                self.meter.stopped.set(true);
                return Err(StorageError::AdmissionFailed);
            }
            if !self.meter.permits(bytes) {
                return Err(StorageError::AdmissionFailed);
            }
        }
        self.original.resize(live)?;
        // The original admission may inspect caller state, including the stop
        // signal. This accepted synchronous reservation is one atomic factory;
        // consume its already checked credit without another stop callback.
        if bytes != 0 {
            self.meter.used.set(self.meter.used.get() + bytes);
        }
        self.live = live;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Memory;

    struct Grant {
        live: usize,
        maximum: usize,
        calls: usize,
    }

    impl Admission for Grant {
        fn resize(&mut self, live: usize) -> Result<(), StorageError> {
            self.calls += 1;
            if live > self.maximum {
                return Err(StorageError::AdmissionFailed);
            }
            self.live = live;
            Ok(())
        }
    }

    #[test]
    fn actual_replacement_factories_admit_work_before_the_original_grant() {
        let mut grant = Grant {
            live: 0,
            maximum: usize::MAX,
            calls: 0,
        };
        let used = Cell::new(0);
        let mut values = Vec::<u64>::new();
        let meter = Meter::new(&used, 0, 15, None);
        {
            let mut original = FactoryAdmission::new(&mut grant, 0, &meter);
            assert_eq!(
                Memory::new(&mut original).reserve(&mut values, 2),
                Err(StorageError::AdmissionFailed)
            );
        }
        assert_eq!(
            (grant.calls, grant.live, used.get(), values.capacity()),
            (0, 0, 0, 0)
        );
        assert_eq!(meter.needed.get(), Some(16));
        let meter = Meter::new(&used, 0, 16, None);
        {
            let mut original = FactoryAdmission::new(&mut grant, 0, &meter);
            Memory::new(&mut original).reserve(&mut values, 2).unwrap();
        }
        values.extend([7, 11]);
        assert_eq!((grant.calls, grant.live, used.get()), (1, 16, 16));
        let meter = Meter::new(&used, 0, 39, None);
        {
            let mut original = FactoryAdmission::new(&mut grant, 16, &meter);
            assert_eq!(
                Memory::resume(&mut original, 16).reserve(&mut values, 3),
                Err(StorageError::AdmissionFailed)
            );
        }
        assert_eq!(
            (grant.calls, grant.live, used.get(), values.capacity()),
            (1, 16, 16, 2)
        );
        assert_eq!(meter.needed.get(), Some(40));
        let meter = Meter::new(&used, 0, 40, None);
        {
            let mut original = FactoryAdmission::new(&mut grant, 16, &meter);
            let mut memory = Memory::resume(&mut original, 16);
            memory.reserve(&mut values, 3).unwrap();
            assert_eq!(memory.admitted_bytes(), 24);
            assert_eq!(values, [7, 11]);
            drop(values);
            memory.release_bytes(24).unwrap();
        }
        assert_eq!((grant.calls, grant.live, used.get()), (4, 0, 40));
    }

    #[test]
    fn refused_original_admission_consumes_no_factory_credit() {
        let mut grant = Grant {
            live: 0,
            maximum: 0,
            calls: 0,
        };
        let used = Cell::new(0);
        let meter = Meter::new(&used, 0, 16, None);
        let mut values = Vec::<u64>::new();
        {
            let mut original = FactoryAdmission::new(&mut grant, 0, &meter);
            assert_eq!(
                Memory::new(&mut original).reserve(&mut values, 2),
                Err(StorageError::AdmissionFailed)
            );
        }
        assert_eq!(
            (grant.calls, grant.live, used.get(), values.capacity()),
            (1, 0, 0, 0)
        );
        assert!(
            !meter.refused(),
            "the original resource refusal is not a work refusal"
        );
    }
}
