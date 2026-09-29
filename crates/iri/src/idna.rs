// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! IDNA2008 host names: label validation, Punycode, and the ASCII form.
//!
//! This is the IDNA2008 protocol of RFC 5891 over the RFC 5892 derived
//! property, the RFC 5892 Appendix A contextual rules, the RFC 5893 Bidi rule
//! and the RFC 3492 Punycode algorithm, with every Unicode table generated from
//! the vendored Unicode 17.0.0 character database
//! (see [`UNICODE_VERSION`]).
//!
//! It is a separate step, never part of IRI parsing: RFC 3987 compares IRIs
//! code point by code point, so [`parse`](crate::parse) keeps an
//! internationalized host exactly as written, and only
//! [`Iri::to_uri`](crate::Iri::to_uri) converts it.
//!
//! | Function | Input | Answer |
//! |---|---|---|
//! | [`is_hostname`] | ASCII only | LDH host name whose every `xn--` label is a valid A-label |
//! | [`to_ascii`] | U-labels, A-labels, LDH labels, as given | The ASCII form, or `None` |
//! | [`map`] | Anything | The local mapping step (RFC 5891 §5.2) |
//! | [`to_ascii_mapped`] | Anything | [`to_ascii`] of [`map`] |
//! | [`is_idn_hostname`] | Anything | `to_ascii_mapped(..).is_some()` |
//!
//! `None` and `false` are the protocol's answer "not a valid name", not a
//! degraded failure: every refusal is a clause of RFC 5890–5893.
//!
//! # The mapping step
//!
//! RFC 5891 §5.2 leaves the mapping of user input to the application. [`map`]
//! applies Unicode's NFKC_Casefold per code point and then NFC — the stability
//! transform RFC 5892 §2.2 measures characters against — with three
//! deliberate departures: ZERO WIDTH NON-JOINER and ZERO WIDTH JOINER are
//! kept (RFC 5892 gives them CONTEXTJ ahead of every rule that would erase
//! them), U+3002 IDEOGRAPHIC FULL STOP becomes the label separator `.`, and a
//! code point whose image contains `.` without being exactly `.` is left as
//! it is, so it is refused as DISALLOWED rather than inventing a label
//! boundary. It is not the UTS 46 mapping.
//!
//! # Fast path
//!
//! An all-ASCII label never touches a Unicode table: it is a byte-class LDH
//! check, and only an `xn--` label is decoded. Normalization and table lookups
//! are paid only by labels that contain non-ASCII.
//!
//! # Examples
//!
//! ```rust
//! use purrdf_iri::idna;
//!
//! assert_eq!(
//!     idna::to_ascii("b\u{fc}cher.example").as_deref(),
//!     Some("xn--bcher-kva.example")
//! );
//! assert!(idna::is_hostname("xn--bcher-kva.example"));
//! // An upper-case letter is not a valid U-label character; the mapping step
//! // folds it first.
//! assert_eq!(idna::to_ascii("B\u{fc}cher.example"), None);
//! assert!(idna::is_idn_hostname("B\u{fc}cher.example"));
//! ```

use purrdf_lex::unicode::{ccc, is_nfc, nfc};

use crate::idna_tables::{
    BIDI, Bidi, DERIVED, Derived, JOINING, JoiningType, MARK, NFKC_CASEFOLD, SCRIPT, Script,
};

/// The Unicode version of every table this module consults, and of the
/// normalization it applies: the workspace's one,
/// [`purrdf_lex::unicode::UNICODE_VERSION`].
pub const UNICODE_VERSION: (u8, u8, u8) = purrdf_lex::unicode::UNICODE_VERSION;

/// The longest label, in octets of its ASCII form (RFC 5890 §2.3.1).
const MAX_LABEL: usize = 63;
/// The longest name, in octets of its ASCII form without a trailing dot: the
/// 255-octet wire limit of RFC 1034 §3.1 less the length octets.
const MAX_NAME: usize = 253;
/// The ACE prefix (RFC 5890 §2.3.2.1).
const ACE_PREFIX: &str = "xn--";

const ZWNJ: char = '\u{200C}';
/// Canonical_Combining_Class Virama (RFC 5892 Appendix A.1, A.2).
const VIRAMA: u8 = 9;

