// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Chunked byte-class scanners over the [`terminals`](crate::terminals) tables.
//!
//! Every scanner here answers one question — where is the first byte of a class
//! in this slice — and answers it the same way:
//!
//! 1. The class is a range table in [`terminals`](crate::terminals) (or, for
//!    [`find_first_xml_special`], a composition of one), projected at compile
//!    time onto a `const [u8; 256]` class table by
//!    the `const fn` `class_table`.
//! 2. The class table is projected again, at compile time, onto its maximal
//!    runs of member bytes (`[lo, hi]` pairs). A byte is a member iff it falls
//!    in a run, and that is tested with one wrapping subtraction and one
//!    unsigned comparison per run — plain comparisons, with no table load.
//! 3. The input is walked in sixteen-byte chunks. Each chunk's sixteen answers
//!    are `0x00`/`0xFF` byte lanes; a branch-free OR of the lanes is the
//!    clean-chunk test, and in the one chunk that holds a hit the lanes, read as
//!    a little-endian `u128` lane mask, give the first hit's offset as their
//!    trailing-zero count over 8. The bytes after the last whole chunk are
//!    answered from the class table.
//!
//! The comparisons of step 2 run over a fixed-size chunk with no data-dependent
//! branch, which is the shape LLVM lowers to packed byte compares and a mask
//! extraction (`pcmpeqb`/`pminub` + `pmovmskb` on x86_64, `cmeq`/`cmhi` +
//! `addp` on aarch64, `i8x16.eq`/`i8x16.lt_u` under wasm `simd128`), and to
//! straight-line scalar code where the target has no vector unit. The
//! result is the same on every target: it is the offset of the first member,
//! and the equivalence tests compare every scanner with the per-byte search it
//! replaces.
//!
//! Each public scanner is its own monomorphic function, so each compiles to one
//! kernel with its class's runs folded in as constants.

use crate::terminals::{
    ScalarRange, TABLE_LIMIT, class_table, in_ranges, iriref_forbidden_ranges,
    json_string_forbidden_ranges, table_index, widen, ws_ranges, xml_char_ranges,
};

/// Bytes per chunk: one 128-bit vector of bytes.
const CHUNK: usize = 16;

/// Bytes per block of the run-time needle searches: four chunks answered by
/// one clean-block test (see [`find_byte`] and [`find_byte_pair`]).
const BLOCK: usize = 4 * CHUNK;

/// An inclusive run `[lo, hi]` of member bytes.
pub type ByteRun = (u8, u8);

/// Narrow a table index to its byte. Every caller passes an index below 256.
#[allow(
    clippy::cast_possible_truncation,
    reason = "every caller passes an index below 256; u8::try_from is not const-callable"
)]
#[inline]
const fn narrow(i: usize) -> u8 {
    i as u8
}

/// How many maximal runs of members `table` holds.
///
/// The array length [`byte_runs`] needs, evaluated at compile time.
#[must_use]
pub const fn count_runs(table: &[u8; 256]) -> usize {
    let mut count = 0;
    let mut i = 0;
    while i < table.len() {
        if table[i] != 0 && (i == 0 || table[i - 1] == 0) {
            count += 1;
        }
        i += 1;
    }
    count
}

/// The maximal runs of members of `table`, in ascending order.
///
/// `N` must be [`count_runs`] of the same table; any other length is a
/// compile-time failure, so the runs are always the whole class and nothing
/// but the class.
#[must_use]
pub const fn byte_runs<const N: usize>(table: &[u8; 256]) -> [ByteRun; N] {
    let mut runs = [(0_u8, 0_u8); N];
    let mut n = 0;
    let mut i = 0;
    while i < table.len() {
        if table[i] != 0 && (i == 0 || table[i - 1] == 0) {
            let mut end = i;
            while end + 1 < table.len() && table[end + 1] != 0 {
                end += 1;
            }
            assert!(n < N, "more runs than the declared run count");
            runs[n] = (narrow(i), narrow(end));
            n += 1;
            i = end;
        }
        i += 1;
    }
    assert!(n == N, "fewer runs than the declared run count");
    runs
}

/// Whether `b` falls in one of `runs`, as comparisons only.
///
/// `b.wrapping_sub(lo) <= hi - lo` is the one-comparison form of
/// `lo <= b && b <= hi`: a byte below `lo` wraps to a value above `hi - lo`.
/// The OR over runs has no early exit, so a caller evaluating it over a whole
/// chunk evaluates it lane by lane with no branch.
#[allow(
    clippy::inline_always,
    reason = "each scanner is one monomorphic kernel: the shared body must be inlined into it \
              so the class's runs fold in as constants and the lane loop vectorizes"
)]
#[inline(always)]
pub fn in_runs(b: u8, runs: &[ByteRun]) -> bool {
    let mut hit = false;
    for &(lo, hi) in runs {
        hit |= b.wrapping_sub(lo) <= hi - lo;
    }
    hit
}

/// The lanes of one chunk: lane `i` is `0xFF` when `chunk[i]`'s membership
/// differs from `flip`'s (`flip = 0x00` marks members, `0xFF` non-members), and
/// `0x00` otherwise.
///
/// A byte per lane rather than a bit per lane, on purpose: the lane loop is a
/// sequence of independent byte compares with no cross-lane dependence, which
/// is the shape the vectorizer turns into packed byte compares on every target
/// that has them. Packing the answers into a `u16` in the same expression
/// (`mask |= u16::from(hit) << i`) was tried first and measured: it vectorized
/// for some classes and not others, and on some targets not at all, because
/// the pack has to be recognized as a whole before any of it pays.
#[allow(
    clippy::inline_always,
    reason = "each scanner is one monomorphic kernel: the shared body must be inlined into it \
              so the class's runs fold in as constants and the lane loop vectorizes"
)]
#[inline(always)]
fn chunk_lanes(chunk: &[u8; CHUNK], runs: &[ByteRun], flip: u8) -> [u8; CHUNK] {
    let mut lanes = [0_u8; CHUNK];
    for (lane, &b) in lanes.iter_mut().zip(chunk) {
        *lane = u8::from(in_runs(b, runs)).wrapping_neg() ^ flip;
    }
    lanes
}

/// The shared body of every scanner: the offset of the first byte whose
/// membership equals `want`, or `None`.
///
/// `want = true` finds the first member; `want = false` finds the first
/// non-member, which is how [`find_first_trivia`] finds the end of a run.
///
/// Per chunk: the lanes are OR-folded without a branch, which is the whole
/// clean-chunk test (a horizontal OR, lowered to a mask extraction). Only a
/// chunk holding a hit reads the lanes as one little-endian `u128` lane mask,
/// whose trailing-zero count over 8 is the first hit's lane.
#[allow(
    clippy::inline_always,
    reason = "each scanner is one monomorphic kernel: the shared body must be inlined into it \
              so the class's runs fold in as constants and the lane loop vectorizes"
)]
#[inline(always)]
fn find_first(bytes: &[u8], runs: &[ByteRun], table: &[u8; 256], want: bool) -> Option<usize> {
    let (chunks, tail) = bytes.as_chunks::<CHUNK>();
    let flip = if want { 0 } else { u8::MAX };
    for (k, chunk) in chunks.iter().enumerate() {
        let lanes = chunk_lanes(chunk, runs, flip);
        if lanes.iter().fold(0, |any, &lane| any | lane) != 0 {
            let first = u128::from_le_bytes(lanes).trailing_zeros() / u8::BITS;
            return Some(k * CHUNK + first as usize);
        }
    }
    tail.iter()
        .position(|&b| (table[usize::from(b)] != 0) == want)
        .map(|i| chunks.len() * CHUNK + i)
}

/// How many maximal runs of member bytes the class table `table` holds: the
/// `RUNS` parameter of the [`ByteClass`] built from it.
///
/// A `const fn`, so the parameter is computed from the table it describes and
/// never written by hand:
///
/// ```rust
/// use purrdf_lex::terminals::{ByteClass, byte_run_count};
///
/// const QUOTES: [u8; 256] = {
///     let mut table = [0_u8; 256];
///     table[b'"' as usize] = 1;
///     table[b'\'' as usize] = 1;
///     table
/// };
/// const CLASS: ByteClass<{ byte_run_count(&QUOTES) }> = ByteClass::from_table(QUOTES);
/// assert_eq!(byte_run_count(&QUOTES), 2);
/// assert_eq!(CLASS.find_first(b"it's"), Some(2));
/// ```
#[must_use]
pub const fn byte_run_count(table: &[u8; 256]) -> usize {
    count_runs(table)
}

