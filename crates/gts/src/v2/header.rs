// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use std::io::{self, Write};

use purrdf_lex::cbor::head;

use super::fields::{Fields, WireError, write_option};
use super::{
    BLAKE3_256, DIGEST_LENGTH, GENERATION, HEADER_KIND, Item, MalformedReason, REGISTRY_VERSION,
};

pub(super) const HEADER_CONTEXT: &str = "gts2 header";

mod resync;

pub use resync::{ResyncCandidate, ResyncRefusal, ResyncScan};

/// P2 profile codes for authored headers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum Profile {
    /// Distribution file.
    Dist = 1,
    /// Signed custody chain.
    Evidence = 2,
    /// Live event stream.
    Stream = 3,
    /// Nested files.
    Bundle = 4,
    /// File tree.
    Files = 5,
    /// Container archive.
    Container = 6,
}

/// Canonical form claims available to the writer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum CanonicalForm {
    /// Canonical form with blank-node labelling version 1.
    V1 = 1,
}

/// The header's declared link to a previous segment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lineage<D> {
    /// Previous segment head in the selected digest algorithm.
    pub previous_head: D,
    /// The declared lineage ordinal.
    pub ordinal: NonZeroU64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::v2) struct ResyncDeclaration<'a> {
    pub(in crate::v2) offset: u64,
    pub(in crate::v2) past: Option<&'a [u8]>,
}

/// An accepted P1 §2.4 header and its original lineage digest bytes.
///
/// A newer registry can name an unavailable hash algorithm. Such a header
/// retains its fields and has no identifier. A profile number is independent
/// of the registry and its admission makes no claim of profile validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header<'a> {
    registry_version: NonZeroU64,
    hash_algorithm: NonZeroU64,
    profile: Option<NonZeroU64>,
    canonical: Option<NonZeroU64>,
    lineage: Option<Lineage<&'a [u8]>>,
    resync: Option<ResyncDeclaration<'a>>,
    identifier: Option<[u8; DIGEST_LENGTH]>,
}

impl<'a> Header<'a> {
    /// Decode the fields of a structurally framed header in wire order.
    ///
    /// # Errors
    /// Returns the first closed-grammar violation, including its byte offset
    /// relative to the start of the item.
    pub fn decode(item: Item<'a>) -> Result<Self, WireError> {
        let mut reader = Fields::new(item.bytes());
        reader.record(7)?;
        reader.fixed_uint(HEADER_KIND, MalformedReason::WrongType)?;
        reader.fixed_uint(GENERATION, MalformedReason::OutOfRange)?;
        let (registry_version, hash_algorithm) = read_registry_hash(&mut reader, false)?;
        let profile = read_profile(&mut reader)?;
        let holds_hash = hash_algorithm.get() == BLAKE3_256;
        let canonical = reader.option(|reader| {
            reader.registry_value(registry_version, u64::from(CanonicalForm::V1 as u8))
        })?;
        let lineage = read_lineage(&mut reader, holds_hash)?;
        Ok(Self {
            registry_version,
            hash_algorithm,
            profile,
            canonical,
            lineage,
            resync: None,
            identifier: holds_hash
                .then(|| purrdf_hash::blake3::derive_key(HEADER_CONTEXT, item.bytes())),
        })
    }

    /// The segment's declared registry version.
    #[must_use]
    pub const fn registry_version(self) -> NonZeroU64 {
        self.registry_version
    }

    /// The segment's declared hash algorithm code.
    #[must_use]
    pub const fn hash_algorithm(self) -> NonZeroU64 {
        self.hash_algorithm
    }

    /// The declared profile code, which may be unknown to this reader.
    #[must_use]
    pub const fn profile(self) -> Option<NonZeroU64> {
        self.profile
    }

    /// The optional canonical form claim as a registry code.
    #[must_use]
    pub const fn canonical(self) -> Option<NonZeroU64> {
        self.canonical
    }

    /// The declared lineage, before any comparison with a previous segment.
    #[must_use]
    pub const fn lineage(self) -> Option<Lineage<&'a [u8]>> {
        self.lineage
    }

    pub(in crate::v2) const fn resync(self) -> Option<ResyncDeclaration<'a>> {
        self.resync
    }

    /// The segment's opening identifier when its hash algorithm is available.
    #[must_use]
    pub const fn identifier(self) -> Option<[u8; DIGEST_LENGTH]> {
        self.identifier
    }
}

fn read_registry_hash(
    fields: &mut Fields<'_>,
    require_held_hash: bool,
) -> Result<(NonZeroU64, NonZeroU64), WireError> {
    let registry = fields.positive(MalformedReason::OutOfRange)?;
    let hash_registry = if require_held_hash {
        NonZeroU64::MIN
    } else {
        registry
    };
    let algorithm = fields.registry_value(hash_registry, BLAKE3_256)?;
    Ok((registry, algorithm))
}

fn read_profile(fields: &mut Fields<'_>) -> Result<Option<NonZeroU64>, WireError> {
    fields.option(|fields| fields.positive(MalformedReason::UnregisteredValue))
}

fn read_digest<'a>(fields: &mut Fields<'a>, holds_hash: bool) -> Result<&'a [u8], WireError> {
    let offset = fields.offset();
    let bytes = fields.bytes()?;
    if holds_hash && bytes.len() != DIGEST_LENGTH {
        return Err(WireError::new(MalformedReason::WrongLength, offset));
    }
    Ok(bytes)
}

fn read_lineage<'a>(
    fields: &mut Fields<'a>,
    holds_hash: bool,
) -> Result<Option<Lineage<&'a [u8]>>, WireError> {
    fields.option(|fields| {
        fields.record(2)?;
        let previous_head = read_digest(fields, holds_hash)?;
        let ordinal = fields.positive(MalformedReason::OutOfRange)?;
        Ok(Lineage {
            previous_head,
            ordinal,
        })
    })
}

/// Write one deterministic registry version 1 header.
///
/// The caller supplies any previously checked lineage and selects any
/// canonical form claim according to the rest of the segment it writes.
///
/// # Errors
/// Returns the writer's original I/O error.
pub fn write_header<W: Write + ?Sized>(
    writer: &mut W,
    profile: Option<Profile>,
    canonical: Option<CanonicalForm>,
    lineage: Option<Lineage<&[u8; DIGEST_LENGTH]>>,
) -> io::Result<()> {
    head::write_head(writer, head::ARRAY, 7)?;
    for value in [HEADER_KIND, GENERATION, REGISTRY_VERSION, BLAKE3_256] {
        head::write_head(writer, head::UNSIGNED, value)?;
    }
    write_option(writer, profile, |writer, profile| {
        head::write_head(writer, head::UNSIGNED, u64::from(profile as u8))
    })?;
    write_option(writer, canonical, |writer, canonical| {
        head::write_head(writer, head::UNSIGNED, u64::from(canonical as u8))
    })?;
    write_option(writer, lineage, |writer, lineage| {
        head::write_head(writer, head::ARRAY, 2)?;
        head::write_bytes(writer, lineage.previous_head)?;
        head::write_head(writer, head::UNSIGNED, lineage.ordinal.get())
    })
}
