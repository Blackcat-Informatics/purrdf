#![allow(
    clippy::cast_lossless,
    clippy::elidable_lifetime_names,
    clippy::inline_always,
    clippy::manual_assert_eq,
    clippy::manual_bit_width,
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use crate::bit_io::BitReaderReversed;
use crate::cpu_kernel::CpuKernel;
use crate::decoding::errors::{FSEDecoderError, FSETableError};
use alloc::vec::Vec;

// Visibility = `pub` so the `pub type FSEDecoder` / `pub type
// SeqFSEDecoder` aliases below don't expose a more-private struct
// (compiler `private_interfaces` warning). External reachability is
// gated by `crate::fse` module visibility: `pub(crate) mod fse` in
// the default build keeps everything crate-internal; under
// `any()` the module becomes `pub mod fse` and
// the struct becomes externally accessible — which is exactly what
// the fuzz harness needs.
pub(crate) struct FSEDecoderImpl<'table, E: FseEntry, const CAP: usize> {
    /// An FSE state value represents an index in the FSE table.
    pub state: E,
    /// A reference to the table used for decoding.
    table: &'table FSETableImpl<E, CAP>,
}

/// Type alias preserved for the HUF-weight-stream callers: the
/// per-state byte (`decode_symbol`) lives on this alias's `Entry`-only
/// inherent impl below.
pub(crate) type FSEDecoder<'table> = FSEDecoderImpl<'table, Entry, 64>;

impl<'t, E: FseEntry, const CAP: usize> FSEDecoderImpl<'t, E, CAP> {
    /// Initialize a new Finite State Entropy decoder.
    pub(crate) fn new(table: &'t FSETableImpl<E, CAP>) -> FSEDecoderImpl<'t, E, CAP> {
        FSEDecoderImpl {
            // Placeholder; `init_state` overwrites it before the first read.
            // (Was `decode[0]`, but the decode array is now `MaybeUninit` — its
            // first slot is uninitialised until the build fills it.)
            state: E::default(),
            table,
        }
    }
}

impl<'t, const CAP: usize> FSEDecoderImpl<'t, Entry, CAP> {
    /// Returns the byte associated with the symbol the internal cursor is pointing at.
    pub(crate) fn decode_symbol(&self) -> u8 {
        self.state.symbol
    }
}

impl<'t, E: FseEntry, const CAP: usize> FSEDecoderImpl<'t, E, CAP> {
    /// Initialize internal state and prepare for decoding. After this, `decode_symbol` can be called
    /// to read the first symbol and `update_state` can be called to prepare to read the next symbol.
    pub(crate) fn init_state<K: CpuKernel>(
        &mut self,
        bits: &mut BitReaderReversed<'_, K>,
    ) -> Result<(), FSEDecoderError> {
        // Uninitialised = no decode entries. (Previously this checked
        // `accuracy_log == 0`, but a valid RLE table has accuracy_log 0
        // with a single entry — upstream zstd's RLE DTable. An empty `decode`
        // vec is the real "never built" signal; the `InvalidTableShape`
        // check below still enforces `decode.len() == 1 << accuracy_log`,
        // which holds for the 1-entry RLE table since `1 << 0 == 1`.)
        if self.table.decode_len == 0 {
            return Err(FSEDecoderError::TableIsUninitialized);
        }
        // Defense-in-depth internal-invariant guard: in normal builds
        // `crate::fse` is not externally reachable, but malformed
        // tables can still arise from internal misuse, future
        // `any()`. Validate up-front that
        // `decode.len() == 1 << accuracy_log` and surface a typed
        // `InvalidTableShape` error (distinct from
        // `TableIsUninitialized` to keep error triage unambiguous).
        // Without this, `read_entry`'s unchecked indexing (the
        // `cfg(not(fuzz_exports))` arm) could hit UB on a malformed
        // table in release builds. `checked_shl` covers the
        // pathological case where `accuracy_log >= usize::BITS`.
        // Branch cost is a single per-call check; the per-sequence
        // hot path (`update_state_fast`) is unaffected.
        let accuracy_log = self.table.accuracy_log;
        let decode_len = self.table.decode_len;
        let expected =
            1usize
                .checked_shl(accuracy_log.into())
                .ok_or(FSEDecoderError::InvalidTableShape {
                    decode_len,
                    accuracy_log,
                })?;
        if decode_len != expected {
            return Err(FSEDecoderError::InvalidTableShape {
                decode_len,
                accuracy_log,
            });
        }
        let new_state = bits.get_bits(self.table.accuracy_log);
        // SAFETY: `accuracy_log` bits read from the bitstream produce
        // `new_state < (1 << accuracy_log) = table_size = decode.len()`.
        // `build_decoding_table` ensures the table is sized exactly
        // `1 << accuracy_log` entries. The bounds check that the
        // checked indexing would emit is provably redundant. Under
        // `any()` `read_entry` falls back to the
        // bounds-checked path — see comment on `read_entry`.
        self.state = self.read_entry(new_state as usize);

        Ok(())
    }

    /// Read `decode[idx]` — bounds-checked under `fuzz_exports`, unchecked
    /// otherwise. The call sites all hold the FSE invariant `idx <
    /// decode.len()` by construction (`init_state` reads
    /// `accuracy_log` bits, `update_state*` derive `next_state` from
    /// `Entry.new_state + add` where `calc_baseline_and_numbits`
    /// guarantees `new_state + (1 << num_bits) - 1 < table_size`).
    /// Under `fuzz_exports` external code can construct a mis-shaped
    /// table that violates the invariant — fall back to checked
    /// indexing so a fuzz harness sees a panic rather than UB, even
    /// when the fuzz binary is built in release mode (which makes
    /// `debug_assert!` a no-op and is the default for `cargo fuzz`).
    #[inline(always)]
    fn read_entry(&self, idx: usize) -> E {
        // SAFETY: `idx` is invariant-bounded by the FSE table-build /
        // state-transition contract to `< decode_len`, and the build wrote
        // every entry in `[0, decode_len)`, so the read is in-bounds AND
        // initialised. LLVM cannot prove this on its own because the invariant
        // spans `build_decoding_table` and the decode call sites.
        unsafe { self.table.decode.get_unchecked(idx).assume_init_read() }
    }

