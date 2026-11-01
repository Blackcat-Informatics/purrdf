#![allow(
    clippy::empty_line_after_doc_comments,
    clippy::inline_always,
    clippy::redundant_else,
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use crate::io::{Error, Read, Write};
use alloc::vec::Vec;

use core::hash::Hasher;

use super::buffer_backend::BufferBackend;
use super::prefetch;
use super::ringbuffer::RingBuffer;
use crate::decoding::errors::DecodeBufferError;

/// Generic decode-side output buffer parameterised over the storage
/// backend ([`BufferBackend`]). The default `RingBuffer` parameter
/// preserves the historical API for callers that don't want to opt
/// into the flat-buffer fast path.
///
/// Two concrete instantiations are used by the decoder:
/// - `DecodeBuffer<RingBuffer>` — wrap-aware ring (default; the
///   pre-existing decode path).
/// - `DecodeBuffer<FlatBuf>` — non-wrapping Vec-backed fast path,
///   selected by [`super::frame_decoder::FrameDecoder`] (via
///   `DecoderScratchKind`) when the frame's `Single_Segment_flag`
///   is set. The compiler emits a separate monomorphisation per
///   backend so wrap dispatch is eliminated entirely on the flat
///   side at compile time rather than branched at runtime.
pub(crate) struct DecodeBuffer<B: BufferBackend = RingBuffer> {
    buffer: B,
    // No dictionary reference is stored: an out-of-window match reads the
    // dictionary content through a call-scoped `dict: Option<&Dictionary>`
    // borrow threaded into `repeat` / `repeat_from_dict` by the decode pipeline
    // (libzstd `ZSTD_refDDict` semantics — read the dict content, no per-frame
    // refcount bump). The borrow checker guarantees the dictionary outlives
    // every read; `DecodeBuffer` itself stays `Send`/`Sync` by auto-derive.
    pub window_size: usize,
    total_output_counter: u64,

    pub hash: twox_hash::XxHash64,
    /// Whether drain hashes the bytes it emits into `hash`. `false` lets a
    /// `FrameDecoder` set to [`ContentChecksum::None`](crate::decoding::ContentChecksum::None)
    /// skip the XXH64 pass on the streaming path. Persists across `reset`
    /// (it mirrors a decoder-level setting, not per-frame state); the frame
    /// layer re-applies it before each decode.
    compute_hash: bool,
}

/// Rollback token produced by [`DecodeBuffer::checkpoint`].
///
/// Snapshots tail / counter / cap. Hash state is NOT snapshotted:
/// no mutation site between `checkpoint()` and the matched
/// `try_restore_checkpoint()` writes to `self.hash`:
///   * `push` and `extend_and_fill` only advance
///     `total_output_counter`.
///   * The inline sequence executor writes through `buffer_mut()`
///     directly, bypassing the wrapper-level
///     `total_output_counter` entirely (`UserSliceBackend::tail`
///     carries the byte count on that path; hashing is deferred to
///     the final full-slice pass in `FrameDecoder::decode_all`).
///   * `drain_to` / `read` DO write hash, but they run BETWEEN
///     blocks, never inside the fused sequence loop the checkpoint
///     guards.
#[derive(Copy, Clone)]
pub(crate) struct DecodeBufferCheckpoint {
    tail: usize,
    total_output_counter: u64,
    cap: usize,
}

impl<B: BufferBackend> Read for DecodeBuffer<B> {
    fn read(&mut self, target: &mut [u8]) -> Result<usize, Error> {
        let max_amount = self.can_drain_to_window_size().unwrap_or(0);
        let amount = max_amount.min(target.len());

        let mut written = 0;
        self.drain_to(amount, |buf| {
            target[written..][..buf.len()].copy_from_slice(buf);
            written += buf.len();
            (buf.len(), Ok(()))
        })?;
        Ok(amount)
    }
}

impl<B: BufferBackend> DecodeBuffer<B> {
    pub(crate) fn new(window_size: usize) -> DecodeBuffer<B> {
        DecodeBuffer {
            buffer: B::new(),
            window_size,
            total_output_counter: 0,

            hash: twox_hash::XxHash64::with_seed(0),

            compute_hash: true,
        }
    }

    /// Wrap a pre-constructed backend (e.g. `FlatBuf::with_capacity`
    /// sized for a single-segment frame) into a `DecodeBuffer`. Used
    /// by `FrameDecoder` (via `DecoderScratchKind::new_flat`) to
    /// supply a `FlatBuf` pre-sized for `frame_content_size` —
    /// the default `new()` constructor would otherwise produce a
    /// zero-capacity backend and force a realloc on the first push.
    ///
    /// Calls `buffer.clear()` so the logical counters (set to zero
    /// here) are not inconsistent with a physically-non-empty backend
    /// the caller might have handed in. On a fresh backend (the only
    /// real call shape today) `clear()` is a no-op — the two stores
    /// it issues vanish in the per-frame reset noise.
    pub(crate) fn from_backend(mut buffer: B, window_size: usize) -> DecodeBuffer<B> {
        buffer.clear();
        DecodeBuffer {
            buffer,
            window_size,
            total_output_counter: 0,

            hash: twox_hash::XxHash64::with_seed(0),

            compute_hash: true,
        }
    }

    /// Enable or disable the drain-time XXH64 pass. Set by the frame layer
    /// from the decoder's [`ContentChecksum`](crate::decoding::ContentChecksum)
    /// mode before each decode (`false` for `None`).

