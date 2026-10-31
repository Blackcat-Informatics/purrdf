#![allow(
    clippy::elidable_lifetime_names,
    clippy::empty_line_after_doc_comments,
    clippy::inline_always,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! User-slice-backed output buffer for the "decode straight into the
//! caller's output slice" fast path.
//!
//! Selected automatically by
//! [`crate::decoding::FrameDecoder::decode_all`] (and
//! [`crate::decoding::FrameDecoder::decode_all_to_vec`]) when ALL of
//! the following hold:
//! - `frame_content_size > 0` — the header-derived content size
//!   is non-zero. This is the actual eligibility condition (NOT
//!   "FCS present"): an empty frame with an explicit FCS=0
//!   declaration on the wire stays on the fallback path because
//!   there is no payload to write into the user slice. To
//!   distinguish "FCS absent" from "FCS=0 explicit" elsewhere in
//!   the decoder, use `FrameHeader::fcs_declared()` (e.g. the
//!   fallback path's post-decode size check does).
//! - `output.len() >= frame_content_size` — the slice holds the
//!   declared content. No `WILDCOPY_OVERLENGTH` slack is required:
//!   when a sequence's literal+match bytes fit but the SIMD wildcopy
//!   overshoot would not, the trailing sequence(s) take the bounded
//!   (non-overshooting) copy in [`UserSliceBackend::exec_sequence_bounded`].
//! - No active dictionary (the persistent dict_content is not
//!   carried into the stack-local DecodeBuffer this backend
//!   builds; dict frames stay on the regular path).
//!
//! `content_checksum_flag` is NOT a precondition: when set, the
//! direct-decode caller accumulates the frame XXH64 incrementally —
//! after each block it hashes that block's freshly-written bytes via
//! [`UserSliceBackend::written_since`] while they are still
//! cache-resident — and stores the final digest into the persistent
//! scratch's hasher so `get_calculated_checksum()` reads the right
//! value. (A single end-of-frame walk re-read the whole output cold:
//! one full extra memory pass on large frames.)
//!
//! Multi-segment frames work via the caller's per-block
//! `DecodeBuffer::drop_to_window_size` invocation — bytes drop
//! out of `len()`'s visible range once decoded output exceeds
//! `window_size`, but physically stay in the user's slice (this
//! backend's `drop_first_n` only advances `head`).
//!
//! The `drop_to_window_size` call runs only BETWEEN blocks, so
//! within a single block `len()` can temporarily exceed
//! `window_size`. `DecodeBuffer::repeat` validates match offsets
//! against `len()` (not `window_size`), so this is not a strict
//! enforcement of the spec's `offset <= window_size` rule — only
//! a coarse end-of-block cap. The fallback path
//! (`FlatBuf`/`RingBuffer`) shares the same limitation. Strict
//! in-block offset bounds would require additional validation
//! that neither path currently performs.
//!
//! When eligible, literal pushes and match-history copies write
//! directly into the user's slice. Compared to
//! `DecodeBuffer<FlatBuf>`, this elides one full `memmove` of the
//! live region (the `read` drain that copies the flat Vec into the
//! user slice) and one anonymous-page allocation cycle per frame.
//! On `level_-7_fast/decodecorpus-z000033/rust_stream` the
//! direct-write path measured -20.33% vs the FlatBuf+drain path on
//! i9-9900K — see #244 for the flamegraph.
//!
//! Selected at compile time via `DecodeBuffer<UserSliceBackend<'a>>`
//! (generic [`BufferBackend`](super::buffer_backend::BufferBackend)
//! parameter). The lifetime parameter binds the backend to the
//! user-provided slice — the backing
//! `DecodeBuffer<UserSliceBackend<'a>>` is stack-local in
//! `decode_all` and does not survive across calls. Persistent
//! decoder state (HUF/FSE tables, offset_hist, sequence cache)
//! lives in `FrameDecoder` and is borrowed in by reference for the
//! call's duration via [`super::scratch::DirectScratch`].

use crate::io::{Error, Read};

use super::buffer_backend::{BufferBackend, WILDCOPY_OVERLENGTH};

