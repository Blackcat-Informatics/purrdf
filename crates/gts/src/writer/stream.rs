// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Framing a GTS log into a caller's sink, one frame at a time.

use core::fmt;
use std::io::Read;

use purrdf_hash::error::IoFailure;
use purrdf_lex::cbor::head::{BYTES, MAP, head_len, push_head, write_text};
use purrdf_lex::cbor::{Value, write_canonical_map};

use super::{
    FrameEncoder, FrameError, FrameOptions, Infallible, WriterError, WriterOptions, plan_catalog,
    push_envelope,
};

/// Bytes of a clear payload held at a time while it is read from a reader.
const CHUNK_SIZE: usize = 128 * 1024;

/// Byte length of a frame or header id (BLAKE3-256).
const ID_LEN: usize = 32;

/// Byte destination for framed output.
///
/// The contract is that of [`std::io::Write::write_all`]: on `Ok` every byte
/// was accepted, in order. There is no flush: durability belongs to the caller
/// that owns the destination.
///
/// Every [`std::io::Write`] is a sink whose error is [`std::io::Error`]. A
/// destination with its own error type implements this trait, or wraps a
/// closure in [`FnSink`]; either way its error reaches the caller unchanged in
/// [`StreamError::Sink`].
pub trait FrameSink {
    /// The destination's own failure, carried out unchanged.
    type Error;

    /// Accept all of `bytes`, or fail. After an error the destination may hold
    /// a prefix of them.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
}

impl<W: std::io::Write> FrameSink for W {
    type Error = std::io::Error;

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.write_all(bytes)
    }
}

/// A closure as a [`FrameSink`]: `FnSink(|bytes: &[u8]| -> Result<(), E> { .. })`.
#[derive(Clone, Copy, Debug)]
pub struct FnSink<F>(
    /// The closure each run of framed bytes is handed to.
    pub F,
);

impl<F, E> FrameSink for FnSink<F>
where
    F: FnMut(&[u8]) -> Result<(), E>,
{
    type Error = E;

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), E> {
        (self.0)(bytes)
    }
}

/// Whether the installed signer signs one frame.
///
/// A frame's signature is outside its id preimage, so this choice never
/// changes a frame id or the chain: only whether the frame carries `"sig"`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameSigning {
    /// The installed signer signs the frame; with none installed the frame is
    /// unsigned. Explicit [`FrameOptions::signature`] bytes take precedence.
    #[default]
    Installed,
    /// The frame carries no signature, whatever signer is installed.
    Unsigned,
}

/// Why a payload read from a reader could not become a frame.
#[derive(Debug)]
#[non_exhaustive]
pub enum PayloadError {
    /// The reader failed. The original error is retained.
    Read(IoFailure),
    /// The reader did not yield exactly the declared number of bytes.
    Length {
        /// Payload byte length the caller declared.
        declared: u64,
        /// Bytes observed when the disagreement was found: the reader's total
        /// when it ended early, more than `declared` when it ran on.
        observed: u64,
    },
}

impl fmt::Display for PayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(cause) => write!(f, "frame payload read failed: {cause}"),
            Self::Length { declared, observed } => write!(
                f,
                "frame payload declared {declared} bytes but its reader yielded {observed}"
            ),
        }
    }
}

purrdf_lex::error_source!(PayloadError { Read });

impl purrdf_hash::error::StructuredError for PayloadError {
    purrdf_hash::error_visit! {
        Read(cause) => "gts-stream-payload-read" { "cause" => Cause(cause) },
        Length { declared, observed } => "gts-stream-payload-length" {
            "declared" => ByteCount(u128::from(*declared)),
            "observed" => ByteCount(u128::from(*observed)),
        },
    }
}

