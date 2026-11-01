#![allow(
    clippy::inline_always,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! `BufferBackend` — the compile-time-dispatched interface for the
//! decoder's output storage.
//!
//! Two concrete impls live alongside this module:
//! [`super::ringbuffer::RingBuffer`] (full wrap-aware semantics, default)
//! and [`super::flat_buf::FlatBuf`] (no-wrap fast path used when the
//! frame header's `Single_Segment_flag` guarantees the decompressed
//! output never exceeds `window_size` and so never wraps).
//!
//! Selection happens through the generic parameter on
//! [`super::decode_buffer::DecodeBuffer<B>`] and cascades through
//! `DecoderScratch<B>` to the block-level decode functions. The
//! compiler monomorphises each backend independently and erases the
//! wrap-checking code path entirely on the flat side. A runtime `enum BufferStorage`
//! paid match-dispatch overhead in every push/repeat and measured a
//! +43–58 % regression on small-frame decompress benchmarks, so the
//! compile-time generic shape is load-bearing.

use crate::io::{Error, Read};

/// Trailing-slack count both backends pad their physical allocation
/// with so SIMD wildcopy reads / writes can overshoot the live region
/// without leaving the allocation. Sized at **32 bytes** so the AVX2
/// chunked kernel in `simd_copy::copy_bytes_overshooting` (32-byte
/// stride via `_mm256_storeu_si256` on x86-64) can fire on tail copies.
/// The kernel gates on `min_buffer_size >= rounded(copy_at_least, 32)`;
/// at the end of a fixed-capacity output buffer that gate fails when
/// slack is < 32, and the dispatch falls through to whatever
/// `ptr::copy_nonoverlapping` lowers to on the target — a
/// platform-specific `memcpy`-like primitive (the source/dest regions
/// are non-overlapping by the caller's contract, so memcpy semantics
/// apply; the exact symbol the linker resolves is libc-specific and
/// not part of any guaranteed contract). Bumping slack from 16 → 32
/// keeps the AVX2 path live across every match-copy and literal-push,
/// avoiding the libc detour.
///
/// Both `RingBuffer` and `FlatBuf` reuse this single constant so the
/// slack contract cannot drift between backends.
pub(crate) const WILDCOPY_OVERLENGTH: usize = 32;

/// Single-compare output-capacity guard for the inline sequence-exec hot
/// path, shared by every [`BufferBackend::exec_sequence_inline`] /
/// `exec_sequence_inline_avx2` override so the per-sequence bounds check has
/// one implementation instead of a duplicated `checked_add` chain.
///
/// Returns `lit_length + match_length` when the literal+match write plus
/// `overshoot` bytes of SIMD wildcopy slack fits within `cap - tail`;
/// otherwise [`ExecuteSequencesError::OutputBufferOverflow`](super::errors::ExecuteSequencesError::OutputBufferOverflow).
///
/// # Preconditions
/// - `tail <= cap`, so `cap - tail` cannot underflow. Holds for every
///   backend: `Vec::len() <= Vec::capacity()` on the flat / growable buffers,
///   and the user-slice tail only ever advances past this same check.
/// - `lit_length` and `match_length` are each bounded by the maximum FSE
///   LL/ML code expansion (~131 KB), so `total` and `total + overshoot`
///   cannot overflow `usize` even on 32-bit.
///
/// Each caller documents why the first precondition holds at its site; the
/// arithmetic safety of the second is the same FSE bound everywhere.
#[inline(always)]
pub(crate) fn sequence_output_fits(
    lit_length: usize,
    match_length: usize,
    tail: usize,
    cap: usize,
    overshoot: usize,
) -> Result<usize, super::errors::ExecuteSequencesError> {
    let total = lit_length + match_length;
    if total + overshoot > cap - tail {
        return Err(super::errors::ExecuteSequencesError::OutputBufferOverflow {
            tail,
            requested: total,
            capacity: cap,
        });
    }
    Ok(total)
}

