// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Prefix codes: decode tables built from code lengths, and length-limited
//! code construction for the encoder.
//!
//! Codes are assigned from lengths as RFC 1951 §3.2.2 describes: shorter codes
//! first, and within one length in symbol order. The stream carries each code
//! most-significant bit first while every other field is least-significant bit
//! first, so a code is bit-reversed once here and both the decoder's table
//! index and the encoder's bit writer can then work LSB-first.

use crate::error::{Alphabet, Error};

// --- Decode-table entries -------------------------------------------------
//
// bits 0–7   bits consumed
// bits 8–11  tag
// bits 12–15 extra-bit count (length / distance), subtable index width (link),
//            or the first literal's code length (pair)
// bits 16–31 value

pub(crate) const TAG_LITERAL: u32 = 0;
pub(crate) const TAG_PAIR: u32 = 1;
pub(crate) const TAG_LENGTH: u32 = 2;
pub(crate) const TAG_END: u32 = 3;
pub(crate) const TAG_LINK: u32 = 4;
pub(crate) const TAG_INVALID: u32 = 5;
pub(crate) const TAG_BAD_SYMBOL: u32 = 6;
pub(crate) const TAG_DISTANCE: u32 = 7;

#[inline]
pub(crate) const fn entry(bits: u32, tag: u32, aux: u32, value: u32) -> u32 {
    bits | (tag << 8) | (aux << 12) | (value << 16)
}
#[inline]
pub(crate) const fn entry_bits(e: u32) -> u32 {
    e & 0xFF
}
#[inline]
pub(crate) const fn entry_tag(e: u32) -> u32 {
    (e >> 8) & 0xF
}
#[inline]
pub(crate) const fn entry_aux(e: u32) -> u32 {
    (e >> 12) & 0xF
}
#[inline]
pub(crate) const fn entry_value(e: u32) -> u32 {
    e >> 16
}

/// What a symbol of the alphabet means once decoded.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kind {
    /// Code-length symbols 0–18: the value is the symbol.
    CodeLength,
    /// Literal/length symbols 0–287.
    LiteralLength,
    /// Distance symbols 0–31.
    Distance,
}

impl Kind {
    const fn alphabet(self) -> Alphabet {
        match self {
            Self::CodeLength => Alphabet::CodeLength,
            Self::LiteralLength => Alphabet::LiteralLength,
            Self::Distance => Alphabet::Distance,
        }
    }

    /// The entry a decoded `symbol` of `bits` code bits becomes.
    fn symbol_entry(self, symbol: usize, bits: u32) -> u32 {
        match self {
            Self::CodeLength => entry(bits, TAG_LITERAL, 0, symbol as u32),
            Self::LiteralLength => match symbol {
                0..=255 => entry(bits, TAG_LITERAL, 0, symbol as u32),
                256 => entry(bits, TAG_END, 0, 0),
                257..=285 => {
                    let index = symbol - 257;
                    entry(
                        bits,
                        TAG_LENGTH,
                        u32::from(crate::tables::LENGTH_EXTRA[index]),
                        u32::from(crate::tables::LENGTH_BASE[index]),
                    )
                }
                _ => entry(bits, TAG_BAD_SYMBOL, 0, symbol as u32),
            },
            Self::Distance => match symbol {
                0..=29 => entry(
                    bits,
                    TAG_DISTANCE,
                    u32::from(crate::tables::DIST_EXTRA[symbol]),
                    u32::from(crate::tables::DIST_BASE[symbol]),
                ),
                _ => entry(bits, TAG_BAD_SYMBOL, 0, symbol as u32),
            },
        }
    }
}

/// Reverse the low `len` bits of `code`.
#[inline]
pub(crate) const fn reverse_bits(code: u32, len: u32) -> u32 {
    code.reverse_bits() >> (32 - len)
}

