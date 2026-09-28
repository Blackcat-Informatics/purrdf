// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Unicode ECMA-262 emission from the shared XSD scanner's normalized IR.
//!
//! JSON Schema 2020-12 Core §6.4 recommends Unicode processing. The target is `RegExp(source, "u")`.
//! Character sets are expanded to explicit scalar ranges,
//! so target Unicode-table versions cannot change their meaning. No flags
//! remain implicit in the emitted source.

use std::fmt::{self, Write};

use regex_syntax::hir::{Class, Hir, HirKind, Look};

use super::{MAX_TRANSLATED_BYTES, XsdRegexError, prepare_with};

/// A pattern cannot be faithfully emitted within the declared resource limits.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Ecma262Error {
    /// The source does not belong to the supported XSD/XPath language.
    Source(XsdRegexError),
    /// Normalization rejected the translated pattern.
    Syntax(String),
    /// Emission exceeds the shared translated-pattern byte limit.
    TooLarge,
    /// The normalized expression contains a construct outside this target.
    Unsupported(&'static str),
}

impl fmt::Display for Ecma262Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(f, "XSD/XPath pattern: {error}"),
            Self::Syntax(error) => write!(f, "pattern normalization: {error}"),
            Self::TooLarge => write!(f, "ECMA-262 pattern exceeds {MAX_TRANSLATED_BYTES} bytes"),
            Self::Unsupported(construct) => {
                write!(f, "unsupported regular-expression translation: {construct}")
            }
        }
    }
}

impl std::error::Error for Ecma262Error {}

/// Translate XSD/XPath source and flags to Unicode ECMA-262 source.
///
/// Execute the result with the `u` flag, as JSON Schema 2020-12 Core §6.4
/// recommends. `s`, `m`, `x`, and `q` are incorporated into the source;
/// callers must not add flags. All classes use explicit Unicode scalar ranges.
/// XPath multiline anchors exclude the position after a final newline. The
/// `i` flag is written into the source too: XPath F&O 3.1 §5.6.2 defines it by
/// case variants (`fn:lower-case(C1) eq fn:lower-case(C2) or fn:upper-case(C1)
/// eq fn:upper-case(C2)`), not by simple case folding, so ECMA-262's own `i`
/// would judge a different language. Each normal character used as an atom
/// becomes the class of its variants, each character class gains the variants
/// of what its characters and ranges match, and every escape (`\p{Lu}`, `\d`,
/// `\i`, …) is unaffected, as the specification requires.
///
/// # Errors
/// Invalid or unsupported source and bounded-size failures are typed errors;
/// an expression with a different accepted language is never returned.
pub fn to_ecma_262(pattern: &str, flags: &str) -> Result<String, Ecma262Error> {
    let prepared = prepare_with(pattern, flags, true).map_err(Ecma262Error::Source)?;
    debug_assert!(!prepared.case_insensitive, "i is in the source");
    let hir = regex_syntax::ParserBuilder::new()
        .dot_matches_new_line(prepared.dot_all)
        .multi_line(prepared.multi_line)
        .build()
        .parse(&prepared.source)
        .map_err(|error| Ecma262Error::Syntax(error.to_string()))?;
    let mut output = String::with_capacity(prepared.source.len());
    emit(&hir, &mut output, Target::Ecma)?;
    Ok(output)
}

/// Translate the proven shared subset of Unicode ECMA-262 into XSD/XPath.
///
/// Explicit Unicode escapes become literal scalar ranges, so the returned
/// source is valid XSD syntax rather than Rust or JavaScript escape syntax.
/// Unsupported semantics return a typed failure before any constraint exists.
pub fn from_ecma_262(pattern: &str) -> Result<String, Ecma262Error> {
    if pattern.len() > MAX_TRANSLATED_BYTES {
        return Err(Ecma262Error::TooLarge);
    }
    if !ecma_262_rust_compatible(pattern) {
        return Err(Ecma262Error::Unsupported(
            "ECMA-262 source outside the proven shared grammar",
        ));
    }
    let hir = regex_syntax::Parser::new()
        .parse(pattern)
        .map_err(|error| Ecma262Error::Syntax(error.to_string()))?;
    let mut output = String::with_capacity(pattern.len());
    emit(&hir, &mut output, Target::Xsd)?;
    if output.len() > super::MAX_SOURCE_BYTES {
        return Err(Ecma262Error::Source(XsdRegexError::TooLarge {
            bytes: output.len(),
            limit: super::MAX_SOURCE_BYTES,
        }));
    }
    Ok(output)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    Ecma,
    Xsd,
}

