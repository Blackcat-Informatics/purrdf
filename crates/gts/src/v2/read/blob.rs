// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use std::borrow::Cow;
use std::collections::TryReserveError;

use super::{GraphReader, ReadError, ReadFailure, Segment, Unit, VerifiedBlob};
use crate::FastMap;
use crate::v2::fields::{Fields, push};
use crate::v2::terms::read_enum;
use crate::v2::{
    BLAKE3_256, ContentError, DIGEST_LENGTH, FoldError, FrameType, MalformedReason, ValueName,
    WireError,
};

type Digest = [u8; DIGEST_LENGTH];
type Entry = (ValueName, usize);

fn content_enum(
    fields: &mut Fields<'_>,
    last: u64,
    registry: NonZeroU64,
) -> Result<u64, ReadFailure> {
    read_enum(fields, last, registry).map_err(|error| match error {
        ContentError::Malformed(error) => ReadFailure::Wire(error),
        error => ReadFailure::Content(error),
    })
}

/// The name, length and media type of an external blob declaration.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExternalBlob<'a> {
    /// BLAKE3 name and declared byte length.
    pub name: ValueName,
    /// Declared media type.
    pub media_type: Option<&'a str>,
}

/// The union of declarations of one blob in the file fold.
#[derive(Debug)]
#[non_exhaustive]
pub struct BlobEntry<'a> {
    /// The qualified name and smallest declared length.
    pub name: ValueName,
    /// Whether any declaration carries or lists the blob's bytes.
    pub present: bool,
    /// Distinct declared media types in deterministic CBOR order.
    pub media_types: Vec<Cow<'a, str>>,
}

/// The rebuild outcome of one recipe or value declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum RebuildState {
    /// Every unit and the completed bytes verified.
    Verified,
    /// No verified unit of a requested digest was available.
    MissingUnit,
    /// A verified unit had the wrong length.
    BlobLength,
    /// The completed bytes disagreed with the declared digest.
    BlobDigest,
    /// The read's total rebuild budget refused the declaration.
    Refused {
        /// The admitted total plus this declaration's length, saturated at `u64::MAX`.
        count: u64,
    },
}

/// The location and outcome of one recipe or value rebuild.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RebuildResult {
    /// File offset of the frame containing the declaration.
    pub offset: usize,
    /// Zero-based position in the pack directory, if any.
    pub member: Option<usize>,
    /// Declared digest and length.
    pub name: ValueName,
    /// Result of checking and rebuilding the declaration.
    pub state: RebuildState,
}

impl purrdf_hash::error::StructuredError for RebuildResult {
    purrdf_hash::error_visit! {
        @record self { offset, member, name, state } => "gts-v2-rebuild-result" {
            "offset" => Unsigned(self.offset as u128),
            "member" => Value(self.member.map_or(purrdf_hash::error::ErrorValue::Absent,
                |member| purrdf_hash::error::ErrorValue::Unsigned(member as u128))),
            "name" => Record(&self.name),
            "state" => Record(&self.state),
        }
    }
}

impl purrdf_hash::error::StructuredError for RebuildState {
    purrdf_hash::error_visit! {
        Verified => "gts-v2-rebuild-verified" {},
        MissingUnit => "gts-v2-rebuild-missing-unit" {},
        BlobLength => "gts-v2-rebuild-blob-length" {},
        BlobDigest => "gts-v2-rebuild-blob-digest" {},
        Refused { count } => "gts-v2-rebuild-refused" { "count" => ByteCount(u128::from(*count)) },
    }
}

pub(super) struct RebuildBudget {
    remaining: Option<u64>,
    admitted: u64,
}

impl RebuildBudget {
    pub(super) const fn new(limit: u64) -> Self {
        Self {
            remaining: Some(limit),
            admitted: 0,
        }
    }

