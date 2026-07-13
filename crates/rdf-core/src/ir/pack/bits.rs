// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Succinct bit-packing primitives: a fixed-width bit-packed integer array
//! ([`IntVector`]), a rank1/select1 succinct bitmap ([`BitVec`] builder →
//! [`RankSelect`] frozen view), and varint/zigzag/delta byte codecs.
//!
//! Every structure here has an OWNED, growable builder form and (where the format
//! is later read back from a byte buffer) a borrowed, zero-copy `*Ref` reader that
//! aliases the caller's bytes directly rather than allocating — the shape later
//! `pack` tasks (the value dictionary, FoQ inverted lists, bitmap-triples) need to
//! memory-map or embed these blocks inside a larger encoded buffer without a
//! deserialize pass. Readers decode multi-byte fields via `from_le_bytes` on
//! explicit byte-slice copies rather than pointer casts, so the caller's buffer
//! need not be aligned to the field's native alignment (`u64`/`u16`) — the whole
//! point of a borrowed reader over caller-owned bytes.
//!
//! `std`-only: no new dependencies, no threads, no filesystem, no wall-clock, no
//! RNG — `wasm32-unknown-unknown`-clean by construction.

use std::fmt;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why decoding a `pack` byte buffer failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackBitsError {
    /// The buffer ended before all the bytes a header promised were present.
    Truncated {
        /// The total leading byte count the format required.
        needed: usize,
        /// The byte count actually available.
        found: usize,
    },
    /// The buffer's header was internally inconsistent (e.g. a stored count that
    /// disagrees with one derived from another header field), or a value fell
    /// outside its documented domain (e.g. an `IntVector` width `> 64`).
    Malformed(&'static str),
}

impl fmt::Display for PackBitsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { needed, found } => write!(
                f,
                "pack: truncated input: needed at least {needed} bytes, found {found}"
            ),
            Self::Malformed(reason) => write!(f, "pack: malformed input: {reason}"),
        }
    }
}

impl std::error::Error for PackBitsError {}

// ---------------------------------------------------------------------------
// Small byte-header helpers shared by every codec in this file.
// ---------------------------------------------------------------------------

/// Read an 8-byte little-endian header field at `*pos`, advancing `*pos` past it.
fn read_header_u64(bytes: &[u8], pos: &mut usize) -> Result<u64, PackBitsError> {
    let end = *pos + 8;
    let slice = bytes.get(*pos..end).ok_or(PackBitsError::Truncated {
        needed: end,
        found: bytes.len(),
    })?;
    let value = u64::from_le_bytes(slice.try_into().expect("slice is exactly 8 bytes"));
    *pos = end;
    Ok(value)
}

/// Read a 4-byte little-endian header field at `*pos`, advancing `*pos` past it.
fn read_header_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, PackBitsError> {
    let end = *pos + 4;
    let slice = bytes.get(*pos..end).ok_or(PackBitsError::Truncated {
        needed: end,
        found: bytes.len(),
    })?;
    let value = u32::from_le_bytes(slice.try_into().expect("slice is exactly 4 bytes"));
    *pos = end;
    Ok(value)
}

/// Read the `idx`-th little-endian `u64` word from a byte slice known to hold at
/// least `(idx + 1) * 8` bytes. Alignment-agnostic (see the [module docs](self)).
fn read_u64_le(bytes: &[u8], idx: usize) -> u64 {
    let start = idx * 8;
    u64::from_le_bytes(
        bytes[start..start + 8]
            .try_into()
            .expect("slice is exactly 8 bytes"),
    )
}

/// Read the `idx`-th little-endian `u16` from a byte slice, alignment-agnostic
/// (see [`read_u64_le`]).
fn read_u16_le(bytes: &[u8], idx: usize) -> u16 {
    let start = idx * 2;
    u16::from_le_bytes(
        bytes[start..start + 2]
            .try_into()
            .expect("slice is exactly 2 bytes"),
    )
}

// ---------------------------------------------------------------------------
// IntVector — fixed-width bit-packed unsigned integer array.
// ---------------------------------------------------------------------------

/// The number of bits needed to represent every value in `0..=max_value`
/// (`ceil(log2(max_value + 1))`), the width a caller should hand to
/// [`IntVector::with_width`].
///
/// `bits_for(0) == 0`: a vector whose only representable value is `0` needs no
/// storage bits at all — every [`IntVector::get`] on a width-0 vector returns `0`
/// without touching the (empty) backing store.
#[must_use]
pub fn bits_for(max_value: u64) -> u32 {
    64 - max_value.leading_zeros()
}

/// The `width`-bit mask: all-ones for `width == 64`, `0` for `width == 0`.
fn mask_for_width(width: u32) -> u64 {
    if width == 0 {
        0
    } else if width == 64 {
        u64::MAX
    } else {
        (1u64 << width) - 1
    }
}

/// A fixed-width bit-packed unsigned integer array: `n` values, each stored in
/// exactly `width` bits (`width` in `0..=64`), packed LSB-first into a flat
/// `Vec<u64>` of backing words — value `i` occupies logical bitstream positions
/// `[i * width, (i + 1) * width)`, and MAY straddle two backing words when `width`
/// does not divide 64.
///
/// Use [`bits_for`] to choose `width` from the maximum value the vector will ever
/// hold. This is the builder; [`to_bytes`](Self::to_bytes) serializes it and
/// [`IntVectorRef`] reads it back without allocating.
#[derive(Debug, Clone)]
pub struct IntVector {
    /// Bits per stored value, `0..=64`.
    width: u32,
    /// Number of stored values.
    len: usize,
    /// The flat LSB-first backing words.
    words: Vec<u64>,
}

impl IntVector {
    /// Start an empty vector storing values in exactly `width` bits.
    ///
    /// # Panics
    ///
    /// Panics if `width > 64`.
    #[must_use]
    pub fn with_width(width: u32) -> Self {
        assert!(
            width <= 64,
            "IntVector::with_width: width must be <= 64, got {width}"
        );
        Self {
            width,
            len: 0,
            words: Vec::new(),
        }
    }

