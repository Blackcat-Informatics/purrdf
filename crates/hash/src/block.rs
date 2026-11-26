// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Block buffering shared by the block-oriented hashers.

use crate::SecretArray;

/// Finish the current block. Return whether the bit length needs a second
/// block, in which case the current block contains only the marker and zeros
/// following the input bytes.
#[inline]
pub(crate) fn pad_md(block: &mut [u8], filled: usize, length: &[u8]) -> bool {
    debug_assert!(length.len() < block.len() && filled < block.len());
    let length_start = block.len() - length.len();
    block[filled] = 0x80;
    if filled >= length_start {
        block[filled + 1..].fill(0);
        true
    } else {
        block[filled + 1..length_start].fill(0);
        block[length_start..].copy_from_slice(length);
        false
    }
}

/// Write the length into a continuation block containing no input bytes.
#[inline]
pub(crate) fn pad_md_length_block(block: &mut [u8], length: &[u8]) {
    let length_start = block.len() - length.len();
    block[..length_start].fill(0);
    block[length_start..].copy_from_slice(length);
}

/// Holds the bytes of a partial block between `update` calls.
///
/// `N` is the capacity; the block length in use is passed per call and never
/// exceeds it (SHA-3's rate varies with the digest length).
#[derive(Clone)]
pub(crate) struct BlockBuffer<const N: usize> {
    bytes: SecretArray<u8, N>,
    filled: usize,
}

impl<const N: usize> BlockBuffer<N> {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: SecretArray::new([0; N]),
            filled: 0,
        }
    }

    /// Absorb `data`, handing `blocks` every run of whole `block_len`-byte
    /// blocks (as one slice whose length is a multiple of `block_len`) in
    /// order, and keeping the remainder.
    #[inline]
    pub(crate) fn absorb(
        &mut self,
        block_len: usize,
        mut data: &[u8],
        mut blocks: impl FnMut(&[u8]),
    ) {
        debug_assert!(block_len <= N && self.filled < block_len);
        if self.filled > 0 {
            let take = (block_len - self.filled).min(data.len());
            self.bytes[self.filled..self.filled + take].copy_from_slice(&data[..take]);
            self.filled += take;
            data = &data[take..];
            if self.filled < block_len {
                return;
            }
            blocks(&self.bytes[..block_len]);
            self.clear();
        }
        let whole = data.len() - data.len() % block_len;
        if whole > 0 {
            blocks(&data[..whole]);
        }
        let rest = &data[whole..];
        self.bytes[..rest.len()].copy_from_slice(rest);
        self.filled = rest.len();
    }

    /// Borrow the incomplete block without consuming or changing the stream.
    pub(crate) fn pending(&self) -> &[u8] {
        &self.bytes[..self.filled]
    }

    /// Append the marker, zero padding and encoded message length for a
    /// Merkle–Damgård hash whose block size equals this buffer's capacity.
    #[inline]
    pub(crate) fn finish_md<const L: usize>(
        &mut self,
        length_field: [u8; L],
        mut blocks: impl FnMut(&[u8]),
    ) {
        if pad_md(&mut self.bytes[..], self.filled, &length_field) {
            blocks(&self.bytes[..]);
            pad_md_length_block(&mut self.bytes[..], &length_field);
        }
        blocks(&self.bytes[..]);
        self.clear();
    }

    /// The buffered bytes followed by the rest of the block, for padding in
    /// place: the whole `block_len`-byte block with the filled length.
    pub(crate) fn block_mut(&mut self, block_len: usize) -> (&mut [u8], usize) {
        let filled = self.filled;
        self.filled = 0;
        (&mut self.bytes[..block_len], filled)
    }

    /// Overwrite the entire owned capacity, including bytes past the length.
    pub(crate) fn clear(&mut self) {
        self.bytes.clear();
        self.filled = 0;
    }

    #[cfg(test)]
    pub(crate) fn assert_cleared(&self) {
        assert_eq!(*self.bytes, [0; N]);
        assert_eq!(self.filled, 0);
    }

    #[cfg(test)]
    pub(crate) fn observe_drop(
        &mut self,
        observer: std::sync::Arc<crate::secret::Observer<u8, N>>,
    ) {
        self.bytes.observe_drop(observer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn buffered_consumption_finish_and_reset_clear_the_full_capacity() {
        let mut buffer = BlockBuffer::<168>::new();
        buffer.absorb(136, &[0xa5; 128], |_| panic!("partial block"));
        assert_eq!(&buffer.bytes[..128], &[0xa5; 128]);
        let seen = Arc::new(AtomicUsize::new(0));
        buffer.bytes.observe_drop(Arc::new({
            let seen = seen.clone();
            move |live| {
                assert_eq!(live, &[0; 168]);
                seen.fetch_add(1, Ordering::SeqCst);
            }
        }));
        let mut cloned = buffer.clone();
        buffer.absorb(136, &[0x5a; 8], |block| {
            assert_eq!(&block[..128], &[0xa5; 128]);
            assert_eq!(&block[128..], &[0x5a; 8]);
        });
        assert_eq!(*buffer.bytes, [0; 168]);
        assert_eq!(buffer.filled, 0);
        assert_eq!(&cloned.bytes[..128], &[0xa5; 128]);
        cloned.clear();
        assert_eq!(*cloned.bytes, [0; 168]);
        assert_eq!(cloned.filled, 0);
        drop(buffer);
        drop(cloned);
        assert_eq!(seen.load(Ordering::SeqCst), 2);
        let mut md = BlockBuffer::<64>::new();
        md.absorb(64, &[0x7f; 63], |_| panic!("partial block"));
        let mut blocks = 0;
        md.finish_md([0x33; 8], |_| blocks += 1);
        assert_eq!(blocks, 2);
        assert_eq!(*md.bytes, [0; 64]);
        assert_eq!(md.filled, 0);
    }
}
