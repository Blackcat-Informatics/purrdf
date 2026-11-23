// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;

use purrdf_hash::{blake3::derive_key, hex::Lower};

use super::FoldError;
use crate::FastMap;
use crate::v2::{DIGEST_LENGTH, ValueReference};

pub(in crate::v2) fn unit_digest(bytes: &[u8]) -> [u8; DIGEST_LENGTH] {
    derive_key("gts2 unit", bytes)
}

/// The digest and length of a GTS v2 reconstructed value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ValueName {
    /// Unit-domain digest of the full decoded bytes.
    pub digest: [u8; DIGEST_LENGTH],
    /// Length of the full decoded bytes.
    pub length: u64,
}

impl ValueName {
    /// Name the supplied bytes using P4 unit identity.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self {
            digest: unit_digest(bytes),
            length: bytes.len() as u64,
        }
    }
}

impl From<ValueReference<'_>> for ValueName {
    fn from(reference: ValueReference<'_>) -> Self {
        Self {
            digest: *reference.digest,
            length: reference.length,
        }
    }
}

impl fmt::Display for ValueName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {} bytes", Lower(&self.digest), self.length)
    }
}

impl purrdf_hash::error::StructuredError for ValueName {
    purrdf_hash::error_visit! {
        @record self { digest, length } => "gts-v2-value-name" {
            "digest" => Bytes(&self.digest),
            "length" => ByteCount(u128::from(self.length)),
        }
    }
}

/// Prior value declarations observed in file order.
#[derive(Debug, Default)]
pub struct ValueStore {
    declarations: FastMap<ValueName, Option<Vec<u8>>>,
}

#[derive(Clone, Copy)]
pub(super) enum Candidate<'a> {
    Missing,
    Unavailable,
    Verified(&'a [u8]),
}

impl ValueStore {
    /// Record a candidate declaration and its verified bytes when available.
    ///
    /// The first verified candidate supplies later references of the same
    /// digest and length. An unavailable candidate preserves its declaration.
    ///
    /// # Errors
    /// Returns a claim mismatch or a host reservation failure.
    pub fn record(&mut self, name: ValueName, bytes: Option<Vec<u8>>) -> Result<(), FoldError> {
        if let Some(bytes) = &bytes {
            let actual = ValueName::of(bytes);
            if name != actual {
                return Err(FoldError::ValueClaim {
                    expected: name,
                    actual,
                });
            }
        }
        if let Some(earlier) = self.declarations.get_mut(&name) {
            if earlier.is_none() {
                *earlier = bytes;
            }
        } else {
            self.declarations.try_reserve(1)?;
            self.declarations.insert(name, bytes);
        }
        Ok(())
    }

    pub(super) fn get(&self, name: ValueName) -> Candidate<'_> {
        match self.declarations.get(&name) {
            None => Candidate::Missing,
            Some(None) => Candidate::Unavailable,
            Some(Some(bytes)) => Candidate::Verified(bytes),
        }
    }
}
