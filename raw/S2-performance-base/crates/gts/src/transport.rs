// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Transport-encoding detection and decoding: gzip and zstd.
//!
//! A transport encoding is **not** a format. It wraps a byte stream whose payload
//! is decided somewhere else entirely — a tar archive for [`from_tar`](crate::from_tar),
//! an RDF document or a GTS file for the `purrdf` CLI — so the decision "is this
//! stream wrapped, and in what" belongs in exactly one place rather than being
//! re-derived (differently) by each consumer. This module is that place; it is the
//! single authority for the two magic-byte signatures, the recognized filename
//! suffixes, and the two decoders.
//!
//! Detection is **content-first**: gzip (`1f 8b`) and zstd (`28 b5 2f fd`) are
//! recognized from the leading bytes, and the filename suffix is consulted only when
//! the content says nothing. No RDF text syntax can collide with either signature —
//! both contain bytes that cannot appear at those positions in valid UTF-8 — so a
//! sniffed match is a decision, not a guess.
//!
//! Decoding is **all-or-nothing**: both decoders are `Read` adapters drained with
//! `read_to_end`, so a truncated or corrupt stream returns `Err` rather than the
//! prefix it managed to inflate. There is no partial success, and therefore no way
//! for a downstream parser to be handed a silently-shortened document.
//!
//! Decoding is also **whole-stream**. A gzip file is a series of members and zstd
//! data a series of frames (RFC 1952 §2.2, RFC 8878 §3.1); every member and every
//! frame is decoded, zstd skippable frames (magic `0x184D2A50`–`0x184D2A5F`) are
//! skipped, and bytes after the last member or frame that do not begin another one
//! are refused rather than ignored — a stream is never silently cut short at its
//! first member.
//!
//! ## Buffered and streaming halves
//!
//! [`decode_transport`] inflates into one `Vec<u8>`; [`transport_reader`] hands back a
//! `Read` that inflates INCREMENTALLY as its consumer pulls, so a streaming parser
//! never materializes the payload. Both wrap the same two decoders, so they cannot
//! disagree about what a stream decodes to. The streaming half preserves the
//! all-or-nothing property in the only form a stream can: a truncated or corrupt frame
//! surfaces as a read error at the point it is reached, and every consumer in this
//! workspace treats a mid-parse read error as a failed parse that produces no output.
//!
//! [`sniff_transport`] is the streaming counterpart of [`detect_transport`]'s
//! content-first sniff: it consumes only the leading magic-byte window and hands back a
//! reader with those bytes PUT BACK, so detection costs a fixed four bytes rather than
//! the whole stream. It works on a pipe, where seeking back is not available.

use std::borrow::Cow;
use std::fmt;
use std::io::{self, Chain, Cursor, Read};

use purrdf_deflate::GzipReader;
use structured_zstd::decoding::{BlockDecodingStrategy, FrameDecoder};

/// The gzip magic bytes (RFC 1952 §2.3.1: `ID1 = 0x1f`, `ID2 = 0x8b`).
const GZIP_MAGIC: &[u8] = &[0x1f, 0x8b];

/// The zstd frame magic number (RFC 8878 §3.1.1: `0xFD2FB528`, little-endian).
const ZSTD_MAGIC: &[u8] = &[0x28, 0xb5, 0x2f, 0xfd];

/// The zstd frame magic number as a value.
const ZSTD_FRAME_MAGIC: u32 = 0xFD2F_B528;

/// Skippable frames carry a magic number `0x184D2A5?` (RFC 8878 §3.1.2): the high
/// 28 bits are fixed and the low four are free.
const ZSTD_SKIPPABLE_MASK: u32 = 0xFFFF_FFF0;
const ZSTD_SKIPPABLE_MAGIC: u32 = 0x184D_2A50;

/// A recognized transport encoding wrapping a payload byte stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportEncoding {
    /// gzip (RFC 1952), decoded by the workspace's native `purrdf-deflate`.
    Gzip,
    /// zstd (RFC 8878): frames decoded by the pure-Rust `structured-zstd` frame
    /// decoder, sequenced frame by frame here.
    Zstd,
}

