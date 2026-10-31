// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Scoped source admission for component spelling and ordered line buffers.

use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::CanonError;
use crate::WorkspaceReservation;

pub(super) struct ScratchGrant<R: WorkspaceReservation> {
    reservation: R,
    failure: Option<R::Error>,
}

impl<R: WorkspaceReservation> Admission for ScratchGrant<R> {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        super::ownership::resize_reservation(&mut self.reservation, &mut self.failure, bytes)
    }
}

impl<R: WorkspaceReservation> ScratchGrant<R> {
    pub(super) fn new(reservation: R) -> Self {
        Self {
            reservation,
            failure: None,
        }
    }

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

/// Preserve the final document's bytewise set ordering without admitting a
/// second line array. First-degree hashing does not call this deduplication.
pub(super) fn sort_unique<S: Admission + ?Sized>(
    lines: &mut Vec<String>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    lines.sort_unstable();
    let mut kept = 0;
    for read in 0..lines.len() {
        if kept > 0 && lines[read] == lines[kept - 1] {
            memory.release_string(std::mem::take(&mut lines[read]))?;
        } else {
            lines.swap(read, kept);
            kept += 1;
        }
    }
    lines.truncate(kept);
    Ok(())
}
