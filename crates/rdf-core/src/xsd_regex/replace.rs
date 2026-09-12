// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Translation of an XPath F&O 3.1 §5.6.2 `fn:replace` replacement string.
//!
//! [`CompiledPattern::replace_all`](super::CompiledPattern::replace_all) must
//! speak XPath's replacement language, not the `regex` crate's. The two agree
//! on `$N` in the common case but diverge everywhere else:
//!
//! * XPath writes a literal `$` as `\$` and a literal `\` as `\\`; the
//!   `regex` crate treats both backslashes as ordinary characters, so copying
//!   an XPath replacement straight through yields the two-character text
//!   `\$`/`\\` instead of the one character meant.
//! * XPath resolves `$N` against the pattern's *own* capture-group count with
//!   a documented carve-out (`N > S` and `N > 9`: the last digit is literal
//!   text and the rest of the number is re-resolved), where the `regex` crate
//!   treats a too-large `$N` as a reference to a (non-existent) group and
//!   substitutes nothing.
//! * XPath makes a malformed replacement a dynamic error [err:FORX0004],
//!   where the `regex` crate substitutes the offending text literally.
//!
//! This module parses the replacement string **once per `replace_all` call**
//! into literal runs and group references, applying the `$N` rules against the
//! pattern's capture-group count, and renders each match through the parsed
//! template. Under the `q` flag none of this runs: the replacement is used as
//! is (F&O §5.6.2), which is `regex::NoExpand`'s semantics.

use super::error::ReplacementError;

/// One piece of a parsed replacement template, in output order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Piece {
    /// Characters copied to the output verbatim (escapes already resolved).
    Literal(String),
    /// The substring captured by capture group `n`; `0` is the whole match.
    Group(usize),
}

/// A replacement string parsed against a pattern's capture-group count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Template {
    pieces: Vec<Piece>,
}