/// A two-level decode table: a primary table indexed by the next
/// `primary_bits` stream bits, and subtables for longer codes.
#[derive(Clone, Debug)]
pub(crate) struct DecodeTable {
    pub(crate) entries: Vec<u32>,
    pub(crate) primary_bits: u32,
    /// Scratch copy of the primary table, reused by the pair pass.
    scratch: Vec<u32>,
}

impl DecodeTable {
    pub(crate) fn new(primary_bits: u32) -> Self {
        Self {
            entries: Vec::new(),
            primary_bits,
            scratch: Vec::new(),
        }
    }

    #[inline]
    pub(crate) const fn primary_mask(&self) -> u64 {
        (1u64 << self.primary_bits) - 1
    }

    /// Build the table for `lengths` (index = symbol). `pairs` merges two
    /// literals into one entry wherever both codes fit in the primary index.
    pub(crate) fn build(&mut self, lengths: &[u8], kind: Kind, pairs: bool) -> Result<(), Error> {
        let primary = self.primary_bits;
        let mut count = [0u32; 16];
        for &len in lengths {
            count[usize::from(len)] += 1;
        }
        count[0] = 0;
        // Kraft check: over-subscription means no prefix code has these lengths.
        let mut left: i64 = 1;
        for &c in &count[1..] {
            left = (left << 1) - i64::from(c);
            if left < 0 {
                return Err(Error::OversubscribedCode {
                    alphabet: kind.alphabet(),
                });
            }
        }
        let mut next = [0u32; 16];
        let mut code = 0u32;
        for len in 1..16 {
            code = (code + count[len - 1]) << 1;
            next[len] = code;
        }

        // Longest code under each primary prefix, to size subtables.
        let primary_size = 1usize << primary;
        let mut reversed = vec![0u32; lengths.len()];
        let mut sub_len = vec![0u8; primary_size];
        for (symbol, &len) in lengths.iter().enumerate() {
            if len == 0 {
                continue;
            }
            let len32 = u32::from(len);
            let rev = reverse_bits(next[usize::from(len)], len32);
            next[usize::from(len)] += 1;
            reversed[symbol] = rev;
            if len32 > primary {
                let prefix = (rev & ((1 << primary) - 1)) as usize;
                sub_len[prefix] = sub_len[prefix].max(len - primary as u8);
            }
        }

        self.entries.clear();
        self.entries
            .resize(primary_size, entry(primary, TAG_INVALID, 0, 0));
        // Subtable links, in prefix order.
        for (prefix, &width) in sub_len.iter().enumerate() {
            if width == 0 {
                continue;
            }
            let offset = self.entries.len() as u32;
            let width = u32::from(width);
            self.entries[prefix] = entry(primary, TAG_LINK, width, offset);
            let invalid = entry(primary + width, TAG_INVALID, 0, 0);
            self.entries
                .extend(std::iter::repeat_n(invalid, 1usize << width));
        }
        for (symbol, &len) in lengths.iter().enumerate() {
            if len == 0 {
                continue;
            }
            let len32 = u32::from(len);
            let rev = reversed[symbol];
            let e = kind.symbol_entry(symbol, len32);
            if len32 <= primary {
                let mut index = rev as usize;
                while index < primary_size {
                    self.entries[index] = e;
                    index += 1 << len32;
                }
            } else {
                let prefix = (rev & ((1 << primary) - 1)) as usize;
                let link = self.entries[prefix];
                let width = entry_aux(link);
                let offset = entry_value(link) as usize;
                let high = (rev >> primary) as usize;
                let step = 1usize << (len32 - primary);
                let mut index = high;
                while index < (1usize << width) {
                    self.entries[offset + index] = e;
                    index += step;
                }
            }
        }
        if pairs {
            self.merge_literal_pairs();
        }
        Ok(())
    }