/// Storage operations the decoder needs from its output buffer.
///
/// The trait surface mirrors the historical `RingBuffer` API the
/// `DecodeBuffer` consumed before the generic split — every method's
/// semantics match what `RingBuffer` already provides; `FlatBuf`'s
/// impl is the no-wrap shape of the same contract.
pub(crate) trait BufferBackend: Sized {
    /// `true` when the backend can execute a single sequence via the
    /// upstream zstd-shape inline `exec_sequence_inline` path (literal
    /// copy + match copy in one straight-line body, no per-call
    /// dispatch through `extend` / `repeat`). Defaults to `false`;
    /// `UserSliceBackend` overrides to `true` on `x86_64` only —
    /// 32-bit `x86` is excluded because the upstream zstd helpers emit SSE2
    /// intrinsics without a `#[target_feature]` gate, and pre-SSE2
    /// i386 / i486 / i586 baselines would SIGILL.
    ///
    /// Reads of this const at the dispatch site fold to a compile-time
    /// branch the optimiser dead-eliminates — the unused arm
    /// (upstream zstd body on `FlatBuf` / `RingBuffer`, existing
    /// `push`/`repeat` body on `UserSliceBackend`) carries no runtime
    /// cost.
    const SUPPORTS_INLINE_SEQUENCE_EXEC: bool = false;

    /// Whether the inline `exec_sequence_inline` dispatch must bump the
    /// `DecodeBuffer::total_output_counter` after each sequence. The non-inline
    /// `push` / `repeat` path always maintains that counter; the inline path
    /// bypasses the wrapper, so backends whose cumulative-output accounting
    /// READS the counter (`RingBuffer` / `FlatBuf` — used by the resume
    /// `output_offset` and the dict-reachability gate) need the inline path to
    /// keep it current. `UserSliceBackend` (the direct path) reads its `tail`
    /// instead and never touches the counter, so it overrides this to `false`
    /// and the per-sequence read-modify-write is dead-eliminated there (the
    /// ~9% it costs on the all-inline direct hot path stays saved). Compile-time
    /// const: the dispatch-site branch folds away per backend.
    const INLINE_EXEC_MAINTAINS_OUTPUT_COUNTER: bool = true;

