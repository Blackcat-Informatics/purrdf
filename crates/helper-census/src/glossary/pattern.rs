// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Python glossary-pattern compatibility over the one bounded ECMA engine.

use purrdf_jsonschema::ecma::{self, CompiledPattern, MatchLimits};
use std::fmt::Write;

const WORD: &str = r"\p{L}\p{N}_";
const SPACE: &str =
    r"\u0009-\u000D\u001C-\u0020\u0085\u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000";

#[derive(Debug)]
pub(super) struct Pattern {
    compiled: CompiledPattern,
    original: String,
}

impl Pattern {
    pub(super) fn anchor(source: &str) -> Result<Self, String> {
        let adapted = format!("(?<![A-Za-z0-9]){}", adapt(source, true)?);
        let compiled =
            ecma::compile(&adapted).map_err(|error| format!("anchor {source:?}: {error}"))?;
        Ok(Self {
            compiled,
            original: source.to_owned(),
        })
    }

    pub(super) fn new(source: &str, ignore_case: bool) -> Result<Self, String> {
        let adapted = adapt(source, ignore_case)?;
        let compiled =
            ecma::compile(&adapted).map_err(|error| format!("pattern {source:?}: {error}"))?;
        Ok(Self {
            compiled,
            original: source.to_owned(),
        })
    }

    pub(super) fn matches(&self, text: &str) -> Result<bool, String> {
        self.with_limits(text, &mut MatchLimits::default())
    }

    fn with_limits(&self, text: &str, limits: &mut MatchLimits) -> Result<bool, String> {
        self.compiled
            .is_match(text, limits)
            .map_err(|error| format!("pattern {:?}: {error}", self.original))
    }
}

pub(super) fn literal(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        if r"\.^$|?*+()[]{}".contains(ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn adapt(source: &str, ignore_case: bool) -> Result<String, String> {
    let mut out = String::new();
    let mut chars = source.chars().peekable();
    let mut class = false;
    let mut unfold_class = false;
    let mut class_start = 0;
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                let escaped = chars.next().ok_or("trailing pattern backslash")?;
                match escaped {
                    'w' => {
                        if !class {
                            out.push_str("(?-i:[");
                        }
                        out.push_str(WORD);
                        if !class {
                            out.push_str("])");
                        }
                    }
                    'W' if !class => write!(out, "(?-i:[^{WORD}])").expect("String write"),
                    's' => {
                        if !class {
                            out.push('[');
                        }
                        out.push_str(SPACE);
                        if !class {
                            out.push(']');
                        }
                    }
                    'S' if !class => write!(out, "[^{SPACE}]").expect("String write"),
                    'd' => out.push_str(if class { r"\p{Nd}" } else { r"[\p{Nd}]" }),
                    'D' if !class => out.push_str(r"[^\p{Nd}]"),
                    'b' if !class => write!(
                        out,
                        "(?-i:(?:(?<=[{WORD}])(?![{WORD}])|(?<![{WORD}])(?=[{WORD}])))"
                    )
                    .expect("String write"),
                    'B' if !class => write!(
                        out,
                        "(?-i:(?:(?=[\\s\\S])|(?<=[\\s\\S]))(?:(?<=[{WORD}])(?=[{WORD}])|(?<![{WORD}])(?![{WORD}])))"
                    )
                    .expect("String write"),
                    'b' => out.push_str(r"\u0008"),
                    'W' | 'S' | 'D' => {
                        return Err(format!(
                            "pattern {source:?}: complemented shorthand inside a class is unsupported"
                        ));
                    }
                    'n' | 'r' | 't' | 'f' | 'v' => {
                        out.push('\\');
                        out.push(escaped);
                    }
                    other if !other.is_alphanumeric() => {
                        if r"\.^$|?*+()[]{}".contains(other) || (class && other == '-') {
                            out.push('\\');
                        }
                        out.push(other);
                    }
                    _ => {
                        return Err(format!(
                            "pattern {source:?}: unsupported escape \\{escaped}"
                        ));
                    }
                }
            }
            '[' if !class => {
                class = true;
                let body: String = chars.clone().take_while(|ch| *ch != ']').collect();
                if body.is_empty() || body == "^" {
                    return Err(format!(
                        "pattern {source:?}: empty or leading-] classes are unsupported; escape a literal ]"
                    ));
                }
                unfold_class = body.contains(r"\w");
                if unfold_class {
                    out.push_str("(?-i:");
                }
                class_start = out.len();
                out.push(ch);
            }
            ']' if class => {
                class = false;
                out.push(ch);
                if ignore_case && !unfold_class {
                    expand_case_class(&mut out, class_start)?;
                }
                if unfold_class {
                    out.push(')');
                }
                unfold_class = false;
            }
            '.' if !class => out.push_str("[^\\n]"),
            '$' if !class => out.push_str("(?=\\n?$)"),
            '(' if chars.peek() == Some(&'?') => {
                let kind = chars.clone().nth(1);
                if !matches!(kind, Some(':' | '=' | '!' | '<'))
                    || (kind == Some('<') && !matches!(chars.clone().nth(2), Some('=' | '!')))
                {
                    return Err(format!(
                        "pattern {source:?}: only noncapturing groups and lookarounds are supported"
                    ));
                }
                if kind == Some('<') {
                    fixed_lookbehind(chars.clone().skip(3), source)?;
                }
                out.push(ch);
            }
            ch if ignore_case && !class && "iIkKsS".contains(ch) => {
                out.push_str(match ch {
                    'i' | 'I' => "[iIİı]",
                    'k' | 'K' => "[kK\u{212a}]",
                    _ => "[sSſ]",
                });
            }
            _ => out.push(ch),
        }
    }
    if ignore_case {
        Ok(format!("(?i:{out})"))
    } else {
        Ok(out)
    }
}

