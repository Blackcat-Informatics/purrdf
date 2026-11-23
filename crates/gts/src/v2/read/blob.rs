// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::borrow::Cow;

use super::{GraphReader, ReadFailure, Segment, Unit, VerifiedBlob};
use crate::FastMap;
use crate::v2::fields::{Fields, push};
use crate::v2::{BLAKE3_256, DIGEST_LENGTH, FrameType, MalformedReason, ValueName, WireError};

type Digest = [u8; DIGEST_LENGTH];
type Entry = (ValueName, usize);

/// The name, length and media type of an external blob declaration.
#[derive(Debug)]
#[non_exhaustive]
pub struct ExternalBlob<'a> {
    /// BLAKE3 name and declared byte length.
    pub name: ValueName,
    /// Declared media type.
    pub media_type: Option<&'a str>,
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

    fn verify(self, bytes: &[u8]) -> Result<ValueName, ReadFailure> {
        let name = ValueName::of(bytes);
        if name.digest == self.digest {
            Ok(name)
        } else {
            Err(ReadFailure::BlobDigest)
        }
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

impl<'a> GraphReader<'a> {
    pub(super) fn observe_blob_frame(
        &mut self,
        kind: FrameType,
        mut content: Fields<'a>,
        bytes: Cow<'a, [u8]>,
        segment: &mut Segment<'a>,
    ) -> Result<(), ReadFailure> {
        match kind {
            FrameType::Chunk | FrameType::Blob => {
                let declared = UnitDeclaration::read(&mut content, kind == FrameType::Blob)?;
                let name = declared.verify(&bytes)?;
                let media = match declared.kind {
                    UnitKind::Chunk => UnitKind::Chunk,
                    UnitKind::Blob(media) => UnitKind::Blob(media.map(Cow::Borrowed)),
                };
                self.store_unit(name, media, bytes, segment)?;
            }
            FrameType::Recipe | FrameType::Value => {
                let recipe = RecipeDeclaration::read(&mut content, kind == FrameType::Recipe)?;
                let entries = recipe.entries(&bytes, 0)?;
                let output = self.rebuild(recipe.name, &entries, segment, |_| None)?;
                self.store_rebuild(
                    recipe.name,
                    recipe.kind.hold(|text| Ok(Cow::Borrowed(text)))?,
                    output,
                )?;
            }
            FrameType::Pack => match bytes {
                Cow::Borrowed(bytes) => {
                    self.read_pack(content, bytes, segment, |unit| Ok(Cow::Borrowed(unit)))?;
                }
                Cow::Owned(bytes) => {
                    self.read_pack(content, &bytes, segment, copy_unit)?;
                }
            },
            FrameType::External => self.external(content)?,
            _ => return Err(ReadFailure::Frame(kind)),
        }
        Ok(())
    }

    fn external(&mut self, mut content: Fields<'a>) -> Result<(), ReadFailure> {
        content.record(3)?;
        content.record(2)?;
        content.fixed_uint(BLAKE3_256, MalformedReason::UnregisteredValue)?;
        let digest = *content.fixed_bytes()?;
        let length = content.uint()?;
        let media_type = content.option(Fields::text)?;
        push(
            &mut self.graph.external_blobs,
            ExternalBlob {
                name: ValueName { digest, length },
                media_type,
            },
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
        &mut self,
        name: ValueName,
        entries: &[Entry],
        segment: &Segment<'a>,
        pack_unit: impl Fn(&Digest) -> Option<&'p [u8]>,
    ) -> Result<Vec<u8>, ReadFailure> {
        self.rebuilt_remaining = self
            .rebuilt_remaining
            .checked_sub(name.length)
            .ok_or(ReadFailure::RebuildCapacity(name))?;
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
        segment: &mut Segment<'a>,
        retain: impl Fn(&'p [u8]) -> Result<Cow<'a, [u8]>, ReadFailure>,
    ) -> Result<(), ReadFailure> {
        content.record(2)?;
        let count = content.positive(MalformedReason::OutOfRange)?;
        let length_offset = content.offset();
        let unit_length = content.uint()?;
        let pack = Pack::read(bytes, count.get())?;
        if pack.unit_length != u128::from(unit_length) {
            return Err(WireError::new(MalformedReason::InconsistentCounts, length_offset).into());
        }
        let members = pack.members()?;
        let mut units = FastMap::default();
        for member in &members {
            if let Member::Unit(unit, bytes) = member {
                unit.verify(bytes)?;
                units.try_reserve(1)?;
                units.entry(unit.digest).or_insert(*bytes);
            }
        }
        for member in members {
            match member {
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
                    let output =
                        self.rebuild(recipe.name, &entries, segment, |id| units.get(id).copied())?;
                    self.store_rebuild(recipe.name, recipe.kind.hold(copy_text)?, output)?;
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
    Recipe(RecipeDeclaration<'a>, Vec<Entry>),
}

struct Pack<'a> {
    directory: Vec<DirectoryEntry<'a>>,
    bodies: &'a [u8],
    body_offset: usize,
    unit_length: u128,
}

impl<'a> Pack<'a> {
    fn read(bytes: &'a [u8], declared_count: u64) -> Result<Self, ReadFailure> {
        let mut fields = Fields::new(bytes);
        fields.record(2)?;
        let count_offset = fields.offset();
        let count = fields.array()?;
        let mut directory = Vec::new();
        let mut total = 0_u128;
        let mut unit_length = 0_u128;
        for _ in 0..count {
            fields.record(3)?;
            let kind_offset = fields.offset();
            let declaration = match fields.uint()? {
                1 => Declaration::Unit(UnitDeclaration::read(&mut fields, false)?),
                2 => Declaration::Unit(UnitDeclaration::read(&mut fields, true)?),
                3 => Declaration::Recipe(RecipeDeclaration::read(&mut fields, true)?),
                4 => Declaration::Recipe(RecipeDeclaration::read(&mut fields, false)?),
                _ => {
                    return Err(
                        WireError::new(MalformedReason::UnregisteredValue, kind_offset).into(),
                    );
                }
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