    /// Upstream zstd's `ZSTD_execSequence` body
    /// (zstd_decompress_block.c:1008-1105). Writes `lit_length` bytes
    /// from `lit_src` at the current tail, then writes `match_length`
    /// bytes via the upstream zstd offset-dispatch (offset ≥ 16 → wildcopy
    /// no-overlap; offset 1..=15 → overlapCopy8 + wildcopy
    /// overlap-src-before-dst).
    ///
    /// Default impl is `unreachable!`; the dispatch site only routes
    /// here when [`Self::SUPPORTS_INLINE_SEQUENCE_EXEC`] is `true`,
    /// which is fixed at compile time per backend type. The
    /// `unreachable!` body costs nothing on backends that gate it
    /// out (the compiler removes the call entirely).
    ///
    /// # Safety
    /// - `lit_src` MUST be derived from the FULL parent literals
    ///   buffer's `as_ptr()` (not a sub-slice). The upstream zstd body issues
    ///   an unconditional 16-byte `_mm_loadu_si128` regardless of
    ///   `lit_length`; reads through `lit_src` must stay within the
    ///   parent buffer's allocated provenance even when
    ///   `lit_length < 16`. Passing a sub-slice's `as_ptr()` whose
    ///   `len() < 16` would be UB even when the bytes beyond
    ///   `lit_length` happen to be valid memory in the backing
    ///   allocation.
    /// - `lit_length + match_length` must fit in the writable tail
    ///   slack (caller's upfront `reserve(MAX_BLOCK_SIZE)` covers
    ///   the regular case; for direct decode the slice's
    ///   `WILDCOPY_OVERLENGTH` slack covers the wildcopy overshoot).
    /// - `offset >= 1` and `offset <= self.len() + lit_length`
    ///   (upstream zstd's `oLitEnd - offset` precondition).
    /// - `match_length >= 1`.
    /// - **Read-side slack on the parent literals buffer**: the upstream zstd
    ///   literal-copy path issues an unconditional `copy16` from
    ///   `lit_src` and, when `lit_length > 16`, a 16-byte-stride
    ///   wildcopy whose final iteration's last byte read is at
    ///   `lit_cur_before + lit_length.next_multiple_of(16) - 1`.
    ///   Callers MUST satisfy two distinct slack bounds against the
    ///   parent buffer length (`lit_len`):
    ///   - `lit_cur_before + 16 <= lit_len` ALWAYS (the
    ///     unconditional `copy16` reads 16 bytes regardless of
    ///     `lit_length`, including the `lit_length == 0` case).
    ///   - `lit_cur_before + lit_length.next_multiple_of(16) <=
    ///     lit_len` ONLY when `lit_length > 16` (the wildcopy tail's
    ///     final 16-byte load reaches through that exact offset).
    ///
    ///   The current dispatch site
    ///   (`sequence_section_decoder::execute_one_sequence_pipelined`)
    ///   enforces both via `inline_path_safe = lit_cur_before + 16 <=
    ///   lit_len && (lit_length <= 16 || lit_cur_before +
    ///   lit_length.next_multiple_of(16) <= lit_len)` and falls
    ///   through to the legacy `push`/`repeat` chain when either
    ///   bound fails — a future caller reusing this hook must
    ///   enforce the same gate or pad the literals buffer with 15
    ///   bytes of slack at allocation time.
    /// - This method writes directly through the backend; the
    ///   wrapper-level `DecodeBuffer::total_output_counter` is NOT
    ///   maintained on this path. Callers that need a byte count for
    ///   the inline-eligible path must read `BufferBackend::tail()`
    ///   (see `FrameDecoder::run_direct_decode`'s post-block FCS
    ///   check). Hash is likewise deferred to the post-block
    ///   full-slice pass in `FrameDecoder::decode_all`.
    #[allow(unused_variables, unused_mut)]
    #[inline(always)]
    unsafe fn exec_sequence_inline(
        &mut self,
        lit_src: *const u8,
        lit_length: usize,
        offset: usize,
        match_length: usize,
    ) -> Result<(), super::errors::ExecuteSequencesError> {
        // Default body is statically unreachable when the dispatch
        // site honours `SUPPORTS_INLINE_SEQUENCE_EXEC`. Backends that
        // return `false` from that const never see this call resolved
        // — the optimiser dead-eliminates the calling branch in the
        // monomorphised caller.
        unreachable!(
            "exec_sequence_inline called on backend whose SUPPORTS_INLINE_SEQUENCE_EXEC is false"
        );
    }

