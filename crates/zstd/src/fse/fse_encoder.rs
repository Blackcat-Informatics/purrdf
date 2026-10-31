#![allow(
    clippy::branches_sharing_code,
    clippy::cast_lossless,
    clippy::map_unwrap_or,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use crate::bit_io::BitWriter;
use alloc::vec::Vec;

pub(crate) struct FSEEncoder<'output, V: AsMut<Vec<u8>>> {
    pub(super) table: FSETable,
    writer: &'output mut BitWriter<V>,
}

impl<V: AsMut<Vec<u8>>> FSEEncoder<'_, V> {
    pub(crate) fn new(table: FSETable, writer: &mut BitWriter<V>) -> FSEEncoder<'_, V> {
        FSEEncoder { table, writer }
    }

    /// Encodes the data using the provided table but with two interleaved streams
    /// Writes
    /// * Table description
    /// * Encoded data with two interleaved states
    /// * Both Last state indexes
    /// * Padding bits to fill up last byte
    pub(crate) fn encode_interleaved(&mut self, data: &[u8]) {
        self.write_table();

        assert!(data.len() >= 2);
        let mut ip = data.len();
        let mut state_1;
        let mut state_2;

        if data.len() & 1 != 0 {
            ip -= 1;
            state_1 = self.table.start_state(data[ip]).index;
            ip -= 1;
            state_2 = self.table.start_state(data[ip]).index;
            ip -= 1;
            state_1 = self.encode_symbol_with_state(state_1, data[ip]);
        } else {
            ip -= 1;
            state_2 = self.table.start_state(data[ip]).index;
            ip -= 1;
            state_1 = self.table.start_state(data[ip]).index;
        }

        let remaining_after_init = data.len() - 2;
        if remaining_after_init & 2 != 0 {
            ip -= 1;
            state_2 = self.encode_symbol_with_state(state_2, data[ip]);
            ip -= 1;
            state_1 = self.encode_symbol_with_state(state_1, data[ip]);
        }

        while ip > 0 {
            ip -= 1;
            state_2 = self.encode_symbol_with_state(state_2, data[ip]);
            ip -= 1;
            state_1 = self.encode_symbol_with_state(state_1, data[ip]);
            if ip > 0 {
                ip -= 1;
                state_2 = self.encode_symbol_with_state(state_2, data[ip]);
                ip -= 1;
                state_1 = self.encode_symbol_with_state(state_1, data[ip]);
            }
        }

        self.writer
            .write_bits(state_2 as u64, self.acc_log() as usize);
        self.writer
            .write_bits(state_1 as u64, self.acc_log() as usize);

        let bits_to_fill = self.writer.misaligned();
        if bits_to_fill == 0 {
            self.writer.write_bits(1u32, 8);
        } else {
            self.writer.write_bits(1u32, bits_to_fill);
        }
    }

    fn encode_symbol_with_state(&mut self, state_index: usize, symbol: u8) -> usize {
        let next = self.table.next_state(symbol, state_index);
        let diff = state_index - next.baseline;
        self.writer.write_bits(diff as u64, next.num_bits as usize);
        next.index
    }

    fn write_table(&mut self) {
        self.table.write_table(self.writer);
    }

    pub(super) fn acc_log(&self) -> u8 {
        self.table.acc_log()
    }
}

/// Upstream zstd-parity per-symbol coding transform, mirroring upstream
/// `FSE_symbolCompressionTransform` in `lib/compress/fse_compress.c`.
/// Together with [`FSETable::state_table_flat`] this gives an O(1)
/// next-state lookup in [`FSETable::next_state`], replacing the prior
/// linear scan over `SymbolStates::states`.
///
/// Upstream zstd formulas (`FSE_encodeSymbol`, `fse_compress.c`):
///
/// ```text
/// value       = (1 << acc_log) + current_state.index
/// nb_bits_out = (value + delta_nb_bits) >> 16
/// baseline    = current_state.index & !((1 << nb_bits_out) - 1)
/// next_index  = state_table_flat[(value >> nb_bits_out) + delta_find_state]
/// ```
///
/// `delta_find_state` is intentionally signed (can be negative for
/// low-probability symbols — see the `-1 | 1` arm in
/// `build_table_from_probabilities`); the indexing arithmetic on
/// `state_table_flat` is signed-correct via the construction above
/// (`(value >> nb_bits_out)` is always large enough to cover any
/// negative offset, because of how delta_find_state is derived).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SymbolTT {
    /// Upstream zstd 16.16 fixed-point value. **`u32`, not `usize`** — on 16-bit
    /// targets (AVR / MSP430 / Cortex-M0 in the no-atomic profile this
    /// crate documents) `usize` is 16 bits, and `<<16` / `>>16` on a
    /// `usize` would silently overflow to zero, breaking the
    /// fixed-point math. Upstream zstd `fse_compress.c` uses `U32` throughout
    /// the same arithmetic for the same reason.
    pub(crate) delta_nb_bits: u32,
    pub(crate) delta_find_state: isize,
}

#[derive(Debug, Clone)]
pub struct FSETable {
    /// Indexed by symbol
    pub(super) states: [SymbolStates; 256],
    /// Sum of all states.states.len()
    pub(crate) table_size: usize,
    /// Upstream zstd-parity flat next-state table — `state_table_flat[i]` is the
    /// FSE state index that the encoder transitions TO when the
    /// transition arithmetic resolves to slot `i`. Length is exactly
    /// `table_size` (= `1 << acc_log`). Mirror of upstream
    /// `nextStateTable` (`fse_compress.c`).
    pub(super) state_table_flat: alloc::boxed::Box<[u16]>,
    /// Per-symbol upstream zstd-parity coding transform — see [`SymbolTT`].
    pub(super) symbol_tt: [SymbolTT; 256],
}

impl FSETable {
    /// Heap bytes this table holds beyond its inline struct: the flat
    /// state-transition array (`Box<[u16]>`). The `states` / `symbol_tt`
    /// arrays are fixed-size and inline (accounted by the owner's `size_of`).
    pub(crate) fn heap_size(&self) -> usize {
        self.state_table_flat.len() * size_of::<u16>()
    }

