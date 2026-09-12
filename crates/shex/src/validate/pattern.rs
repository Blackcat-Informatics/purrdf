// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XSD/XPath regular-expression support for the ShEx `PATTERN` facet
//! (spec §5.4.5).
//!
//! ShEx patterns follow XPath/XQuery `fn:matches` semantics: the match is
//! **partial** (no implicit anchoring — `^`/`$` are explicit metacharacters)
//! and the flag letters `s`, `m`, `i`, `x` and `q` carry their XPath
//! meanings. That is the same dialect SHACL's `sh:pattern` and SPARQL's
//! `REGEX` are specified in, so this module owns no translation of its own:
//! it delegates to [`purrdf_core::xsd_regex::compile`], the one shared
//! implementation, and all three facets carry the same accept set and the
//! same semantics (ETHOS §O).
//!
//! Four things changed when the local translator was retired, all of them
//! previously-wrong answers rather than newly-narrowed ones:
//!
//! * `\i \I \c \C` (the XML name-character escapes) are **supported**, where
//!   the local translator reported them as a facet error;
//! * `\s \S \w \W` are XSD's narrower classes, not the `regex` crate's
//!   Unicode-property ones, and `.` excludes `#x0D` as well as `#x0A`;
//! * `\p{Is…}` names a Unicode **block**, resolved against a generated
//!   table, rather than being handed to `regex-syntax`'s loose name matching
//!   (which silently resolved some `Is`-names as *scripts* instead);
//! * `x` removes exactly `#x9`, `#xA`, `#xD` and `#x20` outside character
//!   classes, rather than the `regex` crate's wider verbose mode — and it no
//!   longer composes with `q`, which per XPath F&O 3.1 §5.6.2 makes `m`, `s`
//!   and `x` inert. The old inline-`(?ismx)` wrap around a `regex::escape`d
//!   body let `x` delete literal spaces from a `q`-literal pattern.
//!
//! Backreferences remain unsupported, now by explicit documented design
//! rather than by accident: the shared compiler translates onto the `regex`
//! crate's DFA engine, which cannot backtrack.
//!
//! # Reachability
//!
//! ShExC cannot express most of the above: the lexer's escape whitelist
//! rejects `\i \c \d \s \b` in a regex literal and its flag scan accepts only
//! `s m i x`, so `q` is unreachable from ShExC source. **ShExJ takes
//! `pattern` and `flags` unfiltered**, which is why these constructs are
//! reachable at all and why the end-to-end tests for them are ShExJ-driven.

use std::sync::Arc;

use purrdf_core::FastMap;
use regex::Regex;

/// One memoized `PATTERN` compile: the shared compiled regex, or the shared
/// facet-violation message for a pattern or flag string that does not compile.
///
/// The error is an `Arc<str>` so a failed probe clones a refcount exactly as a
/// successful probe clones the regex's `Arc`.
type CachedPattern = Result<Arc<Regex>, Arc<str>>;

/// The per-flags layer of a [`PatternCache`]: compile results keyed by the
/// exact `flags` string (the empty string when no flags were given).
type FlagsCache = FastMap<String, CachedPattern>;

/// Per-validation-call cache of compiled `PATTERN` facets.
///
/// The facet is checked **per value node**, so without this an
/// `xsd:string PATTERN` over a thousand-triple neighbourhood compiled the
/// same pattern a thousand times. Keyed pattern-then-flags so a hit probes
/// with **borrowed** strings rather than allocating an owned key per value,
/// matching [`crate`]'s sibling convention in `purrdf-sparql-eval`'s
/// `EvalCtx::regex_cache`.
///
/// The tables use the workspace's fixed-key [`FastMap`] (AHASH with fixed
/// keys, no `RandomState`), per AGENTS.md §4: this is a per-value-node hot
/// path, and a randomly-seeded hasher has no business on it. The canonical
/// spelling is `purrdf-core`'s own [`FastHasher`](purrdf_core::FastHasher)
/// policy, whose aliases the crate already depends on — the same hasher the
/// sibling `EvalCtx::regex_cache` uses.
///
/// Compile **failures** are cached too. A malformed pattern is a facet
/// violation reported once per value node, and recompiling it per value to
/// re-derive the same message is the same waste as recompiling a valid one.
/// The cached error is an `Arc<str>`, so a failed probe clones a refcount
/// exactly as a successful probe clones the compiled regex's `Arc` — neither
/// probe allocates inside the cache.
#[derive(Default)]
pub(crate) struct PatternCache {
    by_pattern: FastMap<String, FlagsCache>,
}

