// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fixture for the runner's console discipline: every case writes to
//! standard output while the runner is itself reporting, one from its own
//! worker thread and one from a thread the case spawns and joins. A runner
//! that held the standard-output lock across the run would block the first
//! such write forever, so `tests/harness.rs` runs this fixture as a child
//! process under a deadline and requires it to finish with every line
//! printed.

use std::process::ExitCode;

use purrdf_testkit::harness::{self, Trial};

fn main() -> ExitCode {
    harness::main((0..8).map(|index| {
        Trial::test(format!("case_{index}_prints"), move || {
            println!("printed by case {index} on its worker");
            std::thread::spawn(move || println!("printed by a thread case {index} spawned"))
                .join()
                .map_err(|_| "the spawned printer panicked".into())
        })
    }))
}
