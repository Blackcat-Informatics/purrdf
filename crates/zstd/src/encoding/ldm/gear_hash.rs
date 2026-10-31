#![allow(
    clippy::unreadable_literal,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Gear rolling hash for LDM split-point detection.
//!
//! Direct port of `ZSTD_ldm_gear_*` from `lib/compress/zstd_ldm.c`
//! v1.5.7. The 256-entry permutation table [`GEAR_TAB`] is reproduced
//! verbatim from `lib/compress/zstd_ldm_geartab.h` so split points
//! computed from any byte stream match the upstream zstd bit-for-bit — a
//! prerequisite for compression-ratio parity.
//!
//! # Algorithm
//!
//! State is a single 64-bit accumulator. For every byte `b` the
//! update is:
//!
//! ```text
//! hash ← (hash << 1) + GEAR_TAB[b]
//! ```
//!
//! Because every entry of the table is a fixed `u64` independent of
//! position, bit `n` of `hash` after `k` updates depends on the last
//! `min(n, k)` bytes — a content-defined window without any explicit
//! ring buffer. A "split point" is signalled whenever
//! `(hash & stop_mask) == 0`. Upstream zstd [`init`] derives `stop_mask` so
//! that on average one split occurs every `2 ^ hash_rate_log` bytes,
//! and the active bits sit at the high end of the rolling window
//! (`min(min_match_length, 64)` bits) — biasing splits toward
//! content-defined boundaries of at least `min_match_length` bytes
//! whenever the parameter ranges allow it.
//!
//! # Upstream zstd anchors
//!
//! * `ZSTD_ldm_gear_init`  → [`GearHashState::new`]
//! * `ZSTD_ldm_gear_reset` → [`reset`]
//! * `ZSTD_ldm_gear_feed`  → [`feed`]
//!
//! # Constants
//!
//! * [`LDM_BUCKET_SIZE_LOG`] — upstream zstd `#define` (= 4, the default /
//!   lower bound used by `LdmParams::adjust_for`'s `BOUNDED`
//!   clamp; the upstream zstd upper bound is the separate
//!   `ZSTD_LDM_BUCKETSIZELOG_MAX = 8` constant, exposed as
//!   `params::LDM_BUCKETSIZELOG_MAX`).
//! * [`LDM_MIN_MATCH_LENGTH`] — upstream zstd `#define`.
//! * [`LDM_HASH_RLOG`] — upstream zstd `#define` (default hash-rate log).
//! * [`LDM_BATCH_SIZE`] — upstream zstd `#define` from
//!   `zstd_compress_internal.h`. Caps the per-call split count so the
//!   caller's `splits` buffer never overflows.

/// Bucket size log for the LDM hash table.
///
/// Upstream zstd: `lib/compress/zstd_ldm.c:19` (`#define LDM_BUCKET_SIZE_LOG 4`).
pub(crate) const LDM_BUCKET_SIZE_LOG: u32 = 4;

/// Default minimum LDM match length in bytes.
///
/// Upstream zstd: `lib/compress/zstd_ldm.c:20`
/// (`#define LDM_MIN_MATCH_LENGTH 64`). Defines both the gear-hash
/// window depth (and therefore the maximum bit weight used for the
/// `stop_mask` placement) and the floor on accepted match lengths.
pub(crate) const LDM_MIN_MATCH_LENGTH: usize = 64;

/// Default hash-rate log (one split every `2 ^ LDM_HASH_RLOG` bytes
/// on average).
///
/// Upstream zstd: `lib/compress/zstd_ldm.c:21` (`#define LDM_HASH_RLOG 7`).
pub(crate) const LDM_HASH_RLOG: u32 = 7;

/// Maximum number of splits the upstream zstd `gear_feed` produces per call.
///
/// Upstream zstd: `lib/compress/zstd_compress_internal.h:335`
/// (`#define LDM_BATCH_SIZE 64`). The caller-owned `splits` array
/// must hold at least this many entries.
pub(crate) const LDM_BATCH_SIZE: usize = 64;

/// Initial rolling-hash value used by upstream zstd `ZSTD_ldm_gear_init`:
/// `state->rolling = ~(U32)0;` — the low 32 bits set, high 32 bits
/// zero. Splitting this out so callers can re-seed mid-stream
/// (mirroring upstream zstd's behaviour at frame / block boundaries).
pub(crate) const GEAR_HASH_INIT: u64 = 0xFFFF_FFFF;

