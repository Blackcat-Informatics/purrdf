// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RFC 1952 gzip: member framing over [`crate::inflate`] and
//! [`crate::deflate`].
//!
//! A gzip file is a series of members with nothing before, between or after
//! them (RFC 1952 §2.2). The decoder decodes **every** member, checks each
//! trailer (CRC-32 and `ISIZE`, the latter modulo 2^32 so a member over 4 GiB
//! decodes), and refuses bytes after the last member that do not begin
//! another. The header's optional fields (`FEXTRA`, `FNAME`, `FCOMMENT`) are
//! parsed and skipped, `FHCRC` is verified, `CM` must be 8 and the reserved
//! `FLG` bits must be clear (§2.3.1.2).
//!
//! The encoder writes one member with `FLG = 0`, `MTIME = 0`, `XFL = 0` and
//! `OS = 255` (unknown), so its bytes carry no time and no host.

use std::io::{self, Read, Write};

use purrdf_hash::crc32::Crc32;

use crate::backend::Backend;
use crate::deflate::{Deflater, Level};
use crate::error::Error;
use crate::inflate::{Inflater, Progress, Sink, SliceSink, Status, VecSink};

/// Checksum the bytes accepted by the destination while they are still hot
/// in the inflater's window. A full slice leaves undelivered bytes untouched.
struct CheckedSink<'a, S> {
    output: &'a mut S,
    crc: &'a mut Crc32,
}

impl<S: Sink> Sink for CheckedSink<'_, S> {
    fn put(&mut self, bytes: &[u8]) -> usize {
        let n = self.output.put(bytes);
        self.crc.update(&bytes[..n]);
        n
    }

    fn written(&self) -> usize {
        self.output.written()
    }
}

const ID1: u8 = 0x1f;
const ID2: u8 = 0x8b;
const CM_DEFLATE: u8 = 8;
const FHCRC: u8 = 1 << 1;
const FEXTRA: u8 = 1 << 2;
const FNAME: u8 = 1 << 3;
const FCOMMENT: u8 = 1 << 4;
const FRESERVED: u8 = 0xE0;

/// The header the encoder writes: ID1 ID2 CM FLG MTIME(4) XFL OS.
const HEADER: [u8; 10] = [ID1, ID2, CM_DEFLATE, 0, 0, 0, 0, 0, 0, 255];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// The fixed ten bytes; `have` gathered so far.
    Fixed {
        have: usize,
    },
    ExtraLength {
        have: usize,
    },
    Extra {
        remaining: u16,
    },
    Name,
    Comment,
    HeaderCrc {
        have: usize,
    },
    Body,
    Trailer {
        have: usize,
    },
    /// After a whole member: the input may end here, or another member begin.
    Boundary,
}

/// A push-based gzip decoder: [`Self::feed`] input in chunks of any size,
/// then [`Self::finish`] when the input has ended.
pub struct GzipDecoder {
    stage: Stage,
    flags: u8,
    fixed: [u8; 10],
    small: [u8; 8],
    header_crc: Crc32,
    crc: Crc32,
    inflater: Inflater,
    /// Members completed.
    members: u64,
    /// Output of the member in progress.
    member_out: u64,
    /// Output of all completed members.
    completed_out: u64,
    /// Input bytes consumed so far, for error offsets.
    total_in: u64,
    /// Offset of the first byte of the member in progress.
    member_start: u64,
    limit: u64,
    error: Option<Error>,
}

impl std::fmt::Debug for GzipDecoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GzipDecoder")
            .field("stage", &self.stage)
            .field("members", &self.members)
            .field("total_out", &self.total_out())
            .finish_non_exhaustive()
    }
}

impl Default for GzipDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl GzipDecoder {
    /// A decoder on the fastest kernel path this processor supports.
    pub fn new() -> Self {
        Self::with_inflater(Inflater::new())
    }

    /// A decoder pinned to `backend`, if this processor can run it.
    pub fn with_backend(backend: Backend) -> Option<Self> {
        Inflater::with_backend(backend).map(Self::with_inflater)
    }