    /// AVX2-tier variant of [`Self::exec_sequence_inline`]. Same
    /// contract but the **no-overlap match-copy** path (`offset >= 32`)
    /// emits 32-byte ymm stores via `wildcopy_no_overlap_avx2`. Invoked only from
    /// `execute_one_sequence_pipelined_avx2` which is itself
    /// `#[target_feature(enable = "avx2,bmi2")]`.
    ///
    /// **Literal copy stays on SSE2 16-byte** (`copy16` +
    /// `wildcopy_no_overlap`) — the inline-path slack gate in
    /// `execute_one_sequence_pipelined_avx2` is built around the
    /// 16-byte literal over-read bound from the SSE2 default; widening
    /// to 32-byte AVX2 stores on literals would require tightening
    /// that gate to `lit_cur_before + 32 <= lit_len`, rejecting more
    /// near-end-of-block sequences from the inline fast path. The
    /// AVX2 divergence is therefore confined to the match-copy side,
    /// where the per-block `reserve(MAX_BLOCK_SIZE)` plus the
    /// `WILDCOPY_OVERLENGTH = 32` slack on `UserSliceBackend`
    /// accommodates the 31-byte ymm overshoot without changing the
    /// caller-side contract.
    ///
    /// Match-copy WILDCOPY overshoot at destination grows from 15 to
    /// 31 bytes for the AVX2 path (32-byte stride overshoots up to 31
    /// bytes past `tail + total`); the override raises
    /// `MAX_WILDCOPY_OVERSHOOT` accordingly.
    ///
    /// Default impl is `unreachable!`. The x86_64 backends override:
    /// `UserSliceBackend` (direct-decode path, fixed slice with
    /// `WILDCOPY_OVERLENGTH` slack) and `FlatBuf` (single-segment
    /// frames, Vec-backed with `with_capacity(+ WILDCOPY_OVERLENGTH)`
    /// slack). Both gate via `SUPPORTS_INLINE_SEQUENCE_EXEC`; runtime
    /// CPU AVX2 presence is gated at the dispatcher in
    /// `sequence_section_decoder::decode_and_execute_sequences` via
    /// `detect_cpu_kernel() == Avx2`. `RingBuffer` does NOT override
    /// — multi-segment frames still go through the layered
    /// `repeat()` chain that handles wrap correctly.
    ///
    /// # Safety
    /// Same preconditions as [`Self::exec_sequence_inline`] plus:
    /// caller MUST be in `#[target_feature(enable = "avx2,bmi2")]`
    /// scope (the only call site is the AVX2-tier execute path which
    /// satisfies this), and the destination slack at the writable
    /// tail MUST be ≥ 31 bytes past `tail + total` (upstream zstd's 16-byte
    /// SIMD-copy overshoot bound doubles for 32-byte ymm stride).
    #[allow(unused_variables, unused_mut, dead_code)]
    #[inline(always)]
    unsafe fn exec_sequence_inline_avx2(
        &mut self,
        lit_src: *const u8,
        lit_length: usize,
        offset: usize,
        match_length: usize,
    ) -> Result<(), super::errors::ExecuteSequencesError> {
        unreachable!(
            "exec_sequence_inline_avx2 called on backend that did not override the default \
             (UserSliceBackend and FlatBuf override on x86_64)"
        );
    }

    /// Base pointer of the contiguous output region, for the inline
    /// match-copy macro `exec_sequence_avx2_inline!` (which expands the
    /// AVX2 `ZSTD_execSequence` body textually at the sequence-loop call
    /// site so it fuses into the per-tier monolith — `#[target_feature]`
    /// functions cannot be `#[inline(always)]`
    /// (compiler restriction). Only valid
    /// when [`Self::SUPPORTS_INLINE_SEQUENCE_EXEC`]; the linear backends
    /// (`UserSliceBackend`, `FlatBuf`) override, `RingBuffer` never reaches
    /// it (gated, wrap-aware fallback).
    ///
    /// # Safety
    /// Caller must hold the macro's preconditions (inline path gated on
    /// `SUPPORTS_INLINE_SEQUENCE_EXEC` + capacity validated by
    /// `sequence_output_fits`).
    // Only reached from the x86_64 AVX2 macro; dead on other targets
    // (i686/aarch64 use the scalar/NEON exec paths), same as
    // `exec_sequence_inline_avx2` above.
    #[allow(dead_code)]
    #[inline(always)]
    unsafe fn inline_exec_base_ptr(&mut self) -> *mut u8 {
        unreachable!("inline_exec_base_ptr on a backend without inline-sequence support")
    }

    /// Commit the post-exec write cursor (grow the live region) after the
    /// inline match-copy macro has written `[tail, new_tail)`.
    /// `UserSliceBackend` advances its cursor; `FlatBuf` `set_len`s the Vec.
    /// Distinct from [`Self::set_tail`], which is a shrink-only rollback
    /// primitive (`new_tail <= len`).
    ///
    /// # Safety
    /// `new_tail` bytes `[0, new_tail)` must be initialised (the macro just
    /// wrote `[tail, new_tail)`); `new_tail <= capacity`.
    #[allow(dead_code)]
    #[inline(always)]
    unsafe fn inline_exec_commit(&mut self, _new_tail: usize) {
        unreachable!("inline_exec_commit on a backend without inline-sequence support")
    }

