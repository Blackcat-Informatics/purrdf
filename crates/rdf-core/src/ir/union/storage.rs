// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original source reservations and the union's inclusive auxiliary ceiling.

use super::UnionError;
use crate::WorkspaceReservation;
use purrdf_lex::allocation::{Admission, StorageError};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) enum Grant<L, R> {
    Left(usize, L),
    Right(usize, R),
}
impl<L: WorkspaceReservation, R: WorkspaceReservation> WorkspaceReservation for Grant<L, R> {
    type Error = UnionError<L::Error, R::Error>;
    fn resize(&mut self, bytes: u64) -> Result<(), Self::Error> {
        match self {
            Self::Left(at, g) => g.resize(bytes).map_err(|e| UnionError::Left(*at, e)),
            Self::Right(at, g) => g.resize(bytes).map_err(|e| UnionError::Right(*at, e)),
        }
    }
}

pub(super) struct Account<L: WorkspaceReservation, R: WorkspaceReservation> {
    // This control storage and its remaining grants die before the bootstrap
    // grant which admitted their allocation.
    pub(super) grants: Vec<Grant<L, R>>,
    pub(super) bootstrap: Grant<L, R>,
    pub(super) live: usize,
    pub(super) limit: usize,
    pub(super) failure: Option<UnionError<L::Error, R::Error>>,
}
impl<L: WorkspaceReservation, R: WorkspaceReservation> Account<L, R> {
    pub(super) fn restore(&mut self, error: StorageError) -> UnionError<L::Error, R::Error> {
        self.failure.take().unwrap_or(UnionError::Storage(error))
    }
}
impl<L: WorkspaceReservation, R: WorkspaceReservation> Admission for Account<L, R> {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if self.failure.is_some() {
            return Err(StorageError::AdmissionFailed);
        }
        if bytes > self.limit {
            self.failure = Some(UnionError::Limit {
                dimension: "auxiliary bytes",
                actual: bytes,
                limit: self.limit,
            });
            return Err(StorageError::AdmissionFailed);
        }
        let result = (|| {
            let amount = u64::try_from(bytes)
                .map_err(|_| UnionError::Storage(StorageError::SizeOverflow))?;
            self.bootstrap.resize(amount)?;
            for grant in &mut self.grants {
                grant.resize(amount)?;
            }
            Ok(())
        })();
        if let Err(e) = result {
            self.failure = Some(e);
            return Err(StorageError::AdmissionFailed);
        }
        self.live = bytes;
        Ok(())
    }
}

pub(super) struct Reservation<'a, A: WorkspaceReservation, B: WorkspaceReservation> {
    pub(super) account: Account<A, B>,
    pub(super) live: &'a AtomicUsize,
    pub(super) limit: usize,
    pub(super) bytes: usize,
    pub(super) control: usize,
}
impl<A: WorkspaceReservation, B: WorkspaceReservation> WorkspaceReservation
    for Reservation<'_, A, B>
{
    type Error = UnionError<A::Error, B::Error>;
    fn resize(&mut self, payload: u64) -> Result<(), Self::Error> {
        let bytes = usize::try_from(payload)
            .ok()
            .and_then(|b| b.checked_add(self.control))
            .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
        let mut old = self.live.load(Ordering::Relaxed);
        loop {
            let next = old
                .checked_sub(self.bytes)
                .and_then(|n| n.checked_add(bytes))
                .ok_or(UnionError::Storage(StorageError::SizeOverflow))?;
            if next > self.limit {
                return Err(UnionError::Limit {
                    dimension: "auxiliary bytes",
                    actual: next,
                    limit: self.limit,
                });
            }
            match self
                .live
                .compare_exchange_weak(old, next, Ordering::AcqRel, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(value) => old = value,
            }
        }
        let previous = self.bytes;
        self.bytes = bytes;
        let result = self
            .account
            .resize(bytes)
            .map_err(|e| self.account.restore(e));
        if result.is_err() && previous > bytes {
            self.live.fetch_add(previous - bytes, Ordering::AcqRel);
            self.bytes = previous;
        }
        result
    }
}
impl<A: WorkspaceReservation, B: WorkspaceReservation> Drop for Reservation<'_, A, B> {
    fn drop(&mut self) {
        // Release guard-vector storage and original grants before refunding the
        // union charge. The bootstrap grant still lives while Vec deallocates.
        drop(std::mem::take(&mut self.account.grants));
        self.live.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
