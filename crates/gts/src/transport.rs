// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

use std::borrow::Cow;
use std::fmt;
use std::io::Read;

/// The gzip magic bytes (RFC 1952 §2.3.1: `ID1 = 0x1f`, `ID2 = 0x8b`).
const GZIP_MAGIC: &[u8] = &[0x1f, 0x8b];

/// The zstd frame magic number (RFC 8878 §3.1.1: `0xFD2FB528`, little-endian).
const ZSTD_MAGIC: &[u8] = &[0x28, 0xb5, 0x2f, 0xfd];

/// A recognized transport encoding wrapping a payload byte stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransportEncoding {
    /// gzip (RFC 1952), decoded by `flate2`'s pure-Rust `miniz_oxide` backend.
    Gzip,
    /// zstd (RFC 8878), decoded by the pure-Rust `structured-zstd` streaming decoder.
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
/// Both decoders are `Read` adapters read with `read_to_end`, so a truncated or
/// corrupt stream returns `Err` and **no** bytes: there is no partial success.
pub fn decode_transport(
    data: &[u8],
    encoding: TransportEncoding,
) -> Result<Vec<u8>, TransportError> {
    let mut out = Vec::new();
    let _decoded_len = match encoding {
        TransportEncoding::Gzip => flate2::read::GzDecoder::new(data)
            .read_to_end(&mut out)
            .map_err(|err| TransportError {
                encoding,
                message: err.to_string(),
            })?,
        TransportEncoding::Zstd => structured_zstd::decoding::StreamingDecoder::new(data)
            .map_err(|err| TransportError {
                encoding,
                message: err.to_string(),
            })?
            .read_to_end(&mut out)
            .map_err(|err| TransportError {
                encoding,
                message: err.to_string(),
            })?,
    };
    Ok(out)
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
    use std::io::Write as _;

    use super::*;

    fn gzip(payload: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(payload).expect("gzip write");
        encoder.finish().expect("gzip finish")
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
        assert!(!err.to_string().is_empty());
        let err = decode_transport(b"plain text", TransportEncoding::Zstd)
            .expect_err("plain bytes are not zstd");
        assert!(!err.to_string().is_empty());
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

    #[test]
    fn decode_detected_borrows_an_unwrapped_stream() {
        let plain = b"not wrapped";
        let decoded = decode_detected(plain, Some("a.nt")).expect("decode");
        assert!(matches!(decoded, Cow::Borrowed(_)));
        assert_eq!(decoded.as_ref(), plain);
    }
}
