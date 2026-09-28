// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen differential vectors: a recorded file replays, any one-byte edit to
//! it is caught by its own digest, a disagreeing implementation is named at
//! its first differing record, and the field encoding round-trips with one
//! spelling per value.

use purrdf_testkit::vectors::{
    self, Recorder, VectorError, VectorFile, decode_bytes, decode_str, encode_bytes, encode_str,
};

/// A small recorded file: input, then the answer of `str::to_uppercase`.
fn recorded() -> String {
    let mut recorder = Recorder::new();
    recorder
        .comment("Uppercase answers, recorded as a fixture.")
        .expect("comment")
        .comment("")
        .expect("blank comment")
        .header("oracle", "str::to_uppercase")
        .expect("header");
    for input in ["", "abc", "a b", "#hash", "tab\there", "é", "back\\slash"] {
        recorder
            .record(&[encode_str(input), encode_str(&input.to_uppercase())])
            .expect("record");
    }
    recorder.render()
}

#[test]
fn a_recorded_file_parses_and_replays() {
    let text = recorded();
    let file = VectorFile::parse(&text).expect("a freshly recorded file verifies");
    assert_eq!(file.header("oracle"), Some("str::to_uppercase"));
    assert_eq!(file.header(vectors::COUNT_KEY), Some("7"));
    assert_eq!(file.records().len(), 7);
    let replayed = file
        .replay(1, |inputs| {
            let input = decode_str(inputs[0]).expect("decodes");
            vec![encode_str(&input.to_uppercase())]
        })
        .expect("the same implementation agrees");
    assert_eq!(replayed, 7);
}

