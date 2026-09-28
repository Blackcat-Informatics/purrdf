// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The analyzer's Unicode layer: full case folding, the four normalization
//! forms, and word boundaries, over tables generated from the vendored Unicode
//! Character Database.
//!
//! * [`case_fold`] — full default case folding, the `C` and `F` mappings of
//!   `CaseFolding.txt` (Unicode core specification §3.13).
//! * [`nfd`], [`nfc`], [`nfkd`], [`nfkc`] — the normalization forms of
//!   `UAX #15`: full decomposition, the canonical ordering algorithm, and the
//!   canonical composition algorithm over the primary composites
//!   (core specification §3.11), with Hangul syllables by the arithmetic of
//!   §3.12.
//! * [`analysis_form`] — `NFD → fold → NFKD → fold → NFKD → NFC`, the
//!   compatibility caseless form (§3.13 D146) completed with NFC.
//! * [`word_bounds`], [`word_indices`] — the default word boundaries of
//!   `UAX #29` (rules WB1 to WB999); [`word_indices`] keeps only segments that
//!   hold an [`is_alphanumeric`] character.
//!
//! Every table is a pure function of the vendored database, so the answers do
//! not depend on the toolchain the crate is built with.
//!
//! # The ASCII fast path
//!
//! An ASCII run is its own NFD, NFKD and NFC, and its full case fold is its
//! ASCII lowercase. The input is scanned eight bytes at a time as one `u64`
//! (a word whose bytes all have the high bit clear is ASCII), and such runs are
//! written to the output directly, bypassing the per-character pipeline. The
//! last ASCII character before a non-ASCII byte is not bypassed, because a
//! combining mark after it may compose with it. Bypassing at an ASCII character
//! is exact: it has combining class 0 in every stage, so it ends every held run
//! of non-starters, and no primary composite has an ASCII second character (the
//! generator refuses to emit tables in which one does), so nothing before it
//! can compose with it.

use crate::unicode_tables as tables;

/// The Unicode version of the normalization, word-break and alphanumeric
/// tables.
pub const UNICODE_VERSION: (u8, u8, u8) = tables::UNICODE_VERSION;

/// The Unicode version of the `CaseFolding.txt` the fold tables come from.
pub const FOLD_UNICODE_VERSION: (u8, u8, u8) = tables::FOLD_UNICODE_VERSION;

/// Hangul syllable arithmetic, core specification §3.12.
const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT;
const S_COUNT: u32 = L_COUNT * N_COUNT;

/// Every byte's high bit, for the eight-byte ASCII scan.
const HIGH_BITS: u64 = 0x8080_8080_8080_8080;

/// One two-stage table lookup.
#[inline]
fn lookup<T: Copy>(index: &[u16], blocks: &[T], c: char) -> T {
    let point = c as u32;
    let block = usize::from(index[(point >> tables::BLOCK_SHIFT) as usize]);
    let low = (point & ((1 << tables::BLOCK_SHIFT) - 1)) as usize;
    blocks[(block << tables::BLOCK_SHIFT) | low]
}

/// `Canonical_Combining_Class`.
#[inline]
fn combining_class(c: char) -> u8 {
    if c < '\u{300}' {
        0
    } else {
        lookup(
            &tables::COMBINING_CLASS_INDEX,
            &tables::COMBINING_CLASS_BLOCKS,
            c,
        )
    }
}

/// The segmentation byte: `Word_Break` value and flags.
#[inline]
fn segmentation(c: char) -> u8 {
    lookup(&tables::SEGMENTATION_INDEX, &tables::SEGMENTATION_BLOCKS, c)
}

/// Whether `c` is `Alphabetic`, or of General_Category `Nd`, `Nl` or `No`:
/// the predicate the word filter keeps a segment by.
#[must_use]
pub fn is_alphanumeric(c: char) -> bool {
    if c.is_ascii() {
        c.is_ascii_alphanumeric()
    } else {
        segmentation(c) & tables::ALPHANUMERIC != 0
    }
}

