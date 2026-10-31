#![allow(
    clippy::empty_line_after_doc_comments,
    clippy::inline_always,
    clippy::manual_assert,
    clippy::manual_bit_width,
    clippy::use_self,
    reason = "Preserve the pinned upstream codec and kernel spelling; local modifications and source identity are documented in PROVENANCE.md"
)]
#![allow(
    missing_docs,
    missing_debug_implementations,
    reason = "Inherited codec API; source identity and modifications are documented in PROVENANCE.md"
)]
//! Use [BitWriter] to write an arbitrary amount of bits into a buffer.
use alloc::vec::Vec;

/// An interface for writing an arbitrary number of bits into a buffer. Write new bits into the buffer with `write_bits`, and
/// obtain the output using `dump`.
#[derive(Debug)]
pub(crate) struct BitWriter<V: AsMut<Vec<u8>>> {
    /// The buffer that's filled with bits
    output: V,
    /// holds a partially filled byte which gets put in outpu when it's fill with a write_bits call
    partial: u64,
    bits_in_partial: usize,
    /// The index pointing to the next unoccupied bit. Effectively just
    /// the number of bits that have been written into the buffer so far.
    bit_idx: usize,
}

impl BitWriter<Vec<u8>> {
    /// Initialize a new writer.
    // Production encode paths write into caller-owned buffers via `from`;
    // the owned-`Vec` constructor (and `dump` below) only serve the
    // dictionary trainer and test/fuzz round-trips.

    pub(crate) fn new() -> Self {
        Self {
            output: Vec::new(),
            partial: 0,
            bits_in_partial: 0,
            bit_idx: 0,
        }
    }
}

impl<V: AsMut<Vec<u8>>> BitWriter<V> {
    /// Initialize a new writer.
    pub(crate) fn from(mut output: V) -> BitWriter<V> {
        BitWriter {
            bit_idx: output.as_mut().len() * 8,
            output,
            partial: 0,
            bits_in_partial: 0,
        }
    }

    /// Get the current index. Can be used to reset to this index or to later change the bits at this index
    pub(crate) fn index(&self) -> usize {
        self.bit_idx + self.bits_in_partial
    }

    /// Reset to an index. Currently only supports resetting to a byte aligned index
    pub(crate) fn reset_to(&mut self, index: usize) {
        assert!(index.is_multiple_of(8));
        self.partial = 0;
        self.bits_in_partial = 0;
        self.bit_idx = index;
        self.output.as_mut().resize(index / 8, 0);
    }

    /// Change the bits at the index. `bits` contains the ǹum_bits` new bits that should be written
    /// Instead of the current content. `bits` *MUST* only contain zeroes in the upper bits outside of the `0..num_bits` range.
    pub(crate) fn change_bits(&mut self, idx: usize, bits: impl Into<u64>, num_bits: usize) {
        self.change_bits_64(idx, bits.into(), num_bits);
    }

    /// Monomorphized version of `change_bits`
    pub(crate) fn change_bits_64(&mut self, mut idx: usize, mut bits: u64, mut num_bits: usize) {
        self.flush();
        assert!(idx + num_bits < self.index());
        assert!(self.index() - (idx + num_bits) > self.bits_in_partial);

        // We might be changing bits unaligned to byte borders.
        // This means the lower bits of the first byte we are touching must stay the same
        if !idx.is_multiple_of(8) {
            // How many (upper) bits will change in the first byte?
            let bits_in_first_byte = 8 - (idx % 8);
            // We don't support only changing a few bits in the middle of a byte
            assert!(bits_in_first_byte <= num_bits);
            // Zero out the upper bits that will be changed while keeping the lower bits intact
            self.output.as_mut()[idx / 8] &= 0xFFu8 >> bits_in_first_byte;
            // Shift the bits up and put them in the now zeroed out bits
            let new_bits = (bits << (8 - bits_in_first_byte)) as u8;
            self.output.as_mut()[idx / 8] |= new_bits;
            // Update the state. Note that we are now definitely working byte aligned
            num_bits -= bits_in_first_byte;
            bits >>= bits_in_first_byte;
            idx += bits_in_first_byte;
        }

        assert!(idx.is_multiple_of(8));
        // We are now byte aligned, change idx to byte resolution
        let mut idx = idx / 8;

        // Update full bytes by just shifting and extracting bytes from the bits
        while num_bits >= 8 {
            self.output.as_mut()[idx] = bits as u8;
            num_bits -= 8;
            bits >>= 8;
            idx += 1;
        }

        // Deal with leftover bits that wont fill a full byte, keeping the upper bits of the original byte intact
        if num_bits > 0 {
            self.output.as_mut()[idx] &= 0xFFu8 << num_bits;
            self.output.as_mut()[idx] |= bits as u8;
        }
    }

