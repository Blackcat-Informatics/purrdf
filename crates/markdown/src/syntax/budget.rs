// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
use super::{Policy, SourcePosition, SyntaxError, SyntaxLimits, code_span::RunIndex};
use purrdf_lex::allocation::{Admission, Memory, StorageError};
pub(super) struct Budget<'a> {
    policy: Policy,
    pub source: &'a str,
    pub limits: SyntaxLimits,
    pub live: usize,
    pub work: u64,
    // This optional scratch can be dropped before admitting required storage.
    code_span_index: Option<(RunIndex, usize)>,
    at: usize,
    failure: Option<SyntaxError>,
}
impl<'a> Budget<'a> {
    pub(super) fn new(source: &'a str, limits: SyntaxLimits, policy: Policy) -> Self {
        Self {
            policy,
            source,
            limits,
            live: 0,
            work: 0,
            code_span_index: None,
            at: 0,
            failure: None,
        }
    }
    pub(super) fn position(&self, at: usize) -> SourcePosition {
        super::position(self.source, at, self.policy)
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
                position: self.position(at),
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
                position: self.position(at),
                cause: StorageError::SizeOverflow,
            })?;
        if next > self.limits.work {
            return Err(SyntaxError::Limit {
                position: self.position(at),
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
                position: self.position(self.at),
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

    pub(super) fn retain_code_span_index(&mut self, index: RunIndex, non_index_live: usize) {
        let index_bytes = self.live - non_index_live;
        self.live = non_index_live;
        self.code_span_index = Some((index, index_bytes));
    }

    pub(super) fn indexed_code_span_close(
        &mut self,
        at: usize,
        width: usize,
        position: &impl Fn(usize) -> usize,
    ) -> Option<Result<Option<usize>, SyntaxError>> {
        // Lookup only charges work. It does not resize the admission account.
        let mut scratch = self.code_span_index.take()?;
        let result = scratch.0.close(at, width, position, self);
        self.code_span_index = Some(scratch);
        Some(result)
    }

    pub(super) fn release_code_span_index(&mut self) {
        self.code_span_index = None;
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
                position: self.position(self.at),
                resource: "auxiliary bytes",
                actual: bytes as u64,
                limit: self.limits.auxiliary_bytes as u64,
            });
            return Err(StorageError::AdmissionFailed);
        }
        if self
            .code_span_index
            .as_ref()
            .is_some_and(|(_, index_bytes)| *index_bytes > self.limits.auxiliary_bytes - bytes)
        {
            self.release_code_span_index();
        }
        self.live = bytes;
        Ok(())
    }
}
