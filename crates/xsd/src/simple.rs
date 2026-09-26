// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The simple (non-numeric, non-temporal) XSD value spaces: `boolean` and `string`.
//!
//! `xsd:string`'s value space is its lexical space, so it has no dedicated parser
//! (the [`crate::parse`] entry maps it straight to [`crate::XsdValue::String`]).

use crate::datatype::XsdDatatype;
use crate::value::XsdError;

/// `xsd:boolean`: lexical space is `true | false | 1 | 0`; canonical is `true|false`.
pub fn parse_boolean(s: &str) -> Result<bool, XsdError> {
    match s {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(XsdError::InvalidLexical {
            datatype: XsdDatatype::Boolean,
            lexical: s.to_string(),
            reason: "expected one of: true, false, 1, 0",
        }),
    }
}

/// Apply the XSD `whiteSpace = replace` facet (the value space of `xsd:normalizedString`).
///
/// Per XSD 1.1 Part 2 §4.3.6, `replace` maps every occurrence of `#x9` (tab), `#xA`
/// (line feed), and `#xD` (carriage return) to a single `#x20` (space). No other
/// character — including other Unicode whitespace such as `U+00A0` — is touched, and
/// no collapsing or trimming is performed (that is the `collapse` facet's job).
///
/// The result has the same number of characters as the input.
#[must_use]
pub fn normalize_whitespace_replace(s: &str) -> String {
    // Chunked precheck: find the first trigger byte sixteen bytes at a time. With
    // none, the value is already its own normal form and is returned unchanged —
    // the common case, and one copy with no per-byte loop.
    let Some(first) = find_first_replaced(s.as_bytes()) else {
        return s.to_owned();
    };
    // Byte scan from the first trigger, whole clean runs copied with one
    // `push_str` each: the three trigger bytes are ASCII and every UTF-8
    // lead/continuation byte is >= 0x80, so slicing at a trigger index is always
    // a char boundary and the output is byte-identical to the per-char map (no
    // per-char push, exact-fit buffer).
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..first]);
    let mut run_start = first;
    for (i, b) in s.bytes().enumerate().skip(first) {
        if matches!(b, b'\t' | b'\n' | b'\r') {
            out.push_str(&s[run_start..i]);
            out.push(' ');
            run_start = i + 1;
        }
    }
    out.push_str(&s[run_start..]);
    out
}

/// Apply the XSD `whiteSpace = collapse` facet (the value space of `xsd:token`).
///
/// Per XSD 1.1 Part 2 §4.3.6, `collapse` first performs the `replace` facet (each
/// `#x9`/`#xA`/`#xD` becomes `#x20`), then collapses every run of contiguous `#x20`
/// spaces to a single space and strips all leading and trailing spaces. Only the
/// four XSD whitespace characters participate; other Unicode whitespace is preserved
/// verbatim (it is not part of the XSD `whiteSpace` facet).
#[must_use]
pub fn normalize_whitespace_collapse(s: &str) -> String {
    // Chunked precheck: a value with no tab, line feed or carriage return, no
    // leading or trailing space and no two adjacent spaces is already collapsed,
    // and is returned unchanged — the common case, and one copy with no per-byte
    // loop.
    if is_collapsed(s.as_bytes()) {
        return s.to_owned();
    }
    // Byte scan copying each non-whitespace run with one `push_str`: the four
    // XSD whitespace bytes are ASCII (never inside a multi-byte sequence), so run
    // boundaries are char boundaries and the output matches the per-char loop.
    let mut out = String::with_capacity(s.len());
    let mut pending_space = false;
    let mut run_start: Option<usize> = None;
    for (i, b) in s.bytes().enumerate() {
        if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
            if let Some(start) = run_start.take() {
                out.push_str(&s[start..i]);
            }
            pending_space = !out.is_empty();
        } else if run_start.is_none() {
            if pending_space {
                out.push(' ');
                pending_space = false;
            }
            run_start = Some(i);
        }
    }
    if let Some(start) = run_start {
        out.push_str(&s[start..]);
    }
    out
}

