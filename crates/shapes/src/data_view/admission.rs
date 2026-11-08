// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original SHACL carrier admission operands and native composition failures.

use purrdf_core::{RdfDiagnostic, ViewLimits};
use purrdf_hash::error::{ErrorValue, ErrorVisitor, StructuredError};

/// A carrier admission refusal, retaining the original counts, limits and cause.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewAdmissionError(Box<Failure>);

#[derive(Clone, Debug, PartialEq, Eq)]
enum Failure {
    Mapping {
        count: u64,
        max_terms: usize,
        max_auxiliary_bytes: usize,
        bytes_per_term: usize,
        auxiliary_bytes: Option<usize>,
        conversion: Option<std::num::TryFromIntError>,
    },
    Composite(RdfDiagnostic),
}

impl ViewAdmissionError {
    pub(super) fn mapping(
        count: u64,
        limits: ViewLimits,
        bytes_per_term: usize,
        auxiliary_bytes: Option<usize>,
        conversion: Option<std::num::TryFromIntError>,
    ) -> Self {
        Self(Box::new(Failure::Mapping {
            count,
            max_terms: limits.max_terms,
            max_auxiliary_bytes: limits.max_auxiliary_bytes,
            bytes_per_term,
            auxiliary_bytes,
            conversion,
        }))
    }

    pub(crate) fn composite(cause: RdfDiagnostic) -> Self {
        Self(Box::new(Failure::Composite(cause)))
    }
}

impl std::fmt::Display for ViewAdmissionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_ref() {
            Failure::Mapping {
                auxiliary_bytes: None,
                ..
            } => f.write_str("SHACL term mapping size overflow"),
            Failure::Mapping { .. } => {
                f.write_str("SHACL view exceeds term or auxiliary retention limit")
            }
            Failure::Composite(cause) => cause.fmt(f),
        }
    }
}

impl std::error::Error for ViewAdmissionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self.0.as_ref() {
            Failure::Composite(cause) => Some(cause),
            Failure::Mapping {
                conversion: Some(cause),
                ..
            } => Some(cause),
            Failure::Mapping { .. } => None,
        }
    }
}

impl StructuredError for ViewAdmissionError {
    fn visit<'e>(&'e self, visitor: &mut dyn ErrorVisitor<'e>) {
        match self.0.as_ref() {
            Failure::Composite(cause) => {
                visitor.variant("shacl-view-composite");
                visitor.value(Some("cause"), ErrorValue::Cause(cause));
            }
            Failure::Mapping {
                count,
                max_terms,
                max_auxiliary_bytes,
                bytes_per_term,
                auxiliary_bytes,
                conversion,
            } => {
                visitor.variant(if auxiliary_bytes.is_some() {
                    "shacl-view-retention-limit"
                } else {
                    "shacl-view-mapping-overflow"
                });
                visitor.value(Some("count"), ErrorValue::Unsigned(u128::from(*count)));
                visitor.value(Some("max_terms"), ErrorValue::Unsigned(*max_terms as u128));
                visitor.value(
                    Some("representation_limit"),
                    ErrorValue::Unsigned(u128::from(u32::MAX)),
                );
                visitor.value(
                    Some("max_auxiliary_bytes"),
                    ErrorValue::ByteCount(*max_auxiliary_bytes as u128),
                );
                visitor.value(
                    Some("bytes_per_term"),
                    ErrorValue::ByteCount(*bytes_per_term as u128),
                );
                visitor.value(
                    Some("auxiliary_bytes"),
                    auxiliary_bytes.map_or(ErrorValue::Absent, |bytes| {
                        ErrorValue::ByteCount(bytes as u128)
                    }),
                );
                visitor.value(
                    Some("cause"),
                    conversion
                        .as_ref()
                        .map_or(ErrorValue::Absent, |cause| ErrorValue::Cause(cause)),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_view::{Dense, ShaclDatasetView};
    use purrdf_core::{CompositeDatasetView, RdfDatasetBuilder};
    use purrdf_testkit::error_capture::Capture;
    use std::error::Error;
    use std::sync::Arc;

    #[test]
    fn mapping_overflow_retains_full_width_count_before_representation_refusal() {
        let error =
            Dense::<CompositeDatasetView>::admit(u64::MAX, ViewLimits::default()).unwrap_err();
        let captured = Capture::read(&error);
        assert_eq!(captured.code, "shacl-view-mapping-overflow");
        assert!(
            matches!(captured.field("count"), ErrorValue::Unsigned(value) if value == u128::from(u64::MAX))
        );
        assert!(matches!(
            captured.field("auxiliary_bytes"),
            ErrorValue::Absent
        ));
        Dense::<CompositeDatasetView>::admit(1, ViewLimits::default()).expect("admitted neighbor");
    }

    #[test]
    fn term_supplement_retains_original_native_composite_failure_after_sources_drop() {
        let error = {
            let base = RdfDatasetBuilder::new().freeze().unwrap();
            let view = ShaclDatasetView::native(base);
            let mut terms = RdfDatasetBuilder::new();
            terms.intern_iri("https://example.org/extra").unwrap();
            let terms = terms.freeze().unwrap();
            view.with_extra_terms(Arc::clone(&terms), ViewLimits::default())
                .expect("admitted neighbor");
            view.with_extra_terms(
                terms,
                ViewLimits {
                    max_terms: 0,
                    ..ViewLimits::default()
                },
            )
            .expect_err("native composite refuses its term limit")
        };
        let cause = error
            .source()
            .unwrap()
            .downcast_ref::<RdfDiagnostic>()
            .unwrap();
        assert_eq!(
            Capture::read(&error).child("cause").code,
            Capture::read(cause).code
        );
        let shapes = crate::ShapesError::from(error);
        assert_eq!(shapes.failure_class(), crate::FailureClass::Processor);
    }
}