/// A [`StreamWriter`] failure, generic over the sink's own error.
#[derive(Debug)]
#[non_exhaustive]
pub enum StreamError<E> {
    /// The header or frame was refused before any of its bytes reached the
    /// sink. The writer is unchanged and usable.
    Frame(WriterError),
    /// A payload reader failed or disagreed with its declared length. The
    /// writer is poisoned exactly when part of the frame had already reached
    /// the sink, as [`StreamWriter::is_poisoned`] reports.
    Payload(PayloadError),
    /// The sink failed. It may hold part of a frame; the writer is poisoned.
    Sink {
        /// Bytes of this writer's output the sink had accepted before the
        /// failing call, including any earlier part of the failed frame.
        acknowledged_bytes: u64,
        /// Length of the chunk offered to the failing call.
        attempted_bytes: usize,
        /// The sink's own error, unchanged.
        cause: E,
    },
    /// An earlier failure left part of a frame in the sink; nothing was written.
    Poisoned,
}

impl<E: fmt::Display> fmt::Display for StreamError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Frame(cause) => cause.fmt(f),
            Self::Payload(cause) => cause.fmt(f),
            Self::Sink {
                acknowledged_bytes,
                cause,
                ..
            } => write!(
                f,
                "GTS stream sink failed after {acknowledged_bytes} acknowledged bytes: {cause}"
            ),
            Self::Poisoned => {
                f.write_str("GTS stream writer is poisoned: its sink holds part of a frame")
            }
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for StreamError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Frame(cause) => Some(cause),
            Self::Payload(cause) => Some(cause),
            Self::Sink { cause, .. } => Some(cause),
            Self::Poisoned => None,
        }
    }
}

impl<E: purrdf_hash::error::StructuredError> purrdf_hash::error::StructuredError
    for StreamError<E>
{
    purrdf_hash::error_visit! {
        Frame(cause) => "gts-stream-frame" { "cause" => Cause(cause) },
        Payload(cause) => "gts-stream-payload" { "cause" => Cause(cause) },
        Sink { acknowledged_bytes, attempted_bytes, cause } => "gts-stream-sink" {
            "acknowledged_bytes" => ByteCount(u128::from(*acknowledged_bytes)),
            "attempted_bytes" => ByteCount(*attempted_bytes as u128),
            "cause" => Cause(cause),
        },
        Poisoned => "gts-stream-poisoned" {},
    }
}

