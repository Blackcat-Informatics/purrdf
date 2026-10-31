#![allow(
    clippy::empty_line_after_doc_comments,
    clippy::ignored_unit_patterns,
    clippy::inconsistent_struct_constructor,
    clippy::manual_let_else,
    clippy::needless_pass_by_value,
    clippy::redundant_closure_for_method_calls,
    clippy::unnecessary_semicolon,
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Framedecoder is the main low-level struct users interact with to decode zstd frames
//!
//! Zstandard compressed data is made of one or more frames. Each frame is independent and can be
//! decompressed independently of other frames. This module contains structures
//! and utilities that can be used to decode a frame.

use super::frame;
use crate::decoding;
use crate::decoding::block_decoder::BlockDecoder;
use crate::decoding::buffer_backend::BufferBackend;
use crate::decoding::dictionary::{Dictionary, DictionaryHandle};
use crate::decoding::errors::{DecodeBlockContentError, FrameDecoderError};
use crate::decoding::flat_buf::FlatBuf;
use crate::decoding::ringbuffer::RingBuffer;
use crate::decoding::scratch::DecoderScratch;
use crate::io::{Error, Read, Write};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::convert::TryInto;

use crate::common::MAXIMUM_ALLOWED_WINDOW_SIZE;

fn block_header_decode_error(
    source: decoding::errors::BlockHeaderReadError,
    _block_index: u32,
    _frame_offset: u32,
) -> FrameDecoderError {
    FrameDecoderError::FailedToReadBlockHeader(source)
}

fn block_body_decode_error(
    source: DecodeBlockContentError,
    _block_index: u32,
    _frame_offset: u32,
    _header: &crate::blocks::block::BlockHeader,
    _header_size: u8,
) -> FrameDecoderError {
    FrameDecoderError::FailedToReadBlockBody(source)
}

/// Low level Zstandard decoder that can be used to decompress frames with fine control over when and how many bytes are decoded.
///
/// This decoder is able to decode frames only partially and gives control
/// over how many bytes/blocks will be decoded at a time (so you don't have to decode a 10GB file into memory all at once).
/// It reads bytes as needed from a provided source and can be read from to collect partial results.
///
/// If you want to just read the whole frame with an `io::Read` without having to deal with manually calling [FrameDecoder::decode_blocks]
/// you can use the provided [crate::decoding::StreamingDecoder] wich wraps this FrameDecoder.
///
/// Workflow is as follows:
/// ```
/// use purrdf_zstd::decoding::BlockDecodingStrategy;
///
/// # #[cfg(all())]
/// use std::io::{Read, Write};
///
/// // no_std environments can use the crate's own Read traits
/// # #[cfg(not(all()))]
/// use purrdf_zstd::io::{Read, Write};
///
/// fn decode_this(mut file: impl Read) {
///     //Create a new decoder
///     let mut frame_dec = purrdf_zstd::decoding::FrameDecoder::new();
///     let mut result = Vec::new();
///
///     // Use reset or init to make the decoder ready to decode the frame from the io::Read
///     frame_dec.reset(&mut file).unwrap();
///
///     // Loop until the frame has been decoded completely
///     while !frame_dec.is_finished() {
///         // decode (roughly) batch_size many bytes
///         frame_dec.decode_blocks(&mut file, BlockDecodingStrategy::UptoBytes(1024)).unwrap();
///
///         // read from the decoder to collect bytes from the internal buffer
///         let bytes_read = frame_dec.read(result.as_mut_slice()).unwrap();
///
///         // then do something with it
///         do_something(&result[0..bytes_read]);
///     }
///
///     // handle the last chunk of data
///     while frame_dec.can_collect() > 0 {
///         let x = frame_dec.read(result.as_mut_slice()).unwrap();
///
///         do_something(&result[0..x]);
///     }
/// }
///
/// fn do_something(data: &[u8]) {
/// # #[cfg(all())]
///     std::io::stdout().write_all(data).unwrap();
/// }
/// ```
pub struct FrameDecoder {
    state: Option<FrameDecoderState>,

    // Registered dictionaries are stored by shared handle (Arc/Rc) so a
    // single content copy is referenced by every frame the decoder decodes
    // (upstream zstd `ZSTD_refDDict`), rather than re-copied into the decode buffer
    // per frame. `add_dict` wraps an owned `Dictionary` into a handle.
    owned_dicts: BTreeMap<u32, DictionaryHandle>,
    #[cfg(target_has_atomic = "ptr")]
    shared_dicts: BTreeMap<u32, DictionaryHandle>,
    #[cfg(not(target_has_atomic = "ptr"))]
    shared_dicts: (),
    /// `ZSTD_f_zstd1_magicless` — when true, [`init`](Self::init) / [`reset`](Self::reset)
    /// expect frames without the 4-byte magic number prefix.
    /// Default false (standard zstd format).
    magicless: bool,
    /// How the optional content checksum is handled. Default
    /// [`ContentChecksum::EmitOnly`] (compute + expose, no error on
    /// mismatch). Set via [`Self::set_content_checksum`].
    content_checksum: ContentChecksum,
}

/// How the decoder treats a frame's optional XXH64 content checksum
/// (RFC 8878 Content_Checksum_flag). The XXH64 pass over the decompressed
/// output is a measurable share of decode time, so it is made skippable.
///
/// ```
/// use purrdf_zstd::decoding::{ContentChecksum, FrameDecoder};
/// let mut decoder = FrameDecoder::new();
/// decoder.set_content_checksum(ContentChecksum::Verify);
/// ```
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum ContentChecksum {
    /// Skip the XXH64 pass entirely: no compute, no verify.
    /// `get_calculated_checksum()` returns `None`.
    None,
    /// Compute the checksum and expose it via the accessors, but do not
    /// error on a mismatch. This is the default and matches the historical
    /// behaviour (callers verify manually if they wish).
    #[default]
    EmitOnly,
    /// Compute the checksum and compare it against the frame's stored value;
    /// a disagreement fails the decode with
    /// [`FrameDecoderError::ChecksumMismatch`](crate::decoding::errors::FrameDecoderError::ChecksumMismatch).
    /// Without the `hash` feature there is no way to compute a digest, so
    /// `Verify` cannot detect a mismatch and behaves like `None`.
    Verify,
}

// Manual Debug: the entropy tables are large internal scratch with no useful
// Debug surface; only the resume coordinates are worth printing (and this lets
// `PartialDecode` keep its derived Debug).

/// Backend-tagged decode scratch — chosen at frame-reset time based
/// on the parsed `FrameHeader.descriptor.single_segment_flag()` and
/// kept stable through the lifetime of the frame. The match in each
/// helper below dispatches **once per call** (e.g. once per block in
/// `decode_block_content`, once per drain in `drain_to_writer`) —
/// never inside the hot push/repeat loop, which is fully
/// monomorphised through the `DecoderScratch<B>` generic.
enum DecoderScratchKind {
    Ring(DecoderScratch<RingBuffer>),
    Flat(DecoderScratch<FlatBuf>),
}

impl DecoderScratchKind {
    fn new_ring(window_size: usize) -> Self {
        // Lazy ring-buffer allocation: do NOT `reserve(window_size)` here.
        // The direct-decode path (`run_direct_decode`) writes through
        // `UserSliceBackend` and never touches the ring; allocating it
        // eagerly wastes one full window of peak memory on the common
        // direct-eligible frame. On the non-direct path the window is
        // pre-reserved once at frame entry (`decode_all_impl` and
        // `decode_blocks` both call `DecoderScratchKind::reserve_buffer`
        // before any block writes), so multi-block frames pay one
        // amortised grow instead of repeated `reserve_amortized` steps
        // per block.
        let s = DecoderScratch::<RingBuffer>::new(window_size);
        Self::Ring(s)
    }

    /// Construct a flat-backed scratch for a single-segment frame.
    /// `frame_content_size` is the upcoming output size in bytes
    /// (== `window_size` when the flag is set).
    ///
    /// Lazy buffer allocation (mirrors [`Self::new_ring`]): do NOT
    /// pre-size the `FlatBuf`. The direct-decode path
    /// (`run_direct_decode`) writes through `UserSliceBackend` and never
    /// touches this buffer, so eagerly allocating a full FCS wastes one
    /// whole content-size of peak memory on the common direct-eligible
    /// single-segment frame. The non-direct fallback reserves it once via
    /// `reserve_buffer(window_size)` at frame entry before any block
    /// write (`FlatBuf::reserve` adds the `WILDCOPY_OVERLENGTH` slack),
    /// and every inline-exec site (trait method and per-kernel macros)
    /// now carries a tight-tail bounded copy, so a tight buffer can never
    /// overshoot regardless of construction-time slack.
    fn new_flat(frame_content_size: usize) -> Self {
        let s = DecoderScratch::<FlatBuf>::new(frame_content_size);
        Self::Flat(s)
    }

    /// Reset (or transition between) backends for a new frame.
    /// Reuses the existing `DecoderScratch` allocations (FSE / HUF
    /// tables, sequence vec, etc.) when the backend kind is unchanged
    /// — only the underlying buffer is re-sized for the new frame.
    /// Building a fresh `DecoderScratch` on every frame would
    /// re-allocate everything and was measured at +255 % vs ring on
    /// small frames; reusing it keeps the small-frame cost flat.
    fn reset(&mut self, frame: &frame::FrameHeader, window_size: usize) {
        if frame.descriptor.single_segment_flag() {
            match self {
                Self::Flat(s) => {
                    s.reset(window_size);
                    // `DecoderScratch::reset` clears the backing buffer and
                    // updates `window_size` WITHOUT reserving it (it may still
                    // resize the per-block scratch Vecs up to
                    // `min(window_size, MAX_BLOCK_SIZE)`). Backing-buffer
                    // capacity is decided one layer up: direct-eligible frames
                    // never touch it, and the non-direct path pre-reserves once
                    // via `reserve_buffer(window_size)` at frame entry.
                }
                Self::Ring(_) => *self = Self::new_flat(window_size),
            }
        } else {
            match self {
                Self::Ring(s) => s.reset(window_size),
                Self::Flat(_) => *self = Self::new_ring(window_size),
            }
        }
    }

    fn init_from_dict(&mut self, dict: &DictionaryHandle) {
        match self {
            Self::Ring(s) => s.init_from_dict(dict),
            Self::Flat(s) => s.init_from_dict(dict),
        }
    }

    #[inline]
    fn buffer_len(&self) -> usize {
        match self {
            Self::Ring(s) => s.buffer.len(),
            Self::Flat(s) => s.buffer.len(),
        }
    }

    fn workspace_bytes(&self) -> usize {
        match self {
            Self::Ring(s) => s.workspace_bytes(),
            Self::Flat(s) => s.workspace_bytes(),
        }
    }

