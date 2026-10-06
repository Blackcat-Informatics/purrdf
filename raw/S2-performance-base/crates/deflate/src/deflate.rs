// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 1951 DEFLATE encoding, deterministic.
//!
//! The matcher follows the shape RFC 1951 §4 describes in prose: hash chains
//! over the input's short windows, newest candidates first, a chain length
//! capped per level, and lazy (deferred) match selection. The block layer is
//! this crate's own: symbol counts are checked every 4096 tokens to decide
//! whether starting a new block (with new codes) pays for its header, and each
//! block is emitted as stored, fixed or dynamic, whichever is exactly smallest.
//!
//! The output depends on the input bytes, level, and explicit sync-flush
//! boundaries — never on the clock, randomness, kernel path, or how ordinary
//! input was split across [`Deflater::write`] calls.

use crate::backend::{Backend, Kernels};
use crate::huffman::{canonical_codes, code_lengths};
use crate::kernels::{HASH_BITS, hash4};
use crate::tables::{
    CODE_LENGTH_ORDER, DIST_BASE, DIST_EXTRA, DIST_SYMBOLS, END_OF_BLOCK, FIXED_DIST_CODES,
    FIXED_DIST_LENGTHS, FIXED_LITLEN_CODES, FIXED_LITLEN_LENGTHS, LENGTH_BASE, LENGTH_CODE,
    LENGTH_EXTRA, LITLEN_SYMBOLS, MAX_CL_CODE_LEN, MAX_CODE_LEN, MAX_MATCH, MIN_MATCH, WINDOW,
    distance_code,
};

/// A compression level, 0 (stored, no compression) to 9 (slowest, densest).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Level(u8);

impl Level {
    /// Stored blocks only.
    pub const NONE: Self = Self(0);
    /// Greedy matching on short chains.
    pub const FASTEST: Self = Self(1);
    /// The default: lazy matching on moderate chains.
    pub const DEFAULT: Self = Self(6);
    /// Lazy matching on long chains.
    pub const BEST: Self = Self(9);

    /// The level `level`, if it is 0–9.
    pub const fn new(level: u8) -> Option<Self> {
        if level <= 9 { Some(Self(level)) } else { None }
    }

    /// The level as a number.
    pub const fn get(self) -> u8 {
        self.0
    }
}

impl Default for Level {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Clone, Copy, Debug)]
struct Params {
    store_only: bool,
    /// Candidates examined per search.
    max_chain: u32,
    /// A pending match at least this long searches a quarter of the chain.
    good: u32,
    /// Lazy evaluation is tried only for matches shorter than this; 0 is
    /// greedy.
    lazy: u32,
    /// A match this long ends the search.
    nice: u32,
    /// Greedy levels insert only this many positions of a longer match.
    insert_limit: u32,
}

const fn params(level: u8) -> Params {
    let (max_chain, good, lazy, nice, insert_limit) = match level {
        0 => (0, 0, 0, 0, 0),
        1 => (4, 4, 0, 8, 4),
        2 => (8, 4, 0, 16, 8),
        3 => (16, 8, 0, 32, 16),
        4 => (16, 4, 4, 16, u32::MAX),
        5 => (32, 8, 16, 32, u32::MAX),
        6 => (128, 8, 16, 128, u32::MAX),
        7 => (256, 16, 32, 128, u32::MAX),
        8 => (1024, 32, 128, 258, u32::MAX),
        _ => (4096, 32, 258, 258, u32::MAX),
    };
    Params {
        store_only: level == 0,
        max_chain,
        good,
        lazy,
        nice,
        insert_limit,
    }
}

/// The hashed window: matches are at least this long.
const MIN_HASHED: usize = 4;
/// Lookahead the matcher requires before it decides a position, unless the
/// input is finished: a longest match plus a hashed window.
const MIN_LOOKAHEAD: usize = MAX_MATCH + MIN_HASHED;
/// The window buffer: two windows plus room for the lookahead to run past the
/// slide point.
const SLIDE_AT: usize = 2 * WINDOW;
const BUF_CAP: usize = SLIDE_AT + 2 * MIN_LOOKAHEAD;
const HASH_SIZE: usize = 1 << HASH_BITS;
const WMASK: usize = WINDOW - 1;
/// Tokens between block-split checks, and the most a block holds.
const CHECK_INTERVAL: usize = 4096;
const MAX_BLOCK_TOKENS: usize = 8 * CHECK_INTERVAL;
/// Two blocks must beat one by at least this many bits to split.
const SPLIT_MARGIN: u64 = 64;

