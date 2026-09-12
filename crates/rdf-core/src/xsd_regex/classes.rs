// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The character-class bodies that [`super::emit`] splices into emitted
//! `regex`-crate source: the `\i`/`\I`/`\c`/`\C` XML-name classes, the
//! `\s`/`\S`/`\w`/`\W` classes, and the surrogate-range test that decides how
//! `\p{Is…}` block escapes naming the three non-scalar surrogate blocks are
//! emitted.

use std::fmt::Write as _;
use std::sync::OnceLock;

use purrdf_iri::terminals::{xml_name_char_ranges, xml_name_start_char_ranges};

/// Push a single inclusive codepoint range as a `regex`-crate bracket-class
/// member: `\u{lo}` for a one-codepoint range, `\u{lo}-\u{hi}` otherwise.
pub(super) fn push_hex_range(out: &mut String, lo: u32, hi: u32) {
    if lo == hi {
        write!(out, "\\u{{{lo:x}}}").expect("writing to a String cannot fail");
    } else {
        write!(out, "\\u{{{lo:x}}}-\\u{{{hi:x}}}").expect("writing to a String cannot fail");
    }
}

/// Push every range in `ranges` as bracket-class members (see
/// [`push_hex_range`]).
fn push_ranges(out: &mut String, ranges: &[(u32, u32)]) {
    for &(lo, hi) in ranges {
        push_hex_range(out, lo, hi);
    }
}

/// The body (without enclosing `[`/`]`) of the `\i`/`\I` bracket expression.
///
/// XSD's `\i` is XML 1.0 `NameStartChar` in full — `':'`, `'_'` and the twelve
/// non-ASCII ranges included. The set is taken from
/// [`purrdf_iri::terminals::xml_name_start_char_ranges`], the ONE transcription
/// of the production in the workspace, rather than re-derived here from
/// Turtle's narrower `PN_CHARS_BASE` plus a hand-patched `":_"` suffix. The
/// suffix abbreviated away the fact that it was reconstructing a different
/// specification's terminal, and that reconstruction could drift from the
/// shared table silently; the accessor is that table, so it cannot.
pub(super) fn name_start_class_body() -> String {
    let mut body = String::new();
    push_ranges(&mut body, xml_name_start_char_ranges());
    body
}

/// The body (without enclosing `[`/`]`) of the `\c`/`\C` bracket expression.
///
/// XSD's `\c` is XML 1.0 `NameChar` in full; the composed set — `NameStartChar`
/// plus `'-'`, `'.'`, `[0-9]`, U+00B7 MIDDLE DOT, the combining marks
/// `[#x300-#x36F]` and the two ties `[#x203F-#x2040]` — comes from
/// [`purrdf_iri::terminals::xml_name_char_ranges`], the shared production.
/// Nothing is added here: the `.` that Turtle expresses by the *shape* of its
/// `PN_LOCAL` production is an ordinary member of XML's `NameChar` class, and
/// the accessor already carries it.
pub(super) fn name_char_class_body() -> String {
    let mut body = String::new();
    push_ranges(&mut body, xml_name_char_ranges());
    body
}

/// The simple-case-fold closure of [`name_start_class_body`], as a
/// `regex`-crate bracket-class body (no enclosing brackets).
///
/// See [`fold_class_body`] for how the closure is computed and why it is
/// computed once rather than left to `regex-syntax`'s per-occurrence folding.
pub(super) fn folded_name_start_class_body() -> &'static str {
    static CELL: OnceLock<String> = OnceLock::new();
    CELL.get_or_init(|| fold_class_body(&name_start_class_body()))
}

/// The simple-case-fold closure of [`name_char_class_body`]; see
/// [`folded_name_start_class_body`].
pub(super) fn folded_name_char_class_body() -> &'static str {
    static CELL: OnceLock<String> = OnceLock::new();
    CELL.get_or_init(|| fold_class_body(&name_char_class_body()))
}