/// The class table holding exactly `needles`: the table a caller with a fixed
/// needle set hands [`ByteClass::from_table`].
///
/// ```
/// use purrdf_lex::scan::{ByteClass, byte_run_count, needle_table};
///
/// const ENDS: [u8; 256] = needle_table(b"/?#");
/// const CLASS: ByteClass<{ byte_run_count(&ENDS) }> = ByteClass::from_table(ENDS);
/// assert_eq!(CLASS.find_first(b"host/path"), Some(4));
/// ```
#[must_use]
pub const fn needle_table(needles: &[u8]) -> [u8; 256] {
    let mut table = [0_u8; 256];
    let mut k = 0;
    while k < needles.len() {
        table[needles[k] as usize] = 1;
        k += 1;
    }
    table
}

/// A caller-defined byte class, scanned in the one formulation every scanner of
/// this module uses.
///
/// The four named scanners ([`find_first_trivia`], [`find_first_iri_body_special`],
/// [`find_first_json_string_special`], [`find_first_xml_special`]) answer the
/// classes a *parser* stops at, and those are terminals, spelled here once. A
/// *writer* stops at classes the grammar does not name — the bytes a given
/// egress law escapes, the bytes a CSV field quotes on, the line feed a
/// line-oriented reader splits at — and those belong to the writer that owns
/// the law. `ByteClass` is how such a writer gets the same kernel without
/// retyping it: it supplies its class as a `const [u8; 256]` membership table
/// (entry `b` non-zero when byte `b` is a member), and [`find_first`](Self::find_first)
/// runs the same scan every scanner in this module runs, over the table's
/// runs (derived from the table at compile time): the input walked in
/// sixteen-byte chunks, each byte answered by one wrapping subtraction and
/// one unsigned comparison per run with no data-dependent branch, the
/// sixteen per-chunk answers OR-folded for a branch-free clean-chunk test,
/// and — in the one chunk that holds a hit — the first hit's offset read as
/// the trailing-zero count over 8 of the lanes taken as a little-endian
/// `u128`.
///
/// `RUNS` must be [`byte_run_count`] of the same table; any other value is a
/// compile-time failure when the class is a `const`, so the runs are always the
/// whole class and nothing but the class.
///
/// Declare the class as a `const` and call it from one ordinary function per
/// class, so each class compiles to one kernel with its runs folded in as
/// constants:
///
/// ```rust
/// use purrdf_lex::terminals::{ByteClass, byte_run_count};
///
/// const LINE_FEED: [u8; 256] = {
///     let mut table = [0_u8; 256];
///     table[b'\n' as usize] = 1;
///     table
/// };
/// const LINES: ByteClass<{ byte_run_count(&LINE_FEED) }> = ByteClass::from_table(LINE_FEED);
///
/// fn find_line_feed(bytes: &[u8]) -> Option<usize> {
///     LINES.find_first(bytes)
/// }
///
/// assert_eq!(find_line_feed(b"one\ntwo"), Some(3));
/// assert_eq!(find_line_feed(b"no break here"), None);
/// assert!(LINES.contains(b'\n') && !LINES.contains(b'\r'));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteClass<const RUNS: usize> {
    /// Entry `b` is non-zero when byte `b` is a member: the tail's answer.
    table: [u8; 256],
    /// The maximal runs of members, as inclusive `[lo, hi]` pairs: the lanes'
    /// answer, as comparisons.
    runs: [ByteRun; RUNS],
}

impl<const RUNS: usize> ByteClass<RUNS> {
    /// The class whose members are the bytes `b` with `table[b] != 0`.
    ///
    /// # Panics
    ///
    /// When `RUNS` is not the [`byte_run_count`] of `table`. In a `const` item that
    /// is a compile-time error rather than a run-time panic.
    #[must_use]
    pub const fn from_table(table: [u8; 256]) -> Self {
        let runs = byte_runs::<RUNS>(&table);
        Self { table, runs }
    }

    /// Whether `b` is a member.
    #[must_use]
    pub const fn contains(&self, b: u8) -> bool {
        self.table[table_index(widen(b))] != 0
    }

    /// The offset of the first member byte of `bytes`, or `None`.
    ///
    /// The same kernel as the named scanners: sixteen-byte chunks answered as
    /// `0x00`/`0xFF` byte lanes by comparisons against the class's runs, a
    /// branch-free OR of the lanes as the clean-chunk test, the hit chunk's
    /// lanes read as a little-endian `u128` whose trailing-zero count over 8 is
    /// the offset, and the tail answered from the table. A class of one run
    /// tests the lanes' maximum instead and re-tests the hit chunk's bytes for
    /// the offset, the form that vectorizes for a single comparison (see
    /// `find_first_in_one_run`). Inlined into its caller, so the caller's
    /// function is the one kernel for its class.
    #[allow(
        clippy::inline_always,
        reason = "a class's kernel is its caller: the scan must be inlined so the class's runs \
                  fold in as constants and the lane loop vectorizes"
    )]
    #[inline(always)]
    #[must_use]
    pub fn find_first(&self, bytes: &[u8]) -> Option<usize> {
        if RUNS == 1 {
            find_first_in_one_run(bytes, &self.runs, &self.table)
        } else {
            find_first(bytes, &self.runs, &self.table, true)
        }
    }
}

/// [`find_first`] for a class of one run, whose clean-chunk test is a
/// maximum rather than an OR.
///
/// The two folds give the same answer — a lane is `0x00` or `0xFF`, so the
/// maximum is non-zero exactly when some lane is — but not the same code. With
/// one run the lane is a single comparison, the OR of sixteen of them is folded
/// to an OR over the comparisons' one-bit results before vectorization, and on
/// aarch64 that reduction was measured to vectorize eight lanes of sixteen and
/// test the rest one byte at a time. The maximum is not folded that way: it
/// lowers to one `cmeq .16b` and an `addp` fold on aarch64, to one `pcmpeqb`
/// and a `pmovmskb` on x86_64 (the pair the OR-fold gives there), and to
/// `i8x16.eq` and `v128.any_true` under wasm `simd128`. The chunk that holds a
/// hit finds its offset by testing its bytes again rather than by reading the
/// lanes as a `u128`: keeping the lanes alive for that read was measured to
/// spill them and scalarize the clean-chunk test on both x86_64 and aarch64. A
/// class of more than one run keeps the OR-fold, which is what the named
/// scanners were measured with.
#[allow(
    clippy::inline_always,
    reason = "a class's kernel is its caller: the scan must be inlined so the class's run \
              folds in as constants and the lane loop vectorizes"
)]
#[inline(always)]
fn find_first_in_one_run(bytes: &[u8], runs: &[ByteRun], table: &[u8; 256]) -> Option<usize> {
    let (chunks, tail) = bytes.as_chunks::<CHUNK>();
    for (k, chunk) in chunks.iter().enumerate() {
        let lanes = chunk_lanes(chunk, runs, 0);
        if lanes.iter().fold(0, |most, &lane| most.max(lane)) != 0 {
            return chunk
                .iter()
                .position(|&b| in_runs(b, runs))
                .map(|first| k * CHUNK + first);
        }
    }
    tail.iter()
        .position(|&b| table[usize::from(b)] != 0)
        .map(|i| chunks.len() * CHUNK + i)
}

