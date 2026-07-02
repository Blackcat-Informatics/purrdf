// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! XPath-flavoured regular-expression support for the ShEx `PATTERN` facet
//! (spec §5.4.5).
//!
//! ShEx patterns follow XPath/XQuery `fn:matches` semantics: the match is
//! **partial** (no implicit anchoring — `^`/`$` are explicit metacharacters)
//! and the flag letters `s`, `m`, `i`, `x` and `q` carry their XPath
//! meanings. The XPath regex grammar is close enough to the `regex` crate's
//! that translation is a light-touch pass:
//!
//! * `\d \D \w \W \s \S`, `\p{…}`/`\P{…}`, anchors, quantifiers,
//!   alternation and groups pass through unchanged (the `regex` crate's
//!   Unicode-default classes are the closest available approximation of the
//!   XPath ones);
//! * XPath character-class subtraction `[a-z-[aeiou]]` is rewritten to the
//!   `regex` crate's difference syntax `[a-z--[aeiou]]`;
//! * the XPath-only multi-char escapes `\i \I \c \C` (XML name characters)
//!   are **unsupported** and reported as a facet error (they do not appear
//!   in the shexTest validation corpus);
//! * the `q` flag (XPath 3.0 "treat as literal") is honoured by escaping
//!   the whole pattern.

use regex::Regex;

/// Compile a ShEx `PATTERN` facet (XPath regex source + flags) into a
/// [`Regex`] that implements `fn:matches` partial-match semantics.
pub(crate) fn compile_pattern(pattern: &str, flags: Option<&str>) -> Result<Regex, String> {
    let flags = flags.unwrap_or("");
    let mut inline = String::new();
    let mut literal = false;
    for flag in flags.chars() {
        match flag {
            's' | 'm' | 'i' | 'x' => inline.push(flag),
            'q' => literal = true,
            other => return Err(format!("unsupported regex flag {other:?}")),
        }
    }
    let body = if literal {
        regex::escape(pattern)
    } else {
        translate_xpath(pattern)?
    };
    let source = if inline.is_empty() {
        body
    } else {
        format!("(?{inline}){body}")
    };
    Regex::new(&source).map_err(|e| format!("invalid pattern /{pattern}/{flags}: {e}"))
}

/// Rewrite an XPath regex into `regex`-crate syntax (see the module doc).
fn translate_xpath(pattern: &str) -> Result<String, String> {
    let mut out = String::with_capacity(pattern.len() + 8);
    let mut chars = pattern.chars().peekable();
    let mut in_class = false;
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                let Some(&escaped) = chars.peek() else {
                    return Err("pattern ends with a dangling backslash".to_owned());
                };
                chars.next();
                match escaped {
                    'i' | 'I' | 'c' | 'C' => {
                        return Err(format!(
                            "unsupported XPath multi-character escape \\{escaped}"
                        ));
                    }
                    _ => {
                        out.push('\\');
                        out.push(escaped);
                    }
                }
            }
            '[' if !in_class => {
                in_class = true;
                out.push('[');
                // A leading `^` (negation) and/or `]` literal pass through.
                if chars.peek() == Some(&'^') {
                    out.push('^');
                    chars.next();
                }
            }
            ']' if in_class => {
                in_class = false;
                out.push(']');
            }
            '-' if in_class && chars.peek() == Some(&'[') => {
                // XPath class subtraction `[...-[...]]` → regex `[...--[...]]`.
                out.push_str("--");
            }
            other => out.push(other),
        }
    }
    Ok(out)
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

    #[test]
    fn class_subtraction_is_rewritten() {
        let re = compile_pattern("^[a-z-[aeiou]]+$", None).expect("compile");
        assert!(re.is_match("bcd"));
        assert!(!re.is_match("bad"));
    }

    #[test]
    fn xml_name_escapes_are_reported() {
        assert!(compile_pattern(r"\i\c*", None).is_err());
    }

    #[test]
    fn digit_class_matches_unicode() {
        let re = compile_pattern(r"^\d+$", None).expect("compile");
        assert!(re.is_match("42"));
        assert!(!re.is_match("4a"));
    }
}
