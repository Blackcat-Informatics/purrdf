#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! RFC 8878 Zstandard decoder.
//!
//! Three entry points are exposed, each with progressively lower-level
//! control:
//!
//! * [`StreamingDecoder`] — implements [`crate::io::Read`] over a compressed
//!   byte stream, transparently parsing the frame header and concatenated
//!   frames. The typical choice for application code.
//! * [`FrameDecoder`] — single-frame interface; use when the caller manages
//!   the input buffer manually (zero-copy slices, network framing, etc).
//! * [`DictionaryHandle`] — pre-parsed dictionary handle. Parse the
//!   dictionary bytes once with [`DictionaryHandle::decode_dict`] and reuse
//!   the handle across every subsequent decode; saves the per-frame
//!   dictionary parse cost when the same dictionary is used many times in a
//!   row.
//!
//! Both decoders expose dictionary-aware constructors / methods,
//! though the exact naming differs:
//!
//! * [`StreamingDecoder::new_with_dictionary_handle`] /
//!   [`StreamingDecoder::new_with_dictionary_bytes`]
//! * [`FrameDecoder::decode_all_with_dict_handle`] /
//!   [`FrameDecoder::decode_all_with_dict_bytes`]
//!
//! The `_handle` variants reuse a previously parsed
//! [`DictionaryHandle`]; the `_bytes` variants parse the dictionary
//! per call (suitable for one-off decodes).
//!
//! Errors surface through [`errors::FrameDecoderError`] and the per-decoder
//! error types in the [`errors`] submodule.

pub mod errors;
mod frame_decoder;
mod streaming_decoder;

pub use dictionary::{Dictionary, DictionaryHandle};
pub use frame_decoder::{BlockDecodingStrategy, ContentChecksum, FrameDecoder};

pub use streaming_decoder::StreamingDecoder;

/// Decompressed size a frame declares in its header, as read by
/// [`read_frame_content_size`] without decoding the frame body.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FrameContentSize {
    /// The header carried an explicit `Frame_Content_Size` field (in bytes).
    Known(u64),
    /// The header did not declare a content size; the true size is only
    /// known after decoding (or from out-of-band knowledge).
    Unknown,
}

/// Read the decompressed size a frame declares in its header, without
/// decoding the frame body.
///
/// Parses only the leading frame header of `src`. Returns
/// [`FrameContentSize::Known`] when the header carries an explicit
/// `Frame_Content_Size`, or [`FrameContentSize::Unknown`] when it does not.
/// This backs the C `ZSTD_getFrameContentSize` entry point, where the two
/// variants map to a concrete size and `ZSTD_CONTENTSIZE_UNKNOWN`.
///
/// # Errors
/// Returns [`ReadFrameHeaderError`](errors::ReadFrameHeaderError) when `src`
/// is too short to hold a header, carries a bad magic number, or begins with
/// a skippable frame.
///
/// ```rust
/// use purrdf_zstd::encoding::{compress_slice_to_vec, CompressionLevel};
/// use purrdf_zstd::decoding::{read_frame_content_size, FrameContentSize};
/// let frame = compress_slice_to_vec(&[42u8; 100], CompressionLevel::Default);
/// assert_eq!(read_frame_content_size(&frame).unwrap(), FrameContentSize::Known(100));
/// ```
pub fn read_frame_content_size(
    src: &[u8],
) -> Result<FrameContentSize, errors::ReadFrameHeaderError> {
    let (header, _consumed) = frame::read_frame_header_with_format(src, false)?;
    Ok(if header.fcs_declared() {
        FrameContentSize::Known(header.frame_content_size())
    } else {
        FrameContentSize::Unknown
    })
}

/// Error from [`find_frame_compressed_size`].
#[derive(Debug)]
pub enum FrameSizeError {
    /// The frame header could not be parsed.
    Header(errors::ReadFrameHeaderError),
    /// The buffer ends before the frame's blocks (or trailing checksum) are
    /// complete.
    Truncated,
    /// A block declared the reserved block type, which is invalid per RFC 8878.
    ReservedBlock,
    /// A block declared a `Block_Size` larger than the frame's
    /// `Block_Maximum_Size` (`min(Window_Size, 128 KiB)`), which is invalid per
    /// RFC 8878 §3.1.1.2. Accepting it would let a corrupt frame pass a size
    /// query and make the no-`Frame_Content_Size` decompressed-bound
    /// under-count (each block can regenerate at most `Block_Maximum_Size`).
    OversizedBlock,
}

