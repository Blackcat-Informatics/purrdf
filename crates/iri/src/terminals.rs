// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exact character classes for the Turtle/SPARQL/XML terminal grammars — the
//! ONE transcription every scanner in the workspace scans with.
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
//! A production whose callers hold different cursor shapes gets two predicates
//! over **one** table rather than two tables: [`is_ws`] and [`is_ws_char`],
//! [`is_iriref_forbidden_byte`] and [`is_iriref_forbidden`]. The suffix names
//! the shape that had to be added, not a second transcription — the lift is
//! sound only because the shared table is proved wholly ASCII, so the widened
//! search answers `false` for every scalar a `u8::try_from` narrowing would have
//! rejected. Both spellings in each pair are names the workspace's terminal gate
//! recognises as this module's, so a local fork under either name is caught.
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
//!
//! `NameStartChar` and `NameChar` come from a DIFFERENT specification — *XML
//! 1.0 Fifth Edition* §2.3, productions `[4]` and `[4a]` — and are the classes
//! `xsd:Name`, `xsd:NCName` and `xsd:ID` are defined over, so they reach RDF
//! through datatype validation rather than through a scanner. They are kept
//! here, beside the Turtle/SPARQL classes they *resemble*, precisely because
//! the resemblance is the trap: `PN_CHARS_BASE` is `NameStartChar` minus `':'`
//! and `'_'`, and `NameChar` is `PN_CHARS` plus `':'` and `'.'`. Neither pair
//! may be aliased to the other — see [`is_xml_name_start_char`].

/// An inclusive Unicode scalar-value range `[lo, hi]`, the unit every
/// production below is transcribed into.
///
/// Public because a scanner that needs the production's *set* rather than a
/// membership test reaches it through the accessor the `terminal!` macro emits
/// beside each predicate (e.g. [`xml_name_start_char_ranges`]).
pub type ScalarRange = (u32, u32);

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

/// How many ranges a composed terminal table holds before it is sorted: the
/// base terminal's full set, the terminal's own ASCII additions, and its own
/// non-ASCII additions, counted together.
///
/// This is the array length the `terminal!` macro needs for the composed table,
/// and it is exact because [`merge`] sorts but never drops or duplicates an
/// entry: the composed table is a permutation of the concatenation.
#[inline]
const fn concat_len(
    base: &[ScalarRange],
    ascii: &[ScalarRange],
    non_ascii: &[ScalarRange],
) -> usize {
    base.len() + ascii.len() + non_ascii.len()
}

