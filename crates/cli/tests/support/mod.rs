// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Shared subprocess transport for CLI integration tests.

use std::io::{ErrorKind, Write as _};
use std::process::{Command, Output, Stdio};

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
