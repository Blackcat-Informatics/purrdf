#![allow(
    clippy::cast_lossless,
    clippy::checked_conversions,
    clippy::needless_pass_by_value,
    clippy::redundant_closure_for_method_calls,
    clippy::struct_excessive_bools,
    clippy::wildcard_imports,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Utilities and interfaces for encoding an entire frame. Allows reusing resources

use alloc::vec::Vec;
use core::convert::TryInto;

use twox_hash::XxHash64;

use core::hash::Hasher;

use super::{
    CompressionLevel, Matcher, block_header::BlockHeader, frame_header::FrameHeader, levels::*,
    match_generator::MatchGeneratorDriver,
};
use crate::common::MAX_BLOCK_SIZE;
use crate::fse::fse_encoder::{FSETable, default_ll_table, default_ml_table, default_of_table};

use crate::io::{Read, Write};

/// A dictionary prepared for the ENCODER side, analogous to zstd's `CDict`
/// (vs the decoder's [`Dictionary`](crate::decoding::Dictionary) / `DDict`).
///
/// It carries the entropy tables, content, and repeat-offset history the
/// compressor needs, but is a distinct type with **no decode path**: there is
/// no way to turn it into a [`DictionaryHandle`](crate::decoding::DictionaryHandle)
/// or feed it to a [`FrameDecoder`](crate::decoding::FrameDecoder). That keeps
/// the compress-only state (which may have been parsed without building the
/// decode lookup tables, see
/// [`set_dictionary_from_bytes`](FrameCompressor::set_dictionary_from_bytes))
/// from ever reaching the decode side — the encoder/decoder dictionary split
/// mirrors C zstd's `CDict` / `DDict`.
#[derive(Clone)]
pub struct EncoderDictionary {
    pub(crate) inner: crate::decoding::Dictionary,
}

impl EncoderDictionary {
    /// Wrap an already-parsed [`Dictionary`](crate::decoding::Dictionary) for
    /// encoder use. A fully-decoded dictionary is valid here; only the encoder
    /// entropy tables, content, and offset history are read.
    pub fn from_dictionary(dictionary: crate::decoding::Dictionary) -> Self {
        Self { inner: dictionary }
    }

    /// Parse a serialized dictionary blob for encoder use, skipping the decode
    /// lookup-table build the encoder never reads (see
    /// `Dictionary::decode_dict_for_encoding`). The encoder entropy tables — and
    /// thus the emitted frame — are identical to a full parse.
    pub fn from_bytes(
        raw_dictionary: &[u8],
    ) -> Result<Self, crate::decoding::errors::DictionaryDecodeError> {
        Ok(Self {
            inner: crate::decoding::Dictionary::decode_dict_for_encoding(raw_dictionary)?,
        })
    }

    /// The dictionary id.
    ///
    /// A dictionary attached for encoding always has a non-zero id (the
    /// `set_dictionary*` / `set_encoder_dictionary` attach path rejects a
    /// zero id). This getter, however, reflects the wrapped dictionary as-is:
    /// an `EncoderDictionary` built via [`Self::from_dictionary`] from a raw
    /// `Dictionary` with `id == 0` reports `0` here until it is attached.
    pub fn id(&self) -> u32 {
        self.inner.id
    }
}

/// An interface for compressing arbitrary data with the ZStandard compression algorithm.
///
/// `FrameCompressor` will generally be used by:
/// 1. Initializing a compressor by providing a buffer of data using `FrameCompressor::new()`
/// 2. Starting compression and writing that compression into a vec using `FrameCompressor::begin`
///
/// # Examples
/// ```
/// use purrdf_zstd::encoding::{FrameCompressor, CompressionLevel};
/// let mock_data: &[_] = &[0x1, 0x2, 0x3, 0x4];
/// let mut output = std::vec::Vec::new();
/// // Initialize a compressor.
/// let mut compressor = FrameCompressor::new(CompressionLevel::Uncompressed);
/// compressor.set_source(mock_data);
/// compressor.set_drain(&mut output);
///
/// // `compress` writes the compressed output into the provided buffer.
/// compressor.compress();
/// ```
pub struct FrameCompressor<
    R: Read = &'static [u8],
    W: Write = Vec<u8>,
    M: Matcher = MatchGeneratorDriver,
> {
    uncompressed_data: Option<R>,
    compressed_data: Option<W>,
    compression_level: CompressionLevel,
    dictionary: Option<EncoderDictionary>,
    dictionary_entropy_cache: Option<CachedDictionaryEntropy>,
    source_size_hint: Option<u64>,
    state: CompressState<M>,
    /// When true, emitted frames omit the 4-byte magic number prefix
    /// (`ZSTD_f_zstd1_magicless`). Default false. The caller is
    /// responsible for ensuring the decoder is configured for the
    /// matching format — wire-format only round-trips with a
    /// magicless-aware decoder.
    magicless: bool,
    /// Whether to emit a trailing XXH64 content checksum and set the frame
    /// header's `Content_Checksum_flag` (semantics of upstream
    /// `ZSTD_c_checksumFlag`). Default `false`, matching the upstream
    /// library default; combined with the `hash` feature at frame-build
    /// time, so without `hash` no checksum is emitted regardless. Set via
    /// [`Self::set_content_checksum`].
    content_checksum: bool,
    /// Whether to record `Frame_Content_Size` in the frame header when the
    /// total size is known (semantics of upstream `ZSTD_c_contentSizeFlag`).
    /// Default `true`, matching upstream. With the flag off the header
    /// carries a window descriptor instead (single-segment requires an FCS,
    /// so it is disabled too). Set via [`Self::set_content_size_flag`].
    content_size_flag: bool,
    /// Whether to record the dictionary ID in the frame header when a
    /// dictionary is attached (semantics of upstream `ZSTD_c_dictIDFlag`).
    /// Default `true`, matching upstream. Decoders can still decode the
    /// frame by being handed the right dictionary explicitly. Set via
    /// [`Self::set_dictionary_id_flag`].
    dict_id_flag: bool,
    /// Upper bound on emitted block sizes (semantics of upstream
    /// `ZSTD_c_targetCBlockSize`): capping the RAW block length at the
    /// target bounds every physical block's compressed payload at the
    /// target too (a compressed block never exceeds its raw input — the
    /// raw-block fallback fires otherwise), so blocks land at or under
    /// `target + 3` header bytes on the wire. `None` = no target (full
    /// 128 KiB blocks). Set via [`Self::set_target_block_size`].
    target_block_size: Option<u32>,

    hasher: XxHash64,

    /// Effective strategy tag when a public-parameter
    /// [`Strategy`](crate::encoding::Strategy) override is active.
    /// `Some` overrides the level-derived `state.strategy_tag` so the
    /// literal-compression gates and dict-attach cutoff see the strategy
    /// the matcher actually runs, not the base level's. `None` keeps the
    /// level-derived tag.
    strategy_override: Option<crate::encoding::strategy::StrategyTag>,
}

#[derive(Clone, Default)]
pub(crate) struct CachedDictionaryEntropy {
    pub(crate) huff: Option<crate::huff0::huff0_encoder::HuffmanTable>,
    pub(crate) ll_previous: Option<PreviousFseTable>,
    pub(crate) ml_previous: Option<PreviousFseTable>,
    pub(crate) of_previous: Option<PreviousFseTable>,
}

impl CachedDictionaryEntropy {
    /// Heap bytes the cached dictionary entropy holds: the literals Huffman
    /// table plus any `Custom` LL/ML/OF FSE tables (the `Arc`-boxed `FSETable`
    /// payload and its flat state array). `Default` / `Rle` variants own no heap.
    pub(crate) fn heap_size(&self) -> usize {
        let mut total = self.huff.as_ref().map_or(0, |h| h.heap_size());
        for prev in [&self.ll_previous, &self.ml_previous, &self.of_previous] {
            if let Some(PreviousFseTable::Custom(table)) = prev {
                total += size_of::<FSETable>() + table.heap_size();
            }
        }
        total
    }

    /// Derive the encoder-side entropy tables a dictionary seeds for the first
    /// block of each frame (the upstream zstd `cdict->cBlockState`): the literals
    /// Huffman table plus the literal-length / match-length / offset FSE
    /// "previous" tables. Shared by [`FrameCompressor`] and
    /// [`crate::encoding::StreamingEncoder`] so both seed identically.
    pub(crate) fn from_dictionary(dictionary: &crate::decoding::Dictionary) -> Self {
        Self {
            huff: dictionary.huf.table.to_encoder_table(),
            ll_previous: dictionary
                .fse
                .literal_lengths
                .to_encoder_table()
                .map(|table| PreviousFseTable::Custom(SharedFseTable::new(table))),
            ml_previous: dictionary
                .fse
                .match_lengths
                .to_encoder_table()
                .map(|table| PreviousFseTable::Custom(SharedFseTable::new(table))),
            of_previous: dictionary
                .fse
                .offsets
                .to_encoder_table()
                .map(|table| PreviousFseTable::Custom(SharedFseTable::new(table))),
        }
    }
}

/// Shared owner for a custom "previous" FSE encoder table. `Arc` on
/// atomic-pointer targets, `Rc` otherwise (keeps `no_std` no-atomics
/// builds compiling, single-thread there anyway), mirroring
/// `decoding::dictionary::SharedDictionary`. Cloning the cached
/// dictionary entropy into the per-frame state is then a refcount bump,
/// not a full `FSETable` copy — the upstream zstd references `cdict->cBlockState`
/// instead of rebuilding it per frame.
#[cfg(target_has_atomic = "ptr")]
pub(crate) type SharedFseTable = alloc::sync::Arc<FSETable>;
#[cfg(not(target_has_atomic = "ptr"))]
pub(crate) type SharedFseTable = alloc::rc::Rc<FSETable>;

#[derive(Clone)]
pub(crate) enum PreviousFseTable {
    // Default tables are immutable and already stored alongside the state, so
    // repeating them only needs a lightweight marker instead of cloning FSETable.
    Default,
    // Shared handle: cloning (per-frame dictionary entropy seed) is a refcount
    // bump. The table is only ever read or REPLACED wholesale (a block that
    // builds a new table swaps in a fresh `SharedFseTable`), never mutated in
    // place, so sharing is sound.
    Custom(SharedFseTable),
    Rle(u8),
}

impl PreviousFseTable {
    pub(crate) fn as_table<'a>(&'a self, default: &'a FSETable) -> Option<&'a FSETable> {
        match self {
            Self::Default => Some(default),
            Self::Custom(table) => Some(table),
            Self::Rle(_) => None,
        }
    }
}

pub(crate) struct FseTables {
    /// The three predefined LL/ML/OF tables are functions of
    /// compile-time-constant distributions. The
    /// [`fse_encoder::FseDefaultTable`] type alias resolves to
    /// `&'static FSETable` when a process-wide cache is available
    /// (atomic-pointer targets, or no-atomic targets with the
    /// `critical-section` feature) and to `Box<FSETable>` on the
    /// cache-less no-atomic path (one per-frame allocation, dropped
    /// with the compressor — no `Box::leak`, no unbounded growth).
    /// Both arms `Deref` to `FSETable`, so consumers in
    /// `encoding/blocks/compressed.rs` borrow through `&` uniformly
    /// without seeing the per-target divergence.
    pub(crate) ll_default: crate::fse::fse_encoder::FseDefaultTable,
    pub(crate) ll_previous: Option<PreviousFseTable>,
    pub(crate) ml_default: crate::fse::fse_encoder::FseDefaultTable,
    pub(crate) ml_previous: Option<PreviousFseTable>,
    pub(crate) of_default: crate::fse::fse_encoder::FseDefaultTable,
    pub(crate) of_previous: Option<PreviousFseTable>,
}