    /// Whether the inline `ZSTD_execSequence` body (the `exec_sequence_*`
    /// macros / [`Self::exec_sequence_inline`]) may run for this
    /// `(lit_length, match_length)` at the current cursor. The inline body
    /// addresses the output linearly (`base + tail …`, match source
    /// `base + tail + lit_length - offset`) with up to 31 bytes of wildcopy
    /// overshoot, so it is only sound when that region is one contiguous run.
    ///
    /// Linear backends (`FlatBuf`, `UserSliceBackend`) are always contiguous,
    /// so the default returns `true` and their `sequence_output_fits` /
    /// tight-tail / grow handling covers capacity. `RingBuffer` overrides this
    /// to reject only the cases where this specific sequence's linear write or
    /// its match source would cross the wrap boundary; a wrapped ring whose
    /// write stays in the contiguous free gap before `head` and whose match
    /// source is the contiguous lower live segment still takes the fast inline
    /// path. The caller falls back to the wrap-correct cold `push` / `repeat`
    /// path only when this returns `false`. `offset` is the resolved match
    /// offset (post-repcode), needed by the ring to verify the match source is
    /// contiguous; linear backends ignore it.
    #[allow(unused_variables)]
    #[inline(always)]
    fn inline_exec_ok(&self, lit_length: usize, match_length: usize, offset: usize) -> bool {
        true
    }

    /// Construct an empty backend. Backend-specific sizing is done
    /// via `with_capacity` constructors on the concrete types (see
    /// [`super::flat_buf::FlatBuf::with_capacity`]).
    fn new() -> Self;

    /// Empty the buffer; reset internal cursors to 0.
    fn clear(&mut self);

    /// Reserve at least `n` bytes of additional writable capacity.
    /// May or may not allocate depending on current free space.
    fn reserve(&mut self, n: usize);

    /// Like [`Self::reserve`], but growth is sized to the request instead
    /// of the amortized-doubling policy. For the one-shot window
    /// pre-reservation: a request that lands one slack past the retained
    /// capacity (e.g. a dictionary prefix already in the buffer) would
    /// otherwise DOUBLE a window-sized allocation. Per-block growth keeps
    /// using `reserve` so streaming paths stay amortized.
    fn reserve_exact(&mut self, n: usize) {
        self.reserve(n);
    }

    /// Fallible variant of [`Self::reserve`] for fixed-capacity
    /// backends. Growable backends (`FlatBuf`, `RingBuffer`) call
    /// `reserve` which always succeeds (or aborts on alloc failure)
    /// and return `Ok`. Fixed-capacity backends (`UserSliceBackend`)
    /// override with a linear `tail + n <= cap` check and return
    /// `Err(BackendOverflow)` when the requested write would land
    /// past the end of the user's slice — letting the safe public
    /// decode APIs surface a structured error instead of panicking
    /// from the per-call `assert!` inside
    /// `extend_from_within_unchecked`.
    fn try_reserve(&mut self, n: usize) -> Result<(), BackendOverflow> {
        self.reserve(n);
        Ok(())
    }

    /// Lower the per-block growth ceiling on backends whose `try_reserve`
    /// may grow without bound. The block sequence decoder sets it to
    /// `len + MAX_BLOCK_SIZE` per block so an over-producing match is rejected
    /// on the cold growth path, bounding decompression-bomb OOMs. Overridden by
    /// the streaming `RingBuffer` and the growable `FlatBuf` (whose fallback
    /// `push`/`repeat` path grows through `try_reserve`). Default no-op for
    /// fixed-capacity backends (`UserSliceBackend`), which are already bounded.
    fn set_max_capacity(&mut self, _max_capacity: usize) {}

    /// Live byte count: bytes between the logical head and tail.
    fn len(&self) -> usize;

