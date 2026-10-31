// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original Zstandard codec owners, inspected in their defining crate.

use purrdf_zstd::decoding::{
    FrameSizeError,
    errors::{FrameDecoderError, ReadFrameHeaderError},
};

/// An original Zstandard refusal without a rendered diagnostic copy.
#[derive(Debug)]
#[non_exhaustive]
pub enum ZstdReason {
    /// Decoding a page body failed.
    PageDecode(FrameDecoderError),
    /// Determining the compressed frame extent failed.
    FrameExtent(FrameSizeError),
    /// Reading the frame header failed.
    FrameHeader(ReadFrameHeaderError),
}

impl purrdf_hash::error::StructuredError for ZstdReason {
    purrdf_hash::error_visit! {
        PageDecode(cause) => "columnar-zstd-page-decode" { "cause" => Cause(cause) },
        FrameExtent(cause) => "columnar-zstd-frame-extent" { "cause" => Cause(cause) },
        FrameHeader(cause) => "columnar-zstd-frame-header" { "cause" => Cause(cause) },
    }
}
impl core::fmt::Display for ZstdReason {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PageDecode(cause) => cause.fmt(f),
            Self::FrameExtent(cause) => write!(f, "{cause:?}"),
            Self::FrameHeader(cause) => cause.fmt(f),
        }
    }
}
purrdf_lex::error_source!(ZstdReason => {
    Self::PageDecode(cause) => cause,
    Self::FrameExtent(cause) => cause,
    Self::FrameHeader(cause) => cause,
});
