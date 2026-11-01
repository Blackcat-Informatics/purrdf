#![allow(
    clippy::fn_params_excessive_bools,
    clippy::inline_always,
    clippy::manual_bit_width,
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Utilities for decoding Huff0 encoded huffman data.

use crate::bit_io::BitReaderReversed;
use crate::decoding::errors::HuffmanTableError;
use crate::fse::{FSEDecoder, FSETable};
use alloc::vec::Vec;
#[cfg(target_arch = "x86")]
use core::arch::x86::_bzhi_u32;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::_bzhi_u64;
#[cfg(target_arch = "aarch64")]
use std::arch::is_aarch64_feature_detected;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use std::arch::is_x86_feature_detected;

use std::sync::OnceLock;

/// The Zstandard specification limits the maximum length of a code to 11 bits.
pub(crate) const MAX_MAX_NUM_BITS: u8 = 11;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum HuffmanDecodeKernel {
    Scalar,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    X86Bmi2,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    X86Avx2,
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    X86Vbmi2,
    #[cfg(target_arch = "aarch64")]
    Aarch64Neon,
    #[cfg(target_arch = "aarch64")]
    Aarch64Sve,
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline(always)]
const fn select_x86_huffman_decode_kernel(
    has_avx512vbmi2: bool,
    has_avx512f: bool,
    has_avx512vl: bool,
    has_avx512bw: bool,
    has_bmi2: bool,
    has_avx2: bool,
) -> HuffmanDecodeKernel {
    if has_avx512vbmi2 && has_avx512f && has_avx512vl && has_avx512bw && has_bmi2 {
        return HuffmanDecodeKernel::X86Vbmi2;
    }
    if has_avx2 && has_bmi2 {
        return HuffmanDecodeKernel::X86Avx2;
    }
    if has_bmi2 {
        return HuffmanDecodeKernel::X86Bmi2;
    }
    HuffmanDecodeKernel::Scalar
}

#[inline(always)]
pub(crate) fn detect_huffman_decode_kernel() -> HuffmanDecodeKernel {
    static KERNEL: OnceLock<HuffmanDecodeKernel> = OnceLock::new();
    *KERNEL.get_or_init(|| {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            let kernel = select_x86_huffman_decode_kernel(
                is_x86_feature_detected!("avx512vbmi2"),
                is_x86_feature_detected!("avx512f"),
                is_x86_feature_detected!("avx512vl"),
                is_x86_feature_detected!("avx512bw"),
                is_x86_feature_detected!("bmi2"),
                is_x86_feature_detected!("avx2"),
            );
            if kernel != HuffmanDecodeKernel::Scalar {
                return kernel;
            }
        }
        #[cfg(target_arch = "aarch64")]
        {
            if is_aarch64_feature_detected!("sve") {
                return HuffmanDecodeKernel::Aarch64Sve;
            }
            if is_aarch64_feature_detected!("neon") {
                return HuffmanDecodeKernel::Aarch64Neon;
            }
        }
        HuffmanDecodeKernel::Scalar
    })
}

pub(crate) struct HuffmanDecoder<'table> {
    table: &'table HuffmanTable,
    /// Read by `decode_symbol_and_advance` on x86 to pick between the
    /// scalar and BMI2 single-symbol decode bodies (single-stream tail
    /// loop after the 4-stream burst). On aarch64 and portable targets
    /// the BMI2 arm doesn't exist and the field is unread — the
    /// 4-stream SIMD-fallback path that previously consumed this
    /// field now dispatches via the [`HufKernel`] trait at
    /// `decompress_literals` entry instead.
    #[cfg_attr(
        not(any(target_arch = "x86", target_arch = "x86_64")),
        allow(dead_code)
    )]
    kernel: HuffmanDecodeKernel,
    /// State is used to index into the table.
    pub state: u64,
}

