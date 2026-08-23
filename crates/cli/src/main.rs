// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `purrdf` binary entry point.
//!
//! A thin shell over [`purrdf_cli::run`], which parses the command line, dispatches
//! it, and maps the outcome to a process exit code. All pipeline logic lives in the
//! `purrdf_cli` library so it is reachable by the crate's benchmarks and integration
//! tests as well as by this binary.

fn main() {
    purrdf_cli::run();
}
