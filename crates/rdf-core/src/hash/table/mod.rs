// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared native hash tables with explicit hashes and controlled work.
//!
//! Derived from hashbrown 0.17.1 under its fixed no-default-feature profile.
//! See the accompanying provenance and original license texts.

#[macro_use]
mod macros;
#[macro_use]
mod adapters;
mod alloc;
mod control;
mod hasher;
mod map;
mod raw;
mod scopeguard;
mod set;
mod table;
mod util;
/// Native global allocator used by the fixed allocator profile.
pub use alloc::Global;
pub use hasher::DefaultHashBuilder;
pub use map::HashMap;
pub use set::HashSet;
pub use table::HashTable;
/// Native map entry and iterator types.
pub mod hash_map {
    pub use super::map::*;
}
/// Native set entry and iterator types.
pub mod hash_set {
    pub use super::set::*;
}
/// Native explicit-hash table entry and iterator types.
pub mod hash_table {
    pub use super::table::*;
}
/// Borrowed key equality. Equal values must hash alike.
pub trait Equivalent<K: ?Sized> {
    /// Whether this borrowed key equals the stored key.
    fn equivalent(&self, key: &K) -> bool;
}
impl<Q: ?Sized, K: ?Sized> Equivalent<K> for Q
where
    Q: Eq,
    K: core::borrow::Borrow<Q>,
{
    fn equivalent(&self, key: &K) -> bool {
        self == key.borrow()
    }
}

mod error;
mod governed;
pub use error::TryReserveError;
pub use governed::{ControlledIter, InsertRefusal};

#[cfg(test)]
mod governed_tests;
