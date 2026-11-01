#![allow(
    clippy::inline_always,
    clippy::unreadable_literal,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Flat `Vec<u32>` hash table used by the upstream zstd-shape Fast strategy
//! match-finder. Direct port of `ZSTD_hash4`/`ZSTD_hash5`/`ZSTD_hash6`/
//! `ZSTD_hash7`/`ZSTD_hash8` from
//! `lib/compress/zstd_compress_internal.h` — multiply-shift on the first
//! `mls` bytes of the suffix at `ptr`, keyed into a power-of-two table
//! sized `1 << hash_log` entries.

use alloc::vec;
use alloc::vec::Vec;

/// Upstream zstd `ZSTD_HASHLOG_MAX` (`lib/zstd.h`). The cap applies uniformly
/// across all five `mls` instantiations (`mls ∈ {4, 5, 6, 7, 8}`): even
/// though `mls >= 5` widens the hash to a `u64` reduction, the Fast
/// strategy's per-level `hashLog` is sourced from the upstream zstd's
/// `ZSTD_defaultCParameters` table where the maximum is `14` (level 1,
/// `srcSize > 256 KB`), and the user-tunable upper bound is `30`.
/// Enforcing this in the constructor catches misuse before the first
/// `hash_ptr` would otherwise panic on the `(32 - hash_log)` /
/// `(64 - hash_log)` shift.
const ZSTD_HASHLOG_MAX: u32 = 30;

/// Upstream zstd multiplicative hash constants — exact bit-for-bit match with
/// `lib/compress/zstd_compress_internal.h` so the table-keying behaviour
/// stays identical to the reference encoder.
/// Non-allocating parameter validation shared by [`FastHashTable::new`] and
/// the constructor-accept-path tests. Extracted so tests can prove the
/// `(1..=ZSTD_HASHLOG_MAX, 4..=8)` accept band without forcing a
/// `1 << ZSTD_HASHLOG_MAX` allocation (≈4 GiB at `ZSTD_HASHLOG_MAX = 30`,
/// well above per-test memory budgets on CI runners). Panics with the
/// same messages the [`FastHashTable::new`] doc-comment cites.
fn validate_params(hash_log: u32, mls: u32) {
    assert!(
        (1..=ZSTD_HASHLOG_MAX).contains(&hash_log),
        "hash_log must be in 1..={ZSTD_HASHLOG_MAX} for upstream zstd-compatible Fast hashing (got {hash_log}); \
         the lower bound prevents a full-word-width shift in hash_ptr, the upper bound is upstream zstd's ZSTD_HASHLOG_MAX",
    );
    assert!(
        (4..=8).contains(&mls),
        "ZSTD Fast strategy only supports mls 4..=8 (got {mls})",
    );
}

const PRIME_4_BYTES: u32 = 0x9E3779B1;
const PRIME_5_BYTES: u64 = 889_523_592_379;
const PRIME_6_BYTES: u64 = 227_718_039_650_203;
const PRIME_7_BYTES: u64 = 58_295_818_150_454_627;
const PRIME_8_BYTES: u64 = 0xCF1BBCDCB7A56463;

/// Flat hash table indexed by `hash_ptr(ptr, hash_log, mls)`. Entries
/// store absolute positions into the encoder's flat history buffer
/// (matches upstream zstd's `U32* hashTable` with `base + matchIdx` lookup).
/// Sentinel `0` is fine because position `0` either belongs to the
/// initial prefix (where the `+= (ip0 == prefixStart)` adjustment at
/// loop entry skips it) or is below `prefixStartIndex` and filtered by
/// the in-range check.
pub(crate) struct FastHashTable {
    table: Vec<u32>,
    /// Upstream zstd `hash_log` — number of bits the hash output is reduced to.
    hash_log: u32,
    /// Upstream zstd `mls` — minimum match length used as the hash input width.
    /// Valid range `4..=8`; the kernel monomorphises over this so it
    /// compiles to a constant inside each instantiation.
    mls: u32,
    /// Epoch bias for continue-mode frame resets (upstream zstd `ZSTD_continueCCtx`
    /// cadence): stored values are `position + bias`, and [`Self::get`]
    /// reads any entry below the current bias as the empty sentinel `0`.
    /// Advancing the bias past every previously-stored value
    /// ([`Self::advance_epoch`]) therefore invalidates the whole table
    /// without the per-frame full-table memset of [`Self::clear`].
    ///
    /// Always `0` on paths that access the table storage directly
    /// ([`Self::hot_state`] — the no-dict kernels) and on cached dict
    /// tables; only the dict-attach main table advances it.
    bias: u32,
}

