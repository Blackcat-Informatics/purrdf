// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dependency-free ULID support for deterministic GTS-generated identifiers.
//!
//! The implementation intentionally exposes only construction from caller-owned
//! entropy or deterministic counters. It does not call an operating-system RNG,
//! JavaScript RNG, `uuid`, `ulid`, `rand`, or `getrandom`.

use std::fmt;
use std::str::FromStr;

use purrdf_lex::crockford::{self, CrockfordError};

const ULID_LEN: usize = crockford::U128_DIGITS;
const TIMESTAMP_BYTES: usize = 6;
const RANDOMNESS_BYTES: usize = 10;
const MAX_RANDOMNESS: u128 = (1u128 << 80) - 1;

purrdf_lex::message_error! {
    /// Error raised for invalid ULID construction or parsing.
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct UlidError, detail;
}

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
        crockford::write_u128(u128::from_be_bytes(self.0), f)
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
        crockford::parse_u128(text)
            .map(|value| Self(value.to_be_bytes()))
            .map_err(|error| {
                UlidError::new(match error {
                    CrockfordError::Length { .. } => {
                        format!("ULID must be {ULID_LEN} Crockford Base32 characters")
                    }
                    CrockfordError::Digit { offset, byte } => format!(
                        "invalid ULID character {:?} at offset {offset}",
                        char::from(byte)
                    ),
                    CrockfordError::Overflow => {
                        "ULID exceeds 128-bit range because the first digit is greater than 7"
                            .to_owned()
                    }
                })
            })
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