    /// Pre-reserve the backing buffer to `window_size` in a single
    /// allocation. Called once on the non-direct (`decode_blocks`) path
    /// after direct-eligibility is ruled out, so multi-segment fallback
    /// decodes don't pay repeated `reserve_amortized` grow steps
    /// (128 KiB → 256 KiB → ... → window) as blocks accumulate.
    ///
    /// Direct-eligible frames never call this and pay zero backing-buffer
    /// allocation for the window, on BOTH backends: `new_ring` and
    /// `new_flat` are each lazy (no pre-reserve), so a direct-eligible
    /// frame writes only through `UserSliceBackend` and leaves this
    /// buffer empty.
    ///
    /// `window_size` is the TARGET visible-window capacity: callers pass
    /// the full window, and the method itself computes the shortfall past
    /// the bytes already buffered before calling the backend's
    /// ADDITIONAL-semantics `reserve_exact`. That keeps re-entries (the
    /// decode_all fallback loop runs `decode_blocks` once per strategy
    /// chunk, and streaming callers invoke it per call) from growing a
    /// window-full buffer toward 2x window, while per-block growth keeps
    /// the amortized `reserve`.
    #[inline]
    fn reserve_buffer(&mut self, window_size: usize) {
        // Exact growth: this is the one-shot pre-reservation, and a request
        // landing one slack past the retained capacity (e.g. a dictionary
        // prefix already loaded into the buffer) must not DOUBLE a
        // window-sized allocation through the amortized policy. Per-block
        // growth keeps the amortized `reserve`.
        //
        // `reserve_exact` takes ADDITIONAL capacity, so request only the
        // shortfall past the bytes already buffered: the decode_all
        // fallback loop re-enters `decode_blocks` once per strategy chunk,
        // and re-requesting the full window each iteration would grow a
        // window-sized buffer toward 2x window.
        match self {
            Self::Ring(s) => {
                let additional = window_size.saturating_sub(s.buffer.len());
                s.buffer.reserve_exact(additional);
            }
            Self::Flat(s) => {
                let additional = window_size.saturating_sub(s.buffer.len());
                s.buffer.reserve_exact(additional);
            }
        }
    }

    fn buffer_drain(&mut self) -> Vec<u8> {
        match self {
            Self::Ring(s) => s.buffer.drain(),
            Self::Flat(s) => s.buffer.drain(),
        }
    }

    fn buffer_drain_to_window_size(&mut self) -> Option<Vec<u8>> {
        match self {
            Self::Ring(s) => s.buffer.drain_to_window_size(),
            Self::Flat(s) => s.buffer.drain_to_window_size(),
        }
    }

    fn buffer_drain_to_writer(&mut self, sink: impl Write) -> Result<usize, Error> {
        match self {
            Self::Ring(s) => s.buffer.drain_to_writer(sink),
            Self::Flat(s) => s.buffer.drain_to_writer(sink),
        }
    }

    fn buffer_drain_to_window_size_writer(&mut self, sink: impl Write) -> Result<usize, Error> {
        match self {
            Self::Ring(s) => s.buffer.drain_to_window_size_writer(sink),
            Self::Flat(s) => s.buffer.drain_to_window_size_writer(sink),
        }
    }

    fn buffer_can_drain(&self) -> usize {
        match self {
            Self::Ring(s) => s.buffer.can_drain(),
            Self::Flat(s) => s.buffer.can_drain(),
        }
    }

    fn buffer_can_drain_to_window_size(&self) -> Option<usize> {
        match self {
            Self::Ring(s) => s.buffer.can_drain_to_window_size(),
            Self::Flat(s) => s.buffer.can_drain_to_window_size(),
        }
    }

    fn buffer_read(&mut self, target: &mut [u8]) -> Result<usize, Error> {
        match self {
            Self::Ring(s) => s.buffer.read(target),
            Self::Flat(s) => s.buffer.read(target),
        }
    }

    fn buffer_read_all(&mut self, target: &mut [u8]) -> Result<usize, Error> {
        match self {
            Self::Ring(s) => s.buffer.read_all(target),
            Self::Flat(s) => s.buffer.read_all(target),
        }
    }

    fn decode_block_content<R: Read>(
        &mut self,
        decoder: &mut BlockDecoder,
        header: &crate::blocks::block::BlockHeader,
        source: R,
        dict: Option<&Dictionary>,
    ) -> Result<u64, DecodeBlockContentError> {
        match self {
            Self::Ring(s) => decoder.decode_block_content(header, s, dict, source),
            Self::Flat(s) => decoder.decode_block_content(header, s, dict, source),
        }
    }

    fn hash_finish(&self) -> u64 {
        use core::hash::Hasher;
        match self {
            Self::Ring(s) => s.buffer.hash.finish(),
            Self::Flat(s) => s.buffer.hash.finish(),
        }
    }

    /// Forward the drain-time hash toggle to the inner `DecodeBuffer`
    /// (streaming path). Called by the frame layer from the decoder's
    /// `ContentChecksum` mode before each decode.

    fn set_compute_hash(&mut self, compute: bool) {
        match self {
            Self::Ring(s) => s.buffer.set_compute_hash(compute),
            Self::Flat(s) => s.buffer.set_compute_hash(compute),
        }
    }
}

struct FrameDecoderState {
    pub frame_header: frame::FrameHeader,
    decoder_scratch: DecoderScratchKind,
    frame_finished: bool,
    block_counter: usize,
    bytes_read_counter: u64,
    check_sum: Option<u32>,
    using_dict: Option<u32>,
    /// The dictionary handle applied to this frame, owned for the frame's whole
    /// decode so the block loop can hand the scratch a `&Dictionary` borrow at
    /// every `Dict`-sourced table read. ONE refcount clone per dict-apply (not
    /// per block, not per frame on the reuse path — the same handle stays held
    /// across `init_with_dict_handle` -> `decode_blocks`); the decode loop
    /// borrows from this field (disjoint from `decoder_scratch`) with zero
    /// further clones. `None` on the no-dict path. Cleared by `reset`.
    active_dict: Option<DictionaryHandle>,
}

pub enum BlockDecodingStrategy {
    All,
    UptoBlocks(usize),
    UptoBytes(usize),
}

impl FrameDecoderState {
    /// Window size to actually reserve for this frame's decode buffer.
    /// A declared content size caps the useful window: matches can never
    /// reference further back than the bytes that will ever exist, so an
    /// encoder-declared window above the FCS (e.g. a level-preset window
    /// on a smaller input) must not inflate the reservation. Every
    /// `reserve_buffer` site routes through this so the cap is uniform
    /// across `decode_all_impl`, `decode_blocks`, and the partial path.
    fn useful_window_size(&self) -> usize {
        let window_size = self.frame_header.window_size().unwrap_or(0);
        if self.frame_header.fcs_declared() {
            window_size.min(self.frame_header.frame_content_size()) as usize
        } else {
            window_size as usize
        }
    }

    /// Construct a new frame decoder state, reading the frame header
    /// from `source`. When `magicless` is `true`, the 4-byte magic
    /// number prefix is NOT consumed (upstream zstd `ZSTD_f_zstd1_magicless`).
    /// Crate-internal — reached only via `FrameDecoder::init` /
    /// `FrameDecoder::init_with_dict_handle`. The decode buffer is
    /// allocated lazily on BOTH backends (`new_ring` and `new_flat`):
    /// direct-eligible frames pay zero buffer allocation, and the
    /// non-direct fallback reserves `window_size` once in
    /// `decode_all_impl` / `decode_blocks` via `reserve_buffer` before
    /// any block write.
    #[inline]
    pub(crate) fn new_with_format(
        source: impl Read,
        magicless: bool,
    ) -> Result<FrameDecoderState, FrameDecoderError> {
        let (frame, header_size) = frame::read_frame_header_with_format(source, magicless)?;
        Self::new_with_parsed_header(frame, header_size)
    }

    /// Build a fresh state from an already-parsed frame header (the non-parsing
    /// tail of [`new_with_format`]). Shared by the `Read` path and the
    /// slice-direct path ([`FrameDecoder::reset_from_slice`]).
    pub(crate) fn new_with_parsed_header(
        frame: frame::FrameHeader,
        header_size: u8,
    ) -> Result<FrameDecoderState, FrameDecoderError> {
        let window_size = frame.window_size()?;

        if window_size > MAXIMUM_ALLOWED_WINDOW_SIZE {
            return Err(FrameDecoderError::WindowSizeTooBig {
                requested: window_size,
            });
        }

        let decoder_scratch = if frame.descriptor.single_segment_flag() {
            DecoderScratchKind::new_flat(window_size as usize)
        } else {
            DecoderScratchKind::new_ring(window_size as usize)
        };
        Ok(FrameDecoderState {
            frame_header: frame,
            frame_finished: false,
            block_counter: 0,
            decoder_scratch,
            bytes_read_counter: u64::from(header_size),
            check_sum: None,
            using_dict: None,
            active_dict: None,
        })
    }

    /// Reset this state for a new frame read from `source`, reusing
    /// existing allocations. When `magicless` is `true`, the frame
    /// header is read WITHOUT expecting a magic-number prefix
    /// (upstream zstd `ZSTD_f_zstd1_magicless`). Crate-internal — reached
    /// only via `FrameDecoder::reset`.
    ///
    /// `DecodeBuffer::reset` no longer reserves window_size for either
    /// backend — capacity decisions live one layer up. Both backends are
    /// lazy: direct-eligible frames pay zero backing-buffer allocation
    /// here (they write through `UserSliceBackend`), and the non-direct
    /// path is pre-reserved by `decode_all_impl` / `decode_blocks` via
    /// `DecoderScratchKind::reserve_buffer(window_size)` before any block
    /// write. A reused scratch whose new frame fits within prior capacity
    /// reuses it; a larger one grows on that same `reserve_buffer` call.
    #[inline]
    pub(crate) fn reset_with_format(
        &mut self,
        source: impl Read,
        magicless: bool,
    ) -> Result<(), FrameDecoderError> {
        let (frame_header, header_size) = frame::read_frame_header_with_format(source, magicless)?;
        self.reset_with_parsed_header(frame_header, header_size)
    }

    /// Apply an already-parsed frame header to this state (the non-parsing tail
    /// of [`reset_with_format`]). Shared by the `Read` path and the slice-direct
    /// path ([`FrameDecoder::reset_from_slice`]).
    #[inline]
    pub(crate) fn reset_with_parsed_header(
        &mut self,
        frame_header: frame::FrameHeader,
        header_size: u8,
    ) -> Result<(), FrameDecoderError> {
        let window_size = frame_header.window_size()?;

        if window_size > MAXIMUM_ALLOWED_WINDOW_SIZE {
            return Err(FrameDecoderError::WindowSizeTooBig {
                requested: window_size,
            });
        }

        self.decoder_scratch
            .reset(&frame_header, window_size as usize);
        self.frame_header = frame_header;
        self.frame_finished = false;
        self.block_counter = 0;
        self.bytes_read_counter = u64::from(header_size);
        self.check_sum = None;
        self.using_dict = None;
        // `active_dict` is intentionally NOT cleared here: it is only ever READ
        // while a scratch table source is `Dict`, which `init_from_dict` arms
        // on a dict frame and which a no-dict frame leaves `Local` (so a stale
        // held handle is never read). Keeping it lets the per-apply `ptr::eq`
        // reuse-check below skip the clone when the SAME dictionary is
        // re-applied frame-over-frame (the CoordiNode per-label-dict hot path)
        // — zero refcount churn on reuse, one clone only on a genuine swap.
        Ok(())
    }

