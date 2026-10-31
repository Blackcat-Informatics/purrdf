#![allow(
    clippy::assert_is_empty,
    clippy::assigning_clones,
    clippy::cast_lossless,
    clippy::manual_midpoint,
    clippy::map_unwrap_or,
    clippy::semicolon_if_nothing_returned,
    clippy::unnecessary_semicolon,
    clippy::unreadable_literal,
    clippy::use_self,
    clippy::useless_let_if_seq,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use alloc::{boxed::Box, vec::Vec};

use crate::{
    bit_io::BitWriter,
    blocks::block::BlockType,
    encoding::block_header::BlockHeader,
    encoding::frame_compressor::{CompressState, FseTables, PreviousFseTable, SharedFseTable},
    encoding::{Matcher, Sequence},
    fse::fse_encoder::{
        FSETable, build_seq_ctable, build_table_from_symbol_counts, fse_header_bits_for_counts,
    },
    huff0::huff0_encoder,
};

const MIN_SEQUENCES_BLOCK_SPLITTING: usize = 300;
const MAX_NB_BLOCK_SPLITS: usize = 196;

/// Upstream zstd `ZSTD_minLiteralsToCompress` (`zstd_compress_literals.c:114-127`):
/// strategy-aware floor below which `compress_literals` does not even
/// attempt huf compression and falls back to raw.
///
/// Formula: `shift = MIN(9 - strategy, 3); mintc = (huf_repeat ==
/// valid) ? 6 : (8 << shift)`. With huf reuse available, the per-block huf
/// header overhead is gone, so the cheap floor is 6 bytes. Without it, the
/// huf tree-description must be serialized per block — alphabet size and
/// max symbol determine its exact byte cost, but on payloads near the
/// per-strategy floor that overhead dominates and the compressed section
/// loses to raw. Upstream zstd's shift table picks the floor per strategy:
/// strategy 1..6 → 64 bytes, strategy 7 (btopt) → 32, strategy 8 (btultra)
/// → 16, strategy 9 (btultra2) → 8.
///
/// Our `StrategyTag` enum has eight variants: `Lazy` covers upstream zstd strategies
/// 4..5 (greedy/lazy/lazy2) and `Btlazy2` is the separate upstream zstd strategy 6.
/// Within the fast..btlazy2 band upstream zstd's shift table is flat: strategies 1..6
/// all pin `shift = MIN(9 - strat, 3) = 3`, so both `Lazy` and `Btlazy2` land
/// on the 64-byte floor. No aggressiveness gradient within this band to
/// preserve (the gradient only starts at btopt).
#[inline]
fn min_literals_to_compress(
    strategy: crate::encoding::strategy::StrategyTag,
    has_huf_table: bool,
) -> usize {
    use crate::encoding::strategy::StrategyTag;
    if has_huf_table {
        return 6;
    }
    let shift: u32 = match strategy {
        StrategyTag::Fast
        | StrategyTag::Dfast
        | StrategyTag::Greedy
        | StrategyTag::Lazy
        | StrategyTag::Btlazy2 => 3,
        StrategyTag::BtOpt => 2,
        StrategyTag::BtUltra => 1,
        StrategyTag::BtUltra2 => 0,
    };
    8usize << shift
}

/// Upstream zstd `ZSTD_minGain` (`zstd_compress_internal.h:677-684`):
/// strategy-aware minimum-compression margin. In upstream zstd it gates both
/// the block-level "compressed block must beat raw + minGain" decision
/// and the literal-section `cLitSize >= srcSize - minGain` fallback.
///
/// Formula: `minlog = (strat >= btultra) ? strat - 1 : 6; (src_size >>
/// minlog) + 2`. So:
/// - fast..btopt (strat 1..7): minlog=6 → ~1.5% margin + 2 bytes
/// - btultra (strat 8): minlog=7 → ~0.78% margin + 2 bytes
/// - btultra2 (strat 9): minlog=8 → ~0.39% margin + 2 bytes
///
/// **Current usage in this crate:** wired into the literal-section
/// raw-fallback gate (`compress_literals` +
/// `estimate_literals_section_bytes`) only — those sites previously
/// had no margin at all (bare `>= raw_section_bytes`).
/// **Not yet wired into** the block-level emit/probe paths
/// (`emit_single_sequence_block`, `SplitEstimator::estimate_subblock_size`),
/// which still use a uniform `(source_len >> 8) + 2` calculation
/// (the btultra2 value applied across all strategies). Migrating
/// those sites is a separate cleanup.
#[inline]
fn min_gain(src_size: usize, strategy: crate::encoding::strategy::StrategyTag) -> usize {
    use crate::encoding::strategy::StrategyTag;
    let minlog: u32 = match strategy {
        StrategyTag::BtUltra => 7,
        StrategyTag::BtUltra2 => 8,
        _ => 6,
    };
    (src_size >> minlog) + 2
}

/// Upstream zstd `compress_literals` raw-fallback gate
/// (`zstd_compress_literals.c:187-188`): emit raw when
/// `cLitSize >= srcSize - minGain`, where `cLitSize` is the HUF payload
/// plus tree description (the bytes `HUF_compress*` writes — excluding
/// the surrounding literals lhSize) and `srcSize` is the literal-payload
/// length. Compares payload-vs-srcSize, NOT on-wire-vs-on-wire, so the
/// gate is symmetric in header overhead.
///
/// Centralized helper so `compress_literals` and
/// `estimate_literals_section_bytes` share the exact same decision and
/// neither side can drift back to the pre-2026-05 on-wire comparison
/// (which inflated the threshold by `compressed_lhsize - raw_lhsize`
/// bytes and rejected marginally-winning compressed sections).
#[inline]
fn use_raw_literal_fallback(
    huf_section_size: usize,
    literals_len: usize,
    strategy: crate::encoding::strategy::StrategyTag,
) -> bool {
    huf_section_size >= literals_len.saturating_sub(min_gain(literals_len, strategy))
}

/// Upstream zstd `kInverseProbabilityLog256`: floor(-log2(x / 256) * 256).
const INVERSE_PROBABILITY_LOG_256: [usize; 256] = [
    0, 2048, 1792, 1642, 1536, 1453, 1386, 1329, 1280, 1236, 1197, 1162, 1130, 1100, 1073, 1047,
    1024, 1001, 980, 960, 941, 923, 906, 889, 874, 859, 844, 830, 817, 804, 791, 779, 768, 756,
    745, 734, 724, 714, 704, 694, 685, 676, 667, 658, 650, 642, 633, 626, 618, 610, 603, 595, 588,
    581, 574, 567, 561, 554, 548, 542, 535, 529, 523, 517, 512, 506, 500, 495, 489, 484, 478, 473,
    468, 463, 458, 453, 448, 443, 438, 434, 429, 424, 420, 415, 411, 407, 402, 398, 394, 390, 386,
    382, 377, 373, 370, 366, 362, 358, 354, 350, 347, 343, 339, 336, 332, 329, 325, 322, 318, 315,
    311, 308, 305, 302, 298, 295, 292, 289, 286, 282, 279, 276, 273, 270, 267, 264, 261, 258, 256,
    253, 250, 247, 244, 241, 239, 236, 233, 230, 228, 225, 222, 220, 217, 215, 212, 209, 207, 204,
    202, 199, 197, 194, 192, 190, 187, 185, 182, 180, 178, 175, 173, 171, 168, 166, 164, 162, 159,
    157, 155, 153, 151, 149, 146, 144, 142, 140, 138, 136, 134, 132, 130, 128, 126, 123, 121, 119,
    117, 115, 114, 112, 110, 108, 106, 104, 102, 100, 98, 96, 94, 93, 91, 89, 87, 85, 83, 82, 80,
    78, 76, 74, 73, 71, 69, 67, 66, 64, 62, 61, 59, 57, 55, 54, 52, 50, 49, 47, 46, 44, 42, 41, 39,
    37, 36, 34, 33, 31, 30, 28, 26, 25, 23, 22, 20, 19, 17, 16, 14, 13, 11, 10, 8, 7, 5, 4, 2, 1,
];

/// Compile-time guarantee that MAX_BLOCK_SIZE fits in the 18-bit size format.
const _: () = assert!(crate::common::MAX_BLOCK_SIZE <= 262_143);

#[derive(Default)]
struct EncodedBlockParts {
    literals: Vec<u8>,
    sequences: Vec<RawSequence>,
}

#[derive(Default)]
pub(crate) struct CompressedBlockScratch {
    parts: EncodedBlockParts,
    partitions: Vec<usize>,
    prefix_sums: SequencePrefixSums,
    compressed: Vec<u8>,
    estimator_sequences: Vec<crate::blocks::sequence_section::Sequence>,
    /// Lazily allocated: only the block-split estimator path uses it, and
    /// `compress_block`'s `mem::take` constructs a throwaway `Default`
    /// scratch every block — an eager workspace made that default pay four
    /// 2 KiB `Box<[usize; 256]>` allocations per block for paths that never
    /// probe a split.
    estimator_workspace: Option<EstimatorWorkspace>,
    /// Reusable scratch for the block-split estimator's inner
    /// `CompressState` — kept across frames so the estimator does not
    /// re-allocate a whole `CompressedBlockScratch` (4×`Box<[u32;256]>`
    /// count tables + Vecs) every frame in a reused compressor. `Box`
    /// breaks the type recursion; `None` by default (lazily filled on
    /// first block-split). The estimator uses `EntropyOnlyMatcher` and
    /// never re-splits, so this nesting is one level deep.
    estimator_inner: Option<Box<CompressedBlockScratch>>,
    /// Persistent slot for `compress_block_encoded`'s pre-block entropy
    /// rollback snapshot. `clone_from` into this slot reuses its `Vec`
    /// buffers across blocks; a fresh `.clone()` per block paid a
    /// malloc + free pair on both Huffman code containers every block.
    pub(crate) huff_rollback: Option<huff0_encoder::HuffmanTable>,
}

impl CompressedBlockScratch {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
struct SequencePrefixSums {
    lit: Vec<usize>,
    ml: Vec<usize>,
}

impl SequencePrefixSums {
    fn rebuild(&mut self, sequences: &[RawSequence]) {
        self.lit.clear();
        self.ml.clear();
        // `Vec::reserve_exact(additional)` adds `additional` elements ABOVE
        // current length, not capacity. Subtracting `capacity` here would
        // request `N - cap` more, leaving the Vec with `cap = max(cap, N-cap)`
        // — still below the `N` we need whenever `cap < N/2`, forcing a
        // reallocation on the very next `push`. After `clear()` length is 0,
        // so subtracting `len()` (here always 0) is the correct delta.
        let target = sequences.len() + 1;
        if self.lit.capacity() < target {
            self.lit.reserve_exact(target - self.lit.len());
        }
        if self.ml.capacity() < target {
            self.ml.reserve_exact(target - self.ml.len());
        }
        self.lit.push(0);
        self.ml.push(0);
        for seq in sequences {
            self.lit
                .push(*self.lit.last().unwrap_or(&0) + seq.ll as usize);
            self.ml
                .push(*self.ml.last().unwrap_or(&0) + seq.ml as usize);
        }
    }

    fn lit_range(&self, start: usize, end: usize) -> usize {
        self.lit[end] - self.lit[start]
    }

    fn ml_range(&self, start: usize, end: usize) -> usize {
        self.ml[end] - self.ml[start]
    }
}

#[derive(Clone, Copy)]
struct RawSequence {
    ll: u32,
    ml: u32,
    offset: u32,
}

struct EntropyOnlyMatcher;

enum HuffmanTableUpdate {
    New(huff0_encoder::HuffmanTable),
    Reused,
    Cleared,
}

impl Matcher for EntropyOnlyMatcher {
    fn get_next_space(&mut self) -> Vec<u8> {
        unreachable!("entropy estimator never requests input space")
    }

    fn get_last_space(&mut self) -> &[u8] {
        unreachable!("entropy estimator never reads source bytes")
    }

    fn commit_space(&mut self, _space: Vec<u8>) {
        unreachable!("entropy estimator never commits input")
    }

    fn skip_matching(&mut self) {
        unreachable!("entropy estimator never updates match state")
    }

    fn start_matching(&mut self, _handle_sequence: impl for<'a> FnMut(Sequence<'a>)) {
        unreachable!("entropy estimator never generates sequences")
    }

    fn reset(&mut self, _level: crate::encoding::CompressionLevel) {}

    fn window_size(&self) -> u64 {
        0
    }
}

/// A block of [`crate::common::BlockType::Compressed`]
pub(crate) fn compress_block<M: Matcher>(state: &mut CompressState<M>, output: &mut Vec<u8>) {
    let mut scratch = core::mem::take(&mut state.block_scratch);
    collect_block_parts(state, &mut scratch.parts);
    encode_block_parts_with_sequence_scratch(
        state,
        &scratch.parts.literals,
        &scratch.parts.sequences,
        output,
        &mut scratch.estimator_sequences,
    );
    state.block_scratch = scratch;
}

