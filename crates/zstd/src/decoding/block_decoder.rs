#![allow(
    clippy::equatable_if_let,
    clippy::manual_assert_eq,
    clippy::needless_pass_by_ref_mut,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use super::super::blocks::block::BlockHeader;
use super::super::blocks::block::BlockType;
use super::super::blocks::literals_section::LiteralsSection;
use super::super::blocks::literals_section::LiteralsSectionType;
use super::super::blocks::sequence_section::SequencesHeader;
use super::literals_section_decoder::{LiteralsView, decode_literals_zerocopy};
use super::sequence_section_decoder::decode_and_execute_sequences;
use crate::common::MAX_BLOCK_SIZE;
use crate::decoding::errors::DecodeSequenceError;
use crate::decoding::errors::{
    BlockHeaderReadError, BlockSizeError, BlockTypeError, DecodeBlockContentError,
    DecompressBlockError,
};
use crate::decoding::scratch::Workspace;
use crate::io::Read;

pub(crate) struct BlockDecoder {
    header_buffer: [u8; 3],
    internal_state: DecoderState,
}

enum DecoderState {
    ReadyToDecodeNextHeader,
    ReadyToDecodeNextBody,
    #[allow(dead_code)]
    Failed, //TODO put "self.internal_state = DecoderState::Failed;" everywhere an unresolvable error occurs
}

/// Create a new [BlockDecoder].
pub(crate) fn new() -> BlockDecoder {
    BlockDecoder {
        internal_state: DecoderState::ReadyToDecodeNextHeader,
        header_buffer: [0u8; 3],
    }
}

impl BlockDecoder {
    /// Decode the body of a single block described by `header` from `source` into `workspace`.
    ///
    /// Returns the number of bytes consumed from `source`.
    /// The decode buffer inside `workspace` may be reserved or grown during
    /// decoding. For some block types the decompressed size is known up front,
    /// but this is not guaranteed before any data is written.
    /// Slice-source fast path for `decode_block_content`. Consumes
    /// the right number of bytes from `*source` (advancing the slice)
    /// without going through the persistent `block_content_buffer`.
    /// Used by `FrameDecoder::decode_all` where `source` is
    /// already a `&[u8]` view into the user's input.
    ///
    /// Returns the number of bytes consumed from `*source`.
    pub(crate) fn decode_block_content_from_slice<W: Workspace>(
        &mut self,
        header: &BlockHeader,
        workspace: &mut W,
        dict: Option<&crate::decoding::dictionary::Dictionary>,
        source: &mut &[u8],
    ) -> Result<u64, DecodeBlockContentError> {
        use DecoderState as State;
        match self.internal_state {
            State::ReadyToDecodeNextBody => { /* ok */ }
            State::Failed => return Err(DecodeBlockContentError::DecoderStateIsFailed),
            State::ReadyToDecodeNextHeader => {
                return Err(DecodeBlockContentError::ExpectedHeaderOfPreviousBlock);
            }
        }

        let block_type = header.block_type;
        match block_type {
            BlockType::RLE => {
                // 1 byte from source via slice; no Read overhead.
                if source.is_empty() {
                    // ErrorKind::UnexpectedEof matches what the streaming path
                    // gets from Read::read_exact on truncated input — callers
                    // and tests can detect truncation by kind alone, identical
                    // across slice-source and Read-source decode entry points.
                    return Err(DecodeBlockContentError::ReadError {
                        step: block_type,
                        source: crate::io::Error::from(crate::io::ErrorKind::UnexpectedEof),
                    });
                }
                // Peek the fill byte without advancing `*source` yet —
                // `try_extend_and_fill` is fallible on fixed-capacity
                // backends, and on `Err` the caller exits early. If we
                // advanced first, the input cursor would diverge from
                // the bytes_read accounting by one byte on the error
                // path. Advance ONLY after the write succeeds, matching
                // the Raw arm's split_at-then-try_push-then-advance shape.
                let fill = source[0];
                workspace
                    .split()
                    .buffer
                    .try_extend_and_fill(fill, header.decompressed_size as usize)
                    .map_err(|_| DecodeBlockContentError::BackendOverflow { step: block_type })?;
                *source = &source[1..];
                self.internal_state = State::ReadyToDecodeNextHeader;
                Ok(1)
            }
            BlockType::Raw => {
                // Raw payload IS the source bytes; push them
                // directly to the buffer. For UserSliceBackend this
                // is a single memcpy from input -> output, no
                // intermediate Vec.
                let n = header.decompressed_size as usize;
                if source.len() < n {
                    return Err(DecodeBlockContentError::ReadError {
                        step: block_type,
                        source: crate::io::Error::from(crate::io::ErrorKind::UnexpectedEof),
                    });
                }
                let (payload, tail) = source.split_at(n);
                // `try_push` returns `Err(BackendOverflow)` on
                // `UserSliceBackend` when the Raw payload would push
                // past the caller's output slice. Growable backends
                // grow on demand and always succeed.
                workspace
                    .split()
                    .buffer
                    .try_push(payload)
                    .map_err(|_| DecodeBlockContentError::BackendOverflow { step: block_type })?;
                *source = tail;
                self.internal_state = State::ReadyToDecodeNextHeader;
                Ok(u64::from(header.decompressed_size))
            }
            BlockType::Reserved => {
                panic!("Reserved-type block should be rejected during header parsing");
            }
            BlockType::Compressed => {
                let n = header.content_size as usize;
                if source.len() < n {
                    return Err(DecodeBlockContentError::ReadError {
                        step: block_type,
                        source: crate::io::Error::from(crate::io::ErrorKind::UnexpectedEof),
                    });
                }
                let (payload, tail) = source.split_at(n);
                self.decompress_block_inplace(header, workspace, dict, payload)?;
                *source = tail;
                self.internal_state = State::ReadyToDecodeNextHeader;
                Ok(u64::from(header.content_size))
            }
        }
    }

