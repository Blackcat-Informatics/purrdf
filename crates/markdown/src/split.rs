// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The split law: an oversize unit into pieces, each within the bound.
//!
//! Dialect-independent. Nothing here knows what a heading or a verse
//! is; it is handed a byte range that some dialect reader called one
//! unit, and it answers with the pieces that range becomes. The one
//! dialect fact it relies on is that the range never spans a heading,
//! which is a property of the range it is given, not of this code.

use crate::profile::Profile;

/// The split law: pieces of at most `max_bytes`, cut at the last
/// newline at or before the bound, else at the last scalar boundary; a
/// continuation reaches back `overlap` bytes, snapped backward to a
/// newline, never before the unit's start.
///
/// The bound holds with no exception, and the seam is what makes it
/// hold. [`slice_markdown`](crate::slice_markdown) refuses a
/// `max_bytes` under [`MIN_MAX_BYTES`](crate::MIN_MAX_BYTES) before a
/// byte of the document is read, so a unit only ever arrives here under
/// a bound of four or more. The widest UTF-8 scalar is four bytes, so
/// the scalar opening a piece ends at or before `ps + 4`, which is at
/// or before `ps + max_bytes` — the bound itself. The fallback's
/// `ceil_boundary(ps + 1)` therefore cannot climb past the bound, the
/// newline branch cuts at an offset the bound already covers, and every
/// cut is at or before the bound. The same fact is what puts `bytes[cut]`
/// in range: a piece is only cut when the unit runs past the bound, so
/// the cut is under the unit's end and under the text's length.
pub(crate) fn split_spans(
    text: &str,
    start: usize,
    end: usize,
    profile: &Profile,
) -> Vec<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut pieces = Vec::new();
    let mut ps = start;
    while ps < end {
        if end - ps <= profile.max_bytes {
            pieces.push((ps, end));
            break;
        }
        let bound = ps + profile.max_bytes;
        let cut = match bytes[ps..=bound].iter().rposition(|b| *b == b'\n') {
            Some(i) if i > 0 => ps + i,
            _ => floor_boundary(text, bound).max(ceil_boundary(text, ps + 1)),
        };
        debug_assert!(
            cut <= bound,
            "a cut is never past the bound: the widest scalar is four bytes and the bound is at least four"
        );
        pieces.push((ps, cut));
        let resume = if bytes[cut] == b'\n' { cut + 1 } else { cut };
        ps = overlap_start(text, ps, cut, profile.overlap).unwrap_or(resume);
    }
    pieces
}

/// The continuation start: `cut - overlap` snapped backward to the
/// byte after a newline (else a scalar boundary), if that is still
/// inside the piece; otherwise none, and the caller resumes at the cut.
fn overlap_start(text: &str, ps: usize, cut: usize, overlap: usize) -> Option<usize> {
    if overlap == 0 {
        return None;
    }
    let candidate = cut.checked_sub(overlap)?;
    if candidate <= ps {
        return None;
    }
    let snapped = text.as_bytes()[ps..candidate]
        .iter()
        .rposition(|b| *b == b'\n')
        .map_or_else(|| floor_boundary(text, candidate), |i| ps + i + 1);
    (snapped > ps).then_some(snapped)
}

/// The last scalar boundary at or before an offset.
pub(crate) fn floor_boundary(text: &str, mut i: usize) -> usize {
    while !text.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// The first scalar boundary at or after an offset.
pub(crate) fn ceil_boundary(text: &str, mut i: usize) -> usize {
    while i < text.len() && !text.is_char_boundary(i) {
        i += 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_split_never_cuts_inside_a_scalar_and_always_progresses() {
        let text = "\u{2042}".repeat(10);
        let mut profile = Profile::new(
            "test",
            1,
            crate::Vocabulary::under("urn:test:").expect("a vocabulary"),
        );
        profile.max_bytes = 4;
        profile.overlap = 0;
        let pieces = split_spans(&text, 0, text.len(), &profile);
        assert!(pieces.iter().all(|&(s, e)| text.is_char_boundary(s)
            && text.is_char_boundary(e)
            && e > s
            && e - s <= 4));
        assert_eq!(pieces.last(), Some(&(27, 30)));
    }
}
