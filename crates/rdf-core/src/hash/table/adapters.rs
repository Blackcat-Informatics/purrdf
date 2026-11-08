// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Method adapters for the native iterator, bucket and entry contracts.
//!
//! Signatures remain at their public implementations. Projections preserve the
//! native iterator and its borrowing, ownership and drop behavior.

macro_rules! native_project_next {
    ([$($signature:tt)*] $lookup:expr; $pattern:pat => $project:expr) => {
        $($signature)* {
            // Retain the native match rather than growing LLVM IR with Option::map.
            match $lookup {
                Some($pattern) => Some($project),
                None => None,
            }
        }
    };
}

macro_rules! native_project_fold {
    ([$($signature:tt)*] $iter:expr, $init:ident, $f:ident; $pattern:pat => $project:expr) => {
        $($signature)* {
            $iter.fold($init, |acc, $pattern| $f(acc, $project))
        }
    };
}

macro_rules! native_controlled_project {
    ([$($signature:tt)*] $lookup:expr; $bucket:ident => $project:expr) => {
        $($signature)* {
            let found = $lookup?;
            // SAFETY: native lookup supplies a live bucket; the signature's
            // table borrow provides the projection's lifetime and exclusivity.
            Ok(found.map(|$bucket| $project))
        }
    };
}

macro_rules! native_project_debug {
    ([$($signature:tt)*] $iter:expr, $f:ident; $pattern:pat => $project:expr) => {
        $($signature)* {
            $f.debug_list().entries($iter.map(|$pattern| $project)).finish()
        }
    };
}

macro_rules! native_entry_debug {
    ([$($signature:tt)*] $this:ident, $f:ident) => {
        $($signature)* {
            match *$this {
                Entry::Vacant(ref value) => $f.debug_tuple("Entry").field(value).finish(),
                Entry::Occupied(ref value) => $f.debug_tuple("Entry").field(value).finish(),
            }
        }
    };
}

macro_rules! native_occupied_debug {
    ([$($signature:tt)*] $this:ident, $f:ident) => {
        $($signature)* {
            $f.debug_struct("OccupiedEntry").field("value", $this.get()).finish()
        }
    };
}

macro_rules! native_entry_modify {
    ([$($signature:tt)*] $this:ident, $f:ident) => {
        $($signature)* {
            match $this {
                Entry::Occupied(mut entry) => {
                    $f(entry.get_mut());
                    Entry::Occupied(entry)
                }
                Entry::Vacant(entry) => Entry::Vacant(entry),
            }
        }
    };
}

macro_rules! native_entry_owner {
    ([$($signature:tt)*] $this:ident) => {
        $($signature)* {
            match $this {
                Entry::Occupied(entry) => entry.table,
                Entry::Vacant(entry) => entry.table,
            }
        }
    };
}

macro_rules! native_default {
    ([$($signature:tt)*] $inner:ident) => {
        $($signature)* { Self { $inner: Default::default() } }
    };
    ([$($signature:tt)*] $inner:ident, $marker:ident) => {
        $($signature)* {
            Self { $inner: Default::default(), $marker: PhantomData }
        }
    };
}

/// Implement shared iteration through the native borrowed iterator method.
macro_rules! native_borrowed_iter {
    ([$($implementation:tt)*] $item:ty, $iter:ty) => {
        $($implementation)* {
            type Item = $item;
            type IntoIter = $iter;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
}