/// Backend that writes directly into a caller-provided `&mut [u8]`
/// output slice. No internal allocation, no drain copy.
///
/// Invariants enforced by the [`BufferBackend`] surface:
/// - `head <= tail <= slice.len()`.
/// - All bytes in `slice[head..tail]` are initialised (written by
///   [`Self::extend`] / [`Self::extend_and_fill`] /
///   [`Self::extend_from_within_unchecked`] /
///   [`Self::extend_from_reader`]).
/// - Bytes in `slice[tail..]` are initialised memory (the slice was
///   passed in as safe `&mut [u8]` so every element is a valid `u8`)
///   but hold contents the decoder has not yet written — they carry
///   whatever the caller put in the buffer before passing it in.
///   The FlatBuf precedent skips zero-pre-fill on `extend` for the
///   same reason: writes happen at the band `[tail, tail + n)` and
///   the rest stays unread by the decoder. Callers must not read
///   past `tail` and expect meaningful decode output.
///
/// The caller MUST size the output slice with at least
/// `frame_content_size` bytes. No `WILDCOPY_OVERLENGTH` slack is
/// required: when the SIMD wildcopy overshoot would run past the slice
/// end, the trailing sequence(s) fall back to the bounded
/// (non-overshooting) copy in [`Self::exec_sequence_bounded`]. The
/// dispatch site in [`crate::decoding::FrameDecoder`] validates this
/// precondition.
///
/// # Safety contract on malformed Compressed blocks
///
/// The safe public decode APIs ([`crate::decoding::FrameDecoder::decode_all`]
/// and [`crate::decoding::FrameDecoder::decode_all_to_vec`]) route
/// through the FALLIBLE write surface:
/// [`Self::try_extend`] / [`Self::try_extend_and_fill`] /
/// [`Self::try_extend_from_within`] for direct writes,
/// [`super::buffer_backend::BufferBackend::try_reserve`] for the
/// match-repeat pre-check inside `DecodeBuffer::repeat_inner`, and
/// `exec_sequence_inline` (which returns
/// `Result<(), ExecuteSequencesError>`). A malformed Compressed
/// block whose payload expands past the declared
/// `frame_content_size` is caught per-write as
/// `ExecuteSequencesError::OutputBufferOverflow` (literal-push /
/// upstream zstd-inline path) or `DecodeBufferError::OutputBufferOverflow`
/// (match-repeat path); `FrameDecoder::run_direct_decode` folds both
/// into a structured `FrameDecoderError::FrameContentSizeMismatch` —
/// the SAME error a Raw / RLE overshoot produces — instead of
/// panicking. The contract is uniform across all three block types:
/// "the frame's content exceeded its declared size".
///
/// The INFALLIBLE entry points (`extend`, `extend_and_fill`,
/// `extend_from_within_unchecked`) remain on the type as defense in
/// depth and as the call shape for inner unsafe blocks where
/// capacity has already been validated by the wrapping `try_*` call.
/// Each retains a release-mode `assert!` so a future caller that
/// invokes the infallible entry point directly with an OOB length
/// fails with a clear diagnostic rather than letting the subsequent
/// unsafe pointer math reach past `slice.len()`. The safe public
/// APIs never reach these `assert!`s on malformed input — the
/// fallible dispatch catches the overshoot one layer up.
pub(crate) struct UserSliceBackend<'a> {
    slice: &'a mut [u8],
    /// Bytes in `slice[..head]` have been drained to the output
    /// stream and are no longer visible through the [`BufferBackend`]
    /// surface. Same semantics as `FlatBuf.head` — see that field's
    /// doc for the "drained prefix remains physically present, used
    /// by future match copies" justification. For the
    /// single-segment direct-decode path `head` stays at 0 until the
    /// frame finishes (no streaming-drain), but the field is kept
    /// for API parity with `FlatBuf` and `RingBuffer`.
    head: usize,
    tail: usize,
}

impl<'a> UserSliceBackend<'a> {
    /// Construct a backend wrapping `slice`. The slice must hold at
    /// least `frame_content_size` bytes; the dispatcher in
    /// `FrameDecoder` enforces this. No `WILDCOPY_OVERLENGTH` trailing
    /// slack is required: when a sequence's literal+match bytes fit but
    /// the SIMD wildcopy overshoot would not, the inline exec paths fall
    /// back to [`Self::exec_sequence_bounded`] (exact, non-overshooting
    /// copies) for that trailing sequence.
    pub(crate) fn from_slice(slice: &'a mut [u8]) -> Self {
        Self {
            slice,
            head: 0,
            tail: 0,
        }
    }

    /// Physical bytes `slice[from..tail]` — the output written since a
    /// previously-observed [`BufferBackend::tail`]. The direct decode
    /// path hashes each block's output through this right after the
    /// block decodes, while the bytes are still cache-resident; a
    /// post-decode whole-output hash walk re-reads the entire frame
    /// cold (a full extra memory pass on large outputs). Only the
    /// XXH64 accumulation reads it, hence the `hash` gate.

    pub(crate) fn written_since(&self, from: usize) -> &[u8] {
        &self.slice[from..self.tail]
    }

    /// Exact, non-overshooting copy for the trailing sequence(s) when the
    /// remaining slice capacity is too tight for the SIMD wildcopy
    /// overshoot of the fast path. Lets the direct-decode gate require
    /// only `cap >= frame_content_size` instead of
    /// `cap >= frame_content_size + WILDCOPY_OVERLENGTH`, halving the
    /// peak allocation on the universal decode-into-FCS-sized-buffer
    /// case (caller no longer needs trailing slack).
    ///
    /// Portable on purpose: this is the slow tail path (a handful of
    /// sequences per frame at most), so the byte-exact copies cost
    /// nothing measurable while keeping one implementation across all
    /// ISA arms.
    ///
    /// # Safety
    /// - `self.tail + lit_length + match_length <= self.slice.len()`
    ///   (caller asserts exact fit before dispatching here).
    /// - `offset >= 1` and `offset <= (self.tail - self.head) +
    ///   lit_length` (upstream zstd's `oLitEnd - offset` precondition), so the
    ///   match source stays inside the already-written window.
    /// - `lit_src` is valid for `lit_length` bytes (this path reads
    ///   exactly `lit_length`, no 16-byte overshoot, so the parent
    ///   literals buffer always covers it).
    #[inline]
    unsafe fn exec_sequence_bounded(
        &mut self,
        lit_src: *const u8,
        lit_length: usize,
        offset: usize,
        match_length: usize,
    ) {
        // SAFETY: forwarded to the shared bounded-copy helper; the
        // doc-comment preconditions match its contract one-for-one
        // (`base[tail..]` writable for `lit+match`, `lit_src` readable for
        // `lit_length`, `offset` inside the written region).
        unsafe {
            super::exec_sequence_inline::exec_sequence_bounded_copy(
                self.slice.as_mut_ptr(),
                self.tail,
                lit_src,
                lit_length,
                offset,
                match_length,
            );
        }
        self.tail += lit_length + match_length;
    }
}

