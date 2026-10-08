// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Host-only receipts for the actual native CI commands.
//! Build outside measured targets. `run REQUEST.json` captures one arm/lane;
//! `compare POLICY.json` validates complete collections before reporting costs.
//! Invoked as `cargo` from the arm's private PATH, this executable adds only
//! supported Cargo telemetry options and delegates to the captured real Cargo.

#[path = "../tests/support/phases.rs"]
mod phases;
#[path = "../tests/support/profile.rs"]
mod profile;

fn main() -> std::io::Result<()> {
    let mut arguments = std::env::args();
    let executable = arguments.next().unwrap_or_default();
    if std::path::Path::new(&executable)
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        == Some("cargo")
    {
        return profile::cargo_shim(arguments.collect());
    }
    let operation = arguments
        .next()
        .ok_or_else(|| std::io::Error::other("expected run REQUEST.json or compare POLICY.json"))?;
    let input = arguments
        .next()
        .ok_or_else(|| std::io::Error::other("missing JSON input path"))?;
    if arguments.next().is_some() {
        return Err(std::io::Error::other("unexpected trailing argument"));
    }
    match operation.as_str() {
        "run" => profile::run(std::path::Path::new(&input)),
        "compare" => profile::compare(std::path::Path::new(&input)),
        _ => Err(std::io::Error::other("expected run or compare")),
    }
}
