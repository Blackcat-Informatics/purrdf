// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact label alphabets for the syntax this workspace *emits* -- blank-node
//! labels, XML `NCName`s, XML character data -- plus the deterministic,
//! injective escape ([`escape_label`]) every serializer applies so an
//! out-of-alphabet label never becomes an unreadable document.
//!
//! # Ingress-liberal, egress-exact
//!
//! The workspace's first-party SPARQL/Turtle lexer
//! (`crates/sparql-algebra/src/lexer.rs`, `is_pn_chars`/`is_pn_chars_base`)
//! deliberately over-accepts on *parse*: every scalar above `0x7F` passes,
//! rather than the exact ranges the grammar names for `PN_CHARS_BASE`. That
//! approximation is sound on ingress -- a superset lexer can only accept
//! documents a conforming parser would also accept, never reject one, and it
//! keeps the lexer's hot path a single branch. It is unsound on *egress*: a
//! label this workspace writes must be re-readable by every external
//! conforming parser, which implements the grammar's exact Unicode ranges,
//! not this workspace's liberal approximation. This module is therefore the
//! exact egress contract for label syntax -- the ranges below are transcribed
//! verbatim from the W3C Turtle/SPARQL `PN_CHARS_BASE`/`PN_CHARS` productions
//! and the XML 1.0 `NameStartChar`/`NameChar`/`Char` productions, not
//! approximated.
//!
//! # The escape contract: serialization stays total
//!
//! A blank-node label is *not* part of a graph's meaning: RDF identifies
//! blank nodes only up to renaming, so swapping `_:x7` for another label is
//! an isomorphism-preserving operation -- every triple's meaning, and the
//! whole graph up to blank-node renaming, is identical before and after.
//! Serializers therefore never refuse a label: [`escape_label`] rewrites an
//! out-of-alphabet label into the target syntax's alphabet at egress, so
//! `parse` -> `serialize` is **total** in every format this workspace emits.
//!
//! The escape is:
//!
//! - **Deterministic** -- a pure function of `(label, alphabet)`, with no
//!   clock, randomness, hash-iteration order, or document-level state, so the
//!   same dataset always serializes to the same bytes.
//! - **Injective by construction** -- distinct labels always escape to
//!   distinct labels, and no *legal* label can collide with the escape of an
//!   illegal one, because a legal label that itself begins with the reserved
//!   [`ESCAPE_MARKER`] is escaped too (see [`escape_label`]). Co-reference is
//!   therefore preserved exactly: two occurrences of one blank node stay one
//!   blank node, and two distinct blank nodes stay distinct.
//! - **Stateless and streaming-safe** -- no per-document collision map is
//!   needed, which is what lets the row-at-a-time SPARQL-results writers use
//!   the same escape as the whole-dataset RDF serializers.
//! - **Pass-through for legal labels** -- a label already legal under the
//!   target alphabet is returned borrowed and byte-identical, so real data
//!   never churns.
//!
//! Injectivity and *idempotence* cannot both hold, and injectivity is the one
//! that carries meaning. An idempotent escape would satisfy
//! `escape(escape(x)) == escape(x)`, which maps the two distinct labels `x`
//! and `escape(x)` onto one -- exactly the blank-node conflation injectivity
//! exists to rule out.
//!
//! # The marker is a reserved namespace, decoded at ingress
//!
//! An injective, non-idempotent egress transform is byte-stable across
//! serialize/parse cycles only if something INVERTS it on the way back in, and
//! [`unescape_label`] is that inverse: every native text codec decodes a
//! blank-node token through it (together with
//! [`BlankScope::unqualify_label`](crate::BlankScope::unqualify_label), the
//! inverse of the scope qualification the escape is applied on top of) before
//! interning. [`ESCAPE_MARKER`] is therefore a RESERVED label namespace: a
//! document token that begins with it and whose body is a well-formed escape
//! body denotes the label it decodes to, not itself. A marker-prefixed token
//! whose body is NOT well-formed is not in the escape's image at all, so it
//! passes through unchanged and is escaped on the way out like any other
//! label -- its bytes can change on the first serialization, and are a fixed
//! point from then on.
//!
//! The composite consequence, which is what callers actually rely on:
//! `serialize(parse(serialize(D)))` is byte-identical to `serialize(D)` for
//! every dataset and every text format with a parser, and a well-formed round
//! trip restores blank-node label IDENTITY, not merely isomorphism.
//!
//! Callers that want a *chosen* relabeling rather than a mechanical escape
//! have explicit recourse operations -- `canonical_relabel`, `skolemize` and
//! `deskolemize` in [`crate::ir`] -- which rewrite the dataset before egress
//! so the labels in the document are the caller's, not the escape's.
//! Canonicalization's `c14n{n}` labels are legal in every alphabet, so the
//! escape is the identity on them and their bytes never move.
//!
//! # Relabeling vs. canonicalization: consistent doctrines, different roles
//!
//! RDFC-1.0 canonicalization refuses to relabel for a different reason: its
//! output bytes are not a rendering choice, they mint the dataset's
//! content-addressed identity (two isomorphic graphs must canonicalize to
//! byte-identical output, and two non-isomorphic graphs must not collide).
//! Relabeling a canonical label to dodge an alphabet constraint would be
//! indistinguishable, downstream, from silently changing which graph the
//! identity was computed over. It never needs to: canonicalization mints
//! `c14n0`, `c14n1`, … labels (see [`crate::ir::canon`]) whose ASCII letters
//! and digits are a subset of every alphabet this module defines, so a
//! canonical label is legal everywhere and [`escape_label`] is the identity
//! on it.