// ---------------------------------------------------------------------------
// Output sinks.

/// Where [`analysis_form`] writes its output.
///
/// Implemented for [`String`] (append) and for [`Compare`] (compare against an
/// expected string without allocating).
pub trait Sink {
    /// Append one character.
    fn push_char(&mut self, c: char);

    /// Append the ASCII lowercase of `ascii`, which is all ASCII.
    fn push_ascii_lowercase(&mut self, ascii: &str);
}

impl Sink for String {
    #[inline]
    fn push_char(&mut self, c: char) {
        self.push(c);
    }

    #[inline]
    fn push_ascii_lowercase(&mut self, ascii: &str) {
        let start = self.len();
        self.push_str(ascii);
        self[start..].make_ascii_lowercase();
    }
}

/// A [`Sink`] that decides whether the streamed output equals an expected
/// string, without building the output.
#[derive(Clone, Debug)]
pub struct Compare<'a> {
    expected: &'a [u8],
    offset: usize,
    equal: bool,
}

impl<'a> Compare<'a> {
    /// A comparison against `expected`.
    #[must_use]
    pub const fn new(expected: &'a str) -> Self {
        Self {
            expected: expected.as_bytes(),
            offset: 0,
            equal: true,
        }
    }

    /// Whether everything streamed into this sink equals the expected string.
    #[must_use]
    pub const fn finish(self) -> bool {
        self.equal && self.offset == self.expected.len()
    }

    #[inline]
    fn advance(&mut self, bytes: &[u8], lowercase: bool) {
        if !self.equal {
            return;
        }
        let end = self.offset + bytes.len();
        let matches = self.expected.get(self.offset..end).is_some_and(|window| {
            if lowercase {
                window
                    .iter()
                    .zip(bytes)
                    .all(|(want, got)| *want == got.to_ascii_lowercase())
            } else {
                window == bytes
            }
        });
        self.equal = matches;
        self.offset = end;
    }
}

impl Sink for Compare<'_> {
    #[inline]
    fn push_char(&mut self, c: char) {
        let mut buffer = [0u8; 4];
        self.advance(c.encode_utf8(&mut buffer).as_bytes(), false);
    }

    #[inline]
    fn push_ascii_lowercase(&mut self, ascii: &str) {
        self.advance(ascii.as_bytes(), true);
    }
}

// ---------------------------------------------------------------------------
// Pipeline stages.

/// One stage of a normalization or folding pipeline.
trait Stage {
    /// Accept the next character.
    fn push(&mut self, c: char);
    /// Release everything held, then pass `ascii` (all ASCII) through,
    /// lowercased if `lowercase` or if this stage folds.
    fn push_ascii(&mut self, ascii: &str, lowercase: bool);
    /// Release everything held: end of input.
    fn flush(&mut self);
}

/// The last stage: a [`Sink`].
struct Out<'s, O: ?Sized + Sink>(&'s mut O);

impl<O: ?Sized + Sink> Stage for Out<'_, O> {
    #[inline]
    fn push(&mut self, c: char) {
        self.0.push_char(c);
    }

    #[inline]
    fn push_ascii(&mut self, ascii: &str, lowercase: bool) {
        if lowercase {
            self.0.push_ascii_lowercase(ascii);
        } else {
            ascii.chars().for_each(|c| self.0.push_char(c));
        }
    }

    fn flush(&mut self) {}
}

/// The last stage of the standalone operations: a [`String`].
struct Collect(String);

impl Stage for Collect {
    #[inline]
    fn push(&mut self, c: char) {
        self.0.push(c);
    }

    #[inline]
    fn push_ascii(&mut self, ascii: &str, lowercase: bool) {
        if lowercase {
            self.0.push_ascii_lowercase(ascii);
        } else {
            self.0.push_str(ascii);
        }
    }

    fn flush(&mut self) {}
}

/// Characters held with their combining classes: inline for ordinary text,
/// spilling to the heap only for a run longer than the inline capacity.
struct Held {
    inline: [(u8, char); Self::INLINE],
    len: usize,
    spill: Vec<(u8, char)>,
}

