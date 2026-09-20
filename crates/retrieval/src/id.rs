// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The versioned, domain-separated content identities this crate issues: the
//! plan, the fusion profile, and the evidence an answer was produced against.
//!
//! All three are built the same way — a BLAKE3 digest over canonical,
//! length-framed bytes, prefixed by a domain string of its own — and the
//! sameness is deliberate. Each answers a different question about an answer
//! (which question, under which law, against which index generations), and each
//! must be comparable to its own kind and never accidentally equal to another
//! kind, which is what the per-domain prefix buys.

use core::fmt;

/// The canonical plan layout this build writes and understands.
///
/// A plan's encoded form begins with this value. A decoder that reads any other
/// version refuses with [`PlanError::VersionMismatch`](crate::PlanError::VersionMismatch)
/// rather than reinterpret the bytes under a layout they were not written for.
///
/// # Why this is 4
///
/// Version 4 records, per stratum, **every input that stratum's depth was
/// derived from** — the registry's declared row bound, the reported cardinality,
/// the selectivity that was actually applied together with the request terms it
/// aggregates over, and whether the request's row bound was licensed to bound
/// this stratum at all. A version-3 plan recorded the depth and roughly half of
/// what produced it, so the number could be read but not checked; version 4
/// makes it recomputable, which is what
/// [`Plan::certify`](crate::Plan::certify) does.
///
/// That completes the argument version 3 began rather than opening a new one.
/// Version 3 appended the [`ReadBound`](crate::ReadBound) because a plan that did
/// not record it "recorded depths whose derivation could not be reconstructed" —
/// but the bound is what the caller *asked for*, and whether it was allowed to
/// bound a given stratum is decided separately, from the shape of the surviving
/// declarations. Two plans could therefore agree on every recorded field and
/// still have derived their depths from different numbers. They cannot now.
///
/// Version 4 also makes a statistics entry's cardinality optional, under the
/// same presence discriminator its selectivity already used, so a provider that
/// reported only a selectivity is recorded rather than dropped. Dropping it was
/// the same defect one level down: the depth moved and the evidence did not.
///
/// This is **not** the append-only change a new discriminator byte is. The
/// derivations sit *inside* the layout, between the depths and the statistics,
/// and the new presence tag sits inside each statistics entry — so a version-3
/// document's remaining bytes lie at different offsets, and a decoder reading
/// them under this layout would take a cardinality's high bytes for a presence
/// tag and answer with a plan nobody wrote. Version 4 refuses it by name instead
/// — `VersionMismatch { found: 3, expected: 4 }`.
///
/// # Why version 3 was not 2
///
/// Version 3 appends the request's [`ReadBound`](crate::ReadBound) after the
/// per-term unserved evidence. That bound is what every stratum depth is derived
/// *from*, so a plan that did not record it recorded depths whose derivation
/// could not be reconstructed — and two plans that read a different number of
/// rows shared one identity. Appending it is not the append-only change a new
/// discriminator byte is either: a version-2 plan's bytes simply end where the
/// bound would begin, and a decoder that read them under this layout would run
/// off the end of a document it should have refused by name. Version 3 refuses it
/// by name instead — `VersionMismatch { found: 2, expected: 3 }`.
///
/// # Why version 2 was not 1
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
/// a plan nobody wrote. The version was therefore bumped so the decoder refuses
/// the old layout by name rather than mis-reading it, which is the same reason it
/// moved again above.
pub const PLAN_VERSION: u16 = 4;

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

/// The canonical evidence layout this build writes and understands.
///
/// An [`EvidenceId`]'s canonical bytes begin with this value, for the same
/// reason a plan's and a profile's do: the layout is a document, and a document
/// that grows or loses a field at a fixed offset must be refused by name rather
/// than re-read under a shape it was not written for. Nothing decodes these
/// bytes today — an attestation map is derived at the end of a fusion and
/// digested on the spot, never transported — so the version is here to keep a
/// later reader honest rather than to serve one now. Writing it costs two bytes
/// and buys the identity a stated shape; omitting it would mean a future field
/// silently re-issuing every identity ever handed out under the old one.
pub const EVIDENCE_VERSION: u16 = 1;