/// The offset of the first byte of `bytes` equal to one of `needles`, or
/// `None`: the body of [`find_byte`] and [`find_byte2`].
///
/// The needles are known only at run time, so a chunk's lanes are equality
/// compares against each needle rather than run compares folded in as
/// constants; otherwise the formulation is [`find_first_in_one_run`]'s. The
/// clean-chunk test is the lanes' maximum, and the chunk that holds a hit
/// re-tests its bytes for the offset, the shape that lowers to a packed
/// compare and one mask extraction per chunk on every vector target.
///
/// A long input is first walked in [`BLOCK`]-byte blocks, four chunks whose
/// lanes fold into one maximum: one mask extraction and one branch per
/// sixty-four bytes rather than per sixteen, which is where a sixteen-byte loop
/// spends its time once the compares are packed. The block's lanes are the
/// chunk's lanes widened, so the vectorizer emits four packed compares on a
/// 128-bit target and two on a 256-bit one; the block that holds a hit and the
/// bytes after the last whole block take the sixteen-byte path.
#[allow(
    clippy::inline_always,
    reason = "each public search is one monomorphic kernel: the shared body must be inlined \
              into it so the needle count folds in and the lane loop vectorizes"
)]
#[inline(always)]
fn find_needles<const N: usize>(bytes: &[u8], needles: [u8; N]) -> Option<usize> {
    let (chunks, _) = bytes.as_chunks::<CHUNK>();
    let (blocks, _) = chunks.as_chunks::<{ BLOCK / CHUNK }>();
    let mut skipped = 0;
    for [a, b, c, d] in blocks {
        let lanes = merge_lanes([
            needle_lanes(a, needles),
            needle_lanes(b, needles),
            needle_lanes(c, needles),
            needle_lanes(d, needles),
        ]);
        if any_lane(&lanes) {
            break;
        }
        skipped += BLOCK;
    }
    find_needles_in_chunks(&bytes[skipped..], needles).map(|i| skipped + i)
}

/// The lanes of one chunk against run-time needles: lane `i` is `0xFF` when
/// `chunk[i]` equals one of `needles`, `0x00` otherwise.
#[allow(
    clippy::inline_always,
    reason = "each public search is one monomorphic kernel: the shared body must be inlined \
              into it so the needle count folds in and the lane loop vectorizes"
)]
#[inline(always)]
fn needle_lanes<const N: usize>(chunk: &[u8; CHUNK], needles: [u8; N]) -> [u8; CHUNK] {
    let mut lanes = [0_u8; CHUNK];
    for (lane, &b) in lanes.iter_mut().zip(chunk) {
        let mut hit = 0;
        for needle in needles {
            hit |= u8::from(b == needle);
        }
        *lane = hit.wrapping_neg();
    }
    lanes
}

/// The lane-wise OR of a block's four chunks of lanes: non-zero somewhere
/// exactly when one of them is.
#[allow(
    clippy::inline_always,
    reason = "each public search is one monomorphic kernel: the shared body must be inlined \
              into it so the lane loop vectorizes"
)]
#[inline(always)]
fn merge_lanes(parts: [[u8; CHUNK]; BLOCK / CHUNK]) -> [u8; CHUNK] {
    let mut lanes = [0_u8; CHUNK];
    for part in parts {
        for (lane, bit) in lanes.iter_mut().zip(part) {
            *lane |= bit;
        }
    }
    lanes
}

/// Whether any lane of a chunk is set, read as one 128-bit word.
///
/// A block's lanes are the OR of four chunks', and a horizontal maximum over
/// them was measured to lower, once LLVM fuses the four compares into one wide
/// compare, to a long extract-and-shuffle reduction per block: a pair search
/// over 70-byte inputs took 60 µs per 4,096 inputs with the maximum and 39 µs
/// with the word test, which is a register-width OR and a test on every target.
#[allow(
    clippy::inline_always,
    reason = "each public search is one monomorphic kernel: the shared body must be inlined \
              into it so the lane loop vectorizes"
)]
#[inline(always)]
fn any_lane(lanes: &[u8; CHUNK]) -> bool {
    u128::from_ne_bytes(*lanes) != 0
}

/// The sixteen-byte walk behind [`find_needles`]: the chunk formulation of
/// [`find_first_in_one_run`] over run-time needles.
#[allow(
    clippy::inline_always,
    reason = "each public search is one monomorphic kernel: the shared body must be inlined \
              into it so the needle count folds in and the lane loop vectorizes"
)]
#[inline(always)]
fn find_needles_in_chunks<const N: usize>(bytes: &[u8], needles: [u8; N]) -> Option<usize> {
    let (chunks, tail) = bytes.as_chunks::<CHUNK>();
    for (k, chunk) in chunks.iter().enumerate() {
        let mut lanes = [0_u8; CHUNK];
        for (lane, &b) in lanes.iter_mut().zip(chunk) {
            let mut hit = false;
            for needle in needles {
                hit |= b == needle;
            }
            *lane = u8::from(hit).wrapping_neg();
        }
        if lanes.iter().fold(0, |most, &lane| most.max(lane)) != 0 {
            return chunk
                .iter()
                .position(|b| needles.contains(b))
                .map(|first| k * CHUNK + first);
        }
    }
    tail.iter()
        .position(|b| needles.contains(b))
        .map(|i| chunks.len() * CHUNK + i)
}

/// The offset of the first byte of `bytes` equal to `needle`, or `None`.
///
/// The workspace's one search for a byte known only at run time — a quote
/// that is `"` or `'` by context, a line terminator a caller chooses. A
/// needle fixed at compile time is a class: declare it as a [`ByteClass`],
/// whose runs fold in as constants.
///
/// Sixteen-byte chunks, each answered by equality compares into `0x00`/`0xFF`
/// lanes whose maximum is the clean-chunk test, then the hit chunk re-tested
/// for the offset, and the tail byte by byte: the shape that lowers to
/// `pcmpeqb`/`pmovmskb` on x86_64, `cmeq` and a pairwise fold on aarch64 and
/// `i8x16.eq` under wasm `simd128`. It is the formulation of every other
/// scanner here, so a sixteen-byte chunk is one packed compare where a
/// word-at-a-time (SWAR) scan does eight bytes of scalar arithmetic, and it
/// has no run-time dispatch, so a short input pays no selection cost.
///
/// # Examples
///
/// ```rust
/// use purrdf_lex::scan::find_byte;
///
/// assert_eq!(find_byte(b"key=value", b'='), Some(3));
/// assert_eq!(find_byte(b"no separator", b'='), None);
/// ```
#[must_use]
pub fn find_byte(bytes: &[u8], needle: u8) -> Option<usize> {
    find_needles(bytes, [needle])
}

/// The offset of the first byte of `bytes` equal to `first` or `second`, or
/// `None`: [`find_byte`] for two needles known only at run time.
///
/// # Examples
///
/// ```rust
/// use purrdf_lex::scan::find_byte2;
///
/// assert_eq!(find_byte2(b"line one\r\nline two", b'\r', b'\n'), Some(8));
/// assert_eq!(find_byte2(b"'quoted' \\", b'"', b'\\'), Some(9));
/// assert_eq!(find_byte2(b"clean", b'"', b'\\'), None);
/// ```
#[must_use]
pub fn find_byte2(bytes: &[u8], first: u8, second: u8) -> Option<usize> {
    find_needles(bytes, [first, second])
}