impl PatternCache {
    /// The compiled regex for `(pattern, flags)`, compiling on first use.
    ///
    /// # Errors
    ///
    /// The facet-violation message for a pattern or flag string that does not
    /// compile — an `Arc<str>` shared with the cache, so a repeat hands out
    /// the same message byte-for-byte without copying it.
    pub(crate) fn compiled(
        &mut self,
        pattern: &str,
        flags: Option<&str>,
    ) -> Result<Arc<Regex>, Arc<str>> {
        let flags = flags.unwrap_or("");
        if let Some(cached) = self
            .by_pattern
            .get(pattern)
            .and_then(|by_flags| by_flags.get(flags))
        {
            return cached.clone();
        }
        let compiled = compile_pattern(pattern, Some(flags))
            .map(Arc::new)
            .map_err(Arc::<str>::from);
        self.by_pattern
            .entry(pattern.to_owned())
            .or_default()
            .insert(flags.to_owned(), compiled.clone());
        compiled
    }
}

/// Compile a ShEx `PATTERN` facet (XSD/XPath regex source + flags) into a
/// [`Regex`] that implements `fn:matches` partial-match semantics.
///
/// This is the uncached primitive; validation goes through
/// [`PatternCache::compiled`].
///
/// # Errors
///
/// A facet-violation message naming the pattern, its flags and the precise
/// reason it does not compile — an unsupported flag letter, a construct the
/// `fn:matches` grammar does not define (`\b`, `\B`), a backreference, an
/// unrecognized Unicode block name, or a malformed pattern.
pub(crate) fn compile_pattern(pattern: &str, flags: Option<&str>) -> Result<Regex, String> {
    let flags = flags.unwrap_or("");
    purrdf_core::xsd_regex::compile(pattern, flags)
        .map(|compiled| compiled.as_regex().clone())
        .map_err(|e| format!("invalid pattern /{pattern}/{flags}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_match_is_not_anchored() {
        let re = compile_pattern("bc", None).expect("compile");
        assert!(re.is_match("abcd"));
        assert!(!re.is_match("abd"));
    }

    #[test]
    fn explicit_anchors_work() {
        let re = compile_pattern("^ab$", None).expect("compile");
        assert!(re.is_match("ab"));
        assert!(!re.is_match("xab"));
    }

    #[test]
    fn flags_translate() {
        let re = compile_pattern("^a.c$", Some("is")).expect("compile");
        assert!(re.is_match("A\nC"));
        let re = compile_pattern("a b", Some("x")).expect("compile");
        assert!(re.is_match("xxabxx"));
    }

    #[test]
    fn q_flag_treats_pattern_as_literal() {
        let re = compile_pattern("a.c", Some("q")).expect("compile");
        assert!(re.is_match("xa.cx"));
        assert!(!re.is_match("abc"));
    }

    /// `q` makes `m`, `s` and `x` inert (XPath F&O 3.1 §5.6.2). The retired
    /// local translator wrapped a `regex::escape`d body in an inline
    /// `(?ismx)` group instead, and `regex::escape` does not escape spaces —
    /// so `x` deleted the literal spaces out of a literal pattern.
    #[test]
    fn q_flag_does_not_compose_with_x() {
        let re = compile_pattern("a b", Some("qx")).expect("compile");
        assert!(
            re.is_match("xa bx"),
            "the space is literal: `q` makes `x` inert"
        );
        assert!(!re.is_match("ab"));
    }

    #[test]
    fn class_subtraction_is_rewritten() {
        let re = compile_pattern("^[a-z-[aeiou]]+$", None).expect("compile");
        assert!(re.is_match("bcd"));
        assert!(!re.is_match("bad"));
    }

    /// Replaces `xml_name_escapes_are_reported`, which asserted the *old*
    /// behaviour: `\i`/`\c` were reported as a facet error because the local
    /// translator did not model them. They are part of the XSD grammar the
    /// facet is specified in, and the shared compiler implements them.
    #[test]
    fn xml_name_escapes_are_supported() {
        let re = compile_pattern(r"^\i\c*$", None).expect("compile");
        assert!(re.is_match("abc123"), "a valid XML name");
        assert!(re.is_match("_x"), "`_` is a NameStartChar");
        assert!(
            !re.is_match("1abc"),
            "a digit is a NameChar but not a NameStartChar"
        );
    }

    #[test]
    fn digit_class_matches_unicode() {
        let re = compile_pattern(r"^\d+$", None).expect("compile");
        assert!(re.is_match("42"));
        assert!(!re.is_match("4a"));
    }

    /// `\p{Is…}` is a Unicode BLOCK. `regex-syntax` resolved `IsGreek` as the
    /// *Script* `Greek` — which covers U+1F00 in Greek Extended — where the
    /// block is `Greek and Coptic` (U+0370–U+03FF), spelled
    /// `IsGreekandCoptic` since Unicode 4.1 renamed it.
    #[test]
    fn block_escapes_are_blocks_not_scripts() {
        let re = compile_pattern(r"^\p{IsGreekandCoptic}$", None).expect("compile");
        assert!(re.is_match("\u{0370}"));
        assert!(
            !re.is_match("\u{1F00}"),
            "U+1F00 is Greek Extended, a different block"
        );
        let latin = compile_pattern(r"^\p{IsBasicLatin}+$", None).expect("compile");
        assert!(latin.is_match("Az0"));
        assert!(!latin.is_match("é"));

        // The pre-4.1 name is refused rather than resolved as the script it
        // is not — which is exactly what the old pass-through did.
        let err = compile_pattern(r"^\p{IsGreek}$", None).expect_err("not a block name");
        assert!(err.contains("IsGreek"), "the message names it: {err}");
    }

    /// `\b`/`\B` are not in the `fn:matches` grammar at all, and
    /// backreferences are in it but cannot run on a DFA engine. Both are
    /// named facet errors rather than a silently different language.
    #[test]
    fn constructs_outside_the_grammar_are_named_errors() {
        let b = compile_pattern(r"\bcd", None).expect_err("\\b is not XSD");
        assert!(b.contains(r"\b"), "the message names the construct: {b}");
        let back = compile_pattern(r"(ab)\1", None).expect_err("no backreferences");
        assert!(
            back.contains("backreference"),
            "the message names the construct: {back}"
        );
    }

    #[test]
    fn unsupported_flag_is_a_named_error() {
        let err = compile_pattern("foo", Some("z")).expect_err("'z' is not a flag");
        assert!(err.contains('z'), "the message names the flag: {err}");
    }

    /// The cache must be a pure memo: same answer, compiled once.
    #[test]
    fn cache_returns_the_same_regex_and_memoizes_failures() {
        let mut cache = PatternCache::default();
        let first = cache.compiled("^ab$", None).expect("compile");
        let second = cache.compiled("^ab$", None).expect("cached");
        assert!(
            Arc::ptr_eq(&first, &second),
            "a hit hands out the same compiled regex, not a fresh one"
        );
        assert!(first.is_match("ab"));

        // Flags are part of the key, not folded into the pattern.
        let cased = cache.compiled("^ab$", Some("i")).expect("compile");
        assert!(cased.is_match("AB"));
        assert!(!first.is_match("AB"));

        let bad_once = cache.compiled("foo", Some("z")).expect_err("bad flag");
        let bad_twice = cache.compiled("foo", Some("z")).expect_err("cached");
        assert_eq!(
            bad_once, bad_twice,
            "a memoized failure reports byte-identical reasons"
        );
    }
}