/// The domain-separation prefix mixed into every [`EvidenceId`].
///
/// Namespaced apart from [`PLAN_ID_DOMAIN`] and [`FUSION_PROFILE_ID_DOMAIN`] so
/// that the same canonical bytes read as evidence, as a plan and as a profile
/// never produce the same identity — the three answer different questions and
/// must never compare equal by accident.
pub const EVIDENCE_ID_DOMAIN: &str = "purrdf:evidence:v1";

/// The length of an [`EvidenceId`] digest in bytes.
pub const EVIDENCE_ID_BYTES: usize = 32;

/// The evidence an answer was produced against: a domain-separated BLAKE3
/// digest over the canonical bytes of its per-stratum attestation map.
///
/// # Why this exists
///
/// An answer is reproducible only against a named index generation, and the
/// two identities that came before this one cannot name it.
/// [`PlanId`] pins the question — which producers, which terms, which depths —
/// and [`FusionProfileId`] pins the law the rows were fused under. Both are
/// derived from configuration, and configuration is exactly what does *not*
/// change when an index is rebuilt underneath a running system. So the same
/// plan, under the same profile, over a rebuilt index produces different rows
/// while both existing identities stay byte-identical. Two such answers look
/// comparable and are not, and nothing in the report says so.
///
/// This is the third identity, and it closes that gap: it is a pure function of
/// what every stratum's producer attested about the index that answered it —
/// which generation, and whether that generation was whole. Two answers are
/// comparable iff all three identities agree, which is **one** equality
/// comparison over a triple rather than a map-by-map diff a caller would have
/// to write, get subtly wrong, and disagree with the next caller about.
///
/// # What a difference does and does not mean
///
/// Unequal ids mean the evidence differed; they do not rank the two answers,
/// because the generation strings they are built from are the hosts' own
/// spellings, recorded verbatim and never parsed or ordered
/// (`purrdf_sparql_eval::IndexGeneration`). Nothing here can say which of two
/// generations is newer, and inventing an order over opaque host strings is
/// precisely the fabrication the attestation seam refuses.
///
/// Equal ids likewise mean the attestations were identical — not that the
/// indexes were whole. A fusion over producers that all declared nothing has a
/// perfectly stable identity over the map of silences, and silence is not a
/// certificate (`purrdf_sparql_eval::ServiceLevel` has no `Whole` variant on
/// purpose). The wholeness question is answered separately and per answer by
/// [`ScoreExactness`](crate::ScoreExactness).
///
/// # Why it is derived from canonical bytes
///
/// Same discipline as [`PlanId`]: the digest is taken over a sorted,
/// length-framed encoding built by this crate's canonical writer, never over a
/// `Hash` or a serde document. Length framing is what makes the encoding
/// injective, so a stratum named `ex:a` attesting generation `bc` cannot encode
/// to the same bytes as one named `ex:ab` attesting `c`; sorting by stratum
/// makes it independent of any iteration order; little-endian integers make it
/// identical on every target, wasm32 included. Two evidence maps are the same
/// evidence iff their bytes are, which is exactly when their ids are equal.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceId([u8; EVIDENCE_ID_BYTES]);

impl EvidenceId {
    /// Digest canonical evidence `bytes` under the evidence domain.
    #[must_use]
    pub fn from_canonical(bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(EVIDENCE_ID_DOMAIN.as_bytes());
        hasher.update(&[0u8]);
        hasher.update(bytes);
        Self(*hasher.finalize().as_bytes())
    }

    /// The raw 32 digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; EVIDENCE_ID_BYTES] {
        &self.0
    }

    /// The lowercase-hex rendering of the digest (64 characters).
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut out = String::with_capacity(EVIDENCE_ID_BYTES * 2);
        for byte in &self.0 {
            use fmt::Write as _;
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}

impl fmt::Debug for EvidenceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvidenceId({})", self.to_hex())
    }
}

impl fmt::Display for EvidenceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}