use core::cmp::Ordering;
use std::borrow::Cow;

use crate::BlankScope;

/// Which label grammar [`is_valid_label`] should check against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelAlphabet {
    /// The W3C Turtle/SPARQL `BLANK_NODE_LABEL` production (the part after
    /// `_:`); see [`is_valid_blank_node_label`]. The alphabet of the Turtle
    /// family (Turtle / TriG / N-Triples / N-Quads) and of every syntax whose
    /// specification types a blank-node identifier as a `BLANK_NODE_LABEL`
    /// rather than as free text (HexTuples, JSON-LD, YAML-LD, the SPARQL
    /// result formats).
    BlankNodeLabel,
    /// The XML 1.0 `NCName` production; see [`is_valid_ncname`]. The alphabet
    /// of the RDF/XML `rdf:nodeID` attribute.
    NcName,
    /// Non-empty text that XML 1.0 character data carries through unchanged;
    /// see [`is_valid_xml_text`]. The alphabet of the TriX `<id>` element.
    XmlText,
}

/// The reserved prefix [`escape_label`] writes in front of every escaped
/// label. A label that begins with it is always escaped -- even when it is
/// otherwise legal -- which is what makes the escape injective without any
/// document-level state.
pub const ESCAPE_MARKER: &str = "purrdfesc_";

/// Whether `label` is legal under `alphabet`.
///
/// Dispatches to [`is_valid_blank_node_label`], [`is_valid_ncname`], or
/// [`is_valid_xml_text`], per [`LabelAlphabet`].
#[must_use]
pub fn is_valid_label(label: &str, alphabet: LabelAlphabet) -> bool {
    match alphabet {
        LabelAlphabet::BlankNodeLabel => is_valid_blank_node_label(label),
        LabelAlphabet::NcName => is_valid_ncname(label),
        LabelAlphabet::XmlText => is_valid_xml_text(label),
    }
}

/// Rewrite `label` into `alphabet`, returning it BORROWED and byte-identical
/// when it is already legal.
///
/// This is the egress escape every serializer applies to a scope-qualified
/// blank-node label. It is a pure, deterministic, stateless function, and it
/// is **injective** for a fixed `alphabet`: distinct inputs always produce
/// distinct outputs, so blank-node co-reference survives serialization exactly
/// (relabeling a blank node preserves the graph up to isomorphism, which is
/// the only identity RDF gives a blank node).
///
/// # The encoding
///
/// An escaped label is [`ESCAPE_MARKER`] followed by the input encoded
/// character by character: an ASCII letter or digit passes through as itself,
/// and every other scalar becomes `_` plus its code point as exactly six
/// uppercase hex digits (`_0000D7` for `×`). The escaped body is therefore
/// always `[A-Za-z0-9_]*`, which is simultaneously legal `BLANK_NODE_LABEL`,
/// legal `NCName` and legal XML character data -- so one encoding satisfies
/// every alphabet in the strictest sense, including the `BLANK_NODE_LABEL`
/// rule that a label may not end in `.`.
///
/// Decoding is unambiguous (a `_` always introduces exactly six hex digits,
/// and no pass-through character is `_`), so the encoding is injective. The
/// escape *image* cannot collide with a pass-through label either, because a
/// label that already begins with [`ESCAPE_MARKER`] is escaped as well, even
/// when it is legal -- the one case where a legal label does not pass through.
#[must_use]
pub fn escape_label(label: &str, alphabet: LabelAlphabet) -> Cow<'_, str> {
    if is_valid_label(label, alphabet) && !label.starts_with(ESCAPE_MARKER) {
        return Cow::Borrowed(label);
    }
    let mut escaped = String::with_capacity(ESCAPE_MARKER.len() + label.len() + 8);
    escaped.push_str(ESCAPE_MARKER);
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            escaped.push(ch);
        } else {
            escaped.push('_');
            push_hex6(ch as u32, &mut escaped);
        }
    }
    debug_assert!(
        is_valid_label(&escaped, alphabet),
        "escape_label must always produce a label legal under the target alphabet"
    );
    Cow::Owned(escaped)
}

