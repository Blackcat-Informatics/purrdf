#![allow(
    clippy::elidable_lifetime_names,
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Structures that wrap around various decoders to make decoding easier.

use super::buffer_backend::BufferBackend;
use super::decode_buffer::DecodeBuffer;
use super::ringbuffer::RingBuffer;
use crate::decoding::dictionary::{Dictionary, DictionaryHandle};
use crate::fse::SeqFSETable;
use crate::huff0::HuffmanTable;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use crate::blocks::sequence_section::{
    MAX_LITERAL_LENGTH_CODE, MAX_MATCH_LENGTH_CODE, MAX_OFFSET_CODE,
};

/// A block level decoding buffer, parameterised over the output
/// storage backend ([`BufferBackend`]). Default `RingBuffer` keeps
/// the historical API; `DecoderScratch<FlatBuf>` is instantiated by
/// [`super::frame_decoder::FrameDecoder`] (via `DecoderScratchKind`)
/// when the frame's `Single_Segment_flag` is set.
pub(crate) struct DecoderScratch<B: BufferBackend = RingBuffer> {
    /// The decoder used for Huffman blocks.
    pub huf: HuffmanScratch,
    /// The decoder used for FSE blocks.
    pub fse: FSEScratch,

    pub buffer: DecodeBuffer<B>,
    pub offset_hist: [u32; 3],

    pub literals_buffer: Vec<u8>,
    pub block_content_buffer: Vec<u8>,
}

/// Borrowed view of all per-call decoder scratch fields as `&mut`
/// references. Returned by [`Workspace::split`] so the block /
/// literals / sequence decoder functions can hold simultaneous
/// independent borrows of distinct fields — the field-split is
/// what makes "borrow huf and literals_buffer at the same time"
/// type-check, both for the owned [`DecoderScratch<B>`] path and the
/// direct-decode path where these fields are borrowed by reference
/// from a [`crate::decoding::FrameDecoder`].
///
/// The lifetime `'a` is the shorter-of (a) the underlying owner's
/// lifetime and (b) the active borrow. The backend type `B` flows
/// through to [`super::decode_buffer::DecodeBuffer<B>`].
pub(crate) struct WorkspaceRef<'a, B: BufferBackend> {
    pub huf: &'a mut HuffmanScratch,
    pub fse: &'a mut FSEScratch,
    pub buffer: &'a mut DecodeBuffer<B>,
    pub offset_hist: &'a mut [u32; 3],
    pub literals_buffer: &'a mut Vec<u8>,
    pub block_content_buffer: &'a mut Vec<u8>,
}

/// Polymorphic accessor for the decoder's per-call scratch state.
/// Both the owned [`DecoderScratch<B>`] (used by the streaming and
/// one-shot `decode_all` paths) and the borrow-ref direct-decode
/// scratch (`DirectScratch`) implement this trait, so the block /
/// literals / sequence decode functions are written once against
/// `Workspace` and instantiated for both shapes via compile-time
/// monomorphisation.
///
/// The single `split` method returns all fields at once as a
/// [`WorkspaceRef`] so callers retain Rust's field-level
/// disjoint-borrow analysis. Per-field accessors would force
/// sequential borrows and break call sites that need e.g.
/// `&mut huf` and `&mut literals_buffer` simultaneously.
pub(crate) trait Workspace {
    type Backend: BufferBackend;
    fn split(&mut self) -> WorkspaceRef<'_, Self::Backend>;
}

impl<B: BufferBackend> Workspace for DecoderScratch<B> {
    type Backend = B;
    fn split(&mut self) -> WorkspaceRef<'_, B> {
        WorkspaceRef {
            huf: &mut self.huf,
            fse: &mut self.fse,
            buffer: &mut self.buffer,
            offset_hist: &mut self.offset_hist,
            literals_buffer: &mut self.literals_buffer,
            block_content_buffer: &mut self.block_content_buffer,
        }
    }
}