impl Clone for FastHashTable {
    fn clone(&self) -> Self {
        Self {
            table: self.table.clone(),
            hash_log: self.hash_log,
            mls: self.mls,
            bias: self.bias,
        }
    }

    // Real buffer reuse: the per-frame dictionary snapshot restore
    // `clone_from`s the whole matcher, and the table is its dominant
    // allocation — copying into the retained buffer avoids a fresh
    // table-sized allocation per frame.
    fn clone_from(&mut self, source: &Self) {
        self.table.clone_from(&source.table);
        self.hash_log = source.hash_log;
        self.mls = source.mls;
        self.bias = source.bias;
    }
}

impl FastHashTable {
    /// Allocate the table at `1 << hash_log` entries, all initialised
    /// to the sentinel `0` position. The encoder is expected to bump
    /// the first real input position to at least `1` so the sentinel
    /// can never be confused with a valid match (the upstream zstd achieves
    /// this via `ip0 += (ip0 == prefixStart)`).
    ///
    /// # Panics
    ///
    /// Parameter-range failures:
    /// - `hash_log` outside `1..=ZSTD_HASHLOG_MAX` (upstream zstd's cap,
    ///   currently `30`). The lower bound exists because `0` would
    ///   make `hash_ptr` shift by the full word width (`32` for
    ///   mls=4, `64` for mls≥5) — UB / panic in Rust. The upper
    ///   bound is the upstream zstd's documented maximum; importantly,
    ///   even on 64-bit targets a `usize::BITS - 1` cap would still
    ///   admit `hash_log ∈ 33..=63` which is invalid for the
    ///   `mls=4` path that shifts by `32 - hash_log` (panics for
    ///   `hash_log >= 32`). Pinning to `ZSTD_HASHLOG_MAX` rejects
    ///   both invalid bands at construction time so every
    ///   subsequent `hash_ptr::<MLS>` call is safe by construction.
    /// - `mls` outside `4..=8`.
    ///
    /// Target-size / allocation failures (per-host, not per-input):
    /// - `1usize << hash_log` overflowing `usize`. Only reachable on
    ///   32-bit hosts at `hash_log >= 32` — but `validate_params`
    ///   already pins `hash_log <= ZSTD_HASHLOG_MAX = 30`, so this
    ///   path is unreachable today for the upstream zstd-compatible band.
    ///   Kept here as a tripwire if `ZSTD_HASHLOG_MAX` is ever
    ///   raised past `31`.
    /// - `entries * size_of::<u32>()` overflowing `usize`. This is
    ///   the deterministic 32-bit failure mode at `hash_log = 30`:
    ///   `1 << 30` entries × 4 bytes = 4 GiB, which is the full
    ///   32-bit address space and overflows the `usize` multiply
    ///   before `vec![]` is even called. The `checked_mul` guard
    ///   surfaces this as a clear panic instead of the opaque
    ///   `Vec`-internal capacity-overflow message.
    /// - Global allocator failure when actually allocating the
    ///   table backing storage — propagates as the standard
    ///   `Vec::with_capacity` allocation-failure panic.
    ///
    /// The first two guards fire BEFORE control reaches `vec![]`
    /// and are deterministic given the inputs and target
    /// architecture. The third bullet IS the `vec![]` allocation
    /// itself — it's the only panic that depends on runtime memory
    /// state.
    pub(crate) fn new(hash_log: u32, mls: u32) -> Self {
        validate_params(hash_log, mls);
        // Per-target allocation feasibility: `1 << hash_log` u32 entries
        // = `1 << (hash_log + 2)` bytes. On 32-bit hosts that overflows
        // `usize` at `hash_log >= 30` (4 GiB exceeds the address space).
        // `validate_params` already pins `hash_log <= ZSTD_HASHLOG_MAX
        // = 30`, but on 32-bit the maximum that actually fits is `<=
        // 29` (2 GiB) — anything larger panics deep inside `Vec::with_
        // capacity` with a generic allocation message. Surface a clear
        // panic at construction so the failure mode is obvious instead.
        let entries = 1usize.checked_shl(hash_log).unwrap_or_else(|| {
            panic!(
                "FastHashTable cannot allocate 2^{hash_log} u32 entries on this target: \
                 `1usize << {hash_log}` overflows {0}-bit usize",
                usize::BITS,
            )
        });
        let bytes = entries.checked_mul(size_of::<u32>()).unwrap_or_else(|| {
            panic!(
                "FastHashTable cannot allocate {entries} u32 entries on this target: \
                 byte size overflows {0}-bit usize",
                usize::BITS,
            )
        });
        // Use `bytes` to compute as a tripwire — actual allocation
        // still goes through `vec![]` so the global allocator picks
        // the strategy (zeroed page mapping, etc.).
        let _ = bytes;
        Self {
            table: vec![0u32; entries],
            hash_log,
            mls,
            bias: 0,
        }
    }

