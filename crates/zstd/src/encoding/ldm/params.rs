#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! LDM tuning parameters and upstream zstd-parity defaults.
//!
//! Direct port of `ldmParams_t` (`zstd_compress_internal.h`) and the
//! `ZSTD_ldm_adjustParameters` derivation logic (`zstd_ldm.c:135`),
//! v1.5.7.
//!
//! Upstream zstd flow:
//!
//! 1. Caller starts with a zeroed `ldmParams_t` (all auto-derive).
//! 2. [`LdmParams::adjust_for`] fills the zero-valued knobs from
//!    `windowLog` / `strategy` using the same `BOUNDED` clamps as
//!    upstream zstd.
//! 3. The returned struct then feeds [`super::gear_hash::GearHashState::new`]
//!    and the bucket hash table.

use super::gear_hash::{LDM_BUCKET_SIZE_LOG, LDM_HASH_RLOG, LDM_MIN_MATCH_LENGTH};

/// Upstream zstd `ZSTD_HASHLOG_MIN` (`zstd.h:1268`).
pub(crate) const LDM_HASHLOG_MIN: u32 = 6;
/// Upstream zstd `ZSTD_HASHLOG_MAX` (`zstd.h:1267`). Upstream zstd caps at
/// `min(WINDOWLOG_MAX, 30)`; both Rust and upstream zstd share the `30` cap
/// because no platform we target exceeds it.
pub(crate) const LDM_HASHLOG_MAX: u32 = 30;
/// Upstream zstd `ZSTD_LDM_BUCKETSIZELOG_MAX` (`zstd.h:1300`).
pub(crate) const LDM_BUCKETSIZELOG_MAX: u32 = 8;

/// Upstream zstd strategy ordinal for `ZSTD_btultra`. Triggers the
/// `minMatchLength /= 2` step in `adjust_for`. Upstream zstd `zstd.h`
/// enumerates strategies as 1..=9 in order:
/// fast, dfast, greedy, lazy, lazy2, btlazy2, btopt, btultra, btultra2.
pub(crate) const STRATEGY_BTULTRA: u32 = 8;

/// LDM parameter set fed to both the gear-hash producer and the
/// bucket hash table. Field semantics mirror upstream zstd `ldmParams_t`.
///
/// All-zero `LdmParams::default()` is the upstream zstd sentinel for
/// "auto-derive every knob"; [`Self::adjust_for`] performs that
/// derivation.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LdmParams {
    /// Compression `windowLog` (mirrors `cParams->windowLog`). The
    /// LDM window size in bytes is `1 << window_log`.
    pub(crate) window_log: u32,
    /// `log2` of the LDM hash-table size (in entries). Upstream zstd name:
    /// `hashLog`. Clamped to `[LDM_HASHLOG_MIN, LDM_HASHLOG_MAX]`.
    pub(crate) hash_log: u32,
    /// `log2` of bytes between successive split-point checks (on
    /// average). Upstream zstd name: `hashRateLog`. Defaults derived from
    /// strategy.
    pub(crate) hash_rate_log: u32,
    /// Minimum accepted LDM match length in bytes. Upstream zstd name:
    /// `minMatchLength`. Defaults to [`LDM_MIN_MATCH_LENGTH`], halved
    /// for `>= btultra`.
    pub(crate) min_match_length: u32,
    /// `log2` of the per-bucket entry count. Upstream zstd name:
    /// `bucketSizeLog`. Clamped to
    /// `[LDM_BUCKET_SIZE_LOG, LDM_BUCKETSIZELOG_MAX]`.
    pub(crate) bucket_size_log: u32,
}

impl LdmParams {
    /// Derive a complete parameter set from caller-supplied
    /// `window_log` + `strategy` using upstream zstd `ZSTD_ldm_adjustParameters`
    /// (`zstd_ldm.c:135`). Strategy is the upstream zstd 1..=9 enum ordinal
    /// (fast=1 .. btultra2=9).
    ///
    /// Returns a fully-initialised `LdmParams` with no zero fields.
    pub(crate) fn adjust_for(window_log: u32, strategy: u32) -> Self {
        Self {
            window_log,
            ..Self::default()
        }
        .derive(strategy)
    }

