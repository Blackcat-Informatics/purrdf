// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Goldens compare byte for byte — CRLF, trailing whitespace and the final
//! newline included — and regeneration writes the produced bytes verbatim;
//! byte goldens do the same for output that is not text, and name the first
//! differing offset. The regeneration switch is exactly `1`, by the one rule
//! `env_flag` states.
//!
//! These run where a test can create a path; wasm32-unknown-unknown has no file
//! system, and the scratch-space types do not exist there.

#![cfg(not(target_arch = "wasm32"))]

use std::process::Command;

use purrdf_testkit::golden::{self, GoldenError, Mode};
use purrdf_testkit::{env_flag, temp_dir};

#[test]
fn an_identical_golden_passes_and_any_byte_difference_fails() {
    let dir = temp_dir!().expect("temp dir");
    let path = dir.path().join("sample.csv");
    std::fs::write(&path, "a,b\r\n1,2 \r\n").expect("write golden");

    golden::check(&path, "a,b\r\n1,2 \r\n", Mode::Compare).expect("identical bytes pass");
    for different in [
        "a,b\n1,2 \n",
        "a,b\r\n1,2\r\n",
        "a,b\r\n1,2 \r\n\n",
        "a,b\r\n1,2 ",
    ] {
        let error = golden::check(&path, different, Mode::Compare).expect_err("differs");
        assert!(
            matches!(error, GoldenError::Differs { .. }),
            "{different:?}"
        );
        assert!(
            error
                .to_string()
                .contains("PURRDF_REGENERATE_GOLDEN=1 rewrites it"),
            "{error}"
        );
    }
}

#[test]
fn a_missing_golden_fails_and_names_the_regeneration_switch() {
    let dir = temp_dir!().expect("temp dir");
    let error = golden::check(&dir.path().join("absent.txt"), "x", Mode::Compare)
        .expect_err("a missing golden is a failure");
    assert!(matches!(error, GoldenError::Read { .. }));
    assert!(
        error
            .to_string()
            .contains("PURRDF_REGENERATE_GOLDEN=1 writes it"),
        "{error}"
    );
}

#[test]
fn regeneration_writes_the_bytes_verbatim_and_creates_directories() {
    let dir = temp_dir!().expect("temp dir");
    let path = dir.path().join("nested/deeper/out.tsv");
    let produced = "?x\t?y\r\n\"a\"\t<b>\r\n";
    golden::check(&path, produced, Mode::Regenerate).expect("regenerates");
    assert_eq!(
        std::fs::read(&path).expect("read back"),
        produced.as_bytes()
    );
    golden::check(&path, produced, Mode::Compare).expect("the regenerated golden compares equal");
}

#[test]
fn the_macro_resolves_the_calling_crates_golden_directory() {
    // `tests/golden/macro_probe.txt` is this crate's own golden, CRLF included.
    purrdf_testkit::assert_golden!("macro_probe.txt", "resolved\r\n");
}

/// Asserts the mode this process reads, when the parent test below asks, and
/// that the mode is `env_flag`'s answer for the same variable.
#[test]
fn mode_probe() {
    if let Some(expected) = std::env::var_os("PURRDF_TESTKIT_EXPECT_MODE") {
        assert_eq!(
            Some(format!("{:?}", Mode::from_env()).as_str()),
            expected.to_str()
        );
        assert_eq!(
            env_flag(golden::REGENERATE_ENV),
            Mode::from_env() == Mode::Regenerate
        );
    }
}

