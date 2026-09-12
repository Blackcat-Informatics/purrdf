// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The ECMA-262 dialect-divergence scan that backs
//! [`super::ecma_262_divergences`].
//!
//! The translation this module used to perform now lives in [`super::emit`],
//! folded over the one cursor in [`super::scan`]; this module keeps only the
//! question an emitter must ask *before* it copies a `sh:pattern`'s source text
//! into an ECMA-262 slot: would that copy change the accepted language?
//!
//! [`ecma_262_divergences`] is a single fold over [`Scanner`]'s token stream:
//! it owns no character cursor and no class bookkeeping. Every character
//! index, every lookahead, and the character-class nesting that decides whether
//! a `.` or a `-[` is the metacharacter or a literal are tokenized once in
//! [`super::scan`] and only read from the yielded [`Token`]s here.

use super::scan::{Scanner, Token};

/// Scan a pattern for constructs that do **not** carry the same meaning in
/// ECMA-262 — the dialect JSON Schema's `pattern`, JavaScript, and most
/// `pattern` consumers use.
///
/// Class-awareness mirrors [`super::emit::translate`]'s own decisions exactly
/// (a `.` or a `-[` inside a character class is not the metacharacter), so a
/// construct is reported here if and only if translation actually rewrote it.
/// Names are returned in first-appearance order with duplicates collapsed, so
/// the caller's message is deterministic.
///
/// This is emphatically **not** a validity check: every construct named here
/// is a perfectly well-formed `sh:pattern`. It answers the narrower question
/// an emitter has to ask before it copies the source text into a different
/// dialect's slot.
pub(super) fn ecma_262_divergences(pattern: &str) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = Vec::new();
    let mut note = |name: &'static str| {
        if !found.contains(&name) {
            found.push(name);
        }
    };
    let mut scanner = Scanner::new(pattern);
    while let Some(token) = scanner.next() {
        match token {
            // ECMA-262 has no such escape. In its non-Unicode mode `\i` is
            // even an *identity escape* — a literal `i` — so copying the text
            // across is silently wrong, not an error the consumer would
            // report.
            Ok(Token::NameEscape { .. }) => note("\\i/\\I/\\c/\\C"),
            // XSD `\s` is four code points; ECMA-262's adds vertical tab,
            // form feed, NBSP, BOM and the Unicode space separators.
            Ok(Token::SpaceEscape { .. }) => note("\\s/\\S"),
            // XSD `\w` is `[^\p{P}\p{Z}\p{C}]`; ECMA-262's is `[A-Za-z0-9_]`.
            Ok(Token::WordEscape { .. }) => note("\\w/\\W"),
            // XSD `\d` is `\p{Nd}` (every Unicode decimal digit);
            // ECMA-262's is `[0-9]`.
            Ok(Token::Escape('d' | 'D')) => note("\\d/\\D"),
            // Only the `Is`-prefixed BLOCK form diverges; a general-category
            // escape is shared syntax (though ECMA-262 needs its `u` flag for
            // it, which JSON Schema's flagless `pattern` cannot set — recorded
            // through the flag path, not here).
            Ok(Token::UnicodeProperty { name, .. }) if name.starts_with("Is") => {
                note("\\p{Is…} block escape");
            }
            // XSD's `.` excludes #x0A and #x0D; ECMA-262's also excludes
            // U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR. A
            // narrow divergence, but a real one.
            Ok(Token::Dot) => note(". wildcard"),
            // ECMA-262 has no class subtraction: `[a-z-[aeiou]]` parses there
            // as the class `a-z`, `-`, `[`, `aeiou` followed by a stray `]` —
            // a different language, accepted without complaint.
            Ok(Token::Subtract) => note("[…-[…]] class subtraction"),
            // The scanner rejects an unterminated `\p{...` name by consuming
            // its whole spelling, leaving no [`Token::UnicodeProperty`] to
            // read. The character cursor this fold replaces still recognized
            // an `Is`-prefixed block escape there, so the same divergence is
            // recovered from the error's source text; no other rejected
            // construct can begin `\p{`/`\P{`.
            Err(_) => {
                let source = scanner.source_text();
                if source.starts_with("\\p{Is") || source.starts_with("\\P{Is") {
                    note("\\p{Is…} block escape");
                }
            }
            // Literals, class brackets, grouping parentheses, back-references,
            // and every other bare escape delimit or mean the same construct
            // in both dialects.
            Ok(_) => {}
        }
    }
    found
}
