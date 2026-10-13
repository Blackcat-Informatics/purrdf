// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! * XPath makes a malformed replacement a dynamic error `err:FORX0004`,
//!   where the `regex` crate substitutes the offending text literally.
//!
//! This module parses the replacement string **once per `replace_all` call**
//! into literal runs and group references, applying the `$N` rules against the
//! pattern's capture-group count, and renders each match through the parsed
//! template. Under the `q` flag none of this runs: the replacement is used as
//! is (F&O §5.6.2), which is `regex::NoExpand`'s semantics.

use super::error::ReplacementError;

/// A borrowed replacement piece; absent groups deliberately expand to nothing.
pub(super) enum Part<'a> {
    Literal(&'a str),
    Group(Option<usize>),
}

pub(super) enum ParseFailure<E> {
    Replacement(ReplacementError),
    Work(E),
}

/// The one replacement cursor, shared by owned compatibility templates and
/// bounded native expansion. It retains no source-sized allocation.
pub(super) struct Cursor<'a> {
    source: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    groups: usize,
    suffix: Option<&'a str>,
}

impl<'a> Cursor<'a> {
    pub(super) fn new(source: &'a str, groups: usize) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            groups,
            suffix: None,
        }
    }

    /// Charge before scanning; even a discarded out-of-range reference spends
    /// work, and operational refusal remains separate from FORX0004.
    pub(super) fn next<E>(
        &mut self,
        charge: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<Part<'a>>, ParseFailure<E>> {
        if let Some(suffix) = self.suffix.take() {
            return Ok(Some(Part::Literal(suffix)));
        }
        let Some(&(offset, ch)) = self.chars.peek() else {
            return Ok(None);
        };
        charge(1).map_err(ParseFailure::Work)?;
        self.chars.next();
        match ch {
            '\\' => {
                charge(1).map_err(ParseFailure::Work)?;
                match self.chars.next() {
                    Some((_, '\\')) => Ok(Some(Part::Literal("\\"))),
                    Some((_, '$')) => Ok(Some(Part::Literal("$"))),
                    _ => Err(ParseFailure::Replacement(
                        ReplacementError::UnescapedBackslash { offset },
                    )),
                }
            }
            '$' => {
                let start = offset + 1;
                let mut end = start;
                while let Some(&(position, digit)) = self.chars.peek() {
                    if !digit.is_ascii_digit() {
                        break;
                    }
                    charge(1).map_err(ParseFailure::Work)?;
                    self.chars.next();
                    end = position + 1;
                }
                if end == start {
                    return Err(ParseFailure::Replacement(
                        ReplacementError::DollarWithoutGroup { offset },
                    ));
                }
                let digits = &self.source[start..end];
                charge(digits.len() as u64).map_err(ParseFailure::Work)?;
                let (group, suffix) = resolve_group(digits, self.groups);
                self.suffix = (!suffix.is_empty()).then_some(suffix);
                Ok(Some(Part::Group(group)))
            }
            _ => {
                let mut end = offset + ch.len_utf8();
                while let Some(&(position, next)) = self.chars.peek() {
                    if matches!(next, '\\' | '$') {
                        break;
                    }
                    charge(1).map_err(ParseFailure::Work)?;
                    self.chars.next();
                    end = position + next.len_utf8();
                }
                Ok(Some(Part::Literal(&self.source[offset..end])))
            }
        }
    }
}

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
        let mut cursor = Cursor::new(replacement, capture_groups);
        let mut unmetered = |_| Ok::<(), std::convert::Infallible>(());
        while let Some(part) = cursor
            .next(&mut unmetered)
            .map_err(|failure| match failure {
                ParseFailure::Replacement(error) => error,
                ParseFailure::Work(never) => match never {},
            })?
        {
            match part {
                Part::Literal(text) => literal.push_str(text),
                Part::Group(group) => {
                    if !literal.is_empty() {
                        pieces.push(Piece::Literal(std::mem::take(&mut literal)));
                    }
                    if let Some(n) = group {
                        pieces.push(Piece::Group(n));
                    }
                }
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
/// The final case is resolved by its equivalent longest admissible prefix:
/// `$23` with five groups is group 2 followed by the literal digit `3`.
fn resolve_group(digits: &str, capture_groups: usize) -> (Option<usize>, &str) {
    // Decimal extension is monotone. The longest prefix at most max(S,9) is
    // exactly the prefix left by the specified right-peeling rule, including
    // arbitrarily many leading zeroes. Overflow means greater, never saturation.
    let ceiling = capture_groups.max(9);
    let mut number = 0_usize;
    let mut end = 0;
    for (index, digit) in digits.bytes().enumerate() {
        let Some(next) = number
            .checked_mul(10)
            .and_then(|n| n.checked_add(usize::from(digit - b'0')))
            .filter(|&n| n <= ceiling)
        else {
            break;
        };
        number = next;
        end = index + 1;
    }
    debug_assert!(end > 0, "the first ASCII digit always fits the ceiling");
    (
        (number == 0 || number <= capture_groups).then_some(number),
        &digits[end..],
    )
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