    /// Construct without allocating the entry storage. Records the requested
    /// `(hash_log, mls)` and validates them (so the deferred allocation is
    /// feasible), but leaves `table` empty. The first per-frame reset sees the
    /// source-size-clamped params and allocates once at the final size via
    /// [`Self::new`] — avoiding the construct-at-level-default-then-realloc
    /// churn (upstream zstd allocates its cwksp lazily at the resolved
    /// `hashLog` for the same reason). [`Self::is_allocated`] reports whether
    /// the storage exists yet; the matcher's reset path always allocates
    /// before the kernel runs, so the hot path never observes the empty state.
    pub(crate) fn new_deferred(hash_log: u32, mls: u32) -> Self {
        validate_params(hash_log, mls);
        Self {
            table: Vec::new(),
            hash_log,
            mls,
            bias: 0,
        }
    }

    /// Whether the entry storage has been allocated. `false` only for a
    /// [`Self::new_deferred`] table that has not yet been allocated by a reset.
    #[inline(always)]
    pub(crate) fn is_allocated(&self) -> bool {
        !self.table.is_empty()
    }

    #[inline(always)]
    pub(crate) fn hash_log(&self) -> u32 {
        self.hash_log
    }

    #[inline(always)]
    pub(crate) fn mls(&self) -> u32 {
        self.mls
    }

    /// Heap bytes held by the table's `Vec<u32>` (its allocated capacity).
    pub(crate) fn heap_size(&self) -> usize {
        self.table.capacity() * size_of::<u32>()
    }

    /// Clear the table back to all-sentinel. Used on encoder reset
    /// between independent frames so a stale absolute index from the
    /// previous frame can't get mistaken for a current-frame match.
    pub(crate) fn clear(&mut self) {
        // `fill(0)` lowers to a single `memset` and is significantly
        // faster than re-allocating; the table can be hundreds of KiB.
        self.table.fill(0);
        self.bias = 0;
    }

    /// Continue-mode frame reset (upstream zstd `ZSTD_continueCCtx` cadence): keep
    /// the table contents and advance the epoch bias past every entry the
    /// previous frame stored, so all of them read back as the empty
    /// sentinel via [`Self::get`]'s epoch filter — no full-table memset.
    ///
    /// `span` must be strictly greater than the largest unbiased position
    /// stored since the last clear/advance (the caller passes its history
    /// high-water mark). Falls back to a real [`Self::clear`] when the
    /// biased position space would no longer fit `u32`.
    pub(crate) fn advance_epoch(&mut self, span: u32) {
        // Stored positions are bounded by the eviction band (2 * max
        // window = 2^31, see the matcher's `window_log <= 30` ceiling),
        // so a bias at or below `u32::MAX - 2^31` can never overflow in
        // `put`.
        const POSITION_CEILING: u32 = 1 << 31;
        match self.bias.checked_add(span) {
            Some(new_bias) if new_bias <= u32::MAX - POSITION_CEILING => self.bias = new_bias,
            _ => self.clear(),
        }
    }