#[test]
fn the_digest_is_the_sha256_of_the_body_lines() {
    let text = recorded();
    let body: String = text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .flat_map(|line| [line, "\n"])
        .collect();
    let file = VectorFile::parse(&text).expect("verifies");
    assert_eq!(
        file.header(vectors::DIGEST_KEY),
        Some(vectors::sha256_hex(body.as_bytes()).as_str())
    );
    // FIPS 180-4 known answers, so the digest is the real SHA-256.
    assert_eq!(
        vectors::sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        vectors::sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn every_one_byte_edit_of_the_body_fails_the_self_hash_check() {
    let text = recorded();
    let body_start = text
        .match_indices('\n')
        .map(|(index, _)| index + 1)
        .find(|&index| !text[index..].starts_with('#'))
        .expect("a body");
    let bytes = text.as_bytes();
    let mut checked = 0;
    for position in body_start..bytes.len() {
        if bytes[position] == b'\n' || !bytes[position].is_ascii() {
            continue;
        }
        let mut edited = bytes.to_vec();
        edited[position] = if bytes[position] == b'A' { b'B' } else { b'A' };
        let edited = String::from_utf8(edited).expect("an ASCII substitution stays UTF-8");
        let error = VectorFile::parse(&edited).expect_err("an edited record is caught");
        assert!(
            matches!(error, VectorError::DigestMismatch { .. }),
            "edit at byte {position}: {error}"
        );
        checked += 1;
    }
    assert!(checked > 40, "the body was swept ({checked} edits)");
}

#[test]
fn a_removed_record_fails_the_count_and_an_unedited_file_passes() {
    let text = recorded();
    let last_line_start = text.trim_end_matches('\n').rfind('\n').expect("lines") + 1;
    let truncated = &text[..last_line_start];
    assert!(matches!(
        VectorFile::parse(truncated),
        Err(VectorError::CountMismatch {
            declared: 7,
            actual: 6
        })
    ));
    VectorFile::parse(&text).expect("the unedited neighbour verifies");
}

#[test]
fn a_missing_digest_header_is_refused() {
    let text = recorded();
    let without: String = text
        .lines()
        .filter(|line| !line.starts_with("# body-sha256: "))
        .flat_map(|line| [line, "\n"])
        .collect();
    assert_eq!(
        VectorFile::parse(&without).expect_err("refused"),
        VectorError::MissingHeader(vectors::DIGEST_KEY)
    );
}

#[test]
fn a_disagreement_names_the_first_differing_record() {
    let text = recorded();
    let file = VectorFile::parse(&text).expect("verifies");
    // An "implementation" that forgets to uppercase non-ASCII.
    let mismatch = file
        .replay(1, |inputs| {
            let input = decode_str(inputs[0]).expect("decodes");
            vec![encode_str(&input.to_ascii_uppercase())]
        })
        .expect_err("the accented record differs");
    assert_eq!(mismatch.mismatches(), 1);
    let first_body_line = text
        .lines()
        .position(|line| !line.starts_with('#'))
        .expect("body")
        + 1;
    assert_eq!(mismatch.first_line(), first_body_line + 5);
    let report = mismatch.to_string();
    assert!(
        report.contains("1 of 7 frozen vectors disagree"),
        "{report}"
    );
    assert!(report.contains("\"É\""), "{report}");
    assert!(report.contains("\"é\""), "{report}");
}

#[test]
fn the_field_encoding_round_trips_with_one_spelling() {
    for text in [
        "",
        "plain",
        " ",
        "\\",
        "\\0",
        "#",
        "a\tb\nc\r",
        "é\u{7f}",
        "\u{0}",
    ] {
        let encoded = encode_str(text);
        assert!(!encoded.contains(['\t', '\n', '\r', ' ']), "{encoded:?}");
        assert!(!encoded.starts_with('#'), "{encoded:?}");
        assert_eq!(decode_str(&encoded).expect("decodes"), text, "{encoded:?}");
    }
    assert_eq!(encode_str(""), vectors::EMPTY_FIELD);
    assert_eq!(
        encode_str("\\0"),
        "\\\\0",
        "a literal backslash-zero is not the empty field"
    );
    let bytes: Vec<u8> = (0..=255).collect();
    let encoded = encode_bytes(&bytes);
    assert!(encoded.is_ascii());
    assert_eq!(decode_bytes(&encoded).expect("decodes"), bytes);
}

#[test]
fn a_second_spelling_of_a_field_is_refused_and_the_canonical_one_accepted() {
    for (bad, good) in [
        ("a b", "a\\x20b"),
        ("\\x41", "A"),
        ("\\x5c", "\\\\"),
        ("\\xE9", "\\xe9"),
        ("", "\\0"),
        ("a\\0", "a0"),
        ("\\q", "q"),
        ("\\x4", "\\x7f"),
    ] {
        assert!(decode_bytes(bad).is_err(), "{bad:?} must be refused");
        decode_bytes(good).unwrap_or_else(|error| panic!("{good:?} must be accepted: {error}"));
    }
    // A text field writes non-ASCII verbatim; a byte-string field escapes it.
    assert!(decode_str("\\xc3\\xa9").is_err());
    assert_eq!(decode_str("é").expect("verbatim"), "é");
    assert!(decode_bytes("é").is_err());
    assert_eq!(decode_bytes("\\xc3\\xa9").expect("escaped"), "é".as_bytes());
}

#[test]
fn the_recorder_refuses_unencoded_fields_and_accepts_encoded_ones() {
    let mut recorder = Recorder::new();
    assert!(recorder.record(&["raw field"]).is_err());
    assert!(recorder.record(&["#comment-like"]).is_err());
    assert!(recorder.record::<&str>(&[]).is_err());
    assert!(recorder.header(vectors::COUNT_KEY, "1").is_err());
    assert!(recorder.header("Bad Key", "1").is_err());
    recorder
        .record(&[encode_str("raw field"), encode_bytes(&[0xff, b'#'])])
        .expect("encoded fields are accepted");
    let text = recorder.render();
    let file = VectorFile::parse(&text).expect("verifies");
    assert_eq!(file.records()[0].fields, ["raw\\x20field", "\\xff\\x23"]);
}
