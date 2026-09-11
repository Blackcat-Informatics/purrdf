// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact character classes for the Turtle/SPARQL terminal grammar — the ONE
//! transcription every scanner in the workspace scans with.
//!
//! # Why a scanner may not approximate
//!
//! A character class inside a scanner is not a membership test, it is a
//! **boundary** test: under maximal munch the scanner consumes the longest run
//! of characters its class admits, so widening the class does not merely widen
//! the accepted language — it *moves the token boundary* in documents that both
//! the liberal and the exact scanner accept. The failure is therefore silent.
//! It does not surface as a parse error; it surfaces as a different parse.
//!
//! The counterexample the workspace actually hit: a scanner whose `PN_CHARS`
//! approximation was "every scalar above `0x7F`" absorbs U+00A0 NO-BREAK SPACE
//! into a name. In
//!
//! ```text
//! SELECT ?s WHERE { ?s<NBSP><urn:ex:p> ?o . ?s <urn:ex:q> ?z }
//! ```
//!
//! the NO-BREAK SPACE after the first `?s` is not `WS`, so it does not end the
//! variable; the liberal class swallows it and the query binds FOUR variables
//! (`?s`, `?s<NBSP>`, `?o`, `?z`) where the grammar names three. The join on
//! `?s` silently becomes a cross product. Every token is well-formed, every
//! test passes, and the answer is wrong.
//!
//! Liberality on ingress is sound only where a decision is a decision about
//! **membership** — a validator may accept a superset and still never reject a
//! conforming document, because it never has to decide where one token stops
//! and the next begins. A scanner has to decide exactly that, so its classes
//! must be exact in BOTH directions: a class that is too wide misparses, a
//! class that is too narrow over-refuses.
//!
//! # What is here
//!
//! Each production below is transcribed once, as a numeric range table with the
//! grammar's own hexadecimal spellings kept verbatim in the source, and is
//! emitted by the `terminal!` macro as three things at once: the `const fn`
//! predicate, a compile-time assertion that the table is sorted and disjoint
//! (which is what the binary search inside it relies on), and — where the
//! production enumerates a small set — a compile-time assertion that the table
//! has exactly the cardinality the production names.
//!
//! Every predicate is ordered **ASCII-first**: the overwhelmingly common scalar
//! in real documents is below `0x80` and costs one comparison against a
//! two-or-three entry table, and only a scalar at or above `0x80` pays for the
//! range search.
//!
//! This module lives in the zero-dependency [`purrdf-iri`](crate) leaf because
//! that is the one crate every parser in the workspace already depends on —
//! `purrdf-sparql-algebra`, `purrdf-shex` and the RDF text codecs — so sharing
//! one transcription costs no new dependency edge and cannot introduce a cycle.
//!
//! # Sources
//!
//! The productions are quoted from the W3C *SPARQL 1.2 Query Language* grammar
//! (§19.8) and the W3C *RDF 1.2 Turtle* grammar (§6.5); the two specifications
//! spell `PN_CHARS_BASE`, `PN_CHARS_U` and `PN_CHARS` character-for-character
//! alike, and `VARNAME` is SPARQL-only.

/// An inclusive Unicode scalar-value range `[lo, hi]`, the unit every
/// production below is transcribed into.
type ScalarRange = (u32, u32);

/// One past the last ASCII scalar. The split point every predicate branches on:
/// below it the answer comes from the production's (tiny) ASCII table, at or
/// above it from the non-ASCII table.
const ASCII_LIMIT: u32 = 0x80;

/// Widen a byte to a scalar value for the range machinery.
///
/// `u32::from` is the right spelling and is not callable in a `const fn` on
/// stable Rust, so the cast is isolated here rather than repeated at each
/// byte-shaped predicate.
#[allow(
    clippy::cast_lossless,
    reason = "u32::from is not const-callable on stable; this is the one widening site"
)]
#[inline]
const fn widen(b: u8) -> u32 {
    b as u32
}

/// Whether `cp` falls in one of `ranges`, by binary search.
///
/// Correct only for a sorted, disjoint table — which is why every table the
/// `terminal!` macro emits carries a compile-time proof of exactly
/// that, rather than a comment asserting it.
#[inline]
const fn in_ranges(cp: u32, ranges: &[ScalarRange]) -> bool {
    let mut lo = 0;
    let mut hi = ranges.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let range = ranges[mid];
        if cp < range.0 {
            hi = mid;
        } else if cp > range.1 {
            lo = mid + 1;
        } else {
            return true;
        }
    }
    false
}