impl Held {
    const INLINE: usize = 32;

    const fn new() -> Self {
        Self {
            inline: [(0, '\0'); Self::INLINE],
            len: 0,
            spill: Vec::new(),
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.len == 0 && self.spill.is_empty()
    }

    #[inline]
    fn push(&mut self, entry: (u8, char)) {
        if self.spill.is_empty() && self.len < Self::INLINE {
            self.inline[self.len] = entry;
            self.len += 1;
        } else {
            if self.spill.is_empty() {
                self.spill.extend_from_slice(&self.inline[..self.len]);
                self.len = 0;
            }
            self.spill.push(entry);
        }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [(u8, char)] {
        if self.spill.is_empty() {
            &mut self.inline[..self.len]
        } else {
            &mut self.spill
        }
    }

    #[inline]
    fn clear(&mut self) {
        self.len = 0;
        self.spill.clear();
    }

    /// Stably sort by combining class — the canonical ordering algorithm
    /// (§3.11) over one run of non-starters — and hand each to `next`.
    fn release_ordered<N: Stage>(&mut self, next: &mut N) {
        let run = self.as_mut_slice();
        // Insertion sort: stable, allocation-free, and the runs are short.
        for i in 1..run.len() {
            let mut j = i;
            while j > 0 && run[j - 1].0 > run[j].0 {
                run.swap(j - 1, j);
                j -= 1;
            }
        }
        for &(_, c) in run.iter() {
            next.push(c);
        }
        self.clear();
    }

    /// Hand each held character to `next` as it stands.
    fn release<N: Stage>(&mut self, next: &mut N) {
        for &(_, c) in self.as_mut_slice().iter() {
            next.push(c);
        }
        self.clear();
    }
}

/// Full decomposition (canonical, or compatibility if `COMPAT`), then the
/// canonical ordering algorithm.
struct Decompose<const COMPAT: bool, N> {
    next: N,
    held: Held,
}

impl<const COMPAT: bool, N: Stage> Decompose<COMPAT, N> {
    const fn new(next: N) -> Self {
        Self {
            next,
            held: Held::new(),
        }
    }

    #[inline]
    fn emit(&mut self, c: char) {
        let class = combining_class(c);
        if class == 0 {
            self.held.release_ordered(&mut self.next);
            self.next.push(c);
        } else {
            self.held.push((class, c));
        }
    }
}

impl<const COMPAT: bool, N: Stage> Stage for Decompose<COMPAT, N> {
    #[inline]
    fn push(&mut self, c: char) {
        if c < '\u{A0}' {
            self.emit(c);
            return;
        }
        let point = c as u32;
        if (S_BASE..S_BASE + S_COUNT).contains(&point) {
            let index = point - S_BASE;
            for part in [
                L_BASE + index / N_COUNT,
                V_BASE + (index % N_COUNT) / T_COUNT,
                T_BASE + index % T_COUNT,
            ] {
                if part != T_BASE
                    && let Some(jamo) = char::from_u32(part)
                {
                    self.emit(jamo);
                }
            }
            return;
        }
        let record = lookup(
            &tables::DECOMPOSITION_INDEX,
            &tables::DECOMPOSITION_BLOCKS,
            c,
        );
        if record == 0 {
            self.emit(c);
            return;
        }
        let (canonical_start, canonical_len, compat_start, compat_len) =
            tables::DECOMPOSITION_RECORDS[usize::from(record)];
        let (start, len) = if COMPAT {
            (compat_start, compat_len)
        } else {
            (canonical_start, canonical_len)
        };
        if len == 0 {
            self.emit(c);
            return;
        }
        let start = usize::from(start);
        for &part in &tables::DECOMPOSITION_CHARS[start..start + usize::from(len)] {
            self.emit(part);
        }
    }

    fn push_ascii(&mut self, ascii: &str, lowercase: bool) {
        self.held.release_ordered(&mut self.next);
        self.next.push_ascii(ascii, lowercase);
    }

    fn flush(&mut self) {
        self.held.release_ordered(&mut self.next);
        self.next.flush();
    }
}

/// Full case folding: `CaseFolding.txt` statuses `C` and `F`.
struct Fold<N> {
    next: N,
}

impl<N: Stage> Stage for Fold<N> {
    #[inline]
    fn push(&mut self, c: char) {
        if c.is_ascii() {
            self.next.push(c.to_ascii_lowercase());
            return;
        }
        let record = lookup(&tables::FOLD_INDEX, &tables::FOLD_BLOCKS, c);
        if record == 0 {
            self.next.push(c);
            return;
        }
        let (start, len) = tables::FOLD_RECORDS[usize::from(record)];
        let start = usize::from(start);
        for &part in &tables::FOLD_CHARS[start..start + usize::from(len)] {
            self.next.push(part);
        }
    }

    fn push_ascii(&mut self, ascii: &str, _lowercase: bool) {
        self.next.push_ascii(ascii, true);
    }

    fn flush(&mut self) {
        self.next.flush();
    }
}

/// The primary composite of `first` and `second`, if there is one.
#[inline]
fn compose_pair(first: char, second: char) -> Option<char> {
    let (a, b) = (first as u32, second as u32);
    if (L_BASE..L_BASE + L_COUNT).contains(&a) && (V_BASE..V_BASE + V_COUNT).contains(&b) {
        let index = (a - L_BASE) * N_COUNT + (b - V_BASE) * T_COUNT;
        return char::from_u32(S_BASE + index);
    }
    if (S_BASE..S_BASE + S_COUNT).contains(&a)
        && (a - S_BASE).is_multiple_of(T_COUNT)
        && (T_BASE + 1..T_BASE + T_COUNT).contains(&b)
    {
        return char::from_u32(a + (b - T_BASE));
    }
    tables::COMPOSITIONS
        .binary_search_by(|&(x, y, _)| (x, y).cmp(&(first, second)))
        .ok()
        .map(|found| tables::COMPOSITIONS[found].2)
}

/// Whether `c`, of combining class `class`, can be the second character of a
/// primary composite at all: a quick check that spares a starter the pair
/// table search, since almost no starter composes with what precedes it.
#[inline]
fn may_compose_second(c: char, class: u8) -> bool {
    if class != 0 {
        return true;
    }
    let point = c as u32;
    (V_BASE..V_BASE + V_COUNT).contains(&point)
        || (T_BASE + 1..T_BASE + T_COUNT).contains(&point)
        || (point >= 0x300 && tables::COMPOSING_STARTERS.binary_search(&c).is_ok())
}

/// The canonical composition algorithm (§3.11) over decomposed, canonically
/// ordered input.
struct Compose<N> {
    next: N,
    /// The last starter, still open to composition.
    starter: Option<char>,
    /// The uncomposed characters after it, all non-starters, in order.
    pending: Held,
    /// The combining class of the last entry of `pending`.
    last_class: u8,
}

impl<N: Stage> Compose<N> {
    const fn new(next: N) -> Self {
        Self {
            next,
            starter: None,
            pending: Held::new(),
            last_class: 0,
        }
    }

    fn release(&mut self) {
        if let Some(starter) = self.starter.take() {
            self.next.push(starter);
        }
        self.pending.release(&mut self.next);
        self.last_class = 0;
    }
}

impl<N: Stage> Stage for Compose<N> {
    #[inline]
    fn push(&mut self, c: char) {
        let class = combining_class(c);
        let Some(starter) = self.starter else {
            if class == 0 {
                self.starter = Some(c);
            } else {
                self.next.push(c);
            }
            return;
        };
        // `c` is blocked from the starter when some character between them
        // has class 0 or a class not below its own. The pending characters are
        // non-starters in canonical order, so the last has the greatest class.
        let blocked = !self.pending.is_empty() && self.last_class >= class;
        if !blocked
            && may_compose_second(c, class)
            && let Some(composite) = compose_pair(starter, c)
        {
            self.starter = Some(composite);
            return;
        }
        if class == 0 {
            self.release();
            self.starter = Some(c);
        } else {
            self.pending.push((class, c));
            self.last_class = class;
        }
    }

    fn push_ascii(&mut self, ascii: &str, lowercase: bool) {
        self.release();
        self.next.push_ascii(ascii, lowercase);
    }

    fn flush(&mut self) {
        self.release();
        self.next.flush();
    }
}

/// The length of the prefix of `bytes` whose bytes all have the high bit
/// `high` (set: non-ASCII) or not (clear: ASCII), eight bytes at a time: a
/// `u64` word is tested with one mask instead of eight comparisons.
#[inline]
fn prefix_len(bytes: &[u8], high: bool) -> usize {
    let (words, tail) = bytes.as_chunks::<8>();
    let mut len = 0;
    for word in words {
        let bits = u64::from_le_bytes(*word);
        // The high bits of the bytes that end the prefix.
        let enders = if high { !bits } else { bits } & HIGH_BITS;
        if enders != 0 {
            return len + enders.trailing_zeros() as usize / 8;
        }
        len += 8;
    }
    len + tail
        .iter()
        .position(|byte| byte.is_ascii() == high)
        .unwrap_or(tail.len())
}

/// The length of the ASCII prefix of `bytes`.
#[inline]
fn ascii_prefix_len(bytes: &[u8]) -> usize {
    prefix_len(bytes, false)
}

/// The length of the non-ASCII prefix of `bytes`.
#[inline]
fn non_ascii_prefix_len(bytes: &[u8]) -> usize {
    prefix_len(bytes, true)
}

/// Run `input` through `pipeline`, bypassing it for ASCII runs as the module
/// documentation describes.
fn drive<P: Stage>(input: &str, pipeline: &mut P) {
    let bytes = input.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let ascii_end = at + ascii_prefix_len(&bytes[at..]);
        if ascii_end == bytes.len() {
            pipeline.push_ascii(&input[at..], false);
            break;
        }
        // Hold back the last ASCII character: a combining mark may follow.
        let slow_start = if ascii_end > at {
            if ascii_end - 1 > at {
                pipeline.push_ascii(&input[at..ascii_end - 1], false);
            }
            ascii_end - 1
        } else {
            at
        };
        // A non-ASCII run ends at an ASCII byte, which is always a character
        // boundary in UTF-8.
        let slow_end = ascii_end + non_ascii_prefix_len(&bytes[ascii_end..]);
        input[slow_start..slow_end]
            .chars()
            .for_each(|c| pipeline.push(c));
        at = slow_end;
    }
    pipeline.flush();
}

/// Run `input` through the pipeline `build` makes around a collector.
fn collect<P: Stage>(
    input: &str,
    build: impl FnOnce(Collect) -> P,
    take: impl FnOnce(P) -> Collect,
) -> String {
    let mut pipeline = build(Collect(String::with_capacity(input.len())));
    drive(input, &mut pipeline);
    take(pipeline).0
}

/// The full case fold of `text`: every character replaced by its
/// `CaseFolding.txt` mapping of status `C` or `F` (core specification §3.13).
#[must_use]
pub fn case_fold(text: &str) -> String {
    collect(text, |next| Fold { next }, |stage| stage.next)
}

/// Normalization Form D (`UAX #15`): canonical decomposition.
#[must_use]
pub fn nfd(text: &str) -> String {
    collect(text, Decompose::<false, _>::new, |stage| stage.next)
}

/// Normalization Form KD (`UAX #15`): compatibility decomposition.
#[must_use]
pub fn nfkd(text: &str) -> String {
    collect(text, Decompose::<true, _>::new, |stage| stage.next)
}

/// Normalization Form C (`UAX #15`): canonical decomposition, then canonical
/// composition.
#[must_use]
pub fn nfc(text: &str) -> String {
    collect(
        text,
        |next| Decompose::<false, _>::new(Compose::new(next)),
        |stage| stage.next.next,
    )
}

/// Normalization Form KC (`UAX #15`): compatibility decomposition, then
/// canonical composition.
#[must_use]
pub fn nfkc(text: &str) -> String {
    collect(
        text,
        |next| Decompose::<true, _>::new(Compose::new(next)),
        |stage| stage.next.next,
    )
}

/// Append the analysis form of `input` to `out`:
/// `NFC(NFKD(fold(NFKD(fold(NFD(input))))))`.
///
/// The inner five steps are the compatibility caseless form of the core
/// specification §3.13 (D146); two strings have the same analysis form exactly
/// when they are compatibility caseless matches.
pub fn analysis_form<O: ?Sized + Sink>(input: &str, out: &mut O) {
    let mut pipeline = Decompose::<false, _>::new(Fold {
        next: Decompose::<true, _>::new(Fold {
            next: Decompose::<true, _>::new(Compose::new(Out(out))),
        }),
    });
    drive(input, &mut pipeline);
}

// ---------------------------------------------------------------------------
// Word boundaries, UAX 29.

/// Before the first character (`sot`), and after the last (`eot`).
const SOT: u8 = 0xFE;
const EOT: u8 = 0xFF;

/// `AHLetter`.
#[inline]
const fn is_ah_letter(value: u8) -> bool {
    value == tables::WB_ALETTER || value == tables::WB_HEBREW_LETTER
}

/// `MidLetter | MidNumLetQ`.
#[inline]
const fn is_mid_letter_q(value: u8) -> bool {
    value == tables::WB_MIDLETTER
        || value == tables::WB_MIDNUMLET
        || value == tables::WB_SINGLE_QUOTE
}

/// `MidNum | MidNumLetQ`.
#[inline]
const fn is_mid_num_q(value: u8) -> bool {
    value == tables::WB_MIDNUM || value == tables::WB_MIDNUMLET || value == tables::WB_SINGLE_QUOTE
}

/// `Extend | Format | ZWJ`, the characters rule WB4 attaches to what precedes
/// them.
#[inline]
const fn is_ignorable(value: u8) -> bool {
    value == tables::WB_EXTEND || value == tables::WB_FORMAT || value == tables::WB_ZWJ
}

/// `Newline | CR | LF`.
#[inline]
const fn is_newline(value: u8) -> bool {
    value == tables::WB_NEWLINE || value == tables::WB_CR || value == tables::WB_LF
}

/// The `Word_Break` value of the first character of `text` that rule WB4 does
/// not attach to its predecessor, or [`EOT`].
fn next_effective(text: &str) -> u8 {
    text.chars()
        .map(|c| segmentation(c) & tables::WB_MASK)
        .find(|&value| !is_ignorable(value))
        .unwrap_or(EOT)
}

/// A forward pass over the default word boundaries.
#[derive(Clone, Debug)]
struct Boundaries<'a> {
    text: &'a str,
    /// The byte offset where the next segment starts.
    start: usize,
    /// The `Word_Break` value of the character before `start`, as written.
    raw: u8,
    /// The value of the last character WB4 did not attach to its predecessor.
    last: u8,
    /// The value of the such character before `last`.
    before_last: u8,
    /// How many `Regional_Indicator`s end at `last`, counting only those WB4
    /// left in place.
    regional: usize,
}