    fn with_inflater(inflater: Inflater) -> Self {
        Self {
            stage: Stage::Fixed { have: 0 },
            flags: 0,
            fixed: [0; 10],
            small: [0; 8],
            header_crc: Crc32::new(),
            crc: Crc32::new(),
            inflater,
            members: 0,
            member_out: 0,
            completed_out: 0,
            total_in: 0,
            member_start: 0,
            limit: u64::MAX,
            error: None,
        }
    }

    /// Refuse, with [`Error::LimitExceeded`], input that decodes to more than
    /// `limit` bytes over all members. Exactly `limit` bytes is accepted.
    pub fn set_limit(&mut self, limit: u64) {
        self.limit = limit;
    }

    /// Members decoded and verified so far.
    pub fn members(&self) -> u64 {
        self.members
    }

    /// Bytes decoded so far over all members.
    pub fn total_out(&self) -> u64 {
        self.completed_out + self.member_out
    }

    /// Decode from `input` into `output`. Returns [`Status::NeedsInput`] when
    /// all input is consumed and all output delivered (call [`Self::finish`]
    /// if the input has ended), or [`Status::OutputFull`].
    pub fn feed(&mut self, input: &[u8], output: &mut [u8]) -> Result<Progress, Error> {
        self.feed_sink(
            input,
            &mut SliceSink {
                out: output,
                written: 0,
            },
        )
    }

    /// Append decoded bytes directly to `output`, checking every member's
    /// trailer. Never returns [`Status::OutputFull`]. Output already appended
    /// is retained on error, as with [`Self::feed`].
    pub fn feed_to_vec(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<Progress, Error> {
        self.feed_sink(
            input,
            &mut VecSink {
                out: output,
                written: 0,
            },
        )
    }

    fn feed_sink<S: Sink>(&mut self, input: &[u8], output: &mut S) -> Result<Progress, Error> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let result = self.run(input, output);
        if let Err(error) = &result {
            self.error = Some(error.clone());
        }
        result
    }

    /// Declare the end of the input. Succeeds only at a member boundary after
    /// at least one member.
    pub fn finish(&self) -> Result<(), Error> {
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        match self.stage {
            Stage::Boundary => Ok(()),
            Stage::Fixed { have: 0 } if self.members == 0 => Err(Error::Empty),
            _ => Err(Error::Truncated),
        }
    }