/// Compose a terminal's full range set — its inherited base, its ASCII
/// additions and its non-ASCII additions — into one sorted, disjoint table.
///
/// This is what a caller that needs a production's *set* rather than a
/// membership test consumes. The three inputs are each already sorted and
/// disjoint (the macro proved that for the two it owns, and the base accessor
/// returns the same property for its own composed table), but they interleave:
/// `NameStartChar`'s `':'` sits below `PN_CHARS_BASE`'s `[A-Z]`, and `NameChar`'s
/// U+00B7 sits below `PN_CHARS_BASE`'s `[#xC0-#xD6]`. The concatenation is
/// therefore sorted here rather than assumed ordered, and the result is proved
/// sorted and disjoint at compile time — the exact precondition
/// [`in_ranges`]'s binary search relies on. Because the three inputs already
/// partition their members (no scalar is admitted twice), the sorted
/// concatenation is its own deduplication.
///
/// The sort is an insertion sort: the largest table in the module is ~30
/// entries, so it is both shorter to read and cheaper to evaluate in `const`
/// than anything asymptotically better would be.
#[inline]
const fn merge<const N: usize>(
    base: &[ScalarRange],
    ascii: &[ScalarRange],
    non_ascii: &[ScalarRange],
) -> [ScalarRange; N] {
    let mut out = [(0_u32, 0_u32); N];
    let mut n = 0;
    let mut i = 0;
    while i < base.len() {
        out[n] = base[i];
        n += 1;
        i += 1;
    }
    i = 0;
    while i < ascii.len() {
        out[n] = ascii[i];
        n += 1;
        i += 1;
    }
    i = 0;
    while i < non_ascii.len() {
        out[n] = non_ascii[i];
        n += 1;
        i += 1;
    }
    let mut i = 1;
    while i < N {
        let key = out[i];
        let mut j = i;
        while j > 0 && (out[j - 1].0 > key.0 || (out[j - 1].0 == key.0 && out[j - 1].1 > key.1)) {
            out[j] = out[j - 1];
            j -= 1;
        }
        out[j] = key;
        i += 1;
    }
    assert!(
        ranges_sorted_disjoint(&out),
        "a composed terminal table must be sorted, non-empty and disjoint",
    );
    out
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
/// A byte-shaped invocation may additionally carry a `char_form:` clause, which
/// emits a `const fn name(char)` **over the same table**. That is the whole
/// reason the clause exists rather than a second invocation: a scanner that
/// holds decoded scalars and a scanner that holds raw bytes are asking one
/// production, and spelling it twice is the fork this module was built to
/// abolish. The lift is exact because the byte arm has already proved the table
/// wholly ASCII, so no scalar at or above [`ASCII_LIMIT`] can be a member and
/// the widened search answers `false` for every one of them — the same answer
/// the caller's own `u8::try_from` narrowing would have produced, without the
/// narrowing having to be re-argued at each call site.
///
/// Every invocation additionally proves its tables sorted and disjoint, and a
/// `cardinality:` clause proves a small enumerated production admits exactly as
/// many scalars as it names.
///
/// Every invocation also emits a `ranges_fn:` accessor — a `const fn` returning
/// the production's full set as a sorted, disjoint `&'static [ScalarRange]`.
/// This is for the caller that needs a *set* rather than a membership test: an
/// emitter that must splice a character class into a regex source, a lookup
/// table, a diagnostic. A scalar-shaped invocation names the accessor of the
/// table it `extends` in the `base_ranges:` clause, so the accessor is the
/// composed set — inherited members included — not just the two local tables.
/// The two spellings are kept apart deliberately: the predicate `extends` a
/// base *predicate* (whose members it includes wholesale and tests through the
/// `||`), and the accessor `base_ranges` names the base *accessor* whose set it
/// composes. A byte-shaped invocation's single table is already its whole set.
macro_rules! terminal {
    (
        $(#[$meta:meta])*
        table $table:ident;
        $vis:vis const fn $name:ident(u8);
        ranges_fn: $ranges_name:ident;
        ranges: [ $( ($lo:literal, $hi:literal) ),* $(,)? ];
        $( cardinality: $cardinality:literal; )?
        $(
            char_form:
            $(#[$char_meta:meta])*
            $char_vis:vis const fn $char_name:ident(char);
        )?
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

        /// The production's full range set, sorted and disjoint.
        ///
        /// A byte-shaped production's table is already its whole set: it is
        /// proved wholly ASCII, so no scalar at or above [`ASCII_LIMIT`] is a
        /// member and the same table is the set over the scalar space.
        #[inline]
        #[must_use]
        $vis const fn $ranges_name() -> &'static [ScalarRange] {
            $table
        }

        $(
            $(#[$char_meta])*
            #[inline]
            #[must_use]
            $char_vis const fn $char_name(c: char) -> bool {
                in_ranges(c as u32, $table)
            }
        )?
    };

    (
        $(#[$meta:meta])*
        tables $ascii_table:ident, $non_ascii_table:ident;
        $vis:vis const fn $name:ident(char) $( extends $base:ident )?;
        base_ranges: $base_expr:expr;
        ranges_fn: $ranges_name:ident;
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

        /// The production's full range set: the inherited base, this
        /// production's ASCII additions, and its non-ASCII additions,
        /// composed into one sorted, disjoint table.
        ///
        /// [`merge`] proves the composed table sorted and disjoint at compile
        /// time, exactly as the macro proves each written table, so the binary
        /// search [`in_ranges`] performs is valid over it too.
        #[inline]
        #[must_use]
        $vis const fn $ranges_name() -> &'static [ScalarRange] {
            const RANGES: [ScalarRange; concat_len(
                $base_expr,
                $ascii_table,
                $non_ascii_table,
            )] = merge($base_expr, $ascii_table, $non_ascii_table);
            &RANGES
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
    ranges_fn: ws_ranges;
    ranges: [
        (0x09, 0x0A), // #x9 CHARACTER TABULATION, #xA LINE FEED
        (0x0D, 0x0D), // #xD CARRIAGE RETURN
        (0x20, 0x20), // #x20 SPACE
    ];
    cardinality: 4;
    char_form:
    /// `WS` for a scanner that holds a decoded [`char`] rather than a raw byte.
    ///
    /// The same four-member table as [`is_ws`], searched over the full scalar
    /// value. Every member is ASCII, so every scalar at or above U+0080 answers
    /// `false` — which is the correct answer and not an approximation: U+00A0
    /// NO-BREAK SPACE, U+2000-U+200A, U+2028, U+2029 and U+3000 carry the
    /// Unicode `White_Space` property and none of them is `WS`.
    ///
    /// This exists so that the narrowing argument is made once. A scalar-holding
    /// scanner would otherwise spell the test `u8::try_from(c).is_ok_and(is_ws)`
    /// and owe a comment explaining why the fallible narrowing cannot lose a
    /// member; ShExC's `@pass` scanner and its shape-map scanner each carried
    /// that argument separately. Two prose copies of one proof drift the same
    /// way two range tables do.
    pub const fn is_ws_char(char);
}

terminal! {
    /// `PN_CHARS_BASE ::= [A-Z] | [a-z] | [#xC0-#xD6] | [#xD8-#xF6] |`
    /// `[#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF] | [#x200C-#x200D] |`
    /// `[#x2070-#x218F] | [#x2C00-#x2FEF] | [#x3001-#xD7FF] | [#xF900-#xFDCF] |`
    /// `[#xFDF0-#xFFFD] | [#x10000-#xEFFFF]`
    ///
    /// The name-start class shared verbatim by SPARQL 1.2 §19.8 and Turtle 1.2
    /// §6.5. It is XML 1.0's [`NameStartChar`](is_xml_name_start_char) minus
    /// `':'` and `'_'` — every other range of the two is identical, which is
    /// what makes the two scalars that differ so easy to lose.
    ///
    /// Note the deliberate holes: U+00D7 MULTIPLICATION SIGN and U+00F7
    /// DIVISION SIGN sit in the gaps at `#xC0-#xD6`/`#xD8-#xF6`, U+037E GREEK
    /// QUESTION MARK sits in the gap at `#x370-#x37D`/`#x37F-#x1FFF`, and the
    /// whole surrogate/private-use region above `#xD7FF` is excluded. An
    /// approximation of the form "anything above `0x7F`" admits all of them.
    tables PN_CHARS_BASE_ASCII, PN_CHARS_BASE_NON_ASCII;
    pub const fn is_pn_chars_base(char);
    base_ranges: &[];
    ranges_fn: pn_chars_base_ranges;
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
    base_ranges: pn_chars_base_ranges();
    ranges_fn: pn_chars_u_ranges;
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
    base_ranges: pn_chars_u_ranges();
    ranges_fn: pn_chars_ranges;
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
    /// The FIRST scalar of `BLANK_NODE_LABEL`: `( PN_CHARS_U | [0-9] )`.
    ///
    /// `BLANK_NODE_LABEL ::= '_:' ( PN_CHARS_U | [0-9] )`
    /// `((PN_CHARS | '.')* PN_CHARS)?` (SPARQL 1.2 §19.8 / Turtle 1.2 §6.5).
    ///
    /// The production is position-dependent and its head is **narrower than its
    /// tail**, which is the whole reason this predicate exists separately from
    /// [`is_pn_chars`]: `'-'`, `'.'`, U+00B7 MIDDLE DOT, the combining marks
    /// `[#x300-#x36F]` and the two ties `[#x203F-#x2040]` may CONTINUE a label
    /// and may not BEGIN one, so `_:a-b` is one label and `_:-b` is not a label
    /// at all. Scanning the head with the continue class silently admits a
    /// label this workspace's own writer refuses to emit — see
    /// `purrdf_rdf_core::blank_label::is_valid_blank_node_label`, the egress
    /// side of the same production.
    ///
    /// It is also narrower than [`is_pn_local_start`]: a `':'` begins a
    /// `PN_LOCAL` and never a blank node label, because the `':'` in `_:` has
    /// already been consumed by the terminal's own literal prefix.
    ///
    /// This is the same SET of scalars as [`is_varname_start`], and deliberately
    /// not the same predicate: two productions in two grammars that happen to
    /// coincide today are not one production, and `VARNAME` is SPARQL-only while
    /// `BLANK_NODE_LABEL` is shared. The coincidence is pinned by a test rather
    /// than assumed by a call.
    tables BLANK_NODE_LABEL_START_ASCII, BLANK_NODE_LABEL_START_NON_ASCII;
    pub const fn is_blank_node_label_start(char) extends is_pn_chars_u;
    base_ranges: pn_chars_u_ranges();
    ranges_fn: blank_node_label_start_ranges;
    ascii: [
        (0x30, 0x39), // [0-9]
    ];
    non_ascii: [];
}

terminal! {
    /// The single-scalar alternatives of `PN_LOCAL`'s FIRST position:
    /// `( PN_CHARS_U | ':' | [0-9] )`.
    ///
    /// `PN_LOCAL ::= (PN_CHARS_U | ':' | [0-9] | PLX)`
    /// `((PN_CHARS | '.' | ':' | PLX)* (PN_CHARS | ':' | PLX))?`
    /// (SPARQL 1.2 §19.8 / Turtle 1.2 §6.5).
    ///
    /// **The head has a fourth alternative this predicate cannot answer.**
    /// `PLX ::= PERCENT | PN_LOCAL_ESC` is not a character class: `PERCENT ::=`
    /// `'%' HEX HEX` and `PN_LOCAL_ESC ::= '\' [_~.-!$&'()*+,;=/?#@%]` are both
    /// multi-scalar shapes, so whether a `'%'` or a `'\'` opens a local name is
    /// a decision about the scalars that FOLLOW it, which only the scanner
    /// holding the cursor can make. This predicate answers the three
    /// alternatives that ARE a character class, and every caller must decide
    /// `PLX` itself — `ex:%20a` and `ex:\~a` are lawful prefixed names whose
    /// local part begins at a scalar this predicate returns `false` for.
    ///
    /// What it refuses is the head that no alternative names: `'-'`, `'.'`,
    /// U+00B7, `[#x300-#x36F]` and `[#x203F-#x2040]` are all `PN_CHARS`, so they
    /// may continue a local name, and none of them may start one. `ex:a-b` is
    /// one prefixed name; `ex:-b` is the empty local name `ex:` followed by the
    /// `'-'` token.
    ///
    /// Wider than [`is_blank_node_label_start`] by exactly the `':'` — a local
    /// name may begin, continue and end with one (`ex::a`, `ex:a:b` are single
    /// prefixed names) — and wider than [`is_pn_chars_base`] by the `'_'` a
    /// `PN_PREFIX` may not begin with. Three productions in one grammar, three
    /// different head classes, no two of them equal.
    tables PN_LOCAL_START_ASCII, PN_LOCAL_START_NON_ASCII;
    pub const fn is_pn_local_start(char) extends is_pn_chars_u;
    base_ranges: pn_chars_u_ranges();
    ranges_fn: pn_local_start_ranges;
    ascii: [
        (0x30, 0x39), // [0-9]
        (0x3A, 0x3A), // ':'
    ];
    non_ascii: [];
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
    base_ranges: pn_chars_u_ranges();
    ranges_fn: varname_start_ranges;
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
    base_ranges: varname_start_ranges();
    ranges_fn: varname_continue_ranges;
    ascii: [];
    non_ascii: [
        (0x00B7, 0x00B7), // #xB7
        (0x0300, 0x036F), // [#x300-#x36F]
        (0x203F, 0x2040), // [#x203F-#x2040]
    ];
}

terminal! {
    /// The scalar a `PN_LOCAL_ESC` escapes — the part of the terminal that IS a
    /// character class.
    ///
    /// ```text
    /// PN_LOCAL_ESC ::= '\' ( '_' | '~' | '.' | '-' | '!' | '$' | '&' | "'"
    ///                      | '(' | ')' | '*' | '+' | ',' | ';' | '=' | '/'
    ///                      | '?' | '#' | '@' | '%' )
    /// ```
    ///
    /// SPARQL 1.2 §19.8 / Turtle 1.2 §6.5. The `'\'` itself is a literal the
    /// scanner has already consumed when it asks this, so what is answered here
    /// is the twenty-member set that may FOLLOW the backslash — and only that
    /// set. A `'\'` followed by anything else is not a `PN_LOCAL_ESC` at all, so
    /// it does not belong to the local name and must end the scan rather than be
    /// absorbed: `ex:a\qb` is the prefixed name `ex:a` and then a syntax error,
    /// never the local name `a\qb` or `aqb`.
    ///
    /// The escaped scalar stands for ITSELF in the local name's value, which is
    /// why the set is exactly the punctuation that would otherwise terminate the
    /// scan or be read as some other token — `dbr:Semantic_analysis_\(linguistics\)`
    /// is one prefixed name whose local part contains literal parentheses.
    ///
    /// Scalar-shaped, because every member is ASCII and both of the workspace's
    /// prefixed-name scanners look at the escaped scalar as a decoded [`char`];
    /// a non-ASCII scalar after the backslash is not escapable, which is why the
    /// non-ASCII table is empty rather than absent.
    ///
    /// Note the deliberate holes. The table's `[#x23-#x2F]` run is contiguous
    /// only by accident of the code chart, and the gaps around it are the point:
    /// `'"'` (`#x22`) is NOT escapable, nor are `'<'` (`#x3C`), `'>'` (`#x3E`),
    /// `'\'` (`#x5C`), `'['`, `']'`, `'^'`, `` '`' ``, `'{'`, `'|'`, `'}'`, the
    /// digits or the letters. A scanner that answered "any ASCII punctuation"
    /// would accept `ex:a\"b` and mint a local name containing a quote.
    tables PN_LOCAL_ESC_ASCII, PN_LOCAL_ESC_NON_ASCII;
    pub const fn is_pn_local_esc(char);
    base_ranges: &[];
    ranges_fn: pn_local_esc_ranges;
    ascii: [
        (0x21, 0x21), // '!'
        (0x23, 0x2F), // '#' '$' '%' '&' '\'' '(' ')' '*' '+' ',' '-' '.' '/'
        (0x3B, 0x3B), // ';'
        (0x3D, 0x3D), // '='
        (0x3F, 0x40), // '?' '@'
        (0x5F, 0x5F), // '_'
        (0x7E, 0x7E), // '~'
    ];
    non_ascii: [];
    cardinality: 20;
}

terminal! {
    /// Whether a byte may NOT appear raw in an `IRIREF` body.
    ///
    /// ```text
    /// IRIREF ::= '<' ( [^#x00-#x20<>"{}|^`\] | UCHAR )* '>'
    /// ```
    ///
    /// SPARQL 1.2 §19.8 / Turtle 1.2 §6.5 (`[18t]`), and ShExC spells it the
    /// same way. Exactly two things are excluded: the range `#x00-#x20` — every
    /// C0 control **plus the SPACE** — and the nine reserved delimiters
    /// ``< > " { } | ^ ` \``. **Nothing else.** Every excluded code point is
    /// ASCII, so a byte test is exact over UTF-8: a multi-byte sequence's lead
    /// and continuation bytes are all `>= 0x80` and can never alias one of them.
    ///
    /// This is stated as the FORBIDDEN set rather than the admitted one because
    /// that is what the production itself enumerates; the admitted class is its
    /// complement over all of Unicode, which no table can hold.
    ///
    /// # Why [`char::is_control`] is not this predicate
    ///
    /// A content class is a terminal like any other, and `is_control` stands in
    /// for this one while being wrong in **both** directions at once, which is
    /// why substituting it is invisible from either side alone:
    ///
    /// * it **admits** `#x20` SPACE, which the production excludes, so
    ///   `<urn:ex:a b>` names an IRI with a space in it;
    /// * it **refuses** U+007F-U+009F, which the production admits, so a lawful
    ///   IRI carrying a C1 control is reported as unterminated;
    /// * and it says nothing at all about `` ` ``, `|`, `^` and `\` — four of
    ///   the nine delimiters — so those are absorbed into the body too. Each of
    ///   those four is a delimiter somewhere in the Turtle family, so admitting
    ///   one lets a document name a node no conforming processor resolves the
    ///   same way.
    ///
    /// # Why non-ASCII whitespace is lawful here
    ///
    /// This is likewise NOT [`char::is_whitespace`]. U+00A0 NO-BREAK SPACE,
    /// U+2000-U+200A, U+2028, U+2029, U+3000 and the rest of the non-ASCII
    /// Unicode whitespace are **lawful** raw inside an `IRIREF`, and they are
    /// `ucschar` under RFC 3987 §2.2 so they survive IRI validation as well. The
    /// workspace's own N-Triples/N-Quads/Turtle/TriG writers emit those scalars
    /// VERBATIM — only `#x00-#x20`, the delimiters and the control blocks ride
    /// as `\uXXXX` — so terminating a body at them would make serialize-then-parse
    /// stop being a round trip. An `IRIREF` body is content, not a token
    /// boundary, and the class that decides it is narrower than any whitespace
    /// property, not wider.
    table IRIREF_FORBIDDEN_RANGES;
    pub const fn is_iriref_forbidden_byte(u8);
    ranges_fn: iriref_forbidden_ranges;
    ranges: [
        (0x00, 0x20), // [#x00-#x20]: the C0 controls and SPACE
        (0x22, 0x22), // '"'
        (0x3C, 0x3C), // '<'
        (0x3E, 0x3E), // '>'
        (0x5C, 0x5C), // '\'
        (0x5E, 0x5E), // '^'
        (0x60, 0x60), // '`'
        (0x7B, 0x7D), // '{' '|' '}'
    ];
    cardinality: 42;
    char_form:
    /// The [`char`] form of [`is_iriref_forbidden_byte`], for a scan that
    /// already holds a decoded scalar — an escape decoder, or a scanner over a
    /// `Vec<char>`.
    ///
    /// The same table, searched over the full scalar value. Every forbidden code
    /// point is ASCII, so every scalar at or above U+0080 is permitted: that
    /// includes U+00A0 and the whole Latin-1 supplement, and it is the
    /// production's answer, not a shortcut.
    pub const fn is_iriref_forbidden(char);
}

terminal! {
    /// `NameStartChar ::= ':' | [A-Z] | '_' | [a-z] | [#xC0-#xD6] |`
    /// `[#xD8-#xF6] | [#xF8-#x2FF] | [#x370-#x37D] | [#x37F-#x1FFF] |`
    /// `[#x200C-#x200D] | [#x2070-#x218F] | [#x2C00-#x2FEF] |`
    /// `[#x3001-#xD7FF] | [#xF900-#xFDCF] | [#xFDF0-#xFFFD] | [#x10000-#xEFFFF]`
    ///
    /// XML 1.0 Fifth Edition §2.3 `[4]`: the class that opens an XML `Name`,
    /// and through it `NCName` and every XSD datatype derived from it
    /// (`xsd:Name`, `xsd:NCName`, `xsd:ID`, `xsd:IDREF`, `xsd:ENTITY`).
    ///
    /// # Why this is NOT [`is_pn_chars_base`]
    ///
    /// The two tables share every one of their twelve non-ASCII ranges,
    /// character for character, and differ by exactly two ASCII scalars — which
    /// is the whole hazard. `NameStartChar` admits:
    ///
    /// * **`':'`**, which SPARQL and Turtle remove because a colon is *their*
    ///   prefix separator: `ex:a` is a prefixed name with local part `a`, while
    ///   XML would happily read `ex:a` as one `Name` (that is why `NCName` has
    ///   to exist as a separate production that subtracts the colon again);
    /// * **`'_'`**, which `PN_CHARS_BASE` also lacks — Turtle adds it one level
    ///   up, in [`PN_CHARS_U`](is_pn_chars_u), because `_:` must stay
    ///   distinguishable as the blank-node prefix.
    ///
    /// So aliasing this to `is_pn_chars_base` would reject `:name` and `_name`,
    /// both of which are lawful XML names, and aliasing it to
    /// [`is_pn_chars_u`] would reject `:name` alone — a refusal that surfaces
    /// only when a document actually carries such a name. The `ascii:` table
    /// below is literally the difference, and the `extends` clause keeps the
    /// twelve shared ranges written once, so the two classes cannot drift apart
    /// while still being impossible to confuse at this site.
    ///
    /// # Why [`char::is_alphabetic`] is not this predicate
    ///
    /// This is the mistake the approximations in this workspace kept making,
    /// and it is not a boundary case: U+00AA FEMININE ORDINAL INDICATOR is
    /// `Alphabetic`, sits below the table's first non-ASCII range `[#xC0-#xD6]`,
    /// and is **not** a `NameStartChar`. Nor are U+00B5 MICRO SIGN or U+00BA
    /// MASCULINE ORDINAL INDICATOR. The deliberate holes of
    /// [`is_pn_chars_base`] are holes here too: U+00D7, U+00F7 and U+037E are
    /// excluded, and so is everything above `#xD7FF` except the two named
    /// blocks and the supplementary planes up to `#xEFFFF`.
    tables XML_NAME_START_CHAR_ASCII, XML_NAME_START_CHAR_NON_ASCII;
    pub const fn is_xml_name_start_char(char) extends is_pn_chars_base;
    base_ranges: pn_chars_base_ranges();
    ranges_fn: xml_name_start_char_ranges;
    ascii: [
        (0x3A, 0x3A), // ':' — present here, ABSENT from PN_CHARS_BASE
        (0x5F, 0x5F), // '_' — present here, ABSENT from PN_CHARS_BASE
    ];
    non_ascii: [];
}

terminal! {
    /// `NameChar ::= NameStartChar | '-' | '.' | [0-9] | #xB7 |`
    /// `[#x300-#x36F] | [#x203F-#x2040]`
    ///
    /// XML 1.0 Fifth Edition §2.3 `[4a]`: every scalar after the first in an
    /// XML `Name`. Position-dependent like the Turtle name productions, so it
    /// takes a second predicate rather than widening
    /// [`is_xml_name_start_char`] — `-a` and `.a` and `0a` are not names, while
    /// `a-a`, `a.a` and `a0` are.
    ///
    /// # The `'.'` is the difference that matters
    ///
    /// Beyond `NameStartChar` this adds exactly what
    /// [`PN_CHARS`](is_pn_chars) adds beyond [`PN_CHARS_U`](is_pn_chars_u) —
    /// `'-'`, `[0-9]`, U+00B7 MIDDLE DOT, the combining diacritical marks
    /// `[#x300-#x36F]`, and the two ties `[#x203F-#x2040]` — **plus `'.'`**.
    ///
    /// That one scalar is why an `xsd:NCName` and a Turtle local name are not
    /// the same language. `'.'` is not a `PN_CHARS` member at all: Turtle
    /// admits it *between* name characters by the shape of the production
    /// (`(PN_CHARS | '.')* PN_CHARS`) and never at either end, so `a.b` is one
    /// local name while `a.` is the local name `a` followed by the statement
    /// terminator. XML puts `'.'` in the character class itself, so `a.` is a
    /// perfectly good XML `Name` with nothing following it. A validator that
    /// answered `xsd:Name` with `is_pn_chars` would refuse the lawful value
    /// `a.`; one that answered a Turtle local name with this class would let a
    /// trailing dot swallow the `'.'` that ends the triple.
    ///
    /// The `':'` inherited from `NameStartChar` is the second difference, and
    /// it is the one `NCName` exists to remove: `NCName` is this class minus
    /// `':'` in both positions, so nothing here may be reused for `NCName`
    /// without subtracting it.
    tables XML_NAME_CHAR_ASCII, XML_NAME_CHAR_NON_ASCII;
    pub const fn is_xml_name_char(char) extends is_xml_name_start_char;
    base_ranges: xml_name_start_char_ranges();
    ranges_fn: xml_name_char_ranges;
    ascii: [
        (0x2D, 0x2D), // '-'
        (0x2E, 0x2E), // '.' — the scalar PN_CHARS does NOT admit
        (0x30, 0x39), // [0-9]
    ];
    non_ascii: [
        (0x00B7, 0x00B7), // #xB7
        (0x0300, 0x036F), // [#x300-#x36F]
        (0x203F, 0x2040), // [#x203F-#x2040]
    ];
}

#[cfg(test)]
mod tests {
    use super::{
        ScalarRange, blank_node_label_start_ranges, in_ranges, iriref_forbidden_ranges,
        is_blank_node_label_start, is_iriref_forbidden, is_iriref_forbidden_byte, is_pn_chars,
        is_pn_chars_base, is_pn_chars_u, is_pn_local_esc, is_pn_local_start, is_varname_continue,
        is_varname_start, is_ws, is_ws_char, is_xml_name_char, is_xml_name_start_char,
        pn_chars_base_ranges, pn_chars_ranges, pn_chars_u_ranges, pn_local_esc_ranges,
        pn_local_start_ranges, ranges_sorted_disjoint, varname_continue_ranges,
        varname_start_ranges, ws_ranges, xml_name_char_ranges, xml_name_start_char_ranges,
    };
    use pretty_assertions::assert_eq;

    /// Every Unicode scalar value, in order.
    fn all_scalars() -> impl Iterator<Item = char> {
        (0..=0x0010_FFFF_u32).filter_map(char::from_u32)
    }

    /// One range accessor under test: its name, its composed table, and the
    /// predicate it must agree with.
    type RangeCase = (&'static str, &'static [ScalarRange], fn(char) -> bool);

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

    /// XML 1.0 5e §2.3 `[4]` `NameStartChar`, transcribed independently from
    /// the specification's own alternation rather than derived from any table
    /// in this module — the point being that it is written out in full,
    /// including the twelve ranges it shares with `PN_CHARS_BASE`, so agreement
    /// is evidence about the production and not about the `extends` clause.
    fn xml_name_start_char_oracle(c: char) -> bool {
        matches!(c,
            ':'
            | 'A'..='Z'
            | '_'
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

    /// XML 1.0 5e §2.3 `[4a]` `NameChar`, transcribed independently.
    fn xml_name_char_oracle(c: char) -> bool {
        xml_name_start_char_oracle(c)
            || matches!(c,
                '-' | '.'
                | '0'..='9'
                | '\u{B7}'
                | '\u{300}'..='\u{36F}'
                | '\u{203F}'..='\u{2040}')
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
            assert_eq!(
                is_xml_name_start_char(c),
                xml_name_start_char_oracle(c),
                "{c:?}"
            );
            assert_eq!(is_xml_name_char(c), xml_name_char_oracle(c), "{c:?}");
        }
    }

    #[test]
    fn every_range_accessor_agrees_with_its_predicate_over_all_scalars() {
        // The macro emits, beside each predicate, an accessor returning the
        // production's full SET as a sorted, disjoint table. Two independent
        // claims are proved here for every one of them, over the whole scalar
        // space rather than sampled:
        //
        // 1. the composed table is sorted and disjoint (the precondition the
        //    accessor's consumers and `in_ranges` rely on); and
        // 2. membership in that table is EXACTLY the predicate's answer —
        //    including the `extends` inheritance, which is why `NameStartChar`
        //    and `NameChar` are checked through their composed accessors and
        //    not through the two tables the macro writes down.
        //
        // A drift between a table and the predicate that reads it is silent
        // everywhere else (the emitter would splice a class the predicate no
        // longer means), so this is the one place it is caught.
        let cases: &[RangeCase] = &[
            ("ws", ws_ranges(), is_ws_char),
            (
                "iriref_forbidden",
                iriref_forbidden_ranges(),
                is_iriref_forbidden,
            ),
            ("pn_chars_base", pn_chars_base_ranges(), is_pn_chars_base),
            ("pn_chars_u", pn_chars_u_ranges(), is_pn_chars_u),
            ("pn_chars", pn_chars_ranges(), is_pn_chars),
            (
                "blank_node_label_start",
                blank_node_label_start_ranges(),
                is_blank_node_label_start,
            ),
            ("pn_local_start", pn_local_start_ranges(), is_pn_local_start),
            ("varname_start", varname_start_ranges(), is_varname_start),
            (
                "varname_continue",
                varname_continue_ranges(),
                is_varname_continue,
            ),
            ("pn_local_esc", pn_local_esc_ranges(), is_pn_local_esc),
            (
                "xml_name_start_char",
                xml_name_start_char_ranges(),
                is_xml_name_start_char,
            ),
            ("xml_name_char", xml_name_char_ranges(), is_xml_name_char),
        ];
        for (name, ranges, predicate) in cases {
            assert!(
                ranges_sorted_disjoint(ranges),
                "{name}: composed ranges must be sorted, non-empty and disjoint",
            );
            for c in all_scalars() {
                assert_eq!(
                    in_ranges(c as u32, ranges),
                    predicate(c),
                    "{name} membership at {c:?}",
                );
            }
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
    fn no_two_head_classes_in_this_grammar_are_the_same_class() {
        // `PN_PREFIX`, `PN_LOCAL` and `BLANK_NODE_LABEL` each open at a
        // different set, and each opens at a set narrower than the one it
        // continues with. Stated as a total function over the scalars so the
        // three cannot drift into one another.
        for c in all_scalars() {
            // `PN_PREFIX` head ⊂ `BLANK_NODE_LABEL` head ⊂ `PN_LOCAL` head.
            if is_pn_chars_base(c) {
                assert!(is_blank_node_label_start(c), "{c:?}");
            }
            if is_blank_node_label_start(c) {
                assert!(is_pn_local_start(c), "{c:?}");
            }
            // Every head is a subset of the tail class it continues into.
            if is_pn_local_start(c) {
                assert!(is_pn_chars(c) || c == ':', "{c:?}");
            }
            // The differences, exactly: `'_'` separates `PN_PREFIX`'s head from
            // a blank node label's, `[0-9]` separates it further, and `':'` is
            // the one scalar a local name's head has that a label's has not.
            assert_eq!(
                is_blank_node_label_start(c),
                is_pn_chars_base(c) || c == '_' || c.is_ascii_digit(),
                "{c:?}"
            );
            assert_eq!(
                is_pn_local_start(c),
                is_blank_node_label_start(c) || c == ':',
                "{c:?}"
            );
            // `VARNAME`'s head is the same SET as a blank node label's; the two
            // predicates exist apart because the two productions do.
            assert_eq!(is_blank_node_label_start(c), is_varname_start(c), "{c:?}");
        }
    }

    #[test]
    fn the_name_heads_refuse_what_only_their_tails_admit() {
        // `PN_CHARS` members that no head class in this grammar names. Each is
        // pinned in BOTH positions, so neither half can be read as blanket
        // strictness: the scalar is refused at the head and admitted after it.
        for c in ['-', '\u{B7}', '\u{300}', '\u{36F}', '\u{203F}', '\u{2040}'] {
            assert!(is_pn_chars(c), "{c:?} must still continue a name");
            assert!(!is_blank_node_label_start(c), "{c:?}");
            assert!(!is_pn_local_start(c), "{c:?}");
            assert!(!is_pn_chars_base(c), "{c:?}");
        }
        // `'.'` is not even a `PN_CHARS` member: it is admitted between name
        // characters by the productions' own structure, never at either end.
        assert!(!is_pn_chars('.'));
        assert!(!is_blank_node_label_start('.'));
        assert!(!is_pn_local_start('.'));
        // The lawful neighbours, so the refusals above are exactness and not a
        // narrowed alphabet: digits and `'_'` open both names, `':'` opens only
        // a local one, and a non-ASCII letter opens all three.
        for c in ['0', '9', '_'] {
            assert!(is_blank_node_label_start(c), "{c:?}");
            assert!(is_pn_local_start(c), "{c:?}");
        }
        assert!(is_pn_local_start(':'));
        assert!(!is_blank_node_label_start(':'));
        for c in ['\u{4E2D}', '\u{65E5}', '\u{E9}'] {
            assert!(is_pn_chars_base(c), "{c:?}");
            assert!(is_blank_node_label_start(c), "{c:?}");
            assert!(is_pn_local_start(c), "{c:?}");
        }
    }

    #[test]
    fn the_char_lift_of_a_byte_production_agrees_on_every_scalar() {
        // The `char_form:` clause is only sound because its table is wholly
        // ASCII. Checked as a total function rather than spot-checked: for every
        // scalar the lift must answer exactly what narrowing-then-testing
        // answers, which is the argument each call site used to have to make.
        for c in all_scalars() {
            assert_eq!(
                is_ws_char(c),
                u8::try_from(c).is_ok_and(is_ws),
                "WS at {c:?}"
            );
            assert_eq!(
                is_iriref_forbidden(c),
                u8::try_from(c).is_ok_and(is_iriref_forbidden_byte),
                "IRIREF content at {c:?}"
            );
        }
    }

    #[test]
    fn pn_local_esc_is_exactly_the_twenty_scalars_the_production_lists() {
        // An independent transcription: the production's own alternation, in the
        // order the grammar writes it, against a table that had to coalesce the
        // set into ranges to be searchable.
        let oracle = |c: char| {
            matches!(
                c,
                '_' | '~'
                    | '.'
                    | '-'
                    | '!'
                    | '$'
                    | '&'
                    | '\''
                    | '('
                    | ')'
                    | '*'
                    | '+'
                    | ','
                    | ';'
                    | '='
                    | '/'
                    | '?'
                    | '#'
                    | '@'
                    | '%'
            )
        };
        for c in all_scalars() {
            assert_eq!(is_pn_local_esc(c), oracle(c), "{c:?}");
        }
        assert_eq!(all_scalars().filter(|&c| is_pn_local_esc(c)).count(), 20);
    }

    #[test]
    fn pn_local_esc_refuses_the_punctuation_beside_the_members_it_names() {
        // The coalesced `[#x23-#x2F]` run makes the gaps the thing to pin: each
        // of these sits one or two scalars from a member and is NOT escapable,
        // so `ex:a\"b` cannot mint a local name carrying a quote.
        for c in ['"', '<', '>', '\\', '[', ']', '^', '`', '{', '|', '}', ':'] {
            assert!(!is_pn_local_esc(c), "{c:?}");
        }
        // Their lawful neighbours, so this is the production and not a narrowed
        // punctuation alphabet.
        for c in ['!', '#', '/', ';', '=', '?', '@', '_', '~', '%'] {
            assert!(is_pn_local_esc(c), "{c:?}");
        }
        // Names and digits are escaped by nothing: `\a` is not a `PN_LOCAL_ESC`.
        for c in ['a', 'Z', '0', '\u{4E2D}'] {
            assert!(!is_pn_local_esc(c), "{c:?}");
        }
    }

    #[test]
    fn the_iriref_content_class_is_wrong_in_both_directions_under_is_control() {
        // The admitted-but-excluded side: SPACE is `#x00-#x20`, so `<urn:ex:a b>`
        // has no reading, yet `char::is_control` calls SPACE ordinary content.
        assert!(is_iriref_forbidden(' '));
        assert!(!' '.is_control());
        // The excluded-but-admitted side: the C1 controls are LAWFUL raw in an
        // `IRIREF` body, and `is_control` refuses all of them.
        for c in ['\u{7F}', '\u{80}', '\u{9F}'] {
            assert!(c.is_control(), "{c:?}");
            assert!(!is_iriref_forbidden(c), "{c:?}");
        }
        // The four delimiters `is_control` says nothing about at all.
        for c in ['`', '|', '^', '\\'] {
            assert!(!c.is_control(), "{c:?}");
            assert!(is_iriref_forbidden(c), "{c:?}");
        }
        // All nine delimiters, and the whole excluded range's endpoints.
        for c in ['<', '>', '"', '{', '}', '|', '^', '`', '\\'] {
            assert!(is_iriref_forbidden(c), "{c:?}");
        }
        assert!(is_iriref_forbidden('\u{0}'));
        assert!(is_iriref_forbidden('\u{20}'));
        assert!(!is_iriref_forbidden('\u{21}'));
        // Non-ASCII whitespace is content, not a boundary: the round trip the
        // workspace's writers depend on.
        for c in ['\u{A0}', '\u{2000}', '\u{2028}', '\u{3000}'] {
            assert!(c.is_whitespace(), "{c:?}");
            assert!(!is_iriref_forbidden(c), "{c:?}");
        }
    }

    #[test]
    fn ws_char_is_the_four_members_and_nothing_that_merely_looks_like_them() {
        for c in [' ', '\t', '\r', '\n'] {
            assert!(is_ws_char(c), "{c:?}");
        }
        for c in ['\u{B}', '\u{C}', '\u{85}', '\u{A0}', '\u{1680}', '\u{3000}'] {
            assert!(c.is_whitespace(), "{c:?}");
            assert!(!is_ws_char(c), "{c:?}");
        }
        assert_eq!(all_scalars().filter(|&c| is_ws_char(c)).count(), 4);
    }

    #[test]
    fn an_xml_name_starts_with_a_narrower_class_than_it_continues_with() {
        // Stated as a total function: `NameStartChar` ⊂ `NameChar` over every
        // scalar value, so no future edit can widen the head past the tail.
        for c in all_scalars() {
            if is_xml_name_start_char(c) {
                assert!(is_xml_name_char(c), "{c:?}");
            }
        }
        // And the containment is PROPER in both positions, so the subset test
        // above is not satisfied by the two classes being one class.
        for c in ['-', '.', '0', '9', '\u{B7}', '\u{300}', '\u{203F}'] {
            assert!(is_xml_name_char(c), "{c:?} must continue a Name");
            assert!(!is_xml_name_start_char(c), "{c:?} must not open a Name");
        }
        // The neighbouring lawful case: these open a Name and continue one, so
        // the refusals above are position-dependence and not a narrowed
        // alphabet.
        for c in [':', '_', 'a', 'Z', '\u{C0}', '\u{4E2D}'] {
            assert!(is_xml_name_start_char(c), "{c:?}");
            assert!(is_xml_name_char(c), "{c:?}");
        }
    }

    #[test]
    fn xml_name_start_char_is_pn_chars_base_plus_exactly_the_colon_and_underscore() {
        // The difference, asserted POSITIVELY and in both directions over every
        // scalar, so that "simplifying" either predicate into an alias of the
        // other fails here rather than in a user's document.
        for c in all_scalars() {
            assert_eq!(
                is_xml_name_start_char(c),
                is_pn_chars_base(c) || c == ':' || c == '_',
                "{c:?}"
            );
            // Every `PN_CHARS_BASE` member is an XML name start; the converse
            // fails on exactly two scalars.
            if is_pn_chars_base(c) {
                assert!(is_xml_name_start_char(c), "{c:?}");
            }
        }
        // The two scalars, named: XML admits them, SPARQL/Turtle do not.
        assert!(is_xml_name_start_char(':'));
        assert!(!is_pn_chars_base(':'));
        assert!(is_xml_name_start_char('_'));
        assert!(!is_pn_chars_base('_'));
        // `PN_CHARS_U` closes only half the gap, so it is not the alias either.
        assert!(is_pn_chars_u('_'));
        assert!(!is_pn_chars_u(':'));
        // Exactly two scalars separate them — a count, so a third slipping in
        // is a failure and not a silently wider class.
        assert_eq!(
            all_scalars()
                .filter(|&c| is_xml_name_start_char(c) != is_pn_chars_base(c))
                .count(),
            2
        );
    }

    #[test]
    fn xml_name_char_is_pn_chars_plus_exactly_the_colon_and_the_dot() {
        // `NameChar` adds to `NameStartChar` precisely what `PN_CHARS` adds to
        // `PN_CHARS_U`, plus `'.'`; with the `':'` inherited from the head
        // class that makes exactly two scalars of difference from `PN_CHARS`.
        for c in all_scalars() {
            assert_eq!(
                is_xml_name_char(c),
                is_pn_chars(c) || c == ':' || c == '.',
                "{c:?}"
            );
        }
        // The `'.'`, positively: this is why an `xsd:NCName` and a Turtle local
        // name disagree. Turtle admits a dot only BETWEEN name characters, by
        // the shape of the production and never by the character class.
        assert!(is_xml_name_char('.'));
        assert!(!is_pn_chars('.'));
        // The `':'`, positively, in the continue position too.
        assert!(is_xml_name_char(':'));
        assert!(!is_pn_chars(':'));
        assert_eq!(
            all_scalars()
                .filter(|&c| is_xml_name_char(c) != is_pn_chars(c))
                .count(),
            2
        );
        // And the neighbour that must still agree: `'-'` is in both classes, so
        // the two differences above are the only ones.
        assert!(is_xml_name_char('-'));
        assert!(is_pn_chars('-'));
    }

    #[test]
    fn an_alphabetic_scalar_is_not_thereby_an_xml_name_start_char() {
        // U+00AA is `Alphabetic`, and it sits in the hole below the table's
        // first non-ASCII range `[#xC0-#xD6]`. Every approximation that reached
        // for `char::is_alphabetic` admitted it, and no XML processor does.
        assert!('\u{AA}'.is_alphabetic());
        assert!(!is_xml_name_start_char('\u{AA}'));
        assert!(!is_xml_name_char('\u{AA}'));
        // Its neighbours in the same hole, for the same reason.
        for c in ['\u{B5}', '\u{BA}', '\u{D7}', '\u{F7}', '\u{37E}'] {
            assert!(!is_xml_name_start_char(c), "{c:?}");
            assert!(!is_xml_name_char(c), "{c:?}");
        }
        // The neighbouring VALID case, so this is exactness and not an
        // ASCII-only refusal: U+00C0 opens the first non-ASCII range and is a
        // `NameStartChar`, as are the range's other end and a CJK ideograph.
        assert!(is_xml_name_start_char('\u{C0}'));
        assert!(is_xml_name_start_char('\u{D6}'));
        assert!(is_xml_name_start_char('\u{4E2D}'));
        // U+00B7 MIDDLE DOT is the mirror trap: not alphabetic, not a name
        // START, and yet a lawful name CONTINUE.
        assert!(!'\u{B7}'.is_alphabetic());
        assert!(!is_xml_name_start_char('\u{B7}'));
        assert!(is_xml_name_char('\u{B7}'));
    }

    #[test]
    fn the_combining_block_continues_an_xml_name_and_never_opens_one() {
        // `[#x300-#x36F]` is 112 scalars, and it is in `NameChar` alone. Swept
        // exhaustively rather than sampled, in BOTH directions, because a head
        // class that admitted a combining mark would let a name begin with an
        // accent that has nothing to sit on.
        const BLOCK: std::ops::RangeInclusive<char> = '\u{300}'..='\u{36F}';
        assert_eq!(BLOCK.count(), 112);
        for c in BLOCK {
            assert!(is_xml_name_char(c), "{c:?} must continue a Name");
            assert!(!is_xml_name_start_char(c), "{c:?} must not open a Name");
        }
        // The scalars on either side of the block OPEN a name — U+02FF ends
        // `[#xF8-#x2FF]` and U+0370 opens `[#x370-#x37D]` — so the
        // continue-only region is exactly the block and the refusal above is
        // about combining marks rather than about everything near them.
        assert!(is_xml_name_start_char('\u{2FF}'));
        assert!(is_xml_name_start_char('\u{370}'));
        // Which pins the edges from the other side as well: the combining
        // block is the only run of 112 scalars this class treats differently
        // from the class that opens a name.
        assert_eq!(
            ('\u{2FF}'..='\u{370}')
                .filter(|&c| is_xml_name_char(c) && !is_xml_name_start_char(c))
                .count(),
            112
        );
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
        const { assert!(is_ws_char(' ')) }
        const { assert!(!is_ws_char('\u{A0}')) }
        const { assert!(is_pn_local_esc('~')) }
        const { assert!(!is_pn_local_esc('"')) }
        const { assert!(is_iriref_forbidden_byte(b' ')) }
        const { assert!(!is_iriref_forbidden('\u{A0}')) }
        const { assert!(is_xml_name_start_char(':')) }
        const { assert!(is_xml_name_start_char('_')) }
        const { assert!(!is_xml_name_start_char('\u{AA}')) }
        const { assert!(!is_xml_name_start_char('.')) }
        const { assert!(is_xml_name_char('.')) }
        const { assert!(!is_xml_name_char('\u{AA}')) }
    }
}
