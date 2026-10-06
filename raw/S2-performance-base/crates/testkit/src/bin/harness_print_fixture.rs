// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The fixture for printing from inside a case. Every case passes, and each
//! one prints `print-fixture case=<name> line=<n>` lines.
//!
//! `tests/harness.rs` runs it natively on four worker threads with
//! `--nocapture` and a deadline. The cases print through
//! [`print_line`], through `println!`, and from a thread the case spawns. That
//! is the traffic that would block forever if the reporter held the standard
//! output lock for the whole run. `scripts/check-wasm-test-runner.sh` runs it
//! on wasm32, where only [`print_line`] reaches the host console, and checks
//! that its lines arrive.

use purrdf_testkit::harness::print_line;

/// Lines each printing case writes.
const LINES: usize = 32;

fn print_lines(case: &str) {
    for line in 0..LINES {
        print_line(&format!("print-fixture case={case} line={line}"));
    }
}

fn prints_through_print_line() {
    print_lines("prints_through_print_line");
}

fn prints_more_through_print_line() {
    print_lines("prints_more_through_print_line");
}

/// `println!` reaches no console on wasm32-unknown-unknown, so this case is
/// native only.
#[cfg(not(target_arch = "wasm32"))]
fn prints_through_println() {
    for line in 0..LINES {
        println!("print-fixture case=prints_through_println line={line}");
    }
}

/// wasm32-unknown-unknown has no threads, so this case is native only.
#[cfg(not(target_arch = "wasm32"))]
fn prints_from_a_spawned_thread() {
    std::thread::spawn(|| print_lines("prints_from_a_spawned_thread"))
        .join()
        .expect("the printing thread finishes");
}

purrdf_testkit::harness_main!(
    prints_through_print_line,
    #[cfg(not(target_arch = "wasm32"))]
    prints_through_println,
    #[cfg(not(target_arch = "wasm32"))]
    prints_from_a_spawned_thread,
    prints_more_through_print_line,
);
