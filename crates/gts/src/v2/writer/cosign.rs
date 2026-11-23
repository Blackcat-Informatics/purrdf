// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::{fmt, num::NonZeroU64};
use std::collections::TryReserveError;

use purrdf_lex::cbor::StructuralError;

use super::{Digest, SegmentWriteError, signature::SignedCommit};
use crate::cose::SigningKeyRef;
use crate::v2::{
    FrameEnvelope, Generation, Header, Item, ItemKind, REGISTRY_VERSION, Range, RangeFull,
    WireError,
};

/// An existing file could not accept a co-signer's commit.
#[derive(Debug)]
#[non_exhaustive]
pub enum CoSignError {
    /// The file has no segment.
    Empty,
    /// The first item does not dispatch to generation 2.
    Generation(Generation),
    /// The file could not be walked to a complete item boundary.
    Framing {
        /// File offset of the affected item.
        offset: usize,
        /// Original framing failure.
        cause: StructuralError,
    },
    /// The final segment header is unreadable.
    Header {
        /// File offset of the header.
        offset: usize,
        /// Original header failure.
        cause: WireError,
    },
    /// The final segment uses an unavailable link hash.
    Hash {
        /// File offset of the header.
        offset: usize,
        /// Registry code of its hash.
        algorithm: NonZeroU64,
    },
    /// The final segment uses a registry outside the writer's authority.
    Registry(NonZeroU64),
    /// The final segment is a resync scope, where a co-signer cannot append.
    ResyncScope(usize),
    /// No co-signing keys were supplied.
    NoSigner,
    /// The file buffer could not reserve space for the commit.
    Reserve(TryReserveError),
    /// The append could not be prepared.
    Write(SegmentWriteError),
}

impl fmt::Display for CoSignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the GTS file has no segment to co-sign"),
            Self::Generation(generation) => {
                write!(f, "cannot co-sign GTS generation {generation:?}")
            }
            Self::Framing { offset, cause } => {
                write!(f, "cannot walk GTS item at byte {offset} ({cause})")
            }
            Self::Header { offset, cause } => {
                write!(f, "cannot co-sign GTS header at byte {offset} ({cause})")
            }
            Self::Hash { offset, algorithm } => {
                write!(
                    f,
                    "GTS header at byte {offset} uses unavailable hash {algorithm}"
                )
            }
            Self::Registry(version) => write!(f, "cannot co-sign GTS registry version {version}"),
            Self::ResyncScope(offset) => {
                write!(f, "cannot co-sign GTS resync scope at byte {offset}")
            }
            Self::NoSigner => f.write_str("a GTS co-signed commit needs at least one signer"),
            Self::Reserve(cause) => write!(
                f,
                "could not reserve space for the GTS co-signed commit ({cause})"
            ),
            Self::Write(cause) => cause.fmt(f),
        }
    }
}

impl std::error::Error for CoSignError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Framing { cause, .. } => Some(cause),
            Self::Header { cause, .. } => Some(cause),
            Self::Reserve(cause) => Some(cause),
            Self::Write(cause) => Some(cause),
            _ => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for CoSignError {
    purrdf_hash::error_visit! {
        Empty => "gts-v2-cosign-empty" {},
        Generation(_generation) => "gts-v2-cosign-generation" {},
        Framing { offset, cause } => "gts-v2-cosign-framing" {
            "offset" => Unsigned(*offset as u128), "cause" => Cause(cause),
        },
        Header { offset, cause } => "gts-v2-cosign-header" {
            "offset" => Unsigned(*offset as u128), "cause" => Cause(cause),
        },
        Hash { offset, algorithm } => "gts-v2-cosign-hash" {
            "offset" => Unsigned(*offset as u128), "algorithm" => Unsigned(u128::from(algorithm.get())),
        },
        Registry(version) => "gts-v2-cosign-registry" { "version" => Unsigned(u128::from(version.get())) },
        ResyncScope(offset) => "gts-v2-cosign-resync-scope" { "offset" => Unsigned(*offset as u128) },
        NoSigner => "gts-v2-cosign-no-signer" {},
        Reserve(cause) => "gts-v2-cosign-reserve" { "cause" => Cause(cause) },
        Write(cause) => "gts-v2-cosign-write" { "cause" => Cause(cause) },
    }
}