    /// Append `value`.
    ///
    /// # Panics
    ///
    /// Debug-asserts `value` fits in `width` bits (always true for `width == 64`).
    pub fn push(&mut self, value: u64) {
        debug_assert!(
            self.width == 64 || value < (1u64 << self.width),
            "IntVector::push: value {value} does not fit in {} bits",
            self.width
        );
        if self.width == 0 {
            // A width-0 vector stores only the value 0, entirely implicit — no
            // backing bits are ever written or read.
            self.len += 1;
            return;
        }
        let bit_pos = self.len as u64 * u64::from(self.width);
        let word_index = (bit_pos / 64) as usize;
        let bit_offset = bit_pos % 64;
        let bits_needed = bit_pos + u64::from(self.width);
        let words_needed = bits_needed.div_ceil(64) as usize;
        if self.words.len() < words_needed {
            self.words.resize(words_needed, 0);
        }
        // Low part: bits [0, min(width, 64 - bit_offset)) of `value` land at
        // [bit_offset, bit_offset + width) of this word; any bits of `value` at or
        // beyond position (64 - bit_offset) are shifted out of the u64 register and
        // silently dropped here — that is exactly correct, they belong to the next
        // word instead.
        self.words[word_index] |= value << bit_offset;
        if bit_offset + u64::from(self.width) > 64 {
            // Straddle: the remaining high bits of `value` (positions
            // [64 - bit_offset, width)) go to bit 0.. of the next word.
            let low_bits = 64 - bit_offset;
            self.words[word_index + 1] |= value >> low_bits;
        }
        self.len += 1;
    }

    /// The number of stored values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` iff no values have been pushed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The configured bit width.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Random access, `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `i >= len()`.
    #[must_use]
    pub fn get(&self, i: usize) -> u64 {
        assert!(
            i < self.len,
            "IntVector::get: index {i} out of range 0..{}",
            self.len
        );
        if self.width == 0 {
            return 0;
        }
        let bit_pos = i as u64 * u64::from(self.width);
        let word_index = (bit_pos / 64) as usize;
        let bit_offset = bit_pos % 64;
        let mut value = self.words[word_index] >> bit_offset;
        if bit_offset + u64::from(self.width) > 64 {
            let low_bits = 64 - bit_offset;
            value |= self.words[word_index + 1] << low_bits;
        }
        value & mask_for_width(self.width)
    }

    /// Serialize to a self-describing little-endian byte buffer: an 8-byte `len`
    /// prefix, a 4-byte `width` prefix, then the backing words (8 bytes each, LE).
    /// [`IntVectorRef::from_bytes`] is the exact inverse.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.serialized_len());
        out.extend_from_slice(&(self.len as u64).to_le_bytes());
        out.extend_from_slice(&self.width.to_le_bytes());
        for &w in &self.words {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    }

    /// The exact byte length [`to_bytes`](Self::to_bytes) produces — lets a caller
    /// embedding several `pack` blocks in one buffer size/advance without
    /// re-serializing.
    #[must_use]
    pub fn serialized_len(&self) -> usize {
        12 + self.words.len() * 8
    }
}

/// A borrowed, zero-copy reader over an [`IntVector::to_bytes`] buffer: aliases
/// the backing words directly out of the caller's byte slice (no allocation, no
/// copy). See the [module docs](self) for why reads go through `from_le_bytes`
/// rather than a pointer cast.
#[derive(Debug, Clone, Copy)]
pub struct IntVectorRef<'a> {
    width: u32,
    len: usize,
    /// Exactly `ceil(len * width / 64) * 8` bytes: the LE-encoded backing words.
    words: &'a [u8],
}

impl<'a> IntVectorRef<'a> {
    /// Parse an [`IntVector::to_bytes`] buffer. `bytes` may carry trailing data
    /// after this vector's own encoding (e.g. a sibling block concatenated after
    /// it) — [`serialized_len`](Self::serialized_len) reports how many leading
    /// bytes were consumed so the caller can advance a cursor past exactly that
    /// many.
    ///
    /// # Errors
    ///
    /// [`PackBitsError::Truncated`] if `bytes` is shorter than the header
    /// promises; [`PackBitsError::Malformed`] if the header's `width` exceeds 64.
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, PackBitsError> {
        let mut pos = 0usize;
        let len = read_header_u64(bytes, &mut pos)? as usize;
        let width = read_header_u32(bytes, &mut pos)?;
        if width > 64 {
            return Err(PackBitsError::Malformed("int_vector: width exceeds 64"));
        }
        let total_bits = len as u64 * u64::from(width);
        let words_len = total_bits.div_ceil(64) as usize;
        let words_bytes_len = words_len * 8;
        let end = pos + words_bytes_len;
        if bytes.len() < end {
            return Err(PackBitsError::Truncated {
                needed: end,
                found: bytes.len(),
            });
        }
        Ok(Self {
            width,
            len,
            words: &bytes[pos..end],
        })
    }

    /// The number of stored values.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` iff this vector stores no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The configured bit width.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The number of leading bytes of the buffer passed to
    /// [`from_bytes`](Self::from_bytes) that this reader consumed.
    #[must_use]
    pub fn serialized_len(&self) -> usize {
        12 + self.words.len()
    }

    /// Random access, `O(1)`; see [`IntVector::get`].
    ///
    /// # Panics
    ///
    /// Panics if `i >= len()`.
    #[must_use]
    pub fn get(&self, i: usize) -> u64 {
        assert!(
            i < self.len,
            "IntVectorRef::get: index {i} out of range 0..{}",
            self.len
        );
        if self.width == 0 {
            return 0;
        }
        let bit_pos = i as u64 * u64::from(self.width);
        let word_index = (bit_pos / 64) as usize;
        let bit_offset = bit_pos % 64;
        let mut value = read_u64_le(self.words, word_index) >> bit_offset;
        if bit_offset + u64::from(self.width) > 64 {
            let low_bits = 64 - bit_offset;
            value |= read_u64_le(self.words, word_index + 1) << low_bits;
        }
        value & mask_for_width(self.width)
    }
}

// ---------------------------------------------------------------------------
// BitVec / RankSelect — succinct rank1/select1/rank0/select0 bitmap.
// ---------------------------------------------------------------------------