    #[inline]
    pub(crate) fn set_compute_hash(&mut self, compute: bool) {
        self.compute_hash = compute;
    }

    /// Arm the per-block decompressed-output ceiling for the block about to
    /// be decoded: the growable backend is told it may grow only up to
    /// `current_len + max_block_output`, so a match that would push this
    /// block's output past `MAX_BLOCK_SIZE` fails its `try_reserve` on the
    /// cold growth path instead of growing the ring to gigabytes (a
    /// decompression-bomb OOM) before the post-block validity check runs.
    /// Called once per block by the sequence decoder, before the sequence
    /// loop. The growable backends (`RingBuffer`, `FlatBuf`) enforce it in
    /// `try_reserve`; fixed-capacity backends (`UserSliceBackend`) are already
    /// bounded and take the trait's no-op default.
    #[inline]
    pub(crate) fn set_block_output_ceiling(&mut self, max_block_output: usize) {
        // Plain add (not saturating): `len()` is the bytes decoded so far and
        // `max_block_output` is one block's `MAX_BLOCK_SIZE` (128 KiB), so the
        // sum is nowhere near `usize::MAX` — overflow is unreachable. Saturating
        // would silently turn the ceiling into `usize::MAX` (no guard at all)
        // if that invariant were ever broken, masking the bug instead of
        // tripping the debug overflow check at its cause.
        let ceiling = self.buffer.len() + max_block_output;
        self.buffer.set_max_capacity(ceiling);
    }

    pub(crate) fn reset(&mut self, window_size: usize) {
        self.window_size = window_size;
        self.buffer.clear();
        // No reserve here: capacity decisions are pushed up to the frame
        // layer. Direct-decode frames (`run_direct_decode`) write through
        // `UserSliceBackend` and never touch this buffer, so a long-lived
        // `FrameDecoder` reused across direct-eligible frames pays zero
        // allocation for the window. The non-direct path is pre-reserved
        // by `FrameDecoder::decode_all_impl` / `decode_blocks` via
        // `DecoderScratchKind::reserve_buffer(window_size)` before any
        // block writes — that is the only call site that knows whether
        // the frame will actually hit this buffer.
        self.total_output_counter = 0;
        // Lift the per-block growth ceiling between frames; the sequence
        // decoder re-arms it per block. Non-block callers stay unbounded.
        self.buffer.set_max_capacity(usize::MAX);

        {
            self.hash = twox_hash::XxHash64::with_seed(0);
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Allocated byte capacity of the backing buffer (the decode window).
    /// Backs the workspace-footprint reporting; the value is the backend's
    /// `cap()` (RingBuffer's ring-indexing capacity / FlatBuf's `Vec`
    /// capacity), so it tracks the real heap reservation.
    pub(crate) fn capacity(&self) -> usize {
        self.buffer.cap()
    }

    /// Active dictionary content bytes, read from the call-scoped `dict` borrow
    /// (no copy). Empty slice when no dictionary is active for this decode.
    #[inline]
    pub(crate) fn dict_content<'d>(
        &self,
        dict: Option<&'d crate::decoding::dictionary::Dictionary>,
    ) -> &'d [u8] {
        match dict {
            Some(d) => &d.dict_content,
            None => &[],
        }
    }

    /// Capture a rollback point covering the buffer's write cursor and the
    /// total-output counter. Pair with [`restore_checkpoint`] to undo
    /// speculative pushes/repeats made after the capture — used by the fused
    /// sequence executor to roll back when the post-loop bitstream
    /// validation rejects a malformed block, restoring the
    /// transactional-on-error semantics the legacy two-pass pipeline had.
    #[inline]
    pub(crate) fn checkpoint(&self) -> DecodeBufferCheckpoint {
        DecodeBufferCheckpoint {
            tail: self.buffer.tail(),
            total_output_counter: self.total_output_counter,
            cap: self.buffer.cap(),
        }
    }

    /// Attempt to restore a checkpoint captured by [`checkpoint`].
    ///
    /// Returns `true` if the rollback was performed; `false` if an
    /// intervening reallocation invalidated the captured tail index
    /// (no state is mutated in that case).
    ///
    /// On a well-formed zstd block the upfront `reserve(MAX_BLOCK_SIZE)`
    /// rules out reallocation, so this returns `true` on the hot path.
    /// On a malformed block whose sequence section decodes past
    /// `MAX_BLOCK_SIZE`, `RingBuffer::reserve_amortized` compacts the
    /// buffer (head=0, tail=s1+s2) and the captured tail index becomes
    /// meaningless — `false` is returned and the caller surfaces a
    /// normal decode `Err` instead of restoring stale state. Reaching
    /// this arm implies the frame is already corrupt; the partial
    /// data left in the buffer is discarded by the `Err` return.
    #[inline]
    pub(crate) fn try_restore_checkpoint(&mut self, cp: DecodeBufferCheckpoint) -> bool {
        if self.buffer.cap() != cp.cap {
            return false;
        }
        // SAFETY: cap-equality above proves the underlying allocation
        // has not been reseated, so the captured `tail` still refers to
        // the same logical and physical position. The caller is also
        // responsible for treating any bytes between the captured tail
        // and the current tail as discarded.
        unsafe { self.buffer.set_tail(cp.tail) };
        self.total_output_counter = cp.total_output_counter;
        // No hash restore: see `DecodeBufferCheckpoint` doc. No
        // mutation site between `checkpoint()` and this call writes
        // to `self.hash` (drain runs between blocks, not inside the
        // fused sequence loop; the inline sequence executor bypasses
        // the wrapper counter entirely via `buffer_mut()`, leaving
        // hashing for the post-block full-slice pass).
        true
    }