impl FseTables {
    pub(crate) fn new() -> Self {
        Self {
            ll_default: default_ll_table(),
            ll_previous: None,
            ml_default: default_ml_table(),
            ml_previous: None,
            of_default: default_of_table(),
            of_previous: None,
        }
    }

    /// Borrow the LL default table as `&FSETable`. Abstracts the cfg
    /// split in [`crate::fse::fse_encoder::FseDefaultTable`] —
    /// `&'static FSETable` (atomic / `critical-section`) auto-derefs
    /// directly; `Box<FSETable>` (cache-less no-atomic) derefs
    /// through `Box`. Both arms yield `&FSETable` uniformly so
    /// downstream consumers can stay cfg-agnostic.
    #[inline]
    #[allow(clippy::borrow_deref_ref)]
    pub(crate) fn ll_default_ref(&self) -> &FSETable {
        &*self.ll_default
    }

    /// Borrow the ML default table as `&FSETable`. See [`Self::ll_default_ref`].
    #[inline]
    #[allow(clippy::borrow_deref_ref)]
    pub(crate) fn ml_default_ref(&self) -> &FSETable {
        &*self.ml_default
    }

    /// Borrow the OF default table as `&FSETable`. See [`Self::ll_default_ref`].
    #[inline]
    #[allow(clippy::borrow_deref_ref)]
    pub(crate) fn of_default_ref(&self) -> &FSETable {
        &*self.of_default
    }
}

const PRESPLIT_BLOCK_MIN: usize = 3500;
const PRESPLIT_THRESHOLD_PENALTY_RATE: u64 = 16;
const PRESPLIT_THRESHOLD_BASE: u64 = PRESPLIT_THRESHOLD_PENALTY_RATE - 2;
const PRESPLIT_THRESHOLD_PENALTY: i32 = 3;
const PRESPLIT_CHUNK_SIZE: usize = 8 << 10;
const PRESPLIT_HASH_LOG_MAX: usize = 10;
const PRESPLIT_HASH_TABLE_SIZE: usize = 1 << PRESPLIT_HASH_LOG_MAX;
const PRESPLIT_KNUTH: u32 = 0x9E37_79B9;
/// Upstream zstd `SEGMENT_SIZE` in `ZSTD_splitBlock_fromBorders` (`zstd_preSplit.c:201`).
/// Two `SEGMENT_SIZE`-byte fingerprints — one from the start, one from the end —
/// drive the cheap border heuristic; a third one from the middle disambiguates
/// where in the block the transition sits.
const PRESPLIT_BORDERS_SEGMENT: usize = 512;

#[derive(Clone)]
struct PreSplitFingerprint {
    events: [u32; PRESPLIT_HASH_TABLE_SIZE],
    nb_events: usize,
}

impl Default for PreSplitFingerprint {
    fn default() -> Self {
        Self {
            events: [0; PRESPLIT_HASH_TABLE_SIZE],
            nb_events: 0,
        }
    }
}

/// Grow `out` ahead of the next block so block emission never lands on an
/// amortized-doubling reallocation mid-frame (whose transient old+new copy
/// spikes peak memory to ~3x the output), sizing the reservation from the
/// compression ratio observed so far instead of the whole-input worst case.
///
/// `blocks_start` is where this frame's blocks begin in `out`, `consumed`
/// the input bytes already emitted as blocks, `remaining` the input
/// bytes still to compress (an estimate is fine: a low one only means one
/// more re-estimate later), and `block_capacity` the active block-size cap
/// (`FrameCompressor::block_capacity`) so a small `targetCBlockSize` does
/// not keep a 128 KiB floor in the buffer or undercount header density.
/// Incompressible input re-estimates to ~the full `compress_bound` after
/// the first block — the old up-front policy's worst case — while
/// compressible input stays at output scale.
fn reserve_for_next_block(
    out: &mut Vec<u8>,
    blocks_start: usize,
    consumed: u64,
    remaining: usize,
    block_capacity: usize,
) {
    // Worst-case single-block output: 3-byte header + raw payload, plus
    // slack for the 4-byte frame checksum trailer and a few extra sub-block
    // headers from the post-split emitters, so neither can reallocate.
    let block_bound = remaining.min(block_capacity) + 3 + 16;
    if out.capacity() - out.len() >= block_bound {
        return;
    }
    let produced = (out.len() - blocks_start) as u64;
    let estimate = if consumed == 0 {
        // No ratio signal yet (capacity exhausted before the first block —
        // only reachable with a caller-shrunk `out`): one block's bound.
        block_bound
    } else {
        // remaining * observed ratio + per-block headers + 1/16 slack so a
        // slightly-worsening tail doesn't force a reallocation per block.
        // u128 keeps the product exact for multi-GiB frames.
        let scaled = ((remaining as u128 * produced as u128) / consumed as u128) as u64;
        let headers = (remaining as u64 / block_capacity.max(1) as u64 + 1) * 3;
        usize::try_from(scaled + scaled / 16 + headers + 64).unwrap_or(usize::MAX)
    };
    // `reserve_exact`: the estimate already carries its own slack, and the
    // whole-buffer doubling policy is exactly what this function exists to
    // avoid. The `produced`-sized floor keeps growth geometric when the
    // ratio estimate lands BELOW one block's bound (highly compressible
    // input): without it every block would trigger a block-sized
    // reallocation — O(blocks) buffer copies — while with it the buffer at
    // least doubles its produced span per reallocation (O(log) copies) and
    // the peak stays at output scale.
    out.reserve_exact(estimate.max(block_bound + produced as usize));
}

fn presplit_hash2(bytes: &[u8], hash_log: usize) -> usize {
    debug_assert!(hash_log >= 8);
    if hash_log == 8 {
        return bytes[0] as usize;
    }
    debug_assert!(hash_log <= PRESPLIT_HASH_LOG_MAX);
    let value = u16::from_le_bytes([bytes[0], bytes[1]]) as u32;
    (value.wrapping_mul(PRESPLIT_KNUTH) >> (32 - hash_log)) as usize
}

fn presplit_record_fingerprint(
    fp: &mut PreSplitFingerprint,
    src: &[u8],
    sampling_rate: usize,
    hash_log: usize,
) {
    fp.events.fill(0);
    fp.nb_events = 0;
    if src.len() < 2 {
        return;
    }
    let limit = src.len() - 1;
    let mut n = 0usize;
    while n < limit {
        fp.events[presplit_hash2(&src[n..], hash_log)] += 1;
        n += sampling_rate;
    }
    // Upstream zstd parity: zstd_preSplit.c records the integer division, not the
    // rounded-up number of sampled events from the loop above.
    fp.nb_events += limit / sampling_rate;
}

/// Single-byte histogram pass — matches upstream zstd `HIST_add` over a small
/// segment with `hashLog == 8` (the `hash2` shortcut at
/// `zstd_preSplit.c:36` returns the raw byte). The byChunks path uses
/// 2-byte hashing for `hashLog >= 9`; this helper exists so the borders
/// heuristic doesn't pay for that wider hash on its 512-byte windows.
fn presplit_record_byte_histogram(fp: &mut PreSplitFingerprint, src: &[u8]) {
    fp.events.fill(0);
    for &b in src {
        fp.events[b as usize] += 1;
    }
    // Upstream zstd `HIST_add` returns the maximum symbol; the caller then sets
    // `nbEvents = SEGMENT_SIZE` explicitly (see `zstd_preSplit.c:213`).
    fp.nb_events = src.len();
}

fn presplit_distance(lhs: &PreSplitFingerprint, rhs: &PreSplitFingerprint, hash_log: usize) -> u64 {
    let slots = 1usize << hash_log;
    let mut distance = 0u64;
    for idx in 0..slots {
        let left = lhs.events[idx] as i128 * rhs.nb_events as i128;
        let right = rhs.events[idx] as i128 * lhs.nb_events as i128;
        // Plain `+`: events/nb_events are per-block sample counts (<= block
        // size), so each |left-right| <= (2^17)^2 and the sum over <= 2^hash_log
        // slots stays far under u64::MAX — no overflow.
        distance += left.abs_diff(right) as u64;
    }
    distance
}

fn presplit_fingerprints_differ(
    reference: &PreSplitFingerprint,
    new_fp: &PreSplitFingerprint,
    penalty: i32,
    hash_log: usize,
) -> bool {
    debug_assert!(reference.nb_events > 0);
    debug_assert!(new_fp.nb_events > 0);
    let p50 = reference.nb_events as u64 * new_fp.nb_events as u64;
    let deviation = presplit_distance(reference, new_fp, hash_log);
    // Plain `*`: p50 <= (block-sample-count)^2 and the (base+penalty) factor is
    // a small constant, so the product stays well under u64::MAX.
    let threshold =
        p50 * (PRESPLIT_THRESHOLD_BASE + penalty as u64) / PRESPLIT_THRESHOLD_PENALTY_RATE;
    deviation >= threshold
}

fn presplit_merge_events(acc: &mut PreSplitFingerprint, new_fp: &PreSplitFingerprint) {
    // Plain `+`: `acc` accumulates only the chunks of a single block (caller
    // loops within one block, <= MAX_BLOCK_SIZE), so the merged sample counts
    // stay far under u32 / usize bounds — no overflow.
    for idx in 0..PRESPLIT_HASH_TABLE_SIZE {
        acc.events[idx] += new_fp.events[idx];
    }
    acc.nb_events += new_fp.nb_events;
}

fn split_block_by_chunks(block: &[u8], level: usize) -> usize {
    debug_assert_eq!(block.len(), MAX_BLOCK_SIZE as usize);
    debug_assert!((1..=4).contains(&level));
    let (sampling_rate, hash_log) = match level - 1 {
        0 => (43, 8),
        1 => (11, 9),
        2 => (5, 10),
        _ => (1, 10),
    };

    let mut past = PreSplitFingerprint::default();
    let mut new_events = PreSplitFingerprint::default();
    let mut penalty = PRESPLIT_THRESHOLD_PENALTY;
    presplit_record_fingerprint(
        &mut past,
        &block[..PRESPLIT_CHUNK_SIZE],
        sampling_rate,
        hash_log,
    );
    let mut pos = PRESPLIT_CHUNK_SIZE;
    while pos <= block.len() - PRESPLIT_CHUNK_SIZE {
        presplit_record_fingerprint(
            &mut new_events,
            &block[pos..pos + PRESPLIT_CHUNK_SIZE],
            sampling_rate,
            hash_log,
        );
        if presplit_fingerprints_differ(&past, &new_events, penalty, hash_log) {
            return pos;
        }
        presplit_merge_events(&mut past, &new_events);
        if penalty > 0 {
            penalty -= 1;
        }
        pos += PRESPLIT_CHUNK_SIZE;
    }
    block.len()
}