    /// Upstream zstd-parity `ZSTD_hashPtr` — multiply-shift hash over the first
    /// `mls` bytes at `ptr`, output reduced to `hash_log` bits.
    ///
    /// # Safety
    ///
    /// **Readable-bytes contract on `ptr`:**
    /// - `MLS == 4`: at least **4** readable bytes (a `u32` load).
    /// - `MLS >= 5`: at least **8** readable bytes — every mls ∈ {5,
    ///   6, 7, 8} path performs an unaligned `u64::read_unaligned`
    ///   and shifts off the unused top bits, so the underlying load
    ///   is always 8 bytes wide regardless of `mls`. Promising only
    ///   `mls` readable bytes for `mls ∈ {5,6,7}` would leave the
    ///   trailing 8-mls bytes of the u64 read past the caller's
    ///   range — UB.
    ///
    /// The kernel satisfies the readable-bytes promise uniformly
    /// via the `ilimit = iend - HASH_READ_SIZE` cap
    /// (`HASH_READ_SIZE = 8`), mirroring upstream zstd's same invariant.
    ///
    /// **`MLS` const-generic contract:**
    /// - `MLS` MUST equal `self.mls()`.
    /// - `MLS` MUST be in `4..=8`.
    ///
    /// Today only `debug_assert_eq!` checks the equality at runtime,
    /// so in release a mismatch silently routes to the wrong hash
    /// formula (different multiply prime, different shift width)
    /// and probes entries indexed by a different formula — garbage
    /// match candidates. Callers must guarantee both invariants
    /// before invoking `hash_ptr`. The crate-internal entry point
    /// [`crate::encoding::simple::fast_kernel::kernel::compress_block_fast`]
    /// enforces both via real `assert!`s before any `hash_ptr` call,
    /// so invocation through the kernel is safe by construction;
    /// direct callers (tests, future helpers) must uphold the
    /// contract themselves.
    #[inline(always)]
    pub(crate) unsafe fn hash_ptr<const MLS: u32>(&self, ptr: *const u8) -> u32 {
        debug_assert_eq!(MLS, self.mls, "monomorphised MLS must match table mls");
        // SAFETY: forwarded — caller upholds `hash_ptr`'s readable-bytes
        // contract; `self.hash_log` is the table's own log.
        unsafe { hash_ptr_raw::<MLS>(ptr, self.hash_log) }
    }

    /// Hoist the table's backing slice + `hash_log` into locals for a hot
    /// loop. Binding `&mut [u32]` to a local caches the `(ptr, len)` once, so
    /// per-position `get_unchecked` / `get_unchecked_mut` don't reload the
    /// `Vec` header on every access — the optimiser otherwise conservatively
    /// re-reads it through `&mut FastHashTable` after each interior write
    /// (the "chases the Vec" reload). Pair with [`hash_ptr_raw`] so the loop
    /// never touches `self` and stays reload-free.
    #[inline(always)]
    pub(crate) fn hot_state(&mut self) -> (&mut [u32], u32) {
        // The raw-slice consumers (no-dict kernels) store and read
        // UNBIASED positions; they may only run on a bias-0 table (the
        // matcher clears — rather than epoch-advances — whenever the next
        // frame is not a dict-attach frame).
        debug_assert_eq!(self.bias, 0, "hot_state requires an unbiased table");
        (self.table.as_mut_slice(), self.hash_log)
    }

    /// Like [`hot_state`] but also exposes the epoch `bias`, so a hot loop on a
    /// POSSIBLY-biased table (the dict-attach kernels) can hoist the backing
    /// slice + `hash_log` and apply the bias inline — `slot.saturating_sub(bias)`
    /// on read, `pos + bias` on write — exactly as [`get`]/[`put`] do, without
    /// re-reading the `Vec` header / `hash_log` / `bias` through `&mut self` on
    /// every access. On a bias-0 table this is identical to `hot_state` + raw
    /// access (`saturating_sub(0)` / `+ 0` fold away).
    #[inline(always)]
    pub(crate) fn hot_state_biased(&mut self) -> (&mut [u32], u32, u32) {
        (self.table.as_mut_slice(), self.hash_log, self.bias)
    }