    /// Pre-allocate capacity for `amount` additional bytes ahead of a batch
    /// of `push`/`repeat` operations, growing exactly (see
    /// `BufferBackend::reserve_exact`): every call site is a one-shot
    /// window-scale reservation where amortized doubling would hold up to
    /// 2x the window.
    #[inline]
    pub(crate) fn reserve_exact(&mut self, amount: usize) {
        self.buffer.reserve_exact(amount);
    }

    /// Mutable backend handle. Lets the inline sequence executor
    /// write straight into the backend's physical storage; the
    /// `tail()` cursor on the backend is the authoritative output
    /// length, so no separate buffer-level counter update is needed.
    /// Crate-internal; gated to the
    /// `BufferBackend::SUPPORTS_INLINE_SEQUENCE_EXEC = true` dispatch
    /// site.
    #[inline]
    #[allow(dead_code)]
    pub(crate) fn buffer_mut(&mut self) -> &mut B {
        &mut self.buffer
    }

    /// Immutable backend handle. `run_direct_decode`'s post-block FCS
    /// check reads `tail()` straight from the backend rather than
    /// going through `total_output_counter`: the inline sequence
    /// executor (see
    /// `sequence_section_decoder::execute_one_sequence_pipelined`)
    /// writes directly through `buffer_mut`, so the
    /// `total_output_counter` field on the wrapper is not maintained
    /// on that path and `tail()` is the only accurate output length.
    #[inline(always)]
    pub(crate) fn buffer_ref(&self) -> &B {
        &self.buffer
    }

    /// Fill `fill_length` bytes of the output with the literal `fill_with`,
    /// advancing the ringbuffer cursor in place. Used by the RLE block path
    /// (upstream commit `fbc1f2ca`) so the decoder doesn't need a stack
    /// scratch buffer to materialise repeated bytes before pushing them.
    /// Mirrors `push`'s `total_output_counter` bookkeeping so
    /// dictionary-repeat validation in `repeat_from_dict` stays accurate
    /// after RLE blocks.
    pub(crate) fn extend_and_fill(&mut self, fill_with: u8, fill_length: usize) {
        self.buffer.extend_and_fill(fill_with, fill_length);
        self.total_output_counter += fill_length as u64;
    }

    /// Read `fill_length` bytes from `read` directly into the ringbuffer's
    /// free slots. Used by the Raw block path (upstream commit `29a56160`)
    /// so the decoder doesn't need a 128 KB stack scratch buffer to stage
    /// each chunk before pushing it. Mirrors `push`'s
    /// `total_output_counter` bookkeeping — only after the read succeeds,
    /// so an EOF/IO error leaves the counter (and `tail`) unchanged.
    pub(crate) fn extend_from_reader<R: Read>(
        &mut self,
        read: R,
        fill_length: usize,
    ) -> Result<(), Error> {
        self.buffer.extend_from_reader(read, fill_length)?;
        self.total_output_counter += fill_length as u64;
        Ok(())
    }

    #[inline]
    pub(crate) fn push(&mut self, data: &[u8]) {
        self.buffer.extend(data);
        self.total_output_counter += data.len() as u64;
    }

    /// Add `n` to the cumulative produced-byte counter for output produced
    /// outside `push` / `repeat` — namely the inline `exec_sequence_inline`
    /// path, which writes through the backend directly and so bypasses the
    /// counter those methods maintain. Called by the sequence dispatch only
    /// for backends whose `INLINE_EXEC_MAINTAINS_OUTPUT_COUNTER` is `true`
    /// (`RingBuffer` / `FlatBuf`), keeping `total_output()` (resume
    /// `output_offset`) and the dict-reachability gate accurate.
    #[inline(always)]
    pub(crate) fn advance_output_counter(&mut self, n: u64) {
        self.total_output_counter += n;
    }

    /// Fallible variant of [`Self::push`]. Returns `Err(BackendOverflow)`
    /// when the underlying backend's `try_extend` rejects the write
    /// (only possible on fixed-capacity backends like
    /// `UserSliceBackend`). Used by the Raw block fast path on the
    /// direct-decode pipeline so a malformed Raw block whose declared
    /// `Block_Size` exceeds the caller's output slice surfaces as a
    /// structured error instead of panicking. Compressed-block
    /// sequence execution is a follow-up.
    #[inline(always)]
    pub(crate) fn try_push(
        &mut self,
        data: &[u8],
    ) -> Result<(), super::buffer_backend::BackendOverflow> {
        self.buffer.try_extend(data)?;
        self.total_output_counter += data.len() as u64;
        Ok(())
    }

    /// Fallible variant of [`Self::extend_and_fill`]. Same contract
    /// as [`Self::try_push`].
    #[inline]
    pub(crate) fn try_extend_and_fill(
        &mut self,
        fill_with: u8,
        fill_length: usize,
    ) -> Result<(), super::buffer_backend::BackendOverflow> {
        self.buffer.try_extend_and_fill(fill_with, fill_length)?;
        self.total_output_counter += fill_length as u64;
        Ok(())
    }

    pub(crate) fn repeat(
        &mut self,
        dict: Option<&crate::decoding::dictionary::Dictionary>,
        offset: usize,
        match_length: usize,
    ) -> Result<(), DecodeBufferError> {
        self.repeat_inner::<false>(dict, offset, match_length)
    }

