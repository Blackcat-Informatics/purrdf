// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Window-bounded decoding through the existing native transform engines.

use std::io::{self, Read};

use super::{Codec, CodecError, CodecUnavailable};

struct Limited<R> {
    inner: R,
    remaining: usize,
    limit: usize,
}

impl<R: Read> Read for Limited<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = output.len().min(self.remaining.saturating_add(1));
        let read = self.inner.read(&mut output[..count])?;
        self.remaining = self.remaining.checked_sub(read).ok_or_else(|| {
            purrdf_hash::error::io_error(
                io::ErrorKind::InvalidData,
                CodecError::Limit { limit: self.limit },
            )
        })?;
        Ok(read)
    }
}

/// Decode a finite transform chain without retaining its decoded payloads.
///
/// Each stage enforces the same caller limit as the eager decoder, including
/// intermediate transforms. Consumers must read to EOF to establish checksums
/// and complete-stream validity before publishing their provisional output.
/// Encryption still requires the existing authenticated decryptor API.
pub fn decode_chain_reader<'a>(
    chain: &[Codec],
    source: impl Read + 'a,
    limit: usize,
) -> io::Result<Box<dyn Read + 'a>> {
    let mut reader: Box<dyn Read + 'a> = Box::new(source);
    for (index, codec) in chain.iter().rev().enumerate() {
        reader = match codec.name.as_str() {
            _ if codec.cls == "encrypt" => {
                if index != 0 {
                    drain(&mut *reader)?;
                }
                return Err(unavailable(
                    "missing-key",
                    CodecUnavailable::MissingKey(codec.name.clone()),
                ));
            }
            "identity" => reader,
            "gzip" => Box::new(purrdf_deflate::gzip::GzipReader::new(reader)),
            "zstd" | "zstd-rsyncable" => Box::new(purrdf_zstd::decoding::FrameSequence::new(
                reader,
                codec.dct.as_deref(),
            )?),
            _ => {
                if index != 0 {
                    drain(&mut *reader)?;
                }
                return Err(unavailable(
                    "unknown-codec",
                    CodecUnavailable::UnknownCodec(codec.name.clone()),
                ));
            }
        };
        reader = Box::new(Limited {
            inner: reader,
            remaining: limit,
            limit,
        });
    }
    if chain.is_empty() {
        reader = Box::new(Limited {
            inner: reader,
            remaining: limit,
            limit,
        });
    }
    Ok(reader)
}

fn drain(source: &mut dyn Read) -> io::Result<()> {
    let mut scratch = [0; 8192];
    loop {
        match source.read(&mut scratch) {
            Ok(0) => return Ok(()),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

fn unavailable(reason: &'static str, detail: CodecUnavailable) -> io::Error {
    purrdf_hash::error::io_error(
        io::ErrorKind::Unsupported,
        CodecError::Unavailable { reason, detail },
    )
}
