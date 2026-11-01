#![allow(
    clippy::cast_lossless,
    clippy::inline_always,
    clippy::needless_pass_by_ref_mut,
    clippy::uninlined_format_args,
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
use crate::cpu_kernel::{CpuKernel, ScalarKernel};
use core::marker::PhantomData;
#[cfg(target_arch = "x86_64")]
use std::sync::OnceLock;

/// Pre-computed mask table: `BIT_MASK[n]` equals the lower `n` bits set,
/// i.e. `(1u64 << n) - 1` for `n` in `0..=64`.
///
/// `mask_lower_bits` no longer reads this table — it computes the mask
/// via `u64::MAX >> (64 - n)` to save a load. The table is still used
/// by the BMI2 PEXT triple-extract path on x86-64 (where the mask is
/// constructed once per call and then fed to `_pext_u64`), and by the
/// tests that verify mask values directly.
#[cfg(target_arch = "x86_64")]
const BIT_MASK: [u64; 65] = {
    let mut table = [0u64; 65];
    let mut i: u32 = 1;
    while i < 64 {
        table[i as usize] = (1u64 << i) - 1;
        i += 1;
    }
    table[64] = u64::MAX;
    table
};

#[cfg(target_arch = "x86_64")]
#[derive(Copy, Clone)]
struct TripleExtractDispatch {
    use_pext: bool,
}

#[cfg(target_arch = "x86_64")]
static TRIPLE_EXTRACT_DISPATCH: OnceLock<TripleExtractDispatch> = OnceLock::new();

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn should_use_pext(vendor: [u8; 12], family: u32) -> bool {
    vendor != *b"AuthenticAMD" || family != 0x17
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn triple_extract_dispatch() -> &'static TripleExtractDispatch {
    TRIPLE_EXTRACT_DISPATCH.get_or_init(detect_triple_extract_dispatch)
}

#[cfg(target_arch = "x86_64")]
fn detect_triple_extract_dispatch() -> TripleExtractDispatch {
    use core::arch::x86_64::__cpuid;
    use std::arch::is_x86_feature_detected;

    if !is_x86_feature_detected!("bmi2") {
        return TripleExtractDispatch { use_pext: false };
    }

    // AMD Zen1/Zen2 execute PEXT/PDEP through a slow microcode path.
    // Keep scalar extraction there and enable PEXT on Intel and newer AMD.
    let leaf0 = __cpuid(0);
    let mut vendor = [0u8; 12];
    *vendor.first_chunk_mut::<4>().unwrap() = leaf0.ebx.to_le_bytes();
    *vendor[4..].first_chunk_mut::<4>().unwrap() = leaf0.edx.to_le_bytes();
    *vendor[8..].first_chunk_mut::<4>().unwrap() = leaf0.ecx.to_le_bytes();
    let eax = __cpuid(1).eax;
    let base_family = (eax >> 8) & 0xF;
    let ext_family = (eax >> 20) & 0xFF;
    let family = if base_family == 0xF {
        base_family + ext_family
    } else {
        base_family
    };

    TripleExtractDispatch {
        use_pext: should_use_pext(vendor, family),
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "bmi2")]
unsafe fn extract_triple_pext(all_three: u64, n1: u8, n2: u8, n3: u8) -> (u64, u64, u64) {
    use core::arch::x86_64::_pext_u64;

    let mask3 = BIT_MASK[n3 as usize];
    let mask2 = BIT_MASK[n2 as usize].wrapping_shl(u32::from(n3));
    let mask1 = BIT_MASK[n1 as usize].wrapping_shl(u32::from(n2) + u32::from(n3));

    let val1 = _pext_u64(all_three, mask1);
    let val2 = _pext_u64(all_three, mask2);
    let val3 = _pext_u64(all_three, mask3);
    (val1, val2, val3)
}

