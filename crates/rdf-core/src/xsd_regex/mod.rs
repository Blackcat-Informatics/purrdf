// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XSD/XPath regular-expression dialect support.
//!
//! `sh:pattern` (SHACL §4.5.3), SPARQL `REGEX`/`REPLACE` (§17.4.3.14), and
//! ShEx `PATTERN` (§5.4.5) all specify their pattern facet via *XPath and
//! XQuery Functions and Operators 3.1* §5.6 `fn:matches`/`fn:replace`, which
//! in turn reuses the `regExp` grammar of XML Schema Part 2 Appendix G.
//! That grammar is a distinct dialect from the `regex` crate's own syntax:
//! [`compile`] is the one shared translation between the two, so `sh:pattern`,
//! `REGEX`/`REPLACE`, and `PATTERN` all carry the same accept set and the
//! same semantics instead of three independently-maintained copies (ETHOS
//! §O).
//!
//! # The one permanent limitation: backreferences
//!
//! XPath F&O 3.1 §5.6.1.4 adds `backReference ::= "\" [1-9][0-9]*` to the
//! `fn:matches` grammar, so backreferences ARE part of the governing
//! dialect. This module rejects every one of them
//! ([`XsdRegexError::Backreference`]), and always will: translation targets
//! the `regex` crate, whose matching engine is a DFA that cannot backtrack,
//! so it structurally cannot execute a backreference no matter how the
//! source text is rewritten. This is a permanent, by-design gap in this
//! implementation, not a bug to be fixed later — supporting backreferences
//! would require subsuming a second, backtracking engine, which is exactly
//! the design this module exists instead of. It is recorded as a known gap
//! in `docs/CONFORMANCE.md` and pinned by the first-party corpus under
//! `crates/rdf-core/corpus/xsd-regex/`.
//!
//! # Everything else: translated, not subsumed
//!
//! Every other construct in the grammar (see `translate`'s module doc for
//! the full construct-by-construct table) is translated into `regex`-crate
//! syntax at compile time: character-class subtraction, the `\i \I \c \C`
//! XML-name multi-character escapes, the `\s \S \w \W` classes (XSD defines
//! these more narrowly than Rust's Unicode defaults), `\p{IsX}`/`\P{IsX}`
//! Unicode block escapes (resolved against the generated block table in
//! this module's private `blocks` submodule, never left to `regex-syntax`'s
//! own — silently different — resolution),
//! the `.` wildcard's `#xA`/`#xD` exclusion, and the `i s m x q` flags.
//!
//! One documented, tested, permanent MINOR divergence remains: under the
//! `m` flag, XPath's `^` excludes the position immediately after a newline
//! that is the last character in the string, and Rust's `multi_line` has no
//! way to express that one exception (pinned by the
//! `known_divergence_m_flag_trailing_newline` test in `translate` — this
//! plan intentionally pins the ACTUAL divergent behavior with a test rather
//! than relying on it silently, so a future `regex` upgrade that happens to
//! change this is caught, not silently trusted).

mod blocks;
mod classes;
mod error;
mod scan;
mod translate;
mod xflag;

pub use error::XsdRegexError;

use std::ops::Deref;

/// A `sh:pattern`/`REGEX`/`PATTERN` pattern compiled by [`compile`].
///
/// Dereferences to the underlying [`regex::Regex`], so existing call sites
/// that only ever called `.is_match()`/`.replace_all()`/etc. on a
/// `&regex::Regex` need no change beyond the type they store.
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    regex: regex::Regex,
    is_literal: bool,
}

impl CompiledPattern {
    /// Whether the source pattern was compiled under the `q` (literal) flag.
    ///
    /// SPARQL's `REPLACE()` needs this bit: per XPath F&O 3.1 §5.6.2, `$`
    /// and `\` in the *replacement* string lose their special meaning when
    /// the pattern was `q`-flagged, so a `q`-literal match must be replaced
    /// with [`regex::Regex::replace_all`] plus [`regex::NoExpand`] rather
    /// than the default `$N`-expanding replacement string.
    #[must_use]
    pub fn is_literal(&self) -> bool {
        self.is_literal
    }

    /// The compiled [`regex::Regex`], for callers that prefer an explicit
    /// accessor over [`Deref`].
    #[must_use]
    pub fn as_regex(&self) -> &regex::Regex {
        &self.regex
    }
}

impl Deref for CompiledPattern {
    type Target = regex::Regex;

    fn deref(&self) -> &regex::Regex {
        &self.regex
    }
}