/// Gear rolling-hash state — `(rolling, stop_mask)`.
///
/// Mirrors upstream zstd `ldmRollingHashState_t` from
/// `lib/compress/zstd_ldm.c:23-26`. Kept `Copy` so the per-block
/// state can be cheaply snapshotted before a speculative scan.
#[derive(Copy, Clone, Debug)]
pub(crate) struct GearHashState {
    /// 64-bit rolling accumulator. Updated as `hash = (hash << 1) +
    /// GEAR_TAB[byte]` for every input byte.
    pub(crate) rolling: u64,
    /// Upstream zstd `stopMask`. A split is registered when
    /// `(rolling & stop_mask) == 0`.
    pub(crate) stop_mask: u64,
}

impl GearHashState {
    /// Build a fresh state for the requested LDM parameters.
    ///
    /// Upstream zstd: `ZSTD_ldm_gear_init` (`zstd_ldm.c:32`). `min_match_length`
    /// is clamped to `64` (the upstream zstd uses
    /// `MIN(params->minMatchLength, 64)` because the hash window can
    /// expose at most 64 useful bits) and the resulting mask sits at
    /// the high end of that window when `hash_rate_log` fits — see
    /// the module-level docs.
    pub(crate) fn new(min_match_length: usize, hash_rate_log: u32) -> Self {
        let max_bits_in_mask = min_match_length.min(64) as u32;
        // Defensive clamp to 63: the degenerate-path shift
        // `1u64 << hash_rate_log` would panic at 64. Every
        // production caller routes through `LdmParams::adjust_for`
        // (`hash_rate_log = LDM_HASH_RLOG - strategy/3` → 4..7) or
        // `window_log - hash_log` (upstream zstd-bounded to 0..27), so the
        // clamp is unreachable today; it guards against future
        // callers / param-API misuse picking up the function
        // directly.
        let hash_rate_log = hash_rate_log.min(63);
        let stop_mask = if hash_rate_log > 0 && hash_rate_log <= max_bits_in_mask {
            ((1u64 << hash_rate_log) - 1) << (max_bits_in_mask - hash_rate_log)
        } else {
            // Degenerate path: simply honour the requested hash rate
            // without trying to bias toward `min_match_length` bits.
            // Upstream zstd `zstd_ldm.c:56`.
            (1u64 << hash_rate_log) - 1
        };
        Self {
            rolling: GEAR_HASH_INIT,
            stop_mask,
        }
    }
}

/// Feed `min_match_length` bytes through the rolling hash WITHOUT
/// emitting any splits.
///
/// Upstream zstd: `ZSTD_ldm_gear_reset` (`zstd_ldm.c:65`). Used at block
/// boundaries and after a skip so the rolling window is primed
/// against the new context before split detection resumes.
///
/// `min_match_length` is named after the parameter that controls it
/// in the upstream zstd — the actual length consumed is `data.len()` capped
/// implicitly by the caller's bounds (upstream zstd requires
/// `data.len() >= min_match_length`; this Rust port lets the caller
/// pass an arbitrary slice for unit testing and skip flexibility).
pub(crate) fn reset(state: &mut GearHashState, data: &[u8]) {
    let mut hash = state.rolling;
    let mut n = 0;
    // Upstream zstd unrolls the inner loop four-wide; we keep the same shape
    // so the bounds check fires identically and the per-iteration
    // arithmetic monomorphises to the same instruction sequence on
    // every backend.
    while n + 3 < data.len() {
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n] as usize]);
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n + 1] as usize]);
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n + 2] as usize]);
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n + 3] as usize]);
        n += 4;
    }
    while n < data.len() {
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n] as usize]);
        n += 1;
    }
    state.rolling = hash;
}

