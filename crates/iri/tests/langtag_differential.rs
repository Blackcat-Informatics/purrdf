// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Differential acceptance test against a frozen oracle table.
//!
//! `crates/iri/src/langtag.rs` claims its accepted language is identical to
//! that of the third-party parser it replaced. That claim used to be prose:
//! the predecessor is banned from the workspace, so nothing could make the
//! claim go red if it were false. This test is the falsifier.
//!
//! `langtag_differential_vectors.txt` freezes one accept/reject verdict per
//! candidate tag. The inputs were generated independently from the RFC 5646
//! §2.1 ABNF, the closed §2.2.8 grandfathered list and the Appendix A worked
//! examples; the verdicts were labelled once by the replaced dependency run as
//! an external oracle, and then frozen. The fixture's own header records both
//! provenance halves; `crates/iri/tests/PROVENANCE.md` carries the audit row.
//!
//! A disagreement here is a **parser** defect. It is never fixed by editing the
//! table — which is why the table carries a digest this test recomputes.

use purrdf_iri::langtag::is_well_formed;
use purrdf_testkit::vectors::sha256_hex;
use std::fmt::Write as _;

/// The frozen table. `include_str!` binds it at compile time, so a missing or
/// renamed fixture is a build failure rather than a silently-empty pass.
const FIXTURE: &str = include_str!("langtag_differential_vectors.txt");

/// The encoding of the empty input (a bare empty field would leave a line
/// ending in its separator tab, which whitespace trimming eats).
const EMPTY_INPUT: &str = "\\0";

/// How many mismatches a failure message lists before summarizing the rest.
const MISMATCHES_SHOWN: usize = 25;

/// One frozen vector.
#[derive(Debug)]
struct Vector {
    /// 1-based line number in the fixture, for failure messages.
    line: usize,
    /// The decoded candidate tag.
    input: String,
    /// `true` when the oracle accepted this input.
    expected: bool,
}

#[test]
fn parser_matches_the_frozen_oracle_verdicts() {
    let vectors = vectors();
    let mut mismatches = Vec::new();
    for vector in &vectors {
        let actual = is_well_formed(&vector.input);
        if actual != vector.expected {
            mismatches.push(format!(
                "  line {}: input {:?}\n      frozen oracle verdict: {}\n      purrdf_iri verdict:    {}",
                vector.line,
                vector.input,
                verdict_name(vector.expected),
                verdict_name(actual),
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} of {} frozen vectors disagree with `purrdf_iri::langtag::is_well_formed`.\n\
         This is a PARSER defect: the acceptance language diverged from the frozen\n\
         oracle table. Do NOT edit the table to match the parser.\n{}",
        mismatches.len(),
        vectors.len(),
        render(&mismatches),
    );
}

#[test]
fn fixture_is_intact() {
    let vectors = vectors();
    let accepted = vectors.iter().filter(|vector| vector.expected).count();
    let rejected = vectors.len() - accepted;

    assert_eq!(
        header_value("vector-count"),
        vectors.len().to_string(),
        "header vector-count must match the body"
    );
    assert_eq!(
        header_value("accept-count"),
        accepted.to_string(),
        "header accept-count must match the body"
    );
    assert_eq!(
        header_value("reject-count"),
        rejected.to_string(),
        "header reject-count must match the body"
    );
    assert_eq!(
        header_value("body-sha256"),
        sha256_hex(body().as_bytes()),
        "the frozen table's digest must match its body; a verdict was edited, \
         or the table was regenerated without updating the header"
    );

    for pair in vectors.windows(2) {
        assert!(
            pair[0].input < pair[1].input,
            "vectors must be sorted and unique: {:?} then {:?} at line {}",
            pair[0].input,
            pair[1].input,
            pair[1].line
        );
    }
}

#[test]
fn fixture_covers_both_verdicts_at_scale() {
    // A table that drifted to all-reject would still pass the differential
    // assert while testing nothing about acceptance. Pin both sides.
    let vectors = vectors();
    let accepted = vectors.iter().filter(|vector| vector.expected).count();
    assert!(
        vectors.len() >= 2_000,
        "the sweep must stay broad; got {} vectors",
        vectors.len()
    );
    assert!(
        accepted >= 500 && vectors.len() - accepted >= 500,
        "both verdicts must stay well represented; got {accepted} accept / {} reject",
        vectors.len() - accepted
    );
}

#[test]
fn input_encoding_round_trips() {
    for raw in [
        "",
        "en-US",
        "a b",
        "\\",
        "\\0",
        "en\u{9}US",
        "en-\u{fc}",
        " ",
    ] {
        assert_eq!(
            decode(&encode(raw)),
            raw,
            "encoding must round-trip {raw:?}"
        );
    }
    assert_eq!(encode(""), EMPTY_INPUT);
    assert_eq!(
        encode("\\0"),
        "\\\\0",
        "a literal backslash-zero is not EMPTY"
    );
}

/// Every vector line of the fixture, in file order.
fn vectors() -> Vec<Vector> {
    let vectors: Vec<Vector> = FIXTURE
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.starts_with('#'))
        .map(|(index, line)| {
            let number = index + 1;
            let (verdict, encoded) = line
                .split_once('\t')
                .unwrap_or_else(|| panic!("line {number} is not `<verdict>\\t<input>`: {line:?}"));
            let expected = match verdict {
                "accept" => true,
                "reject" => false,
                other => panic!("line {number} has unknown verdict {other:?}"),
            };
            Vector {
                line: number,
                input: decode(encoded),
                expected,
            }
        })
        .collect();
    assert!(!vectors.is_empty(), "the frozen table must not be empty");
    vectors
}

/// The body the digest covers: every non-header line, newline terminated.
fn body() -> String {
    FIXTURE
        .lines()
        .filter(|line| !line.starts_with('#'))
        .fold(String::new(), |mut body, line| {
            body.push_str(line);
            body.push('\n');
            body
        })
}

/// The value of a `# <key>: <value>` header field.
fn header_value(key: &str) -> &'static str {
    let prefix = format!("# {key}: ");
    let found = FIXTURE
        .lines()
        .find_map(|line| line.strip_prefix(prefix.as_str()));
    let Some(value) = found else {
        panic!("the fixture header must declare `{key}`")
    };
    value
}