fn emit(hir: &Hir, output: &mut String, target: Target) -> Result<(), Ecma262Error> {
    match hir.kind() {
        // XPath F&O 3.1 §5.6.1.3 extends XSD groups with the `?:` marker.
        HirKind::Empty => output.push_str("(?:)"),
        HirKind::Literal(literal) => {
            let text = std::str::from_utf8(&literal.0)
                .map_err(|_| Ecma262Error::Unsupported("non-Unicode byte literals"))?;
            for character in text.chars() {
                push_character(output, character, false, target);
                check_size(output)?;
            }
        }
        HirKind::Class(Class::Unicode(class)) => {
            if class.ranges().is_empty() {
                output.push_str(if target == Target::Ecma {
                    "[^\\u{0}-\\u{10ffff}]"
                } else {
                    "[a-[a]]"
                });
            } else {
                output.push('[');
                for range in class.ranges() {
                    push_character(output, range.start(), true, target);
                    if range.start() != range.end() {
                        output.push('-');
                        push_character(output, range.end(), true, target);
                    }
                    check_size(output)?;
                }
                output.push(']');
            }
        }
        // HIR canonicalizes an empty Unicode set to an empty byte set. It
        // still denotes the empty language, not a byte-mode matching surface.
        HirKind::Class(Class::Bytes(class)) if class.ranges().is_empty() => {
            output.push_str(if target == Target::Ecma {
                "[^\\u{0}-\\u{10ffff}]"
            } else {
                "[a-[a]]"
            });
        }
        HirKind::Class(Class::Bytes(_)) => {
            return Err(Ecma262Error::Unsupported("byte character classes"));
        }
        HirKind::Look(look) => output.push_str(match look {
            Look::Start => "^",
            Look::End => "$",
            Look::StartLF if target == Target::Ecma => "(?:^|(?<=\\n)(?!$))",
            Look::EndLF if target == Target::Ecma => "(?=\\n|(?<!\\n)$)",
            _ => return Err(Ecma262Error::Unsupported("non-XPath look assertions")),
        }),
        HirKind::Repetition(repetition) => {
            output.push_str("(?:");
            emit(&repetition.sub, output, target)?;
            output.push(')');
            match (repetition.min, repetition.max) {
                (0, None) => output.push('*'),
                (1, None) => output.push('+'),
                (0, Some(1)) => output.push('?'),
                (minimum, Some(maximum)) if minimum == maximum => {
                    write!(output, "{{{minimum}}}").expect("writing to String");
                }
                (minimum, Some(maximum)) => {
                    write!(output, "{{{minimum},{maximum}}}").expect("writing to String");
                }
                (minimum, None) => {
                    write!(output, "{{{minimum},}}").expect("writing to String");
                }
            }
            if !repetition.greedy {
                output.push('?');
            }
        }
        HirKind::Capture(capture) => {
            output.push('(');
            emit(&capture.sub, output, target)?;
            output.push(')');
        }
        HirKind::Concat(children) => {
            for child in children {
                emit(child, output, target)?;
            }
        }
        HirKind::Alternation(children) => {
            output.push_str("(?:");
            for (index, child) in children.iter().enumerate() {
                if index != 0 {
                    output.push('|');
                }
                emit(child, output, target)?;
            }
            output.push(')');
        }
    }
    check_size(output)
}

fn check_size(output: &str) -> Result<(), Ecma262Error> {
    if output.len() > MAX_TRANSLATED_BYTES {
        Err(Ecma262Error::TooLarge)
    } else {
        Ok(())
    }
}