impl<'a> BufferBackend for UserSliceBackend<'a> {
    /// Upstream zstd-shape inline `ZSTD_execSequence` on every target: x86_64
    /// via the SSE2 [`super::exec_sequence_inline::x86`] module
    /// (`_mm_loadu_si128` / `_mm_storeu_si128`, SSE2 is the x86_64
    /// baseline so no `#[target_feature]` gate), all other ISAs via the
    /// architecture-agnostic [`super::exec_sequence_inline::portable`]
    /// module (unaligned u128/u64 moves lowered to NEON `ldr q`/`str q`
    /// on aarch64 and the widest available store elsewhere). Both arms
    /// are gated on this const, unconditionally `true` because an
    /// override exists for every target. Previously non-x86 fell back to
    /// the slow `extend` + `repeat` chain.
    const SUPPORTS_INLINE_SEQUENCE_EXEC: bool = true;

    /// Direct path reads `tail` for its output count and never consults
    /// `total_output_counter`, so the inline path skips the per-sequence
    /// counter RMW here (preserves the ~9% it costs on the all-inline hot
    /// path). See the trait const's doc.
    const INLINE_EXEC_MAINTAINS_OUTPUT_COUNTER: bool = false;

    /// Upstream zstd `ZSTD_execSequence` body — see trait doc for
    /// preconditions / contract.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    unsafe fn exec_sequence_inline(
        &mut self,
        lit_src: *const u8,
        lit_length: usize,
        offset: usize,
        match_length: usize,
    ) -> Result<(), super::errors::ExecuteSequencesError> {
        use super::buffer_backend::sequence_output_fits;
        use super::exec_sequence_inline::x86::{
            copy16, overlap_copy8, wildcopy_no_overlap, wildcopy_overlap_8byte_stride,
        };
        // Fallible capacity check for the literal+match copies, with
        // `overshoot = 0` — the SIMD wildcopy's up-to-15-byte tail overshoot
        // is NOT absorbed by caller-side slice slack (the slice carries none);
        // it is handled by the tight-tail bounded branch below. On a malformed
        // frame whose sequences overproduce past `frame_content_size`, the
        // check returns `ExecuteSequencesError::OutputBufferOverflow` so the
        // safe public decode APIs (`decode_all`, `decode_all_to_vec`)
        // surface a structured `FrameDecoderError` rather than
        // panic on the unsafe write surface.
        //
        // All sums use `checked_*` so adversarial input that would
        // wrap `usize` produces the same error variant instead of
        // wrapping past the slice length and letting the subsequent
        // unsafe pointer math go out of bounds.
        const MAX_WILDCOPY_OVERSHOOT: usize = 15;
        let cap = self.slice.len();
        // `self.tail <= cap` holds on entry (`from_slice` starts at 0 and
        // every prior sequence advanced `tail` only after this same check),
        // satisfying the `tail <= cap` precondition; see `sequence_output_fits`.
        // Hard guard with `overshoot = 0`: errors only on a genuine
        // exact-fit overflow. The <=15-byte SIMD wildcopy slack is
        // handled by the tight-tail branch below, so the caller slice
        // no longer needs `WILDCOPY_OVERLENGTH` trailing slack for the
        // direct-decode path — `cap >= frame_content_size` suffices.
        let total = sequence_output_fits(lit_length, match_length, self.tail, cap, 0)?;
        let new_tail = self.tail + total;
        debug_assert!(offset >= 1);
        debug_assert!(match_length >= 1);
        // Match against the LIVE window (tail - head) per the trait
        // contract, not `tail`. On single-segment frames head stays
        // at 0 so the two are equivalent; on multi-segment frames
        // `drop_to_window_size` advances `head` and asserting
        // against raw `tail` would mask offsets that reach past the
        // window boundary into dropped history.
        let live_len = self.tail - self.head;
        debug_assert!(
            live_len
                .checked_add(lit_length)
                .is_some_and(|end| offset <= end),
            "exec_sequence_inline: offset ({}) exceeds live window (len={} + lit={}, head={}, tail={})",
            offset,
            live_len,
            lit_length,
            self.head,
            self.tail,
        );

        // Tight-tail: the literal+match bytes fit exactly but the
        // <=15-byte wildcopy overshoot of the fast path would write
        // past `cap`. Take the exact (non-overshooting) copy so the
        // final sequence(s) near the buffer end stay in-bounds. Only
        // the trailing sequence(s) of a tightly-sized output slice hit
        // this; the bulk of the frame still takes the SIMD fast path.
        if total + MAX_WILDCOPY_OVERSHOOT > cap - self.tail {
            // SAFETY: `total <= cap - tail` (guard above), so the exact
            // copies stay in-bounds; `offset <= tail + lit_length`
            // keeps the match source valid; `lit_src` reads exactly
            // `lit_length` bytes (no 16-byte overshoot), strictly
            // within the parent literals buffer.
            unsafe { self.exec_sequence_bounded(lit_src, lit_length, offset, match_length) };
            return Ok(());
        }

        // SAFETY: capacity asserted above; pointer arithmetic stays
        // within `self.slice` for the writes (tail + total + overshoot
        // <= slice.len()) and reads (match src = tail + lit_length -
        // offset, bounded by `offset <= tail + lit_length`). Literal
        // reads use the caller-provided `lit_src` whose provenance
        // covers the parent literals buffer (NOT a sub-slice), so
        // the unconditional 16-byte load stays in-bounds even when
        // `lit_length < 16`.
        unsafe {
            let base_mut = self.slice.as_mut_ptr();

            // ── Literal copy ──
            // Upstream zstd: ZSTD_copy16(op, *litPtr); if (litLength > 16)
            //         wildcopy(op+16, *litPtr+16, litLength-16, no_overlap)
            //
            // The unconditional first copy16 may overshoot up to 15
            // bytes past `op + lit_length` into the match-destination
            // region — those bytes are about to be overwritten by the
            // match copy below, so the overshoot is harmless.
            let op_lit = base_mut.add(self.tail);
            copy16(op_lit, lit_src);
            if lit_length > 16 {
                wildcopy_no_overlap(op_lit.add(16), lit_src.add(16), lit_length - 16);
            }

            // ── Match copy ──
            // Upstream zstd uses `oLitEnd = op + litLength` as the match
            // destination; the match source is `oLitEnd - offset`.
            let op_match = base_mut.add(self.tail + lit_length);
            let match_src = base_mut.cast_const().add(self.tail + lit_length - offset);

            if offset >= 16 {
                wildcopy_no_overlap(op_match, match_src, match_length);
            } else {
                // overlap_copy8 + wildcopy(overlap) tail.
                let (op2, ip2) = overlap_copy8(op_match, match_src, offset);
                if match_length > 8 {
                    wildcopy_overlap_8byte_stride(op2, ip2, match_length - 8);
                }
            }
        }
        self.tail = new_tail;
        Ok(())
    }

    /// Non-x86 port of [`Self::exec_sequence_inline`] — identical upstream zstd
    /// `ZSTD_execSequence` shape through the portable wildcopy helpers
    /// (unaligned u128/u64 moves; NEON `ldr q`/`str q` on aarch64).
    /// Mirrors the x86 arm's capacity / offset safety checks exactly.
    #[cfg(not(target_arch = "x86_64"))]
    #[inline(always)]
    unsafe fn exec_sequence_inline(
        &mut self,
        lit_src: *const u8,
        lit_length: usize,
        offset: usize,
        match_length: usize,
    ) -> Result<(), super::errors::ExecuteSequencesError> {
        use super::buffer_backend::sequence_output_fits;
        use super::exec_sequence_inline::portable::{
            copy16, overlap_copy8, wildcopy_no_overlap, wildcopy_overlap_8byte_stride,
        };
        const MAX_WILDCOPY_OVERSHOOT: usize = 15;
        let cap = self.slice.len();
        // `self.tail <= cap` precondition holds as in the SSE2 arm; see
        // `sequence_output_fits`. Hard guard with `overshoot = 0`; the
        // <=15-byte wildcopy slack is handled by the tight-tail branch
        // below (see the x86 arm for the rationale).
        let total = sequence_output_fits(lit_length, match_length, self.tail, cap, 0)?;
        let new_tail = self.tail + total;
        debug_assert!(offset >= 1);
        debug_assert!(match_length >= 1);
        let live_len = self.tail - self.head;
        debug_assert!(
            live_len
                .checked_add(lit_length)
                .is_some_and(|end| offset <= end),
            "exec_sequence_inline: offset ({}) exceeds live window (len={} + lit={}, head={}, tail={})",
            offset,
            live_len,
            lit_length,
            self.head,
            self.tail,
        );

        // Tight-tail bounded copy; see the x86 arm for the rationale.
        if total + MAX_WILDCOPY_OVERSHOOT > cap - self.tail {
            // SAFETY: `total <= cap - tail` (guard above) bounds the
            // exact copies; `offset <= tail + lit_length` keeps the
            // match source valid; `lit_src` reads exactly `lit_length`
            // bytes within the parent literals buffer.
            unsafe { self.exec_sequence_bounded(lit_src, lit_length, offset, match_length) };
            return Ok(());
        }

        // SAFETY: identical invariants to the x86 arm — the capacity
        // check bounds every write (plus the ≤ 15-byte wildcopy
        // overshoot) inside `self.slice`; `offset <= tail + lit_length`
        // keeps the match source in-bounds; `lit_src` carries the parent
        // literals buffer's provenance (dispatch-site `inline_path_safe`
        // gate guarantees the unconditional 16-byte literal read stays
        // valid).
        unsafe {
            let base = self.slice.as_mut_ptr();
            let op_lit = base.add(self.tail);
            copy16(op_lit, lit_src);
            if lit_length > 16 {
                wildcopy_no_overlap(op_lit.add(16), lit_src.add(16), lit_length - 16);
            }

            let op_match = base.add(self.tail + lit_length);
            let match_src = base.cast_const().add(self.tail + lit_length - offset);

            if offset >= 16 {
                wildcopy_no_overlap(op_match, match_src, match_length);
            } else {
                let (op2, ip2) = overlap_copy8(op_match, match_src, offset);
                if match_length > 8 {
                    wildcopy_overlap_8byte_stride(op2, ip2, match_length - 8);
                }
            }
        }
        self.tail = new_tail;
        Ok(())
    }

    /// AVX2-tier override of [`BufferBackend::exec_sequence_inline_avx2`].
    /// Issue #279 round 3 Phase 4: divergent only at the
    /// match-copy no-overlap path with `offset >= 32`, where the
    /// SSE2 16-byte `wildcopy_no_overlap` is replaced by the 32-byte
    /// ymm `wildcopy_no_overlap_avx2`. The mid-offset range `16..=31`
    /// stays on the SSE2 16-byte path (a 32-byte ymm load at
    /// `offset` < 32 would read uninitialised destination bytes
    /// inside the match-copy region BEFORE the first store writes
    /// them — see the threshold comment in the body). Literal copy +
    /// short-offset (`offset < 16`) match path also stay on the
    /// SSE2 helpers — they're capped by the caller-side
    /// `inline_path_safe` gate at 16-byte slack and changing them
    /// would require re-tightening that gate. The 31-byte AVX2 stride
    /// overshoot is contained by the per-sequence `overshoot = 0`
    /// capacity guard plus the tight-tail bounded copy (the slice
    /// carries no trailing slack), not by slice-tail padding.
    ///
    /// # Safety
    /// Same preconditions as [`Self::exec_sequence_inline`] plus
    /// caller in `#[target_feature(enable = "avx2,bmi2")]` scope —
    /// the only call site is
    /// `sequence_section_decoder::execute_one_sequence_pipelined_avx2`
    /// which is itself target_feature-tagged.
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    #[inline]
    unsafe fn exec_sequence_inline_avx2(
        &mut self,
        lit_src: *const u8,
        lit_length: usize,
        offset: usize,
        match_length: usize,
    ) -> Result<(), super::errors::ExecuteSequencesError> {
        use super::buffer_backend::sequence_output_fits;
        use super::exec_sequence_inline::x86::{
            copy16, overlap_copy8, wildcopy_no_overlap, wildcopy_no_overlap_avx2,
            wildcopy_overlap_8byte_stride,
        };
        // 31-byte tail overshoot bound: the AVX2 no-overlap match wildcopy may
        // write up to 31 bytes past `tail + total`. The slice carries NO
        // trailing slack (`from_slice` takes the caller's exact buffer), so
        // this overshoot is handled by the tight-tail bounded branch below
        // rather than absorbed by slice capacity.
        const MAX_WILDCOPY_OVERSHOOT: usize = 31;
        let cap = self.slice.len();
        // `self.tail <= cap` holds on entry (`from_slice` starts at 0 and every
        // prior sequence advanced `tail` only after this same check), satisfying
        // the `tail <= cap` precondition; see `sequence_output_fits`. Hard guard
        // with `overshoot = 0`; the <=31-byte AVX2 wildcopy slack is handled by
        // the tight-tail branch below (see the SSE2 arm for the rationale).
        let total = sequence_output_fits(lit_length, match_length, self.tail, cap, 0)?;
        let new_tail = self.tail + total;
        debug_assert!(offset >= 1);
        debug_assert!(match_length >= 1);
        let live_len = self.tail - self.head;
        debug_assert!(
            live_len
                .checked_add(lit_length)
                .is_some_and(|end| offset <= end),
            "exec_sequence_inline_avx2: offset ({}) exceeds live window (len={} + lit={}, head={}, tail={})",
            offset,
            live_len,
            lit_length,
            self.head,
            self.tail,
        );

        // Tight-tail bounded copy; see the SSE2 arm for the rationale.
        if total + MAX_WILDCOPY_OVERSHOOT > cap - self.tail {
            // SAFETY: `total <= cap - tail` (guard above) bounds the
            // exact copies; `offset <= tail + lit_length` keeps the
            // match source valid; `lit_src` reads exactly `lit_length`
            // bytes within the parent literals buffer.
            unsafe { self.exec_sequence_bounded(lit_src, lit_length, offset, match_length) };
            return Ok(());
        }

        unsafe {
            let base_mut = self.slice.as_mut_ptr();

            // Literal copy — UNCHANGED from SSE2 default. 16-byte
            // copy16 + optional 16-byte-stride wildcopy. Upstream zstd's
            // literal copy is the smaller portion (~24 bytes typical);
            // the AVX2 win lives entirely in the match copy below.
            let op_lit = base_mut.add(self.tail);
            copy16(op_lit, lit_src);
            if lit_length > 16 {
                wildcopy_no_overlap(op_lit.add(16), lit_src.add(16), lit_length - 16);
            }

            // Match copy — divergent on no-overlap fast path.
            let op_match = base_mut.add(self.tail + lit_length);
            let match_src = base_mut.cast_const().add(self.tail + lit_length - offset);

            if offset >= 32 {
                // ✨ AVX2 32-byte stride no-overlap. Each ymm store is
                // 2× the SSE2 throughput; for typical match lengths
                // (32-256 bytes) this halves the iteration count of
                // the wildcopy loop. The 31-byte overshoot is bounded
                // by `MAX_WILDCOPY_OVERSHOOT` above.
                //
                // **Threshold `offset >= 32` (not 16) — correctness bound.**
                // For `offset ∈ 16..=31`, a 32-byte ymm load from
                // `match_src = dst - offset` would read bytes
                // `dst..=dst + (32 - offset - 1)` which are part of
                // the destination region BEFORE the first store has
                // written them. That reads uninitialised destination
                // bytes (or the previous block's tail) and silently
                // corrupts the output. The upstream zstd SSE2 16-byte wildcopy
                // is safe at `offset == 16` because the load reads
                // exactly through `dst - 1`; the AVX2 ymm stride needs
                // an extra 16-byte margin to maintain the same property.
                wildcopy_no_overlap_avx2(op_match, match_src, match_length);
            } else if offset >= 16 {
                // Mid-offset range (16..=31): too small for safe AVX2
                // 32-byte stride (see above), large enough for upstream zstd's
                // SSE2 16-byte no-overlap wildcopy. Keep the
                // SSE2 path for these offsets.
                wildcopy_no_overlap(op_match, match_src, match_length);
            } else {
                // Short-offset path unchanged: overlap_copy8 +
                // 8-byte stride wildcopy. 32-byte SIMD doesn't fit
                // the upstream zstd's overlap_src_before_dst semantics
                // because the 8-byte spread step needs the source
                // to lag the destination by less than the stride
                // width.
                let (op2, ip2) = overlap_copy8(op_match, match_src, offset);
                if match_length > 8 {
                    wildcopy_overlap_8byte_stride(op2, ip2, match_length - 8);
                }
            }
        }
        self.tail = new_tail;
        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    unsafe fn inline_exec_base_ptr(&mut self) -> *mut u8 {
        self.slice.as_mut_ptr()
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    unsafe fn inline_exec_commit(&mut self, new_tail: usize) {
        self.tail = new_tail;
    }

    /// `new()` exists for trait conformance but is not used on the
    /// direct-decode path — the slice is always provided up-front via
    /// [`Self::from_slice`]. Returns an empty backend wrapping an
    /// empty static slice; any subsequent `extend` call will panic
    /// via the capacity check.
    ///
    /// `&mut []` is a zero-length placeholder slice the compiler
    /// emits with `'static` lifetime; assigning it into the
    /// `slice: &'a mut [u8]` field compiles for any `'a` because
    /// `'static` outlives every other lifetime. No aliasing concern
    /// because the length is 0 (no addressable bytes the field
    /// could alias against). No raw-pointer + PhantomData workaround
    /// needed — verified by `cargo check` + the full nextest suite.
    /// This placeholder shape is fine ONLY because `new()` is never
    /// the entry point on the direct-decode path; non-empty
    /// constructions go through `from_slice` with a real `&'a mut [u8]`.
    fn new() -> Self {
        Self {
            slice: &mut [],
            head: 0,
            tail: 0,
        }
    }

    #[inline]
    fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
    }

    #[inline(always)]
    fn try_reserve(&mut self, n: usize) -> Result<(), super::buffer_backend::BackendOverflow> {
        // Fixed-capacity backend: linear `tail + n <= slice.len()`
        // check. Lets safe public decode APIs catch a malformed-frame
        // overshoot here instead of via the `assert!` inside
        // `extend_from_within_unchecked` further down the call chain.
        match self.tail.checked_add(n) {
            Some(new_tail) if new_tail <= self.slice.len() => Ok(()),
            _ => Err(super::buffer_backend::BackendOverflow {
                tail: self.tail,
                requested: n,
                capacity: self.slice.len(),
            }),
        }
    }

    #[inline]
    fn reserve(&mut self, _n: usize) {
        // No-op: capacity is fixed at construction (slice length).
        // The decoder's sequence-execution path issues
        // `buffer.reserve(MAX_BLOCK_SIZE)` upfront as a precaution
        // for FlatBuf's growable Vec; for UserSliceBackend we can
        // never satisfy that precaution because the slice can't
        // grow. The actual write-site debug_asserts in `extend` /
        // `extend_and_fill` / `extend_from_within_unchecked` /
        // `extend_from_reader` catch the real (much smaller)
        // capacity bound — `match_length` and per-block writes are
        // bounded by the well-formed-frame contract such that
        // `tail + write_size <= frame_content_size`.
    }

    #[inline]
    fn len(&self) -> usize {
        self.tail - self.head
    }

    #[inline]
    fn cap(&self) -> usize {
        self.slice.len()
    }

    #[inline]
    fn tail(&self) -> usize {
        self.tail
    }

    #[inline]
    unsafe fn set_tail(&mut self, new_tail: usize) {
        debug_assert!(new_tail >= self.head);
        debug_assert!(new_tail <= self.slice.len());
        self.tail = new_tail;
    }

    // `#[inline(always)]` because perf annotate on the primary bench
    // attributes ~10% of decode time to this method's own prologue /
    // epilogue (subq+pushq on entry, popq+retq on exit) — pure
    // function-call boundary cost on a method that is called from
    // tight literal-emit loops inside `decode_and_execute_sequences`.
    // The body is small (one assert, one copy), so forced inlining
    // does not bloat callers meaningfully. NOT a dispatch + match
    // pattern (the documented Tier-10 negative): the entry has no
    // runtime branch on kernel features — `copy_bytes_overshooting`
    // owns that dispatch internally.
    #[inline(always)]
    fn extend(&mut self, data: &[u8]) {
        let len = data.len();
        let new_tail = self.tail + len;
        // Release-mode capacity assert (mirrors
        // extend_from_within_unchecked). The body issues an unsafe
        // SIMD copy that takes `total_writable` as its
        // upper-bound contract; a malformed Compressed block whose
        // literals expand past the declared frame_content_size
        // would otherwise pass through `debug_assert!` in release
        // builds and turn the unchecked copy into UB. Cost: one
        // compare on the literal-push path — same magnitude as
        // the surrounding bounds-already-baked-in writes.
        //
        // This is the INFALLIBLE entry point. The safe public APIs
        // (`decode_all`, `decode_all_to_vec`) never reach it on
        // malformed input: their dispatch routes through
        // [`Self::try_extend`] (and via `DecodeBuffer::try_push` /
        // `try_reserve` for the match-repeat path), which return
        // `BackendOverflow` and convert into
        // `FrameDecoderError::ExecuteSequencesError` /
        // `FrameContentSizeMismatch`. The `assert!` here covers
        // the case where a future caller wires the infallible
        // entry point into a hot path that doesn't go through the
        // dispatch — the panic message points at the corrupt-frame
        // root cause rather than letting the subsequent unsafe
        // pointer math go OOB.
        assert!(
            new_tail <= self.slice.len(),
            "UserSliceBackend::extend overflows slice (tail+={}, cap={}) — corrupt frame",
            len,
            self.slice.len()
        );
        // Literal pushes (the sequence executor calls this for the
        // literal portion of every sequence) frequently land in the
        // 1..=16 byte range on highly-compressed corpora — exactly
        // the range where libc memmove dispatch overhead dominates
        // the cost of the actual copy. Route through
        // `copy_bytes_overshooting` so short pushes inline a single
        // SIMD / overlapping-u64 store instead.
        let total_writable = self.slice.len() - self.tail;
        // SAFETY: caller-provided `data` is non-aliasing with the
        // backend's slice (it's the literals buffer or input view).
        // `new_tail <= self.slice.len()` (release-mode `assert!`
        // above), so both regions have ≥ `len` valid bytes.
        unsafe {
            super::simd_copy::copy_bytes_overshooting(
                (data.as_ptr(), len),
                (self.slice.as_mut_ptr().add(self.tail), total_writable),
                len,
            );
        }
        self.tail = new_tail;
    }

    #[inline]
    fn extend_and_fill(&mut self, fill_with: u8, fill_length: usize) {
        let new_tail = self.tail + fill_length;
        // Release-mode `assert!` (not `debug_assert!`) for symmetry
        // with `extend` / `extend_from_within_unchecked`. Without it,
        // a malformed Compressed block whose RLE fill expands past
        // the declared `frame_content_size` would panic via the
        // subsequent `self.slice[self.tail..new_tail]` slice index
        // with a less-informative message, AND the in-block writes
        // would already have happened up to the slice length. Fail
        // fast with a clear corruption / capacity diagnostic.
        assert!(
            new_tail <= self.slice.len(),
            "UserSliceBackend::extend_and_fill overflows slice (tail+={}, cap={}) — corrupt frame",
            fill_length,
            self.slice.len()
        );
        // `slice::fill` lowers to a memset on byte slices; the
        // per-byte loop above the rebased commit replaces it with
        // an explicit assignment, which the optimiser does not
        // always promote back. For large RLE blocks the memset
        // path wins ~3-5x on x86_64.
        self.slice[self.tail..new_tail].fill(fill_with);
        self.tail = new_tail;
    }

    fn extend_from_reader<R: Read>(
        &mut self,
        mut read: R,
        fill_length: usize,
    ) -> Result<(), Error> {
        let old = self.tail;
        let new_tail = old + fill_length;
        if new_tail > self.slice.len() {
            return Err(Error::other(
                "UserSliceBackend: raw block exceeds caller-provided output capacity",
            ));
        }
        match read.read_exact(&mut self.slice[old..new_tail]) {
            Ok(()) => {
                self.tail = new_tail;
                Ok(())
            }
            // Don't advance `tail` on failure — the upper bound from
            // the slice borrow above guarantees the `read_exact`
            // attempt didn't write past `new_tail`, but we MUST keep
            // `tail` pointing at the last fully-decoded byte so
            // checkpoint rollback / drain semantics line up with
            // FlatBuf's truncate-on-error shape.
            Err(e) => Err(e),
        }
    }

    // Keep `#[inline]` (hint, not force). An earlier experiment with
    // `#[inline(always)]` regressed primary bench by +2.96% — body
    // is materially larger than `extend` (assert + readable/writable
    // derivation + simd_copy::copy_bytes_overshooting call), and
    // forced inlining bloats each pipeline-slot caller past icache
    // budget. The per-call boundary save is dwarfed by the
    // duplicated body weight; LLVM's heuristic was right to decline.
    #[inline]
    unsafe fn extend_from_within_unchecked(&mut self, start: usize, len: usize) {
        let dst_off = self.tail;
        let src_off = self.head + start;
        debug_assert!(src_off + len <= dst_off);
        // Release-mode capacity check: the trait contract says
        // capacity for `len` bytes past the tail was reserved by
        // the caller, but UserSliceBackend's reserve is a no-op
        // (slice can't grow), so a malformed frame with an out-of-
        // bounds match could otherwise turn the unchecked
        // SIMD copy into UB in release builds. FlatBuf relies on
        // `Vec::reserve`; we have no allocator to call so the
        // check has to be inline. Cost: one compare on a path
        // that's already memory-bound.
        assert!(
            dst_off + len <= self.slice.len(),
            "UserSliceBackend: match write past slice capacity (corrupt frame)"
        );
        // Route the match copy through `simd_copy::copy_bytes_overshooting`
        // rather than `ptr::copy_nonoverlapping`. The latter goes to
        // libc `__memmove_avx_unaligned_erms` on x86_64, which costs
        // ~10ns of dispatch overhead per call — measured at 40% of
        // decode CPU on the L-1 fast c_stream flamegraph because
        // L-1 produces thousands of short matches per frame.
        // `copy_bytes_overshooting` inlines a single SIMD
        // load+store for `len <= 16` with WILDCOPY_OVERLENGTH slack
        // (typical match case), and a byte / overlapping-u64
        // sequence for shorter copies without slack — both bypass
        // the libc memmove dispatch entirely.
        let total_readable = self.tail - src_off;
        let total_writable = self.slice.len() - dst_off;
        // SAFETY: caller's non-overlap precondition gives
        // `src_off + len <= dst_off` (so src/dst regions don't
        // overlap), `total_readable >= len` follows from
        // `src_off + len <= dst_off <= self.tail`, and
        // `total_writable >= len` by the assert above. The
        // helper writes ≤ `total_writable` bytes, so it stays
        // inside the slice even when it overshoots `len`.
        unsafe {
            let base = self.slice.as_mut_ptr();
            super::simd_copy::copy_bytes_overshooting(
                (base.add(src_off), total_readable),
                (base.add(dst_off), total_writable),
                len,
            );
        }
        self.tail = dst_off + len;
    }

    #[inline]
    unsafe fn extend_from_within_unchecked_branchless(&mut self, start: usize, len: usize) {
        // Direct-slice layout never wraps — same forward to the
        // single non-overlapping copy as FlatBuf.
        unsafe { self.extend_from_within_unchecked(start, len) }
    }

    #[inline]
    fn as_slices(&self) -> (&[u8], &[u8]) {
        (&self.slice[self.head..self.tail], &[])
    }

    #[inline]
    fn drop_first_n(&mut self, n: usize) {
        self.head += n;
        debug_assert!(self.head <= self.tail);
    }

    // ── Fallible write surface ──
    //
    // Override the default trait impls (which call panic-on-overflow
    // variants) with explicit capacity checks that return
    // `BackendOverflow` instead. This is the entire point of the
    // backend: a fixed-capacity output slice that cannot grow on
    // demand, so any overshoot must be reported instead of aborting.

    #[inline(always)]
    fn try_extend(&mut self, data: &[u8]) -> Result<(), super::buffer_backend::BackendOverflow> {
        let len = data.len();
        // Use `checked_add` to catch adversarial input where
        // `self.tail + len` would wrap `usize` — without the wrap
        // check, the subsequent `new_tail > self.slice.len()` test
        // can be bypassed by an `len` near `usize::MAX`, turning
        // the fallible write into a release-mode panic via
        // `copy_bytes_overshooting`.
        // BackendOverflow is `Copy` with three usize fields; eager
        // construction is cheaper than a closure indirection on this
        // hot path (clippy::unnecessary_lazy_evaluations).
        let new_tail =
            self.tail
                .checked_add(len)
                .ok_or(super::buffer_backend::BackendOverflow {
                    tail: self.tail,
                    requested: len,
                    capacity: self.slice.len(),
                })?;
        if new_tail > self.slice.len() {
            return Err(super::buffer_backend::BackendOverflow {
                tail: self.tail,
                requested: len,
                capacity: self.slice.len(),
            });
        }
        let total_writable = self.slice.len() - self.tail;
        // SAFETY: `new_tail <= self.slice.len()` (checked above);
        // `data` is non-aliasing with the backend's slice (caller
        // contract — literals buffer / input view).
        unsafe {
            super::simd_copy::copy_bytes_overshooting(
                (data.as_ptr(), len),
                (self.slice.as_mut_ptr().add(self.tail), total_writable),
                len,
            );
        }
        self.tail = new_tail;
        Ok(())
    }

    #[inline(always)]
    fn try_extend_and_fill(
        &mut self,
        fill_with: u8,
        fill_length: usize,
    ) -> Result<(), super::buffer_backend::BackendOverflow> {
        // Same wrap-check rationale as `try_extend` above — an
        // adversarial `fill_length` near `usize::MAX` would wrap
        // `new_tail`, bypass the upper bound, and panic in
        // `slice[tail..new_tail]` (start > end).
        let new_tail =
            self.tail
                .checked_add(fill_length)
                .ok_or(super::buffer_backend::BackendOverflow {
                    tail: self.tail,
                    requested: fill_length,
                    capacity: self.slice.len(),
                })?;
        if new_tail > self.slice.len() {
            return Err(super::buffer_backend::BackendOverflow {
                tail: self.tail,
                requested: fill_length,
                capacity: self.slice.len(),
            });
        }
        self.slice[self.tail..new_tail].fill(fill_with);
        self.tail = new_tail;
        Ok(())
    }

    #[inline(always)]
    fn try_extend_from_within(
        &mut self,
        start: usize,
        len: usize,
    ) -> Result<(), super::buffer_backend::BackendOverflow> {
        // Bound 1: source range fits in live region.
        // The caller's `start` is relative to the live-region head;
        // map to a physical absolute position and check against the
        // current tail. Both `head + start` and `+ len` get
        // `checked_add` so an adversarial `start` or `len` near
        // `usize::MAX` cannot wrap past the bounds checks.
        let abs_start =
            self.head
                .checked_add(start)
                .ok_or(super::buffer_backend::BackendOverflow {
                    tail: self.tail,
                    requested: len,
                    capacity: self.slice.len(),
                })?;
        let abs_end = abs_start
            .checked_add(len)
            .ok_or(super::buffer_backend::BackendOverflow {
                tail: self.tail,
                requested: len,
                capacity: self.slice.len(),
            })?;
        if abs_end > self.tail {
            return Err(super::buffer_backend::BackendOverflow {
                tail: self.tail,
                requested: len,
                capacity: self.slice.len(),
            });
        }
        // Bound 2: destination has capacity for `len`. Same wrap
        // protection — without it, an adversarial `len` near
        // `usize::MAX` wraps and bypasses the upper bound, turning
        // the unchecked write into a release-mode UB / panic.
        // BackendOverflow is `Copy` with three usize fields; eager
        // construction is cheaper than a closure indirection on this
        // hot path (clippy::unnecessary_lazy_evaluations).
        let new_tail =
            self.tail
                .checked_add(len)
                .ok_or(super::buffer_backend::BackendOverflow {
                    tail: self.tail,
                    requested: len,
                    capacity: self.slice.len(),
                })?;
        if new_tail > self.slice.len() {
            return Err(super::buffer_backend::BackendOverflow {
                tail: self.tail,
                requested: len,
                capacity: self.slice.len(),
            });
        }
        // SAFETY: both bounds checked above. Forward to the unsafe
        // variant which performs the wildcopy with the same
        // preconditions the bounds checks established.
        unsafe { self.extend_from_within_unchecked(start, len) };
        Ok(())
    }
}

// `WILDCOPY_OVERLENGTH` is used implicitly via the dispatcher's
// capacity sizing — kept imported here for the doc reference and to
// surface a build error if the constant moves.
const _: () = {
    let _: usize = WILDCOPY_OVERLENGTH;
};
