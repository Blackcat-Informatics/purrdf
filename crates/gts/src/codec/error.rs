// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native transform refusals and opaque external decoder messages.

use core::fmt;

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

/// A native transform failure, retaining external explanations as messages.
#[derive(Debug)]
#[non_exhaustive]
pub enum CodecFailure {
    /// Original native gzip refusal.
    Gzip(purrdf_deflate::Error),
    /// External zstd dictionary loader explanation.
    ZstdDictionary(String),
    /// The output reservation failed before decoding.
    ZstdAllocation {
        /// Requested output-buffer capacity.
        capacity: usize,
        /// Opaque standard allocator explanation.
        message: String,
    },
    /// External zstd decoder explanation.
    ZstdDecode(String),
    /// External zstd header decoder explanation.
    ZstdHeader(String),
    /// External zstd frame-size decoder explanation.
    ZstdTruncated(String),
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
            Self::ZstdDictionary(message) => write!(f, "zstd dictionary load failed: {message}"),
            Self::ZstdAllocation { message, .. } => {
                write!(f, "zstd decode failed: output allocation failed: {message}")
            }
            Self::ZstdDecode(message) => write!(f, "zstd decode failed: {message}"),
            Self::ZstdHeader(message) => write!(f, "zstd frame header is unreadable: {message}"),
            Self::ZstdTruncated(message) => write!(f, "zstd frame is truncated: {message}"),
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

purrdf_lex::error_source!(CodecFailure { Gzip });

impl purrdf_hash::error::StructuredError for CodecFailure {
    purrdf_hash::error_visit! {
        Gzip(cause) => "gts-codec-gzip" { "cause" => Cause(cause) },
        ZstdDictionary(message) => "gts-codec-zstd-dictionary" { "message" => Message(message) },
        ZstdAllocation { capacity, message } => "gts-codec-zstd-allocation" {
            "capacity" => ByteCount(*capacity as u128), "message" => Message(message),
        },
        ZstdDecode(message) => "gts-codec-zstd-decode" { "message" => Message(message) },
        ZstdHeader(message) => "gts-codec-zstd-header" { "message" => Message(message) },
        ZstdTruncated(message) => "gts-codec-zstd-truncated" { "message" => Message(message) },
        CompressedLength { compressed_len, remaining } => "gts-codec-compressed-length" {
            "compressed_len" => ByteCount(*compressed_len as u128), "remaining" => ByteCount(*remaining as u128),
        },
        MissingContentSize => "gts-codec-missing-content-size" {},
        UnusedLevel { level } => "gts-codec-unused-level" { "level" => Signed(i128::from(*level)) },
        UnusedDictionary { bytes } => "gts-codec-unused-dictionary" { "bytes" => ByteCount(*bytes as u128) },
    }
}