/// Frame a GTS log into a caller's sink, one frame at a time.
///
/// The bytes are those [`Writer`](super::Writer) accumulates for the same
/// calls: both run one frame encoder. Nothing is kept per frame, so the state
/// between calls is the 32-byte head, two byte counts and two reused buffers:
/// the frame buffer, which keeps the capacity of the largest frame encoded
/// whole, and, once a payload has been streamed from a reader, one 128 KiB
/// chunk.
///
/// # What is held in memory
///
/// * [`add_frame_with_options`](Self::add_frame_with_options) and
///   [`add_frame_signing`](Self::add_frame_signing) encode one whole frame and
///   hand it to the sink in a single call: memory is bounded by the largest
///   frame, never by the log. While a frame is encoded its data is held twice,
///   once in the frame being built and once in the frame buffer.
/// * [`add_frame_from_reader`](Self::add_frame_from_reader) with no transform
///   and no sealing reads the payload once and holds one 128 KiB chunk of it,
///   whatever its length. Public metadata, recipients and a signature do not
///   change that.
/// * With a transform or sealing, the same call holds the whole payload, its
///   encoded form, and a second copy of the encoded form while the frame is
///   written to the frame buffer. A frame's data is one definite-length byte
///   string, so the transformed length precedes the first transformed byte,
///   and the codecs and the AEAD here are single-shot.
///
/// Sealing panics on a payload longer than AES-GCM's plaintext limit (just
/// under 64 GiB), exactly as [`Writer`](super::Writer) does.
///
/// # Failures
///
/// A refused header or frame ([`StreamError::Frame`]) writes nothing and
/// leaves the writer usable. Once part of a frame has reached the sink and the
/// rest cannot follow — the sink failed, or a payload reader failed or ran
/// short while its frame was being streamed — the writer is poisoned: the head
/// and [`bytes_written`](Self::bytes_written) stay at the last complete frame
/// and every later frame returns [`StreamError::Poisoned`] without touching
/// the sink. To continue, cut the destination back to the offset at which
/// this writer started plus `bytes_written()`, and [`resume`](Self::resume)
/// from [`head`](Self::head). `bytes_written()` alone is not a position in the
/// destination: it starts at the header for [`new`](Self::new) and at zero for
/// `resume`.
///
/// # Not streamed
///
/// `index` footers (they need every frame's offset, type and id), composite
/// signatures, and the typed `add_terms`/`add_quads` conveniences. Nothing is
/// read back.
///
/// # Examples
///
/// Stream a header and two frames, then resume the chain from the stored head
/// with no earlier frame in memory. The result is the log a one-shot
/// [`Writer`](super::Writer) builds:
///
/// ```
/// use purrdf_gts::writer::{FrameOptions, StreamWriter, Writer, WriterOptions};
///
/// let frame = |text: &str| FrameOptions {
///     raw: Some(text.as_bytes().to_vec()),
///     ..FrameOptions::default()
/// };
///
/// let mut log = Vec::new();
/// let mut stream = StreamWriter::new(&mut log, "purrdf.gts", WriterOptions::default())?;
/// stream.add_frame_with_options("blob", frame("one"))?;
/// stream.add_frame_with_options("blob", frame("two"))?;
/// let head = stream.head().to_vec();
/// drop(stream);
///
/// let mut stream = StreamWriter::resume(&mut log, WriterOptions::default(), &head)?;
/// let last = stream.add_frame_with_options("blob", frame("three"))?;
/// assert_eq!(stream.head(), last.as_slice());
/// drop(stream);
///
/// let mut whole = Writer::new("purrdf.gts");
/// for text in ["one", "two", "three"] {
///     whole.add_frame_with_options("blob", frame(text))?;
/// }
/// assert_eq!(log, whole.into_bytes());
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug)]
pub struct StreamWriter<S: FrameSink> {
    sink: S,
    enc: FrameEncoder<Infallible>,
    prev: Vec<u8>,
    // Bytes of complete items (the header and whole frames) the sink accepted.
    written: u64,
    // Bytes the sink accepted in total, including part of a failed frame.
    acknowledged: u64,
    poisoned: bool,
    // One frame at a time; cleared, never shrunk.
    scratch: Vec<u8>,
    // The payload chunk of a frame read from a reader, allocated on first use.
    chunk: Vec<u8>,
}

impl<S: FrameSink> StreamWriter<S> {
    /// Write the header and start a chain at its id.
    ///
    /// The header reaches the sink in one call. Pass `&mut sink` to keep hold
    /// of an [`std::io::Write`] destination when this fails.
    ///
    /// # Errors
    /// [`StreamError::Frame`] when `options` are refused (nothing is written),
    /// [`StreamError::Sink`] when the sink refuses the header.
    pub fn new(
        sink: S,
        profile: &str,
        options: WriterOptions,
    ) -> Result<Self, StreamError<S::Error>> {
        let (enc, prev, header) =
            FrameEncoder::mint(profile, options).map_err(StreamError::Frame)?;
        let mut writer = Self::over(sink, enc, prev);
        writer.emit(&header)?;
        writer.written = writer.acknowledged;
        Ok(writer)
    }

