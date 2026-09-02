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

/// The vendored W3C OWL 2 RL entailment corpus, relative to this crate.
///
/// A path rather than a copy: `scripts/check-corpus-frozen.py` digests those bytes,
/// so a fixture transcribing them here would be a second, un-digested corpus free to
/// drift from the one the conformance scoreboard grades.
const CORPUS: &str = "../sparql-conformance/entailment-suite/w3c-owl2-rl";

#[test]
fn c_abi_smoke() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let smoke_c = format!("{manifest}/tests/smoke.c");
    let header_dir = format!("{manifest}/include");

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
    // Which profile to build the cdylib under, read from THIS binary's own
    // compilation rather than from where the binary happens to sit on disk.
    //
    // This used to be the grandparent directory of `current_exe()`, on the
    // assumption that a test binary always lives in `<profile>/deps/`. Cargo's
    // build-dir layout broke that: intermediates now land under a hash of the
    // build configuration, so the grandparent is a name like `d9a41d75b93a9f20`
    // and the nested build died on "profile `d9a41d75b93a9f20` is not defined".
    // A directory layout is Cargo's to change whenever it likes; whether this
    // compilation has debug assertions is a property of the compilation itself.
    //
    // The mapping is exact for every profile this workspace declares: `dev` is
    // the only one leaving debug assertions on, and `bench` inherits `release`
    // codegen. Matching is a build-time economy, not a correctness requirement —
    // the cdylib is located from Cargo's JSON output below, never from this name.
    let profile = if cfg!(debug_assertions) {
        "dev"
    } else {
        "release"
    };
    let mut cargo_build = Command::new(&cargo);
    cargo_build.args([
        "build",
        "-p",
        "purrdf-capi",
        "--profile",
        profile,
        "--message-format=json-render-diagnostics",
    ]);
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
    // The third argument is the COMMITTED tri-host entailment golden vector. The
    // C program walks it case by case through `purrdf_entail_materialize_to_nquads`
    // and compares both outputs byte for byte, so the artifact the Rust test, the
    // WASM module and the Python suite all check reaches the C ABI too — one
    // artifact, four hosts, rather than a fixture per host.
    let run = Command::new(&bin)
        .arg(format!("{manifest}/../rdf/tests/fixtures/okf-terms.trig"))
        .arg(format!("{manifest}/../rdf/tests/fixtures/okf-terms.json"))
        .arg(format!(
            "{manifest}/../validate/tests/fixtures/regime-boundary.vectors"
        ))
        // Arguments four to six are `webont-imports-011` and the support ontology
        // its premise `owl:imports`, taken from the byte-frozen W3C corpus rather
        // than copied into a fixture of this crate's own. They are what proves the
        // caller-supplied import table reaches a REAL C caller: the header would
        // not even compile against a program passing arrays it does not declare.
        .arg(format!(
            "{manifest}/{CORPUS}/cases/webont-imports-011/premise.rdf"
        ))
        .arg(format!(
            "{manifest}/{CORPUS}/cases/webont-imports-011/conclusion.rdf"
        ))
        .arg(format!("{manifest}/{CORPUS}/imports/support011-A.rdf"))
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
