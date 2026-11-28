#![allow(
    clippy::cast_lossless,
    clippy::explicit_iter_loop,
    clippy::manual_let_else,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! AVX2-tier monolithic sequence-section decoder.
//!
//! One self-contained `#[target_feature(enable = "bmi2,avx2")]` function
//! with the entire decode + execute pipeline as ONE body. Sequence-decode
//! and sequence-execute logic lives in `macro_rules!` blocks that expand
//! textually at every callsite — no inner function CALL boundaries, no
//! reliance on the LLVM inline cost-model (which would not inline through
//! `target_feature` + multiple callsites + `Result<>` panic landings).
//! Macros expand BEFORE LLVM sees the code, guaranteeing zero call
//! overhead regardless of cost-model decisions.
//!
//! BitReader pinned to `Avx2Kernel`; triple-bit extract goes directly
//! through `peek_bits_triple_bmi2` (`_pext_u64` inline at every callsite).
//! Match copy routes to `BufferBackend::exec_sequence_inline_avx2`
//! (32-byte ymm wildcopy).

#![cfg(target_arch = "x86_64")]

use super::buffer_backend::BufferBackend;
use super::decode_buffer::DecodeBuffer;
use super::exec_sequence_inline::exec_sequence_avx2_inline;
use super::scratch::FSEScratch;
use super::sequence_section_decoder::{
    ADVANCE, ADVANCE_MASK, ExecSeq, SeqStreamSetup, init_sequence_stream,
};
use crate::blocks::sequence_section::{MAX_OFFSET_CODE, Sequence, SequencesHeader};
use crate::cpu_kernel::Avx2Kernel;
use crate::decoding::errors::{DecodeSequenceError, DecompressBlockError, ExecuteSequencesError};
use crate::decoding::sequence_execution::do_offset_history;

/// Textual expansion of per-sequence decode. Reads LL/ML/OF state,
/// performs triple-bit extract via `peek_bits_triple_bmi2` (`_pext_u64`
/// inline when vendor cache enables it), advances the bit cursor.
/// Expands at every callsite inside the AVX2 monolith — no function
/// boundary survives compilation.
macro_rules! decode_one_body {
    ($ll_dec:expr, $ml_dec:expr, $of_dec:expr, $br:expr) => {{
        let ll_state = $ll_dec.state;
        let ml_state = $ml_dec.state;
        let of_state = $of_dec.state;

        let ll_value = ll_state.base_value;
        let ll_num_bits = ll_state.num_additional_bits;
        let ml_value = ml_state.base_value;
        let ml_num_bits = ml_state.num_additional_bits;
        let of_num_bits = of_state.num_additional_bits;
        let of_base = of_state.base_value;

        debug_assert!(of_num_bits <= MAX_OFFSET_CODE);

        let sum_wide = u16::from(of_num_bits) + u16::from(ml_num_bits) + u16::from(ll_num_bits);
        let (obits, ml_add, ll_add) = if sum_wide <= 56 {
            // Upstream `ZSTD_decodeSequence` reads OF/ML/LL as three separate
            // `BIT_readBitsFast` after one reload. The fields are independent so
            // the CPU pipelines them; the old PEXT-triple folded all three into
            // one serial `pext` dependency. One `ensure_bits` up front, then
            // three unchecked reads (same bit order, byte-identical).
            $br.ensure_bits(sum_wide as u8);
            (
                $br.get_bits_unchecked(of_num_bits),
                $br.get_bits_unchecked(ml_num_bits),
                $br.get_bits_unchecked(ll_num_bits),
            )
        } else {
            (
                $br.get_bits(of_num_bits),
                $br.get_bits(ml_num_bits),
                $br.get_bits(ll_num_bits),
            )
        };
        let offset = obits as u32 + of_base;
        debug_assert_ne!(offset, 0);

        Sequence {
            ll: ll_value + ll_add as u32,
            ml: ml_value + ml_add as u32,
            of: offset,
        }
    }};
}