    fn run<S: Sink>(&mut self, input: &[u8], output: &mut S) -> Result<Progress, Error> {
        let mut ip = 0;
        let mut written = 0;
        loop {
            match self.stage {
                Stage::Body => {
                    let progress = self
                        .inflater
                        .feed_sink(
                            &input[ip..],
                            &mut CheckedSink {
                                output,
                                crc: &mut self.crc,
                            },
                        )
                        .map_err(|error| match error {
                            Error::LimitExceeded { .. } => {
                                Error::LimitExceeded { limit: self.limit }
                            }
                            other => other,
                        })?;
                    self.member_out += progress.written as u64;
                    written += progress.written;
                    ip += progress.consumed;
                    self.total_in += progress.consumed as u64;
                    match progress.status {
                        Status::StreamEnd => {
                            // Up to seven trailer bytes were already consumed
                            // into the inflater's bit buffer.
                            let tail = self.inflater.trailing_bytes();
                            self.small[..tail.len()].copy_from_slice(tail);
                            self.stage = Stage::Trailer { have: tail.len() };
                        }
                        status => {
                            return Ok(Progress {
                                consumed: ip,
                                written,
                                status,
                            });
                        }
                    }
                }
                Stage::Trailer { have } => {
                    let n = (8 - have).min(input.len() - ip);
                    self.small[have..have + n].copy_from_slice(&input[ip..ip + n]);
                    ip += n;
                    self.total_in += n as u64;
                    if have + n < 8 {
                        self.stage = Stage::Trailer { have: have + n };
                        return Ok(self.need_input(ip, written));
                    }
                    self.end_member()?;
                }
                _ if ip == input.len() => return Ok(self.need_input(ip, written)),
                Stage::Boundary => {
                    self.member_start = self.total_in;
                    self.stage = Stage::Fixed { have: 0 };
                }
                Stage::Fixed { have } => {
                    let byte = input[ip];
                    ip += 1;
                    self.total_in += 1;
                    self.fixed[have] = byte;
                    let bad_magic = match have {
                        0 => byte != ID1,
                        1 => byte != ID2,
                        _ => false,
                    };
                    if bad_magic {
                        return Err(if self.members == 0 {
                            Error::NotGzip
                        } else {
                            Error::TrailingGarbage {
                                offset: self.member_start,
                            }
                        });
                    }
                    if have == 2 && byte != CM_DEFLATE {
                        return Err(Error::UnsupportedMethod { method: byte });
                    }
                    if have == 3 && byte & FRESERVED != 0 {
                        return Err(Error::ReservedFlags { flags: byte });
                    }
                    if have < 9 {
                        self.stage = Stage::Fixed { have: have + 1 };
                        continue;
                    }
                    self.flags = self.fixed[3];
                    self.header_crc = Crc32::new();
                    self.header_crc.update(&self.fixed);
                    self.stage = self.after(FEXTRA - 1);
                }
                Stage::ExtraLength { have } => {
                    self.small[have] = input[ip];
                    self.header_crc.update(&input[ip..=ip]);
                    ip += 1;
                    self.total_in += 1;
                    if have == 0 {
                        self.stage = Stage::ExtraLength { have: 1 };
                    } else {
                        let remaining = u16::from_le_bytes([self.small[0], self.small[1]]);
                        self.stage = if remaining == 0 {
                            self.after(FEXTRA)
                        } else {
                            Stage::Extra { remaining }
                        };
                    }
                }
                Stage::Extra { remaining } => {
                    let n = usize::from(remaining).min(input.len() - ip);
                    self.header_crc.update(&input[ip..ip + n]);
                    ip += n;
                    self.total_in += n as u64;
                    let left = remaining - n as u16;
                    self.stage = if left == 0 {
                        self.after(FEXTRA)
                    } else {
                        Stage::Extra { remaining: left }
                    };
                }
                Stage::Name | Stage::Comment => {
                    let rest = &input[ip..];
                    let (n, ended) = rest
                        .iter()
                        .position(|&b| b == 0)
                        .map_or((rest.len(), false), |z| (z + 1, true));
                    self.header_crc.update(&rest[..n]);
                    ip += n;
                    self.total_in += n as u64;
                    if ended {
                        self.stage = self.after(if self.stage == Stage::Name {
                            FNAME
                        } else {
                            FCOMMENT
                        });
                    }
                }
                Stage::HeaderCrc { have } => {
                    self.small[have] = input[ip];
                    ip += 1;
                    self.total_in += 1;
                    if have == 0 {
                        self.stage = Stage::HeaderCrc { have: 1 };
                    } else {
                        let stored = u16::from_le_bytes([self.small[0], self.small[1]]);
                        let computed = (self.header_crc.finalize() & 0xFFFF) as u16;
                        if stored != computed {
                            return Err(Error::HeaderCrcMismatch { stored, computed });
                        }
                        self.begin_body();
                    }
                }
            }
        }
    }

    fn need_input(&self, consumed: usize, written: usize) -> Progress {
        Progress {
            consumed,
            written,
            status: Status::NeedsInput,
        }
    }

    /// The header stage after the optional field `done` (or after the fixed
    /// header when `done` is below `FEXTRA`).
    fn after(&mut self, done: u8) -> Stage {
        let flags = self.flags;
        if done < FEXTRA && flags & FEXTRA != 0 {
            return Stage::ExtraLength { have: 0 };
        }
        if done < FNAME && flags & FNAME != 0 {
            return Stage::Name;
        }
        if done < FCOMMENT && flags & FCOMMENT != 0 {
            return Stage::Comment;
        }
        if flags & FHCRC != 0 {
            return Stage::HeaderCrc { have: 0 };
        }
        self.begin_body();
        self.stage
    }

    fn begin_body(&mut self) {
        self.inflater.reset();
        self.inflater
            .set_limit(self.limit.saturating_sub(self.completed_out));
        self.crc = Crc32::new();
        self.member_out = 0;
        self.stage = Stage::Body;
    }