impl TransportEncoding {
    /// The canonical lowercase token naming this encoding (`"gzip"` / `"zstd"`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Gzip => "gzip",
            Self::Zstd => "zstd",
        }
    }
}

impl fmt::Display for TransportEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A transport decode failure: the encoding that was applied and why it failed.
///
/// A truncated stream, a corrupt frame, and a stream that is not the claimed
/// encoding at all all arrive here — every one of them as an error, never as a
/// short read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportError {
    /// The encoding whose decoder failed.
    pub encoding: TransportEncoding,
    /// The underlying decoder's message.
    pub message: String,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} decode failed: {}", self.encoding, self.message)
    }
}

impl std::error::Error for TransportError {}

/// Detect the transport encoding wrapping `data`.
///
/// The leading bytes decide first; `source_name` (a path or other label) is a
/// fallback consulted only when the content carries neither signature, so a stream
/// named `.gz` that is not gzip is reported as gzip and then hard-fails in
/// [`decode_transport`] rather than being read as a plain payload.
// Matching stays byte-exact on the (already lowercased) name; multi-part suffixes
// like ".tar.gz" cannot be expressed via Path::extension.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
pub fn detect_transport(data: &[u8], source_name: Option<&str>) -> Option<TransportEncoding> {
    if data.starts_with(GZIP_MAGIC) {
        return Some(TransportEncoding::Gzip);
    }
    if data.starts_with(ZSTD_MAGIC) {
        return Some(TransportEncoding::Zstd);
    }
    let name = source_name?.to_ascii_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") || name.ends_with(".gz") {
        Some(TransportEncoding::Gzip)
    } else if name.ends_with(".tar.zst") || name.ends_with(".tzst") || name.ends_with(".zst") {
        Some(TransportEncoding::Zstd)
    } else {
        None
    }
}

/// The transport suffix `name` carries, and `name` with that suffix removed.
///
/// This is the *naming* half of [`detect_transport`]: a consumer that infers a
/// payload format from a file extension must strip the transport suffix first, or
/// it will try to classify `.gz` as an RDF syntax. Returns `None` when `name` carries
/// no recognized transport suffix.
///
/// The suffix table is ordered longest-first so `.tar.gz` strips whole rather than
/// leaving a stray `.tar` behind.
#[allow(clippy::case_sensitive_file_extension_comparisons)]
pub fn strip_transport_suffix(name: &str) -> Option<(&str, TransportEncoding)> {
    let lower = name.to_ascii_lowercase();
    [
        (".tar.gz", TransportEncoding::Gzip),
        (".tgz", TransportEncoding::Gzip),
        (".gz", TransportEncoding::Gzip),
        (".tar.zst", TransportEncoding::Zstd),
        (".tzst", TransportEncoding::Zstd),
        (".zst", TransportEncoding::Zstd),
    ]
    .into_iter()
    .find(|(suffix, _)| lower.ends_with(suffix))
    .map(|(suffix, encoding)| (&name[..name.len() - suffix.len()], encoding))
}

/// Decode `data` under `encoding`, draining the decoder to completion.
///
/// This drains exactly the reader [`transport_reader`] builds, so the buffered and
/// streaming halves cannot disagree. A truncated or corrupt stream, or bytes after
/// the last member or frame that do not begin another, return `Err` and **no**
/// bytes: there is no partial success.
pub fn decode_transport(
    data: &[u8],
    encoding: TransportEncoding,
) -> Result<Vec<u8>, TransportError> {
    let mut out = Vec::new();
    transport_reader(data, Some(encoding))?
        .read_to_end(&mut out)
        .map_err(|err| TransportError {
            encoding,
            message: err.to_string(),
        })?;
    Ok(out)
}

