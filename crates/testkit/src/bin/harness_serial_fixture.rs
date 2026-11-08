// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Child-process fixture for enforced serial selection and reporting.

use purrdf_testkit::harness::{self, Trial};

fn main() -> std::process::ExitCode {
    harness::serial_main([
        Trial::test("alpha_passes", || Ok(())),
        Trial::test("delta_panics", || panic!("the planted serial failure")),
        Trial::test("epsilon_is_ignored", || panic!("ignored case ran")).with_ignored_flag(true),
    ])
}