/// Decode an escaped label, returning it BORROWED and byte-identical when it is
/// not a well-formed escape.
///
/// The exact inverse of [`escape_label`] on that function's image:
/// `unescape_label(escape_label(label, alphabet)) == label` for every `label`
/// and every alphabet. This is the ingress half of the text round trip — every
/// native codec decodes a parsed blank-node token through it, so an escaped
/// label re-parses as the label it encodes rather than as a fresh
/// marker-prefixed one that egress would escape a second time.
///
/// # What counts as well formed
///
/// [`ESCAPE_MARKER`] followed by a body in which every ASCII letter or digit is
/// a pass-through and every `_` introduces exactly six UPPERCASE hex digits
/// naming a Unicode scalar — precisely the shape [`escape_label`] writes. A
/// marker-prefixed label whose body violates any of that (a short or
/// lowercase hex group, a non-alphanumeric pass-through, a group naming a
/// surrogate or an out-of-range code point) is NOT in the escape's image, so it
/// is returned unchanged.
#[must_use]
pub fn unescape_label(label: &str) -> Cow<'_, str> {
    let Some(body) = label.strip_prefix(ESCAPE_MARKER) else {
        return Cow::Borrowed(label);
    };
    let mut decoded = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch.is_ascii_alphanumeric() {
            decoded.push(ch);
            continue;
        }
        if ch != '_' {
            return Cow::Borrowed(label);
        }
        let mut cp: u32 = 0;
        for _ in 0..6 {
            let Some(digit) = chars.next().and_then(hex6_digit) else {
                return Cow::Borrowed(label);
            };
            cp = cp * 16 + digit;
        }
        let Some(scalar) = char::from_u32(cp) else {
            return Cow::Borrowed(label);
        };
        decoded.push(scalar);
    }
    Cow::Owned(decoded)
}

/// Decode a parsed blank-node token into the `(label, scope)` pair it denotes —
/// the single text-ingress inverse of the egress transform every serializer
/// applies.
///
/// Egress is `(label, scope)` → [`qualify_label`](BlankScope::qualify_label) →
/// [`escape_label`]; ingress is therefore [`unescape_label`] →
/// [`unqualify_label`](BlankScope::unqualify_label), in that order. Composed
/// with the egress pair this is the identity for every `(label, scope)` and
/// every alphabet, which is what makes a parse/serialize cycle byte-stable and
/// label-identity-preserving.
#[must_use]
pub fn decode_blank_label(token: &str) -> (Cow<'_, str>, BlankScope) {
    match unescape_label(token) {
        // The escape passed through, so the scope decode can borrow the token
        // itself and the whole ingress stays allocation-free for real data.
        Cow::Borrowed(unescaped) => BlankScope::unqualify_label(unescaped),
        Cow::Owned(unescaped) => {
            let (label, scope) = BlankScope::unqualify_label(&unescaped);
            (Cow::Owned(label.into_owned()), scope)
        }
    }
}

/// One digit of a fixed-width escape group: `0-9` or UPPERCASE `A-F`, matching
/// exactly what [`push_hex6`] writes. Lowercase is deliberately refused so the
/// decode accepts only the escape's own image.
fn hex6_digit(c: char) -> Option<u32> {
    match c {
        '0'..='9' => Some(c as u32 - '0' as u32),
        'A'..='F' => Some(c as u32 - 'A' as u32 + 10),
        _ => None,
    }
}

/// Append `cp` as exactly six uppercase hex digits (24 bits covers the whole
/// `0..=0x10FFFF` scalar range), the fixed-width escape body [`escape_label`]
/// writes after each `_`.
fn push_hex6(cp: u32, out: &mut String) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for nibble in (0..6).rev() {
        let index = ((cp >> (nibble * 4)) & 0xF) as usize;
        out.push(char::from(HEX[index]));
    }
}

/// Whether `label` is legal as a serialized blank-node label (`_:{label}`).
///
/// Implements the exact W3C Turtle/SPARQL production
/// `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')* PN_CHARS)?`,
/// validating the part after `_:`. A label that fails this check cannot be
/// emitted by any codec without producing a document that no conforming
/// parser (including PurRDF's own) can read back.
#[must_use]
pub fn is_valid_blank_node_label(label: &str) -> bool {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(is_pn_chars_u(first) || first.is_ascii_digit()) {
        return false;
    }
    // The grammar is `(PN_CHARS | '.')* PN_CHARS` after the first character:
    // any run of PN_CHARS/'.' is legal mid-label, but the *final* character
    // must be PN_CHARS (never '.'). Track whether the most recently accepted
    // character was a '.' and reject at the end if so.
    let mut trailing_dot = false;
    for ch in chars {
        if ch == '.' {
            trailing_dot = true;
        } else if is_pn_chars(ch) {
            trailing_dot = false;
        } else {
            return false;
        }
    }
    !trailing_dot
}

/// Whether `prefix` is legal as a mint-time PREFIX for [`is_valid_blank_node_label`]:
/// a string a caller can safely splice in front of every label an evaluator mints,
/// so `{prefix}{stem}{n}` (`stem` one of this workspace's fixed mint stems --
/// `c`, `bnode`, `lc` -- and `n` a decimal counter) is always a legal
/// `BLANK_NODE_LABEL`.
///
/// # Why this differs from [`is_valid_blank_node_label`]
///
/// A PREFIX never occupies the FINAL position of the label it seeds -- the mint
/// stem's first letter and the counter's digits always follow it -- so the
/// `BLANK_NODE_LABEL` grammar's "the last character is never `.`" rule does not
/// apply to a prefix's own last character; it applies to the *mint stem's* last
/// character instead, which is always a decimal digit and therefore always
/// legal. Every OTHER position in the prefix is exactly as constrained as it
/// would be inside a full label: the first character must be a legal
/// `BLANK_NODE_LABEL` lead (`PN_CHARS_U` or a digit) and every character after
/// it must be `PN_CHARS` or `.`.
///
/// The empty prefix is legal: it is exactly "no prefix", and every mint stem
/// already starts with a legal lead character on its own.
#[must_use]
pub fn is_valid_blank_node_label_prefix(prefix: &str) -> bool {
    let mut chars = prefix.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    if !(is_pn_chars_u(first) || first.is_ascii_digit()) {
        return false;
    }
    chars.all(|ch| is_pn_chars(ch) || ch == '.')
}