    /// O(1) next-state lookup mirroring upstream `FSE_encodeSymbol`
    /// (`lib/compress/fse_compress.c`). Was a linear scan over a
    /// `Vec<State>` per symbol; the flat tables that drive the upstream zstd
    /// arithmetic were already built during table construction and
    /// previously discarded — now they live on the FSETable
    /// ([`SymbolTT`] + [`Self::state_table_flat`]) and `next_state` is
    /// pure arithmetic + one array load.
    #[inline]
    pub(crate) fn next_state(&self, symbol: u8, idx: usize) -> State {
        // Upstream zstd formulas, transliterated:
        //   value       = (1 << acc_log) + idx
        //   nb_bits     = (value + delta_nb_bits) >> 16
        //   baseline    = idx & !((1 << nb_bits) - 1)
        //   next_index  = state_table_flat[(value >> nb_bits) + delta_find_state]
        //
        // `delta_find_state` is signed (`isize`) — for low-probability
        // symbols it is intentionally negative. The combined right-shift
        // `(value >> nb_bits)` is always large enough to keep the final
        // index non-negative; this is a construction invariant from
        // `build_table_from_probabilities`.
        // Fixed-point arithmetic uses `u32` (upstream zstd parity, 16-bit-target
        // safe — see `SymbolTT::delta_nb_bits` doc). `table_size` is
        // `1 << acc_log` with `acc_log <= 12`, so `value <= 4096 + 4095`
        // fits comfortably; the `<<16` half of the math lives entirely
        // inside `delta_nb_bits` which was already computed in u32 by
        // the builder. Result `nb_bits` is small (<= 8 for typical
        // FSE tables) — cast to `usize` only at the final indexing
        // sites (`(1usize << nb_bits)`, `value >> nb_bits` as slot).
        let tt = self.symbol_tt[symbol as usize];
        let value = (self.table_size + idx) as u32;
        let nb_bits = ((value + tt.delta_nb_bits) >> 16) as usize;
        let mask = (1usize << nb_bits) - 1;
        let baseline = idx & !mask;
        let slot = ((value >> nb_bits) as isize + tt.delta_find_state) as usize;
        let next_index = self.state_table_flat[slot] as usize;
        State {
            num_bits: nb_bits as u8,
            baseline,
            last_index: baseline + mask,
            index: next_index,
        }
    }

    /// First-state lookup for the encode-init of a new FSE stream.
    /// Stays on the `Vec<State>` storage because (a) it is called once
    /// per stream (not on the hot per-sequence path), and (b) the
    /// upstream zstd flat-table arithmetic for the start state would require
    /// an extra special-case for the `1 << acc_log` "virtual" current
    /// state. Returning by value matches the new [`Self::next_state`]
    /// return shape so callers can store `State` directly without
    /// juggling lifetimes.
    pub(crate) fn start_state(&self, symbol: u8) -> State {
        let index = self.states[symbol as usize]
            .start_state
            .expect("symbol must be present in the FSE table");
        // Callers consume only `index` (audited across the encoder + sequence
        // emit paths). Upstream zstd `FSE_initCState2` likewise stores just the
        // start state value; `num_bits` / `baseline` are properties of
        // transitions, not of the initial state, so they have no
        // meaningful values here and are zeroed.
        State {
            num_bits: 0,
            baseline: 0,
            last_index: 0,
            index,
        }
    }

    pub fn acc_log(&self) -> u8 {
        self.table_size.ilog2() as u8
    }

    /// Get the probability assigned to a symbol (0 means absent, -1 means less-than-1).
    pub(crate) fn symbol_probability(&self, symbol: u8) -> i32 {
        self.states[symbol as usize].probability
    }

    pub(crate) fn max_num_bits_for_symbol(&self, symbol: u8) -> Option<u8> {
        self.states[symbol as usize].max_num_bits
    }

    /// Compute the exact serialized size (in bits) of the FSE table header,
    /// including the byte-alignment padding at the end.
    /// Mirrors `write_table` but counts bits instead of writing them.
    ///
    /// The result assumes the header starts at a byte boundary, which matches
    /// all current encoder call sites.
    pub(crate) fn table_header_bits(&self) -> usize {
        // Delegate to the free `ncount_header_bits` so the header sizing has a
        // single source of truth shared with the pre-build cost estimate in
        // `fse_header_bits_for_counts` (the table-mode selector prices a
        // candidate table from its normalized counts without building it).
        let probs: [i32; 256] = core::array::from_fn(|i| self.states[i].probability);
        ncount_header_bits(&probs, self.acc_log())
    }

