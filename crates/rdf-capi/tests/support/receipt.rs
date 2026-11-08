// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Own the receipt returned by a C call, including early assertion exits.
use super::buffer::{PurrdfBuffer, purrdf_buffer_free};

pub(crate) struct Output(pub(crate) *mut PurrdfBuffer);
impl Default for Output {
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        unsafe { purrdf_buffer_free(self.0) };
    }
}