    /// Continue a chain whose last item has the id `previous`, writing no
    /// header and holding no earlier frame.
    ///
    /// `options.catalog`, `options.dicts` and `options.zstd_level` MUST equal
    /// those the segment's header was written with: they decide the catalog
    /// ids a transformed or sealed frame names, and this writer cannot check
    /// them against a header it does not read. `meta`, `magic_tag` and
    /// `layout` describe a header and are not consulted.
    ///
    /// # Errors
    /// [`StreamError::Frame`] when `previous` is not a 32-byte id or the
    /// catalog options are refused.
    pub fn resume(
        sink: S,
        options: WriterOptions,
        previous: &[u8],
    ) -> Result<Self, StreamError<S::Error>> {
        if previous.len() != ID_LEN {
            return Err(StreamError::Frame(WriterError::InvalidFrame(
                FrameError::PreviousHashLength {
                    actual: previous.len(),
                },
            )));
        }
        let plan = plan_catalog(options.catalog, options.dicts, options.zstd_level)
            .map_err(StreamError::Frame)?;
        Ok(Self::over(sink, plan.into_encoder(), previous.to_vec()))
    }

    fn over(sink: S, enc: FrameEncoder<Infallible>, prev: Vec<u8>) -> Self {
        Self {
            sink,
            enc,
            prev,
            written: 0,
            acknowledged: 0,
            poisoned: false,
            scratch: Vec::new(),
            chunk: Vec::new(),
        }
    }

    /// Install the Ed25519 key that signs frames appended under
    /// [`FrameSigning::Installed`], with a text key id (§9.2).
    pub fn sign_with(&mut self, key: purrdf_ed25519::SigningKey, kid: &str) {
        self.sign_ed25519_with(key, kid.as_bytes());
    }

    /// Install the Ed25519 key that signs frames appended under
    /// [`FrameSigning::Installed`], with an opaque byte-string key id.
    pub fn sign_ed25519_with(&mut self, key: purrdf_ed25519::SigningKey, kid: &[u8]) {
        self.enc.sign_ed25519_with(key, kid);
    }

    /// The id of the last complete item: the header's before any frame, then
    /// each frame's. Store it to [`resume`](Self::resume) the chain, or to name
    /// this segment from the next one's header metadata.
    #[must_use]
    pub fn head(&self) -> &[u8] {
        &self.prev
    }

    /// Bytes of complete items this writer has handed to the sink: the header
    /// (for [`new`](Self::new)) and every whole frame. Part of a failed frame
    /// is not counted, so this is always a frame boundary.
    ///
    /// The count is this writer's own and starts at zero for
    /// [`resume`](Self::resume): it is not an offset in the destination. The
    /// last whole frame ends at the offset where this writer started plus
    /// this count.
    #[must_use]
    pub const fn bytes_written(&self) -> u64 {
        self.written
    }

    /// Whether a failure left part of a frame in the sink, so that no further
    /// frame is accepted.
    #[must_use]
    pub const fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Give the sink back. Nothing is flushed or synchronized.
    #[must_use]
    pub fn into_sink(self) -> S {
        self.sink
    }

    /// Append one frame, signed by the installed signer when there is one, and
    /// return its id. The same call as
    /// [`Writer::add_frame_with_options`](super::Writer::add_frame_with_options).
    ///
    /// # Errors
    /// See [`add_frame_signing`](Self::add_frame_signing).
    pub fn add_frame_with_options(
        &mut self,
        frame_type: &str,
        options: FrameOptions,
    ) -> Result<Vec<u8>, StreamError<S::Error>> {
        self.add_frame_signing(frame_type, options, FrameSigning::Installed)
    }

    /// Append one frame, choosing for this frame whether the installed signer
    /// signs it, and return its id.
    ///
    /// The frame is encoded whole and reaches the sink in one call.
    ///
    /// # Errors
    /// [`StreamError::Frame`] when the options are refused, including explicit
    /// signature bytes on a frame left [`FrameSigning::Unsigned`]: nothing is
    /// written. [`StreamError::Sink`] when the sink fails, which poisons the
    /// writer. [`StreamError::Poisoned`] after such a failure.
    pub fn add_frame_signing(
        &mut self,
        frame_type: &str,
        options: FrameOptions,
        signing: FrameSigning,
    ) -> Result<Vec<u8>, StreamError<S::Error>> {
        self.guard()?;
        let mut scratch = std::mem::take(&mut self.scratch);
        scratch.clear();
        let result = self
            .enc
            .encode_frame(&self.prev, frame_type, options, signing, &mut scratch)
            .map_err(StreamError::Frame)
            .and_then(|id| self.emit(&scratch).map(|()| id));
        self.scratch = scratch;
        let id = result?;
        self.complete(&id);
        Ok(id)
    }

