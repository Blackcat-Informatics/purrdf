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
    // The real tools persist the existing alias manifest, retain its revision,
    // and reuse one native prefix index across a buffered batch.
    use purrdf_core::content_alias::{AliasScope, ScopeId};
    use purrdf_hash::{hex::Digest32, word_id::Reference};
    use purrdf_lex::allocation::Resident;
    let directory = purrdf_testkit::temp_dir!().unwrap();
    let manifest = directory.path().join("snapshot.bin");
    let manifest_path = manifest.to_str().unwrap();
    let scope = ScopeId::new(Digest32::new([7; 32]));
    let scope_text = scope.digest().to_string();
    let mut close = [0; 32];
    close[31] = 1;
    let values = [
        Digest32::new([0x5a; 32]),
        Digest32::new([0; 32]),
        Digest32::new(close),
    ];
    let mut aliases = AliasScope::new(scope, Resident).unwrap();
    aliases.extend(scope, &values).unwrap();
    let revision = aliases
        .presentation(scope)
        .unwrap()
        .revision
        .digest()
        .to_string();
    let rows = format!(
        "{}\n{}\n{}\n{}\n",
        values[0], values[1], values[2], values[0]
    );
    let planned = run_with_input(
        &[
            "word-id",
            "plan",
            "--scope",
            &scope_text,
            "--output",
            manifest_path,
        ],
        rows.as_bytes(),
    );
    assert!(planned.status.success(), "{}", stderr(&planned));
    let snapshot = std::fs::read(&manifest).unwrap();
    let expected = aliases.export(scope).unwrap();
    assert_eq!(snapshot.as_slice(), &*expected);
    expected.release().unwrap();
    assert!(stderr(&planned).contains(&format!("revision {revision}")));
    assert!(
        stderr(&planned)
            .contains("identities 3\nuniform_payload_words 22\nuniform_checked_words 23")
    );
    assert!(stderr(&planned).contains("per_entry_payload_words 1 count 1"));
    let binding = [
        "--manifest",
        manifest_path,
        "--scope",
        &scope_text,
        "--revision",
        &revision,
    ];
    let invoke = |operation: &[&str], input: &[u8]| {
        let mut arguments = vec!["word-id"];
        arguments.extend_from_slice(operation);
        arguments.extend_from_slice(&binding);
        run_with_input(&arguments, input)
    };
    let shortened = invoke(
        &["shorten", "--from", "hex", "--length", "per-entry"],
        format!("{}\n", values[0]).as_bytes(),
    );
    assert!(shortened.status.success(), "{}", stderr(&shortened));
    assert!(stdout_utf8(&shortened).starts_with("b3p1: "));
    let expanded = invoke(&["expand", "--to", "raw"], &shortened.stdout);
    assert!(expanded.status.success(), "{}", stderr(&expanded));
    assert_eq!(expanded.stdout, values[0].as_bytes());
    let inspected = invoke(&["inspect-scoped"], &shortened.stdout);
    assert!(inspected.status.success(), "{}", stderr(&inspected));
    assert!(stdout_utf8(&inspected).contains(&format!(
        "digest {}\npayload_words 1\ncheck_words 1\ndisplayed_words 2",
        values[0]
    )));
    let selected = invoke(
        &[
            "batch-scoped",
            "--from",
            "hex-lines",
            "--to",
            "word-lines",
            "--length",
            "per-entry",
        ],
        rows.as_bytes(),
    );
    assert!(selected.status.success(), "{}", stderr(&selected));
    let restored = invoke(
        &[
            "batch-scoped",
            "--from",
            "word-lines",
            "--to",
            "hex-lines",
            "--length",
            "uniform",
        ],
        &selected.stdout,
    );
    assert!(restored.status.success(), "{}", stderr(&restored));
    assert_eq!(restored.stdout, rows.as_bytes());
    let selector = Reference::from_digest(values[1], 1)
        .unwrap()
        .to_text()
        .unwrap();
    let ambiguous = invoke(&["expand", "--to", "hex"], selector.as_bytes());
    assert_eq!(ambiguous.status.code(), Some(1));
    assert!(stderr(&ambiguous).contains("ambiguous"));
    let unknown = Reference::from_digest(Digest32::new([0xff; 32]), 1)
        .unwrap()
        .to_text()
        .unwrap();
    let refused = invoke(&["expand", "--to", "hex"], unknown.as_bytes());
    assert_eq!(refused.status.code(), Some(1));
    assert!(stderr(&refused).contains("unknown"));
    let mixed = format!("invalid\n{unknown}\n{}", stdout_utf8(&shortened));
    let partially = invoke(
        &[
            "batch-scoped",
            "--from",
            "word-lines",
            "--to",
            "hex-lines",
            "--length",
            "uniform",
        ],
        mixed.as_bytes(),
    );
    assert_eq!(partially.status.code(), Some(1));
    assert_eq!(stdout_utf8(&partially), format!("{}\n", values[0]));
    assert!(stderr(&partially).contains("batch refused 2 records"));
    let missing_revision = run_with_input(
        &[
            "word-id",
            "expand",
            "--to",
            "hex",
            "--manifest",
            manifest_path,
            "--scope",
            &scope_text,
        ],
        &shortened.stdout,
    );
    assert_eq!(missing_revision.status.code(), Some(2));
    let wrong_scope = "08".repeat(32);
    let wrong = run_with_input(
        &[
            "word-id",
            "expand",
            "--to",
            "hex",
            "--manifest",
            manifest_path,
            "--scope",
            &wrong_scope,
            "--revision",
            &revision,
        ],
        &shortened.stdout,
    );
    assert_eq!(wrong.status.code(), Some(1));
    assert!(stderr(&wrong).contains("scope"));
    #[cfg(unix)]
    {
        let hardlink = directory.path().join("snapshot-hardlink.bin");
        std::fs::hard_link(&manifest, &hardlink).unwrap();
        for destination in [manifest_path, hardlink.to_str().unwrap()] {
            let refused = invoke(
                &["expand", "--to", "hex", "--output", destination],
                &shortened.stdout,
            );
            assert_eq!(refused.status.code(), Some(2));
            assert_eq!(std::fs::read(&manifest).unwrap(), snapshot);
            let canonical = invoke(
                &[
                    "expand",
                    "--to",
                    "hex",
                    "--canonical-search",
                    "unlimited",
                    "--canonicalization-receipts",
                    destination,
                ],
                &shortened.stdout,
            );
            assert_eq!(canonical.status.code(), Some(2));
            assert_eq!(std::fs::read(&manifest).unwrap(), snapshot);
        }
        let source = directory.path().join("scoped-input.txt");
        for invalid_first in [false, true] {
            let contents = format!(
                "{}{}",
                if invalid_first { "invalid\n" } else { "" },
                stdout_utf8(&shortened).repeat(16_384)
            );
            std::fs::write(&source, contents).unwrap();
            let mut child = support::purrdf()
                .args([
                    "word-id",
                    "batch-scoped",
                    "--from",
                    "word-lines",
                    "--to",
                    "hex-lines",
                    "--length",
                    "uniform",
                    "--input",
                    source.to_str().unwrap(),
                ])
                .args(binding)
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
    // Overwriting the same filename with a grown snapshot cannot retarget an
    // old envelope. Validation also precedes opening a distinct output.
    aliases.extend(scope, &[Digest32::new([0x5b; 32])]).unwrap();
    let grown = aliases.export(scope).unwrap();
    std::fs::write(&manifest, &*grown).unwrap();
    grown.release().unwrap();
    let output = directory.path().join("preserved.txt");
    std::fs::write(&output, b"preserve").unwrap();
    let stale = invoke(
        &[
            "expand",
            "--to",
            "hex",
            "--output",
            output.to_str().unwrap(),
        ],
        &shortened.stdout,
    );
    assert_eq!(stale.status.code(), Some(1));
    assert!(stderr(&stale).contains("revision"));
    assert_eq!(std::fs::read(&output).unwrap(), b"preserve");
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
