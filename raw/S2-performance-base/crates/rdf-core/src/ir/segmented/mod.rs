// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Certified, provider-backed RDF blocks and bounded read sessions.
//!
//! This versioned representation preserves its dictionary's insertion IDs. It is
//! independent of the eager canonical pack's ID authority. Construction and full
//! certification are explicit resident operations; opening and reading a certified
//! snapshot keep no whole-dictionary, block-directory or translation array.

mod build;
mod format;
mod session;

pub use build::{SegmentedBuildLimits, SegmentedBuilder, SegmentedImage};
pub use session::{
    SegmentedEvidence, SegmentedReadLimits, SegmentedRequest, SegmentedReservation,
    SegmentedSession, SegmentedTermGuard,
};

use std::sync::Arc;

use crate::StopCause;
use purrdf_hash::Domain;

/// The representation identity law, including dictionary IDs, all RDF streams,
/// empty graphs, block indexes, and the format/profile header.
pub const SEGMENTED_SNAPSHOT_DOMAIN: Domain = Domain::new(b"purrdf-segmented-snapshot-v1\0");
/// The leaf law binding a logical block position to its complete bytes.
pub const SEGMENTED_BLOCK_DOMAIN: Domain = Domain::new(b"purrdf-segmented-block-v1\0");
/// The binary authenticated-index node law.
pub const SEGMENTED_NODE_DOMAIN: Domain = Domain::new(b"purrdf-segmented-node-v1\0");

/// The exact ordered provider-request evidence law.
pub const SEGMENTED_REQUESTS_DOMAIN: Domain = Domain::new(b"purrdf-segmented-requests-v1\0");

/// Exact identity of one immutable persisted representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SegmentedSnapshot([u8; 32]);

impl SegmentedSnapshot {
    /// The complete identity bytes. This is representation identity, not RDFC identity.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// Host-owned authority for a persisted full-certification receipt. Authentication
/// must attest that these exact bytes came from successful complete certification,
/// under the same format/profile law. A checksum of an unvalidated data file is
/// insufficient: it authenticates bytes without proving index truth or RDF closure.
///
/// Hosts may pin an independently retained manifest or verify a signed certification
/// statement. The portable kernel performs no network, key lookup, or filesystem I/O.
pub trait SegmentedReceiptAuthority {
    /// Authenticate the exact canonical receipt bytes before they are decoded.
    ///
    /// # Errors
    /// Refuse untrusted, stale, mismatched or unauthenticated certification claims.
    fn authenticate(&self, receipt: &[u8]) -> Result<(), SegmentedError>;
}

/// An external term attachment binds its compact ID to one certified generation.
/// Joins retain only [`crate::GlobalTermId`] after this ingress check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SegmentedHandle {
    snapshot: SegmentedSnapshot,
    id: crate::GlobalTermId,
}
impl SegmentedHandle {
    /// Exact width of a generation-qualified external ID.
    pub const ENCODED_BYTES: usize = 40;

    /// Persist the generation and exact logical ordinal without numeric rounding.
    #[must_use]
    pub fn encode(self) -> [u8; Self::ENCODED_BYTES] {
        let mut bytes = [0_u8; Self::ENCODED_BYTES];
        bytes[..32].copy_from_slice(self.snapshot.as_bytes());
        crate::bytes::put_u64_le(&mut bytes, 32, self.id.index()).expect("fixed handle width");
        bytes
    }

    /// Decode an external attachment. This does not authenticate the claimed
    /// generation: [`SegmentedSession::attach`] must accept it before the compact
    /// ID is used in a query or term read.
    ///
    /// # Errors
    /// Refuses malformed lengths and the reserved terminal ordinal.
    pub fn from_encoded_bytes(bytes: &[u8]) -> Result<Self, SegmentedError> {
        if bytes.len() != Self::ENCODED_BYTES {
            return Err(SegmentedError::Corrupt(
                "wrong external term attachment width",
            ));
        }
        let snapshot = SegmentedSnapshot(
            bytes[..32]
                .try_into()
                .map_err(|_| SegmentedError::Corrupt("truncated term generation"))?,
        );
        let id = crate::bytes::read_u64_le(bytes, 32)
            .and_then(crate::GlobalTermId::checked_from_index)
            .ok_or(SegmentedError::AddressExhausted)?;
        Ok(Self { snapshot, id })
    }

    /// Certified generation of the attachment.
    #[must_use]
    pub const fn snapshot(self) -> SegmentedSnapshot {
        self.snapshot
    }
    /// Compact ID, valid only in that generation.
    #[must_use]
    pub const fn id(self) -> crate::GlobalTermId {
        self.id
    }
}