/// Upstream zstd port of `ZSTD_splitBlock_fromBorders` (`zstd_preSplit.c:198`).
/// Records two 512-byte byte-histograms — one from each end of a 128 KB
/// block — and a third from the middle as a tie-breaker; returns either
/// a quantised split point (32 KB / 64 KB / 96 KB) or the full block
/// size when the two ends look indistinguishable. Cheaper than the
/// chunk-based path because it touches at most 1.5 KB of input
/// regardless of block size.
fn split_block_from_borders(block: &[u8]) -> usize {
    debug_assert_eq!(block.len(), MAX_BLOCK_SIZE as usize);
    let block_size = block.len();
    let mut past = PreSplitFingerprint::default();
    let mut new_fp = PreSplitFingerprint::default();
    presplit_record_byte_histogram(&mut past, &block[..PRESPLIT_BORDERS_SEGMENT]);
    presplit_record_byte_histogram(&mut new_fp, &block[block_size - PRESPLIT_BORDERS_SEGMENT..]);
    // Upstream zstd uses `penalty = 0, hash_log = 8` — i.e. raw byte histogram
    // distance with no threshold padding (`zstd_preSplit.c:214`).
    if !presplit_fingerprints_differ(&past, &new_fp, 0, 8) {
        return block_size;
    }

    let mut middle = PreSplitFingerprint::default();
    let mid_start = block_size / 2 - PRESPLIT_BORDERS_SEGMENT / 2;
    presplit_record_byte_histogram(
        &mut middle,
        &block[mid_start..mid_start + PRESPLIT_BORDERS_SEGMENT],
    );

    let dist_from_begin = presplit_distance(&past, &middle, 8);
    let dist_from_end = presplit_distance(&new_fp, &middle, 8);
    // Upstream zstd `SEGMENT_SIZE * SEGMENT_SIZE / 3` (`zstd_preSplit.c:221`):
    // if the middle is roughly equidistant from both ends, the change
    // sits near the centre — split at the midpoint.
    let min_distance = (PRESPLIT_BORDERS_SEGMENT as u64) * (PRESPLIT_BORDERS_SEGMENT as u64) / 3;
    if dist_from_begin.abs_diff(dist_from_end) < min_distance {
        return 64 * 1024;
    }
    // Larger `dist_from_begin` (i.e. `middle` farther from the head
    // fingerprint, equivalently closer to the tail) means the new
    // statistics already dominate the centre — the transition
    // happened EARLY → emit a small 32 KB head and let the 96 KB
    // tail absorb the rest. Inverse case: `dist_from_end` larger
    // (middle still resembles the head) means the transition is
    // LATE → emit a 96 KB head so the trailing 32 KB carries the
    // new statistics alone.
    if dist_from_begin > dist_from_end {
        32 * 1024
    } else {
        96 * 1024
    }
}

/// Pull a pre-split window into cache with one bandwidth-bound sequential
/// pass before the strided fingerprint histogram + match scan read it.
///
/// The borrowed (no-copy) over-window path matches in place on the caller's
/// input, so the pre-split fingerprint is the FIRST touch of that 128 KiB
/// region — a cache-cold read. `presplit_record_fingerprint` reads it with a
/// `sampling_rate` stride and interleaved random writes into the 1 KiB events
/// table, a latency-bound pattern that pays full DRAM miss latency per line
/// (measured ~3x the cost of an ERMS streaming read of the same bytes). The
/// owned path never hits this because its history-mirror copy already warmed
/// the bytes; this restores that warmth without the copy's write half. One
/// dependent load per 64-byte line (the i9 line size) streams under the
/// hardware prefetcher, so the cold read is paid once at memory bandwidth and
/// every subsequent strided sample lands in L1/L2. `black_box` keeps the loop
/// from being optimized away as a dead read.
#[inline]
fn warm_presplit_window(window: &[u8]) {
    let mut acc = 0u8;
    let mut i = 0usize;
    while i < window.len() {
        acc ^= window[i];
        i += 64;
    }
    core::hint::black_box(acc);
}

pub(crate) fn optimal_block_size(
    level: CompressionLevel,
    block: &[u8],
    remaining_src_size: usize,
    block_size_max: usize,
    savings: i64,
) -> usize {
    let Some(split_level) = config::level_pre_split(level) else {
        return remaining_src_size.min(block_size_max);
    };
    if remaining_src_size < MAX_BLOCK_SIZE as usize || block_size_max < MAX_BLOCK_SIZE as usize {
        return remaining_src_size.min(block_size_max);
    }
    if savings < 3 {
        return MAX_BLOCK_SIZE as usize;
    }
    if block.len() < MAX_BLOCK_SIZE as usize {
        return remaining_src_size.min(block_size_max);
    }
    // Upstream zstd `ZSTD_splitBlock` dispatch (`zstd_preSplit.c:234`):
    // `split_level == 0` → cheap borders heuristic;
    // `split_level == 1..=4` → byChunks with internal sampling level
    // `split_level - 1`.
    let raw_split = if split_level == 0 {
        split_block_from_borders(&block[..MAX_BLOCK_SIZE as usize])
    } else {
        split_block_by_chunks(&block[..MAX_BLOCK_SIZE as usize], split_level)
    };
    raw_split
        .max(PRESPLIT_BLOCK_MIN)
        .min(MAX_BLOCK_SIZE as usize)
}

pub(crate) struct CompressState<M: Matcher> {
    pub(crate) matcher: M,
    pub(crate) last_huff_table: Option<crate::huff0::huff0_encoder::HuffmanTable>,
    /// Recycled `HuffmanTable` buffers: when a block clears or replaces
    /// `last_huff_table`, the old table parks here instead of dropping, so
    /// the next frame's dictionary entropy seed `clone_from`s into existing
    /// allocations. Without this, every dict-seeded frame whose last block
    /// ended raw/RLE paid a fresh two-Vec table clone per frame.
    pub(crate) huff_table_spare: Option<crate::huff0::huff0_encoder::HuffmanTable>,
    pub(crate) fse_tables: FseTables,
    pub(crate) block_scratch: crate::encoding::blocks::CompressedBlockScratch,
    /// Offset history for repeat offset encoding: [rep0, rep1, rep2].
    /// Initialized to [1, 4, 8] per RFC 8878 §3.1.2.5.
    pub(crate) offset_hist: [u32; 3],
    /// Strategy tag resolved from the current `CompressionLevel` at every
    /// `matcher.reset()` call. Used by the literal-compression gates
    /// (`min_literals_to_compress`, `min_gain`) in
    /// `encoding::blocks::compressed` to mirror upstream zstd's strategy-aware
    /// thresholds (`zstd_compress_literals.c:114-127, 187-188`).
    ///
    /// **Invariant (required of every construction site):** must be
    /// initialized from the active `CompressionLevel` via
    /// `StrategyTag::for_compression_level`, and re-synced from the
    /// active level alongside every `matcher.reset()` call so the
    /// level-aware gates stay correct after a level change. The two
    /// reset sites that own this sync are `FrameCompressor::compress`
    /// and `StreamingEncoder::ensure_frame_started`. There is no
    /// `Default` impl — production constructors
    /// (`FrameCompressor::new`, `new_with_matcher`, the streaming
    /// encoder constructor) plumb this explicitly. Tests that build
    /// `CompressState` by hand must also supply a value.
    pub(crate) strategy_tag: crate::encoding::strategy::StrategyTag,
    /// Whether the HUF literal table build runs the table-log search
    /// (`true`) or the cheap single-build (`false`). The search is a clean
    /// ratio win over upstream zstd but costs ~1.5 us per literal section —
    /// negligible on large inputs, ~20% on small ones. The Fast and DoubleFast
    /// matchers are byte-faithful to upstream zstd, so the cheap path ties them;
    /// the search is therefore gated ON only for large (> 128 KiB) Fast and
    /// DoubleFast frames. Higher strategies always keep it (their matchers
    /// diverge, making the search load-bearing for ratio). Set per frame
    /// alongside `strategy_tag` via [`huf_search_enabled`].
    pub(crate) huf_optimal_search: bool,
    /// Mirror of upstream zstd's `ZSTD_literalsCompressionIsDisabled`
    /// (zstd_compress_internal.h): in the default (`auto`) literal-compression
    /// mode the literals section is emitted RAW (no Huffman) when
    /// `strategy == ZSTD_fast && targetLength > 0`. For the levels we resolve,
    /// that is exactly the negative levels (Fast strategy with `targetLength =
    /// -level > 0`; L1/L2 are Fast with `targetLength == 0`). C trades the
    /// literal-Huffman pass for speed there, so matching it keeps both the frame
    /// size and the encode cost in parity on the negative band. Set per frame
    /// alongside `strategy_tag`.
    pub(crate) literal_compression_disabled: bool,
}

/// Whether the HUF literal build should run the table-log search for a
/// frame of `source_size` bytes (see [`CompressState::huf_optimal_search`]).
/// Upstream gates the optimal-depth tableLog probe to
/// `HUF_OPTIMAL_DEPTH_THRESHOLD = ZSTD_btultra` (huf.h:117): only btultra /
/// btultra2 search the tableLog, every lower strategy (fast .. btopt) takes the
/// single-shot fast path (`HUF_optimalTableLog`, huf_compress.c:1284-1287).
/// Mirror that so our literal tableLog choice tracks upstream's instead of
/// spending the search to beat it on ratio at a speed cost.
pub(crate) fn huf_search_enabled(
    strategy: crate::encoding::strategy::StrategyTag,
    _source_size: Option<u64>,
) -> bool {
    use crate::encoding::strategy::StrategyTag;
    matches!(strategy, StrategyTag::BtUltra | StrategyTag::BtUltra2)
}

impl<M: Matcher> CompressState<M> {
    /// Clears `last_huff_table`, parking the table's buffers in
    /// `huff_table_spare` for reuse instead of dropping them.
    #[inline]
    pub(crate) fn clear_huff_table(&mut self) {
        if let Some(table) = self.last_huff_table.take() {
            self.huff_table_spare = Some(table);
        }
    }

    /// Replaces `last_huff_table` with `table`, parking any displaced table
    /// in `huff_table_spare` for reuse.
    #[inline]
    pub(crate) fn replace_huff_table(&mut self, table: crate::huff0::huff0_encoder::HuffmanTable) {
        if let Some(old) = self.last_huff_table.replace(table) {
            self.huff_table_spare = Some(old);
        }
    }
}

/// Per-frame setup resolved once by [`FrameCompressor::prepare_frame`] and
/// consumed by the block loop + [`FrameCompressor::finish_frame`]. Lets the
/// owned `compress()` and the borrowed one-shot path share identical
/// reset / dict-prime / entropy-seed setup and frame-tail emission.
struct FramePrep {
    window_size: u64,
    use_dictionary_state: bool,
    source_size_hint_known: bool,
    initial_size_hint: Option<u64>,
}

/// Initial capacity for the `all_blocks` accumulator, by source-size hint.
/// The frame header is written only after all input is read (so
/// Frame_Content_Size is known), so compressed blocks accumulate in memory
/// first. Seed-size tiers (mirrors upstream zstd `ZSTD_CStreamOutSize` naming):
/// - tiny (`<= 4 KiB` hint): payload-bound seed, `>=` anything a tiny input's
///   compressed output could need.
/// - small (`<= 64 KiB` hint): absorbs one or two `Vec::extend` doublings
///   without over-allocating.
/// - default (one upstream zstd block, `130 KiB`): the value the rest of the encoder
///   is sized around; larger inputs amortise the first doublings cheaply and
///   the residue is dominated by internal `compress_block_encoded` buffers.
///
/// Shared by the owned (`run_owned_block_loop`) and borrowed
/// (`run_borrowed_block_loop`) paths so the tier table can't drift between them.
///
/// `block_capacity` (the active `targetCBlockSize` cap, or the 128 KiB
/// format ceiling) bounds every tier: with a small target the first
/// allocation tracks one capped block + header/checksum slack instead of
/// keeping the upstream zstd-sized floor that only later growth respects.
fn initial_all_blocks_cap(initial_size_hint: Option<u64>, block_capacity: usize) -> usize {
    const TINY_THRESHOLD: u64 = 4 * 1024;
    const SMALL_THRESHOLD: u64 = 64 * 1024;
    const TINY_CAP: usize = 4 * 1024;
    const SMALL_CAP: usize = 16 * 1024;
    const DEFAULT_CAP: usize = 130 * 1024;
    let first_block_cap = block_capacity + 3 + 16;
    match initial_size_hint {
        Some(h) if h <= TINY_THRESHOLD => TINY_CAP.min(first_block_cap),
        Some(h) if h <= SMALL_THRESHOLD => SMALL_CAP.min(first_block_cap),
        _ => DEFAULT_CAP.min(first_block_cap),
    }
}