/// Bits per rank/select superblock: the directory caches an ABSOLUTE rank every
/// [`SUPERBLOCK_BITS`] bits. 512 is a standard choice (small enough that the
/// linear in-superblock block scan below is cheap, large enough that the
/// superblock directory itself stays compact).
const SUPERBLOCK_BITS: usize = 512;
/// Backing words (64 bits each) per superblock (`SUPERBLOCK_BITS / 64`).
const WORDS_PER_SUPERBLOCK: usize = SUPERBLOCK_BITS / 64;

/// A growable bit sequence, backed by `Vec<u64>` words (bit `i` lives in word
/// `i / 64` at position `i % 64`, LSB-first) — the builder for a frozen
/// [`RankSelect`].
#[derive(Debug, Clone, Default)]
pub struct BitVec {
    words: Vec<u64>,
    len: usize,
}

impl BitVec {
    /// An empty bit sequence.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append one bit.
    pub fn push(&mut self, bit: bool) {
        let word_index = self.len / 64;
        if word_index >= self.words.len() {
            self.words.push(0);
        }
        if bit {
            self.words[word_index] |= 1u64 << (self.len % 64);
        }
        self.len += 1;
    }

    /// The number of bits pushed so far.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` iff no bits have been pushed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Grow or truncate to exactly `new_len` bits: growing zero-fills the new
    /// tail; truncating clears any now-out-of-range tail bits still sitting in the
    /// boundary word, so the backing words always stay a clean zero-padded prefix
    /// (the same invariant `push`-only construction produces).
    pub fn set_len(&mut self, new_len: usize) {
        let words_needed = new_len.div_ceil(64);
        self.words.resize(words_needed, 0);
        if !new_len.is_multiple_of(64) {
            let boundary_word = new_len / 64;
            let keep_bits = new_len % 64;
            let mask = (1u64 << keep_bits) - 1;
            self.words[boundary_word] &= mask;
        }
        self.len = new_len;
    }

    /// Freeze into a [`RankSelect`], building the two-level rank directory.
    #[must_use]
    pub fn freeze(self) -> RankSelect {
        RankSelect::build(self.words, self.len)
    }
}

/// The bit position (`0..64`) of the `r`-th (0-based) set bit of `word`.
///
/// Skips whole bytes via `u8::count_ones` before falling back to a bit-by-bit
/// scan of the one byte that contains the target bit (a `count_ones`-guided
/// scan followed by a within-word bit select).
///
/// # Panics
///
/// Panics if `r >= word.count_ones()`; every call site bounds `r` by the rank
/// directory first, so this is a broken-invariant bug if it ever fires.
fn select_in_word(word: u64, r: usize) -> u32 {
    let mut remaining = r;
    let mut base = 0u32;
    let mut w = word;
    while base < 64 {
        let byte = (w & 0xFF) as u8;
        let ones = byte.count_ones() as usize;
        if remaining < ones {
            break;
        }
        remaining -= ones;
        w >>= 8;
        base += 8;
    }
    assert!(
        base < 64,
        "select_in_word: r out of range for word's popcount"
    );
    let mut byte = (w & 0xFF) as u8;
    loop {
        if byte & 1 == 1 {
            if remaining == 0 {
                return base;
            }
            remaining -= 1;
        }
        byte >>= 1;
        base += 1;
    }
}

/// Shared rank/select directory algorithm, implemented once and reused by both
/// the owned [`RankSelect`] and the borrowed [`RankSelectRef`] — the two differ
/// only in HOW they store `words` / `superblock_rank` / `block_rank` (owned
/// slices vs. raw borrowed bytes), not in the rank/select math itself.
///
/// Required methods give `O(1)` element access into the three backing arrays;
/// the provided `dir_*` default methods build `rank1`/`rank0`/`select1`/`select0`
/// on top. `RankSelect`/`RankSelectRef` each expose their own INHERENT methods of
/// the same name (`rank1`, `select1`, ...) that forward here via fully-qualified
/// syntax — Rust always prefers an inherent method over a trait one for ordinary
/// `value.method()` calls, so the public API is the inherent methods; this trait
/// is purely a private code-sharing device.
trait RankSelectDir {
    /// The bitmap's length in bits.
    fn dir_len(&self) -> usize;
    /// The total number of set bits.
    fn dir_total_ones(&self) -> usize;
    /// The number of 64-bit backing words.
    fn dir_words_len(&self) -> usize;
    /// The `word_index`-th backing word.
    fn dir_word(&self, word_index: usize) -> u64;
    /// The number of superblocks in the directory.
    fn dir_superblock_count(&self) -> usize;
    /// The absolute rank at the start of superblock `sb`.
    fn dir_superblock_rank(&self, sb: usize) -> u64;
    /// The rank at the start of word `word_index`, RELATIVE to its superblock.
    fn dir_block_rank(&self, word_index: usize) -> u64;

    /// Absolute rank at the START of `word_index` (i.e. `rank1(word_index * 64)`),
    /// `O(1)`. `word_index == dir_words_len()` (one past the last word) is a valid
    /// input meaning "the whole bitmap" and returns `dir_total_ones()`.
    fn rank_at_word(&self, word_index: usize) -> u64 {
        if word_index >= self.dir_words_len() {
            return self.dir_total_ones() as u64;
        }
        let sb = word_index / WORDS_PER_SUPERBLOCK;
        self.dir_superblock_rank(sb) + self.dir_block_rank(word_index)
    }

    /// Number of set bits in `[0, i)`. `O(1)`: one direct superblock-array index,
    /// one direct block-array index, plus a single-word popcount for the partial
    /// tail — no search.
    ///
    /// # Panics
    ///
    /// Panics if `i > dir_len()`.
    fn rank1(&self, i: usize) -> usize {
        assert!(
            i <= self.dir_len(),
            "rank1: index {i} out of range 0..={}",
            self.dir_len()
        );
        let word_index = i / 64;
        let partial = i % 64;
        let mut r = self.rank_at_word(word_index);
        if partial > 0 {
            let mask = (1u64 << partial) - 1;
            r += u64::from((self.dir_word(word_index) & mask).count_ones());
        }
        r as usize
    }