    /// Advance the internal state **without** an individual refill check.
    ///
    /// # Preconditions (caller-enforced)
    ///
    /// 1. **Bit budget:** enough bits MUST be available in the bit
    ///    reader (e.g. via [`BitReaderReversed::ensure_bits`] with a
    ///    budget that covers this and any other unchecked reads in the
    ///    same batch).
    /// 2. **State initialisation:** [`init_state`] MUST have returned
    ///    `Ok` on this decoder before any `update_state_fast` call.
    ///    Calling `update_state_fast` on a fresh `FSEDecoder::new`
    ///    output (which holds a zero-default `state` and may reference
    ///    an empty `decode` vec) would resolve to
    ///    `read_entry(0).get_unchecked(0)` on an empty slice — UB.
    ///    The empty-table guard in [`update_state`] is intentionally
    ///    omitted here to keep the per-sequence fast path branch-free;
    ///    the only call site (`decode_and_execute_sequences`) always
    ///    succeeds `init_state` before entering the per-sequence loop,
    ///    so the precondition holds by construction.
    ///
    /// This is the "fast path" used in the interleaved sequence decode loop
    /// where a single refill check covers all three FSE state updates.
    ///
    /// [`init_state`]: Self::init_state
    /// [`update_state`]: Self::update_state
    #[inline(always)]
    pub(crate) fn update_state_fast<K: CpuKernel>(&mut self, bits: &mut BitReaderReversed<'_, K>) {
        let num_bits = self.state.num_bits();
        let add = bits.get_bits_unchecked(num_bits);
        let next_state = usize::from(self.state.new_state()) + add as usize;
        // SAFETY: `new_state` and `num_bits` were paired by
        // `calc_baseline_and_numbits` during table construction such that
        // `new_state + (2.pow(num_bits) - 1) < table_size = self.table.decode.len()`.
        // `add` is the value of `num_bits` bits read from the bitstream, so
        // `add < 2.pow(num_bits)` by construction of `BitReaderReversed::get_bits_unchecked`.
        // Therefore `next_state < self.table.decode.len()` and the indexed read
        // is in bounds; LLVM cannot prove this invariant on its own because it
        // spans the table-build and decode call sites. Under
        // `any()` `read_entry` falls back to bounds-checked
        // indexing — see comment on `read_entry`.
        self.state = self.read_entry(next_state);
    }
}

/// FSE decoding involves a decoding table that describes the probabilities of
/// all literals from 0 to the highest present one
///
/// <https://github.com/facebook/zstd/blob/dev/doc/zstd_compression_format.md#fse-table-description>
#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct FSETableImpl<E: FseEntry, const CAP: usize> {
    /// The maximum symbol in the table (inclusive). Limits the probabilities length to max_symbol + 1.
    max_symbol: u8,
    /// The decode table: a fixed-size inline array sized to the worst-case
    /// `1 << max_accuracy_log` for this table's shape (monomorphized per
    /// strategy — HUF weights `CAP=64`, sequence LL/ML/OF `CAP=512`). Only
    /// `decode[..decode_len]` is live; the rest is unused capacity. Storing it
    /// inline (vs a `Vec`) lets `reinit_from` copy the whole table in one
    /// contiguous `memcpy` (array assignment) — mirroring the upstream zstd's
    /// fixed-array `ZSTD_entropyDTables_t` copied by `ZSTD_copyDDictParameters`
    /// — instead of a heap copy through a separate allocation.
    ///
    /// `MaybeUninit` so constructing a fresh table does NOT memset the whole
    /// `CAP`-entry array (12 KiB across the three seq tables): the build writes
    /// every live entry `[0, decode_len)` and the decoder reads only that span
    /// (`init_state` proves `decode_len == 1 << accuracy_log` and every state is
    /// `< decode_len`), so the unused tail never needs initialisation. On tiny
    /// frames that memset dominated the whole decode (the per-`FrameDecoder`
    /// fixed cost); skipping it is the win there.
    decode: [core::mem::MaybeUninit<E>; CAP],
    /// Number of live entries in `decode` (`1 << accuracy_log`).
    decode_len: usize,
    /// Reused scratch buffer for symbol spreading to avoid per-build allocations.
    symbol_spread_buffer: Vec<u8>,
    /// The size of the table is stored in logarithm base 2 format,
    /// with the **size of the table** being equal to `(1 << accuracy_log)`.
    /// This value is used so that the decoder knows how many bits to read from the bitstream.
    pub accuracy_log: u8,
    /// In this context, probability refers to the likelihood that a symbol occurs in the given data.
    /// Given this info, the encoder can assign shorter codes to symbols that appear more often,
    /// and longer codes that appear less often, then the decoder can use the probability
    /// to determine what code was assigned to what symbol.
    ///
    /// The probability of a single symbol is a value representing the proportion of times the symbol
    /// would fall within the data.
    ///
    /// If a symbol probability is set to `-1`, it means that the probability of a symbol
    /// occurring in the data is less than one.
    pub symbol_probabilities: Vec<i32>, //used while building the decode Vector
}

/// Type alias preserved for HUF-weight-stream callers and existing
/// tests; the sequence-section variant is [`SeqFSETable`].
// HUF weight-stream FSE: accuracy_log is capped at 6 (`build_decoder(_, 6)`),
// so the worst-case table is `1 << 6 = 64` entries.
pub(crate) type FSETable = FSETableImpl<Entry, 64>;

/// Compact sequence-section variant. Backed by 8-byte [`SeqSymbol`]
/// entries instead of the 12-byte HUF [`Entry`] — matches upstream zstd
/// `ZSTD_seqSymbol`. The per-entry `symbol` byte is dropped. Build
/// flow: [`FseEntry::from_raw`] zero-inits `base_value` /
/// `num_additional_bits` on insert; the LL / ML / OF enrich passes
/// ([`FSETableImpl::<SeqSymbol>::enrich_with_packed_seq_meta`] +
/// [`FSETableImpl::<SeqSymbol>::enrich_for_offsets`]) populate them
/// in a second walk over `decode[]`, reading the source byte from
/// the persisted `symbol_spread_buffer`.
// Sequence-section FSE (LL / ML / OF): accuracy_log is capped at 9
// (`LL_MAX_LOG` / `ML_MAX_LOG`; OF at 8), so the worst-case table is
// `1 << 9 = 512` entries. OF uses only the first 256 — the extra capacity
// is harmless and keeps a single alias for all three seq tables so the
// contiguous block in `FSEScratch` copies in one memcpy.
#[allow(dead_code)]
pub(crate) type SeqFSETable = FSETableImpl<SeqSymbol, 512>;

impl<E: FseEntry, const CAP: usize> FSETableImpl<E, CAP> {
    /// Initialize a new empty Finite State Entropy decoding table.
    pub fn new(max_symbol: u8) -> Self {
        FSETableImpl {
            max_symbol,
            // Lazy: the first `read_probabilities` grows it. Pre-reserving 256
            // i32 here was a heap allocation paid per fresh table (3 seq + the
            // HUF weight table per `DecoderScratch`), part of the per-frame fixed
            // cost that dominates tiny-frame decode.
            symbol_probabilities: Vec::new(),
            symbol_spread_buffer: Vec::new(),
            // Uninitialised — no `CAP`-entry memset; the build fills the live
            // span before any read (see the field doc).
            decode: [const { core::mem::MaybeUninit::uninit() }; CAP],
            decode_len: 0,
            accuracy_log: 0,
        }
    }

    /// Heap bytes owned by this table. The `decode` table is a fixed inline
    /// array (counted by `size_of`, not here); only the build-scratch vectors
    /// are heap-allocated.
    pub fn heap_bytes(&self) -> usize {
        self.symbol_spread_buffer.capacity()
            + self.symbol_probabilities.capacity() * size_of::<i32>()
    }