pub(crate) fn compress_block_with_post_split<M: Matcher>(
    state: &mut CompressState<M>,
    last_block: bool,
    output: &mut Vec<u8>,
) {
    let mut scratch = core::mem::take(&mut state.block_scratch);
    collect_block_parts(state, &mut scratch.parts);
    if scratch.parts.sequences.len() <= 4 {
        let source_len = state.matcher.get_last_space().len();

        // `block_checksums: Option<&mut Vec<u32>>`; `as_deref_mut` unwraps
        // exactly one level of `&mut`, yielding `Option<&mut Vec<u32>>` here
        // (the blanket `impl<T: ?Sized> Deref for &mut T` has
        // `Target = T`, so the deref chain does NOT cascade into
        // `Vec<u32>::Target = [u32]`). Hence `sink: &mut Vec<u32>` and
        // `Vec::push` is in scope.

        scratch.compressed.clear();
        let mut emit_buffers = SingleSequenceEmitBuffers {
            output,
            compressed: &mut scratch.compressed,
            sequence_scratch: &mut scratch.estimator_sequences,
        };
        let emitted_raw = emit_single_sequence_block(
            state,
            last_block,
            source_len,
            &scratch.parts.literals,
            &scratch.parts.sequences,
            &mut emit_buffers,
        );
        if emitted_raw {
            output.extend_from_slice(state.matcher.get_last_space());
        }
        state.block_scratch = scratch;
        return;
    }

    scratch.partitions.clear();
    scratch.prefix_sums.rebuild(&scratch.parts.sequences);
    let mut workspace = scratch.estimator_workspace.take().unwrap_or_default();
    // Reuse the estimator's inner scratch across frames instead of
    // allocating a fresh `CompressedBlockScratch` (count tables + Vecs)
    // every block-split. Lazily created on the first split.
    let inner_scratch = scratch
        .estimator_inner
        .take()
        .map(|b| *b)
        .unwrap_or_default();
    let mut estimator = SplitEstimator {
        parts: &scratch.parts,
        prefix_sums: &scratch.prefix_sums,
        block_entry: ProbeEntryState {
            last_huff_table: state.last_huff_table.clone(),
            ll_previous: state.fse_tables.ll_previous.clone(),
            ml_previous: state.fse_tables.ml_previous.clone(),
            of_previous: state.fse_tables.of_previous.clone(),
            offset_hist: state.offset_hist,
        },
        scratch_state: CompressState {
            matcher: EntropyOnlyMatcher,
            last_huff_table: state.last_huff_table.clone(),
            huff_table_spare: None,
            fse_tables: clone_fse_tables(&state.fse_tables),
            block_scratch: inner_scratch,
            offset_hist: state.offset_hist,
            strategy_tag: state.strategy_tag,
            huf_optimal_search: state.huf_optimal_search,
            literal_compression_disabled: state.literal_compression_disabled,
        },
        workspace,
    };
    estimator.derive_block_splits(0, scratch.parts.sequences.len(), &mut scratch.partitions);
    scratch.partitions.push(scratch.parts.sequences.len());
    workspace = estimator.workspace;
    scratch.estimator_workspace = Some(workspace);
    // Stash the inner scratch back for the next frame (its buffers stay
    // allocated; the estimator clears them per use).
    scratch.estimator_inner = Some(Box::new(estimator.scratch_state.block_scratch));

    scratch.compressed.clear();
    let mut seq_start = 0usize;
    let mut lit_start = 0usize;
    let mut src_start = 0usize;
    for (partition_idx, &seq_end) in scratch.partitions.iter().enumerate() {
        let last_partition = partition_idx + 1 == scratch.partitions.len();
        let chunk_lit_len = scratch.prefix_sums.lit_range(seq_start, seq_end);
        let chunk_match_len = scratch.prefix_sums.ml_range(seq_start, seq_end);
        let lit_end = if last_partition {
            scratch.parts.literals.len()
        } else {
            lit_start + chunk_lit_len
        };
        let src_size = if last_partition {
            state.matcher.get_last_space().len() - src_start
        } else {
            chunk_lit_len + chunk_match_len
        };

        let mut emit_buffers = SingleSequenceEmitBuffers {
            output,
            compressed: &mut scratch.compressed,
            sequence_scratch: &mut scratch.estimator_sequences,
        };
        let emitted_raw = emit_single_sequence_block(
            state,
            last_block && last_partition,
            src_size,
            &scratch.parts.literals[lit_start..lit_end],
            &scratch.parts.sequences[seq_start..seq_end],
            &mut emit_buffers,
        );
        if emitted_raw {
            output.extend_from_slice(
                &state.matcher.get_last_space()[src_start..src_start + src_size],
            );
        }
        seq_start = seq_end;
        lit_start = lit_end;
        src_start += src_size;
    }
    state.block_scratch = scratch;
}

/// Literal-run length at or above which `append_literals` hands off to
/// `Vec::extend_from_slice` (libc `memcpy` → ERMS `rep movsb` on x86).
/// Below it the inline exact-copy loop wins (no libc call + ERMS startup
/// cost); at/above it the copy is bandwidth-bound and ERMS is faster.
/// Mirrors `simd_copy::BULK_MEMCPY_THRESHOLD` (the match-copy crossover).
const LITERAL_INLINE_COPY_MAX: usize = 2048;

/// Append `lits` to `dst` using inline copy ops, avoiding the libc
/// memcpy call overhead that `Vec::extend_from_slice` lowers to for
/// runtime-sized `ptr::copy_nonoverlapping`. Fast L1 emits literal runs
/// of 1-10 bytes typically — at thousands of sequences per block, the
/// per-emit libc call dominated the hot path (flamegraph:
/// `__memmove_avx_unaligned_erms` chain ≈ 16 % of L1 encode CPU).
///
/// - `len ≤ 32`: `simd_copy::copy_bytes_overshooting` with
///   `src.1 == dst.1 == lit_len` (no overshoot READ — the caller's slice
///   readable slack is unknown), which drops into the byte / overlapping-
///   u64 path, fully inlineable.
/// - `32 < len < 2048`: `simd_copy::copy_exact_medium` — the widest
///   available SIMD tier (AVX2 32B / SSE2 16B / NEON / scalar) doing an
///   EXACT copy (floor bulk + overlapping tier-width tail), the safe
///   upstream zstd-wildcopy analog: matches glibc's store width but drops the
///   libc call, and never overshoots reads (borrowed-input safe).
/// - `len ≥ 2048`: `extend_from_slice` — bandwidth-bound, ERMS wins.
#[inline]
fn append_literals(dst: &mut Vec<u8>, lits: &[u8]) {
    let lit_len = lits.len();
    if lit_len == 0 {
        return;
    }
    if lit_len >= LITERAL_INLINE_COPY_MAX {
        dst.extend_from_slice(lits);
        return;
    }
    // Production callers (`collect_block_parts`) pre-reserve `src_len` of
    // spare capacity, so the sum of all literal runs across a block fits
    // without grow. This is a SAFE fn, so enforce the precondition in
    // release too — a future caller skipping the pre-reserve would
    // otherwise get an OOB write past the `Vec`'s allocation. The branch
    // is cold on the production hot path.
    let cur_len = dst.len();
    if dst.capacity() - cur_len < lit_len {
        dst.reserve(lit_len);
    }
    let dst_ptr = unsafe { dst.as_mut_ptr().add(cur_len) };
    // SAFETY: `lits` is a valid slice (reading `lit_len` bytes from
    // `lits.as_ptr()` is in-bounds); the `dst.reserve(lit_len)` above
    // guarantees `dst_ptr` has `lit_len` bytes of spare capacity. Both
    // paths write EXACTLY `lit_len` bytes (no overshoot).
    unsafe {
        if lit_len <= 32 {
            crate::decoding::simd_copy::copy_bytes_overshooting(
                (lits.as_ptr(), lit_len),
                (dst_ptr, lit_len),
                lit_len,
            );
        } else {
            crate::decoding::simd_copy::copy_exact_medium(lits.as_ptr(), dst_ptr, lit_len);
        }
        dst.set_len(cur_len + lit_len);
    }
}

fn collect_block_parts<M: Matcher>(state: &mut CompressState<M>, parts: &mut EncodedBlockParts) {
    let src_len = state.matcher.get_last_space().len();
    parts.literals.clear();
    parts.sequences.clear();
    // `reserve_exact(N)` adds capacity above LENGTH, not above existing
    // capacity. Both `literals` and `sequences` were just `clear()`-ed (len
    // = 0), so subtracting `len()` ensures `cap >= N` after the call — the
    // older `cap - cap` form left the Vec under-provisioned whenever the
    // existing capacity was less than half of the target.
    if parts.literals.capacity() < src_len {
        parts.literals.reserve_exact(src_len - parts.literals.len());
    }
    let sequence_capacity = src_len / 8;
    if parts.sequences.capacity() < sequence_capacity {
        parts
            .sequences
            .reserve_exact(sequence_capacity - parts.sequences.len());
    }
    state.matcher.start_matching(|seq| match seq {
        Sequence::Literals { literals } => append_literals(&mut parts.literals, literals),
        Sequence::Triple {
            literals,
            offset,
            match_len,
        } => {
            let ll = literals.len() as u32;
            append_literals(&mut parts.literals, literals);
            parts.sequences.push(RawSequence {
                ll,
                ml: match_len as u32,
                offset: offset as u32,
            });
        }
    });
}

