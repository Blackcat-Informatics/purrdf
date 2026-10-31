// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Preserve native refusals and caller progress failures without type erasure.

use crate::allocation::{Admission, Memory, StorageError};

/// The caller's original work and stop authority at a native counting point.
pub trait Progress {
    /// Whether native progress counting points are enabled for this execution.
    /// Resident execution compiles out the callback and keeps its original scans.
    const ENABLED: bool = true;
    /// The unchanged operational failure owned by the caller.
    type Error;
    /// Admit the next concrete work step, or poll the stop authority with zero.
    ///
    /// # Errors
    /// Returns the original caller failure before the operation proceeds.
    fn charge(&mut self, units: u64) -> Result<(), Self::Error>;
}

impl<E, F: FnMut(u64) -> Result<(), E>> Progress for F {
    type Error = E;
    fn charge(&mut self, units: u64) -> Result<(), E> {
        self(units)
    }
}

/// Explicit resident execution without an external work or stop authority.
#[derive(Clone, Copy, Debug, Default)]
pub struct Unmetered;
impl Progress for Unmetered {
    const ENABLED: bool = false;
    type Error = core::convert::Infallible;
    fn charge(&mut self, _units: u64) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// A native operation failed, or its caller refused the next concrete work step.
///
/// The caller's failure is retained unchanged: a lexical error cannot replace a
/// cancellation, and an admission refusal cannot be reported as invalid syntax.
///
/// ```
/// use purrdf_lex::control::ControlledError;
/// let failure: ControlledError<&str, u64> = ControlledError::Control(7);
/// assert!(matches!(failure, ControlledError::Control(7)));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlledError<N, E> {
    /// The original native failure.
    Native(N),
    /// The original caller progress failure.
    Control(E),
}

impl<N, E> From<N> for ControlledError<N, E> {
    fn from(cause: N) -> Self {
        Self::Native(cause)
    }
}

impl<N: core::fmt::Display, E: core::fmt::Display> core::fmt::Display for ControlledError<N, E> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Native(cause) => cause.fmt(formatter),
            Self::Control(cause) => cause.fmt(formatter),
        }
    }
}

impl<N: std::error::Error + 'static, E: std::error::Error + 'static> std::error::Error
    for ControlledError<N, E>
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Native(cause) => Some(cause),
            Self::Control(cause) => Some(cause),
        }
    }
}

/// A borrowed native account restoring its original progress failure at the
/// enclosing [`with_memory_control`] boundary. The marker is never the public
/// operational cause. Storage updates continue through the original account.
pub struct ControlledAdmission<'a, S: ?Sized, C: Progress + ?Sized> {
    storage: &'a mut S,
    progress: &'a mut C,
    failure: Option<C::Error>,
}

impl<S: ?Sized, C: Progress + ?Sized> core::fmt::Debug for ControlledAdmission<'_, S, C> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ControlledAdmission")
            .finish_non_exhaustive()
    }
}

impl<S: Admission + ?Sized, C: Progress + ?Sized> Admission for ControlledAdmission<'_, S, C> {
    fn progress_enabled(&self) -> bool {
        C::ENABLED || self.storage.progress_enabled()
    }
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        self.storage.resize(bytes)
    }

    fn checkpoint(&mut self, units: u64) -> Result<(), StorageError> {
        if self.failure.is_some() {
            return Err(StorageError::AdmissionFailed);
        }
        if let Err(cause) = self.progress.charge(units) {
            self.failure = Some(cause);
            return Err(StorageError::AdmissionFailed);
        }
        self.storage.checkpoint(units)
    }
}

/// Run one native child under its original memory and typed progress authority.
/// No payload or reservation is copied or measured after construction. A native
/// refusal is returned unchanged; a callback refusal is restored before any
/// caller reporting or diagnostic formatting. Failed callbacks are never polled
/// again, and a native failure is never followed by a publication poll.
///
/// The caller retains the original admission through returned payloads, including
/// owned native diagnostic data. This borrowed boundary publishes no new owner.
///
/// # Errors
/// Preserves the first native failure or original caller failure. Success is
/// polled once before publication; refusal destroys the unpublished payload.
pub fn with_memory_control<
    S: Admission + ?Sized,
    C: Progress + ?Sized,
    T,
    N: From<StorageError>,
>(
    memory: &mut Memory<'_, S>,
    progress: &mut C,
    body: impl FnOnce(&mut Memory<'_, ControlledAdmission<'_, S, C>>) -> Result<T, N>,
) -> Result<T, ControlledError<N, C::Error>> {
    progress.charge(0).map_err(ControlledError::Control)?;
    memory.with_account(|storage, live| {
        let mut admission = ControlledAdmission {
            storage,
            progress,
            failure: None,
        };
        let mut child = Memory::resume(&mut admission, live);
        let result = body(&mut child);
        let live = child.admitted_bytes();
        drop(child);
        let result = if result.is_ok() {
            match admission.checkpoint(0) {
                Ok(()) => result,
                Err(cause) => {
                    drop(result);
                    Err(N::from(cause))
                }
            }
        } else {
            result
        };
        let result = match admission.failure {
            Some(cause) => {
                drop(result);
                Err(ControlledError::Control(cause))
            }
            None => result.map_err(ControlledError::Native),
        };
        (result, live)
    })
}

/// Compare original byte strings with bounded native comparison extents.
/// Resident comparison keeps the native slice fast path.
///
/// # Errors
/// Returns the caller's unchanged refusal before the next comparison extent.
pub fn compare_bytes<C: Progress + ?Sized>(
    left: &[u8],
    right: &[u8],
    progress: &mut C,
) -> Result<core::cmp::Ordering, C::Error> {
    if !C::ENABLED {
        return Ok(left.cmp(right));
    }
    let shared = left.len().min(right.len());
    let mut at = 0;
    while at < shared {
        let end = at.saturating_add(256).min(shared);
        progress.charge(u64::try_from(end - at).expect("bounded comparison extent"))?;
        let order = left[at..end].cmp(&right[at..end]);
        if !order.is_eq() {
            return Ok(order);
        }
        at = end;
    }
    progress.charge(1)?;
    Ok(left.len().cmp(&right.len()))
}
