#![allow(
    clippy::cast_ptr_alignment,
    clippy::inline_always,
    clippy::ptr_as_ptr,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Per-CPU-tier SIMD kernels for the optimal parser's price-set update.
//!
//! The btopt / btultra parser refreshes a window of node prices each step.
//! `priceset_range_nonabort_*` are the per-tier kernels (scalar / AVX2 / NEON /
//! SSE4.1 / wasm `simd128`) that the optimal-plan driver dispatches by runtime
//! CPU-feature detection, with the scalar path as the universal fallback.
//! Moved verbatim from `match_generator.rs` (no behaviour change). The kernels
//! read the optimal-parser node / price types and update the BT matcher's
//! price-set scratch; the per-lane math itself is `core::arch` intrinsics.

use super::super::bt::BtMatcher;
use super::super::cost_model::{HcOptState, HcOptimalCostProfile};
use super::super::opt::types::HcOptimalNode;

/// 8-lane `next_cost < node_price` mask for the optimal-parser price-set
/// loop. AVX2 lacks an unsigned `cmplt`, so derive `nc < np` from
/// `min_epu32`: `nc <= np` iff `min(nc,np) == nc`, then exclude equality.
/// Returns a bitmask (bit `k` set => lane `k` improves). Scalar fallback
/// for non-x86 / no-AVX2.
/// 8-lane `next_cost < node_price` mask for the optimal-parser price-set
/// loop. AVX2 lacks an unsigned `cmplt`, so derive `nc < np` from
/// `min_epu32`: `nc <= np` iff `min(nc,np) == nc`, then exclude equality.
/// Returns a bitmask (bit `k` set => lane `k` improves). Compiled on every
/// x86 target (same as the avx2 collect kernel); the cargo `kernel_avx2`
/// feature only gates the runtime dispatch, not compilation.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn priceset_improved_mask8_avx2(next_cost: &[u32; 8], node_price: &[u32]) -> u8 {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::{
        __m256i, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_cmpeq_epi32, _mm256_loadu_si256,
        _mm256_min_epu32, _mm256_movemask_ps,
    };
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::{
        __m256i, _mm256_andnot_si256, _mm256_castsi256_ps, _mm256_cmpeq_epi32, _mm256_loadu_si256,
        _mm256_min_epu32, _mm256_movemask_ps,
    };
    let nc = unsafe { _mm256_loadu_si256(next_cost.as_ptr() as *const __m256i) };
    let np = unsafe { _mm256_loadu_si256(node_price.as_ptr() as *const __m256i) };
    let min = _mm256_min_epu32(nc, np);
    let le = _mm256_cmpeq_epi32(min, nc); // nc <= np
    let eq = _mm256_cmpeq_epi32(nc, np); // nc == np
    let lt = _mm256_andnot_si256(eq, le); // nc < np
    _mm256_movemask_ps(_mm256_castsi256_ps(lt)) as u8
}

/// Inline `next_cost = base_cost + ll0_price + match_price_from_parts(off,ml)`
/// for one match length — the exact `add_prices` chain the scalar loop uses,
/// so the SoA vector path stays byte-identical.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn priceset_next_cost(
    profile: HcOptimalCostProfile,
    stats: &HcOptState,
    ml_cache: &mut [[u32; 2]],
    ml_stamp: u32,
    match_len: usize,
    ll0_price: u32,
    off_price: u32,
    base_cost: u32,
) -> u32 {
    let ml_price =
        BtMatcher::cached_match_length_price(profile, stats, match_len, ml_cache, ml_stamp);
    let seq_cost = BtMatcher::add_prices(
        ll0_price,
        profile.match_price_from_parts(off_price, ml_price, stats),
    );
    BtMatcher::add_prices(base_cost, seq_cost)
}