    /// Live decode entries (`decode[..decode_len]`).
    #[inline(always)]
    pub fn decode(&self) -> &[E] {
        // SAFETY: the build initialised every entry in `[0, decode_len)` and set
        // `decode_len <= CAP`, so the live prefix is fully initialised;
        // `MaybeUninit<E>` shares E's layout, so reinterpreting it as `&[E]` is
        // sound.
        unsafe { core::slice::from_raw_parts(self.decode.as_ptr().cast::<E>(), self.decode_len) }
    }

    /// Reset `self` and update `self`'s state to mirror the provided table.
    /// `symbol_spread_buffer` is build-time scratch + enrich source; every
    /// call site that uses `reinit_from` (predefined-cache copy + dict
    /// scratch init) feeds a SOURCE table whose `decode[]` is ALREADY
    /// enriched, so the spread buffer is dead on the post-reinit path.
    /// Reserve capacity to keep the next `build_decoder` allocation-free,
    /// but skip the bytes copy.
    pub fn reinit_from(&mut self, other: &Self) {
        self.reset();
        // Copy ONLY the decode-time state. `symbol_probabilities` and
        // `symbol_spread_buffer` are build-time scratch ("used while building
        // the decode Vector"): the decode hot path and Repeat-mode reuse read
        // only `decode` + `accuracy_log`, and any block that rebuilds the
        // table (`build_decoder`) repopulates the scratch itself. Skipping
        // them mirrors the upstream zstd copying just the FSE decode table per frame
        // instead of the full build workspace.
        // ONE contiguous memcpy of the whole fixed-size decode array (the
        // monomorphized-per-shape table), mirroring the upstream zstd's single
        // `ZSTD_copyDDictParameters` memcpy — instead of a heap `Vec` copy
        // through a separate allocation. `decode_len` carries the live span.
        self.decode = other.decode;
        self.decode_len = other.decode_len;
        self.accuracy_log = other.accuracy_log;
    }

    /// Empty the table and clear all internal state.
    pub fn reset(&mut self) {
        self.symbol_probabilities.clear();
        self.symbol_spread_buffer.clear();
        self.decode_len = 0;
        self.accuracy_log = 0;
    }

    /// Whether this table holds a built decode table. A just-reset / never-built
    /// table reads as `false`; resetting it again is a no-op, so the per-frame
    /// scratch reset can skip it (mirrors upstream zstd, which never clears
    /// entropy tables per frame — it marks them invalid by flag and rebuilds
    /// lazily). The signal is `decode_len != 0`, NOT `accuracy_log != 0`: a valid
    /// RLE DTable (`build_rle`) has `decode_len == 1` but `accuracy_log == 0`, so
    /// an `accuracy_log` check would miss it and leave a used RLE table uncleared
    /// for a later Repeat-mode frame. `init_state` uses the same `decode_len`
    /// signal for "uninitialized".
    #[inline]
    pub(crate) fn is_populated(&self) -> bool {
        self.decode_len != 0
    }

    /// Build the equivalent encoder-side table from a parsed decoder table.
    pub(crate) fn to_encoder_table(&self) -> Option<crate::fse::fse_encoder::FSETable> {
        if self.accuracy_log == 0 || self.symbol_probabilities.is_empty() {
            return None;
        }
        // The encoder table builder indexes a fixed `1 << 9 = 512`-entry stack
        // scratch, so it can only represent `accuracy_log <= 9` (the sequence
        // FSE wire cap). The decoder accepts tables up to
        // `ENTRY_MAX_ACCURACY_LOG = 16` (a non-sequence table reached through a
        // dictionary can carry one), and slicing the scratch with `1 <<
        // accuracy_log > 512` would panic in release too. Such a table cannot be
        // reused as a sequence encoder table — return None so the caller builds
        // a fresh one instead of indexing past the scratch.
        const MAX_ENCODER_ACCURACY_LOG: u8 = 9;
        if self.accuracy_log > MAX_ENCODER_ACCURACY_LOG {
            return None;
        }

        Some(crate::fse::fse_encoder::build_table_from_probabilities(
            &self.symbol_probabilities,
            self.accuracy_log,
        ))
    }

    /// returns how many BYTEs (not bits) were read while building the decoder
    pub fn build_decoder(&mut self, source: &[u8], max_log: u8) -> Result<usize, FSETableError> {
        self.build_decoder_fused(source, max_log, SeqMeta::None)
    }

    /// [`Self::build_decoder`] with the sequence-axis meta fused into the
    /// build loop — replaces the build-then-enrich double pass on the
    /// hot per-block table rebuild path.
    pub(crate) fn build_decoder_fused(
        &mut self,
        source: &[u8],
        max_log: u8,
        meta: SeqMeta<'_>,
    ) -> Result<usize, FSETableError> {
        let max_log = max_log.min(ENTRY_MAX_ACCURACY_LOG);
        self.accuracy_log = 0;

        let bytes_read = self.read_probabilities(source, max_log)?;
        self.build_decoding_table_meta(meta)?;

        Ok(bytes_read)
    }

    /// Parse the table description into `symbol_probabilities` +
    /// `accuracy_log` WITHOUT building the decoding table.
    ///
    /// Returns the same byte count as [`Self::build_decoder`] (the table
    /// description length), so a caller stepping a cursor over a packed
    /// stream of tables advances identically. Used by the encoder
    /// dictionary load: [`Self::to_encoder_table`] reads only the
    /// probabilities + accuracy log, so building the decode table (and
    /// the `enrich_*` post-passes, which touch only decode entries) is
    /// pure waste there.
    ///
    /// The existing `decode` table is cleared so a reused `FSETableImpl`
    /// can't silently keep decoding against a stale table that no longer
    /// matches the just-parsed probabilities (`init_state` would
    /// otherwise pass whenever the old `decode.len()` still equalled
    /// `1 << accuracy_log`). After this call the table is intentionally
    /// non-decodable until `build_decoding_table` runs.
    // `pub(crate)`: this leaves the table in an intentionally non-decodable
    // partial-init state, so it must not be reachable from the public API
    // (the module is re-exported with `pub use`). Only the crate-internal
    // encoder-dictionary parse calls it.
    pub(crate) fn read_table_probabilities(
        &mut self,
        source: &[u8],
        max_log: u8,
    ) -> Result<usize, FSETableError> {
        let max_log = max_log.min(ENTRY_MAX_ACCURACY_LOG);
        self.accuracy_log = 0;
        self.decode_len = 0;
        self.read_probabilities(source, max_log)
    }