    /// Replace each primary literal entry whose remaining index bits hold a
    /// whole second literal code with a two-literal entry.
    fn merge_literal_pairs(&mut self) {
        let primary = self.primary_bits;
        let size = 1usize << primary;
        self.scratch.clear();
        self.scratch.extend_from_slice(&self.entries[..size]);
        for index in 0..size {
            let first = self.scratch[index];
            if entry_tag(first) != TAG_LITERAL {
                continue;
            }
            let l1 = entry_bits(first);
            let rest = index >> l1;
            let second = self.scratch[rest];
            if entry_tag(second) != TAG_LITERAL {
                continue;
            }
            let l2 = entry_bits(second);
            if l1 + l2 > primary {
                continue;
            }
            let value = entry_value(first) | (entry_value(second) << 8);
            self.entries[index] = entry(l1 + l2, TAG_PAIR, l1, value);
        }
    }
}

// --- Encoder-side construction --------------------------------------------

/// Optimal code lengths for `freqs`, limited to `limit` bits.
///
/// Symbols with a zero count get length 0. When fewer than two symbols are
/// used, two one-bit codes are assigned (the used symbol, if any, and the
/// lowest other symbol) so the emitted code is always complete.
pub(crate) fn code_lengths(freqs: &[u32], limit: u8, lengths: &mut [u8]) {
    debug_assert_eq!(freqs.len(), lengths.len());
    lengths.fill(0);
    let mut leaves: Vec<(u32, u16)> = freqs
        .iter()
        .enumerate()
        .filter(|&(_, &f)| f > 0)
        .map(|(s, &f)| (f, s as u16))
        .collect();
    if leaves.len() < 2 {
        let used = leaves.first().map(|&(_, s)| usize::from(s));
        let other = (0..freqs.len())
            .find(|&s| Some(s) != used)
            .expect("an alphabet has at least two symbols");
        lengths[used.unwrap_or_else(|| usize::from(other == 0))] = 1;
        lengths[other] = 1;
        return;
    }
    leaves.sort_unstable();
    let n = leaves.len();

    // Two-queue merge: leaves in ascending order, internal nodes in creation
    // order (their weights never decrease). Ties take the leaf.
    let mut weight: Vec<u64> = leaves.iter().map(|&(f, _)| u64::from(f)).collect();
    weight.reserve(n - 1);
    let mut parent = vec![0u32; 2 * n - 1];
    let (mut leaf, mut node) = (0usize, n);
    // The lighter of the next leaf and the next unmerged internal node; ties
    // take the leaf.
    let take = |weight: &[u64], leaf: &mut usize, node: &mut usize, built: usize| {
        if *leaf < n && (*node >= built || weight[*leaf] <= weight[*node]) {
            *leaf += 1;
            *leaf - 1
        } else {
            *node += 1;
            *node - 1
        }
    };
    for built in n..(2 * n - 1) {
        let a = take(&weight, &mut leaf, &mut node, built);
        let b = take(&weight, &mut leaf, &mut node, built);
        weight.push(weight[a] + weight[b]);
        parent[a] = built as u32;
        parent[b] = built as u32;
    }
    // Depths, root first.
    let root = 2 * n - 2;
    let mut depth = vec![0u8; 2 * n - 1];
    for index in (0..root).rev() {
        depth[index] = depth[parent[index] as usize] + 1;
    }

    let mut len: Vec<u8> = depth[..n].to_vec();
    limit_lengths(&mut len, limit);
    for (i, &(_, symbol)) in leaves.iter().enumerate() {
        lengths[usize::from(symbol)] = len[i];
    }
}