const MATCH_FLAG: u32 = 1 << 31;
/// Batches shorter than this are hashed inline; longer ones go through the
/// vector hash kernel.
const SCALAR_INSERT: usize = 8;
/// Each run encodes at least one length, so its count cannot exceed this.
const MAX_LENGTH_RUNS: usize = LITLEN_SYMBOLS + DIST_SYMBOLS;

// --- Bit writer ------------------------------------------------------------

#[derive(Debug, Default)]
struct BitWriter {
    acc: u64,
    n: u32,
    out: Vec<u8>,
}

impl BitWriter {
    #[inline]
    fn write(&mut self, bits: u32, count: u32) {
        debug_assert!(count <= 16 && u64::from(bits) >> count == 0);
        self.acc |= u64::from(bits) << self.n;
        self.n += count;
        if self.n >= 32 {
            self.out.extend_from_slice(&(self.acc as u32).to_le_bytes());
            self.acc >>= 32;
            self.n -= 32;
        }
    }

    /// Pad with zero bits to a byte boundary and emit every whole byte.
    fn align(&mut self) {
        while self.n > 0 {
            self.out.push(self.acc as u8);
            self.acc >>= 8;
            self.n = self.n.saturating_sub(8);
        }
        self.acc = 0;
    }

    /// Bits written so far, modulo 8.
    const fn bit_phase(&self) -> u32 {
        self.n % 8
    }
}

// --- Block planning --------------------------------------------------------

/// A dynamic block's codes and their exact header cost.
struct DynamicPlan {
    litlen: [u8; LITLEN_SYMBOLS],
    dist: [u8; DIST_SYMBOLS],
    code_length: [u8; 19],
    /// The run-length coded lengths: (code-length symbol, extra value).
    runs: [(u8, u8); MAX_LENGTH_RUNS],
    n_runs: usize,
    n_lit: usize,
    n_dist: usize,
    n_cl: usize,
    header_bits: u64,
}

impl DynamicPlan {
    fn new(litlen_freq: &[u32; LITLEN_SYMBOLS], dist_freq: &[u32; DIST_SYMBOLS]) -> Self {
        let mut litlen = [0u8; LITLEN_SYMBOLS];
        let mut dist = [0u8; DIST_SYMBOLS];
        code_lengths(litlen_freq, MAX_CODE_LEN, &mut litlen);
        code_lengths(dist_freq, MAX_CODE_LEN, &mut dist);
        let n_lit = 257.max(litlen.iter().rposition(|&l| l > 0).map_or(0, |i| i + 1));
        let n_dist = 1.max(dist.iter().rposition(|&l| l > 0).map_or(0, |i| i + 1));

        let mut runs = [(0u8, 0u8); MAX_LENGTH_RUNS];
        let mut n_runs = 0;
        let mut push_run = |run| {
            runs[n_runs] = run;
            n_runs += 1;
        };
        let mut cl_freq = [0u32; 19];
        let mut sequence = [0u8; MAX_LENGTH_RUNS];
        sequence[..n_lit].copy_from_slice(&litlen[..n_lit]);
        sequence[n_lit..n_lit + n_dist].copy_from_slice(&dist[..n_dist]);
        let sequence = &sequence[..n_lit + n_dist];
        let mut i = 0;
        while i < sequence.len() {
            let value = sequence[i];
            let mut run = sequence[i..].iter().take_while(|&&v| v == value).count();
            i += run;
            if value == 0 {
                while run >= 11 {
                    let n = run.min(138);
                    push_run((18, (n - 11) as u8));
                    run -= n;
                }
                if run >= 3 {
                    push_run((17, (run - 3) as u8));
                    run = 0;
                }
            } else {
                push_run((value, 0));
                run -= 1;
                while run >= 3 {
                    let n = run.min(6);
                    push_run((16, (n - 3) as u8));
                    run -= n;
                }
            }
            for _ in 0..run {
                push_run((value, 0));
            }
        }
        for &(symbol, _) in &runs[..n_runs] {
            cl_freq[usize::from(symbol)] += 1;
        }
        let mut code_length = [0u8; 19];
        code_lengths(&cl_freq, MAX_CL_CODE_LEN, &mut code_length);
        let n_cl = 4.max(
            CODE_LENGTH_ORDER
                .iter()
                .rposition(|&s| code_length[s] > 0)
                .map_or(0, |i| i + 1),
        );
        let mut header_bits = 3 + 5 + 5 + 4 + 3 * n_cl as u64;
        for &(symbol, _) in &runs[..n_runs] {
            header_bits += u64::from(code_length[usize::from(symbol)]);
            header_bits += match symbol {
                16 => 2,
                17 => 3,
                18 => 7,
                _ => 0,
            };
        }
        Self {
            litlen,
            dist,
            code_length,
            runs,
            n_runs,
            n_lit,
            n_dist,
            n_cl,
            header_bits,
        }
    }