// ---- Public surface -----------------------------------------------------------

/// Whether `text` is an ASCII host name: dot-separated LDH labels of 1 to 63
/// octets that neither start nor end with `-`, at most 253 octets in all, in
/// which every label beginning `xn--` (in any case) is a valid A-label — its
/// Punycode decodes to a valid U-label that re-encodes to the same label — and
/// the Bidi rule holds when any decoded label is right-to-left.
///
/// A non-A-label with `--` in positions 3 and 4 is accepted here (it is an
/// RFC 1123 host name); [`to_ascii`] refuses it, because an IDN admits only
/// NR-LDH labels besides A- and U-labels (RFC 5890 §2.3.2.3).
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::idna::is_hostname;
///
/// assert!(is_hostname("www.example.com"));
/// assert!(is_hostname("xn--bcher-kva.example"));
/// assert!(!is_hostname("-example.com"));
/// assert!(!is_hostname("b\u{fc}cher.example"), "non-ASCII is never a host name");
/// assert!(!is_hostname("xn--X.example"), "not Punycode");
/// ```
pub fn is_hostname(text: &str) -> bool {
    if !text.is_ascii() {
        return false;
    }
    domain(text, Policy::Hostname, None)
}

/// The ASCII form of an internationalized domain name, with no mapping step.
///
/// Every label must be an NR-LDH label (kept as written), an A-label (lower
/// cased per RFC 5891 §5.3, validated, and required to round-trip), or a
/// U-label (validated under RFC 5891 §4.2 and encoded). Labels are separated by
/// `.` only, none may be empty, each ASCII label is at most 63 octets, the
/// whole name at most 253, and the Bidi rule of RFC 5893 applies when any label
/// is right-to-left. `None` when any of that fails.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::idna::to_ascii;
///
/// assert_eq!(to_ascii("\u{e0}.\u{5d0}\u{308}").as_deref(), Some("xn--0ca.xn--ssa73l"));
/// assert_eq!(to_ascii("xn--0ca.example").as_deref(), Some("xn--0ca.example"));
/// assert_eq!(to_ascii("a..b"), None, "empty label");
/// assert_eq!(to_ascii("\u{e0}\u{5d0}"), None, "RFC 5893 Bidi rule");
/// ```
pub fn to_ascii(domain_name: &str) -> Option<String> {
    let mut out = String::with_capacity(domain_name.len() + ACE_PREFIX.len());
    domain(domain_name, Policy::Idna, Some(&mut out)).then_some(out)
}

/// The local mapping step (RFC 5891 §5.2): NFKC_Casefold per code point, then
/// NFC, keeping ZERO WIDTH NON-JOINER / JOINER, mapping U+3002 IDEOGRAPHIC
/// FULL STOP to `.`, and leaving any code point whose image would introduce a
/// `.` unmapped. See the module documentation.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::idna::map;
///
/// assert_eq!(map("B\u{dc}CHER.Example"), "b\u{fc}cher.example");
/// assert_eq!(map("\u{ff11}\u{ff12}\u{ff13}"), "123");
/// assert_eq!(map("a\u{3002}b"), "a.b");
/// assert_eq!(map("cafe\u{301}"), "caf\u{e9}");
/// ```
pub fn map(text: &str) -> String {
    if text.is_ascii() {
        return text.to_ascii_lowercase();
    }
    let mut mapped = String::with_capacity(text.len());
    for c in text.chars() {
        // Only a DISALLOWED code point is mapped. One the protocol already
        // admits is its own image: folding it would erase a distinction
        // RFC 5892 draws on purpose (U+00DF and U+03C2 are PVALID exceptions,
        // U+200C/U+200D are CONTEXTJ). An UNASSIGNED one is kept too, so it is
        // refused (RFC 5891 §5.4) rather than erased on the strength of a
        // default property value it may not keep once assigned.
        if lookup(DERIVED, c) != Derived::Disallowed {
            mapped.push(c);
            continue;
        }
        match lookup_mapping(NFKC_CASEFOLD, c) {
            Some(image) if image.contains(&'.') && image != ['.'] => mapped.push(c),
            Some(image) => mapped.extend(image.iter().map(|&m| full_stop(m))),
            None => mapped.push(full_stop(c)),
        }
    }
    nfc(&mapped)
}