    /// Direct table access — `table[hash]`. Bounds-check at index time
    /// is provably redundant because `hash >> (64 - hash_log)` produces
    /// a value `< 1 << hash_log == table.len()`; LLVM cannot infer
    /// this across the `as u32` truncation so we use `get_unchecked`.
    ///
    /// # Safety
    ///
    /// `hash` MUST be a value returned by [`hash_ptr`] on this table
    /// (or on another table with the same `hash_log`), so that
    /// `hash < 1 << hash_log = table.len()`.
    #[inline(always)]
    pub(crate) unsafe fn get(&self, hash: u32) -> u32 {
        debug_assert!((hash as usize) < self.table.len());
        // SAFETY: see method-level doc — `hash` is bounded by the
        // table-size invariant from `hash_ptr`.
        let raw = unsafe { *self.table.get_unchecked(hash as usize) };
        // Epoch filter: entries stored before the last `advance_epoch`
        // (raw < bias, including the all-zero sentinel) must read as the
        // empty sentinel 0 — the saturation floor IS the semantics here.
        // Compiles to sub + cmov; on bias == 0 tables (no-dict, cached
        // dict) it is the identity.
        raw.saturating_sub(self.bias)
    }

    /// Direct table write — `table[hash] = pos`. Same bounds reasoning
    /// as [`get`].
    ///
    /// # Safety
    ///
    /// `hash` MUST be a value returned by [`hash_ptr`] on this table.
    #[inline(always)]
    pub(crate) unsafe fn put(&mut self, hash: u32, pos: u32) {
        debug_assert!((hash as usize) < self.table.len());
        // Cannot overflow: `advance_epoch` caps the bias at
        // `u32::MAX - 2^31` and `pos` is bounded by the eviction band.
        let biased = pos + self.bias;
        // SAFETY: see method-level doc.
        unsafe {
            *self.table.get_unchecked_mut(hash as usize) = biased;
        }
    }
}

/// Free-function form of [`FastHashTable::hash_ptr`] taking `hash_log`
/// explicitly so a hot loop can hoist it into a register once (via
/// [`FastHashTable::hot_state`]) instead of re-reading `self.hash_log` per
/// hash. Bit-for-bit identical to the method.
///
/// # Safety
/// Same readable-bytes contract as [`FastHashTable::hash_ptr`]: `MLS == 4`
/// needs ≥4 readable bytes at `ptr`, `MLS >= 5` needs ≥8. `hash_log` must be
/// in `1..=30` (the constructor's accepted band) so the shift is well-defined.
#[inline(always)]
pub(crate) unsafe fn hash_ptr_raw<const MLS: u32>(ptr: *const u8, hash_log: u32) -> u32 {
    match MLS {
        4 => {
            // SAFETY: caller guarantees ≥4 readable bytes at ptr.
            let u = unsafe { core::ptr::read_unaligned(ptr.cast::<u32>()) }.to_le();
            u.wrapping_mul(PRIME_4_BYTES) >> (32 - hash_log)
        }
        5 => {
            // SAFETY: caller guarantees ≥8 readable bytes (wide u64 load).
            let u = unsafe { core::ptr::read_unaligned(ptr.cast::<u64>()) }.to_le();
            ((u << (64 - 40)).wrapping_mul(PRIME_5_BYTES) >> (64 - hash_log)) as u32
        }
        6 => {
            // SAFETY: caller guarantees ≥8 readable bytes (u64 load).
            let u = unsafe { core::ptr::read_unaligned(ptr.cast::<u64>()) }.to_le();
            ((u << (64 - 48)).wrapping_mul(PRIME_6_BYTES) >> (64 - hash_log)) as u32
        }
        7 => {
            // SAFETY: caller guarantees ≥8 readable bytes (u64 load).
            let u = unsafe { core::ptr::read_unaligned(ptr.cast::<u64>()) }.to_le();
            ((u << (64 - 56)).wrapping_mul(PRIME_7_BYTES) >> (64 - hash_log)) as u32
        }
        8 => {
            // SAFETY: caller guarantees ≥8 readable bytes (full u64).
            let u = unsafe { core::ptr::read_unaligned(ptr.cast::<u64>()) }.to_le();
            (u.wrapping_mul(PRIME_8_BYTES) >> (64 - hash_log)) as u32
        }
        _ => {
            debug_assert!(false, "unsupported MLS {MLS}");
            0
        }
    }
}
