// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! IDNA2008 conformance for `purrdf_iri::idna` and RFC 3987 §3.1 for
//! `Iri::to_uri`.
//!
//! Oracles, all public standards data (see `tests/PROVENANCE.md`):
//!
//! * `IdnaTestV2.txt` 17.0.0, vendored under `crates/iri/unicode/17.0.0/`,
//!   read through one committed row filter ([`Row::verdict`]);
//! * the RFC 3492 §7.1 sample strings;
//! * the RFC 3987 §3.1 examples.

use purrdf_iri::idna::{
    UNICODE_VERSION, is_hostname, is_idn_hostname, map, punycode_decode, punycode_encode, to_ascii,
    to_ascii_mapped,
};
use purrdf_iri::{parse, parse_uri};

// ---- IdnaTestV2.txt -------------------------------------------------------------

/// How many `IdnaTestV2.txt` rows [`Row::verdict`] includes in each lane. A
/// change to the filter or the file moves these numbers, and the test says so.
const INCLUDED_UNMAPPED: usize = 2960;
const INCLUDED_MAPPED: usize = 6202;

/// One data line of `IdnaTestV2.txt`, with the file's blank-column defaults
/// resolved as its header specifies.
struct Row {
    line: usize,
    /// The string columns; `None` where the file writes an ill-formed string.
    source: Option<String>,
    to_unicode: Option<String>,
    to_ascii_n: Option<String>,
    /// The toAsciiN status codes (empty: no error).
    to_ascii_n_status: Vec<String>,
}

/// Why a row is left out; every reason names the clause that makes UTS 46's
/// answer something other than an IDNA2008 answer for that row.
#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Excluded {
    /// A string column holds an unpaired surrogate. IDNA input is a Unicode
    /// string (RFC 5891 §4.1, §5.2), which a surrogate code point is not, and
    /// the file's header says an implementation that cannot hold ill-formed
    /// strings skips these rows.
    IllFormed,
    /// UTS 46 processing mapped the source (the toUnicode column differs from
    /// it), and the source is not the ASCII form either. RFC 5891 §5.2 leaves
    /// mapping to the application, so the expected answer depends on the
    /// UTS 46 mapping table, which is not this crate's mapping.
    Mapped,
    /// The row's toUnicode contains a CONTEXTO code point (RFC 5892 §2.6), and
    /// the file does not test CONTEXTO rules (its header says so), while
    /// RFC 5891 §4.2.3.3 requires a contextual code point to be positively
    /// confirmed by its RFC 5892 Appendix A rule, which this crate evaluates.
    ContextO,
    /// UTS 46 accepts the row, but its toUnicode contains a code point whose
    /// General_Category is outside RFC 5892 §2.1 LetterDigits and which no
    /// earlier rule of the RFC 5892 §3 algorithm (§2.6 Exceptions, §2.5 LDH,
    /// §2.8 JoinControl) admits, so RFC 5892 §3 makes it DISALLOWED. The file
    /// header names this class (UTS 46 "NV8"/"XV8": valid there, not valid
    /// in IDNA2008). Such rows are still checked: `to_ascii` must refuse them.
    NotLetterDigits,
    /// Mapped lane only: the source holds U+1E9E LATIN CAPITAL LETTER SHARP S.
    /// NFKC_Casefold's full case folding maps it to `ss`, UTS 46 to U+00DF;
    /// RFC 5891 §5.2 leaves the mapping to the application.
    CapitalSharpS,
    /// Mapped lane only: UTS 46's one objection to the row is a disallowed
    /// code point (status exactly `[V7]`), and the source holds a tag
    /// character (U+E0001, U+E0020..U+E007F), which is Default_Ignorable and
    /// which NFKC_Casefold therefore erases; RFC 5891 §5.2 leaves the mapping
    /// to the application.
    TagCharacter,
}

/// Which function a pass over the file checks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lane {
    /// `to_ascii(source)`: no mapping step.
    Unmapped,
    /// `to_ascii_mapped(source)`: this crate's mapping step first.
    Mapped,
}

/// The General_Category of every assigned code point, read from the vendored
/// `UnicodeData.txt` — independently of the generated tables under test.
fn general_categories() -> std::collections::HashMap<char, String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/unicode/17.0.0/UnicodeData.txt"
    );
    let text = std::fs::read_to_string(path).expect("UnicodeData.txt is vendored");
    let mut map = std::collections::HashMap::new();
    let mut first: Option<u32> = None;
    for line in text.lines() {
        let fields: Vec<&str> = line.split(';').collect();
        let cp = u32::from_str_radix(fields[0], 16).expect("code point");
        let start = if fields[1].ends_with(", Last>") {
            first.take().expect("range First before Last")
        } else if fields[1].ends_with(", First>") {
            first = Some(cp);
            continue;
        } else {
            cp
        };
        for member in start..=cp {
            if let Some(c) = char::from_u32(member) {
                map.insert(c, fields[2].to_owned());
            }
        }
    }
    map
}