/// Direct-decode scratch: per-call workspace that wraps a
/// stack-local [`DecodeBuffer<UserSliceBackend<'o>>`] over the
/// caller's `&'o mut [u8]` output slice, plus `&'p mut` borrows of
/// the persistent decoder state (HUF / FSE tables, offset_hist,
/// sequence cache, scratch Vecs) owned by [`crate::decoding::FrameDecoder`].
///
/// The lifetime split:
/// - `'o` — caller's output slice (borrowed via `buffer`).
/// - `'p` — FrameDecoder's persistent fields (borrowed via the
///   `&'p mut` fields).
///
/// Implementing [`Workspace`] lets the existing
/// `block_decoder::decode_block_content` / `decompress_block`
/// generic-over-W functions consume this scratch unchanged. The
/// perf rationale: eliminating the `DecodeBuffer::read` drain copy
/// that the owned-buffer path performs, by writing decoded bytes
/// straight into the caller-provided output slice.
///
/// Constructed inside `FrameDecoder::decode_all` and dropped
/// at function exit; never persisted across calls.
pub(crate) struct DirectScratch<'o, 'p> {
    pub huf: &'p mut HuffmanScratch,
    pub fse: &'p mut FSEScratch,
    pub buffer: DecodeBuffer<super::user_slice_buf::UserSliceBackend<'o>>,
    pub offset_hist: &'p mut [u32; 3],
    pub literals_buffer: &'p mut Vec<u8>,
    pub block_content_buffer: &'p mut Vec<u8>,
}

impl<'o, 'p> Workspace for DirectScratch<'o, 'p> {
    type Backend = super::user_slice_buf::UserSliceBackend<'o>;
    fn split(&mut self) -> WorkspaceRef<'_, Self::Backend> {
        // Reborrow the `&'p mut` fields to `&'_ mut` so the returned
        // WorkspaceRef's lifetime is tied to the `&mut self` of this
        // call, not to `'p`. This is what lets nested decode
        // functions hold a WorkspaceRef without freezing the whole
        // `'p`-bound DirectScratch for their entire scope.
        WorkspaceRef {
            huf: &mut *self.huf,
            fse: &mut *self.fse,
            buffer: &mut self.buffer,
            offset_hist: &mut *self.offset_hist,
            literals_buffer: &mut *self.literals_buffer,
            block_content_buffer: &mut *self.block_content_buffer,
        }
    }
}

impl<B: BufferBackend> DecoderScratch<B> {
    pub(crate) fn new(window_size: usize) -> DecoderScratch<B> {
        DecoderScratch {
            huf: HuffmanScratch {
                table: HuffmanTable::new(),
                table_source: TableSource::Local,
            },
            fse: FSEScratch {
                offsets: AlignedFSETable::new(MAX_OFFSET_CODE),
                literal_lengths: AlignedFSETable::new(MAX_LITERAL_LENGTH_CODE),
                match_lengths: AlignedFSETable::new(MAX_MATCH_LENGTH_CODE),
                offsets_long_share: 0,
                ddict_is_cold: false,
                ll_source: TableSource::Local,
                of_source: TableSource::Local,
                ml_source: TableSource::Local,
            },
            buffer: DecodeBuffer::new(window_size),
            offset_hist: [1, 4, 8],

            block_content_buffer: Vec::new(),
            literals_buffer: Vec::new(),
        }
    }

    /// Total heap bytes this scratch holds: the decode-window buffer plus the
    /// per-block literal and block-content buffers and the entropy tables. The
    /// window dominates and scales with the frame; the rest are bounded by the
    /// block maximum and the entropy alphabet.
    pub(crate) fn workspace_bytes(&self) -> usize {
        self.buffer.capacity()
            + self.literals_buffer.capacity()
            + self.block_content_buffer.capacity()
            + self.huf.heap_bytes()
            + self.fse.heap_bytes()
    }

