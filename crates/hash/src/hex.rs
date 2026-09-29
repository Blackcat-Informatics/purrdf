// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Lowercase base16 (RFC 4648 §8) rendering of byte strings.
//!
//! [`Lower`] wraps a byte slice and renders it through [`Display`]: each
//! byte becomes two characters of `0123456789abcdef`, high nibble first, with
//! no separator and no prefix, so a digest renders exactly as its `{:02x}`
//! byte-by-byte form does.
//!
//! ```
//! use purrdf_hash::hex::Lower;
//!
//! assert_eq!(Lower(b"foobar").to_string(), "666f6f626172");
//! assert_eq!(format!("sha256:{}", Lower(&[0x00, 0xff])), "sha256:00ff");
//! ```
//!
//! Rendering never allocates: the characters are produced in a stack buffer
//! and handed to the formatter one chunk at a time, one `write_str` per chunk
//! of up to 128 input bytes (every digest this crate computes is one chunk).
//! The chunk is encoded on SSSE3 (x86-64, detected at run time), NEON
//! (AArch64) or wasm `simd128` (when compiled in), otherwise by the portable
//! table; every path writes the same bytes.

use core::fmt::{self, Alignment, Display, Formatter, Write as _};

use crate::backend::HexBackend;
use crate::dispatch::Backend as _;

/// The lowercase base16 alphabet: RFC 4648 §8's, with `a`–`f` for 10–15.
pub(crate) const ALPHABET: &[u8; 16] = b"0123456789abcdef";

/// Input bytes encoded per `write_str`.
const CHUNK: usize = 128;

/// A byte string rendered as lowercase hexadecimal by [`Display`].
///
/// Construct it with any byte slice, including a `&[u8; N]` or a digest
/// array, which coerce: `Lower(&digest)`. Formatter width, fill, alignment and
/// precision apply to the rendered characters as they do to a `str`.
#[derive(Clone, Copy, Debug)]
pub struct Lower<'a>(pub &'a [u8]);

impl Display for Lower<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let encode = HexBackend::selected().encode_fn();
        if f.width().is_none() && f.precision().is_none() {
            return write_chars(f, self.0, encode, usize::MAX);
        }
        // `str` semantics: precision caps the characters written, width pads
        // what remains, left-aligned unless the formatter says otherwise.
        let total = self.0.len().saturating_mul(2);
        let shown = f
            .precision()
            .map_or(total, |precision| total.min(precision));
        let padding = f.width().map_or(0, |width| width.saturating_sub(shown));
        let (before, after) = match f.align() {
            None | Some(Alignment::Left) => (0, padding),
            Some(Alignment::Right) => (padding, 0),
            Some(Alignment::Center) => (padding / 2, padding - padding / 2),
        };
        let fill = f.fill();
        for _ in 0..before {
            f.write_char(fill)?;
        }
        write_chars(f, self.0, encode, shown)?;
        for _ in 0..after {
            f.write_char(fill)?;
        }
        Ok(())
    }
}

/// Writes the first `limit` characters of the encoding of `bytes`.
fn write_chars(
    f: &mut Formatter<'_>,
    bytes: &[u8],
    encode: crate::arch::HexEncode,
    mut limit: usize,
) -> fmt::Result {
    let mut buffer = [0u8; 2 * CHUNK];
    for chunk in bytes.chunks(CHUNK) {
        if limit == 0 {
            break;
        }
        let out = &mut buffer[..2 * chunk.len()];
        encode(chunk, out);
        let take = out.len().min(limit);
        // Every byte the alphabet holds is ASCII, so this never fails.
        let text = core::str::from_utf8(&out[..take]).map_err(|_| fmt::Error)?;
        f.write_str(text)?;
        limit -= take;
    }
    Ok(())
}

/// The portable encoder: `output` must be exactly twice as long as `input`.
pub(crate) fn encode_portable(input: &[u8], output: &mut [u8]) {
    debug_assert_eq!(output.len(), 2 * input.len());
    let (pairs, _) = output.as_chunks_mut::<2>();
    for (&byte, pair) in input.iter().zip(pairs) {
        *pair = [
            ALPHABET[usize::from(byte >> 4)],
            ALPHABET[usize::from(byte & 0x0f)],
        ];
    }
}