    /// Same as [`repeat`] but the caller asserts a lookahead
    /// prefetch was already issued for this match source ADVANCE
    /// iterations ago, so the in-loop `prefetch_match_source` would
    /// be redundant issue-port pressure on top of the L1 line that's
    /// by now warm. Per-call `reserve` is KEPT — on malformed input
    /// the `extend_from_within_unchecked*` writes assume the buffer
    /// has the required free capacity (only `debug_assert` checks in
    /// release), and a single missing reserve here would turn a
    /// fuzz-corrupt block into out-of-bounds UB. The reserve is
    /// already amortised by the caller's upfront
    /// `reserve(MAX_BLOCK_SIZE)`, so this is a cheap capacity-check
    /// branch, not a real allocation. Used exclusively by the
    /// pipelined sequence executor in
    /// [`crate::decoding::sequence_section_decoder`].
    #[inline(always)]
    pub(crate) fn repeat_lookahead_prefetched(
        &mut self,
        dict: Option<&crate::decoding::dictionary::Dictionary>,
        offset: usize,
        match_length: usize,
    ) -> Result<(), DecodeBufferError> {
        self.repeat_inner::<true>(dict, offset, match_length)
    }

    #[inline(always)]
    fn repeat_inner<const SKIP_PREFETCH: bool>(
        &mut self,
        dict: Option<&crate::decoding::dictionary::Dictionary>,
        offset: usize,
        match_length: usize,
    ) -> Result<(), DecodeBufferError> {
        if offset == 0 {
            return Err(DecodeBufferError::ZeroOffset);
        }

        if match_length == 0 {
            return Ok(());
        }

        // The per-block decompression-bomb ceiling is NOT checked here on
        // every match (that compare bloated the inlined hot loop). It is
        // enforced once on the cold growth path: `set_block_output_ceiling`
        // lowers the backend's `max_capacity` per block, so the
        // `try_reserve` below rejects an over-producing match exactly when
        // it would have to grow the buffer past the ceiling — bounding the
        // OOM on every target while a well-formed block (covered by the
        // upfront `reserve(MAX_BLOCK_SIZE)`) never reaches the check.

        if offset > self.buffer.len() {
            self.repeat_from_dict(dict, offset, match_length)
        } else {
            let buf_len = self.buffer.len();
            let start_idx = buf_len - offset;
            let end_idx = start_idx + match_length;

            // Reserve unconditionally — `extend_from_within_unchecked*`
            // assumes the required free capacity exists; skipping it
            // would turn a malformed block (match_length past the
            // upfront `reserve(MAX_BLOCK_SIZE)`) into release-build
            // UB. The fallible variant surfaces a structured error for
            // both fixed-capacity backends (`UserSliceBackend`: write
            // past the user's slice) and the growable `RingBuffer` (a
            // grow past the per-block `max_capacity` ceiling — the
            // decompression-bomb guard). On the common path the upfront
            // `reserve(MAX_BLOCK_SIZE)` already covers the write, so this
            // is a cheap capacity check, not an allocation.
            self.buffer.try_reserve(match_length).map_err(|o| {
                DecodeBufferError::OutputBufferOverflow {
                    tail: o.tail,
                    requested: o.requested,
                    capacity: o.capacity,
                }
            })?;

            // Record the copy-shape histogram only after the reserve
            // succeeds: on `OutputBufferOverflow` the repeat never runs, so
            // counting it here would inflate the diagnostic with match
            // traffic that was never materialised.

            if !SKIP_PREFETCH {
                self.prefetch_match_source(start_idx, match_length);
            }
            if end_idx > buf_len {
                self.repeat_overlapping(offset, match_length, start_idx);
            } else {
                // SAFETY: start_idx + match_length <= self.buffer.len()
                // (start_idx = buf_len - offset, end_idx = start_idx +
                // match_length, end_idx <= buf_len). The `reserve`
                // above guarantees the destination has enough free
                // capacity for `match_length` more bytes.
                unsafe {
                    if offset >= 16 && use_branchless_wildcopy() {
                        self.buffer
                            .extend_from_within_unchecked_branchless(start_idx, match_length);
                    } else {
                        self.buffer
                            .extend_from_within_unchecked(start_idx, match_length);
                    }
                };
            }

            self.total_output_counter += match_length as u64;
            Ok(())
        }
    }

    #[inline(always)]
    fn repeat_overlapping(&mut self, offset: usize, match_length: usize, start_idx: usize) {
        if offset >= 16 {
            self.repeat_in_chunks(offset, match_length, start_idx, use_branchless_wildcopy());
        } else if offset >= 8 {
            self.repeat_in_chunks(offset, match_length, start_idx, false);
        } else {
            self.repeat_short_offset(offset, match_length, start_idx);
        }
    }