    /// Simply append bytes to the buffer. Only works if the buffer was already byte aligned
    pub(crate) fn append_bytes(&mut self, data: &[u8]) {
        if self.misaligned() != 0 {
            panic!("Don't append bytes when writer is misaligned")
        }
        self.flush();
        self.output.as_mut().extend_from_slice(data);
        self.bit_idx += data.len() * 8;
    }

    /// Pre-reserve additional capacity in the output buffer so the
    /// upstream zstd-faithful FSE fast path can do `flush_bulk` writes without
    /// triggering `Vec::extend_from_slice`'s grow-on-realloc branch.
    /// `additional` is the number of bytes the upcoming bursts will
    /// emit (caller's bit budget / 8, rounded up).
    #[inline]
    pub(crate) fn reserve_output(&mut self, additional: usize) {
        self.output.as_mut().reserve(additional);
    }

    /// Upstream zstd `BIT_addBitsFast` (`bitstream.h:193-200`): always accumulate
    /// `bits` into the bottom of `partial`, no overflow check.
    ///
    /// # Safety
    ///
    /// Caller MUST guarantee BOTH preconditions BEFORE calling:
    ///
    /// 1. `num_bits + bits_in_partial <= 64` AND **not both 64**, i.e.
    ///    if `bits_in_partial == 64` then `num_bits` MUST be 0 — and
    ///    the explicit `num_bits == 0` early-return below handles that
    ///    case without ever evaluating `bits << bits_in_partial`. The
    ///    point of the precondition: any other combination would either
    ///    shift-by-64 (undefined for `u64 << 64` in Rust) or overflow
    ///    `bits_in_partial` past 64.
    /// 2. `num_bits == 64 || bits >> num_bits == 0` — value must be
    ///    clean of high junk past `num_bits`. Dirty high bits leak
    ///    into the packed stream at the next OR.
    ///
    /// The `debug_assert!`s below catch both in test/debug builds but
    /// do NOT survive `cargo build --release`, hence the `unsafe`
    /// signature. The function does not perform any memory-unsafe
    /// operation, but a violation produces a silently-corrupted
    /// output stream that decoders cannot recover — equivalent in
    /// blast radius to undefined behaviour for the consumer.
    ///
    /// Used by the upstream zstd-faithful FSE sequence encoder which knows its
    /// per-sequence bit budget at compile time and inserts explicit
    /// [`Self::flush_bulk`] calls between bursts — saving the
    /// per-call branch + spill that `write_bits_64`'s overflow check
    /// pays.
    #[inline(always)]
    pub(crate) unsafe fn write_bits_64_no_check(&mut self, bits: u64, num_bits: usize) {
        // num_bits == 0 short-circuit: matches upstream zstd `BIT_addBits` no-op
        // semantics AND guards the `bits << self.bits_in_partial` below
        // from a `<< 64` undefined-behaviour evaluation when the
        // accumulator is already full (`bits_in_partial == 64`). Callers
        // that legitimately drain a full container (e.g. the FSE encoder
        // hitting a state-diff burst boundary) can call this with
        // `num_bits = 0` as a no-op without tripping UB.
        if num_bits == 0 {
            return;
        }
        debug_assert!(
            num_bits + self.bits_in_partial <= 64,
            "write_bits_64_no_check would overflow partial: would push to {} bits",
            num_bits + self.bits_in_partial,
        );
        debug_assert!(
            self.bits_in_partial < 64,
            "write_bits_64_no_check called with full accumulator and num_bits>0; \
             caller must flush_bulk before adding more bits",
        );
        debug_assert!(
            num_bits == 64 || bits >> num_bits == 0,
            "value has dirty high bits beyond num_bits={num_bits}",
        );
        self.partial |= bits << self.bits_in_partial;
        self.bits_in_partial += num_bits;
    }

