// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use super::{SyntaxError, SyntaxLimits, position};
use purrdf_lex::allocation::{Admission, Memory, StorageError};
pub(super) struct Budget<'a> {
    pub source: &'a str,
    pub limits: SyntaxLimits,
    pub live: usize,
    pub work: u64,
    at: usize,
    failure: Option<SyntaxError>,
}
impl<'a> Budget<'a> {
    pub(super) fn new(source: &'a str, limits: SyntaxLimits) -> Self {
        Self {
            source,
            limits,
            live: 0,
            work: 0,
            at: 0,
            failure: None,
        }
    }
    pub(super) fn limit(
        &mut self,
        at: usize,
        resource: &'static str,
        actual: usize,
        limit: usize,
    ) -> Result<(), SyntaxError> {
        self.at = at;
        if actual > limit {
            return Err(SyntaxError::Limit {
                position: position(self.source, at),
                resource,
                actual: actual as u64,
                limit: limit as u64,
            });
        }
        Ok(())
    }
    pub(super) fn tick(&mut self, at: usize, amount: usize) -> Result<(), SyntaxError> {
        self.at = at;
        let next = self
            .work
            .checked_add(amount as u64)
            .ok_or_else(|| SyntaxError::Storage {
                position: position(self.source, at),
                cause: StorageError::SizeOverflow,
            })?;
        if next > self.limits.work {
            return Err(SyntaxError::Limit {
                position: position(self.source, at),
                resource: "work",
                actual: next,
                limit: self.limits.work,
            });
        }
        self.work = next;
        Ok(())
    }
    pub(super) fn memory<T>(
        &mut self,
        body: impl FnOnce(&mut Memory<'_, Self>) -> Result<T, StorageError>,
    ) -> Result<T, SyntaxError> {
        let live = self.live;
        let result = body(&mut Memory::resume(self, live));
        result.map_err(|cause| {
            self.failure.take().unwrap_or_else(|| SyntaxError::Storage {
                position: position(self.source, self.at),
                cause,
            })
        })
    }
    pub(super) fn push<T>(&mut self, values: &mut Vec<T>, value: T) -> Result<(), SyntaxError> {
        self.memory(|m| m.push(values, value))
    }
    pub(super) fn release<T>(&mut self, values: Vec<T>) -> Result<(), SyntaxError> {
        self.memory(|m| m.release_vec(values))
    }
}
impl Admission for Budget<'_> {
    fn progress_enabled(&self) -> bool {
        true
    }
    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        if self.failure.is_some() {
            return Err(StorageError::AdmissionFailed);
        }
        let amount = usize::try_from(units).map_err(|_| StorageError::SizeOverflow)?;
        if let Err(error) = self.tick(self.at, amount) {
            self.failure = Some(error);
            return Err(StorageError::AdmissionFailed);
        }
        Ok(())
    }
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes > self.limits.auxiliary_bytes {
            self.failure = Some(SyntaxError::Limit {
                position: position(self.source, self.at),
                resource: "auxiliary bytes",
                actual: bytes as u64,
                limit: self.limits.auxiliary_bytes as u64,
            });
            return Err(StorageError::AdmissionFailed);
        }
        self.live = bytes;
        Ok(())
    }
}