    /// Hold the dictionary handle for this frame's whole decode so the block
    /// loop can borrow `&Dictionary` at every `Dict`-sourced read. Clones the
    /// handle ONLY when it is a different dictionary than the one already held
    /// (`ptr::eq` on the `Arc`'s pointee) — so re-applying the SAME dictionary
    /// frame-over-frame (the reuse hot path) costs zero refcount churn.
    fn set_active_dict(&mut self, dict: &DictionaryHandle) {
        if self
            .active_dict
            .as_ref()
            .is_none_or(|held| !core::ptr::eq(held.as_dict(), dict.as_dict()))
        {
            self.active_dict = Some(dict.clone());
        }
    }
}

impl Default for FrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameDecoder {
    /// This will create a new decoder without allocating anything yet.
    /// init()/reset() will allocate all needed buffers if it is the first time this decoder is used
    /// else they just reset these buffers with not further allocations
    pub fn new() -> FrameDecoder {
        FrameDecoder {
            state: None,

            owned_dicts: BTreeMap::new(),
            #[cfg(target_has_atomic = "ptr")]
            shared_dicts: BTreeMap::new(),
            #[cfg(not(target_has_atomic = "ptr"))]
            shared_dicts: (),
            magicless: false,
            content_checksum: ContentChecksum::EmitOnly,
        }
    }

    /// Heap bytes currently held by the decoder's lazily-grown workspace:
    /// the decode-window buffer plus the per-block literal/content buffers
    /// and the entropy tables. Returns 0 before the first frame is initialised
    /// (no workspace allocated yet). The window allocation dominates and grows
    /// with the frame's window size; this is the value to track for decode-time
    /// memory pressure, mirroring the workspace term of upstream
    /// `ZSTD_sizeof_DCtx`. Shared dictionaries (ref-counted handles) are not
    /// counted, matching upstream excluding `refDDict` memory.
    pub fn workspace_size(&self) -> usize {
        self.state
            .as_ref()
            .map_or(0, |s| s.decoder_scratch.workspace_bytes())
    }

    /// Select how the frame's optional content checksum is handled
    /// (compute, expose, verify, or skip). See [`ContentChecksum`].
    /// Default [`ContentChecksum::EmitOnly`]. Takes effect on the next
    /// decode; safe to call between frames on a reused decoder.
    pub fn set_content_checksum(&mut self, mode: ContentChecksum) {
        self.content_checksum = mode;
    }

    /// Enable or disable magicless frame format
    /// (`ZSTD_f_zstd1_magicless`). When set to `true`, subsequent
    /// [`init`](Self::init) / [`reset`](Self::reset) calls expect the frame header to begin
    /// directly with the frame-header descriptor — no 4-byte magic
    /// number prefix. Default false. Must match the encoder's
    /// magicless setting; the format is unambiguous only when the
    /// caller knows it out-of-band.
    ///
    /// Note: magicless mode also disables skippable-frame detection.
    /// The `0x184D2A50..=0x184D2A5F` skippable-frame magic range is
    /// only recognised when the 4-byte magic prefix is consumed, so
    /// `decode_all` / `init` / `reset` will treat a skippable frame
    /// at the head of a magicless stream as a malformed frame header
    /// (bad descriptor / window-size error) instead of skipping it.
    /// Mixed-format streams that interleave skippable frames must be
    /// pre-split by the caller; `set_magicless(true)` is only safe
    /// when the entire stream is known to be magicless zstd frames.
    pub fn set_magicless(&mut self, magicless: bool) {
        self.magicless = magicless;
    }

    #[cfg(target_has_atomic = "ptr")]
    fn shared_dict_exists(&self, dict_id: u32) -> bool {
        self.shared_dicts.contains_key(&dict_id)
    }

    #[cfg(not(target_has_atomic = "ptr"))]
    fn shared_dict_exists(&self, _dict_id: u32) -> bool {
        false
    }

    fn validate_registered_dictionary(dict: &Dictionary) -> Result<(), FrameDecoderError> {
        use crate::decoding::errors::DictionaryDecodeError as dict_err;

        if dict.id == 0 {
            return Err(FrameDecoderError::from(dict_err::ZeroDictionaryId));
        }
        if let Some(index) = dict.offset_hist.iter().position(|&rep| rep == 0) {
            return Err(FrameDecoderError::from(
                dict_err::ZeroRepeatOffsetInDictionary { index: index as u8 },
            ));
        }
        Ok(())
    }

    /// init() will allocate all needed buffers if it is the first time this decoder is used
    /// else they just reset these buffers with not further allocations
    ///
    /// Note that all bytes currently in the decodebuffer from any previous frame will be lost. Collect them with collect()/collect_to_writer()
    ///
    /// equivalent to reset()
    #[inline]
    pub fn init(&mut self, source: impl Read) -> Result<(), FrameDecoderError> {
        self.reset(source)
    }

    /// Initialize the decoder for a new frame using a pre-parsed dictionary handle.
    ///
    /// If the frame header has a dictionary ID, this validates it against
    /// `dict.id()` and returns [`FrameDecoderError::DictIdMismatch`] on mismatch.
    ///
    /// If the header omits the optional dictionary ID, this still applies the
    /// provided dictionary handle.
    ///
    /// # Warning
    ///
    /// This method always applies `dict` unless the frame header contains a
    /// non-matching dictionary ID. Callers must only use this API when they
    /// already know the frame was encoded with the provided dictionary, even if
    /// the frame header omits the dictionary ID or encodes an explicit
    /// dictionary ID of `0`.
    ///
    /// Passing a dictionary for a frame that was not encoded with it can
    /// silently corrupt the decoded output.
    pub fn init_with_dict_handle(
        &mut self,
        source: impl Read,
        dict: &DictionaryHandle,
    ) -> Result<(), FrameDecoderError> {
        self.reset_with_dict_handle(source, dict)
    }

    /// reset() will allocate all needed buffers if it is the first time this decoder is used
    /// else they just reset these buffers with not further allocations
    ///
    /// Note that all bytes currently in the decodebuffer from any previous frame will be lost. Collect them with collect()/collect_to_writer()
    ///
    /// equivalent to init()
    #[inline]
    pub fn reset(&mut self, source: impl Read) -> Result<(), FrameDecoderError> {
        use FrameDecoderError as err;
        // Fresh frame → start with an empty per-block checksum vec so
        // the values for the next frame don't carry over from the
        // previous one.
        let magicless = self.magicless;
        let dict_id = match &mut self.state {
            Some(s) => {
                s.reset_with_format(source, magicless)?;
                s.frame_header.dictionary_id()
            }
            None => {
                self.state = Some(FrameDecoderState::new_with_format(source, magicless)?);
                self.state
                    .as_ref()
                    .and_then(|state| state.frame_header.dictionary_id())
            }
        };
        // Validate any pinned expectations BEFORE block decode work
        // runs. Catches dict_id substitution / window-descriptor
        // tampering on inputs already authenticated by an outer
        // layer (e.g. AEAD). Returning here leaves `self.state` in
        // a re-resettable shape — next `reset()` re-parses the
        // frame header without intermediate cleanup.

        if let Some(dict_id) = dict_id {
            let state = self.state.as_mut().expect("state initialized");
            let owned_dicts = &self.owned_dicts;
            #[cfg(target_has_atomic = "ptr")]
            let shared_dicts = &self.shared_dicts;
            let dict = owned_dicts
                .get(&dict_id)
                .or_else(|| {
                    #[cfg(target_has_atomic = "ptr")]
                    {
                        shared_dicts.get(&dict_id)
                    }
                    #[cfg(not(target_has_atomic = "ptr"))]
                    {
                        None
                    }
                })
                .ok_or(err::DictNotProvided { dict_id })?;
            state.decoder_scratch.init_from_dict(dict);
            state.set_active_dict(dict);
            state.using_dict = Some(dict_id);
        }
        Ok(())
    }

    /// Slice-direct equivalent of [`reset`](Self::reset) for the in-memory
    /// decode path: parses the frame header straight out of `*input` via
    /// [`frame::read_frame_header_from_slice`] (no `Read`-trait `read_exact`
    /// per field) and advances `*input` past it, then applies it through the
    /// shared parsed-header path. Behaviour — including skippable-frame and
    /// truncation errors, dictionary-id resolution, and pinned-expectation
    /// validation — is identical to `reset`; only the header read avoids the
    /// `io::impls` dispatch.
    pub(crate) fn reset_from_slice(&mut self, input: &mut &[u8]) -> Result<(), FrameDecoderError> {
        use FrameDecoderError as err;
        let magicless = self.magicless;
        let (frame_header, header_size) = frame::read_frame_header_from_slice(input, magicless)?;
        let dict_id = match &mut self.state {
            Some(s) => {
                s.reset_with_parsed_header(frame_header, header_size)?;
                s.frame_header.dictionary_id()
            }
            None => {
                self.state = Some(FrameDecoderState::new_with_parsed_header(
                    frame_header,
                    header_size,
                )?);
                self.state
                    .as_ref()
                    .and_then(|state| state.frame_header.dictionary_id())
            }
        };

        if let Some(dict_id) = dict_id {
            let state = self.state.as_mut().expect("state initialized");
            let owned_dicts = &self.owned_dicts;
            #[cfg(target_has_atomic = "ptr")]
            let shared_dicts = &self.shared_dicts;
            let dict = owned_dicts
                .get(&dict_id)
                .or_else(|| {
                    #[cfg(target_has_atomic = "ptr")]
                    {
                        shared_dicts.get(&dict_id)
                    }
                    #[cfg(not(target_has_atomic = "ptr"))]
                    {
                        None
                    }
                })
                .ok_or(err::DictNotProvided { dict_id })?;
            state.decoder_scratch.init_from_dict(dict);
            state.set_active_dict(dict);
            state.using_dict = Some(dict_id);
        }
        Ok(())
    }