/// Whether RFC 5892 §3 can admit `c` at all: it is LDH (§2.5), a JoinControl
/// (§2.8), a §2.6 PVALID or CONTEXTO exception, or LetterDigits (§2.1).
fn letter_digits_or_earlier_rule(c: char, gc: &std::collections::HashMap<char, String>) -> bool {
    matches!(c, '-' | '0'..='9' | 'a'..='z' | '\u{200C}' | '\u{200D}')
        || matches!(
            c,
            '\u{00DF}' | '\u{03C2}' | '\u{06FD}' | '\u{06FE}' | '\u{0F0B}' | '\u{3007}'
        )
        || is_contexto(c)
        || gc
            .get(&c)
            .is_some_and(|g| matches!(g.as_str(), "Ll" | "Lu" | "Lo" | "Nd" | "Lm" | "Mn" | "Mc"))
}

/// The RFC 5892 §2.6 CONTEXTO exceptions — the only CONTEXTO code points.
fn is_contexto(c: char) -> bool {
    matches!(
        c,
        '\u{00B7}'
            | '\u{0375}'
            | '\u{05F3}'
            | '\u{05F4}'
            | '\u{30FB}'
            | '\u{0660}'..='\u{0669}'
            | '\u{06F0}'..='\u{06F9}'
    )
}

impl Row {
    /// THE row filter: `Ok((source, expected))` to test the row,
    /// `Err(reason)` to skip it.
    ///
    /// A predicate over the source, toUnicode and toAsciiN columns only. The
    /// transitional columns (toAsciiT) are never read: RFC 5891 has no
    /// transitional processing.
    fn verdict(
        &self,
        lane: Lane,
        gc: &std::collections::HashMap<char, String>,
    ) -> Result<(&str, Option<&str>), Excluded> {
        let (Some(source), Some(to_unicode), Some(to_ascii_n)) =
            (&self.source, &self.to_unicode, &self.to_ascii_n)
        else {
            return Err(Excluded::IllFormed);
        };
        if lane == Lane::Unmapped && source != to_unicode && source != to_ascii_n {
            return Err(Excluded::Mapped);
        }
        // The IDNA2008 answer expected from `to_ascii(source)`.
        let expected = self
            .to_ascii_n_status
            .is_empty()
            .then_some(to_ascii_n.as_str());
        // A row UTS 46 already refuses is refused here too, whatever it holds;
        // the two remaining classes only conflict where UTS 46 accepts.
        if expected.is_some() {
            if to_unicode.chars().any(is_contexto) {
                return Err(Excluded::ContextO);
            }
            if !to_unicode
                .chars()
                .all(|c| c == '.' || letter_digits_or_earlier_rule(c, gc))
            {
                return Err(Excluded::NotLetterDigits);
            }
        }
        if lane == Lane::Mapped {
            if source.contains('\u{1E9E}') {
                return Err(Excluded::CapitalSharpS);
            }
            if self.to_ascii_n_status == ["V7"]
                && source
                    .chars()
                    .any(|c| c == '\u{E0001}' || ('\u{E0020}'..='\u{E007F}').contains(&c))
            {
                return Err(Excluded::TagCharacter);
            }
        }
        Ok((source, expected))
    }
}

/// `\uXXXX` and `\x{XXXX}` escapes, per the file's FORMAT section; `None`
/// when an escape names a surrogate, which no Rust string can hold.
fn unescape(field: &str) -> Option<String> {
    let mut out = String::new();
    let mut rest = field;
    while let Some(at) = rest.find('\\') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let (hex, consumed) = if let Some(body) = tail.strip_prefix("\\u") {
            (&body[..4], 6)
        } else if let Some(body) = tail.strip_prefix("\\x{") {
            let close = body.find('}').expect("closed \\x{ escape");
            (&body[..close], close + 4)
        } else {
            panic!("unknown escape in {field:?}");
        };
        let cp = u32::from_str_radix(hex, 16).expect("hex escape");
        out.push(char::from_u32(cp)?);
        rest = &tail[consumed..];
    }
    out.push_str(rest);
    Some(out)
}

