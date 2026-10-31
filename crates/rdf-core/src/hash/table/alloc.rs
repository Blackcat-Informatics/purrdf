// SPDX-FileCopyrightText: 2016 Amanieu d'Antras
// SPDX-License-Identifier: MIT OR Apache-2.0
// Modified for PurRDF. Local changes and their copyright/grant are recorded
// in crates/rdf-core/src/hash/table/NOTICE and PROVENANCE.md.

pub use self::inner::Global;
pub(crate) use self::inner::{Allocator, do_alloc};

mod inner {
    use core::ptr::NonNull;
    use std::alloc::{Layout, alloc, dealloc};

    #[expect(clippy::missing_safety_doc)] // not exposed outside of this crate
    pub unsafe trait Allocator {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, ()>;
        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout);
    }

    /// Native table allocator backed by Rust's global allocation functions.
    ///
    /// This zero-sized handle is the fixed allocator for default tables. Cloning
    /// the handle does not clone an allocation; each table retains ownership of
    /// its own buckets and releases them through the same global allocator.
    #[derive(Copy, Clone, Debug)]
    pub struct Global;

    unsafe impl Allocator for Global {
        #[inline]
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, ()> {
            match unsafe { NonNull::new(alloc(layout)) } {
                Some(data) => {
                    // SAFETY: this is NonNull::slice_from_raw_parts.
                    Ok(unsafe {
                        NonNull::new_unchecked(core::ptr::slice_from_raw_parts_mut(
                            data.as_ptr(),
                            layout.size(),
                        ))
                    })
                }
                None => Err(()),
            }
        }
        #[inline]
        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            unsafe {
                dealloc(ptr.as_ptr(), layout);
            }
        }
    }

    impl Default for Global {
        #[inline]
        fn default() -> Self {
            Global
        }
    }

    pub(crate) fn do_alloc<A: Allocator>(alloc: &A, layout: Layout) -> Result<NonNull<[u8]>, ()> {
        alloc.allocate(layout)
    }
}
