// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 1951 DEFLATE decoding, push-based.
//!
//! An [`Inflater`] is fed input in chunks of any size and hands decoded bytes
//! to the caller's output as it goes, so it serves a `Read` adapter, a
//! JavaScript stream callback, or a one-shot [`decompress`] alike. It never
//! blocks and never asks for more than it is given: [`Inflater::feed`] reports
//! how much input it consumed, how much output it wrote, and whether it wants
//! more input, more output room, or has reached the end of the stream.
//!
//! Stored, fixed-Huffman and dynamic-Huffman blocks are decoded (§3.2.4–
//! §3.2.7). Refusals are typed ([`Error`]); an error is sticky.

use crate::backend::{Backend, Kernels};
use crate::error::{Alphabet, Error};
use crate::huffman::{
    DecodeTable, Kind, TAG_BAD_SYMBOL, TAG_DISTANCE, TAG_END, TAG_LENGTH, TAG_LINK, TAG_LITERAL,
    TAG_PAIR, entry, entry_aux, entry_bits, entry_tag, entry_value,
};
use crate::kernels::COPY_SLACK;
use crate::tables::{
    CODE_LENGTH_ORDER, FIXED_DIST_LENGTHS, FIXED_LITLEN_LENGTHS, LITLEN_SYMBOLS, MAX_MATCH, WINDOW,
};

/// Decoded bytes the buffer holds beyond the window before it slides.
const DECODE_ROOM: usize = 3 * WINDOW;
/// The buffer: 32 KiB of history plus the decode room.
const BUF_LEN: usize = WINDOW + DECODE_ROOM;
/// Slide once the free room drops below this and everything is delivered.
const SLIDE_ROOM: usize = 16 * 1024;
/// Primary index widths of the three tables.
const LITLEN_BITS: u32 = 11;
const DIST_BITS: u32 = 8;
const CODE_LENGTH_BITS: u32 = 7;

/// Why [`Inflater::feed`] returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Every input byte was consumed and every decoded byte delivered; feed
    /// more input (or, if there is none, the stream is truncated).
    NeedsInput,
    /// The output is full and decoded bytes are waiting; feed again with
    /// more output room.
    OutputFull,
    /// The final block has ended and every decoded byte was delivered.
    StreamEnd,
}

/// What one [`Inflater::feed`] call did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Input bytes consumed from the front of the input.
    pub consumed: usize,
    /// Output bytes written to the front of the output.
    pub written: usize,
    /// Why the call returned.
    pub status: Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    BlockHeader,
    StoredHeader,
    Stored { remaining: u32 },
    DynamicCounts,
    DynamicCodeLengths,
    DynamicLengths,
    Symbols,
    LengthExtra { base: u32, extra: u32 },
    Distance { length: u32 },
    DistanceExtra { length: u32, base: u32, extra: u32 },
    Copy { length: u32, distance: u32 },
    Done,
}

/// Why the decode step returned.
enum Step {
    NeedsInput,
    NeedsRoom,
    Done,
}

/// Where decoded bytes go.
pub(crate) trait Sink {
    /// Take as many of `bytes` as fit; return how many were taken.
    fn put(&mut self, bytes: &[u8]) -> usize;
    fn written(&self) -> usize;
}

pub(crate) struct SliceSink<'a> {
    pub(crate) out: &'a mut [u8],
    pub(crate) written: usize,
}

impl Sink for SliceSink<'_> {
    fn put(&mut self, bytes: &[u8]) -> usize {
        let n = bytes.len().min(self.out.len() - self.written);
        self.out[self.written..self.written + n].copy_from_slice(&bytes[..n]);
        self.written += n;
        n
    }
    fn written(&self) -> usize {
        self.written
    }
}

pub(crate) struct VecSink<'a> {
    pub(crate) out: &'a mut Vec<u8>,
    pub(crate) written: usize,
}

impl Sink for VecSink<'_> {
    fn put(&mut self, bytes: &[u8]) -> usize {
        self.out.extend_from_slice(bytes);
        self.written += bytes.len();
        bytes.len()
    }
    fn written(&self) -> usize {
        self.written
    }
}