    /// Header plus coded symbols, extra bits excluded.
    fn coded_bits(&self, litlen_freq: &[u32], dist_freq: &[u32]) -> u64 {
        self.header_bits
            + symbol_bits(litlen_freq, &self.litlen)
            + symbol_bits(dist_freq, &self.dist)
    }
}

fn symbol_bits(freq: &[u32], lengths: &[u8]) -> u64 {
    freq.iter()
        .zip(lengths)
        .map(|(&f, &l)| u64::from(f) * u64::from(l))
        .sum()
}

fn extra_bits(litlen_freq: &[u32], dist_freq: &[u32]) -> u64 {
    let lengths: u64 = litlen_freq[257..]
        .iter()
        .zip(LENGTH_EXTRA)
        .map(|(&f, e)| u64::from(f) * u64::from(e))
        .sum();
    let dists: u64 = dist_freq
        .iter()
        .zip(DIST_EXTRA)
        .map(|(&f, e)| u64::from(f) * u64::from(e))
        .sum();
    lengths + dists
}

/// `log2(x)` in sixteenths, for `x >= 1`: the integer part from the highest
/// set bit, the fraction linearly from the next four bits. Integer-only, so
/// every target computes the same value.
const fn log2_sixteenths(x: u32) -> u64 {
    let k = x.ilog2();
    let frac = if k >= 4 {
        (x >> (k - 4)) & 15
    } else {
        (x << (4 - k)) & 15
    };
    (k * 16 + frac) as u64
}

/// Estimated coded bits of one alphabet's counts under an ideal code (each
/// symbol at least one bit), in sixteenths of a bit, and the used-symbol
/// count.
fn entropy_sixteenths(freq: &[u32]) -> (u64, u64) {
    let total: u32 = freq.iter().sum();
    if total == 0 {
        return (0, 0);
    }
    let whole = log2_sixteenths(total);
    let mut bits = 0u64;
    let mut used = 0u64;
    for &f in freq {
        if f > 0 {
            bits += u64::from(f) * whole.saturating_sub(log2_sixteenths(f)).max(16);
            used += 1;
        }
    }
    (bits, used)
}

/// Estimated bits of `litlen`/`dist` counts as one dynamic block: the ideal
/// code length of every symbol plus about four header bits per used symbol.
/// Only compares two ways of cutting the same tokens, so it need not be
/// exact — it must be cheap, since it runs every `CHECK_INTERVAL` tokens.
fn dynamic_estimate(litlen: &[u32; LITLEN_SYMBOLS], dist: &[u32; DIST_SYMBOLS]) -> u64 {
    let (lit_bits, lit_used) = entropy_sixteenths(litlen);
    let (dist_bits, dist_used) = entropy_sixteenths(dist);
    // The end-of-block symbol (one occurrence) and the fixed header fields.
    (lit_bits + dist_bits) / 16 + 4 * (lit_used + dist_used + 1) + 64
}