    /// Realloc-detection sentinel for
    /// [`super::decode_buffer::DecodeBufferCheckpoint`]. The exact
    /// value is backend-specific (RingBuffer returns its ring-
    /// indexing capacity, which does not include the trailing
    /// [`WILDCOPY_OVERLENGTH`] slack bytes; FlatBuf returns the
    /// full `Vec::capacity` which does include them). The contract
    /// the checkpoint relies on is invariant per-instance: `cap()`
    /// stays equal across calls as long as no reallocation has
    /// happened. Equality is the only operation the checkpoint
    /// performs — the absolute value is never compared across
    /// backends.
    fn cap(&self) -> usize;

    /// Physical write cursor — paired with [`Self::set_tail`] for the
    /// rollback primitive.
    fn tail(&self) -> usize;

    /// Restore the write cursor to a previously captured `tail()`.
    ///
    /// # Safety
    /// - `new_tail` was returned by an earlier `tail()` on this same
    ///   instance.
    /// - `cap()` has not changed since (the caller validates this via
    ///   the checkpoint's `cap` snapshot — both backends would
    ///   silently corrupt their live region otherwise).
    /// - Bytes between `new_tail` and the current tail are discarded
    ///   by the caller and never read again.
    unsafe fn set_tail(&mut self, new_tail: usize);

    /// Append `data` to the tail.
    fn extend(&mut self, data: &[u8]);

    /// Append `fill_length` copies of `fill_with` to the tail.
    /// Backs the RLE block path.
    fn extend_and_fill(&mut self, fill_with: u8, fill_length: usize);

    /// Read exactly `fill_length` bytes from `read` directly into the
    /// tail. Backs the Raw block path.
    fn extend_from_reader<R: Read>(&mut self, read: R, fill_length: usize) -> Result<(), Error>;

    /// Copy `len` bytes from logical position `start` (relative to
    /// the live region's head) to the tail. Non-overlapping case.
    ///
    /// # Safety
    /// - `start + len <= self.len()`.
    /// - Capacity for `len` additional bytes past the current tail
    ///   was reserved by the caller.
    unsafe fn extend_from_within_unchecked(&mut self, start: usize, len: usize);

    /// Branchless variant used on x86 builds where the unchecked
    /// non-overlap precondition allows the chunked wildcopy to skip
    /// the per-iteration overlap check. On backends where the
    /// distinction has no perf delta this simply forwards to
    /// `extend_from_within_unchecked`.
    ///
    /// # Safety
    /// Same as [`Self::extend_from_within_unchecked`].
    unsafe fn extend_from_within_unchecked_branchless(&mut self, start: usize, len: usize);

    /// Two-slice view of the live region. The second slice is empty
    /// on backends that don't wrap (flat path) — the API shape is
    /// preserved so drain code is shared between backends.
    fn as_slices(&self) -> (&[u8], &[u8]);

    /// Advance the head past `n` bytes — they are removed from the
    /// live window but may still be physically present (backing
    /// future match copies). Mirrors the historical
    /// `RingBuffer::drop_first_n` contract.
    fn drop_first_n(&mut self, n: usize);

    // ── Fallible write surface (DoS-safe direct decode path) ──
    //
    // Parallel `try_*` methods that return `Err(BackendOverflow)`
    // instead of panicking when the write would exceed the backend's
    // capacity. Wired across EVERY direct-decode write path: Raw / RLE
    // blocks (`try_extend` / `try_extend_and_fill`), the Compressed
    // block's sequence executor (`exec_sequence_inline` returns
    // `Result`, the fallback chain uses `try_push` +
    // `repeat_lookahead_prefetched`, tail literals use `try_push`), and
    // the match-repeat pre-check (`try_reserve`). Used by the
    // direct-decode path (`decode_all` + descendants) so a malformed
    // block whose decompressed payload exceeds the caller-provided
    // output slice surfaces as a structured
    // `FrameDecoderError::FrameContentSizeMismatch` instead of an abort
    // — uniformly for Raw, RLE, and Compressed blocks (see
    // `FrameDecoder::run_direct_decode`, which folds the Compressed
    // sequence-executor `OutputBufferOverflow` into the same
    // `FrameContentSizeMismatch` contract as the Raw/RLE
    // `BackendOverflow` arm).
    //
    // The growable backends (`FlatBuf`, `RingBuffer`) rely on the
    // default impls below — they delegate to the corresponding
    // panic-on-overflow method (`extend`, `extend_and_fill`,
    // `extend_from_within_unchecked`) and always return `Ok(())`.
    // Those underlying methods grow the backing `Vec` on demand, so
    // there is no capacity-mismatch case to surface as `Err`. No
    // per-backend `try_*` impl exists on `FlatBuf` / `RingBuffer`
    // because the default behaviour is exactly what they want.
    //
    // The fixed-capacity backend (`UserSliceBackend`) overrides each
    // method with an explicit capacity check that returns `Err` on
    // overshoot instead of panicking. The trade-off is one branch
    // per write on the direct-decode path.