/// Whether `label` is a legal XML 1.0 `NCName`.
///
/// Implements the exact production `NCName ::= NCNameStartChar NCNameChar*`,
/// where `NCNameStartChar = NameStartChar - ':'` and `NCNameChar` is
/// `NCNameStartChar` plus `'-' | '.' | [0-9] | #xB7 | [#x0300-#x036F] |
/// [#x203F-#x2040]`. `NCNameStartChar` is character-for-character identical
/// to the Turtle/SPARQL `PN_CHARS_U` alphabet, so this reuses the same
/// range tables as [`is_valid_blank_node_label`]. Unlike a blank-node label,
/// an `NCName` MAY end in `.`: the grammar places no restriction on the
/// final character.
#[must_use]
pub fn is_valid_ncname(label: &str) -> bool {
    let mut chars = label.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !is_pn_chars_u(first) {
        return false;
    }
    chars.all(|ch| is_pn_chars(ch) || ch == '.')
}

/// Whether `label` survives XML 1.0 character data unchanged.
///
/// Requires a non-empty string in which every scalar is a legal XML 1.0 `Char`
/// (`#x9 | #xA | #xD | [#x20-#xD7FF] | [#xE000-#xFFFD] | [#x10000-#x10FFFF]`,
/// so C0 controls and `U+FFFE`/`U+FFFF` are excluded; surrogates are not
/// representable in a `str` at all) and no scalar is whitespace. XML has no
/// representation whatsoever for a C0 control -- not even a character
/// reference -- so such a label cannot be carried by an XML document at any
/// escaping level. Whitespace is excluded for a second reason: XML normalizes
/// line endings and collapses whitespace in attribute values, and this
/// workspace's TriX reader trims `<id>` element text, so a whitespace-bearing
/// label cannot carry identity through an XML round trip even though XML can
/// represent the characters.
#[must_use]
pub fn is_valid_xml_text(label: &str) -> bool {
    !label.is_empty() && label.chars().all(is_xml_text_char)
}

/// One scalar of [`is_valid_xml_text`]: a legal XML 1.0 `Char` that is not
/// whitespace. `#x9`/`#xA`/`#xD` are legal `Char`s but are whitespace, so the
/// whitespace test alone removes them from the `[#x20-…]` gap below.
fn is_xml_text_char(c: char) -> bool {
    !c.is_whitespace()
        && matches!(c as u32, 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x0010_FFFF)
}

/// Inclusive Unicode scalar-value range `[lo, hi]`.
type CharRange = (u32, u32);

/// `PN_CHARS_BASE` from the W3C Turtle/SPARQL grammar, which is also
/// character-for-character the XML 1.0 `NameStartChar` production minus
/// `':'` and `'_'` (`'_'` is folded into `PN_CHARS_U` instead, see
/// [`is_pn_chars_u`]). Ranges are sorted and non-overlapping, which
/// [`in_ranges`] relies on for binary search.
const PN_CHARS_BASE_RANGES: &[CharRange] = &[
    (0x0041, 0x005A),   // [A-Z]
    (0x0061, 0x007A),   // [a-z]
    (0x00C0, 0x00D6),   // [#xC0-#xD6]
    (0x00D8, 0x00F6),   // [#xD8-#xF6]
    (0x00F8, 0x02FF),   // [#xF8-#x2FF]
    (0x0370, 0x037D),   // [#x370-#x37D]
    (0x037F, 0x1FFF),   // [#x37F-#x1FFF]
    (0x200C, 0x200D),   // [#x200C-#x200D]
    (0x2070, 0x218F),   // [#x2070-#x218F]
    (0x2C00, 0x2FEF),   // [#x2C00-#x2FEF]
    (0x3001, 0xD7FF),   // [#x3001-#xD7FF]
    (0xF900, 0xFDCF),   // [#xF900-#xFDCF]
    (0xFDF0, 0xFFFD),   // [#xFDF0-#xFFFD]
    (0x10000, 0xEFFFF), // [#x10000-#xEFFFF]
];

/// The extra ranges `PN_CHARS`/`NCNameChar` fold in beyond `PN_CHARS_U`
/// (beyond `'-'` and `[0-9]`, which are cheap ASCII checks handled inline).
/// Sorted and non-overlapping for [`in_ranges`].
const PN_CHARS_EXTRA_RANGES: &[CharRange] = &[
    (0x00B7, 0x00B7), // #xB7
    (0x0300, 0x036F), // [#x300-#x36F]
    (0x203F, 0x2040), // [#x203F-#x2040]
];