// --- The encoder -----------------------------------------------------------

/// A streaming, deterministic RFC 1951 encoder.
pub struct Deflater {
    params: Params,
    kernels: Kernels,
    win: Vec<u8>,
    end: usize,
    pos: usize,
    /// Positions below this have been entered into the hash chains.
    inserted: usize,
    head: Vec<u32>,
    prev: Vec<u32>,
    /// A match found at `pos − 1`, awaiting the lazy comparison at `pos`.
    pending: Option<(u32, u32)>,
    /// Window index of the first byte of the open block, and of the first
    /// byte not yet covered by a token.
    block_start: usize,
    covered: usize,
    tokens: Vec<u32>,
    litlen_freq: [u32; LITLEN_SYMBOLS],
    dist_freq: [u32; DIST_SYMBOLS],
    /// Counts, token index and window index since the last split check.
    seg_litlen: [u32; LITLEN_SYMBOLS],
    seg_dist: [u32; DIST_SYMBOLS],
    seg_token: usize,
    seg_pos: usize,
    writer: BitWriter,
    hash_scratch: Vec<u32>,
    total_in: u64,
    finished: bool,
}

impl std::fmt::Debug for Deflater {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Deflater")
            .field("total_in", &self.total_in)
            .field("backend", &self.kernels.backend)
            .field("finished", &self.finished)
            .finish_non_exhaustive()
    }
}

impl Deflater {
    /// An encoder at `level` on the fastest kernel path this processor
    /// supports. Every path produces the same bytes.
    pub fn new(level: Level) -> Self {
        Self::with_kernels(level, Backend::selected_kernels())
    }

    /// An encoder pinned to `backend`, if this processor can run it.
    pub fn with_backend(level: Level, backend: Backend) -> Option<Self> {
        backend.kernels().map(|k| Self::with_kernels(level, k))
    }

    fn with_kernels(level: Level, kernels: Kernels) -> Self {
        let params = params(level.0);
        Self {
            params,
            kernels,
            win: Vec::with_capacity(BUF_CAP),
            end: 0,
            pos: 0,
            inserted: 0,
            head: if params.store_only {
                Vec::new()
            } else {
                vec![0; HASH_SIZE]
            },
            prev: if params.store_only {
                Vec::new()
            } else {
                Vec::with_capacity(WINDOW)
            },
            pending: None,
            block_start: 0,
            covered: 0,
            tokens: Vec::with_capacity(CHECK_INTERVAL + 1),
            litlen_freq: [0; LITLEN_SYMBOLS],
            dist_freq: [0; DIST_SYMBOLS],
            seg_litlen: [0; LITLEN_SYMBOLS],
            seg_dist: [0; DIST_SYMBOLS],
            seg_token: 0,
            seg_pos: 0,
            writer: BitWriter::default(),
            hash_scratch: Vec::with_capacity(MAX_MATCH),
            total_in: 0,
            finished: false,
        }
    }

    /// The kernel path in use.
    pub fn backend(&self) -> Backend {
        self.kernels.backend
    }

    /// Input bytes accepted so far.
    pub fn total_in(&self) -> u64 {
        self.total_in
    }

    /// Compress `input`, appending any completed output to `out`.
    ///
    /// # Panics
    ///
    /// After [`Self::finish`].
    pub fn write(&mut self, mut input: &[u8], out: &mut Vec<u8>) {
        assert!(!self.finished, "write after finish");
        self.total_in += input.len() as u64;
        while !input.is_empty() {
            let n = input.len().min(BUF_CAP - self.end);
            // Short streams initialize only the window and chain slots they
            // can reach. Once grown, storage is reused across window slides.
            if self.win.len() < self.end + n {
                self.win.resize(self.end + n, 0);
            }
            let chain_len = (self.end + n).min(WINDOW);
            if !self.params.store_only && self.prev.len() < chain_len {
                self.prev.resize(chain_len, 0);
            }
            self.win[self.end..self.end + n].copy_from_slice(&input[..n]);
            self.end += n;
            input = &input[n..];
            self.compress(false);
        }
        out.append(&mut self.writer.out);
    }

