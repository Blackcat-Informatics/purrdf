// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixed-width little-endian integers at an offset of a byte buffer.
//!
//! Every binary container PurRDF writes — the pack container, the artifact
//! envelope, the PURREMB embedding companion — stores its integers
//! little-endian at fixed offsets. These four functions are the one way to
//! reach them: each takes the buffer and a byte offset, touches exactly the
//! integer's width, and answers [`None`] when the integer would not fit inside
//! the buffer, so a truncated or hostile image is a value the caller turns into
//! its own error rather than a panic.
//!
//! The width is the type's own. A read goes through `first_chunk`, so the
//! array the integer is built from has the integer's size by construction; the
//! bounds check is the one `get(offset..)` and `first_chunk` perform, which the
//! compiler folds into a single comparison.
//!
//! A reader that consumes a buffer front to back (a cursor) takes its integers
//! with `split_first_chunk` and `from_le_bytes` directly; appending an integer
//! to a growing buffer is `extend_from_slice(&value.to_le_bytes())`.
//!
//! Layout arithmetic around these accesses computes through the standard
//! library's checked forms: an offset rounds up to an alignment with
//! [`u64::checked_next_multiple_of`] (or its `usize` twin), and a count of
//! fixed-size units covering a length is [`u64::div_ceil`].

/// The little-endian `u32` at `offset` of `bytes`, or [`None`] when fewer than
/// four bytes remain there.
///
/// ```
/// use purrdf_core::bytes::read_u32_le;
///
/// let bytes = [0xff, 0x78, 0x56, 0x34, 0x12];
/// assert_eq!(read_u32_le(&bytes, 1), Some(0x1234_5678));
/// assert_eq!(read_u32_le(&bytes, 2), None);
/// ```
#[inline]
#[must_use]
pub fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..)?
        .first_chunk()
        .copied()
        .map(u32::from_le_bytes)
}

/// The little-endian `u64` at `offset` of `bytes`, or [`None`] when fewer than
/// eight bytes remain there.
///
/// ```
/// use purrdf_core::bytes::read_u64_le;
///
/// let bytes = 0x0102_0304_0506_0708_u64.to_le_bytes();
/// assert_eq!(read_u64_le(&bytes, 0), Some(0x0102_0304_0506_0708));
/// assert_eq!(read_u64_le(&bytes, 1), None);
/// ```
#[inline]
#[must_use]
pub fn read_u64_le(bytes: &[u8], offset: usize) -> Option<u64> {
    bytes
        .get(offset..)?
        .first_chunk()
        .copied()
        .map(u64::from_le_bytes)
}

/// Write `value` little-endian at `offset` of `bytes`; [`None`] (and nothing
/// written) when fewer than four bytes remain there.
///
/// ```
/// use purrdf_core::bytes::put_u32_le;
///
/// let mut bytes = [0u8; 5];
/// assert_eq!(put_u32_le(&mut bytes, 1, 0x1234_5678), Some(()));
/// assert_eq!(bytes, [0, 0x78, 0x56, 0x34, 0x12]);
/// assert_eq!(put_u32_le(&mut bytes, 2, 1), None);
/// assert_eq!(bytes, [0, 0x78, 0x56, 0x34, 0x12]);
/// ```
#[inline]
#[must_use = "a None is an offset outside the buffer, and nothing was written"]
pub fn put_u32_le(bytes: &mut [u8], offset: usize, value: u32) -> Option<()> {
    *bytes.get_mut(offset..)?.first_chunk_mut()? = value.to_le_bytes();
    Some(())
}

/// Write `value` little-endian at `offset` of `bytes`; [`None`] (and nothing
/// written) when fewer than eight bytes remain there.
///
/// ```
/// use purrdf_core::bytes::put_u64_le;
///
/// let mut bytes = [0u8; 8];
/// assert_eq!(put_u64_le(&mut bytes, 0, 0x0102_0304_0506_0708), Some(()));
/// assert_eq!(bytes, [8, 7, 6, 5, 4, 3, 2, 1]);
/// assert_eq!(put_u64_le(&mut bytes, 1, 0), None);
/// ```
#[inline]
#[must_use = "a None is an offset outside the buffer, and nothing was written"]
pub fn put_u64_le(bytes: &mut [u8], offset: usize, value: u64) -> Option<()> {
    *bytes.get_mut(offset..)?.first_chunk_mut()? = value.to_le_bytes();
    Some(())
}

#[cfg(test)]
mod tests {
    use super::{put_u32_le, put_u64_le, read_u32_le, read_u64_le};

    /// Every offset of every buffer length up to twelve: a read or write
    /// succeeds exactly when the integer fits, and a write that fits reads
    /// back.
    #[test]
    fn an_integer_is_reached_exactly_when_it_fits() {
        for len in 0..=12 {
            let source: Vec<u8> = (0x10_u8..).take(len).collect();
            for offset in 0..=len + 1 {
                let fits32 = offset + 4 <= len;
                let fits64 = offset + 8 <= len;
                assert_eq!(read_u32_le(&source, offset).is_some(), fits32);
                assert_eq!(read_u64_le(&source, offset).is_some(), fits64);
                if fits32 {
                    let expected =
                        u32::from_le_bytes(*source[offset..].first_chunk().expect("it fits"));
                    assert_eq!(read_u32_le(&source, offset), Some(expected));
                }
                let mut buffer = source.clone();
                assert_eq!(
                    put_u32_le(&mut buffer, offset, 0xdead_beef).is_some(),
                    fits32
                );
                if fits32 {
                    assert_eq!(read_u32_le(&buffer, offset), Some(0xdead_beef));
                } else {
                    assert_eq!(buffer, source, "a refused write leaves the buffer alone");
                }
                let mut buffer = source.clone();
                assert_eq!(
                    put_u64_le(&mut buffer, offset, u64::MAX - 1).is_some(),
                    fits64
                );
                if fits64 {
                    assert_eq!(read_u64_le(&buffer, offset), Some(u64::MAX - 1));
                } else {
                    assert_eq!(buffer, source, "a refused write leaves the buffer alone");
                }
            }
        }
    }

    /// An offset past the end, `usize::MAX` included, is refused, never an
    /// overflow.
    #[test]
    fn an_offset_past_the_end_is_refused() {
        let mut bytes = [0u8; 8];
        assert_eq!(read_u32_le(&bytes, usize::MAX), None);
        assert_eq!(read_u64_le(&bytes, usize::MAX), None);
        assert_eq!(put_u32_le(&mut bytes, usize::MAX, 1), None);
        assert_eq!(put_u64_le(&mut bytes, usize::MAX, 1), None);
        // Its neighbour: the last offset the integer fits at is accepted.
        assert_eq!(read_u32_le(&bytes, 4), Some(0));
        assert_eq!(put_u64_le(&mut bytes, 0, 1), Some(()));
    }
}