/// Per-block feeder for `run_owned_block_loop`.
///
/// `fill_block` appends source bytes to `buf` (which already holds any
/// carried pre-split suffix) until `buf.len() == block_capacity` or the
/// source is exhausted, returning `(bytes_appended, reached_eof)`.
/// `reached_eof` is true when no more input follows this block: either the
/// block could not be filled to `block_capacity`, or it filled exactly and the
/// source is confirmed exhausted (the slice knows its length; the reader probes
/// one byte ahead). An input that is an exact multiple of the block size
/// therefore marks its final full block `last_block` rather than emitting a
/// spurious trailing empty block.
///
/// The slice impl exists so the slice entry points
/// (`compress_independent_frame_into`, `compress_oneshot_*` fallbacks)
/// append with one `extend_from_slice` — the generic reader impl must
/// `resize` an initialized target region before `Read::read` can fill it,
/// which costs a zero-fill memset of the whole block on every frame.
pub(crate) trait OwnedBlockSource {
    fn fill_block(
        &mut self,
        buf: &mut Vec<u8>,
        block_capacity: usize,
        size_hint_remaining: Option<u64>,
    ) -> (usize, bool);
}

impl OwnedBlockSource for &[u8] {
    fn fill_block(
        &mut self,
        buf: &mut Vec<u8>,
        block_capacity: usize,
        _size_hint_remaining: Option<u64>,
    ) -> (usize, bool) {
        let want = block_capacity - buf.len();
        let take = want.min(self.len());
        buf.extend_from_slice(&self[..take]);
        *self = &self[take..];
        // EOF when this fill could not top the block to `block_capacity`
        // (`take < want`) OR it exactly consumed the last input bytes
        // (`self` now empty). The slice knows its own length, so a block that
        // exactly fills capacity at end-of-input is reported as the final
        // block here — the loop marks it `last_block` instead of emitting a
        // spurious trailing empty block on the next iteration. Mirrors the C
        // encoder, which marks the last real block last on `ZSTD_e_end`.
        (take, take < want || self.is_empty())
    }
}

/// Adapter routing a generic [`Read`] source through [`OwnedBlockSource`]:
/// preserves the historical sizing behaviour — an initialized target region
/// bounded by the source-size hint, grown (doubling, capped) only when the
/// hint under-counted.
/// `peeked` holds a single look-ahead byte: when a block fills exactly to
/// `block_capacity`, `fill_block` reads one more byte to learn whether the
/// stream ended on that boundary. A `None` from that probe sets EOF (so the
/// just-filled block is marked last, mirroring the C encoder on `ZSTD_e_end`);
/// a byte is stashed here and prepended to the next block instead of leaking a
/// spurious trailing empty block when the input is an exact multiple of the
/// block size.
pub(crate) struct ReaderBlockSource<Rd> {
    pub(crate) reader: Rd,
    peeked: Option<u8>,
}

impl<Rd> ReaderBlockSource<Rd> {
    pub(crate) fn new(reader: Rd) -> Self {
        Self {
            reader,
            peeked: None,
        }
    }
}

impl<Rd: Read> OwnedBlockSource for ReaderBlockSource<Rd> {
    fn fill_block(
        &mut self,
        buf: &mut Vec<u8>,
        block_capacity: usize,
        size_hint_remaining: Option<u64>,
    ) -> (usize, bool) {
        let start = buf.len();
        let mut filled = start;
        let mut reached_eof = false;
        // Prepend the look-ahead byte read past the previous full block. In
        // stream order it follows any carried pre-split suffix already in
        // `buf`, so it is appended after that suffix and counted as part of
        // this block's appended bytes.
        if let Some(b) = self.peeked.take() {
            buf.push(b);
            filled += 1;
        }
        // Size the read buffer to the bytes this block actually expects
        // rather than always zero-filling a full MAX_BLOCK_SIZE: a small
        // frame otherwise pays a 128 KiB `resize(_, 0)` memset per block
        // just to read a few KiB (the zero-fill past `filled` is then
        // truncated away).
        //
        // Overflow-free by construction (no `saturating_*` masking):
        // `filled <= block_capacity` always (the read only ever targets
        // `[filled..len]` with `len <= block_capacity`, and a carried-over
        // pre-split suffix is a `split_off` below `block_capacity`), so
        // `block_capacity - filled` never underflows; pinning `remaining`
        // to `block_capacity` before the `usize` cast keeps the cast and
        // the final add within `usize` on every target.
        let initial_target = match size_hint_remaining {
            Some(remaining) => {
                let remaining = remaining.min(block_capacity as u64) as usize;
                filled + remaining.min(block_capacity - filled)
            }
            // Unknown hint, or an inexact hint already met by prior blocks:
            // read against the full block window.
            None => block_capacity,
        };
        if buf.len() < initial_target {
            buf.resize(initial_target, 0);
        }
        loop {
            if reached_eof || filled == block_capacity {
                break;
            }
            if filled == buf.len() {
                // Hint under-counted the block; grow toward block_capacity
                // (doubling, capped) so reading continues without paying a
                // full-buffer zero up front. `len <= block_capacity` so the
                // double stays well within `usize`; `filled < block_capacity`
                // here (the `== block_capacity` break fired otherwise), so
                // `filled + 1 <= block_capacity`.
                let grow_to = (buf.len() * 2).clamp(filled + 1, block_capacity);
                buf.resize(grow_to, 0);
            }
            let read_end = buf.len();
            let new_bytes = self.reader.read(&mut buf[filled..read_end]).unwrap();
            if new_bytes == 0 {
                reached_eof = true;
                break;
            }
            filled += new_bytes;
        }
        // Look ahead one byte when the block filled exactly to capacity: a
        // 0-byte read means the stream ended on the block boundary, so this
        // block is the last one (the loop marks it `last_block`); otherwise
        // stash the byte for the next block. Without this, an input that is an
        // exact multiple of the block size would emit a spurious trailing
        // empty block (the next iteration reads 0 and serializes an empty
        // last Raw block). A blocking reader's probe read is consistent with
        // the existing pull model — the next `fill_block` would block on the
        // same byte anyway.
        if !reached_eof && filled == block_capacity {
            let mut probe = [0u8; 1];
            if self.reader.read(&mut probe).unwrap() == 0 {
                reached_eof = true;
            } else {
                self.peeked = Some(probe[0]);
            }
        }
        buf.truncate(filled);
        (filled - start, reached_eof)
    }
}

impl<R: Read, W: Write> FrameCompressor<R, W, MatchGeneratorDriver> {
    /// Create a new `FrameCompressor`
    pub fn new(compression_level: CompressionLevel) -> Self {
        Self {
            uncompressed_data: None,
            compressed_data: None,
            compression_level,
            dictionary: None,
            dictionary_entropy_cache: None,
            source_size_hint: None,
            state: CompressState {
                matcher: MatchGeneratorDriver::new(1024 * 128, 1),
                last_huff_table: None,
                huff_table_spare: None,
                fse_tables: FseTables::new(),
                block_scratch: crate::encoding::blocks::CompressedBlockScratch::new(),
                offset_hist: [1, 4, 8],
                strategy_tag: crate::encoding::strategy::StrategyTag::for_compression_level(
                    compression_level,
                ),
                huf_optimal_search: true,
                literal_compression_disabled: matches!(
                    compression_level,
                    CompressionLevel::Level(n) if n < 0
                ),
            },
            magicless: false,
            content_checksum: false,
            content_size_flag: true,
            dict_id_flag: true,
            target_block_size: None,

            hasher: XxHash64::with_seed(0),

            strategy_override: None,
        }
    }

    /// Configure fine-grained compression parameters.
    ///
    /// Resets the base [`CompressionLevel`](crate::encoding::CompressionLevel)
    /// to the parameters' level and installs the per-knob overrides
    /// (window/hash/chain/search logs, strategy, LDM) applied at the next
    /// frame. Pass `None`-equivalent (a builder that overrides nothing)
    /// to fall back to plain level-based compression.
    ///
    /// ```rust
    /// use purrdf_zstd::encoding::{
    ///     CompressionLevel, CompressionParameters, FrameCompressor, Strategy,
    /// };
    /// let params = CompressionParameters::builder(CompressionLevel::Level(19))
    ///     .strategy(Strategy::Btultra2)
    ///     .enable_long_distance_matching(true)
    ///     .build()
    ///     .unwrap();
    /// let mut compressor: FrameCompressor = FrameCompressor::new(CompressionLevel::Default);
    /// compressor.set_parameters(&params);
    /// let compressed = compressor.compress_independent_frame(b"some data to compress");
    /// assert!(!compressed.is_empty());
    /// ```
    pub fn set_parameters(&mut self, params: &crate::encoding::CompressionParameters) {
        self.compression_level = params.level();
        let overrides = params.overrides();
        self.strategy_override = overrides.strategy.map(|s| s.tag());
        // Keep `state.strategy_tag` consistent immediately so the borrowed
        // one-shot eligibility gate (`borrowed_eligible`) and literal gates
        // are correct even before the next `compress()` re-sync. Resolve it
        // size-adaptively (same `resolve_level_params` path `prepare_frame`
        // uses) so a hint already set here yields the same strategy the matcher
        // will run, not the bare level-only mapping.
        self.state.strategy_tag = self.strategy_override.unwrap_or_else(|| {
            config::resolve_level_params(self.compression_level, self.source_size_hint).strategy_tag
        });
        self.state.huf_optimal_search =
            huf_search_enabled(self.state.strategy_tag, self.source_size_hint);
        // C `ZSTD_literalsCompressionIsDisabled` (ps_auto): raw literals iff the
        // EFFECTIVE cParams are the fast strategy with targetLength > 0. A
        // strategy override (e.g. negative level + BtUltra2) flips `strategy_tag`
        // away from fast, so the resolved tag — not the signed level — gates
        // this. For the fast strategy the level table sets targetLength > 0
        // exactly on the negative (acceleration) rows, so absent an explicit
        // targetLength override `level < 0` is that test; an override wins.
        self.state.literal_compression_disabled = self.state.strategy_tag
            == crate::encoding::strategy::StrategyTag::Fast
            && overrides.target_length.map_or_else(
                || {
                    matches!(
                        self.compression_level,
                        CompressionLevel::Level(n) if n < 0
                    )
                },
                |tl| tl > 0,
            );
        self.state.matcher.set_param_overrides(Some(overrides));
    }

    /// Whether the borrowed (no per-block history copy) one-shot loop is
    /// valid for an `input_len`-byte slice under the resolved `prep`.
    ///
    /// `Uncompressed` resolves to `StrategyTag::Fast` but must emit stored
    /// Raw blocks, which the borrowed loop's
    /// `compress_block_encoded_borrowed` (RLE/raw-fast/compressed) does NOT
    /// do, so exclude it; it then takes the owned path's dedicated
    /// Uncompressed arm.
    ///
    /// No window-size gate: over-window inputs are handled too. The owned
    /// path bounds matches to the last `advertised_window` bytes via
    /// `window_low` and evicts/rehashes its history; the borrowed path
    /// computes the identical `window_low = block_end - advertised_window`
    /// and the kernel rejects any hash candidate below it, while the
    /// per-position `put` during the scan keeps in-window slots current,
    /// so it produces byte-identical output to the owned (evicting) path
    /// without ever copying the input into `history`, even when the input
    /// far exceeds the window.
    ///
    /// BUT gate on `input_len <= u32::MAX`: the Fast kernel stores ABSOLUTE
    /// positions in a `u32` hash table, and the borrowed scan walks
    /// absolute input offsets up to `block_end == input.len()`. Past 4 GiB
    /// those offsets truncate / overflow the `u32` position math
    /// (`base_off + ip0 as u32`, `window_low`), panicking or corrupting.
    /// The owned/evicting path keeps the scanned window bounded (positions
    /// stay small), so >4 GiB inputs fall back to it.
    fn borrowed_eligible(&self, input_len: usize, prep: &FramePrep) -> bool {
        if matches!(self.compression_level, CompressionLevel::Uncompressed)
            || input_len > u32::MAX as usize
        {
            return false;
        }
        if prep.use_dictionary_state {
            // The borrowed dict scan runs in VIRTUAL `[dict][input]` coordinates,
            // so the position space is `dict_content.len() + input_len`, not just
            // `input_len`. A large attached dictionary plus an otherwise-allowed
            // input can exceed the `u32` floor the kernel asserts — fall back to
            // the owned (copy) path in that case.
            let fits_u32 = self
                .dictionary
                .as_ref()
                .and_then(|dict| dict.inner.dict_content.len().checked_add(input_len))
                .is_some_and(|virtual_len| virtual_len <= u32::MAX as usize);
            if !fits_u32 {
                return false;
            }
            // Dictionary frames: only the Simple (Fast) backend in attach mode
            // has a borrowed (no input copy) dict scan. Copy-mode dict frames
            // and the other backends still take the owned path.
            return self.state.matcher.borrowed_dict_supported();
        }
        // The borrowed (no-copy, in-place over-window) scan exists for the
        // Simple (Fast), Dfast, and Row backends, and for the HashChain
        // backend's lazy CHAIN parser; BT/optimal (BinaryTree search) stay on
        // the owned path. Every borrowed scan applies the per-position
        // `window_low = abs_ip - advertised_window` offset cap so over-window
        // inputs are matched in place (no input->history copy), matching C's
        // continuous-index + windowLow one-shot behaviour.
        self.state.matcher.borrowed_supported()
    }

