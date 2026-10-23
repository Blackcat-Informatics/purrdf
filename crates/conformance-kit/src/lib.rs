// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Portable conformance support over parsed RDF and result values.
//!
//! Acquisition, parsers, engines and language transports belong to callers. This
//! crate owns structural manifest reading and grading; graph equality delegates
//! to the kernel's fallible canonicalizer. An unavailable or malformed input is
//! an observed non-pass, never an expected semantic rejection.

#![forbid(unsafe_code)]

pub mod bag;
pub mod graph;
pub mod inventory;
pub mod manifest;
pub mod outcome;
pub mod report;
pub mod result_set;
pub mod reviews;

mod identity;

use std::fmt;

/// A grading refusal distinct from an unequal, well-formed pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GradeError {
    /// Native report construction or comparison refused physical or logical storage.
    Storage(purrdf_lex::allocation::StorageError),
    /// The input does not satisfy its declared result or manifest shape.
    Malformed(String),
    /// Well-formed inputs disagree under the declared comparison.
    Mismatch(String),
    /// The kernel could not establish canonical equality within its limits.
    Canonicalization(String),
    /// A report subject has two records for one test: EARL asserts one outcome
    /// per subject and test, so a second would contradict or shadow the first.
    DuplicateAssertion {
        /// The `earl:subject` IRI.
        subject: String,
        /// The `earl:test` IRI asserted twice.
        case: String,
    },
}

impl fmt::Display for GradeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, message) = match self {
            Self::Storage(error) => return write!(f, "storage refused: {error}"),
            Self::DuplicateAssertion { subject, case } => {
                return write!(
                    f,
                    "duplicate assertion: subject {subject} has two records for test {case}"
                );
            }
            Self::Malformed(message) => ("malformed input", message),
            Self::Mismatch(message) => ("mismatch", message),
            Self::Canonicalization(message) => ("canonicalization refused", message),
        };
        write!(f, "{kind}: {message}")
    }
}

purrdf_lex::error_source!(GradeError { Storage });

purrdf_lex::variant_from!(GradeError { Storage(purrdf_lex::allocation::StorageError) });

impl From<purrdf_core::RdfDiagnostic> for GradeError {
    fn from(error: purrdf_core::RdfDiagnostic) -> Self {
        error
            .storage_error()
            .map_or_else(|| Self::Malformed(error.to_string()), Self::Storage)
    }
}

impl From<purrdf_core::CanonError> for GradeError {
    fn from(error: purrdf_core::CanonError) -> Self {
        match error {
            purrdf_core::CanonError::Storage(error) => Self::Storage(error),
            purrdf_core::CanonError::Invalid(error) => error.storage_error().map_or_else(
                || {
                    let cause: purrdf_core::CanonError = purrdf_core::CanonError::Invalid(error);
                    Self::Canonicalization(cause.to_string())
                },
                Self::Storage,
            ),
            other => Self::Canonicalization(other.to_string()),
        }
    }
}

impl
    From<
        purrdf_core::ViewCanonError<
            <purrdf_core::RdfDataset as purrdf_core::DatasetView>::ReadError,
            (),
        >,
    > for GradeError
{
    fn from(
        error: purrdf_core::ViewCanonError<
            <purrdf_core::RdfDataset as purrdf_core::DatasetView>::ReadError,
            (),
        >,
    ) -> Self {
        match error {
            purrdf_core::ViewCanonError::NotReady { error, .. } => match error {},
            purrdf_core::ViewCanonError::Refused(error) => error.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::GradeError;

    #[test]
    fn native_capacity_refusal_survives_report_and_canonicalization_boundaries() {
        let cause: purrdf_lex::allocation::StorageError = purrdf_core::TermId::from_index(u32::MAX)
            .unwrap_err()
            .into();
        for error in [
            GradeError::from(purrdf_core::RdfDiagnostic::from(cause)),
            GradeError::from(purrdf_core::CanonError::Storage(cause)),
            GradeError::from(purrdf_core::CanonError::Invalid(cause.into())),
            GradeError::from(purrdf_core::ViewCanonError::Refused(
                purrdf_core::CanonError::Storage(cause),
            )),
        ] {
            assert_eq!(error, GradeError::Storage(cause));
            assert_eq!(
                std::error::Error::source(&error)
                    .unwrap()
                    .downcast_ref::<purrdf_lex::allocation::StorageError>(),
                Some(&cause)
            );
        }
    }
}