/// On-disk byte length of the FIRST frame in `src` — magic number, frame
/// header, every block, and the trailing content checksum when present —
/// computed by walking the block headers without decoding any block body.
///
/// For a skippable frame, returns its full `8 + Frame_Size` length. This backs
/// the C `ZSTD_findFrameCompressedSize` entry point; the returned value is the
/// offset at which a following concatenated frame would begin.
///
/// # Errors
/// [`FrameSizeError`] when the header is unreadable, the buffer is truncated
/// mid-frame, or a block uses the reserved type.
///
/// ```rust
/// use purrdf_zstd::encoding::{compress_slice_to_vec, CompressionLevel};
/// use purrdf_zstd::decoding::find_frame_compressed_size;
/// let frame = compress_slice_to_vec(&[5u8; 256], CompressionLevel::Default);
/// assert_eq!(find_frame_compressed_size(&frame).unwrap(), frame.len());
/// ```
pub fn find_frame_compressed_size(src: &[u8]) -> Result<usize, FrameSizeError> {
    let (header, header_len) = match frame::read_frame_header_with_format(src, false) {
        Ok(parsed) => parsed,
        // Skippable frame: magic (4) + Frame_Size field (4) + payload.
        Err(errors::ReadFrameHeaderError::SkipFrame { length, .. }) => {
            return 8usize
                .checked_add(length as usize)
                .filter(|end| *end <= src.len())
                .ok_or(FrameSizeError::Truncated);
        }
        Err(e) => return Err(FrameSizeError::Header(e)),
    };

    let walk = walk_blocks(src, header_len as usize, frame_block_size_max(&header))?;
    if header.descriptor.content_checksum_flag() {
        walk.end
            .checked_add(4)
            .filter(|end| *end <= src.len())
            .ok_or(FrameSizeError::Truncated)
    } else {
        Ok(walk.end)
    }
}

/// Result of walking the block sequence of one frame (between the header and
/// the optional trailing checksum).
struct BlockWalk {
    /// Offset just past the last block (before any content checksum).
    end: usize,
    /// Number of blocks in the frame.
    count: u64,
}

/// `Block_Maximum_Size` for the frame: `min(Window_Size, 128 KiB)`. Per RFC
/// 8878 §3.1.1.2 every block's `Block_Size` is bounded by this, and each block
/// regenerates at most this many bytes. Single-segment frames omit the
/// `Window_Descriptor`; their window equals the declared content size.
fn frame_block_size_max(header: &frame::FrameHeader) -> usize {
    let window_size = match header.window_descriptor() {
        Some(desc) => {
            let exponent = u64::from(desc >> 3);
            let mantissa = u64::from(desc & 0x7);
            let window_base = 1u64 << (10 + exponent);
            window_base + (window_base / 8) * mantissa
        }
        None => header.frame_content_size(),
    };
    // The 128 KiB cap keeps the result within usize on every target.
    window_size.min(128 * 1024) as usize
}

/// Walk the block headers of a single frame starting at `start` (the offset of
/// the first block header), validating each fits in `src` and declares a
/// `Block_Size` no larger than `max_block_size` (the frame's
/// `Block_Maximum_Size`). Does not consume the trailing content checksum.
/// Shared by [`find_frame_compressed_size`] and [`frame_decompressed_bound`] so
/// the on-disk-size and block-count views never diverge.
fn walk_blocks(
    src: &[u8],
    start: usize,
    max_block_size: usize,
) -> Result<BlockWalk, FrameSizeError> {
    let mut offset = start;
    let mut count = 0u64;
    loop {
        // 3-byte block header (RFC 8878 §3.1.1.2): bit0 last-block flag,
        // bits1-2 block type, bits3-23 Block_Size.
        let hdr = src
            .get(offset..offset + 3)
            .ok_or(FrameSizeError::Truncated)?;
        let raw = u32::from(hdr[0]) | (u32::from(hdr[1]) << 8) | (u32::from(hdr[2]) << 16);
        let last_block = (raw & 1) != 0;
        let block_type = (raw >> 1) & 0b11;
        let block_size = (raw >> 3) as usize;
        // On-disk bytes following the header: RLE stores a single byte
        // regardless of the run length; Raw/Compressed store Block_Size bytes;
        // the reserved type is invalid.
        let on_disk = match block_type {
            1 => 1,              // RLE
            0 | 2 => block_size, // Raw / Compressed
            _ => return Err(FrameSizeError::ReservedBlock),
        };
        // RFC 8878 §3.1.1.2: Block_Size MUST NOT exceed Block_Maximum_Size for
        // any block type (it bounds both the on-disk Raw/Compressed payload and
        // the RLE/Raw regenerated size). Reject rather than accept a corrupt
        // declaration that would otherwise pass the size query and let the
        // no-FCS bound under-count.
        if block_size > max_block_size {
            return Err(FrameSizeError::OversizedBlock);
        }
        offset = offset
            .checked_add(3 + on_disk)
            .filter(|end| *end <= src.len())
            .ok_or(FrameSizeError::Truncated)?;
        count += 1;
        if last_block {
            break;
        }
    }
    Ok(BlockWalk { end: offset, count })
}