/// The widest magic-byte signature this module recognizes (zstd's four-byte frame
/// magic). [`sniff_transport`] consumes exactly this many bytes and puts them back.
const MAGIC_WINDOW: usize = 4;

/// A stream whose leading magic-byte window has been read and prepended back, so the
/// consumer still sees every byte of the original stream in order.
///
/// Named because [`sniff_transport`] returns it and callers must be able to write the
/// type down (it is the `R` of the [`TransportReader`] they then build).
pub type SniffedStream<R> = Chain<Cursor<Vec<u8>>, R>;

/// Detect the transport encoding wrapping a `Read` stream WITHOUT consuming it.
///
/// The streaming counterpart of [`detect_transport`], and it applies the identical
/// content-first-then-name rule to the identical decision function — this reads the
/// leading four-byte magic window, passes it to [`detect_transport`], and returns a
/// reader with those bytes put back in front. Nothing is seeked, so this is correct on
/// a pipe or on standard input.
///
/// A stream shorter than that window is not an error here: the short prefix is sniffed
/// as-is (no signature will match) and handed back intact.
///
/// # Errors
///
/// Returns the underlying reader's error if the leading window cannot be read.
pub fn sniff_transport<R: Read>(
    mut reader: R,
    source_name: Option<&str>,
) -> io::Result<(Option<TransportEncoding>, SniffedStream<R>)> {
    let mut window = [0u8; MAGIC_WINDOW];
    let mut filled = 0;
    while filled < MAGIC_WINDOW {
        match reader.read(&mut window[filled..])? {
            0 => break,
            read => filled += read,
        }
    }
    let encoding = detect_transport(&window[..filled], source_name);
    Ok((
        encoding,
        Cursor::new(window[..filled].to_vec()).chain(reader),
    ))
}

/// A `Read` stream whose transport wrapper is decoded INCREMENTALLY as the consumer
/// pulls, rather than inflated into one buffer first.
///
/// The streaming twin of [`decode_transport`], built by [`transport_reader`]. The
/// wrapped decoder types are deliberately private: this is a `Read`, and which
/// third-party decoder is behind it is an implementation detail rather than API.
pub struct TransportReader<R: Read> {
    inner: TransportReaderInner<R>,
}

/// The decoder behind a [`TransportReader`], or the undecoded stream itself.
enum TransportReaderInner<R: Read> {
    /// No transport wrapper: bytes pass through untouched.
    Plain(R),
    /// gzip, every member decoded by `purrdf-deflate`.
    // Boxed: the decoder owns its tables and input buffer, and an unboxed variant
    // would make every `TransportReader` — including `Plain` — that large.
    Gzip(Box<GzipReader<R>>),
    /// zstd, every frame decoded. Boxed for the same reason as [`Self::Gzip`].
    Zstd(Box<ZstdFrames<R>>),
}

/// A `Read` over zstd data that decodes it frame by frame (RFC 8878 §3.1).
///
/// Between frames it reads the next four-byte magic number itself: a Zstandard
/// frame is handed to `structured-zstd`'s [`FrameDecoder`], which reads exactly that
/// frame; a skippable frame's user data is skipped by its `Frame_Size`; a clean end
/// of input after at least one frame ends the stream; anything else — a partial
/// magic number, or bytes that begin no frame — is refused.
struct ZstdFrames<R: Read> {
    inner: Counted<R>,
    decoder: FrameDecoder,
    in_frame: bool,
    frames: u64,
    done: bool,
}

/// A reader that counts the bytes it yields, so a decode step that consumed nothing
/// and produced nothing is recognized as stuck rather than retried forever.
struct Counted<R> {
    inner: R,
    count: u64,
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.count += n as u64;
        Ok(n)
    }
}

/// Decoded bytes one decode step asks the frame decoder for.
const ZSTD_STEP: usize = 128 * 1024;