    pub(crate) fn decode_block_content<W: Workspace>(
        &mut self,
        header: &BlockHeader,
        workspace: &mut W,
        dict: Option<&crate::decoding::dictionary::Dictionary>,
        mut source: impl Read,
    ) -> Result<u64, DecodeBlockContentError> {
        match self.internal_state {
            DecoderState::ReadyToDecodeNextBody => { /* Happy :) */ }
            DecoderState::Failed => return Err(DecodeBlockContentError::DecoderStateIsFailed),
            DecoderState::ReadyToDecodeNextHeader => {
                return Err(DecodeBlockContentError::ExpectedHeaderOfPreviousBlock);
            }
        }

        let block_type = header.block_type;
        match block_type {
            BlockType::RLE => {
                let mut buf = [0u8; 1];
                source.read_exact(&mut buf[..]).map_err(|err| {
                    DecodeBlockContentError::ReadError {
                        step: block_type,
                        source: err,
                    }
                })?;
                workspace
                    .split()
                    .buffer
                    .extend_and_fill(buf[0], header.decompressed_size as usize);

                self.internal_state = DecoderState::ReadyToDecodeNextHeader;

                Ok(1)
            }
            BlockType::Raw => {
                // Pass `source` by value rather than `&mut source` — it isn't
                // used after this match arm anyway, so moving avoids the
                // borrow-by-reference indirection. (Both io shims provide a
                // blanket `Read for &mut T`, so `&mut source` would also
                // compile; the by-value form is just cleaner here.)
                workspace
                    .split()
                    .buffer
                    .extend_from_reader(source, header.decompressed_size as usize)
                    .map_err(|err| DecodeBlockContentError::ReadError {
                        step: block_type,
                        source: err,
                    })?;

                self.internal_state = DecoderState::ReadyToDecodeNextHeader;
                Ok(u64::from(header.decompressed_size))
            }

            BlockType::Reserved => {
                panic!(
                    "How did you even get this. The decoder should error out if it detects a reserved-type block"
                );
            }

            BlockType::Compressed => {
                self.decompress_block(header, workspace, dict, source)?;

                self.internal_state = DecoderState::ReadyToDecodeNextHeader;
                Ok(u64::from(header.content_size))
            }
        }
    }

    fn decompress_block<W: Workspace>(
        &mut self,
        header: &BlockHeader,
        workspace: &mut W,
        dict: Option<&crate::decoding::dictionary::Dictionary>,
        mut source: impl Read,
    ) -> Result<(), DecompressBlockError> {
        // Streaming-path entry: copy `content_size` bytes from the
        // `Read` source into `block_content_buffer`, then dispatch
        // to the in-place body via per-field borrows. The direct-
        // decode path (`decode_all`) skips this copy by passing
        // a borrowed slice of the input straight to
        // `decompress_block_inplace_with_parts`.
        let parts = workspace.split();
        parts
            .block_content_buffer
            .resize(header.content_size as usize, 0);
        source.read_exact(parts.block_content_buffer.as_mut_slice())?;
        // Disjoint-fields borrow: `as_slice()` reborrows
        // block_content_buffer as `&[u8]`; the other WorkspaceRef
        // fields stay independently movable into the helper since
        // each is a distinct struct field. No `unsafe` needed —
        // Rust's borrow checker tracks per-field disjointness.
        let raw = parts.block_content_buffer.as_slice();
        self.decompress_block_inplace_with_parts(
            header,
            parts.huf,
            parts.fse,
            parts.buffer,
            parts.offset_hist,
            parts.literals_buffer,
            raw,
            dict,
        )
    }