    /// Upstream zstd `ZSTD_ldm_adjustParameters` derivation over a (possibly
    /// caller-seeded) parameter set: fill every zero-valued knob from
    /// `window_log` / `strategy`, leaving non-zero fields untouched.
    ///
    /// This is the path the public-parameter LDM overrides use:
    /// the caller seeds the knobs it wants to pin (the rest stay zero),
    /// and `derive` completes the set with upstream zstd cross-field
    /// consistency (e.g. `hash_rate_log = window_log - hash_log` when a
    /// caller pins `hash_log`). Calling `adjust_for` and then clobbering
    /// fields afterwards would break that consistency.
    pub(crate) fn derive(self, strategy: u32) -> Self {
        // Runtime `assert!` (not `debug_assert!`) — the body uses
        // raw `LDM_HASH_RLOG - (strategy / 3)`, which underflows
        // `u32` for `strategy >= 24` (`7 - 8 = u32::MAX`) and
        // yields nonsensical `hash_rate_log` in release builds.
        // Upstream zstd enforces the same precondition as a hard
        // `assert(1 <= strategy && strategy <= 9)` at
        // `zstd_ldm.c:149` / `zstd_ldm.c:167`; we mirror that
        // hardness in our `pub(crate)` surface. The check runs
        // once per frame so the cost is negligible.
        assert!(
            (1..=9).contains(&strategy),
            "strategy must be a upstream zstd 1..=9 ordinal (upstream zstd asserts at \
             zstd_ldm.c:149 + zstd_ldm.c:167)"
        );

        let mut params = self;

        // hash_rate_log: upstream zstd `zstd_ldm.c:141-153`. With `hash_log`
        // still zero (the only path we expose), upstream zstd falls into the
        // `7 - strategy/3` branch.
        if params.hash_rate_log == 0 {
            if params.hash_log > 0 && params.window_log > params.hash_log {
                params.hash_rate_log = params.window_log - params.hash_log;
            } else {
                // Map [fast=1, rate 7] .. [btultra2=9, rate 4].
                // Upstream zstd `zstd_ldm.c:151`.
                params.hash_rate_log = LDM_HASH_RLOG - (strategy / 3);
            }
        }

        // hash_log: upstream zstd `zstd_ldm.c:154-160`.
        if params.hash_log == 0 {
            params.hash_log = if params.window_log <= params.hash_rate_log {
                LDM_HASHLOG_MIN
            } else {
                bounded(
                    LDM_HASHLOG_MIN,
                    params.window_log - params.hash_rate_log,
                    LDM_HASHLOG_MAX,
                )
            };
        }

        // min_match_length: upstream zstd `zstd_ldm.c:161-165`.
        if params.min_match_length == 0 {
            let mut mml = LDM_MIN_MATCH_LENGTH as u32;
            if strategy >= STRATEGY_BTULTRA {
                mml /= 2;
            }
            params.min_match_length = mml;
        }

        // bucket_size_log: upstream zstd `zstd_ldm.c:166-169`.
        if params.bucket_size_log == 0 {
            params.bucket_size_log = bounded(LDM_BUCKET_SIZE_LOG, strategy, LDM_BUCKETSIZELOG_MAX);
        }

        params
    }

    /// Hash-table size in entries: `1 << hash_log`.
    pub(crate) const fn hash_table_entries(&self) -> usize {
        1usize << self.hash_log
    }

    /// Per-bucket slot count: `1 << bucket_size_log`.
    pub(crate) const fn bucket_slots(&self) -> usize {
        1usize << self.bucket_size_log
    }
}

/// Upstream zstd `BOUNDED(lo, val, hi)` macro: clamp `val` into `[lo, hi]`.
/// Equivalent to `val.clamp(lo, hi)` but kept as a free function so
/// the upstream zstd-line citations read naturally.
#[inline]
const fn bounded(lo: u32, val: u32, hi: u32) -> u32 {
    if val < lo {
        lo
    } else if val > hi {
        hi
    } else {
        val
    }
}