fn zstd_error(message: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

impl<R: Read> ZstdFrames<R> {
    /// Start decoding, reading up to the first frame's header so a stream that is
    /// not zstd at all is refused here.
    fn new(inner: R) -> io::Result<Self> {
        let mut frames = Self {
            inner: Counted { inner, count: 0 },
            decoder: FrameDecoder::new(),
            in_frame: false,
            frames: 0,
            done: false,
        };
        frames.next_frame()?;
        Ok(frames)
    }

    /// Read into `buf` until it is full or the source ends; the count read.
    fn fill(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut filled = 0;
        while filled < buf.len() {
            match self.inner.read(&mut buf[filled..]) {
                Ok(0) => break,
                Ok(n) => filled += n,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => return Err(err),
            }
        }
        Ok(filled)
    }

    /// Between frames: skip skippable frames until a Zstandard frame starts
    /// (`in_frame`) or the input ends cleanly (`done`).
    fn next_frame(&mut self) -> io::Result<()> {
        loop {
            let mut magic = [0u8; 4];
            let got = self.fill(&mut magic)?;
            if got == 0 {
                if self.frames == 0 {
                    return Err(zstd_error("no zstd frame in the input"));
                }
                self.done = true;
                return Ok(());
            }
            if got < magic.len() {
                return Err(zstd_error(format!(
                    "{got} byte(s) after the last zstd frame do not begin another frame"
                )));
            }
            let value = u32::from_le_bytes(magic);
            if value == ZSTD_FRAME_MAGIC {
                let mut source = magic.as_slice().chain(&mut self.inner);
                self.decoder.reset(&mut source).map_err(zstd_error)?;
                self.frames += 1;
                self.in_frame = true;
                return Ok(());
            }
            if value & ZSTD_SKIPPABLE_MASK == ZSTD_SKIPPABLE_MAGIC {
                let mut size = [0u8; 4];
                if self.fill(&mut size)? < size.len() {
                    return Err(zstd_error(
                        "skippable zstd frame truncated in its size field",
                    ));
                }
                let size = u64::from(u32::from_le_bytes(size));
                let skipped = io::copy(&mut (&mut self.inner).take(size), &mut io::sink())?;
                if skipped < size {
                    return Err(zstd_error(format!(
                        "skippable zstd frame truncated: {skipped} of {size} bytes"
                    )));
                }
                self.frames += 1;
                continue;
            }
            return Err(zstd_error(format!(
                "bytes {magic:02x?} after the last zstd frame do not begin another frame"
            )));
        }
    }
}

impl<R: Read> Read for ZstdFrames<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            if self.in_frame {
                let n = self.decoder.read(buf)?;
                if n > 0 {
                    return Ok(n);
                }
                if self.decoder.is_finished() {
                    self.in_frame = false;
                    continue;
                }
                let before = self.inner.count;
                self.decoder
                    .decode_blocks(&mut self.inner, BlockDecodingStrategy::UptoBytes(ZSTD_STEP))
                    .map_err(zstd_error)?;
                if self.inner.count == before
                    && self.decoder.can_collect() == 0
                    && !self.decoder.is_finished()
                {
                    return Err(zstd_error("zstd frame truncated"));
                }
                continue;
            }
            if self.done {
                return Ok(0);
            }
            self.next_frame()?;
        }
    }
}

impl<R: Read> fmt::Debug for TransportReader<R> {
    /// Names the encoding in force. The wrapped decoders hold multi-kilobyte windows
    /// and are not themselves `Debug`, so the encoding is the whole of the useful
    /// state a diagnostic wants.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let encoding = match self.inner {
            TransportReaderInner::Plain(_) => "none",
            TransportReaderInner::Gzip(_) => TransportEncoding::Gzip.as_str(),
            TransportReaderInner::Zstd(_) => TransportEncoding::Zstd.as_str(),
        };
        f.debug_struct("TransportReader")
            .field("encoding", &encoding)
            .finish_non_exhaustive()
    }
}

impl<R: Read> Read for TransportReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.inner {
            TransportReaderInner::Plain(reader) => reader.read(buf),
            TransportReaderInner::Gzip(decoder) => decoder.read(buf),
            TransportReaderInner::Zstd(decoder) => decoder.read(buf),
        }
    }
}

