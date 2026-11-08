#![allow(
    clippy::cast_lossless,
    clippy::cast_ptr_alignment,
    clippy::explicit_iter_loop,
    clippy::inline_always,
    clippy::manual_let_else,
    clippy::ptr_as_ptr,
    clippy::semicolon_if_nothing_returned,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Row-based match finder (level 4 default backend).
//!
//! Upstream zstd parity: mirrors the `ZSTD_row_*` family in `zstd_lazy.c`. The
//! row hash splits each bucket into `1 << row_log` slots (16 / 32 / 64),
//! each tagged with a 1-byte hash so the search can skip most slots
//! without touching the position table.
//!
//! Shared with `match_generator.rs`
//! (structural split). Mechanical move — names, fields, and bodies
//! are preserved; visibility on the relocated items was opened to
//! `pub(crate)` so `match_generator` can keep dispatching to
//! `RowMatchGenerator` through the `row::` import path.

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::Sequence;
use super::blocks::encode_offset_with_history;
use super::dict_attach::DictAttach;
use super::levels::config::RowConfig;
use super::match_generator::{
    ROW_EMPTY_SLOT, ROW_HASH_BITS, ROW_HASH_KEY_LEN, ROW_LOG, ROW_MIN_MATCH_LEN, ROW_SEARCH_DEPTH,
    ROW_TAG_BITS, ROW_TARGET_LEN,
};

/// Immutable row-hash dictionary index (upstream zstd `ZSTD_RowFindBestMatch`'s
/// `dictMatchState` probe). Built once over the dictionary region and probed as
/// ONE fixed-width row (`<= row_entries` tag-matched candidates) AFTER the live
/// row, so the dict search is bounded (unlike a hash-chain walk) and never
/// re-indexed per frame. `positions` store CONCAT indices (history_start-
/// relative, floor-rebase-invariant); `ROW_EMPTY_SLOT = u32::MAX` marks empty.
#[derive(Debug, Default, Clone)]
pub(crate) struct RowDictTables {
    pub(crate) heads: Vec<u8>,
    pub(crate) positions: Vec<u32>,
    pub(crate) tags: Vec<u8>,
}
use super::match_table::helpers::{
    INCOMPRESSIBLE_SKIP_STEP, best_len_offset_candidate, extend_backwards_shared,
    repcode_candidate_shared,
};
use super::match_table::storage::REBASE_RESET_FLOOR_CEILING;
use super::opt::types::MatchCandidate;

// The row probe reuses the shared `fastpath::FastpathKernel` selection so each
// per-tier `#[target_feature]` probe can inline BOTH the tag-match mask AND the
// matching `fastpath::<tier>::common_prefix_len_ptr` (the tiers must share a
// feature set for the cpl to inline — `Sse42` is a superset of the SSE2 mask
// intrinsics, `Avx2Bmi2` of the AVX2 mask). `select_kernel()` does the runtime
// detect once per process via a `OnceLock`.
use super::fastpath::FastpathKernel;

/// Compile-time row tag-match kernel. Each ZST monomorphises the per-row
/// tag compare so the search hot loop drops the runtime `RowTagKernel`
/// enum branch (one predictable branch + all-tiers' inlined SIMD bodies
/// per position become a single tier's body, no branch). The bare
/// dispatchers select the impl once per block from the runtime-detected
/// `tag_kernel`, so an impl is only instantiated/used on a CPU that
/// supports its ISA — the same contract `RowTagKernel::detect` upholds for
/// the enum's `unsafe` SIMD calls.
pub(crate) trait RowTags: Copy {
    /// Run the row match probe (live row + dict dual-probe) for this kernel.
    /// Forwards to the matcher's per-tier `#[target_feature]` probe method whose
    /// body expands the tier's `row_tag_mask_*!` SIMD inline (no function call),
    /// so the vector tag-match inlines straight-line under the kernel's feature
    /// umbrella instead of crossing the `#[target_feature]` ABI boundary on every
    /// probe (which it does even for baseline NEON/SSE2 — see `fastpath` module
    /// docs). Runtime kernel selection happens once at the `dispatch_tag_kernel!`
    /// site, never inside the per-position hot loop.
    ///
    /// # Safety
    /// The caller (via `dispatch_tag_kernel!`) only selects a kernel whose ISA
    /// `RowTagKernel::detect` confirmed present, upholding the per-tier
    /// `#[target_feature]` contract.
    unsafe fn probe<const ROW_LOG: usize>(
        matcher: &RowMatchGenerator,
        abs_pos: usize,
        lit_len: usize,
        hash: Option<(usize, u8)>,
    ) -> Option<MatchCandidate>;
}

// On wasm32+simd128 the row tier is the compile-time `Simd128Tags`, so the
// scalar fallback ZST is never constructed there (it stays the fallback on
// every other target, and on scalar-only wasm builds).