/// A string column: blank means `default`, `""` means empty; `None` for an
/// ill-formed string.
fn column(field: &str, default: Option<&str>) -> Option<String> {
    match field {
        "" => default.map(str::to_owned),
        "\"\"" => Some(String::new()),
        text => unescape(text),
    }
}

/// A status column: blank means `default`, `[]` means no error.
fn status(field: &str, default: &[String]) -> Vec<String> {
    if field.is_empty() {
        return default.to_vec();
    }
    let inner = field
        .strip_prefix('[')
        .and_then(|f| f.strip_suffix(']'))
        .expect("bracketed status");
    inner
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}

fn rows() -> Vec<Row> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/unicode/17.0.0/IdnaTestV2.txt");
    let text = std::fs::read_to_string(path).expect("IdnaTestV2.txt is vendored");
    assert!(text.contains("# Version: 17.0.0"));
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.splitn(7, ';').map(str::trim).collect();
        assert_eq!(fields.len(), 7, "line {}: {line:?}", index + 1);
        let source = column(fields[0], Some(""));
        let to_unicode = column(fields[1], source.as_deref());
        let to_unicode_status = status(fields[2], &[]);
        let to_ascii_n = column(fields[3], to_unicode.as_deref());
        let to_ascii_n_status = status(fields[4], &to_unicode_status);
        rows.push(Row {
            line: index + 1,
            source,
            to_unicode,
            to_ascii_n,
            to_ascii_n_status,
        });
    }
    rows
}

/// Run one lane over the file: print the counts, and return the included
/// count and every failure.
fn run_lane(lane: Lane) -> (usize, Vec<String>) {
    let rows = rows();
    let gc = general_categories();
    let convert = match lane {
        Lane::Unmapped => to_ascii,
        Lane::Mapped => to_ascii_mapped,
    };
    let mut included = 0_usize;
    let mut excluded = std::collections::BTreeMap::<Excluded, usize>::new();
    let mut failures = Vec::new();
    for row in &rows {
        let (source, expected) = match row.verdict(lane, &gc) {
            Ok(test) => test,
            Err(reason) => {
                *excluded.entry(reason).or_default() += 1;
                if reason == Excluded::NotLetterDigits
                    && let Some(source) = &row.source
                    && convert(source).is_some()
                {
                    failures.push(format!(
                        "line {}: {lane:?} accepted {source:?}, which holds a DISALLOWED code point",
                        row.line
                    ));
                }
                continue;
            }
        };
        included += 1;
        let actual = convert(source);
        if actual.as_deref() != expected {
            failures.push(format!(
                "line {}: {lane:?}({source:?}) = {actual:?}, expected {expected:?} (status {:?})",
                row.line, row.to_ascii_n_status
            ));
        }
    }
    println!(
        "IdnaTestV2.txt {lane:?}: {} rows, {included} included, excluded {excluded:?}",
        rows.len()
    );
    for failure in failures.iter().take(40) {
        println!("{failure}");
    }
    (included, failures)
}

#[test]
fn idna_test_v2_unmapped() {
    assert_eq!(UNICODE_VERSION, (17, 0, 0));
    let (included, failures) = run_lane(Lane::Unmapped);
    assert!(failures.is_empty(), "{} failures", failures.len());
    assert_eq!(included, INCLUDED_UNMAPPED);
}

#[test]
fn idna_test_v2_mapped() {
    let (included, failures) = run_lane(Lane::Mapped);
    assert!(failures.is_empty(), "{} failures", failures.len());
    assert_eq!(included, INCLUDED_MAPPED);
}

// ---- RFC 3492 §7.1 sample strings -------------------------------------------------