/// A push-based RFC 1951 decoder.
pub struct Inflater {
    state: State,
    final_block: bool,
    bitbuf: u64,
    nbits: u32,
    litlen: DecodeTable,
    dist: DecodeTable,
    code_length: DecodeTable,
    tables_fixed: bool,
    hlit: usize,
    hdist: usize,
    hclen: usize,
    cl_index: usize,
    cl_lengths: [u8; 19],
    lengths: [u8; 320],
    len_index: usize,
    /// History and pending output; `COPY_SLACK` writable bytes past `BUF_LEN`.
    window: Vec<u8>,
    pos: usize,
    delivered: usize,
    /// Bytes the stream produced before `window[0]`.
    base: u64,
    limit: u64,
    kernels: Kernels,
    error: Option<Error>,
    // A 64-bit lookahead buffer retains at most seven whole bytes after
    // consuming the nonempty end-of-block code and byte alignment.
    trailing: [u8; 7],
    trailing_len: usize,
}

impl std::fmt::Debug for Inflater {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Inflater")
            .field("state", &self.state)
            .field("total_out", &self.total_out())
            .field("backend", &self.kernels.backend)
            .finish_non_exhaustive()
    }
}

impl Default for Inflater {
    fn default() -> Self {
        Self::new()
    }
}

impl Inflater {
    /// A decoder on the fastest kernel path this processor supports.
    pub fn new() -> Self {
        Self::with_kernels(Backend::selected_kernels())
    }

    /// A decoder pinned to `backend`, if this processor can run it.
    pub fn with_backend(backend: Backend) -> Option<Self> {
        backend.kernels().map(Self::with_kernels)
    }

    fn with_kernels(kernels: Kernels) -> Self {
        Self {
            state: State::BlockHeader,
            final_block: false,
            bitbuf: 0,
            nbits: 0,
            litlen: DecodeTable::new(LITLEN_BITS),
            dist: DecodeTable::new(DIST_BITS),
            code_length: DecodeTable::new(CODE_LENGTH_BITS),
            tables_fixed: false,
            hlit: 0,
            hdist: 0,
            hclen: 0,
            cl_index: 0,
            cl_lengths: [0; 19],
            lengths: [0; 320],
            len_index: 0,
            window: vec![0; BUF_LEN + COPY_SLACK],
            pos: 0,
            delivered: 0,
            base: 0,
            limit: u64::MAX,
            kernels,
            error: None,
            trailing: [0; 7],
            trailing_len: 0,
        }
    }

