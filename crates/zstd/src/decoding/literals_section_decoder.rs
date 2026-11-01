#![allow(
    clippy::cast_lossless,
    clippy::inline_always,
    clippy::manual_assert_eq,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! This module contains the decompress_literals function, used to take a
//! parsed literals header and a source and decompress it.

use super::super::blocks::literals_section::{LiteralsSection, LiteralsSectionType};
use super::scratch::HuffmanScratch;
use crate::bit_io::BitReaderReversed;
#[cfg(target_arch = "x86_64")]
use crate::cpu_kernel::Avx2Kernel;
#[cfg(target_arch = "x86_64")]
use crate::cpu_kernel::Bmi2Kernel;
#[cfg(target_arch = "x86_64")]
use crate::cpu_kernel::CpuKernelTag;

use crate::cpu_kernel::{CpuKernel, ScalarKernel, detect_cpu_kernel};
use crate::decoding::dictionary::Dictionary;
use crate::decoding::errors::DecompressLiteralsError;
use crate::huff0::HuffmanDecoder;
use alloc::vec::Vec;

/// Result of [`decode_literals_zerocopy`]. For Raw sections this is a
/// borrow straight into the input — no memcpy. For RLE / HUF
/// sections it's a borrow of the scratch `literals_buffer` where the
/// data was materialised.
pub(crate) struct LiteralsView<'a> {
    /// Decoded literal bytes available for the sequence executor.
    pub data: &'a [u8],
    /// Bytes consumed from the input literals section payload
    /// (Raw: regenerated_size; HUF: header + jump + 4 streams).
    pub bytes_used: u32,
}

/// Zero-copy variant of [`decode_literals`]. For Raw literal sections
/// returns a slice straight into `source` instead of copying bytes
/// into a Vec — eliminates one memcpy + one zero-touch wave per RAW
/// literal byte on the direct-decode path. RLE / HUF paths still go
/// through `target` because they have to produce new bytes (RLE: N
/// copies of one byte; HUF: indexed burst writes).
///
/// Upstream zstd parity: `dctx->litPtr` is set to either `src` (Raw) or
/// `dctx->litBuffer` (HUF); the seq executor reads from
/// `dctx->litPtr` uniformly.
pub(crate) fn decode_literals_zerocopy<'a>(
    section: &LiteralsSection,
    scratch: &mut HuffmanScratch,
    dict: Option<&Dictionary>,
    source: &'a [u8],
    target: &'a mut Vec<u8>,
) -> Result<LiteralsView<'a>, DecompressLiteralsError> {
    // Snapshot `target.len()` before any decode work — the returned
    // view must point ONLY at the newly-decoded bytes, not at any
    // pre-existing tail the caller forgot to `clear()`. The current
    // in-tree callers clear before this call, but anchoring the
    // view at `base..` makes the API robust against future
    // misuse and matches upstream zstd's `dctx->litPtr` semantics (always
    // points at the current frame's literals, never carries
    // history from earlier blocks' Vecs).
    let base = target.len();
    match section.ls_type {
        LiteralsSectionType::Raw => {
            let n = section.regenerated_size as usize;
            // Bounds check: a truncated frame can claim more raw
            // literals than the source slice carries. Return a
            // structured error instead of panicking on `source[0..n]`.
            if source.len() < n {
                return Err(DecompressLiteralsError::MissingBytesForLiterals {
                    got: source.len(),
                    needed: n,
                });
            }
            // Zero-copy: borrow the payload from source. `target` is
            // left untouched — the caller passes `LiteralsView::data`
            // to the sequence executor instead.
            Ok(LiteralsView {
                data: &source[0..n],
                bytes_used: section.regenerated_size,
            })
        }
        LiteralsSectionType::RLE => {
            // RLE expands one byte to N — has to write into target.
            // Need at least one source byte (the fill byte).
            if source.is_empty() {
                return Err(DecompressLiteralsError::MissingBytesForLiterals { got: 0, needed: 1 });
            }
            target.resize(base + section.regenerated_size as usize, source[0]);
            Ok(LiteralsView {
                data: &target[base..],
                bytes_used: 1,
            })
        }
        LiteralsSectionType::Compressed | LiteralsSectionType::Treeless => {
            let bytes_used = decompress_literals(section, scratch, dict, source, target)?;
            Ok(LiteralsView {
                data: &target[base..],
                bytes_used,
            })
        }
    }
}