    /// Number of unset bits in `[0, i)`. `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `i > dir_len()`.
    fn rank0(&self, i: usize) -> usize {
        i - self.rank1(i)
    }

    /// The position of the `k`-th (0-based) set bit, or `None` if `k >=
    /// dir_total_ones()`.
    ///
    /// `O(log(n / SUPERBLOCK_BITS))`: a binary search over the superblock array,
    /// then a linear scan of at most [`WORDS_PER_SUPERBLOCK`] blocks within it,
    /// then a byte-guided within-word scan ([`select_in_word`]).
    fn select1(&self, k: usize) -> Option<usize> {
        if k >= self.dir_total_ones() {
            return None;
        }
        let k64 = k as u64;
        let sb_count = self.dir_superblock_count();
        let mut lo = 0usize;
        let mut hi = sb_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if self.dir_superblock_rank(mid) <= k64 {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        // `dir_superblock_rank(0) == 0 <= k` always holds, so `lo >= 1` here.
        let sb = lo - 1;
        let w_start = sb * WORDS_PER_SUPERBLOCK;
        let w_end = (w_start + WORDS_PER_SUPERBLOCK).min(self.dir_words_len());
        let mut w = w_start;
        while w + 1 < w_end && self.rank_at_word(w + 1) <= k64 {
            w += 1;
        }
        let remaining = (k64 - self.rank_at_word(w)) as usize;
        let bit = select_in_word(self.dir_word(w), remaining);
        let pos = w * 64 + bit as usize;
        debug_assert!(pos < self.dir_len());
        Some(pos)
    }

    /// The position of the `k`-th (0-based) unset bit, or `None` if `k >=
    /// dir_len() - dir_total_ones()`. Same complexity as [`select1`](Self::select1);
    /// zero-counts are derived from the SAME ones-based directory
    /// (`zeros_before(x) == x - ones_before(x)`), so no second directory is kept.
    fn select0(&self, k: usize) -> Option<usize> {
        let total_zeros = self.dir_len() - self.dir_total_ones();
        if k >= total_zeros {
            return None;
        }
        let k64 = k as u64;
        let zeros_at_sb = |sb: usize| -> u64 {
            (sb * WORDS_PER_SUPERBLOCK) as u64 * 64 - self.dir_superblock_rank(sb)
        };
        let zeros_at_word = |w: usize| -> u64 { w as u64 * 64 - self.rank_at_word(w) };
        let sb_count = self.dir_superblock_count();
        let mut lo = 0usize;
        let mut hi = sb_count;
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if zeros_at_sb(mid) <= k64 {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        let sb = lo - 1;
        let w_start = sb * WORDS_PER_SUPERBLOCK;
        let w_end = (w_start + WORDS_PER_SUPERBLOCK).min(self.dir_words_len());
        let mut w = w_start;
        while w + 1 < w_end && zeros_at_word(w + 1) <= k64 {
            w += 1;
        }
        let remaining = (k64 - zeros_at_word(w)) as usize;
        let bit = select_in_word(!self.dir_word(w), remaining);
        let pos = w * 64 + bit as usize;
        debug_assert!(pos < self.dir_len());
        Some(pos)
    }
}

/// A frozen succinct bitmap with `O(1)` `rank1`/`rank0` and
/// `O(log(n / 512))` `select1`/`select0`, built by [`BitVec::freeze`].
///
/// # Directory layout
///
/// Two levels (`SUPERBLOCK_BITS` = 512, `WORDS_PER_SUPERBLOCK` = 8, both private
/// constants in this module):
///
/// - **Superblock** — one absolute rank (`u64`) cached every 512 bits (every 8
///   backing words).
/// - **Block** — one rank (`u16`, sufficient since a superblock holds at most 512
///   ones), cached per 64-bit word, RELATIVE to its superblock's start.
///
/// The shared rank/select algorithm over this directory (also used by the
/// borrowed [`RankSelectRef`]) is a private `RankSelectDir` trait in this module.
#[derive(Debug, Clone)]
pub struct RankSelect {
    len: usize,
    total_ones: usize,
    words: Box<[u64]>,
    /// Absolute rank at the start of each superblock
    /// (`len == ceil(words.len() / WORDS_PER_SUPERBLOCK)`).
    superblock_rank: Box<[u64]>,
    /// Rank at the start of each word, RELATIVE to its superblock
    /// (`len == words.len()`).
    block_rank: Box<[u16]>,
}

impl RankSelect {
    /// Build the two-level directory over `words` (a `len`-bit bitmap, zero-padded
    /// to the word boundary) in one linear pass.
    fn build(words: Vec<u64>, len: usize) -> Self {
        let words: Box<[u64]> = words.into_boxed_slice();
        let words_len = words.len();
        let superblock_count = words_len.div_ceil(WORDS_PER_SUPERBLOCK);
        let mut superblock_rank = vec![0u64; superblock_count];
        let mut block_rank = vec![0u16; words_len];
        let mut running_total = 0u64;
        for (w, &word) in words.iter().enumerate() {
            if w % WORDS_PER_SUPERBLOCK == 0 {
                superblock_rank[w / WORDS_PER_SUPERBLOCK] = running_total;
            }
            let sb_base = superblock_rank[w / WORDS_PER_SUPERBLOCK];
            block_rank[w] = u16::try_from(running_total - sb_base)
                .expect("superblock-relative rank fits u16 (<= SUPERBLOCK_BITS ones)");
            running_total += u64::from(word.count_ones());
        }
        Self {
            len,
            total_ones: running_total as usize,
            words,
            superblock_rank: superblock_rank.into_boxed_slice(),
            block_rank: block_rank.into_boxed_slice(),
        }
    }

    /// The bitmap's length in bits.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` iff the bitmap has zero length.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The total number of set bits.
    #[must_use]
    pub fn total_ones(&self) -> usize {
        self.total_ones
    }

    /// Number of set bits in `[0, i)`. `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `i > len()`.
    #[must_use]
    pub fn rank1(&self, i: usize) -> usize {
        RankSelectDir::rank1(self, i)
    }

    /// Number of unset bits in `[0, i)`. `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `i > len()`.
    #[must_use]
    pub fn rank0(&self, i: usize) -> usize {
        RankSelectDir::rank0(self, i)
    }

    /// The position of the `k`-th (0-based) set bit, or `None` if `k >=
    /// total_ones()`.
    #[must_use]
    pub fn select1(&self, k: usize) -> Option<usize> {
        RankSelectDir::select1(self, k)
    }

    /// The position of the `k`-th (0-based) unset bit, or `None` if `k >= len() -
    /// total_ones()`.
    #[must_use]
    pub fn select0(&self, k: usize) -> Option<usize> {
        RankSelectDir::select0(self, k)
    }

    /// Serialize to a self-describing little-endian byte buffer:
    /// `len`, `total_ones`, `words.len()`, `superblock_rank.len()` (each an 8-byte
    /// LE `u64` header field), then `superblock_rank` (`u64` each), `block_rank`
    /// (`u16` each), then `words` (`u64` each) — all little-endian.
    /// [`RankSelectRef::from_bytes`] is the exact inverse.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.serialized_len());
        out.extend_from_slice(&(self.len as u64).to_le_bytes());
        out.extend_from_slice(&(self.total_ones as u64).to_le_bytes());
        out.extend_from_slice(&(self.words.len() as u64).to_le_bytes());
        out.extend_from_slice(&(self.superblock_rank.len() as u64).to_le_bytes());
        for &r in &self.superblock_rank {
            out.extend_from_slice(&r.to_le_bytes());
        }
        for &r in &self.block_rank {
            out.extend_from_slice(&r.to_le_bytes());
        }
        for &w in &self.words {
            out.extend_from_slice(&w.to_le_bytes());
        }
        out
    }

    /// The exact byte length [`to_bytes`](Self::to_bytes) produces.
    #[must_use]
    pub fn serialized_len(&self) -> usize {
        32 + self.superblock_rank.len() * 8 + self.block_rank.len() * 2 + self.words.len() * 8
    }
}

impl RankSelectDir for RankSelect {
    fn dir_len(&self) -> usize {
        self.len
    }