    /// Given the provided accuracy log, build a decoding table from that log.
    pub fn build_from_probabilities(
        &mut self,
        acc_log: u8,
        probs: &[i32],
    ) -> Result<(), FSETableError> {
        if acc_log == 0 {
            return Err(FSETableError::AccLogIsZero);
        }
        if acc_log > ENTRY_MAX_ACCURACY_LOG {
            return Err(FSETableError::AccLogTooBig {
                got: acc_log,
                max: ENTRY_MAX_ACCURACY_LOG,
            });
        }
        // Probability sum check: `build_decoding_table` assumes the
        // sum of positive probabilities plus the count of `-1`
        // entries (each contributing one slot at the top of the
        // table) equals exactly `1 << acc_log`. Without this guard
        // the wire-format `parse_wire` path validates the sum
        // upstream, but callers entering through
        // `build_from_probabilities` directly (the Predefined cache
        // and any fuzz / external user) would silently produce a
        // table where `calc_baseline_and_numbits` is given
        // `symbol_count` values exceeding the symbol's actual
        // `prob`, yielding `new_state` / `num_bits` pairs that can
        // overshoot `decode.len()` on the unchecked `read_entry`
        // hot path. Surface as a typed error so the caller can
        // distinguish a malformed input from an internal failure.
        // Strict probability range validation: RFC 8878 §4.1.1 admits
        // only `{-1, 0, 1..=table_size}` as probability values. The
        // wire-format parser never emits anything else, but
        // `build_from_probabilities` is a public entry point reachable
        // from fuzz harnesses and external users — leaving the gate
        // open invites two attack shapes:
        //
        //   1. Silent malformed table: clamping `p < -1` to 0 (the
        //      previous `p.max(0)`) lets `[-2, ...]` satisfy a sum
        //      check whose remaining terms happen to add up to
        //      `table_size`, producing a quietly broken table.
        //   2. DoS via probability-sum overflow: `[i32::MAX, i32::MAX,
        //      0x42] as u32` wraps to `0x40 = 64 = 1 << 6`, satisfies
        //      the sum check, then `build_decoding_table` runs
        //      `for _ in 0..prob` against `prob = i32::MAX`, looping
        //      2^31-1 times per such symbol (worst case: spread-array
        //      out-of-bounds panic, best case: minutes of CPU).
        //
        // Reject any `p > table_size` upfront so the subsequent u32
        // sum is bounded (see the sum's own comment for the
        // wrap-impossibility argument).
        let table_size = 1u32 << acc_log;
        for &p in probs {
            if p < -1 || p > table_size as i32 {
                return Err(FSETableError::InvalidProbability {
                    value: p,
                    table_size,
                    accuracy_log: acc_log,
                });
            }
        }
        // Sum the validated probs in u32. Per-element validation
        // above bounds each non-`-1` value by `table_size <= 1 << 16`,
        // and `probs.len() <= 256`, so the worst-case sum
        // `256 * 65536 = 16M` fits u32 with 8 bits of headroom — no
        // wrap possible. Keeps the public
        // `ProbabilityCounterMismatch.got` field at u32 (no public
        // API break).
        let probability_sum: u32 = probs
            .iter()
            .map(|&p| if p == -1 { 1u32 } else { p as u32 })
            .sum();
        if probability_sum != table_size {
            return Err(FSETableError::ProbabilityCounterMismatch {
                got: probability_sum,
                expected_sum: table_size,
                symbol_probabilities: probs.to_vec(),
            });
        }
        self.symbol_probabilities.clear();
        self.symbol_probabilities.extend_from_slice(probs);
        self.accuracy_log = acc_log;
        self.build_decoding_table()
    }

    /// Build the actual decoding table after probabilities have been
    /// read. Upstream zstd-shape single-pass build: spread symbols into the
    /// scratch buffer, then write `decode` entries in one linear pass
    /// — no intermediate zero-init Vec::resize, no per-call heap
    /// allocation for the symbol counter (stack-allocated since the
    /// max symbol count is bounded by `u8::MAX + 1 = 256`).
    ///
    /// Wraps `build_decoding_table_inner` so the `symbol_spread_buffer`
    /// scratch is unconditionally restored to `self` on every exit
    /// path (success OR error) — otherwise an early `Err` from the
    /// inner pass would drop the taken buffer and force a fresh
    /// allocation on the next build.
    ///
    /// On `Err` the table is also fully `reset()` after the buffer
    /// restore: the inner pass mutates `self.decode` (partial push)
    /// while `self.accuracy_log` / `self.symbol_probabilities` were
    /// already set by the caller (`build_from_probabilities`); leaving
    /// that inconsistency in place would let a subsequent `init_state`
    /// pass the `accuracy_log != 0` gate and read from a partial
    /// `decode` vec — UB. After `reset()` the table is in the same
    /// well-defined empty state a freshly-constructed `FSETable` has;
    /// any subsequent `init_state` returns `TableIsUninitialized`.
    fn build_decoding_table(&mut self) -> Result<(), FSETableError> {
        self.build_decoding_table_meta(SeqMeta::None)
    }

