#![allow(
    clippy::checked_conversions,
    clippy::inline_always,
    clippy::needless_continue,
    clippy::semicolon_if_nothing_returned,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Upstream zstd-shape Fast strategy block compressor — port of
//! `ZSTD_compressBlock_fast_noDict_generic` from
//! `lib/compress/zstd_fast.c`. Includes the 4-cursor
//! (`ip0/ip1/ip2/ip3`) lookahead pipeline with `kSearchStrength`
//! step-doubling, repcode-at-ip2 probe, two explicit-match probes
//! per do-while iter, immediate-rep2 inner loop after match emit,
//! and both upstream zstd variants of the match-found check
//! (`ZSTD_match4Found_branch` + `ZSTD_match4Found_cmov`) selected
//! per-call via the `USE_CMOV` const generic.

use super::count::{count_forward, count_forward_dict_2segment};
use super::hash_table::{FastHashTable, hash_ptr_raw};
use crate::encoding::Sequence;

macro_rules! ktrace {
    ($($arg:tt)*) => {};
}

/// Upstream zstd `kSearchStrength` — defined in `zstd_compress_internal.h:32`
/// as `#define kSearchStrength 8`. The step-skip accelerator advances
/// the per-iteration step every `1 << (kSearchStrength - 1) = 128`
/// bytes when no matches are found, so incompressible regions skip
/// ahead faster than the linear 1-byte advance.
///
/// Previously used `SEARCH_STRENGTH = 6` (`K_STEP_INCR
/// = 32`), causing our step doubling to fire 4× more frequently than
/// upstream zstd — by ip0=1280 our step was ~40 while upstream zstd's was 12. This
/// drove the +7.43% ratio gap on decodecorpus-z000033 at Level(1)
/// Fast: cursor skipped too many positions, missing matches upstream zstd
/// found via finer-grained probing.
const SEARCH_STRENGTH: usize = 8;

/// Upstream zstd `kStepIncr = 1 << (kSearchStrength - 1) = 128` — every this-
/// many bytes of no-match scanning, the per-iteration `step` is
/// bumped by 1 (upstream zstd's `step++` at `zstd_fast.c:343`). Drives the
/// incompressible-region step acceleration.
const K_STEP_INCR: usize = 1 << (SEARCH_STRENGTH - 1);

/// Upstream zstd `HASH_READ_SIZE`. The forward-progress invariant is that the
/// hash read at `ip0` MUST stay inside `[base, iend)`, so the
/// `ilimit = iend - HASH_READ_SIZE` cap is applied to the loop
/// boundary check.
const HASH_READ_SIZE: usize = 8;

/// Minimum length a DICTIONARY match must reach to be committed by the
/// dict-aware (attach-mode) Fast kernel [`compress_block_fast_dict`]. A
/// dictionary match reaches back across the whole input into the dictionary
/// region, so its offset is inherently large — a SHORT one inflates the
/// offset-code FSE alphabet (a single far offset forces the table to span up to
/// its high code) for little literal saving, and can make a dict frame larger
/// than the no-dict frame. Below this floor the kernel skips the dict match and
/// lets the parse find a near (small-offset / repcode) match instead. Only
/// dictionary matches carry this floor; main-table and repcode matches keep the
/// 4-byte minimum. (Upstream zstd commits any 4-byte dict match, but it is not
/// chasing byte-identity — we make our own match decisions where they improve
/// the ratio, per the drop-in-not-binary-parity contract.)
const MIN_DICT_MATCH_LEN: usize = 8;

/// Short-cache tag width for the dictionary hash table (upstream zstd
/// `ZSTD_SHORT_CACHE_TAG_BITS`). Each dict slot packs `(position << TAG_BITS) |
/// tag`, where `tag` is the low `TAG_BITS` of a `hashLog + TAG_BITS`-wide hash.
/// On lookup the stored tag must equal the query tag, so a colliding dict
/// position with a different tag is rejected WITHOUT the candidate load + 4-byte
/// `MEM_read32` the untagged variant pays on every collision — a speed win on
/// the dict-probe hot path. The slot index `hash >> TAG_BITS` is identical to
/// the plain `hashLog` hash, so tagging does not change which slot a position
/// lands in (only adds the collision-reject), keeping the every-position
/// last-wins fill's nearest-occurrence guarantee intact.
pub(crate) const DICT_TAG_BITS: u32 = 8;
pub(crate) const DICT_TAG_MASK: u32 = (1 << DICT_TAG_BITS) - 1;

/// Tagged dictionary-table lookup (upstream zstd
/// `ZSTD_compressBlock_fast_dictMatchState_generic`, zstd_fast.c:546-548).
/// Hashes `ptr` to `dict_hash_log + DICT_TAG_BITS` bits, reads slot
/// `hash >> DICT_TAG_BITS` (== the plain `dict_hash_log` hash), and returns the
/// stored dict position ONLY if the packed tag matches; otherwise `0` (the
/// never-filled sentinel — dict positions start at `1`, and the caller's
/// `dict_idx >= 1` gate rejects it). The tag check rejects collisions without a
/// candidate load.
///
/// # Safety
/// `ptr` must have the readable-bytes context `hash_ptr_raw::<MLS>` requires;
/// `dict_hash_log + DICT_TAG_BITS <= 32` so the slot index stays in range.
#[inline(always)]
pub(crate) unsafe fn dict_lookup<const MLS: u32>(
    dict_table: &FastHashTable,
    ptr: *const u8,
    dict_hash_log: u32,
) -> u32 {
    let hat = unsafe { hash_ptr_raw::<MLS>(ptr, dict_hash_log + DICT_TAG_BITS) };
    // SAFETY: `hat >> DICT_TAG_BITS` < `1 << dict_hash_log` = table len. The dict
    // table is never epoch-advanced (bias 0), so `get` returns the raw stored
    // `(pos << TAG) | tag` word unchanged.
    let stored = unsafe { dict_table.get(hat >> DICT_TAG_BITS) };
    if stored & DICT_TAG_MASK == hat & DICT_TAG_MASK {
        stored >> DICT_TAG_BITS
    } else {
        0
    }
}

/// Upstream zstd's `MEM_read32(ptr)` — unaligned native-endian 4-byte load,
/// used by the raw match probe on the hot path. The result is only
/// ever compared for equality against another `read32` of the same
/// width on the same host, so the byte ordering does not matter — both
/// sides experience the same endianness, and `a == b` holds iff the
/// underlying byte sequences match. No `.to_le()` conversion is needed
/// (upstream zstd's C `MEM_read32` is also implemented as a native-endian
/// `memcpy` for the same reason).
///
/// # Safety
///
/// `ptr` MUST point to at least 4 readable bytes.
#[inline(always)]
unsafe fn read32(ptr: *const u8) -> u32 {
    // SAFETY: caller contract.
    unsafe { core::ptr::read_unaligned(ptr.cast::<u32>()) }
}

/// Upstream zstd `ZSTD_match4Found_cmov`'s dummy buffer — 4 random-ish bytes
/// to compare against when `match_idx < prefix_start_index`. Chosen
/// so the read32 result is very unlikely to coincide with any real
/// 4-byte window from the input. Used by the cmov branchless variant
/// to avoid the unpredictable `match_idx >= prefix_start_index`
/// branch.
const CMOV_DUMMY: [u8; 4] = [0x12, 0x34, 0x56, 0x78];

/// Upstream zstd `ZSTD_match4Found_branch` / `ZSTD_match4Found_cmov`
/// branchless dispatch via const generic.
///
/// # Safety
///
/// - `ip` MUST point to ≥ 4 readable bytes (the kernel only calls
///   this when ip0..ip3 stay within `iend - HASH_READ_SIZE`).
/// - `base` MUST be the start of the same buffer the kernel scans.
///   For an in-window `match_idx >= prefix_start_index`,
///   `base.add(match_idx as usize)` MUST yield ≥ 4 readable bytes
///   (i.e. `match_idx + 4 <= data_len`). The kernel maintains this
///   invariant by only inserting hash-table entries for positions
///   strictly below `ilimit = data_len - HASH_READ_SIZE`, so every
///   in-range `match_idx` returned by the table is automatically
///   ≥ 4 bytes from the buffer end. See the comment block inside
///   the function body for the full derivation.
#[inline(always)]
unsafe fn match_found<const USE_CMOV: bool>(
    ip: *const u8,
    base: *const u8,
    match_idx: u32,
    prefix_start_index: u32,
) -> bool {
    // Upstream zstd-parity hot-path: the ONLY filter on the branch variant is
    // `match_idx < prefix_start_index` (rejects stale entries below
    // the current window). Two safety invariants make additional
    // bounds checks redundant:
    //
    // 1. `match_pos + 4 <= data_len`: hash table entries are only
    //    written for positions visited by the scan, which by
    //    construction stay strictly below `ilimit = data_len -
    //    HASH_READ_SIZE = data_len - 8`. So any in-window
    //    `match_idx >= prefix_start_index` satisfies `match_pos + 4
    //    < data_len`. The `prefix_start_index >= INITIAL_PREFIX_START_INDEX
    //    = 1` rule at the matcher boundary rejects the stale-zero
    //    initial entry that would otherwise alias to position 0.
    //
    // 2. `match_pos < ip_pos`: hash writes precede probes in upstream zstd's
    //    flow (writeback `hashTable[hash0] = current0` happens before
    //    `matchFound(...)` reads matchIdx). Since `current0 < ip0` at
    //    every shift step, `matchIdx <= current0_prev < ip0_now`.
    //
    // Upstream zstd `ZSTD_match4Found_branch` (`zstd_fast.c:128-141`) takes
    // the same invariants and emits exactly one prefix filter + one
    // 4-byte equality compare. The previous defensive bounds checks
    // we carried here added two extra branches per match probe —
    // and the kernel invokes `match_found` TWICE per inner-loop
    // iteration, so the savings compound to ~4 branches/iter
    // dropped on the hot path.
    let match_pos = match_idx as usize;

    if USE_CMOV {
        // Upstream zstd cmov variant (`ZSTD_match4Found_cmov`): pick either
        // `base + match_pos` or `CMOV_DUMMY` based on the prefix
        // filter, then AND with an explicit `in_range` predicate.
        // The compiler typically lowers the if-expression to a
        // `cmov` on x86_64 (the "cmov" name reflects that target —
        // we don't enforce the lowering, since LLVM is free to use
        // a branch where it predicts well). The dummy compare alone
        // is NOT enough — if `read32(ip)` happens to equal
        // `CMOV_DUMMY` (rare but reachable), the out-of-window
        // match would otherwise slip through.
        //
        // Upstream zstd (`ZSTD_match4Found_cmov` lines 119-124) inserts an
        // `__asm__("")` compiler barrier between the two checks to
        // pin codegen order. We don't replicate that — Rust offers
        // `core::sync::atomic::compiler_fence` if needed, but
        // empirically LLVM's lowering here already orders the
        // bytes_match comparison before the in_range AND without a
        // barrier. Revisit only if profiling shows reordering hurt.
        // SAFETY: both candidate addresses have ≥ 4 readable bytes
        // (CMOV_DUMMY is exactly 4 bytes; base+match_pos has ≥ 4
        // by the bounds check above).
        let in_range = match_idx >= prefix_start_index;
        let mval_addr = if in_range {
            unsafe { base.add(match_pos) }
        } else {
            CMOV_DUMMY.as_ptr()
        };
        let bytes_match = unsafe { read32(ip) == read32(mval_addr) };
        // Bitwise AND (not `&&`) is INTENTIONAL — short-circuit
        // would re-introduce a branch on `bytes_match`, defeating
        // the cmov-branchless path. Upstream zstd enforces the same
        // ordering with `__asm__("")` between its two checks
        // (`ZSTD_match4Found_cmov` lines 119-124).
        #[allow(clippy::needless_bitwise_bool)]
        let r = bytes_match & in_range;
        r
    } else {
        // Upstream zstd branch variant (`ZSTD_match4Found_branch`): explicit
        // branch on the prefix filter. Faster when the branch is
        // strongly predictable — that's the typical Fast strategy
        // case where almost all hash table entries are within the
        // current window.
        if match_idx < prefix_start_index {
            return false;
        }
        unsafe { read32(ip) == read32(base.add(match_pos)) }
    }
}