/// Zstandard encodes some types of data in a way that the data must be read
/// back to front to decode it properly. `BitReaderReversed` provides a
/// convenient interface to do that.
pub(crate) struct BitReaderReversed<'s, K: CpuKernel = ScalarKernel> {
    /// Start offset (in bytes) of the 8-byte source window currently
    /// loaded into `bit_container`. Decreases monotonically as bytes
    /// are consumed: `refill` walks it backward toward 0 by
    /// `bits_consumed / 8`, and `bits_remaining()` uses
    /// `index * 8 + (64 - bits_consumed)` to compute how many stream
    /// bits remain. The byte at `source[index]` is the LSB of the
    /// `from_le_bytes` u64 in `bit_container`; the byte at
    /// `source[index + 7]` is the MSB (= the next stream bit at
    /// position 63 of `bit_container`, before any consumption).
    ///
    /// `pub(crate)` so the HUF 4-stream burst hot loop in
    /// `decoding::literals_section_decoder` can run upstream zstd's
    /// `ip[s] -= nb_bytes; bits[s] = MEM_read64(ip[s]) | 1` reload
    /// pattern directly against the byte stream — see
    /// [`Self::bits_consumed`] for the broader rationale.
    pub(crate) index: usize,

    /// How many bits have been consumed from `bit_container`.
    ///
    /// `pub(crate)` so the HUF 4-stream hot loop in
    /// `decoding::literals_section_decoder` can lift the reader state
    /// into a local `bits[4]` register layout (upstream zstd parity with
    /// `huf_decompress.c:HUF_decompress4X1_usingDTable_internal_fast_c_loop`):
    /// inside the burst, all symbol-decode work happens against a
    /// `bits[s]` u64 that fuses the decoder state with pending input
    /// bits, and the field is written back only at the burst boundary.
    /// Outside the burst the field is treated as opaque internal state.
    pub(crate) bits_consumed: u8,

    /// How many bits have been consumed past the end of the input. Will be zero until all the input
    /// has been read.
    extra_bits: usize,

    /// The source data to read from.
    ///
    /// `pub(crate)` — paired with [`Self::index`], the HUF 4-stream
    /// burst hot loop needs direct slice access for the per-iter
    /// upstream zstd-pattern reload (`MEM_read64(source[ip..ip+8])`).
    pub(crate) source: &'s [u8],

    /// The reader doesn't read directly from the source, it reads bits from here, and the container
    /// is "refilled" as it's emptied.
    ///
    /// `pub(crate)` — see [`Self::bits_consumed`] for the rationale.
    pub(crate) bit_container: u64,

    /// Phantom marker for the CPU kernel type parameter `K`. Zero-sized;
    /// drives monomorphisation of methods that route through `K::mask_lower_bits`
    /// without forcing the struct itself to carry runtime kernel state.
    _kernel: PhantomData<K>,

    /// Cached `triple_extract_dispatch().use_pext` snapshot, populated
    /// once in `new()`. `peek_bits_triple` reads this field instead of
    /// re-checking the global `OnceLock` on every sequence — the
    /// per-call atomic load + dispatch-branch was paying ~3 cycles on
    /// every sequence decode (thousands per block × many blocks per
    /// frame). One bool per `BitReaderReversed` lifetime, amortised
    /// across every `peek_bits_triple` in the same decode pass.
    #[cfg(target_arch = "x86_64")]
    pub(crate) use_pext_triple: bool,
}

impl<'s, K: CpuKernel> BitReaderReversed<'s, K> {
    /// How many bits are left to read by the reader.
    pub(crate) fn bits_remaining(&self) -> isize {
        self.index as isize * 8 + (64 - self.bits_consumed as isize) - self.extra_bits as isize
    }