#[cfg_attr(
    all(target_arch = "wasm32", target_feature = "simd128"),
    allow(dead_code)
)]
#[derive(Copy, Clone)]
struct ScalarTags;
impl RowTags for ScalarTags {
    #[inline]
    unsafe fn probe<const ROW_LOG: usize>(
        matcher: &RowMatchGenerator,
        abs_pos: usize,
        lit_len: usize,
        hash: Option<(usize, u8)>,
    ) -> Option<MatchCandidate> {
        // Scalar has no target feature; the probe body runs as-is.
        unsafe { matcher.row_probe_scalar::<ROW_LOG>(abs_pos, lit_len, hash) }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
struct Sse42Tags;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl RowTags for Sse42Tags {
    #[inline]
    unsafe fn probe<const ROW_LOG: usize>(
        matcher: &RowMatchGenerator,
        abs_pos: usize,
        lit_len: usize,
        hash: Option<(usize, u8)>,
    ) -> Option<MatchCandidate> {
        // SAFETY: dispatched only when `tag_kernel == Sse42` (SSE4.2 confirmed).
        unsafe { matcher.row_probe_sse42::<ROW_LOG>(abs_pos, lit_len, hash) }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[derive(Copy, Clone)]
struct Avx2Bmi2Tags;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl RowTags for Avx2Bmi2Tags {
    #[inline]
    unsafe fn probe<const ROW_LOG: usize>(
        matcher: &RowMatchGenerator,
        abs_pos: usize,
        lit_len: usize,
        hash: Option<(usize, u8)>,
    ) -> Option<MatchCandidate> {
        // SAFETY: dispatched only when `tag_kernel == Avx2Bmi2` (AVX2+BMI2 confirmed).
        unsafe { matcher.row_probe_avx2bmi2::<ROW_LOG>(abs_pos, lit_len, hash) }
    }
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
#[derive(Copy, Clone)]
struct NeonTags;
#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
impl RowTags for NeonTags {
    #[inline]
    unsafe fn probe<const ROW_LOG: usize>(
        matcher: &RowMatchGenerator,
        abs_pos: usize,
        lit_len: usize,
        hash: Option<(usize, u8)>,
    ) -> Option<MatchCandidate> {
        // SAFETY: dispatched only when `tag_kernel == Neon` (NEON confirmed).
        unsafe { matcher.row_probe_neon::<ROW_LOG>(abs_pos, lit_len, hash) }
    }
}

// WebAssembly fixed-128-bit SIMD tier. wasm has no runtime CPUID, so this is
// compile-time only: present (and dispatched-to) exactly when the build enables
// `simd128`, selected directly in `dispatch_tag_kernel!` rather than via the
// runtime `FastpathKernel` (which carries no wasm tier).
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
#[derive(Copy, Clone)]
struct Simd128Tags;
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
impl RowTags for Simd128Tags {
    #[inline]
    unsafe fn probe<const ROW_LOG: usize>(
        matcher: &RowMatchGenerator,
        abs_pos: usize,
        lit_len: usize,
        hash: Option<(usize, u8)>,
    ) -> Option<MatchCandidate> {
        // wasm simd128 is compile-time; `row_probe_simd128` needs no
        // `#[target_feature]` and the intrinsics inline directly.
        unsafe { matcher.row_probe_simd128::<ROW_LOG>(abs_pos, lit_len, hash) }
    }
}

/// Resolve the runtime `tag_kernel` (`FastpathKernel`) to a `RowTags` ZST once,
/// then call a `*_k::<K>` method that binds the `row_log` const. The kernel
/// branch runs once per block (cold), so the per-position hot loop is fully
/// monomorphised over the selected tier — no runtime kernel enum inside.
macro_rules! dispatch_tag_kernel {
    ($self:ident . $k_method:ident ( $($arg:expr),* )) => {{
        // wasm32 has no runtime CPUID: when the build enables `simd128`, the
        // tier is resolved at compile time straight to `Simd128Tags`, so the
        // runtime `FastpathKernel` match (native-only) is cfg'd out entirely.
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        {
            $self.$k_method::<Simd128Tags>($($arg),*)
        }
        #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
        {
            match $self.tag_kernel {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                FastpathKernel::Avx2Bmi2 => $self.$k_method::<Avx2Bmi2Tags>($($arg),*),
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                FastpathKernel::Sse42 => $self.$k_method::<Sse42Tags>($($arg),*),
                #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
                FastpathKernel::Neon => $self.$k_method::<NeonTags>($($arg),*),
                FastpathKernel::Scalar => $self.$k_method::<ScalarTags>($($arg),*),
            }
        }
    }};
}

/// Per-tier `#[target_feature]` umbrella over the greedy row kernel: with
/// the umbrella's features a superset of the tier probe's, the
/// `#[inline(always)]` loop body AND the tier's `row_probe_*` merge into ONE
/// compiled function (upstream zstd `ZSTD_compressBlock_greedy_row` shape) instead
/// of paying a non-inlinable `#[target_feature]` call per position.
/// The greedy row parse BODY as a macro: expanded per tier with the tier's
/// own SIMD pairing (`$maskmac` tag-match + `$cpl` prefix kernel), with the
/// probe body expanded inline at the search site — the full upstream zstd
/// `ZSTD_compressBlock_greedy_row` monolith, one compiled function per tier.
macro_rules! greedy_parse_body {
    ($m:expr, $handle:expr, $rl:expr, $use_mask:literal, $maskmac:ident, $cpl:path) => {{
        #[allow(unused_labels)]
        'parse: {
            debug_assert_eq!($rl, $m.row_log);
            $m.ensure_tables();

            let (current_abs_start, current_len) = $m.current_block_range();
            if current_len == 0 {
                break 'parse;
            }
            let backfill_start = $m.backfill_start(current_abs_start);
            if backfill_start < current_abs_start {
                $m.insert_positions::<$rl>(backfill_start, current_abs_start);
            }

            // Upstream zstd mls for repcode probes is 4 (`MEM_read32` compare on
            // `ip+1` against `ip+1-offset_1`, length extended by
            // `ZSTD_count + 4`). The row matcher's `ROW_MIN_MATCH_LEN = 5`
            // gates the *regular* search via the row-table layout; rep
            // probes are independent of the row table and benefit from
            // the lower upstream zstd threshold (a 4-byte rep is cheap to
            // encode and frequently outperforms emitting the bytes as
            // literals).
            const REP_MIN_MATCH_LEN: usize = 4;
            // Outer-loop lookahead floor: at least `REP_MIN_MATCH_LEN + 1`
            // bytes left so the `abs_pos + 1` repcode probe can succeed even
            // in the block tail (the rep probe needs `REP_MIN_MATCH_LEN`
            // bytes one position ahead). This keeps the tail 4-byte rep-only
            // case from falling through as literals.
            const GREEDY_MIN_LOOKAHEAD: usize = REP_MIN_MATCH_LEN + 1;

            let mut pos = 0usize;
            let mut literals_start = 0usize;
            // Software pipeline over the dependent row load: on a miss the next
            // position's (row, tag) is computed ahead and its tag/position rows
            // prefetched, so the probe's row loads (the hottest instructions of
            // this loop — a hash-dependent cache miss per position) overlap the
            // current iteration's tail instead of stalling the next probe.
            // Upstream zstd `ZSTD_RowFindBestMatch` gets the same overlap from its
            // hash-cache + `ZSTD_row_prefetch` pipeline.
            let mut carried: Option<(usize, (usize, u8))> = None;

            while pos + GREEDY_MIN_LOOKAHEAD <= current_len {
                let abs_pos = current_abs_start + pos;
                let lit_len = pos - literals_start;

                // (1) Default start = abs_pos + 1: probe the repcode bank
                //     at the next byte, treating one byte as already
                //     committed to the literal run. Upstream zstd probes only
                //     rep1 here; `repcode_candidate_shared` probes all three
                //     plus the `ll0` fallback because the upstream zstd "ll0" trick
                //     is already baked into our shared helper. The extra
                //     probes only add candidates that have repcode encoding
                //     costs (cheap), so the ratio direction is positive vs
                //     upstream zstd while still landing in the "greedy via repcode"
                //     algorithmic shape.
                let rep_probe_pos = abs_pos + 1;
                let rep_probe_lit_len = lit_len + 1;
                let rep_match = if rep_probe_pos + REP_MIN_MATCH_LEN <= $m.history_abs_end() {
                    repcode_candidate_shared(
                        $m.hash_kernel,
                        $m.live_history(),
                        $m.history_abs_start,
                        $m.offset_hist,
                        rep_probe_pos,
                        rep_probe_lit_len,
                        REP_MIN_MATCH_LEN,
                        $m.max_window_size,
                    )
                } else {
                    None
                };

                // (2) Upstream zstd greedy (depth 0): a repcode hit commits
                // immediately and SKIPS the regular row search
                // (`zstd_lazy.c:2039`, `if (depth==0) goto _storeSequence`).
                // The regular `row_candidate` (SIMD row scan + match
                // extension) is the dominant per-position cost; running it on
                // every rep hit made rep-dense inputs (repetitive logs) up to
                // ~11x slower than upstream, which short-circuits. So only
                // run the regular search when there is no rep to take.
                // `row_candidate` is `&self` (a pure search, no table
                // mutation), so skipping it drops no hash insert: the
                // post-emit `insert_positions(abs_pos, ..)` still indexes the
                // committed span.
                let hash = match carried.take() {
                    Some((carried_pos, rt)) if carried_pos == abs_pos => Some(rt),
                    _ => None,
                };
                let chosen = match rep_match {
                    Some(rep) => Some(rep),
                    // Probe body expanded inline (tier SIMD pairing via the
                    // enclosing monolith macro), not a function call.
                    None => {
                        // Greedy already short-circuited on a rep hit above,
                        // so the probe starts from no candidate here.
                        row_probe_body!(
                            $m, abs_pos, lit_len, hash, None, $rl, $use_mask, $maskmac, $cpl
                        )
                    }
                };

                let Some(candidate) = chosen else {
                    // Upstream zstd `kSearchStrength = 8` shifts hard on miss
                    // (step grows by `lit_len >> 8`). Empirically on our
                    // corpus that recovers ~30% speed but costs ratio by
                    // dropping hash inserts on long literal runs that
                    // would have served future matches. Shift right by
                    // `SKIP_STRENGTH = 10` instead — same shape, ~4×
                    // rarer growth, so the step stays at 1 byte until the
                    // literal run hits ~1 KiB and only then begins
                    // skipping. Lets us keep most of upstream zstd's speed
                    // characteristic without re-introducing the ratio
                    // drain.
                    const SKIP_STRENGTH: u32 = 10;
                    let step = ((lit_len as u32) >> SKIP_STRENGTH) as usize + 1;
                    // Reuse the probe's (row, tag) for the insert (the rep-hit
                    // path never computed one, so fall back to the hashing
                    // insert there — `hash` is `Some` exactly when the probe
                    // ran on this position).
                    match hash {
                        Some((row, tag)) => $m.insert_at::<$rl>(abs_pos, row, tag),
                        None => $m.insert_position::<$rl>(abs_pos),
                    }
                    if step == 1 {
                        let next_abs = abs_pos + 1;
                        if let Some((row, tag)) = $m.hash_and_row(next_abs) {
                            $m.prefetch_row::<$rl>(row);
                            carried = Some((next_abs, (row, tag)));
                        }
                    }
                    pos += step;
                    continue;
                };

                // Emit sequence.
                let start = candidate.start - current_abs_start;
                // Index `[abs_pos, candidate.start + match_len)`, NOT
                // `[candidate.start, candidate.start + match_len)`.
                // `extend_backwards_shared` can move `candidate.start`
                // below `abs_pos` by absorbing literal bytes that the
                // outer loop already indexed on earlier miss iterations
                // via `insert_position(abs_pos)`. Re-indexing them here
                // would write the same `abs_pos -> position` mapping
                // into the row table a second time, evicting more recent
                // / more useful slot tenants from the same row's chain.
                // Measured on `decodecorpus-z000033`: the
                // `candidate.start` lower bound regresses `rust_bytes` by
                // ~+447 over `abs_pos` (537897 -> 538344), so the
                // narrower range is intentional.
                $m.insert_match_span::<$rl>(abs_pos, candidate.start + candidate.match_len);
                // Trailing literals of the current block. Read via
                // `live_history()` (borrowed-aware) sliced at the block's
                // offset within the live region: owned →
                // `live_history()[window_size - current_len..]` (byte-identical
                // to the old `history[history.len()-current_len..]`); borrowed →
                // `live_history()[block_start..]` (the in-place input, since the
                // owned `history` mirror is empty under the no-copy path).
                let current = &$m.live_history()[(current_abs_start - $m.history_abs_start)..];
                let literals = &current[literals_start..start];
                $handle(Sequence::Triple {
                    literals,
                    offset: candidate.offset,
                    match_len: candidate.match_len,
                });
                let _ = encode_offset_with_history(
                    candidate.offset as u32,
                    literals.len() as u32,
                    &mut $m.offset_hist,
                );
                pos = start + candidate.match_len;
                literals_start = pos;
                // Same hash + row prefetch carry as the miss path: the next
                // probe position is known here (right past the emitted match),
                // and its row is otherwise guaranteed cold after the match-span
                // insert walked other rows. The rep probe at the next iteration
                // may consume the position without a row probe — then the
                // carried hash is only reused by the insert-side, never wasted.
                if pos + GREEDY_MIN_LOOKAHEAD <= current_len {
                    let next_abs = current_abs_start + pos;
                    if let Some((row, tag)) = $m.hash_and_row(next_abs) {
                        $m.prefetch_row::<$rl>(row);
                        carried = Some((next_abs, (row, tag)));
                    }
                }

                // Upstream zstd's `lazy_generic` has an immediate-repcode loop here
                // (probing `offset_2` after each main emit and swapping
                // `offset_1 ↔ offset_2` on hit). It was implemented and
                // shipped in earlier iterations of this method but never
                // fired on any test or benchmark workload — the
                // `repcode_candidate_shared` probe at the top of the main
                // loop already evaluates all three rep slots (rep1, rep2,
                // rep3 + the `ll0` fallback), and the immediate-rep slot
                // (`offset_hist[1]` at `lit_len = 0`) is subsumed by the
                // next main-loop iteration's rep probe of the same slot.
                // Upstream zstd's version is single-rep, so the inner loop catches
                // hits its main-loop probe wouldn't; ours is three-rep, so
                // the inner loop is dead by construction. Removed to free
                // the per-iteration check and keep the parser body lean.
            }

            while pos + ROW_HASH_KEY_LEN <= current_len {
                $m.insert_position::<$rl>(current_abs_start + pos);
                pos += 1;
            }

            if literals_start < current_len {
                // Trailing literals of the current block. Read via
                // `live_history()` (borrowed-aware) sliced at the block's
                // offset within the live region: owned →
                // `live_history()[window_size - current_len..]` (byte-identical
                // to the old `history[history.len()-current_len..]`); borrowed →
                // `live_history()[block_start..]` (the in-place input, since the
                // owned `history` mirror is empty under the no-copy path).
                let current = &$m.live_history()[(current_abs_start - $m.history_abs_start)..];
                $handle(Sequence::Literals {
                    literals: &current[literals_start..],
                });
            }
        }
    }};
}

/// Rep + row best-of-two probe (the lazy search step) with the probe body
/// expanded inline under the enclosing tier umbrella.
macro_rules! row_best_match {
    ($m:expr, $abs_pos:expr, $lit_len:expr, $hash:expr, $rl:expr, $use_mask:literal, $maskmac:ident, $cpl:path) => {{
        let rep = $m.repcode_candidate($abs_pos, $lit_len);
        row_probe_body!(
            $m, $abs_pos, $lit_len, $hash, rep, $rl, $use_mask, $maskmac, $cpl
        )
    }};
}

/// The lazy row parse BODY as a macro — same per-tier monolith shape as
/// `greedy_parse_body!` for the lazy levels. Upstream zstd's `ZSTD_searchMax`
/// is `FORCE_INLINE_TEMPLATE` into `ZSTD_compressBlock_lazy_generic`, so the
/// search at every probe site (current position + the `lazy_decide!` lookahead)
/// expands the rep + row-probe body inline via `row_best_match!` rather than
/// calling an out-of-line per-tier `#[target_feature]` search method — keeping
/// the whole per-position pipeline one monolith with no per-position call cost.
macro_rules! lazy_parse_body {
    ($m:expr, $handle:expr, $rl:expr, $use_mask:literal, $maskmac:ident, $cpl:path) => {{
        #[allow(unused_labels)]
        'parse: {
            debug_assert_eq!($rl, $m.row_log);
            $m.ensure_tables();

            let (current_abs_start, current_len) = $m.current_block_range();
            if current_len == 0 {
                break 'parse;
            }
            let backfill_start = $m.backfill_start(current_abs_start);
            if backfill_start < current_abs_start {
                $m.insert_positions::<$rl>(backfill_start, current_abs_start);
            }

            let mls = $m.mls;
            let mut pos = 0usize;
            let mut literals_start = 0usize;
            // Lookahead search result carried into the next iteration
            // (upstream zstd's lazy chain, `zstd_lazy.c` lazy_generic: a
            // deferred position's successor is never searched twice — the
            // depth loop carries the better match forward). `Some(r)` means
            // this iteration's position was already searched as the previous
            // iteration's lookahead, with result `r`.
            let mut carried: Option<Option<MatchCandidate>> = None;
            while pos + mls <= current_len {
                let abs_pos = current_abs_start + pos;
                let lit_len = pos - literals_start;
                // The previous iteration deferred (carried set) — including the
                // depth-2 defer-WITHOUT-carry case (`Some(None)`, where
                // `abs_pos + 1` was cold but `abs_pos + 2` won). `carried.take()`
                // below clears that, so capture it now: if this position then
                // misses, the skip must advance by one so it cannot hop over the
                // proven `abs_pos + 2` winner.
                let deferred_from_prev = carried.is_some();

                // Hash the position ONCE per iteration: the probe consumes it
                // and the defer branch reuses it for the insert (upstream zstd
                // hashes once per position — `ZSTD_RowFindBestMatch` updates
                // the row as part of the search, `zstd_lazy.c`). A carried
                // iteration skips both: the search already ran.
                let (hash, best) = match carried.take() {
                    Some(best) => (None, best),
                    None => {
                        let hash = $m.hash_and_row(abs_pos);
                        // Search spliced INLINE (upstream zstd `ZSTD_searchMax`
                        // is `FORCE_INLINE_TEMPLATE` into `lazy_generic`): expand
                        // the rep + row-probe body here rather than calling an
                        // out-of-line per-tier `#[target_feature]` `$find` method
                        // (a `target_feature` fn cannot inline across the call
                        // boundary, so the call cost + arg marshalling was paid
                        // per position — the dominant instruction-count gap vs C).
                        let best = row_best_match!(
                            $m, abs_pos, lit_len, hash, $rl, $use_mask, $maskmac, $cpl
                        );
                        (hash, best)
                    }
                };
                let picked = 'pick: {
                    let Some(best) = best else { break 'pick None };
                    // The decision is a MACRO (spliced inline); its lookahead
                    // `search` splice expands the row-probe body inline too, so the
                    // whole per-position pipeline stays one `target_feature`
                    // monolith with no out-of-line search call at any probe site.
                    // target_len = MAX: upstream lazy has no sufficient-length
                    // early-out (that is the OPT parser's; one here collapses lazy
                    // to greedy, -7% ratio vs C). The macro weighs candidates by
                    // length (ties to the smaller offset) and returns the carry so
                    // the deferred position is searched once.
                    let lazy_depth = $m.lazy_depth;
                    let history_end = $m.history_abs_end();
                    let mls = $m.mls;
                    let decision = $crate::encoding::lazy_parse::lazy_decide!(
                        best_len = best.match_len,
                        best_off = best.offset,
                        target_len = usize::MAX,
                        lazy_depth = lazy_depth,
                        abs_pos = abs_pos,
                        lit_len = lit_len,
                        history_end = history_end,
                        min_match = mls,
                        // Lookahead search, also spliced inline (no out-of-line
                        // call): the enclosing kernel runs only when its tier was
                        // runtime-detected, upholding the row-probe's
                        // target_feature contract.
                        search =
                            |p, l| row_best_match!($m, p, l, None, $rl, $use_mask, $maskmac, $cpl),
                    );
                    if let Some(carry) = decision {
                        carried = Some(carry);
                        None
                    } else {
                        Some(best)
                    }
                };
                if let Some(candidate) = picked {
                    $m.insert_match_span::<$rl>(abs_pos, candidate.start + candidate.match_len);
                    // Trailing literals of the current block. Read via
                    // `live_history()` (borrowed-aware) sliced at the block's
                    // offset within the live region: owned →
                    // `live_history()[window_size - current_len..]` (byte-identical
                    // to the old `history[history.len()-current_len..]`); borrowed →
                    // `live_history()[block_start..]` (the in-place input, since the
                    // owned `history` mirror is empty under the no-copy path).
                    let current = &$m.live_history()[(current_abs_start - $m.history_abs_start)..];
                    let start = candidate.start - current_abs_start;
                    let literals = &current[literals_start..start];
                    $handle(Sequence::Triple {
                        literals,
                        offset: candidate.offset,
                        match_len: candidate.match_len,
                    });
                    let _ = encode_offset_with_history(
                        candidate.offset as u32,
                        literals.len() as u32,
                        &mut $m.offset_hist,
                    );
                    pos = start + candidate.match_len;
                    literals_start = pos;
                } else {
                    // Reuse the iteration's hash for the defer-path insert
                    // when available (a carried iteration never hashed and
                    // falls back to the rehashing insert).
                    match hash {
                        Some((row, tag)) => $m.insert_at::<$rl>(abs_pos, row, tag),
                        None => $m.insert_position::<$rl>(abs_pos),
                    }
                    if carried.is_some() || deferred_from_prev {
                        // Defer: the lookahead found a better match — step
                        // exactly one to take it next iteration. `deferred_from_prev`
                        // covers the depth-2 defer-without-carry miss so the skip
                        // below can't hop over the proven `abs_pos + 2` winner.
                        pos += 1;
                    } else {
                        // Complete miss: accelerate through weakly-matching
                        // stretches (upstream zstd `ip += (ip - anchor) >>
                        // kSearchStrength + 1`, zstd_lazy.c lazy_generic).
                        // Same softened `SKIP_STRENGTH = 10` as the greedy
                        // kernel above: the step stays 1 until the literal
                        // run hits ~1 KiB, protecting ratio on short runs.
                        const SKIP_STRENGTH: u32 = 10;
                        pos += ((lit_len as u32) >> SKIP_STRENGTH) as usize + 1;
                    }
                }
            }

            while pos + ROW_HASH_KEY_LEN <= current_len {
                $m.insert_position::<$rl>(current_abs_start + pos);
                pos += 1;
            }

            if literals_start < current_len {
                // Trailing literals of the current block. Read via
                // `live_history()` (borrowed-aware) sliced at the block's
                // offset within the live region: owned →
                // `live_history()[window_size - current_len..]` (byte-identical
                // to the old `history[history.len()-current_len..]`); borrowed →
                // `live_history()[block_start..]` (the in-place input, since the
                // owned `history` mirror is empty under the no-copy path).
                let current = &$m.live_history()[(current_abs_start - $m.history_abs_start)..];
                $handle(Sequence::Literals {
                    literals: &current[literals_start..],
                });
            }
        }
    }};
}

/// Per-tier lazy kernels (see `gen_greedy_monolith` — same SIMD pairing).
macro_rules! gen_lazy_monolith {
    ($name:ident, $use_mask:literal, $maskmac:ident, $cpl:path $(, $tf:literal)?) => {
        $(#[target_feature(enable = $tf)])?
        // wasm32+simd128 resolves the dispatch at compile time to the
        // simd128 kernel, leaving the scalar monolith uncalled there
        // (same shape as the `ScalarTags` allowance).
        #[cfg_attr(all(target_arch = "wasm32", target_feature = "simd128"), allow(dead_code))]
        #[allow(unused_unsafe)]
        unsafe fn $name<K: RowTags, const ROW_LOG: usize>(
            &mut self,
            mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
        ) {
            lazy_parse_body!(self, handle_sequence, ROW_LOG, $use_mask, $maskmac, $cpl)
        }
    };
}

/// Per-tier greedy kernels: the parse body AND its probe expand inline under
/// the tier's `#[target_feature]` umbrella with the tier's own SIMD pairing
/// (the `K` parameter is kept only so the dispatch site stays uniform).
macro_rules! gen_greedy_monolith {
    ($name:ident, $use_mask:literal, $maskmac:ident, $cpl:path $(, $tf:literal)?) => {
        $(#[target_feature(enable = $tf)])?
        // wasm32+simd128 resolves the dispatch at compile time to the
        // simd128 kernel, leaving the scalar monolith uncalled there
        // (same shape as the `ScalarTags` allowance).
        #[cfg_attr(all(target_arch = "wasm32", target_feature = "simd128"), allow(dead_code))]
        #[allow(unused_unsafe)]
        unsafe fn $name<K: RowTags, const ROW_LOG: usize>(
            &mut self,
            mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
        ) {
            greedy_parse_body!(self, handle_sequence, ROW_LOG, $use_mask, $maskmac, $cpl)
        }
    };
}

/// Bind the runtime `row_log` (clamped 4..=6) to the const `ROW_LOG` of a
/// `*_rl::<K, ROW_LOG>` hot loop. Mirrors the upstream zstd's per-rowLog variant
/// table; the branch is cold (once per block / call).
macro_rules! dispatch_row_log {
    ($self:ident . $rl_method:ident :: <$k:ty> ( $($arg:expr),* )) => {
        match $self.row_log {
            4 => $self.$rl_method::<$k, 4>($($arg),*),
            5 => $self.$rl_method::<$k, 5>($($arg),*),
            6 => $self.$rl_method::<$k, 6>($($arg),*),
            _ => unreachable!("row_log is clamped to 4..=6 in configure()"),
        }
    };
}

/// Row tag-match mask kernels as `macro_rules!` bodies (upstream zstd
/// `ZSTD_row_getMatchMask`). Per the SW-Rust SIMD rule, the SIMD body is a macro
/// expanded at the call site inside each per-kernel `#[target_feature]` probe so
/// the vector compare + movemask inline straight-line — `#[inline(always)]` +
/// `#[target_feature]` on a function is forbidden
/// (compiler restriction), so a
/// function call would otherwise cross the feature ABI boundary on every probe.
/// Each expands to a `u64` bitmask: bit `j` set iff `tags[j] == tag`. The
/// `row_tag_match_mask_*` wrapper fns below reuse these macros so the
/// bit-identity tests exercise the exact same code the hot path runs.
macro_rules! row_tag_mask_scalar {
    ($tags:expr, $tag:expr) => {{
        let tags: &[u8] = $tags;
        let tag: u8 = $tag;
        let mut mask = 0u64;
        for (j, &t) in tags.iter().enumerate() {
            if t == tag {
                mask |= 1u64 << j;
            }
        }
        mask
    }};
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
macro_rules! row_tag_mask_sse2 {
    ($tags:expr, $tag:expr) => {{
        #[cfg(target_arch = "x86")]
        use core::arch::x86::{_mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8};
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::{
            _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8,
        };
        let tags: &[u8] = $tags;
        let needle = _mm_set1_epi8($tag as i8);
        let mut mask = 0u64;
        let mut off = 0;
        while off + 16 <= tags.len() {
            // SAFETY: `off + 16 <= tags.len()`, so the 16-byte load is in bounds;
            // the enclosing fn carries `#[target_feature(enable = "sse2")]`.
            let v = unsafe { _mm_loadu_si128(tags.as_ptr().add(off) as *const _) };
            let eq = _mm_cmpeq_epi8(v, needle);
            mask |= (_mm_movemask_epi8(eq) as u16 as u64) << off;
            off += 16;
        }
        mask
    }};
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
macro_rules! row_tag_mask_avx2 {
    ($tags:expr, $tag:expr) => {{
        #[cfg(target_arch = "x86")]
        use core::arch::x86::{
            _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8, _mm256_cmpeq_epi8,
            _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_set1_epi8,
        };
        #[cfg(target_arch = "x86_64")]
        use core::arch::x86_64::{
            _mm_cmpeq_epi8, _mm_loadu_si128, _mm_movemask_epi8, _mm_set1_epi8, _mm256_cmpeq_epi8,
            _mm256_loadu_si256, _mm256_movemask_epi8, _mm256_set1_epi8,
        };
        let tags: &[u8] = $tags;
        let tag = $tag;
        let needle = _mm256_set1_epi8(tag as i8);
        let mut mask = 0u64;
        let mut off = 0;
        while off + 32 <= tags.len() {
            // SAFETY: `off + 32 <= tags.len()`; enclosing fn is `target_feature(avx2)`.
            let v = unsafe { _mm256_loadu_si256(tags.as_ptr().add(off) as *const _) };
            let eq = _mm256_cmpeq_epi8(v, needle);
            mask |= (_mm256_movemask_epi8(eq) as u32 as u64) << off;
            off += 32;
        }
        if off + 16 <= tags.len() {
            let needle16 = _mm_set1_epi8(tag as i8);
            // SAFETY: `off + 16 <= tags.len()`; enclosing fn is `target_feature(avx2)`.
            let v = unsafe { _mm_loadu_si128(tags.as_ptr().add(off) as *const _) };
            let eq = _mm_cmpeq_epi8(v, needle16);
            mask |= (_mm_movemask_epi8(eq) as u16 as u64) << off;
        }
        mask
    }};
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
macro_rules! row_tag_mask_neon {
    ($tags:expr, $tag:expr) => {{
        use core::arch::aarch64::{
            vceqq_u8, vdupq_n_u8, vgetq_lane_u8, vld1q_u8, vreinterpretq_u8_u64,
            vreinterpretq_u16_u8, vreinterpretq_u32_u16, vreinterpretq_u64_u32, vshrq_n_u8,
            vsraq_n_u16, vsraq_n_u32, vsraq_n_u64,
        };
        let tags: &[u8] = $tags;
        let needle = vdupq_n_u8($tag);
        let mut mask = 0u64;
        let mut off = 0;
        while off + 16 <= tags.len() {
            // SAFETY: `off + 16 <= tags.len()`; enclosing fn is `target_feature(neon)`.
            let v = unsafe { vld1q_u8(tags.as_ptr().add(off)) };
            let eq = vceqq_u8(v, needle);
            let high = vshrq_n_u8(eq, 7);
            let paired16 = vreinterpretq_u32_u16(vsraq_n_u16(
                vreinterpretq_u16_u8(high),
                vreinterpretq_u16_u8(high),
                7,
            ));
            let paired32 = vreinterpretq_u64_u32(vsraq_n_u32(paired16, paired16, 14));
            let paired64 = vreinterpretq_u8_u64(vsraq_n_u64(paired32, paired32, 28));
            let bits =
                (vgetq_lane_u8(paired64, 0) as u64) | ((vgetq_lane_u8(paired64, 8) as u64) << 8);
            mask |= bits << off;
            off += 16;
        }
        mask
    }};
}

// WebAssembly `simd128` tag-match mask: `i8x16_eq` against the broadcast tag,
// then `i8x16_bitmask` (wasm's direct 16-lane-to-16-bit movemask) over each
// 16-byte chunk. Shape mirrors the SSE2 kernel; `tags.len()` is a multiple of
// 16 so no scalar tail is needed. Compiled only under `target_feature =
// "simd128"`, so the intrinsics are available without a `#[target_feature]`
// attribute (no runtime detection on wasm); only `v128_load` is `unsafe`.
#[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
macro_rules! row_tag_mask_simd128 {
    ($tags:expr, $tag:expr) => {{
        use core::arch::wasm32::{i8x16_bitmask, i8x16_eq, i8x16_splat, v128_load};
        let tags: &[u8] = $tags;
        let needle = i8x16_splat($tag as i8);
        let mut mask = 0u64;
        let mut off = 0;
        while off + 16 <= tags.len() {
            // SAFETY: `off + 16 <= tags.len()`, so the 16-byte unaligned load is
            // in bounds.
            let v = unsafe { v128_load(tags.as_ptr().add(off) as *const _) };
            let eq = i8x16_eq(v, needle);
            mask |= (i8x16_bitmask(eq) as u64) << off;
            off += 16;
        }
        mask
    }};
}

/// Emit a per-kernel row match probe method on `RowMatchGenerator`. The body is
/// written ONCE here and stamped per tier under that tier's `#[target_feature]`
/// umbrella; the tag-match SIMD is expanded inline via the `$maskmac` macro (not
/// a function call), so the vector compare inlines straight-line — no
/// `#[target_feature]` ABI boundary on the per-probe hot path. Runtime kernel
/// selection happens once at the `dispatch_tag_kernel!` site; this method is the
/// per-tier monomorphised hot loop with no kernel branch inside it.
///
/// `$use_mask` is the compile-time bitmask-vs-byte-compare choice; `$maskmac` is
/// the tier's `row_tag_mask_*!`; the optional `$tf` is the `target_feature`.
/// Mirrors the former generic `row_candidate_rl`: live row probe, dict
/// dual-probe, speculative tail gate.
/// The row probe BODY as a macro, expanded both into the per-tier
/// `row_probe_*` functions (non-kernel callers) and directly into the
/// per-tier parse kernels (`greedy_*` / `lazy_*`) where a function-call
/// boundary — non-inlinable across `#[target_feature]` without an
/// `inline(always)` the compiler forbids there — cost a call with operand
/// spills per position. Early exits use the labeled block (`break 'probe`)
/// because a `return` inside a macro body would return from the EXPANSION
/// SITE's function.
macro_rules! row_probe_body {
    ($m:expr, $abs_pos:expr, $lit_len:expr, $hash:expr, $seed:expr, $rl:expr, $use_mask:literal, $maskmac:ident, $cpl:path) => {{
        #[allow(unused_labels)]
        'probe: {
            debug_assert_eq!($rl, $m.row_log);
            let mls = $m.mls;
            let concat = $m.live_history();
            let current_idx = $abs_pos - $m.history_abs_start;
            if current_idx + mls > concat.len() {
                break 'probe None;
            }

            // `hash` carries the (row, tag) the greedy loop already
            // computed for this position (and prefetched the row for);
            // recompute only on the uncarried paths.
            let (row, tag) = match $hash.or_else(|| $m.hash_and_row($abs_pos)) {
                Some(rt) => rt,
                None => break 'probe None,
            };
            let row_entries = 1usize << $rl;
            let row_mask = row_entries - 1;
            let row_base = row << $rl;
            let head = $m.row_heads[row] as usize;
            let max_walk = $m.search_depth.min(row_entries);

            // Prefetch the dict row before the live scan (upstream zstd
            // prefetches the dictMatchState rows up front,
            // zstd_lazy.c:1200 `ZSTD_row_prefetch`), hiding the dict-table
            // load latency behind the live row's work.
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            if let Some(dict) = $m.dict.table() {
                #[cfg(target_arch = "x86")]
                use core::arch::x86::{_MM_HINT_T0, _mm_prefetch};
                #[cfg(target_arch = "x86_64")]
                use core::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
                let drow_base = row << $rl;
                // SAFETY: prefetch is a hint and never faults; indexes are in
                // bounds by the dict-table sizing.
                unsafe {
                    _mm_prefetch(dict.tags.as_ptr().add(drow_base).cast(), _MM_HINT_T0);
                    _mm_prefetch(dict.positions.as_ptr().add(drow_base).cast(), _MM_HINT_T0);
                }
            }

            // SIMD tiers precompute the full bitmask once (the tag-match
            // intrinsic inlines under this method's `#[target_feature]`); the
            // scalar tier (`USE_MASK == false`) const-folds this away and does
            // an on-the-fly per-slot byte compare in the loop.
            let tag_match = if $use_mask {
                $maskmac!(&$m.row_tags[row_base..row_base + row_entries], tag)
            } else {
                0
            };

            // Seeded with the rep candidate (when present) so the tail-gate
            // below prunes row candidates against the rep length from the
            // first hit, and the rep value need not stay live separately
            // across the probe. Merge stays byte-identical: the seed is the
            // permanent lhs, exactly as the former trailing
            // `best_len_offset_candidate(rep, row)` merge made it.
            let mut best: Option<MatchCandidate> = $seed;
            // Upstream zstd `ZSTD_RowFindBestMatch` mask iteration: rotate the tag
            // mask into head (newest-first) order once, then visit ONLY the
            // set bits via tzcnt + clear-lowest. The former per-slot loop
            // burned slot arithmetic + a bit test on EVERY entry (rows are
            // 16-64 wide, typical tag hits 0-2) — ~14% of L10 wall time.
            // `max_walk` bounds ATTEMPTED candidates (upstream zstd `nbAttempts`
            // decrements per mask hit, not per scanned slot), so a depth
            // below the row width searches up to `depth` hits across the
            // WHOLE row — upstream zstd semantics on both the SIMD and scalar tiers
            // (the scalar arm advances to the next on-the-fly tag hit so
            // its visit order and attempt accounting stay bit-identical to
            // the mask tiers).
            let entries_bits: u64 = if row_entries >= 64 {
                u64::MAX
            } else {
                (1u64 << row_entries) - 1
            };
            #[allow(unused_mut)]
            let mut pending: u64 = if $use_mask {
                let m = tag_match & entries_bits;
                if head == 0 {
                    m
                } else {
                    ((m >> head) | (m << (row_entries - head))) & entries_bits
                }
            } else {
                0
            };
            #[allow(unused_mut)]
            let mut scan = 0usize;
            let mut attempts = 0usize;
            while attempts < max_walk {
                let slot_opt = if $use_mask {
                    if pending == 0 {
                        None
                    } else {
                        let i = pending.trailing_zeros() as usize;
                        pending &= pending - 1;
                        Some((head + i) & row_mask)
                    }
                } else {
                    let mut found = None;
                    while scan < row_entries {
                        let s = (head + scan) & row_mask;
                        scan += 1;
                        if $m.row_tags[row_base + s] == tag {
                            found = Some(s);
                            break;
                        }
                    }
                    found
                };
                let Some(slot) = slot_opt else { break };
                attempts += 1;
                let idx = row_base + slot;
                let raw_pos = $m.row_positions[idx];
                if raw_pos == ROW_EMPTY_SLOT {
                    continue;
                }
                let candidate_pos = raw_pos as usize;
                // Lower bound = window low. Owned: `history_abs_start` (eviction
                // floor) is always >= `abs_pos - max_window_size` (window_size <=
                // max_window_size), so the `max` picks it — byte-identical to the
                // pre-window_low check. Borrowed (history_abs_start forced to 0 in
                // set_borrowed_window): the `max` picks `abs_pos - max_window_size`,
                // capping the offset to the advertised window so an over-window
                // in-place scan never emits an unresolvable offset.
                let window_low = $m
                    .history_abs_start
                    .max($abs_pos.saturating_sub($m.max_window_size));
                if candidate_pos < window_low || candidate_pos >= $abs_pos {
                    continue;
                }
                let candidate_idx = candidate_pos - $m.history_abs_start;
                // NOTE: upstream zstd's 4-byte head gate (`MEM_read32(match)
                // == MEM_read32(ip)`, zstd_lazy.c:1265) was measured NEGATIVE
                // here both unconditionally (+7% on match-dense z000033 L6,
                // flat control) and best-gated (+3%); the row walk visits few
                // false tag hits and the SIMD prefix compare's first vector
                // already serves as the cheap reject. The tail gate below is
                // the selective filter that pays.
                // Speculative tail gate (HC `hash_chain_candidate` parity):
                // a 4-byte compare at the length the candidate must reach to
                // outgrow `best` proves whether the full `common_prefix_len`
                // can pay off. Gated on offset-monotonicity since the row walk
                // is not offset-ordered. Ratio-neutral.
                if let Some(b) = best {
                    let new_offset = $abs_pos - candidate_pos;
                    if new_offset >= b.offset
                        && let Some(tail_off) = b.match_len.checked_sub($lit_len + 3)
                    {
                        let m_end = candidate_idx + tail_off + 4;
                        let i_end = current_idx + tail_off + 4;
                        if i_end > concat.len()
                            || m_end > concat.len()
                            || concat[candidate_idx + tail_off..m_end]
                                != concat[current_idx + tail_off..i_end]
                        {
                            continue;
                        }
                    }
                }
                // Per-tier `common_prefix_len_ptr` expanded inline (same feature
                // umbrella as this probe) — no `dispatch_common_prefix_len_ptr`
                // runtime match + `#[target_feature]` call per candidate. `max =
                // concat.len() - current_idx` since `candidate_idx < current_idx`.
                let match_len = unsafe {
                    $cpl(
                        concat.as_ptr().add(candidate_idx),
                        concat.as_ptr().add(current_idx),
                        concat.len() - current_idx,
                    )
                };
                if match_len >= mls {
                    let candidate =
                        $m.extend_backwards(candidate_pos, $abs_pos, match_len, $lit_len);
                    best = best_len_offset_candidate(best, Some(candidate));
                    if best.is_some_and(|b| current_idx + b.match_len >= concat.len()) {
                        break 'probe best;
                    }
                }
            }

            // Dict dual-probe (upstream zstd `ZSTD_RowFindBestMatch` `dictMatchState`):
            // one bounded immutable dict row (concat-indexed positions).
            // The candidate budget is SHARED with the live row (upstream
            // zstd decrements one `nbAttempts` across both rows,
            // zstd_lazy.c:1308): the dict probe only spends what the live
            // walk left over.
            // Match upstream zstd's effective dict search depth: the CDict path
            // adjusts cParams (dict-size-aware searchLog) so the dict probe runs
            // deeper than the bare level searchLog — measured nbAttempts >= 16 to
            // surface the long dict match on the per-label-dict fixtures, vs the
            // 8 from L6's searchLog=3. Floor the dict budget at 16 (a full
            // rowLog-4 row); the dpending mask still bounds it to the row width.
            let dict_budget = max_walk.max(16);
            if attempts < dict_budget
                && let Some(dict) = $m.dict.table()
            {
                let dict_walk = dict_budget - attempts;
                let dict_end = $m.dict.region_len();
                let drow_base = row << $rl;
                let dhead = dict.heads[row] as usize;
                let dtag_match = if $use_mask {
                    $maskmac!(&dict.tags[drow_base..drow_base + row_entries], tag)
                } else {
                    0
                };
                // Same upstream zstd mask iteration as the live row above.
                #[allow(unused_mut)]
                let mut dpending: u64 = if $use_mask {
                    let m = dtag_match & entries_bits;
                    if dhead == 0 {
                        m
                    } else {
                        ((m >> dhead) | (m << (row_entries - dhead))) & entries_bits
                    }
                } else {
                    0
                };
                #[allow(unused_mut)]
                let mut dscan = 0usize;
                let mut dattempts = 0usize;
                while dattempts < dict_walk {
                    let slot_opt = if $use_mask {
                        if dpending == 0 {
                            None
                        } else {
                            let i = dpending.trailing_zeros() as usize;
                            dpending &= dpending - 1;
                            Some((dhead + i) & row_mask)
                        }
                    } else {
                        let mut found = None;
                        while dscan < row_entries {
                            let s = (dhead + dscan) & row_mask;
                            dscan += 1;
                            if dict.tags[drow_base + s] == tag {
                                found = Some(s);
                                break;
                            }
                        }
                        found
                    };
                    let Some(slot) = slot_opt else { break };
                    dattempts += 1;
                    let didx = drow_base + slot;
                    let dp = dict.positions[didx];
                    if dp == ROW_EMPTY_SLOT {
                        continue;
                    }
                    let dp = dp as usize;
                    if dp >= dict_end || dp + mls > concat.len() {
                        continue;
                    }
                    let cand_abs = $m.history_abs_start + dp;
                    if let Some(b) = best {
                        let new_offset = $abs_pos - cand_abs;
                        if new_offset >= b.offset
                            && let Some(tail_off) = b.match_len.checked_sub($lit_len + 3)
                        {
                            let m_end = dp + tail_off + 4;
                            let i_end = current_idx + tail_off + 4;
                            if i_end > concat.len()
                                || m_end > concat.len()
                                || concat[dp + tail_off..m_end]
                                    != concat[current_idx + tail_off..i_end]
                            {
                                continue;
                            }
                        }
                    }
                    let match_len = unsafe {
                        $cpl(
                            concat.as_ptr().add(dp),
                            concat.as_ptr().add(current_idx),
                            concat.len() - current_idx,
                        )
                    };
                    if match_len >= mls {
                        let candidate =
                            $m.extend_backwards(cand_abs, $abs_pos, match_len, $lit_len);
                        best = best_len_offset_candidate(best, Some(candidate));
                        if best.is_some_and(|b| current_idx + b.match_len >= concat.len()) {
                            break 'probe best;
                        }
                    }
                }
            }
            best
        }
    }};
}

macro_rules! gen_row_probe {
    ($name:ident, $use_mask:literal, $maskmac:ident, $cpl:path $(, $tf:literal)?) => {
        $(#[target_feature(enable = $tf)])?
        // `#[inline]` hint (NOT always — forbidden with target_feature):
        // the per-tier parse umbrella enables the same features, so the
        // probe is inlinable there and LLVM takes the single-call-site
        // hint, merging probe + parse loop into one body.
        #[inline]
        #[allow(unused_unsafe)]
        // wasm32+simd128 selects `row_probe_simd128` at compile time, leaving
        // `row_probe_scalar` (the only other tier compiled on wasm) unused.
        #[cfg_attr(all(target_arch = "wasm32", target_feature = "simd128"), allow(dead_code))]
        unsafe fn $name<const ROW_LOG: usize>(
            &self,
            abs_pos: usize,
            lit_len: usize,
            hash: Option<(usize, u8)>,
        ) -> Option<MatchCandidate> {
            // Standalone probe: the caller merges the rep candidate, so the
            // probe itself starts unseeded.
            row_probe_body!(self, abs_pos, lit_len, hash, None, ROW_LOG, $use_mask, $maskmac, $cpl)
        }
    };
}

// Reference mask wrappers for the bit-identity tests (`tag_mask_tests`). The
// `row_tag_mask_*!` macros are the production source of truth (expanded inline
// in the per-kernel `row_probe_*` methods); these fns just give the tests a
// callable handle to assert SIMD == scalar. Gated to the same cfg as the test
// module so they carry no weight in production builds.

#[derive(Clone)]
pub(crate) struct RowMatchGenerator {
    pub(crate) max_window_size: usize,
    /// Per-committed-block lengths of the live window, mirroring the
    /// `HashChain` backend's `chunk_lens`. The block bytes themselves live
    /// only in the contiguous `history` mirror; the input buffers are handed
    /// straight back to the caller's pool in `add_data` rather than retained
    /// here. Retaining them (the old `VecDeque<Vec<u8>>`) held a full
    /// `block_capacity`-sized buffer per committed block, which on a heavily
    /// pre-split frame ballooned the window to many times the live byte count.
    pub(crate) chunk_lens: VecDeque<usize>,
    pub(crate) window_size: usize,
    pub(crate) history: Vec<u8>,
    pub(crate) history_start: usize,
    pub(crate) history_abs_start: usize,
    pub(crate) offset_hist: [u32; 3],
    pub(crate) row_hash_log: usize,
    pub(crate) row_log: usize,
    pub(crate) search_depth: usize,
    pub(crate) target_len: usize,
    /// Regular-search min-match floor (upstream zstd `cParams.minMatch`). A row
    /// candidate must extend to >= `mls` bytes to be accepted. Hoisted to
    /// a local in the parse loops so the per-position compare reads a
    /// register, not this field. Default `ROW_MIN_MATCH_LEN` (5).
    pub(crate) mls: usize,
    pub(crate) lazy_depth: u8,
    /// Cached fastpath kernel for `hash_mix_u64`; see Dfast for rationale.
    pub(crate) hash_kernel: FastpathKernel,
    pub(crate) row_heads: Vec<u8>,
    // Absolute match positions, one per row slot. Stored as `u32` (not
    // `usize`): this is the largest match-finder array, and `u32` halves its
    // footprint vs the upstream zstd-parity `U32` layout. `ROW_EMPTY_SLOT == u32::MAX`
    // is the empty sentinel, so every stored position must stay strictly below
    // it. On a long stream the cumulative absolute cursor would cross `u32::MAX`
    // even while the live window is bounded; `add_data` rebases the coordinate
    // origin down to the oldest live byte before that happens (see
    // [`Self::rebase_positions`]), keeping positions representable without
    // capping frame length.
    pub(crate) row_positions: Vec<u32>,
    pub(crate) row_tags: Vec<u8>,
    /// Cached tag-match SIMD kernel; CPU features are fixed per process, so
    /// resolve once instead of querying per `row_candidate` call. On
    /// wasm32+simd128 the tier is compile-time (`dispatch_tag_kernel!` selects
    /// `Simd128Tags` directly), so the field is unread there.
    #[cfg_attr(
        all(target_arch = "wasm32", target_feature = "simd128"),
        allow(dead_code)
    )]
    tag_kernel: FastpathKernel,
    /// Attached immutable dictionary row index (upstream zstd `dictMatchState`). `Some`
    /// activates the bounded dict probe in `row_candidate_rl`; built once and
    /// cached across frames via `DictAttach`, invalidated on eviction / resize.
    pub(crate) dict: DictAttach<RowDictTables>,
    /// Borrowed (no-copy) one-shot input window: `(ptr, len)` into the
    /// caller's slice. When set, the borrowed scan reads candidate/cursor
    /// bytes straight from here instead of the owned `history` mirror, so an
    /// over-window one-shot input is matched in place (no input->mirror copy).
    /// Raw pointer: the slice must stay live until `clear_borrowed_window` /
    /// `reset` (same contract as the Dfast/Simple borrowed backends).
    pub(crate) borrowed_input: Option<(*const u8, usize)>,
    /// Active borrowed block range `[start, end)` within `borrowed_input`,
    /// staged before each borrowed scan so `live_history()` exposes
    /// `[0, end)` and the parse loop scans `[start, end)`.
    pub(crate) borrowed_block: Option<(usize, usize)>,
}

impl RowMatchGenerator {
    pub(crate) fn new(max_window_size: usize) -> Self {
        Self {
            max_window_size,
            chunk_lens: VecDeque::new(),
            window_size: 0,
            history: Vec::new(),
            history_start: 0,
            history_abs_start: 0,
            offset_hist: [1, 4, 8],
            row_hash_log: ROW_HASH_BITS - ROW_LOG,
            row_log: ROW_LOG,
            search_depth: ROW_SEARCH_DEPTH,
            target_len: ROW_TARGET_LEN,
            mls: ROW_MIN_MATCH_LEN,
            lazy_depth: 1,
            hash_kernel: crate::encoding::fastpath::select_kernel(),
            row_heads: Vec::new(),
            row_positions: Vec::new(),
            row_tags: Vec::new(),
            tag_kernel: crate::encoding::fastpath::select_kernel(),
            dict: DictAttach::new(),
            borrowed_input: None,
            borrowed_block: None,
        }
    }

    /// Heap bytes this matcher owns: history, the row head/position/tag tables,
    /// the chunk-length deque, and any attached dictionary row index.
    pub(crate) fn heap_size(&self) -> usize {
        let u32_sz = size_of::<u32>();
        self.chunk_lens.capacity() * size_of::<usize>()
            + self.history.capacity()
            + self.row_heads.capacity()
            + self.row_positions.capacity() * u32_sz
            + self.row_tags.capacity()
            + self.dict.table().map_or(0, |t| {
                t.heads.capacity() + t.positions.capacity() * u32_sz + t.tags.capacity()
            })
    }

    /// Effective row hash width currently configured (`row_hash_log +
    /// row_log`). The primed-snapshot key records THIS value — the
    /// configured request may exceed the [`ROW_HASH_BITS`] cap below, and
    /// keying on the request while the tables use the clamped width forces
    /// needless dictionary re-primes.
    pub(crate) fn hash_bits(&self) -> usize {
        self.row_hash_log + self.row_log
    }

    pub(crate) fn set_hash_bits(&mut self, bits: usize) {
        // Deliberate deviation from upstream zstd hashLog 21-23 on L9-12: the
        // 20-bit cap keeps the row table L2/L3-resident. Measured on the
        // 1 MiB corpus at L10 (tight pair, flat control): the honest
        // 21-bit table cost +26.8% wall for a 19-byte output delta — our
        // lazy-band ratio already beats the upstream zstd with the capped width.
        let clamped = bits.clamp(self.row_log + 1, ROW_HASH_BITS);
        let row_hash_log = clamped.saturating_sub(self.row_log);
        if self.row_hash_log != row_hash_log {
            self.row_hash_log = row_hash_log;
            self.row_heads.clear();
            self.row_positions.clear();
            self.row_tags.clear();
            // NOTE: do NOT invalidate the dict here. `set_hash_bits` is called
            // twice per frame during level setup (once from `configure` with
            // the level's `hash_bits`, once with the hint-resolved table bits),
            // so `row_hash_log` oscillates every frame even when the level is
            // unchanged. Invalidating here would drop the CDict cache on every
            // frame. The dict is rebuilt by `prime_dict_rows` AFTER setup (final
            // shape), and `prime_dict_rows` self-invalidates a cached index whose
            // shape no longer matches — so a genuine level change is handled
            // there, while the per-frame oscillation is ignored.
        }
    }

    pub(crate) fn configure(&mut self, config: RowConfig) {
        self.row_log = config.row_log.clamp(4, 6);
        self.search_depth = config.search_depth;
        self.target_len = config.target_len;
        // Clamp the min-match floor to >= the hash key width (a shorter
        // floor can't be satisfied: the hash only surfaces candidates
        // sharing the 4-byte key) and a sane upper bound.
        self.mls = config.mls.clamp(ROW_HASH_KEY_LEN, 7);
        self.set_hash_bits(config.hash_bits.max(self.row_log + 1));
    }

    pub(crate) fn reset(&mut self) {
        // Floor-advance reset (same shape as the dfast/HC backends): instead
        // of re-zeroing the row tables per frame (a multi-MiB memset that
        // dominated small/medium-frame encode), advance the absolute
        // coordinate floor past everything ever inserted. Stale entries all
        // hold positions below the new floor, so the probes' existing
        // `candidate_pos < self.history_abs_start` window check rejects them
        // without any clearing — the upstream zstd's persistent-index design. Stale
        // TAGS can still produce the occasional false mask hit whose
        // candidate then fails the window check; the upstream zstd's tag table
        // persists across frames with the same behaviour.
        let next_floor = self.history_abs_start + (self.history.len() - self.history_start);
        self.window_size = 0;
        self.history.clear();
        self.history_start = 0;
        self.offset_hist = [1, 4, 8];
        // Clear borrowed-window state so a following OWNED frame's
        // `current_block_range()` / `live_history()` read the owned mirror,
        // not a stale borrowed range. A borrowed frame re-arms via
        // `set_borrowed_window` after this reset.
        self.borrowed_input = None;
        self.borrowed_block = None;
        if next_floor <= REBASE_RESET_FLOOR_CEILING && !self.row_positions.is_empty() {
            self.history_abs_start = next_floor;
        } else {
            // Bounded fallback: rewind the coordinate space and zero the
            // tables so the absolute cursor cannot climb without bound
            // (mirrors dfast; the u32 packing is separately kept in range
            // by `rebase_positions` in `add_data`).
            self.history_abs_start = 0;
            self.row_heads.fill(0);
            self.row_positions.fill(ROW_EMPTY_SLOT);
            self.row_tags.fill(0);
        }
        // Block buffers are returned to the caller's pool per block in
        // `add_data`, so there is nothing window-side to recycle here.
        self.chunk_lens.clear();
    }

    pub(crate) fn get_last_space(&self) -> &[u8] {
        // SAFETY: the caller keeps the registered input live; staging validates its range.
        unsafe {
            super::history::get_last_space(
                &self.history,
                &self.chunk_lens,
                self.borrowed_input,
                self.borrowed_block,
            )
        }
    }

    pub(crate) fn add_data(&mut self, data: Vec<u8>, mut reuse_space: impl FnMut(Vec<u8>)) {
        assert!(data.len() <= self.max_window_size);
        super::match_table::storage::check_stream_abs_headroom(
            self.history_abs_start,
            self.window_size,
            data.len(),
        );
        // Row stores absolute match positions as `u32` (with `u32::MAX` the
        // empty sentinel). On a long stream the cumulative absolute cursor
        // crosses the u32 range even while the live window stays bounded, so
        // rebase the coordinate origin down to the oldest live byte before the
        // upcoming block's positions would overflow. Cold path — fires at most
        // once per ~4 GiB of stream, and one rebase always suffices because the
        // live window is far smaller than u32::MAX. `check_stream_abs_headroom`
        // above already guards the 32-bit-target `usize` overflow separately.
        if self.history_abs_start + self.window_size + data.len() >= u32::MAX as usize - 1 {
            self.rebase_positions();
        }
        if self.window_size + data.len() > self.max_window_size {
            // Eviction advances `history_start`, staling the dict row index's
            // concat positions — drop the attach (dict slid within/out window).
            self.dict.invalidate();
            // Cap the history buffer near the live window instead of letting
            // the Vec power-of-two double to ~2x window on long streams. Once
            // eviction starts, reserve exactly (window + window/4 + one block)
            // so the buffer grows linearly to that ceiling; `compact_history`'s
            // quarter-window drain then keeps `len` under it, so the Vec never
            // reallocates again. Only fires in the eviction regime (large
            // inputs that fill the window) — small frames keep their tight
            // data-sized buffer untouched.
            let target = self.max_window_size
                + (self.max_window_size >> 2)
                + crate::common::MAX_BLOCK_SIZE as usize;
            if target > self.history.len() && self.history.capacity() < target {
                self.history.reserve_exact(target - self.history.len());
            }
        }
        while self.window_size + data.len() > self.max_window_size {
            let removed_len = self.chunk_lens.pop_front().unwrap();
            self.window_size -= removed_len;
            self.history_start += removed_len;
            self.history_abs_start += removed_len;
        }
        self.compact_history();
        let added = data.len();
        self.history.extend_from_slice(&data);
        self.window_size += added;
        self.chunk_lens.push_back(added);
        // The bytes now live in `history`; return the input buffer to the
        // caller's pool instead of holding a second copy in the window.
        reuse_space(data);
    }

    pub(crate) fn trim_to_window(&mut self) {
        if self.window_size > self.max_window_size {
            self.dict.invalidate();
        }
        while self.window_size > self.max_window_size {
            let removed_len = self.chunk_lens.pop_front().unwrap();
            self.window_size -= removed_len;
            self.history_start += removed_len;
            self.history_abs_start += removed_len;
        }
    }

    /// Rebase the absolute coordinate origin down to the oldest live byte so
    /// stored `u32` match positions stay representable on long (multi-GiB)
    /// streams. Cold path, driven from [`Self::add_data`] when the cursor
    /// nears `u32::MAX`. Subtracts the current `history_abs_start` from every
    /// live `row_positions` entry; entries older than the new origin (already
    /// unreachable through the `candidate_pos < history_abs_start` read guard)
    /// collapse to `ROW_EMPTY_SLOT`. The shift is uniform across the origin and
    /// every stored position, so every match offset is preserved and matching
    /// is unaffected. `row_heads` (slot cursors) and `row_tags` (hash tags)
    /// hold no absolute positions and are left untouched.
    fn rebase_positions(&mut self) {
        let delta = self.history_abs_start;
        if delta == 0 {
            return;
        }
        for slot in self.row_positions.iter_mut() {
            if *slot == ROW_EMPTY_SLOT {
                continue;
            }
            let abs = *slot as usize;
            *slot = if abs < delta {
                ROW_EMPTY_SLOT
            } else {
                (abs - delta) as u32
            };
        }
        self.history_abs_start -= delta;
    }

    pub(crate) fn skip_matching_with_hint_rl<const ROW_LOG: usize>(
        &mut self,
        incompressible_hint: Option<bool>,
    ) {
        debug_assert_eq!(ROW_LOG, self.row_log);
        self.ensure_tables();
        let (current_abs_start, current_len) = self.current_block_range();
        let current_abs_end = current_abs_start + current_len;
        let backfill_start = self.backfill_start(current_abs_start);
        if backfill_start < current_abs_start {
            self.insert_positions::<ROW_LOG>(backfill_start, current_abs_start);
        }
        match incompressible_hint {
            Some(true) => {
                // Sparse step + dense tail: caller declared the block
                // unlikely to compress, so we seed only every
                // `INCOMPRESSIBLE_SKIP_STEP` position plus a small tail to
                // keep cross-block continuity at the boundary.
                self.insert_positions_with_step::<ROW_LOG>(
                    current_abs_start,
                    current_abs_end,
                    INCOMPRESSIBLE_SKIP_STEP,
                );
                let dense_tail = ROW_MIN_MATCH_LEN + INCOMPRESSIBLE_SKIP_STEP;
                let tail_start = current_abs_end
                    .saturating_sub(dense_tail)
                    .max(current_abs_start);
                for pos in tail_start..current_abs_end {
                    if !(pos - current_abs_start).is_multiple_of(INCOMPRESSIBLE_SKIP_STEP) {
                        self.insert_position::<ROW_LOG>(pos);
                    }
                }
            }
            Some(false) => {
                // Dense seeding requested by the caller: the entire
                // skipped range must remain queryable so subsequent
                // blocks can match into it. Currently only used by the
                // dictionary-priming path (upstream zstd's
                // `ZSTD_loadDictionaryContent` does the same dense fill
                // via `ZSTD_row_update_internalImpl` over every dict
                // byte), but the semantic is "dense fill on demand" and
                // future fast-paths (e.g. an RLE / raw-block emitter
                // that still wants cross-block matches into the skipped
                // bytes) can reuse it without rewording the contract.
                self.insert_positions::<ROW_LOG>(current_abs_start, current_abs_end);
            }
            None => {
                // Upstream zstd parity: a plain `skip_matching` (no hint) leaves
                // the row table untouched for the skipped range. Upstream zstd's
                // `ZSTD_row_fillHashCache` only pre-fills the next-scan
                // cache (8 positions of lookahead for SIMD prefetch); it
                // does NOT retroactively insert every byte of a skipped
                // block.
                //
                // Boundary handling: the `backfill_start` insert above
                // covers the `ROW_HASH_KEY_LEN - 1` bytes immediately
                // BEFORE `current_abs_start` (i.e. the previous block's
                // tail), keeping the current block's start hashable as
                // a cross-block match target. The CURRENT skipped
                // block's tail (the `ROW_HASH_KEY_LEN - 1` bytes ending
                // at `current_abs_end`) is itself backfilled lazily —
                // by the NEXT call's own `backfill_start` insert when
                // that call's `current_abs_start` lands at
                // `current_abs_end`. So a parse of block N+1 sees
                // block N's tail in the row table but not its
                // interior, matching upstream zstd.
                //
                // Trade: cross-block matches into a skipped block's
                // interior are lost (rare in practice — `skip_matching`
                // is called on blocks the driver upstream identified as
                // not worth scanning), but the per-block O(block_size)
                // `insert_position` storm is gone. On the L4 large-log-
                // stream bench (~104 MB / 800 blocks) the prior dense
                // fill dominated ~25% of Rust self-time at 131K inserts
                // per block × 800 = ~104M inserts.
            }
        }
    }
    /// Upstream zstd-parity greedy parse for `lazy_depth == 0` (level 5).
    ///
    /// Mirrors `ZSTD_compressBlock_lazy_generic` (`zstd_lazy.c:1560`) with
    /// `depth == 0`, `dictMode == ZSTD_noDict`. The structural features
    /// that distinguish this greedy parse from the lazy parse in
    /// [`Self::start_matching`] (which `lazy_depth >= 1` strategies use):
    ///
    /// 1. **Default `start = pos + 1`**: each iteration first probes the
    ///    repcode bank at `abs_pos + 1` (treating one literal byte as
    ///    already committed). Upstream zstd's `start = ip + 1; matchLength = 0;
    ///    offBase = REPCODE1_TO_OFFBASE;` at the top of the loop body.
    ///    Only if a regular match at `abs_pos` is strictly longer does
    ///    `start` slide back to `abs_pos`. This trades one literal byte
    ///    for an unconditional repcode probe, which is the algorithmic
    ///    reason the strategy is called "greedy" — it greedily picks the
    ///    cheaper repcode encoding (4-5 bits) over a longer-offset
    ///    regular match (9-13 bits) whenever the rep hit is close to
    ///    matching the regular match's length.
    ///
    /// 2. **Hybrid commit, not upstream zstd's pure `goto _storeSequence`**:
    ///    upstream zstd's depth-0 path jumps to `_storeSequence` on the first
    ///    repcode hit and skips the regular search at `abs_pos`. We
    ///    deviate here — both the rep probe at `abs_pos + 1` *and* the
    ///    regular `row_candidate(abs_pos, ..)` are evaluated each
    ///    iteration, and the longer match wins (ties go to rep for
    ///    cheaper encoding via [`best_len_offset_candidate`]). Upstream zstd
    ///    can afford pure commit-on-first-rep because it recovers any
    ///    ratio loss via superblock-level entropy sharing, which we
    ///    don't replicate yet, so the hybrid form avoids a measured
    ///    ratio cliff on decodecorpus. (The row accept floor itself now
    ///    matches upstream zstd's `minMatch = 5` via `ROW_MIN_MATCH_LEN`; the
    ///    remaining un-replicated piece is the cross-block entropy
    ///    sharing, not the match-length threshold.) The hybrid form
    ///    still skips the upstream zstd `lazy_depth == 1` lookahead probe
    ///    that [`start_matching`] above runs unconditionally — the
    ///    speed shape stays upstream zstd-like.
    ///
    /// 3. **Skip-step grows with literal-run length**: on a miss upstream zstd
    ///    advances `ip += ((ip - anchor) >> kSearchStrength) + 1` with
    ///    `kSearchStrength = 8`. The plain matcher steps by 1 — denser
    ///    hash inserts (mild ratio benefit), but the upstream zstd parity skip
    ///    halves the per-byte work on incompressible runs (the
    ///    `lazySkipping` mode in upstream zstd is an extension of the same idea).
    ///
    /// Upstream zstd has an immediate-rep loop after store that probes
    /// `offset_2` for back-to-back hits. It is omitted here: the
    /// main-loop rep probe at `abs_pos + 1` already evaluates all
    /// three rep slots (rep1, rep2, rep3 + the upstream zstd `ll0` fallback)
    /// via [`repcode_candidate_shared`], so the inner-loop slot
    /// upstream zstd's single-rep design would catch is already covered by
    /// the next main-loop iteration. Confirmed dead-on-arrival via a
    /// `panic!` probe across the full 528-test suite + benchmark
    /// matrix (never fires).
    ///
    /// Catch-up backwards extension is already absorbed into the
    /// `MatchCandidate.start` field by `extend_backwards_shared`
    /// (called from `row_candidate` and `repcode_candidate_shared`),
    /// so we don't redo it explicitly.
    ///
    /// `pick_lazy_match` is intentionally not called here — depth == 0
    /// means "no lookahead", emit the first viable hit.
    pub(crate) fn ensure_tables(&mut self) {
        let row_count = 1usize << self.row_hash_log;
        let row_entries = 1usize << self.row_log;
        let total = row_count * row_entries;
        if self.row_positions.len() != total {
            // Resize in place: `set_hash_bits` width changes `clear()` the
            // vecs but keep their capacity. The previous `vec![..]` form
            // re-allocated all three tables on every width change — three
            // malloc/free pairs (~40 KiB) per hinted frame while the
            // configure→hint width pair disagreed, which allocator-slow
            // targets (musl) amplified into the dominant per-frame cost.
            self.row_heads.clear();
            self.row_heads.resize(row_count, 0);
            self.row_positions.clear();
            self.row_positions.resize(total, ROW_EMPTY_SLOT);
            self.row_tags.clear();
            self.row_tags.resize(total, 0);
        }
    }

    fn compact_history(&mut self) {
        super::history::compact_history(
            &mut self.history,
            &mut self.history_start,
            self.max_window_size,
        );
    }

    pub(crate) fn live_history(&self) -> &[u8] {
        // SAFETY: set_borrowed_window requires the input to stay live and unmodified.
        unsafe {
            super::history::live_history(
                &self.history,
                self.history_start,
                self.borrowed_input,
                self.borrowed_block,
            )
        }
    }

    fn history_abs_end(&self) -> usize {
        self.history_abs_start + self.live_history().len()
    }

    /// Register the borrowed input window (the whole caller slice). Borrowed
    /// blocks staged after this read their bytes from `buffer` in place.
    ///
    /// Zeroes `history_abs_start`: borrowed positions are absolute input
    /// offsets (0-based), so the floor-advance reset's persistent non-zero
    /// floor must be cleared for the duration of the borrowed frame. The
    /// owned history is unused while borrowed (no `add_data` copy), so this
    /// is safe; every probed candidate is byte-verified by the prefix
    /// compare, so any stale table entry left from a prior frame is rejected
    /// (or, if its bytes coincidentally match, is a genuine in-window match).
    ///
    /// # Safety
    /// `buffer` must stay live and unmodified until `clear_borrowed_window`
    /// or `reset` — the matcher stores a raw pointer into it.
    pub(crate) unsafe fn set_borrowed_window(&mut self, buffer: &[u8]) {
        super::history::set_borrowed_window(
            buffer,
            &mut self.borrowed_input,
            &mut self.borrowed_block,
            &mut self.history_abs_start,
        );
    }

    pub(crate) fn clear_borrowed_window(&mut self) {
        self.borrowed_input = None;
        self.borrowed_block = None;
    }

    /// Stage `[block_start, block_end)` as the active borrowed block before a
    /// scan so `live_history()` / `current_block_range()` report it.
    pub(crate) fn stage_borrowed_block(&mut self, block_start: usize, block_end: usize) {
        super::history::stage_borrowed_block(
            self.borrowed_input,
            &mut self.borrowed_block,
            block_start,
            block_end,
        );
    }

    /// `(current_abs_start, current_len)` for the active scan. Borrowed: the
    /// staged block range (absolute input offsets). Owned: derived from the
    /// last committed chunk in the live window.
    fn current_block_range(&self) -> (usize, usize) {
        super::history::current_block_range(
            self.borrowed_block,
            &self.chunk_lens,
            self.history_abs_start,
            self.window_size,
        )
    }

    /// Row hash key at `idx`: `key_len` bytes (upstream zstd `mls`, 5-6 on the row
    /// levels) via one masked 8-byte read, degrading to the 4-byte key in
    /// the last <8 bytes of the window. Shared by the live hash and the
    /// dictionary row-index build — the two MUST bucket identically or
    /// dict-region probes go blind.
    ///
    /// The degradation is per window STATE: within one window a position
    /// hashes identically in the probe and the insert. The last <8
    /// positions of a pre-primed dictionary are a separate, unfixable
    /// case — the bytes following them exist only at probe time, so no
    /// fixed build-time key (4-byte, zero-padded, or otherwise) can match
    /// the probe's real-byte key there. Those few dict-tail entries stay
    /// unreachable, mirroring the upstream zstd, whose dictionary load also stops
    /// hashing short of the dictionary end.
    #[inline(always)]
    fn row_key_value(concat: &[u8], idx: usize, key_len: usize) -> u64 {
        if idx + 8 <= concat.len() {
            let v = u64::from_le_bytes(*concat[idx..].first_chunk::<8>().unwrap());
            v & ((1u64 << (key_len * 8)) - 1)
        } else {
            u32::from_le_bytes(*concat[idx..].first_chunk::<ROW_HASH_KEY_LEN>().unwrap()) as u64
        }
    }

    #[inline(always)]
    pub(crate) fn hash_and_row(&self, abs_pos: usize) -> Option<(usize, u8)> {
        let idx = abs_pos - self.history_abs_start;
        let concat = self.live_history();
        if idx + ROW_HASH_KEY_LEN > concat.len() {
            return None;
        }
        // Upstream zstd `ZSTD_hashPtrSalted` hashes `mls` bytes (5-6 on the row
        // levels), not 4: a wider key cuts the false tag hits whose
        // candidates then cost a data load + reject in the probe. Read 8
        // bytes and mask to the key width when the tail allows; the last
        // <8 bytes of the window keep the 4-byte key (a position hashes
        // identically in the probe and the insert either way, so the
        // mixed tail stays self-consistent).
        let value = Self::row_key_value(concat, idx, self.mls.min(6));
        let hash = crate::encoding::fastpath::hash_mix_u64_with_kernel(self.hash_kernel, value);
        let total_bits = self.row_hash_log + ROW_TAG_BITS;
        let combined = hash >> (u64::BITS as usize - total_bits);
        let row_mask = (1usize << self.row_hash_log) - 1;
        let row = ((combined >> ROW_TAG_BITS) as usize) & row_mask;
        let tag = combined as u8;
        Some((row, tag))
    }

    fn backfill_start(&self, current_abs_start: usize) -> usize {
        current_abs_start
            .saturating_sub(ROW_HASH_KEY_LEN - 1)
            .max(self.history_abs_start)
    }

    /// Used only by the dead-code [`Self::start_matching`] (lazy-style
    /// row parse). Kept paired with that method so reviving the lazy
    /// path doesn't have to re-derive the rep+row best-of-two pick.
    #[inline(always)]
    pub(crate) fn best_match_rl<K: RowTags, const ROW_LOG: usize>(
        &self,
        abs_pos: usize,
        lit_len: usize,
    ) -> Option<MatchCandidate> {
        let rep = self.repcode_candidate(abs_pos, lit_len);
        // SAFETY: `K` selected by `dispatch_tag_kernel!` after `detect` confirmed
        // its ISA; `K::probe` upholds the per-tier feature contract.
        let row = unsafe { K::probe::<ROW_LOG>(self, abs_pos, lit_len, None) };
        best_len_offset_candidate(rep, row)
    }

    #[inline(always)]
    pub(crate) fn pick_lazy_match_rl<K: RowTags, const ROW_LOG: usize>(
        &self,
        abs_pos: usize,
        lit_len: usize,
        best: Option<MatchCandidate>,
    ) -> Option<MatchCandidate> {
        let best = best?;
        // Same C-faithful length decision as the production monolith — route
        // through the shared `lazy_decide!` macro (test-only path; its finder is
        // out-of-line here, so the macro adds no register cost). `None` = commit.
        // `target_len = usize::MAX` matches the production row lazy body: upstream
        // lazy has no sufficient-length early-out (that belongs to the OPT parser).
        match crate::encoding::lazy_parse::lazy_decide!(
            best_len = best.match_len,
            best_off = best.offset,
            target_len = usize::MAX,
            lazy_depth = self.lazy_depth,
            abs_pos = abs_pos,
            lit_len = lit_len,
            history_end = self.history_abs_end(),
            min_match = self.mls,
            search = |p, l| self.best_match_rl::<K, ROW_LOG>(p, l),
        ) {
            ::core::option::Option::None => Some(best),
            ::core::option::Option::Some(_) => None,
        }
    }

    #[allow(dead_code)]
    #[inline(always)]
    pub(crate) fn repcode_candidate(
        &self,
        abs_pos: usize,
        lit_len: usize,
    ) -> Option<MatchCandidate> {
        repcode_candidate_shared(
            self.hash_kernel,
            self.live_history(),
            self.history_abs_start,
            self.offset_hist,
            abs_pos,
            lit_len,
            self.mls,
            self.max_window_size,
        )
    }

    // Two-level bounded dispatch: resolve the tag kernel (`K: RowTags`)
    // then the `row_log` const, both cold (once per block / call), into the
    // fully monomorphised `_rl::<K, ROW_LOG>` hot loop. The per-position
    // loop carries no runtime kernel enum and no `row_log` reload. Callers
    // (driver + tests) use these bare names; the hot loops call the `_rl`
    // siblings directly with the type and const already bound. `skip` does
    // no tag compare, so it dispatches on `row_log` only.
    pub(crate) fn start_matching(&mut self, handle_sequence: impl for<'a> FnMut(Sequence<'a>)) {
        // SAFETY: same per-tier umbrella contract as `start_matching_greedy`.
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        {
            // SAFETY: simd128 is a compile-time feature here; no runtime gate.
            unsafe { dispatch_row_log!(self.lazy_simd128::<Simd128Tags>(handle_sequence)) }
        }
        #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
        {
            match self.tag_kernel {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                FastpathKernel::Avx2Bmi2 => unsafe {
                    dispatch_row_log!(self.lazy_avx2bmi2::<Avx2Bmi2Tags>(handle_sequence))
                },
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                FastpathKernel::Sse42 => unsafe {
                    dispatch_row_log!(self.lazy_sse42::<Sse42Tags>(handle_sequence))
                },
                #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
                FastpathKernel::Neon => unsafe {
                    dispatch_row_log!(self.lazy_neon::<NeonTags>(handle_sequence))
                },
                // SAFETY: the scalar kernel has no `#[target_feature]`; the
                // fn is `unsafe` only for macro uniformity.
                FastpathKernel::Scalar => unsafe {
                    dispatch_row_log!(self.lazy_scalar::<ScalarTags>(handle_sequence))
                },
            }
        }
    }

    gen_lazy_monolith!(
        lazy_scalar,
        false,
        row_tag_mask_scalar,
        crate::encoding::fastpath::scalar::common_prefix_len_ptr
    );
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    gen_lazy_monolith!(
        lazy_sse42,
        true,
        row_tag_mask_sse2,
        crate::encoding::fastpath::sse42::common_prefix_len_ptr,
        "sse4.2"
    );
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    gen_lazy_monolith!(
        lazy_avx2bmi2,
        true,
        row_tag_mask_avx2,
        crate::encoding::fastpath::avx2_bmi2::common_prefix_len_ptr,
        "avx2,bmi2"
    );
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    gen_lazy_monolith!(
        lazy_neon,
        true,
        row_tag_mask_neon,
        crate::encoding::fastpath::neon::common_prefix_len_ptr,
        "neon"
    );
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    gen_lazy_monolith!(
        lazy_simd128,
        true,
        row_tag_mask_simd128,
        crate::encoding::fastpath::scalar::common_prefix_len_ptr
    );

    pub(crate) fn start_matching_greedy(
        &mut self,
        handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        // SAFETY: each `greedy_*` umbrella is entered only when its kernel
        // was runtime-detected (`tag_kernel`), upholding the
        // `#[target_feature]` contract; the wasm tier is compile-time.
        #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
        {
            // SAFETY: simd128 is a compile-time feature here; no runtime gate.
            unsafe { dispatch_row_log!(self.greedy_simd128::<Simd128Tags>(handle_sequence)) }
        }
        #[cfg(not(all(target_arch = "wasm32", target_feature = "simd128")))]
        {
            match self.tag_kernel {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                FastpathKernel::Avx2Bmi2 => unsafe {
                    dispatch_row_log!(self.greedy_avx2bmi2::<Avx2Bmi2Tags>(handle_sequence))
                },
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                FastpathKernel::Sse42 => unsafe {
                    dispatch_row_log!(self.greedy_sse42::<Sse42Tags>(handle_sequence))
                },
                #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
                FastpathKernel::Neon => unsafe {
                    dispatch_row_log!(self.greedy_neon::<NeonTags>(handle_sequence))
                },
                // SAFETY: the scalar kernel has no `#[target_feature]`; the
                // fn is `unsafe` only for macro uniformity.
                FastpathKernel::Scalar => unsafe {
                    dispatch_row_log!(self.greedy_scalar::<ScalarTags>(handle_sequence))
                },
            }
        }
    }

    gen_greedy_monolith!(
        greedy_scalar,
        false,
        row_tag_mask_scalar,
        crate::encoding::fastpath::scalar::common_prefix_len_ptr
    );
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    gen_greedy_monolith!(
        greedy_sse42,
        true,
        row_tag_mask_sse2,
        crate::encoding::fastpath::sse42::common_prefix_len_ptr,
        "sse4.2"
    );
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    gen_greedy_monolith!(
        greedy_avx2bmi2,
        true,
        row_tag_mask_avx2,
        crate::encoding::fastpath::avx2_bmi2::common_prefix_len_ptr,
        "avx2,bmi2"
    );
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    gen_greedy_monolith!(
        greedy_neon,
        true,
        row_tag_mask_neon,
        crate::encoding::fastpath::neon::common_prefix_len_ptr,
        "neon"
    );
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    gen_greedy_monolith!(
        greedy_simd128,
        true,
        row_tag_mask_simd128,
        crate::encoding::fastpath::scalar::common_prefix_len_ptr
    );

    pub(crate) fn skip_matching_with_hint(&mut self, incompressible_hint: Option<bool>) {
        match self.row_log {
            4 => self.skip_matching_with_hint_rl::<4>(incompressible_hint),
            5 => self.skip_matching_with_hint_rl::<5>(incompressible_hint),
            6 => self.skip_matching_with_hint_rl::<6>(incompressible_hint),
            _ => unreachable!("row_log is clamped to 4..=6 in configure()"),
        }
    }

    /// Borrowed (no-copy) one-shot equivalent of [`Self::start_matching`]:
    /// stage `[block_start, block_end)` of the registered borrowed window,
    /// then run the SAME parse dispatch. The parse body reads its block range
    /// via `current_block_range()` and its bytes via `live_history()`, both
    /// borrowed-aware, so the staged block is scanned in place (no
    /// `add_data` copy into the owned mirror). `history_abs_start` was forced
    /// to 0 in `set_borrowed_window`, so positions stay absolute input
    /// offsets and the window-low candidate cap bounds offsets to the window.
    pub(crate) fn start_matching_borrowed(
        &mut self,
        block_start: usize,
        block_end: usize,
        greedy: bool,
        handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    ) {
        self.stage_borrowed_block(block_start, block_end);
        if greedy {
            self.start_matching_greedy(handle_sequence);
        } else {
            self.start_matching(handle_sequence);
        }
    }

    /// Borrowed equivalent of [`Self::skip_matching_with_hint`]: stage the
    /// block (so the RLE/Raw emit's `get_last_space` reserve reports it) and
    /// seed the row tables without a copy, mirroring the owned skip.
    pub(crate) fn skip_matching_borrowed(
        &mut self,
        block_start: usize,
        block_end: usize,
        incompressible_hint: Option<bool>,
    ) {
        self.stage_borrowed_block(block_start, block_end);
        self.skip_matching_with_hint(incompressible_hint);
    }

    #[allow(dead_code)]
    pub(crate) fn best_match(&self, abs_pos: usize, lit_len: usize) -> Option<MatchCandidate> {
        dispatch_tag_kernel!(self.best_match_k(abs_pos, lit_len))
    }
    fn best_match_k<K: RowTags>(&self, abs_pos: usize, lit_len: usize) -> Option<MatchCandidate> {
        dispatch_row_log!(self.best_match_rl::<K>(abs_pos, lit_len))
    }

    #[allow(dead_code)]
    pub(crate) fn pick_lazy_match(
        &self,
        abs_pos: usize,
        lit_len: usize,
        best: Option<MatchCandidate>,
    ) -> Option<MatchCandidate> {
        dispatch_tag_kernel!(self.pick_lazy_match_k(abs_pos, lit_len, best))
    }
    fn pick_lazy_match_k<K: RowTags>(
        &self,
        abs_pos: usize,
        lit_len: usize,
        best: Option<MatchCandidate>,
    ) -> Option<MatchCandidate> {
        dispatch_row_log!(self.pick_lazy_match_rl::<K>(abs_pos, lit_len, best))
    }

    // Per-kernel row match probe. Runtime kernel selection happens ONCE via
    // `dispatch_tag_kernel!`; the selected tier's `row_probe_*` method is the
    // monomorphised per-position hot loop with the SIMD tag-match inlined under
    // its `#[target_feature]` umbrella (no dispatcher branch inside the loop).
    #[allow(dead_code)]
    pub(crate) fn row_candidate(&self, abs_pos: usize, lit_len: usize) -> Option<MatchCandidate> {
        dispatch_tag_kernel!(self.row_candidate_k(abs_pos, lit_len))
    }
    fn row_candidate_k<K: RowTags>(
        &self,
        abs_pos: usize,
        lit_len: usize,
    ) -> Option<MatchCandidate> {
        // SAFETY: `dispatch_tag_kernel!` only selects a `K` whose ISA `detect`
        // confirmed present, upholding `K::probe`'s per-tier feature contract.
        match self.row_log {
            4 => unsafe { K::probe::<4>(self, abs_pos, lit_len, None) },
            5 => unsafe { K::probe::<5>(self, abs_pos, lit_len, None) },
            6 => unsafe { K::probe::<6>(self, abs_pos, lit_len, None) },
            _ => unreachable!("row_log is clamped to 4..=6 in configure()"),
        }
    }

    // Each tier pairs its tag-match mask macro with the matching
    // `fastpath::<tier>::common_prefix_len_ptr` so BOTH inline under the tier's
    // `#[target_feature]` umbrella (the cpl features must be a subset of the
    // probe's: SSE4.2 ⊇ the SSE2 mask intrinsics, AVX2+BMI2 ⊇ the AVX2 mask).
    // Scalar uses the on-the-fly per-slot byte compare (`use_mask = false`).
    gen_row_probe!(
        row_probe_scalar,
        false,
        row_tag_mask_scalar,
        crate::encoding::fastpath::scalar::common_prefix_len_ptr
    );
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    gen_row_probe!(
        row_probe_sse42,
        true,
        row_tag_mask_sse2,
        crate::encoding::fastpath::sse42::common_prefix_len_ptr,
        "sse4.2"
    );
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    gen_row_probe!(
        row_probe_avx2bmi2,
        true,
        row_tag_mask_avx2,
        crate::encoding::fastpath::avx2_bmi2::common_prefix_len_ptr,
        "avx2,bmi2"
    );
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    gen_row_probe!(
        row_probe_neon,
        true,
        row_tag_mask_neon,
        crate::encoding::fastpath::neon::common_prefix_len_ptr,
        "neon"
    );
    // wasm simd128 tag-match mask + scalar (portable) cpl. wasm simd128 is
    // compile-time, so no `#[target_feature]` umbrella is passed; mirrors the
    // wasm tier's behavior of vectorising only the tag scan, with the portable
    // prefix-length kernel.
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    gen_row_probe!(
        row_probe_simd128,
        true,
        row_tag_mask_simd128,
        crate::encoding::fastpath::scalar::common_prefix_len_ptr
    );

    fn extend_backwards(
        &self,
        candidate_pos: usize,
        abs_pos: usize,
        match_len: usize,
        lit_len: usize,
    ) -> MatchCandidate {
        extend_backwards_shared(
            self.live_history(),
            self.history_abs_start,
            candidate_pos,
            abs_pos,
            match_len,
            lit_len,
        )
    }

    fn insert_positions<const ROW_LOG: usize>(&mut self, start: usize, end: usize) {
        for pos in start..end {
            self.insert_position::<ROW_LOG>(pos);
        }
    }

    /// Index a just-emitted match span, mirroring the upstream zstd
    /// `ZSTD_row_update_internal` skip-threshold (`zstd_lazy.c:922-940`):
    /// when the span exceeds `SKIP_THRESHOLD` positions, only the first
    /// `MAX_START` and last `MAX_END` are indexed and the interior is
    /// skipped. Indexing every interior byte of a long match is
    /// O(matchlen) and dominates encode time on periodic inputs (e.g.
    /// repeated log lines), where a single greedy/lazy match can span an
    /// entire block: that O(matchlen) fill, not the search, is what left
    /// the row backend ~11x slower than FFI on those streams. The upstream zstd
    /// caps the fill at 96 + 32 positions regardless of match length.
    fn insert_match_span<const ROW_LOG: usize>(&mut self, start: usize, end: usize) {
        super::history::insert_match_span(start, end, |start, end| {
            self.insert_positions::<ROW_LOG>(start, end);
        });
    }

    fn insert_positions_with_step<const ROW_LOG: usize>(
        &mut self,
        start: usize,
        end: usize,
        step: usize,
    ) {
        if step <= 1 {
            self.insert_positions::<ROW_LOG>(start, end);
            return;
        }
        let mut pos = start;
        while pos < end {
            self.insert_position::<ROW_LOG>(pos);
            let next = pos.saturating_add(step);
            if next <= pos {
                break;
            }
            pos = next;
        }
    }

    #[inline(always)]
    fn insert_position<const ROW_LOG: usize>(&mut self, abs_pos: usize) {
        let Some((row, tag)) = self.hash_and_row(abs_pos) else {
            return;
        };
        self.insert_at::<ROW_LOG>(abs_pos, row, tag);
    }

    /// Prefetch a row's tag bytes and position words into L1 ahead of the
    /// next iteration's probe (no-op on targets without a prefetch hint).
    #[inline]
    fn prefetch_row<const ROW_LOG: usize>(&self, row: usize) {
        let row_base = row << ROW_LOG;
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            #[cfg(target_arch = "x86")]
            use core::arch::x86::{_MM_HINT_T0, _mm_prefetch};
            #[cfg(target_arch = "x86_64")]
            use core::arch::x86_64::{_MM_HINT_T0, _mm_prefetch};
            // SAFETY: prefetch is a hint and never faults; the indexes are in
            // bounds by the same `ensure_tables` sizing as `insert_at`.
            unsafe {
                _mm_prefetch(self.row_tags.as_ptr().add(row_base).cast(), _MM_HINT_T0);
                _mm_prefetch(
                    self.row_positions.as_ptr().add(row_base).cast(),
                    _MM_HINT_T0,
                );
            }
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        {
            let _ = row_base;
        }
    }

    /// [`Self::insert_position`] with the (row, tag) pair already computed —
    /// the greedy miss path reuses the probe's hash instead of re-hashing
    /// the same position.
    #[inline(always)]
    fn insert_at<const ROW_LOG: usize>(&mut self, abs_pos: usize, row: usize, tag: u8) {
        // `ROW_LOG` is the compile-time row width for this monomorphisation;
        // the dispatcher guarantees `ROW_LOG == self.row_log` so the table
        // bounds (`ensure_tables` sized by `self.row_log`) hold.
        debug_assert_eq!(ROW_LOG, self.row_log);
        let row_entries = 1usize << ROW_LOG;
        let row_mask = row_entries - 1;
        let row_base = row << ROW_LOG;
        // SAFETY: `hash_and_row` masks `row` to `row_hash_log` bits and
        // `row_heads.len() == 1 << row_hash_log` by `ensure_tables`.
        // `row_base = row << row_log = row * row_entries` and
        // `next < row_entries`, so `row_base + next < row_count *
        // row_entries == row_positions.len() == row_tags.len()`. Both
        // index pairs are provably in bounds; per-byte hot path on
        // fast/dfast/row levels saves ~6 instructions and 3 branches.
        debug_assert!(row < self.row_heads.len());
        debug_assert!(row_base + row_entries <= self.row_positions.len());
        unsafe {
            let head = *self.row_heads.get_unchecked(row) as usize;
            let next = head.wrapping_sub(1) & row_mask;
            *self.row_heads.get_unchecked_mut(row) = next as u8;
            *self.row_tags.get_unchecked_mut(row_base + next) = tag;
            // `abs_pos < u32::MAX` holds: `add_data` caps a Row frame's
            // absolute cursor below `u32::MAX`, so the cast is lossless and
            // never collides with the `ROW_EMPTY_SLOT == u32::MAX` sentinel.
            *self.row_positions.get_unchecked_mut(row_base + next) = abs_pos as u32;
        }
    }

    /// Mark the attached dict row index fully built (CDict cache).
    pub(crate) fn mark_dict_primed(&mut self) {
        self.dict.mark_primed();
    }

    /// Drop the cached dict row index (next frame carries no dict, or eviction /
    /// resize staled the concat positions).
    pub(crate) fn invalidate_dict_cache(&mut self) {
        self.dict.invalidate();
    }

    /// Upstream zstd `ZSTD_RowFindBestMatch` `dictMatchState` attach: index the
    /// just-committed dictionary block (current `chunk_lens` tail) into the
    /// SEPARATE immutable dict row tables instead of the live ones, so the live
    /// rows carry only input and the dict is never re-indexed per frame. The
    /// dual-probe in `row_candidate_rl` searches live + dict (one bounded row
    /// each).
    pub(crate) fn prime_dict_attach_current_block(&mut self) {
        self.ensure_tables();
        let current_len = self.chunk_lens.back().copied().unwrap_or(0);
        if current_len == 0 {
            return;
        }
        let current_abs_start = self.history_abs_start + self.window_size - current_len;
        let current_abs_end = current_abs_start + current_len;
        let start_concat = current_abs_start - self.history_abs_start;
        let end_concat = current_abs_end - self.history_abs_start;
        // Backfill the `ROW_HASH_KEY_LEN - 1` bytes immediately before the
        // block, mirroring the live insert path's `backfill_start`: those
        // starts only become hashable once this block supplies the
        // trailing key bytes, so without the backfill seam-spanning row
        // candidates are dropped from the dict index permanently.
        let prime_start = start_concat.saturating_sub(ROW_HASH_KEY_LEN - 1);
        self.prime_dict_rows(prime_start, end_concat);
    }

    /// Build the immutable dictionary row index over the contiguous-history
    /// concat range `[start_concat, end_concat)`. Mirrors [`Self::insert_position`]'s
    /// row-hash + head-decrement slot write, but writes a CONCAT index (stable
    /// across rebases) into the SEPARATE [`Self::dict`] tables. `ROW_EMPTY_SLOT`
    /// marks empty. Skips the rehash when the CDict cache is already primed.
    fn prime_dict_rows(&mut self, start_concat: usize, end_concat: usize) {
        let row_count = 1usize << self.row_hash_log;
        let row_entries = 1usize << self.row_log;
        let total = row_count * row_entries;
        // Drop a cached dict index built for a different table shape (a genuine
        // level change resizes `row_hash_log`/`row_log`). `set_hash_bits`'s
        // per-frame oscillation does NOT reach here — this runs after setup with
        // the final shape — so a same-level reused frame keeps the cache.
        // Key on the FULL shape, not just `heads.len()`: a level change that
        // keeps `row_hash_log` but changes `row_log` leaves `heads.len()`
        // equal while `positions`/`tags` are sized for the old `row_log`,
        // and the probe path then indexes `row << row_log` slots that the
        // cached table doesn't have (OOB / wrong slots).
        if self.dict.table().is_some_and(|d| {
            d.heads.len() != row_count || d.positions.len() != total || d.tags.len() != total
        }) {
            self.dict.invalidate();
        }
        self.dict.set_region_len(end_concat);
        if self.dict.is_primed() {
            return;
        }
        let row_log = self.row_log;
        let row_hash_log = self.row_hash_log;
        let hash_kernel = self.hash_kernel;
        let key_len = self.mls.min(6);
        let history_start = self.history_start;
        let concat_len = self.history.len() - history_start;
        // Row hash needs `ROW_HASH_KEY_LEN` readable bytes of lookahead.
        let safe_end = concat_len
            .saturating_sub(ROW_HASH_KEY_LEN - 1)
            .min(end_concat);
        if start_concat >= safe_end {
            return;
        }
        // `row_count` / `row_entries` / `total` were computed above for the
        // shape-mismatch check and carry the same values here.
        let row_mask = row_entries - 1;
        let row_count_mask = row_count - 1;
        // Raw history base taken before the mutable dict borrow (disjoint
        // fields; the raw ptr holds no borrow).
        let base = self.history.as_ptr();
        let dict = self.dict.table_mut_or_init(|| RowDictTables {
            heads: alloc::vec![0u8; row_count],
            positions: alloc::vec![ROW_EMPTY_SLOT; total],
            tags: alloc::vec![0u8; total],
        });
        let heads = dict.heads.as_mut_ptr();
        let positions = dict.positions.as_mut_ptr();
        let tags = dict.tags.as_mut_ptr();
        let total_bits = row_hash_log + ROW_TAG_BITS;
        // SAFETY: `base.add(history_start + concat)` is in-bounds for
        // `concat + ROW_HASH_KEY_LEN <= concat_len` (enforced by `safe_end`).
        // `row <= row_count_mask < row_count` (= heads.len()); `row_base + next
        // < total` (= positions.len() = tags.len()). `concat` fits u32 (history
        // is u32-bounded upstream).
        for concat in start_concat..safe_end {
            unsafe {
                // Same key reader as the live `hash_and_row` (width and
                // endianness): any divergence buckets dict rows differently
                // and loses every attached-dict match.
                let value = Self::row_key_value(
                    core::slice::from_raw_parts(base.add(history_start), concat_len),
                    concat,
                    key_len,
                );
                let hash = crate::encoding::fastpath::hash_mix_u64_with_kernel(hash_kernel, value);
                let combined = hash >> (u64::BITS as usize - total_bits);
                let row = ((combined >> ROW_TAG_BITS) as usize) & row_count_mask;
                let tag = combined as u8;
                let row_base = row << row_log;
                let head = *heads.add(row) as usize;
                let next = head.wrapping_sub(1) & row_mask;
                *heads.add(row) = next as u8;
                *tags.add(row_base + next) = tag;
                *positions.add(row_base + next) = concat as u32;
            }
        }
    }
}

// Gated on `all()` because the runtime feature probe
// (`std::arch::is_x86_feature_detected!`) used to skip kernels the host CPU
// lacks is std-only, matching how `RowTagKernel::detect` gates the same probe.