/// Output of [`compress_block_fast`] — the new repcode pair to thread
/// through the next block's invocation, plus the number of literal
/// bytes left at the tail (the caller emits these as a trailing
/// `Sequence::Literals` so the encoder pipeline can flush the block).
pub(crate) struct FastBlockResult {
    pub(crate) rep: [u32; 2],
    pub(crate) tail_literals_len: usize,
}

/// Upstream zstd-parity Fast block compressor, monomorphised over `MLS` (4..=8).
/// Each call processes one full block; produced sequences are emitted
/// via `handle_sequence` in order. The caller is responsible for
/// flushing the trailing literals (returned in `tail_literals_len`)
/// after this function returns.
///
/// # Arguments
///
/// - `data`: the full prefix history followed by the current block,
///   laid out as a single flat buffer (matches upstream zstd's `base`).
/// - `block_start`: byte offset of the current block's first byte
///   within `data`. The kernel hashes/searches only positions in
///   `[block_start, data.len())`, but matches may reach back into the
///   prefix all the way to `bounds.prefix_start_index`.
/// - `bounds: PrefixBounds`: bundle of two upstream zstd-derived absolute
///   floors (kept together so the kernel signature stays inside the
///   clippy 7-argument cap). See [`PrefixBounds`] field docs for the
///   exact semantics:
///   - `prefix_start_index`: sentinel-aware match-table filter (rejects
///     the all-zero empty-slot value at position 0).
///   - `window_low`: upstream zstd `windowLow`-equivalent absolute floor used
///     by the prologue's `max_rep` computation and the backward-extension
///     `match_pos > window_low` bound.
/// - `hash_table`: the encoder's `FastHashTable`. Mutated in place;
///   entries are absolute indices into `data`.
/// - `rep`: incoming `[rep_offset1, rep_offset2]` from the previous
///   block. Returned updated in `FastBlockResult.rep`.
/// - `step_size`: upstream zstd `stepSize = targetLength + !(targetLength) + 1`
///   (min 2). Drives the initial step in the 4-cursor skip schedule.
/// - `handle_sequence`: closure that the kernel invokes once per
///   emitted `Sequence` — equivalent to upstream zstd's `ZSTD_storeSeq`.
///
/// # Preconditions / algorithm invariants
///
/// `compress_block_fast` is a SAFE function — memory-safety holds for
/// every input that doesn't trigger one of the entry-time
/// `assert!`s (see the **Panics** section below for that list). The
/// contract below is about algorithmic correctness (correct output
/// sequences, upstream zstd-parity match coverage), not Rust memory safety.
/// Passing a smaller `data` is well-defined but the kernel falls
/// into the short-input early-return branch and emits no sequences,
/// which may not be what the caller wanted.
///
/// # Panics
///
/// Entry-time `assert!`s reject misuse loudly in every build (debug
/// AND release) rather than silently miscompressing:
/// - `block_start > data.len()` — invalidates the block range and
///   breaks the arithmetic used by both code paths: in the
///   short-input branch `tail_literals_len = data.len() -
///   block_start` underflows; in the main loop
///   `block_start + HASH_READ_SIZE` can wrap and skip the
///   short-input early-return entirely, then `base.add(ip0)`
///   reads out of bounds. Either side is a clean panic instead
///   of UB / garbage output.
/// - `data.len() > u32::MAX as usize` — the kernel stores
///   absolute positions into a u32 hash table and computes offsets
///   as u32, so larger inputs would silently truncate match indices.
/// - `MLS` outside `4..=8` — the upstream zstd's Fast strategy supports
///   only mls 4..=8; out-of-range MLS would route to a
///   non-existent hash formula.
/// - `MLS` != `hash_table.mls()` — a mismatched table layout would
///   cause the kernel to hash with the wrong formula and probe
///   entries indexed by a different formula, leading to garbage
///   match candidates.
///
/// The remaining-block length `data.len() - block_start` SHOULD be
/// at least `HASH_READ_SIZE` (8) bytes — `data` itself may be much
/// longer because it holds the prefix history before `block_start`,
/// so the slice's total size is not the relevant gate. The kernel's
/// short-input early-return (line below) compares precisely
/// `data.len() < block_start + HASH_READ_SIZE`, matching this
/// remaining-block phrasing.
///
/// The `ilimit = data.len() - HASH_READ_SIZE` cap constrains where
/// the main loop hashes and probes — i.e. it stops emitting new
/// matches once `ip0 >= ilimit`. It does NOT mean the trailing 7
/// bytes are ALWAYS literals: an in-progress forward match found at
/// `ip0 < ilimit` extends through `count_forward` and can reach all
/// the way to `iend`, leaving `tail_literals_len = 0`. The kernel
/// reports the actual number of trailing literal bytes (zero or
/// more) in `FastBlockResult.tail_literals_len`, and the caller
/// emits a terminal `Sequence::Literals` only when that value is
/// non-zero.
///
/// # Sequence emission contract
///
/// The kernel emits ONLY `Sequence::Triple` callbacks — one per
/// emitted match (repcode or explicit). Each `Triple` carries the
/// literal-run that precedes the match in its `literals` field, so
/// the kernel never needs a separate `Sequence::Literals` mid-block
/// call. The trailing bytes from the last anchor to the end of
/// `data` are NOT emitted via the closure; they are accounted for
/// by `FastBlockResult.tail_literals_len`, and emitting them as
/// the terminal `Sequence::Literals` (or absorbing them however
/// the caller wants) is the caller's responsibility. This rule
/// applies UNIFORMLY across every exit branch, including the
/// short-input early-return; without that uniformity a caller
/// wrapping the kernel's output would have to special-case "did
/// the kernel already emit the tail" per branch, which is exactly
/// the inconsistency this contract removes.
/// Upstream zstd `prefixStartIndex` + `windowLow` bundled into a single
/// argument so the kernel signature stays within the 7-arg clippy
/// budget. Both fields are u32 absolute positions in the flat
/// history buffer; see [`compress_block_fast`] doc for which path
/// uses which.
#[derive(Clone, Copy)]
pub(crate) struct PrefixBounds {
    /// Sentinel-aware floor for the hash-table `match_idx` filter
    /// (`match_found::<USE_CMOV>` rejects `match_idx <
    /// prefix_start_index`). Caller is expected to maintain
    /// `prefix_start_index >= 1` so the all-zero empty-slot value
    /// can't be confused with a valid match at position 0.
    pub prefix_start_index: u32,
    /// Upstream zstd `windowLow` analogue — the absolute floor of in-window
    /// positions, equals 0 at block 0 / pre-eviction blocks and
    /// advances as the window slides. Drives the prologue's
    /// `max_rep = ip0 - window_low` computation AND the backward-
    /// extension `match_pos > window_low` bound (both paths upstream zstd
    /// expresses against `prefixStart` directly, NOT against a
    /// sentinel-1 floor).
    pub window_low: u32,
}

