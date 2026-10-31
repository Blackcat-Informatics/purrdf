#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use crate::{
    blocks::block::BlockType,
    common::MAX_BLOCK_SIZE,
    encoding::{
        CompressionLevel, Matcher,
        block_header::BlockHeader,
        blocks::{compress_block, compress_block_with_post_split},
        frame_compressor::CompressState,
        incompressible::{
            block_looks_incompressible, block_looks_incompressible_dict,
            block_looks_incompressible_strict, compression_level_allows_raw_fast_path,
        },
        match_generator::MatchGeneratorDriver,
    },
};
use alloc::vec::Vec;

/// Compresses a single block using the shared compressed-block pipeline.
///
/// Used by all compressed levels (Fastest, Default, Better, Best, and numeric levels). The actual
/// compression quality is determined by the matcher backend in `state`,
/// not by this function.
///
/// # Parameters
/// - `state`: [`CompressState`] so the compressor can refer to data before
///   the start of this block
/// - `last_block`: Whether or not this block is going to be the last block in the frame
///   (needed because this info is written into the block header)
/// - `uncompressed_data`: A block's worth of uncompressed data, taken from the
///   larger input
/// - `output`: As `uncompressed_data` is compressed, it's appended to `output`.
// Mirrors the per-block sidecar plumbing of its borrowed sibling
// (`compress_block_encoded_borrowed`): the lsm decompressed-size and
// optional XXH64 checksum out-params push the arg count past the lint's
// threshold. Bundling them into a struct would diverge from the established
// emit-fn shape for no readability gain.
#[allow(clippy::too_many_arguments)]
#[inline]
pub(crate) fn compress_block_encoded<M: Matcher>(
    state: &mut CompressState<M>,
    compression_level: CompressionLevel,
    last_block: bool,
    uncompressed_data: Vec<u8>,
    output: &mut Vec<u8>,
    // When a dictionary is primed, an "incompressible-looking" block can
    // still compress well by matching the dictionary content, so the
    // raw-fast-path (which SKIPS matching) must NOT fire — otherwise the
    // dictionary is never searched and the block is emitted raw. C always
    // searches and only falls back to raw post-hoc; we mirror that here.
    dict_active: bool,
    // Per-physical-block decompressed (regenerated) size sidecar, in
    // block-emit order — 1:1 with `FrameEmitInfo.blocks`, same cardinality
    // discipline as the XXH64 checksum sidecar. Captured under `lsm` alone
    // (not gated on `hash`, not opt-in) because `FrameEmitInfo` is always
    // built under `lsm` and every block needs its `decompressed_size`.
) -> BlockType {
    let block_size = uncompressed_data.len() as u32;
    // First check to see if run length encoding can be used for the entire block
    if uncompressed_data.iter().all(|x| uncompressed_data[0].eq(x)) {
        let rle_byte = uncompressed_data[0];

        state.matcher.commit_space(uncompressed_data);
        state.matcher.skip_matching_with_hint(Some(false));
        let header = BlockHeader {
            last_block,
            block_type: BlockType::RLE,
            block_size,
        };
        // Write the header, then the block
        header.serialize(output);
        output.push(rle_byte);
        BlockType::RLE
    } else if !(dict_active && state.matcher.block_samples_match_dict(&uncompressed_data))
        // Evaluation order matters: the dict-match guard is cheap (a constant
        // `true` for the conservative backends — HC / Row / BT — and a small
        // table probe for Fast), whereas `should_emit_raw_fast_path` runs a
        // WHOLE-BLOCK incompressibility scan on the dict path. When a primed
        // dictionary forces the raw-skip off anyway (guard is `false`), this
        // short-circuit skips that scan entirely instead of computing it and
        // throwing the result away. Boolean result is unchanged (AND commutes).
        && should_emit_raw_fast_path(
            compression_level,
            state.matcher.window_size(),
            &uncompressed_data,
            dict_active,
        )
    {
        state.matcher.commit_space(uncompressed_data);
        state.matcher.skip_matching_with_hint(Some(true));
        let header = BlockHeader {
            last_block,
            block_type: BlockType::Raw,
            block_size,
        };
        header.serialize(output);
        output.extend_from_slice(state.matcher.get_last_space());
        BlockType::Raw
    } else {
        // Compress as a standard compressed block
        let uncompressed_len = uncompressed_data.len();
        state.matcher.commit_space(uncompressed_data);
        if matches!(compression_level, CompressionLevel::Level(16..=22))
            && state.matcher.window_size() >= (1 << 17)
        {
            // This helper may emit multiple physical blocks (compressed or raw)
            // into `output`; the decompressed-size and (if requested) checksum
            // sidecars are pushed per physical block from inside the partition
            // loop so the cardinality matches the decoder's per-block count
            // exactly.

            compress_block_with_post_split(state, last_block, output);
            return BlockType::Compressed;
        }

        let _ = uncompressed_len;

        // Keep rollback snapshots for the oversize fallback path below:
        // `compress_block` can mutate entropy/history state before we know
        // whether the compressed payload fits `MAX_BLOCK_SIZE`.
        let saved_offset_hist = state.offset_hist;
        // Snapshot the Huffman table into the scratch's persistent rollback
        // slot: `clone_from` reuses the slot's buffers across blocks (a
        // fresh `.clone()` paid a malloc + free pair on both code containers
        // every block). FSE previous tables are `SharedFseTable` handles —
        // their clone is a refcount bump, no slot needed.
        let mut saved_huff_table = core::mem::take(&mut state.block_scratch.huff_rollback);
        saved_huff_table.clone_from(&state.last_huff_table);
        let saved_ll_previous = state.fse_tables.ll_previous.clone();
        let saved_ml_previous = state.fse_tables.ml_previous.clone();
        let saved_of_previous = state.fse_tables.of_previous.clone();
        // Compress directly into `output`: reserve the fixed 3-byte block
        // header, append the payload after it, then backfill the header in
        // place once its length is known — no temp `Vec`, no extend-copy.
        let hdr_off = output.len();
        output.extend_from_slice(&[0u8; 3]);
        let payload_off = output.len();
        compress_block(state, output);
        let payload_len = output.len() - payload_off;
        // Fall back to a raw block when the compressed payload is not
        // smaller than the source (`payload >= block_size`) or exceeds the
        // maximum block size. A compressed block that did not shrink is never
        // the right choice: it wastes bytes and, in a single-segment frame
        // (window == content size), can reference past the declared window
        // and fail to decode in a strict decoder. This mirrors the upstream
        // post-hoc raw fallback and applies to every block, dictionary-primed
        // or not — the pre-compression raw-fast-path only catches blocks that
        // already look incompressible, so small inputs that slip past it but
        // fail to shrink still need this post-hoc store-raw.
        if payload_len >= MAX_BLOCK_SIZE as usize || payload_len >= block_size as usize {
            // Roll back the payload + reserved header and the entropy state.
            output.truncate(hdr_off);
            state.offset_hist = saved_offset_hist;
            // Swap (not move) so the slot keeps owning a reusable table
            // allocation for the next block's snapshot.
            core::mem::swap(&mut state.last_huff_table, &mut saved_huff_table);
            state.fse_tables.ll_previous = saved_ll_previous;
            state.fse_tables.ml_previous = saved_ml_previous;
            state.fse_tables.of_previous = saved_of_previous;
            state.block_scratch.huff_rollback = saved_huff_table;
            let header = BlockHeader {
                last_block,
                block_type: BlockType::Raw,
                block_size,
            };
            // Write the header, then the block
            header.serialize(output);
            output.extend_from_slice(state.matcher.get_last_space());
            BlockType::Raw
        } else {
            // Return the snapshot to its slot so the next block's
            // `clone_from` reuses the allocation.
            state.block_scratch.huff_rollback = saved_huff_table;
            let header = BlockHeader {
                last_block,
                block_type: BlockType::Compressed,
                block_size: payload_len as u32,
            };
            // Backfill the reserved 3-byte header in place.
            output[hdr_off..hdr_off + 3].copy_from_slice(&header.to_le_bytes());
            BlockType::Compressed
        }
    }
}