/// Scalar price-set over the match-length range `[start, max]` for the
/// NON-abort optimal modes (btultra / btultra2). Each `match_len` writes a
/// distinct node `pos + match_len`, so order is irrelevant; the improvement
/// test reduces to `next_cost < node_prices[next]` (`reset_opt_nodes` set
/// every beyond-frontier cell to `u32::MAX`, subsuming `next > last_pos`).
/// `#[inline]` so it folds into each per-tier optimal-parser monomorphisation
/// (no call overhead). Returns the highest written `next`.
#[inline]
#[allow(clippy::too_many_arguments)]
// Used by the scalar / sse42 DP wrappers; on aarch64 the dispatch only reaches
// the neon wrapper and on wasm+simd128 only the simd128 wrapper, so this is
// cfg-dead on those targets.
#[cfg_attr(
    any(
        all(target_arch = "aarch64", target_endian = "little"),
        all(target_arch = "wasm32", target_feature = "simd128")
    ),
    allow(dead_code)
)]
pub(crate) fn priceset_range_nonabort_scalar(
    node_prices: &mut [u32],
    nodes: &mut [HcOptimalNode],
    ml_cache: &mut [[u32; 2]],
    ml_stamp: u32,
    profile: HcOptimalCostProfile,
    stats: &HcOptState,
    pos: usize,
    start: usize,
    max: usize,
    ll0_price: u32,
    off_price: u32,
    base_cost: u32,
    off: u32,
    reps: [u32; 3],
    last_pos: usize,
) -> usize {
    let mut new_last = last_pos;
    for ml in start..=max {
        let next_cost = priceset_next_cost(
            profile, stats, ml_cache, ml_stamp, ml, ll0_price, off_price, base_cost,
        );
        let next = pos + ml;
        if next_cost < node_prices[next] {
            node_prices[next] = next_cost;
            nodes[next] = HcOptimalNode {
                off,
                mlen: ml as u32,
                litlen: 0,
                reps,
            };
            if next > new_last {
                new_last = next;
            }
        }
    }
    new_last
}

/// Shared vectorised price-set loop body, generic over the SIMD width `W`.
/// The per-tier `deint` (vector-load plus deinterleave of `W` cached prices,
/// returning `Some` only on an all-warm chunk) and `mask` (per-tier
/// `next_cost` less-than `node_price` bitmask) are passed as zero-sized
/// `impl Fn`s. `#[inline(always)]` plus monomorphisation folds `deint` and
/// `mask` directly into each per-tier wrapper's `target_feature` umbrella, so
/// the intrinsics inline with no call ABI and no runtime feature detection.
/// Cold or out-of-cache chunks, and the sub-`W` remainder, fall back to the
/// scalar `priceset_next_cost` (which fills the cache); writes are
/// scalar-scatter on the improving lanes (1-8% of compares, per the
/// improve-ratio probe). Same signature tail as the scalar variant.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
// Instantiated only by a vector tier wrapper (avx2/sse4.1 on x86, neon on
// aarch64, simd128 on wasm+simd128); a target with none of those (e.g.
// wasm without +simd128) uses only the scalar range, leaving this generic dead.
#[cfg_attr(
    not(any(
        target_arch = "x86",
        target_arch = "x86_64",
        all(target_arch = "aarch64", target_endian = "little"),
        all(target_arch = "wasm32", target_feature = "simd128")
    )),
    allow(dead_code)
)]
fn priceset_range_vec<const W: usize>(
    node_prices: &mut [u32],
    nodes: &mut [HcOptimalNode],
    ml_cache: &mut [[u32; 2]],
    ml_stamp: u32,
    profile: HcOptimalCostProfile,
    stats: &HcOptState,
    pos: usize,
    start: usize,
    max: usize,
    ll0_price: u32,
    off_price: u32,
    base_cost: u32,
    off: u32,
    reps: [u32; 3],
    last_pos: usize,
    deint: impl Fn(&[[u32; 2]], u32) -> Option<[u32; W]>,
    mask: impl Fn(&[u32; W], &[u32]) -> u8,
) -> usize {
    let mut new_last = last_pos;
    let mut buf = [0u32; W];
    // Loop-invariant constant of the byte-identical next_cost chain:
    // next_cost = add_prices(base_cost, add_prices(ll0_price,
    //   match_price_from_parts(off_price, ml_price))) = c_base + ml_price,
    // c_base = base_cost + ll0_price + match_price_from_parts(off_price, 0).
    //
    // This stays bit-exact with the scalar `priceset_next_cost` because both
    // helpers are affine in `ml_price`: `BtMatcher::add_prices(a, b) = a + b`
    // and `match_price_from_parts(off, ml) = off + ml + bias` are plain integer
    // additions, so `match_price_from_parts(off, ml) = match_price_from_parts(
    // off, 0) + ml` and the whole chain collapses to `c_base + ml_price`. The
    // `wrapping_add` here matches the scalar `+` under the cost model's
    // no-overflow invariant (the `debug_assert`s in both helpers). Factoring the
    // combine into one helper per the review suggestion would force a per-lane
    // `match_price_from_parts(off, ml_price)` recompute instead of hoisting the
    // ml-independent `c_base` once — a regression on this hot DP loop — so the
    // hoist is kept and the equivalence documented here instead.
    let c_base = base_cost
        .wrapping_add(ll0_price)
        .wrapping_add(profile.match_price_from_parts(off_price, 0, stats));
    let mut ml = start;
    while ml + W <= max + 1 {
        let vectorised = if ml + W <= ml_cache.len() {
            deint(&ml_cache[ml..ml + W], ml_stamp)
        } else {
            None
        };
        if let Some(prices) = vectorised {
            for (k, slot) in buf.iter_mut().enumerate() {
                *slot = c_base.wrapping_add(prices[k]);
            }
        } else {
            for (k, slot) in buf.iter_mut().enumerate() {
                *slot = priceset_next_cost(
                    profile,
                    stats,
                    ml_cache,
                    ml_stamp,
                    ml + k,
                    ll0_price,
                    off_price,
                    base_cost,
                );
            }
        }
        let base_next = pos + ml;
        let mut bits = mask(&buf, &node_prices[base_next..base_next + W]);
        while bits != 0 {
            let k = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            let next = base_next + k;
            node_prices[next] = buf[k];
            nodes[next] = HcOptimalNode {
                off,
                mlen: (ml + k) as u32,
                litlen: 0,
                reps,
            };
            if next > new_last {
                new_last = next;
            }
        }
        ml += W;
    }
    while ml <= max {
        let next_cost = priceset_next_cost(
            profile, stats, ml_cache, ml_stamp, ml, ll0_price, off_price, base_cost,
        );
        let next = pos + ml;
        if next_cost < node_prices[next] {
            node_prices[next] = next_cost;
            nodes[next] = HcOptimalNode {
                off,
                mlen: ml as u32,
                litlen: 0,
                reps,
            };
            if next > new_last {
                new_last = next;
            }
        }
        ml += 1;
    }
    new_last
}