/// Bytes per precheck chunk: one 128-bit vector of bytes.
const CHUNK: usize = 16;

/// The bytes the `replace` facet rewrites, `#x9`, `#xA` and `#xD`, as inclusive
/// byte runs.
const REPLACED_RUNS: [(u8, u8); 2] = [(b'\t', b'\n'), (b'\r', b'\r')];
/// The byte the `collapse` facet additionally folds, `#x20`, as a byte run.
const SPACE_RUNS: [(u8, u8); 1] = [(b' ', b' ')];

/// Whether `b` falls in one of `runs`: one wrapping subtraction and one unsigned
/// comparison per run, OR-ed with no early exit.
#[allow(
    clippy::inline_always,
    reason = "the lane helpers must be inlined into the chunk loop so the class's runs fold \
              in as constants and the lane compares vectorize"
)]
#[inline(always)]
fn in_runs(b: u8, runs: &[(u8, u8)]) -> bool {
    let mut hit = false;
    for &(lo, hi) in runs {
        hit |= b.wrapping_sub(lo) <= hi - lo;
    }
    hit
}

/// The lanes of one chunk for one class: lane `i` is `0xFF` iff `chunk[i]` is in
/// `runs`, else `0x00`.
///
/// Independent byte comparisons with no cross-lane dependence, stored as
/// `0x00`/`0xFF` bytes: the shape the compiler lowers to packed byte compares
/// on every target that has them. Every facet byte is ASCII, so a lane can never
/// match a byte of a multi-byte UTF-8 sequence.
#[allow(
    clippy::inline_always,
    reason = "the lane helpers must be inlined into the chunk loop so the class's runs fold \
              in as constants and the lane compares vectorize"
)]
#[inline(always)]
fn class_lanes(chunk: &[u8; CHUNK], runs: &[(u8, u8)]) -> [u8; CHUNK] {
    let mut lanes = [0_u8; CHUNK];
    for (lane, &b) in lanes.iter_mut().zip(chunk) {
        *lane = u8::from(in_runs(b, runs)).wrapping_neg();
    }
    lanes
}

/// Whether any lane of `lanes` is set: a branch-free OR over the chunk.
#[allow(
    clippy::inline_always,
    reason = "the lane helpers must be inlined into the chunk loop so the class's runs fold \
              in as constants and the lane compares vectorize"
)]
#[inline(always)]
fn any_lane(lanes: &[u8; CHUNK]) -> bool {
    lanes.iter().fold(0, |any, &lane| any | lane) != 0
}

/// The offset of the first `#x9` / `#xA` / `#xD` byte, or `None`.
///
/// Sixteen-byte chunks are tested whole — a branch-free OR of the lanes is the
/// clean-chunk test — and the one chunk holding a hit is handed to
/// [`first_replaced_lane`]. The bytes after the last whole chunk are tested one at
/// a time.
fn find_first_replaced(bytes: &[u8]) -> Option<usize> {
    let (chunks, tail) = bytes.as_chunks::<CHUNK>();
    for (k, chunk) in chunks.iter().enumerate() {
        if any_lane(&class_lanes(chunk, &REPLACED_RUNS)) {
            return Some(k * CHUNK + first_replaced_lane(chunk));
        }
    }
    tail.iter()
        .position(|&b| in_runs(b, &REPLACED_RUNS))
        .map(|i| chunks.len() * CHUNK + i)
}

/// The lane of the first `#x9` / `#xA` / `#xD` byte in a chunk known to hold one:
/// the chunk's lanes read as a little-endian `u128`, whose trailing-zero count over
/// 8 is that lane.
///
/// Out of line and cold on purpose, and measured: a value that needs rewriting is
/// the uncommon case, and when this extraction shared a body with the clean-chunk
/// test, the wasm `simd128` build kept the lane compares scalar to avoid extracting
/// the lanes twice. Separate, the clean-chunk loop is compares and a reduction only.
#[cold]
#[inline(never)]
fn first_replaced_lane(chunk: &[u8; CHUNK]) -> usize {
    let lanes = class_lanes(chunk, &REPLACED_RUNS);
    (u128::from_le_bytes(lanes).trailing_zeros() / u8::BITS) as usize
}