    /// Append one frame whose payload is the `len` bytes `reader` yields, and
    /// return its id. The bytes are those of the same frame given
    /// `raw: Some(payload)`.
    ///
    /// `options.payload` and `options.raw` must be `None`: the reader is the
    /// source. The reader is read to its end and must yield exactly `len`
    /// bytes; no more than `len` reach the sink.
    ///
    /// With no transform and no sealing the payload is read once and streamed
    /// in bounded chunks. Otherwise it is read whole before anything is
    /// written (see [the memory notes](StreamWriter#what-is-held-in-memory)).
    ///
    /// # Errors
    /// [`StreamError::Frame`] when the options are refused: nothing is read or
    /// written. [`StreamError::Payload`] when the reader fails or disagrees
    /// with `len`, and [`StreamError::Sink`] when the sink fails: either
    /// poisons the writer once part of the frame is in the sink.
    /// [`StreamError::Poisoned`] after such a failure.
    pub fn add_frame_from_reader<R: Read>(
        &mut self,
        frame_type: &str,
        len: u64,
        mut reader: R,
        mut options: FrameOptions,
        signing: FrameSigning,
    ) -> Result<Vec<u8>, StreamError<S::Error>> {
        self.guard()?;
        if options.payload.is_some() || options.raw.is_some() {
            return Err(invalid(FrameError::ReaderSource));
        }
        if options.signature.is_some() && signing == FrameSigning::Unsigned {
            return Err(invalid(FrameError::ConflictingSigning));
        }
        if !options.transform.is_empty() || options.encrypt.is_some() {
            // Refuse a contradictory option set before reading a byte.
            self.enc
                .admit(
                    1,
                    &options.transform,
                    options.zstd_level,
                    options.dict.as_deref(),
                    options.encrypt.is_some(),
                )
                .map_err(StreamError::Frame)?;
            options.raw = Some(read_whole(&mut reader, len).map_err(StreamError::Payload)?);
            return self.add_frame_signing(frame_type, options, signing);
        }
        self.enc
            .admit(1, &[], options.zstd_level, options.dict.as_deref(), false)
            .map_err(StreamError::Frame)?;

        // Everything but the data, in authoring order.
        let mut envelope: Vec<(Value, Value)> = vec![("t".into(), frame_type.into())];
        push_envelope(
            &mut envelope,
            options.pub_meta,
            options.recipients,
            &self.prev,
        );
        let signed = self.enc.signs(options.signature.is_some(), signing);

        // Deterministic CBOR orders a map by its encoded keys, and `"d"` is
        // the least key a frame carries: a frame is its map head, then `"d"`,
        // then every other entry in the order `canonical` gives them. The id
        // preimage is that same sequence without `"id"` and `"sig"` (§6), so
        // one pass over the payload feeds both the sink and the hash.
        let preimage_entries = envelope.len() + 1;
        let mut hasher = purrdf_hash::blake3::Hasher::new();
        hasher.update(&data_prefix(preimage_entries, len));
        let frame_entries = preimage_entries + 1 + usize::from(signed);

        // From here part of the frame may be in the sink.
        self.poisoned = true;
        self.emit(&data_prefix(frame_entries, len))?;
        let mut chunk = std::mem::take(&mut self.chunk);
        chunk.resize(CHUNK_SIZE, 0);
        let copied = self.copy_payload(&mut reader, len, &mut chunk, &mut hasher);
        self.chunk = chunk;
        copied?;

        hasher.update(&entries_of(&envelope));
        let id = hasher.finalize().as_bytes().to_vec();
        envelope.push(("id".into(), Value::Bytes(id.clone())));
        if let Some(sig) = self
            .enc
            .sign(&id, options.signature, signing)
            .map_err(StreamError::Frame)?
        {
            envelope.push(("sig".into(), Value::Bytes(sig)));
        }
        self.emit(&entries_of(&envelope))?;
        self.poisoned = false;
        self.complete(&id);
        Ok(id)
    }