/// Compute the simple-case-fold closure of a `regex`-crate bracket-class
/// **body** and return it in the same form.
///
/// The `\i`/`\I`/`\c`/`\C` bodies are constants, so their case-fold closure is
/// a constant too. Computing it once here lets [`super::emit`] emit the
/// PRE-FOLDED set under `(?-i:…)` for a STANDALONE escape, which removes
/// `regex-syntax`'s per-occurrence, per-codepoint case-folding walk over the
/// ~917k-codepoint astral `NameChar` range — the quadratic blow-up this module
/// exists to close. `regex-syntax` folds at Hir-translation time, before
/// `size_limit` applies, so the fold cannot be bounded by the engine; making
/// the standalone fold a constant removes its cost entirely.
///
/// This constant does NOT reach an escape that appears INSIDE a character
/// class: a group is not a class member, so the `(?-i:…)` scope cannot wrap
/// it. There the fold is instead bounded by count in
/// `emit::translate` — see [`super::MAX_FOLDED_CLASS_ESCAPES`].
///
/// The closure is obtained from `regex` itself (the same engine, and therefore
/// the same Unicode simple-case-fold table, that the old emission relied on):
/// compile `(?i)[<body>]` once and walk every Unicode scalar value, collecting
/// exactly the scalars it accepts. That is deterministic — the sweep is over
/// `0..=0x10FFFF` in order and the ranges are emitted in ascending order — and
/// needs no new dependency. It is exact by construction rather than by a
/// hand-rolled case-folding table that could drift from the engine's.
fn fold_class_body(body: &str) -> String {
    let folded = regex::Regex::new(&format!("(?i)[{body}]"))
        .expect("the canonical XML-name class body always compiles");
    // Every Unicode scalar value, in one haystack, so one DFA pass yields the
    // whole closure. Surrogate code points are not scalar values and are
    // therefore absent (a Rust `&str` cannot contain them anyway).
    let mut haystack = String::with_capacity(0x11_0000);
    for cp in 0..=0x0010_FFFF_u32 {
        if let Some(c) = char::from_u32(cp) {
            haystack.push(c);
        }
    }

    let mut out = String::with_capacity(body.len() + 16);
    let mut run: Option<(u32, u32)> = None;
    for matched in folded.find_iter(&haystack) {
        let cp = matched
            .as_str()
            .chars()
            .next()
            .expect("a bracket class matches exactly one character") as u32;
        match run {
            Some((lo, hi)) if cp == hi + 1 => run = Some((lo, cp)),
            Some((lo, hi)) => {
                push_hex_range(&mut out, lo, hi);
                run = Some((cp, cp));
            }
            None => run = Some((cp, cp)),
        }
    }
    if let Some((lo, hi)) = run {
        push_hex_range(&mut out, lo, hi);
    }
    out
}

/// XSD `\s ::= [#x20\t\n\r]`, exactly 4 codepoints — Rust's own `\s`
/// is the full Unicode `White_Space` property (26 codepoints
/// including U+00A0/U+3000), which is too wide (§3 of the governing
/// plan).
pub(super) const SPACE_CLASS: &str = "[\\u{9}\\u{a}\\u{d}\\u{20}]";

/// The negated form of [`SPACE_CLASS`], emitted for XSD's `\S`.
pub(super) const NOT_SPACE_CLASS: &str = "[^\\u{9}\\u{a}\\u{d}\\u{20}]";

/// XSD `\w ::= [^\p{P}\p{Z}\p{C}]` — Rust's own `\w` is Perl's
/// alphanumeric-plus-underscore-plus-combining-marks class, a
/// different (and differently-shaped) set.
pub(super) const WORD_CLASS: &str = "[^\\p{P}\\p{Z}\\p{C}]";

/// The negated form of [`WORD_CLASS`], emitted for XSD's `\W`.
pub(super) const NOT_WORD_CLASS: &str = "[\\p{P}\\p{Z}\\p{C}]";

/// Whether `lo..=hi` lies wholly inside the UTF-16 surrogate range
/// `U+D800..=U+DFFF`, whose code points are not Unicode scalar values.
///
/// Tested as a range rather than by block name so it stays correct if a future
/// Unicode revision renames or re-partitions the three surrogate blocks: the
/// property that matters is where the code points are, not what they are
/// called.
pub(super) fn is_surrogate_range(lo: u32, hi: u32) -> bool {
    lo >= 0xD800 && hi <= 0xDFFF
}

