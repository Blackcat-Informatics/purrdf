#![allow(
    clippy::inline_always,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Shared helpers and constants consumed by every match finder (Simple
//! / Dfast / Row / Hc). Matchers use their own modules without
//! duplicating these primitives.
//!
//! Phase 1c additionally hosts `common_prefix_len` and the shared
//! match-finder constants (`MIN_MATCH_LEN`, `FAST_HASH_FILL_STEP`,
//! `INCOMPRESSIBLE_SKIP_STEP`) here so the per-matcher modules
//! (`simple`, `dfast`, `row`) depend on this shared infra rather than on
//! each other — breaking the `match_generator` ↔ `simple` reverse
//! dependency the original Phase 1c split introduced.
//!
//! Mechanical move — names, signatures, and bodies are byte-for-byte
//! identical to the pre-extraction monolith; the only intentional API
//! change is `pub(crate)` visibility on the moved items.

use super::super::opt::types::MatchCandidate;
use crate::encoding::fastpath::FastpathKernel;

/// Minimum match length emitted by the Simple / level-1 matcher (and the
/// upstream zstd `ZSTD_fast.c` baseline). Lives here so every matcher and the
/// `match_generator` driver can reference the shared minimum without
/// importing one matcher module from another.
pub(crate) const MIN_MATCH_LEN: usize = 5;
/// Hash-fill stride used by the Simple / level-1 fast pass when
/// backfilling the suffix store. Upstream zstd parity: matches
/// `ZSTD_FAST_HASH_FILL_STEP` in `zstd_fast.c`.
pub(crate) const FAST_HASH_FILL_STEP: usize = 3;
/// Sparse step used when a block was determined to be incompressible —
/// every matcher inserts hash entries with this stride instead of the
/// per-byte dense pattern so the rest of the block costs less CPU.
pub(crate) const INCOMPRESSIBLE_SKIP_STEP: usize = 8;

/// Length of the common prefix of two byte slices, capped at
/// `min(a.len(), b.len())`. Hot path on every match finder; dispatches to
/// the per-CPU `common_prefix_len_ptr` kernel (NEON / SSE4.2 / AVX2+BMI2 /
/// scalar) so the SIMD/CRC heavy lifting lives in one place. See
/// [`crate::encoding::fastpath`] for the per-CPU implementations.
#[inline(always)]
pub(crate) fn common_prefix_len(a: &[u8], b: &[u8]) -> usize {
    let max = a.len().min(b.len());
    // SAFETY: slice `a` / `b` guarantee at least their `len()` initialized
    // bytes; `max` is the minimum so both pointers are valid for `max`
    // bytes.
    unsafe {
        crate::encoding::fastpath::dispatch_common_prefix_len_ptr(a.as_ptr(), b.as_ptr(), max)
    }
}

/// As [`common_prefix_len`] but against an already-resolved [`FastpathKernel`].
/// Hot match-finder loops resolve the kernel once per block and pass it in,
/// so each byte-compare skips the per-call `select_kernel()` `OnceLock` atomic.
#[inline(always)]
pub(crate) fn common_prefix_len_with_kernel(kernel: FastpathKernel, a: &[u8], b: &[u8]) -> usize {
    let max = a.len().min(b.len());
    // SAFETY: as `common_prefix_len` — both pointers are valid for `max` bytes.
    unsafe {
        crate::encoding::fastpath::dispatch_common_prefix_len_ptr_with_kernel(
            kernel,
            a.as_ptr(),
            b.as_ptr(),
            max,
        )
    }
}

/// Pick the better of two candidate matches: longer wins, ties go to
/// the smaller offset (cheaper to encode and better for decompression
/// throughput).
pub(crate) fn best_len_offset_candidate(
    lhs: Option<MatchCandidate>,
    rhs: Option<MatchCandidate>,
) -> Option<MatchCandidate> {
    match (lhs, rhs) {
        (None, other) | (other, None) => other,
        (Some(lhs), Some(rhs)) => {
            if rhs.match_len > lhs.match_len
                || (rhs.match_len == lhs.match_len && rhs.offset < lhs.offset)
            {
                Some(rhs)
            } else {
                Some(lhs)
            }
        }
    }
}

/// Walk a candidate match backwards over the literal run so the matcher
/// can absorb literal bytes that happen to match the byte preceding the
/// candidate. Upstream zstd parity: equivalent to the back-extend done inside
/// every match finder before committing a sequence.
#[inline]
pub(crate) fn extend_backwards_shared(
    concat: &[u8],
    history_abs_start: usize,
    mut candidate_pos: usize,
    mut abs_pos: usize,
    mut match_len: usize,
    lit_len: usize,
) -> MatchCandidate {
    let min_abs_pos = abs_pos - lit_len;
    let concat_ptr = concat.as_ptr();
    let concat_len = concat.len();
    // SAFETY: loop guard `candidate_pos > history_abs_start` and
    // `abs_pos > min_abs_pos` keep both `candidate_pos - history_abs_start - 1`
    // and `abs_pos - history_abs_start - 1` strictly positive (no underflow).
    // Their upper bound is `concat.len() - 1` because both `candidate_pos` and
    // `abs_pos` point at currently-live history. Asserted in debug builds.
    while abs_pos > min_abs_pos && candidate_pos > history_abs_start {
        let cand_off = candidate_pos - history_abs_start - 1;
        let cur_off = abs_pos - history_abs_start - 1;
        debug_assert!(cand_off < concat_len && cur_off < concat_len);
        let cand_byte = unsafe { *concat_ptr.add(cand_off) };
        let cur_byte = unsafe { *concat_ptr.add(cur_off) };
        if cand_byte != cur_byte {
            break;
        }
        candidate_pos -= 1;
        abs_pos -= 1;
        match_len += 1;
    }
    MatchCandidate {
        start: abs_pos,
        offset: abs_pos - candidate_pos,
        match_len,
    }
}

/// Probe the 3 rep-code offsets (with the upstream zstd `ll0` ↦ `rep[0] - 1`
/// fallback) and return the best match found, if any. Hot path:
/// invoked once per encoded byte on lazy / Dfast / Row matchers
/// (~10% exclusive on the default-level profile).
#[inline(always)]
pub(crate) fn repcode_candidate_shared(
    kernel: FastpathKernel,
    concat: &[u8],
    history_abs_start: usize,
    offset_hist: [u32; 3],
    abs_pos: usize,
    lit_len: usize,
    min_match_len: usize,
) -> Option<MatchCandidate> {
    let current_idx = abs_pos - history_abs_start;
    if current_idx + min_match_len > concat.len() {
        return None;
    }
    // The 4-byte gate below relies on a first-4-byte mismatch implying the
    // match can never reach the acceptance floor.
    debug_assert!(min_match_len >= 4, "repcode gate requires min_match >= 4");

    // Called once per input byte (10% exclusive on default-level profile).
    // The previous form built an `[Option<usize>; 3]` and walked it via
    // `into_iter().flatten()`, which the compiler couldn't always unroll
    // through the conditional `then_some` on the last slot. Unroll the
    // 3-rep probe by hand: each branch is a couple of compares + one
    // `common_prefix_len` (already SIMD).
    let mut best: Option<MatchCandidate> = None;

    let (rep0, rep1, rep2_opt) = if lit_len == 0 {
        let r2 = if offset_hist[0] > 1 {
            Some(offset_hist[0] as usize - 1)
        } else {
            None
        };
        (offset_hist[1] as usize, offset_hist[2] as usize, r2)
    } else {
        (
            offset_hist[0] as usize,
            offset_hist[1] as usize,
            Some(offset_hist[2] as usize),
        )
    };

    macro_rules! probe {
        ($rep:expr) => {{
            let rep = $rep;
            if rep != 0 && rep <= abs_pos {
                let candidate_pos = abs_pos - rep;
                // Upstream zstd `MEM_read32` rep gate (`zstd_lazy.c` lazy loop): a
                // first-4-byte mismatch can never reach the
                // `>= min_match_len` floor (asserted >= 4 above), so reject
                // on one scalar compare instead of paying the SIMD count
                // call on every non-matching rep. In-bounds: `current_idx +
                // 4 <= concat.len()` from the entry guard, and
                // `candidate_idx < current_idx`.
                if candidate_pos >= history_abs_start {
                    let candidate_idx = candidate_pos - history_abs_start;
                    // Upstream zstd `MEM_read32` gate: one unaligned u32 compare. The
                    // slice-range form paid two bounds checks per rep probe
                    // (per input byte on the greedy/lazy levels).
                    // SAFETY: the entry guard established `current_idx +
                    // min_match_len <= concat.len()` with `min_match_len >= 4`
                    // (debug-asserted above), and `candidate_idx <
                    // current_idx`, so both 4-byte reads are in bounds.
                    let gate = unsafe {
                        let base = concat.as_ptr();
                        core::ptr::read_unaligned(base.add(candidate_idx).cast::<u32>())
                            == core::ptr::read_unaligned(base.add(current_idx).cast::<u32>())
                    };
                    if gate {
                        let match_len = common_prefix_len_with_kernel(
                            kernel,
                            &concat[candidate_idx..],
                            &concat[current_idx..],
                        );
                        if match_len >= min_match_len {
                            let candidate = extend_backwards_shared(
                                concat,
                                history_abs_start,
                                candidate_pos,
                                abs_pos,
                                match_len,
                                lit_len,
                            );
                            best = best_len_offset_candidate(best, Some(candidate));
                        }
                    }
                }
            }
        }};
    }

    probe!(rep0);
    probe!(rep1);
    if let Some(rep2) = rep2_opt {
        probe!(rep2);
    }
    best
}