    fn build_decoding_table_meta(&mut self, meta: SeqMeta<'_>) -> Result<(), FSETableError> {
        let mut spread = core::mem::take(&mut self.symbol_spread_buffer);
        let result = self.build_decoding_table_inner(&mut spread, meta);
        self.symbol_spread_buffer = spread;
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn build_decoding_table_inner(
        &mut self,
        spread: &mut Vec<u8>,
        meta: SeqMeta<'_>,
    ) -> Result<(), FSETableError> {
        let nb_symbols = self.symbol_probabilities.len();
        if nb_symbols > self.max_symbol as usize + 1 {
            return Err(FSETableError::TooManySymbols { got: nb_symbols });
        }

        let table_size = 1 << self.accuracy_log;
        // The decode array is `[E; CAP]` and the fast-spread scratch is a fixed
        // `FSE_FAST_SPREAD_BUF`; a `table_size` above CAP would overrun both.
        // The wire path never exceeds CAP (sequence acc_log <= 9, HUF <= 6), but
        // `build_from_probabilities` accepts acc_log up to
        // `ENTRY_MAX_ACCURACY_LOG` (16) and is reachable from fuzz, so reject an
        // oversized table as a typed error instead of panic-overrunning.
        if table_size > CAP {
            return Err(FSETableError::AccLogTooBig {
                got: self.accuracy_log,
                max: CAP.trailing_zeros() as u8,
            });
        }

        // === Spread step ===
        // Reuse the persistent scratch buffer; clear then resize to
        // table_size. The resize is the ONLY zero-init in the entire
        // build path now (previous impl also zero-init'd `decode`
        // through `Vec::resize` with a default `Entry`, doubling the
        // write traffic on the build path).
        spread.clear();
        spread.resize(table_size, 0);

        // Upstream zstd `ZSTD_buildFSETable_body`-shape per-symbol counter
        // (upstream zstd `symbolNext[]`). Indexed by symbol byte; initialised
        // to `prob` for positive-probability symbols, `1` for `-1`
        // (low-probability) symbols. The build loop below reads then
        // increments `symbol_next[symbol]` per state placed, and
        // derives `(num_bits, new_state)` from the running counter
        // via pure shifts / `highest_bit_set` — NO division. This
        // replaces the previous `calc_baseline_and_numbits` helper
        // whose `num_states_total / num_state_slices` lowered to a
        // runtime `divl` (~24 cycles, ~24% of FSE-build samples on
        // z000033 L-5 fast — see perf annotate audit). Stack-only
        // (256-symbol alphabet bounded by `u8`); zero-init covers
        // every slot.
        let mut symbol_next = [0u32; 256];

        // Pass 1: place -1 probability symbols at the top of the
        // table AND initialise `symbol_next` for them. Upstream zstd:
        // `tableDecode[highThreshold--].baseValue = s; symbolNext[s]
        // = 1;`.
        //
        // Index loop (not `iter().enumerate().take()`) — LLVM emits
        // a tighter scalar loop without the Iterator::next state
        // machine. The enumerate+take iterator chain was visible as
        // ~1.8% combined self-time on the decode flamegraph.
        let probs = self.symbol_probabilities.as_slice();
        let mut negative_idx = table_size;
        for symbol in 0..nb_symbols {
            let prob = probs[symbol];
            if prob == -1 {
                negative_idx -= 1;
                spread[negative_idx] = symbol as u8;
                symbol_next[symbol] = 1;
            }
        }

        // Pass 2: distribute positive-probability symbols. With NO
        // low-probability (-1) symbols — the common case on small frames, where
        // the encoder keeps `useLowProbCount` off below 2048 sequences — take the
        // upstream zstd fast spread (`ZSTD_buildFSETable_body`,
        // zstd_decompress_block.c:529-574): lay the symbols down in order with
        // 8-byte writes, then distribute them in a branch-miss-free two-stage
        // pass unrolled by 2. This replaces the per-symbol variable-length inner
        // loop + lowprob `while` skip below, which dominated the FSE-build
        // self-time on the decode profile. The two passes are provably identical
        // when there are no -1 symbols: both place the k-th in-order symbol at
        // `next_position`-walk step k.
        if negative_idx == table_size {
            let mut in_order = [0u8; FSE_FAST_SPREAD_BUF];
            let mut pos = 0usize;
            for symbol in 0..nb_symbols {
                let prob = probs[symbol];
                if prob <= 0 {
                    continue;
                }
                symbol_next[symbol] = prob as u32;
                let n = prob as usize;
                // 8 copies of the symbol byte; lay down 8 at a time (counts are
                // mostly <= 8 on small table-logs, so the inner loop rarely runs).
                let bytes = ((symbol as u8) as u64)
                    .wrapping_mul(0x0101_0101_0101_0101)
                    .to_le_bytes();
                in_order[pos..pos + 8].copy_from_slice(&bytes);
                let mut i = 8;
                while i < n {
                    in_order[pos + i..pos + i + 8].copy_from_slice(&bytes);
                    i += 8;
                }
                pos += n;
            }
            let table_mask = table_size - 1;
            let step = (table_size >> 1) + (table_size >> 3) + 3;
            let mut position = 0usize;
            let mut s = 0usize;
            while s < table_size {
                spread[position] = in_order[s];
                spread[(position + step) & table_mask] = in_order[s + 1];
                position = (position + 2 * step) & table_mask;
                s += 2;
            }
        } else {
            let mut position = 0usize;
            for symbol in 0..nb_symbols {
                let prob = probs[symbol];
                if prob <= 0 {
                    continue;
                }
                symbol_next[symbol] = prob as u32;
                let symbol_u8 = symbol as u8;
                for _ in 0..prob {
                    spread[position] = symbol_u8;
                    position = next_position(position, table_size);
                    while position >= negative_idx {
                        position = next_position(position, table_size);
                    }
                }
            }
        }

        // === Build step (upstream zstd formula) ===
        // For each state u in 0..tableSize, upstream zstd `ZSTD_buildFSETable_body`:
        //   nextState = symbolNext[symbol]++
        //   nbBits    = tableLog - ZSTD_highbit32(nextState)
        //   newState  = (nextState << nbBits) - tableSize
        //
        // Identity with our previous `calc_baseline_and_numbits` was
        // verified algebraically: for a symbol with positive prob N,
        // `nextState` walks N..2N-1; `highest_bit_set(nextState)`
        // partitions this range into "double-width" (high_bit ==
        // ceil(log2(N))) and "single-width" (high_bit ==
        // ceil(log2(N))+1) slices, matching the
        // `num_double_width_state_slices` / `num_single_width...`
        // split in the old code. For -1 prob, `symbol_next` is `1`,
        // so `nextState == 1`, `high_bit == 1`, `nbBits ==
        // accuracy_log`, `newState == 0` — exactly the -1 entry
        // shape.
        let accuracy_log = self.accuracy_log;
        let table_size_u32 = table_size as u32;
        // Write the `table_size` live entries directly into the inline,
        // value-initialised `decode` array (no `MaybeUninit` / `set_len`
        // dance needed — the array already holds `E::default()` in every
        // slot). `decode_len = 0` up front means an early `Err` leaves the
        // table "unbuilt" (empty live span), matching the old
        // `clear()`-then-`set_len` contract. `table_size <= CAP` holds because
        // the caller passes `max_log` (OF=8 / LL=ML=9 / HUF=6) and
        // `read_probabilities` rejects `accuracy_log > max_log`, so
        // `1 << accuracy_log <= 1 << max_log == CAP`.
        debug_assert!(
            table_size <= CAP,
            "FSE table_size {table_size} exceeds monomorphized CAP {CAP}",
        );
        self.decode_len = 0;

        // Slice index instead of `spread.iter().take(table_size)`:
        // if `spread.len() < table_size` (a future refactor breaking
        // the upstream `spread.resize(table_size, 0)` invariant), the
        // slice indexing panics here BEFORE the unsafe `set_len`
        // below would claim uninitialised entries. `take()` would
        // silently shorten the loop and leave `slots` half-written,
        // which the post-loop `set_len(table_size)` would then expose
        // as UB. Indexing surfaces the invariant violation as a
        // bounds-check panic instead.
        for (state_idx, &symbol) in spread[..table_size].iter().enumerate() {
            let next_state = symbol_next[symbol as usize];
            // `next_state >= 1` by construction: upstream
            // `read_probabilities` / `build_from_probabilities`
            // validate that `sum(prob, treating -1 as 1) ==
            // table_size`, which guarantees Pass 1 + Pass 2 above
            // fully cover spread[] (no zero defaults survive
            // `spread.resize(table_size, 0)`) and every symbol that
            // appears in spread[] has `symbol_next[s] > 0`.
            // `highest_bit_set(x)` returns `floor(log2(x)) + 1`.
            symbol_next[symbol as usize] = next_state + 1;
            let high_bit = highest_bit_set(next_state);
            // nbBits = accuracy_log - floor(log2(next_state))
            //        = accuracy_log - (high_bit - 1)
            //        = (accuracy_log + 1) - high_bit
            // For -1 prob: next_state = 1, high_bit = 1, nbBits =
            // accuracy_log.
            // For positive prob N: next_state in N..2N-1; max
            // next_state = 2N-1 ≤ 2*table_size - 1, max high_bit
            // = accuracy_log + 1 (when N == table_size), nbBits = 0.
            // So nbBits ∈ [0, accuracy_log], satisfying the
            // unchecked-read invariant in `FSEDecoder::read_entry`.
            let nb = (accuracy_log as u32 + 1).wrapping_sub(high_bit) as u8;
            // FSE invariant gate: keep the explicit `nb > accuracy_log`
            // guard for `build_from_probabilities` (public surface) so
            // a crafted probability vector can't silently violate
            // `new_state + (1 << nb) - 1 < table_size`. With the upstream zstd
            // formula `nb` derives from `high_bit` which is itself
            // bounded by the table-size invariant, but a malformed
            // probability accumulating beyond `table_size` could push
            // high_bit > accuracy_log + 1 and wrap `nb` to a large
            // u8. Reject so the unchecked indexing contract holds.
            if nb > accuracy_log {
                // `decode.len()` is still 0 (set by `clear()` above) —
                // no `set_len` ran, so no uninitialised entry is
                // observable to the outer `build_decoding_table`'s
                // `reset()` path. The partially-filled `slots` buffer
                // is dropped here harmlessly (`MaybeUninit<E>` has no
                // Drop).
                return Err(FSETableError::TableInvariantViolation {
                    prob: self.symbol_probabilities[symbol as usize],
                    symbol,
                    num_bits: nb,
                    accuracy_log,
                });
            }
            // `next_state << nb` ranges [table_size, 2*table_size - (1 << nb)];
            // subtracting `table_size` gives `new_state ∈ [0, table_size - 1]`
            // which fits u16 for any `accuracy_log <= 16`
            // (`ENTRY_MAX_ACCURACY_LOG`). The wire format caps
            // `accuracy_log` at `FSE_MAX_TABLELOG = 9` for sequence
            // tables, well below the u16 bound. Use normal
            // subtraction (not wrapping_sub) so the
            // implicit-overflow debug_assert catches any future
            // formula bug instead of silently producing a
            // malformed entry.
            let new_state_u32 = (next_state << nb) - table_size_u32;
            let entry = E::from_raw(new_state_u32 as u16, symbol, nb);
            // Fused enrich: write the sequence meta in the same pass
            // instead of re-walking the finished table. `from_raw`
            // zero-inits the meta fields, so the no-meta arms match
            // the post-pass results exactly.
            self.decode[state_idx] = core::mem::MaybeUninit::new(match meta {
                SeqMeta::None => entry,
                SeqMeta::Packed(packed) => {
                    let m = packed.get(symbol as usize).copied().unwrap_or(0);
                    entry.with_seq_meta(m & 0x00FF_FFFF, (m >> 24) as u8)
                }
                SeqMeta::Offsets => {
                    if symbol < 32 {
                        entry.with_seq_meta(1u32 << symbol, symbol)
                    } else {
                        entry
                    }
                }
            });
        }

        // Commit the live span. The loop wrote every `state_idx ∈
        // [0, table_size)`; an Err above returns with `decode_len` still 0.
        self.decode_len = table_size;

        Ok(())
    }

    /// Read the accuracy log and the probability table from the source and return the number of bytes
    /// read. If the size of the table is larger than the provided `max_log`, return an error.
    fn read_probabilities(&mut self, source: &[u8], max_log: u8) -> Result<usize, FSETableError> {
        self.symbol_probabilities.clear(); //just clear, we will fill a probability for each entry anyways. No need to force new allocs here

        // Upstream zstd `FSE_readNCount` cursor shape: a flat little-endian bit
        // position over `source`, extracting each field with one whole-word
        // load. The generic `BitReader::get_bits` assembled fields
        // byte-by-byte with per-call divisions — measured ~6% of decode
        // wall on table-dense btopt frames.
        let total_bits = source.len() * 8;
        let mut bit_pos: usize = 0;
        #[inline(always)]
        fn field_at(source: &[u8], bit_pos: usize, n: usize) -> u64 {
            debug_assert!(n <= 32);
            let byte = bit_pos >> 3;
            let mut window = [0u8; 8];
            let take = source.len().saturating_sub(byte).min(8);
            window[..take].copy_from_slice(&source[byte..byte + take]);
            (u64::from_le_bytes(window) >> (bit_pos & 7)) & ((1u64 << n) - 1)
        }
        macro_rules! read_bits {
            ($n:expr) => {{
                let n: usize = $n;
                if total_bits - bit_pos < n {
                    return Err(FSETableError::GetBitsError(
                        crate::bit_io::GetBitsError::NotEnoughRemainingBits {
                            requested: n,
                            remaining: total_bits - bit_pos,
                        },
                    ));
                }
                let v = field_at(source, bit_pos, n);
                bit_pos += n;
                v
            }};
        }

        self.accuracy_log = ACC_LOG_OFFSET + (read_bits!(4) as u8);
        if self.accuracy_log > ENTRY_MAX_ACCURACY_LOG {
            return Err(FSETableError::AccLogTooBig {
                got: self.accuracy_log,
                max: ENTRY_MAX_ACCURACY_LOG,
            });
        }
        if self.accuracy_log > max_log {
            return Err(FSETableError::AccLogTooBig {
                got: self.accuracy_log,
                max: max_log,
            });
        }
        if self.accuracy_log == 0 {
            return Err(FSETableError::AccLogIsZero);
        }

        let probability_sum = 1 << self.accuracy_log;
        let mut probability_counter = 0;

        while probability_counter < probability_sum {
            let max_remaining_value = probability_sum - probability_counter + 1;
            let bits_to_read = highest_bit_set(max_remaining_value);

            let unchecked_value = read_bits!(bits_to_read as usize) as u32;

            let low_threshold = ((1 << bits_to_read) - 1) - (max_remaining_value);
            let mask = (1 << (bits_to_read - 1)) - 1;
            let small_value = unchecked_value & mask;

            let value = if small_value < low_threshold {
                bit_pos -= 1;
                small_value
            } else if unchecked_value > mask {
                unchecked_value - low_threshold
            } else {
                unchecked_value
            };
            //println!("{}, {}, {}", self.symbol_probablilities.len(), unchecked_value, value);

            let prob = (value as i32) - 1;

            self.symbol_probabilities.push(prob);
            if prob != 0 {
                if prob > 0 {
                    probability_counter += prob as u32;
                } else {
                    // probability -1 counts as 1
                    assert!(prob == -1);
                    probability_counter += 1;
                }
            } else {
                //fast skip further zero probabilities
                loop {
                    let skip_amount = read_bits!(2) as usize;

                    self.symbol_probabilities
                        .resize(self.symbol_probabilities.len() + skip_amount, 0);
                    if skip_amount != 3 {
                        break;
                    }
                }
            }
        }

        if probability_counter != probability_sum {
            return Err(FSETableError::ProbabilityCounterMismatch {
                got: probability_counter,
                expected_sum: probability_sum,
                symbol_probabilities: self.symbol_probabilities.clone(),
            });
        }
        if self.symbol_probabilities.len() > self.max_symbol as usize + 1 {
            return Err(FSETableError::TooManySymbols {
                got: self.symbol_probabilities.len(),
            });
        }

        let bytes_read = if bit_pos.is_multiple_of(8) {
            bit_pos / 8
        } else {
            (bit_pos / 8) + 1
        };

        Ok(bytes_read)
    }
}

impl FSETableImpl<SeqSymbol, 512> {
    /// Populate each entry's `base_value` / `num_additional_bits`
    /// from a packed LL / ML meta table. [`SeqSymbol`] has no
    /// per-state byte; the source symbol for slot `i` is read from
    /// the persisted `symbol_spread_buffer` (still in place after
    /// `build_decoding_table` finishes). Mirrors upstream zstd
    /// `ZSTD_buildSeqTable` post-build enrich for LL / ML.
    pub(crate) fn enrich_with_packed_seq_meta(&mut self, packed: &[u32]) {
        debug_assert_eq!(self.decode_len, self.symbol_spread_buffer.len());
        for i in 0..self.decode_len {
            let sym = self.symbol_spread_buffer[i] as usize;
            // SAFETY: `i < decode_len`, and the build initialised every entry in
            // that span before this enrich pass runs.
            let entry = unsafe { self.decode[i].assume_init_mut() };
            if sym < packed.len() {
                let meta = packed[sym];
                entry.base_value = meta & 0x00FF_FFFF;
                entry.num_additional_bits = (meta >> 24) as u8;
            } else {
                entry.base_value = 0;
                entry.num_additional_bits = 0;
            }
        }
    }