    /// Fallible variant of [`Self::extend`].
    /// Returns `Err(BackendOverflow)` on fixed-capacity backends
    /// (`UserSliceBackend`) when the write would exceed the slice
    /// length. Growable backends (FlatBuf / RingBuffer) cannot
    /// return `Err` for capacity reasons — their underlying `Vec`
    /// grows on demand, and a true allocation failure aborts the
    /// process rather than surfacing through `Result` (`Vec`
    /// contract). Default impl delegates to the panic-on-overflow
    /// [`Self::extend`] — backends with non-growable capacity MUST
    /// override.
    fn try_extend(&mut self, data: &[u8]) -> Result<(), BackendOverflow> {
        self.extend(data);
        Ok(())
    }

    /// Fallible variant of [`Self::extend_and_fill`]. Same contract
    /// as [`Self::try_extend`].
    fn try_extend_and_fill(
        &mut self,
        fill_with: u8,
        fill_length: usize,
    ) -> Result<(), BackendOverflow> {
        self.extend_and_fill(fill_with, fill_length);
        Ok(())
    }

    /// Fallible variant of [`Self::extend_from_within_unchecked`].
    /// Validates `start + len <= self.len()` (source bound) and then
    /// `reserve(len)` to grow capacity for the write. The default
    /// impl deliberately omits a linear `tail + len <= cap` check
    /// because `RingBuffer::tail` is a modular wrap-index where
    /// `tail + len > cap` is normal mid-stream (the write straddles
    /// the wrap point). Fixed-capacity backends (`UserSliceBackend`)
    /// override with an explicit linear capacity check that DOES
    /// validate `tail + len <= cap`. On `Err` the backend state is
    /// untouched.
    ///
    /// Unlike the unsafe variant, this is a SAFE entry point: the
    /// bounds check moves into the method, so callers don't need to
    /// satisfy the `Self::extend_from_within_unchecked` safety
    /// contract at the call site.
    ///
    /// NOTE: Retained for the SAFE-surface test matrix and as the
    /// wrap-aware reference impl; not called on production paths
    /// (hence `#[allow(dead_code)]`). The Compressed-block direct path
    /// is already DoS-safe WITHOUT this method: its match-repeat copies
    /// go through `DecodeBuffer::repeat_lookahead_prefetched`, which
    /// pre-checks capacity via [`Self::try_reserve`] before the
    /// unchecked wildcopy, and its literal+match sequence copies go
    /// through `exec_sequence_inline` (returns `Result`). RLE/Raw use
    /// `try_extend_and_fill` / `try_extend`. So every adversarial
    /// overshoot already surfaces as a structured error.
    #[allow(dead_code)]
    fn try_extend_from_within(&mut self, start: usize, len: usize) -> Result<(), BackendOverflow> {
        // Default impl: a SAFE method must NOT delegate to the
        // unsafe variant without validating its safety contract.
        // Validate the source range (`start + len <= self.len()`),
        // then `reserve(len)` to guarantee destination capacity
        // (growable-backend invariant — see the linear vs wrap-aware
        // discussion below). NO eager `tail + len <= cap` check
        // because `RingBuffer::tail` is a modular wrap-index where
        // `tail + len > cap` is normal mid-stream. Fixed-capacity
        // backends (`UserSliceBackend`) override with their own
        // wrap-unaware linear capacity check.
        let tail = self.tail();
        let capacity = self.cap();
        let src_end = start.checked_add(len).ok_or(BackendOverflow {
            tail,
            requested: len,
            capacity,
        })?;
        if src_end > self.len() {
            return Err(BackendOverflow {
                tail,
                requested: len,
                capacity,
            });
        }
        // Growth + linear destination bound:
        //
        // `reserve(len)` is the growable-backend invariant — after
        // it returns, the backend has room for `len` more bytes.
        // For `FlatBuf` that's a linear `Vec::reserve`; for
        // `RingBuffer` it's a wrap-aware grow that maintains the
        // ring invariant. EITHER way, the only check needed by the
        // default impl is the `start + len` source bound above —
        // capacity for the write is guaranteed by `reserve`.
        //
        // We deliberately do NOT add a `tail + len <= cap` check
        // here: `RingBuffer::tail` is a modular index that wraps,
        // so a `tail + len > cap` situation is normal mid-stream
        // (the write straddles the wrap and lands at the head end).
        // An eager linear check would reject valid wrap writes and
        // return `Err(BackendOverflow)` on inputs the underlying
        // `extend_from_within_unchecked` would handle correctly.
        // Fixed-capacity backends (`UserSliceBackend`) override
        // `try_extend_from_within` with their own non-wrap-aware
        // capacity check.
        self.reserve(len);
        // SAFETY: source bound `start + len <= self.len()` checked
        // above; destination capacity guaranteed by the just-called
        // `reserve(len)`, both linear (FlatBuf) and wrap-aware
        // (RingBuffer). Wrap-unaware fixed-capacity backends
        // override this method.
        unsafe { self.extend_from_within_unchecked(start, len) };
        Ok(())
    }
}

