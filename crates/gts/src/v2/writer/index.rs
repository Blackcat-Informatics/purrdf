// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{self, Write};

use purrdf_core::canonical_labelling::{LabellingCapacity, Limit};
use purrdf_lex::allocation::try_boxed;
use purrdf_lex::cbor::head;

use crate::v2::canonical::label_written_rdf;
use crate::v2::fields::{push, write_option};
use crate::v2::index::{IndexContent, TypeRuns};
use crate::v2::item::{LOCATOR_KIND, OFFSETS_KIND};
use crate::v2::parity::{Recorder, RecordingWriter};
use crate::v2::{ContentLabelling, FrameType, RdfFold};

use super::{Buffer, Digest, SegmentWriteError, SegmentWriter, write_parity};

#[derive(Debug, Default)]
pub(super) struct IndexObservations {
    pub(super) runs: TypeRuns,
    pub(super) content: IndexContent,
    lengths: Vec<u64>,
    written: u64,
}

impl IndexObservations {
    pub(super) fn record_length(&mut self, length: u64) -> io::Result<()> {
        self.written = self
            .written
            .checked_add(length)
            .ok_or_else(|| io::Error::other("GTS segment exceeds the wire offset range"))?;
        push(&mut self.lengths, length).map_err(io::Error::other)
    }

    fn write_accelerators(&self, sink: &mut impl Write, index_offset: u64) -> io::Result<()> {
        let mut bytes = 0;
        let mut offsets = RecordingWriter {
            sink,
            recorder: None,
            length: Some(&mut bytes),
        };
        head::write_head(&mut offsets, head::ARRAY, 2)?;
        head::write_head(&mut offsets, head::UNSIGNED, OFFSETS_KIND)?;
        head::write_head(&mut offsets, head::ARRAY, self.lengths.len() as u64)?;
        for &length in &self.lengths {
            head::write_head(&mut offsets, head::UNSIGNED, length)?;
        }
        let distance = (self.written - index_offset)
            .checked_add(bytes)
            .ok_or_else(|| io::Error::other("GTS locator exceeds the wire offset range"))?;
        head::write_head(sink, head::ARRAY, 2)?;
        head::write_head(sink, head::UNSIGNED, LOCATOR_KIND)?;
        // The locator distance is a fixed-width unsigned argument.
        sink.write_all(&[0x1b])?;
        sink.write_all(&distance.to_be_bytes())
    }
}

/// Write a committed RDF segment with an index, parity, offsets and a locator.
///
/// Terms and rows retain arrival order. The writer labels the segment's fold
/// under the supplied limits before writing its opener and flushes the sink
/// after the locator.
///
/// # Errors
/// Returns a fold, labelling, allocation, range, or sink failure. A sink can
/// contain a partial segment when writing fails.
pub fn write_indexed_rdf_segment<W: Write>(
    fold: &RdfFold,
    limit: Limit,
    capacity: &mut LabellingCapacity,
    sink: W,
) -> Result<W, SegmentWriteError> {
    if fold.has_held_rows() || fold.annotation_width_exceeded() {
        return Err(SegmentWriteError::IncompleteFold);
    }
    let labelled =
        label_written_rdf(fold, limit, capacity).map_err(SegmentWriteError::Labelling)?;
    let index = try_boxed(IndexObservations::default()).map_err(io::Error::other)?;
    let parity = try_boxed(Recorder::new_closing()?).map_err(io::Error::other)?;
    let mut writer = SegmentWriter::open(sink, Some(parity), Some(index), None, None)?;
    writer.add_rdf(fold)?;
    writer.add_commit()?;
    writer.finish_indexed(&labelled)
}

impl<W: Write> SegmentWriter<W> {
    pub(super) fn finish_indexed(
        mut self,
        labelled: &ContentLabelling,
    ) -> Result<W, SegmentWriteError> {
        let index_offset = self
            .index
            .as_ref()
            .expect("the indexed writer observes its prefix")
            .written;
        let anchor =
            self.append_index(labelled.index_counts(), labelled.content_digest.as_ref())?;
        let parity = self
            .parity
            .as_mut()
            .expect("an indexed writer records spans");
        if let Some(span) = parity.finish_closing()? {
            write_parity(&mut self.sink, self.index.as_deref_mut(), &anchor, &span)?;
        }
        let index = self
            .index
            .as_ref()
            .expect("an indexed writer measures item lengths");
        index.write_accelerators(&mut self.sink, index_offset)?;
        self.sink.flush()?;
        Ok(self.sink)
    }

    pub(super) fn append_index(
        &mut self,
        counts: [u64; 3],
        digest: Option<&Digest>,
    ) -> Result<Digest, SegmentWriteError> {
        let previous = self.append_head()?;
        let count = self.range.count().get() - 1;
        let root = self.range.root();
        // The indexed writer installed these observations before its first frame.
        let index = self
            .index
            .as_ref()
            .expect("an indexed writer observes the prefix");
        let mut payload = Buffer::default();
        head::write_head(&mut payload, head::ARRAY, 6)?;
        index.runs.write(&mut payload)?;
        index.content.write_directory(&mut payload)?;
        index.content.write_statistics(&mut payload, counts)?;
        head::write_bytes(&mut payload, &root)?;
        write_option(&mut payload, digest, |output, digest| {
            head::write_bytes(output, digest)
        })?;
        index.content.write_pack_members(&mut payload)?;
        self.append(FrameType::Index, &payload.0, |output| {
            head::write_head(output, head::ARRAY, 2)?;
            head::write_head(output, head::UNSIGNED, count)?;
            head::write_bytes(output, &previous)
        })
    }
}