/// Compile an XSD/XPath `regExp` pattern (`sh:pattern`/`REGEX`/`PATTERN`
/// source text) plus its XPath F&O 3.1 §5.6.2 flag string into a
/// [`CompiledPattern`].
///
/// `flags` may contain any combination of `i s m x q`; the empty string
/// means no flags. Any other character is
/// [`XsdRegexError::UnsupportedFlag`].
///
/// # Errors
///
/// Returns [`XsdRegexError`] naming the exact unsupported flag character,
/// unsupported construct (`\b`/`\B`), backreference, unrecognized Unicode
/// block name, or pattern malformation (see [`XsdRegexError`]'s variants)
/// that caused translation or the underlying `regex` compile to fail.
pub fn compile(pattern: &str, flags: &str) -> Result<CompiledPattern, XsdRegexError> {
    let mut case_insensitive = false;
    let mut dot_all = false;
    let mut multi_line = false;
    let mut strip_whitespace = false;
    let mut literal = false;
    for flag in flags.chars() {
        match flag {
            'i' => case_insensitive = true,
            's' => dot_all = true,
            'm' => multi_line = true,
            'x' => strip_whitespace = true,
            'q' => literal = true,
            other => return Err(XsdRegexError::UnsupportedFlag(other)),
        }
    }

    if literal {
        // XPath F&O 3.1 §5.6.2: "This flag [`q`] can be used in conjunction
        // with the `i` flag. If it is used together with the `m`, `s`, or
        // `x` flag, that flag has no effect." All four are still PARSED
        // above (so a genuinely unknown flag letter still errors under `q`),
        // but `m`/`s`/`x` are silently ignored here rather than applied —
        // in particular `x`'s whitespace stripping must NOT run over a
        // `regex::escape`d literal: `regex::escape` does not escape spaces
        // (confirmed: `regex::escape("a b#c")` == `"a b\\#c"`), so applying
        // `x` afterward would delete literal spaces from the pattern, which
        // is exactly the bug this module does not repeat.
        let escaped = regex::escape(pattern);
        let regex = regex::RegexBuilder::new(&escaped)
            .case_insensitive(case_insensitive)
            .build()?;
        return Ok(CompiledPattern {
            regex,
            is_literal: true,
        });
    }

    // The `x` removal is a rewrite of the pattern SOURCE and must happen
    // before translation (and before the builder ever sees the pattern) —
    // `RegexBuilder::ignore_whitespace` is never used, it implements a
    // wider, different rule (see `xflag::strip_x_flag_whitespace`'s doc).
    let source = if strip_whitespace {
        xflag::strip_x_flag_whitespace(pattern)
    } else {
        pattern.to_owned()
    };
    let translated = translate::translate(&source, dot_all)?;
    let regex = regex::RegexBuilder::new(&translated)
        .case_insensitive(case_insensitive)
        .dot_matches_new_line(dot_all)
        .multi_line(multi_line)
        .build()?;
    Ok(CompiledPattern {
        regex,
        is_literal: false,
    })
}

