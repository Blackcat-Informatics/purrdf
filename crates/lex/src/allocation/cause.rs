// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fallible shared ownership of original native diagnostic causes.

use super::{Memory, Resident, SharedOwned, StorageError};
use purrdf_hash::error::NativeError;
use std::error::Error;
use std::sync::OnceLock;

/// An independently admitted original native cause, shared without payload copies.
pub type NativeCause = SharedOwned<Box<dyn NativeError>>;

/// An admitted original standard error with shared lazy presentation.
///
/// The error retains its concrete type for standard error-chain downcasting.
/// This owner claims no structured inspection protocol for the original error.
#[derive(Clone)]
pub struct StandardCause {
    owner: SharedOwned<StandardPayload>,
}

struct StandardPayload {
    error: Box<dyn Error + Send + Sync>,
    type_name: &'static str,
    presentation: OnceLock<String>,
}

impl core::fmt::Debug for StandardCause {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("StandardCause")
            .field("type_name", &self.owner.type_name)
            .finish_non_exhaustive()
    }
}

impl StandardCause {
    /// Borrow the original concrete error without rendering or copying it.
    pub fn error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.owner.error.as_ref()
    }

    /// A diagnostic type breadcrumb, not a stable semantic identifier.
    pub fn type_name(&self) -> &'static str {
        self.owner.type_name
    }

    /// Whether both handles share one original retained payload and cache.
    pub fn shares_owner(&self, other: &Self) -> bool {
        core::ptr::eq(self.owner.as_ref(), other.owner.as_ref())
    }

    /// Render once when presentation is requested; clones share that snapshot.
    ///
    /// This is an infallible presentation boundary, like `Error::to_string`.
    /// Its string allocation follows the resident presentation policy; capture,
    /// cloning and type inspection never invoke it.
    pub fn presentation(&self) -> &str {
        self.owner
            .presentation
            .get_or_init(|| self.owner.error.to_string())
    }
}

/// Retain an original native failure without copying or rendering its payload.
///
/// The erased payload and shared owner use the common fallible allocation path.
/// Copies retain this owner without copying its payload.
///
/// # Errors
/// Returns the original allocation refusal before publishing a partial owner.
pub fn retain_native_cause(cause: impl NativeError + 'static) -> Result<NativeCause, StorageError> {
    retain_cause(cause, |cause| cause as Box<dyn NativeError>)
}

/// Retain a standard error without rendering it or strengthening its protocol.
///
/// Copies share both the concrete error and its first presentation snapshot.
/// # Errors
/// Returns the original storage refusal before publishing a partial owner.
pub fn retain_standard_cause(
    cause: impl Error + Send + Sync + 'static,
) -> Result<StandardCause, StorageError> {
    Ok(StandardCause {
        owner: retain_cause(cause, |cause| StandardPayload {
            type_name: std::any::type_name_of_val(cause.as_ref()),
            error: cause,
            presentation: OnceLock::new(),
        })?,
    })
}

fn retain_cause<E: Send + Sync + 'static, V: Send + Sync + 'static>(
    cause: E,
    erase: impl FnOnce(Box<E>) -> V,
) -> Result<SharedOwned<V>, StorageError> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let cause = memory.boxed(cause)?;
    memory.add_bytes(SharedOwned::<V>::allocation_layout::<Resident>().size())?;
    SharedOwned::try_from_admitted(erase(cause), Resident)
        .map_err(|_| StorageError::AllocationFailed)
}