    /// Compress `input` as one frame's worth of blocks into `out` (appended
    /// from its current end): the borrowed in-place loop when
    /// [`Self::borrowed_eligible`], else the owned (history-copying) loop fed
    /// an in-place `&[u8]` cursor. Returns `total_uncompressed`; the caller
    /// emits the frame header (before this call, when the content size is
    /// known) or the drain tail.
    fn run_one_frame(&mut self, input: &[u8], prep: &FramePrep, out: &mut Vec<u8>) -> u64 {
        if self.borrowed_eligible(input.len(), prep) {
            self.run_borrowed_block_loop(input, out)
        } else {
            let mut cursor: &[u8] = input;
            self.run_owned_block_loop(&mut cursor, prep.initial_size_hint, true, out)
        }
    }

    /// Compress one contiguous `&[u8]` as a single independent Zstd frame,
    /// writing the frame bytes into `out` (its previous contents are
    /// replaced and its allocation reused), reusing this compressor's heavy
    /// state across calls.
    ///
    /// This is the reusable-compression-context (CCtx-equivalent) entry
    /// point, mirroring C `ZSTD_compress2` over a reused `ZSTD_CCtx`:
    /// construct ONE `FrameCompressor` and call this in a loop to emit N
    /// independent, self-describing frames (each carrying its own header,
    /// blocks, and checksum, decodable in isolation, with no cross-frame
    /// match history). Every call resets the per-frame state via
    /// `prepare_frame`: only the allocations are kept, so the
    /// dominant per-frame setup cost (table allocation + dictionary prime)
    /// is paid once instead of N times. Passing the same `out` buffer each
    /// call additionally reuses the output allocation, matching C's
    /// caller-owned `dst` buffer (no per-frame output allocation).
    ///
    /// Reusing the context + `out` across many small frames (the typical
    /// per-block-frame workload) is far cheaper than a fresh
    /// [`compress_slice_to_vec`](crate::encoding::compress_slice_to_vec)
    /// per block, which allocates and primes from scratch each time.
    ///
    /// The input is read in place: no [`Self::set_source`] /
    /// [`Self::set_drain`] setup is required, and the input lifetime is not
    /// baked into the compressor type, so successive calls may pass slices
    /// with unrelated lifetimes. When the Fast (Simple) backend is active
    /// and no dictionary is set, the matcher references the input directly
    /// (no per-block history copy); other backends / dictionary use copy
    /// each block into history exactly as the streaming
    /// [`compress`](Self::compress) path does. The source-size hint is
    /// derived from the input length on every call, so per-frame table
    /// sizing tracks each frame's actual size regardless of any earlier
    /// hint.
    ///
    /// A sticky dictionary set via
    /// [`set_dictionary`](Self::set_dictionary) (or its variants) is primed
    /// into every frame, mirroring `ZSTD_CCtx_loadDictionary` /
    /// `ZSTD_CCtx_refCDict`.
    ///
    /// # Panics
    ///
    /// Panics on encoder error, matching [`Self::compress`] and
    /// [`compress_slice_to_vec`](crate::encoding::compress_slice_to_vec).
    pub fn compress_independent_frame_into(&mut self, input: &[u8], out: &mut Vec<u8>) {
        // Size the next frame from the actual payload, not a stale hint a
        // previous call may have left behind (a wrong hint would change the
        // resolved window/header and could flip borrowed eligibility).
        self.source_size_hint = Some(input.len() as u64);
        let prep = self.prepare_frame();
        // Content size is known up front (one-shot), so write the frame
        // header FIRST and emit blocks STRAIGHT into `out` — no separate
        // `all_blocks` accumulator and no header+blocks copy (which was the
        // dominant per-frame memmove + the only un-amortized per-frame alloc
        // even when the compressor is reused).
        let total_uncompressed = input.len() as u64;
        let emit_checksum = self.content_checksum;
        let checksum_len = if emit_checksum { 4 } else { 0 };
        out.clear();
        // Reserve the header plus ONE block's worst case up front; the block
        // loops then grow `out` from the compression ratio observed so far
        // (`reserve_for_next_block`). Reserving `compress_bound(input_len)`
        // here held a whole-input-sized allocation for the entire frame —
        // ~100 MiB peak on a 100 MiB stream whose compressed output is a few
        // MiB, where the reference implementation's context peaks at
        // window-sized state. Small frames (<= one block) still get their
        // full bound in one shot, so the reused-`out` steady state is
        // unchanged. 18 = max frame header (magic 4 + descriptor 1 + window
        // 1 + dict id 4 + FCS 8).
        let first_block_bound = input.len().min(self.block_capacity()) + 3;
        out.reserve(18 + first_block_bound + checksum_len);
        self.append_frame_header(total_uncompressed, &prep, out);
        let header_len = out.len();
        let _ = self.run_one_frame(input, &prep, out);

        if self.content_checksum {
            out.extend_from_slice(&(self.hasher.finish() as u32).to_le_bytes());
        }

        let _ = header_len;
    }

    /// Convenience wrapper over [`Self::compress_independent_frame_into`]
    /// that allocates and returns a fresh `Vec` per call. Prefer the
    /// `_into` form in tight per-block-frame loops to reuse one output
    /// buffer across frames (the CCtx-equivalent zero-per-call-alloc
    /// output, matching C's caller-owned `dst`).
    ///
    /// ```rust
    /// use purrdf_zstd::encoding::{FrameCompressor, CompressionLevel};
    /// let mut cctx: FrameCompressor = FrameCompressor::new(CompressionLevel::Default);
    /// let frame_a = cctx.compress_independent_frame(b"first block payload");
    /// let frame_b = cctx.compress_independent_frame(b"second block payload");
    /// assert!(!frame_a.is_empty() && !frame_b.is_empty());
    /// ```
    pub fn compress_independent_frame(&mut self, input: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        self.compress_independent_frame_into(input, &mut out);
        out
    }

    /// Borrowed one-shot block loop: walks `input` in `MAX_BLOCK_SIZE`
    /// strides (the Fast backend never pre-splits, so boundaries match the
    /// owned loop), scanning each block range in place against the
    /// borrowed window via `compress_block_encoded_borrowed` — no
    /// per-block `commit_space` copy. Returns `(all_blocks,
    /// total_uncompressed)`. Caller guarantees Fast backend + no
    /// dictionary; over-window inputs are fine (matches are bounded by
    /// `window_low` exactly as the owned evicting path).
    fn run_borrowed_block_loop(&mut self, input: &[u8], out: &mut Vec<u8>) -> u64 {
        // Blocks are appended to `out` starting here. `out` may already hold
        // the frame header (the one-shot compress-into-Vec path writes it
        // first, since the content size is known up front, and the loop
        // emits blocks straight after it — no separate `all_blocks` Vec and
        // no header+blocks copy). Output-size reads below are taken RELATIVE
        // to `blocks_start` so a header prefix never skews the upstream zstd split
        // `savings` gate (which would change block boundaries / wire output).
        let blocks_start = out.len();
        let total_uncompressed = input.len() as u64;
        // Empty input: emit a single empty last Raw block (mirrors the
        // owned loop's empty-file special case).
        if input.is_empty() {
            let header = BlockHeader {
                last_block: true,
                block_type: crate::blocks::block::BlockType::Raw,
                block_size: 0,
            };
            header.serialize(out);

            return total_uncompressed;
        }
        // SAFETY: `input` outlives this call (held by the caller across
        // the call) and is not mutated. Only the Simple backend is active
        // (gated by `compress_oneshot_borrowed`).
        unsafe {
            self.state.matcher.set_borrowed_window(input);
        }
        // Panic-safety: clear the borrowed `(ptr, len)` on EVERY exit,
        // including an unwind from an `assert!` inside the block loop, so
        // a caught-and-reused compressor never retains a dangling window.
        // (The next frame's `reset()` also clears it before any read, but
        // this guard makes the invariant local and unwind-proof.)
        struct ClearBorrowedOnDrop(*mut MatchGeneratorDriver);
        impl Drop for ClearBorrowedOnDrop {
            fn drop(&mut self) {
                // SAFETY: at drop (normal return or unwind) the loop's
                // borrows of the matcher have ended, so this is the only
                // access. `addr_of_mut!` produced this pointer without an
                // intermediate `&mut`, so the interleaved `&mut` uses in
                // the loop did not invalidate it.
                unsafe { (*self.0).clear_borrowed_window() };
            }
        }
        let _clear_guard = ClearBorrowedOnDrop(core::ptr::addr_of_mut!(self.state.matcher));
        let block_capacity = self.block_capacity();
        let mut start = 0usize;
        while start < input.len() {
            reserve_for_next_block(
                out,
                blocks_start,
                start as u64,
                input.len() - start,
                block_capacity,
            );
            // Upstream zstd `ZSTD_compress_frameChunk`: size each block via the cheap
            // fingerprint pre-splitter so a full 128 KiB block is cut at a
            // statistical boundary when it pays. `savings = consumed -
            // produced` mirrors the upstream zstd gate (the first block and
            // incompressible input keep the full 128 KiB). The borrowed window
            // already spans the whole input, so a smaller block is just a
            // narrower `(block_start, block_end)` range into it.
            let savings = start as i64 - (out.len() - blocks_start) as i64;
            // Borrowed path only: warm the pre-split window before the
            // cache-cold strided fingerprint read. Gated to exactly the
            // conditions under which `optimal_block_size` reads `block`
            // (a pre-split level, a full 128 KiB block remaining, the
            // block-size cap admits a full block, and `savings >= 3` so the
            // splitter actually runs) — so non-pre-split levels, the first
            // block, and the trailing partial block pay nothing. See
            // `warm_presplit_window`.
            if savings >= 3
                && input.len() - start >= MAX_BLOCK_SIZE as usize
                && block_capacity >= MAX_BLOCK_SIZE as usize
                && config::level_pre_split(self.compression_level).is_some()
            {
                warm_presplit_window(&input[start..start + MAX_BLOCK_SIZE as usize]);
            }
            let block_len = optimal_block_size(
                self.compression_level,
                &input[start..],
                input.len() - start,
                block_capacity,
                savings,
            );
            let end = (start + block_len).min(input.len());
            let block = &input[start..end];
            let last_block = end == input.len();

            if self.content_checksum {
                self.hasher.write(block);
            }
            let dict_active =
                self.dictionary.is_some() && self.state.matcher.supports_dictionary_priming();
            compress_block_encoded_borrowed(
                &mut self.state,
                self.compression_level,
                last_block,
                block,
                start,
                end,
                dict_active,
                out,
            );
            start = end;
        }
        // `_clear_guard` drops here, clearing the borrowed window.
        total_uncompressed
    }
}