#[inline(always)]
pub(crate) fn compress_block_fast<const MLS: u32, const USE_CMOV: bool>(
    data: &[u8],
    block_start: usize,
    bounds: PrefixBounds,
    hash_table: &mut FastHashTable,
    rep: [u32; 2],
    step_size: usize,
    mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
) -> FastBlockResult {
    let prefix_start_index = bounds.prefix_start_index;
    let window_low = bounds.window_low;
    // Upstream zstd's `stepSize = targetLength + !(targetLength) + 1`
    // (min 2). Callers must pass >= 2; values larger than 2 drive
    // the kernel's acceleration gradient on negative levels.
    // Validated in release builds too: `compress_block_fast` is a
    // safe `pub(crate)` boundary, so any direct caller bypassing
    // `FastKernelMatcher::with_params` / `reset` must not silently
    // mis-iterate the loop cadence on a mis-typed step. The
    // once-per-block branch is negligible relative to the per-block
    // hash/probe work that follows.
    assert!(
        step_size >= 2,
        "Fast kernel requires step_size >= 2 (got {step_size}); \
         the upstream zstd formula clamps to a min of 2",
    );
    // Real runtime check (not debug_assert) — MLS is a const-generic
    // so the wrong value would compile, and a mismatched table at the
    // call site would silently hash/probe with the wrong layout in
    // release: `compress_block_fast::<5, false>(..., &mut
    // FastHashTable::new(_, 4), ...)` would route to the mls=5 hash
    // formula but read entries indexed by the mls=4 hash → garbage
    // match candidates, mis-compression instead of a clean failure.
    // The `(4..=8).contains(&MLS)` check is logically redundant given
    // the `_ => debug_assert!(false)` arm in `FastHashTable::hash_ptr`,
    // but stating it here surfaces the contract at the call site of
    // the entry point and produces a clearer panic message than the
    // hash-table-internal one.
    assert!(
        (4..=8).contains(&MLS),
        "Fast kernel only supports MLS in 4..=8 (got {MLS})",
    );
    assert_eq!(
        MLS,
        hash_table.mls(),
        "compress_block_fast<{MLS}> called with hash_table whose mls = {}; \
         the table's hash formula must match the kernel's monomorphised mls",
        hash_table.mls(),
    );
    // Real runtime checks (not debug_assert) — both run in every
    // build because they catch distinct failure modes:
    //
    // `block_start > data.len()` is a memory-safety risk: it would
    // wrap `block_start + HASH_READ_SIZE` in the short-input guard
    // below, skip the early return, and proceed into the main loop
    // with an out-of-bounds ip0 → OOB read via `base.add(ip0)`.
    //
    // `data.len() > u32::MAX` is an algorithmic-correctness risk:
    // the kernel stores absolute positions into a u32 hash table
    // (`ip0 as u32`) and computes offsets as u32 (`offset as u32`).
    // For inputs above 4 GiB the silent truncation would corrupt
    // match indices and repcode offsets — every downstream pointer
    // read still stays in-bounds (we re-bound by `data.len()`
    // before any dereference), so it's not memory-unsafe, but the
    // emitted sequences would reference wrong positions and the
    // decoder would produce wrong output. Surfacing the bound at
    // entry turns this into a loud assertion instead of silent
    // miscompression.
    assert!(
        block_start <= data.len(),
        "block_start ({block_start}) must not exceed data.len() ({})",
        data.len(),
    );
    assert!(
        data.len() <= u32::MAX as usize,
        "FastKernel does not support data.len() ({}) > u32::MAX ({}); \
         the kernel stores absolute positions in a u32 hash table and \
         u32 offset codes, so larger inputs would silently truncate",
        data.len(),
        u32::MAX,
    );

    // Block too short to do any matching — report the whole block
    // as trailing literals without emitting anything. Upstream zstd mirrors
    // the same shape via the `_cleanup` path (`anchor = istart`,
    // returns `iend - anchor`). The caller emits the
    // `Sequence::Literals` wrapper per the contract above; we don't
    // double-emit here.
    if data.len() < block_start + HASH_READ_SIZE {
        return FastBlockResult {
            rep,
            tail_literals_len: data.len() - block_start,
        };
    }

    let base = data.as_ptr();
    let iend_addr = data.len();
    let ilimit = iend_addr - HASH_READ_SIZE;

    let mut anchor: usize = block_start;
    let mut ip0: usize = block_start;
    // Upstream zstd: `ip0 += (ip0 == prefixStart);`. Equivalent in flat-buffer
    // terms is to ensure ip0 isn't at the absolute zero position
    // (where the sentinel could be confused with a valid match).
    if ip0 == 0 {
        ip0 = 1;
    }

    let mut rep_offset1: u32 = rep[0];
    let mut rep_offset2: u32 = rep[1];
    // Upstream zstd stashes the repcodes when they're out of range for the
    // current block and restores them at `_cleanup`. For phase 1 we
    // mirror the same save/restore so cross-block repcode history
    // stays correct.
    let mut offset_saved1: u32 = 0;
    let mut offset_saved2: u32 = 0;
    {
        // Upstream zstd (`zstd_fast.c:240-244`): `maxRep = curr - windowLow`.
        // `windowLow` is the absolute floor of in-window positions
        // (= 0 at block 0). It is NOT `prefixStartIndex` — upstream zstd's
        // `prefixStartIndex == windowLow` in the canonical fast path,
        // but our `prefix_start_index` carries the sentinel-1 floor
        // for hash-filter purposes. Using `prefix_start_index` here
        // would zero `rep_offset1 = 1` at block 0 (ip0=1 →
        // max_rep=0; 1>0), disabling rep-at-ip2 for the entire first
        // block — see the `block_zero_prologue_preserves_default_rep_offset_one`
        // regression test in `fast_matcher.rs`.
        let max_rep = (ip0 as u32).saturating_sub(window_low);
        if rep_offset2 > max_rep {
            offset_saved2 = rep_offset2;
            rep_offset2 = 0;
        }
        if rep_offset1 > max_rep {
            offset_saved1 = rep_offset1;
            rep_offset1 = 0;
        }
    }

    // Step-skip state: upstream zstd's `step = stepSize` (`targetLength + 1`
    // = 2 for Fast strategy with `targetLength == 0`). The 4-cursor
    // loop walks ip0/ip1 = ip0 + 1 adjacent + ip2/ip3 at `step` gap.
    // `next_step` is the absolute position where step doubles next;
    // upstream zstd increments by `kStepIncr = 1 << (kSearchStrength - 1)`.
    let mut step: usize = step_size;
    let mut next_step: usize = ip0.saturating_add(K_STEP_INCR);

    // 4-cursor upstream zstd port. Outer `'restart` loop matches upstream zstd's
    // `_start:` reentry: every emitted match `goto _start`s back
    // here for a fresh setup. The inner `do-while` walks ip0..ip3
    // with hash precomputation, repcode-at-ip2 probe, two explicit-
    // match probes (at ip0 then at the shifted ip0), and a step-
    // doubling cadence.
    ktrace!(
        "ENTER block_start={} ip0_initial={} ilimit={} window_low={} prefix={} rep1={} rep2={} step={} mls={}",
        block_start,
        ip0,
        ilimit,
        window_low,
        prefix_start_index,
        rep_offset1,
        rep_offset2,
        step_size,
        MLS,
    );
    // Hoist the hash table's backing slice + hash_log into locals so the hot
    // loop's `get`/`put`/hash compute don't re-read the `Vec` header / the
    // `hash_log` field through `&mut FastHashTable` on every access (see
    // `FastHashTable::hot_state`). The table is fixed-size for the frame, so
    // the slice ptr stays valid for the whole loop.
    let (table, hlog) = hash_table.hot_state();
    'restart: while ip0 < ilimit {
        // _start: setup. ip0 already positioned; derive ip1/ip2/ip3
        // from current step. If even ip3 is past ilimit, the loop
        // can't make forward progress on this iteration — drain to
        // the cleanup path below. `checked_add` here defends against
        // a wild `step_size` (or a runaway `step` from the doubling
        // cadence) wrapping past `ilimit` and turning the
        // out-of-range guard below into a false-pass; on overflow we
        // take the same break path as a normal ip3-past-ilimit miss.
        let mut ip1 = ip0 + 1;
        let Some(mut ip2) = ip0.checked_add(step) else {
            break;
        };
        let Some(mut ip3) = ip2.checked_add(1) else {
            break;
        };
        if ip3 > ilimit {
            break;
        }

        // Hash precomputation for ip0 + ip1 (upstream zstd lines 261-262).
        // SAFETY: ip0, ip1 < ilimit = iend - 8, so ≥ 8 readable
        // bytes at each `base + ip*`. MLS ≤ 8 matches hash_ptr's
        // contract.
        let mut hash0 = unsafe { hash_ptr_raw::<MLS>(base.add(ip0), hlog) };
        let mut hash1 = unsafe { hash_ptr_raw::<MLS>(base.add(ip1), hlog) };
        let mut match_idx = unsafe { *table.get_unchecked(hash0 as usize) };
        ktrace!(
            "OUTER ip0={} ip1={} ip2={} ip3={} step={} hash0={} hash1={} match_idx={} rep1={} rep2={}",
            ip0,
            ip1,
            ip2,
            ip3,
            step,
            hash0,
            hash1,
            match_idx,
            rep_offset1,
            rep_offset2
        );

        // Inner do-while body. On any match, break out with the
        // `MatchFound` enum carrying the match coordinates; the
        // post-loop block handles backward/forward extension + emit.
        enum MatchFound {
            Rep {
                new_ip: usize,
                match0: usize,
                m_len: usize,
                // Upstream zstd's `current0` — position of the LAST hash
                // writeback before the rep was found. For the
                // rep-at-ip2 path that's the iter-start ip0 (the
                // writeback at the top of the do-while body).
                // Used post-emit to insert hash at `current0 + 2`
                // (upstream zstd zstd_fast.c:407). Captured BEFORE the
                // backward-extension decrement so it doesn't
                // collapse onto new_ip for rep matches.
                current0: usize,
            },
            Explicit {
                new_ip: usize,
                match_idx: u32,
                // Same role as `current0` on the Rep variant: the
                // position of the LAST writeback before the match.
                // Path 1 (probe at iter-start ip0) → current0 == ip0;
                // path 2 (probe at shifted ip0) → current0 == shifted
                // ip0. Identical to new_ip for explicit since
                // explicit emits don't decrement new_ip pre-emit.
                current0: usize,
            },
        }
        let found: Option<MatchFound> = loop {
            // Repcode probe at ip2 (upstream zstd line 268). Unconditional
            // load — upstream zstd `MEM_read32(ip2 - rep_offset1)` always
            // reads, no `rep_offset1 > 0` short-circuit. Safe even
            // when rep_offset1 == 0 because `ip2 - 0 = ip2`, which
            // reads the same 4 bytes as the equality target below
            // (so the comparison degrades to `read32(ip2) ==
            // read32(ip2)` and the rep branch is correctly suppressed
            // by the `rep_offset1 > 0` guard inside the `if`).
            // SAFETY: ip2 < ilimit ⇒ ≥ 4 readable bytes at ip2; if
            // rep_offset1 > 0 the save/restore prologue ensures
            // `ip2 - rep_offset1 >= prefix_start_index >= 1`, so the
            // backward read stays in-bounds.
            let rval = unsafe { read32(base.add(ip2 - rep_offset1 as usize)) };

            // Writeback hash for ip0 (upstream zstd line 272). Upstream zstd writes
            // BEFORE the rep probe so the hash table reflects ip0
            // even if the iteration's match comes from rep at ip2.
            // SAFETY: hash0 from hash_ptr ⇒ in-bounds; ip0 ≤ u32::MAX
            // by the entry-point cap.
            ktrace!("PUT hash0={} pos={} (iter-start)", hash0, ip0);
            unsafe { *table.get_unchecked_mut(hash0 as usize) = ip0 as u32 };

            // Repcode-at-ip2 check. Bitwise `&` (not short-circuit `&&`)
            // so both operands evaluate unconditionally — the
            // `read32(ip2)` load is always safe (`ip2 < ilimit` by the
            // loop invariant `ip3 <= ilimit` with `ip2 < ip3`, and
            // `ilimit = iend - HASH_READ_SIZE = iend - 8`, so
            // `ip2 + 4 < iend`) and `rval` is already loaded above, so
            // dropping the branch on `rep_offset1 > 0` lets the optimizer
            // fold the combined predicate into a branchless compare (the
            // upstream zstd/reference shape) instead of a short-circuit branch
            // before the load.
            if (rep_offset1 > 0) & (unsafe { read32(base.add(ip2)) } == rval) {
                // Repcode match. ip0 fast-forwards to ip2; backward-
                // extend by 1 if the byte before ip2 also matches.
                // Upstream zstd's `mLength = ip0[-1] == match0[-1]` is a
                // single-byte extension with implicit `new_ip >
                // anchor` AND `match > prefix` checks via the
                // prologue's save/restore on rep_offset1.
                let mut new_ip = ip2;
                let mut match0 = new_ip - rep_offset1 as usize;
                let mut m_len: usize = 4;
                // Upstream zstd bound: `match0 > prefixStart` ≡
                // `match_pos > windowLow` (upstream zstd's prefixStart and
                // windowLow are the same pointer in the no-dict
                // fast path). We use `window_low` here rather than
                // the sentinel-aware `prefix_start_index` so the
                // backward step can reach position 1 (impossible
                // under the sentinel) at block 0.
                // SAFETY: `new_ip > anchor >= 0` ⇒ `new_ip >= 1` and
                // `new_ip - 1 <= ip2 < data.len()`; `match0 > window_low
                // >= 0` ⇒ `match0 >= 1` and `match0 - 1 < new_ip <
                // data.len()`. Both indices are in bounds, so the raw
                // single-byte loads replace bounds-checked indexing on
                // the hot backward-extension path. `base == data.as_ptr()`.
                if new_ip > anchor
                    && match0 > window_low as usize
                    && unsafe { *base.add(new_ip - 1) == *base.add(match0 - 1) }
                {
                    new_ip -= 1;
                    match0 -= 1;
                    m_len += 1;
                }
                // Safe writeback for hash1 — ip1 is BEFORE ip2 (the
                // match site), so its position won't conflict with
                // the match's forward extension. Upstream zstd lines 286-287.
                ktrace!("PUT hash1={} pos={} (rep-emit post)", hash1, ip1);
                unsafe { *table.get_unchecked_mut(hash1 as usize) = ip1 as u32 };
                ktrace!(
                    "MATCH rep new_ip={} match0={} m_len={} offset={}",
                    new_ip,
                    match0,
                    m_len,
                    rep_offset1
                );
                break Some(MatchFound::Rep {
                    new_ip,
                    match0,
                    m_len,
                    // Iter-start ip0 (the writeback at line 426
                    // above) — upstream zstd's `current0` for this path.
                    current0: ip0,
                });
            }

            // First explicit-match probe at ip0 (upstream zstd line 292).
            ktrace!("PROBE1 ip0={} match_idx={}", ip0, match_idx);
            if unsafe {
                match_found::<USE_CMOV>(base.add(ip0), base, match_idx, prefix_start_index)
            } {
                // Safe writeback for hash1 (ip1 = ip0 + 1, before
                // search resumption). Upstream zstd line 296.
                ktrace!("PUT hash1={} pos={} (explicit1 post)", hash1, ip1);
                unsafe { *table.get_unchecked_mut(hash1 as usize) = ip1 as u32 };
                ktrace!(
                    "MATCH explicit1 ip0={} match_idx={} offset={}",
                    ip0,
                    match_idx,
                    ip0 as i64 - match_idx as i64
                );
                break Some(MatchFound::Explicit {
                    new_ip: ip0,
                    match_idx,
                    current0: ip0,
                });
            }

            // Shift: ip0 ← ip1, ip1 ← ip2, ip2 ← ip3. hash0 ← hash1
            // (precomputed last iteration). hash1 is recomputed from
            // the CURRENT ip2 (before the cursor shift below), which
            // becomes the new ip1 — so post-shift `hash1` matches
            // the new `ip1`, NOT the new `ip2`.
            match_idx = unsafe { *table.get_unchecked(hash1 as usize) };
            hash0 = hash1;
            hash1 = unsafe { hash_ptr_raw::<MLS>(base.add(ip2), hlog) };
            ip0 = ip1;
            ip1 = ip2;
            ip2 = ip3;

            // Writeback for new ip0. Upstream zstd lines 314-315.
            ktrace!("PUT hash0={} pos={} (post-shift1)", hash0, ip0);
            unsafe { *table.get_unchecked_mut(hash0 as usize) = ip0 as u32 };

            // Second explicit-match probe at the shifted ip0
            // (upstream zstd line 317).
            ktrace!("PROBE2 ip0={} match_idx={}", ip0, match_idx);
            if unsafe {
                match_found::<USE_CMOV>(base.add(ip0), base, match_idx, prefix_start_index)
            } {
                // Conditional writeback: only safe if `step <= 4`
                // (upstream zstd lines 319-324) — otherwise ip1 might fall
                // past the match start when we resume scanning.
                if step <= 4 {
                    ktrace!("PUT hash1={} pos={} (explicit2 post, step<=4)", hash1, ip1);
                    unsafe { *table.get_unchecked_mut(hash1 as usize) = ip1 as u32 };
                }
                ktrace!(
                    "MATCH explicit2 ip0={} match_idx={} offset={}",
                    ip0,
                    match_idx,
                    ip0 as i64 - match_idx as i64
                );
                break Some(MatchFound::Explicit {
                    new_ip: ip0,
                    match_idx,
                    current0: ip0,
                });
            }

            // Shift again with the larger `step` gap. The second
            // shift is the one that advances ip2/ip3 by `step`
            // rather than by 1; this is where the step-skip kicks
            // in. Upstream zstd lines 329-339.
            match_idx = unsafe { *table.get_unchecked(hash1 as usize) };
            hash0 = hash1;
            hash1 = unsafe { hash_ptr_raw::<MLS>(base.add(ip2), hlog) };
            ip0 = ip1;
            ip1 = ip2;
            // Same overflow defence as the loop-head setup: a wild
            // `step` (e.g. after enough step-doubling cycles) could
            // otherwise wrap `ip0 + step` past `usize::MAX` and bypass
            // the `ip3 > ilimit` guard. On overflow we drain to the
            // post-loop cleanup, identical to the normal "ran out of
            // room" exit.
            let Some(new_ip2) = ip0.checked_add(step) else {
                break None;
            };
            let Some(new_ip3) = ip1.checked_add(step) else {
                break None;
            };
            ip2 = new_ip2;
            ip3 = new_ip3;

            // Step-doubling: upstream zstd lines 342-347. Drives the
            // kSearchStrength-based acceleration on incompressible
            // regions.
            if ip2 >= next_step {
                step += 1;
                next_step = next_step.saturating_add(K_STEP_INCR);
            }

            // do-while termination: if ip3 walks past ilimit, drain.
            if ip3 > ilimit {
                break None;
            }
        };

        // _cleanup path: drain to the post-loop save/restore.
        let Some(found) = found else {
            break 'restart;
        };

        // _offset / _match — backward + forward extension + emit.
        // Upstream zstd's `current0` = position of the LAST hash writeback
        // before the match was found. Captured at break-time on
        // each MatchFound variant so the Rep path's backward
        // extension doesn't collapse `current0` onto the post-
        // backward `new_ip` (upstream zstd zstd_fast.c:407 uses the
        // pre-backward iter-start position).
        let current0 = match found {
            MatchFound::Rep { current0, .. } => current0,
            MatchFound::Explicit { current0, .. } => current0,
        };
        let (mut match_ip, mut match_pos, mut m_len, offset, is_rep) = match found {
            MatchFound::Rep {
                new_ip,
                match0,
                m_len,
                current0: _,
            } => (new_ip, match0, m_len, rep_offset1 as usize, true),
            MatchFound::Explicit {
                new_ip,
                match_idx,
                current0: _,
            } => {
                let match_pos = match_idx as usize;
                // Upstream zstd invariant: hash table writes for ip_pos
                // happen BEFORE the probe reads `match_idx` (see the
                // writeback at the top of the do-while body), so the
                // returned `match_idx` is always strictly less than
                // `new_ip` (it was a hash slot occupant from a prior
                // shift step where `current0_prev < ip0_now`).
                // Upstream zstd `ZSTD_match4Found_branch` relies on the same
                // invariant — neither side adds a release-time check.
                debug_assert!(
                    match_pos < new_ip,
                    "kernel invariant violated: match_pos ({match_pos}) >= new_ip ({new_ip}); \
                     hash table holds forward-pointing entry — driver/test broke writeback ordering"
                );
                let offset = new_ip - match_pos;
                // Rotate the rep stack ahead of backward extension
                // — upstream zstd stores the offset BEFORE the backward
                // walk (lines 381-383). This way the explicit-match
                // path's backward extension is bounded by the
                // anchor + prefix_start_index pair; subsequent
                // iterations get a tighter rep_offset1.
                rep_offset2 = rep_offset1;
                rep_offset1 = offset as u32;
                (new_ip, match_pos, 4usize, offset, false)
            }
        };

        // Backward extension — only for explicit matches; rep path
        // already handled the 1-byte backward step above. Upstream zstd's
        // bound is `match0 > prefixStart` ≡ `match_pos > windowLow`;
        // we mirror it via `window_low` (NOT `prefix_start_index`,
        // which is sentinel-floored at 1 for hash-filter purposes
        // only).
        if !is_rep {
            // SAFETY: each iteration's guard `match_ip > anchor >= 0` and
            // `match_pos > window_low >= 0` give `match_ip >= 1`,
            // `match_pos >= 1`; `match_ip - 1 < match_ip <= ip0 <
            // data.len()` and `match_pos - 1 < match_pos < match_ip`, so
            // both single-byte loads are in bounds. Raw loads replace
            // bounds-checked indexing on the hot backward-extension loop.
            // `base == data.as_ptr()`.
            while match_ip > anchor
                && match_pos > window_low as usize
                && unsafe { *base.add(match_ip - 1) == *base.add(match_pos - 1) }
            {
                match_ip -= 1;
                match_pos -= 1;
                m_len += 1;
            }
        }

        // Forward extension via ZSTD_count.
        // SAFETY: both pointers stay within `data`; iend pointer
        // arithmetic stays in bounds.
        let forward = unsafe {
            count_forward(
                base.add(match_ip + m_len),
                base.add(match_pos + m_len),
                base.add(iend_addr),
            )
        };
        m_len += forward;

        // Emit.
        // SAFETY: the backward-extension loop above stops at
        // `match_ip == anchor` (or a byte mismatch), so `anchor <=
        // match_ip`; `match_ip <= ip0 < data.len()`. The range is valid,
        // so the unchecked slice avoids the bounds pair on the per-match
        // literal gather.
        let literals = unsafe { data.get_unchecked(anchor..match_ip) };
        handle_sequence(Sequence::Triple {
            literals,
            offset,
            match_len: m_len,
        });

        ip0 = match_ip + m_len;
        anchor = ip0;

        // Immediate-rep2 inner loop (upstream zstd lines 404-420). After a
        // match emit, upstream zstd (a) refills the hash for `ip0 - 2` to
        // give the next scan a head start, (b) probes for repcode-2
        // matches at the new ip0 — if found, emit them as lit_len=0
        // rep1 sequences (with rep stack swap) until exhausted.
        if ip0 <= ilimit {
            // Upstream zstd inserts TWO hashes after a match emit
            // (zstd_fast.c:407-408): one at `current0 + 2` (2 bytes
            // past the trigger-ip position, filling a slot inside
            // the now-consumed match), one at `ip0 - 2` (2 bytes
            // before the next scan start). Both are vital — missing
            // either makes the hash table sparser than upstream zstd's and
            // costs subsequent matches.
            //
            // SAFETY: each `base.add(N - 2)` covers ≥ 8 readable
            // bytes when N - 2 ≤ ilimit (ilimit = iend - 8).
            // Non-overflowing bounds check: on 32-bit targets even
            // the raw `current0 + 2` can wrap when `current0`
            // approaches `usize::MAX`, producing a small value that
            // would then pass the `+ HASH_READ_SIZE` check and call
            // `hash_ptr` at the wrong position — out-of-bounds read
            // under the surrounding `unsafe`. Chain the additions
            // through `checked_add` so the slot is skipped if either
            // step overflows.
            let in_range_fwd = current0
                .checked_add(2)
                .and_then(|c| c.checked_add(HASH_READ_SIZE))
                .is_some_and(|end| end <= iend_addr);
            if in_range_fwd {
                // Safe to compute the index here — `in_range_fwd`
                // guarantees `current0 + 2 + HASH_READ_SIZE` fits in
                // `usize`, so the `+ 2` cannot wrap.
                let current0_plus_2 = current0 + 2;
                let h_fwd = unsafe { hash_ptr_raw::<MLS>(base.add(current0_plus_2), hlog) };
                unsafe { *table.get_unchecked_mut(h_fwd as usize) = current0_plus_2 as u32 };
            }
            if ip0 >= 2 {
                let h_back = unsafe { hash_ptr_raw::<MLS>(base.add(ip0 - 2), hlog) };
                unsafe { *table.get_unchecked_mut(h_back as usize) = (ip0 - 2) as u32 };
            }

            // Repcode-2 inner loop. Upstream zstd swaps rep1 / rep2 on each
            // hit so the just-found offset becomes the new rep1.
            while rep_offset2 > 0
                && ip0 <= ilimit
                && ip0 >= rep_offset2 as usize
                && unsafe { read32(base.add(ip0)) == read32(base.add(ip0 - rep_offset2 as usize)) }
            {
                // 4-byte match guaranteed by the equality probe.
                // Extend forward via count_forward starting at
                // ip0 + 4.
                let r_off = rep_offset2 as usize;
                let r_extra = unsafe {
                    count_forward(
                        base.add(ip0 + 4),
                        base.add(ip0 + 4 - r_off),
                        base.add(iend_addr),
                    )
                };
                let r_len = 4 + r_extra;

                // Swap rep1 / rep2 (upstream zstd line 414).
                core::mem::swap(&mut rep_offset1, &mut rep_offset2);

                // Hash refill at ip0 (upstream zstd line 415).
                let h_at = unsafe { hash_ptr_raw::<MLS>(base.add(ip0), hlog) };
                unsafe { *table.get_unchecked_mut(h_at as usize) = ip0 as u32 };

                // Emit lit_len=0 rep1 sequence.
                // SAFETY: this immediate-rep2 branch runs with `anchor ==
                // ip0` before the match (lit_len 0), so `anchor <= ip0`
                // and `ip0 < data.len()`; the unchecked slice avoids the
                // bounds pair on the per-match literal gather.
                handle_sequence(Sequence::Triple {
                    literals: unsafe { data.get_unchecked(anchor..ip0) },
                    offset: r_off,
                    match_len: r_len,
                });

                ip0 += r_len;
                anchor = ip0;
            }
        }

        // `goto _start` — restart the outer loop with fresh step
        // (reset to the level-resolved initial step_size). The
        // `saturating_add` here is symmetric with the inner loop's
        // step-doubling site: ip0 is already < ilimit < data.len()
        // <= u32::MAX, so the wrap would require an unrepresentable
        // `K_STEP_INCR`, but staying defensive keeps the cursor math
        // wrap-free on every cold reset of the outer state.
        step = step_size;
        next_step = ip0.saturating_add(K_STEP_INCR);
        continue 'restart;
    }

    // Repcode save/restore: if `rep_offset1` came in invalid
    // (offset_saved1 != 0) and finished valid (rep_offset1 != 0),
    // then the upstream zstd-saved offset becomes the new rep[1]. Mirrors
    // `offsetSaved2 = ((offsetSaved1 != 0) && (rep_offset1 != 0)) ?
    // offsetSaved1 : offsetSaved2;`.
    if offset_saved1 != 0 && rep_offset1 != 0 {
        offset_saved2 = offset_saved1;
    }

    let final_rep = [
        if rep_offset1 != 0 {
            rep_offset1
        } else {
            offset_saved1
        },
        if rep_offset2 != 0 {
            rep_offset2
        } else {
            offset_saved2
        },
    ];

    FastBlockResult {
        rep: final_rep,
        tail_literals_len: data.len() - anchor,
    }
}