/// [`to_ascii`] of [`map`]: the ASCII form of a host name as a user might type
/// it, or `None` when the mapped name is not a valid IDN.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::idna::to_ascii_mapped;
///
/// assert_eq!(
///     to_ascii_mapped("R\u{e9}sum\u{e9}.Example.org").as_deref(),
///     Some("xn--rsum-bpad.example.org")
/// );
/// assert_eq!(to_ascii_mapped("\u{302e}x.example"), None, "DISALLOWED");
/// ```
pub fn to_ascii_mapped(host: &str) -> Option<String> {
    to_ascii(&map(host))
}

/// Whether `text` is an internationalized host name: [`to_ascii_mapped`]
/// succeeds.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::idna::is_idn_hostname;
///
/// assert!(is_idn_hostname("\u{c2e4}\u{b840}.\u{d14c}\u{c2a4}\u{d2b8}"));
/// assert!(is_idn_hostname("l\u{b7}l"));
/// assert!(!is_idn_hostname("a\u{b7}l"), "MIDDLE DOT needs an l on each side");
/// ```
pub fn is_idn_hostname(text: &str) -> bool {
    to_ascii_mapped(text).is_some()
}

/// RFC 3492 Punycode encoding of `input`, without the ACE prefix; `None` on
/// arithmetic overflow (RFC 3492 §6.4).
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::idna::punycode_encode;
///
/// assert_eq!(punycode_encode("b\u{fc}cher").as_deref(), Some("bcher-kva"));
/// ```
pub fn punycode_encode(input: &str) -> Option<String> {
    let chars: Vec<char> = input.chars().collect();
    punycode::encode(&chars)
}

/// RFC 3492 Punycode decoding of `input` (no ACE prefix); `None` when it is
/// not valid Punycode.
///
/// # Examples
///
/// ```rust
/// use purrdf_iri::idna::punycode_decode;
///
/// assert_eq!(punycode_decode("bcher-kva").as_deref(), Some("b\u{fc}cher"));
/// assert_eq!(punycode_decode("\u{fc}"), None, "input must be ASCII");
/// ```
pub fn punycode_decode(input: &str) -> Option<String> {
    punycode::decode(input).map(|chars| chars.into_iter().collect())
}

// ---- Domain and label validation ----------------------------------------------

/// What a domain name may contain.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Policy {
    /// [`is_hostname`]: ASCII only; R-LDH labels that are not XN-labels pass.
    Hostname,
    /// [`to_ascii`]: NR-LDH, A- and U-labels (RFC 5890 §2.3.2.3).
    Idna,
}

/// Validate `name` under `policy`, writing its ASCII form to `out` if given.
fn domain(name: &str, policy: Policy, mut out: Option<&mut String>) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut ascii_len = 0_usize;
    let mut bidi_domain = false;
    for (index, label) in name.split('.').enumerate() {
        if index > 0 {
            ascii_len += 1;
            if let Some(out) = out.as_deref_mut() {
                out.push('.');
            }
        }
        let Some(form) = label_form(label, policy) else {
            return false;
        };
        let written = match &form {
            LabelForm::Ldh => {
                if let Some(out) = out.as_deref_mut() {
                    out.push_str(label);
                }
                label.len()
            }
            LabelForm::ALabel(u_label) => {
                bidi_domain |= has_rtl(u_label);
                if let Some(out) = out.as_deref_mut() {
                    out.extend(label.chars().map(|c| c.to_ascii_lowercase()));
                }
                label.len()
            }
            LabelForm::ULabel(u_label, encoded) => {
                bidi_domain |= has_rtl(u_label);
                if let Some(out) = out.as_deref_mut() {
                    out.push_str(ACE_PREFIX);
                    out.push_str(encoded);
                }
                ACE_PREFIX.len() + encoded.len()
            }
        };
        if written > MAX_LABEL {
            return false;
        }
        ascii_len += written;
    }
    if ascii_len > MAX_NAME {
        return false;
    }
    // RFC 5893 §2: in a Bidi domain name every label satisfies the Bidi rule.
    // A second pass, because whether the name is a Bidi domain name is known
    // only once every label has been read.
    !bidi_domain
        || name.split('.').all(|label| {
            let chars: Vec<char> = if label.is_ascii() && !is_xn_label(label) {
                label.chars().collect()
            } else if label.is_ascii() {
                match a_label(&label.to_ascii_lowercase()) {
                    Some(u_label) => u_label,
                    None => return false,
                }
            } else {
                label.chars().collect()
            };
            bidi_rule(&chars)
        })
}