/// Branchy offset/repcode resolution + ml/ll reads, parameterised by the
/// bit-reader method `$rd` (`get_bits` = demand-refilled, or
/// `get_bits_unchecked` = no per-read refill check after a prior
/// `ensure_bits`). The control flow mirrors upstream `ZSTD_decodeSequence`:
/// the offset extra-bit read is folded INTO the `ofBits>1 / ==0 / ==1`
/// branches and the repcode history rotation is resolved inline. Expands to
/// `(ll, ml, actual_offset)`; rotates `$hist` in place.
macro_rules! cshape_resolve {
    (
        $rd:ident, $ll_base:expr, $ml_base:expr, $of_base:expr,
        $ll_bits:expr, $ml_bits:expr, $of_bits:expr, $br:expr, $hist:expr
    ) => {{
        let ll_base = $ll_base;
        let of_base = $of_base;
        let ml_bits = $ml_bits;
        let ll_bits = $ll_bits;
        let of_bits = $of_bits;

        let actual_offset: u32 = if of_bits > 1 {
            // Real offset: read ofBits, no repcode. offBase = of_base + raw >= 4.
            let raw = $br.$rd(of_bits) as u32;
            let resolved = (of_base + raw).wrapping_sub(3);
            $hist[2] = $hist[1];
            $hist[1] = $hist[0];
            $hist[0] = resolved;
            resolved
        } else {
            let ll0 = ll_base == 0;
            if of_bits == 0 {
                // Repcode 0 (most common): no offset bits consumed.
                let idx = usize::from(ll0);
                let resolved = $hist[idx];
                $hist[1] = $hist[idx ^ 1];
                $hist[0] = resolved;
                resolved
            } else {
                // ofBits == 1: one bit selects among rep1..rep3. The upstream
                // repcode base for this arm is 1 (our of_base is 2 here).
                let bit = $br.$rd(1) as u32;
                let off_code = 1 + u32::from(ll0) + bit; // in {1,2,3}
                let mut temp = if off_code == 3 {
                    $hist[0].wrapping_sub(1)
                } else {
                    $hist[off_code as usize]
                };
                // 0 is not a valid offset: force corruption to surface downstream
                // (upstream `temp -= !temp`; our executor rejects offset 0).
                temp = temp.wrapping_sub(u32::from(temp == 0));
                if off_code != 1 {
                    $hist[2] = $hist[1];
                }
                $hist[1] = $hist[0];
                $hist[0] = temp;
                temp
            }
        };
        debug_assert_ne!(actual_offset, 0);

        // === Match length + literal length extra bits ===
        let ml = $ml_base
            + if ml_bits > 0 {
                $br.$rd(ml_bits) as u32
            } else {
                0
            };
        let ll = ll_base
            + if ll_bits > 0 {
                $br.$rd(ll_bits) as u32
            } else {
                0
            };

        (ll, ml, actual_offset)
    }};
}

/// Fused decode + offset-resolution for one sequence (upstream zstd shape).
///
/// Mirrors upstream zstd `ZSTD_decodeSequence` (zstd_decompress_block.c:1228-1346)
/// branch-for-branch on the 64-bit path (see [`cshape_resolve`]), with our
/// optimisation woven back in: the common `total <= 56` case does ONE
/// `ensure_bits` up front, then all of/ml/ll reads go through
/// `get_bits_unchecked` (no per-field refill branch). Only the rare
/// wide-offset case (`total > 56`) falls back to demand-refilled `get_bits`.
/// This keeps the winning branchy offset/repcode shape while reclaiming the
/// single-refill efficiency the old PEXT-triple path had.
///
/// Our offset table stores `base_value = 1 << ofCode`, so `of_base + raw` is the
/// offBase domain (1/2/3 = repcodes, >=4 = real offset + 3). The arithmetic here
/// converts to the real-offset domain that the executor consumes and that
/// `offset_hist` records (verified equal to `do_offset_history` across its full
/// test matrix). Expands to `(ll, ml, actual_offset)`; rotates `$hist` in place.
macro_rules! decode_seq_fused_cshape {
    ($ll_dec:expr, $ml_dec:expr, $of_dec:expr, $br:expr, $hist:expr) => {{
        let ll_state = $ll_dec.state;
        let ml_state = $ml_dec.state;
        let of_state = $of_dec.state;

        let ll_base = ll_state.base_value;
        let ml_base = ml_state.base_value;
        let of_base = of_state.base_value;
        let ll_bits = ll_state.num_additional_bits;
        let ml_bits = ml_state.num_additional_bits;
        let of_bits = of_state.num_additional_bits;

        debug_assert!(of_bits <= MAX_OFFSET_CODE);

        // total = exact bits consumed by this sequence in every arm (rep-0
        // reads 0 offset bits so total = ml+ll; rep-1 reads 1; real reads ofBits).
        let total = u16::from(of_bits) + u16::from(ml_bits) + u16::from(ll_bits);
        if total <= 56 {
            $br.ensure_bits(total as u8);
            cshape_resolve!(
                get_bits_unchecked,
                ll_base,
                ml_base,
                of_base,
                ll_bits,
                ml_bits,
                of_bits,
                $br,
                $hist
            )
        } else {
            cshape_resolve!(
                get_bits, ll_base, ml_base, of_base, ll_bits, ml_bits, of_bits, $br, $hist
            )
        }
    }};
}

