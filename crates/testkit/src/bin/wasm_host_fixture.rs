// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fixture `scripts/check-wasm-test-runner.sh` runs on wasm32 to observe the
//! runner's host seal from outside: one case reads the host clock and the host
//! entropy source outside [`without_host_clock_or_entropy`], which must pass, and
//! one reads the clock inside it, which must fail the run naming `Date.now`.
//! The two differ only in the seal, so the passing one is the control that
//! proves the failing one fails because of the seal and not because the read
//! itself is broken.
//!
//! Natively the host's clock cannot be withdrawn, and the cases read nothing.

use std::process::ExitCode;

use purrdf_testkit::harness::{self, Trial, without_host_clock_or_entropy};

/// Read the host clock and entropy source, as code under test would.
fn read_host() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() + js_sys::Math::random()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        0.0
    }
}

fn main() -> ExitCode {
    harness::main([
        Trial::test("an_unsealed_host_read_passes", || {
            let _ = read_host();
            Ok(())
        }),
        Trial::test("a_sealed_host_read_fails_by_name", || {
            let _ = without_host_clock_or_entropy(read_host);
            Ok(())
        }),
    ])
}