/// Borrowed one-shot variant of [`compress_block_encoded`] for the Fast
/// (Simple) backend: the block bytes live at `[block_start, block_end)`
/// of the matcher's registered borrowed window (`set_borrowed_window`),
/// so there is no owned block `Vec` to `commit_space`. Instead the range
/// is staged via `set_borrowed_block`, which routes the subsequent
/// `start_matching` / `skip_matching_with_hint` to the borrowed scan.
///
/// Mirrors `compress_block_encoded`'s RLE / raw-fast-path / compressed
/// branch selection and shares the heavy `compress_block` machinery; the
/// only differences are how the block is acquired (borrowed slice, no
/// copy) and that raw/RLE bodies are emitted straight from `block`. The
/// `Level(16..=22)` post-split branch is unreachable here (the borrowed
/// path is gated to Fast levels), so it is omitted.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compress_block_encoded_borrowed(
    state: &mut CompressState<MatchGeneratorDriver>,
    compression_level: CompressionLevel,
    last_block: bool,
    block: &[u8],
    block_start: usize,
    block_end: usize,
    dict_active: bool,
    output: &mut Vec<u8>,
) -> BlockType {
    // The borrowed one-shot path emits ONE block per staged range (no
    // pre-split partition loop). `borrowed_supported()` is the single source
    // of truth for which backend + search configs have a borrowed scan
    // (Simple / Dfast / Row, and HashChain's lazy CHAIN parser + btlazy2); the
    // optimal BT search stays on the owned path. `borrowed_eligible` gates on
    // the same predicate, so this only ever fires on a wiring bug. Checked at
    // entry (not per-branch) so RLE / raw-fast / compressed paths all stage
    // their borrowed range under the same invariant.
    debug_assert!(
        state.matcher.borrowed_supported(),
        "borrowed one-shot path reached for an unsupported backend/search config",
    );
    let block_size = block.len() as u32;
    if !block.is_empty() && block.iter().all(|x| block[0].eq(x)) {
        let rle_byte = block[0];

        state.matcher.set_borrowed_block(block_start, block_end);
        state.matcher.skip_matching_with_hint(Some(false));
        let header = BlockHeader {
            last_block,
            block_type: BlockType::RLE,
            block_size,
        };
        header.serialize(output);
        output.push(rle_byte);
        BlockType::RLE
    } else if !(dict_active && state.matcher.block_samples_match_dict(block))
        // Same cheap-guard-first short-circuit as the owned path: skip the
        // whole-block dict-incompressibility scan when a primed dict already
        // forces the raw-skip off. Boolean result unchanged (AND commutes).
        && should_emit_raw_fast_path(
            compression_level,
            state.matcher.window_size(),
            block,
            dict_active,
        )
    {
        state.matcher.set_borrowed_block(block_start, block_end);
        state.matcher.skip_matching_with_hint(Some(true));
        let header = BlockHeader {
            last_block,
            block_type: BlockType::Raw,
            block_size,
        };
        header.serialize(output);
        output.extend_from_slice(block);
        BlockType::Raw
    } else {
        // Stage the borrowed range so `compress_block`'s internal
        // `start_matching` scans it in place (no `commit_space` copy).
        state.matcher.set_borrowed_block(block_start, block_end);
        // No post-split branch here: the optimal levels (16-22), the only
        // strategies that post-split, are NOT borrowed-eligible
        // (`borrowed_supported` keeps them owned because the borrowed
        // continuous-index scan yields ratio-worse candidates for their
        // cost-based DP). btlazy2 (L13-15) and every other borrowed backend
        // emit a single block per staged range, handled by the path below.

        let saved_offset_hist = state.offset_hist;
        // Persistent rollback slot — same allocation-reuse rationale as the
        // owned `compress_block_encoded` snapshot above.
        let mut saved_huff_table = core::mem::take(&mut state.block_scratch.huff_rollback);
        saved_huff_table.clone_from(&state.last_huff_table);
        let saved_ll_previous = state.fse_tables.ll_previous.clone();
        let saved_ml_previous = state.fse_tables.ml_previous.clone();
        let saved_of_previous = state.fse_tables.of_previous.clone();
        // Compress directly into `output`: reserve the fixed 3-byte block
        // header, append the payload after it, then backfill the header in
        // place once its length is known. Avoids the per-block temp `Vec`
        // plus the `output.extend(compressed)` copy (the dominant per-frame
        // memmove on this hot path).
        let hdr_off = output.len();
        output.extend_from_slice(&[0u8; 3]);
        let payload_off = output.len();
        compress_block(state, output);
        let payload_len = output.len() - payload_off;
        if payload_len >= MAX_BLOCK_SIZE as usize || payload_len >= block_size as usize {
            // Incompressible (compressed payload not smaller than the source,
            // or over the max block size): roll back the payload + reserved
            // header and the entropy state, then emit a stored Raw block. A
            // non-shrinking compressed block wastes bytes and can reference
            // past a single-segment frame's window (== content size); storing
            // raw matches the upstream post-hoc fallback.
            output.truncate(hdr_off);
            state.offset_hist = saved_offset_hist;
            // Swap (not move) so the slot keeps owning a reusable table
            // allocation for the next block's snapshot.
            core::mem::swap(&mut state.last_huff_table, &mut saved_huff_table);
            state.fse_tables.ll_previous = saved_ll_previous;
            state.fse_tables.ml_previous = saved_ml_previous;
            state.fse_tables.of_previous = saved_of_previous;
            state.block_scratch.huff_rollback = saved_huff_table;
            let header = BlockHeader {
                last_block,
                block_type: BlockType::Raw,
                block_size,
            };
            header.serialize(output);
            output.extend_from_slice(block);
            BlockType::Raw
        } else {
            // Return the snapshot to its slot so the next block's
            // `clone_from` reuses the allocation.
            state.block_scratch.huff_rollback = saved_huff_table;
            let header = BlockHeader {
                last_block,
                block_type: BlockType::Compressed,
                block_size: payload_len as u32,
            };
            output[hdr_off..hdr_off + 3].copy_from_slice(&header.to_le_bytes());
            BlockType::Compressed
        }
    }
}

#[inline]
fn should_emit_raw_fast_path(
    level: CompressionLevel,
    window_size: u64,
    block: &[u8],
    has_dict: bool,
) -> bool {
    if !compression_level_allows_raw_fast_path(level, window_size) {
        return false;
    }
    // With a dictionary attached, a high-entropy-looking block is NOT
    // necessarily incompressible: the dict can supply matches the block's own
    // content gives no hint of (e.g. structured records drawn from the trained
    // dict). So raise the bar to the STRICT head/mid/tail probe before skipping
    // the scan — the same conservative check `Best` uses. The plain no-dict
    // heuristic stays for the no-dict path, where it classifies incompressible
    // blocks very well. Without this, an over-cutoff dict frame whose first
    // block matches the dict gets emitted raw and ignores the dictionary
    // entirely.
    if has_dict {
        return block_looks_incompressible_dict(block);
    }
    if matches!(level, CompressionLevel::Best) {
        return block_looks_incompressible_strict(block);
    }
    block_looks_incompressible(block)
}