/// The first offset `p` at which `bytes[p]` is one of `lead` and
/// `bytes[p + gap]` is one of `trail`, or `None`: the candidate search of a
/// substring filter, which tests two of a needle's positions at once.
///
/// Each pair names one or two bytes; a single byte is written twice. A caller
/// looking for a substring picks two of its positions (the first and last, or
/// its two rarest bytes), finds the offsets where both agree, and verifies the
/// rest there. Testing two positions together is what keeps a common byte from
/// stopping the scan at every occurrence: `e` alone is every tenth byte of
/// English text, `e` followed four bytes later by `l` is far rarer.
///
/// The formulation is [`find_byte`]'s over two views of the input offset by
/// `gap`: sixty-four-byte blocks whose lanes are the AND of the two positions'
/// equality compares fold into one clean-block test, the block holding a hit
/// is re-walked in sixteen-byte chunks, and the positions after the last whole
/// chunk are one more chunk ending at the last position. The lanes of the two
/// views are independent, so the vectorizer lowers each block to packed
/// compares, an AND and one mask test on every vector target
/// (`pcmpeqb`/`pand`/`pmovmskb` on x86_64, `vpcmpeqb` on 256-bit builds,
/// `cmeq`/`and` on aarch64, `i8x16.eq`/`v128.and` under wasm `simd128`), with
/// no run-time dispatch. A pair written as one byte twice takes a kernel that
/// compares it once.
///
/// # Examples
///
/// ```rust
/// use purrdf_lex::scan::find_byte_pair;
///
/// // "needle": `n` and, five bytes on, `e`.
/// let text = b"a nice needle in a haystack";
/// assert_eq!(find_byte_pair(text, [b'n', b'n'], [b'e', b'e'], 5), Some(7));
/// // Either case of each position, as a case-insensitive filter asks.
/// assert_eq!(find_byte_pair(b"A NEEDLE", [b'n', b'N'], [b'e', b'E'], 5), Some(2));
/// assert_eq!(find_byte_pair(b"needl", [b'n', b'n'], [b'e', b'e'], 5), None);
/// ```
#[must_use]
pub fn find_byte_pair(bytes: &[u8], lead: [u8; 2], trail: [u8; 2], gap: usize) -> Option<usize> {
    let ([a, b], [c, d]) = (lead, trail);
    match (a == b, c == d) {
        (true, true) => find_pair_by(bytes, gap, Byte(a), Byte(c)),
        (true, false) => find_pair_by(bytes, gap, Byte(a), Bytes2(c, d)),
        (false, true) => find_pair_by(bytes, gap, Bytes2(a, b), Byte(c)),
        (false, false) => find_pair_by(bytes, gap, Bytes2(a, b), Bytes2(c, d)),
    }
}

/// [`find_byte_pair`] over byte ranges: the first offset `p` at which
/// `bytes[p]` falls in one of the two inclusive runs of `lead` and
/// `bytes[p + gap]` in one of the two runs of `trail`, or `None`.
///
/// This is the search for a position that is a class rather than one or two
/// bytes (`[0-9]`, `[A-Fa-f]`); a single run is written twice, and takes a
/// kernel that tests it once. Each lane is a wrapping subtraction and an
/// unsigned compare per run, the membership test of [`ByteClass`] with the
/// runs known only at run time, in the same blocks, chunks and final chunk as
/// [`find_byte_pair`].
///
/// # Examples
///
/// ```rust
/// use purrdf_lex::scan::find_range_pair;
///
/// // A digit, a letter, a digit: the digits two apart.
/// let digit = [(b'0', b'9'); 2];
/// assert_eq!(find_range_pair(b"see x1a2 here", digit, digit, 2), Some(5));
/// assert_eq!(find_range_pair(b"no digits here", digit, digit, 2), None);
/// // Two runs per position: either case of a hex letter.
/// let hex = [(b'a', b'f'), (b'A', b'F')];
/// assert_eq!(find_range_pair(b"xyz Cab", hex, hex, 1), Some(4));
/// ```
#[must_use]
pub fn find_range_pair(
    bytes: &[u8],
    lead: [ByteRun; 2],
    trail: [ByteRun; 2],
    gap: usize,
) -> Option<usize> {
    let run = |(lo, hi): ByteRun| Run(lo, hi.wrapping_sub(lo));
    let ([a, b], [c, d]) = (lead, trail);
    match (a == b, c == d) {
        (true, true) => find_pair_by(bytes, gap, run(a), run(c)),
        (true, false) => find_pair_by(bytes, gap, run(a), Runs2(run(c), run(d))),
        (false, true) => find_pair_by(bytes, gap, Runs2(run(a), run(b)), run(c)),
        (false, false) => find_pair_by(bytes, gap, Runs2(run(a), run(b)), Runs2(run(c), run(d))),
    }
}

/// One position test of the pair searches: `1` when a byte belongs, `0` when
/// it does not. Byte arithmetic rather than a `bool`, so a lane that combines
/// tests computes all of them: the branch-free shape.
trait PositionTest: Copy {
    fn test(self, b: u8) -> u8;
}

/// One byte.
#[derive(Clone, Copy)]
struct Byte(u8);

/// Either of two bytes.
#[derive(Clone, Copy)]
struct Bytes2(u8, u8);

/// An inclusive run as its low byte and its width minus one.
#[derive(Clone, Copy)]
struct Run(u8, u8);

/// Either of two runs.
#[derive(Clone, Copy)]
struct Runs2(Run, Run);

impl PositionTest for Byte {
    #[allow(
        clippy::inline_always,
        reason = "a lane test must fold into its kernel's lane loop"
    )]
    #[inline(always)]
    fn test(self, b: u8) -> u8 {
        u8::from(b == self.0)
    }
}

impl PositionTest for Bytes2 {
    #[allow(
        clippy::inline_always,
        reason = "a lane test must fold into its kernel's lane loop"
    )]
    #[inline(always)]
    fn test(self, b: u8) -> u8 {
        u8::from(b == self.0) | u8::from(b == self.1)
    }
}

impl PositionTest for Run {
    #[allow(
        clippy::inline_always,
        reason = "a lane test must fold into its kernel's lane loop"
    )]
    #[inline(always)]
    fn test(self, b: u8) -> u8 {
        u8::from(b.wrapping_sub(self.0) <= self.1)
    }
}

impl PositionTest for Runs2 {
    #[allow(
        clippy::inline_always,
        reason = "a lane test must fold into its kernel's lane loop"
    )]
    #[inline(always)]
    fn test(self, b: u8) -> u8 {
        self.0.test(b) | self.1.test(b)
    }
}

/// The lanes of one chunk of a pair search: lane `i` is `0xFF` when `leads[i]`
/// passes `lead` and `trails[i]` passes `trail`.
#[allow(
    clippy::inline_always,
    reason = "each public pair search is one kernel: the lane loop must be inlined into it to \
              vectorize"
)]
#[inline(always)]
fn test_lanes<L: PositionTest, T: PositionTest>(
    leads: &[u8; CHUNK],
    trails: &[u8; CHUNK],
    lead: L,
    trail: T,
) -> [u8; CHUNK] {
    let mut lanes = [0_u8; CHUNK];
    for ((lane, &x), &y) in lanes.iter_mut().zip(leads).zip(trails) {
        *lane = (lead.test(x) & trail.test(y)).wrapping_neg();
    }
    lanes
}

/// The body of [`find_byte_pair`] and [`find_range_pair`]: the first offset
/// `p` at which `bytes[p]` passes `lead` and `bytes[p + gap]` passes `trail`.
#[allow(
    clippy::inline_always,
    reason = "each public pair search is one kernel: the body must be inlined into it so its \
              position tests fold in and the lane loops vectorize"
)]
#[inline(always)]
fn find_pair_by<L: PositionTest, T: PositionTest>(
    bytes: &[u8],
    gap: usize,
    lead: L,
    trail: T,
) -> Option<usize> {
    // `p` ranges over `0..positions`, so that `p + gap` stays in bounds; the
    // two views have that length, so they split into the same chunks.
    let positions = bytes.len().checked_sub(gap)?;
    let (leads, _) = bytes[..positions].as_chunks::<CHUNK>();
    let (trails, _) = bytes[gap..].as_chunks::<CHUNK>();
    let (lead_blocks, _) = leads.as_chunks::<{ BLOCK / CHUNK }>();
    let (trail_blocks, _) = trails.as_chunks::<{ BLOCK / CHUNK }>();
    let lanes_of = |l: &[u8; CHUNK], t: &[u8; CHUNK]| test_lanes(l, t, lead, trail);
    let mut clean_chunks = 0;
    for (l, t) in lead_blocks.iter().zip(trail_blocks) {
        let lanes = merge_lanes([
            lanes_of(&l[0], &t[0]),
            lanes_of(&l[1], &t[1]),
            lanes_of(&l[2], &t[2]),
            lanes_of(&l[3], &t[3]),
        ]);
        if any_lane(&lanes) {
            break;
        }
        clean_chunks += BLOCK / CHUNK;
    }
    // The hit chunk's offset is re-read byte by byte, once per search: reading
    // the lanes as one word for it was measured to spill them into a long
    // widening sequence on every chunk.
    let first_hit = |lanes: &[u8; CHUNK]| lanes.iter().position(|&lane| lane != 0);
    for k in clean_chunks..leads.len() {
        let lanes = lanes_of(&leads[k], &trails[k]);
        if any_lane(&lanes) {
            return first_hit(&lanes).map(|i| k * CHUNK + i);
        }
    }
    let done = leads.len() * CHUNK;
    if done == positions {
        return None;
    }
    if positions >= CHUNK {
        // The positions after the last whole chunk, answered by one more
        // chunk ending at the last position: the positions it shares with the
        // chunk before it are clean, so its first hit is the first hit.
        let last = positions - CHUNK;
        let lanes = lanes_of(
            bytes[last..positions]
                .try_into()
                .expect("a chunk of positions"),
            bytes[last + gap..positions + gap]
                .try_into()
                .expect("a chunk of positions"),
        );
        return first_hit(&lanes).map(|i| last + i);
    }
    (done..positions).find(|&p| lead.test(bytes[p]) & trail.test(bytes[p + gap]) != 0)
}