    /// Returns `true` when the cached vendor policy says PEXT is fast
    /// on the running CPU (Intel + AMD Zen3+) and the bmi2-direct
    /// triple-extract path should be used. AMD Zen1/Zen2 microcode
    /// PEXT is slower than the scalar 3× shift+mask path, so
    /// [`should_use_pext`] caches `false` for those vendors.
    ///
    /// `no_std` x86_64 builds lack the runtime detection (`use_pext_triple`
    /// is std-gated), so this falls back to `true`: callers on
    /// `no_std` rely on compile-time `target_feature = "bmi2"` and
    /// implicitly trust that the chosen target CPU advertises fast
    /// PEXT. Vendor-specific microcode regression remains a
    /// build-time concern there — pin a known-good target with
    /// `RUSTFLAGS="-C target-cpu=..."`.
    #[cfg(target_arch = "x86_64")]
    #[inline(always)]
    pub(crate) fn use_pext_triple_fast(&self) -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            self.use_pext_triple
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            true
        }
    }

    pub(crate) fn new(source: &'s [u8]) -> BitReaderReversed<'s, K> {
        BitReaderReversed {
            index: source.len(),
            bits_consumed: 64,
            source,
            bit_container: 0,
            extra_bits: 0,
            _kernel: PhantomData,
            #[cfg(target_arch = "x86_64")]
            use_pext_triple: triple_extract_dispatch().use_pext,
        }
    }

    /// Refill the bit container with up to 64 fresh bits from `source`.
    ///
    /// Hot path (mid-stream, `self.index >= bytes_consumed`) is `#[inline(always)]`
    /// and folds into every caller — three operations: subtract index, mask
    /// off byte-aligned bit count, load 8 bytes. The pre-PR version wore a
    /// blanket `#[cold]` annotation which actively penalised the hot path
    /// (refill fires roughly every 2 sequences during sequence decode, so
    /// it is NOT cold). The rare edge cases — running out of source, going
    /// past the start of the stream, exhausting all useful bits — branch
    /// out to `refill_slow` which keeps the `#[cold] #[inline(never)]`
    /// treatment they actually deserve.
    #[inline(always)]
    fn refill(&mut self) {
        let bytes_consumed = self.bits_consumed as usize / 8;
        if bytes_consumed == 0 {
            return;
        }

        if self.index >= bytes_consumed {
            // We can safely move the window contained in `bit_container` down by `bytes_consumed`
            // If the reader wasn't byte aligned, the byte that was partially read is now in the highest order bits in the `bit_container`
            self.index -= bytes_consumed;
            // Some bits of the `bits_container` might have been consumed already because we read the window byte aligned
            self.bits_consumed &= 7;
            self.bit_container =
                u64::from_le_bytes(*self.source[self.index..].first_chunk::<8>().unwrap());
        } else {
            self.refill_slow();
        }

        // Assert that at least `56 = 64 - 8` bits are available to read.
        debug_assert!(self.bits_consumed < 8);
    }

    /// End-of-stream refill paths — runs when the next 8-byte window would
    /// underflow the source buffer. Kept `#[cold] #[inline(never)]` so the
    /// hot mid-stream path in [`refill`] folds into call sites without
    /// dragging these branches along.
    #[cold]
    #[inline(never)]
    fn refill_slow(&mut self) {
        if self.index > 0 {
            // Read the last portion of source into the `bit_container`
            if self.source.len() >= 8 {
                self.bit_container = u64::from_le_bytes(*self.source.first_chunk::<8>().unwrap());
            } else {
                let mut value = [0; 8];
                value[..self.source.len()].copy_from_slice(self.source);
                self.bit_container = u64::from_le_bytes(value);
            }

            self.bits_consumed -= 8 * self.index as u8;
            self.index = 0;

            self.bit_container <<= self.bits_consumed;
            self.extra_bits += self.bits_consumed as usize;
            self.bits_consumed = 0;
        } else if self.bits_consumed < 64 {
            // Shift out already used bits and fill up with zeroes
            self.bit_container <<= self.bits_consumed;
            self.extra_bits += self.bits_consumed as usize;
            self.bits_consumed = 0;
        } else {
            // All useful bits have already been read and more than 64 bits have been consumed, all we now do is return zeroes
            self.extra_bits += self.bits_consumed as usize;
            self.bits_consumed = 0;
            self.bit_container = 0;
        }
    }

    /// Read `n` number of bits from the source. Will read at most 56 bits.
    /// If there are no more bits to be read from the source zero bits will be returned instead.
    #[inline(always)]
    pub(crate) fn get_bits(&mut self, n: u8) -> u64 {
        if self.bits_consumed + n > 64 {
            self.refill();
        }

        let value = self.peek_bits(n);
        self.consume(n);
        value
    }

    /// Ensure at least `n` bits are available for subsequent unchecked reads.
    /// After calling this, it is safe to call [`get_bits_unchecked`](Self::get_bits_unchecked)
    /// for a combined total of up to `n` bits without individual refill checks.
    ///
    /// `n` must be at most 56.
    #[inline(always)]
    pub(crate) fn ensure_bits(&mut self, n: u8) {
        debug_assert!(n <= 56);
        if self.bits_consumed + n > 64 {
            self.refill();
        }
    }

    /// Read `n` bits from the source **without** checking whether a refill is
    /// needed. The caller **must** guarantee enough bits are available (e.g. via
    /// a prior [`ensure_bits`](Self::ensure_bits) call).
    #[inline(always)]
    pub(crate) fn get_bits_unchecked(&mut self, n: u8) -> u64 {
        debug_assert!(n <= 56);
        debug_assert!(
            self.bits_consumed + n <= 64,
            "get_bits_unchecked: not enough bits (consumed={}, requested={})",
            self.bits_consumed,
            n
        );
        let value = self.peek_bits(n);
        self.consume(n);
        value
    }

    /// Get the next `n` bits from the source without consuming them.
    /// Caller is responsible for making sure that `n` many bits have been refilled.
    ///
    /// Branchless: when `n == 0` the mask is zero so the result is zero
    /// without a dedicated check. `wrapping_shr` avoids a debug-mode
    /// panic when the computed shift equals 64 (which happens legitimately
    /// when `bits_consumed == 0` and `n == 0`).
    #[inline(always)]
    pub(crate) fn peek_bits(&mut self, n: u8) -> u64 {
        // n == 0 is valid (branchless no-op); otherwise the caller must
        // guarantee bits_consumed + n <= 64 via ensure_bits / get_bits.
        debug_assert!(
            n == 0 || self.bits_consumed + n <= 64,
            "peek_bits: not enough bits (consumed={}, requested={})",
            self.bits_consumed,
            n
        );
        let shift_by = (64u8 - self.bits_consumed).wrapping_sub(n);
        K::mask_lower_bits(self.bit_container.wrapping_shr(shift_by as u32), n)
    }

    /// Get the next `n1` `n2` and `n3` bits from the source without consuming them.
    /// Caller is responsible for making sure that `sum` many bits have been refilled.
    ///
    /// # Contract
    /// `sum` **must** equal `n1 + n2 + n3`. This is enforced by `debug_assert`
    /// but not checked in release builds for performance.
    ///
    /// Branchless: when all widths are zero the masks are zero, producing (0, 0, 0).
    #[inline(always)]
    pub(crate) fn peek_bits_triple(&mut self, sum: u8, n1: u8, n2: u8, n3: u8) -> (u64, u64, u64) {
        debug_assert_eq!(
            u16::from(sum),
            u16::from(n1) + u16::from(n2) + u16::from(n3),
            "peek_bits_triple: sum ({}) must equal n1+n2+n3 ({}+{}+{})",
            sum,
            n1,
            n2,
            n3
        );
        debug_assert!(
            sum == 0 || self.bits_consumed + sum <= 64,
            "peek_bits_triple: not enough bits (consumed={}, requested={})",
            self.bits_consumed,
            sum
        );
        // all_three contains bits like this: |XXXX..XXX111122223333|
        // Where XXX are already consumed bytes, 1/2/3 are bits of the respective value
        // Lower bits are to the right
        let shift_by = (64u8 - self.bits_consumed).wrapping_sub(sum);
        let all_three = self.bit_container.wrapping_shr(shift_by as u32);

        #[cfg(target_arch = "x86_64")]
        if self.use_pext_triple {
            // SAFETY: `use_pext_triple` was set in `new()` from
            // `triple_extract_dispatch().use_pext`, which only returns
            // `true` when BMI2 is runtime-detected; the unsafe call is
            // gated on the same runtime check that the inline-form
            // `try_extract_triple_with_pext` used to perform per-call.
            return unsafe { extract_triple_pext(all_three, n1, n2, n3) };
        }

        let val1 = K::mask_lower_bits(all_three.wrapping_shr(u32::from(n3) + u32::from(n2)), n1);
        let val2 = K::mask_lower_bits(all_three.wrapping_shr(u32::from(n3)), n2);
        let val3 = K::mask_lower_bits(all_three, n3);

        (val1, val2, val3)
    }

    /// BMI2-scoped variant of [`peek_bits`]. The whole body executes
    /// in `#[target_feature(enable = "bmi2")]` scope, so `_bzhi_u64`
    /// inlines as a single `bzhi` instruction at the caller site
    /// instead of crossing the `mask_lower_bits_bmi2_impl` CALL
    /// boundary.
    ///
    /// Use from any caller that is itself `#[target_feature(bmi2)]`-
    /// scoped and has verified the runtime CPU supports BMI2.
    ///
    /// # Safety
    /// Caller MUST ensure BMI2 is available on the running CPU. The
    /// `bzhi` instruction faults with #UD on hardware that does not
    /// advertise BMI2.
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "bmi2")]
    #[inline]
    #[allow(dead_code)]
    pub(crate) unsafe fn peek_bits_bmi2(&mut self, n: u8) -> u64 {
        debug_assert!(
            n == 0 || self.bits_consumed + n <= 64,
            "peek_bits_bmi2: not enough bits (consumed={}, requested={})",
            self.bits_consumed,
            n
        );
        let shift_by = (64u8 - self.bits_consumed).wrapping_sub(n);
        core::arch::x86_64::_bzhi_u64(self.bit_container.wrapping_shr(shift_by as u32), n as u32)
    }

    /// BMI2-scoped variant of [`peek_bits_triple`]. Mirrors the
    /// scalar/K-trait variant but inlines `_pext_u64` directly instead
    /// of crossing the `extract_triple_pext` CALL boundary.
    ///
    /// On AMD Zen1/Zen2 (vendor=AuthenticAMD family=0x17) `_pext_u64`
    /// goes through slow microcode; callers should still consult
    /// `self.use_pext_triple` (populated at construction from the
    /// global dispatch cache) and route to the scalar variant on
    /// those CPUs. This method assumes the caller already gated on
    /// `use_pext_triple == true`.
    ///
    /// # Safety
    /// Caller MUST ensure BMI2 is available AND the running CPU
    /// benefits from `_pext_u64` (i.e. not Zen1/Zen2).
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "bmi2")]
    #[inline]
    pub(crate) unsafe fn peek_bits_triple_bmi2(
        &mut self,
        sum: u8,
        n1: u8,
        n2: u8,
        n3: u8,
    ) -> (u64, u64, u64) {
        debug_assert_eq!(
            u16::from(sum),
            u16::from(n1) + u16::from(n2) + u16::from(n3),
            "peek_bits_triple_bmi2: sum ({}) must equal n1+n2+n3 ({}+{}+{})",
            sum,
            n1,
            n2,
            n3
        );
        debug_assert!(
            sum == 0 || self.bits_consumed + sum <= 64,
            "peek_bits_triple_bmi2: not enough bits (consumed={}, requested={})",
            self.bits_consumed,
            sum
        );
        let shift_by = (64u8 - self.bits_consumed).wrapping_sub(sum);
        let all_three = self.bit_container.wrapping_shr(shift_by as u32);
        // SAFETY: caller's target_feature includes BMI2 per `# Safety`
        // contract; same scope as the enclosing fn.
        unsafe { extract_triple_pext(all_three, n1, n2, n3) }
    }

    /// Consume `n` bits from the source.
    #[inline(always)]
    pub(crate) fn consume(&mut self, n: u8) {
        self.bits_consumed += n;
        debug_assert!(self.bits_consumed <= 64);
    }

    /// Same as calling get_bits three times but slightly more performant.
    ///
    /// Uses a single conditional refill (via [`ensure_bits`](Self::ensure_bits))
    /// instead of unconditionally refilling, avoiding redundant work when the
    /// bit container already holds enough bits.
    #[inline(always)]
    pub(crate) fn get_bits_triple(&mut self, n1: u8, n2: u8, n3: u8) -> (u64, u64, u64) {
        // Compute in u16 to avoid u8 overflow (max realistic sum is ~26,
        // but the type system allows up to 3×255).
        let sum_wide = u16::from(n1) + u16::from(n2) + u16::from(n3);
        if sum_wide <= 56 {
            let sum = sum_wide as u8;
            self.ensure_bits(sum);

            let triple = self.peek_bits_triple(sum, n1, n2, n3);
            self.consume(sum);
            return triple;
        }

        (self.get_bits(n1), self.get_bits(n2), self.get_bits(n3))
    }
}
