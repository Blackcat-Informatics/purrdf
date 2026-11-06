// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original archive coordinates and native decoding refusals.

/// A malformed USTAR record, retaining its original archive byte coordinates.
///
/// Inspect this error directly or through the shared borrowed error visitor.
/// Its human presentation does not encode the retained fields.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UstarError {
    /// The next fixed-size header would overflow the native address space.
    HeaderOffsetOverflow {
        /// Original header offset.
        offset: usize,
    },
    /// The archive ends inside a header.
    TruncatedHeader {
        /// Original header offset.
        offset: usize,
        /// Required header bytes.
        required: usize,
        /// Remaining archive bytes.
        available: usize,
    },
    /// The size field is not an admitted native USTAR size.
    InvalidSize {
        /// Original size-field offset.
        offset: usize,
        /// The complete original field, including padding and terminators.
        field: [u8; 12],
    },
    /// The declared member body would overflow the native address space.
    BodyOffsetOverflow {
        /// Original body offset.
        offset: usize,
        /// Declared body bytes.
        required: usize,
    },
    /// The archive ends inside a declared member body.
    TruncatedBody {
        /// Original body offset.
        offset: usize,
        /// Declared body bytes.
        required: usize,
        /// Remaining archive bytes.
        available: usize,
    },
    /// Member padding would overflow the native address space.
    PaddingOffsetOverflow {
        /// Original padding offset.
        offset: usize,
        /// Required padding bytes.
        required: usize,
    },
    /// The archive ends inside member padding.
    TruncatedPadding {
        /// Original padding offset.
        offset: usize,
        /// Required padding bytes.
        required: usize,
        /// Remaining archive bytes.
        available: usize,
    },
    /// A GNU LongLink body has no final NUL byte.
    UnterminatedLongName {
        /// Original LongLink body offset.
        offset: usize,
        /// Declared LongLink body bytes.
        length: usize,
    },
    /// A GNU LongLink name is not UTF-8.
    LongNameUtf8 {
        /// Original LongLink body offset, before advancing to the next record.
        offset: usize,
        /// Original UTF-8 decoding failure, relative to that body.
        cause: core::str::Utf8Error,
    },
    /// A regular record's name field is not UTF-8.
    NameUtf8 {
        /// Original name-field offset, before advancing to the next record.
        offset: usize,
        /// Original UTF-8 decoding failure, relative to that name field.
        cause: core::str::Utf8Error,
    },
}

impl UstarError {
    /// The original native decoder refusal, when this record failed UTF-8 admission.
    #[must_use]
    pub const fn utf8_error(&self) -> Option<&core::str::Utf8Error> {
        match self {
            Self::LongNameUtf8 { cause, .. } | Self::NameUtf8 { cause, .. } => Some(cause),
            _ => None,
        }
    }
}

impl purrdf_hash::error::StructuredError for UstarError {
    purrdf_hash::error_visit! {
        HeaderOffsetOverflow { offset } => "ustar-header-offset-overflow" {
            "offset" => ByteCount(*offset as u128),
        },
        TruncatedHeader { offset, required, available } => "ustar-truncated-header" {
            "offset" => ByteCount(*offset as u128),
            "required" => ByteCount(*required as u128),
            "available" => ByteCount(*available as u128),
        },
        InvalidSize { offset, field } => "ustar-invalid-size" {
            "offset" => ByteCount(*offset as u128),
            "field" => Bytes(field),
        },
        BodyOffsetOverflow { offset, required } => "ustar-body-offset-overflow" {
            "offset" => ByteCount(*offset as u128),
            "required" => ByteCount(*required as u128),
        },
        TruncatedBody { offset, required, available } => "ustar-truncated-body" {
            "offset" => ByteCount(*offset as u128),
            "required" => ByteCount(*required as u128),
            "available" => ByteCount(*available as u128),
        },
        PaddingOffsetOverflow { offset, required } => "ustar-padding-offset-overflow" {
            "offset" => ByteCount(*offset as u128),
            "required" => ByteCount(*required as u128),
        },
        TruncatedPadding { offset, required, available } => "ustar-truncated-padding" {
            "offset" => ByteCount(*offset as u128),
            "required" => ByteCount(*required as u128),
            "available" => ByteCount(*available as u128),
        },
        UnterminatedLongName { offset, length } => "ustar-unterminated-long-name" {
            "offset" => ByteCount(*offset as u128),
            "length" => ByteCount(*length as u128),
        },
        LongNameUtf8 { offset, cause } => "ustar-long-name-utf8" {
            "offset" => ByteCount(*offset as u128),
            "cause" => Cause(cause),
        },
        NameUtf8 { offset, cause } => "ustar-name-utf8" {
            "offset" => ByteCount(*offset as u128),
            "cause" => Cause(cause),
        },
    }
}

impl core::fmt::Display for UstarError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.write_str(match self {
            Self::HeaderOffsetOverflow { .. } => "USTAR archive: header offset overflow",
            Self::TruncatedHeader { .. } => "USTAR archive: truncated header",
            Self::InvalidSize { .. } => "USTAR archive: unreadable size field",
            Self::BodyOffsetOverflow { .. } => "USTAR archive: member body offset overflow",
            Self::TruncatedBody { .. } => "USTAR archive: member body overruns archive",
            Self::PaddingOffsetOverflow { .. } => "USTAR archive: padded member offset overflow",
            Self::TruncatedPadding { .. } => "USTAR archive: member padding overruns archive",
            Self::UnterminatedLongName { .. } => {
                "USTAR archive: LongLink name is not NUL-terminated"
            }
            Self::LongNameUtf8 { .. } => "USTAR archive: LongLink name is not UTF-8",
            Self::NameUtf8 { .. } => "USTAR archive: member name is not UTF-8",
        })
    }
}

purrdf_hash::error_sources!(UstarError, native utf8_error);