    /// Refuse, with [`Error::LimitExceeded`], any stream that decodes to more
    /// than `limit` bytes. Exactly `limit` bytes is accepted.
    pub fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }

    /// Return to the state of a new decoder (keeping the limit and path).
    pub fn reset(&mut self) {
        // The window, the tables (and whether they hold the fixed code) are
        // kept: nothing decoded later reads a window byte before writing it,
        // and every block rebuilds or revalidates its tables.
        self.state = State::BlockHeader;
        self.final_block = false;
        self.bitbuf = 0;
        self.nbits = 0;
        self.pos = 0;
        self.delivered = 0;
        self.base = 0;
        self.error = None;
        self.trailing_len = 0;
    }

    /// Total bytes decoded so far (delivered or not).
    pub fn total_out(&self) -> u64 {
        self.base + self.pos as u64
    }

    /// Whether the final block has ended.
    pub fn is_done(&self) -> bool {
        self.state == State::Done
    }

    /// The kernel path in use.
    pub fn backend(&self) -> Backend {
        self.kernels.backend
    }

    /// Input bytes that were consumed into the bit buffer but lie after the
    /// end of the stream. Empty until [`Self::is_done`].
    pub fn trailing_bytes(&self) -> &[u8] {
        &self.trailing[..self.trailing_len]
    }

    /// Decode from `input` into `output`.
    pub fn feed(&mut self, input: &[u8], output: &mut [u8]) -> Result<Progress, Error> {
        let mut sink = SliceSink {
            out: output,
            written: 0,
        };
        self.feed_sink(input, &mut sink)
    }

    /// Decode from `input`, appending every decoded byte to `output`. Never
    /// returns [`Status::OutputFull`].
    pub fn feed_to_vec(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<Progress, Error> {
        let mut sink = VecSink {
            out: output,
            written: 0,
        };
        self.feed_sink(input, &mut sink)
    }

    pub(crate) fn feed_sink<S: Sink>(
        &mut self,
        input: &[u8],
        sink: &mut S,
    ) -> Result<Progress, Error> {
        let before = sink.written();
        let (consumed, status) = self.run(input, sink)?;
        Ok(Progress {
            consumed,
            written: sink.written() - before,
            status,
        })
    }

    fn run<S: Sink>(&mut self, input: &[u8], sink: &mut S) -> Result<(usize, Status), Error> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let mut ipos = 0;
        loop {
            let taken = sink.put(&self.window[self.delivered..self.pos]);
            self.delivered += taken;
            if self.delivered < self.pos {
                return Ok((ipos, Status::OutputFull));
            }
            if self.state == State::Done {
                return Ok((ipos, Status::StreamEnd));
            }
            if BUF_LEN - self.pos < SLIDE_ROOM {
                self.slide();
            }
            match self.step(input, &mut ipos) {
                Ok(Step::NeedsInput) => {
                    let taken = sink.put(&self.window[self.delivered..self.pos]);
                    self.delivered += taken;
                    let status = if self.delivered < self.pos {
                        Status::OutputFull
                    } else {
                        Status::NeedsInput
                    };
                    return Ok((ipos, status));
                }
                Ok(Step::NeedsRoom | Step::Done) => {}
                Err(error) => {
                    self.error = Some(error.clone());
                    return Err(error);
                }
            }
        }
    }

    /// Keep the last window of history at the front of the buffer.
    fn slide(&mut self) {
        if self.pos <= WINDOW {
            return;
        }
        let shift = self.pos - WINDOW;
        self.window.copy_within(shift..self.pos, 0);
        self.pos = WINDOW;
        self.delivered -= shift;
        self.base += shift as u64;
    }

    /// The end of the room decoding may write into, and whether it is the
    /// limit (not the buffer) that sets it.
    fn out_end(&self) -> (usize, bool) {
        let budget = self.limit.saturating_sub(self.total_out());
        let room = (BUF_LEN - self.pos) as u64;
        if budget < room {
            (self.pos + budget as usize, true)
        } else {
            (BUF_LEN, false)
        }
    }

    /// No room for another byte: either the limit is reached or the buffer
    /// needs to slide.
    fn out_of_room(&self, limited: bool) -> Result<Step, Error> {
        if limited {
            Err(Error::LimitExceeded { limit: self.limit })
        } else {
            Ok(Step::NeedsRoom)
        }
    }

    // --- Careful bit access ------------------------------------------------

    /// Pull whole input bytes until at least `n` bits are buffered.
    #[inline]
    fn ensure(&mut self, n: u32, input: &[u8], ipos: &mut usize) -> bool {
        while self.nbits < n {
            let Some(&byte) = input.get(*ipos) else {
                return false;
            };
            self.bitbuf |= u64::from(byte) << self.nbits;
            self.nbits += 8;
            *ipos += 1;
        }
        true
    }

    #[inline]
    fn bits(&self, n: u32) -> u32 {
        (self.bitbuf & ((1u64 << n) - 1)) as u32
    }

    #[inline]
    fn consume(&mut self, n: u32) {
        self.bitbuf >>= n;
        self.nbits -= n;
    }

    /// Decode the next symbol's table entry without consuming it, pulling
    /// bytes as needed; `None` when the input runs out first. A pair entry
    /// that the buffered bits cannot yet confirm is returned as its first
    /// literal alone.
    fn peek(&mut self, which: Table, input: &[u8], ipos: &mut usize) -> Option<u32> {
        loop {
            let table = match which {
                Table::LiteralLength => &self.litlen,
                Table::Distance => &self.dist,
                Table::CodeLength => &self.code_length,
            };
            let mut e = table.entries[(self.bitbuf & table.primary_mask()) as usize];
            if entry_tag(e) == TAG_LINK && entry_bits(e) <= self.nbits {
                let width = entry_aux(e);
                let index = (self.bitbuf >> table.primary_bits) & ((1u64 << width) - 1);
                e = table.entries[entry_value(e) as usize + index as usize];
            }
            if entry_tag(e) != TAG_LINK && entry_bits(e) <= self.nbits {
                return Some(e);
            }
            if entry_tag(e) == TAG_PAIR && entry_aux(e) <= self.nbits {
                return Some(entry(entry_aux(e), TAG_LITERAL, 0, entry_value(e) & 0xFF));
            }
            if !self.ensure(self.nbits + 8, input, ipos) {
                return None;
            }
        }
    }

    // --- The state machine ---------------------------------------------------

    fn step(&mut self, input: &[u8], ipos: &mut usize) -> Result<Step, Error> {
        loop {
            match self.state {
                State::Done => return Ok(Step::Done),
                State::BlockHeader => {
                    if !self.ensure(3, input, ipos) {
                        return Ok(Step::NeedsInput);
                    }
                    self.final_block = self.bits(1) == 1;
                    let kind = (self.bitbuf >> 1) & 3;
                    self.consume(3);
                    self.state = match kind {
                        0 => State::StoredHeader,
                        1 => {
                            if !self.tables_fixed {
                                self.litlen.build(
                                    &FIXED_LITLEN_LENGTHS,
                                    Kind::LiteralLength,
                                    true,
                                )?;
                                self.dist
                                    .build(&FIXED_DIST_LENGTHS, Kind::Distance, false)?;
                                self.tables_fixed = true;
                            }
                            State::Symbols
                        }
                        2 => State::DynamicCounts,
                        _ => return Err(Error::InvalidBlockType),
                    };
                }
                State::StoredHeader => {
                    let partial = self.nbits % 8;
                    self.consume(partial);
                    if !self.ensure(32, input, ipos) {
                        return Ok(Step::NeedsInput);
                    }
                    let len = self.bits(16) as u16;
                    let nlen = (self.bitbuf >> 16) as u16;
                    self.consume(32);
                    if len != !nlen {
                        return Err(Error::StoredLengthMismatch { len, nlen });
                    }
                    self.state = State::Stored {
                        remaining: u32::from(len),
                    };
                }
                State::Stored { mut remaining } => {
                    let (end, limited) = self.out_end();
                    // Whole bytes still in the bit buffer come first.
                    while remaining > 0 && self.nbits >= 8 && self.pos < end {
                        self.window[self.pos] = self.bits(8) as u8;
                        self.consume(8);
                        self.pos += 1;
                        remaining -= 1;
                    }
                    if self.nbits < 8 {
                        let n = (remaining as usize)
                            .min(input.len() - *ipos)
                            .min(end - self.pos);
                        self.window[self.pos..self.pos + n]
                            .copy_from_slice(&input[*ipos..*ipos + n]);
                        self.pos += n;
                        *ipos += n;
                        remaining -= n as u32;
                    }
                    if remaining == 0 {
                        self.state = self.after_block();
                        continue;
                    }
                    self.state = State::Stored { remaining };
                    if self.pos == end {
                        return self.out_of_room(limited);
                    }
                    return Ok(Step::NeedsInput);
                }
                State::DynamicCounts => {
                    if !self.ensure(14, input, ipos) {
                        return Ok(Step::NeedsInput);
                    }
                    self.hlit = 257 + self.bits(5) as usize;
                    self.hdist = 1 + ((self.bitbuf >> 5) & 31) as usize;
                    self.hclen = 4 + ((self.bitbuf >> 10) & 15) as usize;
                    self.consume(14);
                    if self.hlit > LITLEN_SYMBOLS {
                        return Err(Error::TooManyLengthCodes {
                            count: self.hlit as u16,
                        });
                    }
                    self.cl_lengths = [0; 19];
                    self.cl_index = 0;
                    self.state = State::DynamicCodeLengths;
                }
                State::DynamicCodeLengths => {
                    while self.cl_index < self.hclen {
                        if !self.ensure(3, input, ipos) {
                            return Ok(Step::NeedsInput);
                        }
                        self.cl_lengths[CODE_LENGTH_ORDER[self.cl_index]] = self.bits(3) as u8;
                        self.consume(3);
                        self.cl_index += 1;
                    }
                    let lengths = self.cl_lengths;
                    self.code_length.build(&lengths, Kind::CodeLength, false)?;
                    self.len_index = 0;
                    self.state = State::DynamicLengths;
                }
                State::DynamicLengths => {
                    let total = self.hlit + self.hdist;
                    while self.len_index < total {
                        let Some(e) = self.peek(Table::CodeLength, input, ipos) else {
                            return Ok(Step::NeedsInput);
                        };
                        if entry_tag(e) != TAG_LITERAL {
                            return Err(Error::InvalidCode {
                                alphabet: Alphabet::CodeLength,
                            });
                        }
                        let symbol = entry_value(e);
                        let code_bits = entry_bits(e);
                        let extra = match symbol {
                            16 => 2,
                            17 => 3,
                            18 => 7,
                            _ => 0,
                        };
                        if !self.ensure(code_bits + extra, input, ipos) {
                            return Ok(Step::NeedsInput);
                        }
                        self.consume(code_bits);
                        let value = self.bits(extra) as usize;
                        self.consume(extra);
                        let (fill, count) = match symbol {
                            0..=15 => (symbol as u8, 1),
                            16 => {
                                if self.len_index == 0 {
                                    return Err(Error::RepeatWithoutPrevious);
                                }
                                (self.lengths[self.len_index - 1], 3 + value)
                            }
                            17 => (0, 3 + value),
                            _ => (0, 11 + value),
                        };
                        if self.len_index + count > total {
                            return Err(Error::CodeLengthsOverrun);
                        }
                        self.lengths[self.len_index..self.len_index + count].fill(fill);
                        self.len_index += count;
                    }
                    if self.lengths[256] == 0 {
                        return Err(Error::MissingEndOfBlock);
                    }
                    let lengths = self.lengths;
                    self.litlen
                        .build(&lengths[..self.hlit], Kind::LiteralLength, true)?;
                    self.dist
                        .build(&lengths[self.hlit..total], Kind::Distance, false)?;
                    self.tables_fixed = false;
                    self.state = State::Symbols;
                }
                State::Symbols => {
                    self.fast_symbols(input, ipos)?;
                    if self.state != State::Symbols {
                        continue;
                    }
                    let (end, limited) = self.out_end();
                    let Some(e) = self.peek(Table::LiteralLength, input, ipos) else {
                        return Ok(Step::NeedsInput);
                    };
                    match entry_tag(e) {
                        TAG_LITERAL => {
                            if self.pos >= end {
                                return self.out_of_room(limited);
                            }
                            self.window[self.pos] = entry_value(e) as u8;
                            self.pos += 1;
                            self.consume(entry_bits(e));
                        }
                        TAG_PAIR => {
                            if self.pos + 2 > end {
                                // One literal at a time at the edge of the room.
                                if self.pos >= end {
                                    return self.out_of_room(limited);
                                }
                                self.window[self.pos] = entry_value(e) as u8;
                                self.pos += 1;
                                self.consume(entry_aux(e));
                            } else {
                                let v = entry_value(e);
                                self.window[self.pos] = v as u8;
                                self.window[self.pos + 1] = (v >> 8) as u8;
                                self.pos += 2;
                                self.consume(entry_bits(e));
                            }
                        }
                        TAG_LENGTH => {
                            self.consume(entry_bits(e));
                            self.state = State::LengthExtra {
                                base: entry_value(e),
                                extra: entry_aux(e),
                            };
                        }
                        TAG_END => {
                            self.consume(entry_bits(e));
                            self.state = self.after_block();
                        }
                        TAG_BAD_SYMBOL => {
                            return Err(Error::InvalidLengthSymbol {
                                symbol: entry_value(e) as u16,
                            });
                        }
                        _ => {
                            return Err(Error::InvalidCode {
                                alphabet: Alphabet::LiteralLength,
                            });
                        }
                    }
                }
                State::LengthExtra { base, extra } => {
                    if !self.ensure(extra, input, ipos) {
                        return Ok(Step::NeedsInput);
                    }
                    let length = base + self.bits(extra);
                    self.consume(extra);
                    self.state = State::Distance { length };
                }
                State::Distance { length } => {
                    let Some(e) = self.peek(Table::Distance, input, ipos) else {
                        return Ok(Step::NeedsInput);
                    };
                    self.check_distance_entry(e)?;
                    self.consume(entry_bits(e));
                    self.state = State::DistanceExtra {
                        length,
                        base: entry_value(e),
                        extra: entry_aux(e),
                    };
                }
                State::DistanceExtra {
                    length,
                    base,
                    extra,
                } => {
                    if !self.ensure(extra, input, ipos) {
                        return Ok(Step::NeedsInput);
                    }
                    let distance = base + self.bits(extra);
                    self.consume(extra);
                    self.check_distance(distance)?;
                    self.state = State::Copy { length, distance };
                }
                State::Copy { length, distance } => {
                    let (end, limited) = self.out_end();
                    let n = (length as usize).min(end - self.pos);
                    if n > 0 {
                        (self.kernels.copy)(&mut self.window, self.pos, distance as usize, n);
                        self.pos += n;
                    }
                    let left = length - n as u32;
                    if left == 0 {
                        self.state = State::Symbols;
                    } else {
                        self.state = State::Copy {
                            length: left,
                            distance,
                        };
                        return self.out_of_room(limited);
                    }
                }
            }
        }
    }

    fn after_block(&mut self) -> State {
        if !self.final_block {
            return State::BlockHeader;
        }
        // Whole bytes left in the bit buffer belong to whatever follows.
        let partial = self.nbits % 8;
        self.consume(partial);
        self.trailing_len = 0;
        while self.nbits >= 8 {
            self.trailing[self.trailing_len] = self.bits(8) as u8;
            self.trailing_len += 1;
            self.consume(8);
        }
        self.bitbuf = 0;
        State::Done
    }

    #[inline]
    fn check_distance_entry(&self, e: u32) -> Result<(), Error> {
        match entry_tag(e) {
            TAG_DISTANCE => Ok(()),
            TAG_BAD_SYMBOL => Err(Error::InvalidDistanceSymbol {
                symbol: entry_value(e) as u16,
            }),
            _ => Err(Error::InvalidCode {
                alphabet: Alphabet::Distance,
            }),
        }
    }

    #[inline]
    fn check_distance(&self, distance: u32) -> Result<(), Error> {
        let available = self.total_out();
        if u64::from(distance) > available {
            return Err(Error::DistanceTooFar {
                distance,
                available,
            });
        }
        Ok(())
    }

    /// The fast symbol loop: runs while at least 8 input bytes and a whole
    /// longest match of output room remain, refilling the bit buffer with one
    /// unaligned 64-bit load per symbol so no bit-availability check is needed.
    fn fast_symbols(&mut self, input: &[u8], ipos: &mut usize) -> Result<(), Error> {
        let (end, _) = self.out_end();
        if end < MAX_MATCH + 2 {
            return Ok(());
        }
        let fast_end = end - (MAX_MATCH + 2);
        let mut bitbuf = self.bitbuf;
        let mut nbits = self.nbits;
        let mut pos = self.pos;
        let mut ip = *ipos;
        let lmask = self.litlen.primary_mask();
        let dmask = self.dist.primary_mask();
        let result = loop {
            if ip + 8 > input.len() || pos > fast_end {
                break Ok(());
            }
            let chunk = u64::from_le_bytes(input[ip..ip + 8].try_into().expect("eight bytes"));
            bitbuf |= chunk << nbits;
            let whole = (63 - nbits) >> 3;
            ip += whole as usize;
            nbits += whole << 3;

            let mut e = self.litlen.entries[(bitbuf & lmask) as usize];
            if entry_tag(e) == TAG_LINK {
                let index = (bitbuf >> LITLEN_BITS) & ((1u64 << entry_aux(e)) - 1);
                e = self.litlen.entries[entry_value(e) as usize + index as usize];
            }
            match entry_tag(e) {
                TAG_LITERAL => {
                    self.window[pos] = entry_value(e) as u8;
                    pos += 1;
                }
                TAG_PAIR => {
                    let v = entry_value(e);
                    self.window[pos] = v as u8;
                    self.window[pos + 1] = (v >> 8) as u8;
                    pos += 2;
                }
                TAG_LENGTH => {
                    let n = entry_bits(e);
                    bitbuf >>= n;
                    nbits -= n;
                    let extra = entry_aux(e);
                    let length = entry_value(e) + (bitbuf & ((1u64 << extra) - 1)) as u32;
                    bitbuf >>= extra;
                    nbits -= extra;
                    let mut d = self.dist.entries[(bitbuf & dmask) as usize];
                    if entry_tag(d) == TAG_LINK {
                        let index = (bitbuf >> DIST_BITS) & ((1u64 << entry_aux(d)) - 1);
                        d = self.dist.entries[entry_value(d) as usize + index as usize];
                    }
                    if let Err(error) = self.check_distance_entry(d) {
                        break Err(error);
                    }
                    let n = entry_bits(d);
                    bitbuf >>= n;
                    nbits -= n;
                    let extra = entry_aux(d);
                    let distance = entry_value(d) + (bitbuf & ((1u64 << extra) - 1)) as u32;
                    bitbuf >>= extra;
                    nbits -= extra;
                    if u64::from(distance) > self.base + pos as u64 {
                        break Err(Error::DistanceTooFar {
                            distance,
                            available: self.base + pos as u64,
                        });
                    }
                    (self.kernels.copy)(&mut self.window, pos, distance as usize, length as usize);
                    pos += length as usize;
                    continue;
                }
                TAG_END => {
                    let n = entry_bits(e);
                    bitbuf >>= n;
                    nbits -= n;
                    self.bitbuf = bitbuf;
                    self.nbits = nbits;
                    self.state = self.after_block_fast();
                    break Ok(());
                }
                TAG_BAD_SYMBOL => {
                    break Err(Error::InvalidLengthSymbol {
                        symbol: entry_value(e) as u16,
                    });
                }
                _ => {
                    break Err(Error::InvalidCode {
                        alphabet: Alphabet::LiteralLength,
                    });
                }
            }
            let n = entry_bits(e);
            bitbuf >>= n;
            nbits -= n;
        };
        if self.state == State::Symbols {
            self.bitbuf = bitbuf;
            self.nbits = nbits;
        }
        // Bits above `nbits` are a copy of the next unconsumed input byte;
        // the careful path requires them clear.
        if self.nbits < 64 {
            self.bitbuf &= (1u64 << self.nbits) - 1;
        }
        self.pos = pos;
        *ipos = ip;
        result
    }

    /// `after_block` for the fast loop, whose bit buffer may hold bits above
    /// `nbits`.
    fn after_block_fast(&mut self) -> State {
        if self.nbits < 64 {
            self.bitbuf &= (1u64 << self.nbits) - 1;
        }
        self.after_block()
    }
}

#[derive(Clone, Copy)]
enum Table {
    LiteralLength,
    Distance,
    CodeLength,
}

/// Decode a whole raw DEFLATE stream. Bytes after the final block are
/// refused ([`Error::TrailingData`]).
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    decompress_with_limit(data, u64::MAX)
}

/// [`decompress`], refusing output beyond `limit` bytes.
pub fn decompress_with_limit(data: &[u8], limit: u64) -> Result<Vec<u8>, Error> {
    let mut inflater = Inflater::new();
    inflater.set_limit(limit);
    let mut out = Vec::new();
    let progress = inflater.feed_to_vec(data, &mut out)?;
    if progress.status != Status::StreamEnd {
        return Err(Error::Truncated);
    }
    if progress.consumed != data.len() || !inflater.trailing_bytes().is_empty() {
        return Err(Error::TrailingData);
    }
    Ok(out)
}
