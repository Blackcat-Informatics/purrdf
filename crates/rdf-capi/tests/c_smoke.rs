// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Drives the C smoke test (`tests/smoke.c`): it compiles the C program against
//! the committed `include/purrdf.h`, links it against the freshly built
//! `libpurrdf` shared library, runs it, and asserts it exits zero. This proves
//! the REAL C-ABI (header + linkage), not just Rust calling Rust.

#![cfg(not(miri))]

use std::path::{Path, PathBuf};
use std::process::Command;

#[path = "support/phases.rs"]
mod phases;

/// The vendored W3C OWL 2 RL entailment corpus, relative to this crate.
///
/// A path rather than a copy: `scripts/check-corpus-frozen.py` digests those bytes,
/// so a fixture transcribing them here would be a second, un-digested corpus free to
/// drift from the one the conformance scoreboard grades.
const CORPUS: &str = "../sparql-conformance/entailment-suite/w3c-owl2-rl";

#[test]
fn c_abi_smoke() -> std::io::Result<()> {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let root = Path::new(manifest).parent().unwrap().parent().unwrap();
    // The profiling launcher delegates to this same Cargo with telemetry flags.
    let cargo = std::env::var("PURRDF_PROFILE_CARGO")
        .or_else(|_| std::env::var("CARGO"))
        .unwrap_or_else(|_| "cargo".to_string());
    let cc = std::env::var("CC").unwrap_or_else(|_| "cc".to_string());
    let receipt = match std::env::var_os("PURRDF_C_PHASE_RECEIPT") {
        Some(path) => PathBuf::from(path),
        None => std::env::current_exe()?.with_extension("phases.json"),
    };
    let mut phases = phases::Recorder::new(receipt, purrdf_lex::json::Object::new().into())?;
    let context = phases.check("host-input-identities", || {
        phases::context(root, &cargo, &cc)
    })?;
    phases.evidence("host", context)?;
    let package = phases.run(
        "cargo-package-identity",
        Command::new(&cargo)
            .args(["pkgid", "-p", "purrdf-capi", "--locked"])
            .current_dir(root),
    )?;
    let package_id = std::str::from_utf8(&package.stdout)
        .map_err(std::io::Error::other)?
        .trim();
    let header = phases.check("header-identity", || {
        phases::identity(&Path::new(manifest).join("include/purrdf.h"))
    })?;
    phases.evidence("header", header)?;
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
    // With debug assertions on, this binary was compiled by `cargo test` under the
    // `test` profile, so that is the profile the cdylib is built under. It used to
    // be `dev`, which has the same codegen (`[profile.test]` inherits `dev` and
    // overrides only `debug`, see Cargo.toml) but different debug info — so every
    // workspace crate under purrdf-capi was compiled a second time, at opt-level 3,
    // for one shared library: most of this test's five-minute run. Under `test`
    // the nested build reuses the units `cargo test` already compiled. Without
    // debug assertions the harness came from `cargo test --release` or `cargo
    // bench`, and `bench` inherits `release` codegen. Matching is a build-time
    // economy, not a correctness requirement — the cdylib is located from Cargo's
    // JSON output below, never from this name.
    let profile = if cfg!(debug_assertions) {
        "test"
    } else {
        "release"
    };
    let mut cargo_build = Command::new(&cargo);
    cargo_build.args([
        "build",
        "--locked",
        "-p",
        "purrdf-capi",
        "--profile",
        profile,
        "--message-format=json-render-diagnostics",
    ]);
    cargo_build.current_dir(root);
    let output = phases.run("cargo-cdylib-preparation", &mut cargo_build)?;
    let artifact = phases.check("cargo-artifact-validation", || {
        phases::cdylib(
            &phases::cargo_messages(&output.stdout, false)?,
            package_id,
            &lib_name,
        )
    })?;
    phases.evidence("cargo_artifact", artifact.message)?;
    let library = phases.check("library-identity", || phases::identity(&artifact.path))?;
    phases.evidence("library", library)?;
    let lib = artifact.path;
    let profile_dir = lib.parent().expect("cdylib profile directory");
    assert!(
        lib.exists(),
        "{lib_name} not found at {} even after building purrdf-capi",
        lib.display()
    );

    let bin = profile_dir.join("purrdf_c_smoke");

    let object = profile_dir.join("purrdf_c_smoke.o");
    phases.run(
        "c-smoke-object-compile",
        Command::new(&cc)
            .arg(&smoke_c)
            .arg("-std=c11")
            .arg(format!("-I{header_dir}"))
            .arg("-c")
            .arg("-o")
            .arg(&object),
    )?;
    phases.run(
        "c-smoke-link",
        Command::new(&cc)
            .arg(&object)
            .arg(format!("-L{}", profile_dir.display()))
            .arg("-lpurrdf")
            .arg("-o")
            .arg(&bin),
    )?;

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
    phases.run(
        "c-smoke-runtime",
        Command::new(&bin)
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
            .env(loader_path_var, profile_dir),
    )?;

    // Compile and run the public projection example too, so its documented
    // ownership/free order and additive project/lift declarations cannot drift.
    let example_c = format!("{manifest}/examples/projection_roundtrip.c");
    let example_bin = profile_dir.join("purrdf_c_projection_example");
    let example_archive = profile_dir.join("purrdf_c_projection_example.tar");
    phases.check(
        "projection-output-preparation",
        || match std::fs::remove_file(&example_archive) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        },
    )?;
    let example_object = profile_dir.join("purrdf_c_projection_example.o");
    phases.run(
        "c-projection-object-compile",
        Command::new(&cc)
            .arg(&example_c)
            .arg("-std=c11")
            .arg(format!("-I{header_dir}"))
            .arg("-c")
            .arg("-o")
            .arg(&example_object),
    )?;
    phases.run(
        "c-projection-link",
        Command::new(&cc)
            .arg(&example_object)
            .arg(format!("-L{}", profile_dir.display()))
            .arg("-lpurrdf")
            .arg("-o")
            .arg(&example_bin),
    )?;
    phases.run(
        "c-projection-runtime",
        Command::new(&example_bin)
            .arg(&example_archive)
            .env(loader_path_var, profile_dir),
    )?;
    phases.check("c-projection-output-validation", || {
        let example_metadata = std::fs::metadata(&example_archive)?;
        if example_metadata.len() == 0 {
            return Err(std::io::Error::other(
                "C projection example did not materialize its archive",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;

            if example_metadata.permissions().mode() & 0o777 != 0o600 {
                return Err(std::io::Error::other(
                    "C projection example archive permissions are not owner-only",
                ));
            }
        }
        Ok(())
    })?;
    let archive = phases.check("projection-output-identity", || {
        phases::identity(&example_archive)
    })?;
    phases.evidence("projection_archive", archive)?;
    phases.check("projection-output-cleanup", || {
        std::fs::remove_file(example_archive)
    })?;
    Ok(())
}