    /// End the stream: compress what is buffered, emit the final block, and
    /// append the remaining output to `out`. A second call does nothing.
    pub fn finish(&mut self, out: &mut Vec<u8>) {
        if !self.finished {
            self.compress(true);
            if let Some((len, dist)) = self.pending.take() {
                self.emit_match(len, dist);
            }
            self.flush_block(self.tokens.len(), true);
            self.writer.align();
            self.finished = true;
        }
        out.append(&mut self.writer.out);
    }

    /// Finish every accepted input byte without ending the stream. The empty
    /// stored block aligns the stream and makes it decodable through this
    /// boundary. Match history remains available to later writes.
    pub fn sync_flush(&mut self, out: &mut Vec<u8>) {
        assert!(!self.finished, "flush after finish");
        self.compress(true);
        if let Some((len, dist)) = self.pending.take() {
            self.emit_match(len, dist);
        }
        self.flush_block(self.tokens.len(), false);
        self.write_stored(self.block_start, 0, false);
        out.append(&mut self.writer.out);
    }

    // --- Matching ----------------------------------------------------------

    fn compress(&mut self, finishing: bool) {
        loop {
            if self.pos >= SLIDE_AT {
                self.slide();
            }
            if self.pos >= self.end || (!finishing && self.end - self.pos < MIN_LOOKAHEAD) {
                return;
            }
            if self.params.store_only {
                let byte = self.win[self.pos];
                self.emit_literal(byte);
                self.pos += 1;
                continue;
            }
            let p = self.pos;
            if let Some((prev_len, prev_dist)) = self.pending {
                let (len, dist) = if prev_len < self.params.lazy {
                    self.search(p, prev_len)
                } else {
                    self.insert_through(p + 1);
                    (0, 0)
                };
                if len > prev_len {
                    let literal = self.win[p - 1];
                    self.emit_literal(literal);
                    self.pending = Some((len, dist));
                    self.pos = p + 1;
                } else {
                    self.pending = None;
                    self.emit_match(prev_len, prev_dist);
                    let next = p - 1 + prev_len as usize;
                    self.insert_through(next);
                    self.pos = next;
                }
                continue;
            }
            let (len, dist) = self.search(p, 0);
            if len == 0 {
                let literal = self.win[p];
                self.emit_literal(literal);
                self.pos = p + 1;
            } else if len >= self.params.lazy {
                self.emit_match(len, dist);
                let next = p + len as usize;
                if len > self.params.insert_limit {
                    self.inserted = self.inserted.max(next);
                } else {
                    self.insert_through(next);
                }
                self.pos = next;
            } else {
                self.pending = Some((len, dist));
                self.pos = p + 1;
            }
        }
    }

    /// Enter every position below `limit` that has a whole hashed window
    /// into the chains.
    fn insert_through(&mut self, limit: usize) {
        let limit = limit.min((self.end + 1).saturating_sub(MIN_HASHED));
        if self.inserted >= limit {
            return;
        }
        let count = limit - self.inserted;
        if count < SCALAR_INSERT {
            for q in self.inserted..limit {
                let window = u32::from_le_bytes(*self.win[q..].first_chunk().expect("four bytes"));
                self.insert_one(q, hash4(window) as usize);
            }
            return;
        }
        let mut scratch = std::mem::take(&mut self.hash_scratch);
        scratch.clear();
        scratch.resize(count, 0);
        (self.kernels.hash)(&self.win[..self.end], self.inserted, &mut scratch);
        for (offset, &h) in scratch.iter().enumerate() {
            let q = self.inserted + offset;
            let h = h as usize;
            self.prev[q & WMASK] = self.head[h];
            self.head[h] = q as u32 + 1;
        }
        self.inserted = limit;
        self.hash_scratch = scratch;
    }

    /// Enter position `q` (the next uninserted one) with hash `h`.
    #[inline]
    fn insert_one(&mut self, q: usize, h: usize) {
        debug_assert_eq!(q, self.inserted);
        self.prev[q & WMASK] = self.head[h];
        self.head[h] = q as u32 + 1;
        self.inserted = q + 1;
    }

