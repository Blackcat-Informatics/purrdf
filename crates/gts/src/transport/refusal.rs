// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! First-party frame-sequencing refusals, separate from external zstd messages.

#[derive(Debug)]
pub(super) enum ZstdRefusal {
    NoFrame,
    TrailingPrefix { actual: usize },
    SkippableSize { actual: usize },
    SkippablePayload { actual: u64, expected: u64 },
    TrailingMagic { actual: [u8; 4] },
    TruncatedFrame,
}

impl ZstdRefusal {
    pub(super) fn into_io(self) -> std::io::Error {
        purrdf_hash::error::io_error(std::io::ErrorKind::InvalidData, self)
    }
}

impl core::fmt::Display for ZstdRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoFrame => f.write_str("no zstd frame in the input"),
            Self::TrailingPrefix { actual } => write!(
                f,
                "{actual} byte(s) after the last zstd frame do not begin another frame"
            ),
            Self::SkippableSize { .. } => {
                f.write_str("skippable zstd frame truncated in its size field")
            }
            Self::SkippablePayload { actual, expected } => write!(
                f,
                "skippable zstd frame truncated: {actual} of {expected} bytes"
            ),
            Self::TrailingMagic { actual } => write!(
                f,
                "bytes {actual:02x?} after the last zstd frame do not begin another frame"
            ),
            Self::TruncatedFrame => f.write_str("zstd frame truncated"),
        }
    }
}
impl std::error::Error for ZstdRefusal {}

impl purrdf_hash::error::StructuredError for ZstdRefusal {
    purrdf_hash::error_visit! {
        NoFrame => "gts-zstd-no-frame" {},
        TrailingPrefix { actual } => "gts-zstd-trailing-prefix" { "actual" => ByteCount(*actual as u128) },
        SkippableSize { actual } => "gts-zstd-skippable-size" {
            "actual" => ByteCount(*actual as u128), "expected" => ByteCount(4),
        },
        SkippablePayload { actual, expected } => "gts-zstd-skippable-payload" {
            "actual" => ByteCount(u128::from(*actual)), "expected" => ByteCount(u128::from(*expected)),
        },
        TrailingMagic { actual } => "gts-zstd-trailing-magic" { "actual" => Bytes(actual) },
        TruncatedFrame => "gts-zstd-truncated-frame" {},
    }
}