/// The `WS` class table, projected from [`ws_ranges`].
const TRIVIA_TABLE: [u8; 256] = class_table(ws_ranges());
const TRIVIA_RUNS: [ByteRun; count_runs(&TRIVIA_TABLE)] = byte_runs(&TRIVIA_TABLE);

/// Find the end of a run of `WS` trivia: the offset of the first byte of
/// `bytes` that is NOT `WS ::= #x20 | #x9 | #xD | #xA`, or `None` when every
/// byte is.
///
/// This is the scan a Turtle/SPARQL tokenizer runs between every pair of
/// tokens. The class is [`is_ws`](crate::terminals::is_ws) exactly — not
/// [`u8::is_ascii_whitespace`], which admits FORM FEED, and not
/// [`char::is_whitespace`], which admits NO-BREAK SPACE — so a run ends at a
/// U+00A0 lead byte, which is where the grammar says it ends. A `#` comment is
/// not `WS` either: the run ends at it, and skipping the comment is the
/// caller's decision.
///
/// Every `WS` member is ASCII, so the offset returned is always a UTF-8 char
/// boundary of a `str`'s bytes.
///
/// # Examples
///
/// ```rust
/// use purrdf_lex::terminals::find_first_trivia;
///
/// assert_eq!(find_first_trivia(b" \t\r\n?s"), Some(4));
/// assert_eq!(find_first_trivia(b"?s"), Some(0));
/// assert_eq!(find_first_trivia(b"   "), None);
/// // NO-BREAK SPACE is not `WS`: the run ends at its lead byte.
/// assert_eq!(find_first_trivia(" \u{a0}".as_bytes()), Some(1));
/// ```
#[must_use]
pub fn find_first_trivia(bytes: &[u8]) -> Option<usize> {
    find_first(bytes, &TRIVIA_RUNS, &TRIVIA_TABLE, false)
}

/// The `IRIREF` forbidden-byte class table, projected from
/// [`iriref_forbidden_ranges`].
const IRI_BODY_TABLE: [u8; 256] = class_table(iriref_forbidden_ranges());
const IRI_BODY_RUNS: [ByteRun; count_runs(&IRI_BODY_TABLE)] = byte_runs(&IRI_BODY_TABLE);

/// Find the first byte of an `IRIREF` body that a single pass must stop at:
/// the offset of the first byte in
/// [`is_iriref_forbidden_byte`](crate::terminals::is_iriref_forbidden_byte),
/// or `None`.
///
/// ```text
/// IRIREF ::= '<' ( [^#x00-#x20<>"{}|^`\] | UCHAR )* '>'
/// ```
///
/// The forbidden set already holds every byte such a pass has to act on, so
/// one scan classifies the whole body at once: the closing `>`, the `\` that
/// opens a `UCHAR`, and every byte the production refuses raw (`#x00-#x20` and
/// `< " { } | ^` and `` ` ``). The caller tells them apart by the byte at the
/// returned offset. Every byte before it is lawful raw content. Every member
/// is ASCII, so the offset is always a UTF-8 char boundary of a `str`'s bytes.
///
/// # Examples
///
/// ```rust
/// use purrdf_lex::terminals::find_first_iri_body_special;
///
/// assert_eq!(find_first_iri_body_special(b"urn:ex:a>"), Some(8)); // the close
/// assert_eq!(find_first_iri_body_special(b"urn:\\u0041>"), Some(4)); // a UCHAR
/// assert_eq!(find_first_iri_body_special(b"urn:ex a>"), Some(6)); // refused raw
/// assert_eq!(find_first_iri_body_special(b"urn:ex:a"), None); // unterminated
/// ```
#[must_use]
pub fn find_first_iri_body_special(bytes: &[u8]) -> Option<usize> {
    find_first(bytes, &IRI_BODY_RUNS, &IRI_BODY_TABLE, true)
}

/// The JSON string-body class table, projected from
/// [`json_string_forbidden_ranges`].
const JSON_STRING_TABLE: [u8; 256] = class_table(json_string_forbidden_ranges());
const JSON_STRING_RUNS: [ByteRun; count_runs(&JSON_STRING_TABLE)] = byte_runs(&JSON_STRING_TABLE);

/// Find the end of a clean run of a JSON string body: the offset of the first
/// byte that is `"`, `\` or below `0x20`, or `None`.
///
/// The class is
/// [`is_json_string_forbidden_byte`](crate::terminals::is_json_string_forbidden_byte),
/// RFC 8259 §7's complement of `unescaped` over ASCII: the quotation mark that
/// ends the string, the reverse solidus that opens an escape, and the C0
/// controls that may not appear raw. Every byte before the offset is
/// `unescaped` content and may be copied whole. DELETE and the C1 controls are
/// `unescaped`, so they do not stop the run. Every member is ASCII, so the
/// offset is always a UTF-8 char boundary of a `str`'s bytes.
///
/// # Examples
///
/// ```rust
/// use purrdf_lex::terminals::find_first_json_string_special;
///
/// assert_eq!(find_first_json_string_special(b"plain text\" tail"), Some(10));
/// assert_eq!(find_first_json_string_special(b"a\\nb"), Some(1));
/// assert_eq!(find_first_json_string_special(b"tab\there"), Some(3));
/// // DELETE and non-ASCII content are `unescaped`.
/// assert_eq!(find_first_json_string_special("\u{7f}caf\u{e9}".as_bytes()), None);
/// ```
#[must_use]
pub fn find_first_json_string_special(bytes: &[u8]) -> Option<usize> {
    find_first(bytes, &JSON_STRING_RUNS, &JSON_STRING_TABLE, true)
}

/// The ASCII scalars the XML 1.0 egress law writes as a reference in at least
/// one context: `&`, `<` and `>` everywhere; carriage return everywhere,
/// because §2.11 would normalize it away; and, inside a double-quoted
/// attribute, `"`, TAB and LINE FEED, because §3.3.3 would normalize the two
/// whitespace scalars to SPACE. This is the replacement set of the workspace's
/// XML escaper (`purrdf_core::xml_escape`), which the SPARQL results XML writer
/// delegates to, written as ranges.
const XML_REFERENCED_RANGES: &[ScalarRange] = &[
    (0x09, 0x0A), // TAB, LINE FEED (attribute context)
    (0x0D, 0x0D), // CARRIAGE RETURN
    (0x22, 0x22), // '"' (attribute context)
    (0x26, 0x26), // '&'
    (0x3C, 0x3C), // '<'
    (0x3E, 0x3E), // '>'
];

/// The first scalar of a UTF-8 surrogate block, which no `str` can hold.
const SURROGATE_LO: u32 = 0xD800;
/// The last scalar of the surrogate block.
const SURROGATE_HI: u32 = 0xDFFF;
/// The largest Unicode scalar value.
const SCALAR_MAX: u32 = 0x0010_FFFF;

/// The UTF-8 lead byte of the encoding of scalar `cp`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "each arm shifts cp into a single byte first; u8::try_from is not const-callable"
)]
const fn utf8_lead(cp: u32) -> u8 {
    if cp < 0x80 {
        cp as u8
    } else if cp < 0x800 {
        0xC0 | (cp >> 6) as u8
    } else if cp < 0x1_0000 {
        0xE0 | (cp >> 12) as u8
    } else {
        0xF0 | (cp >> 18) as u8
    }
}