/// `(code points, Punycode)` exactly as RFC 3492 §7.1 lists them. The RFC
/// writes some digits in upper case to show mixed-case annotation, which this
/// encoder does not produce, so encoder output is compared with the extended
/// part (after the last delimiter) lower-cased.
const RFC_3492_SAMPLES: &[(&[u32], &str)] = &[
    // (A) Arabic (Egyptian)
    (
        &[
            0x0644, 0x064A, 0x0647, 0x0645, 0x0627, 0x0628, 0x062A, 0x0643, 0x0644, 0x0645, 0x0648,
            0x0634, 0x0639, 0x0631, 0x0628, 0x064A, 0x061F,
        ],
        "egbpdaj6bu4bxfgehfvwxn",
    ),
    // (B) Chinese (simplified)
    (
        &[
            0x4ED6, 0x4EEC, 0x4E3A, 0x4EC0, 0x4E48, 0x4E0D, 0x8BF4, 0x4E2D, 0x6587,
        ],
        "ihqwcrb4cv8a8dqg056pqjye",
    ),
    // (C) Chinese (traditional)
    (
        &[
            0x4ED6, 0x5011, 0x7232, 0x4EC0, 0x9EBD, 0x4E0D, 0x8AAA, 0x4E2D, 0x6587,
        ],
        "ihqwctvzc91f659drss3x8bo0yb",
    ),
    // (D) Czech
    (
        &[
            0x0050, 0x0072, 0x006F, 0x010D, 0x0070, 0x0072, 0x006F, 0x0073, 0x0074, 0x011B, 0x006E,
            0x0065, 0x006D, 0x006C, 0x0075, 0x0076, 0x00ED, 0x010D, 0x0065, 0x0073, 0x006B, 0x0079,
        ],
        "Proprostnemluvesky-uyb24dma41a",
    ),
    // (E) Hebrew
    (
        &[
            0x05DC, 0x05DE, 0x05D4, 0x05D4, 0x05DD, 0x05E4, 0x05E9, 0x05D5, 0x05D8, 0x05DC, 0x05D0,
            0x05DE, 0x05D3, 0x05D1, 0x05E8, 0x05D9, 0x05DD, 0x05E2, 0x05D1, 0x05E8, 0x05D9, 0x05EA,
        ],
        "4dbcagdahymbxekheh6e0a7fei0b",
    ),
    // (F) Hindi (Devanagari)
    (
        &[
            0x092F, 0x0939, 0x0932, 0x094B, 0x0917, 0x0939, 0x093F, 0x0928, 0x094D, 0x0926, 0x0940,
            0x0915, 0x094D, 0x092F, 0x094B, 0x0902, 0x0928, 0x0939, 0x0940, 0x0902, 0x092C, 0x094B,
            0x0932, 0x0938, 0x0915, 0x0924, 0x0947, 0x0939, 0x0948, 0x0902,
        ],
        "i1baa7eci9glrd9b2ae1bj0hfcgg6iyaf8o0a1dig0cd",
    ),
    // (G) Japanese (kanji and hiragana)
    (
        &[
            0x306A, 0x305C, 0x307F, 0x3093, 0x306A, 0x65E5, 0x672C, 0x8A9E, 0x3092, 0x8A71, 0x3057,
            0x3066, 0x304F, 0x308C, 0x306A, 0x3044, 0x306E, 0x304B,
        ],
        "n8jok5ay5dzabd5bym9f0cm5685rrjetr6pdxa",
    ),
    // (H) Korean (Hangul syllables)
    (
        &[
            0xC138, 0xACC4, 0xC758, 0xBAA8, 0xB4E0, 0xC0AC, 0xB78C, 0xB4E4, 0xC774, 0xD55C, 0xAD6D,
            0xC5B4, 0xB97C, 0xC774, 0xD574, 0xD55C, 0xB2E4, 0xBA74, 0xC5BC, 0xB9C8, 0xB098, 0xC88B,
            0xC744, 0xAE4C,
        ],
        "989aomsvi5e83db1d2a355cv1e0vak1dwrv93d5xbh15a0dt30a5jpsd879ccm6fea98c",
    ),
    // (I) Russian (Cyrillic)
    (
        &[
            0x043F, 0x043E, 0x0447, 0x0435, 0x043C, 0x0443, 0x0436, 0x0435, 0x043E, 0x043D, 0x0438,
            0x043D, 0x0435, 0x0433, 0x043E, 0x0432, 0x043E, 0x0440, 0x044F, 0x0442, 0x043F, 0x043E,
            0x0440, 0x0443, 0x0441, 0x0441, 0x043A, 0x0438,
        ],
        "b1abfaaepdrnnbgefbaDotcwatmq2g4l",
    ),
    // (J) Spanish
    (
        &[
            0x0050, 0x006F, 0x0072, 0x0071, 0x0075, 0x00E9, 0x006E, 0x006F, 0x0070, 0x0075, 0x0065,
            0x0064, 0x0065, 0x006E, 0x0073, 0x0069, 0x006D, 0x0070, 0x006C, 0x0065, 0x006D, 0x0065,
            0x006E, 0x0074, 0x0065, 0x0068, 0x0061, 0x0062, 0x006C, 0x0061, 0x0072, 0x0065, 0x006E,
            0x0045, 0x0073, 0x0070, 0x0061, 0x00F1, 0x006F, 0x006C,
        ],
        "PorqunopuedensimplementehablarenEspaol-fmd56a",
    ),
    // (K) Vietnamese
    (
        &[
            0x0054, 0x1EA1, 0x0069, 0x0073, 0x0061, 0x006F, 0x0068, 0x1ECD, 0x006B, 0x0068, 0x00F4,
            0x006E, 0x0067, 0x0074, 0x0068, 0x1EC3, 0x0063, 0x0068, 0x1EC9, 0x006E, 0x00F3, 0x0069,
            0x0074, 0x0069, 0x1EBF, 0x006E, 0x0067, 0x0056, 0x0069, 0x1EC7, 0x0074,
        ],
        "TisaohkhngthchnitingVit-kjcr8268qyxafd2f1b9g",
    ),
    // (L) 3<nen>B<gumi><kinpachi><sensei>
    (
        &[
            0x0033, 0x5E74, 0x0042, 0x7D44, 0x91D1, 0x516B, 0x5148, 0x751F,
        ],
        "3B-ww4c5e180e575a65lsy2b",
    ),
    // (M) <amuro><namie>-with-SUPER-MONKEYS
    (
        &[
            0x5B89, 0x5BA4, 0x5948, 0x7F8E, 0x6075, 0x002D, 0x0077, 0x0069, 0x0074, 0x0068, 0x002D,
            0x0053, 0x0055, 0x0050, 0x0045, 0x0052, 0x002D, 0x004D, 0x004F, 0x004E, 0x004B, 0x0045,
            0x0059, 0x0053,
        ],
        "-with-SUPER-MONKEYS-pc58ag80a8qai00g7n9n",
    ),
    // (N) Hello-Another-Way-<sorezore><no><basho>
    (
        &[
            0x0048, 0x0065, 0x006C, 0x006C, 0x006F, 0x002D, 0x0041, 0x006E, 0x006F, 0x0074, 0x0068,
            0x0065, 0x0072, 0x002D, 0x0057, 0x0061, 0x0079, 0x002D, 0x305D, 0x308C, 0x305E, 0x308C,
            0x306E, 0x5834, 0x6240,
        ],
        "Hello-Another-Way--fc4qua05auwb3674vfr0b",
    ),
    // (O) <hitotsu><yane><no><shita>2
    (
        &[
            0x3072, 0x3068, 0x3064, 0x5C4B, 0x6839, 0x306E, 0x4E0B, 0x0032,
        ],
        "2-u9tlzr9756bt3uc0v",
    ),
    // (P) Maji<de>Koi<suru>5<byou><mae>
    (
        &[
            0x004D, 0x0061, 0x006A, 0x0069, 0x3067, 0x004B, 0x006F, 0x0069, 0x3059, 0x308B, 0x0035,
            0x79D2, 0x524D,
        ],
        "MajiKoi5-783gue6qz075azm5e",
    ),
    // (Q) <pafii>de<runba>
    (
        &[
            0x30D1, 0x30D5, 0x30A3, 0x30FC, 0x0064, 0x0065, 0x30EB, 0x30F3, 0x30D0,
        ],
        "de-jg4avhby1noc0d",
    ),
    // (R) <sono><supiido><de>
    (
        &[0x305D, 0x306E, 0x30B9, 0x30D4, 0x30FC, 0x30C9, 0x3067],
        "d9juau41awczczp",
    ),
    // (S) -> $1.00 <-
    (
        &[
            0x002D, 0x003E, 0x0020, 0x0024, 0x0031, 0x002E, 0x0030, 0x0030, 0x0020, 0x003C, 0x002D,
        ],
        "-> $1.00 <--",
    ),
];