/// Textual expansion of per-sequence execute. Fast path: the inlined AVX2
/// match-copy macro [`exec_sequence_avx2_inline`]. Cold path: legacy
/// try_push + repeat_lookahead_prefetched. Expands as a statement-block
/// returning `Result<(), DecompressBlockError>` so the caller can `?`
/// or branch on it as needed.
macro_rules! execute_one_body {
    (
        $buffer:expr,
        $dict:expr,
        $literals_buffer:expr,
        $lit_cur:expr,
        $literals_buffer_len:expr,
        $seq_ll:expr,
        $seq_ml:expr,
        $resolved_offset:expr
    ) => {{
        // Labeled-block expansion — every early exit is
        // `break 'exec_inner Err(...)`, no closure, no `?` operator,
        // so the macro body inlines into the caller with zero CALL
        // boundary even at -Copt-level=0.
        let _result: Result<(), DecompressBlockError> = 'exec_inner: {
            let seq_ll_v: u32 = $seq_ll;
            let seq_ml_v: u32 = $seq_ml;
            let resolved_offset_v: u32 = $resolved_offset;
            let literals_buffer_len_v: usize = $literals_buffer_len;
            let lit_cur_before = *$lit_cur;
            let high = match lit_cur_before
                .checked_add(seq_ll_v as usize)
                .filter(|&h| h <= literals_buffer_len_v)
            {
                Some(h) => h,
                None => {
                    break 'exec_inner Err(ExecuteSequencesError::NotEnoughBytesForSequence {
                        wanted: lit_cur_before.saturating_add(seq_ll_v as usize),
                        have: literals_buffer_len_v,
                    }
                    .into());
                }
            };
            // SAFETY: high <= literals_buffer_len_v, lit_cur_before <= high.
            let lits = unsafe { $literals_buffer.get_unchecked(lit_cur_before..high) };
            *$lit_cur = high;

            if resolved_offset_v == 0 {
                break 'exec_inner Err(ExecuteSequencesError::ZeroOffset.into());
            }

            // Upstream zstd inline-eligibility gates. `inline_exec_ok` lets a wrapping
            // backend (RingBuffer) veto the inline path when the live region is
            // not contiguous at `tail`; linear backends fold it to `true`.
            let inline_path_safe = B::SUPPORTS_INLINE_SEQUENCE_EXEC
                && $buffer.buffer_mut().inline_exec_ok(
                    seq_ll_v as usize,
                    seq_ml_v as usize,
                    resolved_offset_v as usize,
                )
                && lit_cur_before
                    .checked_add(16)
                    .is_some_and(|b| b <= literals_buffer_len_v)
                && (seq_ll_v as usize <= 16
                    || lit_cur_before
                        .checked_add((seq_ll_v as usize).next_multiple_of(16))
                        .is_some_and(|b| b <= literals_buffer_len_v));

            if inline_path_safe {
                let buf_len = $buffer.len();
                let offset = resolved_offset_v as usize;
                let prefix_end_ok = buf_len
                    .checked_add(lits.len())
                    .is_some_and(|end| offset <= end);
                if prefix_end_ok {
                    // SAFETY: parent-slice provenance; offset prefix-resident.
                    let lit_src = unsafe { $literals_buffer.as_ptr().add(lit_cur_before) };
                    // Inline the AVX2 exec body at the call site (no trait-method
                    // call boundary; see `exec_sequence_avx2_inline`).
                    let r = exec_sequence_avx2_inline!(
                        $buffer,
                        lit_src,
                        seq_ll_v as usize,
                        offset,
                        seq_ml_v as usize
                    );
                    // Inline path bypasses the wrapper's output counter; keep it
                    // current for backends that read it (Ring/Flat resume +
                    // dict gate). Const-folded away for UserSliceBackend.
                    if r.is_ok() && B::INLINE_EXEC_MAINTAINS_OUTPUT_COUNTER {
                        $buffer.advance_output_counter((seq_ll_v + seq_ml_v) as u64);
                    }
                    break 'exec_inner r.map_err(DecompressBlockError::ExecuteSequencesError);
                }
            }

            // Cold fallback.
            if let Err(e) = $buffer.try_push(lits) {
                break 'exec_inner Err(ExecuteSequencesError::from(e).into());
            }
            match $buffer.repeat_lookahead_prefetched(
                $dict,
                resolved_offset_v as usize,
                seq_ml_v as usize,
            ) {
                Ok(()) => Ok(()),
                Err(e) => Err(ExecuteSequencesError::from(e).into()),
            }
        };
        _result
    }};
}