/// Binary-search `cp` against a sorted, non-overlapping table of inclusive
/// ranges.
fn in_ranges(cp: u32, ranges: &[CharRange]) -> bool {
    ranges
        .binary_search_by(|&(lo, hi)| {
            if cp < lo {
                Ordering::Greater
            } else if cp > hi {
                Ordering::Less
            } else {
                Ordering::Equal
            }
        })
        .is_ok()
}

/// `PN_CHARS_BASE` (== XML `NameStartChar - ':' - '_'`).
fn is_pn_chars_base(c: char) -> bool {
    in_ranges(c as u32, PN_CHARS_BASE_RANGES)
}

/// `PN_CHARS_U ::= PN_CHARS_BASE | '_'` (== `NCNameStartChar`).
fn is_pn_chars_u(c: char) -> bool {
    c == '_' || is_pn_chars_base(c)
}

/// `PN_CHARS ::= PN_CHARS_U | '-' | [0-9] | #xB7 | [#x300-#x036F] |
/// [#x203F-#x2040]`.
fn is_pn_chars(c: char) -> bool {
    is_pn_chars_u(c) || c == '-' || c.is_ascii_digit() || in_ranges(c as u32, PN_CHARS_EXTRA_RANGES)
}

#[cfg(test)]
mod tests {
    use super::{
        ESCAPE_MARKER, LabelAlphabet, decode_blank_label, escape_label, is_pn_chars, is_pn_chars_u,
        is_valid_blank_node_label, is_valid_blank_node_label_prefix, is_valid_label,
        is_valid_ncname, is_valid_xml_text, unescape_label,
    };
    use crate::BlankScope;
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    /// Every alphabet a serializer targets, for sweeps that must hold on all
    /// of them.
    const ALL_ALPHABETS: &[LabelAlphabet] = &[
        LabelAlphabet::BlankNodeLabel,
        LabelAlphabet::NcName,
        LabelAlphabet::XmlText,
    ];

    /// Adversarial labels: control characters, whitespace, delimiters, the
    /// alphabet boundary gaps, non-ASCII letters, the empty label, and labels
    /// that collide with the reserved escape marker.
    const HOSTILE_LABELS: &[&str] = &[
        "",
        "a",
        "0abc",
        "a.b",
        "trailing.",
        "-lead",
        ".lead",
        "a b",
        "a\tb",
        "a\nb",
        "bad\u{1f}label",
        "\u{7f}",
        "<urn:x>",
        "a:b",
        "a/b",
        "\u{d7}y",
        "日本",
        "c14n0",
        "purrdfesc_a",
        "purrdfesc_",
        "purrdfesc_a_000020b",
    ];

    /// Independent re-transcription of `PN_CHARS_BASE` via `matches!` range
    /// patterns, kept deliberately separate from the production binary-search
    /// table so the exhaustive sweep below cross-checks the table rather than
    /// restating it.
    fn expected_pn_chars_base(cp: u32) -> bool {
        matches!(
            cp,
            0x0041..=0x005A
                | 0x0061..=0x007A
                | 0x00C0..=0x00D6
                | 0x00D8..=0x00F6
                | 0x00F8..=0x02FF
                | 0x0370..=0x037D
                | 0x037F..=0x1FFF
                | 0x200C..=0x200D
                | 0x2070..=0x218F
                | 0x2C00..=0x2FEF
                | 0x3001..=0xD7FF
                | 0xF900..=0xFDCF
                | 0xFDF0..=0xFFFD
                | 0x10000..=0xEFFFF
        )
    }

    fn expected_pn_chars_u(cp: u32) -> bool {
        cp == 0x5F || expected_pn_chars_base(cp)
    }

    fn expected_pn_chars(cp: u32) -> bool {
        expected_pn_chars_u(cp)
            || cp == 0x2D
            || (0x30..=0x39).contains(&cp)
            || cp == 0xB7
            || (0x0300..=0x036F).contains(&cp)
            || (0x203F..=0x2040).contains(&cp)
    }

    #[test]
    fn internal_tables_match_independent_transcription() {
        for cp in 0u32..=0x2FFF {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            assert_eq!(
                is_pn_chars_u(c),
                expected_pn_chars_u(cp),
                "pn_chars_u mismatch at {cp:#06x}"
            );
            assert_eq!(
                is_pn_chars(c),
                expected_pn_chars(cp),
                "pn_chars mismatch at {cp:#06x}"
            );
        }
    }

    /// Exhaustive sweep of all 256 single-byte scalar values, in both the
    /// leading position (a one-character label) and an inner/final position
    /// (`"a" + c`), against the [`LabelAlphabet::BlankNodeLabel`] alphabet.
    #[test]
    fn ascii_sweep_blank_node_label() {
        for byte in 0u32..=255 {
            let c = char::from_u32(byte).expect("0..=255 are always valid Unicode scalars");
            let lead = c.to_string();
            let expected_lead = expected_pn_chars_u(byte) || c.is_ascii_digit();
            assert_eq!(
                is_valid_blank_node_label(&lead),
                expected_lead,
                "lead byte {byte:#04x} ({c:?})"
            );

            // As the final character of "a{c}", the grammar requires PN_CHARS
            // (never '.') -- so the expected verdict is `expected_pn_chars`,
            // not `expected_pn_chars(byte) || c == '.'`.
            let inner = format!("a{c}");
            let expected_inner = expected_pn_chars(byte);
            assert_eq!(
                is_valid_blank_node_label(&inner),
                expected_inner,
                "inner byte {byte:#04x} ({c:?})"
            );
        }
    }