fn fixed_lookbehind(chars: impl Iterator<Item = char>, source: &str) -> Result<(), String> {
    let mut chars = chars;
    let mut class = false;
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            chars.next();
            continue;
        }
        if ch == '[' {
            class = true;
        }
        if ch == ']' {
            class = false;
        }
        if !class && ch == ')' {
            return Ok(());
        }
        if !class && "*+?{|(".contains(ch) {
            return Err(format!(
                "pattern {source:?}: lookbehind must contain fixed-width literals/classes; place alternatives outside it"
            ));
        }
    }
    Err(format!("pattern {source:?}: unterminated lookbehind"))
}

fn expand_case_class(out: &mut String, start: usize) -> Result<(), String> {
    let class = &out[start..];
    let negated = class.starts_with("[^");
    let compiled = ecma::compile(class).map_err(|error| format!("class {class:?}: {error}"))?;
    let mut extra = String::new();
    for (letters, added) in [
        (["i", "I"], "İı"),
        (["s", "S"], "ſ"),
        (["k", "K"], "\u{212a}"),
    ] {
        for letter in letters {
            if compiled
                .is_match(letter, &mut MatchLimits::default())
                .map_err(|error| error.to_string())?
                != negated
            {
                extra.push_str(added);
                break;
            }
        }
    }
    out.pop();
    out.push_str(&extra);
    out.push(']');
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_unicode_classes_and_case() {
        for text in ["é", "中", "²", "_", "٣"] {
            assert!(Pattern::new(r"^\w$", false).unwrap().matches(text).unwrap());
        }
        for text in ["\u{1c}", "\u{85}", "\u{a0}", "\u{2028}"] {
            assert!(Pattern::new(r"^\s$", false).unwrap().matches(text).unwrap());
        }
        assert!(
            !Pattern::new(r"^\s$", false)
                .unwrap()
                .matches("\u{feff}")
                .unwrap()
        );
        assert!(Pattern::new("^.$", false).unwrap().matches("\r").unwrap());
        assert!(!Pattern::new("^.$", false).unwrap().matches("\n").unwrap());
        assert!(!Pattern::new(r"x\b", false).unwrap().matches("x中").unwrap());
        assert!(Pattern::new(r"x\b", false).unwrap().matches("x!").unwrap());
        for (pattern, text) in [("i", "İ"), ("I", "ı"), ("s", "ſ"), ("k", "\u{212a}")] {
            assert!(Pattern::new(pattern, true).unwrap().matches(text).unwrap());
            assert!(!Pattern::new(pattern, false).unwrap().matches(text).unwrap());
        }
        for text in ["İ", "ı", "ſ", "\u{212a}"] {
            assert!(
                Pattern::new("^[a-z]$", true)
                    .unwrap()
                    .matches(text)
                    .unwrap()
            );
        }
        assert!(Pattern::new("^[i]$", true).unwrap().matches("ı").unwrap());
        assert!(Pattern::new("^a-z$", true).unwrap().matches("a-z").unwrap());
        assert!(
            Pattern::new("foo$", false)
                .unwrap()
                .matches("foo\n")
                .unwrap()
        );
        assert!(
            !Pattern::new("foo$", false)
                .unwrap()
                .matches("foo\n\n")
                .unwrap()
        );
        assert!(
            !Pattern::new(r"^\w$", true)
                .unwrap()
                .matches("\u{345}")
                .unwrap()
        );
        assert!(
            !Pattern::new(r"^[\w-]$", true)
                .unwrap()
                .matches("\u{345}")
                .unwrap()
        );
        assert!(!Pattern::new(r"\B", false).unwrap().matches("").unwrap());
        assert!(Pattern::new(r"\B", false).unwrap().matches("!").unwrap());
    }

    #[test]
    fn syntax_and_resource_errors_are_not_no_match() {
        assert!(Pattern::new("[", false).is_err());
        assert!(Pattern::new(r"\q", false).is_err());
        assert!(Pattern::new("[^]", false).is_err());
        assert!(Pattern::new("(?<!a+)", false).is_err());
        assert!(Pattern::new("(?<name>a)", false).is_err());
        let compiled = Pattern::new("(?=a)a", false).unwrap();
        assert!(
            compiled
                .with_limits(
                    "a",
                    &mut MatchLimits {
                        steps: 0,
                        states: 1
                    }
                )
                .is_err()
        );
    }
}
