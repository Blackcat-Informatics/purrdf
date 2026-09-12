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
//! Every other construct in the grammar (see `emit`'s module doc for
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
//! `known_divergence_m_flag_trailing_newline` test in this module — this
//! plan intentionally pins the ACTUAL divergent behavior with a test rather
//! than relying on it silently, so a future `regex` upgrade that happens to
//! change this is caught, not silently trusted).
//!
//! # A recognizer, not a pass-through
//!
//! [`compile`] recognizes the grammar; it does not hand the source text to the
//! `regex` crate and accept whatever that crate accepts. A construct outside
//! the grammar is a named [`XsdRegexError`], never silently compiled with Rust
//! `regex` semantics. The families refused by name:
//!
//! * **Group constructs** — every `(?…` spelling other than the non-capturing
//!   group `(?:` ([`XsdRegexError::UnsupportedGroupConstruct`]): inline flags
//!   `(?i)`, `(?x)`, `(?s)`, `(?m)` and their negations (`(?-…)`), lookaround
//!   (`(?=…)`, `(?!…)`, `(?<=…)`, `(?<!…)`), named groups (`(?<name>…)`,
//!   `(?P<name>…)`) and comments (`(?#…)`).
//! * **Non-grammar escapes** ([`XsdRegexError::UnsupportedEscape`],
//!   [`XsdRegexError::UnsupportedConstruct`],
//!   [`XsdRegexError::UnsupportedNulEscape`]) — `\b`/`\B`, `\0`, and every
//!   escape outside the closed `SingleCharEsc` enumeration and the
//!   multi-character escapes: `\A`, `\z`, `\Z`, `\x41`, `\u{41}`, `\a`, `\f`,
//!   `\v`, `\e`, `\Q`, `\k<…>`, `\G`, `\h`, `\N`, `\R`, `\X`, and a bare
//!   `\p`/`\P`.
//! * **Class-interior Rust-isms** ([`XsdRegexError::UnescapedClassOpen`],
//!   [`XsdRegexError::UnescapedClassClose`],
//!   [`XsdRegexError::LiteralClassCloseAtHead`],
//!   [`XsdRegexError::ContentAfterClassSubtraction`]) — a nested `[` that is
//!   not the operand of a `-[` subtraction, a `]` at the head of a class, an
//!   unescaped `[`/`]` where the grammar has no production, and any content
//!   after a subtraction operand has closed (the operand is the group's last
//!   element). `&`/`~` are ordinary class members here, not the `regex`
//!   crate's set operators.
//! * **Unicode scripts and other property keys**
//!   ([`XsdRegexError::UnknownCategory`]) — `\p{…}` admits only Appendix G's
//!   closed general-category list and `Is`-prefixed Unicode block names
//!   ([`XsdRegexError::UnknownBlock`]); a script name (`Greek`), a
//!   `key=value`/`key:value` property key and a `regex`-crate pseudo-property
//!   are refused by name.
//!
//! # Resource bounds
//!
//! Three named limits, each a hard error rather than a truncation or a
//! fallback, keep a hostile pattern from becoming unbounded work:
//! [`MAX_SOURCE_BYTES`] (64 KiB, bounding the source before the scan and the
//! name-escape expansion), [`MAX_TRANSLATED_BYTES`] (1 MiB, checked after
//! translation and before the engine), and [`MAX_FOLDED_CLASS_ESCAPES`] (32,
//! the number of *in-class* `\i`/`\I`/`\c`/`\C` escapes under `i`, each of
//! which makes `regex-syntax` case-fold a ~917k-codepoint set at
//! Hir-translation time — before its own size limit applies). See those
//! constants for each bound's derivation.
//!
//! # Liberal edges
//!
//! Two acceptances are deliberately wider than the bare Appendix G grammar,
//! recorded here so they are decisions rather than accidents:
//!
//! * `\$` is accepted and is **correct**: XPath F&O 3.1 §5.6.1 extends the XSD
//!   grammar, adding `^` and `$` as metacharacters and `\^`/`\$` to
//!   `SingleCharEsc`, and F&O is the governing dialect (SHACL §4.5.3 → SPARQL
//!   1.1 §17.4.3.14 → `fn:matches`).
//! * `\&` and `\~` are accepted although they are neither `SingleCharEsc` nor
//!   `charClassEsc`. Each reaches the engine as an escape for its own literal
//!   character, so the acceptance is harmless and refusing it would reject a
//!   defensively-escaped pattern; the first-party corpus pins `[a\&b]`/`[\~]`
//!   and the vendored ShExTest corpus pins `\$` as its literal-dollar spelling.
//!
//! `fn:replace`'s `err:FORX0004` (a malformed replacement string) is
//! implemented by [`CompiledPattern::replace_all`]; `err:FORX0003` (the
//! pattern matches a zero-length string) is not.

mod blocks;
mod classes;
mod ecma;
mod emit;
mod error;
mod replace;
mod scan;
mod xflag;

pub use error::{ReplacementError, XsdRegexError};

use std::borrow::Cow;
use std::fmt;