    /// Closed-form offset-code enrich: `base = 1 << code`, `num_add = code`
    /// for `code < 32`. Source byte read from spread buffer.
    pub(crate) fn enrich_for_offsets(&mut self) {
        debug_assert_eq!(self.decode_len, self.symbol_spread_buffer.len());
        for i in 0..self.decode_len {
            let code = self.symbol_spread_buffer[i];
            // SAFETY: `i < decode_len`, entry initialised by the build above.
            let entry = unsafe { self.decode[i].assume_init_mut() };
            entry.base_value = 0;
            entry.num_additional_bits = 0;
            if code < 32 {
                entry.base_value = 1u32 << code;
                entry.num_additional_bits = code;
            }
        }
    }

    /// Build a degenerate single-state RLE table for a sequence axis
    /// whose Compression_Mode is RLE: exactly one symbol, decoded every
    /// sequence with no FSE state-transition bits. Mirrors the upstream zstd
    /// RLE DTable (`accuracy_log = 0`, one entry, `new_state = 0`,
    /// `num_bits = 0`). The caller runs the usual `enrich_with_packed_seq_meta`
    /// (LL/ML) or `enrich_for_offsets` (OF) pass afterward to fill
    /// `base_value` / `num_additional_bits` from `symbol`, so the fused
    /// per-sequence loop reads this axis uniformly with the FSE axes
    /// (init reads 0 state bits, every `update_state` keeps state 0).
    pub(crate) fn build_rle(&mut self, symbol: u8) {
        self.reset();
        // NB: do NOT shrink `max_symbol` to `symbol` — the scratch table
        // is reused across blocks, and a later FSE-mode block's
        // `build_decoder` validates its symbol count against `max_symbol`.
        // Setting it to the single RLE symbol would reject any subsequent
        // table with more symbols (`TooManySymbols`). `reset` leaves
        // `max_symbol` at the axis maximum, which is correct here.
        // Spread buffer drives the enrich pass (symbol per slot); one slot.
        self.symbol_spread_buffer.push(symbol);
        // Single RLE entry at slot 0 (upstream zstd RLE DTable: one state).
        self.decode[0] = core::mem::MaybeUninit::new(SeqSymbol {
            new_state: 0,
            num_bits: 0,
            num_additional_bits: 0,
            base_value: 0,
        });
        self.decode_len = 1;
        // accuracy_log stays 0 (upstream zstd RLE DTable tableLog); init_state
        // reads 0 state bits and update_state keeps the single state.
    }
}

/// Sequence-section decoder alias: reads 8-byte [`SeqSymbol`] entries.
pub(crate) type SeqFSEDecoder<'t> = FSEDecoderImpl<'t, SeqSymbol, 512>;

/// A single entry in an FSE table.
///
/// The first four bytes (`new_state`, `symbol`, `num_bits`) mirror the
/// classical upstream zstd `FSE_decode_t` layout used by every FSE-backed
/// decoder in the crate (sequence-section LL/ML/OF, HUF-weight
/// stream). The trailing `base_value` + `num_additional_bits`
/// fields are populated only by the LL / ML / OF tables in the
/// sequence-section decoder (upstream zstd `ZSTD_seqSymbol` shape) so the
/// per-sequence hot path can read them directly off the active
/// entry instead of issuing a second lookup into a separate
/// metadata table. HUF tables leave these two fields at their
/// default zero — the extra eight bytes per slot are a fixed cost
/// on the small HUF FSE table (≤ 64 entries) and the dominant
/// savings live in the sequence section.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub(crate) struct Entry {
    /// Base index for the next state. The low bits read from the bitstream are
    /// added to this value to produce the final state index.
    pub new_state: u16,
    /// The byte that should be put in the decode output when encountering this state.
    pub symbol: u8,
    /// How many bits should be read from the stream when decoding this entry.
    pub num_bits: u8,
    /// For LL / ML / OF tables: pre-computed code baseline.
    /// `actual_value = base_value + extra_bits_read`. Upstream zstd
    /// `ZSTD_seqSymbol::baseValue`. Populated by the per-table
    /// `enrich_for_*` post-build pass; stays 0 for FSE tables that
    /// don't need it (HUF-weight stream).
    pub base_value: u32,
    /// For LL / ML / OF tables: number of bits to read from the
    /// bitstream after the symbol has been decoded, to obtain the
    /// additional value to add to `base_value`. Upstream zstd
    /// `ZSTD_seqSymbol::nbAdditionalBits`. Populated alongside
    /// `base_value`; stays 0 for FSE tables that don't need it.
    pub num_additional_bits: u8,
}