    /// Materialise an overlapping match (`offset < match_length`) by
    /// exponential doubling rather than fixed `offset`-sized chunks.
    ///
    /// The period occupies `[start_idx, start_idx + offset)` and the output
    /// grows from the current tail. Each step copies the *entire* contiguous
    /// run already materialised from the period base — length `buffer.len() -
    /// start_idx` — and appends it directly after itself: src = `[start_idx,
    /// start_idx + n)`, dst = the tail, with `n <= buffer.len() - start_idx`
    /// so `start_idx + n <= tail`. Every copy is therefore a clean
    /// *non-overlapping adjacent* block that satisfies
    /// `extend_from_within_unchecked`'s contract, and `start_idx` never moves.
    ///
    /// Because the materialised run doubles after each step, this emits
    /// `O(log2(match_length / offset))` copies — each as large as the run so
    /// far — instead of `match_length / offset` offset-sized ones. Fewer call
    /// boundaries and larger contiguous copies (better for the hardware
    /// prefetcher / store streaming) while the bytes produced are identical:
    /// the output is periodic with period `offset`, and copying any prefix of
    /// that period onto its own tail preserves the periodicity.
    ///
    /// `#[inline]` (hint, not force): this is the overlapping-match cold-ish
    /// arm of `repeat_inner`. Forcing it inline bloats the hot fully-inlined
    /// sequence executor and measurably shifts codegen of the common
    /// non-overlapping path (a small-match regression on realistic corpora);
    /// the doubling win comes from issuing fewer/larger copies, not from
    /// inlining, so the per-call boundary here is irrelevant next to the
    /// copy work it dispatches.
    #[inline]
    fn repeat_in_chunks(
        &mut self,
        offset: usize,
        match_length: usize,
        start_idx: usize,
        use_branchless_copy: bool,
    ) {
        debug_assert!(offset >= 8, "doubling path expects offset >= 8");
        let mut remaining = match_length;
        while remaining > 0 {
            // Contiguous, already-correct run from the period base. Since
            // `extend_from_within` keeps `start_idx` fixed and appends at the
            // tail, the run length is exactly `tail - start_idx`; it starts at
            // `offset` and doubles each iteration until capped by `remaining`.
            let run = self.buffer.len() - start_idx;
            let n = usize::min(run, remaining);

            // SAFETY: `n <= run = buffer.len() - start_idx` gives
            // `start_idx + n <= buffer.len()` (the tail / dst start), so the
            // source `[start_idx, start_idx + n)` and the destination at the
            // tail are adjacent and non-overlapping. `repeat()` reserved
            // `match_length` destination capacity up front.
            unsafe {
                if use_branchless_copy {
                    self.buffer
                        .extend_from_within_unchecked_branchless(start_idx, n);
                } else {
                    self.buffer.extend_from_within_unchecked(start_idx, n);
                }
            };
            remaining -= n;
        }
    }

    #[inline(always)]
    fn repeat_short_offset(&mut self, offset: usize, match_length: usize, start_idx: usize) {
        debug_assert!(
            offset > 0,
            "offset must be non-zero to avoid modulo by zero in short-offset path"
        );

        // Read the repeating period (`offset` bytes) from the existing
        // buffer surface. Cap the read at 7 so callers with offset > 7
        // never reach this function — `repeat_overlapping` dispatches
        // the offset >= 8 cases elsewhere.
        debug_assert!(offset <= 7, "repeat_short_offset is the offset<8 path");
        let mut base = [0u8; 7];
        for (i, slot) in base.iter_mut().take(offset).enumerate() {
            *slot = self.byte_at(start_idx + i);
        }

        // Fast path: offset ∈ {1, 2, 4} — the period divides 16, so
        // every 16-byte window of the repeating pattern is identical
        // and one pre-built chunk feeds the entire loop with zero
        // phase tracking. Inner loop = one 16-byte SIMD store + one
        // add.
        //
        // The chunk-build is materialised with literal constants per
        // arm rather than `chunk16[i] = base[i % offset]`. The naive
        // form gets unrolled by LLVM into 14×`divb` (8-bit divides)
        // because the compiler does not propagate `offset == 1|2|4`
        // from the outer match arm into the inner loop's modulo —
        // divb cost ~1% per byte = ~14% of decode time on
        // `decompress/level_-1_fast/decodecorpus-z000033`. Explicit
        // literal arms eliminate the divide entirely.
        if matches!(offset, 1 | 2 | 4) {
            let b0 = base[0];
            let b1 = base[1];
            let b2 = base[2];
            let b3 = base[3];
            let chunk16: [u8; 16] = match offset {
                1 => [b0; 16],
                2 => [
                    b0, b1, b0, b1, b0, b1, b0, b1, b0, b1, b0, b1, b0, b1, b0, b1,
                ],
                4 => [
                    b0, b1, b2, b3, b0, b1, b2, b3, b0, b1, b2, b3, b0, b1, b2, b3,
                ],
                // SAFETY: outer `matches!(offset, 1 | 2 | 4)` rejects
                // any other value; this arm is statically dead and
                // exists only to satisfy match exhaustiveness without
                // a runtime branch.
                _ => unsafe { core::hint::unreachable_unchecked() },
            };
            let mut copied = 0usize;
            while copied + 16 <= match_length {
                self.buffer.extend(&chunk16);
                copied += 16;
            }
            if copied < match_length {
                let tail = match_length - copied;
                self.buffer.extend(&chunk16[..tail]);
            }
            return;
        }

        // offset ∈ {3, 5, 6, 7}: 8-byte phase-pattern path. Each phase
        // is the 8-byte view of the repeating period starting at that
        // sub-position; advancing the cursor by 8 bytes shifts the
        // phase by `8 % offset` (mod offset).
        //
        // A 16-byte version (LCM(offset, 16) ∈ {48, 80, 48, 112}) was
        // measured on Intel i9-9900K — the doubled inner-loop store
        // width was offset by a 7×16 = 112-byte phase-pattern setup
        // cost (2× the 8-byte setup). On `decodecorpus-z000033`
        // short-offset matches are short enough that setup dominates
        // total cost, so the 16-byte version was a net regression on
        // every level except `level_1_fast` (where it broke even). The
        // 8-byte path retained here keeps the setup small (7×8 = 56 B)
        // and is the fastest measured option for these offsets on
        // realistic input.
        let mut phase_patterns = [[0u8; 8]; 7];
        for phase in 0..offset {
            for i in 0..8 {
                phase_patterns[phase][i] = base[(phase + i) % offset];
            }
        }

        let phase_step = 8 % offset;
        let mut phase = 0usize;
        let mut copied = 0usize;
        while copied + 8 <= match_length {
            self.buffer.extend(&phase_patterns[phase]);
            copied += 8;
            phase = (phase + phase_step) % offset;
        }

        if copied < match_length {
            let tail = match_length - copied;
            self.buffer.extend(&phase_patterns[phase][..tail]);
        }
    }