/// Decompress the provided literals section and source into the provided `target`.
/// This function is used when the literals section is `Compressed` or `Treeless`
///
/// Returns the number of bytes read.
fn decompress_literals(
    section: &LiteralsSection,
    scratch: &mut HuffmanScratch,
    dict: Option<&Dictionary>,
    source: &[u8],
    target: &mut Vec<u8>,
) -> Result<u32, DecompressLiteralsError> {
    // Per-block CpuKernel dispatch. `detect_cpu_kernel()` resolves the
    // tag at most once per process: under `all()` via an
    // `OnceLock` cache around `is_x86_feature_detected!`, and under
    // `no_std` it is a `cfg(target_feature = ...)` const at compile
    // time. Either way the match below collapses to a single cmp+jmp
    // on subsequent calls (or to a single arm at codegen on no-std).
    // Each arm dispatches into a target_feature-wrapped outer function
    // so the entire impl::<K> pipeline executes inside the matching
    // target_feature context — without that wrapping, LLVM cannot
    // inline target_feature'd intrinsics (e.g. _bzhi_u64 inside
    // K::mask_lower_bits) through the trait-method call boundary back
    // into the generic caller, and the inlined-intrinsic win
    // evaporates into a function-call trampoline per mask op.
    match detect_cpu_kernel() {
        #[cfg(target_arch = "x86_64")]
        CpuKernelTag::Avx2 => unsafe {
            decompress_literals_avx2(section, scratch, dict, source, target)
        },
        #[cfg(target_arch = "x86_64")]
        CpuKernelTag::Bmi2 => unsafe {
            decompress_literals_bmi2(section, scratch, dict, source, target)
        },
        _ => decompress_literals_impl::<ScalarKernel>(section, scratch, dict, source, target),
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi2,avx2")]
unsafe fn decompress_literals_avx2(
    section: &LiteralsSection,
    scratch: &mut HuffmanScratch,
    dict: Option<&Dictionary>,
    source: &[u8],
    target: &mut Vec<u8>,
) -> Result<u32, DecompressLiteralsError> {
    decompress_literals_impl::<Avx2Kernel>(section, scratch, dict, source, target)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi2")]
unsafe fn decompress_literals_bmi2(
    section: &LiteralsSection,
    scratch: &mut HuffmanScratch,
    dict: Option<&Dictionary>,
    source: &[u8],
    target: &mut Vec<u8>,
) -> Result<u32, DecompressLiteralsError> {
    decompress_literals_impl::<Bmi2Kernel>(section, scratch, dict, source, target)
}

fn decompress_literals_impl<K: CpuKernel>(
    section: &LiteralsSection,
    scratch: &mut HuffmanScratch,
    dict: Option<&Dictionary>,
    source: &[u8],
    target: &mut Vec<u8>,
) -> Result<u32, DecompressLiteralsError> {
    use DecompressLiteralsError as err;

    let compressed_size = section.compressed_size.ok_or(err::MissingCompressedSize)? as usize;
    let num_streams = section.num_streams.ok_or(err::MissingNumStreams)?;
    let base = target.len();
    let regen = section.regenerated_size as usize;

    target.reserve(regen);
    // Bounds-check the header-derived `compressed_size` before slicing: a
    // truncated/corrupt frame can claim more compressed literal bytes than
    // the source carries. Return a structured error instead of panicking on
    // `source[0..compressed_size]` (decoder DoS), matching the Raw/RLE paths.
    let source = source
        .get(..compressed_size)
        .ok_or(err::MissingBytesForLiterals {
            got: source.len(),
            needed: compressed_size,
        })?;
    let mut bytes_read = 0;

    match section.ls_type {
        LiteralsSectionType::Compressed => {
            //read Huffman tree description
            bytes_read += scratch.table.build_decoder(source)?;
            // Copy-on-write "write": a freshly-built table is local, so the
            // section no longer reads the shared dictionary's Huffman table.
            scratch.mark_table_local();
            vprintln!("Built huffman table using {} bytes", bytes_read);
        }
        LiteralsSectionType::Treeless if scratch.huf_table(dict).max_num_bits == 0 => {
            return Err(err::UninitializedHuffmanTable);
        }

        _ => { /* nothing to do, huffman tree has been provided by previous block */ }
    }

    let source = &source[bytes_read as usize..];

    // Copy-on-write source: the dictionary's Huffman table (zero-copy) on a
    // Treeless section that still reads `Dict`, else the locally-built one.
    let table = scratch.huf_table(dict);

    if num_streams == 4 {
        //build jumptable
        if source.len() < 6 {
            return Err(err::MissingBytesForJumpHeader { got: source.len() });
        }
        let jump1 = source[0] as usize + ((source[1] as usize) << 8);
        let jump2 = jump1 + source[2] as usize + ((source[3] as usize) << 8);
        let jump3 = jump2 + source[4] as usize + ((source[5] as usize) << 8);
        bytes_read += 6;
        let source = &source[6..];

        if source.len() < jump3 {
            return Err(err::MissingBytesForLiterals {
                got: source.len(),
                needed: jump3,
            });
        }

        //decode 4 streams with interleaved operations to hide memory latency
        let streams: [&[u8]; 4] = [
            &source[..jump1],
            &source[jump1..jump2],
            &source[jump2..jump3],
            &source[jump3..],
        ];

        let mut decoders: [HuffmanDecoder<'_>; 4] = [
            HuffmanDecoder::new(table),
            HuffmanDecoder::new(table),
            HuffmanDecoder::new(table),
            HuffmanDecoder::new(table),
        ];
        let mut brs: [BitReaderReversed<'_, K>; 4] = [
            BitReaderReversed::<K>::new(streams[0]),
            BitReaderReversed::<K>::new(streams[1]),
            BitReaderReversed::<K>::new(streams[2]),
            BitReaderReversed::<K>::new(streams[3]),
        ];

        // Initialize all 4 streams: skip padding and set initial state
        for i in 0..4 {
            let mut skipped_bits = 0;
            loop {
                let val = brs[i].get_bits(1);
                skipped_bits += 1;
                if val == 1 || skipped_bits > 8 {
                    break;
                }
            }
            if skipped_bits > 8 {
                return Err(DecompressLiteralsError::ExtraPadding { skipped_bits });
            }
            decoders[i].init_state(&mut brs[i]);
        }

        let max_bits = table.max_num_bits as isize;

        // RFC 8878 §3.1.1.3.2: first 3 streams produce ceil(regen_size/4)
        // symbols each, 4th produces the remainder. Pre-allocate target and
        // decode directly into slices — no temporary Vec allocations.
        let seg = regen.div_ceil(4);

        // Stream the burst + drain output through a raw `*mut u8` into
        // `target`'s spare capacity, leaving `target.len()` at `base`
        // until every byte in [base, base+regen) is written. Only then
        // do we commit via `set_len(base + regen)`. This avoids the
        // `__memset_avx2` zero-init pass that a `resize(base+regen, 0)`
        // would emit (~0.5% of decode self-time on z000033 L-5) while
        // staying sound: at no point does the code construct a
        // `&mut [u8]` reference covering uninitialised bytes.
        //
        // SAFETY: `target.reserve(regen)` at the top of this function
        // guarantees `capacity() >= base + regen`, so the raw pointer
        // can safely address every index in [base, base+regen). Error
        // paths exit BEFORE the final `set_len`, leaving `target.len()`
        // at the pre-call `base` value — uninitialised bytes never
        // become observable to any caller. `u8` has no Drop, so the
        // raw uninitialised tail in spare capacity carries no
        // teardown obligations.
        let target_ptr: *mut u8 = target.as_mut_ptr();
        // Clamp every start/end into [base, base+regen] so cursors can
        // never index past the pre-allocated region, even with corrupted
        // frame headers that produce small regen (where N*seg > regen).
        let limit = base + regen;
        let starts: [usize; 4] = [
            base,
            (base + seg).min(limit),
            (base + 2 * seg).min(limit),
            (base + 3 * seg).min(limit),
        ];
        let ends: [usize; 4] = [starts[1], starts[2], starts[3], limit];
        let mut cursors = starts;

        // Upstream zstd-parity 4-stream HUF decode. `bits[s]` is the fused
        // state+stream+sentinel u64 register (see `run_4stream_burst_loop`).
        // Each iter decodes `symbols_per_burst` symbols × 4 streams,
        // then reloads all 4 stream registers via `ip[s] -= nb_bytes;
        // MEM_read64(ip[s]) | 1`.
        let max_num_bits = table.max_num_bits;
        // Safety constraint per upstream zstd `HUF_decompress4X1_usingDTable_internal_fast_c_loop`:
        // before each `bits[s] >> table_shift` read, the sentinel-bit position
        // must be strictly below bit `64 - max_num_bits` (i.e. outside the top
        // `max_num_bits` read region). After `s` shifts the sentinel is at bit
        // `padding_skip + s*max_num_bits`. The N-th read happens after (N-1)
        // shifts, so the inclusive bound is
        //   padding_skip + (N-1)*max_num_bits < 64 - max_num_bits
        // i.e.
        //   padding_skip + N*max_num_bits <= 63
        // Solving for N with padding_skip ≤ 8:
        //   N <= (63 - 8) / max_num_bits = 55 / max_num_bits
        // (Letter `s` is used here for shift-count to avoid colliding with
        // the surrounding generic parameter `K: CpuKernel`.)
        // For max=11: 5 symbols (upstream zstd parity — was 4 with the old off-by-one
        // formula). For max=8: 6 symbols. For max=4: 13.
        let symbols_per_burst: usize = (63 - 8) / max_num_bits as usize;
        let burst_bits = (symbols_per_burst * max_num_bits as usize) as u8;
        let table_shift = (64 - max_num_bits) as u32;
        let packed = table.packed_decode.as_slice();

        // Lockstep cursor invariant: every burst iter advances all 4
        // cursors by `symbols_per_burst` in step, so `cursors[0]`
        // tracks progress for all four streams. `cursor_exit_olimit
        // = starts[0] + min(seg_len[i])` is the cursor value at which
        // the lagging segment runs out — upstream zstd parity with
        // `huf_decompress.c` `olimit`-style single-pointer bound.
        let min_seg_len = (ends[0] - starts[0])
            .min(ends[1] - starts[1])
            .min(ends[2] - starts[2])
            .min(ends[3] - starts[3]);
        // `burst_eligible` is a load-bearing safety gate against
        // adversarial frame headers. If `min_seg_len < symbols_per_burst`
        // (small `regenerated_size` paired with large compressed
        // streams, forging a 4-stream HUF block where
        // `seg = div_ceil(regen, 4) < symbols_per_burst`) then
        // `cursor_burst_ceil` saturates to 0 and `cursors[0] <= 0`
        // is trivially true on entry, admitting a burst whose inner
        // loop would advance `cursors[i]` past `ends[i]` and panic
        // on the `target[cursors[i]]` write. Requiring
        // `min_seg_len >= symbols_per_burst` up front means the
        // burst only runs when a full burst fits inside EVERY
        // segment; the drain phase outside `run_4stream_burst_loop`
        // handles the small-`min_seg_len` case via single-symbol
        // per-stream decode.
        let burst_eligible = symbols_per_burst >= 1 && min_seg_len >= symbols_per_burst;
        let cursor_burst_ceil = (starts[0] + min_seg_len).saturating_sub(symbols_per_burst);

        let bounds = LoopBounds {
            symbols_per_burst,
            burst_bits,
            table_shift,
            cursor_burst_ceil,
            burst_eligible,
            alloc_upper_bound: base + regen,
        };

        // Burst is identical across all kernels (upstream zstd parity: reads
        // `packed[idx]` u16 directly + `MEM_read64` reload pattern,
        // no SIMD intrinsics needed). Single un-genericised call.
        //
        // SAFETY: caller guarantees `brs[s].source` is the same as the
        // stream slice each decoder was initialised against; `target_ptr`
        // addresses an allocation of at least `base + regen` bytes (via
        // the `target.reserve(regen)` above), so cursor writes in
        // [base, base+regen) are in-bounds; `packed` length matches
        // `1 << max_num_bits` by `HuffmanTable::build_decoder`'s `resize`.
        unsafe {
            run_4stream_burst_loop(
                &mut decoders,
                &mut brs,
                target_ptr,
                packed,
                &mut cursors,
                &bounds,
            );
        }

        // Drain remaining symbols from each stream, bounded by segment end.
        // SAFETY: cursors[i] ∈ [base, base+regen) by `starts`/`ends` clamping
        // earlier; `target_ptr.add(cursors[i])` is therefore within the
        // reserved allocation. Each write initialises one previously-
        // uninitialised byte; `target.len()` remains at `base` so no
        // `&mut [u8]` reference is constructed to that byte before it is
        // written. The error exits below DO NOT call `target.truncate(base)`
        // — `target.len()` is still `base` already, so a truncate would be
        // a no-op; eliding it also lets us hold `target_ptr` without an
        // intervening `&mut Vec<u8>` borrow that invalidates the pointer
        // under stacked-borrows.
        // Per-stream tail decode, mirroring upstream zstd `HUF_decodeStreamX1`
        // (huf_decompress.c:546): decode in groups of 4 with ONE reload per
        // group, then the final < 4 symbols per-symbol. The previous form
        // reloaded the bit reader on EVERY symbol, so each trailing symbol near
        // a stream end paid `refill_slow` — the dominant drain cost on a
        // literal-heavy frame.
        let group_bits = 4 * max_num_bits;
        for i in 0..4 {
            // Phase 1 (upstream `HUF_decodeStreamX1` lines 551-557): decode in
            // groups of four with a SINGLE reload per group. Output-based bound
            // (`cursors[i] + 4 <= ends[i]`), like upstream's `p < pEnd-3`, so a
            // group only runs while four whole symbols remain — it never reads
            // past the last symbol into the zero padding.
            while cursors[i] + 4 <= ends[i] {
                brs[i].ensure_bits(group_bits);
                for _ in 0..4 {
                    let byte = decoders[i].decode_symbol_and_advance_no_refill(&mut brs[i]);
                    unsafe {
                        target_ptr.add(cursors[i]).write(byte);
                    }
                    cursors[i] += 1;
                }
            }
            // Phase 2 (upstream `HUF_decodeStreamX1` line 568 `while (p < pEnd)`):
            // the final < 4 symbols. ONE reload covers them (<= 3 * max_num_bits
            // bits), then NO per-symbol reload — so the trailing symbols at the
            // stream end never pay the cold `refill_slow` each. `bits_remaining`
            // is reload-timing-independent (the padding lands in either
            // `extra_bits` or `bits_consumed`, and `(64 - bits_consumed) -
            // extra_bits` is identical), so the end-of-stream check below still
            // holds exactly.
            if cursors[i] < ends[i] {
                brs[i].ensure_bits(group_bits);
                while cursors[i] < ends[i] {
                    let byte = decoders[i].decode_symbol_and_advance_no_refill(&mut brs[i]);
                    unsafe {
                        target_ptr.add(cursors[i]).write(byte);
                    }
                    cursors[i] += 1;
                }
            }
            if brs[i].bits_remaining() != -max_bits {
                return Err(DecompressLiteralsError::BitstreamReadMismatch {
                    read_til: brs[i].bits_remaining(),
                    expected: -max_bits,
                });
            }
        }

        // Verify total decoded count matches expected regenerated size.
        let decoded: usize = cursors.iter().zip(starts.iter()).map(|(c, s)| c - s).sum();
        if decoded != regen {
            return Err(DecompressLiteralsError::DecodedLiteralCountMismatch {
                decoded,
                expected: regen,
            });
        }

        // Commit: every byte in [base, base+regen) was written above
        // (cursors[s] reached ends[s] for all s, and the decoded total
        // equals regen). `target.len()` was `base` until this point —
        // exposing the freshly-initialised tail is now sound.
        // SAFETY: see the `target_ptr` block above.
        unsafe {
            target.set_len(base + regen);
        }

        bytes_read += source.len() as u32;
    } else {
        //just decode the one stream
        assert!(num_streams == 1);
        let mut decoder = HuffmanDecoder::new(table);
        let mut br = BitReaderReversed::<K>::new(source);
        let mut skipped_bits = 0;
        loop {
            let val = br.get_bits(1);
            skipped_bits += 1;
            if val == 1 || skipped_bits > 8 {
                break;
            }
        }
        if skipped_bits > 8 {
            //if more than 7 bits are 0, this is not the correct end of the bitstream. Either a bug or corrupted data
            return Err(DecompressLiteralsError::ExtraPadding { skipped_bits });
        }
        decoder.init_state(&mut br);
        while br.bits_remaining() > -(table.max_num_bits as isize) {
            target.push(decoder.decode_symbol_and_advance(&mut br));
        }
        let expected = -(table.max_num_bits as isize);
        if br.bits_remaining() != expected {
            target.truncate(base);
            return Err(DecompressLiteralsError::BitstreamReadMismatch {
                read_til: br.bits_remaining(),
                expected,
            });
        }
        bytes_read += source.len() as u32;
    }

    if target.len() != base + regen {
        let decoded = target.len() - base;
        target.truncate(base);
        return Err(DecompressLiteralsError::DecodedLiteralCountMismatch {
            decoded,
            expected: regen,
        });
    }

    Ok(bytes_read)
}

/// Loop-invariant constants for [`run_4stream_burst_loop`]. Derived
/// once per `decompress_literals` call; `Copy` so the burst can
/// destructure `*bounds` for register-resident reads.
#[derive(Copy, Clone)]
struct LoopBounds {
    symbols_per_burst: usize,
    burst_bits: u8,
    table_shift: u32,
    cursor_burst_ceil: usize,
    /// Set iff a full burst (`symbols_per_burst` symbols per stream)
    /// can fit in the lagging segment. When false the burst is
    /// hard-disabled and the drain phase outside the burst loop
    /// decodes ALL symbols via the single-symbol path. Setup-site
    /// safety rationale: adversarial / small-regen DoS guard.
    burst_eligible: bool,
    /// Upper bound (exclusive) on cursor values written through
    /// `target_ptr` — equals `base + regen` at the caller. Carried
    /// here so the burst loop's SAFETY argument can be stated
    /// purely in terms of values visible inside the function. The
    /// caller guarantees the allocation behind `target_ptr` covers
    /// `[0, alloc_upper_bound)` (via `target.reserve(regen)`).
    alloc_upper_bound: usize,
}

/// Upstream zstd-parity 4-stream HUF decode burst loop. Single code path —
/// no kernel dispatch, no SIMD-fallback hybrid. Mirrors
/// `huf_decompress.c:HUF_decompress4X1_usingDTable_internal_fast_c_loop`:
/// each outer iter decodes `symbols_per_burst` symbols × 4 streams,
/// then reloads all 4 stream registers from raw source bytes via the
/// `ctz(bits[s])` → `ip[s] -= nb_bytes` → `MEM_read64(ip[s])` pattern.
///
/// State + unconsumed stream + sentinel are fused into one u64
/// per stream (`bits[s]`). The decoder's separate `state` field is
/// reconstructed once at burst exit for the drain phase below.
///
/// # Safety
///
/// All four decoders must share the same table (holds by construction —
/// built from `&scratch.table`).
///
/// `target_ptr` must come from a `Vec<u8>` whose allocation is at least
/// `base + regen` bytes (the caller guarantees this via
/// `target.reserve(regen)` before deriving the pointer). The Vec must
/// not be reallocated or moved while `target_ptr` is in use — the
/// caller holds no other references during the burst+drain phase so
/// this invariant is upheld trivially.
///
/// Writes go through raw `target_ptr.add(cursors[s]).write(byte)` so
/// no Rust reference is ever constructed to the uninitialised tail
/// (`target.len()` stays at `base` until the post-loop `set_len`
/// commits initialisation). Cursor bounds `[base, base+regen)` are
/// enforced by the `starts`/`ends` clamping in the caller.
///
/// Each `brs[s].source` must be the slice the corresponding decoder
/// was initialised against.
#[inline(always)]
unsafe fn run_4stream_burst_loop<K: CpuKernel>(
    decoders: &mut [HuffmanDecoder<'_>; 4],
    brs: &mut [BitReaderReversed<'_, K>; 4],
    target_ptr: *mut u8,
    packed: &[u16],
    cursors: &mut [usize; 4],
    bounds: &LoopBounds,
) {
    let LoopBounds {
        symbols_per_burst,
        burst_bits,
        table_shift,
        cursor_burst_ceil,
        burst_eligible,
        alloc_upper_bound,
    } = *bounds;
    let max_num_bits = (64 - table_shift) as u8;

    // Skip burst entirely if min_seg_len < symbols_per_burst — drain
    // (the single-symbol tail outside this function) handles ALL
    // symbols. See the `burst_eligible` doc on `LoopBounds`. Bailing
    // here also covers the malformed-frame case where `regen` is
    // smaller than a single burst's output (the caller's allocation
    // is then too small to hold even one burst, but the drain path
    // handles symbol-by-symbol decode within the segment ends).
    if !burst_eligible {
        return;
    }

    // Caller-side cursor bound check, restated here so SAFETY reasoning
    // below is self-contained against in-scope values only. The outer
    // gate `cursors[0] <= cursor_burst_ceil` plus lockstep advance
    // ensures every write index is `< cursor_burst_ceil + symbols_per_burst`;
    // requiring that bound to fit inside the caller's allocation makes
    // each `target_ptr.add(idx).write(_)` provably in-bounds. Only
    // meaningful once `burst_eligible == true` (a malformed-frame
    // small-regen path can leave `cursor_burst_ceil` saturated below
    // `alloc_upper_bound - symbols_per_burst` — the burst skip above
    // already bails on that case).
    debug_assert!(
        cursor_burst_ceil + symbols_per_burst <= alloc_upper_bound,
        "caller must size the target allocation so the lockstep-advanced \
         cursors stay within bounds across a full burst",
    );

    // Upstream zstd-parity burst loop. `bits[s]` is the unified u64 register
    // that fuses state + unconsumed stream + sentinel:
    //   bits 63..(64-max_num_bits): current state (next index into `packed`)
    //   below:                       upcoming stream bits, top-aligned
    //   bottom:                      sentinel `1`, position grows upward
    //                                with each consumed bit
    //
    // The encoder side of HUF writes the bitstream backward such that
    // at every byte boundary the top `max_num_bits` of unconsumed
    // stream = current state. So state is implicit in `bits[s]`; we
    // do NOT carry a separate `decoder.state` inside the burst — it
    // is reconstructed via `bits[s] >> table_shift` at the burst exit
    // and written back to `decoders[s].state` for the drain phase.
    //
    // Composition matches upstream zstd `HUF_DecompressFastArgs_init` and
    // `HUF_4X1_RELOAD_STREAM` (huf_decompress.c:795-804): each iter
    // reloads `bits[s] = MEM_read64(ip[s]) | 1; bits[s] <<= nb_bits`
    // after advancing `ip[s] -= nb_bytes` (where nb_bytes/nb_bits
    // come from `ctz(bits[s])` at the end of the previous iter).
    // Initial composition exactly mirrors upstream zstd `HUF_DecompressFastArgs_init`:
    // `bits[s] = (MEM_read64(ip) | 1) << padding_skip`. Top `max_num_bits`
    // of the result is the state value implicitly (HUF stream encoding
    // ensures the top max bits of unconsumed stream at any consumption
    // point = current state machine state), so we don't inject
    // `decoders[s].state` explicitly here — the bit pattern already
    // carries it.
    //
    // `padding_skip = brs[s].bits_consumed - max_num_bits`: `init_state`
    // pre-consumed `max_num_bits` for `decoders[s].state`, so
    // `brs[s].bits_consumed = padding_skip + max_num_bits`. Upstream zstd leaves
    // state implicit; we reverse our pre-consumption by shifting only
    // by `padding_skip` (not by `bits_consumed`) so the top max bits
    // come from the unshifted stream-position-of-state.
    //
    // Sentinel ends up at bit `padding_skip` after the shift, so
    // `ctz(initial bits[s]) = padding_skip` and the first reload's
    // `nb_bytes = (padding_skip + K) / 8` matches upstream zstd's byte-cursor
    // advance from absolute stream position 0.
    // Scalarise the four per-stream registers into named locals so the
    // optimiser keeps them in registers across the whole burst+reload —
    // upstream zstd's hand-written 4X1 fast loop holds all four stream
    // states in registers. The previous `[u64; 4]` / `[usize; 4]` arrays,
    // plus the `cursors` array reached through a `&mut` reference, forced a
    // stack slot per stream: every decoded symbol reloaded `bits[s]` and did
    // a memory RMW on `cursors[s]` — the dominant cost on literal-heavy
    // frames (measured ~20x the upstream per-symbol instruction count).
    //
    // `b{s}` fuses state + unconsumed stream + sentinel (module doc above):
    // initial composition mirrors `HUF_DecompressFastArgs_init`,
    // `(MEM_read64(ip) | 1) << padding_skip`. `nbl{s}` is the sub-byte phase
    // carried into the writeback so the single-symbol drain resumes with
    // `bits_consumed = nbl + max_num_bits`.
    let mut b0 = (brs[0].bit_container | 1) << (brs[0].bits_consumed - max_num_bits);
    let mut b1 = (brs[1].bit_container | 1) << (brs[1].bits_consumed - max_num_bits);
    let mut b2 = (brs[2].bit_container | 1) << (brs[2].bits_consumed - max_num_bits);
    let mut b3 = (brs[3].bit_container | 1) << (brs[3].bits_consumed - max_num_bits);
    let mut ip0 = brs[0].index;
    let mut ip1 = brs[1].index;
    let mut ip2 = brs[2].index;
    let mut ip3 = brs[3].index;
    let mut c0 = cursors[0];
    let mut c1 = cursors[1];
    let mut c2 = cursors[2];
    let mut c3 = cursors[3];
    // `source` is `&[u8]` (a Copy slice reference), so caching it per stream
    // takes no borrow of `brs` — the writeback below can still take `brs`
    // mutably while these stay live.
    let src0 = brs[0].source;
    let src1 = brs[1].source;
    let src2 = brs[2].source;
    let src3 = brs[3].source;

    // Decode one symbol on one stream: index `packed` by the top
    // `max_num_bits` of the fused register, emit the byte, consume the code's
    // bit length. SAFETY: `idx = b >> table_shift` lands in
    // `[0, 1<<max_num_bits) == packed.len()`; lockstep advance keeps every
    // cursor `< cursor_burst_ceil + symbols_per_burst <= alloc_upper_bound`,
    // so each `target_ptr.add(c)` write is in-bounds (caller's `reserve`).
    macro_rules! decode1 {
        ($b:ident, $c:ident) => {{
            let idx = ($b >> table_shift) as usize;
            let entry = unsafe { *packed.get_unchecked(idx) };
            unsafe { target_ptr.add($c).write((entry & 0xFF) as u8) };
            $c += 1;
            $b <<= (entry >> 8) & 0xFF;
        }};
    }
    // Reload one stream (upstream zstd `HUF_4X1_RELOAD_STREAM`):
    // `ip -= ctz>>3; b = (MEM_read64(ip) | 1) << (ctz & 7)`. SAFETY: the
    // `min_ip >= bytes_per_iter_upper` gate at loop entry keeps `ip -=
    // nb_bytes` and the 8-byte window read in-bounds (see the budget note).
    macro_rules! reload1 {
        ($b:ident, $ip:ident, $src:expr) => {{
            let ctz = $b.trailing_zeros();
            $ip -= (ctz >> 3) as usize;
            let nb_bits = (ctz & 7) as u8;
            let new_window = u64::from_le_bytes(unsafe {
                $src.get_unchecked($ip..$ip + 8)
                    .try_into()
                    .unwrap_unchecked()
            });
            $b = (new_window | 1) << nb_bits;
        }};
    }
    // One burst: `$n` symbols across all four streams. `$n` is a literal so
    // the body fully unrolls (upstream zstd's hardcoded `for symbol in 0..N`).
    macro_rules! burst {
        ($n:literal) => {{
            for _ in 0..$n {
                decode1!(b0, c0);
                decode1!(b1, c1);
                decode1!(b2, c2);
                decode1!(b3, c3);
            }
        }};
    }

    // Upstream zstd `iiters` safety budget. Worst-case `nb_bytes` per iter is
    // `floor(ctz_max / 8)` where `ctz_max = pad_max + burst_bits`; taking
    // `pad_max = 8` covers the first iter (sentinel at padding_skip ∈ [1,8])
    // and subsequent iters (nb_bits ∈ [0,7]). The `min_ip >=
    // bytes_per_iter_upper` gate before each iter keeps every stream's
    // `ip -= nb_bytes` plus the `source[ip..ip+8]` read in-bounds without
    // per-stream conditionals.
    let bytes_per_iter_upper = (8 + burst_bits as usize) / 8;
    let mut any_iter = false;

    while c0 <= cursor_burst_ceil {
        let min_ip = ip0.min(ip1).min(ip2).min(ip3);
        if min_ip < bytes_per_iter_upper {
            break;
        }
        any_iter = true;

        // Dispatch on the compile-time symbol count so the dominant cases get
        // a fully-unrolled body. `symbols_per_burst` is loop-invariant, so
        // loop-unswitching hoists the match out of the `while`; each arm
        // expands `burst!` to straight-line scalar code. SPB=5 covers
        // `max_num_bits ∈ {10, 11}` (the large-alphabet, literal-heavy case
        // that dominates decode cost); 6 covers {8, 9}, 7 covers {7}. Rarer
        // small-max tables fall through to the dynamic loop.
        match symbols_per_burst {
            5 => burst!(5),
            6 => burst!(6),
            7 => burst!(7),
            _ => {
                for _ in 0..symbols_per_burst {
                    decode1!(b0, c0);
                    decode1!(b1, c1);
                    decode1!(b2, c2);
                    decode1!(b3, c3);
                }
            }
        }

        // Reload all 4 streams (upstream zstd `HUF_4X1_RELOAD_STREAM`).
        //
        // SAFETY:
        //   * `ip[s] - nb_bytes >= 0`: the `min_ip >= bytes_per_iter_upper`
        //     gate at outer-loop entry guarantees `nb_bytes <= bytes_per_iter_upper`
        //     (where `nb_bytes = ctz(bits[s]) >> 3` and `ctz <= padding_skip
        //     + burst_bits <= 8 + burst_bits`, the bound `bytes_per_iter_upper`
        //     pre-computes).
        //   * `ip[s] + 8 <= source.len()`: `BitReaderReversed::new()`
        //     starts with `bits_consumed = 64`, so the very first
        //     `get_bits(1)` in the per-stream padding-skip loop
        //     above triggers `refill()`. For `source.len() >= 8` that
        //     fast-path establishes `brs[s].index = source.len() - 8`;
        //     `init_state`'s subsequent `get_bits(max_num_bits)`
        //     stays inside the same 8-byte window without another
        //     refill (only `bits_consumed` advances). The
        //     `refill_slow` path used for shorter streams leaves
        //     `index = 0` (with the partial bytes left-shifted into
        //     `bit_container`), making `min_ip = 0 <
        //     bytes_per_iter_upper` so the burst loop exits via
        //     `any_iter = false` BEFORE reaching this reload (the
        //     writeback below is unreachable on `source.len() < 8`).
        //     Within the loop, `ip[s]` only decreases via the line
        //     above this comment, preserving the upper bound.
        // Reload order `(MEM_read64 | 1) << nb_bits` (NOT `(.. << nb_bits) | 1`):
        // the sentinel must land at bit `nb_bits` so the next reload's `ctz`
        // accumulates the sub-byte phase; resetting it to bit 0 loses the
        // phase between reloads.
        reload1!(b0, ip0, src0);
        reload1!(b1, ip1, src1);
        reload1!(b2, ip2, src2);
        reload1!(b3, ip3, src3);
    }

    // Commit cursors to the caller's array (the drain phase reads them)
    // whether or not any burst iter ran.
    cursors[0] = c0;
    cursors[1] = c1;
    cursors[2] = c2;
    cursors[3] = c3;

    // No iter ran → `brs` / `decoders` untouched; the drain resumes from the
    // post-`init_state` reader.
    if !any_iter {
        return;
    }

    // Write back reader + decoder state so the single-symbol drain resumes
    // where the burst stopped: `bits_consumed = nbl + max_num_bits` (the
    // sub-byte phase from the last reload plus the `max_num_bits` consumed
    // for the state just extracted), `state = b >> table_shift`.
    macro_rules! writeback {
        ($i:literal, $b:ident, $ip:ident, $src:expr) => {{
            brs[$i].index = $ip;
            brs[$i].bit_container = u64::from_le_bytes(unsafe {
                $src.get_unchecked($ip..$ip + 8)
                    .try_into()
                    .unwrap_unchecked()
            });
            // bits_consumed = sub-byte phase + max_num_bits. After the final
            // reload b{s} = (window | 1) << nb_bits, so trailing_zeros(b{s}) is
            // exactly that nb_bits (the sentinel sits at bit nb_bits). Recompute
            // it here instead of carrying a per-stream `nbl` register across the
            // whole burst loop — four fewer loop-live values.
            brs[$i].bits_consumed = $b.trailing_zeros() as u8 + max_num_bits;
            decoders[$i].state = $b >> table_shift;
        }};
    }
    writeback!(0, b0, ip0, src0);
    writeback!(1, b1, ip1, src1);
    writeback!(2, b2, ip2, src2);
    writeback!(3, b3, ip3, src3);
}
