// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The recorded choice sequence every generator draws from, and the in-house
//! pseudo-random stream behind it.
//!
//! A generator never sees a random number generator. It asks a [`Choices`] for
//! a bounded integer (`draw(max)` answers something in `0..=max`), and the
//! answer is appended to a record. In random mode the answer comes from a
//! xoshiro256** stream; in replay mode it comes from a stored sequence. Because
//! every value a property sees is a pure function of that record, shrinking
//! edits the record — deletes spans, lowers entries, reorders them — and
//! replays it, and the replayed value is by construction one the generator can
//! produce. That is why shrinking works through `prop_map`, `prop_flat_map`
//! and `prop_filter` alike.

use std::fmt;

/// SplitMix64 (Steele, Lea and Flood): a seed expander and a strong 64-bit
/// finaliser.
#[derive(Debug, Clone)]
pub(crate) struct SplitMix64(u64);

impl SplitMix64 {
    pub(crate) const fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub(crate) const fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// xoshiro256** (Blackman and Vigna), seeded through [`SplitMix64`] as its
/// authors recommend so that no seed yields the all-zero state.
#[derive(Debug, Clone)]
pub(crate) struct Xoshiro256 {
    s: [u64; 4],
}

impl Xoshiro256 {
    pub(crate) const fn from_seed(seed: u64) -> Self {
        let mut expander = SplitMix64::new(seed);
        Self {
            s: [
                expander.next_u64(),
                expander.next_u64(),
                expander.next_u64(),
                expander.next_u64(),
            ],
        }
    }

    #[cfg(test)]
    pub(crate) const fn from_state(s: [u64; 4]) -> Self {
        Self { s }
    }

    pub(crate) const fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// A uniform value in `0..=max`, without modulo bias: draws below
    /// `2^64 mod (max + 1)` are redrawn.
    pub(crate) const fn up_to(&mut self, max: u64) -> u64 {
        if max == u64::MAX {
            return self.next_u64();
        }
        let range = max + 1;
        let threshold = range.wrapping_neg() % range;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return value % range;
            }
        }
    }

    /// A uniform `f64` in `[0, 1)` from the top 53 bits.
    pub(crate) fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

/// Why a generator could not produce a value from the choices it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invalid {
    /// A filter, a size constraint or a transition precondition rejected more
    /// candidates than the run's reject budget allows.
    RejectLimit,
    /// The generator made more draws than one value may take
    /// ([`Choices::MAX_DRAWS`]) — an unbounded recursion or a runaway loop.
    Overrun,
}

impl fmt::Display for Invalid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RejectLimit => f.write_str("the reject budget is exhausted"),
            Self::Overrun => write!(
                f,
                "generation made more than {} draws for one value",
                Choices::MAX_DRAWS
            ),
        }
    }
}

impl std::error::Error for Invalid {}

/// A malformed hexadecimal choice sequence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HexError(String);

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for HexError {}

/// One recorded draw: the answer and the bound it was drawn under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Draw {
    pub(crate) value: u64,
    pub(crate) max: u64,
}

#[derive(Debug, Clone)]
enum Source {
    Random(Xoshiro256),
    Values { values: Vec<u64>, next: usize },
    Bytes { bytes: Vec<u8>, next: usize },
}

/// The source every generator draws from, and the record of what it drew.
///
/// * [`Choices::random`] draws from a seeded xoshiro256** stream.
/// * [`Choices::from_bytes`] and [`Choices::from_hex`] replay a byte string:
///   a draw bounded by `max` consumes the fewest big-endian bytes that can
///   hold `max` (none when `max` is 0) and answers that number modulo
///   `max + 1`. Any byte string therefore drives any generator, and an
///   exhausted string answers 0 — the simplest choice — from then on.
///
/// [`Choices::to_bytes`] writes the record in exactly that encoding, so
/// `Choices::from_bytes(&choices.to_bytes())` reproduces the value `choices`
/// produced. That is the replayable sequence a failing property prints.
#[derive(Debug, Clone)]
pub struct Choices {
    source: Source,
    record: Vec<Draw>,
    rejects_left: u32,
    reject_reason: Option<String>,
}

/// The reject budget of a replayed sequence: a replay is one value, so this
/// bounds one generation rather than a run.
const REPLAY_REJECTS: u32 = 10_000;

impl Choices {
    /// The most draws one value may take before generation is abandoned as
    /// [`Invalid::Overrun`].
    pub const MAX_DRAWS: usize = 1 << 24;

    /// Draw from the xoshiro256** stream seeded with `seed`, with the default
    /// reject budget of [`super::Config`].
    pub fn random(seed: u64) -> Self {
        Self::random_with_budget(seed, super::Config::default().max_rejects)
    }

    pub(crate) fn random_with_budget(seed: u64, rejects_left: u32) -> Self {
        Self::with_source(Source::Random(Xoshiro256::from_seed(seed)), rejects_left)
    }