fn encode_block_parts_with_sequence_scratch<M: Matcher>(
    state: &mut CompressState<M>,
    literals_vec: &[u8],
    raw_sequences: &[RawSequence],
    output: &mut Vec<u8>,
    sequences: &mut Vec<crate::blocks::sequence_section::Sequence>,
) {
    encode_raw_sequences_into(
        raw_sequences,
        &mut state.offset_hist,
        sequences,
        matches!(
            state.strategy_tag,
            crate::encoding::strategy::StrategyTag::Fast
        ),
    );

    // literals section

    let mut writer = BitWriter::from(output);
    // Upstream zstd `compress_literals` (`zstd_compress_literals.c:153-160`):
    // `srcSize < ZSTD_minLiteralsToCompress(strategy, prevHuf->repeatMode)`
    // → `ZSTD_noCompressLiterals` (raw). The threshold is strategy-aware
    // (see `min_literals_to_compress`). With huf reuse available the
    // floor drops to 6 since there is no per-block huf-header overhead.
    let strategy = state.strategy_tag;
    let has_huf_table = state.last_huff_table.is_some();
    let min_lits = min_literals_to_compress(strategy, has_huf_table);
    // RLE pre-check: upstream zstd `compress_literals` reaches RLE only through
    // the `cLitSize == 1` branch (`zstd_compress_literals.c:192-201`)
    // after passing the `min_lits` gate and running a full HUF compress —
    // so upstream zstd emits raw for any all-identical section under `min_lits`
    // (e.g. 8..63 bytes at fast/dfast/greedy/lazy without HUF reuse).
    // RLE and raw share the same lhSize for a given `len`
    // (both use `uncompressed_literals_header_bytes`), so RLE = lhSize + 1
    // and raw = lhSize + len. That makes RLE equal to raw on `len == 1`
    // and smaller by exactly `len - 1` bytes for `len >= 2`, regardless of
    // the lhSize tier (1 / 2 / 3 / 5 bytes). Our pre-check fires for ANY
    // all-identical literal slice regardless of strategy/min_lits.
    // This produces strictly smaller output than upstream zstd on the small
    // all-identical edges while still matching upstream zstd on `>= min_lits`
    // inputs (where upstream zstd's compress+`cLitSize==1` path reaches the same
    // RLE block).
    // Note the order — RLE pre-check runs BEFORE `min_lits`;
    // `estimate_literals_section_bytes` mirrors this exactly so probe
    // costs match emit byte-for-byte.
    //
    // This is the LITERALS-section RLE inside a compressed block, reached only
    // when the block already carries sequences. A block whose ENTIRE content
    // is one repeated byte never gets here: `compress_block_encoded` emits a
    // block-level RLE block (Block_Type 1) for it first. The remaining
    // small-input framing economy (single-segment header, store-raw fallback
    // for non-shrinking blocks) lives in `append_frame_header` /
    // `compress_block_encoded`, not in this literals path.
    if state.literal_compression_disabled {
        // Upstream zstd `ZSTD_literalsCompressionIsDisabled` (auto mode:
        // `strategy == ZSTD_fast && targetLength > 0`, i.e. the negative levels):
        // emit RAW literals, skipping the Huffman pass entirely
        // (`ZSTD_noCompressLiterals`). Trades ratio for encode speed and matches
        // C's negative-band frames byte-for-byte (the literals section there is
        // Raw, not Compressed/RLE).
        raw_literals(literals_vec, &mut writer);
        state.clear_huff_table();
    } else if !literals_vec.is_empty() && all_bytes_identical(literals_vec) {
        rle_literals(literals_vec, &mut writer);
        state.clear_huff_table();
    } else if literals_vec.len() >= min_lits {
        match compress_literals(
            literals_vec,
            state.last_huff_table.as_ref(),
            &mut writer,
            strategy,
            state.huf_optimal_search,
        ) {
            HuffmanTableUpdate::New(table) => {
                state.replace_huff_table(table);
            }
            HuffmanTableUpdate::Reused => {}
            HuffmanTableUpdate::Cleared => {
                state.clear_huff_table();
            }
        }
    } else {
        raw_literals(literals_vec, &mut writer);
        state.clear_huff_table();
    }

    // sequences section

    if sequences.is_empty() {
        writer.write_bits(0u8, 8);
    } else {
        encode_seqnum(sequences.len(), &mut writer);

        // Single-pass histogram of ll/ml/of codes across all sequences.
        // Previously did three separate `sequences.iter().map(...)`
        // passes; folded into one loop here saves the per-element
        // closure overhead (profiling: `Map::fold` +
        // `call_mut` accounted for ~5% of total bench CPU).
        let mut ll_counts = [0usize; 256];
        let mut ml_counts = [0usize; 256];
        let mut of_counts = [0usize; 256];
        // Track the highest code per stream while histogramming so the table
        // selector skips the full-256 reverse scan for `max_symbol` (the small
        // sequence-code alphabets leave ~200 high slots permanently zero).
        let mut ll_max = 0usize;
        let mut ml_max = 0usize;
        let mut of_max = 0usize;
        for seq in sequences.iter() {
            let ll_code = encode_literal_length(seq.ll).0 as usize;
            let ml_code = encode_match_len(seq.ml).0 as usize;
            let of_code = encode_offset(seq.of).0 as usize;
            ll_counts[ll_code] += 1;
            ml_counts[ml_code] += 1;
            of_counts[of_code] += 1;
            ll_max = ll_max.max(ll_code);
            ml_max = ml_max.max(ml_code);
            of_max = of_max.max(of_code);
        }
        let total = sequences.len();

        // Stream codes of the LAST sequence: upstream zstd codes the final symbol
        // of each stream via the FSE init-state and drops one occurrence of it
        // from the emitted table's histogram (see `build_seq_ctable`). `Some`
        // here because these modes are written to the frame.
        let (last_ll, last_ml, last_of) = sequences.last().map_or((0, 0, 0), |seq| {
            (
                encode_literal_length(seq.ll).0 as usize,
                encode_match_len(seq.ml).0 as usize,
                encode_offset(seq.of).0 as usize,
            )
        });

        let ll_mode = choose_table_from_counts(
            state.fse_tables.ll_previous.as_ref(),
            state.fse_tables.ll_default_ref(),
            &mut ll_counts,
            total,
            ll_max,
            9,
            state.strategy_tag,
            Some(last_ll),
        );
        let ml_mode = choose_table_from_counts(
            state.fse_tables.ml_previous.as_ref(),
            state.fse_tables.ml_default_ref(),
            &mut ml_counts,
            total,
            ml_max,
            9,
            state.strategy_tag,
            Some(last_ml),
        );
        let of_mode = choose_table_from_counts(
            state.fse_tables.of_previous.as_ref(),
            state.fse_tables.of_default_ref(),
            &mut of_counts,
            total,
            of_max,
            8,
            state.strategy_tag,
            Some(last_of),
        );

        writer.write_bits(encode_fse_table_modes(&ll_mode, &ml_mode, &of_mode), 8);

        encode_table(&ll_mode, &mut writer);
        encode_table(&of_mode, &mut writer);
        encode_table(&ml_mode, &mut writer);

        encode_sequences(
            sequences,
            &mut writer,
            &ll_mode,
            &ml_mode,
            &of_mode,
            &state.fse_tables,
        );

        let ll_last = into_last_used_table(ll_mode);
        let ml_last = into_last_used_table(ml_mode);
        let of_last = into_last_used_table(of_mode);
        remember_last_used_tables(&mut state.fse_tables, ll_last, ml_last, of_last);
    }
    writer.flush();
}

/// Workspace shared across estimator probes so per-probe cost computation never
/// allocates. Counts are zeroed at the top of every probe.
struct EstimatorWorkspace {
    lit_counts: Box<[usize; 256]>,
    ll_counts: Box<[usize; 256]>,
    ml_counts: Box<[usize; 256]>,
    of_counts: Box<[usize; 256]>,
    sequences: Vec<crate::blocks::sequence_section::Sequence>,
}

impl Default for EstimatorWorkspace {
    fn default() -> Self {
        Self {
            lit_counts: Box::new([0; 256]),
            ll_counts: Box::new([0; 256]),
            ml_counts: Box::new([0; 256]),
            of_counts: Box::new([0; 256]),
            sequences: Vec::new(),
        }
    }
}

/// Dry-run analog of [`encode_block_parts_with_sequence_scratch`]: mirrors the
/// real encoder's `compress_literals` and `choose_table` decisions byte-for-byte
/// (same `last_huff_table` lookup, same FSE mode selection, same
/// `remember_last_used_tables` mutation), and computes the would-be output size
/// in bytes via existing cost primitives instead of running the per-sequence
/// FSE bit-level write. Splitter probes use this path to get the same byte
/// count `encode_block_parts` would produce while saving the dominant
/// `encode_sequences` write cost on every probe.
fn estimate_block_parts_size<M: Matcher>(
    state: &mut CompressState<M>,
    literals_vec: &[u8],
    raw_sequences: &[RawSequence],
    workspace: &mut EstimatorWorkspace,
) -> usize {
    encode_raw_sequences_into(
        raw_sequences,
        &mut state.offset_hist,
        &mut workspace.sequences,
        matches!(
            state.strategy_tag,
            crate::encoding::strategy::StrategyTag::Fast
        ),
    );

    let lit_bytes = estimate_literals_section_bytes(
        literals_vec,
        &mut state.last_huff_table,
        &mut workspace.lit_counts,
        state.strategy_tag,
        state.huf_optimal_search,
        state.literal_compression_disabled,
    );

    let seq_bytes = if workspace.sequences.is_empty() {
        1
    } else {
        estimate_sequences_section_bytes(
            &workspace.sequences,
            &mut state.fse_tables,
            &mut workspace.ll_counts,
            &mut workspace.ml_counts,
            &mut workspace.of_counts,
            state.strategy_tag,
        )
    };

    lit_bytes + seq_bytes
}

fn estimate_literals_section_bytes(
    literals: &[u8],
    last_huff: &mut Option<huff0_encoder::HuffmanTable>,
    counts: &mut [usize; 256],
    strategy: crate::encoding::strategy::StrategyTag,
    huf_search: bool,
    lit_disabled: bool,
) -> usize {
    // Mirror `encode_block_parts_with_sequence_scratch` literal-mode branches
    // **in the same order**. The disabled gate (negative levels: raw literals,
    // no Huffman) is checked FIRST exactly as the emitter does.
    if lit_disabled {
        *last_huff = None;
        return uncompressed_literals_header_bytes(literals.len()) + literals.len();
    }
    // The emitter pre-checks `all_identical`
    // (any non-empty section) BEFORE the `min_lits` gate — RLE and raw
    // share `uncompressed_literals_header_bytes(len)` (1/2/3/5 bytes by
    // length tier), so on all-identical inputs RLE = lhSize + 1 equals
    // raw = lhSize + len at `len == 1` and is smaller by `len - 1` for
    // `len >= 2`. RLE is never worse than raw, so it is selected
    // regardless of strategy. Estimator must use the same ordering and
    // predicate so probe costs match emit byte-for-byte.
    if !literals.is_empty() && all_bytes_identical(literals) {
        *last_huff = None;
        return uncompressed_literals_header_bytes(literals.len()) + 1;
    }
    let min_lits = min_literals_to_compress(strategy, last_huff.is_some());
    if literals.len() < min_lits {
        *last_huff = None;
        return uncompressed_literals_header_bytes(literals.len()) + literals.len();
    }

    // Upstream zstd preferRepeat fast-path: skip the histogram +
    // `build_from_counts` cost. Mirrors upstream zstd's
    // `huf_compress.c:1360-1364` policy — when the prior table
    // is valid for the input, REUSE unconditionally regardless
    // of whether a freshly-built table would compress better.
    // This is a deliberate CPU-avoidance bias on fast-band tiny
    // sections; see `decide_huff_reuse_prefer_repeat_forces_reuse_for_fast_band`
    // test which seeds a fixture where size-comparison would
    // pick new and asserts the override still picks reuse.
    // Mirrors `compress_literals` so both code paths agree
    // byte-for-byte. The prev-table validation
    // (`estimate_compressed_size` returns Some) gates the
    // short-circuit so we still fall through to rebuild when the
    // prior table can't encode the current literals.
    if prefer_repeat_eligible(strategy, literals.len())
        && let Some(prev) = last_huff.as_ref()
        && let Some(reuse_payload) = estimate_huff_payload_bytes_checked(prev, literals)
    {
        let compressed_header = compressed_literals_header_bytes(literals.len());
        let total = compressed_header + reuse_payload; // no tree_desc on reuse
        let raw_section_bytes = uncompressed_literals_header_bytes(literals.len()) + literals.len();
        let huf_section_size = total - compressed_header;
        if use_raw_literal_fallback(huf_section_size, literals.len(), strategy) {
            *last_huff = None;
            return raw_section_bytes;
        }
        return total;
    }

    let (max_sym, largest_count) = crate::histogram::count_bytes(literals, counts);
    // Mirror `compress_literals`' upstream zstd pre-build incompressibility gate
    // byte-for-byte (flat histogram → raw section, no tree build) so
    // splitter probe costs match what the emitter writes.
    if largest_count <= (literals.len() >> 7) + 4 {
        *last_huff = None;
        return uncompressed_literals_header_bytes(literals.len()) + literals.len();
    }
    let new_table =
        huff0_encoder::HuffmanTable::build_from_counts_gated(&counts[..=max_sym], huf_search);

    let Some(new_desc) = new_table.writeable_table_description_size() else {
        *last_huff = None;
        return uncompressed_literals_header_bytes(literals.len()) + literals.len();
    };
    // For lit_size ≥ 256, upstream zstd `compress_literals` calls `encoder.encode4x`
    // which splits the data in 4 streams with a 6-byte jumptable and per-stream
    // byte-aligned padding. Bare `estimate_compressed_size_from_counts` would
    // model a single stream and undercount by ~6–10 bytes per section, biasing
    // splitter probes. We reuse `estimate_compressed_size` on each quarter so
    // the cost matches the actual wire format.
    let new_payload = estimate_huff_payload_bytes(&new_table, literals, counts);

    // Mirror `compress_literals` reuse-vs-new decision **byte-for-byte**.
    // The real encoder compares single-stream `estimate_compressed_size` for
    // both new and old tables (see `compress_literals` below); the actual
    // wire output is the 4-stream `encode4x` layout once the table is chosen.
    // Using the 4-stream `estimate_huff_payload_bytes_checked` here would
    // disagree with the encoder and bias the splitter to pick a different
    // table than the encoder ultimately emits.
    let use_new = decide_huff_reuse_like_encoder(
        &new_table,
        last_huff.as_ref(),
        new_desc,
        literals,
        strategy,
    );
    let reuse_payload = if !use_new {
        // Safe to recompute with 4-stream model now that the table is chosen:
        // the chosen-table path always returns the actual wire cost.
        last_huff
            .as_ref()
            .and_then(|t| estimate_huff_payload_bytes_checked(t, literals))
    } else {
        None
    };

    let payload: usize = if use_new {
        new_payload
    } else {
        reuse_payload.unwrap_or(literals.len())
    };
    let tree_desc = if use_new { new_desc } else { 0 };
    let compressed_header = compressed_literals_header_bytes(literals.len());
    let total = compressed_header + tree_desc + payload;

    // Upstream zstd `compress_literals` raw-fallback gate
    // (`zstd_compress_literals.c:187-188`):
    //   `cLitSize >= srcSize - minGain`
    // where `cLitSize` is the encoded literals payload + tree description
    // (output of `HUF_compress*`, excluding the surrounding lhSize bytes)
    // and `srcSize` is the literal-payload length. In our terms:
    //   - upstream zstd `cLitSize` ≡ `total - compressed_header` (tree_desc + payload)
    //   - upstream zstd `srcSize`  ≡ `literals.len()`
    // Using the on-wire `total >= raw_section_bytes - mg` form (which
    // includes the compressed header on the LHS and the raw header on
    // the RHS) skews the threshold by `compressed_header - raw_header`
    // bytes and rejects compressed sections that upstream zstd would keep,
    // losing ratio. Mirror upstream zstd's payload-vs-srcSize form here.
    let raw_section_bytes = uncompressed_literals_header_bytes(literals.len()) + literals.len();
    let huf_section_size = total - compressed_header; // tree_desc + payload, no lhSize
    if use_raw_literal_fallback(huf_section_size, literals.len(), strategy) {
        *last_huff = None;
        return raw_section_bytes;
    }

    if use_new {
        *last_huff = Some(new_table);
    }
    total
}