    pub(crate) fn reset(&mut self, window_size: usize) {
        self.offset_hist = [1, 4, 8];
        self.literals_buffer.clear();
        self.block_content_buffer.clear();

        // Pre-allocate the per-block scratch Vecs to `min(window_size,
        // MAX_BLOCK_SIZE)` so the first block's
        // `extend_from_slice` / `resize` does not pay anonymous-page
        // first-touch faults inside the decode hot path. `clear()`
        // keeps `capacity()`, so subsequent frames with the same
        // (or smaller) window also avoid realloc. Matches upstream zstd's
        // upfront sizing strategy where `dctx->litExtraBuffer` and
        // the dst layout are sized to `blockSizeMax` at frame init.
        // Measured at ~18% of decode-time page-fault cost on
        // level_-7_fast/decodecorpus-z000033.
        let block_cap = (window_size.min(crate::common::MAX_BLOCK_SIZE as usize)).max(8);
        // Pre-TOUCH (not just reserve) so the kernel maps the
        // anonymous pages here instead of inside the decode hot
        // path. `Vec::reserve` only allocates address space; the
        // first byte-write to each 4 KiB page still triggers a
        // page fault.
        //
        // ONLY when the Vec's capacity is below the target — once
        // a frame has touched the pages once, `clear()` keeps both
        // `capacity()` AND the kernel's anonymous-page mapping, so
        // subsequent frames hit warm memory without re-zeroing.
        // The previous shape (`resize` + `clear` unconditionally)
        // paid an O(block_cap) memset every frame reset, ~37 µs
        // per 128 KiB at AVX2 store rates. Now it's only paid on
        // the very first reset (or after a grow to larger
        // window_size).
        //
        // This matches upstream zstd's `dctx->litExtraBuffer` /
        // `dctx->workspace` lifecycle — touched once at decoder
        // construction, warm across all subsequent frames.
        if self.literals_buffer.capacity() < block_cap {
            self.literals_buffer.resize(block_cap, 0);
            self.literals_buffer.clear();
        }
        if self.block_content_buffer.capacity() < block_cap {
            self.block_content_buffer.resize(block_cap, 0);
            self.block_content_buffer.clear();
        }

        self.buffer.reset(window_size);

        // Mirror upstream zstd's `ZSTD_decompressBegin`: it never clears the
        // entropy tables per frame (marks them invalid by flag, rebuilds lazily).
        // Resetting an already-empty table is a no-op on the observable state
        // (`accuracy_log`/`max_num_bits` stay 0, the buffers stay empty), so a
        // RAW / RLE / zero-sequence frame — which never builds a table — skips
        // the clears (and the Huffman `[0; 13]` rank-count memset) entirely.
        // Byte-identical: the table is cleared exactly when it was built.
        if self.fse.literal_lengths.is_populated() {
            self.fse.literal_lengths.reset();
        }
        if self.fse.match_lengths.is_populated() {
            self.fse.match_lengths.reset();
        }
        if self.fse.offsets.is_populated() {
            self.fse.offsets.reset();
        }
        // Reset the cached pipeline-gate signal alongside the FSE
        // table reset — otherwise scratch reuse across frames could
        // engage the long pipeline on a new frame's Repeat-mode
        // header based on the previous frame's offset distribution
        // (or vice versa: skip the pipeline when the new frame
        // actually has long offsets).
        self.fse.offsets_long_share = 0;
        // Revert any dictionary copy-on-write attachment: a scratch
        // reused from a dict-attached frame must not read the previous
        // dictionary's tables on the next (possibly dict-less) frame.
        self.fse.detach_dict();
        // Pair the one-shot cold-dict flag with `reset`: a scratch
        // reused from a dictionary-attached frame whose blocks never
        // entered sequence decoding (raw-/RLE-only blocks, zero-seq
        // compressed blocks) would otherwise carry the flag into the
        // next frame and mis-apply the cold-dict gate there. Cleared
        // alongside `offsets_long_share` so the no-dict path keeps
        // the documented "no behaviour change" property.
        self.fse.ddict_is_cold = false;

        // Same lazy-entropy skip as the FSE tables above: a Huffman table that
        // was never built this round (`max_num_bits == 0`) is already in its
        // reset state, so re-running `reset` (clearing 6 buffers + the
        // `[0; 13]` rank-count memset) is wasted no-op work on RAW / RLE / raw-
        // literal frames. Byte-identical.
        if self.huf.table.max_num_bits != 0 {
            self.huf.table.reset();
        }
        // Mirror the FSE detach: a reused workspace must not read a
        // previous frame's dictionary Huffman table.
        self.huf.detach_dict();
    }

    pub(crate) fn init_from_dict(&mut self, dict: &DictionaryHandle) {
        let d = dict.as_dict();
        // Copy-on-write: the sequence FSE / Huffman tables are READ from the
        // dictionary by reference (Repeat mode) or rebuilt locally (FSE / RLE /
        // Predefined / Compressed mode) — never eagerly copied here (the upstream
        // zstd `ZSTD_copyDDictParameters` memcpy is elided). This only arms the
        // COW source flags + copies the scalar `offset_hist`; the dictionary
        // itself is threaded into every read as a call-scoped borrow, so it is
        // NOT stored and there is NO per-frame refcount bump.
        self.fse.attach_dict(dict);
        self.huf.attach_dict();
        self.offset_hist = d.offset_hist;
        // Upstream zstd parity: `ZSTD_decompressBegin_usingDDict` sets
        // `dctx->ddictIsCold = 1` so the first block of the frame
        // engages the prefetch decoder regardless of long-offset
        // share. We do the same here; the first
        // `decode_and_execute_sequences` call consumes the flag and
        // resets it to `false`.
        self.fse.ddict_is_cold = true;
    }
}

