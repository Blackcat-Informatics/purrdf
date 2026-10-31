#![allow(
    clippy::inline_always,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Forward match-length counter — direct port of upstream zstd's `ZSTD_count`
//! from `lib/compress/zstd_compress_internal.h`. Compares `pIn` against
//! `pMatch` in `usize`-sized chunks via XOR, falls back to a u32 / u16
//! / u8 tail. The first mismatching byte is located via
//! `trailing_zeros()/8` (little-endian) or `leading_zeros()/8`
//! (big-endian) on the XOR difference, matching upstream zstd's
//! `ZSTD_NbCommonBytes`.
//!
//! The chunk type is `usize` — `u64` on 64-bit hosts, `u32` on 32-bit —
//! to match upstream zstd's `MEM_readST` (`size_t`) loads. On 32-bit targets a
//! u64 chunk would compile to two adjacent 32-bit loads + a 64-bit
//! XOR; using native pointer width keeps the inner loop a single load
//! per pointer per iteration.

/// Count the number of bytes that match starting at `ip` against the
/// reference at `match_ptr`, up to (but not including) `iend`. Returns
/// the match length in bytes — `0` if `*ip != *match_ptr`.
///
/// # Safety
///
/// - `ip` MUST point to `ip_len = (iend as usize) - (ip as usize)`
///   readable bytes. `iend` is the exclusive upper bound; the function
///   never reads at or past it.
/// - `match_ptr` MUST point to at least as many readable bytes as `ip`
///   does up to `iend`. In practice this is naturally satisfied when
///   `match_ptr <= ip` and both pointers live inside the same buffer
///   (a backward match into the encoder's history), since the function
///   only reads chunks from `match_ptr` for the same byte ranges it
///   reads from `ip` — `iend` caps both equally. A naive "trailing 7
///   slack on match_ptr" reading would be overspecified: the 8-byte
///   chunked-load body bails on the first non-matching byte, so the
///   read length on `match_ptr` is always `min(ip_len, common_bytes
///   + chunk_padding)` ≤ what `ip` itself reads.
/// - Neither pointer's range may overlap the destination of a
///   concurrent write — the kernel runs single-threaded over a
///   block-local input slice so this holds by construction.
///
/// # Equivalence to upstream zstd
///
/// Upstream zstd (`ZSTD_count` in `zstd_compress_internal.h`):
/// ```c
/// const BYTE* const pStart = pIn;
/// const BYTE* const pInLoopLimit = pInLimit - (sizeof(size_t)-1);
/// if (pIn < pInLoopLimit) {
///   { size_t const diff = MEM_readST(pMatch) ^ MEM_readST(pIn);
///     if (diff) return ZSTD_NbCommonBytes(diff); }
///   pIn += sizeof(size_t); pMatch += sizeof(size_t);
///   while (pIn < pInLoopLimit) { ... }
/// }
/// if (MEM_64bits() && pIn < pInLimit-3 && MEM_read32(pMatch) == MEM_read32(pIn)) { pIn+=4; pMatch+=4; }
/// if (pIn < pInLimit-1 && MEM_read16(pMatch) == MEM_read16(pIn)) { pIn+=2; pMatch+=2; }
/// if (pIn < pInLimit && *pMatch == *pIn) pIn++;
/// return (size_t)(pIn - pStart);
/// ```
///
/// The Rust port preserves the exact same chunk progression so a
/// future cross-check against the C reference can be byte-identical.
#[inline(always)]
pub(crate) unsafe fn count_forward(ip: *const u8, match_ptr: *const u8, iend: *const u8) -> usize {
    let p_start = ip;
    let mut ip = ip;
    let mut m = match_ptr;

    // `usize`-sized chunk loop matching upstream zstd's `MEM_readST` cadence.
    // CHUNK_SIZE = 8 on 64-bit targets, 4 on 32-bit. The bound check
    // `(ip as usize) + CHUNK_SIZE <= (iend as usize)` ensures every
    // chunked read stays inside the caller's `[ip, iend)` source range.
    const CHUNK_SIZE: usize = size_of::<usize>();
    while (ip as usize) + CHUNK_SIZE <= (iend as usize) {
        // SAFETY: CHUNK_SIZE readable bytes at both pointers per the
        // function contract; pointers are not const-aligned, so
        // `read_unaligned`.
        let a = unsafe { core::ptr::read_unaligned(ip.cast::<usize>()) };
        let b = unsafe { core::ptr::read_unaligned(m.cast::<usize>()) };
        let diff = a ^ b;
        if diff != 0 {
            // Upstream zstd's `ZSTD_NbCommonBytes` picks `__builtin_ctzll`
            // (or `ctzl` on 32-bit) on little-endian and `clzll`/`clzl`
            // on big-endian. Native-endian XOR loads place the first
            // byte of the input in the LOW byte of the chunk on LE
            // and the HIGH byte on BE, so "how many common low-order
            // bytes" translates to `ctz/8` on LE and `clz/8` on BE.
            // Without the cfg gate, BE targets would report the
            // common-bytes count from the wrong end of `diff` and
            // produce wrong match lengths.
            #[cfg(target_endian = "little")]
            let common = (diff.trailing_zeros() / 8) as usize;
            #[cfg(target_endian = "big")]
            let common = (diff.leading_zeros() / 8) as usize;
            // SAFETY: `common < CHUNK_SIZE` (otherwise `diff == 0`),
            // and the caller's source range covers ≥ `common` more
            // bytes (we already verified the chunk fits).
            //
            // Length computed via `as usize` arithmetic instead of
            // `offset_from`: the latter returns `isize` and is UB
            // when the byte distance exceeds `isize::MAX`. On 32-bit
            // hosts a buffer spanning >2 GiB would trigger that
            // exact case (`data.len() <= u32::MAX` is 4 GiB, larger
            // than `isize::MAX = 2 GiB - 1`). Subtracting raw
            // addresses as `usize` is well-defined arithmetic with
            // no UB regardless of distance.
            // SAFETY: same as above — the pointer arithmetic is
            // in-bounds, the `usize` cast of a pointer is always
            // well-defined.
            return unsafe { (ip.add(common) as usize) - (p_start as usize) };
        }
        // SAFETY: pointer arithmetic stays within `[p_start, iend)`
        // because we just consumed a CHUNK_SIZE chunk that fit in range.
        unsafe {
            ip = ip.add(CHUNK_SIZE);
            m = m.add(CHUNK_SIZE);
        }
    }

    // 4-byte tail. On 32-bit targets the chunk loop above already
    // operates in 4-byte strides, so the chunk's bounds invariant
    // (`+ CHUNK_SIZE <= iend` failed → at most 3 bytes left) makes
    // this branch's `+ 4 <= iend` always false — guarding the block
    // with `cfg(target_pointer_width = "64")` mirrors upstream zstd's
    // `MEM_64bits()` gate and lets the optimiser drop the dead check
    // on 32-bit builds.
    //
    // SAFETY: bounds check `+ 4 <= iend` before the read; both ptrs
    // have at least `iend - ip` readable bytes by contract.
    #[cfg(target_pointer_width = "64")]
    if (ip as usize) + 4 <= (iend as usize) {
        let a = unsafe { core::ptr::read_unaligned(ip.cast::<u32>()) };
        let b = unsafe { core::ptr::read_unaligned(m.cast::<u32>()) };
        if a == b {
            // SAFETY: just verified 4 readable bytes; pointer add by 4
            // keeps the pointer ≤ iend.
            unsafe {
                ip = ip.add(4);
                m = m.add(4);
            }
        }
    }

    // 2-byte tail.
    if (ip as usize) + 2 <= (iend as usize) {
        let a = unsafe { core::ptr::read_unaligned(ip.cast::<u16>()) };
        let b = unsafe { core::ptr::read_unaligned(m.cast::<u16>()) };
        if a == b {
            // SAFETY: 2 readable bytes verified.
            unsafe {
                ip = ip.add(2);
                m = m.add(2);
            }
        }
    }

    // 1-byte tail.
    if (ip as usize) < (iend as usize) {
        // SAFETY: 1 readable byte verified.
        let a = unsafe { *ip };
        let b = unsafe { *m };
        if a == b {
            // SAFETY: 1 readable byte verified; pointer add by 1 keeps
            // ip ≤ iend.
            unsafe {
                ip = ip.add(1);
            }
        }
    }

    // Length computed via `as usize` arithmetic (not `offset_from`)
    // to avoid UB when the byte distance exceeds `isize::MAX` on
    // 32-bit hosts; see the in-loop return for the full reasoning.
    (ip as usize) - (p_start as usize)
}