fn push_character(output: &mut String, character: char, in_class: bool, target: Target) {
    if target == Target::Xsd {
        if matches!(character, '\\' | '[' | ']' | '^' | '-')
            || (!in_class
                && matches!(
                    character,
                    '.' | '$' | '|' | '?' | '*' | '+' | '(' | ')' | '{' | '}'
                ))
        {
            output.push('\\');
        }
        output.push(character);
    } else if character.is_ascii_alphanumeric() || (!in_class && character == ' ') {
        output.push(character);
    } else {
        write!(output, "\\u{{{:x}}}", u32::from(character)).expect("writing to String");
    }
}

/// Whether Unicode ECMA-262 source has identical meaning in Rust `regex`.
///
/// This admits a shared, explicitly checked grammar, not merely expressions
/// which Rust can compile. Unicode-sensitive shorthand classes, wildcard and
/// lookaround, inline flags and engine-specific escapes refuse.
/// Explicit scalar ranges, literals, groups and quantifiers are supported.
#[must_use]
pub fn ecma_262_rust_compatible(pattern: &str) -> bool {
    if pattern.len() > MAX_TRANSLATED_BYTES || regex::Regex::new(pattern).is_err() {
        return false;
    }
    let mut chars = pattern.chars().peekable();
    let mut in_class = false;
    let mut can_repeat = false;
    let mut quantified = false;
    let mut lazy = false;
    while let Some(character) = chars.next() {
        if !in_class && matches!(character, '*' | '+' | '?' | '{') {
            if !can_repeat {
                return false;
            }
            if quantified {
                if character == '?' && !lazy {
                    lazy = true;
                    continue;
                }
                return false;
            }
            quantified = true;
            if character != '{' {
                continue;
            }
        } else {
            quantified = false;
            lazy = false;
        }
        can_repeat = in_class || !matches!(character, '^' | '$' | '|' | '(' | '[');
        match character {
            '\\' => {
                let Some(escape) = chars.next() else {
                    return false;
                };
                match escape {
                    'n' | 'r' | 't' | 'f' | 'v' | '^' | '$' | '\\' | '.' | '*' | '+' | '?'
                    | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '/' => {}
                    '-' if in_class => {}
                    'x' => {
                        for _ in 0..2 {
                            if !chars.next().is_some_and(|c| c.is_ascii_hexdigit()) {
                                return false;
                            }
                        }
                    }
                    'u' => {
                        let braced = chars.next_if_eq(&'{').is_some();
                        let mut value = 0_u32;
                        let mut count = 0;
                        while chars.peek().is_some_and(char::is_ascii_hexdigit)
                            && (braced || count < 4)
                        {
                            let digit = chars
                                .next()
                                .and_then(|c| c.to_digit(16))
                                .expect("hex digit");
                            let Some(next) =
                                value.checked_mul(16).and_then(|n| n.checked_add(digit))
                            else {
                                return false;
                            };
                            value = next;
                            count += 1;
                        }
                        if count == 0
                            || (!braced && count != 4)
                            || (braced && chars.next() != Some('}'))
                            || char::from_u32(value).is_none()
                        {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            '[' if !in_class => in_class = true,
            '[' => return false,
            ']' if in_class => in_class = false,
            ']' => return false,
            '&' | '~' | '-' if in_class && chars.peek() == Some(&character) => return false,
            '.' if !in_class => return false,
            '(' if !in_class && chars.next_if_eq(&'?').is_some() => {
                if chars.next() != Some(':') {
                    return false;
                }
            }
            '{' if !in_class => {
                let mut digits = 0;
                while chars.next_if(char::is_ascii_digit).is_some() {
                    digits += 1;
                }
                if digits == 0 {
                    return false;
                }
                if chars.next_if_eq(&',').is_some() {
                    while chars.next_if(char::is_ascii_digit).is_some() {}
                }
                if chars.next() != Some('}') {
                    return false;
                }
            }
            '}' if !in_class => return false,
            _ => {}
        }
    }
    !in_class
}

#[cfg(test)]
mod tests {
    use super::{Ecma262Error, ecma_262_rust_compatible, to_ecma_262};

    #[test]
    fn compatibility_checks_language_not_compilation() {
        for pattern in [
            r"\d", r"\D", r"\s", r"\S", r"\w", r"\W", ".", r"\p{L}", r"\b", r"(?i)a", r"(?=a)",
            r"[a&&b]", r"a++", r"^*", r"\#", "a{",
        ] {
            assert!(
                !ecma_262_rust_compatible(pattern),
                "must refuse {pattern:?}"
            );
        }
        for pattern in [
            "",
            "^abc$",
            "[A-Za-z0-9_]+",
            "(?:a|b){2,4}?",
            r"\u{10000}",
            r"[\u{0}-\u{9}]",
            r"[\-]",
            r"\x41",
            "🦀+",
        ] {
            assert!(ecma_262_rust_compatible(pattern), "must accept {pattern:?}");
        }
    }

    /// Run emitted Unicode ECMA-262 source through the Rust engine. The i-flag
    /// output uses only literals, groups, anchors, explicit `\u{…}` ranges and
    /// general categories, whose meaning the two engines share.
    fn matches(source: &str, flags: &str, input: &str) -> bool {
        let emitted = to_ecma_262(source, flags).expect("i translates");
        regex::Regex::new(&emitted)
            .expect("emitted source compiles")
            .is_match(input)
    }

    /// F&O 3.1 §5.6.2's own examples of the `i` flag, and the two places its
    /// case-variant relation differs from simple case folding.
    #[test]
    fn i_flag_is_written_as_xpath_case_variants() {
        for (source, flags, input, expected) in [
            ("^z$", "i", "Z", true),
            ("^[A-Z]$", "i", "q", true),
            ("^[A-Z]$", "i", "\u{212a}", true),
            ("^[A-Z-[IO]]$", "i", "b", true),
            ("^[A-Z-[IO]]$", "i", "i", false),
            ("^[A-Z-[IO]]$", "i", "O", false),
            ("^[^Q]$", "i", "q", false),
            ("^[^Q]$", "i", "r", true),
            // "All other constructs are unaffected": `\p{Lu}` stays upper-case.
            (r"^\p{Lu}$", "i", "a", false),
            (r"^[\p{Lu}]$", "i", "a", false),
            (r"^[\p{Lu}x]$", "i", "X", true),
            (r"^\i$", "i", "a", true),
            // Dotless ı and i upper-case alike; İ lower-cases to two characters.
            ("^i$", "i", "ı", true),
            ("^i$", "i", "İ", false),
            ("^ı$", "i", "I", true),
            // Leading and trailing `-` stay members; a range ending a class
            // keeps its extent.
            ("^[-a]$", "i", "A", true),
            ("^[-a]$", "i", "-", true),
            ("^[a-]$", "i", "-", true),
            ("^[--a]$", "i", "-", true),
            ("^[\\t-z]$", "i", "K", true),
            // `q` makes every character a normal atom.
            ("a.C", "qi", "xA.cx", true),
            ("a.C", "qi", "xAbcx", false),
            // `x`, `s` and `m` compose.
            ("^ a b $", "ix", "AB", true),
            ("^A.B$", "is", "a\nb", true),
        ] {
            assert_eq!(
                matches(source, flags, input),
                expected,
                "{source:?}/{flags:?} on {input:?}"
            );
        }
    }

    #[test]
    fn invalid_and_over_limit_sources_fail_closed() {
        for source in [r"(a)\1", r"\p{IsMissing}", r"(?i)a", r"[a-z-[b]c]"] {
            assert!(to_ecma_262(source, "").is_err(), "must refuse {source:?}");
        }
        assert!(matches!(
            to_ecma_262("a", "z"),
            Err(Ecma262Error::Source(_))
        ));
        assert!(to_ecma_262(&"a".repeat(super::super::MAX_SOURCE_BYTES + 1), "").is_err());
        // One small source can expand to many Unicode ranges. The output
        // ceiling applies while writing, rather than after unbounded growth.
        assert!(matches!(
            to_ecma_262(&r"\p{L}".repeat(1000), ""),
            Err(Ecma262Error::TooLarge)
        ));
    }
}