#[derive(Clone)]
pub struct HuffmanScratch {
    pub table: HuffmanTable,
    /// Copy-on-write source for the literals Huffman table, mirroring the
    /// sequence-FSE treatment in [`FSEScratch`]: `Dict` reads the shared
    /// dictionary's table by reference (no copy), `Local` reads the
    /// locally-built one. `init_from_dict` attaches as `Dict`; a
    /// `Compressed` literals section rebuilds and flips to `Local`; a
    /// `Treeless` section reuses whatever source is current.
    /// The `Dict`-sourced table reads the dictionary's Huffman table through a
    /// call-scoped `dict: Option<&Dictionary>` borrow threaded into
    /// [`Self::huf_table`] by the decode pipeline — no stored reference, no
    /// refcount. See [`FSEScratch`] for the shared model.
    table_source: TableSource,
}

impl HuffmanScratch {
    pub fn new() -> HuffmanScratch {
        HuffmanScratch {
            table: HuffmanTable::new(),
            table_source: TableSource::Local,
        }
    }

    /// Heap bytes owned by this scratch: the locally-built Huffman table.
    /// A `Dict`-sourced table is read through a shared, ref-counted handle
    /// (not owned here), so it is excluded, mirroring upstream not charging
    /// `refDDict` memory to the decode context.
    pub fn heap_bytes(&self) -> usize {
        self.table.heap_bytes()
    }

    /// Live Huffman literals table: the dictionary's (zero-copy) while the
    /// source is still `Dict`, else the locally-built one. `dict` is the
    /// call-scoped borrow threaded in by the decode pipeline (must be `Some`
    /// when the source is `Dict`).
    pub(crate) fn huf_table<'a>(&'a self, dict: Option<&'a Dictionary>) -> &'a HuffmanTable {
        match self.table_source {
            TableSource::Local => &self.table,
            TableSource::Dict => &expect_dict(dict).huf.table,
        }
    }

    /// Point the literals table at the dictionary's Huffman table (no table
    /// bytes copied): re-arm the COW source only. The dictionary is read later
    /// through the threaded `dict` borrow, never stored — NO refcount bump.
    pub(crate) fn attach_dict(&mut self) {
        // The COW table-source must be re-armed every frame (a prior frame's
        // `Compressed` literals section may have flipped it to `Local`).
        self.table_source = TableSource::Dict;
    }

    /// Drop any dictionary attachment and revert to the local table.
    pub(crate) fn detach_dict(&mut self) {
        self.table_source = TableSource::Local;
    }

    /// Flip to the locally-built table (called after a `Compressed`
    /// literals section rebuilds it — the copy-on-write "write" step).
    #[inline]
    pub(crate) fn mark_table_local(&mut self) {
        self.table_source = TableSource::Local;
    }

    /// Snapshot the live (COW-resolved) Huffman table into `self` as an
    /// owned `Local` copy (LSM resume snapshot/restore): materialises a
    /// `Dict`-sourced table so the result is self-contained.
    pub(crate) fn reinit_resolved_from(
        &mut self,
        other: &HuffmanScratch,
        dict: Option<&Dictionary>,
    ) {
        self.table.reinit_from(other.huf_table(dict));
        self.detach_dict();
    }
}

purrdf_hash::default_from_new!(HuffmanScratch);

/// Whether an entropy table (a sequence FSE axis, or the Huffman
/// literals table) reads its own freshly-built table (`Local`) or the
/// shared dictionary's table by reference (`Dict`). The decode
/// copy-on-write source: see [`FSEScratch`] / [`HuffmanScratch`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TableSource {
    Local,
    Dict,
}

/// Unwrap the call-scoped dictionary borrow at a `Dict`-sourced read. A `Dict`
/// table source is only ever set by `attach_dict`, which the decode pipeline
/// pairs with threading the active dictionary into every read — so `None` here
/// is an internal invariant violation, not a recoverable input error.
#[inline]
fn expect_dict(dict: Option<&Dictionary>) -> &Dictionary {
    dict.expect("a Dict-sourced table requires the active dictionary borrow")
}