    fn charge(&mut self, length: u64) -> Result<(), u64> {
        self.remaining = self
            .remaining
            .and_then(|remaining| remaining.checked_sub(length));
        let total = self.admitted.saturating_add(length);
        if self.remaining.is_none() {
            return Err(total);
        }
        self.admitted = total;
        Ok(())
    }
}

pub(super) struct BlobClaim<'a> {
    name: ValueName,
    present: bool,
    media_type: Option<Cow<'a, str>>,
    offset: usize,
}

#[derive(Clone, Copy)]
enum UnitKind<M> {
    Chunk,
    Blob(Option<M>),
}

#[derive(Clone, Copy)]
struct UnitDeclaration<'a> {
    digest: Digest,
    kind: UnitKind<&'a str>,
}

impl<'a> UnitDeclaration<'a> {
    fn read(fields: &mut Fields<'a>, blob: bool) -> Result<Self, WireError> {
        fields.record(if blob { 2 } else { 1 })?;
        let digest = *fields.fixed_bytes()?;
        let kind = if blob {
            UnitKind::Blob(fields.option(Fields::text)?)
        } else {
            UnitKind::Chunk
        };
        Ok(Self { digest, kind })
    }
}

#[derive(Clone, Copy)]
enum RecipeKind<M> {
    Blob(Option<M>),
    Value,
}

impl<'a> RecipeKind<&'a str> {
    fn hold<M>(
        self,
        retain: impl FnOnce(&'a str) -> Result<M, ReadFailure>,
    ) -> Result<RecipeKind<M>, ReadFailure> {
        Ok(match self {
            Self::Blob(media_type) => RecipeKind::Blob(media_type.map(retain).transpose()?),
            Self::Value => RecipeKind::Value,
        })
    }
}

#[derive(Clone, Copy)]
struct RecipeDeclaration<'a> {
    name: ValueName,
    kind: RecipeKind<&'a str>,
    entry_count: u64,
    count_offset: usize,
}

impl<'a> RecipeDeclaration<'a> {
    fn read(fields: &mut Fields<'a>, blob: bool) -> Result<Self, WireError> {
        fields.record(if blob { 4 } else { 3 })?;
        let digest = *fields.fixed_bytes()?;
        let length = fields.uint()?;
        let kind = if blob {
            RecipeKind::Blob(fields.option(Fields::text)?)
        } else {
            RecipeKind::Value
        };
        let count_offset = fields.offset();
        let entry_count = fields.uint()?;
        Ok(Self {
            name: ValueName { digest, length },
            kind,
            entry_count,
            count_offset,
        })
    }

    fn entries(self, bytes: &[u8], offset: usize) -> Result<Vec<Entry>, ReadFailure> {
        if self.entry_count < 1 + u64::from(matches!(self.kind, RecipeKind::Blob(_))) {
            return Err(WireError::new(MalformedReason::OutOfRange, self.count_offset).into());
        }
        let mut fields = Fields::at(bytes, offset);
        let count = fields.array()?;
        let mut entries = Vec::new();
        for _ in 0..count {
            fields.record(2)?;
            let digest = *fields.fixed_bytes()?;
            let offset = fields.offset();
            let length = fields.uint()?;
            push(&mut entries, (ValueName { digest, length }, offset))?;
        }
        fields.finish()?;
        if count != self.entry_count {
            return Err(WireError::new(MalformedReason::InconsistentCounts, offset).into());
        }
        if let Some((_, offset)) = entries.iter().find(|(entry, _)| entry.length == 0) {
            return Err(WireError::new(MalformedReason::OutOfRange, *offset).into());
        }
        let mut total = 0_u64;
        for (entry, offset) in &entries {
            total = total
                .checked_add(entry.length)
                .ok_or_else(|| WireError::new(MalformedReason::OutOfRange, *offset))?;
        }
        if total != self.name.length {
            return Err(WireError::new(MalformedReason::InconsistentCounts, offset).into());
        }
        Ok(entries)
    }
}