/// Backend write failed. Surfaced only by fallible `try_*` methods
/// on fixed-capacity backends (`UserSliceBackend`); growable backends
/// (`FlatBuf`, `RingBuffer`) never produce this — they grow instead.
///
/// Covers three distinct failure modes on `UserSliceBackend`:
/// 1. **Destination capacity overshoot** — `tail + len > slice.len()`:
///    the new tail would exceed the caller's output slice.
/// 2. **Arithmetic overflow** — `tail.checked_add(len)` overflowed
///    (or `head.checked_add(start)` in `try_extend_from_within`):
///    adversarial `len` near `usize::MAX` triggers the wrap-guard
///    `ok_or` branch.
/// 3. **Source-range violation** (`try_extend_from_within` only) —
///    `abs_end > self.tail`: the requested match-copy source range
///    extends past the live region.
///
/// All three modes return the same struct shape so the caller doesn't
/// need to discriminate; `tail` / `requested` / `capacity` carry the
/// diagnostic context. The decoder converts this into one of two
/// structured variants on the way out of `FrameDecoder`:
/// `ExecuteSequencesError::OutputBufferOverflow` (literal-push and
/// upstream zstd-inline paths inside the sequence executor) or
/// `DecodeBufferError::OutputBufferOverflow` (the match-repeat
/// `try_reserve` pre-check inside `DecodeBuffer::repeat_inner`).
/// On the direct-decode path both are folded by
/// `FrameDecoder::run_direct_decode` into
/// `FrameDecoderError::FrameContentSizeMismatch` — the same
/// caller-visible error a Raw / RLE overshoot yields, so the
/// "content exceeded declared size" contract is uniform across block
/// types. Callers never see `BackendOverflow` directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BackendOverflow {
    /// Current physical write cursor at the moment the write was
    /// attempted.
    pub tail: usize,
    /// Number of bytes the failing write tried to append.
    pub requested: usize,
    /// Total physical capacity of the backend.
    pub capacity: usize,
}

impl core::fmt::Display for BackendOverflow {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "BufferBackend overflow: tail={}, requested={}, capacity={}",
            self.tail, self.requested, self.capacity,
        )
    }
}