/// Whether every range is non-empty, every range ends at or below the maximum
/// Unicode scalar value, and the table is strictly ascending with a gap between
/// neighbours — the precondition [`in_ranges`] searches under.
#[inline]
const fn ranges_sorted_disjoint(ranges: &[ScalarRange]) -> bool {
    let mut i = 0;
    while i < ranges.len() {
        let range = ranges[i];
        if range.0 > range.1 || range.1 > 0x0010_FFFF {
            return false;
        }
        if i > 0 && ranges[i - 1].1 >= range.0 {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether every range lies wholly below [`ASCII_LIMIT`] — the invariant that
/// makes an ASCII-first predicate's fast path exhaustive for ASCII input.
#[inline]
const fn ranges_all_ascii(ranges: &[ScalarRange]) -> bool {
    let mut i = 0;
    while i < ranges.len() {
        if ranges[i].1 >= ASCII_LIMIT {
            return false;
        }
        i += 1;
    }
    true
}

/// Whether every range lies wholly at or above [`ASCII_LIMIT`] — the mirror
/// invariant, which keeps the slow path from having to re-answer ASCII.
#[inline]
const fn ranges_all_non_ascii(ranges: &[ScalarRange]) -> bool {
    let mut i = 0;
    while i < ranges.len() {
        if ranges[i].0 < ASCII_LIMIT {
            return false;
        }
        i += 1;
    }
    true
}

/// How many scalars a table admits, for productions that enumerate a set small
/// enough to name its own cardinality.
#[inline]
const fn range_cardinality(ranges: &[ScalarRange]) -> u32 {
    let mut count = 0;
    let mut i = 0;
    while i < ranges.len() {
        count += ranges[i].1 - ranges[i].0 + 1;
        i += 1;
    }
    count
}

/// Define one grammar terminal: its range table, its compile-time proofs, and
/// its `const fn` predicate.
///
/// The macro exists so that a production is written down exactly once — as
/// numeric ranges, in the grammar's own order — and so that the properties the
/// predicate's implementation silently depends on become compile errors to
/// violate rather than comments asking to be believed. The numeric literals
/// stay in plain source at the call site, so the table a reader (or a gate
/// scanning source text) sees is the table the predicate searches.
///
/// Two shapes:
///
/// * **Byte-shaped** — `const fn name(u8)`, one `ranges:` table. For a
///   production whose every member is ASCII: a byte test is then exact over
///   UTF-8, because a multi-byte sequence's lead and continuation bytes are all
///   `>= 0x80` and can never alias an ASCII member.
/// * **Scalar-shaped** — `const fn name(char)`, an `ascii:` table and a
///   `non_ascii:` table, optionally `extends` another scalar-shaped predicate
///   whose members it includes wholesale. The split is what makes the emitted
///   predicate ASCII-first; the two tables are proved to sit on their own side
///   of [`ASCII_LIMIT`].
///
/// Every invocation additionally proves its tables sorted and disjoint, and a
/// `cardinality:` clause proves a small enumerated production admits exactly as
/// many scalars as it names.
macro_rules! terminal {
    (
        $(#[$meta:meta])*
        table $table:ident;
        $vis:vis const fn $name:ident(u8);
        ranges: [ $( ($lo:literal, $hi:literal) ),* $(,)? ];
        $( cardinality: $cardinality:literal; )?
    ) => {
        /// Inclusive ranges of the production, in grammar order.
        const $table: &[ScalarRange] = &[ $( ($lo, $hi) ),* ];

        const _: () = {
            assert!(
                ranges_sorted_disjoint($table),
                concat!(stringify!($table), " must be sorted, non-empty and disjoint"),
            );
            assert!(
                ranges_all_ascii($table),
                concat!(stringify!($table), " must be wholly ASCII to be byte-testable"),
            );
            $(
                assert!(
                    range_cardinality($table) == $cardinality,
                    concat!(stringify!($table), " must admit exactly the scalars the production names"),
                );
            )?
        };

        $(#[$meta])*
        #[inline]
        #[must_use]
        $vis const fn $name(b: u8) -> bool {
            in_ranges(widen(b), $table)
        }
    };

    (
        $(#[$meta:meta])*
        tables $ascii_table:ident, $non_ascii_table:ident;
        $vis:vis const fn $name:ident(char) $( extends $base:ident )?;
        ascii: [ $( ($alo:literal, $ahi:literal) ),* $(,)? ];
        non_ascii: [ $( ($nlo:literal, $nhi:literal) ),* $(,)? ];
        $( cardinality: $cardinality:literal; )?
    ) => {
        /// ASCII members the production adds, checked first.
        const $ascii_table: &[ScalarRange] = &[ $( ($alo, $ahi) ),* ];
        /// Non-ASCII members the production adds, searched only above ASCII.
        const $non_ascii_table: &[ScalarRange] = &[ $( ($nlo, $nhi) ),* ];

        const _: () = {
            assert!(
                ranges_sorted_disjoint($ascii_table),
                concat!(stringify!($ascii_table), " must be sorted, non-empty and disjoint"),
            );
            assert!(
                ranges_all_ascii($ascii_table),
                concat!(stringify!($ascii_table), " must hold only ASCII scalars"),
            );
            assert!(
                ranges_sorted_disjoint($non_ascii_table),
                concat!(stringify!($non_ascii_table), " must be sorted, non-empty and disjoint"),
            );
            assert!(
                ranges_all_non_ascii($non_ascii_table),
                concat!(stringify!($non_ascii_table), " must hold no ASCII scalars"),
            );
            $(
                assert!(
                    range_cardinality($ascii_table) + range_cardinality($non_ascii_table)
                        == $cardinality,
                    concat!(stringify!($name), " must admit exactly the scalars the production names"),
                );
            )?
        };

        $(#[$meta])*
        #[inline]
        #[must_use]
        $vis const fn $name(c: char) -> bool {
            let cp = c as u32;
            if cp < ASCII_LIMIT {
                return in_ranges(cp, $ascii_table) $( || $base(c) )?;
            }
            in_ranges(cp, $non_ascii_table) $( || $base(c) )?
        }
    };
}

terminal! {
    /// `WS ::= #x20 | #x9 | #xD | #xA` — the whitespace a scanner may skip
    /// between two terminals (SPARQL 1.2 §19.8 `WS`; Turtle 1.2 §6.5 names the
    /// same four code points).
    ///
    /// Byte-shaped on purpose. Every member is ASCII, and in UTF-8 no byte of a
    /// multi-byte sequence is below `0x80`, so testing the raw byte is EXACT —
    /// it can neither miss a member nor alias one — and it is cheaper than
    /// decoding a `char` the scanner would otherwise have to decode just to
    /// discover the byte was not whitespace.
    ///
    /// This is emphatically NOT [`char::is_whitespace`], which answers the
    /// Unicode `White_Space` property: U+00A0 NO-BREAK SPACE, U+2000-U+200A,
    /// U+2028, U+2029, U+3000 and friends all satisfy that property and NONE of
    /// them is `WS`. Skipping them would accept documents the grammar rejects;
    /// worse, treating them as *not* name characters while also not skipping
    /// them is exactly the misparse described in the [module docs](self).
    ///
    /// It is also NOT [`u8::is_ascii_whitespace`], which is a two-sided trap
    /// rather than a merely narrower one: that predicate implements the WhatWG
    /// Infra definition, so it **admits U+000C FORM FEED**, which `WS` does not
    /// name, and it **excludes U+000B VERTICAL TAB**, which the Unicode and
    /// POSIX whitespace definitions do name. Its boundary is set by a third
    /// specification that is not this grammar, so it is wrong in both
    /// directions and coincides with `WS` only by accident on the four members
    /// they share.
    table WS_RANGES;
    pub const fn is_ws(u8);
    ranges: [
        (0x09, 0x0A), // #x9 CHARACTER TABULATION, #xA LINE FEED
        (0x0D, 0x0D), // #xD CARRIAGE RETURN
        (0x20, 0x20), // #x20 SPACE
    ];
    cardinality: 4;
}

terminal! {
    /// `PN_CHARS_BASE ::= [A-Z] | [a-z] | [#xC0-#xD6] | [#xD8-#xF6] |`
    /// `[#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF] | [#x200C-#x200D] |`
    /// `[#x2070-#x218F] | [#x2C00-#x2FEF] | [#x3001-#xD7FF] | [#xF900-#xFDCF] |`
    /// `[#xFDF0-#xFFFD] | [#x10000-#xEFFFF]`
    ///
    /// The name-start class shared verbatim by SPARQL 1.2 §19.8 and Turtle 1.2
    /// §6.5 (and, minus `':'` and `'_'`, by the XML 1.0 `NameStartChar`
    /// production).
    ///
    /// Note the deliberate holes: U+00D7 MULTIPLICATION SIGN and U+00F7
    /// DIVISION SIGN sit in the gaps at `#xC0-#xD6`/`#xD8-#xF6`, U+037E GREEK
    /// QUESTION MARK sits in the gap at `#x370-#x37D`/`#x37F-#x1FFF`, and the
    /// whole surrogate/private-use region above `#xD7FF` is excluded. An
    /// approximation of the form "anything above `0x7F`" admits all of them.
    tables PN_CHARS_BASE_ASCII, PN_CHARS_BASE_NON_ASCII;
    pub const fn is_pn_chars_base(char);
    ascii: [
        (0x41, 0x5A), // [A-Z]
        (0x61, 0x7A), // [a-z]
    ];
    non_ascii: [
        (0x00C0, 0x00D6),     // [#xC0-#xD6]
        (0x00D8, 0x00F6),     // [#xD8-#xF6]
        (0x00F8, 0x02FF),     // [#xF8-#x2FF]
        (0x0370, 0x037D),     // [#x370-#x37D]
        (0x037F, 0x1FFF),     // [#x37F-#x1FFF]
        (0x200C, 0x200D),     // [#x200C-#x200D]
        (0x2070, 0x218F),     // [#x2070-#x218F]
        (0x2C00, 0x2FEF),     // [#x2C00-#x2FEF]
        (0x3001, 0xD7FF),     // [#x3001-#xD7FF]
        (0xF900, 0xFDCF),     // [#xF900-#xFDCF]
        (0xFDF0, 0xFFFD),     // [#xFDF0-#xFFFD]
        (0x10000, 0xEFFFF),   // [#x10000-#xEFFFF]
    ];
}

terminal! {
    /// `PN_CHARS_U ::= PN_CHARS_BASE | '_'`
    ///
    /// SPARQL 1.2 §19.8 / Turtle 1.2 §6.5. One scalar wider than
    /// [`is_pn_chars_base`], and that scalar is ASCII, so the whole difference
    /// is paid on the fast path.
    tables PN_CHARS_U_ASCII, PN_CHARS_U_NON_ASCII;
    pub const fn is_pn_chars_u(char) extends is_pn_chars_base;
    ascii: [
        (0x5F, 0x5F), // '_'
    ];
    non_ascii: [];
}

terminal! {
    /// `PN_CHARS ::= PN_CHARS_U | '-' | [0-9] | #xB7 | [#x300-#x36F] |`
    /// `[#x203F-#x2040]`
    ///
    /// SPARQL 1.2 §19.8 / Turtle 1.2 §6.5: the name-CONTINUE class. Beyond
    /// [`is_pn_chars_u`] it adds `'-'` and `[0-9]` in ASCII, and exactly three
    /// non-ASCII ranges — U+00B7 MIDDLE DOT, the combining diacritical marks
    /// `[#x300-#x36F]`, and `[#x203F-#x2040]` (UNDERTIE, CHARACTER TIE). No
    /// other non-ASCII scalar is added; in particular U+00A0 NO-BREAK SPACE and
    /// the rest of the Unicode whitespace are NOT name characters, which is the
    /// whole point of the [module docs](self).
    tables PN_CHARS_ASCII, PN_CHARS_NON_ASCII;
    pub const fn is_pn_chars(char) extends is_pn_chars_u;
    ascii: [
        (0x2D, 0x2D), // '-'
        (0x30, 0x39), // [0-9]
    ];
    non_ascii: [
        (0x00B7, 0x00B7), // #xB7
        (0x0300, 0x036F), // [#x300-#x36F]
        (0x203F, 0x2040), // [#x203F-#x2040]
    ];
}

terminal! {
    /// The FIRST scalar of `VARNAME`: `( PN_CHARS_U | [0-9] )`.
    ///
    /// `VARNAME ::= ( PN_CHARS_U | [0-9] ) ( PN_CHARS_U | [0-9] | #xB7 |`
    /// `[#x300-#x36F] | [#x203F-#x2040] )*` (SPARQL 1.2 §19.8; SPARQL-only —
    /// Turtle has no variables).
    ///
    /// The production is position-dependent, so it takes TWO predicates: a
    /// variable may not begin with a combining mark, a MIDDLE DOT or an
    /// UNDERTIE even though it may continue with one. See
    /// [`is_varname_continue`].
    ///
    /// `VARNAME` also **excludes `'-'`** in both positions, though
    /// [`is_pn_chars`] includes it: `?a-b` is the variable `?a` followed by the
    /// operator `-` and the variable `b`'s spelling, not one variable named
    /// `a-b`. Scanning a variable with `is_pn_chars` is precisely the
    /// boundary-moving bug this module exists to prevent.
    tables VARNAME_START_ASCII, VARNAME_START_NON_ASCII;
    pub const fn is_varname_start(char) extends is_pn_chars_u;
    ascii: [
        (0x30, 0x39), // [0-9]
    ];
    non_ascii: [];
}

terminal! {
    /// Every SUBSEQUENT scalar of `VARNAME`:
    /// `( PN_CHARS_U | [0-9] | #xB7 | [#x300-#x36F] | [#x203F-#x2040] )`.
    ///
    /// SPARQL 1.2 §19.8. This is [`is_varname_start`] plus the three non-ASCII
    /// ranges `PN_CHARS` also folds in — and, like the start class, it
    /// **excludes `'-'`**, which is the one and only difference between it and
    /// [`is_pn_chars`].
    tables VARNAME_CONTINUE_ASCII, VARNAME_CONTINUE_NON_ASCII;
    pub const fn is_varname_continue(char) extends is_varname_start;
    ascii: [];
    non_ascii: [
        (0x00B7, 0x00B7), // #xB7
        (0x0300, 0x036F), // [#x300-#x36F]
        (0x203F, 0x2040), // [#x203F-#x2040]
    ];
}

#[cfg(test)]
mod tests {
    use super::{
        is_pn_chars, is_pn_chars_base, is_pn_chars_u, is_varname_continue, is_varname_start, is_ws,
    };
    use pretty_assertions::assert_eq;

    /// Every Unicode scalar value, in order.
    fn all_scalars() -> impl Iterator<Item = char> {
        (0..=0x0010_FFFF_u32).filter_map(char::from_u32)
    }

    /// An independent transcription of `PN_CHARS_BASE` as a `matches!` pattern
    /// rather than a range table, to check the table against.
    fn pn_chars_base_oracle(c: char) -> bool {
        matches!(c,
            'A'..='Z'
            | 'a'..='z'
            | '\u{C0}'..='\u{D6}'
            | '\u{D8}'..='\u{F6}'
            | '\u{F8}'..='\u{2FF}'
            | '\u{370}'..='\u{37D}'
            | '\u{37F}'..='\u{1FFF}'
            | '\u{200C}'..='\u{200D}'
            | '\u{2070}'..='\u{218F}'
            | '\u{2C00}'..='\u{2FEF}'
            | '\u{3001}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}'
            | '\u{FDF0}'..='\u{FFFD}'
            | '\u{10000}'..='\u{EFFFF}')
    }

    /// `PN_CHARS_U ::= PN_CHARS_BASE | '_'`, transcribed independently.
    fn pn_chars_u_oracle(c: char) -> bool {
        pn_chars_base_oracle(c) || c == '_'
    }

    /// `PN_CHARS`, transcribed independently.
    fn pn_chars_oracle(c: char) -> bool {
        pn_chars_u_oracle(c)
            || c.is_ascii_digit()
            || matches!(c, '-' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
    }

    #[test]
    fn ws_is_exactly_four_code_points() {
        for b in 0..=u8::MAX {
            assert_eq!(
                is_ws(b),
                matches!(b, 0x20 | 0x09 | 0x0D | 0x0A),
                "byte {b:#04X}"
            );
        }
    }

    #[test]
    fn ws_is_not_ascii_whitespace() {
        // The admitted-but-not-named side: FORM FEED is ASCII whitespace under
        // the WhatWG Infra definition and is not `WS`.
        assert!(0x0C_u8.is_ascii_whitespace());
        assert!(!is_ws(0x0C));
        // The named-but-not-admitted side: VERTICAL TAB is whitespace under
        // Unicode/POSIX and is likewise not `WS`, so neither definition is a
        // model of the production.
        assert!(!0x0B_u8.is_ascii_whitespace());
        assert!(char::from(0x0B_u8).is_whitespace());
        assert!(!is_ws(0x0B));
    }

    #[test]
    fn ws_never_matches_a_utf8_continuation_or_lead_byte() {
        for b in 0x80..=u8::MAX {
            assert!(!is_ws(b), "byte {b:#04X}");
        }
        // Concretely: NO-BREAK SPACE encodes as C2 A0, neither byte of which is
        // `WS`, so a scanner cannot skip it.
        let mut buf = [0_u8; 4];
        for &b in '\u{A0}'.encode_utf8(&mut buf).as_bytes() {
            assert!(!is_ws(b));
        }
    }

    #[test]
    fn no_break_space_is_neither_whitespace_nor_a_name_character() {
        // The misparse in the module docs, pinned from both sides: U+00A0 is
        // not skippable AND not nameable, so it can only be a syntax error.
        assert!('\u{A0}'.is_whitespace());
        assert!(!is_pn_chars_base('\u{A0}'));
        assert!(!is_pn_chars_u('\u{A0}'));
        assert!(!is_pn_chars('\u{A0}'));
        assert!(!is_varname_start('\u{A0}'));
        assert!(!is_varname_continue('\u{A0}'));
        // The neighbouring valid case: the ordinary SPACE that the query
        // *should* have used still separates the tokens.
        assert!(is_ws(b' '));
        // And a real non-ASCII name character is still a name character, so
        // this is exactness, not blanket ASCII-only refusal.
        assert!(is_pn_chars_base('\u{65E5}'));
        assert!(is_varname_start('\u{65E5}'));
    }

    #[test]
    fn tables_agree_with_an_independent_transcription() {
        for c in all_scalars() {
            assert_eq!(is_pn_chars_base(c), pn_chars_base_oracle(c), "{c:?}");
            assert_eq!(is_pn_chars_u(c), pn_chars_u_oracle(c), "{c:?}");
            assert_eq!(is_pn_chars(c), pn_chars_oracle(c), "{c:?}");
        }
    }

    #[test]
    fn the_classes_nest_the_way_the_grammar_nests_them() {
        for c in all_scalars() {
            if is_pn_chars_base(c) {
                assert!(is_pn_chars_u(c), "{c:?}");
            }
            if is_pn_chars_u(c) {
                assert!(is_pn_chars(c), "{c:?}");
                assert!(is_varname_start(c), "{c:?}");
            }
            if is_varname_start(c) {
                assert!(is_varname_continue(c), "{c:?}");
            }
            // `VARNAME`'s continue class and `PN_CHARS` differ in exactly one
            // scalar, the hyphen.
            assert_eq!(is_pn_chars(c), is_varname_continue(c) || c == '-', "{c:?}");
        }
    }

    #[test]
    fn varname_excludes_the_hyphen_pn_chars_admits() {
        assert!(is_pn_chars('-'));
        assert!(!is_varname_start('-'));
        assert!(!is_varname_continue('-'));
    }

    #[test]
    fn varname_is_position_dependent() {
        // Digits start a variable; combining marks and ties only continue one.
        assert!(is_varname_start('0'));
        assert!(is_varname_continue('0'));
        for c in ['\u{B7}', '\u{300}', '\u{36F}', '\u{203F}', '\u{2040}'] {
            assert!(!is_varname_start(c), "{c:?}");
            assert!(is_varname_continue(c), "{c:?}");
        }
        // `_` starts one, via `PN_CHARS_U`.
        assert!(is_varname_start('_'));
    }

    #[test]
    fn the_holes_in_pn_chars_base_are_real() {
        for c in [
            '\u{D7}', '\u{F7}', '\u{37E}', '\u{2000}', '\u{200B}', '\u{3000}',
        ] {
            assert!(!is_pn_chars_base(c), "{c:?}");
        }
        // Their neighbours are admitted, so the holes are holes and not a
        // truncated table.
        for c in ['\u{D6}', '\u{D8}', '\u{F6}', '\u{F8}', '\u{37D}', '\u{37F}'] {
            assert!(is_pn_chars_base(c), "{c:?}");
        }
    }

    #[test]
    fn every_predicate_is_usable_in_const_context() {
        // The predicates are `const fn` so a scanner can fold them into lookup
        // tables at compile time; these assertions are evaluated by the
        // compiler, not at run time, which is what pins the property.
        const { assert!(is_ws(b' ')) }
        const { assert!(!is_ws(0xA0)) }
        const { assert!(is_pn_chars_base('a')) }
        const { assert!(is_pn_chars_u('_')) }
        const { assert!(!is_varname_continue('-')) }
        const { assert!(is_pn_chars('-')) }
    }
}