/// `"accept"` / `"reject"`, for failure messages.
fn verdict_name(accepted: bool) -> &'static str {
    if accepted { "accept" } else { "reject" }
}

/// Renders at most [`MISMATCHES_SHOWN`] mismatches, then a tail count.
fn render(mismatches: &[String]) -> String {
    let mut rendered =
        mismatches
            .iter()
            .take(MISMATCHES_SHOWN)
            .fold(String::new(), |mut text, entry| {
                text.push('\n');
                text.push_str(entry);
                text
            });
    if mismatches.len() > MISMATCHES_SHOWN {
        let _ = write!(
            rendered,
            "\n  ... and {} more",
            mismatches.len() - MISMATCHES_SHOWN
        );
    }
    rendered
}

/// Encodes an input for a fixture line (see the fixture's ENCODING section).
fn encode(input: &str) -> String {
    if input.is_empty() {
        return EMPTY_INPUT.to_owned();
    }
    let mut encoded = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '\\' => encoded.push_str("\\\\"),
            ' ' => encoded.push_str("\\x20"),
            control if control.is_control() && control.is_ascii() => {
                encoded.push_str("\\x");
                purrdf_hash::hex::encode_into(&[control as u8], &mut encoded);
            }
            other => encoded.push(other),
        }
    }
    encoded
}

/// Decodes a fixture line's input field. A malformed escape is a corrupt
/// committed fixture, so it panics rather than degrading.
fn decode(encoded: &str) -> String {
    if encoded == EMPTY_INPUT {
        return String::new();
    }
    let mut decoded = String::with_capacity(encoded.len());
    let mut characters = encoded.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        match characters.next() {
            Some('\\') => decoded.push('\\'),
            Some('x') => {
                let high = characters.next().expect("\\x escape has two hex digits");
                let low = characters.next().expect("\\x escape has two hex digits");
                let digits: String = [high, low].into_iter().collect();
                let byte = purrdf_hash::hex::decode(&digits).expect("\\x escape is hexadecimal");
                decoded.push(char::from(byte[0]));
            }
            other => panic!("unknown escape \\{other:?} in fixture field {encoded:?}"),
        }
    }
    decoded
}