    /// The longest match at `p` longer than `shorter_than_this`, as
    /// `(length, distance)`, or `(0, 0)`; enters `p` into the chains.
    fn search(&mut self, p: usize, shorter_than_this: u32) -> (u32, u32) {
        let max = MAX_MATCH.min(self.end - p);
        if max < MIN_HASHED {
            return (0, 0);
        }
        // Positions before `p` a lazy step skipped are entered first.
        if self.inserted < p {
            self.insert_through(p);
        }
        let window = u32::from_le_bytes(*self.win[p..].first_chunk().expect("four bytes"));
        let h = hash4(window) as usize;
        let mut candidate = self.head[h] as usize;
        let oldest = p.saturating_sub(WINDOW);
        // Most positions in an incompressible stream have no live hash-chain
        // predecessor. Avoid setting up the bounded match walk for those bytes.
        if candidate == 0 || candidate - 1 < oldest {
            if self.inserted == p {
                self.insert_one(p, h);
            }
            return (0, 0);
        }
        let mut chain = self.params.max_chain;
        if shorter_than_this >= self.params.good {
            chain = (chain / 4).max(1);
        }
        let nice = (self.params.nice as usize).min(max);
        let mut best_len = (shorter_than_this as usize).max(MIN_MATCH);
        let mut best_dist = 0usize;
        while candidate != 0 && chain > 0 {
            let c = candidate - 1;
            if c < oldest {
                break;
            }
            if c >= p {
                // `p` itself, entered earlier by a skipped lazy step.
                candidate = self.prev[c & WMASK] as usize;
                chain -= 1;
                continue;
            }
            if best_len < max
                && self.win[c + best_len] == self.win[p + best_len]
                && self.win[c] == self.win[p]
            {
                let len = (self.kernels.match_length)(&self.win[c..c + max], &self.win[p..p + max]);
                if len > best_len {
                    best_len = len;
                    best_dist = p - c;
                    if len >= nice {
                        break;
                    }
                }
            }
            candidate = self.prev[c & WMASK] as usize;
            chain -= 1;
        }
        // Enter `p` after the walk: during it, `prev[p & WMASK]` still links
        // the position one window back, which the walk may visit.
        if self.inserted == p {
            self.insert_one(p, h);
        }
        if best_dist == 0 {
            (0, 0)
        } else {
            (best_len as u32, best_dist as u32)
        }
    }

    /// Discard the older window: flush the block if its bytes would go, move
    /// the newer half down, and rebase the chains.
    fn slide(&mut self) {
        if self.block_start < WINDOW {
            self.flush_block(self.tokens.len(), false);
        }
        self.win.copy_within(WINDOW..self.end, 0);
        self.end -= WINDOW;
        self.pos -= WINDOW;
        self.block_start -= WINDOW;
        self.covered -= WINDOW;
        self.seg_pos -= WINDOW;
        self.inserted = self.inserted.saturating_sub(WINDOW);
        let shift = WINDOW as u32;
        for slot in &mut self.head {
            *slot = slot.saturating_sub(shift);
        }
        for slot in &mut self.prev {
            *slot = slot.saturating_sub(shift);
        }
    }

    // --- Tokens and blocks -------------------------------------------------

    #[inline]
    fn emit_literal(&mut self, byte: u8) {
        self.tokens.push(u32::from(byte));
        self.litlen_freq[usize::from(byte)] += 1;
        self.seg_litlen[usize::from(byte)] += 1;
        self.covered += 1;
        self.after_token();
    }

    #[inline]
    fn emit_match(&mut self, len: u32, dist: u32) {
        self.tokens
            .push(MATCH_FLAG | ((len - 3) << 16) | (dist - 1));
        let lsym = 257 + usize::from(LENGTH_CODE[len as usize - 3]);
        let dsym = distance_code(dist);
        self.litlen_freq[lsym] += 1;
        self.seg_litlen[lsym] += 1;
        self.dist_freq[dsym] += 1;
        self.seg_dist[dsym] += 1;
        self.covered += len as usize;
        self.after_token();
    }