    fn dir_total_ones(&self) -> usize {
        self.total_ones
    }

    fn dir_words_len(&self) -> usize {
        self.words.len()
    }

    fn dir_word(&self, word_index: usize) -> u64 {
        self.words[word_index]
    }

    fn dir_superblock_count(&self) -> usize {
        self.superblock_rank.len()
    }

    fn dir_superblock_rank(&self, sb: usize) -> u64 {
        self.superblock_rank[sb]
    }

    fn dir_block_rank(&self, word_index: usize) -> u64 {
        u64::from(self.block_rank[word_index])
    }
}

/// A borrowed, zero-copy reader over a [`RankSelect::to_bytes`] buffer: aliases
/// the backing words AND the rank directory directly out of the caller's byte
/// slice, so opening one does no allocation and no directory reconstruction (see
/// the [`RankSelect`] docs for the directory layout `rank1`/`select1` share).
#[derive(Debug, Clone, Copy)]
pub struct RankSelectRef<'a> {
    len: usize,
    total_ones: usize,
    words_len: usize,
    superblock_count: usize,
    superblock_bytes: &'a [u8],
    block_bytes: &'a [u8],
    words_bytes: &'a [u8],
}

impl<'a> RankSelectRef<'a> {
    /// Parse a [`RankSelect::to_bytes`] buffer. As with
    /// [`IntVectorRef::from_bytes`], `bytes` may carry trailing data after this
    /// block's own encoding; [`serialized_len`](Self::serialized_len) reports how
    /// many leading bytes were consumed.
    ///
    /// # Errors
    ///
    /// [`PackBitsError::Truncated`] if `bytes` is shorter than the header
    /// promises; [`PackBitsError::Malformed`] if the header is internally
    /// inconsistent (a stored word/superblock count disagreeing with the one
    /// derived from `len`, or `total_ones > len`).
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, PackBitsError> {
        let mut pos = 0usize;
        let len = read_header_u64(bytes, &mut pos)? as usize;
        let total_ones = read_header_u64(bytes, &mut pos)? as usize;
        let words_len = read_header_u64(bytes, &mut pos)? as usize;
        let superblock_count = read_header_u64(bytes, &mut pos)? as usize;
        if total_ones > len {
            return Err(PackBitsError::Malformed(
                "rank_select: total_ones exceeds len",
            ));
        }
        if words_len != len.div_ceil(64) {
            return Err(PackBitsError::Malformed(
                "rank_select: word count disagrees with len",
            ));
        }
        if superblock_count != words_len.div_ceil(WORDS_PER_SUPERBLOCK) {
            return Err(PackBitsError::Malformed(
                "rank_select: superblock count disagrees with word count",
            ));
        }
        let sb_bytes_len = superblock_count * 8;
        let block_bytes_len = words_len * 2;
        let words_bytes_len = words_len * 8;
        let sb_start = pos;
        let sb_end = sb_start + sb_bytes_len;
        let block_end = sb_end + block_bytes_len;
        let words_end = block_end + words_bytes_len;
        if bytes.len() < words_end {
            return Err(PackBitsError::Truncated {
                needed: words_end,
                found: bytes.len(),
            });
        }
        Ok(Self {
            len,
            total_ones,
            words_len,
            superblock_count,
            superblock_bytes: &bytes[sb_start..sb_end],
            block_bytes: &bytes[sb_end..block_end],
            words_bytes: &bytes[block_end..words_end],
        })
    }

    /// The bitmap's length in bits.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// `true` iff the bitmap has zero length.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The total number of set bits.
    #[must_use]
    pub fn total_ones(&self) -> usize {
        self.total_ones
    }

    /// The number of leading bytes of the buffer passed to
    /// [`from_bytes`](Self::from_bytes) that this reader consumed.
    #[must_use]
    pub fn serialized_len(&self) -> usize {
        32 + self.superblock_bytes.len() + self.block_bytes.len() + self.words_bytes.len()
    }

    /// Number of set bits in `[0, i)`. `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `i > len()`.
    #[must_use]
    pub fn rank1(&self, i: usize) -> usize {
        RankSelectDir::rank1(self, i)
    }