#[test]
fn rfc_3492_samples_encode_and_decode() {
    assert_eq!(RFC_3492_SAMPLES.len(), 19, "(A) through (S)");
    for (code_points, punycode) in RFC_3492_SAMPLES {
        let text: String = code_points
            .iter()
            .map(|&cp| char::from_u32(cp).expect("scalar value"))
            .collect();
        // Mixed-case annotation is a property of the extended digits only.
        let expected = match punycode.rfind('-') {
            Some(at) if at > 0 => format!(
                "{}{}",
                &punycode[..=at],
                punycode[at + 1..].to_ascii_lowercase()
            ),
            _ => punycode.to_ascii_lowercase(),
        };
        assert_eq!(punycode_encode(&text).as_deref(), Some(expected.as_str()));
        assert_eq!(punycode_decode(punycode).as_deref(), Some(text.as_str()));
    }
}

#[test]
fn punycode_refusals_beside_accepted_neighbours() {
    // Every digit must be a base-36 digit.
    assert_eq!(punycode_decode("bcher-kva").as_deref(), Some("b\u{fc}cher"));
    assert_eq!(punycode_decode("bcher-kv!"), None);
    // The input must be ASCII.
    assert_eq!(punycode_decode("b\u{fc}cher-kva"), None);
    // An unterminated variable-length integer.
    assert_eq!(punycode_decode("bcher-kv"), None);
    // A delimiter with nothing before it is not consumed, so it is a digit.
    assert_eq!(punycode_decode("-kva"), None);
    // A decoded value beyond U+10FFFF is not a character: `9999a` reaches
    // U+737B5 (a scalar value), `99999a` reaches 4 760 513.
    assert_eq!(punycode_decode("9999a").as_deref(), Some("\u{737B5}"));
    assert_eq!(punycode_decode("99999a"), None);
    // Overflow (RFC 3492 §6.4) is a refusal, not a wrap: the weight passes
    // 2^32 long before this integer ends.
    assert_eq!(punycode_decode("99999999999999a"), None);
}