/// Everything in `(pattern, flags)` that does **not** mean the same thing in
/// ECMA-262 — the dialect JSON Schema's `pattern` keyword, JavaScript, and
/// most other `pattern` consumers are specified in.
///
/// An emitter that copies a `sh:pattern`'s source text into an ECMA-262 slot
/// is performing a dialect change, and this is how it finds out whether that
/// change altered the accepted language. Returns an empty vector when the
/// pattern and its flags mean the same thing in both dialects (the common
/// case — `^[A-Z]+$` and friends).
///
/// Two kinds of divergence are reported:
///
/// * **Constructs**, in first-appearance order, duplicates collapsed, and
///   class-aware — a `.` or a `-[` inside a character class is a literal, not
///   a metacharacter, and is not reported. The list mirrors the translation
///   table exactly: a construct appears here if and only if
///   [`compile`] actually rewrote it.
/// * **Flags**, because ECMA-262 regular-expression *literals* have flags but
///   JSON Schema's `pattern` is a bare string with **no flag surface at all**,
///   so every flag is lost — and `x` and `q` have no ECMA-262 spelling even
///   where flags can be expressed.
///
/// This is not a validity check. Every input it reports on is a perfectly
/// well-formed `sh:pattern`; the question is only whether its meaning
/// survives the copy.
#[must_use]
pub fn ecma_262_divergences(pattern: &str, flags: &str) -> Vec<String> {
    let mut out: Vec<String> = translate::ecma_262_divergences(pattern)
        .into_iter()
        .map(ToOwned::to_owned)
        .collect();
    if !flags.is_empty() {
        out.push(format!(
            "the {flags:?} flag(s) (JSON Schema's `pattern` is a bare ECMA-262 \
             source string with no flag surface)"
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_flag_character_is_named() {
        let err = compile("abc", "z").unwrap_err();
        assert_eq!(err, XsdRegexError::UnsupportedFlag('z'));
    }

    #[test]
    fn anchors_without_m_flag_anchor_the_whole_string() {
        let re = compile("^ab$", "").expect("compile");
        assert!(re.is_match("ab"));
        assert!(!re.is_match("xab"));
        assert!(!re.is_match("abx"));
    }

    #[test]
    fn unanchored_match_is_partial() {
        let re = compile("bc", "").expect("compile");
        assert!(re.is_match("abcd"));
        assert!(!re.is_match("abd"));
    }

    #[test]
    fn dollar_with_m_flag_is_line_end() {
        let re = compile("a$", "m").expect("compile");
        assert!(re.is_match("a\nb"));
        assert!(re.is_match("xa"));
    }

    /// XPath F&O 3.1 §5.6.2: under `m`, `^` matches "the position
    /// immediately after a newline character other than a newline that
    /// appears as the last character in the string" — i.e. NOT immediately
    /// after a *trailing* newline. Rust's `multi_line` has no lookaround
    /// expressive enough to carve out that one position, so it DOES match
    /// there — a permanent, documented, minor divergence. This test pins
    /// the ACTUAL (divergent) behavior so a future `regex` upgrade that
    /// happens to change it is caught by a test failure, not silently
    /// relied upon or silently drifted further.
    #[test]
    fn known_divergence_m_flag_trailing_newline() {
        let re = compile("^", "m").expect("compile");
        let positions: Vec<usize> = re.find_iter("a\n").map(|m| m.start()).collect();
        // Spec-correct XPath behavior would be `[0]` only (position 2, right
        // after the trailing newline, should NOT match). Rust's actual,
        // divergent behavior matches at both position 0 and position 2.
        assert_eq!(
            positions,
            vec![0, 2],
            "if this now reads [0], regex's multi_line semantics changed and \
             the documented divergence in this module's doc comment is stale"
        );
    }

    #[test]
    fn q_flag_treats_pattern_as_literal() {
        let re = compile("a.c", "q").expect("compile");
        assert!(re.is_match("xa.cx"));
        assert!(!re.is_match("abc"));
        assert!(re.is_literal());
    }

    #[test]
    fn q_flag_composes_with_i() {
        let re = compile("ABC", "qi").expect("compile");
        assert!(re.is_match("abc"));
    }

    #[test]
    fn q_flag_ignores_m_and_s_and_x() {
        // `s` would normally make `.` match anything; under `q` the pattern
        // is entirely literal, so it does not matter -- but this also
        // confirms `s`/`m` do not somehow cause an error or a non-literal
        // recompile.
        let re = compile("a.b", "qs").expect("compile");
        assert!(re.is_match("xa.bx"));
        assert!(!re.is_match("axb"));
        assert!(!re.is_match("a\nb"));
    }

    /// The exact bug this module must not repeat (see `translate.rs`'s
    /// `xpath_x_flag_matches_the_specifications_examples` for the `x`-only
    /// case): `regex::escape` does not escape a literal space, so applying
    /// `x`'s whitespace stripping AFTER escaping a `q`-literal pattern would
    /// silently delete the space. Under `q`, `x` must have zero effect.
    #[test]
    fn q_and_x_together_preserve_literal_spaces() {
        let re = compile("a b", "qx").expect("compile");
        assert!(re.is_match("xa bx"));
        assert!(!re.is_match("xabx"));
    }

    #[test]
    fn x_flag_strips_whitespace_outside_classes_only() {
        let re = compile("a b[c d]", "x").expect("compile");
        assert!(re.is_match("ab d")); // outside-class space stripped
        assert!(!re.is_match("a bd")); // ...so this no longer matches
    }

    #[test]
    fn s_flag_makes_dot_match_newlines() {
        let re = compile("^a.b$", "s").expect("compile");
        assert!(re.is_match("a\nb"));
        assert!(re.is_match("a\rb"));
    }

    #[test]
    fn i_flag_is_case_insensitive() {
        let re = compile("^abc$", "i").expect("compile");
        assert!(re.is_match("ABC"));
    }

    #[test]
    fn dot_excludes_lf_and_cr_by_default() {
        let re = compile("^a.b$", "").expect("compile");
        assert!(re.is_match("axb"));
        assert!(!re.is_match("a\nb"));
        assert!(!re.is_match("a\rb"));
    }

    #[test]
    fn backreference_is_rejected_with_the_exact_reference() {
        let err = compile(r"(a)\1", "").unwrap_err();
        assert_eq!(err, XsdRegexError::Backreference("\\1".to_owned()));
    }

    #[test]
    fn bare_word_boundary_escapes_are_rejected() {
        assert_eq!(
            compile(r"a\bc", "").unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\b")
        );
        assert_eq!(
            compile(r"[\B]", "").unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\B")
        );
    }

    #[test]
    fn unknown_block_escape_is_named() {
        let err = compile(r"\p{IsNotARealUnicodeBlock}", "").unwrap_err();
        assert_eq!(
            err,
            XsdRegexError::UnknownBlock("IsNotARealUnicodeBlock".to_owned())
        );
    }

    #[test]
    fn compiled_pattern_derefs_to_regex() {
        let compiled = compile("^ab$", "").expect("compile");
        // `.is_match` here resolves through `Deref<Target = regex::Regex>`.
        assert!(compiled.is_match("ab"));
        assert!(!compiled.is_literal());
        assert_eq!(compiled.as_regex().as_str(), compiled.as_str());
    }
}