    /// Exhaustive sweep of all 256 single-byte scalar values against the
    /// [`LabelAlphabet::NcName`] alphabet.
    #[test]
    fn ascii_sweep_ncname() {
        for byte in 0u32..=255 {
            let c = char::from_u32(byte).expect("0..=255 are always valid Unicode scalars");
            let lead = c.to_string();
            let expected_lead = expected_pn_chars_u(byte);
            assert_eq!(
                is_valid_ncname(&lead),
                expected_lead,
                "lead byte {byte:#04x} ({c:?})"
            );

            // Unlike BlankNodeLabel, NCNameChar (unconditionally) permits '.',
            // including as the final character.
            let inner = format!("a{c}");
            let expected_inner = expected_pn_chars(byte) || c == '.';
            assert_eq!(
                is_valid_ncname(&inner),
                expected_inner,
                "inner byte {byte:#04x} ({c:?})"
            );
        }
    }

    #[test]
    fn unicode_boundary_accepted_blank_node_label() {
        for label in [
            "ü", "日本", "\u{200C}", // ZWNJ, PN_CHARS_BASE
            "a\u{B7}",  // middle dot, PN_CHARS-only, inner position
            "a\u{36F}", // combining mark, PN_CHARS-only, inner position
        ] {
            assert!(is_valid_blank_node_label(label), "{label:?}");
        }
    }

    #[test]
    fn unicode_boundary_rejected_blank_node_label() {
        for label in [
            "\u{D7}",    // × -- the gap just past [#xC0-#xD6]
            "\u{F7}",    // ÷ -- the gap between [#xD8-#xF6] and [#xF8-#x2FF]
            "\u{37E}",   // Greek question mark -- the gap in [#x370-#x37D]/[#x37F-#x1FFF]
            "a\u{2002}", // en-space, not in any PN_CHARS range
            "\u{2028}",  // line separator, not in any PN_CHARS_BASE range
        ] {
            assert!(!is_valid_blank_node_label(label), "{label:?}");
        }
    }

    #[test]
    fn structural_blank_node_label() {
        for label in [
            "0b",
            "b0",
            "_x",
            "a-b.c",
            "a_b",
            "c1.s5",
            "f-3c_c1",
            "ünïcode",
            "日本",
        ] {
            assert!(is_valid_blank_node_label(label), "{label:?}");
        }
        for label in [
            "",
            "\u{1f}",
            "bad\u{1f}label",
            "<urn:x>",
            "a:b",
            "a b",
            "a/b",
            "trailing.", // trailing '.' is disallowed for BLANK_NODE_LABEL specifically
            "-lead",
            ".lead",
            "a\nb",
        ] {
            assert!(!is_valid_blank_node_label(label), "{label:?}");
        }
    }

    #[test]
    fn structural_blank_node_label_prefix() {
        // Legal prefixes, including ones a full-label check would reject:
        // empty (no prefix), and a trailing '.' (illegal as a FULL label's
        // last character, legal here because the mint stem always follows).
        for prefix in [
            "",
            "f",
            "fTag_",
            "f-3c1a2d_",
            "trailing.",
            "0abc",
            "日本",
            "_x",
        ] {
            assert!(is_valid_blank_node_label_prefix(prefix), "{prefix:?}");
        }
        // Illegal prefixes: a bad lead character, or a body character outside
        // PN_CHARS/'.'.
        for prefix in ["-lead", ".lead", "a b", "a\tb", "a:b", "<urn:x>", "a/b"] {
            assert!(!is_valid_blank_node_label_prefix(prefix), "{prefix:?}");
        }
    }