/// Typed refusal at the portable storage/session boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SegmentedError {
    /// The host could not complete a requested read.
    Provider {
        /// Fixed host operation label; retaining an error never allocates text.
        operation: &'static str,
        /// Optional host-native numeric failure code, interpreted by that provider.
        host_code: Option<i32>,
    },
    /// Bytes or index claims disagree with the certified representation.
    Corrupt(&'static str),
    /// The provider exposes another immutable representation.
    SnapshotMismatch,
    /// A logical address, ID, or local buffer size overflowed its checked width.
    AddressExhausted,
    /// The requested live residency exceeds the caller's ceiling.
    Residency {
        /// Proposed live/cumulative charge at the admission boundary.
        requested: u64,
        /// Inclusive caller-selected ceiling.
        limit: u64,
    },
    /// Cumulative provider I/O would exceed its independent work ceiling.
    IoBytes {
        /// Cumulative bytes after the proposed read.
        requested: u64,
        /// Inclusive I/O ceiling.
        limit: u64,
    },
    /// Cache eviction cannot make room while outstanding term guards pin blocks.
    PinnedBlocks,
    /// The exact request ledger is full; the refused read was not issued.
    EvidenceExhausted,
    /// Caller-selected bounded construction cannot admit this input.
    ConstructionLimit,
    /// Host cancellation or deadline refusal, without reading a clock in the kernel.
    Stopped(StopCause),
}

impl std::fmt::Display for SegmentedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Provider {
                operation,
                host_code,
            } => {
                write!(f, "block provider operation failed: {operation}")?;
                if let Some(code) = host_code {
                    write!(f, " (host code {code})")?;
                }
                Ok(())
            }
            Self::Corrupt(message) => write!(f, "invalid segmented snapshot: {message}"),
            Self::SnapshotMismatch => f.write_str("segmented snapshot identity mismatch"),
            Self::AddressExhausted => f.write_str("segmented logical address exhausted"),
            Self::Residency { requested, limit } => {
                write!(f, "segmented live residency {requested} exceeds {limit}")
            }
            Self::IoBytes { requested, limit } => {
                write!(f, "segmented cumulative I/O {requested} exceeds {limit}")
            }
            Self::PinnedBlocks => f.write_str("segmented cache admission is blocked by live pins"),
            Self::EvidenceExhausted => f.write_str("segmented exact request ledger exhausted"),
            Self::ConstructionLimit => {
                f.write_str("segmented resident construction limit exceeded")
            }
            Self::Stopped(cause) => write!(f, "segmented read stopped: {}", cause.label()),
        }
    }
}
impl std::error::Error for SegmentedError {}

/// A streamed RDF line export distinguishes source completeness from sink failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum SegmentedExportError {
    /// The source could not be read completely; abort the unpublished output.
    Source(SegmentedError),
    /// The output drain refused bytes; abort the unpublished output.
    Output(crate::sink::DrainError),
}
impl std::fmt::Display for SegmentedExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(error) => error.fmt(f),
            Self::Output(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for SegmentedExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            Self::Output(error) => Some(error),
        }
    }
}

/// Portable immutable byte provider. Logical positions remain `u64` on every host.
///
/// Implementations may use files, object ranges or browser storage. `read_at` must
/// fill exactly `output.len()` bytes or return an error. The snapshot identity and
/// length must stay fixed for the provider's lifetime. Each admitted block is also
/// authenticated against the caller's certification receipt, so this contract
/// cannot authorize changed or maliciously substituted bytes.
pub trait SegmentedProvider: std::fmt::Debug + Send + Sync {
    /// Identity promised by this immutable provider.
    fn snapshot(&self) -> SegmentedSnapshot;
    /// Total encoded byte length, independent of pointer width.
    fn byte_len(&self) -> u64;
    /// Read a bounded caller-owned destination without allocating another response.
    ///
    /// # Errors
    /// Returns a typed host failure, cancellation, truncation or invalid address.
    fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError>;
}

/// Full validation's authority for subsequent selective reopening.
///
/// Fields and construction are private: a checksum copied from unvalidated input
/// cannot mint this authority. Cloning a receipt retains the same immutable
/// representation. Keep it alongside the provider when reopening; a new untrusted
/// image must pass [`SegmentedImage::certify`] before it can produce complete reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentedReceipt {
    snapshot: SegmentedSnapshot,
    root: [u8; 32],
    block_bytes: u32,
    block_count: u64,
    leaf_count: u64,
    byte_len: u64,
}

impl SegmentedReceipt {
    /// Fixed byte length of the v1 full-certification receipt.
    pub const ENCODED_BYTES: usize = 104;

    /// Encode the complete authority statement without a heap allocation. Store it
    /// through an authenticated host manifest independently of mutable data bytes.
    #[must_use]
    pub fn encode(&self) -> [u8; Self::ENCODED_BYTES] {
        let mut out = [0_u8; Self::ENCODED_BYTES];
        out[..8].copy_from_slice(b"PURRCRT1");
        out[8..40].copy_from_slice(self.snapshot.as_bytes());
        out[40..72].copy_from_slice(&self.root);
        crate::bytes::put_u32_le(&mut out, 72, self.block_bytes).expect("fixed receipt width");
        crate::bytes::put_u64_le(&mut out, 76, self.block_count).expect("fixed receipt width");
        crate::bytes::put_u64_le(&mut out, 84, self.leaf_count).expect("fixed receipt width");
        crate::bytes::put_u64_le(&mut out, 92, self.byte_len).expect("fixed receipt width");
        crate::bytes::put_u32_le(&mut out, 100, 1_u32).expect("fixed receipt width");
        out
    }

