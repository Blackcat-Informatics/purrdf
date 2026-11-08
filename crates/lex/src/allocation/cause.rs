// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fallible shared ownership of original native diagnostic causes.

use super::{Memory, Resident, SharedOwned, StorageError};
use purrdf_hash::error::NativeError;

/// An independently admitted original native cause, shared without payload copies.
pub type NativeCause = SharedOwned<Box<dyn NativeError>>;

/// Retain an original native failure without copying or rendering its payload.
///
/// The erased payload and shared owner use the common fallible allocation path.
/// Copies retain this owner without copying its payload.
///
/// # Errors
/// Returns the original allocation refusal before publishing a partial owner.
pub fn retain_native_cause(cause: impl NativeError + 'static) -> Result<NativeCause, StorageError> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let cause: Box<dyn NativeError> = memory.boxed(cause)?;
    SharedOwned::try_from_admitted(cause, Resident).map_err(|_| StorageError::AllocationFailed)
}