fn estimate_sequences_section_bytes(
    sequences: &[crate::blocks::sequence_section::Sequence],
    fse_tables: &mut FseTables,
    ll_counts: &mut [usize; 256],
    ml_counts: &mut [usize; 256],
    of_counts: &mut [usize; 256],
    strategy: crate::encoding::strategy::StrategyTag,
) -> usize {
    ll_counts.fill(0);
    ml_counts.fill(0);
    of_counts.fill(0);
    let mut extra_bits: usize = 0;
    for seq in sequences {
        let (ll, _, ll_bits) = encode_literal_length(seq.ll);
        let (ml, _, ml_bits) = encode_match_len(seq.ml);
        let (of, _, _) = encode_offset(seq.of);
        ll_counts[ll as usize] += 1;
        ml_counts[ml as usize] += 1;
        of_counts[of as usize] += 1;
        // Upstream zstd: OF code's value equals its additional-bits width.
        extra_bits += ll_bits + ml_bits + of as usize;
    }

    // Same `choose_table` calls as the real encoder — counts the iterator
    // internally, identical decision path.
    let ll_mode = choose_table(
        fse_tables.ll_previous.as_ref(),
        fse_tables.ll_default_ref(),
        sequences.iter().map(|seq| encode_literal_length(seq.ll).0),
        9,
        strategy,
    );
    let ml_mode = choose_table(
        fse_tables.ml_previous.as_ref(),
        fse_tables.ml_default_ref(),
        sequences.iter().map(|seq| encode_match_len(seq.ml).0),
        9,
        strategy,
    );
    let of_mode = choose_table(
        fse_tables.of_previous.as_ref(),
        fse_tables.of_default_ref(),
        sequences.iter().map(|seq| encode_offset(seq.of).0),
        8,
        strategy,
    );

    let ll_bits_chosen =
        fse_section_bits_for_mode(&ll_mode, ll_counts, fse_tables.ll_default_ref());
    let ml_bits_chosen =
        fse_section_bits_for_mode(&ml_mode, ml_counts, fse_tables.ml_default_ref());
    let of_bits_chosen =
        fse_section_bits_for_mode(&of_mode, of_counts, fse_tables.of_default_ref());

    let ll_table_desc_bytes = mode_table_description_bytes(&ll_mode);
    let ml_table_desc_bytes = mode_table_description_bytes(&ml_mode);
    let of_table_desc_bytes = mode_table_description_bytes(&of_mode);

    // nbSeq varint header (upstream zstd RFC 8878 §3.1.1.3.2.1): 1–3 bytes.
    let nb_seq_header = match sequences.len() {
        0..=127 => 1,
        128..=0x7FFF => 2,
        _ => 3,
    };
    let mode_byte = 1;

    let bit_content = ll_bits_chosen + ml_bits_chosen + of_bits_chosen + extra_bits;
    // `encode_sequences` tail: if already byte-aligned, writes one extra byte
    // (`write_bits(1u32, 8)`); else writes `8 - bit_content % 8` padding bits.
    let padding_bits = if bit_content.is_multiple_of(8) {
        8
    } else {
        8 - bit_content % 8
    };
    let stream_bytes = (bit_content + padding_bits) / 8;

    // Mirror state mutation done by `encode_block_parts_with_sequence_scratch`.
    let ll_last = into_last_used_table(ll_mode);
    let ml_last = into_last_used_table(ml_mode);
    let of_last = into_last_used_table(of_mode);
    remember_last_used_tables(fse_tables, ll_last, ml_last, of_last);

    nb_seq_header
        + mode_byte
        + ll_table_desc_bytes
        + of_table_desc_bytes
        + ml_table_desc_bytes
        + stream_bytes
}

/// Bit cost of a sequence section under `mode`, matching what
/// `encode_sequences` would emit: FSE state transitions + final state flush.
fn fse_section_bits_for_mode(
    mode: &FseTableMode<'_>,
    counts: &[usize; 256],
    default: &FSETable,
) -> usize {
    let max_symbol = counts.iter().rposition(|&c| c > 0).unwrap_or_default();
    match mode {
        FseTableMode::Predefined(t) => {
            cross_entropy_cost(counts, max_symbol, t).unwrap_or(0) + t.acc_log() as usize
        }
        FseTableMode::Encoded(t) => {
            // New table built from these very counts — `fse_bit_cost` is
            // strictly more accurate than the `entropy_cost` proxy here.
            fse_bit_cost(counts, max_symbol, t).unwrap_or_else(|| {
                let total: usize = counts[..=max_symbol].iter().sum();
                entropy_cost(counts, max_symbol, total)
            }) + t.acc_log() as usize
        }
        FseTableMode::RepeatLast(prev) => {
            // `PreviousFseTable::Rle(_).as_table()` returns `None`. The real
            // encoder in that case writes no FSE state transitions and no
            // final-state flush — `encode_sequences` short-circuits on a
            // `None` table mapping — so the section costs 0 bits, matching
            // the bare `Rle(_)` arm below. Falling back to `default` here
            // would over-count by the default table's acc_log plus its
            // per-code cross-entropy and bias splitter probes.
            match prev.as_table(default) {
                Some(table) => {
                    fse_bit_cost(counts, max_symbol, table).unwrap_or(0) + table.acc_log() as usize
                }
                None => 0,
            }
        }
        FseTableMode::Rle(_) => 0,
    }
}

/// Byte size of the table description `encode_table` writes for each FSE mode.
fn mode_table_description_bytes(mode: &FseTableMode<'_>) -> usize {
    match mode {
        FseTableMode::Predefined(_) | FseTableMode::RepeatLast(_) => 0,
        FseTableMode::Encoded(table) => table.table_header_bits() / 8,
        FseTableMode::Rle(_) => 1,
    }
}

/// Shared reuse-vs-new Huffman table decision used by both the real encoder
/// (`compress_literals`) and the splitter cost estimator
/// (`estimate_literals_section_bytes`). Returns `true` when a fresh table
/// should be emitted, `false` when the prior table can be reused.
///
/// Decision logic is byte-for-byte the upstream zstd's: the old-table cost is the
/// single-stream `estimate_compressed_size` (returns `None` when the prior
/// table lacks codes for a symbol present in the current literals — in which
/// case we must emit a new table). The new-table cost is its description
/// size plus the single-stream payload estimate. A small-input guard
/// (`new_desc + 12 >= literals.len()`) keeps the reuse path for tiny blocks
/// where the description alone would exceed the literals.
/// Upstream zstd `HUF_flags_preferRepeat` gate (`zstd_compress_literals.c:165`):
/// fast-band strategies (`strategy < ZSTD_lazy` → Fast / Dfast /
/// Greedy in our enum) with short literal sections (≤ 1024 bytes)
/// prefer reusing the previous tree over rebuilding it. Inside
/// upstream zstd's HUF_compress (`huf_compress.c:1360-1364, 1396-1400`),
/// the flag short-circuits the rebuild path when the prior table
/// is valid; we mirror it at our caller layer so the wasted
/// `HuffmanTable::build_from_data` work is also skipped on the
/// fast-band reuse path. Note this is an UNCONDITIONAL reuse
/// override — upstream zstd intentionally picks reuse even when a fresh
/// table would compress better, trading a small ratio loss on
/// tiny sections for the CPU saved on the tree build. The
/// `decide_huff_reuse_like_encoder` helper then implements a
/// MIXED policy: the preferRepeat override fires first for the
/// fast band; outside that band, the existing size-comparison
/// heuristic decides reuse vs rebuild based on estimated bytes.
#[inline]
fn prefer_repeat_eligible(
    strategy: crate::encoding::strategy::StrategyTag,
    literals_len: usize,
) -> bool {
    use crate::encoding::strategy::StrategyTag;
    matches!(
        strategy,
        StrategyTag::Fast | StrategyTag::Dfast | StrategyTag::Greedy
    ) && literals_len <= 1024
}

fn decide_huff_reuse_like_encoder(
    new_table: &huff0_encoder::HuffmanTable,
    last_table: Option<&huff0_encoder::HuffmanTable>,
    new_desc: usize,
    literals: &[u8],
    strategy: crate::encoding::strategy::StrategyTag,
) -> bool {
    let Some(prev) = last_table else {
        return true;
    };
    let Some(old_estimate) = prev.estimate_compressed_size(literals) else {
        return true;
    };
    // Late-stage `HUF_flags_preferRepeat` mirror — kept here for
    // any caller that bypasses the early fast-path in
    // `compress_literals` / `estimate_literals_section_bytes`.
    // The early fast-paths short-circuit BEFORE `build_from_data`
    // / `build_from_counts` to skip wasted tree-build work; this
    // late gate covers the (currently unreachable) shape where the
    // new table is built first and the decision still wants to
    // reuse.
    if prefer_repeat_eligible(strategy, literals.len()) {
        return false;
    }
    let new_estimate = new_table
        .estimate_compressed_size(literals)
        .unwrap_or(literals.len());
    !(old_estimate <= new_desc + new_estimate || new_desc + 12 >= literals.len())
}

/// Mirrors `compress_literals` choice: lit_size < 256 → single huff0 stream
/// (`encode`), else → 4-stream layout (`encode4x`) with a 6-byte jumptable and
/// per-stream byte-aligned padding. Returns the exact wire-format byte cost of
/// the Huffman-encoded payload, excluding the literals section header and the
/// Huffman tree description.
fn estimate_huff_payload_bytes(
    table: &huff0_encoder::HuffmanTable,
    literals: &[u8],
    counts: &[usize; 256],
) -> usize {
    if literals.len() < 256 {
        table.estimate_compressed_size_from_counts(counts)
    } else {
        let split_size = literals.len().div_ceil(4);
        let s1 = &literals[..split_size];
        let s2 = &literals[split_size..split_size * 2];
        let s3 = &literals[split_size * 2..split_size * 3];
        let s4 = &literals[split_size * 3..];
        let mut total = 6; // 3 × u16 jumptable entries
        for stream in [s1, s2, s3, s4] {
            total += table
                .estimate_compressed_size(stream)
                .unwrap_or(stream.len());
        }
        total
    }
}

/// `estimate_huff_payload_bytes` variant that returns `None` when the table
/// can't encode some symbol in `literals` (Huffman codes with `num_bits == 0`).
/// Required to mirror `compress_literals`'s reuse-failure branch where the
/// real encoder bails to the new-table path.
fn estimate_huff_payload_bytes_checked(
    table: &huff0_encoder::HuffmanTable,
    literals: &[u8],
) -> Option<usize> {
    if literals.len() < 256 {
        table.estimate_compressed_size(literals)
    } else {
        let split_size = literals.len().div_ceil(4);
        let s1 = &literals[..split_size];
        let s2 = &literals[split_size..split_size * 2];
        let s3 = &literals[split_size * 2..split_size * 3];
        let s4 = &literals[split_size * 3..];
        let mut total = 6;
        for stream in [s1, s2, s3, s4] {
            total += table.estimate_compressed_size(stream)?;
        }
        Some(total)
    }
}

/// Upstream zstd RFC 8878 §3.1.1.3.1.2 raw/RLE literals header size (bytes).
fn uncompressed_literals_header_bytes(lit_size: usize) -> usize {
    match lit_size {
        0..=31 => 1,
        32..=4095 => 2,
        _ => 3,
    }
}

/// Upstream zstd RFC 8878 §3.1.1.3.1.1 compressed literals section header size (bytes,
/// excluding the Huffman tree description itself).
fn compressed_literals_header_bytes(lit_size: usize) -> usize {
    match lit_size {
        0..1024 => 3,
        1024..16384 => 4,
        _ => 5,
    }
}

struct SingleSequenceEmitBuffers<'a> {
    output: &'a mut Vec<u8>,
    compressed: &'a mut Vec<u8>,
    sequence_scratch: &'a mut Vec<crate::blocks::sequence_section::Sequence>,
}