#[cfg(test)]
mod tests {
    use super::super::emit::translate;
    use purrdf_iri::terminals::{is_xml_name_char, is_xml_name_start_char};

    fn translated_regex(pattern: &str, dot_all: bool) -> regex::Regex {
        let source = translate(pattern, dot_all, false).expect("translate");
        regex::Regex::new(&source).unwrap_or_else(|e| panic!("compile {source:?}: {e}"))
    }

    #[test]
    fn i_and_ii_escapes_standalone() {
        let re = translated_regex(r"^\i+$", false);
        assert!(re.is_match("abc"));
        assert!(re.is_match(":a_b"));
        assert!(!re.is_match("1abc"));
        let re_neg = translated_regex(r"^\I$", false);
        assert!(re_neg.is_match("1"));
        assert!(!re_neg.is_match("a"));
    }

    #[test]
    fn i_escape_splices_inside_an_open_class() {
        let re = translated_regex(r"^[\i0-9]+$", false);
        assert!(re.is_match("a1:_9"));
        assert!(!re.is_match("a!b"));
    }

    #[test]
    fn c_and_cc_escapes() {
        let re = translated_regex(r"^\c+$", false);
        assert!(re.is_match("a-1._:"));
        let re_neg = translated_regex(r"^\C$", false);
        assert!(re_neg.is_match("!"));
        assert!(!re_neg.is_match("a"));
    }

    #[test]
    fn s_escape_is_the_four_codepoint_class_not_unicode_whitespace() {
        let re = translated_regex(r"^\s$", false);
        assert!(re.is_match(" "));
        assert!(re.is_match("\t"));
        assert!(re.is_match("\n"));
        assert!(re.is_match("\r"));
        assert!(!re.is_match("\u{A0}")); // NBSP: Unicode whitespace, not XSD's
        assert!(!re.is_match("\u{3000}")); // IDEOGRAPHIC SPACE, ditto
        let re_neg = translated_regex(r"^\S$", false);
        assert!(re_neg.is_match("a"));
        assert!(!re_neg.is_match(" "));
    }

    #[test]
    fn w_escape_excludes_punctuation_separator_other() {
        let re = translated_regex(r"^\w$", false);
        assert!(re.is_match("a"));
        assert!(re.is_match("1"));
        // `_` is Unicode category Pc (Connector Punctuation) -- XSD's
        // `[^\p{P}\p{Z}\p{C}]` formula excludes it, unlike Perl's `\w`.
        assert!(!re.is_match("_"));
        assert!(!re.is_match(" "));
        assert!(!re.is_match("."));
        let re_neg = translated_regex(r"^\W$", false);
        assert!(re_neg.is_match("_"));
        assert!(re_neg.is_match("."));
        assert!(!re_neg.is_match("a"));
    }

    /// The three surrogate blocks hold no Unicode scalar values, so no `&str`
    /// subject can contain one. Emitting the range literally made the engine
    /// refuse `\u{d800}` with a message naming neither the block nor the
    /// reason; the exact answer is the empty set (and its complement).
    #[test]
    fn surrogate_blocks_are_the_empty_set_not_a_parse_error() {
        assert_eq!(
            translate(r"\p{IsHighSurrogates}", false, false).expect("translate"),
            "[^\\s\\S]"
        );
        assert_eq!(
            translate(r"\P{IsLowSurrogates}", false, false).expect("translate"),
            "[\\s\\S]"
        );
        let empty = translated_regex(r"^\p{IsHighSurrogates}$", false);
        assert!(!empty.is_match("a"));
        assert!(!empty.is_match("\u{10000}"));
        let all = translated_regex(r"^\P{IsHighSurrogates}$", false);
        assert!(all.is_match("a"));
        assert!(all.is_match("\n"), "the complement is every scalar value");

        // Composes by union inside an enclosing class like any other block.
        let union = translated_regex(r"^[\p{IsHighSurrogates}\p{IsBasicLatin}]+$", false);
        assert!(union.is_match("abc"));
        assert!(!union.is_match("\u{3b1}"));

        // A neighbouring non-surrogate block is unaffected by the carve-out.
        let latin = translated_regex(r"^\p{IsBasicLatin}$", false);
        assert!(latin.is_match("A"));
        assert!(!latin.is_match("\u{e9}"));
    }

