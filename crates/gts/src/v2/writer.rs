// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use std::io::{self, Write};

use purrdf_hash::blake3::Hasher;
use purrdf_lex::cbor::head;

use super::fields::write_option;
use super::header::HEADER_CONTEXT;
use super::link::frame_hasher;
use super::payload::payload_digest;
use super::rdf::unit_digest;
use super::{
    BLAKE3_256, DIGEST_LENGTH, FRAME_FIELDS, FRAME_KIND, FrameType, Range, RangeFull, ValueName,
    write_header,
};

mod blob;
mod rdf;

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
    /// A value declaration needs at least one nonempty unit.
    EmptyValue,
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
            Self::EmptyValue => f.write_str("GTS v2 value declarations require a nonempty value"),
        }
    }
}

impl std::error::Error for SegmentWriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::RangeFull(error) => Some(error),
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
        EmptyValue => "gts-v2-empty-value-declaration" {},
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
    range: Range,
    term_base: u128,
}

impl<W: Write> SegmentWriter<W> {
    /// Write the header of a new segment with no profile or canonical claim.
    ///
    /// # Errors
    /// Returns the sink error if the header cannot be written.
    pub fn new(mut sink: W) -> io::Result<Self> {
        let mut hasher = Hasher::new_derive_key(HEADER_CONTEXT);
        write_header(
            &mut HashingWriter {
                sink: &mut sink,
                hasher: &mut hasher,
            },
            None,
            None,
            None,
        )?;
        let head = *hasher.finalize().as_bytes();
        Ok(Self {
            sink,
            head: Some(head),
            range: Range::new(&head),
            term_base: 0,
        })
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
            head::write_head(output, head::ARRAY, 2)?;
            head::write_head(output, head::UNSIGNED, count)?;
            head::write_bytes(output, &root)
        })
    }

    fn append(
        &mut self,
        frame_type: FrameType,
        payload: &[u8],
        content: impl FnOnce(&mut HashingWriter<'_, W>) -> io::Result<()>,
    ) -> Result<Digest, SegmentWriteError> {
        if self.range.count().get() == u64::MAX {
            return Err(SegmentWriteError::RangeFull(RangeFull));
        }
        let predecessor = self.head.take().ok_or(SegmentWriteError::IncompleteFrame)?;
        let mut hasher = frame_hasher(&predecessor);
        head::write_head(&mut self.sink, head::ARRAY, FRAME_FIELDS as u64)?;
        head::write_head(&mut self.sink, head::UNSIGNED, FRAME_KIND)?;
        {
            let mut identity = HashingWriter {
                sink: &mut self.sink,
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
        head::write_head(&mut self.sink, head::ARRAY, 1)?;
        head::write_head(&mut self.sink, head::UNSIGNED, 1)?;
        head::write_bytes(&mut self.sink, payload)?;
        head::write_bytes(&mut self.sink, &identifier)?;
        head::write_head(&mut self.sink, head::ARRAY, 0)?;
        self.range
            .append(&identifier)
            .map_err(SegmentWriteError::RangeFull)?;
        self.head = Some(identifier);
        Ok(identifier)
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