/// Maximum size, in bytes, of a pattern source string accepted by [`compile`].
///
/// 64 KiB is a generous ceiling: it is orders of magnitude beyond any
/// real-world `sh:pattern`/`REGEX`/`PATTERN` (the longest pattern in this
/// repository's own conformance corpus is a few dozen bytes), while still
/// bounding the per-byte work `scan`/`emit` perform before the translated-size
/// bound below applies. The source is materialized as a `Vec<char>` (four
/// bytes per byte of input) and every `\i`/`\c` expands to a ~300-byte
/// bracket class, so an unbounded source is an unbounded amplifier.
pub const MAX_SOURCE_BYTES: usize = 64 * 1024;

/// Maximum size, in bytes, of the translated `regex`-crate source accepted by
/// [`compile`], checked after translation and before the string is handed to
/// [`regex::RegexBuilder`].
///
/// The translation is where amplification actually happens: each `\i`/`\c`
/// (2 source bytes) becomes a ~300-byte bracket class, so a 64 KiB source of
/// nothing but those escapes would translate to megabytes. This bound is the
/// backstop that makes the pathological case a named [`XsdRegexError::TooLarge`]
/// instead of a multi-second Hir-translation walk. It sits well above ordinary
/// expansion — a class-splice `\i` amplifies about 111×, and real patterns are
/// tiny — but far below what a hostile repetition of name escapes produces.
///
/// `regex-syntax` folds case-insensitively at Hir-translation time, before its
/// own `size_limit` is consulted, so this byte bound — together with the
/// in-class name-escape count bound in [`MAX_FOLDED_CLASS_ESCAPES`] — is what
/// stops the translated cost before the engine is entered.
pub const MAX_TRANSLATED_BYTES: usize = 1024 * 1024;

/// Maximum number of `\i`/`\I`/`\c`/`\C` name escapes permitted **inside a
/// character class** while the `i` flag is in force, per [`compile`] call.
///
/// Each of those escapes expands to a bracket-class body containing
/// `\u{10000}-\u{effff}` — 917,504 code points. Standalone, the escape is
/// emitted pre-folded inside a `(?-i:…)` scope (see `emit::translate`), so the
/// enclosing `(?i)` no longer re-walks the set. A group is not a
/// character-class member, however, so an escape *inside* a class cannot carry
/// that scope: it splices as a nested class and `regex-syntax` case-folds the
/// ~917k-codepoint set once per occurrence at Hir-translation time, **before**
/// its own `size_limit` is consulted. Repetition is therefore CPU
/// amplification, and no engine-side bound can stop it, because the fold
/// happens before the engine is entered.
///
/// The bound is a *count* of in-class name escapes, enforced only under `i`:
/// without `i` there is no folding and no amplification, so a pattern far over
/// the limit compiles unchanged, and standalone escapes stay unlimited.
///
/// # Derivation
///
/// The budget is ~250 ms of case-folding work for one `compile`. Measured on
/// the development machine (2026 floating nightly, unoptimized `cargo test`
/// build, constants already initialized): `[\c]` repeated 32 times compiled
/// in 146 ms, 64 times in 250–305 ms across runs, and 128 or more times is
/// refused at once — a marginal ~4.5 ms per in-class occurrence. The budget
/// therefore admits roughly 55 occurrences; it is rounded DOWN to the clean
/// power of two 32, whose measured worst-case compile (146 ms) sits
/// comfortably inside it. Rounding up to 64 was rejected because it straddles
/// the budget (measured 250 ms in one run and 305 ms in another), and a limit
/// that only sometimes meets its own budget is not a bound.
///
/// A census of every pattern this repository ships — the
/// `corpus/xsd-regex/*.cases` files, `sh:pattern`/ShEx `PATTERN`/SPARQL
/// `REGEX` fixtures, and the built-in tests — found a maximum of **one**
/// in-class name escape (in `class-subtraction.cases`), and none under `i`,
/// so the limit is more than 32× above real usage.
pub const MAX_FOLDED_CLASS_ESCAPES: usize = 32;

/// A `sh:pattern`/`REGEX`/`PATTERN` pattern compiled by [`compile`].
///
/// This is the one type the three facets hold, and it exposes **one** accessor
/// to the underlying [`regex::Regex`], [`as_regex`](Self::as_regex). It
/// deliberately does not [`Deref`](std::ops::Deref) to `regex::Regex`: the two
/// have different semantics for `fn:replace` (see
/// [`replace_all`](Self::replace_all)), and a `Deref` would let
/// `compiled.replace_all(…)` silently resolve to the `regex` crate's method
/// with the `q` flag and XPath replacement syntax dropped.
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    regex: regex::Regex,
    is_literal: bool,
}

impl CompiledPattern {
    /// The compiled [`regex::Regex`], for the match-only callers
    /// (`is_match`, `find`, …).
    ///
    /// This is the *only* way to reach the underlying engine: a caller that
    /// needs replacement uses [`replace_all`](Self::replace_all), which
    /// carries the semantics the raw [`regex::Regex`] does not.
    #[must_use]
    pub fn as_regex(&self) -> &regex::Regex {
        &self.regex
    }