fn emit_single_sequence_block<M: Matcher>(
    state: &mut CompressState<M>,
    last_block: bool,
    source_len: usize,
    literals: &[u8],
    sequences: &[RawSequence],
    buffers: &mut SingleSequenceEmitBuffers<'_>,
) -> bool {
    let saved_offset_hist = state.offset_hist;
    let saved_huff_table = state.last_huff_table.clone();
    let saved_ll_previous = state.fse_tables.ll_previous.clone();
    let saved_ml_previous = state.fse_tables.ml_previous.clone();
    let saved_of_previous = state.fse_tables.of_previous.clone();
    buffers.compressed.clear();
    encode_block_parts_with_sequence_scratch(
        state,
        literals,
        sequences,
        buffers.compressed,
        buffers.sequence_scratch,
    );
    let min_gain = (source_len >> 8) + 2;
    if buffers.compressed.len() >= source_len.saturating_sub(min_gain) {
        state.offset_hist = saved_offset_hist;
        state.last_huff_table = saved_huff_table;
        state.fse_tables.ll_previous = saved_ll_previous;
        state.fse_tables.ml_previous = saved_ml_previous;
        state.fse_tables.of_previous = saved_of_previous;
        let header = BlockHeader {
            last_block,
            block_type: BlockType::Raw,
            block_size: source_len as u32,
        };
        header.serialize(buffers.output);
        true
    } else {
        let header = BlockHeader {
            last_block,
            block_type: BlockType::Compressed,
            block_size: buffers.compressed.len() as u32,
        };
        header.serialize(buffers.output);
        buffers.output.extend_from_slice(buffers.compressed);
        false
    }
}

fn encode_raw_sequences_into(
    raw_sequences: &[RawSequence],
    offset_hist: &mut [u32; 3],
    out: &mut Vec<crate::blocks::sequence_section::Sequence>,
    fast_repcode: bool,
) {
    out.clear();
    // `reserve_exact` argument is the increment over LENGTH, not capacity —
    // see `SequencePrefixSums::rebuild` for the full rationale.
    if out.capacity() < raw_sequences.len() {
        out.reserve_exact(raw_sequences.len() - out.len());
    }
    // The strategy branch is hoisted out of the per-sequence loop so the
    // offBase-policy choice is paid once per block, not per sequence. Upstream
    // zstd's fast matcher emits only offBase 1 (rep[0] when litLength > 0,
    // rep[1] when litLength == 0 via the secondary-position check) or an explicit
    // offset — it never emits offBase 2/3. greedy+ search all three repeat
    // offsets, which is what the full `encode_offset_with_history` mirrors.
    if fast_repcode {
        out.extend(
            raw_sequences
                .iter()
                .map(|seq| crate::blocks::sequence_section::Sequence {
                    ll: seq.ll,
                    ml: seq.ml,
                    of: encode_offset_with_history_fast(seq.offset, seq.ll, offset_hist),
                }),
        );
    } else {
        out.extend(
            raw_sequences
                .iter()
                .map(|seq| crate::blocks::sequence_section::Sequence {
                    ll: seq.ll,
                    ml: seq.ml,
                    of: encode_offset_with_history(seq.offset, seq.ll, offset_hist),
                }),
        );
    }
}

fn clone_fse_tables(fse_tables: &FseTables) -> FseTables {
    // The `*_default` fields are cfg-typed via the
    // [`crate::fse::fse_encoder::FseDefaultTable`] alias —
    // `&'static FSETable` on atomic / `critical-section` targets
    // (Copy, zero-cost clone via field-access) and
    // `Box<FSETable>` on the cache-less no-atomic path (needs
    // `Clone::clone` for a deep copy). Method resolution of
    // `.clone()` on `&'static FSETable` resolves via auto-deref to
    // `FSETable::clone` (returns owned `FSETable`) which is the
    // WRONG return type for the atomic arm — the cfg-split below
    // picks the correct expression explicitly per target/feature.
    //
    // The block-split estimator path that calls this helper does
    // not run on the per-frame hot path (it fires only when block
    // pre-splitting decides to estimate sub-block costs, levels
    // 11+), so the no-atomic deep-clone cost is amortised in the
    // broader estimator overhead.
    FseTables {
        #[cfg(target_has_atomic = "ptr")]
        ll_default: fse_tables.ll_default,
        #[cfg(not(target_has_atomic = "ptr"))]
        ll_default: fse_tables.ll_default.clone(),
        ll_previous: fse_tables.ll_previous.clone(),
        #[cfg(target_has_atomic = "ptr")]
        ml_default: fse_tables.ml_default,
        #[cfg(not(target_has_atomic = "ptr"))]
        ml_default: fse_tables.ml_default.clone(),
        ml_previous: fse_tables.ml_previous.clone(),
        #[cfg(target_has_atomic = "ptr")]
        of_default: fse_tables.of_default,
        #[cfg(not(target_has_atomic = "ptr"))]
        of_default: fse_tables.of_default.clone(),
        of_previous: fse_tables.of_previous.clone(),
    }
}

/// Snapshot of the Huffman/FSE/repeat-offset state the real encoder would
/// have at a given partition boundary. Cloning is the only way to thread
/// state through recursive bisect probes (each branch needs its own copy),
/// but the snapshot is small relative to the full encode cost the dry-run
/// estimator replaces.
#[derive(Clone)]
struct ProbeEntryState {
    last_huff_table: Option<huff0_encoder::HuffmanTable>,
    ll_previous: Option<PreviousFseTable>,
    ml_previous: Option<PreviousFseTable>,
    of_previous: Option<PreviousFseTable>,
    offset_hist: [u32; 3],
}

struct SplitEstimator<'a> {
    parts: &'a EncodedBlockParts,
    prefix_sums: &'a SequencePrefixSums,
    block_entry: ProbeEntryState,
    scratch_state: CompressState<EntropyOnlyMatcher>,
    workspace: EstimatorWorkspace,
}

impl SplitEstimator<'_> {
    /// Run a single estimator probe seeded from `entry`. Returns the would-be
    /// emitted byte count for this partition, a `raw_fallback` flag (true
    /// when the estimate said this range will be emitted as a raw block in
    /// the real encoder — the cost is then capped at `source_len + 3`), and
    /// the post-probe state to feed into the sibling partition. When the
    /// partition would raw-fallback, the real encoder restores the entry
    /// state, so we return `entry` unchanged.
    fn estimate_subblock_size(
        &mut self,
        start_idx: usize,
        end_idx: usize,
        entry: &ProbeEntryState,
    ) -> (usize, bool, ProbeEntryState) {
        let lit_start = self.prefix_sums.lit[start_idx];
        let lit_len = self.prefix_sums.lit_range(start_idx, end_idx);
        let match_len = self.prefix_sums.ml_range(start_idx, end_idx);
        let lit_end = if end_idx == self.parts.sequences.len() {
            self.parts.literals.len()
        } else {
            lit_start + lit_len
        };
        self.scratch_state.last_huff_table = entry.last_huff_table.clone();
        self.scratch_state.fse_tables.ll_previous = entry.ll_previous.clone();
        self.scratch_state.fse_tables.ml_previous = entry.ml_previous.clone();
        self.scratch_state.fse_tables.of_previous = entry.of_previous.clone();
        self.scratch_state.offset_hist = entry.offset_hist;
        let emitted_payload = estimate_block_parts_size(
            &mut self.scratch_state,
            &self.parts.literals[lit_start..lit_end],
            &self.parts.sequences[start_idx..end_idx],
            &mut self.workspace,
        );
        let source_len = (lit_end - lit_start) + match_len;
        let min_gain = (source_len >> 8) + 2;
        let raw_fallback = emitted_payload >= source_len.saturating_sub(min_gain);
        let cost = if raw_fallback {
            source_len
        } else {
            emitted_payload
        } + 3;
        // Real emit on raw fallback restores the entry state — see
        // `emit_single_sequence_block`'s saved-state restore branch.
        let post = if raw_fallback {
            entry.clone()
        } else {
            ProbeEntryState {
                last_huff_table: self.scratch_state.last_huff_table.clone(),
                ll_previous: self.scratch_state.fse_tables.ll_previous.clone(),
                ml_previous: self.scratch_state.fse_tables.ml_previous.clone(),
                of_previous: self.scratch_state.fse_tables.of_previous.clone(),
                offset_hist: self.scratch_state.offset_hist,
            }
        };
        (cost, raw_fallback, post)
    }

    fn derive_block_splits(
        &mut self,
        start_idx: usize,
        end_idx: usize,
        partitions: &mut Vec<usize>,
    ) {
        if end_idx - start_idx < MIN_SEQUENCES_BLOCK_SPLITTING
            || partitions.len() >= MAX_NB_BLOCK_SPLITS
        {
            return;
        }
        let entry = self.block_entry.clone();
        let (full, full_raw_fallback, _) = self.estimate_subblock_size(start_idx, end_idx, &entry);
        // G3 — whole-block bail-out before partition split. Upstream zstd
        // `ZSTD_compressSubBlock_multi` (`zstd_compress_superblock.c:530-532`)
        // bails when `estBlockSize > srcSize` (strict). Our trigger is
        // the `raw_fallback` flag from `estimate_subblock_size`, which
        // fires on the **stricter** `emitted_payload >= source_len -
        // min_gain` condition (where `min_gain = (source_len >> 8) + 2`,
        // ≈0.4% margin — see the `min_gain` computation inside
        // `estimate_subblock_size` above). So we bail in a narrow band
        // `[source_len - min_gain, source_len + 3]` where upstream zstd would
        // still recurse and *might* find a compressible split.
        //
        // Why this is safe ratio-wise:
        // - The bail-out routes to `compress_block_with_post_split`'s
        //   single-partition path → `emit_single_sequence_block`,
        //   which applies the SAME `min_gain` expansion fallback (its
        //   `buffers.compressed.len() >= source_len - min_gain` check
        //   right before deciding raw-fallback). So for the
        //   single-partition path specifically, any block we bail on
        //   here would also raw-fallback there by the same threshold —
        //   no wire-output drift from this bail-out vs the "let the
        //   real emit decide" alternative.
        // - Returning here does skip the split case, so this is NOT a
        //   proof that a recursive split could never do better: in
        //   principle, both sub-blocks could compress strictly (no
        //   raw-fallback in either half) and beat the whole-block
        //   outcome. For such a missed split-win to matter, both
        //   sub-blocks would need to compress strictly AND
        //   `cost(first) + cost(second) < source_len + 3`. The wider
        //   upstream zstd band gives at most `min_gain` bytes of theoretical
        //   recoverable ratio per block.
        // - Empirically validated: `compare_ffi --list` REPORT lines
        //   show **zero rust_bytes delta** vs main on every
        //   (scenario, level) cell across the full bench matrix.
        //
        // Returning with `partitions` left empty lets the outer loop
        // emit the block as a single partition, avoiding the bisect's
        // recursive `estimate_subblock_size` walks. Cheap: the `full`
        // probe ran whether or not bisect proceeds, so zero estimator
        // work added on the bail-out path; significant work saved on
        // long-input incompressible-ish blocks at high levels (where
        // optimal parser produces > MIN_SEQUENCES_BLOCK_SPLITTING
        // sequences).
        if full_raw_fallback {
            return;
        }
        self.derive_block_splits_with_full(start_idx, end_idx, full, entry, partitions);
    }

    /// Returns the post-emit state at `end_idx` produced by whichever
    /// partitioning the recursion settles on (single emit OR multiple
    /// nested splits). Callers thread this into the sibling probe so the
    /// right-hand recursion sees the actual upstream zstd-parity state the real
    /// emit would land in, not just the "left as one big partition" state.
    fn derive_block_splits_with_full(
        &mut self,
        start_idx: usize,
        end_idx: usize,
        full: usize,
        entry: ProbeEntryState,
        partitions: &mut Vec<usize>,
    ) -> ProbeEntryState {
        if end_idx - start_idx < MIN_SEQUENCES_BLOCK_SPLITTING
            || partitions.len() >= MAX_NB_BLOCK_SPLITS
        {
            // Leaf: this range will be emitted as a single partition, so the
            // exit state is the post-state of that single-partition probe.
            let (_cost, _raw_fallback, post) =
                self.estimate_subblock_size(start_idx, end_idx, &entry);
            return post;
        }
        let mid_idx = (start_idx + end_idx) / 2;
        let (first, _, first_post) = self.estimate_subblock_size(start_idx, mid_idx, &entry);
        // Upstream zstd parity: score the right half from the left's post-state,
        // not from the parent's block-entry state. Without this propagation
        // `second` is evaluated as a fresh-block start, biasing the
        // `first + second < full` decision toward overly optimistic splits.
        let (second, _, _) = self.estimate_subblock_size(mid_idx, end_idx, &first_post);
        if first + second < full {
            // If the left side gets further split, the true state at
            // `mid_idx` is the left subtree's exit state, not `first_post`.
            // Thread the returned state into the right recursion so the
            // right subtree probes against actual upstream zstd-parity state.
            let left_post =
                self.derive_block_splits_with_full(start_idx, mid_idx, first, entry, partitions);
            if partitions.len() >= MAX_NB_BLOCK_SPLITS {
                return left_post;
            }
            partitions.push(mid_idx);
            return self
                .derive_block_splits_with_full(mid_idx, end_idx, second, left_post, partitions);
        }
        // No split here — this range will be emitted as one partition.
        let (_cost, _raw_fallback, post) = self.estimate_subblock_size(start_idx, end_idx, &entry);
        post
    }
}