purrdf_lex::variant_from!(CoSignError { Write(SegmentWriteError) });

/// Append a signed commit over the final registry 1 header segment of an existing file.
///
/// The append walks complete items from the start and uses their effective
/// identifiers. It does not apply reader payload limits, recover gaps, or
/// interpret content. The existing bytes stay in place.
/// # Errors
/// Returns a typed refusal for unreadable framing or final header, an
/// unavailable link hash or registry, a final resync scope, or a signing or allocation
/// failure. A refusal leaves the file bytes unchanged.
pub fn append_cosigned_commit(
    file: &mut Vec<u8>,
    signers: &[SigningKeyRef<'_>],
) -> Result<Digest, CoSignError> {
    if signers.is_empty() {
        return Err(CoSignError::NoSigner);
    }
    let prefix = walk_prefix(file)?;
    if prefix.range.count().get() == u64::MAX {
        return Err(SegmentWriteError::RangeFull(RangeFull).into());
    }
    let commit = SignedCommit::prepare(&prefix.opener, &prefix.head, &prefix.range, signers)?;
    let additional = commit
        .body()
        .len()
        .saturating_add(commit.signatures().len());
    file.try_reserve(additional).map_err(CoSignError::Reserve)?;
    file.extend_from_slice(commit.body());
    file.extend_from_slice(commit.signatures());
    Ok(commit.identifier)
}

struct Prefix {
    opener: Digest,
    head: Digest,
    range: Range,
}

impl Prefix {
    fn from_header(item: Item<'_>, offset: usize) -> Result<Self, CoSignError> {
        let header = Header::decode(item).map_err(|cause| CoSignError::Header { offset, cause })?;
        let opener = header.identifier().ok_or_else(|| CoSignError::Hash {
            offset,
            algorithm: header.hash_algorithm(),
        })?;
        let version = header.registry_version();
        if version.get() != REGISTRY_VERSION {
            return Err(CoSignError::Registry(version));
        }
        Ok(Self {
            opener,
            head: opener,
            range: Range::new(&opener),
        })
    }
}

struct FramedItem<'a> {
    item: Item<'a>,
    kind: Option<ItemKind>,
    frame: Option<FrameEnvelope<'a>>,
}

fn framed_at(bytes: &[u8], offset: usize) -> Result<Option<FramedItem<'_>>, CoSignError> {
    let failure = |cause| CoSignError::Framing { offset, cause };
    let Some(item) = Item::first(&bytes[offset..]).map_err(failure)? else {
        return Ok(None);
    };
    Ok(Some(FramedItem {
        item,
        kind: item.classify(NonZeroU64::MAX).ok(),
        frame: item.frame().map_err(failure)?,
    }))
}

fn walk_prefix(bytes: &[u8]) -> Result<Prefix, CoSignError> {
    let mut offset = 0;
    let mut current = framed_at(bytes, offset)?.ok_or(CoSignError::Empty)?;
    let generation = current
        .item
        .generation()
        .map_err(|cause| CoSignError::Framing { offset, cause })?;
    if generation != Generation::Second {
        return Err(CoSignError::Generation(generation));
    }
    let mut prefix = Err(CoSignError::Empty);
    loop {
        let end = offset + current.item.bytes().len();
        let next = framed_at(bytes, end)?;
        match current.kind {
            Some(ItemKind::Header) => prefix = Prefix::from_header(current.item, offset),
            Some(ItemKind::Resync) => prefix = Err(CoSignError::ResyncScope(offset)),
            _ => {
                if let (Ok(prefix), Some(frame)) = (&mut prefix, &current.frame) {
                    let witness = next.as_ref().and_then(|item| item.frame.as_ref());
                    if let Some(identifier) =
                        frame.check_link(&prefix.head, witness).effective_identifier
                    {
                        prefix
                            .range
                            .append(&identifier)
                            .map_err(SegmentWriteError::RangeFull)?;
                        prefix.head = identifier;
                    }
                }
            }
        }
        let Some(next) = next else {
            return prefix;
        };
        current = next;
        offset = end;
    }
}