    /// `fn:replace` semantics: replace every match of this pattern in
    /// `haystack` with `replacement`, honouring the `q` flag and XPath F&O 3.1
    /// §5.6.2's replacement syntax internally.
    ///
    /// The decision the caller used to have to make by hand lives here, so the
    /// `q` bit cannot be forgotten at a call site:
    ///
    /// * Under `q`, the replacement is used as is — F&O §5.6.2 says so
    ///   explicitly, and the `regex` crate's [`NoExpand`](regex::NoExpand) is
    ///   exactly that. No error is possible.
    /// * Otherwise the replacement is XPath's own language: `$N` references
    ///   the Nth capture group (with F&O's full out-of-range rule), a literal
    ///   `$` is written `\$`, and a literal `\` is written `\\`.
    ///
    /// # Errors
    ///
    /// [`ReplacementError`] — F&O §5.6.2 `err:FORX0004` — when a non-`q`
    /// replacement contains a `$` not followed by a digit or a `\` that is
    /// neither `\\` nor `\$`. SPARQL's `REPLACE` maps this to an unbound
    /// expression, never a query abort.
    pub fn replace_all<'h>(
        &self,
        haystack: &'h str,
        replacement: &str,
    ) -> Result<Cow<'h, str>, ReplacementError> {
        if self.is_literal {
            // `q`: "the replacement string is used as is" (F&O §5.6.2). With
            // `q`, `$` and `\` in the replacement are ordinary characters,
            // exactly `NoExpand`'s contract, and [err:FORX0004] cannot apply.
            return Ok(self
                .regex
                .replace_all(haystack, regex::NoExpand(replacement)));
        }
        // `captures_len` counts group 0 (the whole match); F&O's `S` is the
        // explicit capture-group count, so subtract it once, here.
        let template =
            replace::Template::parse(replacement, self.regex.captures_len().saturating_sub(1))?;
        Ok(self
            .regex
            .replace_all(haystack, |caps: &regex::Captures<'_>| template.expand(caps)))
    }
}

/// One way a construct in a `sh:pattern`/`REGEX`/`PATTERN` source **changes
/// meaning** when its text is copied verbatim into bare ECMA-262 — the dialect
/// JSON Schema's `pattern` keyword, JavaScript, and most other `pattern`
/// consumers are specified in.
///
/// This is data, not prose: a consumer can branch on the variant (and read the
/// carried property name or flag letters) instead of string-matching the
/// English sentence. [`Display`](std::fmt::Display) renders the exact wording
/// the loss ledger has always carried, so existing notes do not move.
///
/// [`ecma_262_divergences`] returns these in first-appearance order, with
/// duplicates collapsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ecma262Divergence {
    /// `\i`/`\I`/`\c`/`\C`: XSD's XML-name multi-character escapes. ECMA-262
    /// has no such escape, and in its non-Unicode mode reads each as an
    /// *identity* escape (a literal letter).
    NameEscape,
    /// `\s`/`\S`: XSD's four code points versus ECMA-262's wider set (which
    /// adds vertical tab, form feed, NBSP, BOM and the Unicode space
    /// separators).
    SpaceEscape,
    /// `\w`/`\W`: XSD's `[^\p{P}\p{Z}\p{C}]` versus ECMA-262's
    /// `[A-Za-z0-9_]`.
    WordEscape,
    /// `\d`/`\D`: XSD's `\p{Nd}` (every Unicode decimal digit) versus
    /// ECMA-262's `[0-9]`. Reported even though [`compile`] passes the escape
    /// through verbatim.
    DigitEscape,
    /// A `\p{…}`/`\P{…}` Appendix G general category (e.g. `Nd`), which
    /// ECMA-262 only accepts as a Unicode property escape under the `u` flag;
    /// JSON Schema's bare `pattern` string has no flag surface to set it, so
    /// it reads `\p` as an IdentityEscape instead. `name` is the property name
    /// without the `\p{…}` wrapper.
    UnicodeCategory {
        /// The Appendix G general-category name (`L`, `Nd`, `Zs`, …).
        name: String,
    },
    /// A `\p{IsX}`/`\P{IsX}` Unicode **block** escape. ECMA-262 has no
    /// block-name concept at all (its `\p{…}` names are properties and
    /// scripts), so the construct is not merely flag-dependent. `name` is the
    /// block name without the `\p{…}` wrapper.
    UnicodeBlock {
        /// The `Is`-prefixed block name (`IsBasicLatin`, …).
        name: String,
    },
    /// The `.` wildcard: XSD excludes `#xA`/`#xD`; ECMA-262 additionally
    /// excludes U+2028 LINE SEPARATOR and U+2029 PARAGRAPH SEPARATOR.
    DotWildcard,
    /// XSD character-class subtraction `[…-[…]]`. ECMA-262 has no class
    /// subtraction, so it re-parses the text as a different class.
    ClassSubtraction,
    /// One non-empty XSD `sh:flags` string. ECMA-262 regular-expression
    /// *literals* carry flags, but JSON Schema's `pattern` is a bare string
    /// with no flag surface at all, so every flag is lost — and `x`/`q` have
    /// no ECMA-262 spelling even where flags can be expressed.
    Flags {
        /// The exact flag letters from `sh:flags`.
        flags: String,
    },
}