impl<R: Read, W: Write, M: Matcher> FrameCompressor<R, W, M> {
    /// Create a new `FrameCompressor` with a custom matching algorithm implementation
    pub fn new_with_matcher(matcher: M, compression_level: CompressionLevel) -> Self {
        Self {
            uncompressed_data: None,
            compressed_data: None,
            dictionary: None,
            dictionary_entropy_cache: None,
            source_size_hint: None,
            state: CompressState {
                matcher,
                last_huff_table: None,
                huff_table_spare: None,
                fse_tables: FseTables::new(),
                block_scratch: crate::encoding::blocks::CompressedBlockScratch::new(),
                offset_hist: [1, 4, 8],
                strategy_tag: crate::encoding::strategy::StrategyTag::for_compression_level(
                    compression_level,
                ),
                huf_optimal_search: true,
                literal_compression_disabled: matches!(
                    compression_level,
                    CompressionLevel::Level(n) if n < 0
                ),
            },
            compression_level,
            magicless: false,
            content_checksum: false,
            content_size_flag: true,
            dict_id_flag: true,
            target_block_size: None,

            hasher: XxHash64::with_seed(0),

            strategy_override: None,
        }
    }

    /// Enable or disable magicless frame format (`ZSTD_f_zstd1_magicless`).
    ///
    /// When set to `true`, emitted frames omit the 4-byte magic number
    /// prefix. The matching decoder must be configured to expect a
    /// magicless stream — wire-format only round-trips with a
    /// magicless-aware decoder.
    pub fn set_magicless(&mut self, magicless: bool) {
        self.magicless = magicless;
    }

    /// Enable or disable the trailing XXH64 content checksum
    /// (semantics of upstream `ZSTD_c_checksumFlag`). Default `false`,
    /// matching the upstream library default (`ZSTD_c_checksumFlag = 0`)
    /// so out-of-the-box frames carry the same layout and pay the same
    /// costs as the reference implementation.
    ///
    /// When `false`, emitted frames set `Content_Checksum_flag = 0` and carry
    /// no trailing digest; such frames are valid (RFC 8878) and decode
    /// correctly in any [`ContentChecksum`](crate::decoding::ContentChecksum)
    /// mode. Without the `hash` feature no checksum is emitted regardless of
    /// this setting.
    pub fn set_content_checksum(&mut self, emit: bool) {
        self.content_checksum = emit;
    }

    /// Enable or disable recording `Frame_Content_Size` in the frame header
    /// when the total size is known (semantics of upstream
    /// `ZSTD_c_contentSizeFlag`). Default `true`, matching upstream. With
    /// the flag off the header carries a window descriptor instead (and the
    /// single-segment layout, which requires an FCS, is disabled).
    pub fn set_content_size_flag(&mut self, emit: bool) {
        self.content_size_flag = emit;
    }

    /// Enable or disable recording the dictionary ID in the frame header
    /// when a dictionary is attached (semantics of upstream
    /// `ZSTD_c_dictIDFlag`). Default `true`, matching upstream. Frames
    /// emitted with the flag off still decode when the decoder is handed
    /// the dictionary explicitly.
    pub fn set_dictionary_id_flag(&mut self, emit: bool) {
        self.dict_id_flag = emit;
    }

    /// Set an upper bound on emitted block sizes (semantics of upstream
    /// `ZSTD_c_targetCBlockSize`): every physical block's payload is capped
    /// at `target` bytes (+3-byte block header on the wire), trading some
    /// ratio for bounded per-block latency. The value is clamped to
    /// `[MIN_TARGET_BLOCK_SIZE, MAX_BLOCK_SIZE]` (the upstream bounds).
    /// `None` removes the target.
    pub fn set_target_block_size(&mut self, target: Option<u32>) {
        self.target_block_size =
            target.map(|t| t.clamp(crate::common::MIN_TARGET_BLOCK_SIZE, MAX_BLOCK_SIZE));
    }

    /// The active block-size cap: the configured target, or the format's
    /// 128 KiB block ceiling.
    fn block_capacity(&self) -> usize {
        self.target_block_size
            .map_or(MAX_BLOCK_SIZE as usize, |t| t as usize)
    }

    /// Before calling [FrameCompressor::compress] you need to set the source.
    ///
    /// This is the data that is compressed and written into the drain.
    pub fn set_source(&mut self, uncompressed_data: R) -> Option<R> {
        self.uncompressed_data.replace(uncompressed_data)
    }

    /// Before calling [FrameCompressor::compress] you need to set the drain.
    ///
    /// As the compressor compresses data, the drain serves as a place for the output to be writte.
    pub fn set_drain(&mut self, compressed_data: W) -> Option<W> {
        self.compressed_data.replace(compressed_data)
    }

    /// Provide a hint about the total uncompressed size for the next frame.
    ///
    /// When set, the encoder selects smaller hash tables and windows for
    /// small inputs, matching the C zstd source-size-class behavior.
    ///
    /// This hint applies only to frame payload bytes (`size`). Dictionary
    /// history is primed separately and does not inflate the hinted size or
    /// advertised frame window.
    /// Must be called before [`compress`](Self::compress).
    pub fn set_source_size_hint(&mut self, size: u64) {
        self.source_size_hint = Some(size);
    }

    /// Total heap bytes this compressor's allocations hold, excluding the
    /// inline struct: the match-finder tables / history / recycled buffers and
    /// the primed-dictionary snapshot (via the matcher), the retained
    /// Huffman tables (active + recycled spare), the retained dictionary
    /// content, the cached dictionary entropy tables (literals Huffman +
    /// LL/ML/OF FSE), and the per-block sidecar buffers. Lets a context
    /// report its true footprint through `ZSTD_sizeof_CCtx`.
    pub fn heap_size(&self) -> usize {
        let mut total = self.state.matcher.heap_size();
        total += self
            .state
            .last_huff_table
            .as_ref()
            .map_or(0, |table| table.heap_size());
        total += self
            .state
            .huff_table_spare
            .as_ref()
            .map_or(0, |table| table.heap_size());
        total += self
            .dictionary
            .as_ref()
            .map_or(0, |d| d.inner.dict_content.capacity());
        total += self
            .dictionary_entropy_cache
            .as_ref()
            .map_or(0, CachedDictionaryEntropy::heap_size);

        total
    }

    /// Compress the uncompressed data from the provided source as one Zstd frame and write it to the provided drain
    ///
    /// This will repeatedly call [Read::read] on the source to fill up blocks until the source returns 0 on the read call.
    /// All compressed blocks are buffered in memory so that the frame header can include the
    /// `Frame_Content_Size` field (which requires knowing the total uncompressed size). The
    /// entire frame — header, blocks, and optional checksum — is then written to the drain
    /// at the end. This means peak memory usage is O(compressed_size).
    ///
    /// To avoid endlessly encoding from a potentially endless source (like a network socket) you can use the
    /// [Read::take] function
    /// Per-frame setup values resolved by `prepare_frame` and
    /// consumed by the block loop + `finish_frame`. Lets the
    /// owned `compress()` and the borrowed one-shot path share the exact
    /// same reset / dict-prime / entropy-seed setup and frame tail.
    pub fn compress(&mut self) {
        let prep = self.prepare_frame();
        // Take the reader out so `run_owned_block_loop` can borrow it
        // mutably alongside `&mut self` (the rest of the loop touches
        // `self.state` / `self.hasher`, disjoint from the reader). Restored
        // before the frame tail so a reused compressor keeps its source.
        //
        // Deliberately NOT restored on unwind: if the block loop panics the
        // source has been partially consumed, so handing it back would let a
        // `catch_unwind` caller "successfully" compress the remaining tail
        // from an arbitrary midpoint — silent data corruption. Leaving the
        // slot empty makes any post-panic reuse fail loudly at the `expect`
        // below (matcher/entropy state is equally unre-usable after an
        // unwind; the reference implementation likewise requires a context
        // reset after an error).
        let mut source = self
            .uncompressed_data
            .take()
            .expect("source must be set via set_source before compress()");
        // Streaming drain: the content size is only known at EOF, so the
        // frame header can't precede the blocks — accumulate them in a local
        // buffer and let `finish_frame` write header + blocks to the drain.
        let mut all_blocks: Vec<u8> = Vec::with_capacity(initial_all_blocks_cap(
            prep.initial_size_hint,
            self.block_capacity(),
        ));
        let mut block_source = ReaderBlockSource::new(&mut source);
        let total_uncompressed = self.run_owned_block_loop(
            &mut block_source,
            prep.initial_size_hint,
            false,
            &mut all_blocks,
        );
        self.uncompressed_data = Some(source);
        self.finish_frame(all_blocks, total_uncompressed, &prep);
    }