/// Attach-mode dict Fast kernel — flat-buffer port of upstream zstd
/// `ZSTD_compressBlock_fast_dictMatchState_generic` (`zstd_fast.c:483-678`),
/// the 2-cursor dict-aware search C uses for small / unknown-size inputs
/// (`ZSTD_shouldAttachDict`: `pledgedSrcSize <= 8 KB` for the Fast strategy, or
/// size unknown). The caller routes large known-size inputs through the plain
/// 4-cursor [`compress_block_fast`] with the dictionary copied into the window
/// instead (upstream zstd's "copy" mode) — that path already matches/beats the upstream zstd on
/// large corpora, so this attach path exists ONLY to win the small/unknown case
/// the 4-cursor parse-order misses.
///
/// Flat-model adaptations vs the upstream zstd: the dictionary occupies `data[1..
/// dict_end]` immediately before the frame input (mirroring the decoder's
/// `[dict][output]` window), so a dict-match offset is `ip - dict_pos` like any
/// in-window match and match counts cross the dict→input boundary freely (no
/// `ZSTD_count_2segments`, no `dictBase`/`dictIndexDelta`). `dict_table` stores
/// short-cache tagged positions (`(position << DICT_TAG_BITS) | tag`, upstream
/// zstd `ZSTD_SHORT_CACHE` — the tag rejects a colliding slot without a wasted
/// candidate load); `main_table` holds ONLY frame-input positions. Search order
/// mirrors the upstream zstd: rep@ip0+1 → dict (only when the main candidate is
/// invalid) → main. Correctness is independent of table contents (every match
/// byte-verified + bounds-guarded); `MIN_DICT_MATCH_LEN` gates short far dict
/// matches that would fragment the offset-FSE alphabet.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compress_block_fast_dict<const MLS: u32, const USE_CMOV: bool>(
    data: &[u8],
    block_start: usize,
    bounds: PrefixBounds,
    main_table: &mut FastHashTable,
    dict_table: &FastHashTable,
    dict_end: u32,
    rep: [u32; 2],
    step_size: usize,
    mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
) -> FastBlockResult {
    assert!(
        block_start <= data.len(),
        "block_start ({block_start}) must not exceed data.len() ({})",
        data.len(),
    );
    assert!(
        data.len() <= u32::MAX as usize,
        "FastKernel does not support data.len() ({}) > u32::MAX",
        data.len(),
    );
    debug_assert_eq!(MLS, main_table.mls());
    debug_assert_eq!(MLS, dict_table.mls());
    debug_assert_eq!(main_table.hash_log(), dict_table.hash_log());

    let prefix_start_index = bounds.prefix_start_index;
    let window_low = bounds.window_low as usize;
    let dict_end = dict_end as usize;

    if data.len() < block_start + HASH_READ_SIZE {
        return FastBlockResult {
            rep,
            tail_literals_len: data.len() - block_start,
        };
    }

    let base = data.as_ptr();
    let iend_addr = data.len();
    let ilimit = iend_addr - HASH_READ_SIZE;

    let mut anchor: usize = block_start;
    let mut ip0: usize = block_start;
    if ip0 == 0 {
        ip0 = 1;
    }
    let mut ip1 = ip0 + step_size;

    let mut offset_1: u32 = rep[0];
    let mut offset_2: u32 = rep[1];

    // Slot width for the dict table: `dict_lookup` hashes `ptr` to
    // `dict_hash_log + DICT_TAG_BITS` bits — the high `dict_hash_log` select the
    // slot (table length `1 << dict_hash_log`), the low `DICT_TAG_BITS` are the
    // collision tag. That hash is only well-defined while the total width fits 32
    // bits, so guard it here (always-on, not debug_assert): these safe kernels
    // accept any `FastHashTable::new` shape, and a `hash_log > 24` dict table
    // would otherwise reach an out-of-range slot in release.
    let dict_hash_log = dict_table.hash_log();
    assert!(
        dict_hash_log + DICT_TAG_BITS <= 32,
        "tagged Fast dict lookup requires dict hash_log <= {} (got {dict_hash_log})",
        32 - DICT_TAG_BITS,
    );
    // Hoist the main table's backing slice + hash_log + epoch bias into locals
    // so the hot loop's hash/get/put avoid re-reading the `Vec` header / the
    // `hash_log` / `bias` field through `&mut FastHashTable` on every access (the
    // "chases the Vec" reload). The dict kernel runs on a POSSIBLY-biased table,
    // so the bias is applied inline (`saturating_sub` on read, `+ bias` on write)
    // exactly as `FastHashTable::get`/`put` do — byte-identical, reload-free.
    let (main_tbl, main_hlog, main_bias) = main_table.hot_state_biased();

    // Inner-loop result: literals end (where the match copy begins), the raw
    // match offset, the match length, and the upstream zstd `curr` (probe position,
    // for the post-match `curr + 2` fill). `None` → drain to cleanup.
    struct DictMatch {
        lit_end: usize,
        offset: usize,
        m_len: usize,
        curr: usize,
    }

    'outer: while ip1 <= ilimit {
        // SAFETY: ip0 < ip1 <= ilimit = iend - 8 ⇒ ≥ 8 readable bytes at ip0.
        let mut hash0 = unsafe { hash_ptr_raw::<MLS>(base.add(ip0), main_hlog) };
        let mut main_idx =
            unsafe { (*main_tbl.get_unchecked(hash0 as usize)).saturating_sub(main_bias) };
        let mut dict_idx = unsafe { dict_lookup::<MLS>(dict_table, base.add(ip0), dict_hash_log) };
        let mut curr = ip0;

        let found: Option<DictMatch> = loop {
            // SAFETY: ip1 <= ilimit ⇒ ≥ 8 readable bytes at ip1.
            let hash1 = unsafe { hash_ptr_raw::<MLS>(base.add(ip1), main_hlog) };
            // Insert current position into the MAIN table (upstream zstd line 565).
            unsafe { *main_tbl.get_unchecked_mut(hash0 as usize) = curr as u32 + main_bias };

            // Repcode probe for a match starting at ip0 + 1 (upstream zstd 559-574).
            if offset_1 > 0 && curr + 1 >= offset_1 as usize + window_low {
                let rep_index = curr + 1 - offset_1 as usize;
                // SAFETY: ip0+1+4 <= ilimit+5 < iend; rep_index < ip0+1 so
                // rep_index+4 < iend. Both 4-byte reads in bounds.
                if unsafe { read32(base.add(ip0 + 1)) == read32(base.add(rep_index)) } {
                    let m_len = 4 + unsafe {
                        count_forward(
                            base.add(ip0 + 1 + 4),
                            base.add(rep_index + 4),
                            base.add(iend_addr),
                        )
                    };
                    break Some(DictMatch {
                        lit_end: ip0 + 1,
                        offset: offset_1 as usize,
                        m_len,
                        curr,
                    });
                }
            }

            // Dictionary match — taken ONLY when the main candidate is below
            // the window floor, i.e. exactly the indices `match_found` rejects
            // (`match_idx >= prefix_start_index` is in-window). The floor-
            // aligned case `main_idx == prefix_start_index` is a VALID recent
            // candidate, so it must fall through to the main-match probe below
            // rather than the dict path — keeping "recent input wins, dict is
            // the fallback". (Upstream zstd's `matchIndex <= prefixStartIndex` gate is
            // an if/else-if with no main fallthrough; our two-`if` structure
            // needs `<` here to stay consistent with `match_found`'s floor.)
            if main_idx < prefix_start_index {
                let dpos = dict_idx as usize;
                if dict_idx >= 1
                    && dpos < dict_end
                    && dpos >= window_low
                    && unsafe { read32(base.add(ip0)) == read32(base.add(dpos)) }
                {
                    let mut match_ip = ip0;
                    let mut match_pos = dpos;
                    let mut m_len = 4 + unsafe {
                        count_forward(base.add(ip0 + 4), base.add(dpos + 4), base.add(iend_addr))
                    };
                    // Catch-up backward extension into the dict (upstream zstd 586-591).
                    while match_ip > anchor
                        && match_pos > window_low
                        && unsafe { *base.add(match_ip - 1) == *base.add(match_pos - 1) }
                    {
                        match_ip -= 1;
                        match_pos -= 1;
                        m_len += 1;
                    }
                    if m_len >= MIN_DICT_MATCH_LEN {
                        let offset = match_ip - match_pos;
                        offset_2 = offset_1;
                        offset_1 = offset as u32;
                        break Some(DictMatch {
                            lit_end: match_ip,
                            offset,
                            m_len,
                            curr,
                        });
                    }
                }
            }

            // Main match (recent input) — upstream zstd line 600.
            if unsafe { match_found::<USE_CMOV>(base.add(ip0), base, main_idx, prefix_start_index) }
            {
                let mut match_ip = ip0;
                let mut match_pos = main_idx as usize;
                let mut m_len = 4 + unsafe {
                    count_forward(
                        base.add(ip0 + 4),
                        base.add(match_pos + 4),
                        base.add(iend_addr),
                    )
                };
                while match_ip > anchor
                    && match_pos > window_low
                    && unsafe { *base.add(match_ip - 1) == *base.add(match_pos - 1) }
                {
                    match_ip -= 1;
                    match_pos -= 1;
                    m_len += 1;
                }
                let offset = match_ip - match_pos;
                offset_2 = offset_1;
                offset_1 = offset as u32;
                break Some(DictMatch {
                    lit_end: match_ip,
                    offset,
                    m_len,
                    curr,
                });
            }

            // Prepare next iteration (upstream zstd 616-630).
            dict_idx = unsafe { dict_lookup::<MLS>(dict_table, base.add(ip1), dict_hash_log) };
            main_idx =
                unsafe { (*main_tbl.get_unchecked(hash1 as usize)).saturating_sub(main_bias) };
            ip0 = ip1;
            ip1 += step_size;
            if ip1 > ilimit {
                break None;
            }
            curr = ip0;
            hash0 = hash1;
        };

        let Some(m) = found else {
            break 'outer;
        };

        handle_sequence(Sequence::Triple {
            literals: &data[anchor..m.lit_end],
            offset: m.offset,
            match_len: m.m_len,
        });
        ip0 = m.lit_end + m.m_len;
        anchor = ip0;

        if ip0 <= ilimit {
            // Post-match dense fills (upstream zstd 641-642).
            if m.curr + 2 + HASH_READ_SIZE <= iend_addr {
                let h = unsafe { hash_ptr_raw::<MLS>(base.add(m.curr + 2), main_hlog) };
                unsafe {
                    *main_tbl.get_unchecked_mut(h as usize) = (m.curr + 2) as u32 + main_bias
                };
            }
            if ip0 >= 2 {
                let h = unsafe { hash_ptr_raw::<MLS>(base.add(ip0 - 2), main_hlog) };
                unsafe { *main_tbl.get_unchecked_mut(h as usize) = (ip0 - 2) as u32 + main_bias };
            }
            // Immediate repcode-2 loop (upstream zstd 644-663).
            while ip0 <= ilimit
                && offset_2 > 0
                && ip0 >= offset_2 as usize + window_low
                && unsafe { read32(base.add(ip0)) == read32(base.add(ip0 - offset_2 as usize)) }
            {
                let r_off = offset_2 as usize;
                let r_len = 4 + unsafe {
                    count_forward(
                        base.add(ip0 + 4),
                        base.add(ip0 + 4 - r_off),
                        base.add(iend_addr),
                    )
                };
                core::mem::swap(&mut offset_1, &mut offset_2);
                let h = unsafe { hash_ptr_raw::<MLS>(base.add(ip0), main_hlog) };
                unsafe { *main_tbl.get_unchecked_mut(h as usize) = ip0 as u32 + main_bias };
                handle_sequence(Sequence::Triple {
                    literals: &data[anchor..ip0],
                    offset: r_off,
                    match_len: r_len,
                });
                ip0 += r_len;
                anchor = ip0;
            }
        }

        ip1 = ip0 + step_size;
    }

    FastBlockResult {
        rep: [offset_1, offset_2],
        tail_literals_len: data.len() - anchor,
    }
}