/// Bring `len` (leaves in ascending-frequency order) within `limit` bits
/// while keeping a valid prefix code, then spend any freed code space on the
/// most frequent leaves.
fn limit_lengths(len: &mut [u8], limit: u8) {
    let cap: u64 = 1 << limit;
    let kraft = |len: &[u8]| -> u64 { len.iter().map(|&l| 1u64 << (limit - l.min(limit))).sum() };
    if len.iter().all(|&l| l <= limit) {
        return;
    }
    for l in len.iter_mut() {
        *l = (*l).min(limit);
    }
    let mut sum = kraft(len);
    while sum > cap {
        // Lengthen the least frequent leaf among the deepest that can still grow.
        let deepest = len
            .iter()
            .copied()
            .filter(|&l| l < limit)
            .max()
            .expect("room to grow");
        let pick = len.iter().position(|&l| l == deepest).expect("present");
        sum -= 1u64 << (limit - deepest - 1);
        len[pick] += 1;
    }
    // Shorten the most frequent leaves while the code stays a prefix code.
    let mut changed = true;
    while changed {
        changed = false;
        for i in (0..len.len()).rev() {
            if len[i] > 1 {
                let gain = 1u64 << (limit - len[i]);
                if sum + gain <= cap {
                    len[i] -= 1;
                    sum += gain;
                    changed = true;
                }
            }
        }
    }
}

/// Bit-reversed canonical codes for `lengths`, ready for an LSB-first writer.
pub(crate) fn canonical_codes(lengths: &[u8], codes: &mut [u16]) {
    let mut count = [0u32; 16];
    for &len in lengths {
        count[usize::from(len)] += 1;
    }
    count[0] = 0;
    let mut next = [0u32; 16];
    let mut code = 0u32;
    for len in 1..16 {
        code = (code + count[len - 1]) << 1;
        next[len] = code;
    }
    for (symbol, &len) in lengths.iter().enumerate() {
        if len == 0 {
            codes[symbol] = 0;
            continue;
        }
        let c = next[usize::from(len)];
        next[usize::from(len)] += 1;
        codes[symbol] = reverse_bits(c, u32::from(len)) as u16;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kraft_ok(lengths: &[u8], limit: u8) -> bool {
        let sum: u64 = lengths
            .iter()
            .filter(|&&l| l > 0)
            .map(|&l| 1u64 << (limit - l))
            .sum();
        sum <= 1 << limit && lengths.iter().all(|&l| l <= limit)
    }

    #[test]
    fn skewed_frequencies_are_limited_to_fifteen_bits() {
        // Fibonacci counts force an unlimited depth of about 30.
        let mut freqs = vec![0u32; 40];
        let (mut a, mut b) = (1u32, 1u32);
        for f in &mut freqs {
            *f = a;
            let c = a.saturating_add(b);
            a = b;
            b = c;
        }
        let mut lengths = vec![0u8; 40];
        code_lengths(&freqs, 15, &mut lengths);
        assert!(kraft_ok(&lengths, 15));
        assert!(lengths.iter().all(|&l| l > 0));
        let mut small = vec![0u8; 19];
        code_lengths(&freqs[..19], 7, &mut small);
        assert!(kraft_ok(&small, 7));
    }

    #[test]
    fn one_used_symbol_still_gets_a_complete_code() {
        let mut lengths = [0u8; 30];
        let mut freqs = [0u32; 30];
        freqs[7] = 9;
        code_lengths(&freqs, 15, &mut lengths);
        assert_eq!(lengths[7], 1);
        assert_eq!(
            lengths.iter().map(|&l| usize::from(l == 1)).sum::<usize>(),
            2
        );
        let mut none = [0u8; 30];
        code_lengths(&[0; 30], 15, &mut none);
        assert_eq!(none.iter().map(|&l| usize::from(l == 1)).sum::<usize>(), 2);
    }

    #[test]
    fn oversubscribed_lengths_are_refused_and_incomplete_accepted() {
        let mut table = DecodeTable::new(7);
        assert_eq!(
            table.build(&[1, 1, 1], Kind::CodeLength, false),
            Err(Error::OversubscribedCode {
                alphabet: Alphabet::CodeLength
            })
        );
        assert!(table.build(&[1, 1], Kind::CodeLength, false).is_ok());
        assert!(table.build(&[1, 0, 0], Kind::CodeLength, false).is_ok());
    }
}