    /// Upstream zstd `BIT_flushBitsFast` (`bitstream.h:202-214`): write the
    /// full 8 bytes of `partial` directly to the output buffer via a
    /// single `MEM_writeLEST` (unaligned 8-byte LE store), then
    /// advance the Vec's `len` by only `nb_bytes` so the scratch
    /// bytes past the commit point get overwritten by the next
    /// flush. No overflow check — caller must have pre-reserved at
    /// least 8 bytes of spare capacity via [`Self::reserve_output`].
    ///
    /// `extend_from_slice` for tiny (0..=7-byte) tails was previously
    /// hot — its per-call capacity check + memcpy dispatch cost
    /// dwarfs the actual byte write at FSE flush cadence (~54K
    /// flushes per compress on the L1 fast path). The direct
    /// unaligned store path matches upstream zstd's hot loop cycle-for-cycle.
    ///
    /// # Safety
    ///
    /// Caller MUST guarantee that `output.capacity() >= output.len() +
    /// 8` BEFORE calling this method (typically via a prior
    /// [`Self::reserve_output`]). Violating the precondition produces a
    /// 8-byte out-of-bounds write to whatever memory follows the
    /// `Vec`'s allocation — undefined behaviour. The accompanying
    /// `debug_assert!` catches misuse in test/debug builds but does
    /// NOT survive `cargo build --release`, hence the `unsafe`
    /// signature.
    #[inline(always)]
    pub(crate) unsafe fn flush_bulk(&mut self) {
        let nb_bytes = self.bits_in_partial >> 3;
        let bytes = self.partial.to_le_bytes();
        let output = self.output.as_mut();
        let len = output.len();
        debug_assert!(
            output.capacity() >= len + 8,
            "flush_bulk requires 8 bytes of spare capacity; caller forgot reserve_output",
        );
        // SAFETY: the function-level Safety contract requires
        // `output.capacity() >= len + 8`. We write 8 bytes starting at
        // `len`, then commit only `nb_bytes` — the remaining `8 -
        // nb_bytes` bytes stay within capacity but past `len`, and
        // the next flush_bulk overwrites them.
        unsafe {
            let dst = output.as_mut_ptr().add(len);
            core::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, 8);
            output.set_len(len + nb_bytes);
        }
        // `nb_bytes == 8` means the accumulator was full (64 bits). A
        // raw `partial >>= 64` is UB on `u64`, so we zero explicitly.
        // Upstream zstd's `BIT_flushBitsFast` writes a fresh 8 bytes the next
        // round so the post-flush state must be clean either way.
        if nb_bytes == 8 {
            self.partial = 0;
        } else {
            self.partial >>= nb_bytes * 8;
        }
        self.bits_in_partial &= 7;
        self.bit_idx += nb_bytes * 8;
    }

    /// Bridge for the upstream zstd-faithful Huffman encoder (`HufCStream`) so
    /// it can write bytes directly into our backing `Vec<u8>` without
    /// going through the `BitWriter`'s partial-bit accumulator. The
    /// closure receives full mutable access to the underlying `Vec`;
    /// any bytes it appends are integrated into `bit_idx` afterward.
    ///
    /// MUST be called only when the writer is byte-aligned
    /// (`bits_in_partial` a multiple of 8); the assertion mirrors
    /// `append_bytes`. Internally calls `flush()` first so the
    /// closure sees a Vec whose `len()` reflects every bit written so
    /// far.
    pub(crate) fn with_aligned_output_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Vec<u8>) -> R,
    {
        assert!(
            self.bits_in_partial.is_multiple_of(8),
            "with_aligned_output_mut requires byte-aligned writer state",
        );
        self.flush();
        let prev_len = self.output.as_mut().len();
        let result = f(self.output.as_mut());
        let new_len = self.output.as_mut().len();
        // Closure may only APPEND bytes (HufCStream's contract).
        // Promoted to `assert!` (release-active) — a shrink here
        // would underflow `(new_len - prev_len) * 8` in release
        // and corrupt `bit_idx` into a phantom future bit, which
        // propagates silently through downstream `change_bits`
        // callers. This is a correctness invariant, not a debug
        // aid.
        assert!(new_len >= prev_len, "closure must not shrink output");
        self.bit_idx += (new_len - prev_len) * 8;
        result
    }

    /// Flush temporary internal buffers to the output buffer. Only works if this is currently byte aligned
    pub(crate) fn flush(&mut self) {
        assert!(self.bits_in_partial.is_multiple_of(8));
        let full_bytes = self.bits_in_partial / 8;
        self.output
            .as_mut()
            .extend_from_slice(&self.partial.to_le_bytes()[..full_bytes]);
        // `full_bytes == 8` (full accumulator) means a raw
        // `partial >>= 64` would be UB on `u64`. This SAFE function is
        // reachable indirectly via `with_aligned_output_mut`,
        // `change_bits`, and `append_bytes`, so the state IS reachable
        // — zero explicitly when the whole word is consumed instead of
        // shifting.
        if full_bytes == 8 {
            self.partial = 0;
        } else {
            self.partial >>= full_bytes * 8;
        }
        self.bits_in_partial -= full_bytes * 8;
        self.bit_idx += full_bytes * 8;
    }

    /// Write the lower `num_bits` from `bits` into the writer. `bits` *MUST* only contain zeroes in the upper bits outside of the `0..num_bits` range.
    pub(crate) fn write_bits(&mut self, bits: impl Into<u64>, num_bits: usize) {
        self.write_bits_64(bits.into(), num_bits);
    }

    /// This is the special case where we need to flush the partial buffer to the output.
    /// Marked as cold and in a separate function so the optimizer has more information.
    #[cold]
    fn write_bits_64_cold(&mut self, bits: u64, num_bits: usize) {
        assert!(self.bits_in_partial + num_bits >= 64);
        // Fill the partial buffer so it contains 64 bits
        let bits_free_in_partial = 64 - self.bits_in_partial;
        let part = bits << (64 - bits_free_in_partial);
        let merged = self.partial | part;
        // Put the 8 bytes into the output buffer
        self.output
            .as_mut()
            .extend_from_slice(&merged.to_le_bytes());
        self.bit_idx += 64;
        self.partial = 0;
        self.bits_in_partial = 0;

        let mut num_bits = num_bits - bits_free_in_partial;
        let mut bits = bits >> bits_free_in_partial;

        // While we are at it push full bytes into the output buffer instead of polluting the partial buffer
        while num_bits / 8 > 0 {
            let byte = bits as u8;
            self.output.as_mut().push(byte);
            num_bits -= 8;
            self.bit_idx += 8;
            bits >>= 8;
        }

        // The last few bits belong into the partial buffer
        assert!(num_bits < 8);
        if num_bits > 0 {
            let mask = (1 << num_bits) - 1;
            self.partial = bits & mask;
            self.bits_in_partial = num_bits;
        }
    }

    /// Monomorphized version of `change_bits`
    pub(crate) fn write_bits_64(&mut self, bits: u64, num_bits: usize) {
        if num_bits == 0 {
            return;
        }

        // Range gate: `num_bits > 64` would make the dirty-upper-bits
        // shift below `bits >> num_bits` invoke IR-level shift overflow
        // (panic in debug, UB in release-without-overflow-checks). The
        // safe public API must reject this before the shift.
        assert!(
            num_bits <= 64,
            "write_bits_64 num_bits={num_bits} exceeds u64 width",
        );

        // Full-word fast path: when `num_bits == 64` AND the partial
        // buffer is empty, write the whole u64 directly. Falls back to
        // the normal path otherwise — that path's
        // `write_bits_64_cold` does `bits >> bits_free_in_partial`
        // where `bits_free_in_partial = 64 - bits_in_partial`. With
        // `bits_in_partial == 0` and `num_bits == 64` the shift is by
        // 64 (UB). Routing the empty-partial / num_bits==64 case
        // through this fast path avoids the cold-path shift entirely;
        // for `num_bits == 64` with non-empty partial, the cold path's
        // shift is `<= 63` which is well-defined.
        if num_bits == 64 && self.bits_in_partial == 0 {
            self.output.as_mut().extend_from_slice(&bits.to_le_bytes());
            self.bit_idx += 64;
            return;
        }

        // Dirty-upper-bits contract: the caller MUST pre-mask `bits`
        // to the `num_bits` low-bit range. Violation silently corrupts
        // the output bitstream because subsequent `write_bits_64`
        // calls would OR the spurious upper bits into the partial
        // accumulator at the wrong position. Enforce in release too
        // (was `debug_assert!`) — the per-call cost is one shift +
        // compare + predicted-not-taken branch, negligible vs the
        // encoder's per-token shift/merge work, while the safety
        // value is real (encoder call sites are internal but a wrong
        // caller would produce a corrupt compressed blob with no
        // diagnostic signal).
        //
        // The `num_bits == 64` arm short-circuits the shift entirely
        // (right-shift-by-64 on u64 is IR-level UB even though the
        // value is in range here) — the upper-bits check is trivially
        // satisfied because `num_bits == 64` covers the whole u64.
        // For `num_bits < 64`, `bits >> num_bits == 0` is the strict
        // form (the prior `bits.ilog2() <= num_bits` was off-by-one:
        // it accepted `bits == 1 << num_bits` which occupies
        // `num_bits + 1` bits) and is one ALU op vs ilog2's LZCNT
        // round-trip.
        assert!(
            num_bits == 64 || bits >> num_bits == 0,
            "write_bits_64 dirty upper bits: bits=0x{bits:x} occupies {} bits but num_bits={num_bits}",
            u64::BITS - bits.leading_zeros(),
        );

        // fill partial byte first
        if num_bits + self.bits_in_partial < 64 {
            let part = bits << self.bits_in_partial;
            let merged = self.partial | part;
            self.partial = merged;
            self.bits_in_partial += num_bits;
        } else {
            // If the partial buffer can't hold the num_bits we need to make space
            self.write_bits_64_cold(bits, num_bits);
        }
    }

    /// Returns the populated buffer that you've been writing bits into.
    ///
    /// This function consumes the writer, so it cannot be used after
    /// dumping

    pub(crate) fn dump(mut self) -> V {
        if self.misaligned() != 0 {
            panic!(
                "`dump` was called on a bit writer but an even number of bytes weren't written into the buffer. Was: {}",
                self.index()
            )
        }
        self.flush();
        debug_assert_eq!(self.partial, 0);
        self.output
    }

    /// Returns how many bits are missing for an even byte
    pub(crate) fn misaligned(&self) -> usize {
        let idx = self.index();
        if idx.is_multiple_of(8) {
            0
        } else {
            8 - (idx % 8)
        }
    }
}