    /// Number of unset bits in `[0, i)`. `O(1)`.
    ///
    /// # Panics
    ///
    /// Panics if `i > len()`.
    #[must_use]
    pub fn rank0(&self, i: usize) -> usize {
        RankSelectDir::rank0(self, i)
    }

    /// The position of the `k`-th (0-based) set bit, or `None` if `k >=
    /// total_ones()`.
    #[must_use]
    pub fn select1(&self, k: usize) -> Option<usize> {
        RankSelectDir::select1(self, k)
    }

    /// The position of the `k`-th (0-based) unset bit, or `None` if `k >= len() -
    /// total_ones()`.
    #[must_use]
    pub fn select0(&self, k: usize) -> Option<usize> {
        RankSelectDir::select0(self, k)
    }
}

impl RankSelectDir for RankSelectRef<'_> {
    fn dir_len(&self) -> usize {
        self.len
    }

    fn dir_total_ones(&self) -> usize {
        self.total_ones
    }

    fn dir_words_len(&self) -> usize {
        self.words_len
    }

    fn dir_word(&self, word_index: usize) -> u64 {
        read_u64_le(self.words_bytes, word_index)
    }

    fn dir_superblock_count(&self) -> usize {
        self.superblock_count
    }

    fn dir_superblock_rank(&self, sb: usize) -> u64 {
        read_u64_le(self.superblock_bytes, sb)
    }

    fn dir_block_rank(&self, word_index: usize) -> u64 {
        u64::from(read_u16_le(self.block_bytes, word_index))
    }
}

// ---------------------------------------------------------------------------
// Varint / zigzag / delta-list helpers (for the later FoQ inverted lists).
// ---------------------------------------------------------------------------

/// Append `value` as an unsigned LEB128 varint (7 payload bits per byte, MSB =
/// continuation flag): 1..=10 bytes.
pub fn write_varint(out: &mut Vec<u8>, value: u64) {
    let mut v = value;
    loop {
        let byte = (v & 0x7f) as u8;
        v >>= 7;
        if v == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// Decode one unsigned LEB128 varint starting at `*pos`, advancing `*pos` past
/// it.
///
/// # Errors
///
/// [`PackBitsError::Truncated`] if the buffer ends before a terminating byte
/// (continuation bit clear) is seen; [`PackBitsError::Malformed`] if the
/// encoding runs past 64 payload bits (not a valid `u64` varint).
pub fn read_varint(bytes: &[u8], pos: &mut usize) -> Result<u64, PackBitsError> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        if shift >= 64 {
            return Err(PackBitsError::Malformed("varint exceeds 64 bits"));
        }
        let byte = *bytes.get(*pos).ok_or(PackBitsError::Truncated {
            needed: *pos + 1,
            found: bytes.len(),
        })?;
        *pos += 1;
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
    }
}

