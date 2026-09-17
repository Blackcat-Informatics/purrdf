// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The plan's versioned, domain-separated content identity.

use core::fmt;

/// The canonical plan layout this build writes and understands.
///
/// A plan's encoded form begins with this value. A decoder that reads any other
/// version refuses with [`PlanError::VersionMismatch`](crate::PlanError::VersionMismatch)
/// rather than reinterpret the bytes under a layout they were not written for.
pub const PLAN_VERSION: u16 = 1;

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