#[derive(Clone)]
pub struct FSEScratch {
    pub offsets: AlignedFSETable,
    pub literal_lengths: AlignedFSETable,
    pub match_lengths: AlignedFSETable,
    /// Cached "share of offset codes strictly > LONG_OFFSET_CODE_THRESHOLD
    /// (i.e. codes ≥ 23 when the threshold is 22)" scaled to upstream zstd's
    /// `OffFSELog = 8` (256-entry reference).
    /// Updated by [`crate::decoding::sequence_section_decoder`] when
    /// the offsets FSE table is rebuilt (FSE / Predefined modes);
    /// stale-but-correct on Repeat-mode blocks where the table was
    /// not touched — the share is identical to the previous block's.
    /// The sequence-section pipeline gate reads this directly instead
    /// of re-walking `offsets.decode` per block.
    pub offsets_long_share: u32,
    /// Mirrors upstream zstd `ZSTD_DCtx::ddictIsCold`. Set to `true` when a
    /// dictionary is freshly attached (its FSE / HUF tables are not
    /// yet in cache); the first sequence-section decode of the
    /// resulting frame engages the pipelined prefetch decoder
    /// regardless of long-offset share, then clears the flag so
    /// subsequent blocks fall back to the `offsets_long_share`
    /// heuristic. The `num_sequences >= ADVANCE * 2` guard still
    /// applies: blocks too small to fill the lookahead pipeline take
    /// the short-block fallback in both the cold-dict and warm cases.
    /// Without a dictionary the flag stays `false` (cache state of the
    /// predefined and repeat tables is not considered "cold" in the
    /// upstream zstd model).
    pub ddict_is_cold: bool,
    /// Copy-on-write source for each sequence FSE table axis. After
    /// [`DecoderScratch::init_from_dict`] all three point at the shared
    /// dictionary (`Dict`) with **no table bytes copied** (the upstream zstd's
    /// eager `ZSTD_copyDDictParameters` memcpy is elided); a block that
    /// rebuilds an axis (FSE_Compressed / RLE / Predefined mode) writes
    /// the local `AlignedFSETable` and flips that axis to `Local`.
    /// Repeat-mode blocks leave the source untouched, so they read
    /// straight out of the shared dictionary handle until the first
    /// rebuild. On the no-dict path every axis stays `Local`.
    /// The `Dict`-sourced axes read the dictionary's tables through a
    /// call-scoped `dict: Option<&Dictionary>` borrow threaded into the read
    /// accessors by the decode pipeline (resolved once per decode from the
    /// owner — the `FrameDecoder` registry or `FrameDecoderState::active_dict`).
    /// No dictionary reference is STORED here: the borrow checker guarantees the
    /// dictionary outlives every read, with no refcount and no per-frame clone.
    ll_source: TableSource,
    of_source: TableSource,
    ml_source: TableSource,
}

impl FSEScratch {
    /// Heap bytes owned by the three locally-built sequence FSE tables
    /// (LL/ML/OF). The fixed-size decode arrays are inline (counted by
    /// `size_of`); this sums their build-scratch vectors. `Dict`-sourced
    /// tables read a shared handle and are not owned here.
    pub fn heap_bytes(&self) -> usize {
        self.offsets.heap_bytes()
            + self.literal_lengths.heap_bytes()
            + self.match_lengths.heap_bytes()
    }

    pub fn new() -> FSEScratch {
        FSEScratch {
            offsets: AlignedFSETable::new(MAX_OFFSET_CODE),
            literal_lengths: AlignedFSETable::new(MAX_LITERAL_LENGTH_CODE),
            match_lengths: AlignedFSETable::new(MAX_MATCH_LENGTH_CODE),
            offsets_long_share: 0,
            ddict_is_cold: false,
            ll_source: TableSource::Local,
            of_source: TableSource::Local,
            ml_source: TableSource::Local,
        }
    }

