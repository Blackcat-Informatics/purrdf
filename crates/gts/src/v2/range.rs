// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use core::num::NonZeroU64;

use purrdf_hash::blake3::Hasher;
use purrdf_lex::cbor::head;

use super::{DIGEST_LENGTH, LinkState};

type Digest = [u8; DIGEST_LENGTH];

/// A P5 range whose first leaf commits to the segment opener.
#[derive(Debug)]
pub struct Range {
    count: NonZeroU64,
    // The leaf count's set bits select the occupied heights.
    peaks: [Digest; u64::BITS as usize],
}

/// The leaf count cannot admit another position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeFull;

impl fmt::Display for RangeFull {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GTS v2 range leaf count would exceed u64::MAX")
    }
}

impl std::error::Error for RangeFull {}

impl purrdf_hash::error::StructuredError for RangeFull {
    fn visit<'error>(&'error self, visitor: &mut dyn purrdf_hash::error::ErrorVisitor<'error>) {
        visitor.variant("gts-v2-range-full");
        visitor.value(
            Some("maximum"),
            purrdf_hash::error::ErrorValue::Unsigned(u128::from(u64::MAX)),
        );
    }
}

/// The admitted claim of a commit frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitClaim<'a> {
    /// Declared number of ordinary frames preceding the commit.
    pub count: u64,
    /// Declared root of the preceding range.
    pub root: &'a Digest,
}

/// The result of checking a commit against the observed prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitCheck {
    /// Number of ordinary frames preceding the commit.
    pub actual_count: u64,
    /// The claim agrees with the observed count.
    pub count_matches: bool,
    /// The claim agrees with the root over the observed prefix.
    pub root_matches: bool,
    /// Both claims match and the commit link is linked or healed.
    pub valid: bool,
}

impl Range {
    /// Begin a range with the computed identifier of the opener.
    #[must_use]
    pub fn new(opener: &Digest) -> Self {
        let mut peaks = [[0; DIGEST_LENGTH]; u64::BITS as usize];
        peaks[0] = leaf(opener);
        Self {
            count: NonZeroU64::MIN,
            peaks,
        }
    }

    /// Count the opener and all admitted ordinary frame identifiers.
    #[must_use]
    pub const fn count(&self) -> NonZeroU64 {
        self.count
    }

    /// Append one effective identifier of an ordinary frame.
    ///
    /// # Errors
    /// Returns `RangeFull` before changing the range if its count would overflow.
    pub fn append(&mut self, identifier: &Digest) -> Result<(), RangeFull> {
        let next = self.count.checked_add(1).ok_or(RangeFull)?;
        let height = self.count.get().trailing_ones() as usize;
        let mut right = leaf(identifier);
        for left in &self.peaks[..height] {
            right = node(left, &right);
        }
        self.peaks[height] = right;
        self.count = next;
        Ok(())
    }

    /// Compute the root over the occupied peaks in descending height order.
    #[must_use]
    pub fn root(&self) -> Digest {
        let count = self.count.get();
        let mut hasher = Hasher::new_derive_key("gts2 range root");
        update_head(&mut hasher, head::ARRAY, 2);
        update_head(&mut hasher, head::UNSIGNED, count);
        update_head(&mut hasher, head::ARRAY, u64::from(count.count_ones()));
        let mut bits = count;
        while bits != 0 {
            let height = u64::BITS - 1 - bits.leading_zeros();
            update_digest(&mut hasher, &self.peaks[height as usize]);
            bits ^= 1 << height;
        }
        *hasher.finalize().as_bytes()
    }

    /// Judge a commit against the range immediately before it.
    #[must_use]
    pub fn check_commit(&self, claim: CommitClaim<'_>, link: LinkState) -> CommitCheck {
        let actual_count = self.count.get() - 1;
        let count_matches = actual_count == claim.count;
        let root_matches = self.root() == *claim.root;
        CommitCheck {
            actual_count,
            count_matches,
            root_matches,
            valid: count_matches && root_matches && link != LinkState::Broken,
        }
    }
}

fn leaf(identifier: &Digest) -> Digest {
    let mut hasher = Hasher::new_derive_key("gts2 range leaf");
    update_head(&mut hasher, head::ARRAY, 1);
    update_digest(&mut hasher, identifier);
    *hasher.finalize().as_bytes()
}

fn node(left: &Digest, right: &Digest) -> Digest {
    let mut hasher = Hasher::new_derive_key("gts2 range node");
    update_head(&mut hasher, head::ARRAY, 2);
    update_digest(&mut hasher, left);
    update_digest(&mut hasher, right);
    *hasher.finalize().as_bytes()
}

fn update_head(hasher: &mut Hasher, major: u8, argument: u64) {
    head::with_encoded_head(major, argument, |bytes| {
        hasher.update(bytes);
    });
}

fn update_digest(hasher: &mut Hasher, digest: &Digest) {
    update_head(hasher, head::BYTES, DIGEST_LENGTH as u64);
    hasher.update(digest);
}