impl<'a> Boundaries<'a> {
    const fn new(text: &'a str) -> Self {
        Self {
            text,
            start: 0,
            raw: SOT,
            last: SOT,
            before_last: SOT,
            regional: 0,
        }
    }

    /// Take `value` into the left context.
    #[inline]
    fn accept(&mut self, value: u8) {
        // WB4: X (Extend | Format | ZWJ)* → X, except after sot, CR, LF and
        // Newline.
        let attached = is_ignorable(value) && self.raw != SOT && !is_newline(self.raw);
        self.raw = value;
        if !attached {
            self.regional = if value == tables::WB_REGIONAL_INDICATOR {
                if self.last == tables::WB_REGIONAL_INDICATOR {
                    self.regional + 1
                } else {
                    1
                }
            } else {
                0
            };
            self.before_last = self.last;
            self.last = value;
        }
    }

    /// Whether there is a boundary before a character with segmentation byte
    /// `byte`, followed by `rest`. The rules in their numbered order.
    fn breaks_before(&self, byte: u8, rest: &str) -> bool {
        let value = byte & tables::WB_MASK;
        let (raw, last, before_last) = (self.raw, self.last, self.before_last);
        // WB3: CR × LF.
        if raw == tables::WB_CR && value == tables::WB_LF {
            return false;
        }
        // WB3a, WB3b: break after and before Newline | CR | LF.
        if is_newline(raw) || is_newline(value) {
            return true;
        }
        // WB3c: ZWJ × \p{Extended_Pictographic}.
        if raw == tables::WB_ZWJ && byte & tables::EXTENDED_PICTOGRAPHIC != 0 {
            return false;
        }
        // WB3d: WSegSpace × WSegSpace.
        if raw == tables::WB_WSEGSPACE && value == tables::WB_WSEGSPACE {
            return false;
        }
        // WB4: no break before Extend | Format | ZWJ (sot, CR, LF and Newline
        // on the left are decided above).
        if is_ignorable(value) {
            return false;
        }
        let numeric = tables::WB_NUMERIC;
        let katakana = tables::WB_KATAKANA;
        let extend_num_let = tables::WB_EXTENDNUMLET;
        let hebrew = tables::WB_HEBREW_LETTER;
        let joins = (is_ah_letter(last) && is_ah_letter(value)) // WB5
            || (is_ah_letter(last) && is_mid_letter_q(value) && is_ah_letter(next_effective(rest))) // WB6
            || (is_ah_letter(before_last) && is_mid_letter_q(last) && is_ah_letter(value)) // WB7
            || (last == hebrew && value == tables::WB_SINGLE_QUOTE) // WB7a
            || (last == hebrew && value == tables::WB_DOUBLE_QUOTE && next_effective(rest) == hebrew) // WB7b
            || (before_last == hebrew && last == tables::WB_DOUBLE_QUOTE && value == hebrew) // WB7c
            || (last == numeric && value == numeric) // WB8
            || (is_ah_letter(last) && value == numeric) // WB9
            || (last == numeric && is_ah_letter(value)) // WB10
            || (before_last == numeric && is_mid_num_q(last) && value == numeric) // WB11
            || (last == numeric && is_mid_num_q(value) && next_effective(rest) == numeric) // WB12
            || (last == katakana && value == katakana) // WB13
            || ((is_ah_letter(last) || last == numeric || last == katakana || last == extend_num_let)
                && value == extend_num_let) // WB13a
            || (last == extend_num_let
                && (is_ah_letter(value) || value == numeric || value == katakana)) // WB13b
            || (last == tables::WB_REGIONAL_INDICATOR
                && value == tables::WB_REGIONAL_INDICATOR
                && self.regional % 2 == 1); // WB15, WB16
        // WB999: otherwise, break everywhere.
        !joins
    }

