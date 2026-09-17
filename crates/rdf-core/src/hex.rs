// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! the answer for every hex-shaped need in the workspace, and two kinds of call
//! site correctly do something else:
//!
//! * **Hot paths** that render inside a fixpoint's inner loop, where per-byte
//!   [`write!`] formatting is measurable. `purrdf_datalog::chase`'s Skolem
//!   witness-label renderer keeps its own lookup table for this reason; that is
//!   a performance decision backed by its own rationale, not a stray copy.
//! * **Allocation-free renderers** that write into a fixed inline buffer rather
//!   than a heap [`String`] (`crate::ir::canon`'s `HashHex`). They render the
//!   same characters but do not produce this function's return type, so routing
//!   them through it would *add* the allocation they exist to avoid.
//!
//! Anything else that turns a `&[u8]` into an owned lowercase-hex [`String`]
//! should call [`lower`] rather than growing a fifth copy of the loop.

use core::fmt::Write as _;

/// Render `bytes` as lowercase hexadecimal, two characters per byte.
///
/// Leading zero bytes are preserved: each byte is rendered independently and
/// zero-padded to width two, so the output length is always exactly
/// `2 * bytes.len()`. An empty slice renders as an empty string.
///
/// ```
/// use purrdf_core::hex;
///
/// assert_eq!(hex::lower(&[0x00, 0x0f, 0xff]), "000fff");
/// assert_eq!(hex::lower(&[]), "");
/// ```
#[must_use]
pub fn lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        // Infallible: `String`'s `fmt::Write` never returns an error.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::lower;

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