    /// Compressed-block fast path that takes the block content as a
    /// borrowed slice (no per-block memcpy into
    /// `block_content_buffer`).
    ///
    /// Called by:
    /// - `decompress_block` (streaming path) after it has copied the
    ///   compressed bytes from the source `Read` into the persistent
    ///   `block_content_buffer`.
    /// - `decode_block_content_from_slice` (direct-decode path) where
    ///   the source is a `&[u8]` already, so the slice IS the input —
    ///   saving the per-block memcpy + anonymous-page first-touch on
    ///   `block_content_buffer.resize(...)`. At L-1 fast on
    ///   `decodecorpus-z000033` this `block_content_buffer` traffic
    ///   was the largest single contributor to the
    ///   `__memmove_avx_unaligned_erms` + `exc_page_fault` flame
    ///   on the post-#244 baseline (~30% of decode time combined).
    pub(crate) fn decompress_block_inplace<W: Workspace>(
        &mut self,
        header: &BlockHeader,
        workspace: &mut W,
        dict: Option<&crate::decoding::dictionary::Dictionary>,
        raw: &[u8],
    ) -> Result<(), DecompressBlockError> {
        let parts = workspace.split();
        // block_content_buffer is intentionally NOT passed — the
        // in-place body does not need it (raw already IS the block
        // content). Dropping it here keeps the helper signature
        // free of an unused-field reference.
        self.decompress_block_inplace_with_parts(
            header,
            parts.huf,
            parts.fse,
            parts.buffer,
            parts.offset_hist,
            parts.literals_buffer,
            raw,
            dict,
        )
    }