    fn end_member(&mut self) -> Result<(), Error> {
        let [c0, c1, c2, c3, s0, s1, s2, s3] = self.small;
        let stored_crc = u32::from_le_bytes([c0, c1, c2, c3]);
        let stored_size = u32::from_le_bytes([s0, s1, s2, s3]);
        let computed_crc = self.crc.finalize();
        let computed_size = self.member_out as u32;
        if stored_crc != computed_crc {
            return Err(Error::CrcMismatch {
                member: self.members,
                stored: stored_crc,
                computed: computed_crc,
            });
        }
        if stored_size != computed_size {
            return Err(Error::SizeMismatch {
                member: self.members,
                stored: stored_size,
                computed: computed_size,
            });
        }
        self.members += 1;
        self.completed_out += self.member_out;
        self.member_out = 0;
        self.stage = Stage::Boundary;
        Ok(())
    }
}

/// Decode a whole gzip file (every member).
pub fn decompress(data: &[u8]) -> Result<Vec<u8>, Error> {
    decompress_with_limit(data, u64::MAX)
}

/// [`decompress`], refusing more than `limit` decoded bytes.
pub fn decompress_with_limit(data: &[u8], limit: u64) -> Result<Vec<u8>, Error> {
    let mut decoder = GzipDecoder::new();
    decoder.set_limit(limit);
    // Incompressible members need about their input size. Multiplying that
    // estimate retains unused memory without avoiding growth for highly
    // compressed members. Respect a caller's smaller output limit as well.
    let capacity = data
        .len()
        .min(1 << 26)
        .min(usize::try_from(limit).unwrap_or(usize::MAX));
    let mut out = Vec::with_capacity(capacity);
    decoder.feed_to_vec(data, &mut out)?;
    decoder.finish()?;
    Ok(out)
}

/// Compress `data` as one gzip member at `level`.
pub fn compress(data: &[u8], level: Level) -> Vec<u8> {
    // A compressible large input must not reserve half its uncompressed size.
    // Start with at most one output chunk plus framing slack; Vec growth
    // remains amortized for incompressible streams.
    let capacity = (data.len() / 2 + 64).min(64 * 1024 + 64);
    let mut out = Vec::with_capacity(capacity);
    out.extend_from_slice(&HEADER);
    let mut deflater = Deflater::new(level);
    let mut crc = Crc32::new();
    // The complete input is already available: write into the result buffer
    // directly, without the fallible writer adapter's pending-output buffer.
    // Bounded chunks retain the streaming encoder's working-memory bound.
    for chunk in data.chunks(WRITE_CHUNK) {
        deflater.write(chunk, &mut out);
        crc.update(chunk);
    }
    deflater.finish(&mut out);
    out.extend_from_slice(&crc.finalize().to_le_bytes());
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out
}

// --- Read adapter ------------------------------------------------------------

/// A `Read` that decodes the gzip stream read from `R` as it is pulled.
///
/// Damage anywhere — a bad header, a corrupt block, a CRC or size mismatch, a
/// truncated member, trailing garbage — is an `io::Error` at the read that
/// reaches it (`UnexpectedEof` for truncation, `InvalidData` otherwise), with
/// the typed [`Error`] inside. A clean end of input returns `Ok(0)` only after
/// the last member's trailer has been verified.
pub struct GzipReader<R> {
    inner: R,
    decoder: GzipDecoder,
    buf: Box<[u8]>,
    start: usize,
    end: usize,
    eof: bool,
}

impl<R> std::fmt::Debug for GzipReader<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GzipReader")
            .field("decoder", &self.decoder)
            .finish_non_exhaustive()
    }
}

impl<R: Read> GzipReader<R> {
    /// Decode the gzip stream `inner` yields.
    pub fn new(inner: R) -> Self {
        Self::with_decoder(inner, GzipDecoder::new())
    }

    /// Decode with a limit on the decoded bytes (see
    /// [`GzipDecoder::set_limit`]).
    pub fn with_limit(inner: R, limit: u64) -> Self {
        let mut decoder = GzipDecoder::new();
        decoder.set_limit(limit);
        Self::with_decoder(inner, decoder)
    }