    /// The next segment and its byte offset.
    fn next_segment(&mut self) -> Option<(usize, &'a str)> {
        let text = self.text;
        let start = self.start;
        let rest = &text[start..];
        let mut chars = rest.char_indices();
        // WB1: the boundary before the first character, and the boundary
        // before this segment, are already decided.
        let (_, first) = chars.next()?;
        self.accept(segmentation(first) & tables::WB_MASK);
        let mut end = rest.len();
        for (offset, c) in chars {
            let byte = segmentation(c);
            if self.breaks_before(byte, &rest[offset + c.len_utf8()..]) {
                end = offset;
                break;
            }
            self.accept(byte & tables::WB_MASK);
        }
        // WB2: the end of text is a boundary.
        self.start = start + end;
        Some((start, &rest[..end]))
    }
}

/// The segments of `text` between default word boundaries (`UAX #29`), every
/// one of them, in order. Concatenated, they are `text`.
#[must_use]
pub fn word_bounds(text: &str) -> WordBounds<'_> {
    WordBounds(Boundaries::new(text))
}

/// The word segments of `text` with their byte offsets: the segments of
/// [`word_bounds`] that hold at least one [`is_alphanumeric`] character.
#[must_use]
pub fn word_indices(text: &str) -> WordIndices<'_> {
    WordIndices(Boundaries::new(text))
}