#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
enum FseTableMode<'a> {
    Predefined(&'a FSETable),
    Encoded(FSETable),
    Rle(u8),
    RepeatLast(&'a PreviousFseTable),
}

impl FseTableMode<'_> {
    pub(crate) fn as_table<'a>(&'a self, default: &'a FSETable) -> Option<&'a FSETable> {
        match self {
            Self::Predefined(t) => Some(t),
            Self::RepeatLast(previous) => previous.as_table(default),
            Self::Encoded(t) => Some(t),
            Self::Rle(_) => None,
        }
    }
}

fn entropy_cost(counts: &[usize; 256], max_symbol: usize, total: usize) -> usize {
    let mut cost = 0usize;
    for &count in counts.iter().take(max_symbol + 1) {
        if count == 0 {
            continue;
        }
        let mut norm = 256 * count / total;
        if norm == 0 {
            norm = 1;
        }
        cost += count * INVERSE_PROBABILITY_LOG_256[norm];
    }
    cost >> 8
}

fn cross_entropy_cost(counts: &[usize; 256], max_symbol: usize, table: &FSETable) -> Option<usize> {
    let acc_log = table.acc_log();
    if acc_log > 8 {
        return None;
    }
    let shift = 8 - acc_log;
    let mut cost = 0usize;
    for (symbol, &count) in counts.iter().enumerate().take(max_symbol + 1) {
        if count == 0 {
            continue;
        }
        let prob = table.symbol_probability(symbol as u8);
        if prob == 0 {
            return None;
        }
        let norm = if prob == -1 { 1 } else { prob as usize };
        let norm_256 = norm << shift;
        if norm_256 == 0 || norm_256 >= 256 {
            return None;
        }
        cost += count * INVERSE_PROBABILITY_LOG_256[norm_256];
    }
    Some(cost >> 8)
}

fn fse_bit_cost(counts: &[usize; 256], max_symbol: usize, table: &FSETable) -> Option<usize> {
    let table_log = table.acc_log() as usize;
    let table_size = 1usize << table_log;
    let mut cost = 0usize;
    for (symbol, &count) in counts.iter().enumerate().take(max_symbol + 1) {
        if count == 0 {
            continue;
        }
        let prob = table.symbol_probability(symbol as u8);
        if prob == 0 {
            return None;
        }
        let delta_nb_bits = match prob {
            -1 | 1 => (table_log << 16).saturating_sub(table_size),
            prob if prob > 1 => {
                let prob = prob as usize;
                let max_bits_out = table_log - (prob - 1).ilog2() as usize;
                let min_state_plus = prob << max_bits_out;
                (max_bits_out << 16).saturating_sub(min_state_plus)
            }
            _ => return None,
        };
        let min_nb_bits = delta_nb_bits >> 16;
        let threshold = (min_nb_bits + 1) << 16;
        if delta_nb_bits + table_size > threshold {
            return None;
        }
        let delta_from_threshold = threshold - (delta_nb_bits + table_size);
        let normalized_delta = (delta_from_threshold << 8) >> table_log;
        let bit_cost = (min_nb_bits + 1) * 256 - normalized_delta;
        let bad_cost = (table_log + 1) << 8;
        if bit_cost >= bad_cost {
            return None;
        }
        cost += count * bit_cost;
    }
    Some(cost >> 8)
}

fn choose_table<'a>(
    previous: Option<&'a PreviousFseTable>,
    default_table: &'a FSETable,
    data: impl Iterator<Item = u8>,
    max_log: u8,
    strategy: crate::encoding::strategy::StrategyTag,
) -> FseTableMode<'a> {
    // Collect symbol distribution, tracking the highest code so the selector
    // skips the full-256 reverse scan (see `choose_table_from_counts`).
    let mut counts = [0usize; 256];
    let mut total = 0usize;
    let mut max_symbol = 0usize;
    for symbol in data {
        let symbol = symbol as usize;
        counts[symbol] += 1;
        total += 1;
        max_symbol = max_symbol.max(symbol);
    }
    choose_table_from_counts(
        previous,
        default_table,
        &mut counts,
        total,
        max_symbol,
        max_log,
        strategy,
        // Estimator-only path (no emitted table): price the unadjusted histogram,
        // matching upstream's `ZSTD_NCountCost`.
        None,
    )
}

/// Same decision logic as [`choose_table`] but takes pre-computed
/// symbol counts and total directly. Hot-path callers in
/// `compress_literals_and_sequences` use this overload to avoid
/// re-iterating the sequence vec three times (one pass per
/// ll/ml/of stream); the iterator form is kept for the cost
/// estimator's call sites where the data is already in iterator
/// form.
// The eight inputs are the cohesive FSE-table-selection set, each carrying its
// own perf / correctness rationale below (the `&mut` histogram for the no-copy
// emit build, the caller-tracked `max_symbol` / `last_code` that avoid a
// per-stream rescan). Bundling them into a struct would only relocate that
// documented rationale away from the signature without removing any input.
#[expect(
    clippy::too_many_arguments,
    reason = "cohesive FSE-selection inputs, each justified inline"
)]
fn choose_table_from_counts<'a>(
    previous: Option<&'a PreviousFseTable>,
    default_table: &'a FSETable,
    // `&mut` only so the emitted-table build can borrow the histogram, drop the
    // last symbol in place for its normalize, and restore it (see
    // `build_seq_ctable`) — no per-table copy. Every selection-time read
    // re-borrows it immutably; the value is unchanged on return.
    counts: &mut [usize; 256],
    total: usize,
    // The highest symbol code with a non-zero count, tracked by the caller as it
    // builds `counts`. The sequence-code alphabets are tiny (LL <= 35, ML <= 52,
    // OF <= 31) while `counts` is a fixed 256-wide array, so deriving this here
    // via `counts.iter().rposition(..)` would scan ~200 always-zero high slots
    // per stream per block — the dominant cost on small frames (profiled ~35% of
    // a 1 KiB dict-frame encode). The caller already visits every code once, so
    // it carries the running max for free. Equal to the old `rposition` result
    // (every counted code increments its slot, so the max code IS the highest
    // non-zero index), keeping the table selection byte-identical.
    max_symbol: usize,
    max_log: u8,
    strategy: crate::encoding::strategy::StrategyTag,
    // The stream code of the LAST sequence, when this call emits a table that
    // will be written to the frame (`Some`). Upstream zstd drops one occurrence
    // of that code from the histogram before normalizing the emitted custom
    // table (it is coded via the FSE init-state, not a transition). `None` for
    // the cost-estimator call sites, which — like upstream's `ZSTD_NCountCost` —
    // price the unadjusted histogram.
    last_code: Option<usize>,
) -> FseTableMode<'a> {
    if total == 0 {
        return FseTableMode::Predefined(default_table);
    }

    // Distinctness over the live alphabet only (`..=max_symbol`); slots above
    // `max_symbol` are zero by construction, so bounding the scan there matches
    // the full-array result without touching the always-zero tail.
    let distinct_symbols = counts[..=max_symbol]
        .iter()
        .filter(|&&count| count > 0)
        .take(2)
        .count();
    if distinct_symbols == 1 {
        let symbol = max_symbol as u8;
        if let Some(PreviousFseTable::Rle(prev_symbol)) = previous
            && *prev_symbol == symbol
        {
            return FseTableMode::RepeatLast(previous.unwrap());
        }
        if total <= 2 && default_table.symbol_probability(symbol) != 0 {
            return FseTableMode::Predefined(default_table);
        }
        return FseTableMode::Rle(symbol);
    }

    // Fast-band preferRepeat (upstream zstd `ZSTD_selectEncodingType`,
    // `zstd_compress_sequences.c:179-204`): for fast/dfast/greedy with a
    // valid previous table and `< 1000` sequences, reuse it without building
    // a new one. Trades a negligible ratio loss for skipping the per-block
    // FSE table build + header descriptor — the dominant per-sub-block cost
    // when these cheap-match strategies split a block. The validity probe
    // (`fse_bit_cost` is finite) guarantees the previous table covers every
    // symbol in this block, so the reuse can never produce an invalid stream.
    if matches!(
        strategy,
        crate::encoding::strategy::StrategyTag::Fast
            | crate::encoding::strategy::StrategyTag::Dfast
            | crate::encoding::strategy::StrategyTag::Greedy
    ) && total < 1000
        && let Some(prev) = previous
        && let Some(table) = prev.as_table(default_table)
        && fse_bit_cost(counts, max_symbol, table).is_some()
    {
        return FseTableMode::RepeatLast(prev);
    }

    let use_low_prob_count = total >= 2048;

    // Mirror upstream zstd `ZSTD_selectEncodingType()`: compare default
    // cross-entropy, repeat-table FSE bit cost, and the custom compressed
    // table (header + entropy-bound payload). The custom table's header is
    // priced from its normalized counts via `fse_header_bits_for_counts`
    // WITHOUT building the (often-discarded) state tables — the build only
    // runs in the `Choice::New` arm below, when the custom table actually
    // wins. The estimate equals the built table's `table_header_bits()`
    // exactly, so the selection is byte-identical.
    let new_total_cost = (distinct_symbols > 1).then(|| {
        // Plain `+`: both are bit-cost estimates bounded by the block size
        // (<= MAX_BLOCK_SIZE * 8 bits), far under the integer's range.
        fse_header_bits_for_counts(&counts[..=max_symbol], max_log, use_low_prob_count)
            + entropy_cost(counts, max_symbol, total)
    });

    let predefined_cost = cross_entropy_cost(counts, max_symbol, default_table);

    let previous_cost = previous.and_then(|previous| {
        previous
            .as_table(default_table)
            .and_then(|table| fse_bit_cost(counts, max_symbol, table))
    });

    enum Choice {
        Previous,
        Predefined,
        New,
    }

    let mut best: Option<(usize, Choice)> = None;

    if let Some(cost) = previous_cost {
        best = Some((cost, Choice::Previous));
    }

    if let Some(cost) = predefined_cost {
        match best {
            Some((best_cost, _)) if best_cost <= cost => {}
            _ => best = Some((cost, Choice::Predefined)),
        }
    }

    if let Some(cost) = new_total_cost {
        match best {
            Some((best_cost, _)) if best_cost <= cost => {}
            _ => best = Some((cost, Choice::New)),
        }
    }

    match best.map(|(_, choice)| choice) {
        Some(Choice::Previous) => previous
            .map(FseTableMode::RepeatLast)
            .unwrap_or(FseTableMode::Predefined(default_table)),
        Some(Choice::Predefined) => FseTableMode::Predefined(default_table),
        // The custom table won the cost comparison — build it now (the only
        // place the state tables are constructed). `distinct_symbols > 1`
        // held when `new_total_cost` was computed, so the histogram has the
        // two-sample minimum `build_table_from_symbol_counts` requires.
        Some(Choice::New) => FseTableMode::Encoded(match last_code {
            Some(lc) => build_seq_ctable(&mut counts[..=max_symbol], max_log, lc),
            None => {
                build_table_from_symbol_counts(&counts[..=max_symbol], max_log, use_low_prob_count)
            }
        }),
        None => {
            let fallback_counts = [counts[0], 0];
            let fallback = if max_symbol == 0 {
                // `build_table_from_symbol_counts` needs at least two entries, so
                // single-symbol streams use a phantom zero-count second slot here.
                build_table_from_symbol_counts(&fallback_counts, max_log, use_low_prob_count)
            } else {
                build_table_from_symbol_counts(&counts[..=max_symbol], max_log, use_low_prob_count)
            };
            FseTableMode::Encoded(fallback)
        }
    }
}

fn encode_table(mode: &FseTableMode<'_>, writer: &mut BitWriter<&mut Vec<u8>>) {
    match mode {
        FseTableMode::Predefined(_) => {}
        FseTableMode::RepeatLast(_) => {}
        FseTableMode::Encoded(table) => table.write_table(writer),
        FseTableMode::Rle(symbol) => writer.write_bits(*symbol, 8),
    }
}

fn encode_fse_table_modes(
    ll_mode: &FseTableMode<'_>,
    ml_mode: &FseTableMode<'_>,
    of_mode: &FseTableMode<'_>,
) -> u8 {
    fn mode_to_bits(mode: &FseTableMode<'_>) -> u8 {
        match mode {
            FseTableMode::Predefined(_) => 0,
            FseTableMode::Rle(_) => 1,
            FseTableMode::Encoded(_) => 2,
            FseTableMode::RepeatLast(_) => 3,
        }
    }
    mode_to_bits(ll_mode) << 6 | mode_to_bits(of_mode) << 4 | mode_to_bits(ml_mode) << 2
}

