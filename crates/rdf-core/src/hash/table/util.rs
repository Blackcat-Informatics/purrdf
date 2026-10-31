// SPDX-FileCopyrightText: 2016 Amanieu d'Antras
// SPDX-License-Identifier: MIT OR Apache-2.0
// Modified for PurRDF. Local changes and their copyright/grant are recorded
// in crates/rdf-core/src/hash/table/NOTICE and PROVENANCE.md.

// FIXME: Replace with `core::hint::{likely, unlikely}` once they are stable.

#[inline(always)]
#[cold]
fn cold_path() {}

#[inline(always)]
pub(crate) fn likely(b: bool) -> bool {
    if b {
        true
    } else {
        cold_path();
        false
    }
}

#[inline(always)]
pub(crate) fn unlikely(b: bool) -> bool {
    if b {
        cold_path();
        true
    } else {
        false
    }
}

// FIXME: use strict provenance functions once they are stable.
// Implement it with a transmute for now.
#[inline(always)]
pub(crate) fn invalid_mut<T>(addr: usize) -> *mut T {
    unsafe { core::mem::transmute(addr) }
}