/// Feed `data` through the rolling hash, recording every position
/// where `(rolling & stop_mask) == 0` into `splits` until either the
/// data is exhausted or `splits.len()` reaches [`LDM_BATCH_SIZE`].
///
/// Each entry written to `splits` is a 1-based offset *into `data`*
/// (the byte index just past the split), matching the upstream zstd's
/// `splits[*numSplits] = n` semantics where `n` is the post-increment
/// byte count.
///
/// Returns `(bytes_consumed, splits_written)`. The caller is expected
/// to advance its input cursor by `bytes_consumed` and re-call when
/// the previous batch has been drained — exactly the upstream zstd's outer
/// loop in `ZSTD_ldm_generateSequences_internal`.
///
/// # Panics
///
/// Panics if `splits` is shorter than [`LDM_BATCH_SIZE`] — the upstream zstd
/// pre-condition is that the array has at least `LDM_BATCH_SIZE` slots
/// reserved, and feeding into a shorter buffer would silently drop
/// splits.
pub(crate) fn feed(state: &mut GearHashState, data: &[u8], splits: &mut [usize]) -> (usize, usize) {
    assert!(
        splits.len() >= LDM_BATCH_SIZE,
        "splits buffer must hold at least LDM_BATCH_SIZE entries \
         (upstream zstd pre-condition: zstd_ldm.c:96)"
    );
    let mut hash = state.rolling;
    let mask = state.stop_mask;
    let mut n: usize = 0;
    let mut num_splits: usize = 0;

    // 4-wide unrolled head. Upstream zstd `zstd_ldm.c:118-123`. Each iteration
    // increments `n` first, then tests the post-update hash so the
    // split index points to the byte AFTER the matched window — this
    // matches `splits[*numSplits] = n` in upstream zstd (n already
    // incremented).
    while n + 3 < data.len() {
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n] as usize]);
        n += 1;
        if hash & mask == 0 {
            splits[num_splits] = n;
            num_splits += 1;
            if num_splits == LDM_BATCH_SIZE {
                state.rolling = hash;
                return (n, num_splits);
            }
        }
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n] as usize]);
        n += 1;
        if hash & mask == 0 {
            splits[num_splits] = n;
            num_splits += 1;
            if num_splits == LDM_BATCH_SIZE {
                state.rolling = hash;
                return (n, num_splits);
            }
        }
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n] as usize]);
        n += 1;
        if hash & mask == 0 {
            splits[num_splits] = n;
            num_splits += 1;
            if num_splits == LDM_BATCH_SIZE {
                state.rolling = hash;
                return (n, num_splits);
            }
        }
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n] as usize]);
        n += 1;
        if hash & mask == 0 {
            splits[num_splits] = n;
            num_splits += 1;
            if num_splits == LDM_BATCH_SIZE {
                state.rolling = hash;
                return (n, num_splits);
            }
        }
    }
    // Tail loop for the remaining 0..3 bytes. Upstream zstd `zstd_ldm.c:124-126`.
    while n < data.len() {
        hash = (hash << 1).wrapping_add(GEAR_TAB[data[n] as usize]);
        n += 1;
        if hash & mask == 0 {
            splits[num_splits] = n;
            num_splits += 1;
            if num_splits == LDM_BATCH_SIZE {
                state.rolling = hash;
                return (n, num_splits);
            }
        }
    }
    state.rolling = hash;
    (n, num_splits)
}