/// A validated label.
enum LabelForm {
    /// An LDH label that is not an XN-label: written as it is.
    Ldh,
    /// An A-label, with its U-label form.
    ALabel(Vec<char>),
    /// A U-label, with its Punycode encoding.
    ULabel(Vec<char>, String),
}

fn is_xn_label(label: &str) -> bool {
    label
        .get(..ACE_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(ACE_PREFIX))
}

fn label_form(label: &str, policy: Policy) -> Option<LabelForm> {
    if label.is_empty() {
        return None;
    }
    if label.is_ascii() {
        let bytes = label.as_bytes();
        if !bytes
            .iter()
            .all(|&b| b.is_ascii_alphanumeric() || b == b'-')
            || bytes[0] == b'-'
            || bytes[bytes.len() - 1] == b'-'
        {
            return None;
        }
        if is_xn_label(label) {
            return a_label(&label.to_ascii_lowercase()).map(LabelForm::ALabel);
        }
        // RFC 5890 §2.3.1: `--` in positions 3 and 4 makes a reserved LDH
        // label, which an IDN may not contain unless it is an A-label.
        if policy == Policy::Idna && bytes.len() >= 4 && &bytes[2..4] == b"--" {
            return None;
        }
        return Some(LabelForm::Ldh);
    }
    if policy == Policy::Hostname {
        return None;
    }
    let chars: Vec<char> = label.chars().collect();
    if !u_label(&chars) {
        return None;
    }
    let encoded = punycode::encode(&chars)?;
    Some(LabelForm::ULabel(chars, encoded))
}

/// The U-label an A-label (already lower case, `xn--` prefixed) stands for:
/// its Punycode decodes, contains non-ASCII (RFC 5890 §2.3.2.1), is a valid
/// U-label, and re-encodes to exactly the input.
fn a_label(lower: &str) -> Option<Vec<char>> {
    let body = &lower[ACE_PREFIX.len()..];
    let decoded = punycode::decode(body)?;
    if decoded.iter().all(char::is_ascii) || !u_label(&decoded) {
        return None;
    }
    (punycode::encode(&decoded)? == body).then_some(decoded)
}

/// RFC 5891 §4.2 / §5.4 validity of one U-label candidate.
fn u_label(chars: &[char]) -> bool {
    let (Some(&first), Some(&last)) = (chars.first(), chars.last()) else {
        return false;
    };
    // §4.2.3.1 hyphen restrictions.
    if first == '-' || last == '-' || (chars.len() >= 4 && chars[2] == '-' && chars[3] == '-') {
        return false;
    }
    // §4.2.3.2: no leading combining mark.
    if lookup(MARK, first) {
        return false;
    }
    // §4.2.2 and §4.2.3.3: every code point PVALID, or contextual and
    // positively confirmed by its Appendix A rule.
    for (index, &c) in chars.iter().enumerate() {
        let allowed = match lookup(DERIVED, c) {
            Derived::Pvalid => true,
            Derived::ContextJ => context_j(chars, index),
            Derived::ContextO => context_o(chars, index),
            Derived::Disallowed | Derived::Unassigned => false,
        };
        if !allowed {
            return false;
        }
    }
    // §5.4: the label is in NFC.
    is_nfc(&chars.iter().collect::<String>())
}