    /// Reset this decoder for a new frame using a pre-parsed dictionary handle.
    ///
    /// If the frame header has a dictionary ID, this validates it against
    /// `dict.id()` and returns [`FrameDecoderError::DictIdMismatch`] on mismatch.
    ///
    /// If the header omits the optional dictionary ID, this still applies the
    /// provided dictionary handle.
    ///
    /// # Warning
    ///
    /// This method always applies `dict` unless the frame header contains a
    /// non-matching dictionary ID. Callers must only use this API when they
    /// already know the frame was encoded with the provided dictionary, even if
    /// the frame header omits the dictionary ID or encodes an explicit
    /// dictionary ID of `0`.
    ///
    /// Passing a dictionary for a frame that was not encoded with it can
    /// silently corrupt the decoded output.
    pub fn reset_with_dict_handle(
        &mut self,
        source: impl Read,
        dict: &DictionaryHandle,
    ) -> Result<(), FrameDecoderError> {
        use FrameDecoderError as err;
        // Fresh frame → drop the previous frame's per-block checksum
        // digests so the next decode starts with an empty vec.
        // Mirrors the same clear in `reset()`; reset_with_dict_handle
        // is a parallel entry point so it needs its own call.
        Self::validate_registered_dictionary(dict.as_dict())?;
        let magicless = self.magicless;
        // Scope the &mut borrow of `self.state` to the header parse
        // alone, so the subsequent `validate_expectations(&self, ...)`
        // call below can take a fresh shared borrow of self without
        // tripping the borrow checker.
        match &mut self.state {
            Some(s) => s.reset_with_format(source, magicless)?,
            None => {
                self.state = Some(FrameDecoderState::new_with_format(source, magicless)?);
            }
        }
        // Single source of truth: route through the same
        // `validate_expectations` used by `reset()`. Routing through
        // the helper keeps the two code paths from drifting (e.g.,
        // if expect-semantics or error wiring changes later).

        let state = self
            .state
            .as_mut()
            .expect("state populated by reset_with_format/new_with_format");
        if let Some(dict_id) = state.frame_header.dictionary_id()
            && dict_id != dict.id()
        {
            return Err(err::DictIdMismatch {
                expected: dict_id,
                provided: dict.id(),
            });
        }
        state.decoder_scratch.init_from_dict(dict);
        state.set_active_dict(dict);
        state.using_dict = Some(dict.id());
        Ok(())
    }

    /// Slice-direct equivalent of [`reset_with_dict_handle`](Self::reset_with_dict_handle)
    /// for the in-memory decode path: parses the frame header straight out of
    /// `*input` via [`frame::read_frame_header_from_slice`] (no `Read`-trait
    /// `read_exact` per field) and applies it through the shared parsed-header
    /// path, then attaches `dict`. Behaviour — dictionary-id mismatch, pinned
    /// expectations, scratch init — is identical to `reset_with_dict_handle`;
    /// only the header read avoids the `io::impls` dispatch.
    pub(crate) fn reset_from_slice_with_dict_handle(
        &mut self,
        input: &mut &[u8],
        dict: &DictionaryHandle,
    ) -> Result<(), FrameDecoderError> {
        use FrameDecoderError as err;
        Self::validate_registered_dictionary(dict.as_dict())?;
        let magicless = self.magicless;
        let (frame_header, header_size) = frame::read_frame_header_from_slice(input, magicless)?;
        match &mut self.state {
            Some(s) => s.reset_with_parsed_header(frame_header, header_size)?,
            None => {
                self.state = Some(FrameDecoderState::new_with_parsed_header(
                    frame_header,
                    header_size,
                )?);
            }
        }

        let state = self
            .state
            .as_mut()
            .expect("state populated by reset_with_parsed_header/new_with_parsed_header");
        if let Some(dict_id) = state.frame_header.dictionary_id()
            && dict_id != dict.id()
        {
            return Err(err::DictIdMismatch {
                expected: dict_id,
                provided: dict.id(),
            });
        }
        state.decoder_scratch.init_from_dict(dict);
        state.set_active_dict(dict);
        state.using_dict = Some(dict.id());
        Ok(())
    }

    /// Add a dictionary that can be selected dynamically by frame dictionary ID.
    ///
    /// Returns [`FrameDecoderError::DictAlreadyRegistered`] if the ID is already
    /// registered (either as owned or shared).
    pub fn add_dict(&mut self, dict: Dictionary) -> Result<(), FrameDecoderError> {
        Self::validate_registered_dictionary(&dict)?;
        let dict_id = dict.id;
        if self.owned_dicts.contains_key(&dict_id) || self.shared_dict_exists(dict_id) {
            return Err(FrameDecoderError::DictAlreadyRegistered { dict_id });
        }
        self.owned_dicts
            .insert(dict_id, DictionaryHandle::from_dictionary(dict));
        Ok(())
    }

    /// Parse and add a serialized dictionary blob.
    pub fn add_dict_from_bytes(&mut self, raw_dictionary: &[u8]) -> Result<(), FrameDecoderError> {
        let dict = Dictionary::decode_dict(raw_dictionary)?;
        self.add_dict(dict)
    }

    /// Add a pre-parsed dictionary handle for reuse across decoders.
    ///
    /// This API is available on targets with pointer-width atomics
    /// (`target_has_atomic = "ptr"`).
    ///
    /// Returns [`FrameDecoderError::DictAlreadyRegistered`] if the ID is already
    /// registered (either as owned or shared).
    #[cfg(target_has_atomic = "ptr")]
    pub fn add_dict_handle(&mut self, dict: DictionaryHandle) -> Result<(), FrameDecoderError> {
        Self::validate_registered_dictionary(dict.as_dict())?;
        let dict_id = dict.id();
        if self.owned_dicts.contains_key(&dict_id) || self.shared_dicts.contains_key(&dict_id) {
            return Err(FrameDecoderError::DictAlreadyRegistered { dict_id });
        }
        self.shared_dicts.insert(dict_id, dict);
        Ok(())
    }

    pub fn force_dict(&mut self, dict_id: u32) -> Result<(), FrameDecoderError> {
        use FrameDecoderError as err;
        let state = self.state.as_mut().ok_or(err::NotYetInitialized)?;
        let owned_dicts = &self.owned_dicts;
        #[cfg(target_has_atomic = "ptr")]
        let shared_dicts = &self.shared_dicts;

        let dict = owned_dicts
            .get(&dict_id)
            .or_else(|| {
                #[cfg(target_has_atomic = "ptr")]
                {
                    shared_dicts.get(&dict_id)
                }
                #[cfg(not(target_has_atomic = "ptr"))]
                {
                    None
                }
            })
            .ok_or(err::DictNotProvided { dict_id })?;
        state.decoder_scratch.init_from_dict(dict);
        state.set_active_dict(dict);
        state.using_dict = Some(dict_id);

        Ok(())
    }

    /// Returns how many bytes the frame contains after decompression
    pub fn content_size(&self) -> u64 {
        match &self.state {
            None => 0,
            Some(s) => s.frame_header.frame_content_size(),
        }
    }

    /// Returns the checksum that was read from the data. Only available after all bytes have been read. It is the last 4 bytes of a zstd-frame
    pub fn get_checksum_from_data(&self) -> Option<u32> {
        let state = self.state.as_ref()?;

        state.check_sum
    }

    /// Returns the checksum that was calculated while decoding.
    /// Only a sensible value after all decoded bytes have been collected/read from the FrameDecoder.
    /// Returns `None` when the frame header has `content_checksum_flag = 0`:
    /// no hash is computed for such frames (the post-decode XXH64 pass was a
    /// 63 % decode-wall hotspot on flag-off frames; skipping it when the
    /// frame format declares no trailing digest avoids that wasted work).

    pub fn get_calculated_checksum(&self) -> Option<u32> {
        let state = self.state.as_ref()?;
        // `ContentChecksum::None` skips the XXH64 pass entirely, so there is
        // no calculated digest to report.
        if self.content_checksum == ContentChecksum::None {
            return None;
        }
        if !state.frame_header.descriptor.content_checksum_flag() {
            return None;
        }
        let cksum_64bit = state.decoder_scratch.hash_finish();
        //truncate to lower 32bit because reasons...
        Some(cksum_64bit as u32)
    }

    /// Compare the frame's stored content checksum against the digest the
    /// decoder computed, returning [`FrameDecoderError::ChecksumMismatch`] on
    /// disagreement. No-op unless the mode is [`ContentChecksum::Verify`] and
    /// the frame carries a trailing checksum.
    ///
    /// [`decode_all`](Self::decode_all) and the streaming reader call this
    /// automatically. Callers driving [`decode_blocks`](Self::decode_blocks)
    /// directly invoke it themselves once per frame, after the frame is fully
    /// decoded AND fully drained (e.g. via [`collect`](Self::collect)), so both
    /// the stored value and the running digest are final.

    pub fn verify_content_checksum(&self) -> Result<(), FrameDecoderError> {
        if self.content_checksum != ContentChecksum::Verify {
            return Ok(());
        }
        let Some(state) = self.state.as_ref() else {
            return Ok(());
        };
        if !state.frame_header.descriptor.content_checksum_flag() {
            return Ok(());
        }
        let Some(expected) = state.check_sum else {
            return Ok(());
        };
        let calculated = state.decoder_scratch.hash_finish() as u32;
        if expected != calculated {
            return Err(FrameDecoderError::ChecksumMismatch {
                expected,
                calculated,
            });
        }
        Ok(())
    }

    /// Counter for how many bytes have been consumed while decoding the frame
    pub fn bytes_read_from_source(&self) -> u64 {
        let state = match &self.state {
            None => return 0,
            Some(s) => s,
        };
        state.bytes_read_counter
    }

    /// Whether the current frames last block has been decoded yet
    /// If this returns true you can call the drain* functions to get all content
    /// (the read() function will drain automatically if this returns true)
    pub fn is_finished(&self) -> bool {
        let state = match &self.state {
            None => return true,
            Some(s) => s,
        };
        if state.frame_header.descriptor.content_checksum_flag() {
            state.frame_finished && state.check_sum.is_some()
        } else {
            state.frame_finished
        }
    }

    /// Counter for how many blocks have already been decoded
    pub fn blocks_decoded(&self) -> usize {
        let state = match &self.state {
            None => return 0,
            Some(s) => s,
        };
        state.block_counter
    }