// ---- Host names --------------------------------------------------------------------

#[test]
fn label_and_name_lengths_refuse_beside_accepted_neighbours() {
    let label63 = "a".repeat(63);
    let label64 = "a".repeat(64);
    assert!(is_hostname(&label63) && !is_hostname(&label64));
    assert!(to_ascii(&label63).is_some() && to_ascii(&label64).is_none());

    // 253 octets is the longest name; 254 is refused.
    let name253 = format!("{label63}.{label63}.{label63}.{}", "a".repeat(61));
    let name254 = format!("{label63}.{label63}.{label63}.{}", "a".repeat(62));
    assert_eq!((name253.len(), name254.len()), (253, 254));
    assert!(is_hostname(&name253) && !is_hostname(&name254));
    assert!(is_idn_hostname(&name253) && !is_idn_hostname(&name254));

    // A U-label is measured by its A-label: 58 `ü` encode past 63 octets.
    let short = "\u{fc}".repeat(20);
    let long = "\u{fc}".repeat(58);
    let short_ascii = to_ascii(&short).expect("valid U-label");
    assert!(short_ascii.len() <= 63);
    assert_eq!(to_ascii(&long), None);
}

#[test]
fn empty_labels_refuse_beside_accepted_neighbours() {
    for text in ["", ".", "a..b", ".a", "a."] {
        assert!(!is_hostname(text), "{text:?}");
        assert!(!is_idn_hostname(text), "{text:?}");
    }
    assert!(is_hostname("a.b") && is_idn_hostname("a.b"));
    // U+3002 separates labels only after the mapping step.
    assert!(is_idn_hostname("a\u{3002}b") && !is_idn_hostname("a\u{3002}"));
    assert_eq!(to_ascii("a\u{3002}b"), None);
}

#[test]
fn hyphen_rules_refuse_beside_accepted_neighbours() {
    for bad in ["-ab", "ab-", "-"] {
        assert!(!is_hostname(bad) && !is_idn_hostname(bad), "{bad:?}");
    }
    assert!(is_hostname("a-b") && is_idn_hostname("a-b"));
    assert!(is_hostname("a--b") && is_idn_hostname("a--b"));
    // `--` in positions 3 and 4: an RFC 1123 host name, but a reserved LDH
    // label an IDN may not contain (RFC 5890 §2.3.1, §2.3.2.3).
    assert!(is_hostname("ab--cd"));
    assert_eq!(to_ascii("ab--cd"), None);
    assert!(!is_idn_hostname("ab--cd"));
    // The same rule for a U-label (RFC 5891 §4.2.3.1).
    assert_eq!(to_ascii("\u{fc}b--c"), None);
    assert!(to_ascii("\u{fc}b-c").is_some());
}

#[test]
fn hostname_is_ascii_ldh_only() {
    assert!(is_hostname("www.example.com"));
    assert!(is_hostname("1host.example"));
    for bad in [
        "host_name",
        "example.com\n",
        "exa mple.com",
        "b\u{fc}cher.example",
        "\u{212a}elvin.example.com",
    ] {
        assert!(!is_hostname(bad), "{bad:?}");
    }
}

