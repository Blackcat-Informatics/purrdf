// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Dependency-free ULID support for deterministic GTS-generated identifiers.
//!
//! The implementation intentionally exposes only construction from caller-owned
//! entropy or deterministic counters. It does not call an operating-system RNG,
//! JavaScript RNG, `uuid`, `ulid`, `rand`, or `getrandom`.

use std::fmt;
use std::str::FromStr;

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const ULID_LEN: usize = 26;
const TIMESTAMP_BYTES: usize = 6;
const RANDOMNESS_BYTES: usize = 10;
const MAX_RANDOMNESS: u128 = (1u128 << 80) - 1;

/// Error raised for invalid ULID construction or parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UlidError {
    detail: String,
}

impl UlidError {
    fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// Human-readable error detail.
    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for UlidError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for UlidError {}

/// A 128-bit ULID value with canonical Crockford Base32 rendering.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ulid([u8; 16]);

impl Ulid {
    /// Maximum timestamp value representable by the 48-bit ULID timestamp field.
    pub const MAX_TIMESTAMP_MS: u64 = (1u64 << 48) - 1;

    /// Construct a ULID from its timestamp and 80-bit randomness fields.
    pub fn from_parts(
        timestamp_ms: u64,
        randomness: [u8; RANDOMNESS_BYTES],
    ) -> Result<Self, UlidError> {
        if timestamp_ms > Self::MAX_TIMESTAMP_MS {
            return Err(UlidError::new(format!(
                "ULID timestamp {timestamp_ms} exceeds 48-bit range"
            )));
        }

        let mut bytes = [0u8; 16];
        let timestamp = timestamp_ms.to_be_bytes();
        bytes[..TIMESTAMP_BYTES].copy_from_slice(&timestamp[2..]);
        bytes[TIMESTAMP_BYTES..].copy_from_slice(&randomness);
        Ok(Self(bytes))
    }

    /// Construct a deterministic ULID from a timestamp and 80-bit counter.
    pub fn from_counter(timestamp_ms: u64, counter: u128) -> Result<Self, UlidError> {
        if counter > MAX_RANDOMNESS {
            return Err(UlidError::new(
                "ULID counter exceeds 80-bit randomness field",
            ));
        }
        let counter_bytes = counter.to_be_bytes();
        let mut randomness = [0u8; RANDOMNESS_BYTES];
        randomness.copy_from_slice(&counter_bytes[6..]);
        Self::from_parts(timestamp_ms, randomness)
    }

    /// Borrow the raw 16-byte ULID value.
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// Return the 48-bit timestamp field.
    pub fn timestamp_ms(&self) -> u64 {
        let mut bytes = [0u8; 8];
        bytes[2..].copy_from_slice(&self.0[..TIMESTAMP_BYTES]);
        u64::from_be_bytes(bytes)
    }

    /// Return the 80-bit randomness field.
    pub fn randomness(&self) -> [u8; RANDOMNESS_BYTES] {
        let mut bytes = [0u8; RANDOMNESS_BYTES];
        bytes.copy_from_slice(&self.0[TIMESTAMP_BYTES..]);
        bytes
    }
}

impl fmt::Display for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = u128::from_be_bytes(self.0);
        let mut buffer = [0u8; ULID_LEN];
        for (index, byte) in buffer.iter_mut().enumerate() {
            let shift = 125 - index * 5;
            let digit = ((value >> shift) & 0x1f) as usize;
            *byte = CROCKFORD[digit];
        }
        let text = std::str::from_utf8(&buffer).map_err(|_| fmt::Error)?;
        f.write_str(text)
    }
}

impl fmt::Debug for Ulid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ulid({self})")
    }
}

impl FromStr for Ulid {
    type Err = UlidError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() != ULID_LEN {
            return Err(UlidError::new(format!(
                "ULID must be {ULID_LEN} Crockford Base32 characters"
            )));
        }

        let mut value = 0u128;
        for (index, byte) in text.bytes().enumerate() {
            let digit = decode_crockford(byte).ok_or_else(|| {
                UlidError::new(format!(
                    "invalid ULID character {:?} at offset {index}",
                    byte as char
                ))
            })?;
            if index == 0 && digit > 7 {
                return Err(UlidError::new(
                    "ULID exceeds 128-bit range because the first digit is greater than 7",
                ));
            }
            value = (value << 5) | u128::from(digit);
        }

        Ok(Self(value.to_be_bytes()))
    }
}

/// Deterministic ULID sequence for parser/import generated identifiers.
#[derive(Clone, Debug)]
pub struct DeterministicUlidGenerator {
    timestamp_ms: u64,
    next_counter: u128,
}

impl DeterministicUlidGenerator {
    /// Create a generator starting at counter zero.
    pub fn new(timestamp_ms: u64) -> Result<Self, UlidError> {
        Self::with_counter(timestamp_ms, 0)
    }

    /// Create a generator starting at a caller-selected 80-bit counter.
    pub fn with_counter(timestamp_ms: u64, next_counter: u128) -> Result<Self, UlidError> {
        Ulid::from_counter(timestamp_ms, next_counter)?;
        Ok(Self {
            timestamp_ms,
            next_counter,
        })
    }

    /// Return the next deterministic ULID in sequence.
    pub fn next_ulid(&mut self) -> Result<Ulid, UlidError> {
        let ulid = Ulid::from_counter(self.timestamp_ms, self.next_counter)?;
        self.next_counter = self
            .next_counter
            .checked_add(1)
            .ok_or_else(|| UlidError::new("ULID counter overflow"))?;
        Ok(ulid)
    }
}

fn decode_crockford(byte: u8) -> Option<u8> {
    match byte {
        b'0' => Some(0),
        b'1' => Some(1),
        b'2' => Some(2),
        b'3' => Some(3),
        b'4' => Some(4),
        b'5' => Some(5),
        b'6' => Some(6),
        b'7' => Some(7),
        b'8' => Some(8),
        b'9' => Some(9),
        b'A' | b'a' => Some(10),
        b'B' | b'b' => Some(11),
        b'C' | b'c' => Some(12),
        b'D' | b'd' => Some(13),
        b'E' | b'e' => Some(14),
        b'F' | b'f' => Some(15),
        b'G' | b'g' => Some(16),
        b'H' | b'h' => Some(17),
        b'J' | b'j' => Some(18),
        b'K' | b'k' => Some(19),
        b'M' | b'm' => Some(20),
        b'N' | b'n' => Some(21),
        b'P' | b'p' => Some(22),
        b'Q' | b'q' => Some(23),
        b'R' | b'r' => Some(24),
        b'S' | b's' => Some(25),
        b'T' | b't' => Some(26),
        b'V' | b'v' => Some(27),
        b'W' | b'w' => Some(28),
        b'X' | b'x' => Some(29),
        b'Y' | b'y' => Some(30),
        b'Z' | b'z' => Some(31),
        _ => None,
    }
}