/// AVX2-tier monolithic decode + execute. Outer init, RLE dispatch, FSE
/// state init, both pipeline arms, sequence-decode (via
/// `decode_one_body!`) and sequence-execute (via `execute_one_body!`)
/// all live in one function body. Macros guarantee textual expansion
/// at every callsite — no inner function boundaries.
///
/// # Safety
/// Caller must have verified that the runtime CPU advertises BMI2 + AVX2.
/// The dispatcher in `decode_and_execute_sequences` gates this on
/// `detect_cpu_kernel() == Avx2`.
#[target_feature(enable = "bmi2,avx2")]
#[allow(clippy::too_many_lines)]
pub(crate) unsafe fn decode_and_execute_sequences_avx2<'fse, B: BufferBackend>(
    section: &SequencesHeader,
    source: &[u8],
    fse: &'fse mut FSEScratch,
    buffer: &mut DecodeBuffer<B>,
    offset_hist: &mut [u32; 3],
    literals_buffer: &[u8],
    dict: Option<&'fse crate::decoding::dictionary::Dictionary>,
) -> Result<(), DecompressBlockError> {
    let SeqStreamSetup {
        mut br,
        mut ll_dec,
        mut ml_dec,
        mut of_dec,
        max_update_bits,
        old_buffer_size,
        num_sequences,
        use_long_pipeline,
    } = init_sequence_stream::<B, Avx2Kernel>(section, source, fse, buffer, dict)?;
    let literals_buffer_len = literals_buffer.len();
    let mut lit_cur: usize = 0;
    let mut seq_sum: u32 = 0;

    let buffer_checkpoint = buffer.checkpoint();
    let saved_offset_hist = *offset_hist;

    if use_long_pipeline {
        // === Long-pipeline arm (8-deep lookahead ring) ===
        let mut prefetch_pos: usize = old_buffer_size;
        let mut shadow_hist: [u32; 3] = *offset_hist;
        let mut ring: [ExecSeq; ADVANCE] = [ExecSeq {
            ll: 0,
            ml: 0,
            actual_offset: 0,
        }; ADVANCE];

        // Prefill ring with ADVANCE decoded+prefetched sequences.
        for slot in ring.iter_mut() {
            let seq = decode_one_body!(&mut ll_dec, &mut ml_dec, &mut of_dec, &mut br);
            let actual_offset = do_offset_history(seq.of, seq.ll, &mut shadow_hist);
            let match_start = prefetch_pos.wrapping_add(seq.ll as usize);
            let source_idx = match_start.wrapping_sub(actual_offset as usize);
            buffer.prefetch_lookahead_match_source(source_idx);
            prefetch_pos = match_start.wrapping_add(seq.ml as usize);
            *slot = ExecSeq {
                ll: seq.ll,
                ml: seq.ml,
                actual_offset,
            };
            br.ensure_bits(max_update_bits);
            ll_dec.update_state_fast(&mut br);
            ml_dec.update_state_fast(&mut br);
            of_dec.update_state_fast(&mut br);
        }

        // SAFETY: alignment-only asm, no memory or register clobbers.
        unsafe {
            core::arch::asm!(
                ".p2align 6",
                "nop",
                ".p2align 5",
                "nop",
                ".p2align 3",
                options(nomem, nostack, preserves_flags)
            );
        }

        let mut pipeline_err: Option<DecompressBlockError> = None;
        for i in ADVANCE..num_sequences {
            let seq = decode_one_body!(&mut ll_dec, &mut ml_dec, &mut of_dec, &mut br);
            let actual_offset = do_offset_history(seq.of, seq.ll, &mut shadow_hist);
            let match_start = prefetch_pos.wrapping_add(seq.ll as usize);
            let source_idx = match_start.wrapping_sub(actual_offset as usize);
            buffer.prefetch_lookahead_match_source(source_idx);
            prefetch_pos = match_start.wrapping_add(seq.ml as usize);

            let slot = i & ADVANCE_MASK;
            let exec_seq = ring[slot];
            ring[slot] = ExecSeq {
                ll: seq.ll,
                ml: seq.ml,
                actual_offset,
            };

            let r = execute_one_body!(
                buffer,
                dict,
                literals_buffer,
                &mut lit_cur,
                literals_buffer_len,
                exec_seq.ll,
                exec_seq.ml,
                exec_seq.actual_offset
            );
            if let Err(e) = r {
                pipeline_err = Some(e);
                break;
            }
            seq_sum = seq_sum.wrapping_add(exec_seq.ll).wrapping_add(exec_seq.ml);

            if i + 1 < num_sequences {
                br.ensure_bits(max_update_bits);
                ll_dec.update_state_fast(&mut br);
                ml_dec.update_state_fast(&mut br);
                of_dec.update_state_fast(&mut br);
            }
        }

        // Drain the remaining ADVANCE ring slots.
        if pipeline_err.is_none() {
            for k in 0..ADVANCE {
                let slot = (num_sequences + k) & ADVANCE_MASK;
                let exec_seq = ring[slot];
                let r = execute_one_body!(
                    buffer,
                    dict,
                    literals_buffer,
                    &mut lit_cur,
                    literals_buffer_len,
                    exec_seq.ll,
                    exec_seq.ml,
                    exec_seq.actual_offset
                );
                if let Err(e) = r {
                    pipeline_err = Some(e);
                    break;
                }
                seq_sum = seq_sum.wrapping_add(exec_seq.ll).wrapping_add(exec_seq.ml);
            }
        }

        if let Some(e) = pipeline_err {
            if buffer.try_restore_checkpoint(buffer_checkpoint) {
                *offset_hist = saved_offset_hist;
            }
            return Err(e);
        }
        *offset_hist = shadow_hist;
    } else {
        // === Short-block arm (straight single-pass fused loop) ===
        let mut shadow_hist = *offset_hist;
        let mut fallback_err: Option<DecompressBlockError> = None;
        for i in 0..num_sequences {
            let (seq_ll, seq_ml, resolved_offset) = decode_seq_fused_cshape!(
                &mut ll_dec,
                &mut ml_dec,
                &mut of_dec,
                &mut br,
                &mut shadow_hist
            );
            // Advance the FSE states for the NEXT sequence before executing the
            // current one, mirroring upstream `ZSTD_decodeSequence` (which
            // updates the states inside decode, then calls `ZSTD_execSequence`).
            // The execute reads no bits, so moving it after the state update is
            // byte-identical (value bits then state-transition bits are consumed
            // in the same order); it stops the three FSE states and the bit
            // reader from staying live across the heavy match copy, cutting
            // register pressure in the hot loop.
            if i + 1 < num_sequences {
                br.ensure_bits(max_update_bits);
                ll_dec.update_state_fast(&mut br);
                ml_dec.update_state_fast(&mut br);
                of_dec.update_state_fast(&mut br);
            }
            let r = execute_one_body!(
                buffer,
                dict,
                literals_buffer,
                &mut lit_cur,
                literals_buffer_len,
                seq_ll,
                seq_ml,
                resolved_offset
            );
            if let Err(e) = r {
                fallback_err = Some(e);
                break;
            }
            seq_sum = seq_sum.wrapping_add(seq_ll).wrapping_add(seq_ml);
        }
        if let Some(e) = fallback_err {
            let _ = buffer.try_restore_checkpoint(buffer_checkpoint);
            return Err(e);
        }
        *offset_hist = shadow_hist;
    }

    let remaining = br.bits_remaining();
    if remaining != 0 {
        if buffer.try_restore_checkpoint(buffer_checkpoint) {
            *offset_hist = saved_offset_hist;
        }
        if remaining < 0 {
            return Err(DecodeSequenceError::NotEnoughBytesForNumSequences.into());
        }
        return Err(DecodeSequenceError::ExtraBits {
            bits_remaining: remaining,
        }
        .into());
    }

    if lit_cur < literals_buffer_len {
        let rest = &literals_buffer[lit_cur..];
        buffer.try_push(rest).map_err(ExecuteSequencesError::from)?;
        seq_sum = seq_sum.wrapping_add(rest.len() as u32);
    }

    let diff = buffer.len() - old_buffer_size;
    debug_assert_eq!(
        seq_sum as usize, diff,
        "seq_sum {seq_sum} != buffer growth {diff}"
    );
    Ok(())
}
