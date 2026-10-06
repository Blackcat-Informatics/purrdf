// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared subprocess transport and fixture helpers for CLI integration tests.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unused_imports)]

use std::io::{ErrorKind, Write as _};
use std::path::Path;
use std::process::{Command, Output, Stdio};

pub(super) use purrdf_testkit::text::{stderr_utf8, stdout_utf8};

/// A `Command` for the built `purrdf` binary.
pub(super) fn purrdf() -> Command {
    Command::new(env!("CARGO_BIN_EXE_purrdf"))
}

/// Run `purrdf` with `args`, returning the captured [`Output`].
pub(super) fn run(args: &[&str]) -> Output {
    purrdf()
        .args(args)
        .output()
        .expect("spawn the built purrdf binary")
}

/// Run `purrdf` with `args`, writing `stdin_bytes` to its standard input (see
/// [`run_with_stdin`] for how an early-closed pipe is judged).
pub(super) fn run_with_input(args: &[&str], stdin_bytes: &[u8]) -> Output {
    run_with_stdin(purrdf().args(args), stdin_bytes)
}

/// [`run_with_input`] over a text document.
pub(super) fn pipe(args: &[&str], stdin_text: &str) -> Output {
    run_with_input(args, stdin_text.as_bytes())
}

/// stdout of an [`Output`] as a `String`, invalid UTF-8 replaced.
pub(super) fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// stderr of an [`Output`] as a `String`, invalid UTF-8 replaced.
pub(super) fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The exit code of an [`Output`].
pub(super) fn code(out: &Output) -> i32 {
    out.status.code().expect("the process exited normally")
}

/// `dir/name` as an owned UTF-8 path string (the shape [`run`] wants).
pub(super) fn path(dir: &Path, name: &str) -> String {
    dir.join(name)
        .to_str()
        .expect("temp path is valid UTF-8")
        .to_owned()
}

/// Write `contents` to `dir/name`, returning the path.
pub(super) fn write_file(dir: &Path, name: &str, contents: &str) -> String {
    let p = path(dir, name);
    std::fs::write(&p, contents).expect("write fixture file");
    p
}

/// Feed stdin while draining both output pipes, then collect the child's verdict.
///
/// A rejecting child may close stdin before the writer finishes. Only that
/// broken pipe is accepted, and only after a normal nonzero exit: callers still
/// assert the exact status, stdout, and diagnostic. Other write failures,
/// including broken pipes after a successful exit, remain harness failures.
pub(super) fn run_with_stdin(command: &mut Command, bytes: &[u8]) -> Output {
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn the built CLI");
    let mut stdin = child.stdin.take().expect("piped child stdin");
    let (output, written) = std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(bytes));
        let output = child.wait_with_output().expect("capture the CLI output");
        let written = writer.join().expect("join the stdin writer");
        (output, written)
    });
    if let Err(error) = written {
        assert!(
            error.kind() == ErrorKind::BrokenPipe
                && output.status.code().is_some_and(|code| code != 0),
            "write CLI stdin: {error}; status: {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    output
}
