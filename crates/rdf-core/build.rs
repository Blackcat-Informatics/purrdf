// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Records this build's **build identity** for `purrdf_core::distance::BuildIdentity`.
//!
//! The reassociated distance body compiles to different instructions under a different
//! compiler, target CPU, optimisation level or codegen flag, and none of those is a `cfg`
//! the source can read. This script reads them from what `cargo` hands every build script
//! -- `RUSTC` (the compiler itself, never a `RUSTC_WRAPPER`), `TARGET`, `OPT_LEVEL` and
//! `CARGO_ENCODED_RUSTFLAGS` -- and from `rustc -vV` and `rustc --print target-cpus`, and
//! writes the text `distance/build_identity.rs` derives from them into the
//! `PURRDF_BUILD_IDENTITY` compile-time variable. It reads no file, no network and no
//! clock, so identical inputs give identical text.

use std::env;
use std::process::Command;

#[allow(
    unreachable_pub,
    reason = "the module's public input type is the library's public API; in this script \
              it is private"
)]
#[path = "src/distance/build_identity.rs"]
mod build_identity;

/// The variables this script reads beyond those `cargo` sets for it, and those whose
/// change must rerun it.
const WATCHED: [&str; 7] = [
    "RUSTC",
    "TARGET",
    "OPT_LEVEL",
    "CARGO_ENCODED_RUSTFLAGS",
    "RUSTFLAGS",
    "CARGO_BUILD_RUSTFLAGS",
    "CARGO_BUILD_TARGET",
];

/// The standard output of `rustc` run with `args`, or `None` when it cannot run or fails.
fn rustc(compiler: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(compiler).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/distance/build_identity.rs");
    for variable in WATCHED {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    let compiler = env::var("RUSTC").unwrap_or_else(|_| "rustc".to_owned());
    let target = env::var("TARGET").expect("cargo sets TARGET for every build script");
    let opt_level = env::var("OPT_LEVEL").expect("cargo sets OPT_LEVEL for every build script");
    let encoded = env::var("CARGO_ENCODED_RUSTFLAGS").unwrap_or_default();
    let rustflags: Vec<&str> = encoded
        .split('\u{1f}')
        .filter(|flag| !flag.is_empty())
        .collect();

    // Without the compiler's version there is no identity to record, and recording a
    // placeholder would make two different compilers look like one build.
    let version = rustc(&compiler, &["-vV"])
        .unwrap_or_else(|| panic!("`{compiler} -vV` must run to record the build identity"));
    let cpus = rustc(&compiler, &["--print", "target-cpus", "--target", &target]);

    let text = build_identity::compiler_text(&build_identity::BuildInputs {
        rustc_version: &version,
        target_cpus: cpus.as_deref(),
        target: &target,
        rustflags: &rustflags,
        opt_level: &opt_level,
    });
    println!("cargo:rustc-env=PURRDF_BUILD_IDENTITY={text}");
}
