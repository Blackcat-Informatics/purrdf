// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use core::num::NonZeroU64;
use std::io::{self, Write};

use purrdf_core::canonical_labelling::ViewError;
use purrdf_hash::blake3::Hasher;
use purrdf_lex::allocation::try_boxed;
use purrdf_lex::cbor::head;
use purrdf_zstd::encoding::{CompressionLevel, CompressionParameters, compress_with_parameters};

use super::fields::write_option;
use super::header::HEADER_CONTEXT;
use super::link::frame_hasher;
use super::parity::{EncodedParity, GEOMETRY_ONE, Recorder, RecordingWriter};
use super::payload::{ZSTD, matches_zstd_header, payload_digest, stored_digest};
use super::rdf::unit_digest;
use super::{
    BLAKE3_256, CanonicalForm, DIGEST_LENGTH, FRAME_FIELDS, FRAME_KIND, FoldError, FrameType,
    Profile, Range, RangeFull, ReadLabellingError, ValueName, write_header,
};

mod blob;
mod compact;
mod cosign;
mod dictionary;
mod index;
mod packs;
mod rdf;
mod references;
mod signature;

pub(super) use compact::write_structure;
pub use compact::{Compaction, CompactionOutcome, CompactionStop, compact};
pub use cosign::{CoSignError, append_cosigned_commit};
pub use index::write_indexed_rdf_segment;
pub use rdf::{rdf_segment_bytes, write_rdf_segment};
pub(super) use references::ReferenceRecords;
pub use signature::SignatureWriteError;

type Digest = [u8; DIGEST_LENGTH];

/// A failure while appending a generation 2 frame to a segment.
#[derive(Debug)]
#[non_exhaustive]
pub enum SegmentWriteError {
    /// The sink reported an error. The stream may end inside a frame.
    Io(io::Error),
    /// An earlier frame write failed and the sink may contain a partial item.
    IncompleteFrame,
    /// The segment has reached the maximum range leaf count.
    RangeFull(RangeFull),
    /// The dictionary would exceed the term ID range.
    DictionaryFull,
    /// The supplied fold contains held rows or an overflowing multiplicity.
    IncompleteFold,
    /// A reference record uses a term absent from the supplied fold.
    ReferenceTerm(FoldError),
    /// A value declaration needs at least one nonempty unit.
    EmptyValue,
    /// The signature list could not be prepared.
    Signature(SignatureWriteError),
    /// The segment's fold could not be labelled for its closing index.
    Labelling(ViewError<io::Error>),
    /// The source read could not complete the labelling procedure.
    SourceLabelling(Box<ReadLabellingError>),
}