    /// Restore persisted authority only after explicit host authentication. This
    /// authenticates the certificate, not merely the encoded data-file checksum.
    ///
    /// # Errors
    /// Refuses failed authentication, unsupported profiles, inconsistent roots,
    /// terminal addresses or malformed persisted lengths before any data is read.
    pub fn from_authenticated_bytes(
        bytes: &[u8],
        authority: &impl SegmentedReceiptAuthority,
    ) -> Result<Self, SegmentedError> {
        authority.authenticate(bytes)?;
        if bytes.len() != Self::ENCODED_BYTES
            || bytes.get(..8) != Some(b"PURRCRT1".as_slice())
            || crate::bytes::read_u32_le(bytes, 100) != Some(1)
        {
            return Err(SegmentedError::Corrupt("unsupported certification receipt"));
        }
        let snapshot = SegmentedSnapshot(
            bytes[8..40]
                .try_into()
                .map_err(|_| SegmentedError::Corrupt("truncated receipt identity"))?,
        );
        let root = bytes[40..72]
            .try_into()
            .map_err(|_| SegmentedError::Corrupt("truncated receipt root"))?;
        let block_bytes = crate::bytes::read_u32_le(bytes, 72)
            .ok_or(SegmentedError::Corrupt("truncated receipt block width"))?;
        let block_count = crate::bytes::read_u64_le(bytes, 76)
            .ok_or(SegmentedError::Corrupt("truncated receipt count"))?;
        let leaf_count = crate::bytes::read_u64_le(bytes, 84)
            .ok_or(SegmentedError::Corrupt("truncated receipt tree width"))?;
        let byte_len = crate::bytes::read_u64_le(bytes, 92)
            .ok_or(SegmentedError::Corrupt("truncated receipt byte length"))?;
        let expected = block_count
            .checked_mul(u64::from(block_bytes))
            .and_then(|n| {
                leaf_count
                    .checked_mul(64)
                    .and_then(|tree| n.checked_add(tree))
            })
            .ok_or(SegmentedError::AddressExhausted)?;
        if !(512..=1_048_576).contains(&block_bytes)
            || block_count == 0
            || block_count.checked_next_power_of_two() != Some(leaf_count)
            || byte_len != expected
            || snapshot != format::snapshot_digest(&root, block_bytes, block_count)
        {
            return Err(SegmentedError::Corrupt(
                "inconsistent full-certification receipt",
            ));
        }
        Ok(Self {
            snapshot,
            root,
            block_bytes,
            block_count,
            leaf_count,
            byte_len,
        })
    }
    /// Certified representation identity.
    #[must_use]
    pub const fn snapshot(&self) -> SegmentedSnapshot {
        self.snapshot
    }
    /// Persisted byte length covered by this receipt.
    #[must_use]
    pub const fn byte_len(&self) -> u64 {
        self.byte_len
    }
    /// Number of fixed-size authenticated blocks.
    #[must_use]
    pub const fn block_count(&self) -> u64 {
        self.block_count
    }
}

/// A bounded resident byte provider, useful for fixtures and browser-owned bytes.
/// The source bytes are outside a reader's residency budget; the reader's copies,
/// pins, metadata, scratch and exact evidence are inside it.
#[derive(Debug, Clone)]
pub struct SegmentedBytes {
    bytes: Arc<[u8]>,
    snapshot: SegmentedSnapshot,
}

impl SegmentedBytes {
    /// Attach bytes to their representation identity. Attachment is checked against
    /// the certification receipt at open, and every actual read is authenticated.
    #[must_use]
    pub fn new(bytes: Arc<[u8]>, snapshot: SegmentedSnapshot) -> Self {
        Self { bytes, snapshot }
    }
}
impl SegmentedProvider for SegmentedBytes {
    fn snapshot(&self) -> SegmentedSnapshot {
        self.snapshot
    }
    fn byte_len(&self) -> u64 {
        u64::try_from(self.bytes.len()).expect("resident byte length fits u64")
    }
    fn read_at(&self, position: u64, output: &mut [u8]) -> Result<(), SegmentedError> {
        let start = usize::try_from(position).map_err(|_| SegmentedError::AddressExhausted)?;
        let end = start
            .checked_add(output.len())
            .ok_or(SegmentedError::AddressExhausted)?;
        output.copy_from_slice(
            self.bytes
                .get(start..end)
                .ok_or(SegmentedError::Corrupt("truncated provider range"))?,
        );
        Ok(())
    }
}
