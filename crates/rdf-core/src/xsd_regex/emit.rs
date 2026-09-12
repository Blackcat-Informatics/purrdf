// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The construct-by-construct XSD/XPath `regExp` -> `regex`-crate translator
//! (see [`super`]'s module doc for the full construct table this implements).
//!
//! [`translate`] is a single fold over [`Scanner`]'s token stream. It owns no
//! cursor and no class bookkeeping of its own: every character index, every
//! lookahead, the character-class nesting (which nests through XSD's
//! `charClassSub` subtraction, e.g. `[a-z-[aeiou]]`), and the capturing-group
//! count that decides where a back-reference ends are tokenized once in
//! [`super::scan`] and read from the yielded [`Token`]s here. The fold is a
//! flat `match` that appends to one output `String`.
//!
//! Every multi-character-escape rewrite below (`\i \I \c \C \s \S \w \W`, and
//! block escapes `\p{Is…}`/`\P{Is…}`) is emitted as a **self-contained bracket
//! expression** (`[...]`/`[^...]`) rather than a bare, un-bracketed splice:
//! `regex-syntax` supports nested character-class union (`[a[b-c]d]` is valid
//! and means the same as `[abcd]` restricted to `[b-c]`'s member codepoints),
//! so a self-contained bracket expression composes correctly by ordinary set
//! union whether it stands alone as its own atom or is nested inside an
//! already-open class -- one rewrite rule for both contexts, rather than two
//! (verified directly against `regex` 1.13: `Regex::new("[a[b-c]d]")`,
//! `Regex::new("[[^\\u{370}-\\u{3ff}]a]")` and similar nested-negation forms
//! all compile and match as expected).

use super::MAX_FOLDED_CLASS_ESCAPES;
use super::blocks;
use super::classes::{
    NOT_SPACE_CLASS, NOT_WORD_CLASS, SPACE_CLASS, WORD_CLASS, folded_name_char_class_body,
    folded_name_start_class_body, is_surrogate_range, name_char_class_body, name_start_class_body,
    push_hex_range,
};
use super::error::XsdRegexError;
use super::scan::{Scanner, Token};