/// Forward match length for a candidate at virtual position `cand_abs` in the
/// logical `[dict][input]` window, compared against the current input at offset
/// `cur_off`, for the borrowed dual-base dict kernel. Dispatches ONCE on which
/// buffer the candidate lives in (never per byte): an input candidate
/// (`cand_abs >= dict_end`) takes the `read32` 4-byte gate + word-at-a-time
/// [`count_forward`] hot path over the borrowed input; a dict-prefix candidate
/// (`cand_abs < dict_end`) falls to the scalar [`count_forward_dict_2segment`]
/// that crosses the dict→input boundary (the cold fallback, mirroring upstream
/// zstd's `repBase`/`dictBase` candidate split). Returns 0 when the 4-byte gate
/// fails so the caller's `>= 4` check rejects it in one place.
///
/// # Safety
/// `inp_base` must have `block_end` readable bytes; `cur_off + 4 <= block_end`
/// and, for an input candidate, `cand_off + 4 <= block_end`. `dict` covers the
/// `[0, dict_end)` prefix and `inp` covers `[0, block_end)`.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
unsafe fn borrowed_candidate_len<C: Fn(*const u8, *const u8, usize) -> usize>(
    cand_abs: usize,
    cur_off: usize,
    dict_end: usize,
    dict: &[u8],
    inp: &[u8],
    inp_base: *const u8,
    block_end: usize,
    cpl: &C,
) -> usize {
    if cand_abs >= dict_end {
        let cand_off = cand_abs - dict_end;
        // SAFETY: caller guarantees `cur_off + 4 <= block_end` and
        // `cand_off + 4 <= block_end` (cand_off < cur_off). `cpl` (the active
        // tier's `common_prefix_len_ptr`) is bounded by `max = block_end -
        // (cur_off + 4)`, so the current side reads `inp[cur_off+4 ..
        // block_end]` and the candidate side `cand_off < cur_off` more bytes
        // — both within `inp`.
        if unsafe { read32(inp_base.add(cur_off)) != read32(inp_base.add(cand_off)) } {
            return 0;
        }
        4 + unsafe {
            cpl(
                inp_base.add(cur_off + 4),
                inp_base.add(cand_off + 4),
                block_end - (cur_off + 4),
            )
        }
    } else {
        let l = count_forward_dict_2segment(dict, cand_abs, inp, cur_off);
        if l >= 4 { l } else { 0 }
    }
}