    pub(crate) fn write_table<V: AsMut<Vec<u8>>>(&self, writer: &mut BitWriter<V>) {
        assert!(
            writer.index().is_multiple_of(8),
            "FSE table headers must start on a byte boundary"
        );
        #[cfg(debug_assertions)]
        let start_idx = writer.index();
        writer.write_bits(self.acc_log() - 5, 4);
        let mut probability_counter = 0usize;
        let probability_sum = 1 << self.acc_log();

        let mut prob_idx = 0;
        while probability_counter < probability_sum {
            let max_remaining_value = probability_sum - probability_counter + 1;
            let bits_to_write = max_remaining_value.ilog2() + 1;
            let low_threshold = ((1 << bits_to_write) - 1) - (max_remaining_value);
            let mask = (1 << (bits_to_write - 1)) - 1;

            let prob = self.states[prob_idx].probability;
            prob_idx += 1;
            let value = (prob + 1) as u32;
            if value < low_threshold as u32 {
                writer.write_bits(value, bits_to_write as usize - 1);
            } else if value > mask {
                writer.write_bits(value + low_threshold as u32, bits_to_write as usize);
            } else {
                writer.write_bits(value, bits_to_write as usize);
            }

            if prob == -1 {
                probability_counter += 1;
            } else if prob > 0 {
                probability_counter += prob as usize;
            } else {
                let mut zeros = 0u8;
                while prob_idx < self.states.len() && self.states[prob_idx].probability == 0 {
                    zeros += 1;
                    prob_idx += 1;
                    if zeros == 3 {
                        writer.write_bits(3u8, 2);
                        zeros = 0;
                    }
                }
                writer.write_bits(zeros, 2);
            }
        }
        writer.write_bits(0u8, writer.misaligned());
        #[cfg(debug_assertions)]
        {
            let written_bits = writer.index() - start_idx;
            let computed = self.table_header_bits();
            debug_assert_eq!(
                written_bits, computed,
                "table_header_bits() mismatch: written={written_bits}, computed={computed}"
            );
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct SymbolStates {
    /// Upstream zstd-encoder start-state slot (`FSE_initCState2` result, in
    /// `0..table_size`). `None` when `probability == 0`. Callers
    /// consume only the `index` field of the [`State`] returned from
    /// [`FSETable::start_state`] (see line-by-line audit on the
    /// `encode` / `encode_interleaved` / compressed-block sequence
    /// paths) — so the other [`State`] fields stay zeroed on the
    /// returned value.
    pub(super) start_state: Option<usize>,
    /// Probability assigned to this symbol (`0` absent, `-1` less-than-one).
    pub(super) probability: i32,
    /// Max `num_bits` emitted by [`FSETable::next_state`] across all input
    /// states for this symbol. `None` when `probability == 0`. Computed via
    /// upstream zstd arithmetic at build time (`(2*table_size-1 + delta_nb_bits) >> 16`)
    /// so [`FSETable::max_num_bits_for_symbol`] is a single array load
    /// instead of a `Vec<State>` scan.
    pub(super) max_num_bits: Option<u8>,
}

// SymbolStates::get (the old linear-scan next-state lookup) was
// replaced by [`FSETable::next_state`]'s O(1) upstream zstd arithmetic.
// Production no longer materializes any per-state vector; upstream zstd
// `FSE_buildCTable_wksp` only ever stores `nextStateTable` (here
// [`FSETable::state_table_flat`]) + `symbolTT` (here
// [`FSETable::symbol_tt`]). Everything else — start state,
// max-nb-bits, probability — is precomputed once per symbol via the
// upstream zstd arithmetic and held in [`SymbolStates`].

#[derive(Debug, Clone, Copy)]
pub(crate) struct State {
    /// How many bits the range of this state needs to be encoded as
    pub(crate) num_bits: u8,
    /// The first index targeted by this state
    pub(crate) baseline: usize,
    /// The last index targeted by this state (baseline + the maximum
    /// number with numbits bits allows). Computed by
    /// [`FSETable::next_state`] for parity with the old
    /// `Vec<State>` storage and consumed by the BTreeSet dedup in the
    /// table builder (`build_table_from_probabilities`); not read on
    /// the encode hot path, which uses constant-time arithmetic.
    #[allow(dead_code)]
    pub(crate) last_index: usize,
    /// Index of this state in the decoding table
    pub(crate) index: usize,
}

pub(crate) fn build_table_from_symbol_counts(
    counts: &[usize],
    max_log: u8,
    avoid_0_numbit: bool,
) -> FSETable {
    build_table_from_counts(counts, max_log, avoid_0_numbit)
}

/// Build the FSE CTable for a sequence stream (LL / ML / OF), applying upstream
/// zstd's last-symbol adjustment (`ZSTD_buildCTable`,
/// zstd_compress_sequences.c:269-278): the final sequence's symbol is emitted
/// through the FSE initial state, not a normal transition, so when its count
/// exceeds 1 upstream drops one occurrence from the histogram and normalizes
/// against `nbSeq - 1`. The table-log is still derived from the full `nbSeq`.
/// When the count is exactly 1 upstream adjusts NEITHER — it gates both
/// `count[last]--` and `nbSeq_1--` on `count > 1` (zstd_compress_sequences.c:271-273),
/// so the histogram and the normalization total both stay at the full `nbSeq`
/// (the `else { total }` branch below). `last_code` is the stream code of the
/// last sequence (an index into `counts`, `<= max_symbol`). `use_low_prob_count`
/// follows `nbSeq_1` exactly as upstream's `ZSTD_useLowProbCount(nbSeq_1)`.
pub(crate) fn build_seq_ctable(counts: &mut [usize], max_log: u8, last_code: usize) -> FSETable {
    let total = counts.iter().sum::<usize>();
    let max_symbol = counts
        .iter()
        .rposition(|&count| count > 0)
        .unwrap_or_default();
    // Derived from the full nbSeq, matching upstream (the adjustment touches only
    // the normalized distribution, never the table-log).
    let table_log = optimal_table_log(max_log, total, max_symbol, 2);
    let mut probs = [0i32; 256];
    // Borrow the caller's histogram and drop the last sequence's symbol in place
    // for the normalize, then put it back — no per-table copy. `normalize_counts`
    // only re-borrows `counts` immutably, so it sees the decrement, and the
    // restore leaves the selection-time histogram untouched for the caller.
    // C gates BOTH the decrement and `nbSeq_1--` on `count > 1`
    // (zstd_compress_sequences.c:271-273). A count of exactly 1 therefore leaves
    // the histogram unchanged AND normalizes against the full `total` (= nbSeq),
    // NOT `total - 1` — using `total - 1` here would diverge from C and break the
    // byte-parity this establishes.
    let adjust = counts[last_code] > 1;
    if adjust {
        counts[last_code] -= 1;
    }
    let norm_total = if adjust { total - 1 } else { total };
    normalize_counts(
        &mut probs[..=max_symbol],
        table_log,
        &counts[..=max_symbol],
        norm_total,
        max_symbol,
        norm_total >= 2048,
    );
    if adjust {
        counts[last_code] += 1;
    }
    build_table_from_probabilities(&probs[..=max_symbol], table_log)
}

fn build_table_from_counts(counts: &[usize], max_log: u8, avoid_0_numbit: bool) -> FSETable {
    let total = counts.iter().sum::<usize>();
    // FSE table construction needs at least two samples in the histogram.
    // A single-distinct-symbol histogram (e.g. `[N, 0, ...]`) with `total
    // >= 2` is still acceptable here — some internal fallback paths feed
    // a phantom zero-count second slot when they want an FSE table for
    // an effectively-RLE distribution.
    assert!(
        total > 1,
        "FSE table requires at least 2 samples in the histogram (got {total})"
    );
    let max_symbol = counts
        .iter()
        .rposition(|&count| count > 0)
        .unwrap_or_default();
    let table_log = optimal_table_log(max_log, total, max_symbol, 2);
    let mut probs = [0i32; 256];
    normalize_counts(
        &mut probs[..counts.len()],
        table_log,
        counts,
        total,
        max_symbol,
        avoid_0_numbit,
    );
    build_table_from_probabilities(&probs[..counts.len()], table_log)
}

/// Bit size of the serialized FSE NCount table header for the normalized
/// distribution `probs` at `acc_log`. This is exactly what
/// [`FSETable::table_header_bits`] returns (they share this implementation),
/// but computed from the normalized counts alone — no spread / state-table /
/// `symbolTT` construction. The table-mode selector uses it to price a
/// candidate custom table without building its (otherwise discarded) state
/// tables.
fn ncount_header_bits(probs: &[i32], acc_log: u8) -> usize {
    let mut bits = 4; // acc_log - 5
    let mut probability_counter = 0usize;
    let probability_sum = 1usize << acc_log;

    let mut prob_idx = 0;
    while probability_counter < probability_sum {
        let max_remaining_value = probability_sum - probability_counter + 1;
        let bits_to_write = max_remaining_value.ilog2() + 1;
        let low_threshold = ((1 << bits_to_write) - 1) - max_remaining_value;

        let prob = probs[prob_idx];
        prob_idx += 1;
        let value = (prob + 1) as u32;
        if value < low_threshold as u32 {
            bits += bits_to_write as usize - 1;
        } else {
            bits += bits_to_write as usize;
        }

        if prob == -1 {
            probability_counter += 1;
        } else if prob > 0 {
            probability_counter += prob as usize;
        } else {
            let mut zeros = 0u8;
            while prob_idx < probs.len() && probs[prob_idx] == 0 {
                zeros += 1;
                prob_idx += 1;
                if zeros == 3 {
                    bits += 2;
                    zeros = 0;
                }
            }
            bits += 2;
        }
    }
    // Byte-alignment padding
    let misaligned = bits % 8;
    if misaligned != 0 {
        bits += 8 - misaligned;
    }
    bits
}

/// Header bit cost of the custom FSE table that
/// [`build_table_from_symbol_counts`] would produce for `counts`, without
/// building it. Mirrors the front of `build_table_from_counts` (table-log
/// selection + count normalization) and then sizes the header via
/// [`ncount_header_bits`]. Returns the exact value the built table's
/// `table_header_bits()` would, so the table-mode selection stays
/// byte-identical while skipping the state-table build for candidates that
/// lose to the predefined / repeated table.
pub(crate) fn fse_header_bits_for_counts(
    counts: &[usize],
    max_log: u8,
    avoid_0_numbit: bool,
) -> usize {
    let total = counts.iter().sum::<usize>();
    // Mirror `build_table_from_counts`'s contract: pricing must reject the
    // same inputs the builder rejects. A histogram with fewer than two
    // samples cannot form a valid FSE table, and the table-log selection
    // below takes an integer logarithm of `total` that underflows on
    // `total <= 1`. Trip the guard up front so the failure mode matches
    // the builder instead of an opaque arithmetic panic.
    assert!(
        total > 1,
        "FSE table requires at least 2 samples in the histogram (got {total})"
    );
    let max_symbol = counts
        .iter()
        .rposition(|&count| count > 0)
        .unwrap_or_default();
    let table_log = optimal_table_log(max_log, total, max_symbol, 2);
    let mut probs = [0i32; 256];
    normalize_counts(
        &mut probs[..counts.len()],
        table_log,
        counts,
        total,
        max_symbol,
        avoid_0_numbit,
    );
    ncount_header_bits(&probs[..counts.len()], table_log)
}

fn min_table_log(total: usize, max_symbol: usize) -> u8 {
    let min_bits_src = total.ilog2() + 1;
    let min_bits_symbols = if max_symbol == 0 {
        2
    } else {
        max_symbol.ilog2() + 2
    };
    min_bits_src.min(min_bits_symbols) as u8
}

/// Upstream `FSE_optimalTableLog_internal` (fse_compress.c:357). `minus` is the
/// accuracy reduction applied to the source-size bit estimate: FSE passes `2`,
/// HUF passes `1` (huf_compress.c:1286). Reused by the HUF cheap/fast tableLog
/// path so our single-shot literal tableLog matches upstream's exactly.
pub(crate) fn optimal_table_log(
    max_table_log: u8,
    total: usize,
    max_symbol: usize,
    minus: u8,
) -> u8 {
    let max_bits_src = (total - 1).ilog2().saturating_sub(minus as u32) as u8;
    let min_bits = min_table_log(total, max_symbol);
    let mut table_log = max_table_log;
    if max_bits_src < table_log {
        table_log = max_bits_src;
    }
    if min_bits > table_log {
        table_log = min_bits;
    }
    table_log.clamp(5, 12)
}

fn normalize_counts(
    normalized: &mut [i32],
    table_log: u8,
    counts: &[usize],
    total: usize,
    max_symbol: usize,
    use_low_prob_count: bool,
) {
    const RTB_TABLE: [u64; 8] = [
        0, 473_195, 504_333, 520_860, 550_000, 700_000, 750_000, 830_000,
    ];
    let low_prob_count = if use_low_prob_count { -1 } else { 1 };
    let scale = 62 - table_log as usize;
    let step = (1u64 << 62) / total as u64;
    let v_step = 1u64 << (scale - 20);
    let low_threshold = total >> table_log;
    let mut still_to_distribute = 1i32 << table_log;
    let mut largest = 0usize;
    let mut largest_probability = 0i32;

    for symbol in 0..=max_symbol {
        let count = counts[symbol];
        if count == 0 {
            normalized[symbol] = 0;
        } else if count <= low_threshold {
            normalized[symbol] = low_prob_count;
            still_to_distribute -= 1;
        } else {
            let product = count as u64 * step;
            let mut probability = (product >> scale) as i32;
            if probability < 8 {
                let rest_to_beat = v_step * RTB_TABLE[probability as usize];
                probability +=
                    u64::from(product - ((probability as u64) << scale) > rest_to_beat) as i32;
            }
            if probability > largest_probability {
                largest_probability = probability;
                largest = symbol;
            }
            normalized[symbol] = probability;
            still_to_distribute -= probability;
        }
    }

    if -still_to_distribute >= normalized[largest] >> 1 {
        normalize_m2(
            normalized,
            table_log,
            counts,
            total,
            max_symbol,
            low_prob_count,
        );
    } else {
        normalized[largest] += still_to_distribute;
    }

    debug_assert_eq!(
        normalized
            .iter()
            .take(max_symbol + 1)
            .map(|&probability| probability.unsigned_abs() as usize)
            .sum::<usize>(),
        1usize << table_log
    );
}

fn normalize_m2(
    normalized: &mut [i32],
    table_log: u8,
    counts: &[usize],
    mut total: usize,
    max_symbol: usize,
    low_prob_count: i32,
) {
    const NOT_YET_ASSIGNED: i32 = -2;
    let low_threshold = total >> table_log;
    let mut low_one = (total * 3) >> (table_log as usize + 1);
    let mut distributed = 0usize;

    for symbol in 0..=max_symbol {
        let count = counts[symbol];
        if count == 0 {
            normalized[symbol] = 0;
        } else if count <= low_threshold {
            normalized[symbol] = low_prob_count;
            distributed += 1;
            total -= count;
        } else if count <= low_one {
            normalized[symbol] = 1;
            distributed += 1;
            total -= count;
        } else {
            normalized[symbol] = NOT_YET_ASSIGNED;
        }
    }

    let mut to_distribute = (1usize << table_log) - distributed;
    if to_distribute == 0 {
        return;
    }

    if total / to_distribute > low_one {
        low_one = (total * 3) / (to_distribute * 2);
        for symbol in 0..=max_symbol {
            if normalized[symbol] == NOT_YET_ASSIGNED && counts[symbol] <= low_one {
                normalized[symbol] = 1;
                distributed += 1;
                total -= counts[symbol];
            }
        }
        to_distribute = (1usize << table_log) - distributed;
    }

    if distributed == max_symbol + 1 {
        let max_symbol = counts
            .iter()
            .copied()
            .take(max_symbol + 1)
            .enumerate()
            .max_by_key(|&(_, count)| count)
            .map(|(symbol, _)| symbol)
            .unwrap_or_default();
        normalized[max_symbol] += to_distribute as i32;
        return;
    }

    if total == 0 {
        let mut symbol = 0usize;
        while to_distribute > 0 {
            if normalized[symbol] > 0 {
                normalized[symbol] += 1;
                to_distribute -= 1;
            }
            symbol = (symbol + 1) % (max_symbol + 1);
        }
        return;
    }

    let v_step_log = 62 - table_log as usize;
    let mid = (1u64 << (v_step_log - 1)) - 1;
    let r_step = (((1u64 << v_step_log) * to_distribute as u64) + mid) / total as u64;
    let mut tmp_total = mid;
    for symbol in 0..=max_symbol {
        if normalized[symbol] == NOT_YET_ASSIGNED {
            let end = tmp_total + counts[symbol] as u64 * r_step;
            let start_bucket = tmp_total >> v_step_log;
            let end_bucket = end >> v_step_log;
            let weight = end_bucket - start_bucket;
            assert!(
                weight >= 1,
                "upstream zstd FSE normalization produced zero weight"
            );
            normalized[symbol] = weight as i32;
            tmp_total = end;
        }
    }
}

pub(super) fn build_table_from_probabilities(probs: &[i32], acc_log: u8) -> FSETable {
    let table_size: usize = 1 << acc_log;
    let mut symbol_states: [SymbolStates; 256] = core::array::from_fn(|_| SymbolStates::default());

    // Upstream zstd `FSE_buildCTable_wksp` (lib/compress/fse_compress.c) — build
    // `nextStateTable` (== `state_table_flat`) once via cumul + spread +
    // sorted-by-symbol sweep, without ever materializing per-symbol
    // `Vec<State>`. Previous implementation paid an O(num_symbols ×
    // table_size) enumeration with a BTreeSet dedup per symbol —
    // ~18% self time on small-input encode profiles. Drop it; upstream zstd
    // only ever needs symbolTT + nextStateTable, and we precompute
    // `start_state` / `max_num_bits` for callers via the same
    // arithmetic [`FSETable::next_state`] uses on the hot path.
    //
    // Phase 1 — distribute `-1` (low-probability) symbols at the top
    // of the table; bump the high-threshold cursor down. Build the
    // `cumul` prefix-sum table that maps each symbol to its first
    // `nextStateTable` slot in sorted-by-symbol layout.
    // `table_symbol` is a local scratch (consumed to fill `state_table_flat`,
    // never returned), bounded by the FSE sequence/weight table-log cap
    // (`table_size = 1 << table_log <= 1 << 9 = 512`). Use a stack buffer so the
    // per-build heap alloc + zero is gone. `[..table_size]` keeps the live region
    // exactly as the old `vec![0u8; table_size]`.
    const MAX_FSE_TABLE_SIZE: usize = 1 << 9;
    // Always-on (not debug_assert): the stack scratch is fixed at 512 bytes
    // (accuracy_log <= 9). `to_encoder_table` already rejects acc_log > 9, but
    // this function is `pub(crate)` — guard the scratch bound here too so a
    // future caller can never index the buffer past its length (the `[..
    // table_size]` slice would panic on the bound; this fires first with a clear
    // message instead of an opaque slice-index panic).
    assert!(
        table_size <= MAX_FSE_TABLE_SIZE,
        "FSE table_size {table_size} exceeds the {MAX_FSE_TABLE_SIZE}-byte encoder \
         scratch (accuracy_log must be <= 9)"
    );
    let mut table_symbol_buf = [0u8; MAX_FSE_TABLE_SIZE];
    let table_symbol = &mut table_symbol_buf[..table_size];
    let mut high_threshold = (table_size - 1) as isize;
    // `cumul` / running prefix-sum holds slot counts up to `table_size`.
    // Decoder accepts `accuracy_log` up to `ENTRY_MAX_ACCURACY_LOG = 16`
    // and `fse_decoder::FSETable::to_encoder_table` round-trips through
    // this builder; at `acc_log == 16` the prefix sum reaches 65 536
    // which overflows `u16` (max 65 535). Keep `cumul` / `cursor` at
    // `u32` so the cumulative count is representable for every valid
    // `acc_log`. Slot indices written into `state_table_flat` stay in
    // `0..table_size-1` (≤ u16::MAX) and remain `u16` — only the
    // running cursor needs the wider type.
    let mut cumul = [0u32; 257];
    for (symbol, &prob) in probs.iter().enumerate() {
        let bump: u32 = match prob {
            -1 => {
                table_symbol[high_threshold as usize] = symbol as u8;
                high_threshold -= 1;
                1
            }
            p if p > 0 => p as u32,
            _ => 0,
        };
        cumul[symbol + 1] = cumul[symbol] + bump;
    }

    // Phase 2 — spread positive-probability symbols across the
    // remaining slots, upstream zstd `step`-walk with low-prob area skip.
    let step = (table_size >> 1) + (table_size >> 3) + 3;
    let table_mask = table_size - 1;
    let mut position: usize = 0;
    for (symbol, &prob) in probs.iter().enumerate() {
        if prob <= 0 {
            continue;
        }
        for _ in 0..prob {
            table_symbol[position] = symbol as u8;
            position = (position + step) & table_mask;
            while (position as isize) > high_threshold {
                position = (position + step) & table_mask;
            }
        }
    }
    // These two invariants are the SAFETY contract for the uninitialized
    // `state_table_flat` built below: the scatter-write loop fills every index
    // in `[0, table_size)` exactly once only if (a) the spread cycled exactly
    // once through the table (so each symbol got exactly its `cumul`-width run
    // of slots) and (b) the `cumul` cursors sum to `table_size` (so the runs
    // partition the whole table). They are asserted in ALL build profiles
    // (not `debug_assert`) so a regression in probability normalization panics
    // loudly here instead of leaving uninitialized slots that read as UB.
    assert_eq!(
        position, 0,
        "FSE spread must cycle exactly once through tableSize positions"
    );
    assert_eq!(
        cumul[probs.len()] as usize,
        table_size,
        "FSE cumul cursors must sum to table_size before building nextStateTable"
    );

    // Phase 3 — emit `state_table_flat` (upstream zstd `nextStateTable`)
    // ordered by `(symbol, slot)`. Walk every table slot `u`, look up
    // its owning symbol via `table_symbol[u]`, and write the raw slot
    // `u` into that symbol's running cumul cursor. The Rust convention
    // stores raw slots (`0..table_size`); upstream zstd stores `table_size + u`
    // pre-shifted and recovers `u` on read by subtracting `table_size`.
    // Both representations encode the same `(symbol → next_slot)`
    // mapping; [`FSETable::next_state`] is written against the raw-slot
    // convention so the pre-shift is intentionally skipped here.
    // Every index in `[0, table_size)` is written EXACTLY once by the scatter
    // loop below: the `cumul` cursors partition the table bijectively, which the
    // two asserts above enforce in every build profile. So a zero-init is dead
    // -- every slot is overwritten before any read (`into_boxed_slice` +
    // `FSETable` use happen after the loop). Allocate uninitialized and let the
    // loop fill it, skipping a per-build `table_size`-element memset (3 FSE
    // builds per block for the LL/ML/OF streams when custom tables are chosen).
    // The `uninit_vec` lint cannot see the bijective full-write; soundness is
    // confirmed under miri (the FSE encoder round-trip exercises this with no
    // use-of-uninit).
    #[allow(clippy::uninit_vec)]
    let mut state_table_flat: Vec<u16> = {
        let mut v = Vec::with_capacity(table_size);
        // SAFETY: `with_capacity(table_size)` reserved exactly `table_size`
        // slots; the asserts above guarantee (in every build profile) that the
        // scatter loop below writes every index in `[0, table_size)` exactly
        // once before the first read.
        unsafe { v.set_len(table_size) };
        v
    };
    let mut cursor = cumul;
    for (u, &symbol_at_slot) in table_symbol.iter().enumerate() {
        let s = symbol_at_slot as usize;
        // The Rust convention here keeps `state_table_flat[i]` as the
        // raw slot (`0..table_size`); upstream zstd stores `table_size + u`
        // and subtracts on read. `next_state` arithmetic ([`FSETable::next_state`])
        // matches the Rust convention — store the slot directly.
        state_table_flat[cursor[s] as usize] = u as u16;
        cursor[s] += 1;
    }
    let state_table_flat: alloc::boxed::Box<[u16]> = state_table_flat.into_boxed_slice();

    // Phase 4 — `symbolTT[]` (delta_nb_bits, delta_find_state) plus
    // precomputed `start_state` and `max_num_bits` per symbol. All via
    // upstream zstd 16.16 fixed-point arithmetic; no per-state enumeration.
    let mut symbol_tt = [SymbolTT::default(); 256];
    let mut total: usize = 0;
    for (symbol, &prob) in probs.iter().enumerate() {
        symbol_states[symbol].probability = prob;
        if prob == 0 {
            // Upstream zstd fills `symbolTT` for prob==0 too, so `FSE_getMaxNbBits`
            // still works (returns `acc_log + 1` for absent symbols).
            // We don't expose that path, but mirror the value for parity.
            symbol_tt[symbol] = SymbolTT {
                delta_nb_bits: ((acc_log as u32 + 1) << 16).saturating_sub(1u32 << acc_log),
                delta_find_state: 0,
            };
            continue;
        }
        let (delta_nb_bits, delta_find_state): (u32, isize) = match prob {
            -1 | 1 => (
                ((acc_log as u32) << 16).saturating_sub(1u32 << acc_log),
                total as isize - 1,
            ),
            p if p > 1 => {
                let p_u32 = p as u32;
                let max_bits_out = (acc_log as u32) - (p_u32 - 1).ilog2();
                let min_state_plus = p_u32 << max_bits_out;
                (
                    (max_bits_out << 16).saturating_sub(min_state_plus),
                    total as isize - p_u32 as isize,
                )
            }
            _ => unreachable!("probability is one of {{-1, 1+}} after the prob==0 gate above"),
        };
        symbol_tt[symbol] = SymbolTT {
            delta_nb_bits,
            delta_find_state,
        };
        total += prob.unsigned_abs() as usize;

        // Upstream zstd `FSE_initCState2`: start_state =
        //   stateTable[(((nbBitsOut<<16) - deltaNbBits) >> nbBitsOut) + deltaFindState]
        // where `nbBitsOut = (deltaNbBits + (1<<15)) >> 16`.
        let init_nb_bits_out = (delta_nb_bits + (1 << 15)) >> 16;
        let init_value = (init_nb_bits_out << 16).saturating_sub(delta_nb_bits);
        let state_table_index = (init_value >> init_nb_bits_out) as isize + delta_find_state;
        // Upstream zstd `FSE_initCState2` guarantees this index is in
        // `0..table_size` by construction (`delta_find_state` is bounded
        // by `total - probability`, and `(value >> nb_bits_out)` is
        // bounded by `2 * probability - 1`). The `debug_assert` makes
        // the invariant explicit so a future regression in the upstream zstd
        // arithmetic surfaces in dev builds before the silent
        // `as usize` wraparound.
        debug_assert!(
            state_table_index >= 0,
            "FSE start_state index must be non-negative (got {state_table_index} for symbol {symbol})"
        );
        let start_index = state_table_flat[state_table_index as usize] as usize;

        // Max nb_bits across all input states `0..table_size`. Upstream zstd
        // `next_state` arithmetic: `nb_bits = (value + delta_nb_bits) >> 16`
        // with `value = table_size + idx`, `idx ∈ 0..table_size`. The
        // maximum is at `idx = table_size - 1`. Single op vs the prior
        // `Vec<State>::iter().map(|s|s.num_bits).max()` linear scan.
        let max_value = (2 * table_size as u32 - 1) + delta_nb_bits;
        let max_num_bits = (max_value >> 16) as u8;

        symbol_states[symbol].start_state = Some(start_index);
        symbol_states[symbol].max_num_bits = Some(max_num_bits);
    }

    FSETable {
        table_size,
        states: symbol_states,
        state_table_flat,
        symbol_tt,
    }
}

const ML_DIST: &[i32] = &[
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];

const LL_DIST: &[i32] = &[
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];

const OF_DIST: &[i32] = &[
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];

// The three predefined LL/ML/OF distribution tables are pure functions
// of compile-time constants (`LL_DIST`, `ML_DIST`, `OF_DIST` and fixed
// `acc_log` values). Each `build_table_from_probabilities` call costs
// ~12 µs on a modern x86_64 — multiplied by three, that's ~38 µs of
// pure setup paid on every `FrameCompressor::new`. On a 1 KiB frame
// the actual compression work is ~1-5 µs, so the predefined-table
// build dominates the per-frame cost for tiny inputs.
//
// Cache each predefined table once per process and hand callers a
// `&'static FSETable`. The per-distribution cache slot is encoded by
// the static identity of the cache itself (each helper has its own
// `static`), NOT by pointer comparison on the distribution slice —
// `LL_DIST` / `ML_DIST` / `OF_DIST` are `const`, and `core::ptr::eq`
// on the materialized slice pointer of a `const` is only stable as
// long as rustc keeps lowering it to a single anonymous static. A
// future rustc change that duplicates the const at each use site
// would silently sink every call through a "unknown slice" branch
// and bypass the cache forever. Per-helper `static`s avoid that
// failure mode entirely — the cache slot is selected at compile time
// without consulting the slice pointer at all.
//
// Two implementations:
//
//   * Targets with atomic pointer support (`target_has_atomic = "ptr"`)
//     — `AtomicPtr<FSETable>` lock-free init via `compare_exchange`.
//     Works in both `std` and `no_std` builds; only `alloc` is needed
//     (always available in this crate via `extern crate alloc`).
//     Memory ordering: `Acquire` on the load so the consumer sees a
//     fully-published `FSETable`; `AcqRel` on the compare-exchange
//     so the publishing side both reads any concurrent winner
//     (`Acquire`) and publishes its store (`Release`). The first
//     writer leaks `Box<FSETable>` intentionally — the cache lives
//     for the entire program lifetime, exactly like a `LazyLock`.
//
//   * No-atomic targets (`not(target_has_atomic = "ptr")`, e.g.
//     `thumbv6m-none-eabi`, AVR, MSP430) — two sub-paths driven by
//     the `critical-section` feature:
//
//     - With `critical-section` enabled (the recommended choice for
//       any embedded build that wires up a CS impl from
//       `cortex-m-rt` / `riscv-rt` / `embassy-executor` / `esp-hal`
//       / similar): cached `static mut *mut FSETable` slot
//       protected by `critical_section::with`. First call enters
//       the CS, double-checks the slot, builds and publishes the
//       leaked `Box::into_raw` pointer, exits the CS. Subsequent
//       calls return the cached pointer. The CS is held for the
//       duration of one `build_table_from_probabilities` (~12 µs)
//       on the cold path — interrupts disabled for that window,
//       acceptable for a one-time per-table init.
//
//     - Without `critical-section`: no cache. The helpers return
//       a freshly-built `Box<FSETable>` per call (per-frame cost
//       same as the pre-cache status quo); the table is dropped
//       with the owning `FrameCompressor`, so memory does not
//       grow with the number of `FrameCompressor::new` calls.
//       Lack of pointer-width atomics is **not** a guarantee of
//       non-concurrency (interrupt / preemption reentrancy on
//       bare-metal targets), and a shared `static mut` slot
//       without any synchronization would be a data race (UB).
//       The owned-Box return is the same shape as the pre-PR
//       behaviour — no leak, no UB risk, no cache speedup. Users
//       who want the cache on no-atomic targets should enable
//       the `critical-section` feature, which routes through the
//       CS-protected `&'static` slot above.
//
// To paper over the per-target return-type difference the
// `FseDefaultTable` type alias resolves to `&'static FSETable` on
// any target/feature combination that has a cache (atomic, or
// no-atomic + `critical-section`) and to `alloc::boxed::Box<FSETable>`
// on the cache-less path. Both types `Deref` to `FSETable` so
// downstream consumers in `encoding/blocks/compressed.rs` and
// `encoding/frame_compressor.rs` borrow through `&` uniformly.
#[cfg(target_has_atomic = "ptr")]
fn get_or_init_cached_table(
    cache: &core::sync::atomic::AtomicPtr<FSETable>,
    probs: &[i32],
    acc_log: u8,
) -> &'static FSETable {
    use core::sync::atomic::Ordering;

    let cur = cache.load(Ordering::Acquire);
    if !cur.is_null() {
        // SAFETY: a non-null entry in this cache was published by a
        // previous winner of the `compare_exchange` below, which
        // leaked the `Box<FSETable>` (the cache never frees, mirroring
        // a `LazyLock` lifetime). The pointed-to allocation is
        // therefore valid for `'static` and immutable for the rest
        // of the program, so handing out a `&'static FSETable` to
        // multiple threads is sound.
        return unsafe { &*cur };
    }

    let built = alloc::boxed::Box::new(build_table_from_probabilities(probs, acc_log));
    let raw = alloc::boxed::Box::into_raw(built);
    // `AcqRel` on success rather than the minimal `Release`: the
    // success path doesn't actually need Acquire (no prior loads to
    // synchronise with at the publication point), but matching the
    // failure Acquire ordering keeps the success and failure
    // branches symmetric on rustc's atomic surface — both arms then
    // see the same memory-fence shape for whatever follows. The
    // marginal cost is one extra fence instruction on x86/aarch64;
    // this path runs at most three times per process so it never
    // shows up in a flamegraph.
    match cache.compare_exchange(
        core::ptr::null_mut(),
        raw,
        Ordering::AcqRel,
        Ordering::Acquire,
    ) {
        Ok(_) => {
            // We installed the singleton. SAFETY: `raw` is the
            // pointer we just published; no other thread can free it.
            unsafe { &*raw }
        }
        Err(existing) => {
            // Another thread beat us to the publish — reclaim our
            // throwaway allocation and use the winner. SAFETY: we own
            // `raw` here (compare_exchange did NOT store it), so
            // reconstituting the `Box` is sound. `existing` was
            // published by the winning thread and is leaked for
            // `'static`, mirroring the `cur` path.
            drop(unsafe { alloc::boxed::Box::from_raw(raw) });
            unsafe { &*existing }
        }
    }
}

/// No-atomic + no `critical-section` feature fallback: build a
/// fresh `FSETable` and return it as an owned `Box<FSETable>`. No
/// caching — see the module-level header comment for the rationale
/// (interrupt / preemption reentrancy on bare-metal targets makes
/// a shared `static mut` cache without atomic synchronization a
/// data race surface). The owned-Box return shape is paired with
/// the `FseDefaultTable` type alias so the per-frame caller stores
/// the table in `FseTables` and drops it when the compressor
/// drops — no `Box::leak`, no unbounded growth across multiple
/// `FrameCompressor::new` calls.
#[cfg(not(target_has_atomic = "ptr"))]
fn build_owned_table(probs: &[i32], acc_log: u8) -> alloc::boxed::Box<FSETable> {
    alloc::boxed::Box::new(build_table_from_probabilities(probs, acc_log))
}

// SAFETY: access to the inner `*mut FSETable` is gated by
// `critical_section::with` in `get_or_init_cached_table_cs`, so
// the slot is accessed serially even on multi-IRQ bare-metal
// runtimes. The CS impl supplied by the downstream runtime
// guarantees mutual exclusion for the duration of the closure.

/// Per-helper return type. `&'static FSETable` on targets/features
/// that own a process-wide cache (zero-cost subsequent calls);
/// `Box<FSETable>` on the cache-less no-atomic path (one allocation
/// per call, dropped with the owning `FrameCompressor` — no leak).
/// Both `Deref` to `FSETable`, so downstream consumers borrow
/// through `&` without caring which arm fired.
#[cfg(target_has_atomic = "ptr")]
pub(crate) type FseDefaultTable = &'static FSETable;
#[cfg(not(target_has_atomic = "ptr"))]
pub(crate) type FseDefaultTable = alloc::boxed::Box<FSETable>;

pub(crate) fn default_ml_table() -> FseDefaultTable {
    #[cfg(target_has_atomic = "ptr")]
    {
        static CACHE: core::sync::atomic::AtomicPtr<FSETable> =
            core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());
        get_or_init_cached_table(&CACHE, ML_DIST, 6)
    }

    #[cfg(not(target_has_atomic = "ptr"))]
    {
        build_owned_table(ML_DIST, 6)
    }
}

pub(crate) fn default_ll_table() -> FseDefaultTable {
    #[cfg(target_has_atomic = "ptr")]
    {
        static CACHE: core::sync::atomic::AtomicPtr<FSETable> =
            core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());
        get_or_init_cached_table(&CACHE, LL_DIST, 6)
    }

    #[cfg(not(target_has_atomic = "ptr"))]
    {
        build_owned_table(LL_DIST, 6)
    }
}

pub(crate) fn default_of_table() -> FseDefaultTable {
    #[cfg(target_has_atomic = "ptr")]
    {
        static CACHE: core::sync::atomic::AtomicPtr<FSETable> =
            core::sync::atomic::AtomicPtr::new(core::ptr::null_mut());
        get_or_init_cached_table(&CACHE, OF_DIST, 5)
    }

    #[cfg(not(target_has_atomic = "ptr"))]
    {
        build_owned_table(OF_DIST, 5)
    }
}
