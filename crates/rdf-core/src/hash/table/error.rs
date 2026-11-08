// SPDX-FileCopyrightText: 2016 Amanieu d'Antras
// SPDX-License-Identifier: MIT OR Apache-2.0
// Modified for PurRDF. Local changes and their copyright/grant are recorded
// in crates/rdf-core/src/hash/table/NOTICE and PROVENANCE.md.

/// The error type for `try_reserve` methods.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum TryReserveError {
    /// Error due to the computed capacity exceeding the collection's maximum
    /// (usually `isize::MAX` bytes).
    CapacityOverflow,

    /// The memory allocator returned an error
    AllocError {
        /// The layout of the allocation request that failed.
        layout: std::alloc::Layout,
    },
}

// matches std::collections::TryReserveError
impl core::fmt::Display for TryReserveError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("memory allocation failed")?;
        let reason = match self {
            Self::CapacityOverflow => {
                " because the computed capacity exceeded the collection's maximum"
            }
            Self::AllocError { .. } => " because the memory allocator returned an error",
        };
        f.write_str(reason)
    }
}

impl core::error::Error for TryReserveError {}