/// Dual-base port of [`compress_block_fast_dict`] for the *borrowed* dict-attach
/// path: the dictionary lives in a buffer (`dict`, the `[0, dict_end)` prefix)
/// SEPARATE from the borrowed frame input (`inp`, read in place), so the flat
/// single-base kernel above cannot be used. Positions live in the logical
/// `[dict][input]` window: a dict byte `d` has virtual position `d`; an input
/// byte at offset `i` has virtual position `dict_end + i`. The main hash table
/// stores virtual input positions (`dict_end + i`); the immutable `dict_table`
/// stores dict positions (`< dict_end`). An emitted offset is `cur_abs -
/// cand_abs`.
///
/// Monomorphised on `MLS` + `USE_CMOV` exactly like the owned kernel, so it
/// keeps that path's full machinery — `read32` 4-byte gate, word-at-a-time
/// [`count_forward`], the step-ramped `ip0`/`ip1` two-position lookahead, the
/// repcode-at-`ip0+1` probe, post-match dense fills, backward catch-up
/// extension and the immediate repcode-2 loop — none of which the prior scalar
/// greedy scan had (it was +68% slower than this owned-kernel shape). The only
/// dual-base divergences: current-position reads are always input
/// (`inp_base.add(off)`); a candidate's match length goes through
/// [`borrowed_candidate_len`], which routes dict-prefix candidates (the rep
/// state after dict priming, and the dict-table fallback) through the 2-segment
/// counter while keeping input candidates on the fast flat path.
#[inline(always)]
#[allow(clippy::too_many_arguments)]
fn compress_block_fast_dict_borrowed_impl<
    const MLS: u32,
    const USE_CMOV: bool,
    C: Fn(*const u8, *const u8, usize) -> usize,
