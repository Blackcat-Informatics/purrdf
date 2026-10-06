// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fixture for the wasm32 runner's host seal.
//!
//! `scripts/check-wasm-test-runner.sh` runs it on wasm32 one case at a time
//! (`--exact <name>`). The cases whose names end in `_fails` read a host clock
//! or entropy source inside [`without_host_clock_or_entropy`]. Each must fail
//! its run, and the error must name the source. The cases whose names end in
//! `_passes` make the same read outside the seal, or after it, and must pass.
//! Natively there is no host seal. Only the case that checks the seal hands
//! back its computation's value is registered there.

use purrdf_testkit::harness::without_host_clock_or_entropy;

#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code, reason = "the expansion of #[wasm_bindgen] host imports")]
mod host {
    use wasm_bindgen::prelude::wasm_bindgen;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = Math, js_name = random)]
        fn math_random_import() -> f64;
    }

    /// The raw `Date.now` global, read here rather than through the evaluator's one
    /// import: the fixture proves the runner's seal, so it must reach the global itself.
    /// The import sits inside the function so the helpers ledger names it exactly
    /// (`wasm-host-clock`).
    pub(super) fn date_now() -> f64 {
        #[wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = Date, js_name = now)]
            fn date_now_import() -> f64;
        }
        date_now_import()
    }

    pub(super) fn math_random() -> f64 {
        math_random_import()
    }
}

/// The seal is transparent to a computation that reads no host source.
fn the_seal_returns_the_computations_value_passes() {
    assert_eq!(without_host_clock_or_entropy(|| 6 * 7), 42);
}

#[cfg(target_arch = "wasm32")]
fn a_host_clock_read_outside_the_seal_passes() {
    assert!(host::date_now() > 0.0, "Date.now answers outside the seal");
}

#[cfg(target_arch = "wasm32")]
fn a_host_clock_read_inside_the_seal_fails() {
    let now = without_host_clock_or_entropy(host::date_now);
    panic!("the sealed Date.now answered {now}");
}

#[cfg(target_arch = "wasm32")]
fn a_host_entropy_read_outside_the_seal_passes() {
    let value = host::math_random();
    assert!((0.0..1.0).contains(&value), "Math.random answered {value}");
}

#[cfg(target_arch = "wasm32")]
fn a_host_entropy_read_inside_the_seal_fails() {
    let value = without_host_clock_or_entropy(host::math_random);
    panic!("the sealed Math.random answered {value}");
}

/// Leaving the seal gives the clock back.
#[cfg(target_arch = "wasm32")]
fn a_host_clock_read_after_the_seal_passes() {
    without_host_clock_or_entropy(|| ());
    assert!(host::date_now() > 0.0, "Date.now answers after the seal");
}

/// Leaving an inner seal does not lift the outer one.
#[cfg(target_arch = "wasm32")]
fn a_host_clock_read_after_a_nested_seal_fails() {
    let now = without_host_clock_or_entropy(|| {
        without_host_clock_or_entropy(|| ());
        host::date_now()
    });
    panic!("Date.now answered {now} inside the outer seal");
}

purrdf_testkit::harness_main!(
    the_seal_returns_the_computations_value_passes,
    #[cfg(target_arch = "wasm32")]
    a_host_clock_read_outside_the_seal_passes,
    #[cfg(target_arch = "wasm32")]
    a_host_clock_read_inside_the_seal_fails,
    #[cfg(target_arch = "wasm32")]
    a_host_entropy_read_outside_the_seal_passes,
    #[cfg(target_arch = "wasm32")]
    a_host_entropy_read_inside_the_seal_fails,
    #[cfg(target_arch = "wasm32")]
    a_host_clock_read_after_the_seal_passes,
    #[cfg(target_arch = "wasm32")]
    a_host_clock_read_after_a_nested_seal_fails,
);
