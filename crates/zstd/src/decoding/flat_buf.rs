#![allow(
    clippy::inline_always,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Vec-backed flat output buffer for the "frame fits in window" fast path.
//!
//! When the frame's `Single_Segment_flag` is set the decompressed output
//! never exceeds `window_size`, the ring layout never wraps, and the
//! whole `DecodeBuffer` surface collapses to a growing `Vec<u8>` plus a
//! logical head index for streamed drains. Skipping the ring buffer's
//! wrap-dispatch on every push/repeat/drain is the win this module is
//! targeted at.
//!
//! Selected at compile time via `DecodeBuffer<FlatBuf>` (generic
//! [`BufferBackend`](super::buffer_backend::BufferBackend)
//! parameter). The earlier `enum BufferStorage { Ring, Flat }` attempt
//! paid runtime match overhead in every hot-path entry and measured a
//! +43–58 % regression on small-frame decompress — generic mono-
//! morphisation strips that match at compile time per call site.

use crate::io::{Error, Read};
use alloc::vec::Vec;

use super::buffer_backend::{BackendOverflow, BufferBackend, WILDCOPY_OVERLENGTH};

pub(crate) struct FlatBuf {
    buf: Vec<u8>,
    /// Bytes in `buf[..head]` have already been handed to the
    /// output sink and are no longer visible through the
    /// [`BufferBackend`] surface (`len`, `as_slices`,
    /// `extend_from_within_unchecked` all index relative to `head`).
    /// They live on physically in the allocation because the linear
    /// `Vec` layout never reuses that region — discarding them would
    /// require a memmove of the active window.
    ///
    /// Scope: `FlatBuf` is selected by `DecodeBuffer<FlatBuf>` only
    /// for frames whose `FrameHeader.descriptor.single_segment_flag()`
    /// is set. Such frames decode in a single segment of exactly
    /// `frame_content_size` bytes and never trigger
    /// `drain_to_window_size_writer` mid-stream — drain (and the
    /// corresponding `drop_first_n` head advance) only happens at
    /// end-of-frame. The "drained prefix no longer visible to
    /// `repeat`" semantics therefore match `RingBuffer`'s
    /// behaviour for the same call shape (both backends expose only
    /// `head..tail` through `len`/`as_slices`), and the FlatBuf
    /// path can't observe a streaming-drain scenario where the
    /// distinction would matter.
    head: usize,
    /// Per-block decompression-bomb growth ceiling, mirroring
    /// [`super::ringbuffer::RingBuffer`]. `usize::MAX` is unbounded; the
    /// sequence decoder lowers it to `len + MAX_BLOCK_SIZE` per block via
    /// [`BufferBackend::set_max_capacity`]. `FlatBuf` is growable (the inline
    /// exec path is capacity-bounded, but the fallback `push`/`repeat` route
    /// grows through `try_reserve`), so it must honour this ceiling to bound a
    /// malformed single-segment block's output instead of growing toward OOM.
    max_capacity: usize,
}

impl BufferBackend for FlatBuf {
    /// FlatBuf opts into the upstream zstd-shape inline `exec_sequence_inline`
    /// path on every target: x86_64 via the SSE2
    /// `exec_sequence_inline::x86` module, all other ISAs via the
    /// architecture-agnostic `portable` module (the `cfg(not(x86_64))`
    /// arm below). FlatBuf is selected for single-segment frames
    /// (frame_content_size known up-front, single block of
    /// literals+matches). Production sizing goes through `reserve`
    /// (which adds `WILDCOPY_OVERLENGTH`); the SIMD overshoot the inline
    /// path may emit is additionally bounded by the tight-tail copy in
    /// the inline-exec sites, so a tight buffer never overshoots. Both
    /// arms are gated on this const, which is
    /// unconditionally `true` because FlatBuf provides an override for
    /// every target.
    const SUPPORTS_INLINE_SEQUENCE_EXEC: bool = true;

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
        // Fallible capacity check. The caller's per-block
        // `reserve(MAX_BLOCK_SIZE)` plus the `WILDCOPY_OVERLENGTH`
        // slack `reserve` adds covers well-formed frames,
        // but a malformed sequence stream can produce a
        // `lit_length + match_length` that exceeds the reserved
        // headroom. Surface that as `OutputBufferOverflow` (mirrors
        // `UserSliceBackend::exec_sequence_inline`) so the safe
        // public decode APIs see a structured error instead of UB
        // from writing past `Vec::capacity()`.
        const MAX_WILDCOPY_OVERSHOOT: usize = 15;
        let cap = self.buf.capacity();
        let buf_len = self.buf.len();
        // `buf.len() <= buf.capacity()` (a `Vec` invariant) satisfies the
        // `tail <= cap` precondition; see `sequence_output_fits`.
        let total = sequence_output_fits(
            lit_length,
            match_length,
            buf_len,
            cap,
            MAX_WILDCOPY_OVERSHOOT,
        )?;
        debug_assert!(offset >= 1);
        debug_assert!(match_length >= 1);
        let live_len = buf_len - self.head;
        debug_assert!(
            live_len + lit_length >= offset,
            "FlatBuf::exec_sequence_inline: offset {offset} exceeds live window",
        );