#[test]
fn a_labels_refuse_beside_accepted_neighbours() {
    assert!(is_hostname("xn--bcher-kva.example"));
    // RFC 5891 §5.3: an A-label is lower-cased before it is decoded.
    assert!(is_hostname("XN--BCHER-KVA.example"));
    assert_eq!(
        to_ascii("XN--BCHER-KVA.example").as_deref(),
        Some("xn--bcher-kva.example")
    );
    // Not Punycode.
    assert!(!is_hostname("xn--bcher-kv.example"));
    // Decodes, but to ASCII only: not an A-label (RFC 5890 §2.3.2.1).
    assert_eq!(punycode_decode("example-").as_deref(), Some("example"));
    assert!(!is_hostname("xn--example-.example"));
    // Decodes to a DISALLOWED code point (U+2603 SNOWMAN).
    let snowman = format!("xn--{}", punycode_encode("\u{2603}").expect("encodes"));
    assert!(!is_hostname(&snowman));
    // Decodes to a label that fails the Bidi rule on its own.
    let bidi = format!("xn--{}", punycode_encode("\u{e0}\u{5d0}").expect("encodes"));
    assert!(!is_hostname(&bidi));
    let bidi_ok = format!(
        "xn--{}",
        punycode_encode("\u{5d0}\u{5d1}").expect("encodes")
    );
    assert!(is_hostname(&bidi_ok));
    // Punycode keeps the case of basic code points, and upper case is
    // DISALLOWED in a U-label; an A-label is lower-cased before decoding, so
    // its case never reaches the U-label.
    assert_eq!(punycode_decode("Bcher-kva").as_deref(), Some("B\u{fc}cher"));
    assert_eq!(to_ascii("B\u{fc}cher"), None);
    assert!(is_hostname("xn--Bcher-kva"));
}

#[test]
fn u_label_rules_refuse_beside_accepted_neighbours() {
    // NFC (RFC 5891 §5.4): the unmapped path refuses a decomposed label; the
    // mapping step composes it.
    assert_eq!(to_ascii("caf\u{e9}").as_deref(), Some("xn--caf-dma"));
    assert_eq!(to_ascii("cafe\u{301}"), None);
    assert_eq!(
        to_ascii_mapped("cafe\u{301}").as_deref(),
        Some("xn--caf-dma")
    );
    // Leading combining mark (RFC 5891 §4.2.3.2).
    assert!(to_ascii("\u{e0}\u{301}").is_some());
    assert_eq!(to_ascii("\u{301}\u{e0}"), None);
    // DISALLOWED and UNASSIGNED (RFC 5891 §4.2.2).
    assert!(to_ascii("a\u{3b1}").is_some());
    assert_eq!(to_ascii("a\u{2603}"), None);
    assert_eq!(to_ascii("a\u{378}"), None);
    // PVALID exceptions survive the mapping step unfolded (RFC 5892 §2.6).
    assert_eq!(
        to_ascii_mapped("fa\u{df}.de").as_deref(),
        Some("xn--fa-hia.de")
    );
    assert_eq!(map("\u{3c2}"), "\u{3c2}");
}

#[test]
fn bidi_rule_refuses_beside_accepted_neighbours() {
    assert!(to_ascii("\u{5d0}1").is_some(), "RTL label ending in EN");
    assert_eq!(to_ascii("1\u{5d0}"), None, "condition 1");
    assert_eq!(to_ascii("\u{5d0}a"), None, "condition 2");
    assert_eq!(to_ascii("\u{5d0}\u{5d1}\u{2d}"), None, "hyphen at the end");
    assert_eq!(to_ascii("\u{5d0}1\u{660}"), None, "condition 4");
    // In a Bidi domain name every label obeys the rule, LTR labels included.
    assert!(to_ascii("\u{5d0}.a1").is_some());
    assert_eq!(to_ascii("\u{5d0}.1a"), None, "condition 1 in an LTR label");
    // Without an RTL label the rule does not apply.
    assert!(to_ascii("1a.b").is_some());
}

#[test]
fn contextual_rules_refuse_beside_accepted_neighbours() {
    let pairs: &[(&str, &str)] = &[
        ("l\u{b7}l", "a\u{b7}l"),
        ("\u{3b1}\u{375}\u{3b2}", "\u{3b1}\u{375}s"),
        ("\u{5d0}\u{5f3}\u{5d1}", "a\u{5f3}\u{5d1}"),
        ("\u{5d0}\u{5f4}\u{5d1}", "\u{5f4}\u{5d1}"),
        ("\u{30fb}\u{30a1}", "def\u{30fb}abc"),
        ("\u{628}\u{660}\u{628}", "\u{628}\u{660}\u{6f0}"),
        ("\u{6f0}0", "\u{6f0}\u{660}"),
        ("\u{915}\u{94d}\u{200d}\u{937}", "\u{915}\u{200d}\u{937}"),
        ("\u{915}\u{94d}\u{200c}\u{937}", "\u{200c}\u{937}"),
        (
            "\u{628}\u{64a}\u{200c}\u{628}\u{64a}",
            "\u{628}\u{64a}\u{200c}",
        ),
    ];
    for (valid, refused) in pairs {
        assert!(is_idn_hostname(valid), "{valid:?}");
        assert!(!is_idn_hostname(refused), "{refused:?}");
    }
}