    #[inline(always)]
    fn byte_at(&self, idx: usize) -> u8 {
        let (s1, s2) = self.buffer.as_slices();
        if idx < s1.len() {
            s1[idx]
        } else {
            s2[idx - s1.len()]
        }
    }

    #[inline(always)]
    fn prefetch_match_source(&self, start_idx: usize, match_length: usize) {
        if match_length < 64 {
            return;
        }
        let (s1, s2) = self.buffer.as_slices();
        if start_idx < s1.len() {
            prefetch::prefetch_slice_t1(&s1[start_idx..]);
        } else {
            let idx = start_idx - s1.len();
            if idx < s2.len() {
                prefetch::prefetch_slice_t1(&s2[idx..]);
            }
        }
    }

    /// Lookahead-friendly prefetch issued ahead of execute. The
    /// in-loop `prefetch_match_source` above fires at the moment of
    /// the copy, so it can't hide DRAM latency for cold long-distance
    /// match sources. Pipelined callers compute the match source
    /// logical index 3-4 sequences in advance and call this helper —
    /// by the time the corresponding `repeat()` reaches the actual
    /// load, the line is already in-flight.
    ///
    /// `start_idx` is a logical index into the current buffer (same
    /// frame as `buffer.len()`). Indices outside `[0, buffer.len())`
    /// are silently dropped — the cases this guards against include
    /// intra-block self-overlap (source falls past the not-yet-
    /// written cursor), `wrapping_sub` underflow on a caller that
    /// computed `match_start - offset` with an offset larger than
    /// match_start (e.g. a stale or malformed sequence), and
    /// dictionary-sourced matches whose logical position predates
    /// the buffer's current frame. Upstream zstd (`PREFETCH_L1` in
    /// `ZSTD_prefetchMatch` — we mirror that with `prefetch_slice`
    /// → `_MM_HINT_T0` / `pldl1keep`, see the body comment) tolerates
    /// invalid addresses by spec, but in
    /// safe Rust the cheapest equivalent is to bound-check the
    /// logical position before chasing the slice.
    #[inline(always)]
    pub(crate) fn prefetch_lookahead_match_source(&self, start_idx: usize) {
        if start_idx >= self.buffer.len() {
            return;
        }
        // Upstream zstd's `ZSTD_prefetchMatch` issues two `PREFETCH_L1` hints
        // per match — one at `match`, one at `match + CACHELINE_SIZE`.
        // We mirror that intent via `prefetch_slice` (`_MM_HINT_T0` on
        // x86 / `pldl1keep` on aarch64 → L1 destination) with extent
        // capped at 2 × 64 B = 128 B. In the contiguous case the helper
        // emits at most two prefetch instructions, matching upstream zstd
        // exactly. In the wrap-boundary case the same 128 B budget is
        // split across `s1_tail` and `s2[0..]`, which can emit up to
        // four cache-line prefetches total (two per slice when each
        // side covers a full 64 B) — still bounded, still L1, still
        // less than the helper's MAX_LINES = 4 ceiling. The lookahead
        // depth (ADVANCE) is small enough that L1 should hold the line
        // across the gap; if profiling later shows L1 eviction
        // pressure we can revisit T1/L2.
        const PREFETCH_EXTENT: usize = 128;
        const CACHE_LINE: usize = 64;
        let (s1, s2) = self.buffer.as_slices();
        if start_idx < s1.len() {
            let s1_tail = &s1[start_idx..];
            let s1_bound = core::cmp::min(s1_tail.len(), PREFETCH_EXTENT);
            // `prefetch_slice` no-ops on slices shorter than one cache
            // line — sensible for bulk prefetch, but wrong for the
            // wrap-boundary case where the cache line containing
            // `start_idx` IS the line we need warmed even if the
            // remaining contiguous extent is < 64 B. Fall back to the
            // single-line variant in that case so the match-start
            // line is always hinted.
            if s1_bound >= CACHE_LINE {
                prefetch::prefetch_slice(&s1_tail[..s1_bound]);
            } else {
                prefetch::prefetch_first_line_l1(&s1_tail[..s1_bound]);
            }
            // Wrap continuation: when the match source straddles the
            // s1/s2 boundary and the s1 tail is shorter than the
            // PREFETCH_EXTENT we asked for, top up the rest from
            // s2[0..]. Without this the upstream "up to two cache
            // lines" intent silently collapses to one (or zero if
            // s1_tail is the last sub-line of s1).
            if s1_bound < PREFETCH_EXTENT {
                let remaining = PREFETCH_EXTENT - s1_bound;
                let s2_bound = core::cmp::min(s2.len(), remaining);
                if s2_bound >= CACHE_LINE {
                    prefetch::prefetch_slice(&s2[..s2_bound]);
                } else if s2_bound > 0 {
                    prefetch::prefetch_first_line_l1(&s2[..s2_bound]);
                }
            }
        } else {
            // `start_idx < self.buffer.len()` from the early return,
            // `buffer.len() == s1.len() + s2.len()`, and the else
            // branch establishes `start_idx >= s1.len()`. So
            // `idx = start_idx - s1.len() < s2.len()` by construction
            // — no explicit `idx < s2.len()` guard needed.
            let idx = start_idx - s1.len();
            let tail = &s2[idx..];
            let bound = core::cmp::min(tail.len(), PREFETCH_EXTENT);
            if bound >= CACHE_LINE {
                prefetch::prefetch_slice(&tail[..bound]);
            } else {
                prefetch::prefetch_first_line_l1(&tail[..bound]);
            }
        }
    }

