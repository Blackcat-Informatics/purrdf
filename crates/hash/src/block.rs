// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Block buffering shared by the block-oriented hashers.

/// Holds the bytes of a partial block between `update` calls.
///
/// `N` is the capacity; the block length in use is passed per call and never
/// exceeds it (SHA-3's rate varies with the digest length).
#[derive(Clone)]
pub(crate) struct BlockBuffer<const N: usize> {
    bytes: [u8; N],
    filled: usize,
}

impl<const N: usize> BlockBuffer<N> {
    pub(crate) const fn new() -> Self {
        Self {
            bytes: [0; N],
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
            self.filled = 0;
        }
        let whole = data.len() - data.len() % block_len;
        if whole > 0 {
            blocks(&data[..whole]);
        }
        let rest = &data[whole..];
        self.bytes[..rest.len()].copy_from_slice(rest);
        self.filled = rest.len();
    }

    /// The Merkle–Damgård finish shared by MD5 and SHA-1 (64-byte blocks):
    /// append `0x80`, zeros up to 56 mod 64, then the 8-byte encoded message
    /// bit length, handing the one or two final blocks to `blocks`.
    pub(crate) fn finish_md(&mut self, length_field: [u8; 8], mut blocks: impl FnMut(&[u8])) {
        debug_assert!(N >= 64 && self.filled < 64);
        let filled = self.filled;
        self.bytes[filled] = 0x80;
        self.bytes[filled + 1..64].fill(0);
        if filled >= 56 {
            blocks(&self.bytes[..64]);
            self.bytes[..56].fill(0);
        }
        self.bytes[56..64].copy_from_slice(&length_field);
        blocks(&self.bytes[..64]);
        self.filled = 0;
    }

    /// The buffered bytes followed by the rest of the block, for padding in
    /// place: the whole `block_len`-byte block with the filled length.
    pub(crate) fn block_mut(&mut self, block_len: usize) -> (&mut [u8], usize) {
        let filled = self.filled;
        self.filled = 0;
        (&mut self.bytes[..block_len], filled)
    }

    /// Forget the buffered bytes.
    pub(crate) fn clear(&mut self) {
        self.filled = 0;
    }
}