fn remember_last_used_tables(
    fse_tables: &mut FseTables,
    ll_last: Option<PreviousFseTable>,
    ml_last: Option<PreviousFseTable>,
    of_last: Option<PreviousFseTable>,
) {
    remember_last_used_table(&mut fse_tables.ll_previous, ll_last);
    remember_last_used_table(&mut fse_tables.ml_previous, ml_last);
    remember_last_used_table(&mut fse_tables.of_previous, of_last);
}

fn remember_last_used_table(slot: &mut Option<PreviousFseTable>, next: Option<PreviousFseTable>) {
    if let Some(next) = next {
        *slot = Some(next);
    }
}

fn into_last_used_table(mode: FseTableMode<'_>) -> Option<PreviousFseTable> {
    match mode {
        FseTableMode::Encoded(table) => Some(PreviousFseTable::Custom(SharedFseTable::new(table))),
        FseTableMode::Predefined(_) => Some(PreviousFseTable::Default),
        FseTableMode::Rle(symbol) => Some(PreviousFseTable::Rle(symbol)),
        FseTableMode::RepeatLast(_) => None,
    }
}

fn encode_sequences(
    sequences: &[crate::blocks::sequence_section::Sequence],
    writer: &mut BitWriter<&mut Vec<u8>>,
    ll_mode: &FseTableMode<'_>,
    ml_mode: &FseTableMode<'_>,
    of_mode: &FseTableMode<'_>,
    defaults: &FseTables,
) {
    fn mode_table<'a>(mode: &'a FseTableMode<'_>, default: &'a FSETable) -> Option<&'a FSETable> {
        mode.as_table(default)
    }

    let sequence = sequences[sequences.len() - 1];
    let (ll_code, ll_add_bits, ll_num_bits) = encode_literal_length(sequence.ll);
    let (of_code, of_add_bits, of_num_bits) = encode_offset(sequence.of);
    let (ml_code, ml_add_bits, ml_num_bits) = encode_match_len(sequence.ml);
    let ll_table = mode_table(ll_mode, defaults.ll_default_ref());
    let ml_table = mode_table(ml_mode, defaults.ml_default_ref());
    let of_table = mode_table(of_mode, defaults.of_default_ref());
    let mut ll_state = ll_table.map(|table| table.start_state(ll_code));
    let mut ml_state = ml_table.map(|table| table.start_state(ml_code));
    let mut of_state = of_table.map(|table| table.start_state(of_code));

    writer.write_bits(ll_add_bits, ll_num_bits);
    writer.write_bits(ml_add_bits, ml_num_bits);
    writer.write_bits(of_add_bits, of_num_bits);

    // Upstream zstd-faithful sequence loop: write state diffs + extras via
    // unchecked fast-path adds with explicit `flush_bulk` calls at
    // safe burst boundaries. Per-sequence bit budget:
    //   state diffs: of (<=8) + ml (<=9) + ll (<=9) = 26 bits → one
    //                burst between flushes.
    //   extras:      ll (<=16) + ml (<=16) + of (<=24) = 56 bits →
    //                one burst between flushes.
    //
    // Total per sequence: 82 bits ⇒ at least 2 flushes (one per burst).
    // Mirrors upstream zstd `ZSTD_encodeSequences_body`
    // (`zstd_compress_sequences.c:303-360`) which uses BIT_addBitsFast
    // + BIT_flushBitsFast at the same burst boundaries.
    //
    // Pre-reserve output capacity for the worst-case sequence section
    // size (~10 bytes/sequence + 32 byte slack) so the per-flush
    // `extend_from_slice` never triggers a Vec realloc.
    if sequences.len() > 1 {
        writer.reserve_output(sequences.len() * 12 + 64);
        // Pre-loop flush: the safe `write_bits` calls above for the
        // final sequence's add_bits leave `bits_in_partial` in
        // 0..=63. Before the first unchecked-add burst we drain to
        // < 8 leftover so the per-burst budget math (state diffs ≤
        // 30 + leftover ≤ 8 = 38 < 64) holds invariantly.
        // SAFETY: `reserve_output` above guarantees capacity ≥
        // current_len + sequences.len() * 12 + 64 ≥ current_len + 8.
        unsafe {
            writer.flush_bulk();
        }
        for sequence in (0..=sequences.len() - 2).rev() {
            let sequence = sequences[sequence];
            let (ll_code, ll_add_bits, ll_num_bits) = encode_literal_length(sequence.ll);
            let (of_code, of_add_bits, of_num_bits) = encode_offset(sequence.of);
            let (ml_code, ml_add_bits, ml_num_bits) = encode_match_len(sequence.ml);

            // State diffs burst: max 30 bits (10+10+9 worst case for
            // acc_log ≤ 9 ll/ml + acc_log ≤ 8 of) + ≤ 7 leftover from
            // prior flush = ≤ 37 bits total — well under 64.
            //
            // SAFETY (for every `write_bits_64_no_check` below):
            // - the prior `flush_bulk` left `bits_in_partial ≤ 7`;
            // - each FSE state diff has `next.num_bits ≤ acc_log ≤ 10`;
            //   three diffs back-to-back add ≤ 30 bits → total ≤ 37,
            //   well below the 64-bit accumulator cap.
            // - `diff = state.index - next.baseline` cannot exceed
            //   `(1 << num_bits) - 1`, so `diff >> num_bits == 0`.
            // `reserve_output(sequences.len() * 12 + 64)` above
            // pre-allocated enough spare capacity to cover every
            // per-sequence flush in this loop (≤ 16 bytes per
            // sequence, plus the 32-byte slack on top of the 64-byte
            // header reserve).
            if let (Some(table), Some(state)) = (of_table, of_state) {
                let next = table.next_state(of_code, state.index);
                let diff = state.index - next.baseline;
                unsafe {
                    writer.write_bits_64_no_check(diff as u64, next.num_bits as usize);
                }
                of_state = Some(next);
            }
            if let (Some(table), Some(state)) = (ml_table, ml_state) {
                let next = table.next_state(ml_code, state.index);
                let diff = state.index - next.baseline;
                unsafe {
                    writer.write_bits_64_no_check(diff as u64, next.num_bits as usize);
                }
                ml_state = Some(next);
            }
            if let (Some(table), Some(state)) = (ll_table, ll_state) {
                let next = table.next_state(ll_code, state.index);
                let diff = state.index - next.baseline;
                unsafe {
                    writer.write_bits_64_no_check(diff as u64, next.num_bits as usize);
                }
                ll_state = Some(next);
            }
            unsafe {
                writer.flush_bulk();
            }

            // Extras burst: ll (≤16) + ml (≤16) + of (≤ window_log,
            // up to 30 for our max window_log). With ≤ 7 leftover from
            // the prior flush_bulk, total ll+ml+of+partial can exceed
            // 64 once of_num_bits > 25. Upstream zstd handles this via
            // `longOffsets` mode that splits high offsets across two
            // BIT_addBits calls; we instead drain the partial after ml
            // and write of into a fresh container. The branch matches
            // upstream zstd's `MEM_32bits()` flush-between-each-component
            // shape on the 32-bit build (which has the same 64-bit
            // container constraint).
            //
            // SAFETY: `encode_literal_length` / `encode_match_len`
            // bound `*_num_bits ≤ 16` and return a clean `*_add_bits`
            // (low `num_bits` bits only). `encode_offset` bounds
            // `of_num_bits ≤ ilog2(of)`, capped at the encoder's
            // `window_log` ≤ 30; the conditional flush_bulk above
            // drains the partial when of_num_bits crosses the 24-bit
            // threshold where the sum could exceed 64.
            unsafe {
                writer.write_bits_64_no_check(ll_add_bits as u64, ll_num_bits);
                writer.write_bits_64_no_check(ml_add_bits as u64, ml_num_bits);
            }
            if of_num_bits > 24 {
                unsafe {
                    writer.flush_bulk();
                }
            }
            unsafe {
                writer.write_bits_64_no_check(of_add_bits as u64, of_num_bits);
                writer.flush_bulk();
            }
        }
    }
    if let (Some(state), Some(table)) = (ml_state, ml_table) {
        writer.write_bits(state.index as u64, table.table_size.ilog2() as usize);
    }
    if let (Some(state), Some(table)) = (of_state, of_table) {
        writer.write_bits(state.index as u64, table.table_size.ilog2() as usize);
    }
    if let (Some(state), Some(table)) = (ll_state, ll_table) {
        writer.write_bits(state.index as u64, table.table_size.ilog2() as usize);
    }

    let bits_to_fill = writer.misaligned();
    if bits_to_fill == 0 {
        writer.write_bits(1u32, 8);
    } else {
        writer.write_bits(1u32, bits_to_fill);
    }
}

fn encode_seqnum(seqnum: usize, writer: &mut BitWriter<impl AsMut<Vec<u8>>>) {
    const UPPER_LIMIT: usize = 0xFFFF + 0x7F00;
    match seqnum {
        1..=127 => writer.write_bits(seqnum as u32, 8),
        128..=0x7FFF => {
            let upper = ((seqnum >> 8) | 0x80) as u8;
            let lower = seqnum as u8;
            writer.write_bits(upper, 8);
            writer.write_bits(lower, 8);
        }
        0x8000..=UPPER_LIMIT => {
            let encode = seqnum - 0x7F00;
            let upper = (encode >> 8) as u8;
            let lower = encode as u8;
            writer.write_bits(255u8, 8);
            writer.write_bits(upper, 8);
            writer.write_bits(lower, 8);
        }
        _ => unreachable!(),
    }
}

fn encode_literal_length(len: u32) -> (u8, u32, usize) {
    match len {
        0..=15 => (len as u8, 0, 0),
        16..=17 => (16, len - 16, 1),
        18..=19 => (17, len - 18, 1),
        20..=21 => (18, len - 20, 1),
        22..=23 => (19, len - 22, 1),
        24..=27 => (20, len - 24, 2),
        28..=31 => (21, len - 28, 2),
        32..=39 => (22, len - 32, 3),
        40..=47 => (23, len - 40, 3),
        48..=63 => (24, len - 48, 4),
        64..=127 => (25, len - 64, 6),
        128..=255 => (26, len - 128, 7),
        256..=511 => (27, len - 256, 8),
        512..=1023 => (28, len - 512, 9),
        1024..=2047 => (29, len - 1024, 10),
        2048..=4095 => (30, len - 2048, 11),
        4096..=8191 => (31, len - 4096, 12),
        8192..=16383 => (32, len - 8192, 13),
        16384..=32767 => (33, len - 16384, 14),
        32768..=65535 => (34, len - 32768, 15),
        65536..=131071 => (35, len - 65536, 16),
        131072.. => unreachable!(),
    }
}

fn encode_match_len(len: u32) -> (u8, u32, usize) {
    match len {
        0..=2 => unreachable!(),
        3..=34 => (len as u8 - 3, 0, 0),
        35..=36 => (32, len - 35, 1),
        37..=38 => (33, len - 37, 1),
        39..=40 => (34, len - 39, 1),
        41..=42 => (35, len - 41, 1),
        43..=46 => (36, len - 43, 2),
        47..=50 => (37, len - 47, 2),
        51..=58 => (38, len - 51, 3),
        59..=66 => (39, len - 59, 3),
        67..=82 => (40, len - 67, 4),
        83..=98 => (41, len - 83, 4),
        99..=130 => (42, len - 99, 5),
        131..=258 => (43, len - 131, 7),
        259..=514 => (44, len - 259, 8),
        515..=1026 => (45, len - 515, 9),
        1027..=2050 => (46, len - 1027, 10),
        2051..=4098 => (47, len - 2051, 11),
        4099..=8194 => (48, len - 4099, 12),
        8195..=16386 => (49, len - 8195, 13),
        16387..=32770 => (50, len - 16387, 14),
        32771..=65538 => (51, len - 32771, 15),
        65539..=131074 => (52, len - 65539, 16),
        131075.. => unreachable!(),
    }
}

/// Convert an actual byte offset into the encoded offset code, using repeat offset
/// history per RFC 8878 §3.1.2.5. Updates `offset_hist` in place.
///
/// Encoded offset codes: 1/2/3 = repeat offsets, N+3 = new absolute offset N.
pub(in crate::encoding) fn encode_offset_with_history(
    actual_offset: u32,
    lit_len: u32,
    offset_hist: &mut [u32; 3],
) -> u32 {
    let encoded = if lit_len > 0 {
        if actual_offset == offset_hist[0] {
            1
        } else if actual_offset == offset_hist[1] {
            2
        } else if actual_offset == offset_hist[2] {
            3
        } else {
            actual_offset + 3
        }
    } else {
        // When lit_len == 0, repeat offset mapping shifts per RFC 8878:
        // code 1 → rep[1], code 2 → rep[2], code 3 → rep[0]-1
        if actual_offset == offset_hist[1] {
            1
        } else if actual_offset == offset_hist[2] {
            2
        } else if actual_offset == offset_hist[0].wrapping_sub(1) && offset_hist[0] > 1 {
            3
        } else {
            actual_offset + 3
        }
    };

    // Update history (same rules as decoder)
    if lit_len > 0 {
        match encoded {
            1 => { /* rep[0] stays the same */ }
            2 => {
                offset_hist[1] = offset_hist[0];
                offset_hist[0] = actual_offset;
            }
            _ => {
                offset_hist[2] = offset_hist[1];
                offset_hist[1] = offset_hist[0];
                offset_hist[0] = actual_offset;
            }
        }
    } else {
        match encoded {
            1 => {
                offset_hist[1] = offset_hist[0];
                offset_hist[0] = actual_offset;
            }
            2 => {
                offset_hist[2] = offset_hist[1];
                offset_hist[1] = offset_hist[0];
                offset_hist[0] = actual_offset;
            }
            _ => {
                offset_hist[2] = offset_hist[1];
                offset_hist[1] = offset_hist[0];
                offset_hist[0] = actual_offset;
            }
        }
    }

    encoded
}