/// Wrap `reader` in the decoder for `encoding`, or pass it through when `encoding` is
/// `None`.
///
/// This is the streaming half of [`decode_transport`]: the same two decoders, driven
/// incrementally. A truncated or corrupt frame surfaces as an `io::Error` from
/// [`Read::read`] at the point the damage is reached — never as a silent short read —
/// so a parser driven from this stream fails rather than seeing a shortened document.
///
/// # Errors
///
/// Returns [`TransportError`] when the stream does not begin with a readable zstd
/// frame header (a leading skippable frame is skipped first). gzip's adapter is
/// infallible to construct and reports damage on the first read instead.
pub fn transport_reader<R: Read>(
    reader: R,
    encoding: Option<TransportEncoding>,
) -> Result<TransportReader<R>, TransportError> {
    let inner = match encoding {
        None => TransportReaderInner::Plain(reader),
        Some(TransportEncoding::Gzip) => {
            TransportReaderInner::Gzip(Box::new(GzipReader::new(reader)))
        }
        Some(TransportEncoding::Zstd) => {
            TransportReaderInner::Zstd(Box::new(ZstdFrames::new(reader).map_err(|err| {
                TransportError {
                    encoding: TransportEncoding::Zstd,
                    message: err.to_string(),
                }
            })?))
        }
    };
    Ok(TransportReader { inner })
}