    /// Every legal prefix, concatenated in front of every one of this
    /// workspace's fixed mint stems plus a decimal counter, must produce a
    /// full legal `BLANK_NODE_LABEL` -- the property the prefix check exists
    /// to guarantee.
    #[test]
    fn legal_prefix_always_yields_a_legal_minted_label() {
        const STEMS: &[&str] = &["c", "bnode", "lc"];
        for prefix in ["", "f", "fTag_", "f-3c1a2d_", "trailing.", "0abc", "日本"] {
            assert!(is_valid_blank_node_label_prefix(prefix), "{prefix:?}");
            for stem in STEMS {
                for n in [0u64, 1, 42] {
                    let minted = format!("{prefix}{stem}{n}");
                    assert!(
                        is_valid_blank_node_label(&minted),
                        "prefix {prefix:?} + stem {stem:?} + {n} = {minted:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn structural_ncname() {
        for label in [
            "b0",
            "_x",
            "a-b.c",
            "a_b",
            "c1.s5",
            "f-3c_c1",
            "ünïcode",
            "日本",
        ] {
            assert!(is_valid_ncname(label), "{label:?}");
        }
        for label in [
            "",
            "\u{1f}",
            "bad\u{1f}label",
            "<urn:x>",
            "a:b",
            "a b",
            "a/b",
            "-lead",
            ".lead",
            "a\nb",
            "0b", // NCNameStartChar excludes digits, unlike PN_CHARS_U | [0-9]
        ] {
            assert!(!is_valid_ncname(label), "{label:?}");
        }
        // Unlike BLANK_NODE_LABEL, NCName places no constraint on the final
        // character: a trailing '.' is syntactically legal.
        assert!(is_valid_ncname("trailing."));
    }

    #[test]
    fn dispatch_via_is_valid_label() {
        assert!(is_valid_label("a-b.c", LabelAlphabet::BlankNodeLabel));
        assert!(!is_valid_label("0b", LabelAlphabet::NcName));
        assert!(is_valid_label("b0", LabelAlphabet::NcName));
        assert!(is_valid_label("<urn:x>", LabelAlphabet::XmlText));
    }

    #[test]
    fn xml_text_admits_representable_non_whitespace_and_refuses_the_rest() {
        for label in ["a", "<urn:x>", "a.b", "trailing.", "日本", "\u{d7}y", "&"] {
            assert!(is_valid_xml_text(label), "{label:?}");
        }
        for label in [
            "",
            "a b",
            "a\tb",
            "a\nb",
            "a\rb",
            "bad\u{1f}label",
            "\u{fffe}",
            "\u{ffff}",
            "\u{85}", // NEL: whitespace
            "\u{a0}", // NBSP: whitespace
        ] {
            assert!(!is_valid_xml_text(label), "{label:?}");
        }
        // U+007F is a legal XML 1.0 Char (the exclusion is C0, not DEL).
        assert!(is_valid_xml_text("\u{7f}"));
    }

    // ── escape_label ────────────────────────────────────────────────────────

    #[test]
    fn legal_labels_are_borrowed_byte_identically() {
        // The one legal label that does NOT pass through is a label carrying
        // the reserved marker, which must be escaped away from the image.
        let legal: &[(&str, LabelAlphabet)] = &[
            ("alpha", LabelAlphabet::BlankNodeLabel),
            ("beta.s2", LabelAlphabet::BlankNodeLabel),
            ("0abc", LabelAlphabet::BlankNodeLabel),
            ("日本", LabelAlphabet::BlankNodeLabel),
            ("c14n0", LabelAlphabet::BlankNodeLabel),
            ("alpha", LabelAlphabet::NcName),
            ("trailing.", LabelAlphabet::NcName),
            ("<urn:x>", LabelAlphabet::XmlText),
        ];
        for &(label, alphabet) in legal {
            let escaped = escape_label(label, alphabet);
            assert!(
                matches!(escaped, Cow::Borrowed(_)),
                "{label:?} under {alphabet:?} must pass through borrowed"
            );
            assert_eq!(escaped, label);
        }
    }

    #[test]
    fn escape_output_is_always_legal_under_the_target_alphabet() {
        for &alphabet in ALL_ALPHABETS {
            for label in HOSTILE_LABELS {
                let escaped = escape_label(label, alphabet);
                assert!(
                    is_valid_label(&escaped, alphabet),
                    "escape of {label:?} under {alphabet:?} is illegal: {escaped:?}"
                );
            }
        }
    }

    /// Property sweep: EVERY single-scalar label over a broad code-point range,
    /// plus that scalar in an inner position, escapes to a legal label under
    /// every alphabet.
    #[test]
    fn escape_output_is_legal_for_every_scalar_position() {
        for cp in (0u32..=0x2FFF).chain([
            0xD7FF,
            0xE000,
            0xFFFD,
            0xFFFE,
            0xFFFF,
            0x0001_0000,
            0x0010_FFFF,
        ]) {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            for label in [c.to_string(), format!("a{c}"), format!("{c}z")] {
                for &alphabet in ALL_ALPHABETS {
                    let escaped = escape_label(&label, alphabet);
                    assert!(
                        is_valid_label(&escaped, alphabet),
                        "escape of {label:?} ({cp:#06x}) under {alphabet:?} is illegal: {escaped:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn escape_is_injective_over_the_hostile_table() {
        for &alphabet in ALL_ALPHABETS {
            let mut seen: BTreeMap<String, &str> = BTreeMap::new();
            for label in HOSTILE_LABELS {
                let escaped = escape_label(label, alphabet).into_owned();
                if let Some(previous) = seen.insert(escaped.clone(), label) {
                    panic!("{alphabet:?} maps {previous:?} and {label:?} both to {escaped:?}");
                }
            }
        }
    }

    /// The marker-collision case stated explicitly: a LEGAL label that happens
    /// to equal the escape of an illegal one must itself be escaped, so the
    /// two never conflate.
    #[test]
    fn a_legal_label_equal_to_an_escape_image_is_escaped_away_from_it() {
        let illegal = "a b";
        let image = escape_label(illegal, LabelAlphabet::BlankNodeLabel).into_owned();
        assert_eq!(image, "purrdfesc_a_000020b");
        assert!(
            is_valid_blank_node_label(&image),
            "the escape image is itself a legal label"
        );
        let twin = escape_label(&image, LabelAlphabet::BlankNodeLabel).into_owned();
        assert_ne!(
            twin, image,
            "a legal label matching the escape image must be escaped away from it"
        );
        assert!(twin.starts_with(ESCAPE_MARKER));
    }

    #[test]
    fn escape_is_deterministic_across_runs() {
        for &alphabet in ALL_ALPHABETS {
            for label in HOSTILE_LABELS {
                assert_eq!(
                    escape_label(label, alphabet),
                    escape_label(label, alphabet),
                    "{label:?} under {alphabet:?}"
                );
            }
        }
    }

    #[test]
    fn escape_encoding_is_the_documented_marker_plus_fixed_width_hex() {
        assert_eq!(
            escape_label("bad\u{1f}label", LabelAlphabet::BlankNodeLabel),
            "purrdfesc_bad_00001Flabel"
        );
        assert_eq!(
            escape_label("0abc", LabelAlphabet::NcName),
            "purrdfesc_0abc"
        );
        assert_eq!(escape_label("", LabelAlphabet::NcName), ESCAPE_MARKER);
        assert_eq!(
            escape_label("\u{10FFFF}", LabelAlphabet::BlankNodeLabel),
            "purrdfesc__10FFFF"
        );
    }

    // ── unescape_label / decode_blank_label ─────────────────────────────────

    /// The load-bearing inverse property: unescaping an escaped label returns the
    /// original bytes, for every hostile label and every alphabet.
    #[test]
    fn unescape_inverts_escape_over_the_hostile_table() {
        for &alphabet in ALL_ALPHABETS {
            for label in HOSTILE_LABELS {
                let escaped = escape_label(label, alphabet);
                assert_eq!(
                    unescape_label(&escaped).as_ref(),
                    *label,
                    "{label:?} under {alphabet:?} escaped to {escaped:?}"
                );
            }
        }
    }

    /// The same inverse over an exhaustive scalar sweep, in every character
    /// position the escape distinguishes.
    #[test]
    fn unescape_inverts_escape_for_every_scalar_position() {
        for cp in (0u32..=0x2FFF).chain([
            0xD7FF,
            0xE000,
            0xFFFD,
            0xFFFE,
            0xFFFF,
            0x0001_0000,
            0x0010_FFFF,
        ]) {
            let Some(c) = char::from_u32(cp) else {
                continue;
            };
            for label in [c.to_string(), format!("a{c}"), format!("{c}z")] {
                for &alphabet in ALL_ALPHABETS {
                    let escaped = escape_label(&label, alphabet);
                    assert_eq!(
                        unescape_label(&escaped).as_ref(),
                        label.as_str(),
                        "{label:?} ({cp:#06x}) under {alphabet:?} escaped to {escaped:?}"
                    );
                }
            }
        }
    }

    /// A label that is not a well-formed escape passes through byte-identically
    /// and without allocating — including a marker-prefixed label whose body the
    /// escape could never have written.
    #[test]
    fn unescape_passes_through_labels_outside_the_escape_image() {
        for label in [
            "",
            "a",
            "a.b",
            "purrdfesc",
            "purrdfesc__12",     // a hex group shorter than six digits
            "purrdfesc__00002e", // lowercase hex is not what the escape writes
            "purrdfesc_a.b",     // '.' is not a pass-through character
            "purrdfesc__00D800", // a surrogate code point is not a scalar
            "purrdfesc__110000", // beyond the last Unicode scalar
            "purrdfesc_日本",    // a non-ASCII pass-through never survives escape
        ] {
            let decoded = unescape_label(label);
            assert_eq!(decoded.as_ref(), label, "{label:?}");
            assert!(
                matches!(decoded, Cow::Borrowed(_)),
                "{label:?} must pass through without allocating"
            );
        }
    }

    /// The composite ingress decode inverts the composite egress transform for
    /// every `(label, scope)` pair and every alphabet — the property the
    /// byte-stability of a parse/serialize cycle rests on.
    #[test]
    fn decode_blank_label_inverts_qualify_then_escape() {
        let scopes = [
            BlankScope::DEFAULT,
            BlankScope(1),
            BlankScope(2),
            BlankScope(u32::MAX),
        ];
        for &alphabet in ALL_ALPHABETS {
            for label in HOSTILE_LABELS {
                for &scope in &scopes {
                    let qualified = scope.qualify_label(label);
                    let emitted = escape_label(&qualified, alphabet);
                    let (decoded, decoded_scope) = decode_blank_label(&emitted);
                    assert_eq!(
                        (decoded.as_ref(), decoded_scope),
                        (*label, scope),
                        "{label:?} @ {scope:?} under {alphabet:?} emitted as {emitted:?}"
                    );
                }
            }
        }
    }

    /// A REWRITTEN label's body is pure `[A-Za-z0-9_]`, which is why one
    /// encoding satisfies every alphabet at once (a pass-through keeps the
    /// caller's bytes and is exempt by construction).
    #[test]
    fn rewritten_labels_are_ascii_word_characters_only() {
        for &alphabet in ALL_ALPHABETS {
            for label in HOSTILE_LABELS {
                let Cow::Owned(escaped) = escape_label(label, alphabet) else {
                    continue;
                };
                assert!(
                    escaped
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_'),
                    "{escaped:?}"
                );
            }
        }
    }
}
