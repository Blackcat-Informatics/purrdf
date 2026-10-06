// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fixture `tests/harness.rs` runs as a child process: five cases — three
//! that pass, one that panics, one ignored — handed to the runner exactly as a
//! `harness = false` test target hands its own. Running it as a real process
//! is what lets the oracle observe the exit status and the console output
//! instead of a model of them. `scripts/check-wasm-test-runner.sh` runs the
//! same fixture on wasm32, through the wasm32 test runner.

use std::process::ExitCode;

use purrdf_testkit::harness::{self, Trial};

fn main() -> ExitCode {
    harness::main([
        Trial::test("alpha_passes", || Ok(())),
        Trial::test("beta_passes", || Ok(())),
        Trial::test("delta_panics", || panic!("the planted failure")),
        Trial::test("epsilon_is_ignored", || {
            panic!("an ignored case must not run")
        })
        .with_ignored_flag(true),
        Trial::test("gamma_passes", || Ok(())),
    ])
}
