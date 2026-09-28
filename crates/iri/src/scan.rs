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
//!    [`class_table`](crate::terminals::class_table).
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

/// An inclusive run `[lo, hi]` of member bytes.
pub(crate) type ByteRun = (u8, u8);

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
pub(crate) const fn count_runs(table: &[u8; 256]) -> usize {
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
pub(crate) const fn byte_runs<const N: usize>(table: &[u8; 256]) -> [ByteRun; N] {
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
pub(crate) fn in_runs(b: u8, runs: &[ByteRun]) -> bool {
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
/// use purrdf_iri::terminals::{ByteClass, byte_run_count};
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
/// use purrdf_iri::terminals::{ByteClass, byte_run_count};
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
/// use purrdf_iri::terminals::find_first_trivia;
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
/// use purrdf_iri::terminals::find_first_iri_body_special;
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
/// use purrdf_iri::terminals::find_first_json_string_special;
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
/// use purrdf_iri::terminals::find_first_xml_special;
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

    /// A fixed-seed generator (SplitMix64), so every run draws the same inputs.
    struct SplitMix(u64);

    impl SplitMix {
        const fn next(&mut self) -> u64 {
            purrdf_testkit::rng::splitmix64_next(&mut self.0)
        }

        fn below(&mut self, n: usize) -> usize {
            usize::try_from(self.next() % n as u64).expect("below n")
        }
    }

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

    fn random_bytes(rng: &mut SplitMix, alphabet: &[u8], len: usize, clean: u8) -> Vec<u8> {
        // Mostly one clean byte, so matches land at every offset rather than
        // almost always in the first lane.
        (0..len)
            .map(|_| {
                if rng.below(4) == 0 {
                    alphabet[rng.below(alphabet.len())]
                } else {
                    clean
                }
            })
            .collect()
    }

    fn random_str(rng: &mut SplitMix, len: usize, clean: char) -> String {
        (0..len)
            .map(|_| {
                if rng.below(4) == 0 {
                    SCALARS[rng.below(SCALARS.len())]
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
        let mut rng = SplitMix(0x0005_EED0_FC1A_55E5);
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
        let mut rng = SplitMix(0x0C0F_FEE0_00A1_1000);
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
        let mut rng = SplitMix(0x007E_A700_00BE_EF00);
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
        let mut rng = SplitMix(0x00B7_E0C1_A550_0001);
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