/// Mark in `table` the lead byte of every scalar in `[lo, hi]` that a `str`
/// can hold (the surrogates are skipped: no `str` encodes one). The lead byte
/// is nondecreasing in the scalar, so the leads of a range are the run from
/// the lead of its first scalar to the lead of its last.
const fn mark_leads(table: &mut [u8; 256], lo: u32, hi: u32) {
    let pieces = [
        (
            lo,
            if hi < SURROGATE_LO {
                hi
            } else {
                SURROGATE_LO - 1
            },
        ),
        (
            if lo > SURROGATE_HI {
                lo
            } else {
                SURROGATE_HI + 1
            },
            hi,
        ),
    ];
    let mut p = 0;
    while p < pieces.len() {
        let (a, b) = pieces[p];
        if a <= b {
            let mut lead = utf8_lead(a);
            let last = utf8_lead(b);
            loop {
                table[table_index(widen(lead))] = 1;
                if lead == last {
                    break;
                }
                lead += 1;
            }
        }
        p += 1;
    }
}

/// The class [`find_first_xml_special`] stops at, derived from the XML `Char`
/// range table and the referenced set.
///
/// * every ASCII scalar outside `Char` (the C0 controls other than TAB, LINE
///   FEED and CARRIAGE RETURN);
/// * every scalar in [`XML_REFERENCED_RANGES`];
/// * the UTF-8 lead byte of every non-ASCII scalar outside `Char` that a `str`
///   can hold. The `Char` table's non-ASCII gaps are the surrogates, which no
///   `str` holds, and U+FFFE-U+FFFF, so this is exactly the lead byte `0xEF`
///   of the three-byte block U+F000-U+FFFF.
const fn xml_special_table() -> [u8; 256] {
    let mut table = [0_u8; 256];
    let mut cp = 0_u32;
    while cp < 0x80 {
        if !in_ranges(cp, xml_char_ranges()) || in_ranges(cp, XML_REFERENCED_RANGES) {
            table[table_index(cp)] = 1;
        }
        cp += 1;
    }
    let ranges = xml_char_ranges();
    let mut next = 0x80_u32;
    let mut r = 0;
    while r < ranges.len() {
        let (lo, hi) = ranges[r];
        if hi >= 0x80 {
            let lo = if lo < 0x80 { 0x80 } else { lo };
            if lo > next {
                mark_leads(&mut table, next, lo - 1);
            }
            next = hi + 1;
        }
        r += 1;
    }
    if next <= SCALAR_MAX {
        mark_leads(&mut table, next, SCALAR_MAX);
    }
    table
}

/// The XML egress class table.
const XML_TABLE: [u8; 256] = xml_special_table();
const XML_RUNS: [ByteRun; count_runs(&XML_TABLE)] = byte_runs(&XML_TABLE);

// Every scalar below `TABLE_LIMIT` that the class admits is ASCII or a lead
// byte, never a continuation byte: a scanner stopping inside a sequence would
// hand its caller an offset that is not a char boundary.
const _: () = {
    let mut b = 0x80;
    while b < 0xC0 {
        assert!(
            XML_TABLE[b] == 0,
            "the XML class must not stop at a continuation byte"
        );
        b += 1;
    }
    assert!(TABLE_LIMIT == 0x100, "class tables have one entry per byte");
};