    /// Snapshot the live (COW-resolved) sequence tables into `self` as
    /// owned `Local` copies. Used by the LSM resume snapshot/restore
    /// path (`FrameDecoder::export_entropy` / `restore_entropy`): the
    /// result must be self-contained, so any `Dict`-sourced axis in
    /// `other` is materialised by copying the dictionary's table bytes
    /// into the local buffer and the source is set to `Local`.
    pub fn reinit_from(&mut self, other: &Self, dict: Option<&Dictionary>) {
        self.literal_lengths.reinit_from(other.ll_table(dict));
        self.offsets.reinit_from(other.of_table(dict));
        self.match_lengths.reinit_from(other.ml_table(dict));
        // Copy the precomputed long-offset share instead of re-walking
        // the offsets table; the dict computes it once at build time and
        // it is stale-but-correct across Repeat-mode blocks.
        self.offsets_long_share = other.offsets_long_share;
        // Clear the cold-dict pipeline gate: a local-only snapshot has no
        // dictionary attached, so carrying a stale `true` here would mis-arm
        // the prefetch pipeline on the restored frame.
        self.ddict_is_cold = false;
        self.ll_source = TableSource::Local;
        self.of_source = TableSource::Local;
        self.ml_source = TableSource::Local;
    }

    /// Live LL decode table: the dictionary's (zero-copy) when the axis is
    /// still `Dict`-sourced, else the locally-built one. `dict` is the
    /// call-scoped borrow threaded in by the decode pipeline; it must be
    /// `Some` whenever any axis is `Dict`-sourced (the caller resolves it from
    /// the active dictionary before reading).
    pub(crate) fn ll_table<'a>(&'a self, dict: Option<&'a Dictionary>) -> &'a SeqFSETable {
        match self.ll_source {
            TableSource::Local => &self.literal_lengths,
            TableSource::Dict => &expect_dict(dict).fse.literal_lengths,
        }
    }

    /// Live OF decode table (see [`Self::ll_table`]).
    pub(crate) fn of_table<'a>(&'a self, dict: Option<&'a Dictionary>) -> &'a SeqFSETable {
        match self.of_source {
            TableSource::Local => &self.offsets,
            TableSource::Dict => &expect_dict(dict).fse.offsets,
        }
    }

    /// Live ML decode table (see [`Self::ll_table`]).
    pub(crate) fn ml_table<'a>(&'a self, dict: Option<&'a Dictionary>) -> &'a SeqFSETable {
        match self.ml_source {
            TableSource::Local => &self.match_lengths,
            TableSource::Dict => &expect_dict(dict).fse.match_lengths,
        }
    }

    /// Point every sequence FSE axis at the dictionary's tables (no table bytes
    /// copied — the eager per-frame entropy-table memcpy is elided). Only the
    /// COW source flags + the precomputed long-offset share are set; the
    /// dictionary itself is read later through the threaded `dict` borrow, never
    /// stored — so this costs NO refcount bump.
    pub(crate) fn attach_dict(&mut self, dict: &DictionaryHandle) {
        self.offsets_long_share = dict.as_dict().fse.offsets_long_share;
        // Re-arm all three COW axes every frame (a prior frame may have rebuilt
        // any of them to `Local`).
        self.ll_source = TableSource::Dict;
        self.of_source = TableSource::Dict;
        self.ml_source = TableSource::Dict;
    }

    /// Revert all axes to `Local` (called on scratch `reset` so a reused
    /// workspace does not read a previous frame's dictionary tables).
    pub(crate) fn detach_dict(&mut self) {
        self.ll_source = TableSource::Local;
        self.of_source = TableSource::Local;
        self.ml_source = TableSource::Local;
    }

    /// Flip an axis to read its locally-built table (called by
    /// `maybe_update_fse_tables` after FSE_Compressed / RLE / Predefined
    /// rebuilds — the copy-on-write "write" step).
    #[inline]
    pub(crate) fn mark_ll_local(&mut self) {
        self.ll_source = TableSource::Local;
    }
    #[inline]
    pub(crate) fn mark_of_local(&mut self) {
        self.of_source = TableSource::Local;
    }
    #[inline]
    pub(crate) fn mark_ml_local(&mut self) {
        self.ml_source = TableSource::Local;
    }
}

purrdf_hash::default_from_new!(FSEScratch);

// Keep LL/ML/OF table *objects* cache-line aligned to avoid cross-table placement
// effects in DecoderScratch when they are accessed in the same decode hot loop.
// Note: this aligns the table containers, not the `Vec<SeqSymbol>` backing allocations.
#[cfg_attr(target_arch = "aarch64", repr(align(128)))]
#[cfg_attr(not(target_arch = "aarch64"), repr(align(64)))]
#[derive(Clone)]
pub struct AlignedFSETable(SeqFSETable);

impl AlignedFSETable {
    fn new(max_symbol: u8) -> Self {
        Self(SeqFSETable::new(max_symbol))
    }
}

impl Deref for AlignedFSETable {
    type Target = SeqFSETable;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for AlignedFSETable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