/// Whether `bytes` is already in `collapse` normal form: no `#x9` / `#xA` /
/// `#xD`, no leading or trailing `#x20`, and no two adjacent `#x20`.
///
/// Per sixteen-byte chunk, a space at lane `i` whose predecessor is also a space
/// is found by comparing the space lanes with themselves shifted by one lane; the
/// predecessor of lane 0 is the last lane of the previous chunk, carried across.
/// Every test is a branch-free OR folded over the chunk.
fn is_collapsed(bytes: &[u8]) -> bool {
    if bytes.first() == Some(&b' ') || bytes.last() == Some(&b' ') {
        return false;
    }
    let (chunks, tail) = bytes.as_chunks::<CHUNK>();
    let mut previous_space = false;
    for chunk in chunks {
        if any_lane(&class_lanes(chunk, &REPLACED_RUNS)) {
            return false;
        }
        let space = class_lanes(chunk, &SPACE_RUNS);
        let space_bits = u128::from_le_bytes(space);
        // Lane `i` of the shifted value is lane `i - 1` of `space`, and lane 0 is
        // the carried last lane of the previous chunk.
        let preceded =
            (space_bits << u8::BITS) | u128::from(u8::from(previous_space).wrapping_neg());
        let adjacent = space_bits & preceded;
        if adjacent != 0 {
            return false;
        }
        previous_space = space[CHUNK - 1] != 0;
    }
    for &b in tail {
        if in_runs(b, &REPLACED_RUNS) || (b == b' ' && previous_space) {
            return false;
        }
        previous_space = b == b' ';
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pre-byte-scan per-char `replace` facet, kept as the oracle.
    fn replace_char_reference(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '\t' | '\n' | '\r' => ' ',
                other => other,
            })
            .collect()
    }

    /// The pre-byte-scan per-char `collapse` facet, kept as the oracle.
    fn collapse_char_reference(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut pending_space = false;
        for ch in s.chars() {
            if matches!(ch, ' ' | '\t' | '\n' | '\r') {
                pending_space = !out.is_empty();
            } else {
                if pending_space {
                    out.push(' ');
                    pending_space = false;
                }
                out.push(ch);
            }
        }
        out
    }

    /// `normalize_whitespace_replace` as it was before the chunked precheck.
    fn replace_byte_scan_reference(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut run_start = 0;
        for (i, b) in s.bytes().enumerate() {
            if matches!(b, b'\t' | b'\n' | b'\r') {
                out.push_str(&s[run_start..i]);
                out.push(' ');
                run_start = i + 1;
            }
        }
        out.push_str(&s[run_start..]);
        out
    }

    /// `normalize_whitespace_collapse` as it was before the chunked precheck.
    fn collapse_byte_scan_reference(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut pending_space = false;
        let mut run_start: Option<usize> = None;
        for (i, b) in s.bytes().enumerate() {
            if matches!(b, b' ' | b'\t' | b'\n' | b'\r') {
                if let Some(start) = run_start.take() {
                    out.push_str(&s[start..i]);
                }
                pending_space = !out.is_empty();
            } else if run_start.is_none() {
                if pending_space {
                    out.push(' ');
                    pending_space = false;
                }
                run_start = Some(i);
            }
        }
        if let Some(start) = run_start {
            out.push_str(&s[start..]);
        }
        out
    }

    /// A fixed-seed generator (SplitMix64), so every run draws the same inputs.
    struct SplitMix(u64);

    impl SplitMix {
        fn below(&mut self, n: usize) -> usize {
            let z = purrdf_testkit::rng::splitmix64_next(&mut self.0);
            usize::try_from(z % 1_000_003).expect("small") % n
        }
    }

    /// Every byte on either side of the facet's classes (`#x9-#xA`, `#xD`,
    /// `#x20`), and non-ASCII scalars of every UTF-8 length including the Unicode
    /// whitespace the facet must leave alone.
    const FACET_SCALARS: &[char] = &[
        '\u{8}',
        '\t',
        '\n',
        '\u{b}',
        '\u{c}',
        '\r',
        '\u{e}',
        '\u{1f}',
        ' ',
        '!',
        'a',
        '\u{7f}',
        '\u{85}',
        '\u{a0}',
        '\u{e9}',
        '\u{2028}',
        '\u{3000}',
        '\u{feff}',
        '\u{1f600}',
    ];

    /// Fixed-seed random values at lengths 0..=70 and beyond, mostly clean so the
    /// precheck's clean path, and hits at every chunk offset, are both reached.
    fn facet_corpus() -> Vec<String> {
        let mut rng = SplitMix(0x05DF_ACE7_5EED);
        let mut out = Vec::new();
        for len in (0..=70).chain([127, 128, 129, 1000]) {
            for density in [0, 1, 4] {
                for _ in 0..12 {
                    let value: String = (0..len)
                        .map(|_| {
                            if density != 0 && rng.below(16) < density {
                                FACET_SCALARS[rng.below(FACET_SCALARS.len())]
                            } else if rng.below(6) == 0 {
                                // Single spaces keep collapse's clean path reachable.
                                ' '
                            } else {
                                'x'
                            }
                        })
                        .collect();
                    out.push(value);
                }
            }
        }
        out
    }

    #[test]
    fn chunked_replace_matches_the_byte_scan_and_char_references() {
        let mut clean = 0_usize;
        for value in facet_corpus() {
            let got = normalize_whitespace_replace(&value);
            assert_eq!(got, replace_byte_scan_reference(&value), "{value:?}");
            assert_eq!(got, replace_char_reference(&value), "{value:?}");
            clean += usize::from(find_first_replaced(value.as_bytes()).is_none());
        }
        assert!(clean > 0, "the corpus reaches the clean path");
    }

    #[test]
    fn chunked_collapse_matches_the_byte_scan_and_char_references() {
        let mut clean = [0_usize; 2];
        for value in facet_corpus() {
            let got = normalize_whitespace_collapse(&value);
            assert_eq!(got, collapse_byte_scan_reference(&value), "{value:?}");
            assert_eq!(got, collapse_char_reference(&value), "{value:?}");
            // The precheck says "already collapsed" exactly when collapsing is the
            // identity: never a false "clean", never a missed one.
            let collapsed = is_collapsed(value.as_bytes());
            assert_eq!(collapsed, got == value, "{value:?}");
            clean[usize::from(collapsed)] += 1;
        }
        assert!(clean.iter().all(|&n| n > 0), "{clean:?}");
    }

    /// Adjacent spaces are caught at every position, including across the seam
    /// between two chunks, and a single space at the same place is not.
    #[test]
    fn collapse_precheck_sees_adjacent_spaces_across_chunk_seams() {
        for at in 1..40 {
            let mut double = "x".repeat(41).into_bytes();
            double[at] = b' ';
            double[at + 1] = b' ';
            let double = String::from_utf8(double).expect("ascii");
            assert!(!is_collapsed(double.as_bytes()), "double space at {at}");
            let mut single = "x".repeat(41).into_bytes();
            single[at] = b' ';
            let single = String::from_utf8(single).expect("ascii");
            assert!(is_collapsed(single.as_bytes()), "single space at {at}");
        }
    }

    const WHITESPACE_FACET_CORPUS: &[&str] = &[
        "",
        " ",
        "\t\n\r ",
        "a",
        "  a  ",
        "a\tb\nc\rd e",
        "\t\ta\n\nb\r\rc  d\t \n \r ",
        "ünïcödé\tstring\nwith\rmultibyte",
        " \u{00A0}nbsp\u{00A0}is not xsd whitespace\u{00A0} ",
        "日本語\t中文\n한국어\r  العربية ",
        "\u{1F600}\t\u{1F601}\n\u{1F602}\r\u{1F603}  \u{1F604}",
        "\u{2028}\u{2029}\u{3000}\t\u{FEFF}",
        "tail\t",
        "\thead",
        "a\r\nb",
        "é",
        "\té\n",
    ];

    #[test]
    fn byte_scan_replace_matches_char_reference() {
        for input in WHITESPACE_FACET_CORPUS {
            assert_eq!(
                normalize_whitespace_replace(input),
                replace_char_reference(input),
                "input = {input:?}"
            );
        }
    }

    #[test]
    fn byte_scan_collapse_matches_char_reference() {
        for input in WHITESPACE_FACET_CORPUS {
            assert_eq!(
                normalize_whitespace_collapse(input),
                collapse_char_reference(input),
                "input = {input:?}"
            );
        }
    }

    #[test]
    fn boolean_lexicals() {
        assert_eq!(parse_boolean("true"), Ok(true));
        assert_eq!(parse_boolean("1"), Ok(true));
        assert_eq!(parse_boolean("false"), Ok(false));
        assert_eq!(parse_boolean("0"), Ok(false));
        assert!(parse_boolean("TRUE").is_err());
        assert!(parse_boolean("yes").is_err());
        assert!(parse_boolean("").is_err());
    }

    // ── whiteSpace = replace (xsd:normalizedString) ───────────────────────────────

    #[test]
    fn replace_maps_each_xsd_whitespace_to_space() {
        assert_eq!(normalize_whitespace_replace("\two\nw"), " wo w");
        assert_eq!(
            normalize_whitespace_replace("hey\nthere\ta tab\rcarriage return"),
            "hey there a tab carriage return"
        );
    }

    #[test]
    fn replace_preserves_length_and_does_not_collapse_or_trim() {
        // Two consecutive whitespace characters become two spaces (no collapse);
        // leading/trailing whitespace becomes leading/trailing spaces (no trim).
        assert_eq!(
            normalize_whitespace_replace("\tBeing a Doctor Is\n\ta Full-Time Job\r"),
            " Being a Doctor Is  a Full-Time Job "
        );
    }

    #[test]
    fn replace_leaves_non_xsd_whitespace_untouched() {
        // U+00A0 (no-break space) is NOT an XSD whitespace character.
        assert_eq!(normalize_whitespace_replace("a\u{00A0}b"), "a\u{00A0}b");
        assert_eq!(normalize_whitespace_replace(""), "");
    }

    // ── whiteSpace = collapse (xsd:token) ─────────────────────────────────────────

    #[test]
    fn collapse_collapses_runs_and_strips_ends() {
        assert_eq!(
            normalize_whitespace_collapse("       hey\nthere      "),
            "hey there"
        );
        assert_eq!(
            normalize_whitespace_collapse("\tBeing a Doctor    Is\n\ta Full-Time Job\r"),
            "Being a Doctor Is a Full-Time Job"
        );
    }

    #[test]
    fn collapse_leading_trailing_interior() {
        assert_eq!(
            normalize_whitespace_collapse(
                "\n  hey -  white  space is collapsed for xsd:token       and preceding and trailing whitespace is stripped     "
            ),
            "hey - white space is collapsed for xsd:token and preceding and trailing whitespace is stripped"
        );
    }

    #[test]
    fn collapse_edge_cases() {
        assert_eq!(normalize_whitespace_collapse(""), "");
        assert_eq!(normalize_whitespace_collapse("   "), "");
        assert_eq!(normalize_whitespace_collapse("\t\n\r"), "");
        assert_eq!(normalize_whitespace_collapse("word"), "word");
        // Non-XSD whitespace is preserved (not a separator).
        assert_eq!(
            normalize_whitespace_collapse("  a\u{00A0}b  "),
            "a\u{00A0}b"
        );
    }
}