>(
    inp: &[u8],
    dict: &[u8],
    block_start: usize,
    block_end: usize,
    main_table: &mut FastHashTable,
    dict_table: &FastHashTable,
    bounds: PrefixBounds,
    rep: [u32; 2],
    step_size: usize,
    mut handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    cpl: C,
) -> FastBlockResult {
    assert!(
        block_start <= block_end && block_end <= inp.len(),
        "borrowed dict block bounds out of range: start={block_start} end={block_end} inp_len={}",
        inp.len(),
    );
    let dict_end = dict.len();
    assert!(
        dict_end >= 1,
        "borrowed dict kernel requires a non-empty dictionary (sentinel-0 safety)",
    );
    // Checked virtual-length bound: the [dict][input] coordinate floor relies on
    // `dict_end + block_end` fitting `u32`; validate with `checked_add` rather
    // than adding then checking after (which could itself overflow `usize`).
    assert!(
        dict_end
            .checked_add(block_end)
            .is_some_and(|v| v <= u32::MAX as usize),
        "FastKernel does not support dict_end + block_end > u32::MAX (dict_end={dict_end}, block_end={block_end})",
    );
    // Release checks (not debug_assert): these guard UNCHECKED table access in
    // the hot loop — a mismatched hash_log would hash with `main_table` then
    // index past `dict_table`, and `step_size < 2` would stall the scan.
    assert!(
        step_size >= 2,
        "borrowed dict kernel requires step_size >= 2 (got {step_size})",
    );
    assert_eq!(MLS, main_table.mls());
    assert_eq!(MLS, dict_table.mls());
    assert_eq!(main_table.hash_log(), dict_table.hash_log());

    // Window bounds in VIRTUAL `[dict][input]` coords, so the gates match the
    // owned flat kernel: `window_low` is the absolute floor, `prefix_start_index`
    // the sentinel-aware floor for the hash-slot filter.
    let prefix_start_index = bounds.prefix_start_index;
    let window_low = bounds.window_low as usize;

    if block_end < block_start + HASH_READ_SIZE {
        return FastBlockResult {
            rep,
            tail_literals_len: block_end - block_start,
        };
    }

    let inp_base = inp.as_ptr();
    // Last input offset with HASH_READ_SIZE readable bytes ahead.
    let ilimit = block_end - HASH_READ_SIZE;

    let mut anchor: usize = block_start;
    // Input offset 0 maps to virtual `dict_end >= 1`, so it never aliases the
    // empty-slot sentinel 0 — no `ip0 == 0` fixup needed (unlike the owned flat
    // kernel where position 0 is the dict's first byte).
    let mut ip0: usize = block_start;
    let mut ip1 = ip0 + step_size;

    let mut offset_1: u32 = rep[0];
    let mut offset_2: u32 = rep[1];

    // Same tagged-lookup width guard as the owned kernel: `dict_lookup` hashes to
    // `dict_hash_log + DICT_TAG_BITS` bits, well-defined only while that fits 32
    // bits. Always-on (not debug_assert) — a `hash_log > 24` dict table would
    // reach an out-of-range slot in release otherwise.
    let dict_hash_log = dict_table.hash_log();
    assert!(
        dict_hash_log + DICT_TAG_BITS <= 32,
        "tagged Fast dict lookup requires dict hash_log <= {} (got {dict_hash_log})",
        32 - DICT_TAG_BITS,
    );
    // Hoist the main table's backing slice + hash_log + epoch bias into locals so
    // the hot loop avoids re-reading the `Vec` header / `hash_log` / `bias`
    // through `&mut FastHashTable` on every access; bias is applied inline exactly
    // as `get`/`put` do (matches the owned dict kernel).
    let (main_tbl, main_hlog, main_bias) = main_table.hot_state_biased();

    // Inner-loop result: literals end (input offset), raw match offset, match
    // length, and the upstream zstd `curr` probe offset for the post-match `curr + 2`
    // fill. `None` → drain to cleanup.
    struct DictMatch {
        lit_end: usize,
        offset: usize,
        m_len: usize,
        curr: usize,
    }

    'outer: while ip1 <= ilimit {
        // SAFETY: ip0 < ip1 <= ilimit = block_end - 8 ⇒ ≥ 8 readable bytes at ip0.
        let mut hash0 = unsafe { hash_ptr_raw::<MLS>(inp_base.add(ip0), main_hlog) };
        let mut main_idx =
            unsafe { (*main_tbl.get_unchecked(hash0 as usize)).saturating_sub(main_bias) };
        let mut dict_idx =
            unsafe { dict_lookup::<MLS>(dict_table, inp_base.add(ip0), dict_hash_log) };
        let mut curr = ip0;

        let found: Option<DictMatch> = loop {
            // SAFETY: ip1 <= ilimit ⇒ ≥ 8 readable bytes at ip1.
            let hash1 = unsafe { hash_ptr_raw::<MLS>(inp_base.add(ip1), main_hlog) };
            let cur_abs = dict_end + curr;
            // Insert current virtual position into the MAIN table.
            unsafe { *main_tbl.get_unchecked_mut(hash0 as usize) = cur_abs as u32 + main_bias };

            // Repcode probe for a match starting at curr + 1. The rep candidate
            // may sit in the dict (offsets primed from the dictionary's
            // repToConfirm) or in the input; `borrowed_candidate_len` routes it.
            if offset_1 > 0 && cur_abs + 1 >= offset_1 as usize + window_low {
                let rep_abs = cur_abs + 1 - offset_1 as usize;
                let m_len = unsafe {
                    borrowed_candidate_len(
                        rep_abs,
                        curr + 1,
                        dict_end,
                        dict,
                        inp,
                        inp_base,
                        block_end,
                        &cpl,
                    )
                };
                if m_len >= 4 {
                    break Some(DictMatch {
                        lit_end: curr + 1,
                        offset: offset_1 as usize,
                        m_len,
                        curr,
                    });
                }
            }

            // Dictionary match — taken ONLY when the main candidate is below the
            // window floor (exactly the indices the main probe rejects), keeping
            // "recent input wins, dict is the fallback".
            if main_idx < prefix_start_index {
                let dpos = dict_idx as usize;
                if dict_idx >= 1 && dpos < dict_end && dpos >= window_low {
                    let m0 = count_forward_dict_2segment(dict, dpos, inp, curr);
                    if m0 >= 4 {
                        let mut match_ip = curr;
                        let mut match_pos = dpos;
                        let mut m_len = m0;
                        // Backward catch-up into the dict (current side in input,
                        // candidate side in the dict prefix).
                        while match_ip > anchor
                            && match_pos > window_low
                            && unsafe { *inp_base.add(match_ip - 1) == dict[match_pos - 1] }
                        {
                            match_ip -= 1;
                            match_pos -= 1;
                            m_len += 1;
                        }
                        if m_len >= MIN_DICT_MATCH_LEN {
                            let offset = (dict_end + match_ip) - match_pos;
                            offset_2 = offset_1;
                            offset_1 = offset as u32;
                            break Some(DictMatch {
                                lit_end: match_ip,
                                offset,
                                m_len,
                                curr,
                            });
                        }
                    }
                }
            }

            // Main match (recent input) — `main_idx` is a virtual input position
            // (`>= dict_end`); the candidate read is at input offset
            // `main_idx - dict_end`.
            // INVARIANT: every main-table entry is either the empty sentinel 0
            // or a VIRTUAL input position `>= dict_end` — the scan stores
            // `dict_end + off` and the borrowed priming rebases primed offsets by
            // `dict_end` too (no raw offsets in `[1, dict_end)`). So
            // `main_idx >= prefix_start_index` (which is `>= 1`) already implies
            // `main_idx >= dict_end`, and `main_idx - dict_end` cannot underflow
            // — no per-candidate range check is needed on the hot path.
            debug_assert!(
                main_idx == 0 || main_idx as usize >= dict_end,
                "main-table entry must be the sentinel or a virtual input position (>= dict_end)",
            );
            let main_valid = if USE_CMOV {
                let in_range = main_idx >= prefix_start_index;
                // SAFETY: when `in_range`, `main_idx >= prefix_start_index >= 1`
                // and (per the invariant) `>= dict_end`, so `main_idx - dict_end`
                // is a valid input offset with ≥ 4 readable bytes; otherwise
                // `CMOV_DUMMY` (4 bytes) is read instead.
                let mval_addr = if in_range {
                    unsafe { inp_base.add(main_idx as usize - dict_end) }
                } else {
                    CMOV_DUMMY.as_ptr()
                };
                let bytes_match = unsafe { read32(inp_base.add(ip0)) == read32(mval_addr) };
                #[allow(clippy::needless_bitwise_bool)]
                let r = bytes_match & in_range;
                r
            } else {
                main_idx >= prefix_start_index
                    && unsafe {
                        read32(inp_base.add(ip0))
                            == read32(inp_base.add(main_idx as usize - dict_end))
                    }
            };
            if main_valid {
                let mut match_ip = ip0;
                let mut match_pos = main_idx as usize - dict_end;
                let mut m_len = 4 + unsafe {
                    cpl(
                        inp_base.add(ip0 + 4),
                        inp_base.add(match_pos + 4),
                        block_end - (ip0 + 4),
                    )
                };
                while match_ip > anchor
                    && match_pos > 0
                    && (dict_end + match_pos) > window_low
                    && unsafe { *inp_base.add(match_ip - 1) == *inp_base.add(match_pos - 1) }
                {
                    match_ip -= 1;
                    match_pos -= 1;
                    m_len += 1;
                }
                let offset = match_ip - match_pos;
                offset_2 = offset_1;
                offset_1 = offset as u32;
                break Some(DictMatch {
                    lit_end: match_ip,
                    offset,
                    m_len,
                    curr,
                });
            }

            // Prepare next iteration.
            dict_idx = unsafe { dict_lookup::<MLS>(dict_table, inp_base.add(ip1), dict_hash_log) };
            main_idx =
                unsafe { (*main_tbl.get_unchecked(hash1 as usize)).saturating_sub(main_bias) };
            ip0 = ip1;
            ip1 += step_size;
            if ip1 > ilimit {
                break None;
            }
            curr = ip0;
            hash0 = hash1;
        };

        let Some(m) = found else {
            break 'outer;
        };

        handle_sequence(Sequence::Triple {
            literals: &inp[anchor..m.lit_end],
            offset: m.offset,
            match_len: m.m_len,
        });
        ip0 = m.lit_end + m.m_len;
        anchor = ip0;

        if ip0 <= ilimit {
            // Post-match dense fills (virtual positions).
            if m.curr + 2 + HASH_READ_SIZE <= block_end {
                let h = unsafe { hash_ptr_raw::<MLS>(inp_base.add(m.curr + 2), main_hlog) };
                unsafe {
                    *main_tbl.get_unchecked_mut(h as usize) =
                        (dict_end + m.curr + 2) as u32 + main_bias
                };
            }
            if ip0 >= 2 {
                let h = unsafe { hash_ptr_raw::<MLS>(inp_base.add(ip0 - 2), main_hlog) };
                unsafe {
                    *main_tbl.get_unchecked_mut(h as usize) =
                        (dict_end + ip0 - 2) as u32 + main_bias
                };
            }
            // Immediate repcode-2 loop. The rep candidate may straddle into the
            // dict; `borrowed_candidate_len` routes it.
            while ip0 <= ilimit && offset_2 > 0 && dict_end + ip0 >= offset_2 as usize + window_low
            {
                let rep_abs = dict_end + ip0 - offset_2 as usize;
                let r_len = unsafe {
                    borrowed_candidate_len(
                        rep_abs, ip0, dict_end, dict, inp, inp_base, block_end, &cpl,
                    )
                };
                if r_len < 4 {
                    break;
                }
                let r_off = offset_2 as usize;
                core::mem::swap(&mut offset_1, &mut offset_2);
                let h = unsafe { hash_ptr_raw::<MLS>(inp_base.add(ip0), main_hlog) };
                unsafe {
                    *main_tbl.get_unchecked_mut(h as usize) = (dict_end + ip0) as u32 + main_bias
                };
                handle_sequence(Sequence::Triple {
                    literals: &inp[anchor..ip0],
                    offset: r_off,
                    match_len: r_len,
                });
                ip0 += r_len;
                anchor = ip0;
            }
        }

        ip1 = ip0 + step_size;
    }

    FastBlockResult {
        rep: [offset_1, offset_2],
        tail_literals_len: block_end - anchor,
    }
}