/// 256-entry random-permutation table from upstream zstd
/// `lib/compress/zstd_ldm_geartab.h`. Reproduced verbatim — DO NOT
/// REGENERATE: byte-parity of split points (and therefore LDM ratio
/// parity vs upstream) depends on every entry matching the upstream zstd
/// value exactly.
#[rustfmt::skip]
pub(crate) const GEAR_TAB: [u64; 256] = [
    0xf5b8f72c5f77775c, 0x84935f266b7ac412, 0xb647ada9ca730ccc,
    0xb065bb4b114fb1de, 0x34584e7e8c3a9fd0, 0x4e97e17c6ae26b05,
    0x3a03d743bc99a604, 0xcecd042422c4044f, 0x76de76c58524259e,
    0x9c8528f65badeaca, 0x86563706e2097529, 0x2902475fa375d889,
    0xafb32a9739a5ebe6, 0xce2714da3883e639, 0x021eaf821722e69e,
    0x0037b628620b628,  0x049a8d455d88caf5, 0x8556d711e6958140,
    0x04f7ae74fc605c1f, 0x829f0c3468bd3a20, 0x4ffdc885c625179e,
    0x8473de048a3daf1b, 0x51008822b05646b2, 0x69d75d12b2d1cc5f,
    0x8c9d4a19159154bc, 0xc3cc10f4abbd4003, 0xd06ddc1cecb97391,
    0xbe48e6e7ed80302e, 0x3481db31cee03547, 0xacc3f67cdaa1d210,
    0x65cb771d8c7f96cc, 0x8eb27177055723dd, 0xc789950d44cd94be,
    0x934feadc3700b12b, 0x5e485f11edbdf182, 0x1e2e2a46fd64767a,
    0x2969ca71d82efa7c, 0x9d46e9935ebbba2e, 0xe056b67e05e6822b,
    0x94d73f55739d03a0, 0xcd7010bdb69b5a03, 0x455ef9fcd79b82f4,
    0x869cb54a8749c161, 0x38d1a4fa6185d225, 0xb475166f94bbe9bb,
    0xa4143548720959f1, 0x7aed4780ba6b26ba, 0xd0ce264439e02312,
    0x84366d746078d508, 0xa8ce973c72ed17be, 0x21c323a29a430b01,
    0x9962d617e3af80ee, 0xab0ce91d9c8cf75b, 0x530e8ee6d19a4dbc,
    0x2ef68c0cf53f5d72, 0xc03a681640a85506, 0x496e4e9f9c310967,
    0x78580472b59b14a0, 0x273824c23b388577, 0x66bf923ad45cb553,
    0x47ae1a5a2492ba86, 0x35e304569e229659, 0x4765182a46870b6f,
    0x6cbab625e9099412, 0xddac9a2e598522c1, 0x7172086e666624f2,
    0xdf5003ca503b7837, 0x88c0c1db78563d09, 0x58d51865acfc289d,
    0x177671aec65224f1, 0xfb79d8a241e967d7, 0x2be1e101cad9a49a,
    0x6625682f6e29186b, 0x399553457ac06e50, 0x035dffb4c23abb74,
    0x429db2591f54aade, 0xc52802a8037d1009, 0x6acb27381f0b25f3,
    0xf45e2551ee4f823b, 0x8b0ea2d99580c2f7, 0x3bed519cbcb4e1e1,
    0x0ff452823dbb010a, 0x9d42ed614f3dd267, 0x5b9313c06257c57b,
    0xa114b8008b5e1442, 0xc1fe311c11c13d4b, 0x66e8763ea34c5568,
    0x8b982af1c262f05d, 0xee8876faaa75fbb7, 0x8a62a4d0d172bb2a,
    0xc13d94a3b7449a97, 0x6dbbba9dc15d037c, 0xc786101f1d92e0f1,
    0xd78681a907a0b79b, 0xf61aaf2962c9abb9, 0x2cfd16fcd3cb7ad9,
    0x868c5b6744624d21, 0x25e650899c74ddd7, 0xba042af4a7c37463,
    0x4eb1a539465a3eca, 0xbe09dbf03b05d5ca, 0x774e5a362b5472ba,
    0x47a1221229d183cd, 0x504b0ca18ef5a2df, 0xdffbdfbde2456eb9,
    0x46cd2b2fbee34634, 0xf2aef8fe819d98c3, 0x357f5276d4599d61,
    0x24a5483879c453e3, 0x088026889192b4b9, 0x28da96671782dbec,
    0x4ef37c40588e9aaa, 0x8837b90651bc9fb3, 0xc164f741d3f0e5d6,
    0xbc135a0a704b70ba, 0x069cd868f7622ada, 0xbc37ba89e0b9c0ab,
    0x47c14a01323552f6, 0x4f00794bacee98bb, 0x7107de7d637a69d5,
    0x88af793bb6f2255e, 0xf3c6466b8799b598, 0xc288c616aa7f3b59,
    0x81ca63cf42fca3fd, 0x88d85ace36a2674b, 0x0d056bd3792389e7,
    0xe55c396c4e9dd32d, 0xbefb504571e6c0a6, 0x96ab32115e91e8cc,
    0xbf8acb18de8f38d1, 0x66dae58801672606, 0x833b6017872317fb,
    0xb87c16f2d1c92864, 0xdb766a74e58b669c, 0x89659f85c61417be,
    0xc8daad856011ea0c, 0x76a4b565b6fe7eae, 0xa469d085f6237312,
    0xaaf0365683a3e96c, 0x4dbb746f8424f7b8, 0x0638755af4e4acc1,
    0x3d7807f5bde64486, 0x17be6d8f5bbb7639, 0x0903f0cd44dc35dc,
    0x67b672eafdf1196c, 0xa676ff93ed4c82f1, 0x521d1004c5053d9d,
    0x37ba9ad09ccc9202, 0x84e54d297aacfb51, 0x0a0b4b776a143445,
    0x0820d471e20b348e, 0x1874383cb83d46dc, 0x97edeec7a1efe11c,
    0xb330e50b1bdc42aa, 0x1dd91955ce70e032, 0xa514cdb88f2939d5,
    0x2791233fd90db9d3, 0x7b670a4cc50f7a9b, 0x77c07d2a05c6dfa5,
    0xe3778b6646d0a6fa, 0xb39c8eda47b56749, 0x933ed448addbef28,
    0xaf846af6ab7d0bf4, 0x0e5af208eb666e49, 0x5e6622f73534cd6a,
    0x297daeca42ef5b6e, 0x862daef3d35539a6, 0xe68722498f8e1ea9,
    0x981c53093dc0d572, 0xfa09b0bfbf86fbf5, 0x30b1e96166219f15,
    0x70e7d466bdc4fb83, 0x5a66736e35f2a8e9, 0xcddb59d2b7c1baef,
    0xd6c7d247d26d8996, 0xea4e39eac8de1ba3, 0x539c8bb19fa3aff2,
    0x09f90e4c5fd508d8, 0xa34e5956fbaf3385, 0x2e2f8e151d3ef375,
    0x173691e9b83faec1, 0xb85a8d56bf016379, 0x8382381267408ae3,
    0xb90f901bbdc0096d, 0x7c6ad32933bcec65, 0x76bb5e2f2c8ad595,
    0x390f851a6cf46d28, 0xc3e6064da1c2da72, 0xc52a0c101cfa5389,
    0xd78eaf84a3fbc530, 0x3781b9e2288b997e, 0x73c2f6dea83d05c4,
    0x04228e364c5b5ed7, 0x9d7a3edf0da43911, 0x8edcfeda24686756,
    0x5e7667a7b7a9b3a1, 0x4c4f389fa143791d, 0xb08bc1023da7cddc,
    0x7ab4be3ae529b1cc, 0x754e6132dbe74ff9, 0x71635442a839df45,
    0x2f6fb1643fbe52de, 0x961e0a42cf7a8177, 0xf3b45d83d89ef2ea,
    0xee3de4cf4a6e3e9b, 0xcd6848542c3295e7, 0xe4cee1664c78662f,
    0x9947548b474c68c4, 0x25d73777a5ed8b0b, 0x00c915b1d636b7fc,
    0x21c2ba75d9b0d2da, 0x5f6b5dcf608a64a1, 0xdcf333255ff9570c,
    0x633b922418ced4ee, 0xc136dde0b004b34a, 0x58cc83b05d4b2f5a,
    0x5eb424dda28e42d2, 0x62df47369739cd98, 0xb4e0b42485e4ce17,
    0x16e1f0c1f9a8d1e7, 0x8ec3916707560ebf, 0x62ba6e2df2cc9db3,
    0xcbf9f4ff77d83a16, 0x78d9d7d07d2bbcc4, 0xef554ce1e02c41f4,
    0x8d7581127eccf94d, 0xa9b53336cb3c8a05, 0x38c42c0bf45c4f91,
    0x640893cdf4488863, 0x80ec34bc575ea568, 0x39f324f5b48eaa40,
    0xe9d9ed1f8eff527f, 0x9224fc058cc5a214, 0xbaba00b04cfe7741,
    0x309a9f120fcf52af, 0xa558f3ec65626212, 0x424bec8b7adabe2f,
    0x41622513a6aea433, 0xb88da2d5324ca798, 0xd287733b245528a4,
    0x9a44697e6d68aec3, 0x7b1093be2f49bb28, 0x50bbec632e3d8aad,
    0x6cd90723e1ea8283, 0x897b9e7431b02bf3, 0x219efdcb338a7047,
    0x3b0311f0a27c0656, 0xdb17bf91c0db96e7, 0x8cd4fd6b4e85a5b2,
    0xfab071054ba6409d, 0x40d6fe831fa9dfd9, 0xaf358debad7d791e,
    0xeb8d0e25a65e3e58, 0xbbcbd3df14e08580, 0x0cf751f27ecdab2b,
    0x2b4da14f2613d8f4,
];