    #[cold]
    fn repeat_from_dict(
        &mut self,
        dict: Option<&crate::decoding::dictionary::Dictionary>,
        offset: usize,
        match_length: usize,
    ) -> Result<(), DecodeBufferError> {
        // Reachability gate: dict-source matches are only valid while
        // the dictionary content is still inside the visible window.
        // `RingBuffer` / `FlatBuf` maintain `total_output_counter`
        // (push / repeat / inline-exec counter bump). On the direct
        // path (`UserSliceBackend`) the inline executor skips the
        // counter, so `buffer.len()` carries the cumulative output
        // until the first between-blocks `drop_to_window_size()`, and
        // that drop latches the counter above the window before capping
        // the visible length — the max of the two stays accurate for
        // every backend.
        let total_output = self.total_output_counter.max(self.buffer.len() as u64);
        if total_output <= self.window_size as u64 {
            // at least part of that repeat is from the dictionary content
            let bytes_from_dict = offset - self.buffer.len();

            // The dictionary content slice is tied to the call-scoped `dict`
            // borrow (owner-kept-alive for the decode), trivially disjoint from
            // the `self.buffer` mutation below, allowing the read+extend without
            // an intermediate copy. `None` → empty slice → the length guard
            // rejects (the no-dict path).
            let dict_content: &[u8] = self.dict_content(dict);
            let dict_len = dict_content.len();

            if bytes_from_dict > dict_len {
                return Err(DecodeBufferError::NotEnoughBytesInDictionary {
                    got: dict_len,
                    need: bytes_from_dict,
                });
            }

            // Enforce the per-block bomb ceiling on the dictionary-backed
            // output too: the `extend` below appends `dict_slice` directly
            // (the inline `push`/`repeat` guard never runs for it), so without
            // this an over-producing match satisfied from the dictionary could
            // grow past the armed ceiling. Reserving the full `match_length`
            // covers both the dict portion here and the buffer-history
            // remainder the recursive `repeat` appends.
            self.buffer.try_reserve(match_length).map_err(|o| {
                DecodeBufferError::OutputBufferOverflow {
                    tail: o.tail,
                    requested: o.requested,
                    capacity: o.capacity,
                }
            })?;

            if bytes_from_dict < match_length {
                let dict_slice = &dict_content[dict_len - bytes_from_dict..];
                prefetch::prefetch_slice(dict_slice);
                self.buffer.extend(dict_slice);

                self.total_output_counter += bytes_from_dict as u64;
                return self.repeat(dict, self.buffer.len(), match_length - bytes_from_dict);
            } else {
                let low = dict_len - bytes_from_dict;
                let high = low + match_length;
                let dict_slice = &dict_content[low..high];
                prefetch::prefetch_slice(dict_slice);
                self.buffer.extend(dict_slice);
                self.total_output_counter += match_length as u64;
            }
            Ok(())
        } else {
            Err(DecodeBufferError::OffsetTooBig {
                offset,
                buf_len: self.buffer.len(),
            })
        }
    }

    /// Check if and how many bytes can currently be drawn from the buffer
    pub(crate) fn can_drain_to_window_size(&self) -> Option<usize> {
        if self.buffer.len() > self.window_size {
            Some(self.buffer.len() - self.window_size)
        } else {
            None
        }
    }

    //How many bytes can be drained if the window_size does not have to be maintained
    pub(crate) fn can_drain(&self) -> usize {
        self.buffer.len()
    }

    /// Drain as much as possible while retaining enough so that decoding si still possible with the required window_size
    /// At best call only if can_drain_to_window_size reports a 'high' number of bytes to reduce allocations
    pub(crate) fn drain_to_window_size(&mut self) -> Option<Vec<u8>> {
        //TODO investigate if it is possible to return the std::vec::Drain iterator directly without collecting here
        match self.can_drain_to_window_size() {
            None => None,
            Some(can_drain) => {
                let mut vec = Vec::with_capacity(can_drain);
                self.drain_to(can_drain, |buf| {
                    vec.extend_from_slice(buf);
                    (buf.len(), Ok(()))
                })
                .ok()?;
                Some(vec)
            }
        }
    }

    pub(crate) fn drain_to_window_size_writer(
        &mut self,
        mut sink: impl Write,
    ) -> Result<usize, Error> {
        match self.can_drain_to_window_size() {
            None => Ok(0),
            Some(can_drain) => self.drain_to(can_drain, |buf| write_all_bytes(&mut sink, buf)),
        }
    }

