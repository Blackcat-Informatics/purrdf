// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The ONE transcription of the little-endian, alignment-agnostic byte framing every
//! fixed-layout codec in this crate reads and writes.
//!
//! Every multi-byte field a PurRDF container stores — the pack and artifact headers,
//! the PURREMB directory and its section records, the canonical term encoding — is a
//! fixed-width little-endian integer decoded through `from_le_bytes` over an explicit
//! byte-slice copy, never a pointer cast, so a caller's buffer may sit at any address
//! and the encoding is byte-identical on every target, the 32-bit
//! `wasm32-unknown-unknown` build included. Variable-length fields are *framed*: an
//! eight-byte little-endian length, then exactly that many bytes. No byte value is
//! reserved as a delimiter, because RDF strings are arbitrary UTF-8 and any byte may
//! appear in a payload.
//!
//! Each codec used to transcribe the same six-line helpers privately. They are stated
//! once here so that a bounds rule, a width, or an endianness cannot drift between two
//! codecs that must agree on the bytes. The codecs' own helper names remain as thin
//! wrappers over these, so their byte layouts — frozen by goldens — are unchanged.
//!
//! [`ByteCursor`] is the reader: a position over a borrowed slice whose every read is
//! bounds-checked and whose refusal is the caller's own error type, built from a static
//! reason through the constructor the caller supplies. The free functions are the
//! writers and the offset-addressed readers a fixed-layout header uses.

use core::fmt;

/// A bounds-checked little-endian reader over a borrowed byte slice.
///
/// Every read either yields the value and advances past it or refuses with `E`, built
/// by the `error` constructor from a static reason; the cursor never panics on short
/// input and never reads past the slice. The reason strings are stable and name the
/// fault (`"unexpected end of input"`, `"length exceeds the address space"`), so a
/// caller's error type can carry them verbatim.
pub struct ByteCursor<'a, E> {
    bytes: &'a [u8],
    position: usize,
    error: fn(&'static str) -> E,
}

impl<E> fmt::Debug for ByteCursor<'_, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ByteCursor")
            .field("len", &self.bytes.len())
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

/// The refusal for a read that would run past the end of the slice.
const TRUNCATED: &str = "unexpected end of input";

/// The refusal for a stored length that this target's `usize` cannot hold.
const LENGTH_OVERFLOW: &str = "length exceeds the address space";

impl<'a, E> ByteCursor<'a, E> {
    /// A cursor at the start of `bytes`, refusing through `error`.
    #[must_use]
    pub const fn new(bytes: &'a [u8], error: fn(&'static str) -> E) -> Self {
        Self {
            bytes,
            position: 0,
            error,
        }
    }

    /// The next `n` bytes, borrowed from the underlying slice, advancing past them.
    ///
    /// # Errors
    ///
    /// Fewer than `n` bytes remain.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], E> {
        let end = self
            .position
            .checked_add(n)
            .ok_or_else(|| (self.error)(TRUNCATED))?;
        let slice = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| (self.error)(TRUNCATED))?;
        self.position = end;
        Ok(slice)
    }

    /// The next byte.
    ///
    /// # Errors
    ///
    /// No byte remains.
    pub fn u8(&mut self) -> Result<u8, E> {
        Ok(self.take(1)?[0])
    }

    /// The next two bytes as a little-endian `u16`.
    ///
    /// # Errors
    ///
    /// Fewer than two bytes remain.
    pub fn u16_le(&mut self) -> Result<u16, E> {
        self.take(2)
            .map(|b| u16::from_le_bytes(b.try_into().expect("take yields exactly two bytes")))
    }

    /// The next four bytes as a little-endian `u32`.
    ///
    /// # Errors
    ///
    /// Fewer than four bytes remain.
    pub fn u32_le(&mut self) -> Result<u32, E> {
        self.take(4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("take yields exactly four bytes")))
    }

    /// The next eight bytes as a little-endian `u64`.
    ///
    /// # Errors
    ///
    /// Fewer than eight bytes remain.
    pub fn u64_le(&mut self) -> Result<u64, E> {
        self.take(8)
            .map(|b| u64::from_le_bytes(b.try_into().expect("take yields exactly eight bytes")))
    }

    /// The next sixteen bytes as a little-endian `i128`.
    ///
    /// # Errors
    ///
    /// Fewer than sixteen bytes remain.
    pub fn i128_le(&mut self) -> Result<i128, E> {
        self.take(16)
            .map(|b| i128::from_le_bytes(b.try_into().expect("take yields exactly sixteen bytes")))
    }

    /// A stored length: a little-endian `u64` narrowed to this target's `usize`.
    ///
    /// # Errors
    ///
    /// Fewer than eight bytes remain, or the stored value does not fit a `usize` (a
    /// length past 4 GiB on a 32-bit target such as `wasm32-unknown-unknown`).
    pub fn length(&mut self) -> Result<usize, E> {
        let stored = self.u64_le()?;
        usize::try_from(stored).map_err(|_| (self.error)(LENGTH_OVERFLOW))
    }

    /// A framed field: its [`length`](Self::length), then exactly that many bytes,
    /// borrowed — the reader of [`push_framed`].
    ///
    /// # Errors
    ///
    /// The length cannot be read or held, or fewer bytes than it names remain.
    pub fn framed(&mut self) -> Result<&'a [u8], E> {
        let n = self.length()?;
        self.take(n)
    }

    /// How many bytes remain unread.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    /// Whether every byte has been read.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.position == self.bytes.len()
    }

    /// The offset of the next byte to be read, from the start of the slice.
    #[must_use]
    pub const fn position(&self) -> usize {
        self.position
    }
}

