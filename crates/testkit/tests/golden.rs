// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Goldens compare byte for byte — CRLF, trailing whitespace and the final
//! newline included — and regeneration writes the produced bytes verbatim.
//!
//! These run where a test can create a path; wasm32-unknown-unknown has no file
//! system, and the scratch-space types do not exist there.

#![cfg(not(target_arch = "wasm32"))]

use std::process::Command;

use purrdf_testkit::golden::{self, GoldenError, Mode};
use purrdf_testkit::temp_dir;

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

/// Asserts the mode this process reads, when the parent test below asks.
#[test]
fn mode_probe() {
    if let Some(expected) = std::env::var_os("PURRDF_TESTKIT_EXPECT_MODE") {
        assert_eq!(
            Some(format!("{:?}", Mode::from_env()).as_str()),
            expected.to_str()
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
        (Some(""), "Compare"),
        (Some(" 1"), "Compare"),
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

/// A variable no test run sets, so a binary golden under it always compares.
const UNSET_SWITCH: &str = "PURRDF_TESTKIT_NEVER_SET_REGENERATE";

#[test]
fn an_identical_binary_golden_passes_and_a_difference_names_its_offset() {
    let dir = temp_dir!().expect("temp dir");
    let path = dir.path().join("artifact.bin");
    let golden_bytes = [0x00, 0xFF, 0x80, b'\r', b'\n', 0x7F];
    std::fs::write(&path, golden_bytes).expect("write golden");

    golden::check_bytes(&path, &golden_bytes, UNSET_SWITCH).expect("identical bytes pass");
    for (different, offset) in [
        (&[0x00, 0xFF, 0x81, b'\r', b'\n', 0x7F][..], 2),
        (&[0x00, 0xFF, 0x80, b'\r', b'\n'][..], 5),
        (&[0x00, 0xFF, 0x80, b'\r', b'\n', 0x7F, 0x00][..], 6),
        (&[][..], 0),
    ] {
        let error = golden::check_bytes(&path, different, UNSET_SWITCH).expect_err("differs");
        match &error {
            GoldenError::BytesDiffer {
                env_var,
                expected_len,
                actual_len,
                first_difference,
                ..
            } => {
                assert_eq!(*env_var, UNSET_SWITCH);
                assert_eq!(*expected_len, golden_bytes.len());
                assert_eq!(*actual_len, different.len());
                assert_eq!(*first_difference, offset, "{different:?}");
            }
            other => panic!("a byte difference, got {other:?}"),
        }
        assert!(
            error.to_string().contains(&format!("at byte {offset} ")),
            "{error}"
        );
        assert!(
            error
                .to_string()
                .contains(&format!("{UNSET_SWITCH}=1 rewrites it")),
            "{error}"
        );
    }
}

#[test]
fn a_missing_binary_golden_fails_and_names_its_own_switch() {
    let dir = temp_dir!().expect("temp dir");
    let error = golden::check_bytes(&dir.path().join("absent.bin"), b"x", UNSET_SWITCH)
        .expect_err("a missing golden is a failure");
    assert!(matches!(error, GoldenError::Read { env_var, .. } if env_var == UNSET_SWITCH));
    assert!(
        error
            .to_string()
            .contains(&format!("{UNSET_SWITCH}=1 writes it")),
        "{error}"
    );
}

#[test]
fn a_text_check_against_a_golden_that_is_not_utf8_reports_a_byte_difference() {
    let dir = temp_dir!().expect("temp dir");
    let path = dir.path().join("not-text.txt");
    std::fs::write(&path, [b'a', 0xFF]).expect("write golden");
    let error = golden::check(&path, "ab", Mode::Compare).expect_err("differs");
    assert!(
        matches!(
            error,
            GoldenError::BytesDiffer {
                first_difference: 1,
                ..
            }
        ),
        "{error:?}"
    );
}

/// The switch a binary golden's regeneration probe reads.
const BYTES_SWITCH: &str = "PURRDF_TESTKIT_BYTES_REGENERATE";

/// Checks a binary golden under [`BYTES_SWITCH`], when the parent test below
/// asks, and asserts whether the check passed.
#[test]
fn bytes_probe() {
    if let Some(path) = std::env::var_os("PURRDF_TESTKIT_BYTES_PATH") {
        let passed = golden::check_bytes(path.as_ref(), b"\x00\xFFbinary", BYTES_SWITCH).is_ok();
        let expect_pass = std::env::var_os("PURRDF_TESTKIT_EXPECT_PASS").is_some_and(|v| v == "1");
        assert_eq!(passed, expect_pass);
    }
}

#[test]
fn a_binary_golden_regenerates_only_when_its_own_switch_is_one() {
    let exe = std::env::current_exe().expect("this test binary");
    let dir = temp_dir!().expect("temp dir");
    for (index, (value, regenerates)) in [
        (Some("1"), true),
        (Some("0"), false),
        (Some(""), false),
        (Some("yes"), false),
        (None, false),
    ]
    .into_iter()
    .enumerate()
    {
        let path = dir.path().join(format!("case-{index}/golden.bin"));
        let mut child = Command::new(&exe);
        child
            .args(["--exact", "bytes_probe", "--test-threads=1"])
            .env("PURRDF_TESTKIT_BYTES_PATH", &path)
            .env(
                "PURRDF_TESTKIT_EXPECT_PASS",
                if regenerates { "1" } else { "0" },
            )
            // The general switch does not regenerate a golden with a switch of its own.
            .env(golden::REGENERATE_ENV, "1");
        match value {
            Some(value) => child.env(BYTES_SWITCH, value),
            None => child.env_remove(BYTES_SWITCH),
        };
        let output = child.output().expect("run the probe");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{value:?}: {stdout}");
        assert!(
            stdout.contains("1 passed"),
            "{value:?}: the probe ran: {stdout}"
        );
        if regenerates {
            assert_eq!(
                std::fs::read(&path).expect("regenerated"),
                b"\x00\xFFbinary",
                "{value:?}"
            );
        } else {
            assert!(!path.exists(), "{value:?} wrote the golden");
        }
    }
}