impl<'t> HuffmanDecoder<'t> {
    /// Create a new decoder with the provided table
    pub(crate) fn new(table: &'t HuffmanTable) -> HuffmanDecoder<'t> {
        HuffmanDecoder {
            table,
            kernel: detect_huffman_decode_kernel(),
            state: 0,
        }
    }

    /// Initialize internal state and prepare to decode data. Then
    /// `decode_symbol_and_advance` can be used for full decode steps.
    /// The 4-stream batched fallback path used by
    /// `literals_section_decoder` lives in the [`HufKernel`] trait
    /// impls (`decode4_unchecked` + `advance_state`) and is selected
    /// once via `match detect_huffman_decode_kernel() { ... }`.
    #[inline(always)]
    pub(crate) fn init_state<K: crate::cpu_kernel::CpuKernel>(
        &mut self,
        br: &mut BitReaderReversed<'_, K>,
    ) -> u8 {
        let num_bits = self.table.max_num_bits;
        let new_bits = br.get_bits(num_bits);
        self.state = new_bits;
        num_bits
    }

    /// Decode symbol and advance state in one table lookup.
    #[inline(always)]
    pub(crate) fn decode_symbol_and_advance<K: crate::cpu_kernel::CpuKernel>(
        &mut self,
        br: &mut BitReaderReversed<'_, K>,
    ) -> u8 {
        // On x86 the BMI2 kernel uses `_bzhi_u64` and is a real
        // perf win over the scalar `((state << n) & mask) | new_bits`
        // sequence, so the runtime match is load-bearing. On aarch64
        // both NEON and SVE arms previously aliased the scalar body
        // verbatim — the match was paying a 3-arm dispatch cost for
        // zero benefit. Collapsed to a direct scalar call there.
        // The enum's Aarch64Neon / Aarch64Sve variants are themselves
        // cfg-gated to target_arch = "aarch64", so under the outer
        // x86 cfg below they don't exist — the match here is
        // exhaustive on Scalar + X86Bmi2/Avx2/Vbmi2 alone, and an
        // inner `cfg(target_arch = "aarch64")` arm would be dead
        // (outer x86 cfg already false on aarch64).
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            match self.kernel {
                HuffmanDecodeKernel::Scalar => self.decode_symbol_and_advance_scalar(br),
                HuffmanDecodeKernel::X86Bmi2
                | HuffmanDecodeKernel::X86Avx2
                | HuffmanDecodeKernel::X86Vbmi2 => {
                    // SAFETY: This path is selected only after runtime/static feature checks.
                    unsafe { self.decode_symbol_and_advance_x86_bmi2(br) }
                }
            }
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        {
            // aarch64 and portable targets: the X86* arms compile out
            // entirely, so the match would collapse to a single arm.
            // Bypass the match and call scalar directly — both
            // Aarch64Neon and Aarch64Sve specialisations were
            // verbatim clones of the scalar body (they were dropped
            // in an earlier commit), and no NEON/SVE intrinsics
            // exist for the single-symbol decode shape.
            self.decode_symbol_and_advance_scalar(br)
        }
    }

    #[inline(always)]
    fn decode_symbol_and_advance_scalar<K: crate::cpu_kernel::CpuKernel>(
        &mut self,
        br: &mut BitReaderReversed<'_, K>,
    ) -> u8 {
        let packed = self.table.packed_decode[self.state as usize];
        let num_bits = (packed >> 8) as u8;
        let new_bits = br.get_bits(num_bits);
        self.state = ((self.state << num_bits) & self.table.state_mask) | new_bits;
        packed as u8
    }

    /// Decode one symbol WITHOUT refilling the bit reader. The caller must have
    /// guaranteed enough buffered bits (via `ensure_bits`) for this and any
    /// batched siblings. Mirrors upstream zstd `HUF_decodeSymbolX1`
    /// (`BIT_lookBitsFast` + `BIT_skipBits`, no per-symbol reload): the drain
    /// reloads once per group of symbols instead of once per symbol, so the
    /// tail symbols near a stream end don't each pay `refill_slow`. The advance
    /// uses the scalar form (byte-identical to the BMI2 `_bzhi` path) since the
    /// drain handles only a handful of trailing symbols per stream.
    #[inline(always)]
    pub(crate) fn decode_symbol_and_advance_no_refill<K: crate::cpu_kernel::CpuKernel>(
        &mut self,
        br: &mut BitReaderReversed<'_, K>,
    ) -> u8 {
        let packed = self.table.packed_decode[self.state as usize];
        let num_bits = (packed >> 8) as u8;
        let new_bits = br.get_bits_unchecked(num_bits);
        self.state = ((self.state << num_bits) & self.table.state_mask) | new_bits;
        packed as u8
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "bmi2")]
    unsafe fn decode_symbol_and_advance_x86_bmi2<K: crate::cpu_kernel::CpuKernel>(
        &mut self,
        br: &mut BitReaderReversed<'_, K>,
    ) -> u8 {
        let packed = self.table.packed_decode[self.state as usize];
        let num_bits = (packed >> 8) as u8;
        let new_bits = br.get_bits(num_bits);
        self.state = unsafe { self.advance_state_x86_bmi2(num_bits, new_bits) };
        packed as u8
    }

    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    #[target_feature(enable = "bmi2")]
    unsafe fn advance_state_x86_bmi2(&self, num_bits: u8, new_bits: u64) -> u64 {
        #[cfg(target_arch = "x86_64")]
        {
            _bzhi_u64(self.state << num_bits, u32::from(self.table.max_num_bits)) | new_bits
        }
        #[cfg(target_arch = "x86")]
        {
            let shifted = ((self.state << num_bits) & u64::from(u32::MAX)) as u32;
            u64::from(_bzhi_u32(shifted, u32::from(self.table.max_num_bits))) | new_bits
        }
    }