    #[inline]
    fn after_token(&mut self) {
        if self.tokens.len() - self.seg_token >= CHECK_INTERVAL {
            self.check_block_split();
        }
    }

    /// Rare block planning stays outside the per-token update path.
    #[cold]
    fn check_block_split(&mut self) {
        let n = self.tokens.len();
        if n >= MAX_BLOCK_TOKENS {
            self.flush_block(n, false);
            return;
        }
        if self.seg_token > 0 && !self.params.store_only {
            let mut before_lit = self.litlen_freq;
            let mut before_dist = self.dist_freq;
            for (b, s) in before_lit.iter_mut().zip(&self.seg_litlen) {
                *b -= s;
            }
            for (b, s) in before_dist.iter_mut().zip(&self.seg_dist) {
                *b -= s;
            }
            let whole = dynamic_estimate(&self.litlen_freq, &self.dist_freq);
            let split = dynamic_estimate(&before_lit, &before_dist)
                + dynamic_estimate(&self.seg_litlen, &self.seg_dist);
            if split + SPLIT_MARGIN < whole {
                self.flush_block(self.seg_token, false);
            }
        }
        self.seg_token = self.tokens.len();
        self.seg_pos = self.covered;
        self.seg_litlen = [0; LITLEN_SYMBOLS];
        self.seg_dist = [0; DIST_SYMBOLS];
    }

    /// Emit the first `count` tokens as one block.
    fn flush_block(&mut self, count: usize, final_block: bool) {
        let whole = count == self.tokens.len();
        let (mut litlen_freq, dist_freq) = if whole {
            (self.litlen_freq, self.dist_freq)
        } else {
            let mut l = self.litlen_freq;
            let mut d = self.dist_freq;
            for (b, s) in l.iter_mut().zip(&self.seg_litlen) {
                *b -= s;
            }
            for (b, s) in d.iter_mut().zip(&self.seg_dist) {
                *b -= s;
            }
            (l, d)
        };
        let block_end = if whole { self.covered } else { self.seg_pos };
        if count == 0 && !final_block {
            self.block_start = block_end;
            return;
        }
        litlen_freq[usize::from(END_OF_BLOCK)] = 1;
        let raw_len = block_end - self.block_start;

        let extra = extra_bits(&litlen_freq, &dist_freq);
        let plan = DynamicPlan::new(&litlen_freq, &dist_freq);
        let dynamic_bits = plan.coded_bits(&litlen_freq, &dist_freq) + extra;
        let fixed_bits = 3
            + symbol_bits(&litlen_freq, &FIXED_LITLEN_LENGTHS)
            + symbol_bits(&dist_freq, &FIXED_DIST_LENGTHS)
            + extra;
        let stored_bits = self.stored_bits(raw_len);
        let bfinal = u32::from(final_block);

        if self.params.store_only || (stored_bits < dynamic_bits && stored_bits < fixed_bits) {
            self.write_stored(self.block_start, raw_len, final_block);
        } else if dynamic_bits <= fixed_bits {
            self.writer.write(bfinal | (2 << 1), 3);
            self.write_dynamic_header(&plan);
            let mut litlen_codes = [0u16; LITLEN_SYMBOLS];
            let mut dist_codes = [0u16; DIST_SYMBOLS];
            canonical_codes(&plan.litlen, &mut litlen_codes);
            canonical_codes(&plan.dist, &mut dist_codes);
            self.write_tokens(count, &litlen_codes, &plan.litlen, &dist_codes, &plan.dist);
        } else {
            self.writer.write(bfinal | (1 << 1), 3);
            self.write_tokens(
                count,
                &FIXED_LITLEN_CODES,
                &FIXED_LITLEN_LENGTHS,
                &FIXED_DIST_CODES,
                &FIXED_DIST_LENGTHS,
            );
        }

        // Drop the emitted tokens; what remains is the last segment.
        self.tokens.drain(..count);
        if whole {
            self.litlen_freq = [0; LITLEN_SYMBOLS];
            self.dist_freq = [0; DIST_SYMBOLS];
            self.seg_litlen = [0; LITLEN_SYMBOLS];
            self.seg_dist = [0; DIST_SYMBOLS];
            self.seg_pos = self.covered;
        } else {
            self.litlen_freq = self.seg_litlen;
            self.dist_freq = self.seg_dist;
        }
        self.seg_token = 0;
        self.block_start = block_end;
    }