    /// `\i` must match EXACTLY XML 1.0 `NameStartChar` and `\c` exactly XML 1.0
    /// `NameChar`, as those productions are spelled once in
    /// [`purrdf_iri::terminals`] and reached here through the composed range
    /// accessors — `is_xml_name_start_char` and `is_xml_name_char` are the
    /// predicates over the same tables, so this pins the emitted class to the
    /// shared terminal rather than to a second transcription that merely
    /// agrees today.
    ///
    /// A full sweep over all ~1.1M scalar values is cheap for a compiled DFA,
    /// and it is the only shape that proves the class is exact in BOTH
    /// directions: a hand-patched suffix could drop a range's boundary scalar
    /// without any sampled vector noticing. Boundary codepoints of every range
    /// are exercised by construction since the sweep is exhaustive, not
    /// sampled.
    #[test]
    fn i_and_c_escapes_match_the_shared_xml_name_terminals_exactly() {
        let i_re = translated_regex(r"^\i$", false);
        let c_re = translated_regex(r"^\c$", false);
        for cp in 0_u32..=0x0010_FFFF {
            if (0xD800..=0xDFFF).contains(&cp) {
                continue; // surrogates: not representable as a `char`
            }
            let c = char::from_u32(cp).expect("valid scalar value");
            let s = c.to_string();
            assert_eq!(
                i_re.is_match(&s),
                is_xml_name_start_char(c),
                "\\i mismatch at U+{cp:04X} {c:?}"
            );
            assert_eq!(
                c_re.is_match(&s),
                is_xml_name_char(c),
                "\\c mismatch at U+{cp:04X} {c:?}"
            );
        }
    }

    /// Build the ORACLE: what `compile(pattern, "i")` matched before this
    /// module stopped leaving name-escape folding to `regex-syntax`. The old
    /// path is exactly `translate(pattern, _, case_insensitive = false)`
    /// followed by the builder's global `(?i)`, so prepending `(?i)` to the
    /// raw-body translation reproduces it byte-for-byte.
    fn old_i_regex(pattern: &str, dot_all: bool) -> regex::Regex {
        let source = translate(pattern, dot_all, false).expect("translate (old path)");
        regex::Regex::new(&format!("(?i){source}")).expect("compile old-path regex")
    }

    /// Assert two single-character-class regexes accept exactly the same
    /// scalars, over the whole Unicode scalar space (surrogates excluded).
    fn assert_same_scalar_language(new: &regex::Regex, old: &regex::Regex, label: &str) {
        for cp in 0_u32..=0x0010_FFFF {
            if (0xD800..=0xDFFF).contains(&cp) {
                continue; // surrogates: not representable as a `char`
            }
            let c = char::from_u32(cp).expect("valid scalar value");
            let s = c.to_string();
            assert_eq!(
                new.is_match(&s),
                old.is_match(&s),
                "{label} mismatch at U+{cp:04X} {c:?}"
            );
        }
    }

    /// The pre-folded `(?-i:…)` rewrite must not change the matched language
    /// for `\i`, `\I`, `\c`, `\C` — standalone, spliced inside a class, and as
    /// a class-subtraction operand. Each new `compile(.., "i")` is compared
    /// against the old translation over the whole scalar space, so a
    /// case-fold closure that dropped or added a single scalar fails here.
    #[test]
    fn name_escapes_under_i_match_the_old_language_exactly() {
        for pattern in [
            r"^\i$",
            r"^\I$",
            r"^\c$",
            r"^\C$",
            r"^[\i0-9]+$",
            r"^[a-z-[\c]]+$",
            r"^[^\i x]+$",
        ] {
            let new = super::super::compile(pattern, "i").expect("compile under i");
            let old = old_i_regex(pattern, false);
            assert_same_scalar_language(new.as_regex(), &old, pattern);
        }
    }
}
