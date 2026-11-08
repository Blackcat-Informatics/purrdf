#![allow(
    clippy::cast_lossless,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use crate::blocks::block::BlockType;
use alloc::vec::Vec;

#[derive(Debug)]
pub(crate) struct BlockHeader {
    /// Signals if this block is the last one.
    /// The frame will end after this block.
    pub last_block: bool,
    /// Influences the meaning of `block_size`.
    pub block_type: BlockType,
    /// - For `Raw` blocks, this is the size of the block's
    ///   content in bytes.
    /// - For `RLE` blocks, there will be a single byte follwing
    ///   the header, repeated `block_size` times.
    /// - For `Compressed` blocks, this is the length of
    ///   the compressed data.
    ///
    /// **This value must not be greater than 21 bits in length.**
    pub block_size: u32,
}

impl BlockHeader {
    /// Write encoded binary representation of this header into the provided buffer.
    pub(crate) fn serialize(self, output: &mut Vec<u8>) {
        vprintln!("Serializing block with the header: {self:?}");
        output.extend_from_slice(&self.to_le_bytes());
    }

    /// The fixed 3-byte little-endian block header. Block headers are always
    /// exactly 3 bytes (RFC 8878 §3.1.1.2), so callers that reserved space
    /// up front can backfill the header in place via this array without a
    /// `Vec` round-trip (the compress-into-output emit path relies on this).
    #[inline]
    pub(crate) fn to_le_bytes(&self) -> [u8; 3] {
        let encoded_block_type = match self.block_type {
            BlockType::Raw => 0u32,
            BlockType::RLE => 1,
            BlockType::Compressed => 2,
            BlockType::Reserved => panic!("You cannot use a reserved block type"),
        };
        let mut block_header = self.block_size << 3;
        block_header |= encoded_block_type << 1;
        block_header |= self.last_block as u32;
        let le = block_header.to_le_bytes();
        [le[0], le[1], le[2]]
    }
}