/// Forward match length for a candidate whose bytes begin in the dictionary
/// prefix and may continue into the active input, compared against the current
/// input position. This is the 2-segment count the borrowed dict-attach path
/// needs: the dictionary lives in a buffer SEPARATE from the borrowed input,
/// so the flat single-base [`count_forward`] cannot reach across the
/// dict/input boundary. The candidate side reads `dict[cand..]` and, once the
/// dictionary is exhausted, continues at `inp[0..]` (the logical `[dict][input]`
/// window); the current side reads `inp[cur..]`. Mirrors upstream zstd
/// `ZSTD_count_2segments` for a dict-prefix match that extends past the
/// dictionary boundary into the active input.
///
/// Word-at-a-time per segment, mirroring upstream zstd `ZSTD_count_2segments`
/// (it calls `ZSTD_count` on each side of the split): the candidate's dict
/// remainder is counted against the current input with [`count_forward`], and
/// if the candidate exhausts the dict still matching, a second [`count_forward`]
/// continues from the input start. A dict-attach match on dictionary-trained
/// data hits the dict on nearly every position, so the dict segment is on the
/// HOT path — a per-byte boundary loop here was the dominant cost of the
/// borrowed dict kernel; the segmented word-at-a-time count removes it.
///
/// `cand < dict.len()` is required (a dict-prefix candidate); the kernel only
/// calls this for `cand_abs < dict_end`.
///
/// `#[inline]` so the shared 2-segment primitive folds into each backend's
/// borrowed dual-base dict kernel (Fast/Dfast/Row) rather than paying an
/// out-of-line call on the dict-match path.
#[inline]
pub(crate) fn count_forward_dict_2segment(
    dict: &[u8],
    cand: usize,
    inp: &[u8],
    cur: usize,
) -> usize {
    // Release assertions: this is a safe `pub(crate)` fn that does raw pointer
    // math below. `cand >= dict.len()` would make the dict segment read OOB and
    // `cur > inp.len()` would underflow `cur_avail`; enforce the kernel's
    // contract here so a future caller can't silently corrupt memory.
    assert!(
        cand < dict.len(),
        "count_forward_dict_2segment requires cand ({cand}) < dict.len() ({})",
        dict.len(),
    );
    assert!(
        cur <= inp.len(),
        "count_forward_dict_2segment requires cur ({cur}) <= inp.len() ({})",
        inp.len(),
    );
    let dict_len = dict.len();
    let inp_len = inp.len();
    let cur_avail = inp_len - cur;
    if cur_avail == 0 {
        return 0;
    }
    // Segment 1: candidate reads `dict[cand..dict_len]`, current reads
    // `inp[cur..]`. Bounded by whichever side runs out first.
    let seg1 = (dict_len - cand).min(cur_avail);
    // SAFETY: reads `inp[cur..cur+seg1]` and `dict[cand..cand+seg1]`; `seg1 <=
    // cur_avail` keeps the current side in bounds and `seg1 <= dict_len - cand`
    // keeps the candidate side within the dict.
    let m1 = unsafe {
        count_forward(
            inp.as_ptr().add(cur),
            dict.as_ptr().add(cand),
            inp.as_ptr().add(cur + seg1),
        )
    };
    // Mismatch inside the dict segment, or the current input is exhausted →
    // the match ends here.
    if m1 < seg1 || seg1 == cur_avail {
        return m1;
    }
    // The candidate exhausted the dict (`m1 == dict_len - cand`) and the current
    // input still has bytes left. Segment 2: the candidate logically continues
    // at `inp[0..]` (the input directly follows the dict in the `[dict][input]`
    // window); the current side continues at `inp[cur + m1..]`.
    let cur2 = cur + m1;
    // SAFETY: reads `inp[cur2..inp_len]` (current) and `inp[0..inp_len-cur2]`
    // (candidate); both stay within `inp`. `count_forward`'s `iend` caps the
    // current side at the input end.
    let m2 = unsafe {
        count_forward(
            inp.as_ptr().add(cur2),
            inp.as_ptr(),
            inp.as_ptr().add(inp_len),
        )
    };
    m1 + m2
}