/// Translate an XSD/XPath `regExp` pattern body into `regex`-crate syntax.
///
/// `dot_all` is the caller's `s` flag: when set, an unescaped `.` outside a
/// character class is left untouched (the caller applies
/// `RegexBuilder::dot_matches_new_line(true)` instead); when clear, `.` is
/// rewritten per XPath F&O 3.1 §5.6.2 (excludes both `#xA` and `#xD`, not
/// only `#xA` as Rust's own default does).
///
/// `case_insensitive` is the caller's `i` flag. The caller still sets
/// `RegexBuilder::case_insensitive(true)`; the flag is threaded here only so
/// the `\i`/`\I`/`\c`/`\C` escapes can be emitted PRE-FOLDED inside a
/// `(?-i:…)` scope. That scope is what stops `regex-syntax` from re-walking
/// the ~917k-codepoint astral name range once per occurrence at
/// Hir-translation time (see [`folded_name_start_class_body`]); the matched
/// language is unchanged because the emitted set is the exact simple-case-fold
/// closure `regex-syntax` would have produced. A name escape that appears
/// INSIDE a character class cannot carry that scope (a group is not a class
/// member), so in-class occurrences are counted and the pass refuses past
/// [`super::MAX_FOLDED_CLASS_ESCAPES`] under `i` instead.
///
/// Must be called AFTER [`super::xflag::strip_x_flag_whitespace`] when the
/// `x` flag is set — the two operate on the same raw pattern text, and `x`'s
/// removal has to happen first (its whitespace-exemption tracking is
/// textual, not aware of any of the rewrites below).
pub(super) fn translate(
    pattern: &str,
    dot_all: bool,
    case_insensitive: bool,
) -> Result<String, XsdRegexError> {
    let mut out = String::with_capacity(pattern.len() + 16);
    let mut scanner = Scanner::new(pattern);
    // XSD's `\i`/`\c` (and their negations) are the only constructs whose `i`
    // rewrite differs by context. A standalone atom can be emitted pre-folded
    // inside a `(?-i:…)` scope; a character-class member cannot (a group is
    // not a class member), so an in-class escape splices as a nested class and
    // the enclosing `i` re-folds the closed set. That re-fold is semantically
    // idempotent but costs a full ~917k-codepoint walk per occurrence, so
    // `folded_class_escapes` counts them and the pass refuses past
    // [`super::MAX_FOLDED_CLASS_ESCAPES`] under `i`. Tracking the depth here —
    // rather than in the scanner — keeps the rewrite to one comparison.
    let mut class_depth: usize = 0;
    // In-class name escapes seen while `i` is in force; the count the bound is
    // applied to at the end of the pass. Counting here keeps the module's
    // single-cursor rule: no second walk over the pattern is introduced.
    let mut folded_class_escapes: usize = 0;
    // Every `\`-escaped construct is handled by the `Escape`/`NameEscape`/
    // `SpaceEscape`/`WordEscape`/`UnicodeProperty`/`Backreference` arms below
    // in ANY context (in or out of a character class): `\b`/`\B` and
    // backreferences are rejected by [`Scanner`] in both contexts per the
    // XSD/XPath grammar (neither is defined by it at all), and every other
    // rewrite composes correctly via nested bracket-class union (see the
    // module doc) regardless of context, so there is no class-depth branch
    // here.
    while let Some(token) = scanner.next() {
        match token? {
            Token::Literal(c) => out.push(c),
            // `&` and `~` are ordinary members of an XSD character class
            // (`XmlCharIncDash ::= [^\#x5B#x5D]`), but `regex-syntax` reads an
            // unescaped `&&` as set INTERSECTION and `~~` as SYMMETRIC
            // DIFFERENCE. Escaping each one preserves the XSD language
            // exactly: `[a&&b]` stays the three members a, &, b.
            Token::ClassMember(c) => {
                out.push('\\');
                out.push(c);
            }
            Token::Escape(c) => {
                // `\d`/`\D` already default to `\p{Nd}`/its complement in
                // `regex-syntax`, exactly XSD's definition — confirmed by
                // direct testing (`Regex::new(r"^\d$").unwrap().is_match("\u{0660}")`,
                // U+0660 ARABIC-INDIC DIGIT ZERO, is `true`). No rewrite is
                // needed; a `\p`/`\P` not followed by `{` is passed through
                // here unchanged for the same reason.
                out.push('\\');
                out.push(c);
            }
            Token::NameEscape { negated, chars } => {
                // Count BEFORE emitting so the bound is enforced for exactly
                // the construct that carries the re-fold cost.
                if case_insensitive && class_depth > 0 {
                    folded_class_escapes += 1;
                }
                // Under `i` the canonical, pre-folded set is a process-wide
                // constant; without it the raw body is (re)built per
                // occurrence, exactly as before this change.
                let raw_body;
                let body: &str = if case_insensitive {
                    if chars {
                        folded_name_char_class_body()
                    } else {
                        folded_name_start_class_body()
                    }
                } else {
                    raw_body = if chars {
                        name_char_class_body()
                    } else {
                        name_start_class_body()
                    };
                    &raw_body
                };
                // Standalone (depth 0): scope `i` off so the pre-folded set is
                // not walked again by `regex-syntax`; the set is already the
                // exact closure, so the language is identical to today's.
                // Inside a class: splices as a nested class, where the outer
                // `i` re-folds the closed set to itself; those occurrences are
                // bounded above by `MAX_FOLDED_CLASS_ESCAPES`.
                if case_insensitive && class_depth == 0 {
                    out.push_str("(?-i:[");
                    if negated {
                        out.push('^');
                    }
                    out.push_str(body);
                    out.push_str("])");
                } else {
                    out.push('[');
                    if negated {
                        out.push('^');
                    }
                    out.push_str(body);
                    out.push(']');
                }
            }
            Token::SpaceEscape { negated } => {
                out.push_str(if negated {
                    NOT_SPACE_CLASS
                } else {
                    SPACE_CLASS
                });
            }
            Token::WordEscape { negated } => {
                out.push_str(if negated { NOT_WORD_CLASS } else { WORD_CLASS });
            }
            Token::UnicodeProperty { negated, name } => {
                emit_unicode_property(&mut out, negated, &name)?;
            }
            Token::Dot => {
                out.push_str(if dot_all { "." } else { "[^\\u{a}\\u{d}]" });
            }
            Token::ClassOpen { negated } => {
                class_depth += 1;
                if negated {
                    // XSD `[^g-e]` is (complement of `g`) minus `e`, but
                    // Rust's `[^g--e]` applies `^` to the WHOLE class
                    // expression — i.e. the complement of (`g` minus `e`),
                    // a different set. Opening an extra bracket and putting
                    // the negation on the inner one restores XSD's binding:
                    // `[[^g]--e]`. Emitted for every negated group, not only
                    // the ones a subtraction follows, because the wrap is
                    // semantically free when it does not (`[[^g]]` == `[^g]`)
                    // and deciding otherwise would need a lookahead over the
                    // whole group. [`Scanner`] tracks which level is negated;
                    // [`Scanner::closes_negated_wrap`] tells the `-[` and `]`
                    // arms below when that inner group closes.
                    out.push_str("[[^");
                } else {
                    out.push('[');
                }
            }
            Token::Subtract => {
                // XPath class subtraction `[...-[...]]` -> regex's own `--`
                // difference operator. Close the inner negated group before
                // the difference operator, so `--` subtracts from the
                // complement rather than around it.
                if scanner.closes_negated_wrap() {
                    out.push(']');
                }
                out.push_str("--");
            }
            Token::ClassClose => {
                class_depth -= 1;
                // The inner negated group's own `]` was already emitted by
                // the `Subtract` arm above; only an unsubtracted negated
                // group still owes one here.
                if scanner.closes_negated_wrap() {
                    out.push(']');
                }
                out.push(']');
            }
            // XML Schema Part 2 Appendix G (as amended by XPath F&O 3.1
            // §5.6.1.3) decides in [`Scanner`] whether a `(` is capturing; the
            // emitted `regex` spelling is the only thing left here.
            Token::GroupOpen { capturing } => {
                out.push_str(if capturing { "(" } else { "(?:" });
            }
            Token::GroupClose => out.push(')'),
            Token::Backreference(n) => {
                // [`Scanner`] has already validated that the reference names a
                // capturing group whose `)` precedes it; this implementation
                // declines to execute every back-reference (see the module
                // doc), so it is reported with the exact spelling that was
                // tokenized.
                return Err(XsdRegexError::Backreference(format!("\\{n}")));
            }
        }
    }
    if case_insensitive && folded_class_escapes > MAX_FOLDED_CLASS_ESCAPES {
        return Err(XsdRegexError::TooManyFoldedClassEscapes {
            count: folded_class_escapes,
            limit: MAX_FOLDED_CLASS_ESCAPES,
        });
    }
    Ok(out)
}