/// Vector-load 8 cached ml-prices for the optimal parser's price-set, given a
/// run of 8 contiguous `[price, generation]` cells. Returns `Some(prices)`
/// only when ALL eight cells are warm (`generation == stamp`) — the common
/// (~91-98%) case — so the caller can fold them with one broadcast constant;
/// any cold cell returns `None` to route the chunk through the scalar fill
/// (which recomputes + repopulates the misses). Deinterleaves with cheap
/// in-128-lane ops (`shuffle_epi32` + `unpack*_epi64`) and a single cross-lane
/// `permute4x64` for the ordered prices — avoiding the latency-bound chain of
/// cross-lane `permutevar8x32`s that lost to pipelined scalar loads on
/// high-chunk-count fixtures.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
#[inline]
unsafe fn priceset_cached_prices8_avx2(cells: &[[u32; 2]], stamp: u32) -> Option<[u32; 8]> {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::{
        __m256i, _mm256_castsi256_ps, _mm256_cmpeq_epi32, _mm256_loadu_si256, _mm256_movemask_ps,
        _mm256_permute4x64_epi64, _mm256_set1_epi32, _mm256_shuffle_epi32, _mm256_storeu_si256,
        _mm256_unpackhi_epi64, _mm256_unpacklo_epi64,
    };
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::{
        __m256i, _mm256_castsi256_ps, _mm256_cmpeq_epi32, _mm256_loadu_si256, _mm256_movemask_ps,
        _mm256_permute4x64_epi64, _mm256_set1_epi32, _mm256_shuffle_epi32, _mm256_storeu_si256,
        _mm256_unpackhi_epi64, _mm256_unpacklo_epi64,
    };
    debug_assert!(cells.len() >= 8);
    let base = cells.as_ptr() as *const __m256i;
    // v0 = [p0 g0 p1 g1 | p2 g2 p3 g3], v1 = [p4 g4 p5 g5 | p6 g6 p7 g7].
    let v0 = unsafe { _mm256_loadu_si256(base) };
    let v1 = unsafe { _mm256_loadu_si256(base.add(1)) };
    // In-128-lane group prices then gens: [p g p g] -> [p p g g] (control 0xD8).
    let s0 = _mm256_shuffle_epi32(v0, 0xD8); // [p0 p1 g0 g1 | p2 p3 g2 g3]
    let s1 = _mm256_shuffle_epi32(v1, 0xD8); // [p4 p5 g4 g5 | p6 p7 g6 g7]
    // Gens (hi 64 of each 128-lane) — order irrelevant for the all-equal test.
    let gens = _mm256_unpackhi_epi64(s0, s1);
    let eq = _mm256_cmpeq_epi32(gens, _mm256_set1_epi32(stamp as i32));
    if _mm256_movemask_ps(_mm256_castsi256_ps(eq)) as u8 != 0xFF {
        return None;
    }
    // Prices (lo 64 of each 128-lane): [p0 p1 p4 p5 | p2 p3 p6 p7] as 64-bit
    // chunks [c0 c1 c2 c3] = [p0p1 p4p5 p2p3 p6p7]; reorder to [c0 c2 c1 c3]
    // (control 0xD8) for in-order [p0..p7].
    let p_scrambled = _mm256_unpacklo_epi64(s0, s1);
    let prices = _mm256_permute4x64_epi64(p_scrambled, 0xD8);
    let mut out = [0u32; 8];
    unsafe { _mm256_storeu_si256(out.as_mut_ptr() as *mut __m256i, prices) };
    Some(out)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn priceset_range_nonabort_avx2(
    node_prices: &mut [u32],
    nodes: &mut [HcOptimalNode],
    ml_cache: &mut [[u32; 2]],
    ml_stamp: u32,
    profile: HcOptimalCostProfile,
    stats: &HcOptState,
    pos: usize,
    start: usize,
    max: usize,
    ll0_price: u32,
    off_price: u32,
    base_cost: u32,
    off: u32,
    reps: [u32; 3],
    last_pos: usize,
) -> usize {
    priceset_range_vec::<8>(
        node_prices,
        nodes,
        ml_cache,
        ml_stamp,
        profile,
        stats,
        pos,
        start,
        max,
        ll0_price,
        off_price,
        base_cost,
        off,
        reps,
        last_pos,
        // SAFETY: both closures run inside this fn's avx2 target_feature umbrella.
        |cells, stamp| unsafe { priceset_cached_prices8_avx2(cells, stamp) },
        |nc, np| unsafe { priceset_improved_mask8_avx2(nc, np) },
    )
}

/// NEON 4-lane vector-load + deinterleave of cached ml-prices. `vld2q_u32`
/// deinterleaves the 4 contiguous `[price, generation]` pairs natively into
/// two registers (prices, gens) — no shuffle chain. `Some(prices)` only when
/// all 4 generations equal `stamp` (`vminvq` of the equality mask is all-ones).
#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
#[target_feature(enable = "neon")]
#[inline]
unsafe fn priceset_cached_prices4_neon(cells: &[[u32; 2]], stamp: u32) -> Option<[u32; 4]> {
    use core::arch::aarch64::{vceqq_u32, vdupq_n_u32, vld2q_u32, vminvq_u32, vst1q_u32};
    debug_assert!(cells.len() >= 4);
    // SAFETY: caller's neon umbrella; `cells` is >= 4 pairs = 8 contiguous u32.
    let pair = unsafe { vld2q_u32(cells.as_ptr() as *const u32) };
    let eq = vceqq_u32(pair.1, vdupq_n_u32(stamp));
    if vminvq_u32(eq) != u32::MAX {
        return None;
    }
    let mut out = [0u32; 4];
    unsafe { vst1q_u32(out.as_mut_ptr(), pair.0) };
    Some(out)
}

/// NEON 4-lane `next_cost < node_price` bitmask. NEON has an unsigned compare
/// (`vcltq_u32`) but no movemask; AND the all-ones lane mask with lane weights
/// `[1,2,4,8]` and horizontal-add (`vaddvq_u32`) to pack the 4 bits.
#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
#[target_feature(enable = "neon")]
#[inline]
unsafe fn priceset_improved_mask4_neon(next_cost: &[u32; 4], node_price: &[u32]) -> u8 {
    use core::arch::aarch64::{vaddvq_u32, vandq_u32, vcltq_u32, vld1q_u32, vst1q_u32};
    // SAFETY: neon umbrella; both spans are 4 u32 wide.
    let nc = unsafe { vld1q_u32(next_cost.as_ptr()) };
    let np = unsafe { vld1q_u32(node_price.as_ptr()) };
    let lt = vcltq_u32(nc, np);
    let weights: [u32; 4] = [1, 2, 4, 8];
    let w = unsafe { vld1q_u32(weights.as_ptr()) };
    let bits = vandq_u32(lt, w);
    let _ = vst1q_u32; // silence unused import on some toolchains
    vaddvq_u32(bits) as u8
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
#[target_feature(enable = "neon")]
#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn priceset_range_nonabort_neon(
    node_prices: &mut [u32],
    nodes: &mut [HcOptimalNode],
    ml_cache: &mut [[u32; 2]],
    ml_stamp: u32,
    profile: HcOptimalCostProfile,
    stats: &HcOptState,
    pos: usize,
    start: usize,
    max: usize,
    ll0_price: u32,
    off_price: u32,
    base_cost: u32,
    off: u32,
    reps: [u32; 3],
    last_pos: usize,
) -> usize {
    priceset_range_vec::<4>(
        node_prices,
        nodes,
        ml_cache,
        ml_stamp,
        profile,
        stats,
        pos,
        start,
        max,
        ll0_price,
        off_price,
        base_cost,
        off,
        reps,
        last_pos,
        // SAFETY: both closures run inside this fn's neon target_feature umbrella.
        |cells, stamp| unsafe { priceset_cached_prices4_neon(cells, stamp) },
        |nc, np| unsafe { priceset_improved_mask4_neon(nc, np) },
    )
}

/// SSE4.1 4-lane vector-load + deinterleave of cached ml-prices. Two 128-bit
/// loads of `[price, gen]` pairs, `shuffle_epi32(0xD8)` groups prices then gens
/// within each, `unpacklo/hi_epi64` separates them. `Some(prices)` only when
/// all 4 generations equal `stamp`.
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.2")]
#[inline]
unsafe fn priceset_cached_prices4_sse41(cells: &[[u32; 2]], stamp: u32) -> Option<[u32; 4]> {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::{
        __m128i, _mm_castsi128_ps, _mm_cmpeq_epi32, _mm_loadu_si128, _mm_movemask_ps,
        _mm_set1_epi32, _mm_shuffle_epi32, _mm_storeu_si128, _mm_unpackhi_epi64,
        _mm_unpacklo_epi64,
    };
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::{
        __m128i, _mm_castsi128_ps, _mm_cmpeq_epi32, _mm_loadu_si128, _mm_movemask_ps,
        _mm_set1_epi32, _mm_shuffle_epi32, _mm_storeu_si128, _mm_unpackhi_epi64,
        _mm_unpacklo_epi64,
    };
    debug_assert!(cells.len() >= 4);
    let base = cells.as_ptr() as *const __m128i;
    let v0 = unsafe { _mm_loadu_si128(base) }; // [p0 g0 p1 g1]
    let v1 = unsafe { _mm_loadu_si128(base.add(1)) }; // [p2 g2 p3 g3]
    let s0 = _mm_shuffle_epi32(v0, 0xD8); // [p0 p1 g0 g1]
    let s1 = _mm_shuffle_epi32(v1, 0xD8); // [p2 p3 g2 g3]
    let gens = _mm_unpackhi_epi64(s0, s1); // [g0 g1 g2 g3]
    let eq = _mm_cmpeq_epi32(gens, _mm_set1_epi32(stamp as i32));
    if _mm_movemask_ps(_mm_castsi128_ps(eq)) as u8 & 0x0F != 0x0F {
        return None;
    }
    let prices = _mm_unpacklo_epi64(s0, s1); // [p0 p1 p2 p3]
    let mut out = [0u32; 4];
    unsafe { _mm_storeu_si128(out.as_mut_ptr() as *mut __m128i, prices) };
    Some(out)
}

/// SSE4.1 4-lane `next_cost < node_price` bitmask (unsigned compare via
/// `min_epu32`, like the AVX2 path).
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.2")]
#[inline]
unsafe fn priceset_improved_mask4_sse41(next_cost: &[u32; 4], node_price: &[u32]) -> u8 {
    #[cfg(target_arch = "x86")]
    use core::arch::x86::{
        __m128i, _mm_andnot_si128, _mm_castsi128_ps, _mm_cmpeq_epi32, _mm_loadu_si128,
        _mm_min_epu32, _mm_movemask_ps,
    };
    #[cfg(target_arch = "x86_64")]
    use core::arch::x86_64::{
        __m128i, _mm_andnot_si128, _mm_castsi128_ps, _mm_cmpeq_epi32, _mm_loadu_si128,
        _mm_min_epu32, _mm_movemask_ps,
    };
    let nc = unsafe { _mm_loadu_si128(next_cost.as_ptr() as *const __m128i) };
    let np = unsafe { _mm_loadu_si128(node_price.as_ptr() as *const __m128i) };
    let min = _mm_min_epu32(nc, np);
    let le = _mm_cmpeq_epi32(min, nc);
    let eq = _mm_cmpeq_epi32(nc, np);
    let lt = _mm_andnot_si128(eq, le);
    (_mm_movemask_ps(_mm_castsi128_ps(lt)) as u8) & 0x0F
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.2")]
#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn priceset_range_nonabort_sse41(
    node_prices: &mut [u32],
    nodes: &mut [HcOptimalNode],
    ml_cache: &mut [[u32; 2]],
    ml_stamp: u32,
    profile: HcOptimalCostProfile,
    stats: &HcOptState,
    pos: usize,
    start: usize,
    max: usize,
    ll0_price: u32,
    off_price: u32,
    base_cost: u32,
    off: u32,
    reps: [u32; 3],
    last_pos: usize,
) -> usize {
    priceset_range_vec::<4>(
        node_prices,
        nodes,
        ml_cache,
        ml_stamp,
        profile,
        stats,
        pos,
        start,
        max,
        ll0_price,
        off_price,
        base_cost,
        off,
        reps,
        last_pos,
        // SAFETY: both closures run inside this fn's sse4.2 target_feature umbrella.
        |cells, stamp| unsafe { priceset_cached_prices4_sse41(cells, stamp) },
        |nc, np| unsafe { priceset_improved_mask4_sse41(nc, np) },
    )
}

/// wasm `simd128` 4-lane vector-load + deinterleave of cached ml-prices.
/// `u32x4_shuffle` selects the price (even) and gen (odd) lanes across the two
/// loaded vectors natively. `Some(prices)` only when all 4 gens equal `stamp`
/// (`u32x4_all_true` of the equality vector).
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#[target_feature(enable = "simd128")]
#[inline]
unsafe fn priceset_cached_prices4_simd128(cells: &[[u32; 2]], stamp: u32) -> Option<[u32; 4]> {
    use core::arch::wasm32::{
        u32x4_all_true, u32x4_eq, u32x4_shuffle, u32x4_splat, v128, v128_load, v128_store,
    };
    debug_assert!(cells.len() >= 4);
    let base = cells.as_ptr() as *const v128;
    let v0 = unsafe { v128_load(base) }; // [p0 g0 p1 g1]
    let v1 = unsafe { v128_load(base.add(1)) }; // [p2 g2 p3 g3]
    // Lanes 0..3 index v0, 4..7 index v1.
    let gens = u32x4_shuffle::<1, 3, 5, 7>(v0, v1); // [g0 g1 g2 g3]
    let eq = u32x4_eq(gens, u32x4_splat(stamp));
    if !u32x4_all_true(eq) {
        return None;
    }
    let prices = u32x4_shuffle::<0, 2, 4, 6>(v0, v1); // [p0 p1 p2 p3]
    let mut out = [0u32; 4];
    unsafe { v128_store(out.as_mut_ptr() as *mut v128, prices) };
    Some(out)
}

/// wasm `simd128` 4-lane `next_cost < node_price` bitmask. wasm has a native
/// unsigned compare (`u32x4_lt`) and `u32x4_bitmask` to pack the lanes.
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#[target_feature(enable = "simd128")]
#[inline]
unsafe fn priceset_improved_mask4_simd128(next_cost: &[u32; 4], node_price: &[u32]) -> u8 {
    use core::arch::wasm32::{u32x4_bitmask, u32x4_lt, v128, v128_load};
    let nc = unsafe { v128_load(next_cost.as_ptr() as *const v128) };
    let np = unsafe { v128_load(node_price.as_ptr() as *const v128) };
    u32x4_bitmask(u32x4_lt(nc, np))
}

#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#[target_feature(enable = "simd128")]
#[inline]
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn priceset_range_nonabort_simd128(
    node_prices: &mut [u32],
    nodes: &mut [HcOptimalNode],
    ml_cache: &mut [[u32; 2]],
    ml_stamp: u32,
    profile: HcOptimalCostProfile,
    stats: &HcOptState,
    pos: usize,
    start: usize,
    max: usize,
    ll0_price: u32,
    off_price: u32,
    base_cost: u32,
    off: u32,
    reps: [u32; 3],
    last_pos: usize,
) -> usize {
    priceset_range_vec::<4>(
        node_prices,
        nodes,
        ml_cache,
        ml_stamp,
        profile,
        stats,
        pos,
        start,
        max,
        ll0_price,
        off_price,
        base_cost,
        off,
        reps,
        last_pos,
        // SAFETY: both closures run inside this fn's simd128 target_feature umbrella.
        |cells, stamp| unsafe { priceset_cached_prices4_simd128(cells, stamp) },
        |nc, np| unsafe { priceset_improved_mask4_simd128(nc, np) },
    )
}
