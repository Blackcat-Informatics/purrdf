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
    /// generated [`super::blocks::UNICODE_BLOCKS`] table (i.e. is not a
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
                 DFA-based regex engine that cannot backtrack (see \
                 https://github.com/Blackcat-Informatics/purrdf/issues/295)"
            ),
            Self::UnknownBlock(name) => write!(
                f,
                "unrecognized Unicode block name {name:?} in a \\p{{...}}/\\P{{...}} \
                 block escape"
            ),
            Self::Malformed(message) => write!(f, "malformed pattern: {message}"),
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
            | Self::Malformed(_) => None,
        }
    }
}

impl From<regex::Error> for XsdRegexError {
    fn from(err: regex::Error) -> Self {
        Self::Compile(err)
    }
}