    /// Advance the backend's head past any bytes beyond `window_size`
    /// without producing them to a sink — the bytes remain physically
    /// present (the backend's allocation never shrinks), but they are
    /// no longer visible through [`Self::len`] / `as_slices` /
    /// `repeat`. Used by the direct-decode path on multi-segment
    /// frames where the caller's output IS the buffer, so the bytes
    /// don't need to be drained anywhere — they just need to drop
    /// out of `len()` so the offset-bound match validation
    /// (`offset <= buffer.len()`) coincides with the spec's
    /// window-size rule (`offset <= window_size`).
    ///
    /// Does NOT update the rolling content checksum. On the direct
    /// path the caller (`FrameDecoder::decode_all`) hashes the
    /// final `output[..content_size]` slice ONCE at end of decode
    /// (single sequential xxhash pass over cache-hot data) and
    /// propagates the digest into the persistent scratch's hasher.
    /// Hashing inside `drop_to_window_size` would re-hash the same
    /// bytes per block (this method runs once per block on
    /// multi-segment frames), which is wasted work — the end-of-
    /// decode walk covers the entire output uniformly.
    ///
    /// Returns the number of bytes whose visibility was discarded.
    ///
    /// Catches `total_output_counter` up to the visible length before
    /// dropping (never adds — adding the dropped amount would
    /// double-count on backends whose `push` / `repeat` already
    /// counted those bytes). On the direct path the inline executors
    /// skip the counter, so without the catch-up the cap below would
    /// shrink `len()` back under `window_size` and reopen
    /// `repeat_from_dict`'s reachability gate even though cumulative
    /// output already exceeded the window. A drop only ever happens
    /// when `len() > window_size`, so the catch-up permanently latches
    /// the counter above the window — exactly what the boolean gate
    /// needs. On `RingBuffer` / `FlatBuf` the counter is already exact
    /// (`>= len()`) and the `max` is a no-op.
    pub(crate) fn drop_to_window_size(&mut self) -> usize {
        match self.can_drain_to_window_size() {
            None => 0,
            Some(can_drop) => {
                self.total_output_counter = self.total_output_counter.max(self.buffer.len() as u64);
                self.buffer.drop_first_n(can_drop);
                can_drop
            }
        }
    }

    /// drain the buffer completely
    pub(crate) fn drain(&mut self) -> Vec<u8> {
        let (slice1, slice2) = self.buffer.as_slices();

        if self.compute_hash {
            self.hash.write(slice1);
            self.hash.write(slice2);
        }

        let mut vec = Vec::with_capacity(slice1.len() + slice2.len());
        vec.extend_from_slice(slice1);
        vec.extend_from_slice(slice2);
        self.buffer.clear();
        vec
    }

    pub(crate) fn drain_to_writer(&mut self, mut sink: impl Write) -> Result<usize, Error> {
        let write_limit = self.buffer.len();
        self.drain_to(write_limit, |buf| write_all_bytes(&mut sink, buf))
    }

    pub(crate) fn read_all(&mut self, target: &mut [u8]) -> Result<usize, Error> {
        let amount = self.buffer.len().min(target.len());

        let mut written = 0;
        self.drain_to(amount, |buf| {
            target[written..][..buf.len()].copy_from_slice(buf);
            written += buf.len();
            (buf.len(), Ok(()))
        })?;
        Ok(amount)
    }

    /// Semantics of write_bytes:
    /// Should dump as many of the provided bytes as possible to whatever sink until no bytes are left or an error is encountered
    /// Return how many bytes have actually been dumped to the sink.
    fn drain_to(
        &mut self,
        amount: usize,
        mut write_bytes: impl FnMut(&[u8]) -> (usize, Result<(), Error>),
    ) -> Result<usize, Error> {
        if amount == 0 {
            return Ok(0);
        }

        struct DrainGuard<'a, B: BufferBackend> {
            buffer: &'a mut B,
            amount: usize,
        }

        impl<B: BufferBackend> Drop for DrainGuard<'_, B> {
            fn drop(&mut self) {
                if self.amount != 0 {
                    self.buffer.drop_first_n(self.amount);
                }
            }
        }

        let mut drain_guard = DrainGuard {
            buffer: &mut self.buffer,
            amount: 0,
        };

        let (slice1, slice2) = drain_guard.buffer.as_slices();
        let n1 = slice1.len().min(amount);
        let n2 = slice2.len().min(amount - n1);

        if n1 != 0 {
            let (written1, res1) = write_bytes(&slice1[..n1]);

            if self.compute_hash {
                self.hash.write(&slice1[..written1]);
            }
            drain_guard.amount += written1;

            // Apparently this is what clippy thinks is the best way of expressing this
            res1?;

            // Only if the first call to write_bytes was not a partial write we can continue with slice2
            // Partial writes SHOULD never happen without res1 being an error, but lets just protect against it anyways.
            if written1 == n1 && n2 != 0 {
                let (written2, res2) = write_bytes(&slice2[..n2]);

                if self.compute_hash {
                    self.hash.write(&slice2[..written2]);
                }
                drain_guard.amount += written2;

                // Apparently this is what clippy thinks is the best way of expressing this
                res2?;
            }
        }

        let amount_written = drain_guard.amount;
        // Make sure we don't accidentally drop `DrainGuard` earlier.
        drop(drain_guard);

        Ok(amount_written)
    }
}

/// Like Write::write_all but returns partial write length even on error
fn write_all_bytes(mut sink: impl Write, buf: &[u8]) -> (usize, Result<(), Error>) {
    let mut written = 0;
    while written < buf.len() {
        match sink.write(&buf[written..]) {
            Ok(0) => return (written, Ok(())),
            Ok(w) => written += w,
            Err(e) => return (written, Err(e)),
        }
    }
    (written, Ok(()))
}

#[inline(always)]
fn use_branchless_wildcopy() -> bool {
    cfg!(any(target_arch = "x86", target_arch = "x86_64"))
}