impl<'a> GraphReader<'a, '_> {
    pub(super) fn observe_blob_frame(
        &mut self,
        kind: FrameType,
        mut content: Fields<'a>,
        bytes: Cow<'a, [u8]>,
        location: (usize, u64),
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadFailure> {
        match kind {
            FrameType::Chunk | FrameType::Blob => {
                let declared = UnitDeclaration::read(&mut content, kind == FrameType::Blob)?;
                if let UnitKind::Blob(media_type) = declared.kind {
                    self.claim_blob(
                        ValueName {
                            digest: declared.digest,
                            length: bytes.len() as u64,
                        },
                        true,
                        media_type.map(Cow::Borrowed),
                        location.0,
                    )?;
                }
                let Some(name) = self.verify_blob_unit(declared, &bytes, location.0)? else {
                    return Ok(());
                };
                let media = match declared.kind {
                    UnitKind::Chunk => UnitKind::Chunk,
                    UnitKind::Blob(media) => UnitKind::Blob(media.map(Cow::Borrowed)),
                };
                self.store_unit(name, media, bytes, segment)?;
            }
            FrameType::Recipe | FrameType::Value => {
                let recipe = RecipeDeclaration::read(&mut content, kind == FrameType::Recipe)?;
                let entries = recipe.entries(&bytes, 0)?;
                self.rebuild_and_store(
                    recipe.name,
                    recipe.kind.hold(|text| Ok(Cow::Borrowed(text)))?,
                    &entries,
                    segment,
                    |_| None,
                    (location.0, None),
                )?;
            }
            FrameType::Pack => match bytes {
                Cow::Borrowed(bytes) => {
                    self.read_pack(content, bytes, location, segment, |unit| {
                        Ok(Cow::Borrowed(unit))
                    })?;
                }
                Cow::Owned(bytes) => {
                    self.read_pack(content, &bytes, location, segment, copy_unit)?;
                }
            },
            FrameType::External => self.external(content, segment.registry, location.0)?,
            _ => return Err(ReadFailure::Frame(kind)),
        }
        Ok(())
    }

    fn claim_blob(
        &mut self,
        name: ValueName,
        present: bool,
        media_type: Option<Cow<'a, str>>,
        offset: usize,
    ) -> Result<(), TryReserveError> {
        push(
            &mut self.blob_claims,
            BlobClaim {
                name,
                present,
                media_type,
                offset,
            },
        )
    }

    pub(super) fn fold_blob_claims(&mut self) -> Result<(), TryReserveError> {
        self.blob_claims
            .sort_unstable_by_key(|claim| (claim.name.digest, claim.name.length));
        for claim in self.blob_claims.drain(..) {
            let set = &mut self.graph.blob_set;
            if set
                .last()
                .is_none_or(|entry| entry.name.digest != claim.name.digest)
            {
                push(
                    set,
                    BlobEntry {
                        name: claim.name,
                        present: false,
                        media_types: Vec::new(),
                    },
                )?;
            }
            let entry = set
                .last_mut()
                .expect("each claim belongs to the current entry");
            entry.present |= claim.present;
            if claim.name.length != entry.name.length {
                push(&mut self.graph.blob_length_conflicts, claim.offset)?;
            }
            if let Some(media_type) = claim.media_type {
                push(&mut entry.media_types, media_type)?;
            }
        }
        for entry in &mut self.graph.blob_set {
            // Canonical CBOR text headers compare by byte length before text.
            entry
                .media_types
                .sort_unstable_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));
            entry.media_types.dedup();
        }
        self.graph.blob_length_conflicts.sort_unstable();
        Ok(())
    }

    fn verify_blob_unit(
        &mut self,
        unit: UnitDeclaration<'_>,
        bytes: &[u8],
        offset: usize,
    ) -> Result<Option<ValueName>, ReadFailure> {
        let name = ValueName::of(bytes);
        if name.digest == unit.digest {
            Ok(Some(name))
        } else {
            self.graph
                .retain_content_failure(offset, ReadFailure::BlobDigest)?;
            Ok(None)
        }
    }

    fn external(
        &mut self,
        mut content: Fields<'a>,
        registry: NonZeroU64,
        offset: usize,
    ) -> Result<(), ReadFailure> {
        content.record(3)?;
        content.record(2)?;
        content_enum(&mut content, BLAKE3_256, registry)?;
        let digest = *content.fixed_bytes()?;
        let length = content.uint()?;
        let media_type = content.option(Fields::text)?;
        let name = ValueName { digest, length };
        self.claim_blob(name, false, media_type.map(Cow::Borrowed), offset)?;
        push(
            &mut self.graph.external_blobs,
            ExternalBlob { name, media_type },
        )?;
        Ok(())
    }

    fn store_unit(
        &mut self,
        name: ValueName,
        kind: UnitKind<Cow<'a, str>>,
        bytes: Cow<'a, [u8]>,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadFailure> {
        segment.units.try_reserve(1)?;
        let unit = match kind {
            UnitKind::Blob(media_type) => {
                let index = self.graph.blobs.len();
                push(
                    &mut self.graph.blobs,
                    VerifiedBlob {
                        name,
                        media_type,
                        bytes,
                    },
                )?;
                Unit::Blob(index)
            }
            UnitKind::Chunk => Unit::Chunk(bytes),
        };
        segment.units.entry(name.digest).or_insert(unit);
        Ok(())
    }

    fn rebuild<'p>(
        &self,
        entries: &[Entry],
        segment: &Segment<'a>,
        pack_unit: impl Fn(&Digest) -> Option<&'p [u8]>,
    ) -> Result<Vec<u8>, ReadFailure> {
        let mut output = Vec::new();
        for &(entry, _) in entries {
            let bytes = match segment.units.get(&entry.digest) {
                Some(Unit::Blob(index)) => self.graph.blobs[*index].bytes.as_ref(),
                Some(Unit::Chunk(bytes)) => bytes.as_ref(),
                None => pack_unit(&entry.digest).ok_or(ReadFailure::MissingUnit(entry))?,
            };
            if bytes.len() as u64 != entry.length {
                return Err(ReadFailure::UnitLength(entry));
            }
            output.try_reserve(bytes.len())?;
            output.extend_from_slice(bytes);
        }
        Ok(output)
    }

    fn rebuild_and_store<'p>(
        &mut self,
        name: ValueName,
        kind: RecipeKind<Cow<'a, str>>,
        entries: &[Entry],
        segment: &Segment<'a>,
        pack_unit: impl Fn(&Digest) -> Option<&'p [u8]>,
        (offset, member): (usize, Option<usize>),
    ) -> Result<(), ReadFailure> {
        let is_value = matches!(kind, RecipeKind::Value);
        if let RecipeKind::Blob(media_type) = &kind {
            let media_type = match media_type {
                Some(Cow::Borrowed(text)) => Some(Cow::Borrowed(*text)),
                Some(Cow::Owned(text)) => Some(copy_text(text)?),
                None => None,
            };
            self.claim_blob(name, true, media_type, offset)?;
        }
        let refused = self.rebuilt.charge(name.length).err();
        let result = match refused {
            Some(_) => Err(ReadFailure::RebuildCapacity(name)),
            None => self
                .rebuild(entries, segment, pack_unit)
                .and_then(|bytes| self.store_rebuild(name, kind, bytes)),
        };
        let state = match result {
            Ok(()) => RebuildState::Verified,
            Err(failure) => {
                let state = match (&failure, refused) {
                    (ReadFailure::RebuildCapacity(_), Some(count)) => {
                        RebuildState::Refused { count }
                    }
                    (ReadFailure::MissingUnit(_), _) => RebuildState::MissingUnit,
                    (ReadFailure::UnitLength(_), _) => RebuildState::BlobLength,
                    (
                        ReadFailure::BlobDigest | ReadFailure::Fold(FoldError::ValueClaim { .. }),
                        _,
                    ) => RebuildState::BlobDigest,
                    _ => return Err(failure),
                };
                if is_value {
                    // The declaration remains a candidate for later terms.
                    self.values.record(name, None).map_err(ReadFailure::Fold)?;
                } else {
                    self.graph.note_content_omission(offset);
                }
                push(
                    &mut self.graph.frame_failures,
                    ReadError { offset, failure },
                )?;
                state
            }
        };
        push(
            &mut self.graph.rebuilds,
            RebuildResult {
                offset,
                member,
                name,
                state,
            },
        )?;
        Ok(())
    }

    fn store_rebuild(
        &mut self,
        name: ValueName,
        kind: RecipeKind<Cow<'a, str>>,
        output: Vec<u8>,
    ) -> Result<(), ReadFailure> {
        match kind {
            RecipeKind::Blob(media_type) => {
                if ValueName::of(&output) != name {
                    return Err(ReadFailure::BlobDigest);
                }
                push(
                    &mut self.graph.blobs,
                    VerifiedBlob {
                        name,
                        media_type,
                        bytes: Cow::Owned(output),
                    },
                )?;
            }
            RecipeKind::Value => self
                .values
                .record(name, Some(output))
                .map_err(ReadFailure::Fold)?,
        }
        Ok(())
    }

    fn read_pack<'p>(
        &mut self,
        mut content: Fields<'_>,
        bytes: &'p [u8],
        (offset, position): (usize, u64),
        segment: &mut Segment<'a>,
        retain: impl Fn(&'p [u8]) -> Result<Cow<'a, [u8]>, ReadFailure>,
    ) -> Result<(), ReadFailure> {
        content.record(2)?;
        let count = content.positive(MalformedReason::OutOfRange)?;
        let length_offset = content.offset();
        let unit_length = content.uint()?;
        let pack = Pack::read(bytes, count.get(), segment.registry)?;
        if pack.unit_length != u128::from(unit_length) {
            return Err(WireError::new(MalformedReason::InconsistentCounts, length_offset).into());
        }
        let mut members = pack.members()?;
        let mut units = FastMap::default();
        for (member_index, member) in members.iter_mut().enumerate() {
            if let Member::Unit(unit, bytes) = member {
                segment
                    .index_content
                    .record_unit(unit.digest, position, member_index)?;
                if let UnitKind::Blob(media_type) = unit.kind {
                    self.claim_blob(
                        ValueName {
                            digest: unit.digest,
                            length: bytes.len() as u64,
                        },
                        true,
                        media_type.map(copy_text).transpose()?,
                        offset,
                    )?;
                }
                if self.verify_blob_unit(*unit, bytes, offset)?.is_some() {
                    units.try_reserve(1)?;
                    units.entry(unit.digest).or_insert(*bytes);
                } else {
                    *member = Member::DamagedUnit;
                }
            }
        }
        for (member_index, member) in members.into_iter().enumerate() {
            match member {
                Member::DamagedUnit => {}
                Member::Unit(unit, bytes) => {
                    let kind = match unit.kind {
                        UnitKind::Chunk if segment.units.contains_key(&unit.digest) => continue,
                        UnitKind::Chunk => UnitKind::Chunk,
                        UnitKind::Blob(media) => UnitKind::Blob(media.map(copy_text).transpose()?),
                    };
                    let name = ValueName {
                        digest: unit.digest,
                        length: bytes.len() as u64,
                    };
                    self.store_unit(name, kind, retain(bytes)?, segment)?;
                }
                Member::Recipe(recipe, entries) => {
                    self.rebuild_and_store(
                        recipe.name,
                        recipe.kind.hold(copy_text)?,
                        &entries,
                        segment,
                        |id| units.get(id).copied(),
                        (offset, Some(member_index)),
                    )?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Declaration<'a> {
    Unit(UnitDeclaration<'a>),
    Recipe(RecipeDeclaration<'a>),
}

struct DirectoryEntry<'a> {
    declaration: Declaration<'a>,
    length: u64,
    length_offset: usize,
}

enum Member<'a> {
    Unit(UnitDeclaration<'a>, &'a [u8]),
    DamagedUnit,
    Recipe(RecipeDeclaration<'a>, Vec<Entry>),
}

struct Pack<'a> {
    directory: Vec<DirectoryEntry<'a>>,
    bodies: &'a [u8],
    body_offset: usize,
    unit_length: u128,
}

impl<'a> Pack<'a> {
    fn read(
        bytes: &'a [u8],
        declared_count: u64,
        registry: NonZeroU64,
    ) -> Result<Self, ReadFailure> {
        let mut fields = Fields::new(bytes);
        fields.record(2)?;
        let count_offset = fields.offset();
        let count = fields.array()?;
        let mut directory = Vec::new();
        let mut total = 0_u128;
        let mut unit_length = 0_u128;
        for _ in 0..count {
            fields.record(3)?;
            let declaration = match content_enum(&mut fields, 4, registry)? {
                1 => Declaration::Unit(UnitDeclaration::read(&mut fields, false)?),
                2 => Declaration::Unit(UnitDeclaration::read(&mut fields, true)?),
                3 => Declaration::Recipe(RecipeDeclaration::read(&mut fields, true)?),
                4 => Declaration::Recipe(RecipeDeclaration::read(&mut fields, false)?),
                _ => unreachable!("content enum admits only member kinds 1 through 4"),
            };
            let length_offset = fields.offset();
            let length = fields.uint()?;
            total += u128::from(length);
            if matches!(declaration, Declaration::Unit(_)) {
                unit_length += u128::from(length);
            }
            push(
                &mut directory,
                DirectoryEntry {
                    declaration,
                    length,
                    length_offset,
                },
            )?;
        }
        let bodies_offset = fields.offset();
        let bodies = fields.bytes()?;
        let body_offset = fields.offset() - bodies.len();
        fields.finish()?;
        if count != declared_count {
            return Err(WireError::new(MalformedReason::InconsistentCounts, count_offset).into());
        }
        if total != bodies.len() as u128 {
            return Err(WireError::new(MalformedReason::InconsistentCounts, bodies_offset).into());
        }
        Ok(Self {
            directory,
            bodies,
            body_offset,
            unit_length,
        })
    }

    fn members(self) -> Result<Vec<Member<'a>>, ReadFailure> {
        let mut members = Vec::new();
        members.try_reserve_exact(self.directory.len())?;
        let mut bodies = self.bodies;
        let mut offset = self.body_offset;
        for entry in self.directory {
            let length = usize::try_from(entry.length).expect("the total length fits this payload");
            let (bytes, rest) = bodies.split_at(length);
            let member = match entry.declaration {
                Declaration::Unit(unit) => {
                    if matches!(unit.kind, UnitKind::Chunk) && length == 0 {
                        return Err(WireError::new(
                            MalformedReason::OutOfRange,
                            entry.length_offset,
                        )
                        .into());
                    }
                    Member::Unit(unit, bytes)
                }
                Declaration::Recipe(recipe) => {
                    Member::Recipe(recipe, recipe.entries(bytes, offset)?)
                }
            };
            members.push(member);
            bodies = rest;
            offset += length;
        }
        Ok(members)
    }
}

fn copy_unit<'a>(bytes: &[u8]) -> Result<Cow<'a, [u8]>, ReadFailure> {
    let mut output = Vec::new();
    output.try_reserve_exact(bytes.len())?;
    output.extend_from_slice(bytes);
    Ok(Cow::Owned(output))
}

fn copy_text<'a>(text: &str) -> Result<Cow<'a, str>, ReadFailure> {
    let mut output = String::new();
    output.try_reserve_exact(text.len())?;
    output.push_str(text);
    Ok(Cow::Owned(output))
}
