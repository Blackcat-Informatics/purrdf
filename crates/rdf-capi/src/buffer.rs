// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The opaque owned-byte-buffer handle returned by serialize / query-json /
//! term-to-ntriples / GTS-write entry points.

use crate::status::PurrdfStatus;

/// An owned byte buffer. Opaque to C; read via `purrdf_buffer_data`, release
/// with `purrdf_buffer_free`.
#[derive(Debug)]
pub struct PurrdfBuffer(pub(crate) Vec<u8>);

impl PurrdfBuffer {
    /// Heap-allocate a buffer handle from owned bytes.
    pub(crate) fn into_raw(bytes: Vec<u8>) -> *mut Self {
        Box::into_raw(Box::new(Self(bytes)))
    }
}

/// Expose the buffer's bytes as a borrowed pointer + length. The pointer is
/// valid until `purrdf_buffer_free(buf)`; the C side must not free it. For an
/// empty buffer `*out_ptr` may be a non-null dangling pointer with `*out_len == 0`.
///
/// # Safety
/// `buf` must be a live buffer handle; `out_ptr`/`out_len` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_buffer_data(
    buf: *const PurrdfBuffer,
    out_ptr: *mut *const u8,
    out_len: *mut usize,
) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if buf.is_null() || out_ptr.is_null() || out_len.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            let bytes = &(*buf).0;
            *out_ptr = bytes.as_ptr();
            *out_len = bytes.len();
            PurrdfStatus::Ok as i32
        })
    }
}

/// Release a buffer handle. No-op on null.
///
/// # Safety
/// `buf` must be null or a live buffer handle not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_buffer_free(buf: *mut PurrdfBuffer) {
    unsafe {
        ffi_guard!((), {
            if !buf.is_null() {
                drop(Box::from_raw(buf));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_exposes_bytes() {
        let raw = PurrdfBuffer::into_raw(b"hello".to_vec());
        let mut ptr: *const u8 = std::ptr::null();
        let mut len: usize = 0;
        unsafe {
            assert_eq!(
                purrdf_buffer_data(raw, &raw mut ptr, &raw mut len),
                PurrdfStatus::Ok as i32
            );
            assert_eq!(len, 5);
            let slice = std::slice::from_raw_parts(ptr, len);
            assert_eq!(slice, b"hello");
            purrdf_buffer_free(raw);
        }
    }

    #[test]
    fn null_args_are_safe() {
        unsafe {
            let mut ptr: *const u8 = std::ptr::null();
            let mut len: usize = 0;
            assert_eq!(
                purrdf_buffer_data(std::ptr::null(), &raw mut ptr, &raw mut len),
                PurrdfStatus::NullPointer as i32
            );
            purrdf_buffer_free(std::ptr::null_mut());
        }
    }
}