        unsafe {
            let base_mut = self.buf.as_mut_ptr();

            // Literal copy: upstream zstd `ZSTD_copy16` + optional wildcopy tail.
            let op_lit = base_mut.add(buf_len);
            copy16(op_lit, lit_src);
            if lit_length > 16 {
                wildcopy_no_overlap(op_lit.add(16), lit_src.add(16), lit_length - 16);
            }

            // Match copy.
            let op_match = base_mut.add(buf_len + lit_length);
            let match_src = base_mut.cast_const().add(buf_len + lit_length - offset);

            if offset >= 16 {
                wildcopy_no_overlap(op_match, match_src, match_length);
            } else {
                let (op2, ip2) = overlap_copy8(op_match, match_src, offset);
                if match_length > 8 {
                    wildcopy_overlap_8byte_stride(op2, ip2, match_length - 8);
                }
            }

            // Bump len. Capacity asserted above; this is safe.
            self.buf.set_len(buf_len + total);
        }
        Ok(())
    }

    /// Non-x86 port of [`Self::exec_sequence_inline`] — identical upstream zstd
    /// `ZSTD_execSequence` shape, but the wildcopy helpers come from the
    /// portable module (16-byte `u128` / 8-byte `u64` unaligned moves,
    /// lowered to NEON `ldr q`/`str q` on aarch64 and the widest store
    /// available elsewhere). Without this arm the non-x86 decode path
    /// fell through to the slow `try_push` + `repeat` trait chain; the
    /// inline form cuts the match-copy cost that dominates match-heavy
    /// decode.
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
        // Fallible capacity check mirrors the x86 arm: the 16-byte
        // wildcopy overshoots up to 15 bytes past `tail + total`, which
        // the `WILDCOPY_OVERLENGTH` slack `reserve` adds covers for
        // well-formed frames; malformed input surfaces as
        // `OutputBufferOverflow` instead of a write past capacity.
        const MAX_WILDCOPY_OVERSHOOT: usize = 15;
        let cap = self.buf.capacity();
        let buf_len = self.buf.len();
        // `buf.len() <= buf.capacity()` (a `Vec` invariant) satisfies the
        // `tail <= cap` precondition; see `sequence_output_fits`.
        let total = sequence_output_fits(
            lit_length,
            match_length,
            buf_len,
            cap,
            MAX_WILDCOPY_OVERSHOOT,
        )?;
        debug_assert!(offset >= 1);
        debug_assert!(match_length >= 1);
        let live_len = buf_len - self.head;
        debug_assert!(
            live_len + lit_length >= offset,
            "FlatBuf::exec_sequence_inline: offset {offset} exceeds live window",
        );

        // SAFETY: capacity check above guarantees writes (plus the
        // ≤ 15-byte wildcopy overshoot) stay within `buf.capacity()`;
        // `live_len + lit_length >= offset` keeps the match source
        // in-bounds. Same invariants the x86 arm relies on.
        unsafe {
            let base_mut = self.buf.as_mut_ptr();

            let op_lit = base_mut.add(buf_len);
            copy16(op_lit, lit_src);
            if lit_length > 16 {
                wildcopy_no_overlap(op_lit.add(16), lit_src.add(16), lit_length - 16);
            }

            let op_match = base_mut.add(buf_len + lit_length);
            let match_src = base_mut.cast_const().add(buf_len + lit_length - offset);

            if offset >= 16 {
                wildcopy_no_overlap(op_match, match_src, match_length);
            } else {
                let (op2, ip2) = overlap_copy8(op_match, match_src, offset);
                if match_length > 8 {
                    wildcopy_overlap_8byte_stride(op2, ip2, match_length - 8);
                }
            }

            self.buf.set_len(buf_len + total);
        }
        Ok(())
    }

    /// AVX2-tier override — same shape as [`Self::exec_sequence_inline`]
    /// but the no-overlap match-copy uses 32-byte ymm wildcopy via
    /// `wildcopy_no_overlap_avx2` when `offset >= 32`. Mid-offset range
    /// (16..=31) keeps the SSE2 16-byte stride for correctness (32-byte
    /// load at offset 16..31 would read uninitialised destination
    /// bytes; same bound as the `UserSliceBackend::exec_sequence_inline_avx2`
    /// override).
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
        // Fallible capacity check. AVX2 32-byte stride overshoots up
        // to 31 bytes past `tail + total`; the
        // `WILDCOPY_OVERLENGTH = 32` slack `reserve` adds covers
        // well-formed frames, but malformed inputs that exceed the
        // reserved headroom surface as `OutputBufferOverflow` instead
        // of UB.
        const MAX_WILDCOPY_OVERSHOOT: usize = 31;
        let cap = self.buf.capacity();
        let buf_len = self.buf.len();
        // `buf.len() <= buf.capacity()` (a `Vec` invariant) satisfies the
        // `tail <= cap` precondition; see `sequence_output_fits`.
        let total = sequence_output_fits(
            lit_length,
            match_length,
            buf_len,
            cap,
            MAX_WILDCOPY_OVERSHOOT,
        )?;
        debug_assert!(offset >= 1);
        debug_assert!(match_length >= 1);
        let live_len = buf_len - self.head;
        debug_assert!(
            live_len + lit_length >= offset,
            "FlatBuf::exec_sequence_inline_avx2: offset {offset} exceeds live window",
        );

        unsafe {
            let base_mut = self.buf.as_mut_ptr();

            // Literal copy stays on SSE2 16-byte — caller-side
            // inline-path slack gate is 16-byte literal bound.
            let op_lit = base_mut.add(buf_len);
            copy16(op_lit, lit_src);
            if lit_length > 16 {
                wildcopy_no_overlap(op_lit.add(16), lit_src.add(16), lit_length - 16);
            }

            // Match copy — divergent on no-overlap fast path.
            let op_match = base_mut.add(buf_len + lit_length);
            let match_src = base_mut.cast_const().add(buf_len + lit_length - offset);

            if offset >= 32 {
                wildcopy_no_overlap_avx2(op_match, match_src, match_length);
            } else if offset >= 16 {
                wildcopy_no_overlap(op_match, match_src, match_length);
            } else {
                let (op2, ip2) = overlap_copy8(op_match, match_src, offset);
                if match_length > 8 {
                    wildcopy_overlap_8byte_stride(op2, ip2, match_length - 8);
                }
            }

            self.buf.set_len(buf_len + total);
        }
        Ok(())
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    unsafe fn inline_exec_base_ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr()
    }

    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    unsafe fn inline_exec_commit(&mut self, new_tail: usize) {
        // The macro wrote `[buf.len(), new_tail)`; grow the Vec to expose it.
        // SAFETY: new_tail <= capacity (sequence_output_fits validated it) and
        // `[0, new_tail)` is now initialised.
        unsafe { self.buf.set_len(new_tail) };
    }

    fn new() -> Self {
        Self {
            buf: Vec::new(),
            head: 0,
            max_capacity: usize::MAX,
        }
    }

    #[inline]
    fn clear(&mut self) {
        self.buf.clear();
        self.head = 0;
    }

    #[inline]
    fn reserve(&mut self, n: usize) {
        // `Vec::reserve(additional)` guarantees
        // `capacity >= len + additional`; passing
        // `n + WILDCOPY_OVERLENGTH` is the exact contract callers
        // need (room for `n` bytes plus the SIMD overshoot slack).
        //
        // Previous attempts computed the reserve amount as
        // `(n - available)` or `(needed - capacity)`, both of which
        // under-reserve when `len > 0`. Concrete repro: on a
        // multi-frame stream where frame 2 has `window_size > frame
        // 1's capacity` and `len == 0` post-reset, `available ==
        // old_capacity`, so `additional = (n - old_capacity) +
        // slack`; `Vec::reserve` then only ensures
        // `new_capacity >= len + additional = (n - old_capacity) +
        // slack`, which is short by `old_capacity`. Subsequent
        // `extend_from_within_unchecked` then panicked on the
        // `dst_off + len <= capacity` debug assert.
        // libFuzzer artifact crash-e33ba082… exercises exactly that
        // shape.
        //
        // `checked_add` (not `saturating_add`): callers reserve a bounded
        // amount (a block's `MAX_BLOCK_SIZE`, or a `try_reserve` amount already
        // gated by the per-block ceiling), so `n + WILDCOPY_OVERLENGTH` cannot
        // overflow in practice. If it ever did, saturating to `usize::MAX`
        // would silently mask the bad input before `Vec::reserve` panicked on
        // it anyway — fail loudly at the cause instead.
        let additional = n
            .checked_add(WILDCOPY_OVERLENGTH)
            .expect("FlatBuf::reserve amount + wildcopy slack overflows usize");
        self.buf.reserve(additional);
    }

    #[inline]
    fn reserve_exact(&mut self, n: usize) {
        // Same contract as `reserve` (n additional + wildcopy slack), but
        // exact growth: the window pre-reservation must not double a
        // window-sized buffer when a dictionary prefix already occupies
        // part of the retained capacity.
        let additional = n
            .checked_add(WILDCOPY_OVERLENGTH)
            .expect("FlatBuf::reserve_exact amount + wildcopy slack overflows usize");
        self.buf.reserve_exact(additional);
    }

    #[inline]
    fn set_max_capacity(&mut self, max_capacity: usize) {
        self.max_capacity = max_capacity;
    }

    #[inline]
    fn try_reserve(&mut self, n: usize) -> Result<(), BackendOverflow> {
        // Enforce the per-block ceiling FIRST, before the no-growth fast path:
        // the ceiling bounds this block's OUTPUT, not just allocation, so a
        // write that would push live output past it must be rejected even when
        // it fits the current capacity (e.g. a large pre-reserved FCS buffer
        // whose spare exceeds `MAX_BLOCK_SIZE`). This is where the
        // decompression bomb is bounded on FlatBuf's fallback push/repeat path
        // (the inline exec path is already capacity-bounded by
        // `sequence_output_fits`). `max_capacity = usize::MAX` between blocks
        // makes this a no-op for unbounded callers.
        if self
            .len()
            .checked_add(n)
            .is_none_or(|needed| needed > self.max_capacity)
        {
            return Err(BackendOverflow {
                tail: self.buf.len(),
                requested: n,
                capacity: self.max_capacity,
            });
        }
        // Within the ceiling: if the live region plus this write (and the
        // wildcopy slack `reserve` adds) already fits the current allocation,
        // no growth is needed. `capacity >= len` is a `Vec` invariant, so the
        // subtraction cannot underflow.
        let free = self.buf.capacity() - self.buf.len();
        if n.checked_add(WILDCOPY_OVERLENGTH)
            .is_some_and(|need| free >= need)
        {
            return Ok(());
        }
        self.reserve(n);
        Ok(())
    }

    #[inline]
    fn len(&self) -> usize {
        self.buf.len() - self.head
    }

    #[inline]
    fn cap(&self) -> usize {
        self.buf.capacity()
    }

    #[inline]
    fn tail(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    unsafe fn set_tail(&mut self, new_tail: usize) {
        debug_assert!(new_tail >= self.head);
        debug_assert!(new_tail <= self.buf.len());
        // SAFETY: forwarded to Vec::set_len. `new_tail` must come
        // from a previous `tail()` on this same instance (the
        // checkpoint's cap snapshot guarantees no realloc), so the
        // bytes re-exposed in `0..new_tail` were already written and
        // are initialised. Bytes between `new_tail` and the prior
        // tail are discarded by the caller per
        // `BufferBackend::set_tail` and never read again. The
        // trailing slack region past `buf.len()` is intentionally
        // uninitialised (see `with_capacity`) and never read by any
        // FlatBuf code path.
        unsafe { self.buf.set_len(new_tail) };
    }

    #[inline]
    fn extend(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    #[inline]
    fn extend_and_fill(&mut self, fill_with: u8, fill_length: usize) {
        let new_len = self.buf.len() + fill_length;
        self.buf.resize(new_len, fill_with);
    }

    fn extend_from_reader<R: Read>(
        &mut self,
        mut read: R,
        fill_length: usize,
    ) -> Result<(), Error> {
        // Forming `&mut [u8]` over uninitialised `Vec` spare
        // capacity is UB even before any write — `&mut T` must
        // always reference initialised, valid memory of the target
        // type. Initialise via `Vec::resize(.., 0)` first, then
        // hand the resulting initialised slice to `read_exact`.
        // The earlier "read straight into spare capacity to skip
        // the zero-fill" shape traded soundness for a ~one-memset-
        // per-128-KiB-raw-block win; not worth the UB.
        // On read failure, truncate the Vec back to its pre-call
        // length so observable behaviour matches the previous
        // truncate-on-error shape.
        let old = self.buf.len();
        let new_len = old + fill_length;
        // Routes through `BufferBackend::reserve`, which keeps the
        // `WILDCOPY_OVERLENGTH` slack invariant uniform with
        // `with_capacity` / inline `reserve` growth paths.
        self.reserve(fill_length);
        self.buf.resize(new_len, 0);
        let read_slot = &mut self.buf[old..new_len];
        match read.read_exact(read_slot) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.buf.truncate(old);
                Err(e)
            }
        }
    }

    #[inline]
    unsafe fn extend_from_within_unchecked(&mut self, start: usize, len: usize) {
        let dst_off = self.buf.len();
        let src_off = self.head + start;
        debug_assert!(src_off + len <= dst_off);
        debug_assert!(dst_off + len <= self.buf.capacity());
        // Route through `simd_copy::copy_bytes_overshooting` so short
        // match copies (the common L-1 fast pattern) hit the inline
        // SIMD / overlapping-u64 fast paths instead of going to
        // libc `__memmove_avx_unaligned_erms` via
        // `ptr::copy_nonoverlapping`. The dispatch cost was 40% of
        // decode CPU on the L-1 c_stream flamegraph.
        let total_readable = self.buf.len() - src_off;
        let total_writable = self.buf.capacity() - dst_off;
        // SAFETY: caller's non-overlap precondition gives
        // `src_off + len <= dst_off`. `total_readable >= len` since
        // `src_off + len <= dst_off <= self.buf.len()`.
        // `total_writable >= len` because Vec capacity covers the
        // upfront reserve. The helper may overshoot up to
        // `total_writable` (= cap - dst_off, which includes the
        // WILDCOPY_OVERLENGTH slack `reserve` adds).
        unsafe {
            let base = self.buf.as_mut_ptr();
            super::simd_copy::copy_bytes_overshooting(
                (base.add(src_off), total_readable),
                (base.add(dst_off), total_writable),
                len,
            );
            self.buf.set_len(dst_off + len);
        }
    }

    #[inline]
    unsafe fn extend_from_within_unchecked_branchless(&mut self, start: usize, len: usize) {
        // Flat layout never has overlap concerns the branchless variant
        // was designed for — forward to the single non-overlapping copy.
        // SAFETY: forwarded.
        unsafe { self.extend_from_within_unchecked(start, len) }
    }

    #[inline]
    fn as_slices(&self) -> (&[u8], &[u8]) {
        (&self.buf[self.head..], &[])
    }

    #[inline]
    fn drop_first_n(&mut self, n: usize) {
        self.head += n;
        debug_assert!(self.head <= self.buf.len());
    }
}
