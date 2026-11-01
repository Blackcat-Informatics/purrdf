// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native transform refusals and their original codec causes.

use core::fmt;
use purrdf_zstd::decoding::{
    FrameSizeError,
    errors::{DictionaryDecodeError, FrameDecoderError, ReadFrameHeaderError},
};

/// Why a transform chain could not be applied.
#[derive(Debug)]
#[non_exhaustive]
pub enum CodecError {
    /// Missing capability; the container may preserve the frame as opaque.
    Unavailable {
        /// Container reason token, independent of human presentation.
        reason: &'static str,
        /// Original codec selection or decryption refusal.
        detail: CodecUnavailable,
    },
    /// A codec or its supplied configuration refused the operation.
    Failed(CodecFailure),
    /// A caller's decoded-byte ceiling refused the output.
    Limit {
        /// Maximum decoded bytes per transform.
        limit: usize,
    },
}

impl CodecError {
    /// A known missing capability, as distinct from corrupt bytes, failed
    /// authentication, storage refusal or a malformed codec configuration.
    #[must_use]
    pub fn is_capability_unavailable(&self) -> bool {
        matches!(
            self,
            Self::Unavailable {
                detail: CodecUnavailable::MissingKey(_)
                    | CodecUnavailable::UnknownCodec(_)
                    | CodecUnavailable::NoDecryptor(_),
                ..
            }
        )
    }
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable { reason, detail } => write!(f, "{reason}: {detail}"),
            Self::Failed(detail) => detail.fmt(f),
            Self::Limit { limit } => write!(f, "decoded transform output exceeds {limit} bytes"),
        }
    }
}

purrdf_lex::error_source!(CodecError => {
    Self::Unavailable { detail, .. } => detail,
    Self::Failed(detail) => detail,
});

impl purrdf_hash::error::StructuredError for CodecError {
    purrdf_hash::error_visit! {
        Unavailable { reason, detail } => "gts-codec-unavailable" {
            "reason" => Code(reason), "detail" => Cause(detail),
        },
        Failed(detail) => "gts-codec-failed" { "detail" => Cause(detail) },
        Limit { limit } => "gts-codec-limit" { "limit" => ByteCount(*limit as u128) },
    }
}

/// Original missing codec or cryptographic capability.
#[derive(Debug)]
#[non_exhaustive]
pub enum CodecUnavailable {
    /// No key was installed for this encryption codec.
    MissingKey(String),
    /// The reader does not know this codec.
    UnknownCodec(String),
    /// The writer cannot encode this codec.
    WriterCodec(String),
    /// The installed decryptor does not implement this codec.
    NoDecryptor(String),
    /// The native envelope decoder or authenticator refused the bytes.
    Decrypt {
        /// Requested encryption codec.
        codec: String,
        /// Original native refusal.
        cause: crate::cose::Encrypt0Error,
    },
}

impl fmt::Display for CodecUnavailable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey(name) => write!(f, "no key for encrypt codec '{name}'"),
            Self::UnknownCodec(name) => write!(f, "unknown codec '{name}'"),
            Self::WriterCodec(name) => write!(f, "writer cannot encode with codec '{name}'"),
            Self::NoDecryptor(name) => write!(f, "no decryptor for encrypt codec '{name}'"),
            Self::Decrypt { codec, cause } => write!(f, "{codec} decrypt failed: {cause}"),
        }
    }
}

purrdf_lex::error_source!(CodecUnavailable => { Self::Decrypt { cause, .. } => cause });

impl purrdf_hash::error::StructuredError for CodecUnavailable {
    purrdf_hash::error_visit! {
        MissingKey(name) => "gts-codec-missing-key" { "codec" => Text(name) },
        UnknownCodec(name) => "gts-codec-unknown" { "codec" => Text(name) },
        WriterCodec(name) => "gts-codec-writer-unknown" { "codec" => Text(name) },
        NoDecryptor(name) => "gts-codec-no-decryptor" { "codec" => Text(name) },
        Decrypt { codec, cause } => "gts-codec-decrypt" {
            "codec" => Text(codec), "cause" => Cause(cause),
        },
    }
}

