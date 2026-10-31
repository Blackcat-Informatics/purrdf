// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The source's original admission for immutable canonical output and refusals.

use purrdf_lex::allocation::{Admission, Memory, StorageError};

use super::CanonError;
use crate::{DatasetView, OwnedWorkspaceReservation};

pub(super) struct SourceGrant<E> {
    reservation: OwnedWorkspaceReservation<E>,
    live: usize,
    failure: Option<E>,
}

impl<E> Admission for SourceGrant<E> {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        let bytes = u64::try_from(bytes).map_err(|_| StorageError::SizeOverflow)?;
        if self.failure.is_some() {
            return Err(StorageError::AdmissionFailed);
        }
        self.reservation.resize(bytes).map_err(|cause| {
            self.failure = Some(cause);
            StorageError::AdmissionFailed
        })
    }
}

impl<E> SourceGrant<E> {
    pub(super) fn new<D: DatasetView<ReadError = E>>(view: &D) -> Result<Self, CanonError<E>> {
        let reservation = view
            .reserve_owned_workspace(0)
            .ok_or(CanonError::OwnedWorkspaceUnavailable)?
            .map_err(CanonError::OwnedAdmission)?;
        Ok(Self {
            reservation,
            live: 0,
            failure: None,
        })
    }

    pub(super) fn memory<T>(
        &mut self,
        operation: impl FnOnce(&mut Memory<'_, Self>) -> Result<T, StorageError>,
    ) -> Result<T, CanonError<E>> {
        let mut memory = Memory::resume(self, self.live);
        let result = operation(&mut memory);
        let live = memory.admitted_bytes();
        self.live = live;
        result.map_err(|error| match (error, self.failure.take()) {
            (StorageError::AdmissionFailed, Some(cause)) => CanonError::OwnedAdmission(cause),
            (error, _) => CanonError::Storage(error),
        })
    }
}