impl fmt::Display for SegmentWriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "could not write the GTS v2 frame ({error})"),
            Self::IncompleteFrame => f.write_str("an earlier GTS v2 frame write did not complete"),
            Self::RangeFull(error) => error.fmt(f),
            Self::DictionaryFull => f.write_str("GTS v2 term IDs exceed the wire range"),
            Self::IncompleteFold => {
                f.write_str("GTS v2 RDF fold contains held rows or an overflowing multiplicity")
            }
            Self::ReferenceTerm(error) => error.fmt(f),
            Self::EmptyValue => f.write_str("GTS v2 value declarations require a nonempty value"),
            Self::Signature(error) => error.fmt(f),
            Self::Labelling(error) => error.fmt(f),
            Self::SourceLabelling(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SegmentWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::RangeFull(error) => Some(error),
            Self::Signature(error) => Some(error),
            Self::ReferenceTerm(error) => Some(error),
            Self::Labelling(error) => Some(error),
            Self::SourceLabelling(error) => Some(error.as_ref()),
            Self::IncompleteFrame
            | Self::DictionaryFull
            | Self::IncompleteFold
            | Self::EmptyValue => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for SegmentWriteError {
    purrdf_hash::error_visit! {
        Io(cause) => "gts-v2-segment-write" { "cause" => Cause(cause) },
        IncompleteFrame => "gts-v2-incomplete-write" {},
        RangeFull(cause) => "gts-v2-segment-range-full" { "cause" => Cause(cause) },
        DictionaryFull => "gts-v2-dictionary-full" {},
        IncompleteFold => "gts-v2-incomplete-fold" {},
        ReferenceTerm(cause) => "gts-v2-reference-write-term" { "cause" => Cause(cause) },
        EmptyValue => "gts-v2-empty-value-declaration" {},
        Signature(cause) => "gts-v2-signature-write" { "cause" => Cause(cause) },
        Labelling(cause) => "gts-v2-index-labelling" { "cause" => Cause(cause) },
        SourceLabelling(cause) => "gts-v2-source-labelling" { "cause" => Cause(cause.as_ref()) },
    }
}

impl From<io::Error> for SegmentWriteError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Stream a registry version 1 GTS v2 segment to a caller-owned sink.
#[derive(Debug)]
pub struct SegmentWriter<W> {
    sink: W,
    head: Option<Digest>,
    opener: Digest,
    range: Range,
    term_base: u128,
    parity: Option<Box<Recorder>>,
    index: Option<Box<index::IndexObservations>>,
    compression: Option<CompressionParameters>,
    flush_frames: bool,
}

impl<W: Write> SegmentWriter<W> {
    /// Write the header of a new segment with no profile or canonical claim.
    ///
    /// # Errors
    /// Returns the sink error if the header cannot be written.
    pub fn new(sink: W) -> io::Result<Self> {
        Self::open(sink, None, None, None, None)
    }

    /// Write a header and emit parity for each completed four MiB span.
    ///
    /// A span's parity follows the ordinary frame that completed it. The
    /// writer leaves an unfinished span open when its sink is extracted.
    ///
    /// # Errors
    /// Returns an error if allocation or the header write fails.
    pub fn new_with_parity(sink: W) -> io::Result<Self> {
        let parity = try_boxed(Recorder::new()?).map_err(io::Error::other)?;
        Self::open(sink, Some(parity), None, None, None)
    }

    fn open(
        mut sink: W,
        mut parity: Option<Box<Recorder>>,
        mut index: Option<Box<index::IndexObservations>>,
        profile: Option<Profile>,
        canonical: Option<CanonicalForm>,
    ) -> io::Result<Self> {
        let compression = canonical
            .map(|_| {
                CompressionParameters::builder(CompressionLevel::Level(12))
                    .window_log(26)
                    .build()
            })
            .transpose()
            .map_err(io::Error::other)?;
        let mut hasher = Hasher::new_derive_key(HEADER_CONTEXT);
        let mut length = 0;
        write_header(
            &mut HashingWriter {
                sink: &mut RecordingWriter {
                    sink: &mut sink,
                    recorder: parity.as_deref_mut(),
                    length: index.as_ref().map(|_| &mut length),
                },
                hasher: &mut hasher,
            },
            profile,
            canonical,
            None,
        )?;
        let head = *hasher.finalize().as_bytes();
        if let Some(parity) = &mut parity {
            parity.set_opener(head);
        }
        if let Some(index) = &mut index {
            index.record_length(length)?;
        }
        Ok(Self {
            sink,
            head: Some(head),
            opener: head,
            range: Range::new(&head),
            term_base: 0,
            parity,
            index,
            compression,
            flush_frames: false,
        })
    }

    fn from_opener(sink: W, opener: Digest, index: Box<index::IndexObservations>) -> Self {
        Self {
            sink,
            head: Some(opener),
            opener,
            range: Range::new(&opener),
            term_base: 0,
            parity: None,
            index: Some(index),
            compression: None,
            flush_frames: true,
        }
    }

    /// The head of the complete segment while the writer can still append.
    #[must_use]
    pub const fn head(&self) -> Option<&Digest> {
        self.head.as_ref()
    }

    /// Recover the sink at its current position.
    #[must_use]
    pub fn into_inner(self) -> W {
        self.sink
    }

    /// Append a whole blob and return its unit digest.
    ///
    /// The optional media type is an opaque label.
    ///
    /// # Errors
    /// Returns a range or sink error. A sink error prevents further appends.
    pub fn add_blob(
        &mut self,
        bytes: &[u8],
        media_type: Option<&str>,
    ) -> Result<Digest, SegmentWriteError> {
        let digest = unit_digest(bytes);
        self.append(FrameType::Blob, bytes, |output| {
            head::write_head(output, head::ARRAY, 2)?;
            head::write_bytes(output, &digest)?;
            write_option(output, media_type, head::write_text)
        })?;
        Ok(digest)
    }

    /// Append a blob declaration whose bytes are supplied outside this frame.
    ///
    /// The name must describe the external bytes. Returns the new frame
    /// identifier.
    ///
    /// # Errors
    /// Returns a range or sink error. A sink error prevents further appends.
    pub fn add_external_blob(
        &mut self,
        name: ValueName,
        media_type: Option<&str>,
    ) -> Result<Digest, SegmentWriteError> {
        self.append(FrameType::External, &[], |output| {
            head::write_head(output, head::ARRAY, 3)?;
            head::write_head(output, head::ARRAY, 2)?;
            head::write_head(output, head::UNSIGNED, BLAKE3_256)?;
            head::write_bytes(output, &name.digest)?;
            head::write_head(output, head::UNSIGNED, name.length)?;
            write_option(output, media_type, head::write_text)
        })
    }

    /// Commit the prefix through the current head and return the new frame identifier.
    ///
    /// # Errors
    /// Returns a range or sink error. A sink error prevents further appends.
    pub fn add_commit(&mut self) -> Result<Digest, SegmentWriteError> {
        let count = self.range.count().get() - 1;
        let root = self.range.root();
        self.append(FrameType::Commit, &[], |output| {
            write_commit_content(output, count, &root)
        })
    }

    fn append_chunk(&mut self, bytes: &[u8], digest: &Digest) -> Result<Digest, SegmentWriteError> {
        self.append(FrameType::Chunk, bytes, |output| {
            head::write_head(output, head::ARRAY, 1)?;
            head::write_bytes(output, digest)
        })
    }

    fn append(
        &mut self,
        frame_type: FrameType,
        payload: &[u8],
        content: impl FnOnce(&mut HashingWriter<'_, RecordingWriter<'_, W>>) -> io::Result<()>,
    ) -> Result<Digest, SegmentWriteError> {
        let compressed = self
            .compression
            .filter(|_| payload.len() >= 64)
            .map(|parameters| compress_with_parameters(payload, &parameters))
            .filter(|stored| stored.len().saturating_add(16) < payload.len());
        if let Some(stored) = &compressed
            && !matches_zstd_header(payload.len() as u64, stored)
        {
            return Err(
                io::Error::other("GTS compression produced an incompatible zstd header").into(),
            );
        }
        self.append_with(frame_type, |sink, predecessor| {
            let identifier = write_stored_frame_body(
                sink,
                predecessor,
                frame_type,
                payload,
                compressed.as_deref(),
                content,
            )?;
            head::write_head(sink, head::ARRAY, 0)?;
            Ok(identifier)
        })
    }

    fn append_head(&self) -> Result<Digest, SegmentWriteError> {
        if self.range.count().get() == u64::MAX {
            return Err(SegmentWriteError::RangeFull(RangeFull));
        }
        self.head.ok_or(SegmentWriteError::IncompleteFrame)
    }

    fn append_with(
        &mut self,
        frame_type: FrameType,
        write: impl FnOnce(&mut RecordingWriter<'_, W>, &Digest) -> io::Result<Digest>,
    ) -> Result<Digest, SegmentWriteError> {
        let predecessor = self.append_head()?;
        let code = Some(u64::from(frame_type as u8));
        if let Some(index) = &mut self.index {
            index.runs.reserve_next(code).map_err(io::Error::other)?;
        }
        self.head = None;
        let mut length = 0;
        let identifier = write(
            &mut RecordingWriter {
                sink: &mut self.sink,
                recorder: self.parity.as_deref_mut(),
                length: self.index.as_ref().map(|_| &mut length),
            },
            &predecessor,
        )?;
        if self.flush_frames {
            self.sink.flush()?;
        }
        if let Some(index) = &mut self.index {
            index.record_length(length)?;
            index.runs.record(code);
        }
        if let Some(parity) = &mut self.parity {
            for span in parity.drain() {
                write_parity(
                    &mut self.sink,
                    self.index.as_deref_mut(),
                    &identifier,
                    &span,
                )?;
            }
        }
        self.range
            .append(&identifier)
            .map_err(SegmentWriteError::RangeFull)?;
        self.head = Some(identifier);
        Ok(identifier)
    }
}

fn write_parity(
    sink: &mut impl Write,
    index: Option<&mut index::IndexObservations>,
    anchor: &Digest,
    span: &EncodedParity,
) -> io::Result<()> {
    let mut length = 0;
    write_frame(
        &mut RecordingWriter {
            sink,
            recorder: None,
            length: index.as_ref().map(|_| &mut length),
        },
        anchor,
        FrameType::Parity,
        &span.payload,
        |output| {
            head::write_head(output, head::ARRAY, 5)?;
            head::write_head(output, head::UNSIGNED, GEOMETRY_ONE)?;
            head::write_head(output, head::UNSIGNED, span.first)?;
            head::write_head(output, head::UNSIGNED, span.end)?;
            head::write_bytes(output, anchor)?;
            head::write_bytes(output, &span.table)
        },
    )?;
    if let Some(index) = index {
        index.record_length(length)?;
    }
    Ok(())
}

fn write_commit_content(output: &mut impl Write, count: u64, root: &Digest) -> io::Result<()> {
    head::write_head(output, head::ARRAY, 2)?;
    head::write_head(output, head::UNSIGNED, count)?;
    head::write_bytes(output, root)
}

fn write_frame<W: Write>(
    sink: &mut W,
    predecessor: &Digest,
    frame_type: FrameType,
    payload: &[u8],
    content: impl FnOnce(&mut HashingWriter<'_, W>) -> io::Result<()>,
) -> io::Result<Digest> {
    let identifier = write_frame_body(sink, predecessor, frame_type, payload, content)?;
    head::write_head(sink, head::ARRAY, 0)?;
    Ok(identifier)
}

fn write_frame_body<W: Write>(
    sink: &mut W,
    predecessor: &Digest,
    frame_type: FrameType,
    payload: &[u8],
    content: impl FnOnce(&mut HashingWriter<'_, W>) -> io::Result<()>,
) -> io::Result<Digest> {
    write_stored_frame_body(sink, predecessor, frame_type, payload, None, content)
}

fn write_stored_frame_body<W: Write>(
    sink: &mut W,
    predecessor: &Digest,
    frame_type: FrameType,
    payload: &[u8],
    compressed: Option<&[u8]>,
    content: impl FnOnce(&mut HashingWriter<'_, W>) -> io::Result<()>,
) -> io::Result<Digest> {
    let mut hasher = frame_hasher(predecessor);
    head::write_head(sink, head::ARRAY, FRAME_FIELDS as u64)?;
    head::write_head(sink, head::UNSIGNED, FRAME_KIND)?;
    {
        let mut identity = HashingWriter {
            sink,
            hasher: &mut hasher,
        };
        head::write_head(&mut identity, head::UNSIGNED, u64::from(frame_type as u8))?;
        identity.write_all(&[
            0xf4 + u8::from(frame_type.critical()),
            0xf4 + u8::from(frame_type.transparent()),
        ])?;
        content(&mut identity)?;
        head::write_head(&mut identity, head::UNSIGNED, payload.len() as u64)?;
        head::write_bytes(&mut identity, &payload_digest(payload))?;
    }
    let identifier = *hasher.finalize().as_bytes();
    if let Some(compressed) = compressed {
        const CODEC: NonZeroU64 = NonZeroU64::new(ZSTD).expect("zstd has a registered code");
        head::write_head(sink, head::ARRAY, 3)?;
        head::write_head(sink, head::UNSIGNED, 2)?;
        head::write_head(sink, head::UNSIGNED, CODEC.get())?;
        head::write_bytes(sink, &stored_digest(CODEC, compressed))?;
        head::write_bytes(sink, compressed)?;
    } else {
        head::write_head(sink, head::ARRAY, 1)?;
        head::write_head(sink, head::UNSIGNED, 1)?;
        head::write_bytes(sink, payload)?;
    }
    head::write_bytes(sink, &identifier)?;
    Ok(identifier)
}

#[derive(Default)]
struct Buffer(Vec<u8>);

impl Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.try_reserve(bytes.len()).map_err(io::Error::other)?;
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct HashingWriter<'a, W> {
    sink: &'a mut W,
    hasher: &'a mut Hasher,
}

impl<W: Write> Write for HashingWriter<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.sink.write_all(bytes)?;
        self.hasher.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.sink.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::{FrameType, SegmentWriteError, SegmentWriter, head};
    use crate::v2::rdf::unit_digest;
    use crate::v2::{
        ContentError, MalformedReason, RdfFold, ReadCapacity, ReadFailure, ReadGraph, Term, read,
    };

    #[test]
    fn annotations_compare_written_references_before_rebuilding_values() {
        let digest = unit_digest(b"x");
        for kind in [5, 7] {
            for second_length in [1, 2] {
                let mut writer = SegmentWriter::new(Vec::new()).unwrap();
                let mut fold = RdfFold::default();
                fold.intern_term(Term::Iri("http://example.org/p".into()))
                    .unwrap();
                writer.add_rdf(&fold).unwrap();

                // One block of two references with the same digest. Neither
                // value is declared. For lexical references, both forms
                // select the same datatype IRI.
                let block = if kind == 5 { 0x86 } else { 0x83 };
                let mut payload = vec![0x82, 0x81, block, kind, 0x82];
                for _ in 0..2 {
                    head::write_bytes(&mut payload, &digest).unwrap();
                }
                payload.extend_from_slice(&[0x82, 1, second_length]);
                if kind == 5 {
                    payload.extend_from_slice(&[0x82, 1, 1, 0x82, 0, 0, 0x80]);
                }
                payload.push(0x80);
                writer
                    .append(FrameType::Terms, &payload, |output| {
                        output.write_all(&[0x84, 1, 0x81, 0x82, kind, 2, 2, 1])
                    })
                    .unwrap();

                // Two rows in the default graph. The object IDs differ and
                // every other column is the same.
                let rows = [
                    0x85, 0x81, 0x82, 0x81, 1, 2, 0x82, 0, 0, 0x82, 0, 0, 0x82, 1, 1, 0x82, 1, 0,
                ];
                writer
                    .append(FrameType::Annotations, &rows, |output| {
                        output.write_all(&[0x82, 2, 2])
                    })
                    .unwrap();
                writer.add_commit().unwrap();
                let bytes = writer.into_inner();
                let ReadGraph::Second(graph) = read(&bytes, ReadCapacity::default()).unwrap()
                else {
                    panic!("the writer emits generation 2");
                };
                let duplicate = second_length == 1;
                assert_eq!(graph.left_out, u64::from(duplicate));
                assert_eq!(graph.rdf.has_held_rows(), !duplicate);
                assert_eq!(
                    graph.frame_failures.iter().any(|finding| matches!(
                        finding.failure,
                        ReadFailure::Content(ContentError::Malformed(wire))
                            if wire.reason == MalformedReason::UnorderedOrRepeated
                    )),
                    duplicate
                );
            }
        }
    }

    #[test]
    fn parity_writer_matches_the_two_span_placement_vector() {
        let mut bytes = Vec::with_capacity(8_389_487);
        bytes.extend((0_u8..=255).cycle().take(8_389_487));
        let digest = unit_digest(&bytes);
        let mut writer = SegmentWriter::new_with_parity(Vec::new()).unwrap();
        writer.append_chunk(&bytes, &digest).unwrap();

        // The pinned P5 crossing vector covers one chunk that reaches two
        // span ends. Its complete byte identity covers both parity payloads.
        let expected = purrdf_hash::hex::decode_32(
            "d785c9e04c9a06937eeb9e1b3068126da2f9d3762f2ebdc9cd156141bb0c239d",
        )
        .unwrap();
        let output = writer.into_inner();
        assert_eq!(purrdf_hash::blake3::hash(&output).as_bytes(), &expected);

        let mut partial = vec![0; 8_389_620];
        let mut writer = SegmentWriter::new_with_parity(partial.as_mut_slice()).unwrap();
        let error = writer.append_chunk(&bytes, &digest).unwrap_err();
        assert!(
            matches!(error, SegmentWriteError::Io(error) if error.kind() == std::io::ErrorKind::WriteZero)
        );
        assert_eq!(writer.head(), None);
        assert!(matches!(
            writer.add_commit(),
            Err(SegmentWriteError::IncompleteFrame)
        ));
    }
}