/// Upper bound on the decompressed size of the FIRST frame in `src`, without
/// decoding the body. Backs the C `ZSTD_decompressBound` (per-frame term).
///
/// Returns the exact size when the header declares `Frame_Content_Size`;
/// otherwise a valid (loose) bound of `block_count * block_size_max`, where
/// `block_size_max = min(window_size, 128 KiB)` — every block decompresses to
/// at most that many bytes. Skippable frames contribute `0`.
///
/// # Errors
/// [`FrameSizeError`] on an unreadable header, truncation, or a reserved block.
pub fn frame_decompressed_bound(src: &[u8]) -> Result<u64, FrameSizeError> {
    let (header, header_len) = match frame::read_frame_header_with_format(src, false) {
        Ok(parsed) => parsed,
        // Skippable frame contributes 0, but its full payload must be present:
        // truncation is an error per this function's contract.
        Err(errors::ReadFrameHeaderError::SkipFrame { length, .. }) => {
            return 8usize
                .checked_add(length as usize)
                .filter(|end| *end <= src.len())
                .map(|_| 0)
                .ok_or(FrameSizeError::Truncated);
        }
        Err(e) => return Err(FrameSizeError::Header(e)),
    };

    // Walk the blocks (and the optional checksum trailer) so a truncated frame
    // is rejected even when Frame_Content_Size is declared — without this the
    // declared-FCS path would return a bound for an incomplete buffer. The
    // per-frame block maximum both bounds the walk and scales the no-FCS bound.
    let block_size_max = frame_block_size_max(&header);
    let walk = walk_blocks(src, header_len as usize, block_size_max)?;
    if header.descriptor.content_checksum_flag() {
        walk.end
            .checked_add(4)
            .filter(|end| *end <= src.len())
            .ok_or(FrameSizeError::Truncated)?;
    }

    if header.fcs_declared() {
        return Ok(header.frame_content_size());
    }
    // Saturating is intentional here: this is an UPPER bound, so capping at the
    // maximum representable value is the correct ceiling for a pathologically
    // large frame, not a masked arithmetic bug. Each of `walk.count` blocks
    // regenerates at most `block_size_max` bytes (now enforced by `walk_blocks`,
    // so the bound can no longer be undercut by an oversized block header).
    Ok(walk.count.saturating_mul(block_size_max as u64))
}

/// Frame header fields decoded by [`read_frame_header_info`], mirroring the
/// values the C `ZSTD_getFrameHeader` fills into a `ZSTD_FrameHeader`.
#[derive(Copy, Clone, Debug)]
pub struct FrameHeaderInfo {
    /// Declared decompressed size, or [`FrameContentSize::Unknown`] when the
    /// header omits the `Frame_Content_Size` field.
    pub content_size: FrameContentSize,
    /// Decoder window size in bytes (the minimum buffer needed to decode the
    /// frame). For single-segment frames this equals the content size.
    pub window_size: u64,
    /// Dictionary id required to decode the frame, if the header carries one.
    pub dictionary_id: Option<u32>,
    /// Whether a 32-bit content checksum trails the frame.
    pub content_checksum: bool,
    /// Header length in bytes, measured in the parsed input format: it includes
    /// the 4-byte magic number in the default format, but excludes it when
    /// parsed as magicless (`read_frame_header_info(.., true)`), since those 4
    /// bytes are not present on the wire in that mode.
    pub header_size: usize,
}

/// Length in bytes of the frame header at the start of `src`, including the
/// 4-byte magic number (the offset at which the first block begins). Backs the
/// C `ZSTD_frameHeaderSize`.
///
/// # Errors
/// [`ReadFrameHeaderError`](errors::ReadFrameHeaderError) when the header is
/// too short, has a bad magic number, or is a skippable frame.
pub fn frame_header_size(src: &[u8]) -> Result<usize, errors::ReadFrameHeaderError> {
    let (_header, consumed) = frame::read_frame_header_with_format(src, false)?;
    Ok(consumed as usize)
}

