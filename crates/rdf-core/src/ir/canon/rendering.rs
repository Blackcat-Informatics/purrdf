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

/// The first-degree lines of one blank node: all their text in one buffer and
/// an offset pair per line. Cleared and reused from one blank node to the next;
/// both buffers are admitted through the caller's `Memory` and released with it.
#[derive(Default)]
pub(super) struct LineArena {
    pub(super) text: String,
    pub(super) spans: Vec<(usize, usize)>,
}

impl LineArena {
    pub(super) fn clear(&mut self) {
        self.text.clear();
        self.spans.clear();
    }

    /// Order the lines bytewise.
    pub(super) fn sort(&mut self) {
        let bytes = self.text.as_bytes();
        self.spans
            .sort_unstable_by(|&(a, b), &(c, d)| bytes[a..b].cmp(&bytes[c..d]));
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = &str> + Clone {
        self.spans
            .iter()
            .map(|&(start, end)| &self.text[start..end])
    }

    pub(super) fn release<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        memory.release_string(self.text)?;
        memory.release_vec(self.spans)
    }
}