/// A native transform failure, retaining original codec owners.
#[derive(Debug)]
#[non_exhaustive]
pub enum CodecFailure {
    /// Original native gzip refusal.
    Gzip(purrdf_deflate::Error),
    /// Original dictionary refusal from the decoder.
    ZstdDecodeDictionary(FrameDecoderError),
    /// Original dictionary refusal from the encoder.
    ZstdEncodeDictionary(DictionaryDecodeError),
    /// The output reservation failed before decoding.
    ZstdAllocation {
        /// Requested output-buffer capacity.
        capacity: usize,
        /// Original standard allocator refusal.
        cause: std::collections::TryReserveError,
    },
    /// Original frame decoder refusal.
    ZstdDecode(FrameDecoderError),
    /// Original frame header refusal.
    ZstdHeader(ReadFrameHeaderError),
    /// Original compressed frame extent refusal.
    ZstdTruncated(FrameSizeError),
    /// A decoder reported a compressed size outside its input range.
    CompressedLength {
        /// Reported compressed byte count.
        compressed_len: usize,
        /// Remaining encoded input bytes.
        remaining: usize,
    },
    /// A frame omitted the content-size declaration needed for its cut point.
    MissingContentSize,
    /// A level was supplied without a zstd-family transform.
    UnusedLevel {
        /// Original caller-selected level.
        level: i32,
    },
    /// A dictionary was supplied without a zstd-family transform.
    UnusedDictionary {
        /// Original dictionary byte count.
        bytes: usize,
    },
}

impl fmt::Display for CodecFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gzip(cause) => write!(f, "gzip decode failed: {cause}"),
            Self::ZstdDecodeDictionary(cause) => write!(f, "zstd dictionary load failed: {cause}"),
            Self::ZstdEncodeDictionary(cause) => write!(f, "zstd dictionary load failed: {cause}"),
            Self::ZstdAllocation { cause, .. } => {
                write!(f, "zstd decode failed: output allocation failed: {cause}")
            }
            Self::ZstdDecode(cause) => write!(f, "zstd decode failed: {cause}"),
            Self::ZstdHeader(cause) => write!(f, "zstd frame header is unreadable: {cause}"),
            Self::ZstdTruncated(cause) => write!(f, "zstd frame is truncated: {cause:?}"),
            Self::CompressedLength {
                compressed_len,
                remaining,
            } => write!(
                f,
                "zstd frame reports invalid compressed length {compressed_len} for {remaining} remaining bytes"
            ),
            Self::MissingContentSize => f.write_str(
                "zstd frame omits its Frame_Content_Size, so its cut point is unobservable",
            ),
            Self::UnusedLevel { .. } => {
                f.write_str("zstd_level requires a zstd or zstd-rsyncable transform")
            }
            Self::UnusedDictionary { .. } => {
                f.write_str("dict requires a zstd or zstd-rsyncable transform")
            }
        }
    }
}

purrdf_lex::error_source!(CodecFailure => {
    Self::Gzip(cause) => cause,
    Self::ZstdDecodeDictionary(cause) => cause,
    Self::ZstdEncodeDictionary(cause) => cause,
    Self::ZstdAllocation { cause, .. } => cause,
    Self::ZstdDecode(cause) => cause,
    Self::ZstdHeader(cause) => cause,
    Self::ZstdTruncated(cause) => cause,
});

impl purrdf_hash::error::StructuredError for CodecFailure {
    purrdf_hash::error_visit! {
        Gzip(cause) => "gts-codec-gzip" { "cause" => Cause(cause) },
        ZstdDecodeDictionary(cause) => "gts-codec-zstd-decode-dictionary" { "cause" => Cause(cause) },
        ZstdEncodeDictionary(cause) => "gts-codec-zstd-encode-dictionary" { "cause" => Cause(cause) },
        ZstdAllocation { capacity, cause } => "gts-codec-zstd-allocation" {
            "capacity" => ByteCount(*capacity as u128), "cause" => Cause(cause),
        },
        ZstdDecode(cause) => "gts-codec-zstd-decode" { "cause" => Cause(cause) },
        ZstdHeader(cause) => "gts-codec-zstd-header" { "cause" => Cause(cause) },
        ZstdTruncated(cause) => "gts-codec-zstd-truncated" { "cause" => Cause(cause) },
        CompressedLength { compressed_len, remaining } => "gts-codec-compressed-length" {
            "compressed_len" => ByteCount(*compressed_len as u128), "remaining" => ByteCount(*remaining as u128),
        },
        MissingContentSize => "gts-codec-missing-content-size" {},
        UnusedLevel { level } => "gts-codec-unused-level" { "level" => Signed(i128::from(*level)) },
        UnusedDictionary { bytes } => "gts-codec-unused-dictionary" { "bytes" => ByteCount(*bytes as u128) },
    }
}
