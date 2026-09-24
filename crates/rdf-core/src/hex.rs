// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one-shot lowercase-hex renderer shared by every crate downstream of the
//! IR kernel.
//!
//! "Render these bytes as lowercase hex" is the single most-copied four-line
//! function in this workspace: digests in error messages, `blake3:<hex>` blob
//! addresses, content ids, pack certificates, proof-step keys, GTS frame ids.
//! Every copy was the same `for byte in bytes { write!(out, "{byte:02x}") }`
//! accumulate loop, and copies of one operation are exactly the DUPLICATE shape
//! the workspace's standing goals forbid. There is therefore **one**
//! transcription of it, here, in the crate that is a common ancestor of those
//! consumers.
//!
//! It is pure `core`/`alloc` over [`String`]: no dependencies (the third-party
//! `hex` crate is banned workspace-wide and is not coming back), no
//! `std::fs`/`std::io`, wasm32-clean.
//!
//! # Scope: one-shot rendering only
//!
//! [`lower`] is for call sites that render a handful of digests per operation —
//! an error `Display`, a content address, a cache key. It is deliberately NOT
//! the answer for every hex-shaped need in the workspace. These kinds of call
//! site correctly do something else:
//!
//! * **Hot paths** that render inside a fixpoint's inner loop into a label
//!   buffer that already holds its prefix. `purrdf_datalog::chase`'s Skolem
//!   witness-label renderer keeps its own lookup table for this reason, and it
//!   cannot reach this crate's formatting-free [`lower`] without a temporary
//!   `String` per witness; that is a performance decision backed by its own
//!   rationale, not a stray copy.
//! * **Allocation-free renderers** that write into a fixed inline buffer or a
//!   caller-supplied byte sink rather than a heap [`String`] — `crate::ir::canon`'s
//!   `HashHex`, and the LPG projection's block renderer. They render the same
//!   characters but do not produce this function's return type, so routing them
//!   through it would *add* the allocation they exist to avoid.
//! * **Crates that cannot reach this one.** `purrdf-gts` declares exactly one
//!   first-party dependency, the zero-dependency events crate; giving it an edge
//!   to the IR kernel to share four lines would invert the layering that puts
//!   `purrdf-rdf` above both. It keeps one renderer of its own, in `wire`.
//! * **Selective escapes, which are not this operation.** `crate::ir::skolem`
//!   and `purrdf_shapes::rules`'s focus tag both emit `-{byte:02x}` for
//!   non-alphanumeric bytes ONLY, passing the rest through. That is an escape,
//!   not a rendering: most input bytes never become hex at all.
//! * **Renderers that append into a caller's accumulator.**
//!   `purrdf_shapes::schema_import`'s property-shape label does render every
//!   byte — it is this operation — but it writes into one `String` already
//!   holding a prefix and shared across several parts, so calling a function
//!   that *returns* a `String` would add a temporary allocation per part for no
//!   gain. Reach for [`lower`] when you want the value; write in place when you
//!   are building one buffer out of many pieces.
//!
//! Anything else that turns a `&[u8]` into an owned lowercase-hex [`String`]
//! should call [`lower`]. It is the only transcription of the loop that should
//! exist outside the cases above — check this list before adding another, and
//! add to the list rather than leaving a new copy unexplained.
//!
//! This list is deliberately not numbered. An earlier revision of it said "two
//! kinds" above four bullets, and the pull request that introduced this module
//! carried a count of the copies it had folded that was stale one commit later.
//! A tally in prose has no gate behind it.

/// The lowercase hex digit of nibble `n` (`0..16`), as comparisons only.
///
/// `n + b'0'` is the digit for `n <= 9`; the letters start 39 bytes after
/// `b'9' + 1`, so a nibble above 9 adds 39 more. The comparison yields a
/// `0x00`/`0xFF` byte and the addend is selected by masking, with no branch, so
/// a loop over a buffer of nibbles lowers to packed byte compares and adds.
#[inline]
const fn nibble_digit(n: u8) -> u8 {
    let letter = ((n > 9) as u8).wrapping_neg();
    n + b'0' + (letter & (b'a' - b'9' - 1))
}