#[cfg(target_endian = "little")]
const _: [(); 0] = [(); core::mem::offset_of!(Entry, new_state)];
#[cfg(target_endian = "little")]
const _: [(); 2] = [(); core::mem::offset_of!(Entry, symbol)];
#[cfg(target_endian = "little")]
const _: [(); 3] = [(); core::mem::offset_of!(Entry, num_bits)];
#[cfg(target_endian = "little")]
const _: [(); 4] = [(); core::mem::offset_of!(Entry, base_value)];
#[cfg(target_endian = "little")]
const _: [(); 8] = [(); core::mem::offset_of!(Entry, num_additional_bits)];
// 12 bytes: 4 (header) + 4 (base_value) + 1 (num_additional_bits)
// + 3 bytes tail padding for natural u32 alignment.
#[cfg(target_endian = "little")]
const _: [(); 12] = [(); size_of::<Entry>()];

/// Compact sequence-section FSE entry, mirroring upstream zstd's
/// `ZSTD_seqSymbol` exactly: no `symbol` field (the sequence-section
/// decoder reads `base_value` / `num_additional_bits` directly off
/// the active state and never needs the source byte). 8 bytes vs
/// the 12-byte HUF-grade `Entry`. Field order matches upstream zstd so the
/// init-state path can issue a single aligned 8-byte load.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
#[doc(hidden)]
pub struct SeqSymbol {
    /// Base index for the next state. Low bits read from the
    /// bitstream are added to this value to produce the final state.
    pub new_state: u16,
    /// Bits to read from the stream when transitioning out of this
    /// state.
    pub num_bits: u8,
    /// Bits to read after the symbol decodes, to add to `base_value`.
    /// Upstream zstd `ZSTD_seqSymbol::nbAdditionalBits`.
    pub num_additional_bits: u8,
    /// Pre-computed code baseline. `actual_value = base_value +
    /// extra_bits_read`. Upstream zstd `ZSTD_seqSymbol::baseValue`.
    pub base_value: u32,
}