    // aarch64 NEON / SVE kernels for `decode_symbol_and_advance` were
    // identical clones of the scalar body — no NEON/SVE intrinsics
    // were ever in use here (the SIMD kernels live in `decode4_*`
    // below, where they actually batch four streams). The aarch64
    // arm of `decode_symbol_and_advance` now calls
    // `decode_symbol_and_advance_scalar` directly; keeping the
    // duplicate functions around just so the match could enumerate
    // them was dead code.
}

/// A Huffman decoding table contains a list of Huffman prefix codes and their associated values
#[derive(Clone)]
pub struct HuffmanTable {
    /// Packed `symbol | (num_bits << 8)` per state index, exposed
    /// `pub(crate)` because the HUF 4-stream burst hot path in
    /// `literals_section_decoder::decode_literals` indexes it directly
    /// (`packed_decode[idx]`) for a single-load table lookup matching
    /// upstream zstd `huf_decompress.c:dtable[index]`. This is the primary
    /// (and only) 4-stream decode lookup table since the previous
    /// SIMD-fallback dispatch was removed in favour of upstream zstd's
    /// always-firing burst.
    ///
    /// **`u16` (matches upstream zstd `HUF_DEltX1` layout exactly).** Upstream zstd's
    /// `dtable[index]` returns a 2-byte entry — low byte is `symbol`,
    /// high byte is `nbBits`. We mirror that representation so the
    /// table size is `2 × (1 << max_num_bits)` bytes instead of `4 ×`.
    /// At `max_num_bits = 11` (zstd spec ceiling) that's 4 KiB vs the
    /// older 8 KiB representation — halves L1d footprint on the hot
    /// HUF decode path. Worth the change because the L-7 Fast workload
    /// (and any small-alphabet poorly-compressed input) decodes most
    /// output bytes through this table; literal-buffer pressure on
    /// L1d makes the cache hit rate sensitive to table footprint.
    pub(crate) packed_decode: Vec<u16>,
    /// The weight of a symbol is the number of occurrences in a table.
    /// This value is used in constructing a binary tree referred to as
    /// a Huffman tree. Once this tree is constructed, it can be used to build the
    /// lookup table
    weights: Vec<u8>,
    /// The maximum size in bits a prefix code in the encoded data can be.
    /// This value is used so that the decoder knows how many bits
    /// to read from the bitstream before checking the table. This
    /// value must be 11 or lower.
    pub max_num_bits: u8,
    state_mask: u64,
    bits: Vec<u8>,
    bit_ranks: Vec<u32>,
    /// Running `sum(1 << (w - 1))` accumulated WHILE weights decode
    /// (upstream zstd `HUF_readStats` keeps the same running stats), replacing a
    /// separate post-decode pass over `weights`.
    weight_sum: u32,
    /// Per-weight occurrence counts accumulated during weight decode
    /// (upstream zstd `rankStats`); `bit_ranks` derives from this without
    /// re-walking the code-length array.
    weight_rank_count: [u32; (MAX_MAX_NUM_BITS as usize) + 1],
    /// Inferred last weight, stashed by `compute_huffman_bits` for the
    /// `bit_ranks` derivation in `build_table_from_weights`.
    last_weight: u8,
    /// In some cases, the list of weights is compressed using FSE compression.
    fse_table: FSETable,
}

impl HuffmanTable {
    /// Heap bytes owned by this table (decode lookup table plus the
    /// weight-decoding scratch vectors and the weight-stream FSE table).
    pub fn heap_bytes(&self) -> usize {
        self.packed_decode.capacity() * size_of::<u16>()
            + self.weights.capacity()
            + self.bits.capacity()
            + self.bit_ranks.capacity() * size_of::<u32>()
            + self.fse_table.heap_bytes()
    }

