// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Record independent `XXH64` answers from a caller-selected reference executable.
//! The native implementation is never called while recording these answers.
//! The executable accepts `--version` and `-H64 --seed SEED -`, with full-width
//! unsigned seeds. Some `xxhsum` versions refuse valid seeds near `u64::MAX`.

use std::io::Write as _;
use std::process::{Command, Stdio};

use purrdf_testkit::rng::splitmix64_bytes;
use purrdf_testkit::vectors::Recorder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let oracle = std::env::args_os().nth(1).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "usage: gen_xxh64_vectors ORACLE",
        )
    })?;
    let version = Command::new(&oracle).arg("--version").output()?;
    assert!(
        version.status.success(),
        "oracle version failed: {version:?}"
    );
    let mut version_text = version.stdout;
    version_text.extend(version.stderr);
    let version_text = String::from_utf8(version_text)?;
    let mut recorder = Recorder::new();
    recorder.comment("Independent XXH64 answers; input is splitmix64_bytes(length, 0).")?;
    recorder.comment("Fields: decimal hash seed, input length, hexadecimal unsigned digest.")?;
    recorder.header(
        "oracle",
        version_text
            .lines()
            .next()
            .expect("the oracle identifies itself"),
    )?;
    recorder.header("command", "ORACLE -H64 --seed SEED -")?;
    let lengths: Vec<usize> = (0..=65)
        .chain([127, 128, 129, 255, 256, 257, 1023, 1024, 1025, 65_536])
        .collect();
    for seed in [0, 1, 0x8000_0000_0000_0001, u64::MAX] {
        for &length in &lengths {
            let mut child = Command::new(&oracle)
                .args(["-H64", "--seed", &seed.to_string(), "-"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;
            child
                .stdin
                .take()
                .expect("stdin was configured as a pipe")
                .write_all(&splitmix64_bytes(length, 0))?;
            let output = child.wait_with_output()?;
            assert!(
                output.status.success(),
                "oracle failed for seed {seed}: {output:?}"
            );
            let text = String::from_utf8(output.stdout)?;
            let digest = u64::from_str_radix(
                text.split_ascii_whitespace()
                    .next()
                    .expect("a digest token"),
                16,
            )?;
            recorder.record(&[
                seed.to_string(),
                length.to_string(),
                format!("{digest:016x}"),
            ])?;
        }
    }
    std::fs::write(
        purrdf_testkit::paths::workspace_root!()
            .join("crates/hash-conformance/tests/vectors/xxh64_differential_vectors.txt"),
        recorder.render(),
    )?;
    Ok(())
}
