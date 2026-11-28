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
//! BMI2-tier monolithic sequence-section decoder.
//!
//! Same shape as the AVX2 monolith: `macro_rules!` blocks expand the
//! decode + execute bodies textually at every callsite inside one
//! `#[target_feature(enable = "bmi2")]` function. Match copy uses the
//! SSE2 16-byte `exec_sequence_inline` (no AVX2/VBMI2 widening).

#![cfg(target_arch = "x86_64")]

use super::buffer_backend::BufferBackend;
use super::decode_buffer::DecodeBuffer;
use super::exec_sequence_inline::exec_sequence_sse2_inline;
use super::scratch::FSEScratch;
use super::sequence_section_decoder::{
    ADVANCE, ADVANCE_MASK, ExecSeq, SeqStreamSetup, init_sequence_stream,
};
use crate::blocks::sequence_section::{MAX_OFFSET_CODE, Sequence, SequencesHeader};
use crate::cpu_kernel::Bmi2Kernel;
use crate::decoding::errors::{DecodeSequenceError, DecompressBlockError, ExecuteSequencesError};
use crate::decoding::sequence_execution::do_offset_history;

/// Textual decode-one body. PEXT-direct via `peek_bits_triple_bmi2`
/// when vendor cache enables it.
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
            let sum = sum_wide as u8;
            $br.ensure_bits(sum);
            // SAFETY: enclosing fn is target_feature(bmi2).
            let triple = if $br.use_pext_triple_fast() {
                unsafe { $br.peek_bits_triple_bmi2(sum, of_num_bits, ml_num_bits, ll_num_bits) }
            } else {
                $br.peek_bits_triple(sum, of_num_bits, ml_num_bits, ll_num_bits)
            };
            $br.consume(sum);
            triple
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

/// Textual execute-one body. SSE2 16-byte match copy via
/// `exec_sequence_inline`. Labeled-block early exits, no closure.
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
                    // Inline the SSE2 exec body at the call site (no trait-method
                    // call boundary; BMI2 tier has no AVX2, so the 16-byte xmm
                    // wildcopy variant is used — see `exec_sequence_sse2_inline`).
                    let r = exec_sequence_sse2_inline!(
                        $buffer,
                        lit_src,
                        seq_ll_v as usize,
                        offset,
                        seq_ml_v as usize
                    );
                    // Inline path bypasses the wrapper's output counter; keep it
                    // current for backends that read it (Ring/Flat). Const-folded
                    // away for UserSliceBackend.
                    if r.is_ok() && B::INLINE_EXEC_MAINTAINS_OUTPUT_COUNTER {
                        $buffer.advance_output_counter((seq_ll_v + seq_ml_v) as u64);
                    }
                    break 'exec_inner r.map_err(DecompressBlockError::ExecuteSequencesError);
                }
            }

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

/// BMI2-tier monolithic decode + execute.
///
/// # Safety
/// Caller must have verified BMI2 availability.
#[target_feature(enable = "bmi2")]
#[allow(clippy::too_many_lines)]
pub(crate) unsafe fn decode_and_execute_sequences_bmi2<'fse, B: BufferBackend>(
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
    } = init_sequence_stream::<B, Bmi2Kernel>(section, source, fse, buffer, dict)?;
    let literals_buffer_len = literals_buffer.len();
    let mut lit_cur: usize = 0;
    let mut seq_sum: u32 = 0;

    let buffer_checkpoint = buffer.checkpoint();
    let saved_offset_hist = *offset_hist;

    if use_long_pipeline {
        let mut prefetch_pos: usize = old_buffer_size;
        let mut shadow_hist: [u32; 3] = *offset_hist;
        let mut ring: [ExecSeq; ADVANCE] = [ExecSeq {
            ll: 0,
            ml: 0,
            actual_offset: 0,
        }; ADVANCE];

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

        // SAFETY: alignment-only asm.
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
        let mut shadow_hist = *offset_hist;
        let mut fallback_err: Option<DecompressBlockError> = None;
        for i in 0..num_sequences {
            let seq = decode_one_body!(&mut ll_dec, &mut ml_dec, &mut of_dec, &mut br);
            let resolved_offset = do_offset_history(seq.of, seq.ll, &mut shadow_hist);
            let r = execute_one_body!(
                buffer,
                dict,
                literals_buffer,
                &mut lit_cur,
                literals_buffer_len,
                seq.ll,
                seq.ml,
                resolved_offset
            );
            if let Err(e) = r {
                fallback_err = Some(e);
                break;
            }
            seq_sum = seq_sum.wrapping_add(seq.ll).wrapping_add(seq.ml);

            if i + 1 < num_sequences {
                br.ensure_bits(max_update_bits);
                ll_dec.update_state_fast(&mut br);
                ml_dec.update_state_fast(&mut br);
                of_dec.update_state_fast(&mut br);
            }
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