/// Find the first byte of an XML value that XML 1.0 egress must act on: the
/// offset of the first byte in the class below, or `None` when the whole value
/// may be copied verbatim in either context.
///
/// The class is the union of what an XML escaper must *replace* and what it
/// must *check*, over both contexts it writes (character data and a
/// double-quoted attribute):
///
/// * **replace** — `&`, `<`, `>` and CARRIAGE RETURN in both contexts; `"`,
///   TAB and LINE FEED in an attribute (XML 1.0 §2.4, §2.11, §3.3.3). This is
///   the replacement set of `purrdf_core::xml_escape`, the escaper the SPARQL
///   results XML writer delegates to. A character-data caller that reaches a
///   `"`, TAB or LINE FEED copies it and scans on.
/// * **check** — every byte that can begin a scalar outside XML 1.0 §2.2 `[2]`
///   `Char`
///   ([`is_xml_char`](crate::terminals::is_xml_char)): the C0 controls other
///   than TAB, LINE FEED and CARRIAGE RETURN, which are refused outright, and
///   the lead byte `0xEF`, whose three-byte block U+F000-U+FFFF holds the two
///   non-ASCII scalars a `str` can carry that `Char` excludes, U+FFFE and
///   U+FFFF. At an `0xEF` the caller decodes the scalar and tests it; the rest
///   of that block is lawful content.
///
/// Every other byte is lawful content that needs no reference in either
/// context, including `'`, DELETE, the C1 controls and every other non-ASCII
/// scalar. The offset is always a UTF-8 char boundary of a `str`'s bytes: the
/// class holds ASCII bytes and one lead byte, never a continuation byte.
///
/// # Examples
///
/// ```rust
/// use purrdf_lex::terminals::find_first_xml_special;
///
/// assert_eq!(find_first_xml_special(b"a < b"), Some(2));
/// assert_eq!(find_first_xml_special(b"Tom's"), None);
/// assert_eq!(find_first_xml_special(b"bell\x07"), Some(4)); // not a Char
/// // U+FFFF is not a Char, and its lead byte begins the block that holds it.
/// assert_eq!(find_first_xml_special("ok\u{ffff}".as_bytes()), Some(2));
/// assert_eq!(find_first_xml_special("caf\u{e9}".as_bytes()), None);
/// ```
#[must_use]
pub fn find_first_xml_special(bytes: &[u8]) -> Option<usize> {
    find_first(bytes, &XML_RUNS, &XML_TABLE, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminals::{
        is_iriref_forbidden_byte, is_json_string_forbidden_byte, is_ws, is_xml_char,
    };
    use purrdf_testkit::rng::SplitMix64;

    /// The per-byte search each scanner replaces: the first byte whose
    /// range-table membership (by the binary search the predicates ran before
    /// they had class tables) equals `want`.
    fn reference(bytes: &[u8], ranges: &[ScalarRange], want: bool) -> Option<usize> {
        bytes
            .iter()
            .position(|&b| in_ranges(widen(b), ranges) == want)
    }

    /// The XML reference, per scalar rather than per byte: the first scalar an
    /// escaper must replace in either context or that is not a `Char`, or — at
    /// an `0xEF` lead — any scalar of the U+F000-U+FFFF block, which is where
    /// the byte class stops so the caller can decide.
    fn xml_reference(s: &str) -> Option<usize> {
        s.char_indices()
            .find(|&(_, c)| {
                !is_xml_char(c)
                    || matches!(c, '&' | '<' | '>' | '\r' | '"' | '\t' | '\n')
                    || ('\u{F000}'..='\u{FFFF}').contains(&c)
            })
            .map(|(i, _)| i)
    }

    /// The bytes that sit on either side of every run boundary of every class,
    /// plus the bytes that begin and continue multi-byte sequences.
    fn boundary_bytes() -> Vec<u8> {
        let mut out = Vec::new();
        for runs in [
            &TRIVIA_RUNS[..],
            &IRI_BODY_RUNS[..],
            &JSON_STRING_RUNS[..],
            &XML_RUNS[..],
        ] {
            for &(lo, hi) in runs {
                out.extend([lo.wrapping_sub(1), lo, hi, hi.wrapping_add(1)]);
            }
        }
        out.extend([
            0x00, 0x7F, 0x80, 0xBF, 0xC0, 0xC2, 0xE0, 0xED, 0xEE, 0xEF, 0xF0, 0xF4, 0xFF,
        ]);
        out.extend(b"aZ09#<>\\\"{}|^`&'%:/?@.-_~ ");
        out
    }

    /// Scalars around every non-ASCII class decision, each encoded as UTF-8.
    const SCALARS: &[char] = &[
        'a',
        ' ',
        '\t',
        '\n',
        '\r',
        '#',
        '<',
        '>',
        '\\',
        '"',
        '&',
        '\'',
        '\u{7}',
        '\u{1F}',
        '\u{7F}',
        '\u{80}',
        '\u{9F}',
        '\u{A0}',
        '\u{E9}',
        '\u{2028}',
        '\u{3000}',
        '\u{D7FF}',
        '\u{E000}',
        '\u{EFFF}',
        '\u{F000}',
        '\u{FFFD}',
        '\u{FFFE}',
        '\u{FFFF}',
        '\u{10000}',
        '\u{1F408}',
        '\u{10FFFF}',
    ];

    /// Lengths 0..=70 cover every tail length and one to four whole chunks;
    /// the larger ones cover long clean runs.
    fn lengths() -> impl Iterator<Item = usize> {
        (0..=70).chain([127, 128, 129, 255, 256, 1000, 4099])
    }

    fn random_bytes(rng: &mut SplitMix64, alphabet: &[u8], len: usize, clean: u8) -> Vec<u8> {
        // Mostly one clean byte, so matches land at every offset rather than
        // almost always in the first lane.
        (0..len)
            .map(|_| {
                if rng.below_usize(4) == 0 {
                    alphabet[rng.below_usize(alphabet.len())]
                } else {
                    clean
                }
            })
            .collect()
    }

    fn random_str(rng: &mut SplitMix64, len: usize, clean: char) -> String {
        (0..len)
            .map(|_| {
                if rng.below_usize(4) == 0 {
                    SCALARS[rng.below_usize(SCALARS.len())]
                } else {
                    clean
                }
            })
            .collect()
    }

    #[test]
    fn class_tables_match_their_ranges() {
        for b in 0..=u8::MAX {
            let cp = widen(b);
            assert_eq!(
                (TRIVIA_TABLE[usize::from(b)] != 0),
                in_ranges(cp, ws_ranges()),
                "{b:#04X}"
            );
            assert_eq!(
                IRI_BODY_TABLE[usize::from(b)] != 0,
                in_ranges(cp, iriref_forbidden_ranges()),
                "{b:#04X}"
            );
            assert_eq!(
                JSON_STRING_TABLE[usize::from(b)] != 0,
                in_ranges(cp, json_string_forbidden_ranges()),
                "{b:#04X}"
            );
            // The tables the scanners use are the tables the predicates use.
            assert_eq!((TRIVIA_TABLE[usize::from(b)] != 0), is_ws(b), "{b:#04X}");
            assert_eq!(
                IRI_BODY_TABLE[usize::from(b)] != 0,
                is_iriref_forbidden_byte(b),
                "{b:#04X}"
            );
            assert_eq!(
                JSON_STRING_TABLE[usize::from(b)] != 0,
                is_json_string_forbidden_byte(b),
                "{b:#04X}"
            );
        }
    }

    /// The comparison form each scanner runs over a chunk agrees with its class
    /// table on every byte value, in every lane.
    #[test]
    fn run_compares_match_the_class_table_in_every_lane() {
        for (name, runs, table) in [
            ("trivia", &TRIVIA_RUNS[..], &TRIVIA_TABLE),
            ("iri body", &IRI_BODY_RUNS[..], &IRI_BODY_TABLE),
            ("json string", &JSON_STRING_RUNS[..], &JSON_STRING_TABLE),
            ("xml", &XML_RUNS[..], &XML_TABLE),
        ] {
            for b in 0..=u8::MAX {
                let member = table[usize::from(b)] != 0;
                assert_eq!(in_runs(b, runs), member, "{name} {b:#04X}");
                for lane in 0..CHUNK {
                    let mut chunk = [if member { b'a' } else { b }; CHUNK];
                    // Put `b` in one lane and a byte of the opposite answer in the
                    // rest, so the mask must single out exactly that lane.
                    let other = (0..=u8::MAX)
                        .find(|&o| (table[usize::from(o)] != 0) != member)
                        .expect("every class has members and non-members");
                    chunk.fill(other);
                    chunk[lane] = b;
                    for (flip, want) in [(0x00, member), (0xFF, !member)] {
                        let lanes = chunk_lanes(&chunk, runs, flip);
                        for (i, &got) in lanes.iter().enumerate() {
                            let hit = if i == lane { want } else { !want };
                            assert_eq!(
                                got,
                                if hit { 0xFF } else { 0x00 },
                                "{name} {b:#04X} in lane {lane}, flip {flip:#04X}, lane {i}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// The XML class is exactly what its documentation says it is.
    #[test]
    fn xml_class_is_the_referenced_set_the_non_chars_and_one_lead_byte() {
        let members: Vec<u8> = (0..=u8::MAX)
            .filter(|&b| XML_TABLE[usize::from(b)] != 0)
            .collect();
        let mut expected: Vec<u8> = (0x00..0x20).collect();
        expected.extend([b'"', b'&', b'<', b'>', 0xEF]);
        assert_eq!(members, expected);
        // The neighbours that must stay clean: the apostrophe XML egress never
        // references, DELETE and the C1 controls (lawful `Char`s), and the lead
        // bytes of the Hangul block beside the surrogates.
        for clean in [b'\'', 0x7F, 0xC2, 0xED, 0xEE, 0xF0] {
            assert_eq!(XML_TABLE[usize::from(clean)], 0, "{clean:#04X}");
        }
    }

    #[test]
    fn byte_scanners_agree_with_the_per_byte_search() {
        let alphabet = boundary_bytes();
        let mut rng = SplitMix64::new(0x0005_EED0_FC1A_55E5);
        type Scanner = fn(&[u8]) -> Option<usize>;
        let cases: [(&str, Scanner, &[ScalarRange], bool, u8); 3] = [
            ("trivia", find_first_trivia, ws_ranges(), false, b' '),
            (
                "iri body",
                find_first_iri_body_special,
                iriref_forbidden_ranges(),
                true,
                b'a',
            ),
            (
                "json string",
                find_first_json_string_special,
                json_string_forbidden_ranges(),
                true,
                b'a',
            ),
        ];
        let mut hits = [0_usize; 3];
        for len in lengths() {
            for _ in 0..40 {
                for (k, &(name, scan, ranges, want, clean)) in cases.iter().enumerate() {
                    let bytes = random_bytes(&mut rng, &alphabet, len, clean);
                    // Every buffer offset 0..=3, so the chunks start misaligned too.
                    for skip in 0..4.min(bytes.len() + 1) {
                        let input = &bytes[skip..];
                        let got = scan(input);
                        assert_eq!(got, reference(input, ranges, want), "{name} {input:02X?}");
                        hits[k] += usize::from(got.is_some_and(|i| i >= CHUNK));
                    }
                }
            }
        }
        // Non-vacuity: every scanner found matches past its first chunk.
        assert!(hits.iter().all(|&h| h > 0), "{hits:?}");
    }

    #[test]
    fn xml_scanner_agrees_with_the_per_scalar_search() {
        let mut rng = SplitMix64::new(0x0C0F_FEE0_00A1_1000);
        for len in lengths() {
            for _ in 0..40 {
                let s = random_str(&mut rng, len, 'x');
                for (skip, _) in s.char_indices().take(4) {
                    let input = &s[skip..];
                    assert_eq!(
                        find_first_xml_special(input.as_bytes()),
                        xml_reference(input),
                        "{input:?}"
                    );
                }
            }
        }
    }

    /// Over arbitrary text, every scanner returns a char boundary, and agrees
    /// with the per-byte search on the text's bytes.
    #[test]
    fn scanners_agree_on_text_and_stop_on_char_boundaries() {
        let mut rng = SplitMix64::new(0x007E_A700_00BE_EF00);
        for len in lengths() {
            for _ in 0..20 {
                let s = random_str(&mut rng, len, 'a');
                let bytes = s.as_bytes();
                for (got, expected) in [
                    (
                        find_first_trivia(bytes),
                        reference(bytes, ws_ranges(), false),
                    ),
                    (
                        find_first_iri_body_special(bytes),
                        reference(bytes, iriref_forbidden_ranges(), true),
                    ),
                    (
                        find_first_json_string_special(bytes),
                        reference(bytes, json_string_forbidden_ranges(), true),
                    ),
                    (find_first_xml_special(bytes), xml_reference(&s)),
                ] {
                    assert_eq!(got, expected, "{s:?}");
                    if let Some(i) = got {
                        assert!(s.is_char_boundary(i), "{s:?} at {i}");
                    }
                }
            }
        }
    }

    /// A single-byte class, and one with a non-ASCII lead byte among ASCII
    /// members: the two shapes writer-owned classes take.
    const LINE_FEED_TABLE: [u8; 256] = {
        let mut table = [0_u8; 256];
        table[b'\n' as usize] = 1;
        table
    };
    const LINE_FEED: ByteClass<{ byte_run_count(&LINE_FEED_TABLE) }> =
        ByteClass::from_table(LINE_FEED_TABLE);
    const MIXED_TABLE: [u8; 256] = {
        let mut table = [0_u8; 256];
        let mut b = 0;
        while b < 0x20 {
            table[b] = 1;
            b += 1;
        }
        table[b'"' as usize] = 1;
        table[b',' as usize] = 1;
        table[0x7F] = 1;
        table[0xC2] = 1;
        table
    };
    const MIXED: ByteClass<{ byte_run_count(&MIXED_TABLE) }> = ByteClass::from_table(MIXED_TABLE);
    const JSON_STRING: ByteClass<{ byte_run_count(&JSON_STRING_TABLE) }> =
        ByteClass::from_table(JSON_STRING_TABLE);

    #[test]
    fn byte_class_agrees_with_the_per_byte_search_and_the_named_scanners() {
        assert_eq!(byte_run_count(&LINE_FEED_TABLE), 1);
        assert_eq!(byte_run_count(&MIXED_TABLE), 5);
        for b in 0..=u8::MAX {
            assert_eq!(LINE_FEED.contains(b), b == b'\n', "{b:#04X}");
            assert_eq!(MIXED.contains(b), MIXED_TABLE[usize::from(b)] != 0);
        }
        let mut alphabet = boundary_bytes();
        alphabet.extend([b'\n', b',', 0xC2, 0xC3]);
        let mut rng = SplitMix64::new(0x00B7_E0C1_A550_0001);
        let mut hits = [0_usize; 2];
        for len in lengths() {
            for _ in 0..40 {
                let bytes = random_bytes(&mut rng, &alphabet, len, b'a');
                for skip in 0..4.min(bytes.len() + 1) {
                    let input = &bytes[skip..];
                    for (k, (class, table)) in [
                        (LINE_FEED.find_first(input), &LINE_FEED_TABLE),
                        (MIXED.find_first(input), &MIXED_TABLE),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let expected = input.iter().position(|&b| table[usize::from(b)] != 0);
                        assert_eq!(class, expected, "{input:02X?}");
                        hits[k] += usize::from(class.is_some_and(|i| i >= CHUNK));
                    }
                    // A class built from a named scanner's table is that scanner.
                    assert_eq!(
                        JSON_STRING.find_first(input),
                        find_first_json_string_special(input)
                    );
                }
            }
        }
        assert!(hits.iter().all(|&h| h > 0), "{hits:?}");
    }

    #[test]
    fn needle_and_pair_searches_agree_with_the_per_byte_search() {
        // Lengths straddle every block and chunk boundary, and each input is
        // searched at several skips, so a hit lands in the wide blocks, in the
        // sixteen-byte chunks after them and in the byte tail.
        let alphabet = *b"abnNeE\x80\xFF";
        let mut rng = SplitMix64::new(0x00B7_E0C1_A550_0002);
        let mut hits = [0_usize; 4];
        for len in lengths() {
            for _ in 0..24 {
                let bytes = random_bytes(&mut rng, &alphabet, len, b'.');
                for skip in 0..4.min(bytes.len() + 1) {
                    let input = &bytes[skip..];
                    let one = find_byte(input, b'n');
                    assert_eq!(one, input.iter().position(|&b| b == b'n'), "{input:02X?}");
                    let two = find_byte2(input, b'n', 0xFF);
                    let expected = input.iter().position(|&b| b == b'n' || b == 0xFF);
                    assert_eq!(two, expected, "{input:02X?}");
                    hits[0] += usize::from(one.is_some_and(|i| i >= BLOCK));
                    for gap in [0, 1, 5, 17, 70] {
                        let (lead, trail) = (*b"nN", *b"ee");
                        let found = find_byte_pair(input, lead, trail, gap);
                        let expected = (0..input.len().saturating_sub(gap))
                            .find(|&p| lead.contains(&input[p]) && trail.contains(&input[p + gap]));
                        assert_eq!(found, expected, "gap {gap} in {input:02X?}");
                        hits[1] += usize::from(found.is_some_and(|i| i >= BLOCK));
                        hits[2] += usize::from(found.is_some_and(|i| i < CHUNK));
                        // One byte written twice takes its own kernel.
                        let single = find_byte_pair(input, [b'n'; 2], [b'e'; 2], gap);
                        let expected = (0..input.len().saturating_sub(gap))
                            .find(|&p| input[p] == b'n' && input[p + gap] == b'e');
                        assert_eq!(single, expected, "gap {gap} in {input:02X?}");
                        // Runs: `a`-`b` or `N`, then `e` or any byte from 0x80.
                        let (lead, trail) =
                            ([(b'a', b'b'), (b'N', b'N')], [(b'e', b'e'), (0x80, 0xFF)]);
                        let ranged = find_range_pair(input, lead, trail, gap);
                        let expected = (0..input.len().saturating_sub(gap)).find(|&p| {
                            matches!(input[p], b'a' | b'b' | b'N')
                                && (input[p + gap] == b'e' || input[p + gap] >= 0x80)
                        });
                        assert_eq!(ranged, expected, "gap {gap} in {input:02X?}");
                        hits[3] += usize::from(ranged.is_some_and(|i| i >= BLOCK));
                        // The mixed kernels: one side a single byte or run.
                        let mixed = find_byte_pair(input, [b'n'; 2], [b'e', 0xFF], gap);
                        let expected = (0..input.len().saturating_sub(gap))
                            .find(|&p| input[p] == b'n' && matches!(input[p + gap], b'e' | 0xFF));
                        assert_eq!(mixed, expected, "gap {gap} in {input:02X?}");
                        let mixed = find_range_pair(input, [(b'a', b'b'); 2], trail, gap);
                        let expected = (0..input.len().saturating_sub(gap)).find(|&p| {
                            matches!(input[p], b'a' | b'b')
                                && (input[p + gap] == b'e' || input[p + gap] >= 0x80)
                        });
                        assert_eq!(mixed, expected, "gap {gap} in {input:02X?}");
                    }
                }
            }
        }
        assert!(hits.iter().all(|&h| h > 0), "{hits:?}");
        // A gap at or past the input's length has no position to test.
        assert_eq!(find_byte_pair(b"ne", [b'n'; 2], [b'e'; 2], 2), None);
        assert_eq!(find_byte_pair(b"", [b'n'; 2], [b'e'; 2], 0), None);
        assert_eq!(find_byte_pair(b"ne", [b'n'; 2], [b'e'; 2], 1), Some(0));
    }

    #[test]
    fn empty_and_all_clean_inputs_find_nothing() {
        assert_eq!(find_first_trivia(b""), None);
        assert_eq!(find_first_iri_body_special(b""), None);
        assert_eq!(find_first_json_string_special(b""), None);
        assert_eq!(find_first_xml_special(b""), None);
        let clean = [b'a'; 100];
        assert_eq!(find_first_iri_body_special(&clean), None);
        assert_eq!(find_first_json_string_special(&clean), None);
        assert_eq!(find_first_xml_special(&clean), None);
        // And the neighbour: the same input with one member at the far end.
        let mut dirty = clean;
        dirty[99] = b'"';
        assert_eq!(find_first_iri_body_special(&dirty), Some(99));
        assert_eq!(find_first_json_string_special(&dirty), Some(99));
        assert_eq!(find_first_xml_special(&dirty), Some(99));
        assert_eq!(find_first_trivia(&[b' '; 100]), None);
        assert_eq!(find_first_trivia(&clean), Some(0));
    }
}
