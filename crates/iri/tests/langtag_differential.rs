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
fn sha256_matches_fips_known_answers() {
    // The digest above is only tamper evidence if this implementation is the
    // real SHA-256. FIPS 180-4 / RFC 6234 known answers.
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
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
            control if control.is_control() && u32::from(control) < 0x80 => {
                let _ = write!(encoded, "\\x{:02x}", u32::from(control));
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
                let byte = u8::from_str_radix(&digits, 16).expect("\\x escape is hexadecimal");
                decoded.push(char::from(byte));
            }
            other => panic!("unknown escape \\{other:?} in fixture field {encoded:?}"),
        }
    }
    decoded
}

/// The 64 round constants of FIPS 180-4 §4.2.2.
const ROUND_CONSTANTS: [u32; 64] = [
    0x428a_2f98,
    0x7137_4491,
    0xb5c0_fbcf,
    0xe9b5_dba5,
    0x3956_c25b,
    0x59f1_11f1,
    0x923f_82a4,
    0xab1c_5ed5,
    0xd807_aa98,
    0x1283_5b01,
    0x2431_85be,
    0x550c_7dc3,
    0x72be_5d74,
    0x80de_b1fe,
    0x9bdc_06a7,
    0xc19b_f174,
    0xe49b_69c1,
    0xefbe_4786,
    0x0fc1_9dc6,
    0x240c_a1cc,
    0x2de9_2c6f,
    0x4a74_84aa,
    0x5cb0_a9dc,
    0x76f9_88da,
    0x983e_5152,
    0xa831_c66d,
    0xb003_27c8,
    0xbf59_7fc7,
    0xc6e0_0bf3,
    0xd5a7_9147,
    0x06ca_6351,
    0x1429_2967,
    0x27b7_0a85,
    0x2e1b_2138,
    0x4d2c_6dfc,
    0x5338_0d13,
    0x650a_7354,
    0x766a_0abb,
    0x81c2_c92e,
    0x9272_2c85,
    0xa2bf_e8a1,
    0xa81a_664b,
    0xc24b_8b70,
    0xc76c_51a3,
    0xd192_e819,
    0xd699_0624,
    0xf40e_3585,
    0x106a_a070,
    0x19a4_c116,
    0x1e37_6c08,
    0x2748_774c,
    0x34b0_bcb5,
    0x391c_0cb3,
    0x4ed8_aa4a,
    0x5b9c_ca4f,
    0x682e_6ff3,
    0x748f_82ee,
    0x78a5_636f,
    0x84c8_7814,
    0x8cc7_0208,
    0x90be_fffa,
    0xa450_6ceb,
    0xbef9_a3f7,
    0xc671_78f2,
];

/// The lowercase hex SHA-256 digest of `data`.
///
/// First-party because `purrdf-iri` is a zero-dependency crate: a `sha2`
/// dev-dependency here would be the only third-party name in its manifest that
/// the fixture gate needs, and the algorithm is 40 lines. Proven against the
/// FIPS 180-4 known answers by `sha256_matches_fips_known_answers`.
fn sha256_hex(data: &[u8]) -> String {
    let mut state: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    let bit_length = u64::try_from(data.len())
        .expect("input length fits in u64")
        .wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());
    let (blocks, remainder) = message.as_chunks::<64>();
    debug_assert!(
        remainder.is_empty(),
        "padding makes the length a multiple of 64"
    );
    for block in blocks {
        compress(&mut state, block);
    }
    state
        .iter()
        .fold(String::with_capacity(64), |mut hex, word| {
            let _ = write!(hex, "{word:08x}");
            hex
        })
}

/// One 64-byte block of the FIPS 180-4 §6.2.2 compression function.
fn compress(state: &mut [u32; 8], block: &[u8; 64]) {
    let mut schedule = [0u32; 64];
    let (words, _) = block.as_chunks::<4>();
    for (slot, bytes) in schedule.iter_mut().zip(words) {
        *slot = u32::from_be_bytes(*bytes);
    }
    for index in 16..64 {
        let prev15 = schedule[index - 15];
        let prev2 = schedule[index - 2];
        let sigma0 = prev15.rotate_right(7) ^ prev15.rotate_right(18) ^ (prev15 >> 3);
        let sigma1 = prev2.rotate_right(17) ^ prev2.rotate_right(19) ^ (prev2 >> 10);
        schedule[index] = schedule[index - 16]
            .wrapping_add(sigma0)
            .wrapping_add(schedule[index - 7])
            .wrapping_add(sigma1);
    }

    // `work` holds a..h at indices 0..8; the per-round shift h<-g<-...<-a is a
    // rotation of that array, which keeps the round body free of eight renames.
    let mut work = *state;
    for (constant, word) in ROUND_CONSTANTS.iter().zip(schedule) {
        let e = work[4];
        let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choose = (e & work[5]) ^ (!e & work[6]);
        let temp1 = work[7]
            .wrapping_add(sum1)
            .wrapping_add(choose)
            .wrapping_add(*constant)
            .wrapping_add(word);
        let a = work[0];
        let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & work[1]) ^ (a & work[2]) ^ (work[1] & work[2]);
        let temp2 = sum0.wrapping_add(majority);
        work.rotate_right(1);
        work[4] = work[4].wrapping_add(temp1);
        work[0] = temp1.wrapping_add(temp2);
    }

    for (slot, value) in state.iter_mut().zip(work) {
        *slot = slot.wrapping_add(value);
    }
}
