// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Drives the C smoke test (`tests/smoke.c`): it compiles the C program against
//! the committed `include/purrdf.h`, links it against the freshly built
//! `libpurrdf` shared library, runs it, and asserts it exits zero. This proves
//! the REAL C-ABI (header + linkage), not just Rust calling Rust.

#![cfg(not(miri))]

use std::path::PathBuf;
use std::process::Command;

fn cdylib_artifact(messages: &[u8], lib_name: &str) -> Option<PathBuf> {
    let messages = std::str::from_utf8(messages).ok()?;
    for line in messages.lines() {
        let Ok(message) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if message.get("reason").and_then(serde_json::Value::as_str) != Some("compiler-artifact")
            || !message
                .pointer("/target/kind")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "cdylib"))
        {
            continue;
        }
        let Some(filenames) = message
            .get("filenames")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        for filename in filenames.iter().filter_map(serde_json::Value::as_str) {
            let path = PathBuf::from(filename);
            if path.file_name().and_then(|name| name.to_str()) == Some(lib_name) {
                return Some(path);
            }
        }
    }
    None
}

#[test]
fn c_abi_smoke() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let smoke_c = format!("{manifest}/tests/smoke.c");
    let header_dir = format!("{manifest}/include");

    // The integration-test binary lives under `<profile>/deps/<name>-<hash>`,
    // including when Cargo routes intermediates through a separate build dir,
    // so its grandparent still names the active profile.
    let test_exe = std::env::current_exe().expect("current_exe");
    let test_profile_dir: PathBuf = test_exe
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
    // The cdylib is a separate build artifact that `cargo test` / `cargo
    // nextest` do NOT build as a dependency of this test binary. Always build
    // it before linkage: existence alone is insufficient because a prior test
    // run may have left a stale shared library for older Rust sources.
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let profile = test_profile_dir
        .file_name()
        .and_then(|name| name.to_str())
        .expect("Cargo profile directory name");
    let mut cargo_build = Command::new(&cargo);
    cargo_build.args([
        "build",
        "-p",
        "purrdf-capi",
        "--message-format=json-render-diagnostics",
    ]);
    if profile != "debug" {
        cargo_build.args(["--profile", profile]);
    }
    let output = cargo_build
        .output()
        .expect("failed to invoke cargo to build the libpurrdf cdylib");
    assert!(
        output.status.success(),
        "cargo build -p purrdf-capi for profile `{profile}` failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let lib = cdylib_artifact(&output.stdout, &lib_name).unwrap_or_else(|| {
        panic!(
            "Cargo did not report {lib_name} for profile `{profile}`:\n{}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    let profile_dir = lib.parent().expect("cdylib profile directory");
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
        .arg(format!("{manifest}/../rdf/tests/fixtures/okf-terms.trig"))
        .arg(format!("{manifest}/../rdf/tests/fixtures/okf-terms.json"))
        .env(loader_path_var, profile_dir)
        .status()
        .expect("failed to run the C smoke binary");
    assert!(run.success(), "C smoke binary returned a failure exit code");

    // Compile and run the public projection example too, so its documented
    // ownership/free order and additive project/lift declarations cannot drift.
    let example_c = format!("{manifest}/examples/projection_roundtrip.c");
    let example_bin = profile_dir.join("purrdf_c_projection_example");
    let example_archive = profile_dir.join("purrdf_c_projection_example.tar");
    let _ = std::fs::remove_file(&example_archive);
    let compile_example = Command::new(&cc)
        .arg(&example_c)
        .arg("-std=c11")
        .arg(format!("-I{header_dir}"))
        .arg(format!("-L{}", profile_dir.display()))
        .arg("-lpurrdf")
        .arg("-o")
        .arg(&example_bin)
        .status()
        .expect("failed to compile the C projection example");
    assert!(
        compile_example.success(),
        "C projection example failed to compile/link"
    );
    let run_example = Command::new(&example_bin)
        .arg(&example_archive)
        .env(loader_path_var, profile_dir)
        .status()
        .expect("failed to run the C projection example");
    assert!(
        run_example.success(),
        "C projection example returned a failure exit code"
    );
    let example_metadata =
        std::fs::metadata(&example_archive).expect("C projection example archive metadata");
    assert!(
        example_metadata.len() > 0,
        "C projection example did not materialize its archive"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;

        assert_eq!(
            example_metadata.permissions().mode() & 0o777,
            0o600,
            "C projection example archive permissions are not owner-only"
        );
    }
    let _ = std::fs::remove_file(example_archive);
}
