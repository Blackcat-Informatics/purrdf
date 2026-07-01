// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drives the C smoke test (`tests/smoke.c`): it compiles the C program against
//! the committed `include/purrdf.h`, links it against the freshly built
//! `libpurrdf` shared library, runs it, and asserts it exits zero. This proves
//! the REAL C-ABI (header + linkage), not just Rust calling Rust.

#![cfg(not(miri))]

use std::path::PathBuf;
use std::process::Command;

#[test]
fn c_abi_smoke() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let smoke_c = format!("{manifest}/tests/smoke.c");
    let header_dir = format!("{manifest}/include");

    // The integration-test binary lives at `<target>/<profile>/deps/<name>-<hash>`,
    // so its grandparent is the profile dir where the cdylib is emitted. This is
    // robust to a custom `CARGO_TARGET_DIR`.
    let test_exe = std::env::current_exe().expect("current_exe");
    let profile_dir: PathBuf = test_exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("profile dir")
        .to_path_buf();

    // Build the platform-correct shared-library file name: `libpurrdf.so` on
    // Linux, `libpurrdf.dylib` on macOS, `purrdf.dll` on Windows. `DLL_SUFFIX`
    // already includes the leading dot.
    let lib_name = format!(
        "{}purrdf{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX
    );
    let lib = profile_dir.join(&lib_name);
    if !lib.exists() {
        // The cdylib is a separate build artifact that `cargo test` / `cargo
        // nextest` do NOT build as a dependency of this test binary. Build it on
        // demand so the smoke is hermetic in EVERY lane (the workspace
        // `cargo nextest` run and the dedicated `make capi-check`), not only when
        // some earlier step happened to build the cdylib first.
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let status = Command::new(&cargo)
            .args(["build", "-p", "purrdf-capi"])
            .status()
            .expect("failed to invoke cargo to build the libpurrdf cdylib");
        assert!(status.success(), "cargo build -p purrdf-capi failed");
    }
    assert!(
        lib.exists(),
        "{lib_name} not found at {} even after building purrdf-capi",
        lib.display()
    );

    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let bin = profile_dir.join("purrdf_c_smoke");

    let compile = Command::new(&cc)
        .arg(&smoke_c)
        .arg("-std=c11")
        .arg(format!("-I{header_dir}"))
        .arg(format!("-L{}", profile_dir.display()))
        .arg("-lpurrdf")
        .arg("-o")
        .arg(&bin)
        .status()
        .expect("failed to invoke the C compiler");
    assert!(compile.success(), "C smoke failed to compile/link");

    // The loader's library-search env var is platform-specific: `LD_LIBRARY_PATH`
    // on Linux/BSD, `DYLD_LIBRARY_PATH` on macOS, `PATH` on Windows.
    let loader_path_var = if cfg!(target_os = "macos") {
        "DYLD_LIBRARY_PATH"
    } else if cfg!(target_os = "windows") {
        "PATH"
    } else {
        "LD_LIBRARY_PATH"
    };
    let run = Command::new(&bin)
        .env(loader_path_var, &profile_dir)
        .status()
        .expect("failed to run the C smoke binary");
    assert!(run.success(), "C smoke binary returned a failure exit code");
}