    /// Inner body of [`Self::decompress_block_inplace`] that takes
    /// the scratch fields as disjoint `&mut` references. The split
    /// happens at the call site (so the streaming path can keep its
    /// `block_content_buffer` borrow alive while passing `raw =
    /// block_content_buffer.as_slice()` here without aliasing the
    /// other scratch fields). All `unsafe` from the previous
    /// lifetime-widening trick is gone — disjoint-field borrows
    /// type-check naturally.
    #[allow(clippy::too_many_arguments)]
    fn decompress_block_inplace_with_parts<'d, B: super::buffer_backend::BufferBackend>(
        &mut self,
        header: &BlockHeader,
        huf: &mut crate::decoding::scratch::HuffmanScratch,
        fse: &'d mut crate::decoding::scratch::FSEScratch,
        buffer: &mut crate::decoding::decode_buffer::DecodeBuffer<B>,
        offset_hist: &mut [u32; 3],
        literals_buffer: &mut alloc::vec::Vec<u8>,
        raw: &[u8],
        dict: Option<&'d crate::decoding::dictionary::Dictionary>,
    ) -> Result<(), DecompressBlockError> {
        let mut section = LiteralsSection::new();
        let bytes_in_literals_header = section.parse_from_header(raw)?;
        let raw = &raw[bytes_in_literals_header as usize..];
        vprintln!(
            "Found {} literalssection with regenerated size: {}, and compressed size: {:?}",
            section.ls_type,
            section.regenerated_size,
            section.compressed_size
        );

        let upper_limit_for_literals = match section.compressed_size {
            Some(x) => x as usize,
            None => match section.ls_type {
                LiteralsSectionType::RLE => 1,
                LiteralsSectionType::Raw => section.regenerated_size as usize,
                _ => panic!("Bug in this library"),
            },
        };

        if raw.len() < upper_limit_for_literals {
            return Err(DecompressBlockError::MalformedSectionHeader {
                expected_len: upper_limit_for_literals,
                remaining_bytes: raw.len(),
            });
        }

        let raw_literals = &raw[..upper_limit_for_literals];
        vprintln!("Slice for literals: {}", raw_literals.len());

        literals_buffer.clear(); //all literals of the previous block must have been used in the sequence execution anyways. just be defensive here
        // Zero-copy literals view — for Raw sections this borrows
        // straight into `raw_literals` (no memcpy into the Vec).
        // For RLE / HUF it materialises into `literals_buffer`
        // and `data` is a slice over that buffer. Eliminates a major
        // memcpy contributor (Vec::extend_from_slice → spec_extend)
        // flagged at ~20% of decode time on the L-1 fast c_stream
        // flamegraph.
        let LiteralsView {
            data: literals_view,
            bytes_used: bytes_used_in_literals_section,
        } = decode_literals_zerocopy(&section, huf, dict, raw_literals, literals_buffer)?;
        assert!(
            section.regenerated_size as usize == literals_view.len(),
            "Wrong number of literals: {}, Should have been: {}",
            literals_view.len(),
            section.regenerated_size
        );
        assert!(bytes_used_in_literals_section == upper_limit_for_literals as u32);

        let raw = &raw[upper_limit_for_literals..];
        vprintln!("Slice for sequences with headers: {}", raw.len());

        let mut seq_section = SequencesHeader::new();
        let bytes_in_sequence_header = seq_section.parse_from_header(raw)?;
        let raw = &raw[bytes_in_sequence_header as usize..];
        vprintln!(
            "Found sequencessection with sequences: {} and size: {}",
            seq_section.num_sequences,
            raw.len()
        );

        assert!(
            u32::from(bytes_in_literals_header)
                + bytes_used_in_literals_section
                + u32::from(bytes_in_sequence_header)
                + raw.len() as u32
                == header.content_size
        );
        vprintln!("Slice for sequences: {}", raw.len());

        if seq_section.num_sequences != 0 {
            // Fused decode + execute: avoids the Vec<Sequence> round-trip
            // and inlines the per-iter execute_one_sequence work next to
            // the FSE state advance. RLE-mode axes are handled in the same
            // fused loop via a degenerate single-state FSE table built by
            // `maybe_update_fse_tables`, so there is no separate two-pass
            // path. Pass field-level borrows from the WorkspaceRef so `raw`
            // (immutable view into block_content_buffer) can coexist
            // with the mutable borrows on the FSE / decode-buffer /
            // offset-hist fields.
            decode_and_execute_sequences(
                &seq_section,
                raw,
                fse,
                buffer,
                offset_hist,
                literals_view,
                dict,
            )?;
        } else {
            if !raw.is_empty() {
                return Err(DecompressBlockError::DecodeSequenceError(
                    DecodeSequenceError::ExtraBits {
                        bits_remaining: raw.len() as isize * 8,
                    },
                ));
            }
            buffer.push(literals_view);
        }

        Ok(())
    }

    /// Reads 3 bytes from the provided reader and returns
    /// the deserialized header and the number of bytes read.
    pub(crate) fn read_block_header(
        &mut self,
        mut r: impl Read,
    ) -> Result<(BlockHeader, u8), BlockHeaderReadError> {
        //match self.internal_state {
        //    DecoderState::ReadyToDecodeNextHeader => {/* Happy :) */},
        //    DecoderState::Failed => return Err(format!("Cant decode next block if failed along the way. Results will be nonsense")),
        //    DecoderState::ReadyToDecodeNextBody => return Err(format!("Cant decode next block header, while expecting to decode the body of the previous block. Results will be nonsense")),
        //}

        r.read_exact(&mut self.header_buffer[0..3])?;

        let btype = self.block_type()?;
        if let BlockType::Reserved = btype {
            return Err(BlockHeaderReadError::FoundReservedBlock);
        }

        let block_size = self.block_content_size()?;
        let decompressed_size = match btype {
            BlockType::Raw => block_size,
            BlockType::RLE => block_size,
            BlockType::Reserved => 0, //should be caught above, this is an error state
            BlockType::Compressed => 0, //unknown but will be smaller than 128kb (or window_size if that is smaller than 128kb)
        };
        let content_size = match btype {
            BlockType::Raw => block_size,
            BlockType::Compressed => block_size,
            BlockType::RLE => 1,
            BlockType::Reserved => 0, //should be caught above, this is an error state
        };

        let last_block = self.is_last();

        self.reset_buffer();
        self.internal_state = DecoderState::ReadyToDecodeNextBody;

        //just return 3. Blockheaders always take 3 bytes
        Ok((
            BlockHeader {
                last_block,
                block_type: btype,
                decompressed_size,
                content_size,
            },
            3,
        ))
    }

    fn reset_buffer(&mut self) {
        self.header_buffer[0] = 0;
        self.header_buffer[1] = 0;
        self.header_buffer[2] = 0;
    }

    fn is_last(&self) -> bool {
        self.header_buffer[0] & 0x1 == 1
    }

    fn block_type(&self) -> Result<BlockType, BlockTypeError> {
        let t = (self.header_buffer[0] >> 1) & 0x3;
        match t {
            0 => Ok(BlockType::Raw),
            1 => Ok(BlockType::RLE),
            2 => Ok(BlockType::Compressed),
            3 => Ok(BlockType::Reserved),
            other => Err(BlockTypeError::InvalidBlocktypeNumber { num: other }),
        }
    }

    fn block_content_size(&self) -> Result<u32, BlockSizeError> {
        let val = self.block_content_size_unchecked();
        if val > MAX_BLOCK_SIZE {
            Err(BlockSizeError::BlockSizeTooLarge { size: val })
        } else {
            Ok(val)
        }
    }

    fn block_content_size_unchecked(&self) -> u32 {
        u32::from(self.header_buffer[0] >> 3) //push out type and last_block flags. Retain 5 bit
            | (u32::from(self.header_buffer[1]) << 5)
            | (u32::from(self.header_buffer[2]) << 13)
    }
}