    /// Create a new, empty table.
    pub fn new() -> HuffmanTable {
        HuffmanTable {
            packed_decode: Vec::new(),

            weights: Vec::with_capacity(256),
            max_num_bits: 0,
            state_mask: 0,
            bits: Vec::with_capacity(256),
            bit_ranks: Vec::with_capacity(11),
            weight_sum: 0,
            weight_rank_count: [0; (MAX_MAX_NUM_BITS as usize) + 1],
            last_weight: 0,
            fse_table: FSETable::new(255),
        }
    }

    /// Completely empty the table then repopulate as a replica
    /// of `other`.
    pub fn reinit_from(&mut self, other: &Self) {
        self.reset();
        // Copy ONLY the decode-time state. `weights` / `bits` /
        // `rank scratch and the weight-decoding `fse_table` are build-time
        // scratch (repopulated when a block carries a new HUF table); the
        // literal-decode hot path reads only `packed_decode` + `max_num_bits`
        // + `state_mask`. Skipping the rest mirrors the upstream zstd copying just the
        // HUF decode table per frame, not the full build workspace.
        self.packed_decode.extend_from_slice(&other.packed_decode);
        self.max_num_bits = other.max_num_bits;
        self.state_mask = other.state_mask;
    }

    /// Completely empty the table of all data.
    pub fn reset(&mut self) {
        self.packed_decode.clear();
        self.weights.clear();
        self.max_num_bits = 0;
        self.state_mask = 0;
        self.bits.clear();
        self.bit_ranks.clear();
        self.weight_sum = 0;
        self.weight_rank_count = [0; (MAX_MAX_NUM_BITS as usize) + 1];
        self.last_weight = 0;
        self.fse_table.reset();
    }

    /// Build the equivalent encoder-side Huffman table from parsed weights.
    pub(crate) fn to_encoder_table(&self) -> Option<crate::huff0::huff0_encoder::HuffmanTable> {
        if self.bits.is_empty() || self.max_num_bits == 0 {
            return None;
        }

        let max_bits = usize::from(self.max_num_bits);
        let weights = self
            .bits
            .iter()
            .copied()
            .map(|num_bits| {
                if num_bits == 0 {
                    0
                } else {
                    max_bits - usize::from(num_bits) + 1
                }
            })
            .collect::<Vec<_>>();
        Some(crate::huff0::huff0_encoder::HuffmanTable::build_from_weights(&weights))
    }

    /// Read from `source` and decode the input, populating the huffman decoding table.
    ///
    /// Returns the number of bytes read.
    pub fn build_decoder(&mut self, source: &[u8]) -> Result<u32, HuffmanTableError> {
        self.packed_decode.clear();

        let bytes_used = self.read_weights(source)?;
        self.build_table_from_weights()?;
        Ok(bytes_used)
    }

    /// Parse the weight header into `bits` + `max_num_bits` WITHOUT building
    /// the decode lookup table. Returns the same byte count as
    /// [`Self::build_decoder`] (the weight-header length), so a caller
    /// stepping a cursor over packed tables advances identically. Used by
    /// the encoder dictionary load: [`Self::to_encoder_table`] reads only
    /// `bits` + `max_num_bits`, so filling `packed_decode` is pure waste.
    ///
    /// Crate-internal: the table it produces is intentionally non-decodable
    /// (empty `packed_decode`); only `decode_dict_for_encoding` calls it, and
    /// its result is wrapped in an `EncoderDictionary` that has no decode path.
    pub(crate) fn build_weights_only(&mut self, source: &[u8]) -> Result<u32, HuffmanTableError> {
        self.packed_decode.clear();

        let bytes_used = self.read_weights(source)?;
        self.compute_huffman_bits()?;
        Ok(bytes_used)
    }