/// The iterator [`word_bounds`] returns.
#[derive(Clone, Debug)]
pub struct WordBounds<'a>(Boundaries<'a>);

impl<'a> Iterator for WordBounds<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        self.0.next_segment().map(|(_, segment)| segment)
    }
}

/// The iterator [`word_indices`] returns.
#[derive(Clone, Debug)]
pub struct WordIndices<'a>(Boundaries<'a>);

impl<'a> Iterator for WordIndices<'a> {
    type Item = (usize, &'a str);

    fn next(&mut self) -> Option<(usize, &'a str)> {
        loop {
            let (start, segment) = self.0.next_segment()?;
            if segment.chars().any(is_alphanumeric) {
                return Some((start, segment));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Compose, Decompose, Fold, Out, Stage, analysis_form, ascii_prefix_len, non_ascii_prefix_len,
    };

    /// The analysis form with no ASCII bypass: every character through the
    /// pipeline.
    fn analysis_form_without_bypass(input: &str) -> String {
        let mut out = String::new();
        let mut pipeline = Decompose::<false, _>::new(Fold {
            next: Decompose::<true, _>::new(Fold {
                next: Decompose::<true, _>::new(Compose::new(Out(&mut out))),
            }),
        });
        input.chars().for_each(|c| pipeline.push(c));
        pipeline.flush();
        out
    }

    /// The ASCII bypass is exact: every ASCII character, alone and before and
    /// after every non-starter and every character that composes, analyzes
    /// the same with and without it.
    #[test]
    fn the_ascii_bypass_agrees_with_the_pipeline() {
        let mut interesting: Vec<char> = crate::unicode_tables::COMPOSITIONS
            .iter()
            .flat_map(|&triple| <[char; 3]>::from(triple))
            .collect();
        interesting.extend([
            '\u{345}', '\u{212A}', '\u{FB01}', '\u{1E9E}', '\u{130}', '\u{AC00}', '\u{1161}',
            '\u{11A8}',
        ]);
        interesting.sort_unstable();
        interesting.dedup();
        for ascii in (0u8..0x80).map(char::from) {
            for &other in &interesting {
                for input in [
                    format!("{ascii}{other}"),
                    format!("{other}{ascii}"),
                    format!("ABCDEFGH{ascii}{other}{ascii}xyzXYZ12"),
                    format!("{other}{other}{ascii}{ascii}{other}"),
                ] {
                    let mut bypassed = String::new();
                    analysis_form(&input, &mut bypassed);
                    assert_eq!(bypassed, analysis_form_without_bypass(&input), "{input:?}");
                }
            }
        }
    }

    #[test]
    fn the_word_scans_agree_with_a_byte_scan() {
        let samples: [&[u8]; 6] = [
            b"",
            b"abcdefgh",
            b"abcdefghi\xC3\xA9",
            b"\xC3\xA9abc",
            b"abcdefgh12345678\xE4\xB8\xADxyz",
            b"\xE4\xB8\xAD\xE4\xB8\xAD\xE4\xB8\xAD\xE4\xB8\xAD!",
        ];
        for bytes in samples {
            for start in 0..bytes.len() {
                let tail = &bytes[start..];
                assert_eq!(
                    ascii_prefix_len(tail),
                    tail.iter()
                        .position(|b| !b.is_ascii())
                        .unwrap_or(tail.len())
                );
                assert_eq!(
                    non_ascii_prefix_len(tail),
                    tail.iter().position(u8::is_ascii).unwrap_or(tail.len())
                );
            }
        }
    }
}