/// Map a signed integer to an unsigned one so small-magnitude values of EITHER
/// sign encode as small varints: `0, -1, 1, -2, 2, ... -> 0, 1, 2, 3, 4, ...`.
/// The standard "zigzag" mapping (as used by Protocol Buffers); round-trips every
/// `i64` including `i64::MIN`.
#[must_use]
pub fn zigzag_encode(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

/// The inverse of [`zigzag_encode`].
#[must_use]
pub fn zigzag_decode(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

/// Encode a monotonically NON-DECREASING `values` slice as successive unsigned
/// varint deltas (`values[0] - 0, values[1] - values[0], ...`), so a sorted id
/// list with small gaps compresses to a handful of bytes per entry. The element
/// COUNT is NOT embedded — a caller reconstructing the list supplies it
/// externally (e.g. from the enclosing block's own header), matching
/// [`DeltaListRef::new`].
///
/// # Panics
///
/// Debug-asserts `values` is non-decreasing.
pub fn write_delta_list(out: &mut Vec<u8>, values: &[u64]) {
    let mut prev = 0u64;
    for &v in values {
        debug_assert!(
            v >= prev,
            "write_delta_list: values must be non-decreasing (got {v} after {prev})"
        );
        write_varint(out, v - prev);
        prev = v;
    }
}

/// A borrowed, streaming reader over a [`write_delta_list`] byte stream: yields
/// the reconstructed (non-decreasing) values one at a time without allocating.
/// The caller supplies `count` (the number of values) at construction, matching
/// [`write_delta_list`]'s contract that the count is not self-describing.
#[derive(Debug, Clone)]
pub struct DeltaListRef<'a> {
    bytes: &'a [u8],
    pos: usize,
    remaining: usize,
    prev: u64,
}

impl<'a> DeltaListRef<'a> {
    /// Start reading `count` delta-encoded values from `bytes`.
    #[must_use]
    pub fn new(bytes: &'a [u8], count: usize) -> Self {
        Self {
            bytes,
            pos: 0,
            remaining: count,
            prev: 0,
        }
    }

    /// The number of leading bytes of `bytes` consumed so far — lets a caller
    /// advance past exactly this delta list when it is embedded in a larger
    /// buffer. Meaningful once the iterator is fully drained (or has errored).
    #[must_use]
    pub fn consumed_len(&self) -> usize {
        self.pos
    }
}

impl Iterator for DeltaListRef<'_> {
    type Item = Result<u64, PackBitsError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        match read_varint(self.bytes, &mut self.pos) {
            Ok(delta) => {
                self.prev += delta;
                Some(Ok(self.prev))
            }
            Err(e) => {
                // Stop after an error rather than looping on a stuck cursor.
                self.remaining = 0;
                Some(Err(e))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // -- IntVector ------------------------------------------------------------

    #[test]
    fn bits_for_boundaries() {
        assert_eq!(bits_for(0), 0);
        assert_eq!(bits_for(1), 1);
        assert_eq!(bits_for(2), 2);
        assert_eq!(bits_for(3), 2);
        assert_eq!(bits_for(4), 3);
        assert_eq!(bits_for(u64::MAX), 64);
    }

    #[test]
    fn int_vector_empty() {
        let v = IntVector::with_width(5);
        assert!(v.is_empty());
        assert_eq!(v.len(), 0);
        let bytes = v.to_bytes();
        let r = IntVectorRef::from_bytes(&bytes).expect("parses");
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn int_vector_single_element() {
        let mut v = IntVector::with_width(9);
        v.push(300);
        assert_eq!(v.len(), 1);
        assert_eq!(v.get(0), 300);
        let bytes = v.to_bytes();
        let r = IntVectorRef::from_bytes(&bytes).expect("parses");
        assert_eq!(r.get(0), 300);
    }

    #[test]
    fn int_vector_all_zeros() {
        let mut v = IntVector::with_width(11);
        for _ in 0..50 {
            v.push(0);
        }
        for i in 0..50 {
            assert_eq!(v.get(i), 0);
        }
    }

    #[test]
    fn int_vector_all_ones_bits() {
        let width = 13;
        let max = (1u64 << width) - 1;
        let mut v = IntVector::with_width(width);
        for _ in 0..40 {
            v.push(max);
        }
        for i in 0..40 {
            assert_eq!(v.get(i), max);
        }
    }

    #[test]
    fn int_vector_cross_word_width() {
        // width=9 never divides 64, so a long enough run straddles word
        // boundaries repeatedly.
        let mut v = IntVector::with_width(9);
        let values: Vec<u64> = (0..200).map(|i| (i * 7 + 3) % 512).collect();
        for &val in &values {
            v.push(val);
        }
        for (i, &val) in values.iter().enumerate() {
            assert_eq!(v.get(i), val, "mismatch at index {i}");
        }
        let bytes = v.to_bytes();
        let r = IntVectorRef::from_bytes(&bytes).expect("parses");
        for (i, &val) in values.iter().enumerate() {
            assert_eq!(r.get(i), val, "ref mismatch at index {i}");
        }
    }

    #[test]
    fn int_vector_width_zero() {
        let mut v = IntVector::with_width(0);
        for _ in 0..10 {
            v.push(0);
        }
        assert_eq!(v.len(), 10);
        for i in 0..10 {
            assert_eq!(v.get(i), 0);
        }
        let bytes = v.to_bytes();
        let r = IntVectorRef::from_bytes(&bytes).expect("parses");
        assert_eq!(r.len(), 10);
        for i in 0..10 {
            assert_eq!(r.get(i), 0);
        }
    }

    #[test]
    fn int_vector_width_64() {
        let mut v = IntVector::with_width(64);
        let values = [0u64, 1, u64::MAX, 0xDEAD_BEEF_CAFE_F00D, 42];
        for &val in &values {
            v.push(val);
        }
        for (i, &val) in values.iter().enumerate() {
            assert_eq!(v.get(i), val);
        }
        let bytes = v.to_bytes();
        let r = IntVectorRef::from_bytes(&bytes).expect("parses");
        for (i, &val) in values.iter().enumerate() {
            assert_eq!(r.get(i), val);
        }
    }

    #[test]
    fn int_vector_ref_rejects_truncated_input() {
        let mut v = IntVector::with_width(20);
        for i in 0..30 {
            v.push(i);
        }
        let bytes = v.to_bytes();
        let err = IntVectorRef::from_bytes(&bytes[..bytes.len() - 1]).unwrap_err();
        assert!(matches!(err, PackBitsError::Truncated { .. }));
    }

    #[test]
    fn int_vector_ref_rejects_bad_width() {
        let mut bad = Vec::new();
        bad.extend_from_slice(&0u64.to_le_bytes()); // len = 0
        bad.extend_from_slice(&65u32.to_le_bytes()); // width = 65 (invalid)
        let err = IntVectorRef::from_bytes(&bad).unwrap_err();
        assert!(matches!(err, PackBitsError::Malformed(_)));
    }

    // -- BitVec / RankSelect ----------------------------------------------------

    fn naive_rank1(bits: &[bool], i: usize) -> usize {
        bits[..i].iter().filter(|&&b| b).count()
    }

    fn naive_select1(bits: &[bool], k: usize) -> Option<usize> {
        bits.iter()
            .enumerate()
            .filter(|&(_, &b)| b)
            .nth(k)
            .map(|(i, _)| i)
    }

    fn naive_select0(bits: &[bool], k: usize) -> Option<usize> {
        bits.iter()
            .enumerate()
            .filter(|&(_, &b)| !b)
            .nth(k)
            .map(|(i, _)| i)
    }

    fn build_rank_select(bits: &[bool]) -> RankSelect {
        let mut b = BitVec::new();
        for &bit in bits {
            b.push(bit);
        }
        b.freeze()
    }

    #[test]
    fn rank_select_empty() {
        let rs = build_rank_select(&[]);
        assert_eq!(rs.len(), 0);
        assert_eq!(rs.total_ones(), 0);
        assert_eq!(rs.rank1(0), 0);
        assert_eq!(rs.select1(0), None);
        assert_eq!(rs.select0(0), None);
    }

    #[test]
    fn rank_select_all_zeros() {
        let bits = vec![false; 200];
        let rs = build_rank_select(&bits);
        assert_eq!(rs.total_ones(), 0);
        assert_eq!(rs.rank1(0), 0);
        assert_eq!(rs.rank1(200), 0);
        assert_eq!(rs.select1(0), None);
        assert_eq!(rs.select0(0), Some(0));
        assert_eq!(rs.select0(199), Some(199));
        assert_eq!(rs.select0(200), None);
    }

    #[test]
    fn rank_select_all_ones() {
        let bits = vec![true; 200];
        let rs = build_rank_select(&bits);
        assert_eq!(rs.total_ones(), 200);
        assert_eq!(rs.rank1(0), 0);
        assert_eq!(rs.rank1(200), 200);
        assert_eq!(rs.select1(0), Some(0));
        assert_eq!(rs.select1(199), Some(199));
        assert_eq!(rs.select1(200), None);
        assert_eq!(rs.select0(0), None);
    }

    #[test]
    fn rank_select_boundaries_and_first_last_set_bit() {
        // A pattern spanning several superblocks (SUPERBLOCK_BITS = 512) with a
        // known first/last set bit and gaps.
        let mut bits = vec![false; 1500];
        bits[0] = true;
        bits[1] = true;
        bits[511] = true;
        bits[512] = true;
        bits[1000] = true;
        bits[1499] = true;
        let rs = build_rank_select(&bits);
        let ones = bits.iter().filter(|&&b| b).count();
        assert_eq!(rs.total_ones(), ones);
        assert_eq!(rs.rank1(0), 0);
        assert_eq!(rs.rank1(bits.len()), ones);
        assert_eq!(rs.select1(0), Some(0));
        assert_eq!(rs.select1(ones - 1), Some(1499));
        assert_eq!(rs.select1(ones), None);
        for i in 0..=bits.len() {
            assert_eq!(rs.rank1(i), naive_rank1(&bits, i), "rank1 mismatch at {i}");
        }
    }

    #[test]
    fn rank_select_serialization_round_trip() {
        let mut bits = vec![false; 777];
        for i in (0..777).step_by(5) {
            bits[i] = true;
        }
        let rs = build_rank_select(&bits);
        let bytes = rs.to_bytes();
        let r = RankSelectRef::from_bytes(&bytes).expect("parses");
        assert_eq!(r.len(), rs.len());
        assert_eq!(r.total_ones(), rs.total_ones());
        for i in 0..=bits.len() {
            assert_eq!(r.rank1(i), rs.rank1(i));
        }
        for k in 0..rs.total_ones() {
            assert_eq!(r.select1(k), rs.select1(k));
        }
    }

    #[test]
    fn rank_select_ref_rejects_truncated_input() {
        let bits = vec![true; 100];
        let rs = build_rank_select(&bits);
        let bytes = rs.to_bytes();
        let err = RankSelectRef::from_bytes(&bytes[..bytes.len() - 1]).unwrap_err();
        assert!(matches!(err, PackBitsError::Truncated { .. }));
    }

    #[test]
    fn bit_vec_set_len_grows_and_truncates() {
        let mut b = BitVec::new();
        b.push(true);
        b.push(true);
        b.set_len(70); // grow past one word boundary; the new tail must read as 0
        assert_eq!(b.len(), 70);
        let rs = b.clone().freeze();
        assert_eq!(rs.rank1(70), 2);

        let mut b2 = b;
        b2.set_len(1); // truncate: the bit at index 1 must be dropped
        let rs2 = b2.freeze();
        assert_eq!(rs2.rank1(1), 1);
        assert_eq!(rs2.total_ones(), 1);
    }

    // -- varint / zigzag / delta -------------------------------------------------

    #[test]
    fn varint_round_trip_edge_values() {
        for &v in &[0u64, 1, 127, 128, 300, u64::from(u32::MAX), u64::MAX] {
            let mut out = Vec::new();
            write_varint(&mut out, v);
            let mut pos = 0;
            let decoded = read_varint(&out, &mut pos).expect("decodes");
            assert_eq!(decoded, v);
            assert_eq!(pos, out.len());
        }
    }

    #[test]
    fn varint_read_past_end_is_truncated() {
        // A single continuation byte with nothing after it.
        let bytes = [0x80u8];
        let mut pos = 0;
        let err = read_varint(&bytes, &mut pos).unwrap_err();
        assert!(matches!(err, PackBitsError::Truncated { .. }));
    }

    #[test]
    fn zigzag_round_trip_edge_values() {
        for &v in &[0i64, -1, 1, -2, 2, i64::MAX, i64::MIN] {
            assert_eq!(zigzag_decode(zigzag_encode(v)), v);
        }
        assert_eq!(zigzag_encode(0), 0);
        assert_eq!(zigzag_encode(-1), 1);
        assert_eq!(zigzag_encode(1), 2);
        assert_eq!(zigzag_encode(i64::MIN), u64::MAX);
    }

    #[test]
    fn delta_list_round_trip() {
        let values: Vec<u64> = vec![0, 0, 3, 3, 10, 1000, 1000, u64::MAX];
        let mut out = Vec::new();
        write_delta_list(&mut out, &values);
        let decoded: Result<Vec<u64>, PackBitsError> =
            DeltaListRef::new(&out, values.len()).collect();
        assert_eq!(decoded.expect("decodes"), values);
    }

    #[test]
    fn delta_list_empty() {
        let out = Vec::new();
        let decoded: Vec<u64> = DeltaListRef::new(&out, 0)
            .collect::<Result<_, _>>()
            .expect("decodes");
        assert!(decoded.is_empty());
    }

    // -- proptest -----------------------------------------------------------------

    proptest! {
        #[test]
        fn proptest_int_vector_round_trips(
            values in prop::collection::vec(any::<u64>(), 0..200)
        ) {
            let max = values.iter().copied().max().unwrap_or(0);
            let width = bits_for(max);
            let mut v = IntVector::with_width(width);
            for &val in &values {
                v.push(val);
            }
            for (i, &val) in values.iter().enumerate() {
                prop_assert_eq!(v.get(i), val);
            }
            let bytes = v.to_bytes();
            let r = IntVectorRef::from_bytes(&bytes).expect("parses");
            prop_assert_eq!(r.len(), values.len());
            prop_assert_eq!(r.width(), width);
            for (i, &val) in values.iter().enumerate() {
                prop_assert_eq!(r.get(i), val);
            }
        }

        #[test]
        fn proptest_rank_select_matches_naive_oracle(
            bits in prop::collection::vec(any::<bool>(), 0..600)
        ) {
            let rs = build_rank_select(&bits);
            for i in 0..=bits.len() {
                prop_assert_eq!(rs.rank1(i), naive_rank1(&bits, i));
                prop_assert_eq!(rs.rank0(i), i - naive_rank1(&bits, i));
            }
            let total_ones = bits.iter().filter(|&&b| b).count();
            let total_zeros = bits.len() - total_ones;
            for k in 0..=total_ones {
                prop_assert_eq!(rs.select1(k), naive_select1(&bits, k));
            }
            for k in 0..=total_zeros {
                prop_assert_eq!(rs.select0(k), naive_select0(&bits, k));
            }

            let bytes = rs.to_bytes();
            let r = RankSelectRef::from_bytes(&bytes).expect("parses");
            for i in 0..=bits.len() {
                prop_assert_eq!(r.rank1(i), naive_rank1(&bits, i));
            }
            for k in 0..=total_ones {
                prop_assert_eq!(r.select1(k), naive_select1(&bits, k));
            }
        }
    }
}
