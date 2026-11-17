// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original certificate causes and owned CBOR refusal operands.

use std::fmt;

use purrdf_gts::compact::CompactRefusedError;
use purrdf_hash::error::StructuredError;
use purrdf_lex::cbor::{DecodeError, Value};

use super::RefoldCanon;
use crate::{CanonError, RdfDiagnostic};

/// The required shape of a certificate field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateFieldType {
    /// One CBOR text string.
    Text,
    /// One CBOR array.
    Array,
}

/// An original CBOR decoder refusal or certificate-shape failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum CertificateCborError {
    /// Original decoder error, including its private offset and kind.
    Decode(DecodeError),
    /// The decoded root is not a map.
    RootType {
        /// Original decoded root.
        actual: Value,
    },
    /// A required field is absent.
    MissingField {
        /// Original required key.
        field: &'static str,
    },
    /// The first matching field has the wrong shape.
    FieldType {
        /// Original required key.
        field: &'static str,
        /// Required CBOR shape.
        expected: CertificateFieldType,
        /// Original decoded field value.
        actual: Value,
    },
    /// The first non-text element of a required text array.
    ArrayElementType {
        /// Original required key.
        field: &'static str,
        /// Original zero-based array position.
        index: usize,
        /// Original decoded element.
        actual: Value,
    },
    /// The `refold_canon` field names a canonicalization this build does not
    /// know; it is never read as another one.
    UnknownCanon {
        /// The field's text, as decoded.
        value: String,
    },
    /// The certificate records a canonicalization the decoding type cannot
    /// carry: a [`CompactionCertificate`](super::CompactionCertificate) is
    /// always `rdfc10-sha256`.
    UnexpectedCanon {
        /// The canonicalization the certificate records.
        canon: RefoldCanon,
    },
}

impl fmt::Display for CertificateCborError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Decode(cause) => write!(f, "cannot parse certificate CBOR: {cause}"),
            Self::RootType { .. } => f.write_str("certificate CBOR root must be a map"),
            Self::MissingField { field } => write!(f, "certificate CBOR missing {field:?}"),
            Self::FieldType {
                field, expected, ..
            } => {
                let description = match expected {
                    CertificateFieldType::Text => "a text string",
                    CertificateFieldType::Array => "an array",
                };
                write!(f, "{field:?} must be {description}")
            }
            Self::ArrayElementType { field, .. } => {
                write!(f, "{field:?} must be an array of text strings")
            }
            Self::UnknownCanon { value } => {
                write!(f, "unknown refold canonicalization {value:?}")
            }
            Self::UnexpectedCanon { canon } => write!(
                f,
                "the certificate records the refold canonicalization {:?}, not rdfc10-sha256",
                canon.as_str()
            ),
        }
    }
}

purrdf_hash::error_sources!(CertificateCborError, Decode);

impl StructuredError for CertificateCborError {
    purrdf_hash::error_visit! {
        Decode(cause) => "rdf-certificate-cbor-decode" { "cause" => Cause(cause) },
        RootType { actual } => "rdf-certificate-cbor-root-type" {
            "expected" => Code("map"), "actual" => Record(actual),
        },
        MissingField { field } => "rdf-certificate-cbor-missing-field" {
            "field" => Code(field),
        },
        FieldType { field, expected, actual } => "rdf-certificate-cbor-field-type" {
            "field" => Code(field),
            "expected" => Code(match expected {
                CertificateFieldType::Text => "text", CertificateFieldType::Array => "array",
            }),
            "actual" => Record(actual),
        },
        ArrayElementType { field, index, actual } => "rdf-certificate-cbor-array-element-type" {
            "field" => Code(field), "index" => Unsigned(*index as u128),
            "expected" => Code("text"), "actual" => Record(actual),
        },
        UnknownCanon { value } => "rdf-certificate-cbor-unknown-canon" {
            "value" => Text(value),
        },
        UnexpectedCanon { canon } => "rdf-certificate-cbor-unexpected-canon" {
            "canon" => Code(canon.as_str()),
        },
    }
}

/// Errors from certificate authoring, verification, and serialization.
#[derive(Debug)]
#[non_exhaustive]
pub enum CertifyError {
    /// Original compaction pipeline refusal.
    Refused(CompactRefusedError),
    /// The GTS dataset bridge, or a fold, reported this diagnostic.
    Dataset(RdfDiagnostic),
    /// Original canonicalization source, profile or search-control refusal.
    CanonRefused(CanonError),
    /// Original CBOR decoder or certificate-shape refusal.
    Cbor(CertificateCborError),
    /// A freshly authored compaction changed the content-refold digest.
    Invariant {
        /// Original pre-compaction digest.
        pre_refold_digest: String,
        /// Original post-compaction digest.
        post_refold_digest: String,
    },
    /// Two certificates of different refold canonicalizations cannot be
    /// chained: their digests are not comparable.
    CanonMismatch {
        /// The first certificate's canonicalization.
        first: RefoldCanon,
        /// The second certificate's canonicalization.
        second: RefoldCanon,
    },
}

impl fmt::Display for CertifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(cause) => write!(f, "compaction refused: {cause}"),
            Self::Dataset(cause) => write!(f, "GTS dataset bridge failed: {cause}"),
            Self::CanonRefused(cause) => write!(f, "compaction verification failed: {cause}"),
            Self::Cbor(cause) => write!(f, "compaction certificate CBOR error: {cause}"),
            Self::Invariant {
                pre_refold_digest,
                post_refold_digest,
            } => write!(
                f,
                "compaction certificate invariant violated: compaction changed the content-refold digest: pre={pre_refold_digest} post={post_refold_digest}"
            ),
            Self::CanonMismatch { first, second } => write!(
                f,
                "cannot compose a {} certificate with a {} certificate",
                first.as_str(),
                second.as_str()
            ),
        }
    }
}

purrdf_hash::error_sources!(CertifyError, Refused, Dataset, CanonRefused, Cbor);
purrdf_lex::variant_from!(CertifyError {
    Refused(CompactRefusedError), Dataset(RdfDiagnostic), Cbor(CertificateCborError)
});

impl StructuredError for CertifyError {
    purrdf_hash::error_visit! {
        Refused(cause) => "rdf-certificate-compaction-refused" { "cause" => Cause(cause) },
        Dataset(cause) => "rdf-certificate-dataset" { "cause" => Cause(cause) },
        CanonRefused(cause) => "rdf-certificate-canonicalization" { "cause" => Cause(cause) },
        Cbor(cause) => "rdf-certificate-cbor" { "cause" => Cause(cause) },
        Invariant { pre_refold_digest, post_refold_digest } => "rdf-certificate-content-changed" {
            "pre_refold_digest" => Text(pre_refold_digest),
            "post_refold_digest" => Text(post_refold_digest),
        },
        CanonMismatch { first, second } => "rdf-certificate-canon-mismatch" {
            "first" => Code(first.as_str()), "second" => Code(second.as_str()),
        },
    }
}