#[test]
fn only_the_value_one_switches_to_regeneration() {
    // The switch is read from the environment, which a test must not modify
    // under its siblings; each value is observed in a child run of this binary.
    let exe = std::env::current_exe().expect("this test binary");
    let cases = [
        (Some("1"), "Regenerate"),
        (Some("0"), "Compare"),
        (Some("true"), "Compare"),
        (Some("yes"), "Compare"),
        (Some(""), "Compare"),
        (Some(" 1"), "Compare"),
        (Some("1 "), "Compare"),
        (Some("1\n"), "Compare"),
        (Some("01"), "Compare"),
        (Some("11"), "Compare"),
        (None, "Compare"),
    ];
    for (value, expected) in cases {
        let mut child = Command::new(&exe);
        child
            .args(["--exact", "mode_probe", "--test-threads=1"])
            .env("PURRDF_TESTKIT_EXPECT_MODE", expected);
        match value {
            Some(value) => child.env(golden::REGENERATE_ENV, value),
            None => child.env_remove(golden::REGENERATE_ENV),
        };
        let output = child.output().expect("run the probe");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{value:?}: {stdout}");
        assert!(
            stdout.contains("1 passed"),
            "{value:?}: the probe ran: {stdout}"
        );
    }
}

#[test]
fn an_identical_byte_golden_passes_and_any_byte_difference_fails_at_its_offset() {
    let dir = temp_dir!().expect("temp dir");
    let path = dir.path().join("sample.bin");
    let golden: &[u8] = b"\x00\xff\r\n\x80";
    std::fs::write(&path, golden).expect("write golden");

    golden::check_bytes(&path, golden, Mode::Compare).expect("identical bytes pass");
    let longer = [golden, b"\x00"].concat();
    let cases: [(&[u8], usize); 5] = [
        (b"\x00\xfe\r\n\x80", 1),
        (b"\x00\xff\n\n\x80", 2),
        (&golden[..4], 4),
        (&longer, 5),
        (b"", 0),
    ];
    for (different, offset) in cases {
        let error = golden::check_bytes(&path, different, Mode::Compare).expect_err("differs");
        assert!(
            matches!(error, GoldenError::DiffersBytes { .. }),
            "{different:?}"
        );
        let text = error.to_string();
        assert!(
            text.contains(&format!("at byte {offset}:")),
            "{different:?}: {text}"
        );
        assert!(
            text.contains("PURRDF_REGENERATE_GOLDEN=1 rewrites it"),
            "{text}"
        );
    }
    // The report names the bytes on both sides of the first difference.
    let error =
        golden::check_bytes(&path, b"\x00\xfe\r\n\x80", Mode::Compare).expect_err("differs");
    let text = error.to_string();
    assert!(
        text.contains("at byte 1: golden has 0xff, output has 0xfe (5 golden bytes, 5 produced)"),
        "{text}"
    );
    let error = golden::check_bytes(&path, &golden[..4], Mode::Compare).expect_err("differs");
    let text = error.to_string();
    assert!(
        text.contains("at byte 4: golden has 0x80, output has none (the data ends) (5 golden bytes, 4 produced)"),
        "{text}"
    );
}

#[test]
fn a_missing_byte_golden_fails_and_names_the_regeneration_switch() {
    let dir = temp_dir!().expect("temp dir");
    let error = golden::check_bytes(&dir.path().join("absent.bin"), b"x", Mode::Compare)
        .expect_err("a missing golden is a failure");
    assert!(matches!(error, GoldenError::Read { .. }));
    assert!(
        error
            .to_string()
            .contains("PURRDF_REGENERATE_GOLDEN=1 writes it"),
        "{error}"
    );
}

#[test]
fn byte_regeneration_writes_the_bytes_verbatim_and_creates_directories() {
    let dir = temp_dir!().expect("temp dir");
    let path = dir.path().join("nested/deeper/out.bin");
    let produced: &[u8] = b"\xff\xfe\x00\r\n\x80\x81";
    golden::check_bytes(&path, produced, Mode::Regenerate).expect("regenerates");
    assert_eq!(std::fs::read(&path).expect("read back"), produced);
    golden::check_bytes(&path, produced, Mode::Compare)
        .expect("the regenerated golden compares equal");
}

#[test]
fn the_bytes_macro_resolves_the_calling_crates_golden_directory() {
    // `tests/golden/macro_probe.bin` is this crate's own golden: a NUL, a
    // byte that is not UTF-8, a CRLF, and a lone continuation byte.
    purrdf_testkit::assert_golden_bytes!("macro_probe.bin", b"\x00\xff\r\n\x80");
}