    fn prepare_frame(&mut self) -> FramePrep {
        // Reset per-frame introspection state so a re-used compressor
        // doesn't carry over the previous frame's layout/checksums.

        let initial_size_hint = self.source_size_hint;
        let source_size_hint_known = initial_size_hint.is_some();
        let use_dictionary_state =
            !matches!(self.compression_level, CompressionLevel::Uncompressed)
                && self.state.matcher.supports_dictionary_priming()
                && self.dictionary.is_some();
        if let Some(size_hint) = self.source_size_hint.take() {
            // Keep source-size hint scoped to payload bytes; dictionary priming
            // is applied separately and should not force larger matcher sizing.
            self.state.matcher.set_source_size_hint(size_hint);
        }
        // Hand the matcher the dictionary's content size so its binary-tree /
        // hash-chain tables shrink to the dictionary's cParams tier (upstream zstd CDict
        // economics: the dictionary supplies long matches, so a source-sized live
        // table is wasted peak memory). The eviction window stays source-sized so
        // the dictionary bytes remain referenceable. Set before `reset` (which
        // consumes it) and only when a dictionary will actually be primed.
        if use_dictionary_state && let Some(dict) = self.dictionary.as_ref() {
            self.state
                .matcher
                .set_dictionary_size_hint(dict.inner.dict_content.len());
        }
        // Clearing buffers to allow re-using of the compressor
        self.state.matcher.reset(self.compression_level);
        self.state.offset_hist = [1, 4, 8];
        // Sync `state.strategy_tag` to the level resolved at this reset so
        // the literal-compression gates (`min_literals_to_compress` /
        // `min_gain` in `encoding::blocks::compressed`) see the correct
        // strategy for the next frame. Frame-by-frame level changes go
        // through this same `compress()` entry point, so re-syncing here
        // covers level switches without touching the matcher dispatch.
        // A public-parameter strategy override wins over the level's
        // derived tag so the literal-compression gates and dict-attach cutoff
        // below see the strategy the matcher actually runs. Otherwise resolve
        // the strategy SIZE-ADAPTIVELY through the same path the matcher's reset
        // used (`resolve_level_params` -> `get_cparams`, the port of upstream
        // `ZSTD_getCParams`): a small frame promotes a level to a higher
        // strategy (e.g. L13 over a <=16 KiB frame becomes btultra). Re-deriving
        // from the bare level would make the literal-compression / HUF-search
        // gates disagree with the matcher's actual parse on small frames (the
        // gate would think btlazy2 and skip the HUF table-log search the btultra
        // frame runs, costing a few bytes on small literal sections).
        self.state.strategy_tag = self.strategy_override.unwrap_or_else(|| {
            config::resolve_level_params(self.compression_level, initial_size_hint).strategy_tag
        });
        // `initial_size_hint` (captured before the `.take()` above) — by here
        // `self.source_size_hint` is None.
        self.state.huf_optimal_search =
            huf_search_enabled(self.state.strategy_tag, initial_size_hint);
        let cached_entropy = if use_dictionary_state {
            self.dictionary_entropy_cache.as_ref()
        } else {
            None
        };
        if use_dictionary_state && let Some(dict) = self.dictionary.as_ref() {
            // This state drives sequence encoding, while matcher priming below updates
            // the match generator's internal repeat-offset history for match finding.
            self.state.offset_hist = dict.inner.offset_hist;
            // Upstream zstd `ZSTD_shouldAttachDict` (`zstd_compress.c`): a
            // precomputed-dictionary table is COPIED into the working context
            // only when the source is larger than a per-strategy cutoff; at or
            // below it (and for unknown size) the upstream zstd ATTACHES the dictionary
            // tables by reference (no per-frame table touch at all). We don't
            // have an attach-by-reference path yet, so:
            //   - large source (> cutoff): reuse the captured prime snapshot
            //     (a table copy) instead of re-hashing the dictionary — the
            //     upstream zstd COPY regime, where the copy is cheaper than re-priming;
            //   - small / unknown source: re-prime (the snapshot copy of the
            //     whole table would cost MORE than the sparse re-prime here,
            //     which is exactly why the upstream zstd attaches by reference instead).
            // `attachDictSizeCutoffs` per strategy: fast 8K, dfast 16K,
            // greedy/lazy/btopt 32K, btultra/btultra2 8K. Expressed as the
            // ceil-log bucket (8K = 2^13, 16K = 2^14, 32K = 2^15) so the
            // decision uses the SAME bucketed representation as the driver's
            // attach/copy gate (`reset_size_log`) — comparing
            // `source_size_ceil_log(hint)` on the full u64 avoids the `as usize`
            // truncation that could diverge from the driver on 32-bit targets.
            // For a power-of-two cutoff `2^k`, `ceil_log2(hint) > k` is exactly
            // `hint > 2^k`, so this is identical to the raw `hint > cutoff` on
            // 64-bit.
            let cutoff_log = match self.state.strategy_tag {
                // Fast always attaches now (the copy-mode owned path memmoved the
                // whole input into history every frame); keep the copy-snapshot
                // gate in sync with the matcher's attach cutoff so Fast never
                // captures/restores a copy snapshot it can no longer use.
                crate::encoding::strategy::StrategyTag::Fast => config::FAST_ATTACH_DICT_CUTOFF_LOG,
                crate::encoding::strategy::StrategyTag::BtUltra
                | crate::encoding::strategy::StrategyTag::BtUltra2 => 13,
                crate::encoding::strategy::StrategyTag::Dfast => 14,
                crate::encoding::strategy::StrategyTag::Greedy
                | crate::encoding::strategy::StrategyTag::Lazy
                | crate::encoding::strategy::StrategyTag::Btlazy2
                | crate::encoding::strategy::StrategyTag::BtOpt => 15,
            };
            if self.state.matcher.dictionary_is_resident() {
                // Re-borrow fast path: the previous frame's reset kept this
                // dict's bytes + cached index resident, so skip the re-commit /
                // re-index and only reapply the offset history.
                self.state
                    .matcher
                    .reapply_resident_dictionary(dict.inner.offset_hist);
            } else {
                let prefer_copy_snapshot =
                    initial_size_hint.is_some_and(|s| config::source_size_ceil_log(s) > cutoff_log);
                let restored = prefer_copy_snapshot
                    && self
                        .state
                        .matcher
                        .restore_primed_dictionary(self.compression_level);
                if !restored {
                    self.state.matcher.prime_with_dictionary(
                        dict.inner.dict_content.as_slice(),
                        dict.inner.offset_hist,
                    );
                    if prefer_copy_snapshot {
                        self.state
                            .matcher
                            .capture_primed_dictionary(self.compression_level);
                    }
                }
            }
        }
        if let Some(cache) = cached_entropy {
            // Refill an empty slot from the recycled spare before
            // `clone_from`: `Option::clone_from(None ← Some)` falls back to
            // a fresh clone (two Vec allocations), while `Some ← Some`
            // delegates to the table's buffer-reusing `clone_from`. Frames
            // whose last block cleared the table would otherwise re-clone
            // the dict seed every frame.
            match &cache.huff {
                Some(src) => {
                    if self.state.last_huff_table.is_none() {
                        self.state.last_huff_table = self.state.huff_table_spare.take();
                    }
                    match &mut self.state.last_huff_table {
                        Some(dst) => dst.clone_from(src),
                        slot => *slot = Some(src.clone()),
                    }
                }
                None => self.state.clear_huff_table(),
            }
        } else {
            self.state.clear_huff_table();
        }
        // `clone_from` keeps frame-to-frame seeding cheap for reused compressors by
        // reusing existing allocations where possible instead of reallocating every frame.
        if let Some(cache) = cached_entropy {
            self.state
                .fse_tables
                .ll_previous
                .clone_from(&cache.ll_previous);
            self.state
                .fse_tables
                .ml_previous
                .clone_from(&cache.ml_previous);
            self.state
                .fse_tables
                .of_previous
                .clone_from(&cache.of_previous);
        } else {
            self.state.fse_tables.ll_previous = None;
            self.state.fse_tables.ml_previous = None;
            self.state.fse_tables.of_previous = None;
        }
        let ll_entropy = cached_entropy.and_then(|cache| match cache.ll_previous.as_ref() {
            Some(PreviousFseTable::Custom(table)) => Some(table.as_ref()),
            _ => None,
        });
        let ml_entropy = cached_entropy.and_then(|cache| match cache.ml_previous.as_ref() {
            Some(PreviousFseTable::Custom(table)) => Some(table.as_ref()),
            _ => None,
        });
        let of_entropy = cached_entropy.and_then(|cache| match cache.of_previous.as_ref() {
            Some(PreviousFseTable::Custom(table)) => Some(table.as_ref()),
            _ => None,
        });
        self.state.matcher.seed_dictionary_entropy(
            self.state.last_huff_table.as_ref(),
            ll_entropy,
            ml_entropy,
            of_entropy,
        );

        {
            self.hasher = XxHash64::with_seed(0);
        }
        let window_size = self.state.matcher.window_size();
        assert!(
            window_size != 0,
            "matcher reported window_size == 0, which is invalid"
        );
        FramePrep {
            window_size,
            use_dictionary_state,
            source_size_hint_known,
            initial_size_hint,
        }
    }

    /// Owned streaming block loop: reads blocks from the caller-provided
    /// `source` reader, optionally pre-splits, hashes for the content
    /// checksum, and emits each block via `compress_block_encoded`,
    /// accumulating the block bytes. Returns `(all_blocks,
    /// total_uncompressed)`. The source is passed in (rather than read
    /// from `self.uncompressed_data`) so the streaming `compress` path can
    /// feed the configured reader while the slice paths
    /// (`compress_oneshot_borrowed`, `compress_independent_frame`) feed an
    /// in-place `&[u8]` cursor without baking its lifetime into the
    /// compressor type.
    fn run_owned_block_loop<S: OwnedBlockSource>(
        &mut self,
        source: &mut S,
        initial_size_hint: Option<u64>,
        // Whether `initial_size_hint` is the input's exact length (the
        // one-shot slice paths) or a caller-provided estimate (the streaming
        // `Read` path, where `set_source_size_hint` is advisory). An exact
        // hint drives the one-shot ratio reservation; an estimate is only
        // trusted up to a small lookahead past the bytes actually read.
        hint_is_exact: bool,
        out: &mut Vec<u8>,
    ) -> u64 {
        // Compressed blocks are appended to `out` from its current end. The
        // streaming drain path passes a fresh buffer (the frame header is
        // written to the drain afterward, since Frame_Content_Size is only
        // known once the reader hits EOF); the one-shot compress-into-Vec
        // path passes `out` already holding the header. The upstream zstd split
        // `savings` gate below accumulates block-relative (`before_len`)
        // output deltas, so a header prefix never skews it.
        let blocks_start = out.len();
        let mut total_uncompressed: u64 = 0;
        let mut pending_input: Vec<u8> = Vec::new();
        let mut reached_eof = false;
        let mut savings = 0i64;
        // Compress block by block
        loop {
            // Read up to one upstream zstd block. When the pre-block splitter keeps a
            // suffix, top it back up before compressing the next block, matching
            // ZSTD_compress_frameChunk() over a contiguous input buffer.
            let block_capacity = self.block_capacity();
            // Always draw the block buffer from the matcher's recycled pool
            // (its capacity already covers the block size, so the resize below
            // stays in-place). Any carried pre-split suffix is copied in, and
            // `pending_input` is retained as a reusable carry buffer. The prior
            // approach `split_off`'d a fresh suffix Vec per pre-split and
            // `reserve_exact`-grew it to `block_capacity` every block; on a
            // heavily pre-split frame that churned one block-sized allocation
            // per split (~12 MB over ~90 splits on a 1 MiB corpus input).
            let mut uncompressed_data = self.state.matcher.get_next_space();
            uncompressed_data.clear();
            uncompressed_data.extend_from_slice(&pending_input);
            pending_input.clear();
            if !reached_eof {
                // Remaining-bytes expectation for the reader source's sizing
                // (`None` = unknown, or an inexact hint already met by prior
                // blocks). The slice source appends directly and ignores it.
                let size_hint_remaining = match initial_size_hint {
                    Some(hint) if hint > total_uncompressed => Some(hint - total_uncompressed),
                    _ => None,
                };
                let (appended, eof) =
                    source.fill_block(&mut uncompressed_data, block_capacity, size_hint_remaining);
                total_uncompressed += appended as u64;
                reached_eof = eof;
            }
            let mut last_block = reached_eof;
            let remaining_for_split = if reached_eof {
                uncompressed_data.len()
            } else {
                block_capacity
            };
            if !matches!(self.compression_level, CompressionLevel::Uncompressed)
                && uncompressed_data.len() == block_capacity
            {
                let block_len = optimal_block_size(
                    self.compression_level,
                    &uncompressed_data,
                    remaining_for_split,
                    block_capacity,
                    savings,
                );
                if block_len < uncompressed_data.len() {
                    // Carry the kept suffix into the reusable `pending_input`
                    // buffer (cleared, capacity retained) instead of allocating
                    // a fresh Vec via `split_off`. Next iteration copies it back
                    // into a pooled block buffer. The block currently being
                    // compressed is truncated to the chosen split length.
                    pending_input.clear();
                    pending_input.extend_from_slice(&uncompressed_data[block_len..]);
                    uncompressed_data.truncate(block_len);
                    last_block = false;
                }
            }
            // As we read, hash that data too (skipped when the content
            // checksum is disabled).

            if self.content_checksum {
                self.hasher.write(&uncompressed_data);
            }
            // Per-physical-block XXH64 (low 32 bits) for the optional
            // per-block checksum sidecar. Hashing happens INSIDE the
            // block emitters (RLE / Raw fast-path / Compressed /
            // post-split partitions), so the digests vector has
            // exactly one entry per physical Block_Header written to
            // `all_blocks` — 1:1 with `FrameEmitInfo.blocks`. See
            // `enable_per_block_checksums` rustdoc.
            // Size the output ahead of this block's emission from the ratio
            // observed so far (see `reserve_for_next_block`); with no usable
            // size hint, ensure one block's worst case and let the doubling
            // growth policy amortize across blocks.
            let emitted =
                total_uncompressed - uncompressed_data.len() as u64 - pending_input.len() as u64;
            match initial_size_hint {
                Some(hint) if hint >= total_uncompressed => {
                    // An advisory hint (streaming path) is only trusted up to
                    // a small lookahead past the bytes actually read: a hint
                    // far above the real input would otherwise reserve the
                    // whole phantom remainder up front.
                    let hint_remaining = hint - emitted;
                    let remaining = if hint_is_exact {
                        hint_remaining
                    } else {
                        let buffered = total_uncompressed - emitted;
                        const HINT_LOOKAHEAD: u64 = 64 * 1024;
                        hint_remaining.min(buffered + HINT_LOOKAHEAD)
                    };
                    reserve_for_next_block(
                        out,
                        blocks_start,
                        emitted,
                        remaining as usize,
                        self.block_capacity(),
                    );
                }
                _ => {
                    out.reserve(uncompressed_data.len() + 3 + 16);
                }
            }
            // Special handling is needed for compression of a totally empty file
            if uncompressed_data.is_empty() {
                let header = BlockHeader {
                    last_block: true,
                    block_type: crate::blocks::block::BlockType::Raw,
                    block_size: 0,
                };
                header.serialize(out);

                break;
            }

            match self.compression_level {
                CompressionLevel::Uncompressed => {
                    let header = BlockHeader {
                        last_block,
                        block_type: crate::blocks::block::BlockType::Raw,
                        block_size: uncompressed_data.len().try_into().unwrap(),
                    };
                    header.serialize(out);

                    out.extend_from_slice(&uncompressed_data);
                    savings +=
                        uncompressed_data.len() as i64 - (3 + uncompressed_data.len()) as i64;
                }
                CompressionLevel::Fastest
                | CompressionLevel::Default
                | CompressionLevel::Better
                | CompressionLevel::Best
                | CompressionLevel::Level(_) => {
                    let before_len = out.len();
                    let block_len = uncompressed_data.len();
                    // A primed dictionary makes "incompressible-looking"
                    // blocks matchable against the dict, so the raw-fast-
                    // path inside must be bypassed (it skips matching).
                    // Mirror prepare_frame's `use_dictionary_state`: a dict
                    // is only PRIMED (and thus matchable) when the matcher
                    // supports priming — a non-priming matcher ignores an
                    // attached dictionary, so the raw-fast-path must stay
                    // enabled for it. (This arm is already non-Uncompressed.)
                    let dict_active = self.dictionary.is_some()
                        && self.state.matcher.supports_dictionary_priming();
                    compress_block_encoded(
                        &mut self.state,
                        self.compression_level,
                        last_block,
                        uncompressed_data,
                        out,
                        dict_active,
                    );
                    savings += block_len as i64 - (out.len() - before_len) as i64;
                }
            }
            if last_block && pending_input.is_empty() {
                break;
            }
        }
        total_uncompressed
    }