impl Template {
    /// Parse `replacement` for a pattern with `capture_groups` explicit
    /// capturing groups (F&O §5.6.2's `S`, not counting group 0).
    ///
    /// # Errors
    ///
    /// [`ReplacementError`] for a `$` not followed by a digit and for a `\`
    /// that is neither `\\` nor `\$` — the two `err:FORX0004` conditions of
    /// F&O 3.1 §5.6.2.
    pub(super) fn parse(
        replacement: &str,
        capture_groups: usize,
    ) -> Result<Self, ReplacementError> {
        let mut pieces = Vec::new();
        let mut literal = String::new();
        let mut chars = replacement.char_indices().peekable();
        while let Some((offset, c)) = chars.next() {
            match c {
                '\\' => match chars.next() {
                    Some((_, '\\')) => literal.push('\\'),
                    Some((_, '$')) => literal.push('$'),
                    _ => return Err(ReplacementError::UnescapedBackslash { offset }),
                },
                '$' => {
                    // `$N` consumes the whole run of consecutive digits: the
                    // number N is "all the digits that consecutively follow".
                    let mut digits = String::new();
                    while let Some(&(_, d)) = chars.peek() {
                        if d.is_ascii_digit() {
                            digits.push(d);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if digits.is_empty() {
                        return Err(ReplacementError::DollarWithoutGroup { offset });
                    }
                    if !literal.is_empty() {
                        pieces.push(Piece::Literal(std::mem::take(&mut literal)));
                    }
                    let (group, suffix) = resolve_group(&digits, capture_groups);
                    if let Some(n) = group {
                        pieces.push(Piece::Group(n));
                    }
                    if !suffix.is_empty() {
                        pieces.push(Piece::Literal(suffix.to_owned()));
                    }
                }
                other => literal.push(other),
            }
        }
        if !literal.is_empty() {
            pieces.push(Piece::Literal(literal));
        }
        Ok(Self { pieces })
    }

    /// Render `caps` through this template.
    ///
    /// A group that did not participate in the match (including the
    /// out-of-range references the spec maps to the empty string) expands to
    /// nothing, which is exactly `Captures::get`'s `None`.
    pub(super) fn expand(&self, caps: &regex::Captures<'_>) -> String {
        let mut out = String::new();
        for piece in &self.pieces {
            match piece {
                Piece::Literal(s) => out.push_str(s),
                Piece::Group(n) => {
                    if let Some(m) = caps.get(*n) {
                        out.push_str(m.as_str());
                    }
                }
            }
        }
        out
    }
}

/// Resolve the digit run of a `$N` reference against `s`, the pattern's
/// explicit capture-group count.
///
/// Returns the group to expand (if any) and the trailing digits that are
/// literal text. This is F&O 3.1 §5.6.2's four-way rule, verbatim:
///
/// * `N = 0` → the whole match (group 0);
/// * `1 <= N <= S` → group `N`;
/// * `S < N <= 9` → the empty string;
/// * `N > S` and `N > 9` → the last digit is literal text, and the rule is
///   reapplied to the number formed by stripping it.
///
/// The last case is the "peel from the right" loop below: `$23` with five
/// groups is group 2 followed by the literal digit `3`.
fn resolve_group(digits: &str, capture_groups: usize) -> (Option<usize>, &str) {
    let s = u64::try_from(capture_groups).unwrap_or(u64::MAX);
    let mut end = digits.len();
    loop {
        // A run long enough to overflow `u64` is certainly larger than any
        // real group count, so saturating is exactly the comparison wanted.
        let n = digits[..end].parse::<u64>().unwrap_or(u64::MAX);
        if n == 0 {
            return (Some(0), &digits[end..]);
        }
        if n <= s {
            let group = usize::try_from(n).expect("n <= s, and s was a usize");
            return (Some(group), &digits[end..]);
        }
        if n <= 9 {
            return (None, &digits[end..]);
        }
        // `N > S` and `N > 9`: the last digit is literal text, so peel it off
        // and reapply the rule to the remaining prefix. `end` cannot reach 0:
        // a single digit always satisfies `n <= 9`.
        end -= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal(text: &str) -> Piece {
        Piece::Literal(text.to_owned())
    }

    #[test]
    fn dollar_escape_and_backslash_escape_resolve_to_one_character() {
        let template = Template::parse(r"a\$b\\c", 0).expect("valid replacement");
        assert_eq!(template.pieces, vec![literal(r"a$b\c")]);
    }

    #[test]
    fn group_reference_resolves_in_range() {
        // Three groups: `$1`, `$3` and `$0` are all in-range references.
        let template = Template::parse("$1-$3-$0", 3).expect("valid replacement");
        assert_eq!(
            template.pieces,
            vec![
                Piece::Group(1),
                literal("-"),
                Piece::Group(3),
                literal("-"),
                Piece::Group(0)
            ]
        );
    }

    #[test]
    fn out_of_range_group_below_ten_is_empty() {
        let template = Template::parse("x$5y", 1).expect("valid replacement");
        assert_eq!(template.pieces, vec![literal("x"), literal("y")]);
    }

    #[test]
    fn over_ten_reference_peels_trailing_digits_as_literals() {
        // The spec's own example: `$23` with five groups is group 2 + "3".
        let template = Template::parse("$23", 5).expect("valid replacement");
        assert_eq!(template.pieces, vec![Piece::Group(2), literal("3")]);
        // With one group, `$23` strips "3", leaving `$2` which is out of range
        // and below ten: empty, then the literal "3".
        let template = Template::parse("$23", 1).expect("valid replacement");
        assert_eq!(template.pieces, vec![literal("3")]);
    }

    #[test]
    fn bare_dollar_and_bare_backslash_are_forx0004() {
        assert_eq!(
            Template::parse("$x", 0).unwrap_err(),
            ReplacementError::DollarWithoutGroup { offset: 0 }
        );
        assert_eq!(
            Template::parse("$", 0).unwrap_err(),
            ReplacementError::DollarWithoutGroup { offset: 0 }
        );
        assert_eq!(
            Template::parse(r"\n", 0).unwrap_err(),
            ReplacementError::UnescapedBackslash { offset: 0 }
        );
        assert_eq!(
            Template::parse("a\\", 0).unwrap_err(),
            ReplacementError::UnescapedBackslash { offset: 1 }
        );
    }
}