/// Append `value` as two little-endian bytes.
#[inline]
pub fn put_u16_le(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Append `value` as four little-endian bytes.
#[inline]
pub fn put_u32_le(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Append `value` as eight little-endian bytes.
#[inline]
pub fn put_u64_le(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}

/// Append `bytes` behind its length, so the field can be read back without a
/// terminator and without knowing anything about its contents.
///
/// The length is a `u64` little-endian prefix rather than a separator byte or an
/// escape scheme: RDF strings are arbitrary UTF-8 (a lexical form may contain NUL, a
/// blank label may contain the marker, an IRI may contain anything the producer
/// wrote), so NO byte value is available as a delimiter. A fixed-width count is the
/// only framing that is oblivious to the payload. Little-endian and a fixed 8 bytes
/// make the encoding byte-identical on every target, including the 32-bit
/// `wasm32-unknown-unknown` build where `usize` is narrower. Read back with
/// [`ByteCursor::framed`].
#[inline]
pub fn push_framed(out: &mut Vec<u8>, bytes: &[u8]) {
    // `usize` is at most 64 bits wide on every supported target, so the cast is
    // lossless; `as` rather than `try_from` keeps the writer infallible.
    out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Overwrite the four bytes at `offset` with `value`, little-endian.
///
/// # Panics
///
/// `out` does not hold `offset + 4` bytes: a fixed-layout writer patches a field it
/// has already reserved, so a short buffer is a programming error.
#[inline]
pub fn write_u32_le_at(out: &mut [u8], offset: usize, value: u32) {
    out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

/// Overwrite the eight bytes at `offset` with `value`, little-endian.
///
/// # Panics
///
/// `out` does not hold `offset + 8` bytes; see [`write_u32_le_at`].
#[inline]
pub fn write_u64_le_at(out: &mut [u8], offset: usize, value: u64) {
    out[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

/// The little-endian `u32` at `offset`, or `None` when the slice does not hold
/// `offset + 4` bytes (an overflowing `offset` included).
#[inline]
#[must_use]
pub fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let field: [u8; 4] = bytes.get(offset..end)?.try_into().ok()?;
    Some(u32::from_le_bytes(field))
}

/// The little-endian `u64` at `offset`, or `None` when the slice does not hold
/// `offset + 8` bytes (an overflowing `offset` included).
#[inline]
#[must_use]
pub fn read_u64_le(bytes: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let field: [u8; 8] = bytes.get(offset..end)?.try_into().ok()?;
    Some(u64::from_le_bytes(field))
}

/// The smallest multiple of `align` that is at least `value`, or `None` when it would
/// overflow `usize` or when `align` is zero.
///
/// This is exactly [`usize::checked_next_multiple_of`], named for what a section
/// writer uses it for: the padded offset at which the next aligned section begins.
/// Stated here so every codec that pads to an alignment asks the same question of the
/// same function; an unchecked `(value + align - 1) & !(align - 1)` in one codec and a
/// checked one in another would disagree exactly at the overflow a hostile length
/// reaches.
#[inline]
#[must_use]
pub const fn align_up(value: usize, align: usize) -> Option<usize> {
    value.checked_next_multiple_of(align)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test's own error type: the reason string, kept verbatim.
    #[derive(Debug, PartialEq, Eq)]
    struct Refused(&'static str);

    fn cursor(bytes: &[u8]) -> ByteCursor<'_, Refused> {
        ByteCursor::new(bytes, Refused)
    }

    #[test]
    fn cursor_reads_every_width_little_endian_and_advances() {
        let mut out = Vec::new();
        out.push(0x7f);
        put_u16_le(&mut out, 0x0102);
        put_u32_le(&mut out, 0x0304_0506);
        put_u64_le(&mut out, 0x0708_090a_0b0c_0d0e);
        out.extend_from_slice(&(-3i128).to_le_bytes());
        push_framed(&mut out, b"abc");
        let mut c = cursor(&out);
        assert_eq!(c.position(), 0);
        assert_eq!(c.remaining(), out.len());
        assert_eq!(c.u8(), Ok(0x7f));
        assert_eq!(c.u16_le(), Ok(0x0102));
        assert_eq!(c.u32_le(), Ok(0x0304_0506));
        assert_eq!(c.u64_le(), Ok(0x0708_090a_0b0c_0d0e));
        assert_eq!(c.i128_le(), Ok(-3));
        assert_eq!(c.framed(), Ok(&b"abc"[..]));
        assert!(c.is_empty());
        assert_eq!(c.position(), out.len());
        assert_eq!(c.remaining(), 0);
        // The bytes are the canonical little-endian spelling, not a host-order dump.
        assert_eq!(&out[1..3], &[0x02, 0x01]);
        assert_eq!(&out[3..7], &[0x06, 0x05, 0x04, 0x03]);
    }

    #[test]
    fn cursor_refuses_a_short_read_without_advancing() {
        let bytes = [1u8, 2, 3];
        let mut c = cursor(&bytes);
        assert_eq!(c.u32_le(), Err(Refused(TRUNCATED)));
        assert_eq!(c.position(), 0, "a refused read leaves the cursor in place");
        assert_eq!(c.u16_le(), Ok(0x0201));
        assert_eq!(c.u16_le(), Err(Refused(TRUNCATED)));
        assert_eq!(c.take(1), Ok(&[3u8][..]));
        assert_eq!(c.u8(), Err(Refused(TRUNCATED)));
        assert_eq!(c.take(usize::MAX), Err(Refused(TRUNCATED)));
        assert!(c.is_empty());
    }

    #[test]
    fn framed_refuses_a_length_past_the_payload_and_reads_an_empty_frame() {
        let mut out = Vec::new();
        put_u64_le(&mut out, 4);
        out.extend_from_slice(b"ab");
        assert_eq!(cursor(&out).framed(), Err(Refused(TRUNCATED)));
        let mut empty = Vec::new();
        push_framed(&mut empty, b"");
        let mut c = cursor(&empty);
        assert_eq!(c.framed(), Ok(&b""[..]));
        assert!(c.is_empty());
    }

    #[test]
    fn length_refuses_a_value_wider_than_usize() {
        let mut out = Vec::new();
        put_u64_le(&mut out, u64::MAX);
        let refused = cursor(&out).length();
        if usize::BITS < 64 {
            assert_eq!(refused, Err(Refused(LENGTH_OVERFLOW)));
        } else {
            assert_eq!(refused, Ok(usize::MAX));
        }
        // The valid neighbour: a length that fits is read.
        let mut small = Vec::new();
        put_u64_le(&mut small, 7);
        assert_eq!(cursor(&small).length(), Ok(7));
    }

    #[test]
    fn offset_readers_and_writers_agree_and_refuse_out_of_range() {
        let mut buffer = vec![0u8; 16];
        write_u32_le_at(&mut buffer, 2, 0xdead_beef);
        write_u64_le_at(&mut buffer, 8, 0x0102_0304_0506_0708);
        assert_eq!(read_u32_le(&buffer, 2), Some(0xdead_beef));
        assert_eq!(read_u64_le(&buffer, 8), Some(0x0102_0304_0506_0708));
        assert_eq!(&buffer[2..6], &[0xef, 0xbe, 0xad, 0xde]);
        assert_eq!(read_u32_le(&buffer, 13), None);
        assert_eq!(read_u64_le(&buffer, 9), None);
        assert_eq!(read_u32_le(&buffer, usize::MAX), None);
        assert_eq!(read_u64_le(&buffer, usize::MAX - 3), None);
        // The valid neighbours at the very end of the buffer.
        assert_eq!(read_u32_le(&buffer, 12), Some(0x0102_0304));
        assert_eq!(read_u64_le(&buffer, 8), Some(0x0102_0304_0506_0708));
    }

    #[test]
    fn push_framed_matches_the_canonical_term_framing() {
        let mut out = Vec::new();
        push_framed(&mut out, b"s");
        assert_eq!(out, [1, 0, 0, 0, 0, 0, 0, 0, b's']);
    }

    #[test]
    fn align_up_is_checked_next_multiple_of() {
        assert_eq!(align_up(0, 8), Some(0));
        assert_eq!(align_up(1, 8), Some(8));
        assert_eq!(align_up(8, 8), Some(8));
        assert_eq!(align_up(9, 8), Some(16));
        assert_eq!(align_up(usize::MAX, 8), None);
        assert_eq!(align_up(5, 0), None);
        assert_eq!(align_up(5, 1), Some(5));
    }
}
