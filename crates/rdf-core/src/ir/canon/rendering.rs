// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Scoped source admission for component spelling and ordered line buffers.

use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::CanonError;
use crate::WorkspaceReservation;

type ReservationFactory<R> = fn() -> Result<R, <R as WorkspaceReservation>::Error>;

pub(super) struct ScratchGrant<R: WorkspaceReservation, F = ReservationFactory<R>> {
    reservation: Option<R>,
    factory: Option<F>,
    failure: Option<R::Error>,
}

impl<R, F> Admission for ScratchGrant<R, F>
where
    R: WorkspaceReservation,
    F: FnOnce() -> Result<R, R::Error>,
{
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if self.failure.is_some() {
            return Err(StorageError::AdmissionFailed);
        }
        if self.reservation.is_none() {
            if bytes == 0 {
                return Ok(());
            }
            let factory = self
                .factory
                .take()
                .expect("an uninitialized scratch grant retains its factory");
            self.reservation = Some(factory().map_err(|cause| {
                self.failure = Some(cause);
                StorageError::AdmissionFailed
            })?);
        }
        super::ownership::resize_reservation(
            self.reservation
                .as_mut()
                .expect("positive scratch growth initialized its reservation"),
            &mut self.failure,
            bytes,
        )
    }
}

impl<R: WorkspaceReservation> ScratchGrant<R> {
    pub(super) fn new(reservation: R) -> Self {
        Self {
            reservation: Some(reservation),
            factory: None,
            failure: None,
        }
    }

    /// Empty scratch does not acquire a provider reservation. The first positive
    /// layout requires the original capability and preserves its typed refusal.
    pub(super) fn deferred<F>(factory: F) -> ScratchGrant<R, F>
    where
        F: FnOnce() -> Result<R, R::Error>,
    {
        ScratchGrant {
            reservation: None,
            factory: Some(factory),
            failure: None,
        }
    }
}

impl<R, F> ScratchGrant<R, F>
where
    R: WorkspaceReservation,
    F: FnOnce() -> Result<R, R::Error>,
{
    /// The body destroys all scratch before returning; only independently owned
    /// results may leave. Failed locals die before `try_scope` refunds them, and
    /// the concrete source guard outlives both the body and its memory.
    pub(super) fn run<T>(
        mut self,
        operation: impl FnOnce(&mut Memory<'_, Self>) -> Result<T, CanonError<R::Error>>,
    ) -> Result<T, CanonError<R::Error>> {
        let result = Memory::new(&mut self).try_scope(operation);
        result.map_err(|error| self.restore(error))
    }

    /// Restore a callback's original provider refusal before crossing another
    /// scratch scope whose cleanup may have its own refusal.
    pub(super) fn restore(&mut self, error: CanonError<R::Error>) -> CanonError<R::Error> {
        match (error, self.failure.take()) {
            (CanonError::Storage(StorageError::AdmissionFailed), Some(cause)) => {
                CanonError::Read(cause)
            }
            (error, _) => error,
        }
    }
}

pub(super) fn release_lines<S: Admission + ?Sized>(
    mut lines: Vec<String>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    for line in &mut lines {
        memory.release_string(std::mem::take(line))?;
    }
    memory.release_vec(lines)
}