#[test]
fn mapping_step() {
    assert_eq!(map("Example.COM"), "example.com");
    assert_eq!(map("\u{ff11}\u{ff12}\u{ff13}"), "123");
    assert_eq!(map("a\u{200b}b"), "ab", "Default_Ignorable is erased");
    assert_eq!(map("a\u{200c}b"), "a\u{200c}b", "CONTEXTJ is kept");
    assert_eq!(map("a\u{ff0e}b\u{ff61}c\u{3002}d"), "a.b.c.d");
    // U+2488 DIGIT ONE FULL STOP would fold to `1.` and invent a label.
    assert_eq!(map("\u{2488}a"), "\u{2488}a");
    assert!(!is_idn_hostname("\u{2488}a") && is_idn_hostname("1.a"));
    // An UNASSIGNED code point is kept, and so refused.
    assert_eq!(map("a\u{e0f9b}"), "a\u{e0f9b}");
    assert!(!is_idn_hostname("a\u{e0f9b}"));
    // An A-label produced by the mapping step is decoded as one.
    assert!(is_idn_hostname("\u{ff58}\u{ff4e}--bcher-kva"));
    assert!(is_idn_hostname("XN--BCHER-KVA"));
}

// ---- RFC 3987 §3.1 --------------------------------------------------------------------

#[test]
fn rfc_3987_section_3_1_examples() {
    let cases = [
        // The ireg-name example, in its ToASCII variant.
        (
            "http://r\u{e9}sum\u{e9}.example.org",
            "http://xn--rsum-bpad.example.org",
        ),
        // An existing escape is kept; the non-ASCII code point is encoded.
        (
            "http://www.example.org/red%09ros\u{e9}#red",
            "http://www.example.org/red%09ros%C3%A9#red",
        ),
        // Supplementary-plane code points: four octets each.
        (
            "http://example.com/\u{10300}\u{10301}\u{10302}",
            "http://example.com/%F0%90%8C%80%F0%90%8C%81%F0%90%8C%82",
        ),
    ];
    for (iri, uri) in cases {
        let converted = parse(iri).expect("valid IRI").to_uri();
        assert_eq!(converted.as_str(), uri);
        let reparsed = parse_uri(converted.as_str()).expect("the result is a URI");
        assert_eq!(reparsed, converted, "spans agree with a fresh parse");
    }
}

#[test]
fn to_uri_components_and_fallback() {
    // An all-ASCII IRI maps to itself.
    let ascii = "http://user@example.org:8080/a%20b?q=1#f";
    assert_eq!(parse(ascii).expect("valid").to_uri().as_str(), ascii);
    // A host IDNA refuses is percent-encoded instead (the other §3.1 route).
    let fallback = parse("http://\u{2603}.example/").expect("valid").to_uri();
    assert_eq!(fallback.as_str(), "http://%E2%98%83.example/");
    // Userinfo, port, query (with iprivate) and fragment.
    let full = parse("http://us\u{e9}r@b\u{fc}cher.example:80/p\u{e9}?\u{e000}#\u{e9}")
        .expect("valid")
        .to_uri();
    assert_eq!(
        full.as_str(),
        "http://us%C3%A9r@xn--bcher-kva.example:80/p%C3%A9?%EE%80%80#%C3%A9"
    );
    assert_eq!(full.authority(), Some("us%C3%A9r@xn--bcher-kva.example:80"));
    assert_eq!(full.query(), Some("%EE%80%80"));
    // A relative reference and an IP-literal host.
    let relative = parse("/caf\u{e9}").expect("valid").to_uri();
    assert_eq!(relative.as_str(), "/caf%C3%A9");
    assert!(!relative.has_scheme());
    let literal = parse("http://[::1]/\u{e9}").expect("valid").to_uri();
    assert_eq!(literal.as_str(), "http://[::1]/%C3%A9");
    for converted in [fallback, full, relative, literal] {
        assert_eq!(parse_uri(converted.as_str()).expect("a URI"), converted);
    }
}

#[test]
fn parsing_never_applies_idna() {
    // RFC 3987 compares by code point: the host stays as written.
    let iri = parse("http://B\u{fc}cher.example/").expect("valid");
    assert_eq!(iri.authority(), Some("B\u{fc}cher.example"));
}