    /// Decodes blocks from a reader. It requires that the framedecoder has been initialized first.
    /// The Strategy influences how many blocks will be decoded before the function returns
    /// This is important if you want to manage memory consumption carefully. If you don't care
    /// about that you can just choose the strategy "All" and have all blocks of the frame decoded into the buffer
    pub fn decode_blocks(
        &mut self,
        mut source: impl Read,
        strat: BlockDecodingStrategy,
    ) -> Result<bool, FrameDecoderError> {
        use FrameDecoderError as err;
        // Apply the content-checksum mode to the streaming drain hash before
        // any block decodes into the ring. Hash only when a digest is both
        // wanted (mode != None) AND present in the frame (content_checksum_flag
        // set) — a flag-off frame has nothing to verify or expose, so hashing
        // it is wasted work. Mirrors the direct path and get_calculated_checksum.

        let checksum_mode = self.content_checksum;
        let state = self.state.as_mut().ok_or(err::NotYetInitialized)?;

        {
            let compute_hash = checksum_mode != ContentChecksum::None
                && state.frame_header.descriptor.content_checksum_flag();
            state.decoder_scratch.set_compute_hash(compute_hash);
        }

        // Streaming entry point: pre-reserve the backing buffer to
        // the FCS-capped window so multi-block frames don't pay repeated
        // `reserve_amortized` grow steps (128 KiB → 256 KiB → ... →
        // window) as blocks accumulate. `decode_all` does the same up
        // front in `decode_all_impl`; this mirrors it for callers
        // driving `decode_blocks` directly. Idempotent — the
        // backend's `reserve` early-returns when capacity is already
        // sufficient.
        let useful_window = state.useful_window_size();
        state.decoder_scratch.reserve_buffer(useful_window);

        let mut block_dec = decoding::block_decoder::new();

        let buffer_size_before = state.decoder_scratch.buffer_len();
        let block_counter_before = state.block_counter;
        loop {
            vprintln!("################");
            vprintln!("Next Block: {}", state.block_counter);
            vprintln!("################");
            // Capture the failing-block coordinates BEFORE the header read so
            // the error carries where it happened: `bytes_read_counter` is the
            // frame-absolute offset of this block's header (not yet advanced),
            // `block_counter` its 0-based index. Used by both the header- and
            // body-error builders below (block-precise recovery under `lsm`).
            let block_index = state.block_counter as u32;
            let block_frame_offset = state.bytes_read_counter as u32;
            let (block_header, block_header_size) =
                block_dec.read_block_header(&mut source).map_err(|source| {
                    block_header_decode_error(source, block_index, block_frame_offset)
                })?;
            state.bytes_read_counter += u64::from(block_header_size);

            vprintln!();
            vprintln!(
                "Found {} block with size: {}, which will be of size: {}",
                block_header.block_type,
                block_header.content_size,
                block_header.decompressed_size
            );

            // Only expose the held dictionary while THIS frame is dict-backed
            // (`using_dict` is set per dict-apply, cleared on reset). A reused
            // decoder keeps `active_dict` across a no-dict frame for the
            // `ptr::eq` reuse-skip, so it must be gated here or a stray
            // out-of-window offset on a dictless frame would resolve against the
            // stale dictionary content instead of erroring.
            let dict_ref = if state.using_dict.is_some() {
                state.active_dict.as_ref().map(|h| h.as_dict())
            } else {
                None
            };
            let bytes_read_in_block_body = state
                .decoder_scratch
                .decode_block_content(&mut block_dec, &block_header, &mut source, dict_ref)
                .map_err(|source| {
                    block_body_decode_error(
                        source,
                        block_index,
                        block_frame_offset,
                        &block_header,
                        block_header_size,
                    )
                })?;
            state.bytes_read_counter += bytes_read_in_block_body;

            // Per-block XXH64 (low 32 bits) of the just-decompressed
            // bytes. Hashed from `last_n_as_slices` so RingBuffer wrap
            // is handled in-place, no extra copy.

            state.block_counter += 1;

            vprintln!("Output: {}", state.decoder_scratch.buffer_len());

            if block_header.last_block {
                state.frame_finished = true;
                if state.frame_header.descriptor.content_checksum_flag() {
                    let mut chksum = [0u8; 4];
                    source
                        .read_exact(&mut chksum)
                        .map_err(err::FailedToReadChecksum)?;
                    state.bytes_read_counter += 4;
                    let chksum = u32::from_le_bytes(chksum);
                    state.check_sum = Some(chksum);
                }
                break;
            }

            match strat {
                BlockDecodingStrategy::All => { /* keep going */ }
                BlockDecodingStrategy::UptoBlocks(n) => {
                    if state.block_counter - block_counter_before >= n {
                        break;
                    }
                }
                BlockDecodingStrategy::UptoBytes(n) => {
                    if state.decoder_scratch.buffer_len() - buffer_size_before >= n {
                        break;
                    }
                }
            }
        }

        Ok(state.frame_finished)
    }

    /// Collect bytes and retain window_size bytes while decoding is still going on.
    /// After decoding of the frame (is_finished() == true) has finished it will collect all remaining bytes
    pub fn collect(&mut self) -> Option<Vec<u8>> {
        let finished = self.is_finished();
        let state = self.state.as_mut()?;
        if finished {
            Some(state.decoder_scratch.buffer_drain())
        } else {
            state.decoder_scratch.buffer_drain_to_window_size()
        }
    }

    /// Collect bytes and retain window_size bytes while decoding is still going on.
    /// After decoding of the frame (is_finished() == true) has finished it will collect all remaining bytes
    pub fn collect_to_writer(&mut self, w: impl Write) -> Result<usize, Error> {
        let finished = self.is_finished();
        let state = match &mut self.state {
            None => return Ok(0),
            Some(s) => s,
        };
        if finished {
            state.decoder_scratch.buffer_drain_to_writer(w)
        } else {
            state.decoder_scratch.buffer_drain_to_window_size_writer(w)
        }
    }

    /// How many bytes can currently be collected from the decodebuffer, while decoding is going on this will be lower than the actual decodbuffer size
    /// because window_size bytes need to be retained for decoding.
    /// After decoding of the frame (is_finished() == true) has finished it will report all remaining bytes
    pub fn can_collect(&self) -> usize {
        let finished = self.is_finished();
        let state = match &self.state {
            None => return 0,
            Some(s) => s,
        };
        if finished {
            state.decoder_scratch.buffer_can_drain()
        } else {
            state
                .decoder_scratch
                .buffer_can_drain_to_window_size()
                .unwrap_or(0)
        }
    }

    /// Decodes as many blocks as possible from the source slice and reads from the decodebuffer into the target slice
    /// The source slice may contain only parts of a frame but must contain at least one full block to make progress
    ///
    /// By all means use decode_blocks if you have a io.Reader available. This is just for compatibility with other decompressors
    /// which try to serve an old-style c api
    ///
    /// Returns (read, written), if read == 0 then the source did not contain a full block and further calls with the same
    /// input will not make any progress!
    ///
    /// Note that no kind of block can be bigger than 128kb.
    /// So to be safe use at least 128*1024 (max block content size) + 3 (block_header size) + 18 (max frame_header size) bytes as your source buffer
    ///
    /// You may call this function with an empty source after all bytes have been decoded. This is equivalent to just call decoder.read(&mut target)
    pub fn decode_from_to(
        &mut self,
        source: &[u8],
        target: &mut [u8],
    ) -> Result<(usize, usize), FrameDecoderError> {
        use FrameDecoderError as err;
        let bytes_read_at_start = match &self.state {
            Some(s) => s.bytes_read_counter,
            None => 0,
        };

        if !self.is_finished() || self.state.is_none() {
            let mut mt_source = source;

            if self.state.is_none() {
                self.init(&mut mt_source)?;
            }

            //pseudo block to scope "state" so we can borrow self again after the block
            {
                let state = match &mut self.state {
                    Some(s) => s,
                    None => panic!("Bug in library"),
                };
                let mut block_dec = decoding::block_decoder::new();

                // Honour the content-checksum mode on this hand-rolled decode
                // loop (it does not go through `decode_blocks`): hash only when
                // a digest is wanted and the frame carries one. `None` skips the
                // XXH64 pass; verification happens after the final drain below.

                {
                    let compute_hash = self.content_checksum != ContentChecksum::None
                        && state.frame_header.descriptor.content_checksum_flag();
                    state.decoder_scratch.set_compute_hash(compute_hash);
                }

                if state.frame_header.descriptor.content_checksum_flag()
                    && state.frame_finished
                    && state.check_sum.is_none()
                {
                    // The trailing checksum arrived on a separate call (the last
                    // block finished earlier). Consume it and fall through to the
                    // shared `self.read` + post-drain verify below — NOT an early
                    // return — so any output still buffered from a prior
                    // small-`target` call is flushed on this call too, and the
                    // checksum is verified through the one shared path.
                    if mt_source.len() >= 4 {
                        let chksum = mt_source[..4].try_into().expect("optimized away");
                        state.bytes_read_counter += 4;
                        let chksum = u32::from_le_bytes(chksum);
                        state.check_sum = Some(chksum);
                        mt_source = &mt_source[4..];
                    }
                }

                loop {
                    // The frame is fully decoded (last block seen, trailer
                    // consumed above); no more blocks to read. Any leftover
                    // bytes are not a block header — stop before misreading them.
                    if state.frame_finished {
                        break;
                    }
                    //check if there are enough bytes for the next header
                    if mt_source.len() < 3 {
                        break;
                    }
                    let block_index = state.block_counter as u32;
                    let block_frame_offset = state.bytes_read_counter as u32;
                    let (block_header, block_header_size) = block_dec
                        .read_block_header(&mut mt_source)
                        .map_err(|source| {
                            block_header_decode_error(source, block_index, block_frame_offset)
                        })?;

                    // check the needed size for the block before updating counters.
                    // If not enough bytes are in the source, the header will have to be read again, so act like we never read it in the first place
                    if mt_source.len() < block_header.content_size as usize {
                        break;
                    }
                    state.bytes_read_counter += u64::from(block_header_size);

                    // Only expose the held dictionary while THIS frame is dict-backed
                    // (`using_dict` is set per dict-apply, cleared on reset). A reused
                    // decoder keeps `active_dict` across a no-dict frame for the
                    // `ptr::eq` reuse-skip, so it must be gated here or a stray
                    // out-of-window offset on a dictless frame would resolve against the
                    // stale dictionary content instead of erroring.
                    let dict_ref = if state.using_dict.is_some() {
                        state.active_dict.as_ref().map(|h| h.as_dict())
                    } else {
                        None
                    };
                    let bytes_read_in_block_body = state
                        .decoder_scratch
                        .decode_block_content(
                            &mut block_dec,
                            &block_header,
                            &mut mt_source,
                            dict_ref,
                        )
                        .map_err(|source| {
                            block_body_decode_error(
                                source,
                                block_index,
                                block_frame_offset,
                                &block_header,
                                block_header_size,
                            )
                        })?;
                    state.bytes_read_counter += bytes_read_in_block_body;
                    state.block_counter += 1;

                    if block_header.last_block {
                        state.frame_finished = true;
                        if state.frame_header.descriptor.content_checksum_flag() {
                            //if there are enough bytes handle this here. Else the block at the start of this function will handle it at the next call
                            if mt_source.len() >= 4 {
                                let chksum = mt_source[..4].try_into().expect("optimized away");
                                state.bytes_read_counter += 4;
                                let chksum = u32::from_le_bytes(chksum);
                                state.check_sum = Some(chksum);
                            }
                        }
                        break;
                    }
                }
            }
        }

        let result_len = self.read(target).map_err(err::FailedToDrainDecodebuffer)?;
        // Once the frame is fully decoded and drained, the running digest is
        // final: validate it in `Verify` mode (no-op otherwise). Same finish
        // point as the streaming reader.

        if self.is_finished() && self.can_collect() == 0 {
            self.verify_content_checksum()?;
        }
        let bytes_read_at_end = match &mut self.state {
            Some(s) => s.bytes_read_counter,
            None => panic!("Bug in library"),
        };
        let read_len = bytes_read_at_end - bytes_read_at_start;
        Ok((read_len as usize, result_len))
    }

