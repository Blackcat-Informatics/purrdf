// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact label alphabets for the syntax this workspace *emits*: blank-node
//! labels, XML `NCName`s, and the unconstrained catch-all used where a
//! consumer imposes no grammar of its own.
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
//! and the XML 1.0 `NameStartChar`/`NameChar` productions, not approximated.
//! A label that fails [`is_valid_label`] cannot be emitted by any codec
//! without producing a document no conforming parser -- including PurRDF's
//! own -- can read back.
//!
//! # The three writers
//!
//! This workspace has exactly three egress paths for blank-node identity,
//! and each has a different relationship to this module:
//!
//! - `native_codecs` Turtle is **label-preserving**: it writes the blank
//!   node's existing label back out verbatim, so an input label that is not
//!   valid Turtle syntax would otherwise round-trip into an unparsable
//!   document. That path is where [`is_valid_blank_node_label`] is the
//!   validation gate.
//! - `turtle_render` mints its own structural `_:bN` labels from a counter;
//!   it never echoes a caller-supplied label, so it is immune to this
//!   problem by construction and has no call into this module.
//! - RDFC-1.0 canonicalization mints `c14n0`, `c14n1`, … labels (see
//!   `crates::ir::canon`). Those labels are ASCII letters and digits only,
//!   which is a subset of every alphabet this module defines, so canonical
//!   labels are legal everywhere without needing a check.
//!
//! # Relabeling vs. canonicalization: consistent doctrines, different roles
//!
//! Serialization is free to relabel a blank node -- swapping `_:x7` for
//! `_:x7_esc0` (or a `turtle_render` structural `_:bN`) is an
//! isomorphism-preserving operation: it changes no triple's *meaning*: the
//! graph up to blank-node renaming is identical before and after. Escaping
//! or substituting an out-of-alphabet label is therefore always a legitimate
//! move on a *serialization* label.
//!
//! RDFC-1.0 canonicalization refuses that same move for a different reason:
//! its output bytes are not a rendering choice, they mint the dataset's
//! content-addressed identity (two isomorphic graphs must canonicalize to
//! byte-identical output, and two non-isomorphic graphs must not collide).
//! Relabeling a canonical label to dodge an alphabet constraint would be
//! indistinguishable, downstream, from silently changing which graph the
//! identity was computed over. Both doctrines refuse to let an invalid label
//! leak into a document; they differ only in *how* they refuse -- a
//! serialization writer may pick a fresh legal label, while a canonicalizer
//! must never need to, because its alphabet (plain ASCII `cNNN` counters) was
//! chosen to be legal everywhere in the first place.

use core::cmp::Ordering;

/// Which label grammar [`is_valid_label`] should check against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelAlphabet {
    /// The W3C Turtle/SPARQL `BLANK_NODE_LABEL` production (the part after
    /// `_:`); see [`is_valid_blank_node_label`].
    BlankNodeLabel,
    /// The XML 1.0 `NCName` production; see [`is_valid_ncname`].
    NcName,
    /// No grammar beyond "non-empty": any string with at least one
    /// `char` is accepted.
    Unconstrained,
}

/// Whether `label` is legal under `alphabet`.
///
/// Dispatches to [`is_valid_blank_node_label`], [`is_valid_ncname`], or a
/// non-empty check, per [`LabelAlphabet`].
#[must_use]
pub fn is_valid_label(label: &str, alphabet: LabelAlphabet) -> bool {
    match alphabet {
        LabelAlphabet::BlankNodeLabel => is_valid_blank_node_label(label),
        LabelAlphabet::NcName => is_valid_ncname(label),
        LabelAlphabet::Unconstrained => !label.is_empty(),
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

/// Inclusive Unicode scalar-value range `[lo, hi]`.
type CharRange = (u32, u32);

/// `PN_CHARS_BASE` from the W3C Turtle/SPARQL grammar, which is also
/// character-for-character the XML 1.0 `NameStartChar` production minus
/// `':'`. Ranges are sorted and non-overlapping, which [`in_ranges`] relies
/// on for binary search.
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

/// `PN_CHARS_BASE` (== XML `NameStartChar - ':'`).
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
        LabelAlphabet, is_pn_chars, is_pn_chars_u, is_valid_blank_node_label, is_valid_label,
        is_valid_ncname,
    };

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
    }

    #[test]
    fn unconstrained_accepts_any_non_empty_string() {
        for label in ["a b", "<urn:x>"] {
            assert!(
                is_valid_label(label, LabelAlphabet::Unconstrained),
                "{label:?}"
            );
        }
        assert!(!is_valid_label("", LabelAlphabet::Unconstrained));
    }
}