#[cfg(target_endian = "little")]
const _: [(); 0] = [(); core::mem::offset_of!(SeqSymbol, new_state)];
#[cfg(target_endian = "little")]
const _: [(); 2] = [(); core::mem::offset_of!(SeqSymbol, num_bits)];
#[cfg(target_endian = "little")]
const _: [(); 3] = [(); core::mem::offset_of!(SeqSymbol, num_additional_bits)];
#[cfg(target_endian = "little")]
const _: [(); 4] = [(); core::mem::offset_of!(SeqSymbol, base_value)];
#[cfg(target_endian = "little")]
const _: [(); 8] = [(); size_of::<SeqSymbol>()];

/// Common interface every FSE-table entry type must provide to
/// participate in [`FSETable`] / [`FSEDecoder`]. Both [`Entry`]
/// (HUF-weight stream) and [`SeqSymbol`] (sequence-section LL / ML /
/// OF) implement it.
///
/// `num_bits` / `new_state` are the state-transition fields read on
/// the hot path; `from_raw` is the build-time constructor used by
/// `build_decoding_table`. The HUF entry stores `symbol` directly,
/// the sequence-section entry derives `base_value` /
/// `num_additional_bits` from caller-provided meta and discards
/// `symbol`.
#[doc(hidden)]
/// Sequence-axis meta source fused into the table-build loop. The upstream zstd
/// fills `baseValue` / `nbAdditionalBits` during table construction
/// (`ZSTD_buildSeqTable`); building first and enriching in a second
/// full-table pass doubles the entry traffic on table-dense frames.
#[derive(Clone, Copy)]
pub(crate) enum SeqMeta<'a> {
    /// No sequence meta (Huffman-weight / literal tables).
    None,
    /// Packed LL / ML meta: `base = m & 0x00FF_FFFF`, `bits = m >> 24`.
    Packed(&'a [u32]),
    /// Closed-form OF meta: `base = 1 << code`, `bits = code` for
    /// `code < 32`; zeros otherwise.
    Offsets,
}

pub trait FseEntry: Copy + Default {
    /// Bits to read on state transition. Hot-path access.
    fn num_bits(&self) -> u8;
    /// Base index for next state. Hot-path access.
    fn new_state(&self) -> u16;
    /// Attach sequence meta (`base_value` / `num_additional_bits`) during
    /// the build loop. Entries without those fields keep the no-op
    /// default.
    #[inline(always)]
    fn with_seq_meta(self, base_value: u32, num_additional_bits: u8) -> Self {
        let _ = (base_value, num_additional_bits);
        self
    }

    /// Build-time constructor from the raw (new_state, symbol, num_bits)
    /// triple produced by `build_decoding_table`. Implementations may
    /// drop `symbol` (e.g. [`SeqSymbol`] mirrors upstream zstd `ZSTD_seqSymbol`
    /// which has no per-state byte) — the sequence-section decoder
    /// fills `base_value` / `num_additional_bits` via a separate enrich
    /// pass.
    fn from_raw(new_state: u16, symbol: u8, num_bits: u8) -> Self;
}

impl FseEntry for Entry {
    #[inline(always)]
    fn num_bits(&self) -> u8 {
        self.num_bits
    }
    #[inline(always)]
    fn new_state(&self) -> u16 {
        self.new_state
    }
    #[inline(always)]
    fn from_raw(new_state: u16, symbol: u8, num_bits: u8) -> Self {
        Entry {
            new_state,
            symbol,
            num_bits,
            base_value: 0,
            num_additional_bits: 0,
        }
    }
}

impl FseEntry for SeqSymbol {
    #[inline(always)]
    fn with_seq_meta(mut self, base_value: u32, num_additional_bits: u8) -> Self {
        self.base_value = base_value;
        self.num_additional_bits = num_additional_bits;
        self
    }

    #[inline(always)]
    fn num_bits(&self) -> u8 {
        self.num_bits
    }
    #[inline(always)]
    fn new_state(&self) -> u16 {
        self.new_state
    }
    #[inline(always)]
    fn from_raw(new_state: u16, _symbol: u8, num_bits: u8) -> Self {
        // `symbol` is intentionally dropped: the upstream zstd `ZSTD_seqSymbol`
        // layout has no per-state byte. LL / ML / OF tables fill the
        // `base_value` / `num_additional_bits` fields via the enrich
        // post-pass that follows `build_decoder`.
        SeqSymbol {
            new_state,
            num_bits,
            num_additional_bits: 0,
            base_value: 0,
        }
    }
}

/// This value is added to the first 4 bits of the stream to determine the
/// `Accuracy_Log`
const ACC_LOG_OFFSET: u8 = 5;
const ENTRY_MAX_ACCURACY_LOG: u8 = 16;

fn highest_bit_set(x: u32) -> u32 {
    assert!(x > 0);
    u32::BITS - x.leading_zeros()
}

//utility functions for building the decoding table from probabilities
/// Calculate the position of the next entry of the table given the current
/// position and size of the table.
/// In-order spread scratch for the fast `build_decoding_table_inner` path:
/// the largest table reaching it is `1 << 9 = 512` entries (the
/// `table_size <= CAP` guard in `build_decoding_table_inner` rejects anything
/// larger before the spread runs), plus 8 bytes of slack so the 8-byte symbol
/// lay-down can over-write past the last symbol without bounds-checking each
/// tail write.
const FSE_FAST_SPREAD_BUF: usize = 512 + 8;

fn next_position(mut p: usize, table_size: usize) -> usize {
    p += (table_size >> 1) + (table_size >> 3) + 3;
    p &= table_size - 1;
    p
}
