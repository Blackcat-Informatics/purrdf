// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{self, Write};

use purrdf_lex::cbor::head;

use crate::v2::fields::push;
use crate::v2::{BlobEntry, FrameType, ValueName};
use crate::{FastMap, FastSet};

use super::blob::{RecipeKind, cut, recipe_content, recipe_payload};
use super::{Buffer, Digest, SegmentWriteError, SegmentWriter, write_option};

// P4 §7 cuts packs by the aggregate length of their bodies.
const PACK_BYTES: usize = 16_777_216;

#[derive(Clone, Copy)]
enum Member {
    Chunk(Digest),
    Blob(Digest),
    Recipe(ValueName, u64),
    Value(ValueName, u64),
}

#[derive(Default)]
struct Pack {
    directory: Buffer,
    bodies: Buffer,
    count: usize,
    unit_bytes: u64,
    units: Vec<(Digest, usize)>,
}

impl Pack {
    fn add<W: Write>(
        &mut self,
        writer: &mut SegmentWriter<W>,
        member: Member,
        media: Option<&str>,
        bytes: &[u8],
    ) -> Result<(), SegmentWriteError> {
        if self.bodies.0.len().saturating_add(bytes.len()) > PACK_BYTES {
            self.flush(writer)?;
        }
        let directory = &mut self.directory;
        head::write_head(directory, head::ARRAY, 3)?;
        let kind = match member {
            Member::Chunk(_) => 1,
            Member::Blob(_) => 2,
            Member::Recipe(..) => 3,
            Member::Value(..) => 4,
        };
        head::write_head(directory, head::UNSIGNED, kind)?;
        match member {
            Member::Chunk(digest) | Member::Blob(digest) => {
                let blob = matches!(member, Member::Blob(_));
                head::write_head(directory, head::ARRAY, if blob { 2 } else { 1 })?;
                head::write_bytes(directory, &digest)?;
                if blob {
                    write_option(directory, media, head::write_text)?;
                }
                self.unit_bytes += bytes.len() as u64;
                push(&mut self.units, (digest, self.count)).map_err(io::Error::other)?;
            }
            Member::Recipe(name, count) => {
                recipe_content(directory, name, RecipeKind::Blob(media), count)?;
            }
            Member::Value(name, count) => {
                recipe_content(directory, name, RecipeKind::Value, count)?;
            }
        }
        head::write_head(directory, head::UNSIGNED, bytes.len() as u64)?;
        self.bodies.write_all(bytes)?;
        self.count += 1;
        Ok(())
    }

    fn flush<W: Write>(&mut self, writer: &mut SegmentWriter<W>) -> Result<(), SegmentWriteError> {
        if self.count == 0 {
            return Ok(());
        }
        let mut payload = Buffer::default();
        head::write_head(&mut payload, head::ARRAY, 2)?;
        head::write_head(&mut payload, head::ARRAY, self.count as u64)?;
        payload.write_all(&self.directory.0)?;
        head::write_bytes(&mut payload, &self.bodies.0)?;
        writer.append(FrameType::Pack, &payload.0, |output| {
            head::write_head(output, head::ARRAY, 2)?;
            head::write_head(output, head::UNSIGNED, self.count as u64)?;
            head::write_head(output, head::UNSIGNED, self.unit_bytes)
        })?;
        if let Some(index) = &mut writer.index {
            let position = writer.range.count().get() - 2;
            for (digest, member) in self.units.drain(..) {
                index
                    .content
                    .record_unit(digest, position, member)
                    .map_err(io::Error::other)?;
            }
        }
        self.directory.0.clear();
        self.bodies.0.clear();
        self.count = 0;
        self.unit_bytes = 0;
        Ok(())
    }
}

fn emit_unit<W: Write>(
    writer: &mut SegmentWriter<W>,
    pack: &mut Pack,
    emitted: &mut FastSet<Digest>,
    single_units: &FastMap<Digest, Option<&str>>,
    name: ValueName,
    bytes: &[u8],
) -> Result<(), SegmentWriteError> {
    if !emitted.contains(&name.digest) {
        emitted.try_reserve(1).map_err(io::Error::other)?;
        emitted.insert(name.digest);
        match single_units.get(&name.digest) {
            Some(&media) => pack.add(writer, Member::Blob(name.digest), media, bytes)?,
            None => pack.add(writer, Member::Chunk(name.digest), None, bytes)?,
        }
    }
    Ok(())
}

fn emit_units<W: Write>(
    writer: &mut SegmentWriter<W>,
    pack: &mut Pack,
    emitted: &mut FastSet<Digest>,
    single_units: &FastMap<Digest, Option<&str>>,
    mut bytes: &[u8],
) -> Result<Vec<ValueName>, SegmentWriteError> {
    let mut parts = Vec::new();
    while !bytes.is_empty() {
        let (part, rest) = bytes.split_at(cut(bytes));
        let name = ValueName::of(part);
        push(&mut parts, name).map_err(io::Error::other)?;
        emit_unit(writer, pack, emitted, single_units, name, part)?;
        bytes = rest;
    }
    Ok(parts)
}

pub(super) fn write_values<W: Write>(
    writer: &mut SegmentWriter<W>,
    values: &mut [(ValueName, &[u8])],
) -> Result<(), SegmentWriteError> {
    values.sort_unstable_by_key(|(name, _)| name.digest);
    let mut pack = Pack::default();
    let mut emitted = FastSet::default();
    let single_units = FastMap::default();
    for group in values.chunk_by(|a, b| a.0.digest == b.0.digest) {
        let (name, bytes) = group[0];
        let parts = emit_units(writer, &mut pack, &mut emitted, &single_units, bytes)?;
        let count = parts.len() as u64;
        let recipe = recipe_payload(parts.into_iter())?;
        pack.add(writer, Member::Value(name, count), None, &recipe.0)?;
    }
    pack.flush(writer)
}

pub(super) fn write_blobs<W: Write>(
    writer: &mut SegmentWriter<W>,
    blobs: &[(&BlobEntry<'_>, Option<&[u8]>)],
) -> Result<(), SegmentWriteError> {
    let mut pack = Pack::default();
    let mut emitted = FastSet::default();
    let mut single_units = FastMap::default();
    for &(entry, bytes) in blobs {
        if let Some(bytes) = bytes
            && cut(bytes) == bytes.len()
        {
            single_units.try_reserve(1).map_err(io::Error::other)?;
            single_units.insert(entry.name.digest, least_media(entry));
        }
    }
    for &(entry, bytes) in blobs {
        let Some(bytes) = bytes else {
            continue;
        };
        if single_units.contains_key(&entry.name.digest) {
            emit_unit(
                writer,
                &mut pack,
                &mut emitted,
                &single_units,
                entry.name,
                bytes,
            )?;
        } else {
            let parts = emit_units(writer, &mut pack, &mut emitted, &single_units, bytes)?;
            let count = parts.len() as u64;
            let recipe = recipe_payload(parts.into_iter())?;
            pack.add(
                writer,
                Member::Recipe(entry.name, count),
                least_media(entry),
                &recipe.0,
            )?;
        }
    }
    pack.flush(writer)?;
    for &(entry, bytes) in blobs {
        if entry.media_types.is_empty() && bytes.is_none() {
            writer.add_external_blob(entry.name, None)?;
        }
        for media in entry.media_types.iter().skip(usize::from(bytes.is_some())) {
            writer.add_external_blob(entry.name, Some(media))?;
        }
    }
    Ok(())
}

fn least_media<'a>(entry: &'a BlobEntry<'_>) -> Option<&'a str> {
    entry.media_types.first().map(AsRef::as_ref)
}
