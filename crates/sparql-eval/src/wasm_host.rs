// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The two JavaScript host reads needed by SPARQL evaluation on wasm32.
//!
//! Each call looks up the current global `Date.now` or `Math.random`. The wasm
//! test runner temporarily replaces those globals inside its host seal, so a
//! prohibited clock or entropy read must still trap after module startup.
//!
//! The module is `#[doc(hidden)] pub` rather than private for one reader: the
//! WebAssembly package (`purrdf-wasm`) declares its host imports in one place and
//! takes its wall clock from [`date_now`] here rather than declaring `Date.now` a
//! second time. This is not a supported API — the functions exist so that every
//! clock read in the package goes through the one import the host seal governs.

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = Date, js_name = now)]
    fn date_now_import() -> f64;

    #[wasm_bindgen(js_namespace = Math, js_name = random)]
    fn math_random_import() -> f64;
}

/// Unix epoch milliseconds from the browser or Node host.
///
/// A **wall** clock: the governor's [`WallDeadline`](crate::WallDeadline) reads it on
/// wasm32 in place of the monotonic clock it reads on native targets, and only
/// differences of the value are meaningful to it.
#[inline]
pub fn date_now() -> f64 {
    date_now_import()
}

/// One host pseudorandom draw in `[0, 1)`.
#[inline]
pub fn math_random() -> f64 {
    math_random_import()
}