    /// Stream exactly `len` payload bytes from `reader` to the sink and the id
    /// hash, one chunk at a time.
    fn copy_payload<R: Read>(
        &mut self,
        reader: &mut R,
        len: u64,
        chunk: &mut [u8],
        hasher: &mut purrdf_hash::blake3::Hasher,
    ) -> Result<(), StreamError<S::Error>> {
        let mut copied = 0_u64;
        loop {
            let read = match reader.read(chunk) {
                Ok(0) => break,
                Ok(read) => read,
                Err(cause) if cause.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(cause) => return Err(StreamError::Payload(PayloadError::Read(cause.into()))),
            };
            let observed = copied.saturating_add(read as u64);
            if observed > len {
                return Err(StreamError::Payload(PayloadError::Length {
                    declared: len,
                    observed,
                }));
            }
            hasher.update(&chunk[..read]);
            self.emit(&chunk[..read])?;
            copied = observed;
        }
        if copied != len {
            return Err(StreamError::Payload(PayloadError::Length {
                declared: len,
                observed: copied,
            }));
        }
        Ok(())
    }

    fn guard(&self) -> Result<(), StreamError<S::Error>> {
        if self.poisoned {
            return Err(StreamError::Poisoned);
        }
        Ok(())
    }

    /// Hand `bytes` to the sink. A failure poisons the writer.
    fn emit(&mut self, bytes: &[u8]) -> Result<(), StreamError<S::Error>> {
        if let Err(cause) = self.sink.write_bytes(bytes) {
            self.poisoned = true;
            return Err(StreamError::Sink {
                acknowledged_bytes: self.acknowledged,
                attempted_bytes: bytes.len(),
                cause,
            });
        }
        self.acknowledged = self.acknowledged.saturating_add(bytes.len() as u64);
        Ok(())
    }

    /// Advance the chain past a frame the sink holds whole.
    fn complete(&mut self, id: &[u8]) {
        self.written = self.acknowledged;
        self.prev.clear();
        self.prev.extend_from_slice(id);
    }
}

fn invalid<E>(detail: FrameError) -> StreamError<E> {
    StreamError::Frame(WriterError::InvalidFrame(detail))
}

/// The start of a frame map of `entries` entries whose first is a `len`-byte
/// `"d"` byte string: the map head, the key and the byte-string head.
fn data_prefix(entries: usize, len: u64) -> Vec<u8> {
    let mut prefix = Vec::with_capacity(20);
    push_head(&mut prefix, MAP, entries as u64);
    write_text(&mut prefix, "d").expect("writing to a Vec cannot fail");
    push_head(&mut prefix, BYTES, len);
    prefix
}

/// The deterministic encoding of `entries` as a map, without the map's head:
/// the bytes that follow `"d"` in a frame holding these other entries.
fn entries_of(entries: &[(Value, Value)]) -> Vec<u8> {
    let mut map = Vec::new();
    write_canonical_map(entries, &[], &mut map).expect("writing to a Vec cannot fail");
    map.drain(..head_len(entries.len() as u64));
    map
}

/// Read a payload of exactly `len` bytes whole.
fn read_whole<R: Read>(reader: &mut R, len: u64) -> Result<Vec<u8>, PayloadError> {
    let mut payload = Vec::new();
    // One byte past the declared length is enough to see a reader that runs on.
    reader
        .take(len.saturating_add(1))
        .read_to_end(&mut payload)
        .map_err(|cause| PayloadError::Read(cause.into()))?;
    let observed = payload.len() as u64;
    if observed != len {
        return Err(PayloadError::Length {
            declared: len,
            observed,
        });
    }
    Ok(payload)
}