    /// Read weights from the provided source.
    ///
    /// The huffman table is represented in the input data as a list of weights.
    /// After the header, weights are read, then a Huffman decoding table
    /// can be constructed using that list of weights.
    ///
    /// Returns the number of bytes read.
    fn read_weights(&mut self, source: &[u8]) -> Result<u32, HuffmanTableError> {
        use HuffmanTableError as err;

        if source.is_empty() {
            return Err(err::SourceIsEmpty);
        }
        let header = source[0];
        let mut bits_read = 8;

        match header {
            // If the header byte is less than 128, the series of weights
            // is compressed using two interleaved FSE streams that share
            // a distribution table.
            0..=127 => {
                let fse_stream = &source[1..];
                if header as usize > fse_stream.len() {
                    return Err(err::NotEnoughBytesForWeights {
                        got_bytes: fse_stream.len(),
                        expected_bytes: header,
                    });
                }
                //fse decompress weights
                let bytes_used_by_fse_header = self.fse_table.build_decoder(fse_stream, 6)?;

                if bytes_used_by_fse_header > header as usize {
                    return Err(err::FSETableUsedTooManyBytes {
                        used: bytes_used_by_fse_header,
                        available_bytes: header,
                    });
                }

                vprintln!(
                    "Building fse table for huffman weights used: {}",
                    bytes_used_by_fse_header
                );
                // Huffman headers are compressed using two interleaved
                // FSE bitstreams, where the first state (decoder) handles
                // even symbols, and the second handles odd symbols.
                let mut dec1 = FSEDecoder::new(&self.fse_table);
                let mut dec2 = FSEDecoder::new(&self.fse_table);

                let compressed_start = bytes_used_by_fse_header;
                let compressed_length = header as usize - bytes_used_by_fse_header;

                let compressed_weights = &fse_stream[compressed_start..];
                if compressed_weights.len() < compressed_length {
                    return Err(err::NotEnoughBytesToDecompressWeights {
                        have: compressed_weights.len(),
                        need: compressed_length,
                    });
                }
                let compressed_weights = &compressed_weights[..compressed_length];
                let mut br =
                    BitReaderReversed::<crate::cpu_kernel::ScalarKernel>::new(compressed_weights);

                bits_read += (bytes_used_by_fse_header + compressed_length) * 8;

                //skip the 0 padding at the end of the last byte of the bit stream and throw away the first 1 found
                let mut skipped_bits = 0;
                loop {
                    let val = br.get_bits(1);
                    skipped_bits += 1;
                    if val == 1 || skipped_bits > 8 {
                        break;
                    }
                }
                if skipped_bits > 8 {
                    //if more than 7 bits are 0, this is not the correct end of the bitstream. Either a bug or corrupted data
                    return Err(err::ExtraPadding { skipped_bits });
                }

                dec1.init_state(&mut br)?;
                dec2.init_state(&mut br)?;

                // Disjoint-field borrow: `dec1`/`dec2` hold `&self.fse_table`,
                // so the weight sink is taken as a field borrow. The running
                // stats (upstream zstd `HUF_readStats` shape) accumulate in locals
                // and commit to `self` after the loop, replacing the
                // separate weight-sum and rank-count passes.
                let weights = &mut self.weights;
                weights.clear();
                let mut weight_sum = 0u32;
                let mut weight_rank_count = [0u32; (MAX_MAX_NUM_BITS as usize) + 1];
                macro_rules! push_weight {
                    ($w:expr) => {{
                        let w: u8 = $w;
                        if w > MAX_MAX_NUM_BITS {
                            return Err(err::WeightBiggerThanMaxNumBits { got: w });
                        }
                        // The cap lives here, not at the loop bottom: the two
                        // early-break paths push a final weight after the
                        // bottom check would have run, and 256 explicit
                        // weights would wrap symbol index 256 to 0 through
                        // the u8 entry packing in the table fill.
                        if weights.len() >= 255 {
                            return Err(err::TooManyWeights {
                                got: weights.len() + 1,
                            });
                        }
                        weight_rank_count[w as usize] += 1;
                        if w > 0 {
                            weight_sum += 1u32 << (w - 1);
                        }
                        weights.push(w);
                    }};
                }

                // The weights FSE table is built with a max accuracy_log of 6
                // (the `build_decoder(fse_stream, 6)` call above), so each state
                // update reads at most 6 bits. Refilling once per loop step
                // (two interleaved updates = up to 12 bits) lets both updates
                // use the unchecked fast advance instead of a per-symbol refill
                // branch, mirroring the upstream zstd's single reload per decode step.
                // `bits_remaining()` still tracks end-of-stream via `extra_bits`
                // (maintained by `refill_slow`), so the termination checks below
                // fire identically.
                const WEIGHTS_REFILL_BUDGET: u8 = 12;

                // The two decoders take turns decoding a single symbol and updating their state.
                loop {
                    br.ensure_bits(WEIGHTS_REFILL_BUDGET);

                    push_weight!(dec1.decode_symbol());
                    dec1.update_state_fast(&mut br);

                    if br.bits_remaining() <= -1 {
                        //collect final states
                        push_weight!(dec2.decode_symbol());
                        break;
                    }

                    push_weight!(dec2.decode_symbol());
                    dec2.update_state_fast(&mut br);

                    if br.bits_remaining() <= -1 {
                        //collect final states
                        push_weight!(dec1.decode_symbol());
                        break;
                    }
                }
                self.weight_sum = weight_sum;
                self.weight_rank_count = weight_rank_count;
            }
            // If the header byte is greater than or equal to 128,
            // weights are directly represented, where each weight is
            // encoded directly as a 4 bit field. The weights will
            // always be encoded with full bytes, meaning if there's
            // an odd number of weights, the last weight will still
            // occupy a full byte.
            _ => {
                // weights are directly encoded
                let weights_raw = &source[1..];
                let num_weights = header - 127;
                self.weights.clear();
                let mut weight_sum = 0u32;
                let mut weight_rank_count = [0u32; (MAX_MAX_NUM_BITS as usize) + 1];

                let bytes_needed = if num_weights.is_multiple_of(2) {
                    num_weights as usize / 2
                } else {
                    (num_weights as usize / 2) + 1
                };

                if weights_raw.len() < bytes_needed {
                    return Err(err::NotEnoughBytesInSource {
                        got: weights_raw.len(),
                        need: bytes_needed,
                    });
                }

                // Unpack both nibbles per source byte in one iteration —
                // the per-index parity branch defeated unrolling and read
                // every byte twice. The running stats accumulate alongside
                // (upstream zstd `HUF_readStats` shape).
                let mut push_weight = |w: u8, weights: &mut Vec<u8>| -> Result<(), err> {
                    if w > MAX_MAX_NUM_BITS {
                        return Err(err::WeightBiggerThanMaxNumBits { got: w });
                    }
                    weight_rank_count[w as usize] += 1;
                    if w > 0 {
                        weight_sum += 1u32 << (w - 1);
                    }
                    weights.push(w);
                    Ok(())
                };
                let mut idx = 0usize;
                for &byte in &weights_raw[..bytes_needed] {
                    push_weight(byte >> 4, &mut self.weights)?;
                    idx += 1;
                    if idx < num_weights as usize {
                        push_weight(byte & 0xF, &mut self.weights)?;
                        idx += 1;
                    }
                }
                self.weight_sum = weight_sum;
                self.weight_rank_count = weight_rank_count;
                bits_read += num_weights as usize * 4;
            }
        }

        let bytes_read = if bits_read % 8 == 0 {
            bits_read / 8
        } else {
            (bits_read / 8) + 1
        };
        Ok(bytes_read as u32)
    }

