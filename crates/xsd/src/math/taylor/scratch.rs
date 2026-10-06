// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit reusable coefficient storage. A checkout holds no mutex guard.

use core::fmt;
use std::sync::{Arc, Mutex};

use super::super::{FixedInterval, MathError};

/// A reusable Taylor pool cannot be checked out simultaneously.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaylorScratchError {
    /// Another branded workspace already owns this pool's buffers.
    InUse,
}
impl fmt::Display for TaylorScratchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Taylor scratch already checked out")
    }
}
impl std::error::Error for TaylorScratchError {}

pub(super) struct ScratchBuffers {
    pub(super) coefficients: Box<[FixedInterval]>,
    pub(super) generations: Box<[u64]>,
}

struct ScratchInner {
    coefficients: usize,
    jets: usize,
    buffers: Mutex<Option<ScratchBuffers>>,
}

/// Immutable-capacity worker pool for branded Taylor coefficient workspaces.
/// Its buffers are returned and cleared on success, refusal and unwinding.
#[derive(Clone)]
pub struct TaylorScratch(Arc<ScratchInner>);

impl fmt::Debug for TaylorScratch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TaylorScratch")
            .field("coefficients", &self.0.coefficients)
            .field("jets", &self.0.jets)
            .finish_non_exhaustive()
    }
}

impl TaylorScratch {
    pub(super) fn new(
        coefficients: usize,
        jets: usize,
        zero: FixedInterval,
    ) -> Result<Self, MathError> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(coefficients)
            .map_err(|_| MathError::WorkspaceExhausted)?;
        values.resize(coefficients, zero);
        let mut generations = Vec::new();
        generations
            .try_reserve_exact(jets)
            .map_err(|_| MathError::WorkspaceExhausted)?;
        generations.resize(jets, 0);
        Ok(Self(Arc::new(ScratchInner {
            coefficients,
            jets,
            buffers: Mutex::new(Some(ScratchBuffers {
                coefficients: values.into_boxed_slice(),
                generations: generations.into_boxed_slice(),
            })),
        })))
    }

    pub(super) fn supports(&self, coefficients: usize, jets: usize) -> bool {
        self.0.coefficients >= coefficients && self.0.jets >= jets
    }

    /// Actual reusable heap, including buffer and shared ownership headers.
    #[must_use]
    pub fn retained_bytes(&self) -> usize {
        Self::required_bytes(self.0.coefficients, self.0.jets).expect("constructed Taylor capacity")
    }

    /// Whether two worker handles borrow the same retained allocation.
    #[must_use]
    pub fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }

    pub(super) fn required_bytes(coefficients: usize, jets: usize) -> Option<usize> {
        coefficients
            .checked_mul(size_of::<FixedInterval>())?
            .checked_add(jets.checked_mul(size_of::<u64>())?)?
            .checked_add(size_of::<ScratchInner>() + 2 * size_of::<usize>())
    }

    pub(super) fn checkout(&self) -> Result<ScratchBuffers, MathError> {
        let mut buffers = self
            .0
            .buffers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = buffers
            .take()
            .ok_or(MathError::TaylorScratch(TaylorScratchError::InUse));
        drop(buffers);
        result
    }

    pub(super) fn return_buffers(&self, storage: ScratchBuffers) {
        let previous = self
            .0
            .buffers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .replace(storage);
        debug_assert!(previous.is_none(), "one exclusive Taylor checkout");
    }

    pub(super) fn available(&self) -> Result<(), MathError> {
        let buffers = self
            .0
            .buffers
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = if buffers.is_some() {
            Ok(())
        } else {
            Err(MathError::TaylorScratch(TaylorScratchError::InUse))
        };
        drop(buffers);
        result
    }
}