impl fmt::Display for Ecma262Divergence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameEscape => f.write_str("\\i/\\I/\\c/\\C"),
            Self::SpaceEscape => f.write_str("\\s/\\S"),
            Self::WordEscape => f.write_str("\\w/\\W"),
            Self::DigitEscape => f.write_str("\\d/\\D"),
            // The block wording is exactly what this reporter has always
            // emitted; `name` is carried for consumers that branch on the
            // variant, not to widen the ledger text.
            Self::UnicodeBlock { .. } => f.write_str("\\p{Is…} block escape"),
            Self::UnicodeCategory { name } => write!(
                f,
                "\\p{{{name}}} category escape (ECMA-262 needs the `u` flag, which JSON \
                 Schema's flagless `pattern` cannot set)"
            ),
            Self::DotWildcard => f.write_str(". wildcard"),
            Self::ClassSubtraction => f.write_str("[…-[…]] class subtraction"),
            Self::Flags { flags } => write!(
                f,
                "the {flags:?} flag(s) (JSON Schema's `pattern` is a bare ECMA-262 \
                 source string with no flag surface)"
            ),
        }
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
/// block name, pattern malformation (see [`XsdRegexError`]'s variants), or an
/// over-limit source/translated size ([`XsdRegexError::TooLarge`]) or in-class
/// name-escape count ([`XsdRegexError::TooManyFoldedClassEscapes`]) that caused
/// translation or the underlying `regex` compile to fail.
pub fn compile(pattern: &str, flags: &str) -> Result<CompiledPattern, XsdRegexError> {
    if pattern.len() > MAX_SOURCE_BYTES {
        return Err(XsdRegexError::TooLarge {
            bytes: pattern.len(),
            limit: MAX_SOURCE_BYTES,
        });
    }

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
        if escaped.len() > MAX_TRANSLATED_BYTES {
            return Err(XsdRegexError::TooLarge {
                bytes: escaped.len(),
                limit: MAX_TRANSLATED_BYTES,
            });
        }
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
    let translated = emit::translate(&source, dot_all, case_insensitive)?;
    if translated.len() > MAX_TRANSLATED_BYTES {
        return Err(XsdRegexError::TooLarge {
            bytes: translated.len(),
            limit: MAX_TRANSLATED_BYTES,
        });
    }
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

/// Everything in `(pattern, flags)` whose **meaning differs between the
/// XSD/XPath `regExp` dialect and bare ECMA-262** — the dialect JSON Schema's
/// `pattern` keyword, JavaScript, and most other `pattern` consumers are
/// specified in.
///
/// An emitter that copies a `sh:pattern`'s source text into an ECMA-262 slot
/// is performing a dialect change, and this is how it finds out whether that
/// change altered the accepted language. Returns an empty vector when the
/// pattern and its flags mean the same thing in both dialects (the common
/// case — `^[A-Z]+$` and friends).
///
/// The question is **meaning**, which is deliberately a different question
/// from whether [`compile`] had to rewrite the source. Neither direction
/// implies the other:
///
/// * `\d`/`\D` is reported even though [`compile`] passes it through
///   verbatim, because XSD's `\d` is `\p{Nd}` (every Unicode decimal digit)
///   while ECMA-262's is `[0-9]`.
/// * `\p{L}` is reported even though [`compile`] passes that general-category
///   escape through verbatim, because ECMA-262 reads `\p` as an IdentityEscape
///   (the literal characters `p{L}`) unless the `u` flag is set — and JSON
///   Schema's bare `pattern` string has no flag surface with which to set it.
///
/// Divergences are returned as [`Ecma262Divergence`] values so a consumer can
/// branch on the kind instead of string-matching English prose; the
/// [`Display`](std::fmt::Display) rendering of each one is the exact note text
/// this reporter has always produced. They are in first-appearance order with
/// duplicates collapsed.
///
/// Two kinds are reported:
///
/// * **Constructs**, class-aware — a `.` or a `-[` inside a character class is
///   a literal, not a metacharacter, and is not reported.
/// * **Flags**, because ECMA-262 regular-expression *literals* have flags but
///   JSON Schema's `pattern` is a bare string with **no flag surface at all**,
///   so every flag is lost — and `x` and `q` have no ECMA-262 spelling even
///   where flags can be expressed.
///
/// This is not a validity check. Every input it reports on is a perfectly
/// well-formed `sh:pattern`; the question is only whether its meaning
/// survives the copy.
#[must_use]
pub fn ecma_262_divergences(pattern: &str, flags: &str) -> Vec<Ecma262Divergence> {
    let mut out = ecma::ecma_262_divergences(pattern);
    if !flags.is_empty() {
        out.push(Ecma262Divergence::Flags {
            flags: flags.to_owned(),
        });
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
        assert!(re.as_regex().is_match("ab"));
        assert!(!re.as_regex().is_match("xab"));
        assert!(!re.as_regex().is_match("abx"));
    }

    #[test]
    fn unanchored_match_is_partial() {
        let re = compile("bc", "").expect("compile");
        assert!(re.as_regex().is_match("abcd"));
        assert!(!re.as_regex().is_match("abd"));
    }

    #[test]
    fn dollar_with_m_flag_is_line_end() {
        let re = compile("a$", "m").expect("compile");
        assert!(re.as_regex().is_match("a\nb"));
        assert!(re.as_regex().is_match("xa"));
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
        let positions: Vec<usize> = re.as_regex().find_iter("a\n").map(|m| m.start()).collect();
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
        assert!(re.as_regex().is_match("xa.cx"));
        assert!(!re.as_regex().is_match("abc"));
        // The `q` bit lives inside `replace_all`: under `q` the replacement is
        // used verbatim, so `$1` is two literal characters (F&O §5.6.2).
        assert_eq!(
            re.replace_all("xa.cx", "$1").expect("q replacement"),
            "x$1x"
        );
    }

    #[test]
    fn q_flag_composes_with_i() {
        let re = compile("ABC", "qi").expect("compile");
        assert!(re.as_regex().is_match("abc"));
    }

    #[test]
    fn q_flag_ignores_m_and_s_and_x() {
        // `s` would normally make `.` match anything; under `q` the pattern
        // is entirely literal, so it does not matter -- but this also
        // confirms `s`/`m` do not somehow cause an error or a non-literal
        // recompile.
        let re = compile("a.b", "qs").expect("compile");
        assert!(re.as_regex().is_match("xa.bx"));
        assert!(!re.as_regex().is_match("axb"));
        assert!(!re.as_regex().is_match("a\nb"));
    }

    /// The exact bug this module must not repeat (see `xflag.rs`'s
    /// `xpath_x_flag_matches_the_specifications_examples` for the `x`-only
    /// case): `regex::escape` does not escape a literal space, so applying
    /// `x`'s whitespace stripping AFTER escaping a `q`-literal pattern would
    /// silently delete the space. Under `q`, `x` must have zero effect.
    #[test]
    fn q_and_x_together_preserve_literal_spaces() {
        let re = compile("a b", "qx").expect("compile");
        assert!(re.as_regex().is_match("xa bx"));
        assert!(!re.as_regex().is_match("xabx"));
    }

    #[test]
    fn x_flag_strips_whitespace_outside_classes_only() {
        let re = compile("a b[c d]", "x").expect("compile");
        assert!(re.as_regex().is_match("ab d")); // outside-class space stripped
        assert!(!re.as_regex().is_match("a bd")); // ...so this no longer matches
    }

    #[test]
    fn s_flag_makes_dot_match_newlines() {
        let re = compile("^a.b$", "s").expect("compile");
        assert!(re.as_regex().is_match("a\nb"));
        assert!(re.as_regex().is_match("a\rb"));
    }

    #[test]
    fn i_flag_is_case_insensitive() {
        let re = compile("^abc$", "i").expect("compile");
        assert!(re.as_regex().is_match("ABC"));
    }

    #[test]
    fn dot_excludes_lf_and_cr_by_default() {
        let re = compile("^a.b$", "").expect("compile");
        assert!(re.as_regex().is_match("axb"));
        assert!(!re.as_regex().is_match("a\nb"));
        assert!(!re.as_regex().is_match("a\rb"));
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

    /// The generated block table's size is pinned from this NON-generated
    /// file. `blocks.rs` is `@generated`, and its own structural tests only
    /// check the rows that survived a regeneration, so a table truncated
    /// wholesale would pass them; the exact row count catches that. The
    /// compile-level cases pin the inclusive `[lo, hi]` endpoints of the last
    /// (highest-codepoint) block and of one astral block, which the spliced
    /// class ranges must preserve at both edges.
    #[test]
    fn unicode_block_table_size_and_astral_boundaries_are_pinned() {
        assert_eq!(
            blocks::UNICODE_BLOCKS.len(),
            338,
            "a truncated regeneration of the block table must fail here"
        );
        assert_eq!(
            blocks::UNICODE_BLOCKS
                .last()
                .map(|(name, lo, hi)| (*name, *lo, *hi)),
            Some(("IsSupplementaryPrivateUseArea-B", 0x0010_0000, 0x0010_FFFF)),
        );

        let last = compile(r"^\p{IsSupplementaryPrivateUseArea-B}$", "").expect("last block");
        assert!(
            last.as_regex().is_match("\u{100000}"),
            "lo endpoint is inclusive"
        );
        assert!(
            last.as_regex().is_match("\u{10FFFF}"),
            "hi endpoint is inclusive"
        );
        assert!(
            !last.as_regex().is_match("\u{FFFFF}"),
            "one below lo is outside the block"
        );

        let astral = compile(r"^\p{IsTags}$", "").expect("astral block");
        assert!(
            astral.as_regex().is_match("\u{E0000}"),
            "lo endpoint is inclusive"
        );
        assert!(
            astral.as_regex().is_match("\u{E007F}"),
            "hi endpoint is inclusive"
        );
        assert!(
            !astral.as_regex().is_match("\u{E0080}"),
            "one above hi is outside the block"
        );
    }

    #[test]
    fn compiled_pattern_exposes_the_regex_through_one_accessor() {
        let compiled = compile("^ab$", "").expect("compile");
        // `.is_match` goes through the explicit `as_regex()` accessor: the
        // type no longer `Deref`s, so a `regex`-crate method cannot be reached
        // by accident with different replacement semantics.
        assert!(compiled.as_regex().is_match("ab"));
        assert_eq!(compiled.as_regex().as_str(), "^ab$");
    }

    // ── fn:replace replacement syntax (XPath F&O 3.1 §5.6.2) ─────────────────

    /// XPath writes a literal `$` as `\$` and a literal `\` as `\\`; the
    /// `regex` crate treats a backslash as an ordinary character, so copying
    /// its replacement syntax through produced the two characters `\$`/`\\`
    /// instead of the one meant.
    #[test]
    fn replace_resolves_xpath_dollar_and_backslash_escapes() {
        let re = compile("b", "").expect("compile");
        assert_eq!(re.replace_all("abc", r"\$").expect("valid"), "a$c");
        assert_eq!(re.replace_all("abc", r"\\").expect("valid"), r"a\c");
        // The two compose with surrounding literal text and group references.
        let groups = compile("(a)(b)", "").expect("compile");
        assert_eq!(groups.replace_all("ab", r"$2\$1").expect("valid"), r"b$1");
    }

    /// `$N` keeps the `regex` crate's in-range behaviour (`$1` still expands)
    /// but gains F&O's out-of-range rule: `$0` is the whole match, `S < N <= 9`
    /// is empty, and `N > S` with `N > 9` peels the last digit off as literal
    /// text.
    #[test]
    fn replace_group_references_follow_fn_replace() {
        let re = compile("(a)(b)(c)", "").expect("compile");
        assert_eq!(re.replace_all("abc", "$3$2$1").expect("valid"), "cba");
        assert_eq!(re.replace_all("abc", "$0").expect("valid"), "abc");
        // S = 3, so `$5` is in (S, 9] and substitutes the empty string.
        assert_eq!(re.replace_all("abc", "x$5y").expect("valid"), "xy");
        // The spec's worked example, here with S = 3: `$23` peels "3" off,
        // leaving `$2`, so the result is group 2 followed by a literal "3".
        assert_eq!(re.replace_all("abc", "$23").expect("valid"), "b3");
    }

    /// A malformed replacement is F&O §5.6.2 [err:FORX0004], not a literal
    /// substitution — a `$` with no digit and a `\` that is neither escape.
    #[test]
    fn replace_malformed_replacement_is_forx0004() {
        let re = compile("b", "").expect("compile");
        assert!(matches!(
            re.replace_all("abc", "$x"),
            Err(ReplacementError::DollarWithoutGroup { .. })
        ));
        assert!(matches!(
            re.replace_all("abc", "$"),
            Err(ReplacementError::DollarWithoutGroup { .. })
        ));
        assert!(matches!(
            re.replace_all("abc", r"\n"),
            Err(ReplacementError::UnescapedBackslash { .. })
        ));
    }

    /// Under `q` the replacement is used as is (F&O §5.6.2), so the XPath
    /// escapes are NOT interpreted and [err:FORX0004] does not apply.
    #[test]
    fn replace_under_q_is_verbatim_and_never_errors() {
        let re = compile("a.c", "q").expect("compile");
        assert_eq!(re.replace_all("xa.cx", r"\$1").expect("q"), r"x\$1x");
        assert_eq!(re.replace_all("abc", "$x").expect("q"), "abc");
    }

    /// The source bound is a refusal at the boundary, naming both numbers so
    /// an operator can see exactly how far over the input was.
    #[test]
    fn source_over_the_limit_is_refused_with_both_numbers() {
        let oversized = "a".repeat(MAX_SOURCE_BYTES + 1);
        let err = compile(&oversized, "").unwrap_err();
        assert_eq!(
            err,
            XsdRegexError::TooLarge {
                bytes: MAX_SOURCE_BYTES + 1,
                limit: MAX_SOURCE_BYTES,
            }
        );
        let message = err.to_string();
        assert!(
            message.contains(&(MAX_SOURCE_BYTES + 1).to_string())
                && message.contains(&MAX_SOURCE_BYTES.to_string()),
            "the message must name both numbers: {message}"
        );
    }

    /// A source exactly at the limit is still accepted; the bound is `>`, not
    /// `>=`, so the documented limit is usable rather than off-by-one.
    #[test]
    fn source_at_the_limit_compiles() {
        let at_limit = "a".repeat(MAX_SOURCE_BYTES);
        compile(&at_limit, "").expect("a source exactly at MAX_SOURCE_BYTES");
    }

    /// A pattern that translates just under the translated bound compiles.
    /// `\c` (2 bytes) expands under `i` to a ~309-byte scoped, pre-folded
    /// class, so 3000 of them stays below the 1 MiB bound.
    #[test]
    fn translated_length_just_under_the_limit_compiles() {
        let pattern = r"\c".repeat(3000);
        compile(&pattern, "i").expect("translated length just under MAX_TRANSLATED_BYTES");
    }

    /// The pathological amplification this module must stop: enough `\c`
    /// escapes that the translated source exceeds the bound. It must be a
    /// named `TooLarge` returned promptly — the pre-folded emission means the
    /// translation is a linear string build, never a per-occurrence
    /// `regex-syntax` fold walk, so the rejection cannot consume seconds of CPU
    /// or gigabytes of RSS.
    #[test]
    fn translated_over_the_limit_is_refused_promptly() {
        let pattern = r"\c".repeat(4000);
        let started = std::time::Instant::now();
        let err = compile(&pattern, "i").unwrap_err();
        let elapsed = started.elapsed();
        let XsdRegexError::TooLarge { bytes, limit } = err else {
            panic!("expected TooLarge, got {err:?}");
        };
        assert!(bytes > limit, "{bytes} must exceed {limit}");
        assert_eq!(limit, MAX_TRANSLATED_BYTES);
        assert!(
            elapsed < std::time::Duration::from_secs(5),
            "rejection took {elapsed:?}, which is not prompt"
        );
        let message = err.to_string();
        assert!(
            message.contains(&bytes.to_string()) && message.contains(&limit.to_string()),
            "the message must name both numbers: {message}"
        );
    }

    /// The in-class case-folding bound: a pattern AT the limit compiles, and
    /// is the worst case the ~250 ms budget covers. `[\c]` repeated
    /// `MAX_FOLDED_CLASS_ESCAPES` times is exactly that worst case.
    #[test]
    fn folded_class_escapes_at_the_limit_compiles() {
        let pattern = r"[\c]".repeat(MAX_FOLDED_CLASS_ESCAPES);
        compile(&pattern, "i").expect("a pattern exactly at MAX_FOLDED_CLASS_ESCAPES");
    }

    /// One in-class name escape over the limit is refused with the new named
    /// variant, and the message names both numbers and says the construct is
    /// bounded because each occurrence is case-folded.
    #[test]
    fn folded_class_escapes_one_over_the_limit_is_refused_with_both_numbers() {
        let pattern = r"[\c]".repeat(MAX_FOLDED_CLASS_ESCAPES + 1);
        let err = compile(&pattern, "i").unwrap_err();
        assert_eq!(
            err,
            XsdRegexError::TooManyFoldedClassEscapes {
                count: MAX_FOLDED_CLASS_ESCAPES + 1,
                limit: MAX_FOLDED_CLASS_ESCAPES,
            }
        );
        let message = err.to_string();
        assert!(
            message.contains(&(MAX_FOLDED_CLASS_ESCAPES + 1).to_string())
                && message.contains(&MAX_FOLDED_CLASS_ESCAPES.to_string()),
            "the message must name both numbers: {message}"
        );
        assert!(
            message.contains("case-fold"),
            "the message must say WHY the construct is bounded: {message}"
        );
    }

    /// Without the `i` flag there is no case folding and therefore no
    /// amplification, so the in-class bound does not apply: a pattern far over
    /// the limit compiles unchanged.
    #[test]
    fn folded_class_escape_bound_does_not_apply_without_i() {
        let pattern = r"[\c]".repeat(MAX_FOLDED_CLASS_ESCAPES * 8);
        compile(&pattern, "").expect("no `i` flag means no folding and no bound");
    }

    /// A STANDALONE name escape under `i` is emitted pre-folded inside
    /// `(?-i:…)` and costs nothing per occurrence, so it stays unlimited.
    #[test]
    fn standalone_name_escapes_are_unlimited_under_i() {
        let pattern = r"\c".repeat(MAX_FOLDED_CLASS_ESCAPES * 8);
        compile(&pattern, "i").expect("standalone escapes are pre-folded, so unlimited");
    }

    /// The count is per-occurrence through class subtraction and a negation:
    /// `[a-z-[\c]]` and `[^\c]` each contribute exactly ONE in-class escape,
    /// so the limit-th compiles and the limit+1-th is refused. A count that
    /// missed the nested operand (subtraction) or the negated open would let
    /// one of these through one occurrence at a time.
    #[test]
    fn folded_class_escape_bound_counts_through_subtraction_and_negation() {
        for one in [r"[a-z-[\c]]", r"[^\c]"] {
            let at = one.repeat(MAX_FOLDED_CLASS_ESCAPES);
            compile(&at, "i").unwrap_or_else(|e| panic!("at limit {one:?}: {e}"));
            let over = one.repeat(MAX_FOLDED_CLASS_ESCAPES + 1);
            assert_eq!(
                compile(&over, "i").unwrap_err(),
                XsdRegexError::TooManyFoldedClassEscapes {
                    count: MAX_FOLDED_CLASS_ESCAPES + 1,
                    limit: MAX_FOLDED_CLASS_ESCAPES,
                },
                "{one:?} must count each in-class escape exactly once"
            );
        }
    }

    /// A single class may hold many name escapes (`[\i0-9]`); the count is per
    /// occurrence, not per class, so a class one entry over the limit is
    /// refused exactly like one class too many.
    #[test]
    fn folded_class_escape_bound_counts_multiple_escapes_in_one_class() {
        let at = format!("[{}]", r"\c".repeat(MAX_FOLDED_CLASS_ESCAPES));
        compile(&at, "i").expect("a single class exactly at the limit");
        let over = format!("[{}]", r"\c".repeat(MAX_FOLDED_CLASS_ESCAPES + 1));
        assert_eq!(
            compile(&over, "i").unwrap_err(),
            XsdRegexError::TooManyFoldedClassEscapes {
                count: MAX_FOLDED_CLASS_ESCAPES + 1,
                limit: MAX_FOLDED_CLASS_ESCAPES,
            }
        );
    }

    /// Every [`Ecma262Divergence`] variant must be reachable from some
    /// scanner-produced construct, and every such construct must map to the
    /// variant the corrected invariant says it does: a construct is reported
    /// when its **meaning** differs between the dialects, not when (or only
    /// when) translation rewrote it.
    ///
    /// This is the structural replacement for the deleted "the list mirrors
    /// the translator" claim. [`variant_name`] is an exhaustive
    /// match over [`Ecma262Divergence`], so adding a variant is a compile
    /// error until it appears in the table below with a scanner construct it
    /// comes from; [`super::ecma`]'s `Token` match is likewise exhaustive, so
    /// a new scanner token cannot be silently classified as no-divergence.
    #[test]
    fn every_divergence_variant_is_pinned_to_a_scanner_construct() {
        // (pattern, flags, expected) — one row per variant, plus the two
        // shapes the old "if and only if translation rewrote it" rule got
        // wrong.
        let table: &[(&str, &str, Vec<Ecma262Divergence>)] = &[
            (r"\i", "", vec![Ecma262Divergence::NameEscape]),
            (r"\s", "", vec![Ecma262Divergence::SpaceEscape]),
            (r"\w", "", vec![Ecma262Divergence::WordEscape]),
            (r"\d", "", vec![Ecma262Divergence::DigitEscape]),
            (
                r"\p{Nd}",
                "",
                vec![Ecma262Divergence::UnicodeCategory {
                    name: "Nd".to_owned(),
                }],
            ),
            (
                r"\p{IsBasicLatin}",
                "",
                vec![Ecma262Divergence::UnicodeBlock {
                    name: "IsBasicLatin".to_owned(),
                }],
            ),
            (".", "", vec![Ecma262Divergence::DotWildcard]),
            (
                r"[a-z-[aeiou]]",
                "",
                vec![Ecma262Divergence::ClassSubtraction],
            ),
            (
                "a",
                "i",
                vec![Ecma262Divergence::Flags {
                    flags: "i".to_owned(),
                }],
            ),
        ];

        for (pattern, flags, expected) in table {
            assert_eq!(
                &ecma_262_divergences(pattern, flags),
                expected,
                "the reporter must map {pattern:?} / {flags:?} to {expected:?}"
            );
            for divergence in expected {
                let _ = variant_name(divergence);
            }
        }

        // `\d` IS reported though translation leaves the text verbatim (the
        // meaning still differs), and `\p{L}`/`\P{Nd}` are reported for the
        // same reason — the unflagged `u`-flag dependency is the category's,
        // not the reporter's, to state.
        assert_eq!(
            ecma_262_divergences(r"\d", ""),
            vec![Ecma262Divergence::DigitEscape]
        );
        assert_eq!(
            ecma_262_divergences(r"\p{L}", ""),
            vec![Ecma262Divergence::UnicodeCategory {
                name: "L".to_owned(),
            }]
        );
        assert_eq!(
            ecma_262_divergences(r"\P{Nd}", ""),
            vec![Ecma262Divergence::UnicodeCategory {
                name: "Nd".to_owned(),
            }]
        );

        // Class-awareness still holds: a `.` inside a class is a literal, and
        // there is no `-[` subtraction in `.`-only text.
        assert_eq!(
            ecma_262_divergences(r"[.]", ""),
            Vec::<Ecma262Divergence>::new()
        );
    }

    /// The exhaustive variant-name match exists only so adding an
    /// [`Ecma262Divergence`] variant is a compile error until
    /// [`every_divergence_variant_is_pinned_to_a_scanner_construct`] (and
    /// therefore the scanner construct it comes from) is updated.
    fn variant_name(divergence: &Ecma262Divergence) -> &'static str {
        match divergence {
            Ecma262Divergence::NameEscape => "NameEscape",
            Ecma262Divergence::SpaceEscape => "SpaceEscape",
            Ecma262Divergence::WordEscape => "WordEscape",
            Ecma262Divergence::DigitEscape => "DigitEscape",
            Ecma262Divergence::UnicodeCategory { .. } => "UnicodeCategory",
            Ecma262Divergence::UnicodeBlock { .. } => "UnicodeBlock",
            Ecma262Divergence::DotWildcard => "DotWildcard",
            Ecma262Divergence::ClassSubtraction => "ClassSubtraction",
            Ecma262Divergence::Flags { .. } => "Flags",
        }
    }

    /// [`Ecma262Divergence`]'s [`Display`](std::fmt::Display) is the loss
    /// ledger's note text. The existing cases must not move, so the two
    /// wordings the committed golden ledger carries are pinned verbatim.
    #[test]
    fn divergence_display_is_the_original_ledger_wording() {
        assert_eq!(Ecma262Divergence::NameEscape.to_string(), "\\i/\\I/\\c/\\C");
        assert_eq!(
            Ecma262Divergence::Flags {
                flags: "i".to_owned(),
            }
            .to_string(),
            "the \"i\" flag(s) (JSON Schema's `pattern` is a bare ECMA-262 source string with \
             no flag surface)"
        );
    }
}