/// RFC 5892 Appendix A.1 (ZERO WIDTH NON-JOINER) and A.2 (ZERO WIDTH JOINER).
fn context_j(chars: &[char], index: usize) -> bool {
    let before = index.checked_sub(1).map(|i| chars[i]);
    if before.is_some_and(|b| ccc(b) == VIRAMA) {
        return true;
    }
    if chars[index] != ZWNJ {
        return false;
    }
    // (Joining_Type:{L,D})(Joining_Type:T)*\u200C(Joining_Type:T)*(Joining_Type:{R,D})
    let left = chars[..index]
        .iter()
        .rev()
        .map(|&c| lookup(JOINING, c))
        .find(|&j| j != JoiningType::Transparent);
    let right = chars[index + 1..]
        .iter()
        .map(|&c| lookup(JOINING, c))
        .find(|&j| j != JoiningType::Transparent);
    matches!(left, Some(JoiningType::Left | JoiningType::Dual))
        && matches!(right, Some(JoiningType::Right | JoiningType::Dual))
}

/// RFC 5892 Appendix A.3–A.9, evaluated in full.
fn context_o(chars: &[char], index: usize) -> bool {
    let before = index.checked_sub(1).map(|i| chars[i]);
    let after = chars.get(index + 1).copied();
    match chars[index] {
        // A.3 MIDDLE DOT.
        '\u{00B7}' => before == Some('l') && after == Some('l'),
        // A.4 GREEK LOWER NUMERAL SIGN (KERAIA).
        '\u{0375}' => after.is_some_and(|a| lookup(SCRIPT, a) == Script::Greek),
        // A.5 HEBREW PUNCTUATION GERESH, A.6 GERSHAYIM.
        '\u{05F3}' | '\u{05F4}' => before.is_some_and(|b| lookup(SCRIPT, b) == Script::Hebrew),
        // A.7 KATAKANA MIDDLE DOT.
        '\u{30FB}' => chars.iter().any(|&c| {
            matches!(
                lookup(SCRIPT, c),
                Script::Hiragana | Script::Katakana | Script::Han
            )
        }),
        // A.8 ARABIC-INDIC DIGITS.
        '\u{0660}'..='\u{0669}' => !chars.iter().any(|c| ('\u{06F0}'..='\u{06F9}').contains(c)),
        // A.9 EXTENDED ARABIC-INDIC DIGITS.
        '\u{06F0}'..='\u{06F9}' => !chars.iter().any(|c| ('\u{0660}'..='\u{0669}').contains(c)),
        // A CONTEXTO code point with no rule is refused (RFC 5891 §5.4).
        _ => false,
    }
}

/// Whether a label is an RTL label (RFC 5893 §1.4): it contains R, AL or AN.
fn has_rtl(chars: &[char]) -> bool {
    chars.iter().any(|&c| {
        matches!(
            lookup(BIDI, c),
            Bidi::Right | Bidi::ArabicLetter | Bidi::ArabicNumber
        )
    })
}

/// The six conditions of the RFC 5893 §2 Bidi rule, for one label.
fn bidi_rule(chars: &[char]) -> bool {
    let classes: Vec<Bidi> = chars.iter().map(|&c| lookup(BIDI, c)).collect();
    // Condition 1.
    if !matches!(
        classes.first(),
        Some(Bidi::Left | Bidi::Right | Bidi::ArabicLetter)
    ) {
        return false;
    }
    let end = classes.iter().rev().find(|&&b| b != Bidi::NonspacingMark);
    if has_rtl(chars) {
        // Condition 2.
        let allowed = classes.iter().all(|b| {
            matches!(
                b,
                Bidi::Right
                    | Bidi::ArabicLetter
                    | Bidi::ArabicNumber
                    | Bidi::EuropeanNumber
                    | Bidi::EuropeanSeparator
                    | Bidi::CommonSeparator
                    | Bidi::EuropeanTerminator
                    | Bidi::OtherNeutral
                    | Bidi::BoundaryNeutral
                    | Bidi::NonspacingMark
            )
        });
        // Condition 3.
        let ends = matches!(
            end,
            Some(Bidi::Right | Bidi::ArabicLetter | Bidi::EuropeanNumber | Bidi::ArabicNumber)
        );
        // Condition 4.
        let digits_mixed =
            classes.contains(&Bidi::EuropeanNumber) && classes.contains(&Bidi::ArabicNumber);
        allowed && ends && !digits_mixed
    } else {
        // Condition 5.
        let allowed = classes.iter().all(|b| {
            matches!(
                b,
                Bidi::Left
                    | Bidi::EuropeanNumber
                    | Bidi::EuropeanSeparator
                    | Bidi::CommonSeparator
                    | Bidi::EuropeanTerminator
                    | Bidi::OtherNeutral
                    | Bidi::BoundaryNeutral
                    | Bidi::NonspacingMark
            )
        });
        // Condition 6.
        let ends = matches!(end, Some(Bidi::Left | Bidi::EuropeanNumber));
        allowed && ends
    }
}

