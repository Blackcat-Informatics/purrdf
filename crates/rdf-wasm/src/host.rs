// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one home of this crate's JavaScript host imports.
//!
//! `Reflect.get` is declared here once and read by the option reader, the asynchronous
//! job's option decoding and the test-flag reader. The host clock (`Date.now`) is not
//! declared here: its one import is `purrdf_sparql_eval::wasm_host`, which this crate
//! reaches through the evaluator it already depends on. A second declaration of either
//! import fails `scripts/check-shared-helpers.py`.

use purrdf_lex::allocation::{Ceiling, Memory, StorageError};
use wasm_bindgen::prelude::*;

/// A concrete byte-bridge refusal, mapped by each actual native operation.
pub(crate) enum ByteCaptureError {
    Host(JsValue),
    Length,
    Limit { maximum: usize },
    Storage(StorageError),
}

/// The original admitted byte-copy owner, retained throughout native work.
pub(crate) struct CapturedBytes {
    pub(crate) bytes: Vec<u8>,
    _admission: Ceiling,
}
impl CapturedBytes {
    pub(crate) fn capture(value: &JsValue, maximum: usize) -> Result<Self, ByteCaptureError> {
        let captured = capture_bytes(value).map_err(ByteCaptureError::Host)?;
        let length = byte_length(&captured).map_err(ByteCaptureError::Host)?;
        if !length.is_finite()
            || length.fract() != 0.0
            || length < 0.0
            || length > f64::from(u32::MAX)
        {
            return Err(ByteCaptureError::Length);
        }
        // The finite integral nonnegative length is bounded by u32::MAX.
        let length = length as u32 as usize;
        if length > maximum {
            return Err(ByteCaptureError::Limit { maximum });
        }
        let mut admission = Ceiling(maximum);
        let mut bytes = Vec::new();
        Memory::new(&mut admission)
            .reserve(&mut bytes, length)
            .map_err(ByteCaptureError::Storage)?;
        bytes.resize(length, 0);
        copy_bytes(&captured, &mut bytes).map_err(ByteCaptureError::Host)?;
        Ok(Self {
            bytes,
            _admission: admission,
        })
    }
}

#[wasm_bindgen]
extern "C" {
    /// `Reflect.get(target, key)`. `catch`, so a getter that throws is an `Err` the caller
    /// maps to its own refusal instead of an exception crossing the wasm boundary.
    #[wasm_bindgen(js_namespace = Reflect, js_name = get, catch)]
    fn reflect_get_import(target: &JsValue, key: &str) -> Result<JsValue, JsValue>;

    /// Shared property attachment for original native host error evidence.
    #[wasm_bindgen(js_namespace = Reflect, js_name = set, catch)]
    pub(crate) fn reflect_set(
        target: &JsValue,
        key: &JsValue,
        value: &JsValue,
    ) -> Result<bool, JsValue>;
}

#[wasm_bindgen(raw_module = "./purrdf_bytes.mjs")]
extern "C" {
    /// Retain an intrinsic Uint8Array view without copying its backing bytes.
    #[wasm_bindgen(catch, js_name = captureBytes)]
    pub(crate) fn capture_bytes(value: &JsValue) -> Result<JsValue, JsValue>;

    /// Preserve the entire JavaScript length before checked native conversion.
    #[wasm_bindgen(catch, js_name = byteLength)]
    pub(crate) fn byte_length(value: &JsValue) -> Result<f64, JsValue>;

    /// Fill an already admitted native slice through the host's intrinsic copy.
    #[wasm_bindgen(catch, js_name = copyBytes)]
    pub(crate) fn copy_bytes(source: &JsValue, destination: &mut [u8]) -> Result<(), JsValue>;

    /// Copy retained native output into an independent host Uint8Array.
    #[wasm_bindgen(catch, js_name = ownedBytes)]
    pub(crate) fn owned_bytes(source: &[u8]) -> Result<JsValue, JsValue>;
}

/// `Reflect.get(target, key)`, with a throwing getter as an `Err` the caller maps to its
/// own refusal.
///
/// # Errors
///
/// The value a throwing getter (or a `target` that is not an object) threw.
#[inline]
pub fn reflect_get(target: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    reflect_get_import(target, key)
}