    /// Decode multiple frames into the output slice.
    ///
    /// `input` must contain an exact number of frames. Skippable frames are allowed and will be
    /// skipped during decode.
    ///
    /// `output` must be large enough to hold the decompressed data. If you don't know
    /// how large the output will be, use [`FrameDecoder::decode_blocks`] instead.
    ///
    /// This calls [`FrameDecoder::init`], and all bytes currently in the decoder will be lost.
    ///
    /// Returns the number of bytes written to `output`.
    pub fn decode_all(
        &mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, FrameDecoderError> {
        self.decode_all_impl(input, output, |this, src| this.reset_from_slice(src))
    }

    /// Decode multiple frames into the output slice using a pre-parsed dictionary handle.
    ///
    /// `input` must contain an exact number of frames. Skippable frames are allowed and will be
    /// skipped during decode.
    ///
    /// `output` must be large enough to hold the decompressed data. If you don't know
    /// how large the output will be, use [`FrameDecoder::decode_blocks`] instead.
    ///
    /// This calls [`FrameDecoder::init_with_dict_handle`], and all bytes currently in the
    /// decoder will be lost.
    ///
    /// # Warning
    ///
    /// Each decoded frame is initialized with `dict`, even when a frame header
    /// omits the optional dictionary ID. Callers must only use this API when
    /// they already know the input frames were encoded with the provided
    /// dictionary; otherwise decoded output can be silently corrupted.
    pub fn decode_all_with_dict_handle(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        dict: &DictionaryHandle,
    ) -> Result<usize, FrameDecoderError> {
        {
            self.decode_all_impl(input, output, |this, src| {
                this.reset_from_slice_with_dict_handle(src, dict)
            })
        }
    }

    /// Whether the decoder sits at the very start of an initialised frame:
    /// the header has been read (state populated) but no block has been
    /// decoded and the frame is not finished. In this state the wrapped
    /// source is positioned exactly after the frame header, so
    /// [`Self::decode_current_frame_to_vec`] can decode the rest of the frame
    /// straight from the remaining source bytes.
    pub(crate) fn is_at_frame_start(&self) -> bool {
        self.state
            .as_ref()
            .is_some_and(|s| s.block_counter == 0 && !s.frame_finished)
    }

    /// Decode the CURRENT (already-initialised) frame, APPENDING the
    /// decompressed bytes to `output`, and return the number appended.
    ///
    /// `input` must be the frame's post-header bytes (the wrapped source after
    /// `init` consumed the header). Unlike [`Self::decode_all_to_vec`] this
    /// neither re-reads a header nor requires the caller to pre-reserve
    /// capacity: a frame that declares its content size decodes DIRECTLY into
    /// freshly-grown `output` capacity via the single-copy direct path
    /// ([`Self::run_direct_decode`]) — bypassing the `Ring`/`FlatBuf` →
    /// `read()` drain copy the streaming loop pays — while an unsized frame
    /// falls back to the window-bounded ring drain (still one copy, into
    /// `output`). Backs [`StreamingDecoder`](crate::decoding::StreamingDecoder)'s
    /// `read_to_end` fast path; the caller must ensure
    /// [`Self::is_at_frame_start`].
    ///
    /// # Errors
    ///
    /// Propagates any [`FrameDecoderError`] from block decode, content-size
    /// mismatch, or (in `Verify` mode) checksum validation.
    pub(crate) fn decode_current_frame_to_vec(
        &mut self,
        mut input: &[u8],
        output: &mut Vec<u8>,
        dict: Option<&DictionaryHandle>,
    ) -> Result<usize, FrameDecoderError> {
        let start_len = output.len();
        // The current frame is already initialised (its header consumed by the
        // caller, WITH `dict` applied if the decoder was constructed with one).
        // Decode it, then decode any FOLLOWING concatenated / skippable frames
        // in `input` so the whole source is consumed to EOF and nothing is
        // dropped (matching `read_to_end` semantics).
        self.decode_one_frame_to_vec(&mut input, output)?;
        self.decode_concatenated_frames_to_vec(&mut input, output, dict)?;
        Ok(output.len() - start_len)
    }

    /// Initialise and decode every frame remaining in `input` (concatenated /
    /// skippable), APPENDING to `output`. `input` is advanced as frames are
    /// consumed; on return it is empty. Re-initialisation honours `dict`: when
    /// `Some`, each following frame is initialised via
    /// [`Self::init_with_dict_handle`] so a forced dictionary is preserved even
    /// for frames that omit the dictionary id (plain [`Self::init`] would
    /// resolve dictionaries by id only). Backs the `read_to_end` fast path (the
    /// frames after the current one) and its mid-frame fallback (the frames
    /// after the partially-read one).
    pub(crate) fn decode_concatenated_frames_to_vec(
        &mut self,
        input: &mut &[u8],
        output: &mut Vec<u8>,
        dict: Option<&DictionaryHandle>,
    ) -> Result<usize, FrameDecoderError> {
        let start_len = output.len();
        while !input.is_empty() {
            let init_result = match dict {
                Some(d) => self.init_with_dict_handle(&mut *input, d),
                None => self.init(&mut *input),
            };
            match init_result {
                Ok(_) => {}
                Err(FrameDecoderError::ReadFrameHeaderError(
                    decoding::errors::ReadFrameHeaderError::SkipFrame { length, .. },
                )) => {
                    *input = input
                        .get(length as usize..)
                        .ok_or(FrameDecoderError::FailedToSkipFrame)?;
                    continue;
                }
                Err(e) => return Err(e),
            }
            self.decode_one_frame_to_vec(&mut *input, output)?;
        }
        Ok(output.len() - start_len)
    }

    /// Decode the single CURRENT (already-initialised) frame, APPENDING to
    /// `output`. Helper for [`Self::decode_current_frame_to_vec`].
    fn decode_one_frame_to_vec(
        &mut self,
        input: &mut &[u8],
        output: &mut Vec<u8>,
    ) -> Result<usize, FrameDecoderError> {
        let frame_start = output.len();
        let (content_size, fcs_declared) = {
            let s = self.state.as_ref().expect("frame is initialised");
            (
                s.frame_header.frame_content_size(),
                s.frame_header.fcs_declared(),
            )
        };
        // Direct path: a declared, non-empty content size that FITS in `usize`
        // (and whose end offset does not overflow). `usize::try_from` guards the
        // 32-bit / oversized-FCS truncation; an unrepresentable size falls
        // through to the window-bounded ring drain rather than allocating a
        // truncated buffer that would violate `run_direct_decode`'s precondition.
        //
        // Plausibility gate: the direct path `resize`s `output` to the declared
        // size up front, so a tiny/truncated frame declaring a huge (but
        // representable) FCS would allocate + zero that whole size before the
        // body is validated. zstd's per-block ceiling is MAX_BLOCK_SIZE from as
        // little as ~4 input bytes, so the declared size cannot legitimately
        // exceed `input.len() * (MAX_BLOCK_SIZE / 4)`. Anything larger falls
        // through to the ring drain, which grows only as real bytes are produced
        // and errors out cheaply on truncated input. `input` spans the remaining
        // source (this frame plus any following ones), so the bound only ever
        // over-permits — a legitimate frame is never forced off the direct path.
        // saturating_mul is intentional: an overflow means the available input
        // is so large that any representable FCS is plausible (cap = "no limit").
        const MAX_DECOMPRESSION_RATIO: usize = (crate::common::MAX_BLOCK_SIZE / 4) as usize;
        if content_size > 0
            && let Ok(cs) = usize::try_from(content_size)
            && cs <= input.len().saturating_mul(MAX_DECOMPRESSION_RATIO)
            && let Some(frame_end) = frame_start.checked_add(cs)
        {
            // Reserve exactly the frame's content and decode straight into it
            // (single copy, no ring). The direct path writes precisely
            // `content_size` bytes (erroring otherwise), so the grown region is
            // fully written.
            output.resize(frame_end, 0);
            // On error, drop the just-grown (zeroed) tail before propagating so
            // callers never observe bytes that were never decoded.
            let written =
                match self.run_direct_decode(&mut *input, &mut output[frame_start..], content_size)
                {
                    Ok(n) => n,
                    Err(e) => {
                        output.truncate(frame_start);
                        return Err(e);
                    }
                };
            output.truncate(frame_start + written);

            self.verify_content_checksum()?;
            return Ok(written);
        }
        // The ring-drain fallback below pre-reserves `useful_window_size()`
        // (= `window.min(FCS)`), which for a single-segment frame is the
        // declared FCS itself — so a truncated single-segment frame lying about
        // its size would still allocate the pledged window before the body
        // errors, sidestepping the direct-path gate above. Reject such a frame
        // up front when its declared (FCS-bearing) window exceeds what the
        // available input could plausibly produce. Frames without a declared
        // size keep their window-descriptor reservation (already capped at
        // `MAXIMUM_ALLOWED_WINDOW_SIZE` at init); a small-window multi-segment
        // frame still falls through to the ring drain, which errors cheaply on
        // the truncated body.
        if fcs_declared
            && let Some(state) = self.state.as_ref()
            && state.useful_window_size() > input.len().saturating_mul(MAX_DECOMPRESSION_RATIO)
        {
            return Err(FrameDecoderError::FrameContentSizeMismatch {
                declared: content_size,
                produced: 0,
            });
        }
        // No declared size, explicit FCS=0, or an unrepresentable FCS: window-
        // bounded ring drain, appended directly to `output` via
        // `collect_to_writer` (no staging buffer).
        loop {
            self.decode_blocks(&mut *input, BlockDecodingStrategy::UptoBytes(1024 * 1024))?;
            self.collect_to_writer(&mut *output)
                .map_err(FrameDecoderError::FailedToDrainDecodebuffer)?;
            if self.is_finished() {
                // Final flush of the retained window tail.
                self.collect_to_writer(&mut *output)
                    .map_err(FrameDecoderError::FailedToDrainDecodebuffer)?;
                break;
            }
        }
        let produced = (output.len() - frame_start) as u64;
        // A declared content size MUST match what the body produced — otherwise
        // accept the same corrupt frames `decode_all_impl` rejects (e.g. an
        // explicit FCS=0 whose body emits bytes). Use `fcs_declared()` so an
        // on-wire FCS=0 is validated, while an unknown size is not.
        if fcs_declared && produced != content_size {
            return Err(FrameDecoderError::FrameContentSizeMismatch {
                declared: content_size,
                produced,
            });
        }

        self.verify_content_checksum()?;
        Ok(produced as usize)
    }

    /// Default-feature decode_all_impl: no visitor parameter so the
    /// no-lsm build's call surface and codegen are byte-identical to
    /// the original implementation. Compiles only when `lsm` is OFF.

    fn decode_all_impl(
        &mut self,
        mut input: &[u8],
        mut output: &mut [u8],
        mut init_frame: impl FnMut(&mut Self, &mut &[u8]) -> Result<(), FrameDecoderError>,
    ) -> Result<usize, FrameDecoderError> {
        let mut total_bytes_written = 0;
        while !input.is_empty() {
            match init_frame(self, &mut input) {
                Ok(_) => {}
                Err(FrameDecoderError::ReadFrameHeaderError(
                    decoding::errors::ReadFrameHeaderError::SkipFrame { length, .. },
                )) => {
                    input = input
                        .get(length as usize..)
                        .ok_or(FrameDecoderError::FailedToSkipFrame)?;
                    continue;
                }
                Err(e) => return Err(e),
            };
            // Per-frame direct-path dispatch. Now safe to route the
            // public `decode_all` here because
            // `UserSliceBackend::exec_sequence_inline` returns
            // `Result<(), ExecuteSequencesError>` instead of
            // panicking on capacity overflow; the error propagates
            // up as `FrameDecoderError`. Eligibility (FCS > 0,
            // remaining `output` slice holds the declared content)
            // puts the frame on the fast path that bypasses the
            // FlatBuf/Ring -> `read()` drain copy. Ineligible frames
            // (no FCS, output too small) fall through to the legacy
            // `decode_blocks` + `read` drain loop below. Dictionary
            // frames are eligible: `run_direct_decode` hands the
            // shared dict handle to its buffer, and beyond-prefix
            // offsets resolve through `repeat_from_dict`.
            let (content_size, fcs_declared) = {
                let state_ref = self.state.as_ref().expect("init populated state");
                (
                    state_ref.frame_header.frame_content_size(),
                    state_ref.frame_header.fcs_declared(),
                )
            };
            // Direct decode requires only that the caller slice holds the
            // declared content; the inline sequence-exec path no longer
            // needs `WILDCOPY_OVERLENGTH` trailing slack because the
            // trailing sequence(s) take the bounded (non-overshooting)
            // copy in `UserSliceBackend::exec_sequence_bounded`. This is
            // the universal "decode into an FCS-sized buffer" case (a
            // caller sizing `output` to exactly `frame_content_size`),
            // so dropping the slack requirement halves its peak alloc.
            //
            // Per-block checksums collected inside `run_direct_decode`
            // post-loop (over recorded (start, end) ranges of `output`)
            // so the direct path stays eligible AND keeps the
            // window-size cap (`drop_to_window_size`) between blocks
            // that the spec relies on for `offset <= window_size`
            // validation. Path choice no longer alters checksum
            // semantics.
            let direct_eligible = content_size > 0 && (output.len() as u64) >= content_size;
            if direct_eligible {
                let written = self.run_direct_decode(&mut input, output, content_size)?;
                output = &mut output[written..];
                total_bytes_written += written;
                // Per-frame content-checksum verification (no-op unless the
                // mode is `Verify` and the frame carries a checksum).

                self.verify_content_checksum()?;
                continue;
            }
            // Non-direct fallback: pre-reserve the backing buffer to
            // `window_size` in a single allocation before block decode
            // starts, so multi-segment frames don't pay repeated
            // `reserve_amortized` grow steps as blocks accumulate (each
            // block only reserves MAX_BLOCK_SIZE = 128 KiB, so a window
            // > 128 KiB otherwise grows through several intermediate
            // sizes with `alloc_zeroed + memcpy` each time).
            if let Some(state) = self.state.as_mut() {
                // FCS-capped via `useful_window_size` — the same cap
                // `decode_blocks` applies, so its per-iteration reserve in
                // the loop below cannot grow the buffer back to the raw
                // frame window.
                let useful_window = state.useful_window_size();
                state.decoder_scratch.reserve_buffer(useful_window);
            }
            let frame_start_total = total_bytes_written;
            loop {
                self.decode_blocks(&mut input, BlockDecodingStrategy::UptoBytes(1024 * 1024))?;
                let bytes_written = self
                    .read(output)
                    .map_err(FrameDecoderError::FailedToDrainDecodebuffer)?;
                output = &mut output[bytes_written..];
                total_bytes_written += bytes_written;
                if self.can_collect() != 0 {
                    return Err(FrameDecoderError::TargetTooSmall);
                }
                if self.is_finished() {
                    break;
                }
            }
            // Per-frame FCS validation on the legacy fallback path.
            // Use `fcs_declared()` (NOT `content_size > 0`) so an
            // empty frame with explicit FCS=0 on the wire still gets
            // validated.
            if fcs_declared {
                let produced = (total_bytes_written - frame_start_total) as u64;
                if produced != content_size {
                    return Err(FrameDecoderError::FrameContentSizeMismatch {
                        declared: content_size,
                        produced,
                    });
                }
            }
            // Per-frame content-checksum verification on the drain path: the
            // frame is fully decoded and drained here (is_finished + nothing
            // left to collect), so the running digest and stored value are
            // final. No-op unless the mode is `Verify`.

            self.verify_content_checksum()?;
        }

        Ok(total_bytes_written)
    }

    /// Decode multiple frames into the output slice using a serialized dictionary.
    ///
    /// # Warning
    ///
    /// Each decoded frame is initialized with the parsed dictionary, even when a
    /// frame header omits the optional dictionary ID. Callers must only use this
    /// API when they already know the input frames were encoded with that
    /// dictionary; otherwise decoded output can be silently corrupted.
    pub fn decode_all_with_dict_bytes(
        &mut self,
        input: &[u8],
        output: &mut [u8],
        raw_dictionary: &[u8],
    ) -> Result<usize, FrameDecoderError> {
        let dict = DictionaryHandle::decode_dict(raw_dictionary)?;
        self.decode_all_with_dict_handle(input, output, &dict)
    }

    /// Decode multiple frames into the extra capacity of the output vector.
    ///
    /// `input` must contain an exact number of frames.
    ///
    /// `output` must have enough spare capacity to hold the decompressed
    /// data. This adds no extra slack: exact-fit output is now eligible
    /// for the direct decode path, so a `Vec::with_capacity(fcs)` is
    /// decoded straight into without a growth/reallocation. It will NOT
    /// grow the vector to fit the decompressed payload itself; the
    /// caller's pre-allocated capacity must already cover the data. If
    /// you don't know how large the output will be, use
    /// [`FrameDecoder::decode_blocks`] instead.
    ///
    /// This calls [`FrameDecoder::init`], and all bytes currently in the decoder will be lost.
    ///
    /// The length of the output vector is updated to include the
    /// decompressed data. The length is not changed if an error occurs.
    pub fn decode_all_to_vec(
        &mut self,
        input: &[u8],
        output: &mut Vec<u8>,
    ) -> Result<(), FrameDecoderError> {
        let len = output.len();
        let cap = output.capacity();
        output.resize(cap, 0);
        match self.decode_all(input, &mut output[len..]) {
            Ok(bytes_written) => {
                let new_len = core::cmp::min(len + bytes_written, cap); // Sanitizes `bytes_written`.
                output.resize(new_len, 0);
                Ok(())
            }
            Err(e) => {
                output.resize(len, 0);
                Err(e)
            }
        }
    }

    /// Single-frame direct-decode path. Decodes one zstd frame into
    /// `output[..content_size]` via a stack-local
    /// `DecodeBuffer<UserSliceBackend>`, bypassing the per-block
    /// FlatBuf/Ring -> `read()` drain copy.
    ///
    /// # Preconditions (caller-enforced)
    ///
    /// - `self.init` (or `init_with_dict_handle`) was called for
    ///   this frame so `self.state` is populated.
    /// - `content_size` matches `self.state.frame_header
    ///   .frame_content_size()` and is `> 0` (caller already passed
    ///   the eligibility gate).
    /// - `output.len() >= content_size`. No `WILDCOPY_OVERLENGTH`
    ///   trailing slack is required: the trailing sequence(s) take the
    ///   bounded (non-overshooting) copy in
    ///   [`UserSliceBackend::exec_sequence_bounded`].
    ///
    /// Dictionary frames are supported: the scratch buffer's shared
    /// dict handle is forwarded to the stack-local `DecodeBuffer`, so
    /// offsets reaching past the frame's own output resolve through
    /// `repeat_from_dict` (the ext-dict slow path).
    ///
    /// On return, `input` points at the byte immediately after the
    /// frame's checksum (or after the last block, when the frame
    /// has `content_checksum_flag = 0`). `self.state.frame_finished`
    /// is set so [`Self::is_finished`] reports `true`.
    fn run_direct_decode(
        &mut self,
        input: &mut &[u8],
        output: &mut [u8],
        content_size: u64,
    ) -> Result<usize, FrameDecoderError> {
        use super::block_decoder;
        use super::decode_buffer::DecodeBuffer;
        use super::scratch::DirectScratch;
        use super::user_slice_buf::UserSliceBackend;
        use crate::io::Read;
        use FrameDecoderError as err;

        let state = self
            .state
            .as_mut()
            .expect("caller ensures init populated state");

        // Fast path: a frame that is a single RAW block spanning the whole
        // declared content. Upstream zstd handles this as one `ZSTD_copyRawBlock`
        // (a `memmove`) inside `ZSTD_decompressFrame`; do the same here — a direct
        // `copy_from_slice` into the caller's output slice — skipping the
        // `DirectScratch` / `DecodeBuffer` / `UserSliceBackend` wrapper
        // construction and the general per-block loop. Incompressible payloads
        // (random / already-compressed data) emit exactly this shape, so the win
        // lands on small high-entropy frames where the per-frame machinery, not
        // the 1-block copy, dominates.
        {
            let mut probe = *input;
            let mut header_dec = block_decoder::new();
            if let Ok((bh, hsize)) = header_dec.read_block_header(&mut probe) {
                let n = bh.decompressed_size as usize;
                if bh.last_block
                    && matches!(bh.block_type, crate::blocks::block::BlockType::Raw)
                    && n as u64 == content_size
                    && probe.len() >= n
                    && output.len() >= n
                {
                    output[..n].copy_from_slice(&probe[..n]);
                    *input = &probe[n..];
                    state.bytes_read_counter += u64::from(hsize) + n as u64;
                    state.block_counter += 1;
                    // Consume the trailing 4-byte content checksum UNCONDITIONALLY
                    // when the frame declares one — exactly like the general
                    // direct loop and `decode_blocks`. Only the hash SEEDING is
                    // `hash`-gated; the byte consumption / counter / `check_sum`
                    // must not be, or a no-`hash` build leaves the 4 bytes in
                    // `*input` (misparsed as the next frame) and never sets
                    // `check_sum` (so `is_finished` stays false).
                    if state.frame_header.descriptor.content_checksum_flag() {
                        let mut chksum = [0u8; 4];
                        Read::read_exact(input, &mut chksum).map_err(err::FailedToReadChecksum)?;
                        state.bytes_read_counter += 4;
                        state.check_sum = Some(u32::from_le_bytes(chksum));
                        // Mirror the general path: seed the scratch hash so
                        // `verify_content_checksum` / `get_calculated_checksum`
                        // read the digest. Skipped under `ContentChecksum::None`.

                        if self.content_checksum != ContentChecksum::None {
                            use core::hash::Hasher;
                            let mut h = twox_hash::XxHash64::with_seed(0);
                            h.write(&output[..n]);
                            match &mut state.decoder_scratch {
                                DecoderScratchKind::Flat(s) => s.buffer.hash = h,
                                DecoderScratchKind::Ring(s) => s.buffer.hash = h,
                            }
                        }
                    }

                    state.frame_finished = true;
                    return Ok(n);
                }
            }
        }

        // Borrow persistent fields out of whichever scratch variant
        // `init` produced (Flat for single_segment, Ring for
        // multi-segment) — both expose the same HUF/FSE/Vec
        // fields; only `buffer` differs and we don't use that here.
        // Macro-style binding avoids the closure / generic
        // gymnastics of returning multiple `&mut` from a match arm.
        // Resolve the dictionary borrow for this frame BEFORE taking the
        // `&mut` field borrows below — `active_dict` is a disjoint field, so
        // the shared borrow coexists with the mutable scratch borrows. It is
        // threaded as a call-scoped argument into every `Dict`-sourced read
        // (the direct path's `repeat_from_dict` ext-dict slow path), mirroring
        // C's per-frame pointer hand-off with zero refcount churn.
        // Only expose the held dictionary while THIS frame is dict-backed
        // (`using_dict` is set per dict-apply, cleared on reset). A reused
        // decoder keeps `active_dict` across a no-dict frame for the
        // `ptr::eq` reuse-skip, so it must be gated here or a stray
        // out-of-window offset on a dictless frame would resolve against the
        // stale dictionary content instead of erroring.
        let dict_ref = if state.using_dict.is_some() {
            state.active_dict.as_ref().map(|h| h.as_dict())
        } else {
            None
        };
        let (huf, fse, offset_hist, literals_buffer, block_content_buffer, window_size) =
            match &mut state.decoder_scratch {
                DecoderScratchKind::Flat(s) => (
                    &mut s.huf,
                    &mut s.fse,
                    &mut s.offset_hist,
                    &mut s.literals_buffer,
                    &mut s.block_content_buffer,
                    s.buffer.window_size,
                ),
                DecoderScratchKind::Ring(s) => (
                    &mut s.huf,
                    &mut s.fse,
                    &mut s.offset_hist,
                    &mut s.literals_buffer,
                    &mut s.block_content_buffer,
                    s.buffer.window_size,
                ),
            };
        let backend = UserSliceBackend::from_slice(output);
        let buffer = DecodeBuffer::from_backend(backend, window_size);
        let mut direct = DirectScratch {
            huf,
            fse,
            offset_hist,
            literals_buffer,
            block_content_buffer,
            buffer,
        };

        // Block loop. Mirrors `decode_blocks` (without the
        // strategy-bounded early exit — we always decode the whole
        // frame in one shot for the direct path). Keeps
        // `state.bytes_read_counter` / `state.block_counter` in
        // sync with `decode_blocks` so post-call accessors
        // (`bytes_read_from_source`, `blocks_decoded`) return
        // accurate values.
        let mut block_dec = block_decoder::new();
        // Track total output bytes against the declared
        // `frame_content_size` via the buffer's actual write
        // counter — `BlockHeader.decompressed_size` is 0 for
        // Compressed blocks (the header parser can't know the
        // expanded size before decoding the body), so per-header
        // tracking would always count 0 for those blocks and
        // miscount frames that aren't pure Raw/RLE.
        let mut produced: u64 = 0;
        // Per-block output ranges captured during the direct-path
        // loop. After the loop we re-borrow `output` (post-drop of
        // `direct`) and XXH64 each range into
        // `self.computed_block_checksums`, so the digests vector
        // stays consistent with the legacy `decode_blocks` path
        // regardless of which dispatch the frame took.
        // `Vec::new()` does not allocate, so this stays free when
        // `per_block_checksums_enabled` is false: the `push` and the
        // post-loop hashing loop are both gated by the same flag.

        // Frame-level XXH64, accumulated PER BLOCK right after each block
        // decodes — the bytes are still cache-resident then. The previous
        // shape hashed the whole output once after the loop, which re-read
        // the entire frame cold: a full extra memory pass that the
        // reference implementation does not make (it hashes incrementally
        // per block). Invisible on outputs that fit L3, ~1.14x wall on a
        // 100 MiB all-raw decode and the dominant CI gap on
        // bandwidth-limited hosts.

        let mut running_hash: Option<twox_hash::XxHash64> =
            if state.frame_header.descriptor.content_checksum_flag()
                && self.content_checksum != ContentChecksum::None
            {
                Some(twox_hash::XxHash64::with_seed(0))
            } else {
                None
            };
        loop {
            // Failing-block coordinates captured before the header read (see
            // the `decode_blocks` loop for the rationale).
            let block_index = state.block_counter as u32;
            let block_frame_offset = state.bytes_read_counter as u32;
            let (block_header, hsize) =
                block_dec.read_block_header(&mut *input).map_err(|source| {
                    block_header_decode_error(source, block_index, block_frame_offset)
                })?;
            state.bytes_read_counter += u64::from(hsize);
            // Pre-flight FCS check ONLY for Raw / RLE blocks where
            // `decompressed_size` is the actual block output size.
            // For Compressed blocks the header field is 0; the
            // post-decode check below catches overflow via the
            // backend's actual write counter delta.
            let block_upper = u64::from(block_header.decompressed_size);
            if block_upper > 0 && produced + block_upper > content_size {
                // Frame is corrupt — Raw/RLE block headers claim
                // more output than the FCS allows.
                return Err(err::FrameContentSizeMismatch {
                    declared: content_size,
                    produced: produced + block_upper,
                });
            }
            // Slice-source fast path: consume the block body
            // straight from `input` without copying into the
            // persistent `block_content_buffer`.
            let body_consumed = match block_dec.decode_block_content_from_slice(
                &block_header,
                &mut direct,
                dict_ref,
                &mut *input,
            ) {
                Ok(n) => n,
                // Defense-in-depth: RLE / Raw block whose declared
                // `decompressed_size` slipped past the per-block
                // pre-flight above and tripped the backend's
                // fallible write surface.
                Err(DecodeBlockContentError::BackendOverflow { .. }) => {
                    // Use saturating_add on the
                    // `produced + decompressed_size` sum. Each block
                    // is bounded by 128 KiB (MAX_BLOCK_SIZE), but
                    // accumulated `produced` can grow toward
                    // u64::MAX across adversarial frames. Saturating
                    // avoids a panic on the error path itself.
                    return Err(err::FrameContentSizeMismatch {
                        declared: content_size,
                        produced: produced
                            .saturating_add(u64::from(block_header.decompressed_size)),
                    });
                }
                // Compressed-block in-block overshoot: the sequence
                // executor (upstream zstd-inline path) or the match-repeat
                // fallback tripped the fixed-capacity backend's per-write
                // check. Unlike Raw/RLE, a Compressed block carries no
                // header-declared output size, so `produced` is computed
                // from the partial fill: `tail` bytes were written before
                // the failing op, and `requested` is what overflowed —
                // their sum is a strict lower bound on the frame's true
                // expanded size and is always > `content_size` (the
                // direct path is only entered when the slice is sized to
                // `content_size + WILDCOPY_OVERLENGTH`, so any overflow
                // means the frame exceeded the declared FCS, never a
                // caller-undersized buffer). Folds into the same
                // `FrameContentSizeMismatch` contract as Raw/RLE.
                Err(DecodeBlockContentError::DecompressBlockError(
                    decoding::errors::DecompressBlockError::ExecuteSequencesError(ref e),
                )) if e.output_overflow_requested().is_some() => {
                    let requested = e
                        .output_overflow_requested()
                        .expect("guard guarantees Some") as u64;
                    let tail = direct.buffer.buffer_ref().tail() as u64;
                    return Err(err::FrameContentSizeMismatch {
                        declared: content_size,
                        produced: tail.saturating_add(requested),
                    });
                }
                Err(e) => {
                    return Err(block_body_decode_error(
                        e,
                        block_index,
                        block_frame_offset,
                        &block_header,
                        hsize,
                    ));
                }
            };
            // Hash this block's freshly-written bytes while they are hot
            // (see `running_hash` above). `tail()` is the physical write
            // cursor: `drop_to_window_size` below only advances the head,
            // so `[prev_tail, tail)` is exactly this block's output.

            if let Some(hasher) = running_hash.as_mut() {
                use core::hash::Hasher;
                hasher.write(direct.buffer.buffer_ref().written_since(produced as usize));
            }
            produced = direct.buffer.buffer_ref().tail() as u64;
            // Post-decode FCS overflow check.
            if produced > content_size {
                return Err(err::FrameContentSizeMismatch {
                    declared: content_size,
                    produced,
                });
            }
            state.bytes_read_counter += body_consumed;
            state.block_counter += 1;

            // Cap the visible buffer at window_size between blocks
            // so the next block's match-offset validation matches
            // the spec's `offset <= window_size` rule.
            direct.buffer.drop_to_window_size();
            if block_header.last_block {
                if state.frame_header.descriptor.content_checksum_flag() {
                    let mut chksum = [0u8; 4];
                    input
                        .read_exact(&mut chksum)
                        .map_err(err::FailedToReadChecksum)?;
                    state.bytes_read_counter += 4;
                    state.check_sum = Some(u32::from_le_bytes(chksum));
                }
                break;
            }
        }
        // Final sanity: blocks summed to exactly `content_size`.
        if produced != content_size {
            return Err(err::FrameContentSizeMismatch {
                declared: content_size,
                produced,
            });
        }

        let written = content_size as usize;
        state.frame_finished = true;
        // `direct`'s last use is in the decode loop above; NLL therefore
        // releases its `&mut output` borrow before here, freeing `output` for
        // the hash re-borrow below. No explicit `drop(direct)` is needed:
        // `DirectScratch` now holds only borrowed dict POINTERS (not an owned
        // `Arc`), so it is not a `Drop` type whose glue would hold the borrow
        // to end-of-scope.
        // Per-block XXH64 (low 32 bits) over the captured ranges.
        // Mirrors `decode_blocks`' per-block hashing so the digests
        // vector stays identical regardless of which dispatch path
        // the frame took. Ranges were recorded inside the loop while
        // `direct` held a mutable borrow on `output`; now that the
        // borrow is dropped we can read the slices directly.

        if let Some(hasher) = running_hash {
            // Propagate the per-block-accumulated hasher state (see the
            // `running_hash` rationale above the loop) so the frame-tail
            // XXH64 check and `get_calculated_checksum()` read the digest.
            // `running_hash` is `None` for flag-off frames or
            // `ContentChecksum::None` — nothing to verify there, and
            // `get_calculated_checksum()` returns `None`, matching the skip.
            match &mut state.decoder_scratch {
                DecoderScratchKind::Flat(s) => s.buffer.hash = hasher,
                DecoderScratchKind::Ring(s) => s.buffer.hash = hasher,
            }
        }
        Ok(written)
    }
}

/// Read bytes from the decode_buffer that are no longer needed. While the frame is not yet finished
/// this will retain window_size bytes, else it will drain it completely
impl Read for FrameDecoder {
    fn read(&mut self, target: &mut [u8]) -> Result<usize, Error> {
        let state = match &mut self.state {
            None => return Ok(0),
            Some(s) => s,
        };
        if state.frame_finished {
            state.decoder_scratch.buffer_read_all(target)
        } else {
            state.decoder_scratch.buffer_read(target)
        }
    }
}