/// Decode the leading frame header fields of `src` without decoding the body.
///
/// Backs the C `ZSTD_getFrameHeader`. When `magicless` is `true` the 4-byte
/// magic prefix is assumed absent (the `ZSTD_f_zstd1_magicless` format); the
/// caller must know out-of-band that the stream is magicless. The reported
/// [`FrameHeaderInfo::window_size`] is the raw value derived from the header
/// (no maximum-window policy applied here; that bound is enforced at decode
/// time), so callers see the frame's own declared window even when it exceeds
/// a decoder limit.
///
/// # Errors
/// As [`read_frame_content_size`].
///
/// ```rust
/// use purrdf_zstd::encoding::{compress_slice_to_vec, CompressionLevel};
/// use purrdf_zstd::decoding::{read_frame_header_info, FrameContentSize};
/// let frame = compress_slice_to_vec(&[7u8; 512], CompressionLevel::Default);
/// let info = read_frame_header_info(&frame, false).unwrap();
/// assert_eq!(info.content_size, FrameContentSize::Known(512));
/// assert!(info.window_size >= 512);
/// ```
pub fn read_frame_header_info(
    src: &[u8],
    magicless: bool,
) -> Result<FrameHeaderInfo, errors::ReadFrameHeaderError> {
    let (header, consumed) = frame::read_frame_header_with_format(src, magicless)?;
    let content_size = if header.fcs_declared() {
        FrameContentSize::Known(header.frame_content_size())
    } else {
        FrameContentSize::Unknown
    };
    // Compute the window size without the decode-time maximum-window check
    // (RFC 8878 §3.1.1.1.2). `window_descriptor()` returns `None` for a
    // single-segment frame, where the window equals the content size.
    let window_size = match header.window_descriptor() {
        Some(desc) => {
            let exponent = u64::from(desc >> 3);
            let mantissa = u64::from(desc & 0x7);
            let window_base = 1u64 << (10 + exponent);
            window_base + (window_base / 8) * mantissa
        }
        None => header.frame_content_size(),
    };
    Ok(FrameHeaderInfo {
        content_size,
        window_size,
        dictionary_id: header.dictionary_id(),
        content_checksum: header.descriptor.content_checksum_flag(),
        header_size: consumed as usize,
    })
}

pub(crate) mod block_decoder;
pub(crate) mod buffer_backend;
pub(crate) mod decode_buffer;
pub(crate) mod dictionary;
pub(crate) mod exec_sequence_inline;
// FlatBuf is the compile-time-monomorphised "frame fits in window"
// backend selected via `DecodeBuffer<FlatBuf>`. `FrameDecoder`'s
// `DecoderScratchKind` picks it when the frame header has
// `Single_Segment_flag` set; the ring backend remains the default
// for multi-segment frames. See backlog item #132 for the wiring
// rationale.
pub(crate) mod flat_buf;
pub(crate) mod frame;
pub(crate) mod literals_section_decoder;
pub(crate) mod prefetch;
mod ringbuffer;
#[allow(dead_code)]
pub(crate) mod scratch;
// Per-kernel monolithic sequence-section decoder entry points. Each
// kernel has its own self-contained function with the full pipeline
// (outer init, both arms, decode_one, execute_one) inlined inside one
// `#[target_feature]`-scoped body. The dispatcher in
// `sequence_section_decoder::decode_and_execute_sequences` selects the
// kernel ONCE per call via cached `detect_cpu_kernel`. aarch64 Neon
// and Sve still go through the K-generic
// `decode_and_execute_sequences_impl` shared body until their own
// monoliths land.
//
// The shared helpers (`decode_and_execute_sequences_impl`,
// `run_pipelined_sequence_loop`, `decode_one_sequence_inline`, the
// `execute_one_sequence_pipelined*` wrappers) live on aarch64
// (Neon/Sve dispatch arms in `decode_and_execute_sequences`) and in
// tests, but are orphan on x86_64 production builds where the
// per-kernel monoliths bypass them entirely. Each carries
// `#[allow(dead_code)]` so the `-D warnings` clippy gate stays green
// on x86_64 without losing the cross-arch reuse. The vestigial
// `_bmi2`/`_avx2`/`_vbmi2` variants are pre-R12 macro-dispatch
// helpers with no remaining callers; they should be cleaned up in
// a follow-up PR once the per-kernel monolithic shape is fully
// settled.
#[cfg(target_arch = "x86_64")]
pub(crate) mod seq_decoder_avx2;
#[cfg(target_arch = "x86_64")]
pub(crate) mod seq_decoder_bmi2;
pub(crate) mod seq_decoder_scalar;

pub(crate) mod sequence_execution;
pub(crate) mod sequence_section_decoder;
pub(crate) mod simd_copy;

// `UserSliceBackend` is the compile-time-monomorphised backend that
// writes directly into the caller's `&mut [u8]` output slice, used
// by the `FrameDecoder::decode_all` direct-decode path. It
// eliminates the `FlatBuf` drain copy + anonymous-page-fault cost
// on large literal sections. Wiring happens via
// `DecodeBuffer<UserSliceBackend<'a>>`; the lifetime binds the
// backend to the caller's slice for the call duration.
pub(crate) mod user_slice_buf;