    /// Replay `bytes` (see the type's documentation for the encoding).
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self::with_source(
            Source::Bytes {
                bytes: bytes.to_vec(),
                next: 0,
            },
            REPLAY_REJECTS,
        )
    }

    /// Replay a hexadecimal choice sequence, as printed by a failing property.
    /// ASCII whitespace between digits is ignored.
    pub fn from_hex(hex: &str) -> Result<Self, HexError> {
        Ok(Self::from_bytes(&decode_hex(hex)?))
    }

    pub(crate) fn replay_values(values: Vec<u64>, rejects_left: u32) -> Self {
        Self::with_source(Source::Values { values, next: 0 }, rejects_left)
    }

    pub(crate) fn replay_budget() -> u32 {
        REPLAY_REJECTS
    }

    const fn with_source(source: Source, rejects_left: u32) -> Self {
        Self {
            source,
            record: Vec::new(),
            rejects_left,
            reject_reason: None,
        }
    }

    /// Whether the choices come from the random stream rather than a replay.
    pub const fn is_random(&self) -> bool {
        matches!(self.source, Source::Random(_))
    }

    /// A choice in `0..=max`, recorded. Shrinking lowers recorded choices
    /// towards 0, so a generator should map 0 to its simplest value.
    pub fn draw(&mut self, max: u64) -> Result<u64, Invalid> {
        self.draw_with(max, |rng| rng.up_to(max))
    }

    /// A choice in `0..=max` whose random-mode answer is `pick`'s rather than a
    /// uniform one. Replays read it like any other draw. This is how weighted
    /// and target-sized choices keep their distribution while staying
    /// shrinkable.
    fn draw_with(
        &mut self,
        max: u64,
        pick: impl FnOnce(&mut Xoshiro256) -> u64,
    ) -> Result<u64, Invalid> {
        if self.record.len() >= Self::MAX_DRAWS {
            return Err(Invalid::Overrun);
        }
        let value = match &mut self.source {
            Source::Random(rng) => pick(rng),
            Source::Values { values, next } => {
                let raw = values.get(*next).copied().unwrap_or(0);
                *next += 1;
                reduce(raw, max)
            }
            Source::Bytes { bytes, next } => {
                let mut raw = 0u64;
                for _ in 0..byte_width(max) {
                    raw = (raw << 8) | u64::from(bytes.get(*next).copied().unwrap_or(0));
                    *next += 1;
                }
                reduce(raw, max)
            }
        };
        debug_assert!(value <= max, "a pick answered {value} above {max}");
        self.record.push(Draw { value, max });
        Ok(value)
    }

    /// A boolean that is `true` with probability `p` in random mode.
    pub fn weighted_bool(&mut self, p: f64) -> Result<bool, Invalid> {
        Ok(self.draw_with(1, |rng| u64::from(rng.unit() < p))? == 1)
    }

    /// A boolean whose random-mode answer is `wanted`: a size-driving
    /// continuation flag that a replay may turn off.
    pub(crate) fn forced_bool(&mut self, wanted: bool) -> Result<bool, Invalid> {
        Ok(self.draw_with(1, |_| u64::from(wanted))? == 1)
    }

    /// An index into `weights`, chosen in proportion to them in random mode.
    /// Index 0 is the shrink target.
    pub fn weighted_index(&mut self, weights: &[u32]) -> Result<usize, Invalid> {
        assert!(
            !weights.is_empty(),
            "a weighted choice needs an alternative"
        );
        let total: u64 = weights.iter().map(|&weight| u64::from(weight)).sum();
        assert!(total > 0, "a weighted choice needs a positive total weight");
        let max = (weights.len() - 1) as u64;
        let index = self.draw_with(max, |rng| {
            let mut ticket = rng.up_to(total - 1);
            for (index, &weight) in weights.iter().enumerate() {
                let weight = u64::from(weight);
                if ticket < weight {
                    return index as u64;
                }
                ticket -= weight;
            }
            unreachable!("the ticket is below the total weight")
        })?;
        Ok(index as usize)
    }

    /// A random-mode target size in `min..=max`, not recorded: a replay's
    /// size is decided by its continuation flags alone.
    pub(crate) fn target_size(&mut self, min: usize, max: usize) -> usize {
        match &mut self.source {
            Source::Random(rng) => min + rng.up_to((max - min) as u64) as usize,
            Source::Values { .. } | Source::Bytes { .. } => max,
        }
    }

    /// Charge one rejected candidate — a filter miss, an unmet size, a failed
    /// precondition — against the reject budget. `reason` names the rejecter
    /// in the error when the budget runs out.
    pub fn reject(&mut self, reason: &str) -> Result<(), Invalid> {
        if self.rejects_left == 0 {
            self.reject_reason = Some(reason.to_owned());
            return Err(Invalid::RejectLimit);
        }
        self.rejects_left -= 1;
        Ok(())
    }

    pub(crate) const fn rejects_left(&self) -> u32 {
        self.rejects_left
    }

    pub(crate) fn take_reject_reason(&mut self) -> Option<String> {
        self.reject_reason.take()
    }

    /// How many choices have been drawn.
    pub fn len(&self) -> usize {
        self.record.len()
    }

    /// Whether nothing has been drawn.
    pub fn is_empty(&self) -> bool {
        self.record.is_empty()
    }

    pub(crate) fn values(&self) -> Vec<u64> {
        self.record.iter().map(|draw| draw.value).collect()
    }

    /// The record in the byte encoding [`Choices::from_bytes`] replays.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.record.len());
        for draw in &self.record {
            let width = byte_width(draw.max);
            bytes.extend_from_slice(&draw.value.to_be_bytes()[8 - width..]);
        }
        bytes
    }

    /// The record as lowercase hexadecimal, the form a failing property
    /// prints and [`Choices::from_hex`] reads.
    pub fn to_hex(&self) -> String {
        encode_hex(&self.to_bytes())
    }
}