// ---- Table lookups -------------------------------------------------------------

/// The value of the run containing `c` in a `(start, value)` table that
/// starts at code point 0.
fn lookup<T: Copy>(table: &[(u32, T)], c: char) -> T {
    let after = table.partition_point(|&(start, _)| start <= u32::from(c));
    table[after - 1].1
}

fn lookup_mapping(table: &'static [(char, &'static [char])], c: char) -> Option<&'static [char]> {
    table
        .binary_search_by_key(&c, |&(key, _)| key)
        .ok()
        .map(|i| table[i].1)
}

fn full_stop(c: char) -> char {
    if c == '\u{3002}' { '.' } else { c }
}

// ---- Punycode (RFC 3492 §5, §6) -------------------------------------------------

mod punycode {
    const BASE: u32 = 36;
    const TMIN: u32 = 1;
    const TMAX: u32 = 26;
    const SKEW: u32 = 38;
    const DAMP: u32 = 700;
    const INITIAL_BIAS: u32 = 72;
    const INITIAL_N: u32 = 0x80;
    const DELIMITER: char = '-';

    /// §6.1 bias adaptation.
    fn adapt(delta: u32, numpoints: u32, first_time: bool) -> u32 {
        let mut delta = if first_time { delta / DAMP } else { delta / 2 };
        delta += delta / numpoints;
        let mut k = 0;
        while delta > ((BASE - TMIN) * TMAX) / 2 {
            delta /= BASE - TMIN;
            k += BASE;
        }
        k + ((BASE - TMIN + 1) * delta) / (delta + SKEW)
    }

    /// The threshold `t` for position `k` under `bias` (§6.2, §6.3).
    fn threshold(k: u32, bias: u32) -> u32 {
        if k <= bias {
            TMIN
        } else if k >= bias + TMAX {
            TMAX
        } else {
            k - bias
        }
    }

    fn digit_value(byte: u8) -> Option<u32> {
        match byte {
            b'a'..=b'z' => Some(u32::from(byte - b'a')),
            b'A'..=b'Z' => Some(u32::from(byte - b'A')),
            b'0'..=b'9' => Some(u32::from(byte - b'0') + 26),
            _ => None,
        }
    }

    fn digit_char(digit: u32) -> char {
        debug_assert!(digit < BASE, "a Punycode digit is below the base");
        let byte = if digit < 26 {
            b'a' + digit as u8
        } else {
            b'0' + (digit - 26) as u8
        };
        char::from(byte)
    }

    /// §6.2 decoding.
    pub(super) fn decode(input: &str) -> Option<Vec<char>> {
        if !input.is_ascii() {
            return None;
        }
        // Everything before the last delimiter is basic code points, copied;
        // the delimiter is consumed only when something preceded it.
        let (basic, extended) = match input.rfind(DELIMITER) {
            Some(at) if at > 0 => (&input[..at], &input[at + 1..]),
            _ => ("", input),
        };
        let mut output: Vec<char> = basic.chars().collect();
        let mut n = INITIAL_N;
        let mut i: u32 = 0;
        let mut bias = INITIAL_BIAS;
        let mut digits = extended.bytes();
        let mut remaining = extended.len();
        while remaining > 0 {
            let old_i = i;
            let mut w: u32 = 1;
            let mut k = BASE;
            loop {
                let digit = digit_value(digits.next()?)?;
                remaining -= 1;
                i = i.checked_add(digit.checked_mul(w)?)?;
                let t = threshold(k, bias);
                if digit < t {
                    break;
                }
                w = w.checked_mul(BASE - t)?;
                k += BASE;
            }
            let length = u32::try_from(output.len()).ok()?.checked_add(1)?;
            bias = adapt(i - old_i, length, old_i == 0);
            n = n.checked_add(i / length)?;
            i %= length;
            if n < INITIAL_N {
                return None;
            }
            output.insert(i as usize, char::from_u32(n)?);
            i += 1;
        }
        Some(output)
    }