/// Append the translation of one `\p{…}`/`\P{…}` token; `negated` is the `P`
/// spelling. The `Is`-prefixed block form is resolved against the generated
/// `super::blocks` table, never left to `regex-syntax`'s own (silently
/// different) resolution; a general-category escape is shared syntax between
/// the two dialects and is passed through.
fn emit_unicode_property(out: &mut String, negated: bool, name: &str) -> Result<(), XsdRegexError> {
    if name.starts_with("Is") {
        let (lo, hi) =
            blocks::lookup(name).ok_or_else(|| XsdRegexError::UnknownBlock(name.to_owned()))?;
        if is_surrogate_range(lo, hi) {
            // Three of Unicode's blocks — High Surrogates, High Private Use
            // Surrogates and Low Surrogates — consist entirely of code points
            // that are NOT Unicode scalar values. A Rust `&str` is UTF-8 over
            // scalar values, so no subject string this engine can be handed
            // will ever contain one.
            //
            // "Matches nothing" is therefore the EXACT answer over the domain,
            // not a degraded approximation of one, and `\P{Is…}` of such a
            // block matches every representable character for the same reason.
            // Emitting the range literally is what does not work: `regex`
            // refuses `\u{d800}` outright, so the caller got an opaque
            // "hexadecimal literal is not a Unicode scalar value" parse error
            // naming neither the block nor the reason.
            //
            // Refusing the pattern instead would be over-refusal: the escape
            // is well-formed XSD naming a block that genuinely exists, and
            // over-refusal is the direction this dialect work is least willing
            // to move in. Both forms are self-contained bracket expressions, so
            // they compose by union inside an enclosing class exactly as every
            // other block escape does.
            out.push_str(if negated { "[\\s\\S]" } else { "[^\\s\\S]" });
        } else {
            out.push('[');
            if negated {
                out.push('^');
            }
            push_hex_range(out, lo, hi);
            out.push(']');
        }
    } else {
        // A general-category escape (`\p{L}`, `\p{Nd}`, ...) is legitimately
        // shared syntax between XSD and `regex-syntax` — never intercepted.
        out.push('\\');
        out.push(if negated { 'P' } else { 'p' });
        out.push('{');
        out.push_str(name);
        out.push('}');
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translated_regex(pattern: &str, dot_all: bool) -> regex::Regex {
        let source = translate(pattern, dot_all, false).expect("translate");
        regex::Regex::new(&source).unwrap_or_else(|e| panic!("compile {source:?}: {e}"))
    }

    #[test]
    fn class_subtraction_is_rewritten() {
        let re = translated_regex("^[a-z-[aeiou]]+$", false);
        assert!(re.is_match("bcd"));
        assert!(!re.is_match("bad"));
    }

    /// XSD's `charClassSub` takes a `negCharGroup` on its left as readily as
    /// a `posCharGroup`, and the two dialects bind the negation on opposite
    /// sides of the difference: `[^0-9-[a-z]]` is XSD's (¬digits) ∖ (a–z),
    /// while Rust's `[^0-9--[a-z]]` is ¬(digits ∖ a–z) — which is just
    /// ¬digits, and therefore matches every letter. Asserted on the emitted
    /// source as well as the behaviour, because the bug was one bracket.
    #[test]
    fn negated_class_subtraction_binds_to_the_negation() {
        assert_eq!(
            translate("[^0-9-[a-z]]", false, false).expect("translate"),
            "[[^0-9]--[a-z]]"
        );
        let re = translated_regex("^[^0-9-[a-z]]+$", false);
        assert!(re.is_match("ABC"), "upper case is in neither subtrahend");
        assert!(!re.is_match("a"), "a-z is subtracted from the complement");
        assert!(!re.is_match("5"), "digits are excluded by the negation");

        // The wrap is emitted for every negated group, subtraction or not,
        // and `[[^x]]` means exactly what `[^x]` means.
        assert_eq!(
            translate("[^abc]", false, false).expect("translate"),
            "[[^abc]]"
        );
        let plain = translated_regex("^[^abc]$", false);
        assert!(plain.is_match("d"));
        assert!(!plain.is_match("a"));

        // A negated subtrahend nests the same way.
        let nested = translated_regex("^[a-z-[^aeiou]]+$", false);
        assert!(nested.is_match("aei"), "a-z intersected with the vowels");
        assert!(!nested.is_match("b"));
    }

    /// A back-reference's digits are tokenized against the capturing-group
    /// count that precedes it (F&O §5.6.1.4), not by a greedy digit run. The
    /// whole contract for a back-reference here is an error that names the
    /// construct, so a wrong tokenization is a message that lies about the
    /// pattern.
    #[test]
    fn backreference_tokenization_follows_the_preceding_group_count() {
        let named = |pattern: &str| match translate(pattern, false, false) {
            Err(XsdRegexError::Backreference(reference)) => reference,
            other => panic!("expected a back-reference for {pattern:?}, got {other:?}"),
        };
        // One group: `\12` is `\1` followed by a literal `2`.
        assert_eq!(named(r"(a)\12"), r"\1");
        // Ten groups: the second digit joins.
        assert_eq!(named(r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)\10"), r"\10");
        // Twelve groups: so does a `\12`.
        assert_eq!(named(r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)\12"), r"\12");
        // A non-capturing group is not counted...
        assert_eq!(named(r"(?:a)(b)\1"), r"\1");
        // ...nor is a parenthesis inside a character class, or an escaped one.
        assert_eq!(named(r"[()](a)\1"), r"\1");
        assert_eq!(named(r"\((a)\1"), r"\1");

        // A reference to a group that does not exist, or whose `)` has not
        // been seen, is MALFORMED — no engine can run it — which is a
        // different answer from one this engine declines to execute.
        for pattern in [r"a\9b", r"(a\1)", r"(?:a)\1"] {
            match translate(pattern, false, false) {
                Err(XsdRegexError::Malformed(message)) => {
                    assert!(
                        message.contains("does not exist") || message.contains("closing"),
                        "{pattern:?}: {message}"
                    );
                }
                other => panic!("expected malformed for {pattern:?}, got {other:?}"),
            }
        }
    }

    /// `\0` is neither a back-reference (XPath F&O §5.6.1.4's production
    /// starts at `\1`) nor a `SingleCharEsc` (XML Schema Appendix G
    /// enumerates those). Letting it reach the engine produced
    /// "backreferences are not supported", which is wrong about what the
    /// pattern contains.
    #[test]
    fn nul_escape_is_refused_as_itself_not_as_a_backreference() {
        let err = translate("^a\\0b$", false, false).expect_err("\\0 is not a construct");
        let message = err.to_string();
        assert!(
            message.contains("\\0"),
            "the message must name the construct: {message}"
        );
        assert!(
            !message.contains("backreference"),
            "\\0 is not a back-reference, and the message must not say it is: {message}"
        );
        // A real back-reference still reports as one.
        let back = translate("(a)\\1", false, false).expect_err("no backreferences");
        assert!(back.to_string().contains("backreference"));
    }

    /// `&` and `~` are ordinary XSD class members, so the emitter escapes
    /// them away from `regex-syntax`'s set operators. Without this,
    /// `[^&&<>]` -- the "no ampersand or angle brackets" anti-injection
    /// shape -- emitted `[[^&&<>]]`, whose inner `&&` was an INTERSECTION,
    /// and matched `<script>&` at exit zero with no diagnostic.
    #[test]
    fn class_members_are_escaped_away_from_regex_set_operators() {
        assert_eq!(
            translate("[a&&b]", false, false).expect("translate"),
            r"[a\&\&b]"
        );
        assert_eq!(
            translate("[a~~b]", false, false).expect("translate"),
            r"[a\~\~b]"
        );

        let members = translated_regex("^[a&&b]$", false);
        for subject in ["a", "&", "b"] {
            assert!(members.is_match(subject), "[a&&b] must match {subject:?}");
        }
        assert!(!members.is_match("c"), "[a&&b] must not match c");
        // Intersection would have matched only "" (empty intersection of a
        // and b), so this is the regression the escape closes.
        assert!(!members.is_match("ab"));

        let anti = translated_regex("^[^&&<>]*$", false);
        assert!(
            !anti.is_match("<script>&"),
            "[^&&<>] is 'no ampersand or angle brackets', not match-everything"
        );
        assert!(anti.is_match("safe"));
    }

    #[test]
    fn nested_class_subtraction_is_rewritten() {
        // A subtraction inside a subtraction's own right-hand side.
        let re = translated_regex("^[a-z-[a-c-[b]]]+$", false);
        // Right-hand side `[a-c-[b]]` = {a, c}; overall = a-z minus {a, c}.
        assert!(re.is_match("d"));
        assert!(!re.is_match("a"));
        assert!(!re.is_match("c"));
    }

    #[test]
    fn dot_excludes_lf_and_cr_outside_class() {
        let re = translated_regex("^a.b$", false);
        assert!(re.is_match("axb"));
        assert!(!re.is_match("a\nb"));
        assert!(!re.is_match("a\rb"));
    }

    #[test]
    fn dot_is_literal_inside_class() {
        let re = translated_regex("^[.]$", false);
        assert!(re.is_match("."));
        assert!(!re.is_match("a"));
    }

    #[test]
    fn dot_all_flag_leaves_dot_untouched_for_builder() {
        // `dot_all = true` means the translator must NOT rewrite `.`; the
        // caller applies `dot_matches_new_line` at the builder instead.
        let source = translate("^a.b$", true, false).expect("translate");
        assert_eq!(source, "^a.b$");
    }

    #[test]
    fn d_escape_is_unchanged_and_matches_non_ascii_decimal_digits() {
        let re = translated_regex(r"^\d+$", false);
        assert!(re.is_match("42"));
        assert!(!re.is_match("4a"));
        assert!(re.is_match("\u{0660}")); // ARABIC-INDIC DIGIT ZERO
        let re_neg = translated_regex(r"^\D$", false);
        assert!(re_neg.is_match("a"));
        assert!(!re_neg.is_match("4"));
    }

    #[test]
    fn general_category_escape_passes_through_unchanged() {
        let re = translated_regex(r"^\p{L}+$", false);
        assert!(re.is_match("abc"));
        assert!(!re.is_match("123"));
    }

    #[test]
    fn is_block_escape_is_spliced_from_the_table() {
        let re = translated_regex(r"^\p{IsBasicLatin}$", false);
        assert!(re.is_match("A"));
        assert!(!re.is_match("\u{0100}")); // Latin Extended-A: outside the block
        let re_neg = translated_regex(r"^\P{IsBasicLatin}$", false);
        assert!(re_neg.is_match("\u{0100}"));
        assert!(!re_neg.is_match("A"));
    }

    #[test]
    fn unknown_block_name_is_rejected() {
        let err = translate(r"\p{IsNotARealBlock}", false, false).unwrap_err();
        assert_eq!(
            err,
            XsdRegexError::UnknownBlock("IsNotARealBlock".to_owned())
        );
    }

    #[test]
    fn script_vs_block_confusion_is_closed() {
        // `\p{IsGreek}` is NOT a recognized block name (the real UCD block
        // is "Greek and Coptic", normalizing to `IsGreekandCoptic` per XML
        // Schema Part 2 §G.4.2.3) -- Rust's own un-intercepted resolution
        // silently accepts "IsGreek" anyway by falling back to the *Script*
        // `Greek` (confirmed live: `Regex::new(r"\p{IsGreek}")` compiles and
        // matches U+1F00 GREEK SMALL LETTER ALPHA WITH PSILI, which is in
        // the Greek Extended block, not Greek and Coptic). This translator
        // must reject that spelling outright rather than let it fall
        // through to Rust's resolution.
        let err = translate(r"\p{IsGreek}", false, false).unwrap_err();
        assert_eq!(err, XsdRegexError::UnknownBlock("IsGreek".to_owned()));

        // The correctly-spelled block escape must resolve to the BLOCK
        // (U+0370-03FF), not the wider Script: it matches a codepoint in
        // Greek and Coptic but not one in Greek Extended.
        let re = translated_regex(r"^\p{IsGreekandCoptic}$", false);
        assert!(re.is_match("\u{0391}")); // GREEK CAPITAL LETTER ALPHA
        assert!(!re.is_match("\u{1F00}")); // Greek Extended, not this block
    }

    #[test]
    fn backreference_is_rejected_single_digit() {
        let err = translate(r"(a)\1", false, false).unwrap_err();
        assert_eq!(err, XsdRegexError::Backreference("\\1".to_owned()));
    }

    #[test]
    fn backreference_is_rejected_multi_digit() {
        let err = translate(r"(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)\10", false, false).unwrap_err();
        assert_eq!(err, XsdRegexError::Backreference("\\10".to_owned()));
    }

    #[test]
    fn bare_b_and_bb_are_rejected_outside_a_class() {
        assert_eq!(
            translate(r"a\bc", false, false).unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\b")
        );
        assert_eq!(
            translate(r"\B", false, false).unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\B")
        );
    }

    #[test]
    fn bare_b_and_bb_are_rejected_inside_a_class() {
        assert_eq!(
            translate(r"[\b]", false, false).unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\b")
        );
        assert_eq!(
            translate(r"[\B]", false, false).unwrap_err(),
            XsdRegexError::UnsupportedConstruct("\\B")
        );
    }

    #[test]
    fn dangling_backslash_is_rejected() {
        assert!(matches!(
            translate("a\\", false, false),
            Err(XsdRegexError::Malformed(_))
        ));
    }

    #[test]
    fn unterminated_class_is_rejected() {
        assert!(matches!(
            translate("[abc", false, false),
            Err(XsdRegexError::Malformed(_))
        ));
    }

    #[test]
    fn unterminated_block_escape_name_is_rejected() {
        assert!(matches!(
            translate(r"\p{IsBasicLatin", false, false),
            Err(XsdRegexError::Malformed(_))
        ));
    }

    #[test]
    fn translation_introduces_zero_new_capturing_groups() {
        let re = translated_regex(r"^(\i+)\s(\c*)\p{IsBasicLatin}$", false);
        // 2 explicit groups in the source + the implicit whole-match group.
        assert_eq!(re.captures_len(), 3);
    }

    /// The `i` flag switches the name escapes to the PRE-FOLDED body inside a
    /// `(?-i:…)` scope; without `i` the raw body is emitted, exactly as before
    /// this rewrite. This is the observable contract the CPU fix rests on:
    /// the scope is what stops `regex-syntax` from re-walking the astral name
    /// range once per occurrence.
    #[test]
    fn name_escapes_are_pre_folded_under_i_only() {
        let with_i = translate(r"^\c$", false, true).expect("translate under i");
        assert!(
            with_i.contains("(?-i:["),
            "under i the escape must carry a scoped, pre-folded class: {with_i}"
        );
        let without_i = translate(r"^\c$", false, false).expect("translate without i");
        assert!(
            !without_i.contains("(?-i:"),
            "without i there is nothing to scope: {without_i}"
        );
        // ...and the scoped form survives the surrounding `(?i)` applied by the
        // builder, because that is exactly what the caller does.
        let scoped = regex::Regex::new(&format!("(?i){with_i}")).expect("compile scoped");
        assert!(scoped.is_match("A"));
        assert!(scoped.is_match("a"));
    }
}