/// The bytes a draw bounded by `max` consumes: the fewest that hold `max`.
const fn byte_width(max: u64) -> usize {
    (64 - max.leading_zeros() as usize).div_ceil(8)
}

const fn reduce(raw: u64, max: u64) -> u64 {
    if max == u64::MAX {
        raw
    } else {
        raw % (max + 1)
    }
}

pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(char::from(DIGITS[usize::from(byte >> 4)]));
        out.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    out
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, HexError> {
    let digits: Vec<u8> = hex
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if !digits.len().is_multiple_of(2) {
        return Err(HexError(format!(
            "a choice sequence has an even number of hex digits, not {}",
            digits.len()
        )));
    }
    let (pairs, _) = digits.as_chunks::<2>();
    pairs
        .iter()
        .map(|pair| {
            let nibble = |digit: u8| {
                char::from(digit)
                    .to_digit(16)
                    .map(|value| value as u8)
                    .ok_or_else(|| {
                        HexError(format!(
                            "`{}` is not a hexadecimal digit",
                            char::from(digit).escape_default()
                        ))
                    })
            };
            Ok((nibble(pair[0])? << 4) | nibble(pair[1])?)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Choices, SplitMix64, Xoshiro256, byte_width, decode_hex, encode_hex};

    #[test]
    fn splitmix64_matches_the_reference_outputs() {
        // The first outputs of the reference implementation from seed 0.
        let mut generator = SplitMix64::new(0);
        assert_eq!(generator.next_u64(), 0xE220_A839_7B1D_CDAF);
        assert_eq!(generator.next_u64(), 0x6E78_9E6A_A1B9_65F4);
        assert_eq!(generator.next_u64(), 0x06C4_5D18_8009_454F);
    }

    #[test]
    fn xoshiro256_starstar_matches_the_reference_output() {
        // The reference implementation's first output from state [1, 2, 3, 4]
        // is rotl(2 * 5, 7) * 9.
        let mut generator = Xoshiro256::from_state([1, 2, 3, 4]);
        assert_eq!(generator.next_u64(), 11_520);
        assert_eq!(generator.next_u64(), 0);
        assert_eq!(generator.next_u64(), 1_509_978_240);
    }

    #[test]
    fn up_to_stays_in_bounds_and_reaches_both_ends() {
        let mut generator = Xoshiro256::from_seed(7);
        let mut seen = [false; 5];
        for _ in 0..10_000 {
            let value = generator.up_to(4);
            seen[value as usize] = true;
        }
        assert_eq!(seen, [true; 5]);
        assert_eq!(generator.up_to(0), 0);
    }

    #[test]
    fn byte_width_is_the_fewest_bytes_holding_the_bound() {
        assert_eq!(byte_width(0), 0);
        assert_eq!(byte_width(1), 1);
        assert_eq!(byte_width(255), 1);
        assert_eq!(byte_width(256), 2);
        assert_eq!(byte_width(u64::MAX), 8);
    }

    #[test]
    fn hex_round_trips_and_refuses_what_is_not_hex() {
        assert_eq!(encode_hex(&[0x00, 0xab, 0x7f]), "00ab7f");
        assert_eq!(decode_hex("00ab7f"), Ok(vec![0x00, 0xab, 0x7f]));
        assert_eq!(decode_hex("00 AB\n7f"), Ok(vec![0x00, 0xab, 0x7f]));
        assert!(decode_hex("0").is_err());
        assert!(decode_hex("0g").is_err());
        assert_eq!(decode_hex(""), Ok(Vec::new()));
    }

    #[test]
    fn a_recorded_sequence_replays_from_its_bytes() {
        let mut random = Choices::random(99);
        let drawn: Vec<u64> = [0, 1, 7, 300, 70_000, u64::MAX]
            .iter()
            .map(|&max| random.draw(max).expect("draws"))
            .collect();
        let mut replay = Choices::from_bytes(&random.to_bytes());
        let replayed: Vec<u64> = [0, 1, 7, 300, 70_000, u64::MAX]
            .iter()
            .map(|&max| replay.draw(max).expect("draws"))
            .collect();
        assert_eq!(drawn, replayed);
        assert_eq!(replay.to_bytes(), random.to_bytes());
        // Exhausted replays answer 0.
        assert_eq!(replay.draw(1_000), Ok(0));
    }
}