    /// Append the frame header bytes onto `out` once the total payload size
    /// is known (so `Frame_Content_Size` / `single_segment` can be set).
    /// Appends rather than returns so the one-shot path serializes straight
    /// into the reused output buffer with no per-frame header `Vec`.
    fn append_frame_header(&self, total_uncompressed: u64, prep: &FramePrep, out: &mut Vec<u8>) {
        // Match the upstream zstd framing policy (`ZSTD_writeFrameHeader`):
        // single-segment whenever the content size is known and the whole
        // source fits the active window (`contentSizeFlag && windowSize >=
        // srcSize`). A single-segment frame REQUIRES an FCS field, so
        // suppressing the content size (`content_size_flag` off) forces the
        // windowed layout. There is no lower size bound: small payloads
        // benefit most, since a windowed frame cannot encode a content size
        // below 256 in fewer than 4 FCS bytes (the 1-byte FCS class is
        // single-segment-only, see `find_fcs_field_size`), whereas a
        // single-segment frame stores it in one byte and omits the window
        // descriptor. The single-segment window equals the FCS, so a block
        // must never reference past the content: the post-hoc raw fallback in
        // the block emitters guarantees any non-shrinking block is stored raw,
        // and genuine matches stay within the already-emitted output.
        // Dictionary frames qualify too (the dictionary is decoder setup
        // state, not part of the regenerated segment), keeping the decoder's
        // single-allocation path (our decoder caps reservation to
        // min(window, FCS) either way).
        let single_segment = self.content_size_flag
            && prep.source_size_hint_known
            && total_uncompressed <= prep.window_size;
        let header = FrameHeader {
            frame_content_size: self.content_size_flag.then_some(total_uncompressed),
            single_segment,
            content_checksum: self.content_checksum,
            dictionary_id: if prep.use_dictionary_state && self.dict_id_flag {
                self.dictionary.as_ref().map(|dict| dict.inner.id as u64)
            } else {
                None
            },
            window_size: if single_segment {
                None
            } else {
                Some(prep.window_size)
            },
            magicless: self.magicless,
        };
        header.serialize(out);
    }

    /// Write the frame header, accumulated block bytes, and optional
    /// trailing content checksum to the configured drain; populate
    /// `frame_emit_info` (lsm). Header and blocks are written separately to
    /// avoid shifting `all_blocks` to prepend the header. Used by
    /// `compress` and `compress_oneshot_borrowed`.
    fn finish_frame(&mut self, all_blocks: Vec<u8>, total_uncompressed: u64, prep: &FramePrep) {
        let mut header_buf: Vec<u8> = Vec::with_capacity(18);
        self.append_frame_header(total_uncompressed, prep, &mut header_buf);
        // Snapshot the checksum before borrowing the drain field so the
        // `self.hasher` read and the `self.compressed_data` write don't
        // both need `&mut self` simultaneously.

        let checksum_bytes = self
            .content_checksum
            .then(|| (self.hasher.finish() as u32).to_le_bytes());
        let drain = self.compressed_data.as_mut().unwrap();
        drain.write_all(&header_buf).unwrap();
        drain.write_all(&all_blocks).unwrap();
        // With the `hash` feature AND the content checksum enabled, the header
        // set `Content_Checksum_flag` and the 32-bit digest is written at the
        // end of the frame. Disabled => no trailing bytes, flag stays 0.

        if let Some(checksum_bytes) = checksum_bytes {
            drain.write_all(&checksum_bytes).unwrap();
        }
    }

    /// Get a mutable reference to the source
    pub fn source_mut(&mut self) -> Option<&mut R> {
        self.uncompressed_data.as_mut()
    }

    /// Get a mutable reference to the drain
    pub fn drain_mut(&mut self) -> Option<&mut W> {
        self.compressed_data.as_mut()
    }

    /// Get a reference to the source
    pub fn source(&self) -> Option<&R> {
        self.uncompressed_data.as_ref()
    }

    /// Get a reference to the drain
    pub fn drain(&self) -> Option<&W> {
        self.compressed_data.as_ref()
    }

    /// Retrieve the source
    pub fn take_source(&mut self) -> Option<R> {
        self.uncompressed_data.take()
    }

    /// Retrieve the drain
    pub fn take_drain(&mut self) -> Option<W> {
        self.compressed_data.take()
    }

    /// Before calling [FrameCompressor::compress] you can replace the matcher
    pub fn replace_matcher(&mut self, mut match_generator: M) -> M {
        core::mem::swap(&mut match_generator, &mut self.state.matcher);
        match_generator
    }

    /// Before calling [FrameCompressor::compress] you can replace the compression level.
    ///
    /// This also clears any fine-grained parameter overrides installed via
    /// [`set_parameters`](Self::set_parameters): reverting to a bare level
    /// means plain level-based tuning, not the previous frame's customized
    /// strategy / LDM / log overrides. To keep overriding, call
    /// [`set_parameters`](Self::set_parameters) again with the new base level.
    pub fn set_compression_level(
        &mut self,
        compression_level: CompressionLevel,
    ) -> CompressionLevel {
        let old = self.compression_level;
        self.compression_level = compression_level;
        // Resync the raw-literals gate: negative levels disable literal (Huffman)
        // compression (C `ZSTD_literalsCompressionIsDisabled`). `prepare_frame`
        // never recomputes this, so it must be refreshed on the level switch the
        // same way the constructors and `set_parameters` do.
        self.state.literal_compression_disabled = matches!(
            compression_level,
            CompressionLevel::Level(n) if n < 0
        );
        // Drop sticky overrides so the level switch yields plain geometry.
        self.strategy_override = None;
        self.state.matcher.clear_param_overrides();
        old
    }

    /// Get the current compression level
    pub fn compression_level(&self) -> CompressionLevel {
        self.compression_level
    }

    /// Attach a pre-parsed dictionary to be used for subsequent compressions.
    ///
    /// In compressed modes, the dictionary id is written only when the active
    /// matcher supports dictionary priming.
    /// Uncompressed mode and non-priming matchers ignore the attached dictionary
    /// at encode time.
    pub fn set_dictionary(
        &mut self,
        dictionary: crate::decoding::Dictionary,
    ) -> Result<Option<EncoderDictionary>, crate::decoding::errors::DictionaryDecodeError> {
        self.attach_dictionary(EncoderDictionary::from_dictionary(dictionary))
    }

    /// Parse and attach a serialized dictionary blob.
    ///
    /// Parses with the encoder-only path (skips the FSE/HUF decode lookup-table
    /// build the encoder never reads); the entropy ENCODER tables — and thus
    /// the emitted frame — are identical to a full parse.
    pub fn set_dictionary_from_bytes(
        &mut self,
        raw_dictionary: &[u8],
    ) -> Result<Option<EncoderDictionary>, crate::decoding::errors::DictionaryDecodeError> {
        self.attach_dictionary(EncoderDictionary::from_bytes(raw_dictionary)?)
    }

    /// Attach an already-parsed [`EncoderDictionary`] without reparsing a raw
    /// blob.
    ///
    /// Accepts an `EncoderDictionary` produced once via
    /// [`EncoderDictionary::from_bytes`] / [`EncoderDictionary::from_dictionary`]
    /// or handed back by [`Self::clear_dictionary`] / the `set_dictionary*`
    /// return value, so callers can reattach or reuse a prepared dictionary
    /// across compressions without re-running the dictionary parse each time.
    /// Returns the previously-attached dictionary, if any.
    pub fn set_encoder_dictionary(
        &mut self,
        dictionary: EncoderDictionary,
    ) -> Result<Option<EncoderDictionary>, crate::decoding::errors::DictionaryDecodeError> {
        self.attach_dictionary(dictionary)
    }

    /// Remove the attached dictionary, returning it as an [`EncoderDictionary`].
    pub fn clear_dictionary(&mut self) -> Option<EncoderDictionary> {
        self.dictionary_entropy_cache = None;
        // Drop the CDict prime snapshot — it is keyed to the dictionary
        // being removed and must not be restored against a different (or no)
        // dictionary on the next frame.
        self.state.matcher.invalidate_primed_dictionary();
        self.dictionary.take()
    }

    /// Validate `enc`, build the encoder entropy cache from it, store it, and
    /// return the previously-attached dictionary. Shared by every public
    /// attach entry point: `set_dictionary`, `set_dictionary_from_bytes`, and
    /// `set_encoder_dictionary`.
    fn attach_dictionary(
        &mut self,
        enc: EncoderDictionary,
    ) -> Result<Option<EncoderDictionary>, crate::decoding::errors::DictionaryDecodeError> {
        let dictionary = &enc.inner;
        if dictionary.id == 0 {
            return Err(crate::decoding::errors::DictionaryDecodeError::ZeroDictionaryId);
        }
        if let Some(index) = dictionary.offset_hist.iter().position(|&rep| rep == 0) {
            return Err(
                crate::decoding::errors::DictionaryDecodeError::ZeroRepeatOffsetInDictionary {
                    index: index as u8,
                },
            );
        }
        self.dictionary_entropy_cache = Some(CachedDictionaryEntropy::from_dictionary(dictionary));
        // A previously-captured CDict prime snapshot belongs to the OLD
        // dictionary; drop it so the first frame with the new dictionary
        // re-primes (and re-captures) instead of restoring stale tables.
        self.state.matcher.invalidate_primed_dictionary();
        Ok(self.dictionary.replace(enc))
    }
}