/// Fast-matcher offset→offBase conversion, mirroring upstream zstd's
/// `ZSTD_compressBlock_fast`: emit offBase 1 only for the immediate repeat
/// offset (`rep[0]` when `lit_len > 0`, `rep[1]` when `lit_len == 0` — the
/// litLength-0 rotation per RFC 8878 §3.1.2.5), and an explicit offset
/// otherwise. Unlike [`encode_offset_with_history`] it never probes `rep[1]`
/// (`lit_len > 0`) or `rep[2]`/`rep[0]-1` (`lit_len == 0`), so it does not
/// rewrite an explicit offset that happens to coincide with a deeper repeat
/// into offBase 2/3. That keeps the negative/fast band's sequence stream
/// byte-identical to the C reference (the deeper-repeat rewrite both costs a
/// per-sequence probe the fast matcher never pays and can shift the FSE symbol
/// histogram the wrong way). The repeat-offset history update follows directly
/// from the emitted code, identical to the full converter's rules.
pub(in crate::encoding) fn encode_offset_with_history_fast(
    actual_offset: u32,
    lit_len: u32,
    offset_hist: &mut [u32; 3],
) -> u32 {
    if lit_len > 0 {
        if actual_offset == offset_hist[0] {
            return 1; // rep[0] match: history unchanged
        }
    } else if actual_offset == offset_hist[1] {
        // litLength-0 offBase 1 decodes as rep[1]: promote it, demote rep[0].
        offset_hist[1] = offset_hist[0];
        offset_hist[0] = actual_offset;
        return 1;
    }
    // Explicit offset: rotate the full repeat-offset history.
    offset_hist[2] = offset_hist[1];
    offset_hist[1] = offset_hist[0];
    offset_hist[0] = actual_offset;
    actual_offset + 3
}

fn encode_offset(len: u32) -> (u8, u32, usize) {
    let log = len.ilog2();
    let lower = len & ((1 << log) - 1);
    (log as u8, lower, log as usize)
}

fn all_bytes_identical(literals: &[u8]) -> bool {
    literals
        .first()
        .is_some_and(|&first| literals.iter().all(|&byte| byte == first))
}

fn write_uncompressed_literals_header(
    section_type: u8,
    literals_len: usize,
    writer: &mut BitWriter<&mut Vec<u8>>,
) {
    writer.write_bits(section_type, 2);
    match literals_len {
        0..=31 => {
            writer.write_bits(0u8, 1);
            writer.write_bits(literals_len as u8, 5);
        }
        32..=4095 => {
            writer.write_bits(1u8, 2);
            writer.write_bits(literals_len as u16, 12);
        }
        _ => {
            writer.write_bits(3u8, 2);
            writer.write_bits(literals_len as u32, 20);
        }
    }
}

fn raw_literals(literals: &[u8], writer: &mut BitWriter<&mut Vec<u8>>) {
    write_uncompressed_literals_header(0, literals.len(), writer);
    writer.append_bytes(literals);
}

fn rle_literals(literals: &[u8], writer: &mut BitWriter<&mut Vec<u8>>) {
    debug_assert!(!literals.is_empty());
    debug_assert!(all_bytes_identical(literals));
    write_uncompressed_literals_header(1, literals.len(), writer);
    writer.append_bytes(&literals[..1]);
}

/// Reuse-only literals emit. Writes the full RFC 8878 §3.1.1.3.1.1
/// treeless literals section: type bits (`0b11`), 2-bit
/// size_format, the regenerated (uncompressed) literals length
/// field, the compressed length field placeholder (patched after
/// the huf payload is emitted), and the huf-encoded payload using
/// `last_table` (no tree description, since the decoder reuses the
/// previously-emitted one). Used by `compress_literals` when the
/// upstream zstd preferRepeat gate short-circuits the rebuild path.
/// Mirrors the post-decide reuse branch at the bottom of
/// `compress_literals` byte-for-byte (same size_format ladder, same
/// min_gain raw-fallback gate) so the wire output is identical to
/// the size-comparison reuse path when both would pick reuse.
fn emit_reuse_literals(
    literals: &[u8],
    last_table: &huff0_encoder::HuffmanTable,
    writer: &mut BitWriter<&mut Vec<u8>>,
    reset_idx: usize,
    strategy: crate::encoding::strategy::StrategyTag,
) -> HuffmanTableUpdate {
    writer.write_bits(3u8, 2); // treeless compressed literals type
    assert!(
        literals.len() <= 262_143,
        "literals exceed RFC 8878 18-bit size limit (262143)"
    );
    let (size_format, size_bits) = match literals.len() {
        0..256 => (0b00u8, 10),
        256..1024 => (0b01, 10),
        1024..16384 => (0b10, 14),
        _ => (0b11, 18),
    };
    writer.write_bits(size_format, 2);
    writer.write_bits(literals.len() as u32, size_bits);
    let size_index = writer.index();
    writer.write_bits(0u32, size_bits);
    let index_before = writer.index();
    let mut encoder = huff0_encoder::HuffmanEncoder::new(last_table, writer);
    if size_format == 0 {
        encoder.encode(literals, false);
    } else {
        encoder.encode4x(literals, false);
    }
    let encoded_len = (writer.index() - index_before) / 8;
    writer.change_bits(size_index, encoded_len as u64, size_bits);
    let total_len = (writer.index() - reset_idx) / 8;

    let compressed_header_len = compressed_literals_header_bytes(literals.len());
    let huf_section_size = total_len - compressed_header_len;
    if use_raw_literal_fallback(huf_section_size, literals.len(), strategy) {
        writer.reset_to(reset_idx);
        raw_literals(literals, writer);
        HuffmanTableUpdate::Cleared
    } else {
        HuffmanTableUpdate::Reused
    }
}

fn compress_literals(
    literals: &[u8],
    last_table: Option<&huff0_encoder::HuffmanTable>,
    writer: &mut BitWriter<&mut Vec<u8>>,
    strategy: crate::encoding::strategy::StrategyTag,
    huf_search: bool,
) -> HuffmanTableUpdate {
    let reset_idx = writer.index();

    // Upstream zstd preferRepeat fast-path: when Fast/Dfast/Greedy on
    // <=1024-byte literals AND the prior table can encode this
    // input (`estimate_compressed_size` returns Some), skip the
    // expensive `HuffmanTable::build_from_data` and route the
    // emit straight through the reuse path. Mirrors upstream zstd's
    // HUF_compress shape: `huf_compress.c:1360-1364` checks the
    // flag BEFORE the histogram + tree-build, so the rebuild cost
    // is avoided on fast-band tiny sections. Without this gate,
    // we paid `build_from_data` then short-circuited at the
    // decide-helper — wasted CPU on the hot fast-level path.
    if prefer_repeat_eligible(strategy, literals.len())
        && let Some(prev) = last_table
        && prev.estimate_compressed_size(literals).is_some()
    {
        return emit_reuse_literals(literals, prev, writer, reset_idx, strategy);
    }

    let mut counts = [0usize; 256];
    let (max_symbol, largest_count) = crate::histogram::count_bytes(literals, &mut counts);
    // Upstream zstd pre-build incompressibility gate (`huf_compress.c`,
    // `HUF_compress_internal`): a histogram this flat
    // (`largest <= (srcSize >> 7) + 4`) is heuristically not worth
    // compressing — bail to raw BEFORE the tree build and the full
    // `encode4x` pass. Without it, near-random literals paid histogram +
    // sort + tree + a full encode of the section only for the post-hoc
    // `use_raw_literal_fallback` below to throw it all away (~65% of
    // frame time on the random-payload dict scenarios). The single-symbol
    // case (`largest == srcSize`) never reaches here: the block emitter
    // routes all-identical sections to RLE first.
    if largest_count <= (literals.len() >> 7) + 4 {
        raw_literals(literals, writer);
        return HuffmanTableUpdate::Cleared;
    }

    let new_encoder_table =
        huff0_encoder::HuffmanTable::build_from_counts_gated(&counts[..=max_symbol], huf_search);

    let Some(new_table_description_size) = new_encoder_table.writeable_table_description_size()
    else {
        raw_literals(literals, writer);
        return HuffmanTableUpdate::Cleared;
    };
    // Shared with the splitter cost estimator
    // (`estimate_literals_section_bytes`) so both code paths agree on which
    // table they would pick for a given `(new_table, last_table, literals)`
    // input.
    let new_table = decide_huff_reuse_like_encoder(
        &new_encoder_table,
        last_table,
        new_table_description_size,
        literals,
        strategy,
    );
    let encoder_table = if new_table {
        &new_encoder_table
    } else {
        last_table.expect("reuse path implies prior table exists")
    };

    if new_table {
        writer.write_bits(2u8, 2); // compressed literals type
    } else {
        writer.write_bits(3u8, 2); // treeless compressed literals type
    }

    // RFC 8878 §3.1.1.3.1.1 Size_Format (spec limits):
    //   0b00: single stream, 10-bit (≤ 1023)  |  0b01: 4 streams, 10-bit (≤ 1023)
    //   0b10: 4 streams, 14-bit (≤ 16383)     |  0b11: 4 streams, 18-bit (≤ 262143)
    //
    // Runtime: hard guard — truncated 18-bit writes produce corrupt streams.
    // Note: format args omitted intentionally to avoid uncoverable dead code in coverage.
    assert!(
        literals.len() <= 262_143,
        "literals exceed RFC 8878 18-bit size limit (262143)"
    );
    let (size_format, size_bits) = match literals.len() {
        0..256 => (0b00u8, 10),
        256..1024 => (0b01, 10),
        1024..16384 => (0b10, 14),
        _ => (0b11, 18),
    };

    writer.write_bits(size_format, 2);
    writer.write_bits(literals.len() as u32, size_bits);
    let size_index = writer.index();
    writer.write_bits(0u32, size_bits);
    let index_before = writer.index();
    let mut encoder = huff0_encoder::HuffmanEncoder::new(encoder_table, writer);
    if size_format == 0 {
        encoder.encode(literals, new_table)
    } else {
        encoder.encode4x(literals, new_table)
    };
    let encoded_len = (writer.index() - index_before) / 8;
    writer.change_bits(size_index, encoded_len as u64, size_bits);
    let total_len = (writer.index() - reset_idx) / 8;

    // Upstream zstd `compress_literals` raw-fallback gate
    // (`zstd_compress_literals.c:187-188`):
    //   `cLitSize >= srcSize - minGain`
    // where upstream zstd's `cLitSize` is the encoded literals payload plus the
    // tree description (output of `HUF_compress*`, excluding the
    // surrounding `lhSize` literals header), and `srcSize` is the
    // literal-payload length. In our terms:
    //   - upstream zstd `cLitSize` ≡ `total_len - compressed_literals_header_bytes`
    //     (i.e. tree_desc + huf_payload, no lhSize)
    //   - upstream zstd `srcSize`  ≡ `literals.len()`
    // Comparing `total_len >= raw_section_bytes - minGain` (with the
    // compressed-section lhSize on the LHS and raw-section header on
    // the RHS) skews the threshold by `compressed_header - raw_header`
    // bytes and rejects compressed sections that upstream zstd would keep —
    // direct ratio loss. Mirror upstream zstd's payload-vs-srcSize form here.
    // `minGain` is strategy-aware (`min_gain` helper above; ~1.56% for
    // fast..btopt, ~0.78% for btultra, ~0.39% for btultra2). Saturating
    // subtraction covers tiny inputs where `literals.len() < minGain`.
    let compressed_header_len = compressed_literals_header_bytes(literals.len());
    let huf_section_size = total_len - compressed_header_len; // tree_desc + payload, no lhSize
    if use_raw_literal_fallback(huf_section_size, literals.len(), strategy) {
        writer.reset_to(reset_idx);
        raw_literals(literals, writer);
        HuffmanTableUpdate::Cleared
    } else if new_table {
        HuffmanTableUpdate::New(new_encoder_table)
    } else {
        HuffmanTableUpdate::Reused
    }
}