// Per-tier `#[target_feature]` wrappers around the borrowed dual-base dict
// kernel. Each carries the tier's umbrella so the inlined `_impl` (and the
// `common_prefix_len_ptr` it calls for the hot input-match extension) compiles
// with that tier's SIMD — the 32-byte AVX2 / 16-byte SSE4.2 / NEON /
// wasm-simd128 compare instead of the generic word-at-a-time scalar count.
// Same pattern the Dfast / Row backends use. The dict-prefix 2-segment count
// (cold fallback) stays scalar inside `_impl`.
macro_rules! fast_dict_borrowed_wrapper {
    ($(#[$attr:meta])* $name:ident, $cpl:path) => {
        $(#[$attr])*
        #[allow(clippy::too_many_arguments)]
        unsafe fn $name<const MLS: u32, const USE_CMOV: bool>(
            inp: &[u8],
            dict: &[u8],
            block_start: usize,
            block_end: usize,
            main_table: &mut FastHashTable,
            dict_table: &FastHashTable,
            bounds: PrefixBounds,
            rep: [u32; 2],
            step_size: usize,
            handle_sequence: impl for<'a> FnMut(Sequence<'a>),
        ) -> FastBlockResult {
            compress_block_fast_dict_borrowed_impl::<MLS, USE_CMOV, _>(
                inp,
                dict,
                block_start,
                block_end,
                main_table,
                dict_table,
                bounds,
                rep,
                step_size,
                handle_sequence,
                // SAFETY: only invoked from within this `#[target_feature]`
                // umbrella, so the tier's CPU features are guaranteed present.
                |l, r, m| unsafe { $cpl(l, r, m) },
            )
        }
    };
}

fast_dict_borrowed_wrapper!(
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "avx2,bmi2")]
    cbfd_borrowed_avx2_bmi2,
    crate::encoding::fastpath::avx2_bmi2::common_prefix_len_ptr
);

fast_dict_borrowed_wrapper!(
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "sse4.2")]
    cbfd_borrowed_sse42,
    crate::encoding::fastpath::sse42::common_prefix_len_ptr
);

fast_dict_borrowed_wrapper!(
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    #[target_feature(enable = "neon")]
    cbfd_borrowed_neon,
    crate::encoding::fastpath::neon::common_prefix_len_ptr
);

fast_dict_borrowed_wrapper!(
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    #[target_feature(enable = "simd128")]
    cbfd_borrowed_simd128,
    crate::encoding::fastpath::simd128::common_prefix_len_ptr
);

/// Dispatch the borrowed dual-base dict kernel to the resolved per-tier SIMD
/// wrapper. `kernel` is the matcher's once-resolved [`FastpathKernel`]; the
/// scalar arm carries no `#[target_feature]` (generic word-at-a-time count).
#[allow(clippy::too_many_arguments)]
pub(crate) fn compress_block_fast_dict_borrowed<const MLS: u32, const USE_CMOV: bool>(
    inp: &[u8],
    dict: &[u8],
    block_start: usize,
    block_end: usize,
    main_table: &mut FastHashTable,
    dict_table: &FastHashTable,
    bounds: PrefixBounds,
    rep: [u32; 2],
    step_size: usize,
    handle_sequence: impl for<'a> FnMut(Sequence<'a>),
    kernel: crate::encoding::fastpath::FastpathKernel,
) -> FastBlockResult {
    // Used by the per-tier match arms below; on a target with no SIMD tier
    // (e.g. wasm32 without simd128) only the scalar fallback compiles and the
    // import is unused.
    #[allow(unused_imports)]
    use crate::encoding::fastpath::FastpathKernel;
    // The scalar fallback: generic word-at-a-time `count_forward` for the input
    // match extension, no SIMD umbrella.
    let scalar = |inp: &[u8],
                  dict: &[u8],
                  main_table: &mut FastHashTable,
                  dict_table: &FastHashTable,
                  handle_sequence: &mut dyn for<'a> FnMut(Sequence<'a>)|
     -> FastBlockResult {
        compress_block_fast_dict_borrowed_impl::<MLS, USE_CMOV, _>(
            inp,
            dict,
            block_start,
            block_end,
            main_table,
            dict_table,
            bounds,
            rep,
            step_size,
            handle_sequence,
            // SAFETY: `count_forward` is a safe word-at-a-time count; the
            // pointer/length contract is identical to the SIMD tiers.
            |l, r, m| unsafe { count_forward(l, r, l.add(m)) },
        )
    };
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        match kernel {
            FastpathKernel::Avx2Bmi2 => unsafe {
                cbfd_borrowed_avx2_bmi2::<MLS, USE_CMOV>(
                    inp,
                    dict,
                    block_start,
                    block_end,
                    main_table,
                    dict_table,
                    bounds,
                    rep,
                    step_size,
                    handle_sequence,
                )
            },
            FastpathKernel::Sse42 => unsafe {
                cbfd_borrowed_sse42::<MLS, USE_CMOV>(
                    inp,
                    dict,
                    block_start,
                    block_end,
                    main_table,
                    dict_table,
                    bounds,
                    rep,
                    step_size,
                    handle_sequence,
                )
            },
            FastpathKernel::Scalar => {
                let mut hs = handle_sequence;
                scalar(inp, dict, main_table, dict_table, &mut hs)
            }
        }
    }
    #[cfg(all(target_arch = "aarch64", target_endian = "little"))]
    {
        match kernel {
            FastpathKernel::Neon => unsafe {
                cbfd_borrowed_neon::<MLS, USE_CMOV>(
                    inp,
                    dict,
                    block_start,
                    block_end,
                    main_table,
                    dict_table,
                    bounds,
                    rep,
                    step_size,
                    handle_sequence,
                )
            },
            FastpathKernel::Scalar => {
                let mut hs = handle_sequence;
                scalar(inp, dict, main_table, dict_table, &mut hs)
            }
        }
    }
    #[cfg(all(target_arch = "wasm32", target_feature = "simd128"))]
    {
        match kernel {
            FastpathKernel::Simd128 => unsafe {
                cbfd_borrowed_simd128::<MLS, USE_CMOV>(
                    inp,
                    dict,
                    block_start,
                    block_end,
                    main_table,
                    dict_table,
                    bounds,
                    rep,
                    step_size,
                    handle_sequence,
                )
            },
            FastpathKernel::Scalar => {
                let mut hs = handle_sequence;
                scalar(inp, dict, main_table, dict_table, &mut hs)
            }
        }
    }
    #[cfg(not(any(
        target_arch = "x86",
        target_arch = "x86_64",
        all(target_arch = "aarch64", target_endian = "little"),
        all(target_arch = "wasm32", target_feature = "simd128")
    )))]
    {
        let _ = kernel;
        let mut hs = handle_sequence;
        scalar(inp, dict, main_table, dict_table, &mut hs)
    }
}