/// Detect and decode in one step, borrowing `data` unchanged when it is not wrapped.
pub fn decode_detected<'a>(
    data: &'a [u8],
    source_name: Option<&str>,
) -> Result<Cow<'a, [u8]>, TransportError> {
    match detect_transport(data, source_name) {
        None => Ok(Cow::Borrowed(data)),
        Some(encoding) => Ok(Cow::Owned(decode_transport(data, encoding)?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gzip(payload: &[u8]) -> Vec<u8> {
        purrdf_deflate::gzip::compress(payload, purrdf_deflate::Level::DEFAULT)
    }

    #[test]
    fn detection_is_content_first_then_name() {
        let framed = gzip(b"hello");
        assert_eq!(
            detect_transport(&framed, None),
            Some(TransportEncoding::Gzip)
        );
        assert_eq!(detect_transport(b"hello", None), None);
        assert_eq!(
            detect_transport(b"hello", Some("a.nt.gz")),
            Some(TransportEncoding::Gzip)
        );
        assert_eq!(
            detect_transport(b"hello", Some("a.nt.ZST")),
            Some(TransportEncoding::Zstd)
        );
        assert_eq!(detect_transport(b"hello", Some("a.nt")), None);
    }

    #[test]
    fn gzip_round_trips_and_a_truncated_stream_hard_fails() {
        let framed = gzip(b"<http://example.org/s> <http://example.org/p> \"o\" .\n");
        let decoded = decode_transport(&framed, TransportEncoding::Gzip).expect("decode");
        assert!(decoded.starts_with(b"<http://example.org/s>"));

        // Truncation is an error, not a short read: no prefix escapes the decoder.
        let truncated = &framed[..framed.len() - 4];
        let err = decode_transport(truncated, TransportEncoding::Gzip)
            .expect_err("a truncated gzip stream must fail");
        assert_eq!(err.encoding, TransportEncoding::Gzip);
    }

    #[test]
    fn a_stream_that_is_not_the_claimed_encoding_fails() {
        let err = decode_transport(b"plain text", TransportEncoding::Gzip)
            .expect_err("plain bytes are not gzip");
        assert_ne!(err.to_string(), "");
        let err = decode_transport(b"plain text", TransportEncoding::Zstd)
            .expect_err("plain bytes are not zstd");
        assert_ne!(err.to_string(), "");
    }

    #[test]
    fn suffix_stripping_recovers_the_payload_name() {
        assert_eq!(
            strip_transport_suffix("data.nt.gz"),
            Some(("data.nt", TransportEncoding::Gzip))
        );
        assert_eq!(
            strip_transport_suffix("data.trig.ZST"),
            Some(("data.trig", TransportEncoding::Zstd))
        );
        assert_eq!(
            strip_transport_suffix("archive.tar.gz"),
            Some(("archive", TransportEncoding::Gzip))
        );
        assert_eq!(strip_transport_suffix("data.nt"), None);
    }

    /// Read a stream through the sniff + streaming-decoder pair, the way a streaming
    /// parser does.
    fn stream_decode(data: &[u8], source_name: Option<&str>) -> io::Result<Vec<u8>> {
        let (encoding, stream) = sniff_transport(data, source_name)?;
        let mut reader =
            transport_reader(stream, encoding).map_err(|err| io::Error::other(err.to_string()))?;
        let mut out = Vec::new();
        reader.read_to_end(&mut out)?;
        Ok(out)
    }

    #[test]
    fn streaming_decode_matches_the_buffered_decode_byte_for_byte() {
        // The two halves must never disagree about what a stream decodes to: same
        // bytes, wrapped or not, gzip or plain.
        let payload = b"<http://example.org/s> <http://example.org/p> \"o\" .\n".repeat(64);
        for (data, name) in [(gzip(&payload), None), (payload.clone(), Some("plain.nt"))] {
            let buffered = decode_detected(&data, name)
                .expect("buffered decode")
                .into_owned();
            let streamed = stream_decode(&data, name).expect("streaming decode");
            assert_eq!(
                buffered, streamed,
                "streaming and buffered decode must agree"
            );
            assert_eq!(streamed, payload);
        }
    }

    #[test]
    fn sniff_puts_the_magic_window_back() {
        // A short, unwrapped stream must survive the sniff with every byte intact,
        // including one shorter than the magic window itself.
        for payload in [b"".as_slice(), b"a", b"abc", b"abcdefgh"] {
            let (encoding, mut stream) = sniff_transport(payload, None).expect("sniff");
            assert_eq!(encoding, None);
            let mut out = Vec::new();
            stream.read_to_end(&mut out).expect("read");
            assert_eq!(out, payload);
        }
    }

    #[test]
    fn a_truncated_stream_fails_the_streaming_decoder_too() {
        // All-or-nothing survives the streaming form: the damage is reported as a read
        // error, so a parser driven from this stream fails instead of seeing a
        // shortened document.
        let framed = gzip(&b"<http://example.org/s> <http://example.org/p> \"o\" .\n".repeat(64));
        let truncated = &framed[..framed.len() - 8];
        stream_decode(truncated, None).expect_err("a truncated gzip stream must fail");
    }

    #[test]
    fn a_stream_that_is_not_the_declared_encoding_fails_the_streaming_decoder() {
        let mut reader = transport_reader(b"plain text".as_slice(), Some(TransportEncoding::Gzip))
            .expect("gzip adapter constructs");
        let mut out = Vec::new();
        reader
            .read_to_end(&mut out)
            .expect_err("plain bytes are not gzip");
        transport_reader(b"plain text".as_slice(), Some(TransportEncoding::Zstd))
            .expect_err("plain bytes are not a zstd frame");
    }

    fn zstd(payload: &[u8]) -> Vec<u8> {
        structured_zstd::encoding::compress_to_vec(
            payload,
            structured_zstd::encoding::CompressionLevel::Fastest,
        )
    }

    /// A skippable frame (RFC 8878 §3.1.2) with magic nibble `nibble`.
    fn skippable(nibble: u8, user_data: &[u8]) -> Vec<u8> {
        let mut frame = vec![0x50 | nibble, 0x2a, 0x4d, 0x18];
        frame.extend_from_slice(&(user_data.len() as u32).to_le_bytes());
        frame.extend_from_slice(user_data);
        frame
    }

    /// Both halves, which must agree on every stream they accept.
    fn both(data: &[u8], encoding: TransportEncoding) -> Result<Vec<u8>, String> {
        let buffered = decode_transport(data, encoding).map_err(|e| e.to_string());
        let streamed = transport_reader(data, Some(encoding))
            .map_err(|e| e.to_string())
            .and_then(|mut reader| {
                let mut out = Vec::new();
                reader
                    .read_to_end(&mut out)
                    .map(|_| out)
                    .map_err(|e| e.to_string())
            });
        assert_eq!(
            buffered.is_ok(),
            streamed.is_ok(),
            "buffered {buffered:?} vs streamed {streamed:?}"
        );
        if let (Ok(a), Ok(b)) = (&buffered, &streamed) {
            assert_eq!(a, b);
        }
        buffered
    }

    #[test]
    fn zstd_two_frames_both_decoded() {
        let data = [zstd(b"first frame, "), zstd(b"second frame")].concat();
        assert_eq!(
            both(&data, TransportEncoding::Zstd).expect("two frames"),
            b"first frame, second frame"
        );
        // One frame alone is the neighbour: exactly its own payload.
        assert_eq!(
            both(&zstd(b"first frame, "), TransportEncoding::Zstd).expect("one frame"),
            b"first frame, "
        );
    }

    #[test]
    fn zstd_skippable_frame_skipped() {
        let a = zstd(b"alpha ");
        let b = zstd(b"beta");
        for data in [
            [skippable(0, b"meta"), a.clone(), b.clone()].concat(),
            [a.clone(), skippable(0xf, b"x"), b.clone()].concat(),
            [a.clone(), b, skippable(7, b"")].concat(),
        ] {
            assert_eq!(
                both(&data, TransportEncoding::Zstd).expect("skippable"),
                b"alpha beta"
            );
        }
        // A stream of skippable frames only is a stream of zero bytes.
        assert_eq!(
            both(&skippable(1, b"only"), TransportEncoding::Zstd).expect("only skippable"),
            b""
        );
        // A skippable frame whose data is cut short is refused.
        let mut cut = [a.clone(), skippable(0, b"metadata")].concat();
        cut.truncate(cut.len() - 1);
        both(&cut, TransportEncoding::Zstd).expect_err("truncated skippable frame");
        // Magic numbers just outside the skippable range are not skippable.
        let mut outside = skippable(0, b"meta");
        outside[0] = 0x60;
        both(&[a, outside].concat(), TransportEncoding::Zstd)
            .expect_err("0x184D2A60 is not a skippable magic");
    }

    #[test]
    fn zstd_trailing_bytes_refused() {
        let frame = zstd(b"payload");
        assert_eq!(
            both(&frame, TransportEncoding::Zstd).expect("valid"),
            b"payload"
        );
        for trailing in [&b"junk"[..], b"\0", b"\x28\xb5\x2f", b"\0\0\0\0\0\0\0\0"] {
            both(
                &[frame.as_slice(), trailing].concat(),
                TransportEncoding::Zstd,
            )
            .expect_err("trailing bytes after the last frame");
        }
        both(b"", TransportEncoding::Zstd).expect_err("no frame at all");
        let mut truncated = frame;
        truncated.pop();
        both(&truncated, TransportEncoding::Zstd).expect_err("a truncated frame");
    }

    #[test]
    fn gzip_transport_decodes_every_member_and_refuses_garbage() {
        let data = [gzip(b"one "), gzip(b"two")].concat();
        assert_eq!(
            both(&data, TransportEncoding::Gzip).expect("members"),
            b"one two"
        );
        both(
            &[data.as_slice(), b"garbage"].concat(),
            TransportEncoding::Gzip,
        )
        .expect_err("trailing garbage");
    }

    #[test]
    fn decode_detected_borrows_an_unwrapped_stream() {
        let plain = b"not wrapped";
        let decoded = decode_detected(plain, Some("a.nt")).expect("decode");
        assert!(matches!(decoded, Cow::Borrowed(_)));
        assert_eq!(decoded.as_ref(), plain);
    }
}