    /// Exact bits of `len` raw bytes as stored blocks from the current bit
    /// position.
    fn stored_bits(&self, len: usize) -> u64 {
        let pieces = len.div_ceil(65_535).max(1) as u64;
        let first_pad = u64::from((8 - (self.writer.bit_phase() + 3) % 8) % 8);
        first_pad + 3 + (pieces - 1) * 8 + pieces * 32 + 8 * len as u64
    }

    fn write_stored(&mut self, start: usize, len: usize, final_block: bool) {
        let mut offset = 0;
        loop {
            let n = (len - offset).min(65_535);
            let last = offset + n == len;
            self.writer.write(u32::from(final_block && last), 3);
            self.writer.align();
            let n16 = n as u16;
            self.writer.out.extend_from_slice(&n16.to_le_bytes());
            self.writer.out.extend_from_slice(&(!n16).to_le_bytes());
            self.writer
                .out
                .extend_from_slice(&self.win[start + offset..start + offset + n]);
            offset += n;
            if last {
                break;
            }
        }
    }

    fn write_dynamic_header(&mut self, plan: &DynamicPlan) {
        self.writer.write((plan.n_lit - 257) as u32, 5);
        self.writer.write((plan.n_dist - 1) as u32, 5);
        self.writer.write((plan.n_cl - 4) as u32, 4);
        for &symbol in &CODE_LENGTH_ORDER[..plan.n_cl] {
            self.writer.write(u32::from(plan.code_length[symbol]), 3);
        }
        let mut cl_codes = [0u16; 19];
        canonical_codes(&plan.code_length, &mut cl_codes);
        for &(symbol, extra) in &plan.runs[..plan.n_runs] {
            let s = usize::from(symbol);
            self.writer
                .write(u32::from(cl_codes[s]), u32::from(plan.code_length[s]));
            match symbol {
                16 => self.writer.write(u32::from(extra), 2),
                17 => self.writer.write(u32::from(extra), 3),
                18 => self.writer.write(u32::from(extra), 7),
                _ => {}
            }
        }
    }

    fn write_tokens(
        &mut self,
        count: usize,
        litlen_codes: &[u16],
        litlen_lengths: &[u8],
        dist_codes: &[u16],
        dist_lengths: &[u8],
    ) {
        let writer = &mut self.writer;
        for &token in &self.tokens[..count] {
            if token & MATCH_FLAG == 0 {
                let s = token as usize;
                writer.write(u32::from(litlen_codes[s]), u32::from(litlen_lengths[s]));
                continue;
            }
            let len = ((token >> 16) & 0xFF) + 3;
            let dist = (token & 0xFFFF) + 1;
            let li = usize::from(LENGTH_CODE[len as usize - 3]);
            let ls = 257 + li;
            writer.write(u32::from(litlen_codes[ls]), u32::from(litlen_lengths[ls]));
            writer.write(
                len - u32::from(LENGTH_BASE[li]),
                u32::from(LENGTH_EXTRA[li]),
            );
            let ds = distance_code(dist);
            writer.write(u32::from(dist_codes[ds]), u32::from(dist_lengths[ds]));
            writer.write(dist - u32::from(DIST_BASE[ds]), u32::from(DIST_EXTRA[ds]));
        }
        let eob = usize::from(END_OF_BLOCK);
        writer.write(u32::from(litlen_codes[eob]), u32::from(litlen_lengths[eob]));
    }
}

/// Compress `data` into one raw DEFLATE stream at `level`.
pub fn compress(data: &[u8], level: Level) -> Vec<u8> {
    let mut deflater = Deflater::new(level);
    let mut out = Vec::with_capacity((data.len() / 2 + 64).min(64 * 1024 + 64));
    deflater.write(data, &mut out);
    deflater.finish(&mut out);
    out
}