/// Render `bytes` as lowercase hexadecimal, two characters per byte.
///
/// Leading zero bytes are preserved: each byte is rendered independently and
/// zero-padded to width two, so the output length is always exactly
/// `2 * bytes.len()`. An empty slice renders as an empty string.
///
/// The digits are written into a buffer sized up front, two per input byte,
/// each nibble mapped to its digit by [`nibble_digit`]'s comparisons, so the
/// loop has no formatting call and no branch per byte.
///
/// ```
/// use purrdf_core::hex;
///
/// assert_eq!(hex::lower(&[0x00, 0x0f, 0xff]), "000fff");
/// assert_eq!(hex::lower(&[]), "");
/// ```
#[must_use]
pub fn lower(bytes: &[u8]) -> String {
    let mut out = vec![0_u8; bytes.len() * 2];
    let (pairs, _) = out.as_chunks_mut::<2>();
    for (pair, &byte) in pairs.iter_mut().zip(bytes) {
        *pair = [nibble_digit(byte >> 4), nibble_digit(byte & 0x0F)];
    }
    // Every byte written is an ASCII hex digit, so the buffer is UTF-8.
    String::from_utf8(out).expect("hex digits are ASCII")
}

#[cfg(test)]
mod tests {
    use super::lower;
    use core::fmt::Write as _;

    /// The per-byte `write!` rendering [`lower`] replaced: its oracle.
    fn lower_formatted(bytes: &[u8]) -> String {
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            // Infallible: `String`'s `fmt::Write` never returns an error.
            let _ = write!(out, "{byte:02x}");
        }
        out
    }

    /// Every byte value, alone and in a run of every length up to 70 (so any
    /// chunked body and its tail are both exercised), renders exactly as the
    /// formatted loop did.
    #[test]
    fn matches_the_formatted_rendering() {
        for byte in 0..=u8::MAX {
            assert_eq!(lower(&[byte]), lower_formatted(&[byte]), "byte {byte:#04x}");
        }
        let all: Vec<u8> = (0..=u8::MAX).rev().chain(0..=u8::MAX).collect();
        assert_eq!(lower(&all), lower_formatted(&all));
        for len in 0..=70 {
            let run = &all[100..100 + len];
            assert_eq!(lower(run), lower_formatted(run), "length {len}");
        }
    }

    #[test]
    fn empty_input_renders_empty() {
        assert_eq!(lower(&[]), "");
    }

    /// Every one of the 256 byte values renders as exactly two lowercase hex
    /// characters, in order — the property every folded call site relied on.
    #[test]
    fn every_byte_value_renders_as_two_lowercase_chars() {
        let all: Vec<u8> = (0..=u8::MAX).collect();
        let rendered = lower(&all);
        assert_eq!(rendered.len(), 512);
        assert!(
            rendered
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "rendering must be lowercase hex only, got {rendered}"
        );
        for (index, byte) in all.iter().enumerate() {
            let pair = &rendered[index * 2..index * 2 + 2];
            assert_eq!(
                u8::from_str_radix(pair, 16).expect("a hex pair parses"),
                *byte,
                "byte {byte:#04x} round-trips through its rendered pair"
            );
        }
    }

    /// The classic "treat the bytes as one integer and lose the leading zero"
    /// bug: a leading zero byte must still occupy its two characters.
    #[test]
    fn leading_zero_bytes_are_not_dropped() {
        assert_eq!(lower(&[0x00, 0x00, 0x01]), "000001");
    }

    /// Agrees with an independent nibble-lookup rendering — deliberately a
    /// different algorithm from [`lower`]'s per-byte `write!`, so the two cannot
    /// share a bug, and so the folded call sites' output is pinned against
    /// something other than a restatement of the implementation.
    #[test]
    fn agrees_with_an_independent_nibble_lookup() {
        const DIGITS: &[u8; 16] = b"0123456789abcdef";
        let bytes: Vec<u8> = (0..64u16).map(|i| (i * 7 % 256) as u8).collect();
        let mut expected = String::with_capacity(bytes.len() * 2);
        for byte in &bytes {
            expected.push(char::from(DIGITS[usize::from(byte >> 4)]));
            expected.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
        }
        assert_eq!(lower(&bytes), expected);
    }
}