    /// Compute per-symbol code lengths (`bits`) + `max_num_bits` from the
    /// parsed `weights`, returning `max_bits`. This is the slice of the
    /// table build the ENCODER side needs: [`Self::to_encoder_table`] reads
    /// only `bits` + `max_num_bits`. The decode lookup-table fill
    /// (`bit_ranks` / `packed_decode` / rank offsets) is decoder-only and
    /// lives in [`Self::build_table_from_weights`].
    fn compute_huffman_bits(&mut self) -> Result<u8, HuffmanTableError> {
        use HuffmanTableError as err;

        self.bits.clear();
        self.bits.resize(self.weights.len() + 1, 0);

        // `weight_sum` was accumulated while the weights decoded
        // (`read_weights` validates each weight <= MAX_MAX_NUM_BITS as it
        // lands), so no re-walk of `weights` is needed here.
        let weight_sum: u32 = self.weight_sum;

        if weight_sum == 0 {
            return Err(err::MissingWeights);
        }

        let max_bits = highest_bit_set(weight_sum) as u8;
        let left_over = (1 << max_bits) - weight_sum;

        //left_over must be power of two
        if !left_over.is_power_of_two() {
            return Err(err::LeftoverIsNotAPowerOf2 { got: left_over });
        }

        let last_weight = highest_bit_set(left_over) as u8;
        self.last_weight = last_weight;

        for symbol in 0..self.weights.len() {
            let bits = if self.weights[symbol] > 0 {
                max_bits + 1 - self.weights[symbol]
            } else {
                0
            };
            self.bits[symbol] = bits;
        }

        self.bits[self.weights.len()] = max_bits + 1 - last_weight;
        self.max_num_bits = max_bits;
        self.state_mask = (1_u64 << max_bits) - 1;

        if max_bits > MAX_MAX_NUM_BITS {
            return Err(err::MaxBitsTooHigh { got: max_bits });
        }

        Ok(max_bits)
    }