    fn with_decoder(inner: R, decoder: GzipDecoder) -> Self {
        Self {
            inner,
            decoder,
            buf: vec![0u8; 32 * 1024].into_boxed_slice(),
            start: 0,
            end: 0,
            eof: false,
        }
    }

    /// The underlying reader.
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// The underlying reader (bytes it has already yielded may be buffered
    /// here, unread by the decoder).
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> Read for GzipReader<R> {
    fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
        if out.is_empty() {
            return Ok(0);
        }
        loop {
            if self.start == self.end && !self.eof {
                let n = self.inner.read(&mut self.buf)?;
                if n == 0 {
                    self.eof = true;
                }
                self.start = 0;
                self.end = n;
            }
            let progress = self.decoder.feed(&self.buf[self.start..self.end], out)?;
            self.start += progress.consumed;
            if progress.written > 0 {
                return Ok(progress.written);
            }
            if self.eof && self.start == self.end {
                self.decoder.finish()?;
                return Ok(0);
            }
        }
    }
}

// --- Write adapter -----------------------------------------------------------

/// A `Write` that gzips everything written to it into `W`: one member,
/// deterministic bytes. Call [`Self::finish`] to write the final block and
/// trailer; a writer dropped unfinished leaves a truncated member, which every
/// decoder refuses.
pub struct GzipWriter<W: Write> {
    inner: W,
    deflater: Deflater,
    crc: Crc32,
    size: u64,
    out: Vec<u8>,
    out_pos: usize,
}

impl<W: Write> std::fmt::Debug for GzipWriter<W> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GzipWriter")
            .field("deflater", &self.deflater)
            .field("size", &self.size)
            .finish_non_exhaustive()
    }
}

/// Output gathered before it is passed to the inner writer.
const WRITE_CHUNK: usize = 16 * 1024;

impl<W: Write> GzipWriter<W> {
    /// Gzip into `inner` at `level`.
    pub fn new(inner: W, level: Level) -> Self {
        Self::with_deflater(inner, Deflater::new(level))
    }

    /// Gzip on a pinned kernel path, if this processor can run it.
    pub fn with_backend(inner: W, level: Level, backend: Backend) -> Option<Self> {
        Deflater::with_backend(level, backend).map(|d| Self::with_deflater(inner, d))
    }

    fn with_deflater(inner: W, deflater: Deflater) -> Self {
        Self {
            inner,
            deflater,
            crc: Crc32::new(),
            size: 0,
            out: HEADER.to_vec(),
            out_pos: 0,
        }
    }

    fn drain(&mut self) -> io::Result<()> {
        while self.out_pos < self.out.len() {
            match self.inner.write(&self.out[self.out_pos..]) {
                Ok(0) => return Err(io::Error::from(io::ErrorKind::WriteZero)),
                Ok(n) => self.out_pos += n,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        self.out.clear();
        self.out_pos = 0;
        Ok(())
    }

    /// Write the final block and the trailer, flush, and return the inner
    /// writer.
    pub fn finish(mut self) -> io::Result<W> {
        self.deflater.finish(&mut self.out);
        let crc = self.crc.finalize();
        self.out.extend_from_slice(&crc.to_le_bytes());
        self.out
            .extend_from_slice(&(self.size as u32).to_le_bytes());
        self.drain()?;
        self.inner.flush()?;
        Ok(self.inner)
    }
}

impl<W: Write> Write for GzipWriter<W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if data.is_empty() {
            return Ok(0);
        }
        // An error here has consumed none of this call's input. Once a slice
        // is accepted, leave its output pending for the next call or flush.
        self.drain()?;
        let taken = data.len().min(WRITE_CHUNK);
        self.deflater.write(&data[..taken], &mut self.out);
        self.crc.update(&data[..taken]);
        self.size += taken as u64;
        Ok(taken)
    }

    /// Make every accepted plaintext byte decodable by writing a DEFLATE sync
    /// boundary, then flush the inner writer. Flush boundaries are part of the
    /// deterministic encoded bytes.
    fn flush(&mut self) -> io::Result<()> {
        self.drain()?;
        self.deflater.sync_flush(&mut self.out);
        self.drain()?;
        self.inner.flush()
    }
}
