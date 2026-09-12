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
//!
//! The question this fold answers is **meaning**, not whether
//! [`super::emit::translate`] rewrote the token — the two are different in
//! both directions (see [`super::ecma_262_divergences`]'s docs). The `match`
//! over [`Token`] below is deliberately exhaustive over the `Ok` variants, with
//! no catch-all: adding a token to the scanner forces a reviewer to decide
//! here whether its meaning survives the dialect change, rather than silently
//! defaulting to "it does".

use super::Ecma262Divergence;
use super::scan::{Scanner, Token};

/// Scan a pattern for constructs whose meaning **differs between the XSD/XPath
/// dialect and bare ECMA-262** — the dialect JSON Schema's `pattern`,
/// JavaScript, and most `pattern` consumers use.
///
/// Class-awareness mirrors [`super::emit::translate`]'s own decisions (a `.`
/// or a `-[` inside a character class is not the metacharacter), but a
/// construct is reported here when its MEANING changes across the dialect
/// boundary, which is not the same as whether translation rewrote it. Values
/// are returned in first-appearance order with duplicates collapsed (by
/// rendered wording), so the caller's message is deterministic.
///
/// This is emphatically **not** a validity check: every construct named here
/// is a perfectly well-formed `sh:pattern`. It answers the narrower question
/// an emitter has to ask before it copies the source text into a different
/// dialect's slot.
pub(super) fn ecma_262_divergences(pattern: &str) -> Vec<Ecma262Divergence> {
    let mut found: Vec<Ecma262Divergence> = Vec::new();
    let mut note = |divergence: Ecma262Divergence| {
        let rendered = divergence.to_string();
        if !found
            .iter()
            .any(|existing| existing.to_string() == rendered)
        {
            found.push(divergence);
        }
    };
    let mut scanner = Scanner::new(pattern);
    while let Some(token) = scanner.next() {
        match token {
            // ECMA-262 has no such escape. In its non-Unicode mode `\i` is
            // even an *identity escape* — a literal `i` — so copying the text
            // across is silently wrong, not an error the consumer would
            // report.
            Ok(Token::NameEscape { .. }) => note(Ecma262Divergence::NameEscape),
            // XSD `\s` is four code points; ECMA-262's adds vertical tab,
            // form feed, NBSP, BOM and the Unicode space separators.
            Ok(Token::SpaceEscape { .. }) => note(Ecma262Divergence::SpaceEscape),
            // XSD `\w` is `[^\p{P}\p{Z}\p{C}]`; ECMA-262's is `[A-Za-z0-9_]`.
            Ok(Token::WordEscape { .. }) => note(Ecma262Divergence::WordEscape),
            // XSD `\d` is `\p{Nd}` (every Unicode decimal digit);
            // ECMA-262's is `[0-9]`. Reported despite the verbatim
            // pass-through: the MEANING differs.
            Ok(Token::Escape('d' | 'D')) => note(Ecma262Divergence::DigitEscape),
            // EVERY `\p{…}`/`\P{…}` diverges, and for one of two reasons:
            //
            // * an `Is`-prefixed BLOCK name is not an ECMA-262 concept at
            //   all, so the construct cannot mean the same thing in any
            //   ECMA-262 mode;
            // * an Appendix G general CATEGORY (`\p{L}`, `\p{Nd}`, …) is
            //   only a Unicode property escape under ECMA-262's `u` flag —
            //   without it `\p` is an IdentityEscape and the text matches the
            //   literal characters `p{L}`. JSON Schema's `pattern` is a bare
            //   string with no flag surface, so the `u` flag can never be
            //   set, and the meaning changes. The flag path in the public
            //   wrapper only fires for `sh:flags`, so it cannot cover this.
            Ok(Token::UnicodeProperty { name, .. }) => {
                if name.starts_with("Is") {
                    note(Ecma262Divergence::UnicodeBlock { name });
                } else {
                    note(Ecma262Divergence::UnicodeCategory { name });
                }
            }
            // XSD's `.` excludes #x0A and #x0D; ECMA-262's also excludes
            // U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR. A
            // narrow divergence, but a real one.
            Ok(Token::Dot) => note(Ecma262Divergence::DotWildcard),
            // ECMA-262 has no class subtraction: `[a-z-[aeiou]]` parses there
            // as the class `a-z`, `-`, `[`, `aeiou` followed by a stray `]` —
            // a different language, accepted without complaint.
            Ok(Token::Subtract) => note(Ecma262Divergence::ClassSubtraction),
            // The scanner rejects an unterminated `\p{...` name by consuming
            // its whole spelling, leaving no [`Token::UnicodeProperty`] to
            // read. The character cursor this fold replaces still recognized
            // a block escape there, so the same divergence is recovered from
            // the error's source text; no other rejected construct can begin
            // `\p{`/`\P{`. The recovered name is classified by the same `Is`
            // rule as the token path, so the two cannot disagree.
            Err(_) => {
                let source = scanner.source_text();
                if let Some(rest) = source
                    .strip_prefix("\\p{")
                    .or_else(|| source.strip_prefix("\\P{"))
                {
                    if rest.starts_with("Is") {
                        note(Ecma262Divergence::UnicodeBlock {
                            name: rest.to_owned(),
                        });
                    } else {
                        note(Ecma262Divergence::UnicodeCategory {
                            name: rest.to_owned(),
                        });
                    }
                }
            }
            // Literals, class brackets, grouping parentheses, back-references,
            // and every other bare escape delimit or mean the same construct
            // in both dialects. Listed explicitly (no `_` catch-all) so a new
            // scanner token cannot slip past this decision.
            Ok(
                Token::Literal(_)
                | Token::ClassMember(_)
                | Token::Escape(_)
                | Token::ClassOpen { .. }
                | Token::ClassClose
                | Token::GroupOpen { .. }
                | Token::GroupClose
                | Token::Backreference(_),
            ) => {}
        }
    }
    found
}
