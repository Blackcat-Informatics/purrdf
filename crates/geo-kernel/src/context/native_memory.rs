// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The original policy and physical account around native geometry buffers.

use super::{MetricContext, WorkProgress};
use crate::GeoError;
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use std::cell::RefCell;

struct PolicyAdmission<'a, 'context> {
    native: &'a mut dyn Admission,
    context: &'a RefCell<&'context mut MetricContext>,
    admitted: usize,
    failure: Option<GeoError>,
}

impl Admission for PolicyAdmission<'_, '_> {
    fn resize(&mut self, next: usize) -> Result<(), StorageError> {
        let growth = next.saturating_sub(self.admitted);
        if growth != 0 {
            let bytes = u64::try_from(growth).map_err(|_| StorageError::SizeOverflow)?;
            if let Err(original) = self.context.borrow_mut().admit_workspace(bytes) {
                self.failure.get_or_insert(original);
                return Err(StorageError::AdmissionFailed);
            }
        }
        if let Err(original) = self.native.resize(next) {
            if growth != 0 {
                let _ = self.context.borrow_mut().release_workspace(growth as u64);
            }
            return Err(original);
        }
        if next < self.admitted {
            let bytes =
                u64::try_from(self.admitted - next).map_err(|_| StorageError::SizeOverflow)?;
            if let Err(original) = self.context.borrow_mut().release_workspace(bytes) {
                self.failure.get_or_insert(original);
                return Err(StorageError::AdmissionFailed);
            }
        }
        self.admitted = next;
        Ok(())
    }
}

/// Share the original physical buffer body with a metric policy. Short context
/// borrows permit exact callbacks between growth operations; no second account
/// or buffer implementation is created. Native Memory records each allocation
/// before birth and each refund after destruction.
pub(crate) fn with_policy_memory<T>(
    context: &RefCell<&mut MetricContext>,
    outer: &mut Memory<'_, dyn Admission + '_>,
    body: impl FnOnce(&mut Memory<'_, dyn Admission + '_>) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    outer.with_account(|native, admitted| {
        let mut pricing = PolicyAdmission {
            native,
            context,
            admitted,
            failure: None,
        };
        let mut memory = Memory::resume(&mut pricing as &mut dyn Admission, admitted);
        let result = body(&mut memory);
        let admitted = memory.admitted_bytes();
        let result = match pricing.failure {
            Some(original) => {
                drop(result);
                Err(original)
            }
            None => result,
        };
        (result, admitted)
    })
}

/// Continue an already owned native buffer account under the same policy.
pub(crate) fn native_memory<T>(
    context: &mut MetricContext,
    progress: &mut WorkProgress<'_>,
    resident: &mut usize,
    body: impl FnOnce(&mut Memory<'_, dyn Admission + '_>) -> Result<T, GeoError>,
) -> Result<T, GeoError> {
    let (result, admitted) = progress
        .with_storage(*resident, |outer| {
            let result = with_policy_memory(&RefCell::new(&mut *context), outer, body);
            Ok::<_, core::convert::Infallible>((result, outer.admitted_bytes()))
        })
        .unwrap_or_else(|never| match never {});
    *resident = admitted;
    match result {
        Ok(value) => progress.context_poll(context).map(|()| value),
        Err(original) => Err(original),
    }
}
