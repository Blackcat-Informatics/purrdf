// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one home of this crate's JavaScript host imports.
//!
//! `Reflect.get` is declared here once and read by the option reader, the asynchronous
//! job's option decoding and the test-flag reader. The host clock (`Date.now`) is not
//! declared here: its one import is `purrdf_sparql_eval::wasm_host`, which this crate
//! reaches through the evaluator it already depends on. A second declaration of either
//! import fails `scripts/check-shared-helpers.py`.

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// `Reflect.get(target, key)`. `catch`, so a getter that throws is an `Err` the caller
    /// maps to its own refusal instead of an exception crossing the wasm boundary.
    #[wasm_bindgen(js_namespace = Reflect, js_name = get, catch)]
    fn reflect_get_import(target: &JsValue, key: &str) -> Result<JsValue, JsValue>;
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