    /// §6.3 encoding.
    pub(super) fn encode(input: &[char]) -> Option<String> {
        let mut output: String = input.iter().filter(|c| c.is_ascii()).collect();
        let basic = u32::try_from(output.len()).ok()?;
        let length = u32::try_from(input.len()).ok()?;
        if basic > 0 {
            output.push(DELIMITER);
        }
        let mut n = INITIAL_N;
        let mut delta: u32 = 0;
        let mut bias = INITIAL_BIAS;
        let mut handled = basic;
        while handled < length {
            let m = input
                .iter()
                .map(|&c| u32::from(c))
                .filter(|&c| c >= n)
                .min()?;
            delta = delta.checked_add((m - n).checked_mul(handled + 1)?)?;
            n = m;
            for &c in input {
                let c = u32::from(c);
                if c < n {
                    delta = delta.checked_add(1)?;
                }
                if c == n {
                    let mut q = delta;
                    let mut k = BASE;
                    loop {
                        let t = threshold(k, bias);
                        if q < t {
                            break;
                        }
                        output.push(digit_char(t + (q - t) % (BASE - t)));
                        q = (q - t) / (BASE - t);
                        k += BASE;
                    }
                    output.push(digit_char(q));
                    bias = adapt(delta, handled + 1, handled == basic);
                    delta = 0;
                    handled += 1;
                }
            }
            delta = delta.checked_add(1)?;
            n = n.checked_add(1)?;
        }
        Some(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_start_at_zero_and_are_sorted() {
        fn check<T>(name: &str, table: &[(u32, T)]) {
            assert_eq!(table.first().map(|r| r.0), Some(0), "{name}");
            assert!(table.windows(2).all(|w| w[0].0 < w[1].0), "{name}");
        }
        check("DERIVED", DERIVED);
        check("JOINING", JOINING);
        check("BIDI", BIDI);
        check("SCRIPT", SCRIPT);
        check("MARK", MARK);
        assert!(NFKC_CASEFOLD.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn rfc_5892_exceptions_are_in_the_table() {
        for c in [
            '\u{DF}', '\u{3C2}', '\u{6FD}', '\u{6FE}', '\u{F0B}', '\u{3007}',
        ] {
            assert_eq!(lookup(DERIVED, c), Derived::Pvalid, "{c:?}");
        }
        for c in [
            '\u{B7}', '\u{375}', '\u{5F3}', '\u{5F4}', '\u{30FB}', '\u{660}', '\u{6F9}',
        ] {
            assert_eq!(lookup(DERIVED, c), Derived::ContextO, "{c:?}");
        }
        for c in [
            '\u{640}', '\u{7FA}', '\u{302E}', '\u{302F}', '\u{3031}', '\u{303B}',
        ] {
            assert_eq!(lookup(DERIVED, c), Derived::Disallowed, "{c:?}");
        }
        assert_eq!(lookup(DERIVED, ZWNJ), Derived::ContextJ);
        assert_eq!(lookup(DERIVED, '\u{200D}'), Derived::ContextJ);
    }

    #[test]
    fn contextual_rules_refuse_beside_accepting_neighbours() {
        let label = |s: &str| u_label(&s.chars().collect::<Vec<_>>());
        assert!(label("l\u{b7}l") && !label("a\u{b7}l") && !label("l\u{b7}"));
        assert!(label("\u{3b1}\u{375}\u{3b2}") && !label("\u{3b1}\u{375}s"));
        assert!(label("\u{5d0}\u{5f3}\u{5d1}") && !label("a\u{5f3}\u{5d1}"));
        assert!(label("\u{30fb}\u{3041}") && !label("def\u{30fb}abc"));
        assert!(label("\u{628}\u{660}\u{628}") && !label("\u{628}\u{660}\u{6f0}"));
        assert!(label("\u{915}\u{94d}\u{200d}\u{937}") && !label("\u{915}\u{200d}\u{937}"));
        assert!(label("\u{628}\u{64a}\u{200c}\u{628}\u{64a}") && !label("a\u{200c}b"));
    }
}
