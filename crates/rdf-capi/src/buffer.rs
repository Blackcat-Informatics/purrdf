// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The opaque owned-byte-buffer handle returned by serialize / query-json /
//! term-to-ntriples / GTS-write entry points.

use crate::handles::free_handle;
use crate::status::PurrdfStatus;

/// Owned bytes or an immutable canonical result under its original admission.
#[derive(Debug)]
enum BufferBytes {
    Owned(Vec<u8>),
    Canonical(purrdf_core::Canonicalized),
}

/// An owned byte buffer. Opaque to C; read via `purrdf_buffer_data`, release
/// with `purrdf_buffer_free`. Canonical payloads retain their source's grant.
#[derive(Debug)]
pub struct PurrdfBuffer {
    bytes: BufferBytes,
}

impl PurrdfBuffer {
    /// Borrow the original bytes for the lifetime of this opaque handle.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        match &self.bytes {
            BufferBytes::Owned(bytes) => bytes,
            BufferBytes::Canonical(canonical) => canonical.nquads().as_bytes(),
        }
    }
}

impl From<Vec<u8>> for PurrdfBuffer {
    fn from(bytes: Vec<u8>) -> Self {
        Self {
            bytes: BufferBytes::Owned(bytes),
        }
    }
}

impl From<String> for PurrdfBuffer {
    fn from(text: String) -> Self {
        Self::from(text.into_bytes())
    }
}

impl From<purrdf_core::Canonicalized> for PurrdfBuffer {
    fn from(canonical: purrdf_core::Canonicalized) -> Self {
        Self {
            bytes: BufferBytes::Canonical(canonical),
        }
    }
}

impl From<purrdf_validate::regime::ReasoningText> for PurrdfBuffer {
    fn from(text: purrdf_validate::regime::ReasoningText) -> Self {
        match text {
            purrdf_validate::regime::ReasoningText::Rendered(text) => Self::from(text),
            purrdf_validate::regime::ReasoningText::Canonical(canonical) => Self::from(canonical),
        }
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
            let bytes = (*buf).as_bytes();
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
    unsafe { free_handle::<PurrdfBuffer>(buf) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handles::into_handle;

    #[test]
    fn data_exposes_bytes() {
        let raw = into_handle(PurrdfBuffer::from(b"hello".to_vec()));
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
