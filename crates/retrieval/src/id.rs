// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The plan's versioned, domain-separated content identity.

use core::fmt;

/// The canonical plan layout this build writes and understands.
///
/// A plan's encoded form begins with this value. A decoder that reads any other
/// version refuses with [`PlanError::VersionMismatch`](crate::PlanError::VersionMismatch)
/// rather than reinterpret the bytes under a layout they were not written for.
///
/// # Why this is 2
///
/// Version 1 carried a per-stratum weight map between the depths and the
/// statistics snapshot. Nothing read it: the planner wrote the identity weight
/// for every stratum unconditionally, and the weights that actually fuse an
/// answer come from a [`FusionProfile`](crate::FusionProfile), which §8 of the
/// design record deliberately keeps out of the plan. A plan recording a weight
/// of one and a profile weighting that stratum a thousandth both admitted and
/// fused with nothing reconciling them, so the field made plan identity
/// sensitive to a number that decided nothing. It is gone.
///
/// Removing a field is not the append-only change a new discriminator byte is
/// (see `plan.rs`'s tag space, which grows without moving this value). The
/// weight map sat *inside* the layout, so a version-1 plan's remaining bytes lie
/// at different offsets under version 2: a decoder reading those bytes would
/// take the old weight count for the statistics source's length and answer with
/// a plan nobody wrote. The version is therefore bumped so the decoder refuses
/// the old layout by name — `VersionMismatch { found: 1, expected: 2 }` — rather
/// than mis-reading it.
pub const PLAN_VERSION: u16 = 2;

/// The domain-separation prefix mixed into every [`PlanId`].
///
/// Two different kinds of document hashed with the same algorithm would collide
/// only if their bytes did, but prefixing the domain is what makes a plan digest
/// *not* a digest of the same bytes under any other purpose — the discipline the
/// crate's other content identities follow.
pub const PLAN_ID_DOMAIN: &str = "purrdf:plan:v1";

/// The length of a [`PlanId`] digest in bytes.
pub const PLAN_ID_BYTES: usize = 32;

/// A plan's content identity: a domain-separated BLAKE3 digest over its
/// canonical bytes.
///
/// The identity is derived from canonical bytes, never from `Hash` or from a
/// serde document: two plans are the same plan iff their canonical encodings are
/// byte-identical, which is exactly when their ids are equal. A changed field is
/// a changed plan and therefore a changed id.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlanId([u8; PLAN_ID_BYTES]);

impl PlanId {
    /// Digest canonical plan `bytes` under the plan domain.
    #[must_use]
    pub fn from_canonical(bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(PLAN_ID_DOMAIN.as_bytes());
        hasher.update(&[0u8]);
        hasher.update(bytes);
        Self(*hasher.finalize().as_bytes())
    }

    /// The raw 32 digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; PLAN_ID_BYTES] {
        &self.0
    }

    /// The lowercase-hex rendering of the digest (64 characters).
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(PLAN_ID_BYTES * 2);
        for byte in &self.0 {
            use fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}

impl fmt::Debug for PlanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "PlanId({})", self.to_hex())
    }
}

impl fmt::Display for PlanId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// The canonical fusion-profile layout this build writes and understands.
///
/// A profile's encoded form begins with this value. As with a plan, a decoder
/// that reads any other version refuses rather than reinterpret the bytes under
/// a layout they were not written for.
pub const FUSION_PROFILE_VERSION: u16 = 1;

/// The domain-separation prefix mixed into every [`FusionProfileId`].
///
/// Namespaced apart from [`PLAN_ID_DOMAIN`] so the same canonical bytes hashed
/// for a plan and for a fusion profile never produce the same identity.
pub const FUSION_PROFILE_ID_DOMAIN: &str = "purrdf:fusion-profile:v1";

/// The length of a [`FusionProfileId`] digest in bytes.
pub const FUSION_PROFILE_ID_BYTES: usize = 32;

/// A fusion profile's content identity: a domain-separated BLAKE3 digest over
/// its canonical bytes.
///
/// Identity is what makes a profile a law rather than a knob. Two answers fused
/// under profiles with different `K`, different weights, a different decay rule
/// or a different tie-break are answers to different questions, and the digest
/// is how a report names which law was in force.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FusionProfileId([u8; FUSION_PROFILE_ID_BYTES]);

impl FusionProfileId {
    /// Digest canonical fusion-profile `bytes` under the fusion-profile domain.
    #[must_use]
    pub fn from_canonical(bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(FUSION_PROFILE_ID_DOMAIN.as_bytes());
        hasher.update(&[0u8]);
        hasher.update(bytes);
        Self(*hasher.finalize().as_bytes())
    }

    /// The raw 32 digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; FUSION_PROFILE_ID_BYTES] {
        &self.0
    }

    /// The lowercase-hex rendering of the digest (64 characters).
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(FUSION_PROFILE_ID_BYTES * 2);
        for byte in &self.0 {
            use fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}

impl fmt::Debug for FusionProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FusionProfileId({})", self.to_hex())
    }
}

impl fmt::Display for FusionProfileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}
