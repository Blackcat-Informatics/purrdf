// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Real full-ID conversion, raw framing, inspection and ordered batch refusal.
mod support;
use support::{run_with_input, stderr, stdout_utf8};

#[test]
fn explicit_full_formats_and_batch_records_preserve_original_digests() {
    let raw: Vec<_> = (0..32).collect();
    let encoded = run_with_input(&["word-id", "encode", "--from", "raw"], &raw);
    assert!(encoded.status.success(), "{}", stderr(&encoded));
    let words = stdout_utf8(&encoded);
    assert!(words.starts_with("b3w1: "));
    assert_eq!(words.trim_end().split(' ').count(), 24);
    let decoded = run_with_input(&["word-id", "decode", "--to", "raw"], &encoded.stdout);
    assert!(decoded.status.success(), "{}", stderr(&decoded));
    assert_eq!(decoded.stdout, raw);
    let inspected = run_with_input(&["word-id", "inspect", "--from", "words"], &encoded.stdout);
    assert!(inspected.status.success(), "{}", stderr(&inspected));
    assert!(
        stdout_utf8(&inspected).contains("payload_words 22\ncheck_words 1\ndisplayed_words 23")
    );
    let hex = purrdf_hash::hex::encode(&raw);
    assert!(stdout_utf8(&inspected).contains(&format!("digest {hex}")));
    let bad = run_with_input(
        &["word-id", "decode", "--to", "hex"],
        words.to_uppercase().as_bytes(),
    );
    assert_eq!(bad.status.code(), Some(1));
    assert_eq!(bad.stdout, [] as [u8; 0]);
    let mut invalid_hex = [b'0'; 64];
    invalid_hex[0] = b'g';
    invalid_hex[40] = 0xff;
    let first_digit = run_with_input(&["word-id", "encode", "--from", "hex"], &invalid_hex);
    assert_eq!(first_digit.status.code(), Some(1));
    assert_eq!(first_digit.stdout, [] as [u8; 0]);
    assert!(stderr(&first_digit).contains("byte 0"));
    // Cross the reader's chunk boundary: the refused record must be drained
    // without growing its retained buffer or consuming the following record.
    let batch = format!("{hex}\n{}\n{hex}\n", "x".repeat(9000));
    let converted = run_with_input(
        &[
            "word-id",
            "batch",
            "--from",
            "hex-lines",
            "--to",
            "word-lines",
        ],
        batch.as_bytes(),
    );
    assert_eq!(converted.status.code(), Some(1));
    assert_eq!(converted.stdout, encoded.stdout.repeat(2));
    assert!(stderr(&converted).contains("record 2, stream byte 65"));
    let recovered = run_with_input(
        &[
            "word-id",
            "batch",
            "--from",
            "word-lines",
            "--to",
            "hex-lines",
        ],
        &converted.stdout,
    );
    assert!(recovered.status.success(), "{}", stderr(&recovered));
    assert_eq!(stdout_utf8(&recovered), format!("{hex}\n{hex}\n"));
    #[cfg(unix)]
    {
        let directory = purrdf_testkit::temp_dir!().unwrap();
        let input = directory.path().join("input.txt");
        let alias = directory.path().join("alias.txt");
        std::fs::write(&input, batch.as_bytes()).unwrap();
        std::fs::hard_link(&input, &alias).unwrap();
        let refused = support::run(&[
            "word-id",
            "batch",
            "--from",
            "hex-lines",
            "--to",
            "word-lines",
            "--input",
            input.to_str().unwrap(),
            "--output",
            alias.to_str().unwrap(),
        ]);
        assert_eq!(refused.status.code(), Some(2));
        assert_eq!(std::fs::read(&input).unwrap(), batch.as_bytes());
        let distinct = directory.path().join("distinct.txt");
        std::fs::write(&distinct, b"previous output").unwrap();
        std::fs::write(&input, format!("{hex}\n")).unwrap();
        let success = support::run(&[
            "word-id",
            "batch",
            "--from",
            "hex-lines",
            "--to",
            "word-lines",
            "--input",
            input.to_str().unwrap(),
            "--output",
            distinct.to_str().unwrap(),
        ]);
        assert!(success.status.success(), "{}", stderr(&success));
        assert_eq!(std::fs::read(&distinct).unwrap(), encoded.stdout);
        assert_eq!(
            std::fs::read(&input).unwrap(),
            format!("{hex}\n").as_bytes()
        );
        // Close stdout on a file-backed input larger than pipe capacity. The
        // child must stop during production, preserving any earlier refusal.
        let many = format!("{hex}\n").repeat(16_384);
        for invalid_first in [false, true] {
            let contents = if invalid_first {
                format!("invalid\n{many}")
            } else {
                many.clone()
            };
            std::fs::write(&input, contents).unwrap();
            let mut child = support::purrdf()
                .args([
                    "word-id",
                    "batch",
                    "--from",
                    "hex-lines",
                    "--to",
                    "word-lines",
                    "--input",
                    input.to_str().unwrap(),
                ])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            drop(child.stdout.take());
            let result = child.wait_with_output().unwrap();
            assert_eq!(
                result.status.code(),
                Some(i32::from(invalid_first)),
                "{}",
                stderr(&result)
            );
            if invalid_first {
                assert!(stderr(&result).contains("batch refused 1 records"));
            }
        }
    }
}
