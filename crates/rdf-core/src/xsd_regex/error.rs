// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The precise, named failure modes of [`super::compile`].
//!
//! Every variant states, in its `Display` text, exactly what construct or
//! character was rejected and why (ETHOS §H: "errors are communication") --
//! never a generic "invalid pattern" message. This is deliberately a flat
//! enum rather than a single string: callers (SHACL, SPARQL, ShEx) each want
//! to render their own diagnostic prefix around the same precise cause.

use std::fmt;

/// Why [`super::compile`] rejected an XSD/XPath `sh:pattern`/`REGEX`/`PATTERN`
/// source string, or its `flags` string.
#[derive(Debug, Clone, PartialEq)]
pub enum XsdRegexError {
    /// A character in the `flags` argument is not one of `i s m x q`.
    UnsupportedFlag(char),
    /// A construct the XSD/XPath `regExp` grammar does not define at all
    /// (currently only `\b` and `\B`, XPath word-boundary escapes with no
    /// equivalent in the governing grammar). Carries the exact two-character
    /// spelling that was rejected (e.g. `"\\b"`).
    UnsupportedConstruct(&'static str),
    /// A backreference (`\1` .. `\9`, or any `\` followed by
    /// `[1-9][0-9]*` per XPath F&O 3.1 §5.6.1.4's `backReference` grammar
    /// production). This is a **permanent, by-design** limitation of this
    /// implementation, not a bug: `purrdf_core::xsd_regex` translates
    /// patterns onto the `regex` crate's DFA-based engine, which cannot
    /// backtrack and therefore cannot ever support backreferences. Carries
    /// the exact backreference spelling that was rejected (e.g. `"\\12"`).
    Backreference(String),
    /// A `\p{Is…}`/`\P{Is…}` block-escape name that does not appear in the
    /// generated `super::blocks::UNICODE_BLOCKS` table (i.e. is not a
    /// recognized Unicode block name, per XML Schema Part 2 §G.4.2.3's
    /// "normalized block name" transform). Carries the exact name that was
    /// looked up (e.g. `"IsNotARealBlock"`, including the `Is` prefix).
    UnknownBlock(String),
    /// The pattern source is malformed independently of any specific
    /// construct above -- a dangling trailing backslash, an unterminated
    /// character class (`[` with no matching `]`), or an unterminated
    /// `\p{`/`\P{` block-escape name (no matching `}`). Carries a message
    /// naming the exact defect.
    Malformed(String),
    /// The pattern source, or its translated `regex`-crate form, exceeds a
    /// hard resource bound. Carries the measured size in bytes and the named
    /// limit that was exceeded, so an operator can see both numbers and know
    /// exactly how far over the input was.
    ///
    /// This is a refusal at the [`super::compile`] boundary, not a
    /// silent truncation or a degraded fallback: `sh:pattern`, SPARQL
    /// `REGEX`/`REPLACE`, and ShEx `PATTERN` all admit untrusted text, and on
    /// `wasm32-unknown-unknown` a multi-megabyte pattern is an unrecoverable
    /// `memory.grow` trap rather than a `Result`. The limits are deliberately
    /// far above any real-world pattern (see the constants on
    /// [`super::compile`]).
    TooLarge {
        /// The measured size of the offending string, in bytes.
        bytes: usize,
        /// The named limit (`MAX_SOURCE_BYTES` or `MAX_TRANSLATED_BYTES`) it
        /// exceeded, in bytes.
        limit: usize,
    },
    /// The pattern contains more in-class `\i`/`\I`/`\c`/`\C` name escapes
    /// under the `i` flag than [`super::MAX_FOLDED_CLASS_ESCAPES`] permits.
    /// Carries the counted occurrences and the named limit, so an operator
    /// sees both numbers.
    ///
    /// This is a **case-folding cost bound**, not a syntax restriction: each
    /// of those four escapes expands to a bracket class containing the
    /// ~917k-codepoint astral XML-name range, and `regex-syntax` case-folds
    /// that set once **per occurrence** at Hir-translation time. A standalone
    /// escape is emitted pre-folded inside a `(?-i:…)` scope and is therefore
    /// unlimited, but a group is not a character-class member, so an escape
    /// *inside* a class cannot be scoped and the enclosing `i` re-folds it.
    /// The construct is bounded by count for that reason alone; a pattern
    /// below the limit is emitted exactly as before.
    TooManyFoldedClassEscapes {
        /// The number of in-class name escapes counted in the pattern.
        count: usize,
        /// The named limit (`MAX_FOLDED_CLASS_ESCAPES`) that `count` exceeded.
        limit: usize,
    },
    /// Translation succeeded, but the resulting `regex`-crate source still
    /// failed to compile. This should not happen for any pattern this
    /// module's translation rules produce, but is retained as defense in
    /// depth (ETHOS §H: never swallow a possible failure) rather than
    /// `unwrap()`-ing the builder result.
    Compile(regex::Error),
}

impl fmt::Display for XsdRegexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFlag(c) => write!(
                f,
                "unsupported regex flag character {c:?} (supported: i, s, m, x, q)"
            ),
            Self::UnsupportedConstruct(construct) => write!(
                f,
                "unsupported construct {construct} -- not defined by the XSD/XPath \
                 regExp grammar (XML Schema Part 2 Appendix G)"
            ),
            Self::Backreference(reference) => write!(
                f,
                "backreference {reference} is not supported -- this is a permanent \
                 limitation of this implementation, which translates patterns onto a \
                 DFA-based regex engine that cannot backtrack, and no rewriting of \
                 the source text can reach a capability the engine does not have"
            ),
            Self::UnknownBlock(name) => write!(
                f,
                "unrecognized Unicode block name {name:?} in a \\p{{...}}/\\P{{...}} \
                 block escape"
            ),
            Self::Malformed(message) => write!(f, "malformed pattern: {message}"),
            Self::TooLarge { bytes, limit } => write!(
                f,
                "pattern is {bytes} bytes, which exceeds the {limit}-byte limit"
            ),
            Self::TooManyFoldedClassEscapes { count, limit } => write!(
                f,
                "pattern contains {count} in-class \\i/\\I/\\c/\\C name escapes under the \
                 `i` flag, which exceeds the {limit}-escape limit -- a standalone name \
                 escape is emitted pre-folded and unlimited, but inside a character class \
                 it cannot be scoped and the enclosing `i` case-folds its \
                 ~917k-codepoint range once per occurrence, so the construct is bounded \
                 to keep compile time finite"
            ),
            Self::Compile(err) => write!(f, "pattern failed to compile after translation: {err}"),
        }
    }
}

impl std::error::Error for XsdRegexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Compile(err) => Some(err),
            Self::UnsupportedFlag(_)
            | Self::UnsupportedConstruct(_)
            | Self::Backreference(_)
            | Self::UnknownBlock(_)
            | Self::Malformed(_)
            | Self::TooLarge { .. }
            | Self::TooManyFoldedClassEscapes { .. } => None,
        }
    }
}

impl From<regex::Error> for XsdRegexError {
    fn from(err: regex::Error) -> Self {
        Self::Compile(err)
    }
}