    fn build_table_from_weights(&mut self) -> Result<(), HuffmanTableError> {
        let max_bits = self.compute_huffman_bits()?;

        // Derive the code-length histogram from the per-weight counts
        // accumulated during weight decode instead of re-walking `bits`:
        // a weight `w > 0` maps to `bits = max_bits + 1 - w`, weight 0
        // stays code-length 0, and the inferred last symbol contributes
        // one extra entry at its derived length.
        self.bit_ranks.clear();
        self.bit_ranks.resize((max_bits + 1) as usize, 0);
        self.bit_ranks[0] = self.weight_rank_count[0];
        for w in 1..=max_bits {
            self.bit_ranks[(max_bits + 1 - w) as usize] = self.weight_rank_count[w as usize];
        }
        self.bit_ranks[(max_bits + 1 - self.last_weight) as usize] += 1;

        let table_size = 1usize << self.max_num_bits;

        // Starting offset for each code-length rank, in a fixed local
        // sized 16 so the fill loop below indexes it with a masked code
        // length and the optimizer drops the bounds check (code lengths
        // cap at MAX_MAX_NUM_BITS = 11). Same descending prefix walk as
        // the previous heap-allocated form.
        let mut rank_start = [0usize; 16];
        for bits in (1..=max_bits).rev() {
            rank_start[bits as usize - 1] = rank_start[bits as usize]
                + self.bit_ranks[bits as usize] as usize * (1 << (max_bits - bits));
        }

        // The rank walk partitions [0, table_size) into one contiguous run
        // per code length (no gaps, no overlap), so the fill loop below
        // initialises every slot — the upstream zstd likewise skips the table
        // pre-zero (`ZSTD_memset ... is not necessary`). Assert the total
        // span equals `table_size` before trusting the unchecked `set_len`.
        assert!(
            rank_start[0] == table_size,
            "rank_start[0]: {} should be: {}",
            rank_start[0],
            table_size
        );

        // Write into the uninitialised tail (`spare_capacity_mut`) and only
        // `set_len` after the fill completes, avoiding the redundant
        // zero-then-overwrite of a `resize(_, 0)`.
        self.packed_decode.clear();
        self.packed_decode.reserve(table_size);
        let slots_ptr = self
            .packed_decode
            .spare_capacity_mut()
            .as_mut_ptr()
            .cast::<u16>();

        // Sort symbols by code length (counting sort, preserving symbol order
        // within a length so canonical Huffman assignment stays correct), then
        // fill the table one code-length GROUP at a time. Within a group the run
        // length `1 << (max_bits - len)` and the entry's `nbBits = len` are
        // CONSTANT, so the `match` on the run length hoists out of the symbol
        // loop and lets the optimiser emit a straight-line specialised store per
        // group — the upstream zstd `HUF_readDTableX1_wksp` shape. The previous
        // per-symbol form recomputed a variable run length and used a
        // runtime-trip-count `while`, which blocked unrolling.
        //
        // `sym_off[len]` is the start index of the length-`len` run inside
        // `sorted`; the prefix sum walks weight-0 first (those symbols are
        // skipped by the `1..=max_bits` fill below).
        let mut sorted = [0u8; 256];
        let mut sym_off = [0usize; 16];
        let mut acc = 0usize;
        // Prefix sum of the per-code-length counts: `sym_off[l]` is the start
        // index of the length-`l` run in `sorted`. `bit_ranks` has exactly
        // `max_bits + 1` entries (<= 12), so zipping bounds the walk.
        for (off, &rank) in sym_off.iter_mut().zip(self.bit_ranks.iter()) {
            *off = acc;
            acc += rank as usize;
        }
        {
            let mut cursor = sym_off;
            for (symbol, &len) in self.bits.iter().enumerate() {
                // `len <= max_bits <= 11`; the mask proves the 16-slot index in
                // range. At most 256 symbols (weights capped at 255 + the
                // inferred last), so every cursor stays < 256 and `symbol` fits u8.
                let g = (len & 0xF) as usize;
                unsafe {
                    *sorted.get_unchecked_mut(cursor[g]) = symbol as u8;
                }
                cursor[g] += 1;
            }
        }

        // `filled` re-proves full [0, table_size) coverage in release builds
        // before `set_len` exposes the entries.
        let mut filled = 0usize;
        for len in 1..=max_bits {
            let count = self.bit_ranks[len as usize] as usize;
            if count == 0 {
                continue;
            }
            let run_len = 1usize << (max_bits - len);
            let grp = sym_off[len as usize];
            let mut u = rank_start[len as usize];
            // Release-mode bound for the whole group's run of writes (a register
            // compare): a `bit_ranks`/`bits` desync must fail loudly, not OOB.
            assert!(
                u + count * run_len <= table_size,
                "huffman length-{len} group [{u}, +{}) escapes table {table_size}",
                count * run_len,
            );
            // The entry packs the symbol byte + its code length; `* 0x0001…`
            // broadcasts it into four `u16` lanes of a 64-bit store.
            macro_rules! packed64 {
                ($s:expr) => {{
                    let symbol = unsafe { *sorted.get_unchecked(grp + $s) };
                    let packed = u16::from(symbol) | (u16::from(len) << 8);
                    (packed, u64::from(packed) * 0x0001_0001_0001_0001)
                }};
            }
            // SAFETY (all arms): the per-group assert bounds `[u, u + count*run_len)`
            // inside the `reserve(table_size)` capacity; four `u16` are 8
            // padding-free bytes, so each unaligned u64 store writes exactly four
            // entries. `run_len` is a power of two, so the `>= 16` arm's 16-step
            // covers the run exactly.
            match run_len {
                1 => {
                    for s in 0..count {
                        let (packed, _) = packed64!(s);
                        unsafe { slots_ptr.add(u).write(packed) };
                        u += 1;
                    }
                }
                2 => {
                    for s in 0..count {
                        let (packed, _) = packed64!(s);
                        unsafe {
                            slots_ptr.add(u).write(packed);
                            slots_ptr.add(u + 1).write(packed);
                        }
                        u += 2;
                    }
                }
                4 => {
                    for s in 0..count {
                        let (_, p64) = packed64!(s);
                        unsafe { slots_ptr.add(u).cast::<u64>().write_unaligned(p64) };
                        u += 4;
                    }
                }
                8 => {
                    for s in 0..count {
                        let (_, p64) = packed64!(s);
                        unsafe {
                            slots_ptr.add(u).cast::<u64>().write_unaligned(p64);
                            slots_ptr.add(u + 4).cast::<u64>().write_unaligned(p64);
                        }
                        u += 8;
                    }
                }
                _ => {
                    // run_len is a power of two (1 << (max_bits - len)); the 1/2/4/8
                    // arms above are explicit, so this fallback only sees run_len in
                    // {16, 32, 64, ...} — every value here is >= 16 and divisible by
                    // 16, which the 16-entry-per-iteration store loop relies on.
                    debug_assert!(
                        run_len >= 16 && run_len.is_multiple_of(16),
                        "fallback arm expects run_len >= 16 and divisible by 16, got {run_len}"
                    );
                    for s in 0..count {
                        let (_, p64) = packed64!(s);
                        let mut off = 0usize;
                        while off < run_len {
                            unsafe {
                                slots_ptr.add(u + off).cast::<u64>().write_unaligned(p64);
                                slots_ptr
                                    .add(u + off + 4)
                                    .cast::<u64>()
                                    .write_unaligned(p64);
                                slots_ptr
                                    .add(u + off + 8)
                                    .cast::<u64>()
                                    .write_unaligned(p64);
                                slots_ptr
                                    .add(u + off + 12)
                                    .cast::<u64>()
                                    .write_unaligned(p64);
                            }
                            off += 16;
                        }
                        u += run_len;
                    }
                }
            }
            filled += count * run_len;
        }

        // SAFETY: the rank walk partitions [0, table_size) (anchored by the
        // `rank_start[0] == table_size` assert) and `filled` proves every slot in
        // that range was written, so no uninitialised entry is exposed.
        assert!(
            filled == table_size,
            "huffman table fill covered {filled} of {table_size} slots",
        );
        unsafe {
            self.packed_decode.set_len(table_size);
        }

        Ok(())
    }
}

purrdf_hash::default_from_new!(HuffmanTable);

/// Assert that the provided value is greater than zero, and returns the
/// 32 - the number of leading zeros
fn highest_bit_set(x: u32) -> u32 {
    assert!(x > 0);
    u32::BITS - x.leading_zeros()
}
